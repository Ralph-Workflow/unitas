use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use unitas_editor_abi::{DispatchTable, builtin_dispatch_table};

type TestResult = Result<(), Box<dyn Error>>;

const MATERIAL_GUID: &str = "11111111111111111111111111111111";
const TEXT_GUID: &str = "22222222222222222222222222222222";
const SCENE_GUID: &str = "33333333333333333333333333333333";
const SCRIPT_GUID: &str = "44444444444444444444444444444444";

const MATERIAL: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: Brick\n";
const SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n--- !u!1 &1000\nGameObject:\n  m_Name: Cube\n  m_Material: {fileID: 2100000, guid: 11111111111111111111111111111111, type: 2}\n";
const TIME_MANAGER: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_FixedTimestep: 0.02\n  m_MaximumAllowedTimestep: 0.33333334\n  m_TimeScale: 1\n  m_MaximumParticleTimestep: 0.03\n";
const TAG_MANAGER: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!78 &1\nTagManager:\n  serializedVersion: 2\n  tags:\n  - Player\n  layers:\n  - Default\n  m_SortingLayers:\n  - name: UI\n    uniqueID: 1\n";

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas-editor-abi-{name}-{}-{nanos}",
        std::process::id()
    ));
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

fn build_project(root: &Path) -> std::io::Result<()> {
    write_file(&root.join("Assets/Materials/Brick.mat"), MATERIAL)?;
    write_file(
        &root.join("Assets/Materials/Brick.mat.meta"),
        &meta_with_labels(MATERIAL_GUID, "NativeFormatImporter:", &["masonry"]),
    )?;
    write_file(&root.join("Assets/Notes.txt"), "release notes\n")?;
    write_file(
        &root.join("Assets/Notes.txt.meta"),
        &meta_with_labels(TEXT_GUID, "TextScriptImporter:", &[]),
    )?;
    write_file(&root.join("Assets/Scenes/Main.unity"), SCENE)?;
    write_file(
        &root.join("Assets/Scenes/Main.unity.meta"),
        &meta_with_labels(SCENE_GUID, "NativeFormatImporter:", &[]),
    )?;
    write_file(
        &root.join("Assets/Scripts/Player.cs"),
        "public class Player {}\n",
    )?;
    write_file(
        &root.join("Assets/Scripts/Player.cs.meta"),
        &meta_with_labels(SCRIPT_GUID, "MonoImporter:", &[]),
    )?;
    write_file(
        &root.join("ProjectSettings/TimeManager.asset"),
        TIME_MANAGER,
    )?;
    write_file(&root.join("ProjectSettings/TagManager.asset"), TAG_MANAGER)?;
    write_file(
        &root.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.20f1\n",
    )
}

fn project_args(root: &Path) -> Value {
    json!({
        "projectPath": root.to_string_lossy(),
        "trustStorePath": root.join("trust.json").to_string_lossy(),
    })
}

fn table() -> Result<DispatchTable, Box<dyn Error>> {
    Ok(builtin_dispatch_table()?)
}

fn assets_of(result: &Value) -> Vec<String> {
    result["assets"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["path"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn assets_search_lists_project_assets_and_filters_by_label() -> TestResult {
    let root = temp_dir("search")?;
    build_project(&root)?;
    let table = table()?;

    let all = table.invoke("assets.search", &project_args(&root));
    assert_eq!(all["error"], json!(null));
    assert_eq!(all["count"], json!(4));

    let mut paths = assets_of(&all);
    paths.sort();
    assert_eq!(
        paths,
        [
            "Assets/Materials/Brick.mat",
            "Assets/Notes.txt",
            "Assets/Scenes/Main.unity",
            "Assets/Scripts/Player.cs",
        ]
    );

    let filtered = table.invoke(
        "assets.search",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "label": "masonry",
        }),
    );
    assert_eq!(assets_of(&filtered), ["Assets/Materials/Brick.mat"]);

    let named = table.invoke(
        "assets.search",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "nameContains": "Notes",
        }),
    );
    assert_eq!(assets_of(&named), ["Assets/Notes.txt"]);
    Ok(())
}

#[test]
fn assets_preview_reports_class_size_and_state() -> TestResult {
    let root = temp_dir("preview")?;
    build_project(&root)?;
    let table = table()?;

    let preview = table.invoke(
        "assets.preview",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "path": "Assets/Materials/Brick.mat",
        }),
    );
    assert_eq!(preview["error"], json!(null));
    assert_eq!(preview["guid"], json!(MATERIAL_GUID));
    assert_eq!(preview["className"], json!("Material"));
    assert!(preview["byteSize"].as_u64().unwrap_or_default() > 0);
    assert_eq!(preview["state"], json!("ready"));

    let missing = table.invoke(
        "assets.preview",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "path": "Assets/Materials/None.mat",
        }),
    );
    assert_eq!(missing["error"]["kind"], json!("tool-failed"));
    Ok(())
}

#[test]
fn assets_move_delete_restore_and_duplicate_keep_the_project_consistent() -> TestResult {
    let root = temp_dir("mutate")?;
    build_project(&root)?;
    let table = table()?;

    assert_move_and_duplicate(&table, &root);
    assert_delete_and_restore(&table, &root);
    Ok(())
}

fn assert_move_and_duplicate(table: &DispatchTable, root: &Path) {
    let moved = table.invoke(
        "assets.move",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "from": "Assets/Materials/Brick.mat",
            "to": "Assets/Materials/Stone.mat",
        }),
    );
    assert_eq!(moved["error"], json!(null));
    assert_eq!(moved["moved"], json!(true));
    assert!(root.join("Assets/Materials/Stone.mat").is_file());
    assert!(!root.join("Assets/Materials/Brick.mat").exists());

    let duplicated = table.invoke(
        "assets.duplicate",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "from": "Assets/Materials/Stone.mat",
            "to": "Assets/Materials/Copy.mat",
        }),
    );
    assert_eq!(duplicated["error"], json!(null));
    assert_ne!(duplicated["guid"], json!(MATERIAL_GUID));
    assert!(root.join("Assets/Materials/Copy.mat").is_file());
}

fn assert_delete_and_restore(table: &DispatchTable, root: &Path) {
    let deleted = table.invoke(
        "assets.delete",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "path": "Assets/Materials/Stone.mat",
        }),
    );
    assert_eq!(deleted["error"], json!(null));
    assert!(!root.join("Assets/Materials/Stone.mat").exists());

    let restored = table.invoke(
        "assets.restore",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "path": "Assets/Materials/Stone.mat",
        }),
    );
    assert_eq!(restored["error"], json!(null));
    assert!(root.join("Assets/Materials/Stone.mat").is_file());
}

#[test]
fn assets_favorites_and_recents_track_the_open_project() -> TestResult {
    let root = temp_dir("favorites")?;
    build_project(&root)?;
    let table = table()?;

    assert_favorites_add_and_list(&table, &root)?;
    assert_recents_and_favorite_removal(&table, &root)?;
    Ok(())
}

fn assert_favorites_add_and_list(table: &DispatchTable, root: &Path) -> TestResult {
    let added = table.invoke(
        "assets.favorites",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "action": "add",
            "guid": MATERIAL_GUID,
        }),
    );
    assert_eq!(added["added"], json!(true));
    assert_eq!(added["favorites"], json!([MATERIAL_GUID]));

    let listed = table.invoke(
        "assets.favorites",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
        }),
    );
    assert_eq!(listed["favorites"], json!([MATERIAL_GUID]));
    Ok(())
}

fn assert_recents_and_favorite_removal(table: &DispatchTable, root: &Path) -> TestResult {
    let preview = table.invoke(
        "assets.preview",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "path": "Assets/Notes.txt",
        }),
    );
    assert_eq!(preview["error"], json!(null));

    let recents = table.invoke(
        "assets.recents",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
        }),
    );
    assert_eq!(recents["recent"], json!([TEXT_GUID]));

    let removed = table.invoke(
        "assets.favorites",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "action": "remove",
            "guid": MATERIAL_GUID,
        }),
    );
    assert_eq!(removed["removed"], json!(true));
    assert_eq!(removed["favorites"], json!([]));
    Ok(())
}

#[test]
fn assets_dependencies_report_direct_and_reverse_dependencies() -> TestResult {
    let root = temp_dir("dependencies")?;
    build_project(&root)?;
    let table = table()?;

    let scene = table.invoke(
        "assets.dependencies",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "guid": SCENE_GUID,
        }),
    );
    assert_eq!(scene["error"], json!(null));
    assert_eq!(scene["dependencies"], json!([MATERIAL_GUID]));

    let material = table.invoke(
        "assets.dependencies",
        &json!({
            "projectPath": root.to_string_lossy(),
            "trustStorePath": root.join("trust.json").to_string_lossy(),
            "guid": MATERIAL_GUID,
        }),
    );
    assert_eq!(material["reverseDependencies"], json!([SCENE_GUID]));
    Ok(())
}

#[test]
fn settings_inspect_reports_the_project_settings_families() -> TestResult {
    let root = temp_dir("settings")?;
    build_project(&root)?;
    let table = table()?;

    let settings = table.invoke("settings.inspect", &project_args(&root));
    assert_eq!(settings["error"], json!(null));
    assert_eq!(settings["fixedTimestep"], json!(0.02));
    assert_eq!(settings["sortingLayers"], json!(["UI"]));
    assert_eq!(settings["tags"], json!(["Player"]));
    Ok(())
}

#[test]
fn settings_update_writes_the_value_back_into_the_project_settings_file() -> TestResult {
    let root = temp_dir("settings_update")?;
    build_project(&root)?;
    let table = table()?;

    let updated = table.invoke(
        "settings.update",
        &json!({
            "projectPath": root.to_string_lossy(),
            "file": "TimeManager.asset",
            "key": "TimeManager.m_FixedTimestep",
            "value": "0.01",
        }),
    );
    assert_eq!(updated["error"], json!(null));
    assert_eq!(updated["updated"], json!(true));

    let written = fs::read_to_string(root.join("ProjectSettings/TimeManager.asset"))?;
    assert!(written.contains("m_FixedTimestep: 0.01"));
    assert!(written.contains("m_TimeScale: 1"));

    let settings = table.invoke("settings.inspect", &project_args(&root));
    assert_eq!(settings["fixedTimestep"], json!(0.01));

    let unknown = table.invoke(
        "settings.update",
        &json!({
            "projectPath": root.to_string_lossy(),
            "file": "TimeManager.asset",
            "key": "TimeManager.m_Missing",
            "value": "1",
        }),
    );
    assert_eq!(unknown["error"]["kind"], json!("tool-failed"));
    Ok(())
}

#[test]
fn compat_report_lists_findings_and_owned_feature_states() -> TestResult {
    let root = temp_dir("compat_report")?;
    build_project(&root)?;
    let table = table()?;

    let report = table.invoke("compat.report", &project_args(&root));
    assert_eq!(report["error"], json!(null));
    assert_eq!(report["declaredUnityVersion"], json!("2022.3.20f1"));
    assert_eq!(report["effectiveBehaviorVersion"], json!("2022.3"));
    assert_eq!(report["assetFindings"].as_array().map(Vec::len), Some(4));
    assert!(report["ownedFeatures"].as_array().is_some_and(|states| {
        states
            .iter()
            .any(|state| state["featureId"] == json!("PRJ-002"))
    }));
    Ok(())
}

#[test]
fn compat_package_matrix_reports_the_supported_packages() -> TestResult {
    let table = table()?;

    let matrix = table.invoke("compat.package_matrix", &json!({}));
    assert_eq!(matrix["error"], json!(null));
    let packages = matrix["packages"]
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    assert!(packages > 0);
    assert!(matrix["packages"].as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry["packageId"] == json!("com.unity.render-pipelines.universal"))
    }));
    Ok(())
}

#[test]
fn compat_version_info_reports_the_declared_and_effective_versions() -> TestResult {
    let root = temp_dir("version_info")?;
    build_project(&root)?;
    let table = table()?;

    let version = table.invoke("compat.version_info", &project_args(&root));
    assert_eq!(version["error"], json!(null));
    assert_eq!(version["declaredUnityVersion"], json!("2022.3.20f1"));
    assert_eq!(version["effectiveBehaviorVersion"], json!("2022.3"));
    assert_eq!(version["migration"], json!("current"));
    Ok(())
}

#[test]
fn diagnostics_list_reports_the_trust_blocked_script() -> TestResult {
    let root = temp_dir("diagnostics")?;
    build_project(&root)?;
    let table = table()?;

    let diagnostics = table.invoke("diagnostics.list", &project_args(&root));
    assert_eq!(diagnostics["error"], json!(null));
    let entries = diagnostics["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(entries.iter().any(|entry| {
        entry["category"] == json!("import")
            && entry["severity"] == json!("warning")
            && entry["state"] == json!("blocked by trust")
    }));
    Ok(())
}

#[test]
fn profiler_capture_reports_timing_markers_and_counters() -> TestResult {
    let root = temp_dir("profiler")?;
    build_project(&root)?;
    let table = table()?;

    let capture = table.invoke("profiler.capture", &project_args(&root));
    assert_eq!(capture["error"], json!(null));
    let markers = capture["markers"].as_array().cloned().unwrap_or_default();
    assert_eq!(markers.len(), 3);
    assert_eq!(markers[0]["name"], json!("project.open"));
    assert!(capture["totalMicros"].as_u64().is_some());
    assert_eq!(capture["counters"]["assets"], json!(4));
    Ok(())
}

#[test]
fn accessibility_inspect_reports_fonts_text_assets_and_input_axes() -> TestResult {
    let root = temp_dir("accessibility")?;
    build_project(&root)?;
    write_file(&root.join("Assets/Fonts/Body.ttf"), "font bytes\n")?;
    write_file(
        &root.join("Assets/Fonts/Body.ttf.meta"),
        &meta_with_labels(
            "55555555555555555555555555555555",
            "TrueTypeFontImporter:",
            &[],
        ),
    )?;
    let table = table()?;

    let inspection = table.invoke("accessibility.inspect", &project_args(&root));
    assert_eq!(inspection["error"], json!(null));
    assert_eq!(inspection["fonts"], json!(["Assets/Fonts/Body.ttf"]));
    assert_eq!(inspection["textAssets"], json!(["Assets/Notes.txt"]));
    assert_eq!(inspection["sortingLayers"], json!(["UI"]));
    Ok(())
}
