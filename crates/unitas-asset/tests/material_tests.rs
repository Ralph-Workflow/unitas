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

const EMISSION_MATERIAL: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!21 &2100000
Material:
  m_Name: Emissive
  m_Shader: {fileID: 46, guid: 0000000000000000f000000000000000, type: 0}
  m_SavedProperties:
    serializedVersion: 3
    m_TexEnvs:
    - _MainTex:
        m_Texture: {fileID: 0}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _BumpMap:
        m_Texture: {fileID: 2800000, guid: 22222222222222222222222222222222, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    - _EmissionMap:
        m_Texture: {fileID: 2800000, guid: 33333333333333333333333333333333, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    m_Floats:
    - _Glossiness: 0.25
    - _Smoothness: 0.75
    - _Metallic: 0.5
    m_Colors:
    - _Color: {r: 0.1, g: 0.2, b: 0.3, a: 1}
    - _EmissionColor: {r: 0.9, g: 0.8, b: 0.7, a: 1}
";

const SAMPLE_ROOT: &str = "../../samples/roll-a-ball";
const SAMPLE_PICKUP_MATERIAL: &str = "Assets/Materials/Pickup.mat";
const SAMPLE_GOLD_MATERIAL: &str = "Assets/Materials/Gold.mat";
const SAMPLE_PICKUP_TEXTURE_META: &str = "Assets/Textures/Pickup.png.meta";
const SAMPLE_PICKUP_NORMAL_META: &str = "Assets/Textures/PickupNormal.png.meta";
const SAMPLE_PICKUP_EMISSION_META: &str = "Assets/Textures/PickupEmission.png.meta";
const SAMPLE_PICKUP_TEXTURE: &str = "Assets/Textures/Pickup.png";
const SAMPLE_PICKUP_NORMAL_TEXTURE: &str = "Assets/Textures/PickupNormal.png";
const SAMPLE_PICKUP_EMISSION_TEXTURE: &str = "Assets/Textures/PickupEmission.png";

type TestResult = Result<(), Box<dyn Error>>;

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAMPLE_ROOT)
}

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
            bump_tex_guid: None,
            emission_tex_guid: None,
            glossiness: Some(0.5),
            metallic: Some(0.0),
            emission_color: None,
        }
    );
    Ok(())
}

#[test]
fn sample_pickup_material_carries_albedo_bump_emission_and_surface_properties() -> TestResult {
    let root = sample_root();
    let material = parse_material(&root.join(SAMPLE_PICKUP_MATERIAL))?;
    let albedo_guid = meta::parse_meta_file(&root.join(SAMPLE_PICKUP_TEXTURE_META))?;
    let bump_guid = meta::parse_meta_file(&root.join(SAMPLE_PICKUP_NORMAL_META))?;
    let emission_guid = meta::parse_meta_file(&root.join(SAMPLE_PICKUP_EMISSION_META))?;

    assert_eq!(
        material.main_tex_guid.as_deref(),
        Some(albedo_guid.as_str())
    );
    assert_eq!(material.bump_tex_guid.as_deref(), Some(bump_guid.as_str()));
    assert_eq!(
        material.emission_tex_guid.as_deref(),
        Some(emission_guid.as_str())
    );
    assert_eq!(material.glossiness, Some(0.6));
    assert_eq!(material.metallic, Some(0.0));
    assert_eq!(material.emission_color, Some([0.25, 1.0, 0.9, 1.0]));
    assert_eq!(material.color, [0.18, 0.77, 0.71, 1.0]);

    let index = GuidIndex::build(&root)?;
    assert_eq!(
        index.get(&albedo_guid),
        Some(root.join(SAMPLE_PICKUP_TEXTURE).as_path())
    );
    assert_eq!(
        index.get(&bump_guid),
        Some(root.join(SAMPLE_PICKUP_NORMAL_TEXTURE).as_path())
    );
    assert_eq!(
        index.get(&emission_guid),
        Some(root.join(SAMPLE_PICKUP_EMISSION_TEXTURE).as_path())
    );
    Ok(())
}

#[test]
fn sample_gold_material_reports_no_bump_or_emission_properties() -> TestResult {
    let root = sample_root();
    let material = parse_material(&root.join(SAMPLE_GOLD_MATERIAL))?;
    assert_eq!(material.main_tex_guid, None);
    assert_eq!(material.bump_tex_guid, None);
    assert_eq!(material.emission_tex_guid, None);
    assert_eq!(material.glossiness, Some(0.8));
    assert_eq!(material.metallic, Some(1.0));
    assert_eq!(material.emission_color, None);
    assert_eq!(material.color, [1.0, 0.84, 0.0, 1.0]);
    Ok(())
}

#[test]
fn material_parses_bump_emission_and_smoothness_properties() -> TestResult {
    let stream = unity_yaml::parse(EMISSION_MATERIAL)?;
    let material = parse_material_stream(&stream)?;
    assert_eq!(
        material.bump_tex_guid.as_deref(),
        Some("22222222222222222222222222222222")
    );
    assert_eq!(
        material.emission_tex_guid.as_deref(),
        Some("33333333333333333333333333333333")
    );
    assert_eq!(material.glossiness, Some(0.75));
    assert_eq!(material.metallic, Some(0.5));
    assert_eq!(material.emission_color, Some([0.9, 0.8, 0.7, 1.0]));
    assert_eq!(material.color, [0.1, 0.2, 0.3, 1.0]);
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
