use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_asset::meta::{self, GuidIndex, MetaError};
use unitas_asset::project::{GuidLookup, Project};
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml::Node;

const PROJECT_DIR: &str = "tests/fixtures/project";
const CAMERA_GUID: &str = "4e9b7d1f2c3a4b5c6d7e8f9a0b1c2d3e";
const MATERIAL_GUID: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f809";
const TEXTURE_GUID: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";

#[test]
fn meta_guid_is_parsed_from_text() -> Result<(), MetaError> {
    let text = "fileFormatVersion: 2\nguid: 4e9b7d1f2c3a4b5c6d7e8f9a0b1c2d3e\nfolderAsset: yes\n";
    let guid = meta::parse_meta_text(text, Path::new("Camera.prefab.meta"))?;
    assert_eq!(guid, CAMERA_GUID);
    Ok(())
}

#[test]
fn meta_guid_is_parsed_from_file() -> Result<(), MetaError> {
    let path = Path::new(PROJECT_DIR).join("Camera.prefab.meta");
    let guid = meta::parse_meta_file(&path)?;
    assert_eq!(guid, CAMERA_GUID);
    Ok(())
}

#[test]
fn meta_missing_guid_is_an_error() -> Result<(), MetaError> {
    let result = meta::parse_meta_text("fileFormatVersion: 2\n", Path::new("empty.meta"));
    assert!(matches!(result, Err(MetaError::MissingGuid { .. })));
    Ok(())
}

#[test]
fn meta_invalid_guid_is_an_error() -> Result<(), MetaError> {
    let result = meta::parse_meta_text("guid: not-a-guid\n", Path::new("bad.meta"));
    assert!(matches!(result, Err(MetaError::InvalidGuid { .. })));
    Ok(())
}

#[test]
fn meta_index_maps_guid_to_asset_path() -> Result<(), MetaError> {
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    assert_eq!(index.len(), 3);
    assert!(!index.is_empty());
    assert_eq!(
        index.get(CAMERA_GUID),
        Some(Path::new("tests/fixtures/project/Camera.prefab"))
    );
    assert_eq!(
        index.get(MATERIAL_GUID),
        Some(Path::new("tests/fixtures/project/Material.mat"))
    );
    assert_eq!(
        index.get(TEXTURE_GUID),
        Some(Path::new("tests/fixtures/project/Textures/Wood.png"))
    );
    assert!(index.contains(CAMERA_GUID));
    Ok(())
}

#[test]
fn meta_index_returns_none_for_unknown_guid() -> Result<(), MetaError> {
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    assert!(index.get("00000000000000000000000000000000").is_none());
    assert!(!index.contains("00000000000000000000000000000000"));
    Ok(())
}

#[test]
fn meta_index_rejects_duplicate_guid() -> Result<(), MetaError> {
    let mut index = GuidIndex::default();
    index.insert(String::from(CAMERA_GUID), PathBuf::from("first.prefab"))?;
    let result = index.insert(String::from(CAMERA_GUID), PathBuf::from("second.prefab"));
    assert!(matches!(result, Err(MetaError::DuplicateGuid { .. })));
    assert_eq!(index.len(), 1);
    Ok(())
}

#[test]
fn meta_index_rejects_invalid_guid() -> Result<(), MetaError> {
    let mut index = GuidIndex::default();
    let result = index.insert(String::from("not-a-guid"), PathBuf::from("first.prefab"));
    assert!(matches!(result, Err(MetaError::InvalidGuid { .. })));
    assert!(index.is_empty());
    Ok(())
}

#[test]
fn meta_index_alias_works() -> Result<(), MetaError> {
    let mut index = unitas_asset::MetaIndex::default();
    index.insert(String::from(CAMERA_GUID), PathBuf::from("Camera.prefab"))?;
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(CAMERA_GUID), Some(Path::new("Camera.prefab")));
    Ok(())
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("unitas_asset_meta_{name}_{}", std::process::id()))
}

#[test]
fn prj003_outcome2_moving_an_asset_with_its_meta_preserves_the_guid_and_every_reference(
) -> Result<(), Box<dyn Error>> {
    let root = temp_root("prj003_outcome2");
    let _ = fs::remove_dir_all(&root);
    let project_root = root.join("project");
    let guid = "aaaaaaaabbbbbbbbccccccccdddddddd";
    let material_relative = Path::new("Assets/Materials/Brick.mat");
    let moved_relative = Path::new("Assets/Materials/MovedBrick.mat");

    fs::create_dir_all(project_root.join("Assets/Materials"))?;
    fs::create_dir_all(project_root.join("Assets/Scenes"))?;
    fs::write(
        project_root.join(material_relative),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Brick\n",
    )?;
    fs::write(
        project_root.join("Assets/Materials/Brick.mat.meta"),
        format!("fileFormatVersion: 2\nguid: {guid}\nNativeFormatImporter:\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"),
    )?;
    let scene_path = project_root.join("Assets/Scenes/Main.unity");
    fs::write(
        &scene_path,
        format!("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: BrickHolder\n  m_Material: {{fileID: 2100000, guid: {guid}, type: 2}}\n"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let (mut project, report) = Project::open(&project_root, &store)?;
    match report.inventory.lookup(guid) {
        GuidLookup::Asset(entry) => assert_eq!(entry.path, material_relative),
        other => return Err(format!("expected the material asset, got {other:?}").into()),
    }

    project.move_asset(material_relative, moved_relative)?;

    assert!(!project_root.join(material_relative).is_file());
    assert!(!project_root.join("Assets/Materials/Brick.mat.meta").is_file());
    assert!(project_root.join(moved_relative).is_file());
    assert!(project_root.join("Assets/Materials/MovedBrick.mat.meta").is_file());

    let index = GuidIndex::build(&project_root)?;
    assert_eq!(
        index.get(guid),
        Some(project_root.join(moved_relative).as_path())
    );

    let scene = unitas_asset::unity_yaml::parse_file(&scene_path)?;
    let reference = scene
        .documents
        .first()
        .and_then(|document| document.root.get("GameObject"))
        .and_then(|body| body.get("m_Material"))
        .ok_or("the scene lost its material reference")?;
    let reference_guid = reference
        .get("guid")
        .and_then(Node::as_str)
        .ok_or("the material reference lost its guid")?;
    assert_eq!(reference_guid, guid);
    assert_eq!(reference.get("fileID").and_then(Node::as_i64), Some(2100000));
    assert_eq!(
        index.get(reference_guid),
        Some(project_root.join(moved_relative).as_path())
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj003_outcome2_duplicate_guids_and_duplicate_local_ids_cannot_resolve_arbitrarily_and_missing_sub_asset_stays_missing(
) -> Result<(), Box<dyn Error>> {
    let root = temp_root("prj003_outcome2_duplicates");
    let _ = fs::remove_dir_all(&root);
    let project_root = root.join("project");
    let duplicate_guid = "11111111111111111111111111111111";
    let texture_guid = "22222222222222222222222222222222";

    fs::create_dir_all(project_root.join("Assets/Materials"))?;
    fs::create_dir_all(project_root.join("Assets/Textures"))?;
    fs::create_dir_all(project_root.join("ProjectSettings"))?;
    fs::write(
        project_root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.16f1\n",
    )?;
    fs::write(
        project_root.join("Assets/Materials/First.mat"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: First\n",
    )?;
    fs::write(
        project_root.join("Assets/Materials/First.mat.meta"),
        format!("fileFormatVersion: 2\nguid: {duplicate_guid}\nNativeFormatImporter:\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"),
    )?;
    fs::write(
        project_root.join("Assets/Materials/Second.mat"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Second\n",
    )?;
    fs::write(
        project_root.join("Assets/Materials/Second.mat.meta"),
        format!("fileFormatVersion: 2\nguid: {duplicate_guid}\nNativeFormatImporter:\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"),
    )?;
    fs::write(project_root.join("Assets/Textures/Atlas.png"), "png")?;
    fs::write(
        project_root.join("Assets/Textures/Atlas.png.meta"),
        format!(
            "fileFormatVersion: 2\nguid: {texture_guid}\nTextureImporter:\n  externalObjects: {{}}\n  serializedVersion: 12\n  spriteMode: 2\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: Slice_A\n"
        ),
    )?;

    assert!(matches!(
        GuidIndex::build(&project_root),
        Err(MetaError::DuplicateGuid { ref guid, .. }) if guid == duplicate_guid
    ));

    let store = TrustStore::new(root.join("trust.json"));
    let (_, report) = Project::open(&project_root, &store)?;
    match report.inventory.lookup(duplicate_guid) {
        GuidLookup::Duplicate(dup) => {
            assert_eq!(
                dup.paths,
                vec![
                    PathBuf::from("Assets/Materials/First.mat"),
                    PathBuf::from("Assets/Materials/Second.mat"),
                ]
            );
        }
        other => return Err(format!("expected duplicate guid report, got {other:?}").into()),
    }

    let dup_local_stream = unitas_asset::unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &777\nGameObject:\n  m_Name: First\n--- !u!1 &777\nGameObject:\n  m_Name: Second\n",
    )?;
    assert!(matches!(
        unitas_asset::scene::assemble_scene(&dup_local_stream),
        Err(unitas_asset::SceneError::DuplicateFileId { file_id: 777 })
    ));

    let missing_sub_asset_id: i64 = 21399999;
    let scene_path = project_root.join("Assets/MissingSub.unity");
    let scene_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: SpriteHolder\n--- !u!212 &20\nSpriteRenderer:\n  m_GameObject: {{fileID: 10}}\n  m_Sprite: {{fileID: {missing_sub_asset_id}, guid: {texture_guid}, type: 3}}\n"
    );
    fs::write(&scene_path, &scene_yaml)?;
    let stream = unitas_asset::unity_yaml::parse_file(&scene_path)?;
    unitas_asset::scene_save::save_stream(&scene_path, &stream)?;
    let reloaded = unitas_asset::unity_yaml::parse_file(&scene_path)?;
    let sprite_ref = reloaded.documents[1]
        .root
        .get("SpriteRenderer")
        .and_then(|n| n.get("m_Sprite"))
        .ok_or("missing m_Sprite")?;
    assert_eq!(
        sprite_ref.get("fileID").and_then(Node::as_i64),
        Some(missing_sub_asset_id)
    );
    assert_eq!(
        sprite_ref.get("guid").and_then(Node::as_str),
        Some(texture_guid)
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj003_outcome3_model_reimport_keeps_sub_asset_references_and_builtin_resource_references_keep_identity(
) -> Result<(), Box<dyn Error>> {
    let root = temp_root("prj003_outcome3");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;

    let model_guid = "33333333333333333333333333333333";
    let obj_v1 = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 0 1 1\ng Hull\nusemtl Steel\nf 1 2 3\ng Turret\nusemtl Brass\nf 4 5 6\n";
    let obj_v2 = b"v 0 0 0\nv 2 0 0\nv 0 2 0\nv 0 0 1\nv 1 0 1\nv 0 1 1\ng Turret\nusemtl Brass\nf 4 5 6\ng Hull\nusemtl Steel\nf 1 2 3\n";
    let imported_v1 = unitas_asset::model::import_model(obj_v1, "Tank.obj")?;
    let imported_v2 = unitas_asset::model::import_model(obj_v2, "Tank.obj")?;

    let v1_hull = imported_v1
        .sub_meshes
        .iter()
        .find(|s| s.sub_asset_id == "Hull/Steel")
        .ok_or("missing Hull/Steel in v1")?;
    let v2_hull = imported_v2
        .sub_meshes
        .iter()
        .find(|s| s.sub_asset_id == "Hull/Steel")
        .ok_or("missing Hull/Steel after reimport")?;
    assert_eq!(v1_hull.sub_asset_id, v2_hull.sub_asset_id);
    assert_eq!(v1_hull.name, v2_hull.name);

    let builtin_cube_file_id =
        unitas_asset::primitive::builtin_file_id(unitas_asset::primitive::BuiltinPrimitive::Cube);
    let builtin_guid = unitas_asset::primitive::BUILTIN_EXTRA_GUID;
    let scene_path = root.join("ModelAndBuiltin.unity");
    let scene_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_Name: TankAndBuiltin\n--- !u!33 &11\nMeshFilter:\n  m_GameObject: {{fileID: 10}}\n  m_Mesh: {{fileID: 4300000, guid: {model_guid}, type: 3}}\n  m_SubAssetId: Hull/Steel\n--- !u!33 &12\nMeshFilter:\n  m_GameObject: {{fileID: 10}}\n  m_Mesh: {{fileID: {builtin_cube_file_id}, guid: {builtin_guid}, type: 0}}\n"
    );
    fs::write(&scene_path, &scene_yaml)?;
    let stream = unitas_asset::unity_yaml::parse_file(&scene_path)?;
    unitas_asset::scene_save::save_stream(&scene_path, &stream)?;
    let reloaded = unitas_asset::unity_yaml::parse_file(&scene_path)?;

    let model_filter = reloaded.documents[1]
        .root
        .get("MeshFilter")
        .ok_or("missing model MeshFilter")?;
    assert_eq!(
        model_filter
            .get("m_Mesh")
            .and_then(|m| m.get("guid"))
            .and_then(Node::as_str),
        Some(model_guid)
    );
    assert_eq!(
        model_filter.get("m_SubAssetId").and_then(Node::as_str),
        Some(v2_hull.sub_asset_id.as_str())
    );

    let builtin_mesh = reloaded.documents[2]
        .root
        .get("MeshFilter")
        .and_then(|f| f.get("m_Mesh"))
        .ok_or("missing builtin m_Mesh")?;
    let reloaded_file_id = builtin_mesh
        .get("fileID")
        .and_then(Node::as_i64)
        .ok_or("missing builtin fileID")?;
    let reloaded_guid = builtin_mesh
        .get("guid")
        .and_then(Node::as_str)
        .ok_or("missing builtin guid")?;
    assert_eq!(reloaded_file_id, 10202);
    assert_eq!(reloaded_guid, builtin_guid);
    assert_eq!(
        unitas_asset::primitive::builtin_primitive_by_identity(reloaded_file_id, reloaded_guid),
        Some(unitas_asset::primitive::BuiltinPrimitive::Cube)
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
