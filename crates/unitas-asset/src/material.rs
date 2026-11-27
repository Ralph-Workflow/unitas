use std::fmt;
use std::path::Path;

use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Node, Stream, YamlError};

const MATERIAL_CLASS: u32 = 21;
const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const URP_BASE_MAP: &str = "_BaseMap";
const MAIN_TEX: &str = "_MainTex";
const METALLIC_GLOSS_MAP: &str = "_MetallicGlossMap";
const BUMP_TEX: &str = "_BumpMap";
const EMISSION_TEX: &str = "_EmissionMap";
const URP_BASE_COLOR: &str = "_BaseColor";
const BASE_COLOR: &str = "_Color";
const EMISSION_COLOR: &str = "_EmissionColor";
const GLOSSINESS: &str = "_Glossiness";
const SMOOTHNESS: &str = "_Smoothness";
const METALLIC: &str = "_Metallic";

const BUILTIN_SHADER_GUID: &str = "0000000000000000f000000000000000";
const URP_LIT_SHADER_GUID: &str = "933532a4fcc9baf4fa0491de14d08ed7";
const URP_SIMPLE_LIT_SHADER_GUID: &str = "8d2bb70cbf9db8d4da26e15b26e74248";
const URP_UNLIT_SHADER_GUID: &str = "650dd9526735d5b46b79224bc6e94025";

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub shader_name: Option<String>,
    pub shader_guid: String,
    pub color: [f32; 4],
    pub main_tex_guid: Option<String>,
    pub bump_tex_guid: Option<String>,
    pub emission_tex_guid: Option<String>,
    pub glossiness: Option<f32>,
    pub metallic: Option<f32>,
    pub emission_color: Option<[f32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialError {
    MissingMaterial,
    MissingShader,
    Yaml { error: YamlError },
}

impl fmt::Display for MaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaterialError::MissingMaterial => {
                write!(formatter, "stream contains no Material document")
            }
            MaterialError::MissingShader => {
                write!(formatter, "Material document has no m_Shader guid")
            }
            MaterialError::Yaml { error } => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for MaterialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MaterialError::Yaml { error } => Some(error),
            _ => None,
        }
    }
}

impl From<YamlError> for MaterialError {
    fn from(error: YamlError) -> Self {
        MaterialError::Yaml { error }
    }
}

pub fn parse_material(path: &Path) -> Result<Material, MaterialError> {
    let stream = unity_yaml::parse_file(path)?;
    let index = path
        .parent()
        .and_then(|parent| GuidIndex::build(parent).ok());
    extract_material(&stream, index.as_ref())
}

pub fn parse_material_with_guids(
    path: &Path,
    index: &GuidIndex,
) -> Result<Material, MaterialError> {
    let stream = unity_yaml::parse_file(path)?;
    parse_material_stream_with_guids(&stream, index)
}

pub fn parse_material_stream(stream: &Stream) -> Result<Material, MaterialError> {
    extract_material(stream, None)
}

pub fn parse_material_stream_with_guids(
    stream: &Stream,
    index: &GuidIndex,
) -> Result<Material, MaterialError> {
    extract_material(stream, Some(index))
}

fn extract_material(stream: &Stream, index: Option<&GuidIndex>) -> Result<Material, MaterialError> {
    let body = stream
        .documents
        .iter()
        .find(|document| document.class_id == MATERIAL_CLASS && !document.stripped)
        .and_then(|document| document.root.get("Material"))
        .ok_or(MaterialError::MissingMaterial)?;

    let shader_node = body.get("m_Shader");
    let shader_guid = shader_node
        .and_then(|shader| shader.get("guid"))
        .and_then(Node::as_str)
        .filter(|guid| !guid.is_empty())
        .map(str::to_string)
        .ok_or(MaterialError::MissingShader)?;
    let shader_file_id = shader_node
        .and_then(|shader| shader.get("fileID"))
        .and_then(Node::as_i64);

    let shader_name = index
        .and_then(|guids| guids.get(&shader_guid))
        .and_then(|shader_path| shader_path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .or_else(|| known_shader_name(&shader_guid, shader_file_id).map(str::to_string));

    let saved = body.get("m_SavedProperties");
    let color = saved
        .and_then(|saved| {
            extract_color(saved, URP_BASE_COLOR).or_else(|| extract_color(saved, BASE_COLOR))
        })
        .unwrap_or(DEFAULT_COLOR);
    let main_tex_guid = saved.and_then(|saved| {
        extract_tex_guid(saved, URP_BASE_MAP).or_else(|| extract_tex_guid(saved, MAIN_TEX))
    });
    let bump_tex_guid = saved.and_then(|saved| extract_tex_guid(saved, BUMP_TEX));
    let emission_tex_guid = saved.and_then(|saved| extract_tex_guid(saved, EMISSION_TEX));
    let glossiness = saved
        .and_then(|saved| extract_float(saved, SMOOTHNESS))
        .or_else(|| saved.and_then(|saved| extract_float(saved, GLOSSINESS)));
    let metallic = saved
        .and_then(|saved| extract_float(saved, METALLIC))
        .or_else(|| {
            saved
                .and_then(|saved| extract_tex_guid(saved, METALLIC_GLOSS_MAP))
                .map(|_| 1.0)
        });
    let emission_color = saved.and_then(|saved| extract_color(saved, EMISSION_COLOR));

    Ok(Material {
        shader_name,
        shader_guid,
        color,
        main_tex_guid,
        bump_tex_guid,
        emission_tex_guid,
        glossiness,
        metallic,
        emission_color,
    })
}

fn known_shader_name(guid: &str, file_id: Option<i64>) -> Option<&'static str> {
    if guid.eq_ignore_ascii_case(URP_LIT_SHADER_GUID) {
        return Some("Universal Render Pipeline/Lit");
    }
    if guid.eq_ignore_ascii_case(URP_SIMPLE_LIT_SHADER_GUID) {
        return Some("Universal Render Pipeline/Simple Lit");
    }
    if guid.eq_ignore_ascii_case(URP_UNLIT_SHADER_GUID) {
        return Some("Universal Render Pipeline/Unlit");
    }
    if guid.eq_ignore_ascii_case(BUILTIN_SHADER_GUID) {
        return match file_id {
            Some(7) => Some("Legacy Shaders/Diffuse"),
            Some(10750) => Some("Unlit/Texture"),
            Some(10752) => Some("Unlit/Transparent"),
            Some(10753) => Some("Sprites/Default"),
            Some(10755) => Some("Unlit/Color"),
            _ => None,
        };
    }
    None
}

fn extract_color(saved: &Node, name: &str) -> Option<[f32; 4]> {
    let entries = saved.get("m_Colors")?.as_sequence()?;
    for entry in entries {
        if let Some(color_node) = entry.get(name) {
            let r = color_node.get("r").and_then(Node::as_f64)? as f32;
            let g = color_node.get("g").and_then(Node::as_f64)? as f32;
            let b = color_node.get("b").and_then(Node::as_f64)? as f32;
            let a = color_node.get("a").and_then(Node::as_f64)? as f32;
            return Some([r, g, b, a]);
        }
    }
    None
}

fn extract_float(saved: &Node, name: &str) -> Option<f32> {
    let entries = saved.get("m_Floats")?.as_sequence()?;
    for entry in entries {
        if let Some(value) = entry.get(name) {
            let value = value.as_f64()? as f32;
            return Some(value);
        }
    }
    None
}

fn extract_tex_guid(saved: &Node, name: &str) -> Option<String> {
    let entries = saved.get("m_TexEnvs")?.as_sequence()?;
    for entry in entries {
        if let Some(texture) = entry.get(name) {
            let guid = texture
                .get("m_Texture")
                .and_then(|texture| texture.get("guid"))
                .and_then(Node::as_str)
                .filter(|guid| !guid.is_empty())?;
            return Some(guid.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn Error>>;

    const URP_LIT_GUID: &str = "933532a4fcc9baf4fa0491de14d08ed7";
    const URP_SIMPLE_LIT_GUID: &str = "8d2bb70cbf9db8d4da26e15b26e74248";
    const URP_UNLIT_GUID: &str = "650dd9526735d5b46b79224bc6e94025";
    const BUILTIN_SHADER_GUID: &str = "0000000000000000f000000000000000";

    const URP_LIT_MATERIAL: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!21 &2100000
Material:
  m_Name: UrpCrate
  m_Shader: {fileID: 4800000, guid: 933532a4fcc9baf4fa0491de14d08ed7, type: 3}
  m_SavedProperties:
    serializedVersion: 3
    m_TexEnvs:
    - _BaseMap:
        m_Texture: {fileID: 2800000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _MainTex:
        m_Texture: {fileID: 0}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _MetallicGlossMap:
        m_Texture: {fileID: 2800000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _BumpMap:
        m_Texture: {fileID: 2800000, guid: cccccccccccccccccccccccccccccccc, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _EmissionMap:
        m_Texture: {fileID: 2800000, guid: dddddddddddddddddddddddddddddddd, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    m_Floats:
    - _Smoothness: 0.85
    - _Glossiness: 0.4
    - _Metallic: 0.25
    m_Colors:
    - _BaseColor: {r: 0.2, g: 0.6, b: 0.9, a: 0.75}
    - _Color: {r: 1, g: 1, b: 1, a: 1}
    - _EmissionColor: {r: 0.5, g: 0.25, b: 0.1, a: 1}
";

    const URP_LIT_METALLIC_MAP_ONLY: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!21 &2100000
Material:
  m_Name: UrpMetallicMapOnly
  m_Shader: {fileID: 4800000, guid: 933532a4fcc9baf4fa0491de14d08ed7, type: 3}
  m_SavedProperties:
    serializedVersion: 3
    m_TexEnvs:
    - _MetallicGlossMap:
        m_Texture: {fileID: 2800000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 3}
";

    #[test]
    fn urp_lit_material_maps_base_map_base_color_metallic_gloss_and_shader_name() -> TestResult {
        let stream = unity_yaml::parse(URP_LIT_MATERIAL)?;
        let material = parse_material_stream(&stream)?;
        assert_eq!(material.shader_guid, URP_LIT_GUID);
        assert_eq!(
            material.shader_name.as_deref(),
            Some("Universal Render Pipeline/Lit")
        );
        assert_eq!(
            material.main_tex_guid.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            material.bump_tex_guid.as_deref(),
            Some("cccccccccccccccccccccccccccccccc")
        );
        assert_eq!(
            material.emission_tex_guid.as_deref(),
            Some("dddddddddddddddddddddddddddddddd")
        );
        assert_eq!(material.color, [0.2, 0.6, 0.9, 0.75]);
        assert_eq!(material.glossiness, Some(0.85));
        assert_eq!(material.metallic, Some(0.25));
        assert_eq!(material.emission_color, Some([0.5, 0.25, 0.1, 1.0]));

        let map_only_stream = unity_yaml::parse(URP_LIT_METALLIC_MAP_ONLY)?;
        let map_only = parse_material_stream(&map_only_stream)?;
        assert_eq!(map_only.metallic, Some(1.0));
        assert_eq!(map_only.color, [1.0, 1.0, 1.0, 1.0]);
        Ok(())
    }

    #[test]
    fn known_urp_and_builtin_shader_identities_populate_shader_name_when_not_in_guid_index(
    ) -> TestResult {
        let cases: &[(&str, i64, Option<&str>)] = &[
            (URP_LIT_GUID, 4800000, Some("Universal Render Pipeline/Lit")),
            (
                URP_SIMPLE_LIT_GUID,
                4800000,
                Some("Universal Render Pipeline/Simple Lit"),
            ),
            (
                URP_UNLIT_GUID,
                4800000,
                Some("Universal Render Pipeline/Unlit"),
            ),
            (BUILTIN_SHADER_GUID, 7, Some("Legacy Shaders/Diffuse")),
            (BUILTIN_SHADER_GUID, 10750, Some("Unlit/Texture")),
            (BUILTIN_SHADER_GUID, 10752, Some("Unlit/Transparent")),
            (BUILTIN_SHADER_GUID, 10753, Some("Sprites/Default")),
            (BUILTIN_SHADER_GUID, 10755, Some("Unlit/Color")),
            ("ffffffffffffffffffffffffffffffff", 4800000, None),
        ];
        let empty_index = GuidIndex::default();
        for &(guid, file_id, expected_name) in cases {
            let yaml = format!(
                "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Test\n  m_Shader: {{fileID: {file_id}, guid: {guid}, type: 3}}\n"
            );
            let stream = unity_yaml::parse(&yaml)?;
            let material = parse_material_stream_with_guids(&stream, &empty_index)?;
            assert_eq!(material.shader_name.as_deref(), expected_name);
        }

        let mut custom_index = GuidIndex::default();
        custom_index.insert(
            String::from(URP_LIT_GUID),
            PathBuf::from("Assets/Shaders/CustomOverride.shader"),
        )?;
        let stream = unity_yaml::parse(URP_LIT_MATERIAL)?;
        let overridden = parse_material_stream_with_guids(&stream, &custom_index)?;
        assert_eq!(overridden.shader_name.as_deref(), Some("CustomOverride"));
        Ok(())
    }
}
