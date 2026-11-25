use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_asset::material::{
    parse_material, parse_material_stream, parse_material_stream_with_guids,
    parse_material_with_guids, Material, MaterialError,
};
use unitas_asset::meta::{self, GuidIndex};
use unitas_asset::unity_yaml::{self, YamlError};

const MATERIAL_FIXTURE: &str = "tests/fixtures/project/Material.mat";
const WOOD_TEXTURE_META: &str = "tests/fixtures/project/Textures/Wood.png.meta";
const STANDARD_SHADER_GUID: &str = "0000000000000000f000000000000000";
const CUSTOM_SHADER_GUID: &str = "11111111111111111111111111111111";

const CUSTOM_SHADER_MATERIAL: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!21 &2100000
Material:
  m_Name: LitMaterial
  m_Shader: {fileID: 4800000, guid: 11111111111111111111111111111111, type: 3}
  m_SavedProperties:
    serializedVersion: 3
    m_TexEnvs:
    - _MainTex:
        m_Texture: {fileID: 0}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    m_Colors:
    - _Color: {r: 0.25, g: 0.75, b: 0.5, a: 0.8}
";

const MISSING_MATERIAL_DOC: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: NotAMaterial
";

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn material_fixture_parses_shader_color_and_main_texture() -> TestResult {
    let wood_guid = meta::parse_meta_file(Path::new(WOOD_TEXTURE_META))?;
    let material = parse_material(Path::new(MATERIAL_FIXTURE))?;
    assert_eq!(
        material,
        Material {
            shader_name: None,
            shader_guid: String::from(STANDARD_SHADER_GUID),
            color: [1.0, 0.5, 0.25, 1.0],
            main_tex_guid: Some(wood_guid),
        }
    );
    Ok(())
}

#[test]
fn material_resolves_shader_name_from_guid_index() -> TestResult {
    let mut index = GuidIndex::default();
    index.insert(
        String::from(CUSTOM_SHADER_GUID),
        PathBuf::from("Assets/Shaders/CustomLit.shader"),
    )?;
    let stream = unity_yaml::parse(CUSTOM_SHADER_MATERIAL)?;
    let material = parse_material_stream_with_guids(&stream, &index)?;
    assert_eq!(material.shader_name, Some(String::from("CustomLit")));
    assert_eq!(material.shader_guid, CUSTOM_SHADER_GUID);
    assert_eq!(material.color, [0.25, 0.75, 0.5, 0.8]);
    assert_eq!(material.main_tex_guid, None);

    let mut standard_index = GuidIndex::default();
    standard_index.insert(
        String::from(STANDARD_SHADER_GUID),
        PathBuf::from("Shaders/Standard.shader"),
    )?;
    let fixture_material = parse_material_with_guids(Path::new(MATERIAL_FIXTURE), &standard_index)?;
    assert_eq!(fixture_material.shader_name, Some(String::from("Standard")));
    Ok(())
}

#[test]
fn material_parse_resolves_sibling_shader_meta_via_guid_index() -> TestResult {
    let temp = std::env::temp_dir().join(format!("unitas_material_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp)?;
    let shader_meta = temp.join("CustomLit.shader.meta");
    let mat_path = temp.join("LitMaterial.mat");
    std::fs::write(
        &shader_meta,
        format!("fileFormatVersion: 2\nguid: {CUSTOM_SHADER_GUID}\n"),
    )?;
    std::fs::write(&mat_path, CUSTOM_SHADER_MATERIAL)?;
    let material = parse_material(&mat_path);
    let _ = std::fs::remove_dir_all(&temp);
    let material = material?;
    assert_eq!(material.shader_name, Some(String::from("CustomLit")));
    assert_eq!(material.shader_guid, CUSTOM_SHADER_GUID);
    Ok(())
}

#[test]
fn material_missing_file_returns_yaml_io_error() {
    let result = parse_material(Path::new("tests/fixtures/project/DoesNotExist.mat"));
    assert!(matches!(
        result,
        Err(MaterialError::Yaml {
            error: YamlError::Io { .. }
        })
    ));
}

#[test]
fn material_stream_without_material_document_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(MISSING_MATERIAL_DOC)?;
    let result = parse_material_stream(&stream);
    assert!(matches!(result, Err(MaterialError::MissingMaterial)));
    Ok(())
}
