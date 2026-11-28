use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_build::{
    build_project, execute_build, launch_bundle, BuildConfiguration, BuildMode, BuildOutcome,
    BuildReport, BuildTarget, BundleError, BundleManifest, PlayerStartupError,
    BUILD_REPORT_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "unitas_build_e2e_test_{label}_{}_{id}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn fixture_scene(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn end_to_end_build_report_determinism_and_headless_player_launch(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("e2e_launch")?;
    let alpha = fixture_scene("scene_alpha.unity");
    let beta = fixture_scene("scene_beta.unity");

    let host = BuildTarget::host();
    let dev_config = BuildConfiguration::new(
        vec![alpha, beta],
        "Unitas",
        "E2EPlayerGame",
        "com.unitas.e2eplayer",
        "1.0.0",
        host,
    )
    .with_mode(BuildMode::Development);

    let dev_out_1 = workspace.join("dev_out_1");
    let dev_out_2 = workspace.join("dev_out_2");

    let (dev_bundle_1, dev_report_1) = match build_project(&dev_config, &dev_out_1, 42) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    let (dev_bundle_2, dev_report_2) = match build_project(&dev_config, &dev_out_2, 42) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };

    assert_dev_bundle_determinism_and_report(
        &dev_out_1,
        &dev_bundle_1,
        &dev_bundle_2,
        &dev_report_1,
        &dev_report_2,
    )?;

    let release_config = dev_config.with_mode(BuildMode::Release);
    let release_out = workspace.join("release_out");
    let (release_bundle, release_report) = match build_project(&release_config, &release_out, 35) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    assert!(!release_out
        .join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME)
        .exists());
    assert!(!release_report.development_diagnostics_included);
    assert_ne!(
        dev_bundle_1.manifest.manifest_hash,
        release_bundle.manifest.manifest_hash
    );

    assert_headless_and_cli_launches(&dev_out_1, &release_out, &dev_bundle_1)?;
    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_dev_bundle_determinism_and_report(
    dev_out_1: &std::path::Path,
    dev_bundle_1: &unitas_build::StagedBundle,
    dev_bundle_2: &unitas_build::StagedBundle,
    dev_report_1: &BuildReport,
    dev_report_2: &BuildReport,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(dev_bundle_1.manifest, dev_bundle_2.manifest);
    assert_eq!(
        fs::read(&dev_bundle_1.manifest_path)?,
        fs::read(&dev_bundle_2.manifest_path)?
    );
    assert!(dev_out_1
        .join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME)
        .is_file());
    assert_eq!(dev_report_1.outcome, BuildOutcome::Succeeded);
    assert_eq!(dev_report_2.outcome, BuildOutcome::Succeeded);
    assert!(dev_report_1.development_diagnostics_included);
    assert!(dev_report_1.total_size_bytes > 0);
    assert_eq!(dev_report_1.output_location.as_deref(), Some(dev_out_1));
    assert_eq!(
        dev_report_1.launch_artifact.as_deref(),
        Some(dev_bundle_1.launch_artifact_path.as_path())
    );
    for cat in ["scenes", "metadata", "launcher", "diagnostics"] {
        assert!(dev_report_1
            .category_sizes
            .iter()
            .any(|entry| entry.category == cat && entry.size_bytes > 0));
    }
    assert!(dev_report_1.major_contributors.len() >= 4);

    let report_file = dev_out_1.join(BUILD_REPORT_FILENAME);
    assert!(report_file.is_file());
    let loaded_report = BuildReport::read_from_file(&report_file)?;
    assert_eq!(&loaded_report, dev_report_1);
    Ok(())
}

fn assert_headless_and_cli_launches(
    dev_out_1: &std::path::Path,
    release_out: &std::path::Path,
    dev_bundle_1: &unitas_build::StagedBundle,
) -> Result<(), Box<dyn std::error::Error>> {
    let dev_session = launch_bundle(dev_out_1, 5, 1.0 / 60.0)?;
    assert_eq!(dev_session.frames_stepped, 5);
    assert_eq!(dev_session.loaded_scene_count, 1);
    assert!(dev_session.root_object_count >= 1);
    assert!(dev_session.development_diagnostics_enabled);
    assert_eq!(dev_session.manifest, dev_bundle_1.manifest);

    let release_session = launch_bundle(release_out, 5, 1.0 / 60.0)?;
    assert_eq!(release_session.frames_stepped, 5);
    assert_eq!(release_session.loaded_scene_count, 1);
    assert!(release_session.root_object_count >= 1);
    assert!(!release_session.development_diagnostics_enabled);

    let Some(bin_path) = unitas_build::package::locate_player_binary() else {
        return Ok(());
    };
    let cli_ok = std::process::Command::new(&bin_path)
        .arg(dev_out_1)
        .arg("--frames")
        .arg("3")
        .arg("--dt")
        .arg("0.016")
        .output()?;
    assert!(cli_ok.status.success());
    let stdout = String::from_utf8(cli_ok.stdout)?;
    assert!(stdout.contains("launched_product=E2EPlayerGame"));
    assert!(stdout.contains("frames_stepped=3"));
    Ok(())
}

#[test]
fn startup_failures_are_distinct_for_missing_manifest_missing_scene_and_corrupt_scene(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("startup_errors")?;
    let empty_bundle = workspace.join("empty_bundle");
    fs::create_dir_all(&empty_bundle)?;

    let missing_manifest_result = launch_bundle(&empty_bundle, 1, 1.0 / 60.0);
    assert!(matches!(
        missing_manifest_result,
        Err(PlayerStartupError::ManifestLoad { ref bundle_dir, .. }) if bundle_dir == &empty_bundle
    ));

    let alpha = fixture_scene("scene_alpha.unity");
    let config = BuildConfiguration::for_host(vec![alpha], "StartupErrorsGame");

    let missing_scene_dir = assert_missing_scene_failure(&workspace, &config)?;
    let staged_corrupt = assert_corrupt_scene_failure(&workspace, &config)?;
    assert_no_startup_scene_failure(&workspace, &staged_corrupt.manifest)?;

    if let Some(bin_path) = unitas_build::package::locate_player_binary() {
        let cli_fail = std::process::Command::new(&bin_path)
            .arg(&missing_scene_dir)
            .output()?;
        assert!(!cli_fail.status.success());
        let stderr = String::from_utf8(cli_fail.stderr)?;
        assert!(stderr.contains("startup scene does not exist"));
    }

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_missing_scene_failure(
    workspace: &std::path::Path,
    config: &BuildConfiguration,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let missing_scene_dir = workspace.join("missing_scene_bundle");
    let (staged_missing, _) = match build_project(config, &missing_scene_dir, 10) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    let staged_scene_path =
        missing_scene_dir.join(&staged_missing.manifest.startup_scene_staged_path);
    fs::remove_file(&staged_scene_path)?;

    let missing_scene_result = launch_bundle(&missing_scene_dir, 1, 1.0 / 60.0);
    assert!(matches!(
        missing_scene_result,
        Err(PlayerStartupError::StartupSceneMissing { ref scene_path }) if scene_path == &staged_scene_path
    ));
    Ok(missing_scene_dir)
}

fn assert_corrupt_scene_failure(
    workspace: &std::path::Path,
    config: &BuildConfiguration,
) -> Result<unitas_build::StagedBundle, Box<dyn std::error::Error>> {
    let corrupt_scene_dir = workspace.join("corrupt_scene_bundle");
    let (staged_corrupt, _) = match build_project(config, &corrupt_scene_dir, 10) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    let corrupt_scene_path =
        corrupt_scene_dir.join(&staged_corrupt.manifest.startup_scene_staged_path);
    fs::write(&corrupt_scene_path, "not a valid unity yaml scene")?;

    let corrupt_scene_result = launch_bundle(&corrupt_scene_dir, 1, 1.0 / 60.0);
    assert!(matches!(
        corrupt_scene_result,
        Err(PlayerStartupError::SceneLoadFailed { ref scene_path, ref message })
            if scene_path == &corrupt_scene_path && !message.is_empty()
    ));
    Ok(staged_corrupt)
}

fn assert_no_startup_scene_failure(
    workspace: &std::path::Path,
    base_manifest: &BundleManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let no_scene_dir = workspace.join("no_scene_bundle");
    fs::create_dir_all(&no_scene_dir)?;
    let mut empty_manifest = base_manifest.clone();
    empty_manifest.startup_scene_staged_path.clear();
    empty_manifest.entries.clear();
    empty_manifest.total_content_bytes = 0;
    empty_manifest.manifest_hash = empty_manifest.compute_manifest_hash();
    fs::write(
        no_scene_dir.join("bundle.manifest"),
        empty_manifest.to_canonical_string(),
    )?;

    let no_startup_result = launch_bundle(&no_scene_dir, 1, 1.0 / 60.0);
    assert!(matches!(
        no_startup_result,
        Err(PlayerStartupError::NoStartupSceneConfigured { ref bundle_dir }) if bundle_dir == &no_scene_dir
    ));
    Ok(())
}

#[test]
fn failed_build_validation_and_missing_source_scene_produce_failed_build_report(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("failed_build_report")?;
    let host = BuildTarget::host();

    let invalid_config = BuildConfiguration::new(
        Vec::new(),
        "",
        "BrokenGame",
        "com.unitas.broken",
        "0.1.0",
        host,
    );
    let invalid_out = workspace.join("invalid_out");
    let outcome_bundle = execute_build(&invalid_config, &invalid_out, 15);
    assert!(outcome_bundle.bundle.is_none());
    assert_eq!(outcome_bundle.report.outcome, BuildOutcome::Failed);
    assert!(outcome_bundle.report.launch_artifact.is_none());
    assert!(outcome_bundle.report.output_location.is_none());
    assert!(outcome_bundle.report.errors.iter().any(|diag| {
        diag.code == "setting.scenes"
            && diag
                .remedy
                .as_deref()
                .is_some_and(|remedy| !remedy.is_empty())
    }));
    assert!(outcome_bundle.report.errors.iter().any(|diag| {
        diag.code == "setting.company_name"
            && diag
                .remedy
                .as_deref()
                .is_some_and(|remedy| !remedy.is_empty())
    }));

    let missing_source_scene = workspace.join("DoesNotExist.unity");
    let missing_scene_config =
        BuildConfiguration::for_host(vec![missing_source_scene.clone()], "MissingSceneGame");
    let missing_out = workspace.join("missing_out");
    let build_res = build_project(&missing_scene_config, &missing_out, 20);
    assert!(matches!(
        build_res.as_ref().map_err( AsRef::as_ref ),
        Err((BundleError::SceneMissing { ref path }, ref report))
            if path == &missing_source_scene
                && report.outcome == BuildOutcome::Failed
                && report.launch_artifact.is_none()
                && report.errors.iter().any(|diag| {
                    diag.code == "content.scene_missing"
                        && diag.remedy.as_deref().is_some_and(|remedy| !remedy.is_empty())
                })
    ));

    let _ = BundleManifest::from_canonical_str("");
    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn staged_bundle_cooks_textures_under_cooked_with_manifest_excludes_raw_textures_and_reuses_cache(
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_build::player::install_project_trust;
    use unitas_build::sample::stage_marble_rush_bundle_for_host;

    let workspace = temp_dir("cooked_bundle_pipeline")?;
    let bundle_dir = workspace.join("marble_rush_bundle");
    let staged = stage_marble_rush_bundle_for_host(&bundle_dir, None)?;
    install_project_trust(&bundle_dir)?;

    assert_staged_cooked_textures_and_meshes(&staged.output_dir)?;
    assert_staged_raw_assets_excluded(&staged.output_dir);
    assert_staged_cooked_audio(&staged.output_dir)?;
    assert_cooked_cache_reuse(&workspace)?;

    let session = launch_bundle(&bundle_dir, 2, 1.0 / 60.0)?;
    assert!(session.root_object_count > 0);

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_staged_cooked_textures_and_meshes(
    output_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_build::cooked::{
        CookedMesh, CookedMeshManifest, CookedTexture, CookedTextureManifest, COOKED_DIR_NAME,
        COOKED_MANIFEST_FILENAME, COOKED_MESH_MANIFEST_FILENAME,
    };

    let cooked_dir = output_dir.join(COOKED_DIR_NAME);
    assert!(cooked_dir.is_dir());
    assert!(cooked_dir.join(COOKED_MANIFEST_FILENAME).is_file());
    assert!(cooked_dir.join(COOKED_MESH_MANIFEST_FILENAME).is_file());

    let manifest = CookedTextureManifest::load_from_dir(&cooked_dir)
        .ok_or("expected cooked manifest in bundle")?;
    let pickup_entry = manifest
        .get_by_asset_path("Assets/Textures/Pickup.png")
        .ok_or("expected Assets/Textures/Pickup.png in cooked manifest")?;
    let cooked_file = cooked_dir.join(&pickup_entry.cooked_path);
    assert!(cooked_file.is_file());
    let decoded = CookedTexture::decode(&fs::read(&cooked_file)?)?;
    assert_eq!(decoded.width, pickup_entry.width);
    assert_eq!(decoded.height, pickup_entry.height);

    let mesh_manifest = CookedMeshManifest::load_from_dir(&cooked_dir)
        .ok_or("expected cooked mesh manifest in bundle")?;
    for asset_path in [
        "Assets/Models/ConcaveFrame.obj",
        "Assets/Models/ArenaProp.fbx",
    ] {
        let entry = mesh_manifest
            .get_by_asset_path(asset_path)
            .ok_or("expected mesh entry in cooked mesh manifest")?;
        let cooked_mesh_path = cooked_dir.join(&entry.cooked_path);
        assert!(cooked_mesh_path.is_file());
        let decoded_mesh = CookedMesh::decode(&fs::read(&cooked_mesh_path)?)?;
        assert_eq!(decoded_mesh.vertex_count(), entry.vertex_count);
        assert_eq!(decoded_mesh.index_count(), entry.index_count);
    }
    Ok(())
}

fn assert_staged_raw_assets_excluded(output_dir: &std::path::Path) {
    for rel in [
        "Assets/Textures/Pickup.png",
        "Assets/Models/ConcaveFrame.obj",
        "Assets/Models/ArenaProp.fbx",
        "Assets/Audio/CollectChime.wav",
    ] {
        assert!(!output_dir.join(rel).exists());
        assert!(output_dir.join(format!("{rel}.meta")).is_file());
    }
}

fn assert_staged_cooked_audio(
    output_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_build::cooked::{CookedAudio, CookedAudioManifest, COOKED_DIR_NAME};

    let cooked_dir = output_dir.join(COOKED_DIR_NAME);
    let audio_manifest = CookedAudioManifest::load_from_dir(&cooked_dir)
        .ok_or("expected cooked audio manifest in bundle")?;
    let chime_entry = audio_manifest
        .get_by_asset_path("Assets/Audio/CollectChime.wav")
        .ok_or("expected Assets/Audio/CollectChime.wav in cooked audio manifest")?;
    let cooked_audio_file = cooked_dir.join(&chime_entry.cooked_path);
    assert!(cooked_audio_file.is_file());
    let decoded_audio = CookedAudio::decode(&fs::read(&cooked_audio_file)?)?;
    assert_eq!(decoded_audio.channels, chime_entry.channels);
    assert_eq!(decoded_audio.sample_rate, chime_entry.sample_rate);
    assert_eq!(
        u32::try_from(decoded_audio.samples.len()).unwrap_or(0),
        chime_entry.sample_count
    );
    Ok(())
}

fn assert_cooked_cache_reuse(
    workspace: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_build::cooked::{
        stage_project_cooked_audio_with_cache, stage_project_cooked_meshes_with_cache,
        stage_project_cooked_textures_with_cache, COOKED_DIR_NAME, COOKED_MANIFEST_FILENAME,
        COOKED_MESH_MANIFEST_FILENAME,
    };

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("workspace root")?
        .join("samples")
        .join("marble-rush");
    let cache_dir = workspace.join("import_cache");
    let first_out = workspace.join("out_first");
    let (_, first_report) =
        stage_project_cooked_textures_with_cache(&project_root, &cache_dir, &first_out)?;
    assert!(first_report.imported > 0);
    assert_eq!(first_report.reused, 0);

    let second_out = workspace.join("out_second");
    let (_, second_report) =
        stage_project_cooked_textures_with_cache(&project_root, &cache_dir, &second_out)?;
    assert_eq!(second_report.imported, 0);
    assert_eq!(second_report.reused, first_report.imported);

    let audio_cache_dir = workspace.join("audio_import_cache");
    let (_, first_audio_report) =
        stage_project_cooked_audio_with_cache(&project_root, &audio_cache_dir, &first_out)?;
    assert!(first_audio_report.imported > 0);
    assert_eq!(first_audio_report.reused, 0);

    let (_, second_audio_report) =
        stage_project_cooked_audio_with_cache(&project_root, &audio_cache_dir, &second_out)?;
    assert_eq!(second_audio_report.imported, 0);
    assert_eq!(second_audio_report.reused, first_audio_report.imported);

    let mesh_cache_dir = workspace.join("mesh_import_cache");
    let (_, first_mesh_report) =
        stage_project_cooked_meshes_with_cache(&project_root, &mesh_cache_dir, &first_out)?;
    assert!(first_mesh_report.imported > 0);
    assert_eq!(first_mesh_report.reused, 0);
    assert!(first_out
        .join(COOKED_DIR_NAME)
        .join(COOKED_MANIFEST_FILENAME)
        .is_file());
    assert!(first_out
        .join(COOKED_DIR_NAME)
        .join(COOKED_MESH_MANIFEST_FILENAME)
        .is_file());

    let (_, second_mesh_report) =
        stage_project_cooked_meshes_with_cache(&project_root, &mesh_cache_dir, &second_out)?;
    assert_eq!(second_mesh_report.imported, 0);
    assert_eq!(second_mesh_report.reused, first_mesh_report.imported);
    Ok(())
}
