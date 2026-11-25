use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::database::{AssetFilter, DatabaseError};
use unitas_asset::project::{AssetKind, GuidLookup, Project, BASELINE_UNITY_VERSION};
use unitas_asset::trust::TrustStore;

type TestResult = Result<(), Box<dyn Error>>;

const MATERIAL_GUID: &str = "11111111111111111111111111111111";
const STONE_MATERIAL_GUID: &str = "55555555555555555555555555555555";
const TEXTURE_GUID: &str = "22222222222222222222222222222222";
const SCENE_GUID: &str = "33333333333333333333333333333333";

const MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: Brick\n";

const STONE_MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: StoneBrick\n";

const SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n--- !u!1 &1000\nGameObject:\n  m_Name: Cube\n  m_Material: {fileID: 2100000, guid: 11111111111111111111111111111111, type: 2}\n";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path =
        std::env::temp_dir().join(format!("unitas-db-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn meta_with_labels(guid: &str, importer: &str, labels: &[&str]) -> String {
    let mut text = format!("fileFormatVersion: 2\nguid: {guid}\n");
    if !labels.is_empty() {
        text.push_str("labels:\n");
        for label in labels {
            text.push_str(&format!("- {label}\n"));
        }
    }
    text.push_str(&format!(
        "{importer}\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"
    ));
    text
}

fn texture_meta(guid: &str) -> String {
    format!(
        "fileFormatVersion: 2\nguid: {guid}\nlabels:\n- environment\nTextureImporter:\n  externalObjects: {{}}\n  serializedVersion: 12\n  spriteMode: 2\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: Brick_Slice_A\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 0\n        width: 8\n        height: 8\n    - serializedVersion: 2\n      name: Tile_Slice_B\n      rect:\n        serializedVersion: 2\n        x: 8\n        y: 0\n        width: 8\n        height: 8\n"
    )
}

fn build_fixture(root: &Path) -> std::io::Result<()> {
    write_file(&root.join("Assets/Materials/Brick.mat"), MATERIAL)?;
    write_file(
        &root.join("Assets/Materials/Brick.mat.meta"),
        &meta_with_labels(
            MATERIAL_GUID,
            "NativeFormatImporter:",
            &["architecture", "masonry"],
        ),
    )?;
    write_file(
        &root.join("Assets/Materials/StoneBrick.mat"),
        STONE_MATERIAL,
    )?;
    write_file(
        &root.join("Assets/Materials/StoneBrick.mat.meta"),
        &meta_with_labels(STONE_MATERIAL_GUID, "NativeFormatImporter:", &["nature"]),
    )?;
    write_file(
        &root.join("Assets/Textures/BrickSheet.png"),
        "not a real png",
    )?;
    write_file(
        &root.join("Assets/Textures/BrickSheet.png.meta"),
        &texture_meta(TEXTURE_GUID),
    )?;
    write_file(&root.join("Assets/Scenes/Main.unity"), SCENE)?;
    write_file(
        &root.join("Assets/Scenes/Main.unity.meta"),
        &meta_with_labels(SCENE_GUID, "NativeFormatImporter:", &["level"]),
    )?;
    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        &format!(
            "m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\nm_EditorVersionWithRevision: {BASELINE_UNITY_VERSION}.16f1 (0cf5a3b0e6a3)\n"
        ),
    )?;
    Ok(())
}

fn referenced_material_guid(scene_path: &Path) -> Result<String, Box<dyn Error>> {
    let text = fs::read_to_string(scene_path)?;
    for line in text.lines() {
        if let Some(index) = line.find("guid:") {
            let value = line[index + "guid:".len()..]
                .split([',', '}'])
                .next()
                .unwrap_or("")
                .trim();
            if value.len() == 32 {
                return Ok(value.to_string());
            }
        }
    }
    Err("scene holds no GUID reference".into())
}

#[test]
fn move_preserves_guid_and_reference() -> TestResult {
    let root = temp_dir("move")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;
    assert_eq!(ref_guid, MATERIAL_GUID);

    project.move_asset(
        Path::new("Assets/Materials/Brick.mat"),
        Path::new("Assets/Moved/Walls/BrickWall.mat"),
    )?;

    assert!(!project_root.join("Assets/Materials/Brick.mat").exists());
    assert!(!project_root
        .join("Assets/Materials/Brick.mat.meta")
        .exists());
    assert!(project_root
        .join("Assets/Moved/Walls/BrickWall.mat")
        .is_file());
    assert!(project_root
        .join("Assets/Moved/Walls/BrickWall.mat.meta")
        .is_file());

    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID);
            assert_eq!(
                entry.path,
                PathBuf::from("Assets/Moved/Walls/BrickWall.mat")
            );
        }
        other => return Err(format!("expected moved asset, got {other:?}").into()),
    }

    let duplicate_dest = project.move_asset(
        Path::new("Assets/Moved/Walls/BrickWall.mat"),
        Path::new("Assets/Materials/StoneBrick.mat"),
    );
    assert_eq!(
        duplicate_dest,
        Err(DatabaseError::DestinationExists {
            path: PathBuf::from("Assets/Materials/StoneBrick.mat"),
        })
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn delete_exposes_missing_reference() -> TestResult {
    let root = temp_dir("delete")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;

    project.delete_asset(Path::new("Assets/Materials/Brick.mat"))?;

    assert!(!project_root.join("Assets/Materials/Brick.mat").exists());
    assert!(!project_root
        .join("Assets/Materials/Brick.mat.meta")
        .exists());
    assert!(project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat")
        .is_file());
    assert!(project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat.meta")
        .is_file());

    assert_eq!(project.inventory().lookup(&ref_guid), GuidLookup::Missing);

    let (_, reopened_report) = Project::open(&project_root, &store)?;
    assert_eq!(
        reopened_report.inventory.lookup(&ref_guid),
        GuidLookup::Missing
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn restore_re_resolves_reference() -> TestResult {
    let root = temp_dir("restore")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (mut project, _) = Project::open(&project_root, &store)?;
    let ref_guid = referenced_material_guid(&project_root.join("Assets/Scenes/Main.unity"))?;

    project.delete_asset(Path::new("Assets/Materials/Brick.mat"))?;
    assert_eq!(project.inventory().lookup(&ref_guid), GuidLookup::Missing);

    project.restore_asset(Path::new("Assets/Materials/Brick.mat"))?;

    assert!(project_root.join("Assets/Materials/Brick.mat").is_file());
    assert!(project_root
        .join("Assets/Materials/Brick.mat.meta")
        .is_file());
    assert!(!project_root
        .join(".unitas/trash/Assets/Materials/Brick.mat")
        .exists());

    match project.inventory().lookup(&ref_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.guid, MATERIAL_GUID);
            assert_eq!(entry.path, PathBuf::from("Assets/Materials/Brick.mat"));
        }
        other => return Err(format!("expected restored asset, got {other:?}").into()),
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn search_combines_filters_and_distinguishes_sub_assets() -> TestResult {
    let root = temp_dir("search")?;
    let project_root = root.join("project");
    build_fixture(&project_root)?;
    let store = TrustStore::new(root.join("trust.json"));

    let (project, _) = Project::open(&project_root, &store)?;

    let combined_main = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: Some("Material".to_string()),
        label: Some("architecture".to_string()),
        include_sub_assets: false,
    });
    assert_eq!(combined_main.len(), 1);
    assert_eq!(combined_main[0].guid, MATERIAL_GUID);
    assert_eq!(combined_main[0].kind, AssetKind::Main);

    let all_brick_main = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: None,
        label: None,
        include_sub_assets: false,
    });
    assert_eq!(all_brick_main.len(), 3);
    assert!(all_brick_main
        .iter()
        .all(|entry| entry.kind == AssetKind::Main));

    let all_brick_with_subs = project.search_assets(&AssetFilter {
        name_contains: Some("Brick".to_string()),
        class: None,
        label: None,
        include_sub_assets: true,
    });
    assert_eq!(all_brick_with_subs.len(), 5);
    let sub_matches: Vec<_> = all_brick_with_subs
        .iter()
        .filter(|entry| entry.kind == AssetKind::Sub)
        .collect();
    assert_eq!(sub_matches.len(), 2);

    let specific_sprite_sub = project.search_assets(&AssetFilter {
        name_contains: Some("Slice_A".to_string()),
        class: Some("Sprite".to_string()),
        label: Some("Brick_Slice_A".to_string()),
        include_sub_assets: true,
    });
    assert_eq!(specific_sprite_sub.len(), 1);
    assert_eq!(specific_sprite_sub[0].kind, AssetKind::Sub);
    assert_eq!(specific_sprite_sub[0].guid, TEXTURE_GUID);

    let sub_excluded_when_disabled = project.search_assets(&AssetFilter {
        name_contains: Some("Slice_A".to_string()),
        class: Some("Sprite".to_string()),
        label: Some("Brick_Slice_A".to_string()),
        include_sub_assets: false,
    });
    assert!(sub_excluded_when_disabled.is_empty());

    fs::remove_dir_all(&root)?;
    Ok(())
}
