use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_build::bundle::{
    build_addressables_content_for_host, build_addressables_content_update_for_host,
    build_content_catalog_for_host, build_content_update_for_host, validate_content_bundle,
    AddressablesGroupConfig, AddressablesProfile, ContentBuildMode, ContentBundleInput,
    ContentCatalog, ContentRejectionReason, CONTENT_CATALOG_FILENAME,
};
use unitas_build::player::apply_content_update_to_player;
use unitas_build::report::{BuildDiagnostic, BuildOutcome, ContentBuildReport};
use unitas_build::{build_project, launch_bundle, BuildConfiguration, BuildMode, BuildTarget};

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
        vec![boot_scene, level_one_scene, level_two_scene.clone()],
        "Unitas",
        "CatalogGame",
        "com.unitas.cataloggame",
        "1.0.0",
        host,
    )
    .with_mode(BuildMode::Release);

    let v1_out = workspace.join("v1_content_out");
    let v1_build = build_content_catalog_for_host(&v1_config, &v1_out, Some(host))?;
    assert_v1_content_catalog(&v1_out, &v1_build, host)?;

    fs::write(&level_two_scene, sample_unity_scene("LevelTwoUpdatedBoss"))?;

    let update_out = workspace.join("v1_update_out");
    let update_build =
        build_content_update_for_host(&v1_config, &v1_build.catalog, &update_out, Some(host))?;
    assert_v1_content_update(&update_out, &v1_build, &update_build, host)?;

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_v1_content_catalog(
    v1_out: &std::path::Path,
    v1_build: &unitas_build::bundle::ContentBuildResult,
    host: BuildTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(v1_build.catalog.unitas_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(v1_build.catalog.player_compatibility_version, "1.0");
    assert_eq!(v1_build.catalog.target, host);
    assert_eq!(v1_build.catalog.bundles.len(), 3);
    assert_eq!(v1_build.emitted_bundles.len(), 3);
    assert!(v1_build.unchanged_bundles.is_empty());
    assert!(v1_build.rejected_bundles.is_empty());
    assert!(v1_out.join(CONTENT_CATALOG_FILENAME).is_file());

    let loaded_v1_catalog = ContentCatalog::load_from_dir(v1_out)?;
    assert_eq!(loaded_v1_catalog, v1_build.catalog);
    assert!(loaded_v1_catalog.is_compatible_with_player("1.0.0", host));
    assert!(loaded_v1_catalog.is_compatible_with_player("1.0.4", host));
    assert!(!loaded_v1_catalog.is_compatible_with_player("2.0.0", host));
    Ok(())
}

fn assert_v1_content_update(
    update_out: &std::path::Path,
    v1_build: &unitas_build::bundle::ContentBuildResult,
    update_build: &unitas_build::bundle::ContentBuildResult,
    host: BuildTarget,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let loaded_update_catalog = ContentCatalog::load_from_dir(update_out)?;
    assert_eq!(loaded_update_catalog, update_build.catalog);

    let report =
        ContentBuildReport::from_build_result(update_build, ContentBuildMode::ContentUpdate, 25);
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

    let inputs = build_rejected_bundle_test_inputs(&workspace, host, mismatched_target)?;
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
    assert_rejected_bundle_reasons(&validation_result, host, mismatched_target);
    assert_rejected_bundles_excluded_and_reported(&validation_result, &out_dir)?;

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn build_rejected_bundle_test_inputs(
    workspace: &std::path::Path,
    host: BuildTarget,
    mismatched_target: BuildTarget,
) -> Result<Vec<ContentBundleInput>, Box<dyn std::error::Error>> {
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

    Ok(vec![
        ContentBundleInput::new("valid_level", valid_scene_path, host, "1.0"),
        ContentBundleInput::new("characters_unity", unity_fs_path, host, "1.0"),
        ContentBundleInput::new("env_unityweb", unity_web_path, host, "1.0"),
        ContentBundleInput::new("props_unityraw", unity_raw_path, host, "1.0"),
        ContentBundleInput::new("future_player", incompatible_player_path, host, "2.0"),
        ContentBundleInput::new("wrong_target", wrong_target_path, mismatched_target, "1.0"),
    ])
}

fn assert_rejected_bundle_reasons(
    validation_result: &unitas_build::bundle::ContentBuildResult,
    host: BuildTarget,
    mismatched_target: BuildTarget,
) {
    let find_rejection = |name: &str| {
        validation_result
            .rejected_bundles
            .iter()
            .find(|item| item.bundle_name == name)
    };

    for (name, expected_sig) in [
        ("characters_unity", "UnityFS"),
        ("env_unityweb", "UnityWeb"),
        ("props_unityraw", "UnityRaw"),
    ] {
        assert!(matches!(
            find_rejection(name),
            Some(rej) if matches!(
                rej.reason,
                ContentRejectionReason::UnityBuiltArchive { ref signature } if signature == expected_sig
            )
        ));
    }

    assert!(matches!(
        find_rejection("future_player"),
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::IncompatiblePlayerVersion {
                ref bundle_version,
                ref expected_version,
            } if bundle_version == "2.0" && expected_version == "1.0"
        )
    ));

    assert!(matches!(
        find_rejection("wrong_target"),
        Some(rej) if matches!(
            rej.reason,
            ContentRejectionReason::TargetMismatch {
                bundle_target,
                expected_target,
            } if bundle_target == mismatched_target && expected_target == host
        )
    ));
}

fn assert_rejected_bundles_excluded_and_reported(
    validation_result: &unitas_build::bundle::ContentBuildResult,
    out_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let report =
        ContentBuildReport::from_build_result(validation_result, ContentBuildMode::FullCatalog, 12);
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
    Ok(())
}

#[test]
fn bld_005_story_25_addressables_profile_groups_dependencies_and_shipped_player_content_update_without_rebuild(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("bld_005_story_25_addressables_and_player_update")?;
    let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
    fs::create_dir_all(&scenes_dir)?;

    let boot_scene = scenes_dir.join("Boot.unity");
    let arena_scene = scenes_dir.join("Arena.unity");
    let boss_scene = scenes_dir.join("Boss.unity");

    fs::write(&boot_scene, sample_unity_scene("BootRootV1"))?;
    fs::write(&arena_scene, sample_unity_scene("ArenaRootV1"))?;
    fs::write(&boss_scene, sample_unity_scene("BossRootV1"))?;

    let host = BuildTarget::host();
    let player_config = BuildConfiguration::new(
        vec![boot_scene.clone(), arena_scene.clone(), boss_scene.clone()],
        "Unitas",
        "ShippedGame",
        "com.unitas.shippedgame",
        "1.0.0",
        host,
    )
    .with_mode(BuildMode::Release);

    let player_dir = workspace.join("shipped_player");
    let (staged_player, _player_report) = match build_project(&player_config, &player_dir, 30) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    let initial_launcher_bytes = fs::read(&staged_player.launch_artifact_path)?;
    let initial_session = launch_bundle(&player_dir, 2, 1.0 / 60.0)?;
    assert_eq!(initial_session.root_object_count, 1);

    let (profile, groups) =
        build_addressables_profile_and_groups(boot_scene.clone(), arena_scene, boss_scene);

    let v1_content_dir = workspace.join("v1_addressables");
    let v1_result = build_addressables_content_for_host(
        &player_config,
        &profile,
        &groups,
        &v1_content_dir,
        Some(host),
    )?;
    assert_v1_addressables_catalog(&v1_content_dir, &v1_result, host)?;

    assert_v2_addressables_update_and_player_apply(
        &workspace,
        &boot_scene,
        &player_config,
        &profile,
        &groups,
        &v1_result.catalog,
        (&player_dir, &staged_player, &initial_launcher_bytes),
    )?;

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn build_addressables_profile_and_groups(
    boot_scene: PathBuf,
    arena_scene: PathBuf,
    boss_scene: PathBuf,
) -> (AddressablesProfile, Vec<AddressablesGroupConfig>) {
    let mut profile = AddressablesProfile::new(
        "ProductionCDN",
        "ServerData/[BuildTarget]",
        "https://cdn.unitas.example/content/[BuildTarget]",
    );
    profile.set_variable("Env", "prod");

    let groups = vec![
        AddressablesGroupConfig::new(
            "core_shared",
            vec![boot_scene],
            "{BuildPath}/{Env}/core",
            "{LoadPath}/{Env}/core",
            Vec::new(),
        ),
        AddressablesGroupConfig::new(
            "world_arena",
            vec![arena_scene],
            "{BuildPath}/{Env}/world",
            "{LoadPath}/{Env}/world",
            vec![String::from("core_shared")],
        ),
        AddressablesGroupConfig::new(
            "world_boss",
            vec![boss_scene],
            "{BuildPath}/{Env}/boss",
            "{LoadPath}/{Env}/boss",
            vec![String::from("core_shared"), String::from("world_arena")],
        ),
    ];
    (profile, groups)
}

fn assert_v1_addressables_catalog(
    v1_content_dir: &std::path::Path,
    v1_result: &unitas_build::bundle::ContentBuildResult,
    host: BuildTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        v1_result.catalog.unitas_version,
        env!("CARGO_PKG_VERSION"),
        "BLD-005 story 25: catalog must record unitas version"
    );
    assert_eq!(
        v1_result.catalog.target, host,
        "BLD-005 story 25: catalog must record target platform"
    );
    assert!(
        v1_result
            .catalog
            .compatible_player_versions
            .contains(&String::from("1.0.x")),
        "BLD-005 story 25: catalog must state compatible shipped player versions"
    );
    assert_eq!(
        v1_result.catalog.profile_name.as_deref(),
        Some("ProductionCDN"),
        "BLD-005 story 25: catalog must record Addressables profile identity"
    );
    assert!(
        !v1_result.catalog.catalog_id.is_empty(),
        "BLD-005 story 25: catalog must record catalog identity"
    );

    let target_id = host.identifier();
    let boss_entry = v1_result
        .catalog
        .bundles
        .iter()
        .find(|entry| entry.group_name == "world_boss")
        .ok_or("BLD-005 story 25: missing world_boss bundle entry")?;
    assert_eq!(
        boss_entry.build_path,
        format!("ServerData/{target_id}/prod/boss/world_boss_000_Boss.bundle"),
        "BLD-005 story 25: Addressables group build path must expand profile variables and [BuildTarget]"
    );
    assert_eq!(
        boss_entry.load_path,
        format!("https://cdn.unitas.example/content/{target_id}/prod/boss/world_boss_000_Boss.bundle"),
        "BLD-005 story 25: Addressables group load path must expand profile variables and [BuildTarget]"
    );
    assert_eq!(
        boss_entry.dependencies,
        vec![String::from("core_shared"), String::from("world_arena")],
        "BLD-005 story 25: bundle entry must record its dependencies"
    );
    assert!(
        v1_content_dir.join(&boss_entry.staged_path).is_file(),
        "BLD-005 story 25: bundle must be staged at its resolved profile build path"
    );
    Ok(())
}

fn assert_v2_addressables_update_and_player_apply(
    workspace: &std::path::Path,
    boot_scene: &std::path::Path,
    player_config: &BuildConfiguration,
    profile: &AddressablesProfile,
    groups: &[AddressablesGroupConfig],
    v1_catalog: &ContentCatalog,
    player_state: (&std::path::Path, &unitas_build::StagedBundle, &[u8]),
) -> Result<(), Box<dyn std::error::Error>> {
    let (player_dir, staged_player, initial_launcher_bytes) = player_state;
    let host = player_config.target;
    let updated_boot_yaml = format!(
        "{}\n--- !u!1 &200\nGameObject:\n  m_Name: LiveEventBanner\n--- !u!4 &201\nTransform:\n  m_GameObject: {{fileID: 200}}\n  m_Father: {{fileID: 0}}\n  m_Children: []\n  m_LocalPosition: {{x: 1, y: 2, z: 3}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n",
        sample_unity_scene("BootRootV2")
    );
    fs::write(boot_scene, updated_boot_yaml)?;

    let v2_update_dir = workspace.join("v2_addressables_update");
    let v2_update = build_addressables_content_update_for_host(
        player_config,
        profile,
        groups,
        v1_catalog,
        &v2_update_dir,
        Some(host),
    )?;

    assert_eq!(v2_update.emitted_bundles.len(), 1);
    assert_eq!(v2_update.emitted_bundles[0].group_name, "core_shared");
    assert_eq!(v2_update.unchanged_bundles.len(), 2);
    assert_ne!(v2_update.catalog.catalog_hash, v1_catalog.catalog_hash);

    let v2_report =
        ContentBuildReport::from_build_result(&v2_update, ContentBuildMode::ContentUpdate, 18);
    assert_eq!(v2_report.catalog_id, v2_update.catalog.catalog_id);
    assert_eq!(
        v2_report.compatible_player_versions,
        v2_update.catalog.compatible_player_versions
    );
    assert_eq!(v2_report.bundles.len(), 3);
    let report_json = v2_report.to_json();
    let parsed_v2_report = ContentBuildReport::from_json(&report_json)?;
    assert_eq!(parsed_v2_report, v2_report);

    let reloaded_v2_catalog = ContentCatalog::load_from_dir(&v2_update_dir)?;
    assert_eq!(reloaded_v2_catalog, v2_update.catalog);

    let updated_session = apply_content_update_to_player(player_dir, &v2_update, 3, 1.0 / 60.0)?;
    assert_eq!(
        fs::read(&staged_player.launch_artifact_path)?,
        initial_launcher_bytes
    );
    assert_eq!(updated_session.root_object_count, 2);
    Ok(())
}

fn collect_relative_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_relative_files(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map(std::path::Path::to_path_buf)
                .unwrap_or(path);
            out.push(rel);
        }
    }
    Ok(())
}

#[test]
fn ser005_outcome17_project_open_and_build_to_named_output_folder_writes_only_sln_csproj_and_named_build_output(
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_asset::project::{MigrationStatus, Project};
    use unitas_asset::trust::{TrustOrigin, TrustStore};

    let workspace = temp_dir("ser005_outcome17_build_footprint")?;
    let project_root = workspace.join("project_2022");
    let per_user_dir = workspace.join("unitas_user_state");
    fs::create_dir_all(&per_user_dir)?;
    let (scene_path, initial_project_files) = setup_ser005_project(&project_root)?;

    let store = TrustStore::new(per_user_dir.join("trust.json"));
    let (_, open_report) = Project::open(&project_root, &store)?;
    assert_eq!(open_report.declared_version.as_deref(), Some("2022.3.16f1"));
    assert_eq!(open_report.migration, MigrationStatus::Current);
    assert!(!project_root
        .join("ProjectSettings/UnitasVersion.txt")
        .exists());

    let mut after_open_files = Vec::new();
    collect_relative_files(&project_root, &project_root, &mut after_open_files)?;
    after_open_files.sort();
    assert_eq!(after_open_files, initial_project_files);

    store.trust_project(&project_root.to_string_lossy(), TrustOrigin::Person)?;

    let (sln_rel, csproj_rel) = write_project_solution_files(&project_root)?;
    build_and_verify_ser005_output(&workspace, &scene_path)?;
    assert_ser005_footprint(
        &workspace,
        &project_root,
        &initial_project_files,
        &[csproj_rel, sln_rel],
    )?;

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn setup_ser005_project(
    project_root: &std::path::Path,
) -> Result<(PathBuf, Vec<PathBuf>), Box<dyn std::error::Error>> {
    use unitas_asset::project::BASELINE_UNITY_VERSION;

    fs::create_dir_all(project_root.join("Assets/Scenes"))?;
    fs::create_dir_all(project_root.join("ProjectSettings"))?;

    let scene_path = project_root.join("Assets/Scenes/Main.unity");
    let scene_meta_path = project_root.join("Assets/Scenes/Main.unity.meta");
    let version_path = project_root.join("ProjectSettings/ProjectVersion.txt");
    let settings_path = project_root.join("ProjectSettings/ProjectSettings.asset");

    fs::write(&scene_path, sample_unity_scene("FootprintRoot"))?;
    fs::write(
        &scene_meta_path,
        "fileFormatVersion: 2\nguid: 33333333333333333333333333333333\nNativeFormatImporter:\n  externalObjects: {}\n  serializedVersion: 2\n  userData: \n",
    )?;
    fs::write(
        &version_path,
        format!("m_EditorVersion: {BASELINE_UNITY_VERSION}.16f1\n"),
    )?;
    fs::write(
        &settings_path,
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  companyName: Unitas\n  productName: FootprintGame\n  bundleVersion: 1.0.0\n",
    )?;

    let mut initial_project_files = Vec::new();
    collect_relative_files(project_root, project_root, &mut initial_project_files)?;
    initial_project_files.sort();
    Ok((scene_path, initial_project_files))
}

fn write_project_solution_files(
    project_root: &std::path::Path,
) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let sln_rel = PathBuf::from("project_2022.sln");
    let csproj_rel = PathBuf::from("Assembly-CSharp.csproj");
    fs::write(
        project_root.join(&sln_rel),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n",
    )?;
    fs::write(
        project_root.join(&csproj_rel),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    )?;
    Ok((sln_rel, csproj_rel))
}

fn build_and_verify_ser005_output(
    workspace: &std::path::Path,
    scene_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_build::player::BUILD_REPORT_FILENAME;
    use unitas_build::BUNDLE_MANIFEST_FILENAME;

    let host = BuildTarget::host();
    let config = BuildConfiguration::new(
        vec![scene_path.to_path_buf()],
        "Unitas",
        "FootprintGame",
        "com.unitas.footprintgame",
        "1.0.0",
        host,
    )
    .with_mode(BuildMode::Release);

    let named_output_dir = workspace.join("NamedBuildOutput");
    let (staged_player, build_report) = match build_project(&config, &named_output_dir, 12) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    assert_eq!(build_report.outcome, BuildOutcome::Succeeded);
    assert!(staged_player.launch_artifact_path.exists());
    assert!(named_output_dir.join(BUNDLE_MANIFEST_FILENAME).is_file());
    assert!(named_output_dir.join(BUILD_REPORT_FILENAME).is_file());

    let catalog_result = build_content_catalog_for_host(&config, &named_output_dir, Some(host))?;
    assert_eq!(catalog_result.emitted_bundles.len(), 1);
    assert!(named_output_dir.join(CONTENT_CATALOG_FILENAME).is_file());

    let launched = launch_bundle(&named_output_dir, 1, 1.0 / 60.0)?;
    assert_eq!(launched.root_object_count, 1);
    Ok(())
}

fn assert_ser005_footprint(
    workspace: &std::path::Path,
    project_root: &std::path::Path,
    initial_project_files: &[PathBuf],
    expected_added: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    for generated in ["Library", "Temp", "Logs", "obj", "UserSettings"] {
        let generated_path = project_root.join(generated);
        assert!(
            !generated_path.exists(),
            "generated directory written into project content: {}",
            generated_path.display()
        );
    }
    assert!(!project_root
        .join("ProjectSettings/UnitasVersion.txt")
        .exists());

    let mut after_build_project_files = Vec::new();
    collect_relative_files(project_root, project_root, &mut after_build_project_files)?;
    after_build_project_files.sort();

    let added_in_project: Vec<PathBuf> = after_build_project_files
        .iter()
        .filter(|p| !initial_project_files.contains(p))
        .cloned()
        .collect();
    assert_eq!(added_in_project, expected_added);

    let mut workspace_files = Vec::new();
    collect_relative_files(workspace, workspace, &mut workspace_files)?;
    workspace_files.sort();
    for rel in &workspace_files {
        let top = rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_default();
        assert!(
            top == "project_2022" || top == "unitas_user_state" || top == "NamedBuildOutput",
            "unexpected file written outside project content: {}",
            rel.display()
        );
    }
    Ok(())
}
