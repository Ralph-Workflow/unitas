use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::meta;
use unitas_asset::primitive::{
    builtin_primitive_by_identity, BuiltinPrimitive, BUILTIN_EXTRA_GUID,
};
use unitas_asset::project::{
    AssetInventory, AssetKind, GuidLookup, MigrationStatus, Project, ASSESSED_ON_EDIT,
    BASELINE_UNITY_VERSION,
};
use unitas_asset::scene_save;
use unitas_asset::trust::{TrustOrigin, TrustOutcome, TrustRefusal, TrustStore};
use unitas_asset::unity_yaml::{self, Node};

type TestResult = Result<(), Box<dyn Error>>;

const MATERIAL_GUID: &str = "11111111111111111111111111111111";
const TEXTURE_GUID: &str = "22222222222222222222222222222222";
const SCENE_GUID: &str = "33333333333333333333333333333333";
const SCRIPT_GUID: &str = "44444444444444444444444444444444";

const MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: Brick\n";

const SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n--- !u!1 &1000\nGameObject:\n  m_Name: Cube\n  m_Material: {fileID: 2100000, guid: 11111111111111111111111111111111, type: 2}\n";

const EDITOR_SCRIPT: &str = "using UnityEditor;\n\n[InitializeOnLoad]\npublic static class EditorBootstrap\n{\n    static EditorBootstrap()\n    {\n    }\n}\n";

const MANIFEST: &str = r#"{
  "dependencies": {
    "com.unity.ide.rider": "3.0.28",
    "com.example.git-package": {
      "git": "https://example.invalid/com.example.git-package.git",
      "hash": "1111111111111111111111111111111111111111"
    }
  }
}"#;

const LOCK: &str = r#"{
  "dependencies": {
    "com.unity.ide.rider": {
      "version": "3.0.28",
      "depth": 0,
      "source": "registry",
      "dependencies": { "com.unity.modules.imgui": "1.0.0" }
    },
    "com.unity.modules.imgui": {
      "version": "1.0.0",
      "depth": 1,
      "source": "registry"
    }
  }
}"#;

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

fn meta(guid: &str, importer: &str) -> String {
    format!(
        "fileFormatVersion: 2\nguid: {guid}\n{importer}\n  externalObjects: {{}}\n  serializedVersion: 2\n  userData: \n"
    )
}

fn texture_meta(guid: &str) -> String {
    format!(
        "fileFormatVersion: 2\nguid: {guid}\nTextureImporter:\n  externalObjects: {{}}\n  serializedVersion: 12\n  spriteMode: 2\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: Tile_A\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 0\n        width: 8\n        height: 8\n    - serializedVersion: 2\n      name: Tile_B\n      rect:\n        serializedVersion: 2\n        x: 8\n        y: 0\n        width: 8\n        height: 8\n"
    )
}

fn build_project(root: &Path, version: &str) -> std::io::Result<()> {
    write_file(&root.join("Assets/Materials/Brick.mat"), MATERIAL)?;
    write_file(
        &root.join("Assets/Materials/Brick.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;
    write_file(&root.join("Assets/Textures/Tile.png"), "not a real png")?;
    write_file(
        &root.join("Assets/Textures/Tile.png.meta"),
        &texture_meta(TEXTURE_GUID),
    )?;
    write_file(&root.join("Assets/Scenes/Main.unity"), SCENE)?;
    write_file(
        &root.join("Assets/Scenes/Main.unity.meta"),
        &meta(SCENE_GUID, "NativeFormatImporter:"),
    )?;
    write_file(
        &root.join("Assets/Editor/EditorBootstrap.cs"),
        EDITOR_SCRIPT,
    )?;
    write_file(
        &root.join("Assets/Editor/EditorBootstrap.cs.meta"),
        &meta(SCRIPT_GUID, "MonoImporter:"),
    )?;

    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        &format!(
            "m_EditorVersion: {version}\nm_EditorVersionWithRevision: {version} (0cf5a3b0e6a3)\n"
        ),
    )?;
    write_file(
        &root.join("ProjectSettings/ProjectSettings.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  serializedVersion: 26\n  companyName: Example\n  productName: Sample\n  bundleVersion: 1.0.0\n",
    )?;
    write_file(
        &root.join("ProjectSettings/TimeManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  serializedVersion: 5\n  m_FixedTimestep: 0.02\n",
    )?;
    write_file(
        &root.join("ProjectSettings/DynamicsManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nDynamicsManager:\n  serializedVersion: 3\n  m_Gravity: {x: 0, y: -9.81, z: 0}\n",
    )?;
    write_file(
        &root.join("ProjectSettings/InputManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!13 &1\nInputManager:\n  serializedVersion: 2\n  m_Axes:\n  - serializedVersion: 3\n    m_Name: Fire1\n    negativeButton: left\n    positiveButton: right\n    gravity: 1\n    dead: 0.1\n    sensitivity: 3\n    snap: 0\n    invert: 0\n    type: 0\n    axis: 0\n",
    )?;

    write_file(&root.join("Packages/manifest.json"), MANIFEST)?;
    write_file(&root.join("Packages/packages-lock.json"), LOCK)?;
    Ok(())
}

fn snapshot(root: &Path) -> std::io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(PathBuf, Vec<u8>)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.clone());
            files.push((relative, fs::read(&path)?));
        }
    }
    Ok(())
}

fn guid_references(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    collect_asset_files(&root.join("Assets"), &mut files);
    let mut references = Vec::new();
    for path in files {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        for line in text.lines() {
            let index = match line.find("guid:") {
                Some(index) => index,
                None => continue,
            };
            let value = line[index + "guid:".len()..].trim();
            let value = value.split([',', '}']).next().unwrap_or(value).trim();
            if value.len() == 32 && value.chars().all(|character| character.is_ascii_hexdigit()) {
                references.push((value.to_string(), path.to_string_lossy().to_string()));
            }
        }
    }
    references
}

fn collect_asset_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_asset_files(&path, files);
        } else if !path.to_string_lossy().ends_with(".meta") {
            files.push(path);
        }
    }
}

fn target_of(inventory: &AssetInventory, guid: &str) -> Option<String> {
    match inventory.lookup(guid) {
        GuidLookup::Asset(entry) => Some(entry.path.to_string_lossy().to_string()),
        GuidLookup::Duplicate(record) => Some(
            record
                .paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<String>>()
                .join(", "),
        ),
        GuidLookup::Missing => None,
    }
}

#[test]
fn project_open_twice_yields_identical_identities() -> TestResult {
    let root = temp_dir("project-open-twice")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    let store = TrustStore::new(root.join("trust.json"));

    let (first, first_report) = Project::open(&project_root, &store)?;
    let (second, second_report) = Project::open(&project_root, &store)?;

    assert_eq!(first_report.inventory, second_report.inventory);
    assert_eq!(first.inventory(), second.inventory());
    assert_eq!(
        first_report.declared_version,
        second_report.declared_version
    );
    assert_eq!(first_report.migration, second_report.migration);
    assert_eq!(first_report.migration, MigrationStatus::Current);

    let references = guid_references(&project_root);
    assert!(!references.is_empty(), "the fixture holds no reference");
    for (guid, source) in references {
        let first_target = target_of(&first_report.inventory, &guid);
        let second_target = target_of(&second_report.inventory, &guid);
        assert_eq!(
            first_target, second_target,
            "the reference from {source} changed target between opens"
        );
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn project_open_ignores_library_folder() -> TestResult {
    let root = temp_dir("project-library")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    let store = TrustStore::new(root.join("trust.json"));

    let (_, without_library) = Project::open(&project_root, &store)?;

    write_file(&project_root.join("Library/ArtifactDB"), "cache")?;
    write_file(&project_root.join("Library/Cached.asset"), "cached")?;
    write_file(
        &project_root.join("Library/Cached.asset.meta"),
        &meta("99999999999999999999999999999999", "NativeFormatImporter:"),
    )?;

    let (_, with_library) = Project::open(&project_root, &store)?;

    assert_eq!(without_library.inventory, with_library.inventory);
    assert!(with_library
        .inventory
        .entries
        .iter()
        .all(|entry| !entry.path.starts_with("Library")));

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn project_open_2021_3_reports_migration_without_writing() -> TestResult {
    let root = temp_dir("project-migration")?;
    let project_root = root.join("project");
    build_project(&project_root, "2021.3.16f1")?;
    let store = TrustStore::new(root.join("trust.json"));

    let before = snapshot(&project_root)?;
    let version_path = project_root.join("ProjectSettings/ProjectVersion.txt");
    let version_before = fs::read_to_string(&version_path)?;

    let (_, report) = Project::open(&project_root, &store)?;

    assert_eq!(report.declared_version.as_deref(), Some("2021.3.16f1"));
    match &report.migration {
        MigrationStatus::RequiresUpgrade(preview) => {
            assert!(
                preview
                    .assets
                    .iter()
                    .any(|asset| asset == "Assets/Materials/Brick.mat"),
                "the preview misses the material: {:?}",
                preview.assets
            );
            assert!(
                preview
                    .assets
                    .iter()
                    .any(|asset| asset.contains(ASSESSED_ON_EDIT)),
                "the preview misses the asset assessed on edit: {:?}",
                preview.assets
            );
            assert!(
                preview
                    .settings
                    .iter()
                    .any(|setting| setting == "TimeManager.asset"),
                "the preview misses the settings: {:?}",
                preview.settings
            );
            assert!(
                preview
                    .packages
                    .iter()
                    .any(|package| package == "com.unity.ide.rider"),
                "the preview misses the packages: {:?}",
                preview.packages
            );
        }
        other => return Err(format!("expected an upgrade preview, got {other:?}").into()),
    }

    let after = snapshot(&project_root)?;
    assert_eq!(before, after, "opening changed the project directory");
    assert_eq!(version_before, fs::read_to_string(&version_path)?);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn open_report_enumerates_assets_packages_settings() -> TestResult {
    let root = temp_dir("project-report")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    let store = TrustStore::new(root.join("trust.json"));

    let (project, report) = Project::open(&project_root, &store)?;

    let main_paths: Vec<String> = report
        .inventory
        .entries
        .iter()
        .filter(|entry| entry.kind == AssetKind::Main)
        .map(|entry| entry.path.to_string_lossy().to_string())
        .collect();
    assert_eq!(
        main_paths,
        vec![
            "Assets/Editor/EditorBootstrap.cs".to_string(),
            "Assets/Materials/Brick.mat".to_string(),
            "Assets/Scenes/Main.unity".to_string(),
            "Assets/Textures/Tile.png".to_string(),
        ]
    );
    let sub_paths: Vec<String> = report
        .inventory
        .entries
        .iter()
        .filter(|entry| entry.kind == AssetKind::Sub)
        .map(|entry| entry.path.to_string_lossy().to_string())
        .collect();
    assert_eq!(
        sub_paths,
        vec![
            "Assets/Textures/Tile.png".to_string(),
            "Assets/Textures/Tile.png".to_string()
        ]
    );

    assert_eq!(
        report.recognized_packages,
        vec![
            "com.example.git-package".to_string(),
            "com.unity.ide.rider".to_string(),
            "com.unity.modules.imgui".to_string(),
        ]
    );
    assert_eq!(
        report.recognized_settings,
        vec![
            "ProjectSettings.asset".to_string(),
            "TimeManager.asset".to_string(),
            "DynamicsManager.asset".to_string(),
            "InputManager.asset".to_string(),
        ]
    );
    assert_eq!(project.capabilities(), report.capabilities);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn untrusted_open_grants_no_capabilities() -> TestResult {
    let root = temp_dir("project-trust")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    let store = TrustStore::new(root.join("trust.json"));
    let project_path = project_root.to_string_lossy().to_string();

    let (project, report) = Project::open(&project_root, &store)?;
    assert!(!report.capabilities.may_run_code);
    assert!(!report.capabilities.may_fetch);
    assert!(report.capabilities.restricted);
    assert_eq!(project.capabilities(), report.capabilities);

    assert!(report
        .inventory
        .entries
        .iter()
        .any(|entry| entry.path.ends_with("EditorBootstrap.cs")));
    assert!(report
        .recognized_packages
        .contains(&"com.example.git-package".to_string()));

    let outcome = store.request_trust_from_automation(&project_path);
    assert_eq!(
        outcome,
        TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant)
    );
    let (_, still_untrusted) = Project::open(&project_root, &store)?;
    assert!(!still_untrusted.capabilities.may_run_code);
    assert!(!still_untrusted.capabilities.may_fetch);

    store.trust_project(&project_path, TrustOrigin::Person)?;
    let (_, trusted) = Project::open(&project_root, &store)?;
    assert!(trusted.capabilities.may_run_code);
    assert!(!trusted.capabilities.may_fetch);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn duplicate_guids_are_reported_not_resolved() -> TestResult {
    let root = temp_dir("project-duplicates")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    write_file(&project_root.join("Assets/Materials/Clone.mat"), MATERIAL)?;
    write_file(
        &project_root.join("Assets/Materials/Clone.mat.meta"),
        &meta(MATERIAL_GUID, "NativeFormatImporter:"),
    )?;
    let store = TrustStore::new(root.join("trust.json"));

    let (_, report) = Project::open(&project_root, &store)?;

    let duplicate = match report.inventory.duplicate_guids.first() {
        Some(record) => record,
        None => return Err("the duplicated GUID was not reported".into()),
    };
    assert_eq!(duplicate.guid, MATERIAL_GUID);
    assert_eq!(
        duplicate.paths,
        vec![
            PathBuf::from("Assets/Materials/Brick.mat"),
            PathBuf::from("Assets/Materials/Clone.mat"),
        ]
    );
    assert!(!report
        .inventory
        .entries
        .iter()
        .any(|entry| entry.guid == MATERIAL_GUID));
    match report.inventory.lookup(MATERIAL_GUID) {
        GuidLookup::Duplicate(record) => assert_eq!(record.guid, MATERIAL_GUID),
        other => return Err(format!("expected the duplicate record, got {other:?}").into()),
    }

    match report.inventory.lookup(TEXTURE_GUID) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.path, PathBuf::from("Assets/Textures/Tile.png"));
        }
        other => return Err(format!("expected the texture asset, got {other:?}").into()),
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

fn set_yaml_scalar(node: &mut Node, keys: &[&str], value: &str) -> bool {
    let (key, rest) = match keys.split_first() {
        Some(parts) => parts,
        None => return false,
    };
    match node {
        Node::Mapping(entries) => {
            for (entry_key, entry_value) in entries.iter_mut() {
                if entry_key != key {
                    continue;
                }
                if rest.is_empty() {
                    *entry_value = Node::Scalar(value.to_string());
                    return true;
                }
                return set_yaml_scalar(entry_value, rest, value);
            }
            false
        }
        _ => false,
    }
}

fn remove_yaml_key(node: &mut Node, keys: &[&str]) -> bool {
    let (key, rest) = match keys.split_first() {
        Some(parts) => parts,
        None => return false,
    };
    match node {
        Node::Mapping(entries) => {
            if rest.is_empty() {
                let len_before = entries.len();
                entries.retain(|(k, _)| k != key);
                return entries.len() != len_before;
            }
            for (entry_key, entry_value) in entries.iter_mut() {
                if entry_key == key {
                    return remove_yaml_key(entry_value, rest);
                }
            }
            false
        }
        _ => false,
    }
}

#[test]
fn project_round_trip_without_library_preserves_unedited_files_and_writes_no_generated_folders(
) -> TestResult {
    let root = temp_dir("project-no-library-round-trip")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;
    let store = TrustStore::new(root.join("trust.json"));

    let generated_folders = ["Library", "Temp", "Logs", "obj", "UserSettings"];
    for folder in generated_folders {
        assert!(!project_root.join(folder).exists());
    }

    let (_, initial_report) = Project::open(&project_root, &store)?;
    let mat_path = project_root.join("Assets/Materials/Brick.mat");
    let scene_path = project_root.join("Assets/Scenes/Main.unity");
    let tex_path = project_root.join("Assets/Textures/Tile.png");
    let tex_meta_path = project_root.join("Assets/Textures/Tile.png.meta");
    let version_path = project_root.join("ProjectSettings/ProjectVersion.txt");

    let mat_original = fs::read(&mat_path)?;
    let scene_original = fs::read(&scene_path)?;
    let tex_original = fs::read(&tex_path)?;
    let tex_meta_original = fs::read(&tex_meta_path)?;
    let version_original = fs::read(&version_path)?;

    let mut mat_stream = unity_yaml::parse_file(&mat_path)?;
    scene_save::save_stream(&mat_path, &mat_stream)?;
    assert_eq!(fs::read(&mat_path)?, mat_original);

    assert!(set_yaml_scalar(
        &mut mat_stream.documents[0].root,
        &["Material", "m_Name"],
        "BrickPolished"
    ));
    scene_save::save_stream(&mat_path, &mat_stream)?;

    let (_, reopened_report) = Project::open(&project_root, &store)?;
    for folder in generated_folders {
        assert!(
            !project_root.join(folder).exists(),
            "generated folder {folder} was created"
        );
    }

    assert_eq!(fs::read(&scene_path)?, scene_original);
    assert_eq!(fs::read(&tex_path)?, tex_original);
    assert_eq!(fs::read(&tex_meta_path)?, tex_meta_original);
    assert_eq!(fs::read(&version_path)?, version_original);
    assert_eq!(reopened_report.inventory, initial_report.inventory);

    let sub_entries: Vec<_> = reopened_report
        .inventory
        .entries
        .iter()
        .filter(|entry| entry.kind == AssetKind::Sub && entry.guid == TEXTURE_GUID)
        .collect();
    assert_eq!(sub_entries.len(), 2);

    let reloaded_mat = unity_yaml::parse_file(&mat_path)?;
    assert_eq!(
        reloaded_mat.documents[0]
            .root
            .get("Material")
            .and_then(|n| n.get("m_Name"))
            .and_then(Node::as_str),
        Some("BrickPolished")
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn project_2021_3_edit_on_save_migrates_only_edited_asset_and_preserves_unedited_files(
) -> TestResult {
    let root = temp_dir("project-2021-3-edit-on-save")?;
    let project_root = root.join("project");
    build_project(&project_root, "2021.3.16f1")?;
    let second_mat_path = project_root.join("Assets/Materials/Mortar.mat");
    let second_mat_guid = "55555555555555555555555555555555";
    let second_mat_content = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: Mortar\n";
    write_file(&second_mat_path, second_mat_content)?;
    write_file(
        &project_root.join("Assets/Materials/Mortar.mat.meta"),
        &meta(second_mat_guid, "NativeFormatImporter:"),
    )?;

    let store = TrustStore::new(root.join("trust.json"));
    let snapshot_before_open = snapshot(&project_root)?;
    let (_, report_before) = Project::open(&project_root, &store)?;
    assert_eq!(snapshot(&project_root)?, snapshot_before_open);

    match &report_before.migration {
        MigrationStatus::RequiresUpgrade(preview) => {
            assert!(preview
                .assets
                .contains(&String::from("Assets/Materials/Brick.mat")));
            assert!(preview
                .assets
                .contains(&String::from("Assets/Materials/Mortar.mat")));
        }
        other => return Err(format!("expected RequiresUpgrade, got {other:?}").into()),
    }

    let brick_path = project_root.join("Assets/Materials/Brick.mat");
    let scene_path = project_root.join("Assets/Scenes/Main.unity");
    let version_path = project_root.join("ProjectSettings/ProjectVersion.txt");
    let mortar_bytes_before = fs::read(&second_mat_path)?;
    let scene_bytes_before = fs::read(&scene_path)?;
    let version_bytes_before = fs::read(&version_path)?;

    let mut brick_stream = unity_yaml::parse_file(&brick_path)?;
    assert!(set_yaml_scalar(
        &mut brick_stream.documents[0].root,
        &["Material", "m_Name"],
        "Brick2022"
    ));
    assert!(remove_yaml_key(
        &mut brick_stream.documents[0].root,
        &["Material", "serializedVersion"]
    ));
    scene_save::save_stream(&brick_path, &brick_stream)?;

    assert_eq!(fs::read(&second_mat_path)?, mortar_bytes_before);
    assert_eq!(fs::read(&scene_path)?, scene_bytes_before);
    assert_eq!(fs::read(&version_path)?, version_bytes_before);

    let (_, report_after) = Project::open(&project_root, &store)?;
    assert_eq!(report_after.inventory, report_before.inventory);
    match &report_after.migration {
        MigrationStatus::RequiresUpgrade(preview) => {
            assert!(!preview
                .assets
                .contains(&String::from("Assets/Materials/Brick.mat")));
            assert!(preview
                .assets
                .contains(&String::from("Assets/Materials/Mortar.mat")));
        }
        other => return Err(format!("expected RequiresUpgrade, got {other:?}").into()),
    }

    let references = guid_references(&project_root);
    assert!(references.iter().any(|(guid, _)| guid == MATERIAL_GUID));
    assert_eq!(
        target_of(&report_after.inventory, MATERIAL_GUID),
        Some(String::from("Assets/Materials/Brick.mat"))
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn meta_unrecognized_fields_builtin_resource_and_missing_sub_asset_reference_round_trip(
) -> TestResult {
    let root = temp_dir("project-meta-builtin-subasset")?;
    let meta_path = root.join("CustomAsset.mat.meta");
    let meta_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1001 &1\nfileFormatVersion: 2\nguid: {MATERIAL_GUID}\nCustomExperimentalImporter:\n  serializedVersion: 9\n  customFlag: enabled\n  nestedConfig:\n    alpha: 42\n    beta: preserved\n  userData: custom_user_payload\n  assetBundleName: bundle_main\n"
    );
    write_file(&meta_path, &meta_yaml)?;

    let parsed_meta_stream = unity_yaml::parse_file(&meta_path)?;
    assert_eq!(scene_save::emit_stream(&parsed_meta_stream), meta_yaml);
    scene_save::save_stream(&meta_path, &parsed_meta_stream)?;
    assert_eq!(fs::read_to_string(&meta_path)?, meta_yaml);
    assert_eq!(meta::parse_meta_file(&meta_path)?, MATERIAL_GUID);

    let mut edited_meta_stream = parsed_meta_stream;
    assert!(set_yaml_scalar(
        &mut edited_meta_stream.documents[0].root,
        &["CustomExperimentalImporter", "userData"],
        "updated_user_payload"
    ));
    scene_save::save_stream(&meta_path, &edited_meta_stream)?;
    assert_eq!(meta::parse_meta_file(&meta_path)?, MATERIAL_GUID);

    let reloaded_meta = unity_yaml::parse_file(&meta_path)?;
    let importer_node = reloaded_meta.documents[0]
        .root
        .get("CustomExperimentalImporter")
        .ok_or("missing CustomExperimentalImporter")?;
    assert_eq!(
        importer_node.get("customFlag").and_then(Node::as_str),
        Some("enabled")
    );
    assert_eq!(
        importer_node
            .get("nestedConfig")
            .and_then(|n| n.get("beta"))
            .and_then(Node::as_str),
        Some("preserved")
    );
    assert_eq!(
        importer_node.get("userData").and_then(Node::as_str),
        Some("updated_user_payload")
    );

    let scene_path = root.join("BuiltinAndMissingSubAsset.unity");
    let missing_sub_asset_file_id: i64 = 21300999;
    let scene_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: BuiltinCubeHolder\n--- !u!33 &330\nMeshFilter:\n  m_GameObject: {{fileID: 100}}\n  m_Mesh: {{fileID: 10202, guid: {BUILTIN_EXTRA_GUID}, type: 0}}\n--- !u!212 &2120\nSpriteRenderer:\n  m_GameObject: {{fileID: 100}}\n  m_Sprite: {{fileID: {missing_sub_asset_file_id}, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    write_file(&scene_path, &scene_yaml)?;

    let mut scene_stream = unity_yaml::parse_file(&scene_path)?;
    assert_eq!(scene_save::emit_stream(&scene_stream), scene_yaml);
    assert!(set_yaml_scalar(
        &mut scene_stream.documents[0].root,
        &["GameObject", "m_Name"],
        "BuiltinCubeHolderEdited"
    ));
    scene_save::save_stream(&scene_path, &scene_stream)?;

    let reloaded_scene = unity_yaml::parse_file(&scene_path)?;
    let mesh_ref = reloaded_scene.documents[1]
        .root
        .get("MeshFilter")
        .and_then(|n| n.get("m_Mesh"))
        .ok_or("missing m_Mesh")?;
    let mesh_file_id = mesh_ref
        .get("fileID")
        .and_then(Node::as_i64)
        .ok_or("missing mesh fileID")?;
    let mesh_guid = mesh_ref
        .get("guid")
        .and_then(Node::as_str)
        .ok_or("missing mesh guid")?;
    assert_eq!(mesh_file_id, 10202);
    assert_eq!(mesh_guid, BUILTIN_EXTRA_GUID);
    assert_eq!(
        builtin_primitive_by_identity(mesh_file_id, mesh_guid),
        Some(BuiltinPrimitive::Cube)
    );

    let sprite_ref = reloaded_scene.documents[2]
        .root
        .get("SpriteRenderer")
        .and_then(|n| n.get("m_Sprite"))
        .ok_or("missing m_Sprite")?;
    assert_eq!(
        sprite_ref.get("fileID").and_then(Node::as_i64),
        Some(missing_sub_asset_file_id)
    );
    assert_eq!(
        sprite_ref.get("guid").and_then(Node::as_str),
        Some(TEXTURE_GUID)
    );
    assert_eq!(sprite_ref.get("type").and_then(Node::as_i64), Some(3));

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj001_ser005_unity_2020_3_and_2021_3_migration_preview_first_save_notice_build_and_gitignore_footprint(
) -> TestResult {
    let root = temp_dir("project-2020-2021-footprint")?;
    let project_2020 = root.join("project_2020");
    let project_2021 = root.join("project_2021");
    build_project(&project_2020, "2020.3.48f1")?;
    build_project(&project_2021, "2021.3.16f1")?;

    let per_user_dir = root.join("per_user_state");
    fs::create_dir_all(&per_user_dir)?;
    let store = TrustStore::new(per_user_dir.join("trust.json"));

    for (proj_dir, expected_ver) in [
        (&project_2020, "2020.3.48f1"),
        (&project_2021, "2021.3.16f1"),
    ] {
        let snap_before = snapshot(proj_dir)?;
        let (_, report) = Project::open(proj_dir, &store)?;
        assert_eq!(report.declared_version.as_deref(), Some(expected_ver));
        assert_eq!(snapshot(proj_dir)?, snap_before);
        assert!(!proj_dir.join("ProjectSettings/UnitasVersion.txt").exists());

        match &report.migration {
            MigrationStatus::RequiresUpgrade(preview) => {
                assert!(preview
                    .assets
                    .contains(&String::from("Assets/Materials/Brick.mat")));
                assert!(preview
                    .settings
                    .contains(&String::from("TimeManager.asset")));
                assert!(preview
                    .packages
                    .contains(&String::from("com.unity.ide.rider")));
            }
            other => return Err(format!("expected RequiresUpgrade, got {other:?}").into()),
        }
    }

    let first_save_notice = match &Project::open(&project_2021, &store)?.1.migration {
        MigrationStatus::RequiresUpgrade(preview) => format!(
            "Saving upgraded assets {:?} in Unity {} form; these assets will require Unity {} LTS to open in Unity.",
            preview.assets, BASELINE_UNITY_VERSION, BASELINE_UNITY_VERSION
        ),
        _ => String::new(),
    };
    assert!(first_save_notice.contains("Assets/Materials/Brick.mat"));
    assert!(first_save_notice.contains("2022.3"));

    store.trust_project(&project_2021.to_string_lossy(), TrustOrigin::Person)?;
    let cache_file = per_user_dir.join("import_cache.bin");
    write_file(&cache_file, "cached_import_payload")?;

    let (_, report_with_cache) = Project::open(&project_2021, &store)?;
    fs::remove_file(&cache_file)?;
    let (_, report_after_cache_delete) = Project::open(&project_2021, &store)?;
    assert_eq!(
        report_with_cache.inventory,
        report_after_cache_delete.inventory
    );

    let snap_before_build = snapshot(&project_2021)?;
    let initial_paths: Vec<PathBuf> = snap_before_build.iter().map(|(p, _)| p.clone()).collect();

    let sln_path = project_2021.join("project_2021.sln");
    let csproj_path = project_2021.join("Assembly-CSharp.csproj");
    let build_out_dir = root.join("Builds/Standalone");
    write_file(&sln_path, "Microsoft Visual Studio Solution File")?;
    write_file(&csproj_path, "<Project Sdk=\"Microsoft.NET.Sdk\" />")?;

    let assembled_main =
        unitas_asset::scene::assemble_scene_file(&project_2021.join("Assets/Scenes/Main.unity"))?;
    assert_eq!(assembled_main.len(), 1);
    let staged_scene_bytes = fs::read(project_2021.join("Assets/Scenes/Main.unity"))?;
    fs::create_dir_all(build_out_dir.join("scenes"))?;
    fs::write(
        build_out_dir.join("scenes/000_Main.unity"),
        &staged_scene_bytes,
    )?;
    write_file(
        &build_out_dir.join("bundle.manifest"),
        &format!(
            "format_version=1\nproduct_name=Sample\nstartup_scene_staged_path=scenes/000_Main.unity\nentry_count=1\nentry.0.root_object_count={}\n",
            assembled_main.roots().len()
        ),
    )?;
    write_file(&build_out_dir.join("SamplePlayer.app"), "player_binary")?;

    for generated in ["Library", "Temp", "Logs", "obj", "UserSettings"] {
        assert!(!project_2021.join(generated).exists());
    }
    assert!(
        !project_2021
            .join("ProjectSettings/UnitasVersion.txt")
            .exists(),
        "SER-005 outcome 17: opening and building alone must not write UnitasVersion.txt"
    );
    assert!(sln_path.is_file());
    assert!(csproj_path.is_file());
    assert!(build_out_dir.join("SamplePlayer.app").is_file());
    assert!(build_out_dir.join("bundle.manifest").is_file());
    assert!(build_out_dir.join("scenes/000_Main.unity").is_file());

    let snap_after_build = snapshot(&project_2021)?;
    let added_at_project_root: Vec<PathBuf> = snap_after_build
        .iter()
        .map(|(p, _)| p.clone())
        .filter(|p| !initial_paths.contains(p))
        .collect();
    assert_eq!(
        added_at_project_root,
        vec![
            PathBuf::from("Assembly-CSharp.csproj"),
            PathBuf::from("project_2021.sln"),
        ]
    );

    let review_policy_rel = PathBuf::from("ProjectSettings/UnitasReviewPolicy.json");
    let unitas_version_rel = PathBuf::from("ProjectSettings/UnitasVersion.txt");
    write_file(
        &project_2021.join(&review_policy_rel),
        "{\"requireApproval\":true}\n",
    )?;
    write_file(
        &project_2021.join(&unitas_version_rel),
        "unitas_version: 0.1.0\n",
    )?;

    let unity_gitignore_prefixes = [
        "Library/",
        "Temp/",
        "Obj/",
        "obj/",
        "Build/",
        "Builds/",
        "Logs/",
        "UserSettings/",
        ".unitas/",
    ];
    let is_git_tracked = |rel: &Path| -> bool {
        let s = rel.to_string_lossy();
        if s.ends_with(".csproj") || s.ends_with(".sln") {
            return false;
        }
        !unity_gitignore_prefixes
            .iter()
            .any(|prefix| s.starts_with(prefix))
    };

    let all_project_files = snapshot(&project_2021)?;
    let tracked_paths: Vec<PathBuf> = all_project_files
        .into_iter()
        .map(|(rel, _)| rel)
        .filter(|rel| is_git_tracked(rel))
        .collect();

    assert!(tracked_paths.contains(&review_policy_rel));
    assert!(tracked_paths.contains(&unitas_version_rel));
    assert!(!tracked_paths
        .iter()
        .any(|p| p.starts_with("per_user_state")));
    assert!(!tracked_paths.contains(&PathBuf::from("project_2021.sln")));
    assert!(!tracked_paths.contains(&PathBuf::from("Assembly-CSharp.csproj")));

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj003_ser004_missing_asset_and_sub_asset_keep_original_identity_with_similarly_named_siblings_and_duplicate_local_file_ids_are_rejected(
) -> TestResult {
    let root = temp_dir("project-missing-identity-dup-local-id")?;
    let project_root = root.join("project");
    build_project(&project_root, &format!("{BASELINE_UNITY_VERSION}.16f1"))?;

    let sibling_mat_guid = "66666666666666666666666666666666";
    write_file(
        &project_root.join("Assets/Materials/Brick_Copy.mat"),
        MATERIAL,
    )?;
    write_file(
        &project_root.join("Assets/Materials/Brick_Copy.mat.meta"),
        &meta(sibling_mat_guid, "NativeFormatImporter:"),
    )?;

    let missing_mat_guid = "dead0000dead0000dead0000dead0000";
    let missing_mat_file_id: i64 = 2100000;
    let missing_sprite_file_id: i64 = 21300888;
    let scene_path = project_root.join("Assets/Scenes/Main.unity");
    let scene_with_missing = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n--- !u!1 &1000\nGameObject:\n  m_Name: BrickHolder\n  m_Material: {{fileID: {missing_mat_file_id}, guid: {missing_mat_guid}, type: 2}}\n  m_Sprite: {{fileID: {missing_sprite_file_id}, guid: {TEXTURE_GUID}, type: 3}}\n"
    );
    write_file(&scene_path, &scene_with_missing)?;

    let store = TrustStore::new(root.join("trust.json"));
    let (mut project, report) = Project::open(&project_root, &store)?;

    assert_eq!(
        report.inventory.lookup(missing_mat_guid),
        GuidLookup::Missing
    );
    match report.inventory.lookup(sibling_mat_guid) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.path, PathBuf::from("Assets/Materials/Brick_Copy.mat"));
        }
        other => return Err(format!("expected sibling asset, got {other:?}").into()),
    }

    let mut scene_stream = unity_yaml::parse_file(&scene_path)?;
    assert!(set_yaml_scalar(
        &mut scene_stream.documents[1].root,
        &["GameObject", "m_Name"],
        "BrickHolderRenamed"
    ));
    scene_save::save_stream(&scene_path, &scene_stream)?;

    let reloaded_scene = unity_yaml::parse_file(&scene_path)?;
    let go_node = reloaded_scene.documents[1]
        .root
        .get("GameObject")
        .ok_or("missing GameObject")?;
    let mat_ref = go_node.get("m_Material").ok_or("missing m_Material")?;
    assert_eq!(
        mat_ref.get("guid").and_then(Node::as_str),
        Some(missing_mat_guid)
    );
    assert_eq!(
        mat_ref.get("fileID").and_then(Node::as_i64),
        Some(missing_mat_file_id)
    );
    assert_ne!(
        mat_ref.get("guid").and_then(Node::as_str),
        Some(sibling_mat_guid)
    );

    let sprite_ref = go_node.get("m_Sprite").ok_or("missing m_Sprite")?;
    assert_eq!(
        sprite_ref.get("guid").and_then(Node::as_str),
        Some(TEXTURE_GUID)
    );
    assert_eq!(
        sprite_ref.get("fileID").and_then(Node::as_i64),
        Some(missing_sprite_file_id)
    );

    project.move_asset(
        Path::new("Assets/Materials/Brick.mat"),
        Path::new("Assets/Materials/MovedBrick.mat"),
    )?;
    match project.inventory().lookup(MATERIAL_GUID) {
        GuidLookup::Asset(entry) => {
            assert_eq!(entry.path, PathBuf::from("Assets/Materials/MovedBrick.mat"));
        }
        other => return Err(format!("expected moved Brick.mat, got {other:?}").into()),
    }

    let dup_local_id_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &777\nGameObject:\n  m_Name: FirstWith777\n--- !u!1 &777\nGameObject:\n  m_Name: SecondWith777\n";
    let merge_err = unitas_asset::merge::merge_bytes(
        dup_local_id_yaml.as_bytes(),
        dup_local_id_yaml.as_bytes(),
        dup_local_id_yaml.as_bytes(),
    )
    .err()
    .ok_or("expected duplicate local fileID error")?;
    assert!(merge_err
        .to_string()
        .contains("file id 777 is used by more than one object"));

    let dup_transform_owner_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: DuplicateOwnerNode\n--- !u!4 &201\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_Children: []\n--- !u!4 &202\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_Children: []\n";
    let dup_stream = unity_yaml::parse(dup_transform_owner_yaml)?;
    let scene_err = unitas_asset::scene::assemble_scene(&dup_stream)
        .err()
        .ok_or("expected DuplicateTransformOwner error")?;
    assert!(matches!(
        scene_err,
        unitas_asset::SceneError::DuplicateTransformOwner {
            game_object_file_id: 100
        }
    ));

    let obj_v1 = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 2 0 1\nv 0 2 1\ng HeroMesh\nusemtl ArmorMat\nf 1 2 3\ng ShieldMesh\nusemtl MetalMat\nf 4 5 6\n";
    let obj_v2_reimported = b"v 0 0 0\nv 3 0 0\nv 0 3 0\nv 0 0 1\nv 2 0 1\nv 0 2 1\ng HeroMesh\nusemtl ArmorMat\nf 1 2 3\ng ShieldMesh\nusemtl MetalMat\nf 4 5 6\n";
    let combined_v1 = unitas_asset::model::import_model(obj_v1, "Hero.obj")?;
    let combined_v2 = unitas_asset::model::import_model(obj_v2_reimported, "Hero.obj")?;
    let sub_ids_v1: Vec<String> = combined_v1
        .sub_meshes
        .iter()
        .map(|s| s.sub_asset_id.clone())
        .collect();
    let sub_ids_v2: Vec<String> = combined_v2
        .sub_meshes
        .iter()
        .map(|s| s.sub_asset_id.clone())
        .collect();
    assert_eq!(
        sub_ids_v1,
        vec![
            String::from("HeroMesh/ArmorMat"),
            String::from("ShieldMesh/MetalMat"),
        ]
    );
    assert_eq!(sub_ids_v1, sub_ids_v2);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj001_outcome1_outcome2_outcome12_open_2020_3_2021_3_2022_3_with_or_without_library_and_upgrade_action_round_trip(
) -> TestResult {
    let root = temp_dir("prj001-outcomes-1-2-12")?;
    let store = TrustStore::new(root.join("trust.json"));

    let mut reference_inventory: Option<AssetInventory> = None;
    let mut reference_targets: Option<Vec<(String, Option<String>)>> = None;

    for (index, version) in ["2020.3.48f1", "2021.3.16f1", "2022.3.16f1"]
        .into_iter()
        .enumerate()
    {
        let project_root = root.join(format!("project_{index}"));
        build_project(&project_root, version)?;

        let (_, report_no_lib_first) = Project::open(&project_root, &store)?;
        let (_, report_no_lib_second) = Project::open(&project_root, &store)?;
        assert_eq!(
            report_no_lib_first.inventory, report_no_lib_second.inventory,
            "PRJ-001 Outcome 1: opening {version} twice without Library/ must yield identical asset identities"
        );

        write_file(&project_root.join("Library/ArtifactDB"), "unity-cache")?;
        write_file(
            &project_root.join("Library/Cached.asset.meta"),
            &meta("ffffffffffffffffffffffffffffffff", "NativeFormatImporter:"),
        )?;
        let (_, report_with_lib) = Project::open(&project_root, &store)?;
        assert_eq!(
            report_no_lib_first.inventory, report_with_lib.inventory,
            "PRJ-001 Outcome 1: opening {version} with or without Library/ must yield identical asset identities"
        );

        let mut targets: Vec<(String, Option<String>)> = guid_references(&project_root)
            .into_iter()
            .map(|(guid, _)| {
                let resolved = target_of(&report_with_lib.inventory, &guid);
                (guid, resolved)
            })
            .collect();
        targets.sort();

        if let Some(expected_inv) = &reference_inventory {
            assert_eq!(
                &report_with_lib.inventory, expected_inv,
                "PRJ-001 Outcome 1: 2020.3, 2021.3, and 2022.3 in-place open must yield identical asset identities"
            );
        } else {
            reference_inventory = Some(report_with_lib.inventory.clone());
        }
        if let Some(expected_targets) = &reference_targets {
            assert_eq!(
                &targets, expected_targets,
                "PRJ-001 Outcome 1: 2020.3, 2021.3, and 2022.3 in-place open must yield identical reference targets"
            );
        } else {
            reference_targets = Some(targets);
        }

        let compatibility =
            unitas_asset::project::build_compatibility_report(&project_root, &report_with_lib);
        assert!(
            compatibility.accounts_for_all(&report_with_lib),
            "PRJ-001 Outcome 12: compatibility report must account for every recognized asset, package, and setting"
        );
    }

    let mig_root = root.join("project_upgrade_2021");
    build_project(&mig_root, "2021.3.16f1")?;
    let snap_before_open = snapshot(&mig_root)?;
    let (mut mig_project, mig_report) = Project::open(&mig_root, &store)?;
    assert_eq!(
        mig_report.effective_behavior_version, BASELINE_UNITY_VERSION,
        "PRJ-001 Outcome 2: 2021.3 project must open with 2022.3 LTS effective behavior"
    );
    assert_eq!(
        snapshot(&mig_root)?,
        snap_before_open,
        "PRJ-001 Outcome 2: nothing rewritten on open"
    );

    let preview = mig_project.preview_upgrade()?;
    assert!(
        preview
            .assets
            .contains(&String::from("Assets/Materials/Brick.mat")),
        "PRJ-001 Outcome 2: pre-write upgrade preview must list affected assets"
    );
    assert!(
        preview
            .settings
            .contains(&String::from("TimeManager.asset")),
        "PRJ-001 Outcome 2: pre-write upgrade preview must list affected settings"
    );
    assert!(
        preview
            .packages
            .contains(&String::from("com.unity.ide.rider")),
        "PRJ-001 Outcome 2: pre-write upgrade preview must list affected packages"
    );

    let committed = mig_project.commit_upgrade("2022.3.16f1")?;
    assert_eq!(
        committed, preview,
        "PRJ-001 Outcome 2: committed upgrade must match pre-write preview"
    );
    let (_, upgraded_report) = Project::open(&mig_root, &store)?;
    assert_eq!(
        upgraded_report.declared_version.as_deref(),
        Some("2022.3.16f1"),
        "PRJ-001 Outcome 2: explicit upgrade action records 2022.3 LTS in ProjectVersion.txt"
    );
    assert_eq!(
        upgraded_report.migration,
        MigrationStatus::Current,
        "PRJ-001 Outcome 2: upgraded project reopens as Current"
    );
    assert_eq!(
        upgraded_report.inventory, mig_report.inventory,
        "PRJ-001 Exact round-trip: asset identities survive upgrade load, edit, save"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
