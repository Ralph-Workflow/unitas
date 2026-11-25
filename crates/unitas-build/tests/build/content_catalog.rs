use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_build::bundle::{
    build_content_catalog_for_host, build_content_update_for_host, validate_content_bundle,
    ContentBuildMode, ContentBundleInput, ContentCatalog, ContentRejectionReason,
    CONTENT_CATALOG_FILENAME,
};
use unitas_build::report::{BuildDiagnostic, BuildOutcome, ContentBuildReport};
use unitas_build::{BuildConfiguration, BuildMode, BuildTarget};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "unitas_build_content_catalog_test_{label}_{}_{id}",
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

fn sample_unity_scene(root_name: &str) -> String {
    format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: {root_name}\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_Father: {{fileID: 0}}\n  m_Children: []\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n"
    )
}

#[test]
fn content_update_diffing_against_v1_catalog_emits_only_changed_bundles_and_updated_catalog(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("story_25_content_update")?;
    let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
    fs::create_dir_all(&scenes_dir)?;

    let boot_scene = scenes_dir.join("Boot.unity");
    let level_one_scene = scenes_dir.join("LevelOne.unity");
    let level_two_scene = scenes_dir.join("LevelTwo.unity");

    fs::copy(fixture_scene("scene_alpha.unity"), &boot_scene)?;
    fs::copy(fixture_scene("scene_beta.unity"), &level_one_scene)?;
    fs::write(&level_two_scene, sample_unity_scene("LevelTwoInitial"))?;

    let host = BuildTarget::host();
    let v1_config = BuildConfiguration::new(
        vec![
            boot_scene.clone(),
            level_one_scene.clone(),
            level_two_scene.clone(),
        ],
        "Unitas",
        "CatalogGame",
        "com.unitas.cataloggame",
        "1.0.0",
        host,
    )
    .with_mode(BuildMode::Release);

    let v1_out = workspace.join("v1_content_out");
    let v1_build = build_content_catalog_for_host(&v1_config, &v1_out, Some(host))?;

    assert_eq!(v1_build.catalog.unitas_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(v1_build.catalog.player_compatibility_version, "1.0");
    assert_eq!(v1_build.catalog.target, host);
    assert_eq!(v1_build.catalog.bundles.len(), 3);
    assert_eq!(v1_build.emitted_bundles.len(), 3);
    assert!(v1_build.unchanged_bundles.is_empty());
    assert!(v1_build.rejected_bundles.is_empty());
    assert!(v1_out.join(CONTENT_CATALOG_FILENAME).is_file());

    let loaded_v1_catalog = ContentCatalog::load_from_dir(&v1_out)?;
    assert_eq!(loaded_v1_catalog, v1_build.catalog);
    assert!(loaded_v1_catalog.is_compatible_with_player("1.0.0", host));
    assert!(loaded_v1_catalog.is_compatible_with_player("1.0.4", host));
    assert!(!loaded_v1_catalog.is_compatible_with_player("2.0.0", host));

    fs::write(&level_two_scene, sample_unity_scene("LevelTwoUpdatedBoss"))?;

    let update_out = workspace.join("v1_update_out");
    let update_build =
        build_content_update_for_host(&v1_config, &v1_build.catalog, &update_out, Some(host))?;

    assert_eq!(update_build.emitted_bundles.len(), 1);
    let emitted = &update_build.emitted_bundles[0];
    assert!(emitted.bundle_name.contains("LevelTwo"));
    assert!(update_out.join(&emitted.staged_path).is_file());

    assert_eq!(update_build.unchanged_bundles.len(), 2);
    for unchanged_name in &update_build.unchanged_bundles {
        let unchanged_entry = update_build
            .catalog
            .bundles
            .iter()
            .find(|entry| &entry.bundle_name == unchanged_name);
        assert!(unchanged_entry.is_some());
        if let Some(entry) = unchanged_entry {
            assert!(!update_out.join(&entry.staged_path).exists());
        }
    }

    assert_eq!(update_build.catalog.bundles.len(), 3);
    assert_ne!(
        update_build.catalog.catalog_hash,
        v1_build.catalog.catalog_hash
    );
    assert!(update_build
        .catalog
        .is_compatible_with_player("1.0.0", host));

    let loaded_update_catalog = ContentCatalog::load_from_dir(&update_out)?;
    assert_eq!(loaded_update_catalog, update_build.catalog);

    let report = ContentBuildReport::from_build_result(
        &update_build,
        ContentBuildMode::ContentUpdate,
        25,
    );
    assert_eq!(report.outcome, BuildOutcome::Succeeded);
    assert_eq!(report.unitas_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(report.player_compatibility_version, "1.0");
    assert_eq!(report.target, host);
    assert_eq!(report.emitted_bundles.len(), 1);
    assert_eq!(report.unchanged_bundles.len(), 2);
    assert!(report.rejected_bundles.is_empty());

    let report_json = report.to_json();
    let parsed_report = ContentBuildReport::from_json(&report_json)?;
    assert_eq!(parsed_report, report);

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn unity_built_and_incompatible_bundles_are_reported_by_name_and_reason_and_excluded_from_staging(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("story_25_rejected_bundles")?;
    let host = BuildTarget::host();
    let mismatched_target = if host == BuildTarget::WINDOWS_X86_64 {
        BuildTarget::LINUX_X86_64
    } else {
        BuildTarget::WINDOWS_X86_64
    };

    let valid_scene_path = workspace.join("ValidScene.unity");
    fs::write(&valid_scene_path, sample_unity_scene("ValidRoot"))?;

    let unity_fs_path = workspace.join("characters_unity.bundle");
    let mut unity_fs_bytes = b"UnityFS\x00\x00\x00\x00\x075.x.x\x002022.3.20f1\x00".to_vec();
    unity_fs_bytes.extend_from_slice(&[0u8; 32]);
    fs::write(&unity_fs_path, &unity_fs_bytes)?;

    let unity_web_path = workspace.join("env_unityweb.bundle");
    fs::write(&unity_web_path, b"UnityWeb\x00\x00\x00\x00\x033.x.x\x00")?;

    let unity_raw_path = workspace.join("props_unityraw.bundle");
    fs::write(&unity_raw_path, b"UnityRaw\x00\x00\x00\x00\x033.x.x\x00")?;

    let incompatible_player_path = workspace.join("future_player.bundle");
    fs::write(
        &incompatible_player_path,
        sample_unity_scene("FuturePlayerContent"),
    )?;

    let wrong_target_path = workspace.join("wrong_target.bundle");
    fs::write(&wrong_target_path, sample_unity_scene("WrongTargetContent"))?;

    let inputs = vec![
        ContentBundleInput::new("valid_level", valid_scene_path, host, "1.0"),
        ContentBundleInput::new("characters_unity", unity_fs_path, host, "1.0"),
        ContentBundleInput::new("env_unityweb", unity_web_path, host, "1.0"),
        ContentBundleInput::new("props_unityraw", unity_raw_path, host, "1.0"),
        ContentBundleInput::new("future_player", incompatible_player_path, host, "2.0"),
        ContentBundleInput::new(
            "wrong_target",
            wrong_target_path,
            mismatched_target,
            "1.0",
        ),
    ];

    let out_dir = workspace.join("validated_content_out");
    let validation_result = validate_content_bundle(&inputs, host, "1.0", &out_dir)?;

    assert_eq!(validation_result.emitted_bundles.len(), 1);
    assert_eq!(
        validation_result.emitted_bundles[0].bundle_name,
        "valid_level"
    );
    assert!(out_dir
        .join(&validation_result.emitted_bundles[0].staged_path)
        .is_file());

    assert_eq!(validation_result.rejected_bundles.len(), 5);

    let find_rejection = |name: &str| {
        validation_result
            .rejected_bundles
            .iter()
            .find(|item| item.bundle_name == name)
    };

    let unity_fs_rej = find_rejection("characters_unity");
    assert!(matches!(
        unity_fs_rej,
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::UnityBuiltArchive { ref signature } if signature == "UnityFS"
        )
    ));

    let unity_web_rej = find_rejection("env_unityweb");
    assert!(matches!(
        unity_web_rej,
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::UnityBuiltArchive { ref signature } if signature == "UnityWeb"
        )
    ));

    let unity_raw_rej = find_rejection("props_unityraw");
    assert!(matches!(
        unity_raw_rej,
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::UnityBuiltArchive { ref signature } if signature == "UnityRaw"
        )
    ));

    let future_rej = find_rejection("future_player");
    assert!(matches!(
        future_rej,
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::IncompatiblePlayerVersion {
                ref bundle_version,
                ref expected_version,
            } if bundle_version == "2.0" && expected_version == "1.0"
        )
    ));

    let target_rej = find_rejection("wrong_target");
    assert!(matches!(
        target_rej,
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::TargetMismatch {
                bundle_target,
                expected_target,
            } if bundle_target == mismatched_target && expected_target == host
        )
    ));

    for rejected_name in [
        "characters_unity",
        "env_unityweb",
        "props_unityraw",
        "future_player",
        "wrong_target",
    ] {
        assert!(!validation_result
            .catalog
            .bundles
            .iter()
            .any(|b| b.bundle_name == rejected_name));
        let staged_candidate = out_dir
            .join("ContentBundles")
            .join(format!("{rejected_name}.bundle"));
        assert!(!staged_candidate.exists());
    }

    let report = ContentBuildReport::from_build_result(
        &validation_result,
        ContentBuildMode::FullCatalog,
        12,
    );
    assert_eq!(report.outcome, BuildOutcome::Failed);
    assert_eq!(report.rejected_bundles.len(), 5);
    for rej in &validation_result.rejected_bundles {
        let diag = BuildDiagnostic::from_rejected_content_bundle(rej);
        assert_eq!(diag.source.as_deref(), Some(rej.bundle_name.as_str()));
        assert!(diag.message.contains(&rej.bundle_name));
        assert!(report.errors.contains(&diag));
    }

    let json = report.to_json();
    let round_trip = ContentBuildReport::from_json(&json)?;
    assert_eq!(round_trip, report);

    fs::remove_dir_all(&workspace)?;
    Ok(())
}
