use std::fmt;
use std::path::Path;

use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Node, Stream, YamlError};

const MATERIAL_CLASS: u32 = 21;
const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const MAIN_TEX: &str = "_MainTex";
const BUMP_TEX: &str = "_BumpMap";
const EMISSION_TEX: &str = "_EmissionMap";
const BASE_COLOR: &str = "_Color";
const EMISSION_COLOR: &str = "_EmissionColor";
const GLOSSINESS: &str = "_Glossiness";
const SMOOTHNESS: &str = "_Smoothness";
const METALLIC: &str = "_Metallic";

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

    let shader_guid = body
        .get("m_Shader")
        .and_then(|shader| shader.get("guid"))
        .and_then(Node::as_str)
        .filter(|guid| !guid.is_empty())
        .map(str::to_string)
        .ok_or(MaterialError::MissingShader)?;

    let shader_name = index
        .and_then(|guids| guids.get(&shader_guid))
        .and_then(|shader_path| shader_path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(str::to_string);

    let saved = body.get("m_SavedProperties");
    let color = saved
        .and_then(|saved| extract_color(saved, BASE_COLOR))
        .unwrap_or(DEFAULT_COLOR);
    let main_tex_guid = saved.and_then(|saved| extract_tex_guid(saved, MAIN_TEX));
    let bump_tex_guid = saved.and_then(|saved| extract_tex_guid(saved, BUMP_TEX));
    let emission_tex_guid = saved.and_then(|saved| extract_tex_guid(saved, EMISSION_TEX));
    let glossiness = saved
        .and_then(|saved| extract_float(saved, SMOOTHNESS))
        .or_else(|| saved.and_then(|saved| extract_float(saved, GLOSSINESS)));
    let metallic = saved.and_then(|saved| extract_float(saved, METALLIC));
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
