use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::binary_ser::{Endianness, SUPPORTED_VERSION};
use unitas_asset::dependencies::DependencyGraph;
use unitas_asset::project::{Project, BASELINE_UNITY_VERSION};
use unitas_asset::trust::TrustStore;

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_GUID: &str = "11111111111111111111111111111111";
const MATERIAL_GUID: &str = "22222222222222222222222222222222";
const PREFAB_GUID: &str = "33333333333333333333333333333333";
const SCENE_GUID: &str = "44444444444444444444444444444444";
const BINARY_BUNDLE_GUID: &str = "55555555555555555555555555555555";

const CYCLE_A_GUID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CYCLE_B_GUID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CYCLE_C_GUID: &str = "cccccccccccccccccccccccccccccccc";
const LEAF_GUID: &str = "dddddddddddddddddddddddddddddddd";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn write_bytes(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn meta(guid: &str, importer: &str) -> String {
    format!(
        "fileFormatVersion: 2\nguid: {guid}\n{importer}\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"
    )
}

fn build_binary_serialized_asset(referenced_guid: &str) -> Vec<u8> {
    let unity_version = format!("{BASELINE_UNITY_VERSION}.16f1");
    let type_region: &[u8] = &[0x7f, 0x01, 0x02, 0x03];
    let payload = format!("guid: {referenced_guid}").into_bytes();
    let header_size = 20usize;

    let build_meta = |data_offset: u64| -> Vec<u8> {
        let mut meta = Vec::new();
        meta.extend_from_slice(&(unity_version.len() as u32).to_be_bytes());
        meta.extend_from_slice(unity_version.as_bytes());
        meta.extend_from_slice(&19i32.to_be_bytes());
        meta.push(1);
        meta.extend_from_slice(&1u32.to_be_bytes());
        meta.extend_from_slice(&(type_region.len() as u32).to_be_bytes());
        meta.extend_from_slice(type_region);
        meta.extend_from_slice(&1u32.to_be_bytes());
        meta.extend_from_slice(&1i64.to_be_bytes());
        meta.extend_from_slice(&21i32.to_be_bytes());
        meta.extend_from_slice(&data_offset.to_be_bytes());
        meta.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        meta.push(0);
        meta
    };

    let probe = build_meta(0);
    let data_offset = header_size + probe.len();
    let metadata = build_meta(data_offset as u64);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(metadata.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&SUPPORTED_VERSION.to_be_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_be_bytes());
    bytes.push(match Endianness::Big {
        Endianness::Little => 0,
        Endianness::Big => 1,
    });
    bytes.extend_from_slice(&[0, 0, 0]);
    bytes.extend_from_slice(&metadata);
    bytes.extend_from_slice(&payload);
    let file_size = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&file_size.to_be_bytes());
    bytes
}

#[test]
fn dependencies_direct_and_transitive_reverse() -> TestResult {
    let root = temp_dir("dependencies-chain")?;
    let project_root = root.join("project");

    write_file(
        &project_root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;

    write_file(&project_root.join("Assets/Textures/Albedo.png"), "png-bytes")?;
    write_file(
        &project_root.join("Assets/Textures/Albedo.png.meta"),
        &meta(TEXTURE_GUID, "TextureImporter:"),
    )?;

    let material_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n  m_SavedProperties:\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {{fileID: 2800000, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    write_file(&project_root.join("Assets/Materials/Stone.mat"), &material_yaml)?;
    write_file(
        &project_root.join("Assets/Materials/Stone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;

    let prefab_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: Pillar\n--- !u!23 &2300000\nMeshRenderer:\n  m_Materials:\n  - {{fileID: 2100000, guid: {MATERIAL_GUID}, type: 2}}\n"
    );
    write_file(&project_root.join("Assets/Prefabs/Pillar.prefab"), &prefab_yaml)?;
    write_file(
        &project_root.join("Assets/Prefabs/Pillar.prefab.meta"),
        &meta(PREFAB_GUID, "NativeFormatImporter:"),
    )?;

    let scene_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1001 &100100000\nPrefabInstance:\n  m_SourcePrefab: {{fileID: 100100000, guid: {PREFAB_GUID}, type: 3}}\n"
    );
    write_file(&project_root.join("Assets/Scenes/Level.unity"), &scene_yaml)?;
    write_file(
        &project_root.join("Assets/Scenes/Level.unity.meta"),
        &meta(SCENE_GUID, "NativeFormatImporter:"),
    )?;

    let binary_bytes = build_binary_serialized_asset(MATERIAL_GUID);
    write_bytes(&project_root.join("Assets/Bundles/Compiled.asset"), &binary_bytes)?;
    write_file(
        &project_root.join("Assets/Bundles/Compiled.asset.meta"),
        &meta(BINARY_BUNDLE_GUID, "NativeFormatImporter:"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;
    let graph = DependencyGraph::build(&project)?;

    assert_eq!(graph.dependencies_of(TEXTURE_GUID), Vec::<&str>::new());
    assert_eq!(graph.dependencies_of(MATERIAL_GUID), vec![TEXTURE_GUID]);
    assert_eq!(graph.dependencies_of(PREFAB_GUID), vec![MATERIAL_GUID]);
    assert_eq!(graph.dependencies_of(SCENE_GUID), vec![PREFAB_GUID]);
    assert_eq!(graph.dependencies_of(BINARY_BUNDLE_GUID), vec![MATERIAL_GUID]);

    assert_eq!(
        graph.reverse_dependencies_of(TEXTURE_GUID, false),
        vec![MATERIAL_GUID]
    );
    assert_eq!(
        graph.reverse_dependencies_of(TEXTURE_GUID, true),
        vec![MATERIAL_GUID, PREFAB_GUID, SCENE_GUID, BINARY_BUNDLE_GUID]
    );
    assert_eq!(
        graph.reverse_dependencies_of(MATERIAL_GUID, false),
        vec![PREFAB_GUID, BINARY_BUNDLE_GUID]
    );
    assert_eq!(
        graph.reverse_dependencies_of(MATERIAL_GUID, true),
        vec![PREFAB_GUID, SCENE_GUID, BINARY_BUNDLE_GUID]
    );
    assert!(graph.cycles().is_empty());

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn dependency_cycle_is_displayed_without_failure() -> TestResult {
    let root = temp_dir("dependencies-cycle")?;
    let project_root = root.join("project");

    write_file(
        &project_root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;

    let asset_a = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: NodeA\n  next: {{fileID: 11400000, guid: {CYCLE_B_GUID}, type: 2}}\n"
    );
    let asset_b = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: NodeB\n  next: {{fileID: 11400000, guid: {CYCLE_C_GUID}, type: 2}}\n"
    );
    let asset_c = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: NodeC\n  next: {{fileID: 11400000, guid: {CYCLE_A_GUID}, type: 2}}\n  leaf: {{fileID: 2800000, guid: {LEAF_GUID}, type: 3}}\n"
    );

    write_file(&project_root.join("Assets/Data/NodeA.asset"), &asset_a)?;
    write_file(
        &project_root.join("Assets/Data/NodeA.asset.meta"),
        &meta(CYCLE_A_GUID, "NativeFormatImporter:"),
    )?;
    write_file(&project_root.join("Assets/Data/NodeB.asset"), &asset_b)?;
    write_file(
        &project_root.join("Assets/Data/NodeB.asset.meta"),
        &meta(CYCLE_B_GUID, "NativeFormatImporter:"),
    )?;
    write_file(&project_root.join("Assets/Data/NodeC.asset"), &asset_c)?;
    write_file(
        &project_root.join("Assets/Data/NodeC.asset.meta"),
        &meta(CYCLE_C_GUID, "NativeFormatImporter:"),
    )?;
    write_file(&project_root.join("Assets/Data/Leaf.png"), "leaf")?;
    write_file(
        &project_root.join("Assets/Data/Leaf.png.meta"),
        &meta(LEAF_GUID, "TextureImporter:"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;
    let graph = DependencyGraph::build(&project)?;

    let cycles = graph.cycles();
    assert_eq!(
        cycles,
        vec![vec![
            CYCLE_A_GUID.to_string(),
            CYCLE_B_GUID.to_string(),
            CYCLE_C_GUID.to_string(),
        ]]
    );

    assert_eq!(
        graph.reverse_dependencies_of(CYCLE_A_GUID, false),
        vec![CYCLE_C_GUID]
    );
    assert_eq!(
        graph.reverse_dependencies_of(CYCLE_A_GUID, true),
        vec![CYCLE_B_GUID, CYCLE_C_GUID]
    );
    assert_eq!(
        graph.reverse_dependencies_of(LEAF_GUID, true),
        vec![CYCLE_A_GUID, CYCLE_B_GUID, CYCLE_C_GUID]
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
