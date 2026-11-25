use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::binary_ser::{Endianness, SUPPORTED_VERSION};
use unitas_asset::dependencies::{DependencyGraph, ImportCandidate, ImportOutcome, ImportSession};
use unitas_asset::project::{Project, BASELINE_UNITY_VERSION};
use unitas_asset::scene_save;
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml::{self, Node};

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_GUID: &str = "11111111111111111111111111111111";
const MATERIAL_GUID: &str = "22222222222222222222222222222222";
const PREFAB_GUID: &str = "33333333333333333333333333333333";
const SCENE_GUID: &str = "44444444444444444444444444444444";
const BINARY_BUNDLE_GUID: &str = "55555555555555555555555555555555";
const UNRELATED_GUID: &str = "66666666666666666666666666666666";

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

    write_file(
        &project_root.join("Assets/Textures/Albedo.png"),
        "png-bytes",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Albedo.png.meta"),
        &meta(TEXTURE_GUID, "TextureImporter:"),
    )?;

    let material_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n  m_SavedProperties:\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {{fileID: 2800000, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    write_file(
        &project_root.join("Assets/Materials/Stone.mat"),
        &material_yaml,
    )?;
    write_file(
        &project_root.join("Assets/Materials/Stone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;

    let prefab_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: Pillar\n--- !u!23 &2300000\nMeshRenderer:\n  m_Materials:\n  - {{fileID: 2100000, guid: {MATERIAL_GUID}, type: 2}}\n"
    );
    write_file(
        &project_root.join("Assets/Prefabs/Pillar.prefab"),
        &prefab_yaml,
    )?;
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
    write_bytes(
        &project_root.join("Assets/Bundles/Compiled.asset"),
        &binary_bytes,
    )?;
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
    assert_eq!(
        graph.dependencies_of(BINARY_BUNDLE_GUID),
        vec![MATERIAL_GUID]
    );

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
fn prj005_outcome7_dependency_cycles_are_displayed_without_failure() -> TestResult {
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

fn replace_guid_in_node(node: &mut Node, old_guid: &str, new_guid: &str) -> bool {
    match node {
        Node::Mapping(entries) => {
            let mut changed = false;
            for (key, val) in entries.iter_mut() {
                if key == "guid" {
                    if let Node::Scalar(current) = val {
                        if current == old_guid {
                            *current = new_guid.to_string();
                            changed = true;
                        }
                    }
                } else if replace_guid_in_node(val, old_guid, new_guid) {
                    changed = true;
                }
            }
            changed
        }
        Node::Sequence(items) => {
            let mut changed = false;
            for item in items.iter_mut() {
                if replace_guid_in_node(item, old_guid, new_guid) {
                    changed = true;
                }
            }
            changed
        }
        Node::Scalar(_) => false,
    }
}

#[test]
fn dependency_and_reverse_dependency_round_trip_across_unchanged_save_and_edited_save() -> TestResult
{
    let root = temp_dir("dependencies-round-trip")?;
    let project_root = root.join("project");

    write_file(
        &project_root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;

    write_file(
        &project_root.join("Assets/Textures/Albedo.png"),
        "png-bytes",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Albedo.png.meta"),
        &meta(TEXTURE_GUID, "TextureImporter:"),
    )?;

    write_file(
        &project_root.join("Assets/Textures/AltAlbedo.png"),
        "png-bytes-2",
    )?;
    write_file(
        &project_root.join("Assets/Textures/AltAlbedo.png.meta"),
        &meta(LEAF_GUID, "TextureImporter:"),
    )?;

    let material_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n  m_SavedProperties:\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {{fileID: 2800000, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    let material_path = project_root.join("Assets/Materials/Stone.mat");
    write_file(&material_path, &material_yaml)?;
    write_file(
        &project_root.join("Assets/Materials/Stone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;

    let prefab_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: Pillar\n--- !u!23 &2300000\nMeshRenderer:\n  m_Materials:\n  - {{fileID: 2100000, guid: {MATERIAL_GUID}, type: 2}}\n"
    );
    write_file(
        &project_root.join("Assets/Prefabs/Pillar.prefab"),
        &prefab_yaml,
    )?;
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

    let cycle_a = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: NodeA\n  next: {{fileID: 11400000, guid: {CYCLE_B_GUID}, type: 2}}\n"
    );
    let cycle_b = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: NodeB\n  next: {{fileID: 11400000, guid: {CYCLE_A_GUID}, type: 2}}\n"
    );
    write_file(&project_root.join("Assets/Data/NodeA.asset"), &cycle_a)?;
    write_file(
        &project_root.join("Assets/Data/NodeA.asset.meta"),
        &meta(CYCLE_A_GUID, "NativeFormatImporter:"),
    )?;
    write_file(&project_root.join("Assets/Data/NodeB.asset"), &cycle_b)?;
    write_file(
        &project_root.join("Assets/Data/NodeB.asset.meta"),
        &meta(CYCLE_B_GUID, "NativeFormatImporter:"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;
    let graph_initial = DependencyGraph::build(&project)?;

    assert_eq!(
        graph_initial.dependencies_of(MATERIAL_GUID),
        vec![TEXTURE_GUID]
    );
    assert_eq!(
        graph_initial.reverse_dependencies_of(TEXTURE_GUID, false),
        vec![MATERIAL_GUID]
    );
    assert_eq!(
        graph_initial.reverse_dependencies_of(TEXTURE_GUID, true),
        vec![MATERIAL_GUID, PREFAB_GUID, SCENE_GUID]
    );
    assert!(graph_initial
        .reverse_dependencies_of(LEAF_GUID, false)
        .is_empty());
    assert!(graph_initial
        .reverse_dependencies_of(LEAF_GUID, true)
        .is_empty());
    assert_eq!(
        graph_initial.cycles(),
        vec![vec![CYCLE_A_GUID.to_string(), CYCLE_B_GUID.to_string()]]
    );

    let mut material_stream = unity_yaml::parse_file(&material_path)?;
    scene_save::save_stream(&material_path, &material_stream)?;
    assert_eq!(fs::read_to_string(&material_path)?, material_yaml);

    let graph_after_unchanged = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_after_unchanged.dependencies_of(MATERIAL_GUID),
        graph_initial.dependencies_of(MATERIAL_GUID)
    );
    assert_eq!(
        graph_after_unchanged.reverse_dependencies_of(TEXTURE_GUID, false),
        graph_initial.reverse_dependencies_of(TEXTURE_GUID, false)
    );
    assert_eq!(
        graph_after_unchanged.reverse_dependencies_of(TEXTURE_GUID, true),
        graph_initial.reverse_dependencies_of(TEXTURE_GUID, true)
    );
    assert_eq!(graph_after_unchanged.cycles(), graph_initial.cycles());

    assert!(replace_guid_in_node(
        &mut material_stream.documents[0].root,
        TEXTURE_GUID,
        LEAF_GUID
    ));
    scene_save::save_stream(&material_path, &material_stream)?;

    let (reopened_project, _) = Project::open(&project_root, &store)?;
    let graph_after_edit = DependencyGraph::build(&reopened_project)?;

    assert_eq!(
        graph_after_edit.dependencies_of(MATERIAL_GUID),
        vec![LEAF_GUID]
    );
    assert!(graph_after_edit
        .reverse_dependencies_of(TEXTURE_GUID, false)
        .is_empty());
    assert!(graph_after_edit
        .reverse_dependencies_of(TEXTURE_GUID, true)
        .is_empty());
    assert_eq!(
        graph_after_edit.reverse_dependencies_of(LEAF_GUID, false),
        vec![MATERIAL_GUID]
    );
    assert_eq!(
        graph_after_edit.reverse_dependencies_of(LEAF_GUID, true),
        vec![MATERIAL_GUID, PREFAB_GUID, SCENE_GUID]
    );
    assert_eq!(
        graph_after_edit.dependencies_of(PREFAB_GUID),
        vec![MATERIAL_GUID]
    );
    assert_eq!(
        graph_after_edit.dependencies_of(SCENE_GUID),
        vec![PREFAB_GUID]
    );
    assert_eq!(
        graph_after_edit.cycles(),
        vec![vec![CYCLE_A_GUID.to_string(), CYCLE_B_GUID.to_string()]]
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj005_outcome7_pre_delete_reverse_dependency_inspection_missing_reference_retention_and_restore_round_trip(
) -> TestResult {
    let root = temp_dir("dependencies-delete-restore")?;
    let project_root = root.join("project");

    write_file(
        &project_root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;

    write_file(
        &project_root.join("Assets/Textures/Albedo.png"),
        "png-bytes",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Albedo.png.meta"),
        &meta(TEXTURE_GUID, "TextureImporter:"),
    )?;

    let sibling_guid = "99999999999999999999999999999999";
    write_file(
        &project_root.join("Assets/Textures/Albedo_Backup.png"),
        "png-sibling",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Albedo_Backup.png.meta"),
        &meta(sibling_guid, "TextureImporter:"),
    )?;

    let material_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n  m_SavedProperties:\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {{fileID: 2800000, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    let material_path = project_root.join("Assets/Materials/Stone.mat");
    write_file(&material_path, &material_yaml)?;
    write_file(
        &project_root.join("Assets/Materials/Stone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;

    let prefab_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: Pillar\n--- !u!23 &2300000\nMeshRenderer:\n  m_Materials:\n  - {{fileID: 2100000, guid: {MATERIAL_GUID}, type: 2}}\n"
    );
    write_file(
        &project_root.join("Assets/Prefabs/Pillar.prefab"),
        &prefab_yaml,
    )?;
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

    let store = TrustStore::new(root.join("trust.json"));
    let (mut project, _) = Project::open(&project_root, &store)?;

    let pre_delete_graph = DependencyGraph::build(&project)?;
    let direct_dependents = pre_delete_graph.reverse_dependencies_of(TEXTURE_GUID, false);
    let transitive_dependents = pre_delete_graph.reverse_dependencies_of(TEXTURE_GUID, true);
    assert_eq!(direct_dependents, vec![MATERIAL_GUID]);
    assert_eq!(
        transitive_dependents,
        vec![MATERIAL_GUID, PREFAB_GUID, SCENE_GUID]
    );

    project.delete_asset(Path::new("Assets/Textures/Albedo.png"))?;
    assert_eq!(
        project.inventory().lookup(TEXTURE_GUID),
        unitas_asset::project::GuidLookup::Missing
    );

    let graph_while_deleted = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_while_deleted.dependencies_of(MATERIAL_GUID),
        vec![TEXTURE_GUID]
    );
    assert_eq!(
        graph_while_deleted.reverse_dependencies_of(TEXTURE_GUID, false),
        vec![MATERIAL_GUID]
    );
    assert_eq!(
        graph_while_deleted.reverse_dependencies_of(TEXTURE_GUID, true),
        vec![MATERIAL_GUID, PREFAB_GUID, SCENE_GUID]
    );
    assert!(graph_while_deleted
        .reverse_dependencies_of(sibling_guid, true)
        .is_empty());

    let mat_stream_while_deleted = unity_yaml::parse_file(&material_path)?;
    scene_save::save_stream(&material_path, &mat_stream_while_deleted)?;
    assert_eq!(fs::read_to_string(&material_path)?, material_yaml);

    project.restore_asset(Path::new("Assets/Textures/Albedo.png"))?;
    match project.inventory().lookup(TEXTURE_GUID) {
        unitas_asset::project::GuidLookup::Asset(entry) => {
            assert_eq!(entry.path, PathBuf::from("Assets/Textures/Albedo.png"));
        }
        other => return Err(format!("expected restored Albedo.png, got {other:?}").into()),
    }

    let graph_after_restore = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_after_restore.reverse_dependencies_of(TEXTURE_GUID, false),
        direct_dependents
    );
    assert_eq!(
        graph_after_restore.reverse_dependencies_of(TEXTURE_GUID, true),
        transitive_dependents
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

fn referenced_texture_project(root: &Path) -> std::io::Result<()> {
    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;
    write_file(&root.join("Assets/Textures/Albedo.png"), "png-bytes")?;
    write_file(
        &root.join("Assets/Textures/Albedo.png.meta"),
        &meta(TEXTURE_GUID, "TextureImporter:"),
    )?;
    let material_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n  m_SavedProperties:\n    m_TexEnvs:\n    - _MainTex:\n        m_Texture: {{fileID: 2800000, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    write_file(&root.join("Assets/Materials/Stone.mat"), &material_yaml)?;
    write_file(
        &root.join("Assets/Materials/Stone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;
    Ok(())
}

#[test]
fn prj002_outcome3_move_delete_and_restore_preserve_expose_and_restore_the_reference() -> TestResult
{
    let root = temp_dir("dependencies-move-delete-restore")?;
    let project_root = root.join("project");
    referenced_texture_project(&project_root)?;

    let store = TrustStore::new(root.join("trust.json"));
    let (mut project, _) = Project::open(&project_root, &store)?;

    project.move_asset(
        Path::new("Assets/Textures/Albedo.png"),
        Path::new("Assets/Artwork/Albedo.png"),
    )?;
    let graph_after_move = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_after_move.dependencies_of(MATERIAL_GUID),
        vec![TEXTURE_GUID],
        "moving the referenced asset keeps the reference"
    );
    assert_eq!(
        graph_after_move.reverse_dependencies_of(TEXTURE_GUID, false),
        vec![MATERIAL_GUID]
    );

    project.delete_asset(Path::new("Assets/Artwork/Albedo.png"))?;
    assert_eq!(
        project.inventory().lookup(TEXTURE_GUID),
        unitas_asset::project::GuidLookup::Missing,
        "deleting the referenced asset exposes the missing reference"
    );
    let graph_while_deleted = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_while_deleted.dependencies_of(MATERIAL_GUID),
        vec![TEXTURE_GUID],
        "the reference to the deleted asset stays visible as missing"
    );

    project.restore_asset(Path::new("Assets/Artwork/Albedo.png"))?;
    match project.inventory().lookup(TEXTURE_GUID) {
        unitas_asset::project::GuidLookup::Asset(entry) => {
            assert_eq!(entry.path, PathBuf::from("Assets/Artwork/Albedo.png"));
        }
        other => return Err(format!("expected the restored asset, got {other:?}").into()),
    }
    let graph_after_restore = DependencyGraph::build(&project)?;
    assert_eq!(
        graph_after_restore.dependencies_of(MATERIAL_GUID),
        vec![TEXTURE_GUID],
        "restoring the identified asset restores the reference"
    );
    assert_eq!(
        graph_after_restore.reverse_dependencies_of(TEXTURE_GUID, false),
        vec![MATERIAL_GUID]
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj004_outcome6_reimport_touches_only_dependency_affected_assets() -> TestResult {
    let root = temp_dir("dependencies-reimport")?;
    let project_root = root.join("project");
    referenced_texture_project(&project_root)?;
    write_file(
        &project_root.join("Assets/Textures/Unrelated.png"),
        "other-png-bytes",
    )?;
    write_file(
        &project_root.join("Assets/Textures/Unrelated.png.meta"),
        &meta(UNRELATED_GUID, "TextureImporter:"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (project, _) = Project::open(&project_root, &store)?;
    let graph = DependencyGraph::build(&project)?;

    let candidate = |path: &str, guid: &str, source: Vec<u8>| ImportCandidate {
        path: PathBuf::from(path),
        guid: guid.to_string(),
        source,
        metadata: String::new(),
        settings: "TextureImporter".to_string(),
        importer_version: "1".to_string(),
        dependencies: graph
            .dependencies_of(guid)
            .iter()
            .map(|dependency| dependency.to_string())
            .collect(),
    };

    let albedo = candidate(
        "Assets/Textures/Albedo.png",
        TEXTURE_GUID,
        fs::read(project_root.join("Assets/Textures/Albedo.png"))?,
    );
    let material = candidate(
        "Assets/Materials/Stone.mat",
        MATERIAL_GUID,
        fs::read(project_root.join("Assets/Materials/Stone.mat"))?,
    );
    let unrelated = candidate(
        "Assets/Textures/Unrelated.png",
        UNRELATED_GUID,
        fs::read(project_root.join("Assets/Textures/Unrelated.png"))?,
    );

    let mut session = ImportSession::new();
    let mut importer = |_: &ImportCandidate| ImportOutcome::ready();

    let first = session.run(
        &[albedo.clone(), material.clone(), unrelated.clone()],
        &mut importer,
    );
    assert_eq!(first.imported, 3);
    assert_eq!(first.reused, 0);

    let changed = candidate(
        "Assets/Textures/Albedo.png",
        TEXTURE_GUID,
        b"changed-png-bytes".to_vec(),
    );
    let second = session.run(
        &[changed.clone(), material.clone(), unrelated.clone()],
        &mut importer,
    );

    assert_eq!(
        second.imported, 2,
        "only the changed asset and its dependent"
    );
    assert_eq!(second.reused, 1, "the unrelated asset keeps its result");
    let changed_record = second
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the changed texture")?;
    assert!(!changed_record.reused);
    assert_eq!(changed_record.reason, "source content changed");
    let dependent_record = second
        .record(Path::new("Assets/Materials/Stone.mat"))
        .ok_or("expected a record for the dependent material")?;
    assert!(!dependent_record.reused);
    assert_eq!(
        dependent_record.reason,
        format!("dependency changed: {TEXTURE_GUID}")
    );
    let unrelated_record = second
        .record(Path::new("Assets/Textures/Unrelated.png"))
        .ok_or("expected a record for the unrelated texture")?;
    assert!(unrelated_record.reused);
    assert!(
        unrelated_record.reason.contains("unchanged"),
        "expected the unrelated asset to state that its inputs did not change: {}",
        unrelated_record.reason
    );

    let mut meta_changed = changed;
    meta_changed.metadata = "sRGBTexture: 0".to_string();
    let third = session.run(
        &[meta_changed, material.clone(), unrelated.clone()],
        &mut importer,
    );
    assert_eq!(
        third.imported, 2,
        "PRJ-004 Outcome 6 .meta change reimports only the asset and its dependent"
    );
    assert_eq!(
        third.reused, 1,
        "PRJ-004 Outcome 6 unrelated asset is reused after .meta change"
    );
    let meta_record = third
        .record(Path::new("Assets/Textures/Albedo.png"))
        .ok_or("expected a record for the metadata-changed texture")?;
    assert_eq!(meta_record.reason, "metadata changed");

    fs::remove_dir_all(&root)?;
    Ok(())
}
