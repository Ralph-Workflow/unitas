
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project::{
    AssetInventory, AssetKind, GuidLookup, MigrationStatus, Project, ASSESSED_ON_EDIT,
    BASELINE_UNITY_VERSION,
};
use unitas_asset::trust::{TrustOrigin, TrustOutcome, TrustRefusal, TrustStore};

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
    write_file(
        &root.join("Assets/Materials/Brick.mat"),
        MATERIAL,
    )?;
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
    write_file(&root.join("Assets/Editor/EditorBootstrap.cs"), EDITOR_SCRIPT)?;
    write_file(
        &root.join("Assets/Editor/EditorBootstrap.cs.meta"),
        &meta(SCRIPT_GUID, "MonoImporter:"),
    )?;

    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        &format!("m_EditorVersion: {version}\nm_EditorVersionWithRevision: {version} (0cf5a3b0e6a3)\n"),
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
            let value = value
                .split([',', '}'])
                .next()
                .unwrap_or(value)
                .trim();
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
    assert_eq!(first_report.declared_version, second_report.declared_version);
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
    write_file(
        &project_root.join("Library/Cached.asset"),
        "cached",
    )?;
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
        vec!["Assets/Textures/Tile.png".to_string(), "Assets/Textures/Tile.png".to_string()]
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
            assert_eq!(entry.path, PathBuf::from("Assets/Textures/Tile.png"))
        }
        other => return Err(format!("expected the texture asset, got {other:?}").into()),
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}
