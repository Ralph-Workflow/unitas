use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_build::bundle::{stage_bundle_for_host, PROJECT_SETTINGS_DIR_NAME};
use unitas_build::config::{BuildConfiguration, BuildMode};
use unitas_build::licensing::licensing_record;
use unitas_build::package::package_windows_app;
use unitas_build::sample::stage_marble_rush_bundle_for_host;
use unitas_build::targets::BuildTarget;
use unitas_render::ShadowSettings;
use unitas_runtime::presenter::{shadow_settings_for_runtime, DESKTOP_QUALITY_TARGET_GROUP};
use unitas_runtime::Runtime;

type TestResult = Result<(), Box<dyn Error>>;

fn test_output_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let directory = std::env::temp_dir()
        .join("unitas-build-tests")
        .join(format!("{label}_{}", std::process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory)?;
    }
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn fixture_scene(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn bundled_player_carries_project_quality_settings() -> TestResult {
    let output_dir = test_output_dir("project_settings_bundle")?;
    let staged = stage_marble_rush_bundle_for_host(&output_dir, None)?;

    let quality_asset = output_dir
        .join(PROJECT_SETTINGS_DIR_NAME)
        .join("QualitySettings.asset");
    assert!(
        quality_asset.is_file(),
        "the bundle must carry the project quality settings at '{}'",
        quality_asset.display()
    );

    let startup_scene = output_dir.join(&staged.manifest.startup_scene_staged_path);
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&startup_scene)?;

    let level = runtime
        .quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP)
        .ok_or("the bundled Standalone quality level did not resolve")?;
    assert_eq!(level.name, "High");
    assert_eq!(level.shadows, 2);

    let shadows = shadow_settings_for_runtime(&runtime);
    assert!(
        shadows.enabled,
        "the bundled Standalone quality level must enable directional shadows"
    );
    assert!(shadows.soft);
    assert_eq!(shadows.resolution_index, 2);
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn packaged_player_carries_project_quality_settings() -> TestResult {
    let output_dir = test_output_dir("packaged_player_settings")?;
    let staged = stage_marble_rush_bundle_for_host(&output_dir, None)?;
    let player_binary = output_dir.join("unitas-player-dummy");
    fs::write(&player_binary, b"unitas dummy player binary")?;
    let licensing = licensing_record(&staged.manifest.application_identifier);
    let packaged = package_windows_app(
        &staged.manifest,
        &output_dir,
        &output_dir.join("windows-layout"),
        &player_binary,
        &licensing,
        None,
    )?;

    assert!(packaged
        .data_dir
        .join(PROJECT_SETTINGS_DIR_NAME)
        .join("QualitySettings.asset")
        .is_file());

    let startup_scene = packaged
        .data_dir
        .join(&staged.manifest.startup_scene_staged_path);
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&startup_scene)?;
    assert!(shadow_settings_for_runtime(&runtime).enabled);
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn bundle_without_project_settings_keeps_the_shadow_fallback() -> TestResult {
    let output_dir = test_output_dir("project_settings_fallback")?;
    let config = BuildConfiguration::new(
        vec![fixture_scene("scene_alpha.unity")],
        "Unitas",
        "SettingsLessGame",
        "com.unitas.settingsless",
        "1.0.0",
        BuildTarget::host(),
    )
    .with_mode(BuildMode::Development);
    let staged = stage_bundle_for_host(&config, &output_dir, None)?;

    assert!(
        !output_dir.join(PROJECT_SETTINGS_DIR_NAME).exists(),
        "a project without settings must not gain any"
    );

    let startup_scene = output_dir.join(&staged.manifest.startup_scene_staged_path);
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&startup_scene)?;

    assert!(runtime
        .quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP)
        .is_none());
    assert_eq!(
        shadow_settings_for_runtime(&runtime),
        ShadowSettings::disabled()
    );
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}
