use super::*;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "unitas_build_bundle_test_{label}_{}_{id}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn sample_unity_scene(root_name: &str) -> String {
    format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: {root_name}\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_Father: {{fileID: 0}}\n  m_Children: []\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n"
    )
}

#[test]
fn content_hash_hex_matches_sha256_test_vectors() {
    let expected_empty = [
        "e3b0c442", "98fc1c14", "9afbf4c8", "996fb924", "27ae41e4", "649b934c", "a495991b",
        "7852b855",
    ]
    .concat();
    let expected_abc = [
        "ba7816bf", "8f01cfea", "414140de", "5dae2223", "b00361a3", "96177a9c", "b410ff61",
        "f20015ad",
    ]
    .concat();
    assert_eq!(content_hash_hex(b""), expected_empty);
    assert_eq!(content_hash_hex(b"abc"), expected_abc);
}

#[test]
fn repeated_builds_produce_byte_identical_manifests_and_hashes(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("determinism")?;
    let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
    fs::create_dir_all(&scenes_dir)?;
    let main_scene = scenes_dir.join("Main.unity");
    let level_scene = scenes_dir.join("LevelTwo.unity");
    fs::write(&main_scene, sample_unity_scene("MainRoot"))?;
    fs::write(&level_scene, sample_unity_scene("LevelRoot"))?;

    let host = BuildTarget::MACOS_ARM64;
    let config = BuildConfiguration::new(
        vec![main_scene, level_scene],
        "Unitas",
        "DeterministicGame",
        "com.unitas.deterministic",
        "1.2.3",
        host,
    );

    let out_one = workspace.join("out_one");
    let out_two = workspace.join("out_two");

    let bundle_one = stage_bundle_for_host(&config, &out_one, Some(host))?;
    let bundle_two = stage_bundle_for_host(&config, &out_two, Some(host))?;

    let manifest_bytes_one = fs::read(&bundle_one.manifest_path)?;
    let manifest_bytes_two = fs::read(&bundle_two.manifest_path)?;
    assert_eq!(manifest_bytes_one, manifest_bytes_two);
    assert_eq!(bundle_one.manifest, bundle_two.manifest);
    assert_eq!(
        bundle_one.manifest.manifest_hash,
        bundle_two.manifest.manifest_hash
    );
    assert_eq!(bundle_one.manifest.manifest_hash.len(), 64);
    assert_eq!(bundle_one.manifest.entries.len(), 2);
    assert_eq!(
        bundle_one.manifest.startup_scene_staged_path,
        "Assets/scene_000_Main.unity"
    );
    for entry in &bundle_one.manifest.entries {
        assert_eq!(entry.root_object_count, 1);
        assert_eq!(entry.content_hash.len(), 64);
        assert!(out_one.join(&entry.staged_path).is_file());
    }
    assert!(bundle_one.launch_artifact_path.is_file());

    let round_trip = BundleManifest::load_from_dir(&out_one)?;
    assert_eq!(round_trip, bundle_one.manifest);

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn development_and_release_builds_differ_by_diagnostics_marker_and_clean_stale_marker(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("dev_vs_release")?;
    let scene_path = workspace.join("Main.unity");
    fs::write(&scene_path, sample_unity_scene("PlayerRoot"))?;

    let host = BuildTarget::MACOS_ARM64;
    let out_dir = workspace.join("build_out");

    let dev_config = BuildConfiguration::new(
        vec![scene_path.clone()],
        "Unitas",
        "DiagGame",
        "com.unitas.diag",
        "0.5.0",
        host,
    )
    .with_development_build(true);

    let dev_bundle = stage_bundle_for_host(&dev_config, &out_dir, Some(host))?;
    assert!(dev_bundle.manifest.development_diagnostics);
    assert_eq!(
        dev_bundle.manifest.diagnostics_marker,
        Some(DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE.to_string())
    );
    let marker_path = out_dir.join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME);
    assert_eq!(
        dev_bundle.diagnostics_marker_path,
        Some(marker_path.clone())
    );
    assert!(marker_path.is_file());

    let release_config = dev_config.with_development_build(false);
    let release_bundle = stage_bundle_for_host(&release_config, &out_dir, Some(host))?;
    assert!(!release_bundle.manifest.development_diagnostics);
    assert_eq!(release_bundle.manifest.diagnostics_marker, None);
    assert_eq!(release_bundle.diagnostics_marker_path, None);
    assert!(!marker_path.exists());
    assert_ne!(
        dev_bundle.manifest.manifest_hash,
        release_bundle.manifest.manifest_hash
    );

    let loaded_release = BundleManifest::load_from_dir(&out_dir)?;
    assert_eq!(loaded_release, release_bundle.manifest);

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn missing_or_malformed_scene_fails_without_writing_manifest_or_launch_artifact(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("invalid_scenes")?;
    let host = BuildTarget::MACOS_ARM64;

    let missing_scene = workspace.join("DoesNotExist.unity");
    let missing_out = workspace.join("missing_out");
    let missing_config = BuildConfiguration::new(
        vec![missing_scene],
        "Unitas",
        "MissingSceneGame",
        "com.unitas.missing",
        "1.0.0",
        host,
    );

    let missing_result = stage_bundle_for_host(&missing_config, &missing_out, Some(host));
    assert!(matches!(
        missing_result,
        Err(BundleError::SceneMissing { .. })
    ));
    assert!(!missing_out.join(BUNDLE_MANIFEST_FILENAME).exists());
    assert!(!missing_out
        .join(host.executable_name(&missing_config.product_name))
        .exists());

    let broken_scene = workspace.join("Broken.unity");
    fs::write(
        &broken_scene,
        "%YAML 1.1\n\
%TAG !u! tag:unity3d.com,2011:\n\
--- !u!4 &201\n\
Transform:\n\
  m_GameObject: {fileID: 999}\n",
    )?;
    let broken_out = workspace.join("broken_out");
    let broken_config = BuildConfiguration::new(
        vec![broken_scene],
        "Unitas",
        "BrokenSceneGame",
        "com.unitas.broken",
        "1.0.0",
        host,
    );

    let broken_result = stage_bundle_for_host(&broken_config, &broken_out, Some(host));
    assert!(matches!(
        broken_result,
        Err(BundleError::SceneAssembly { .. })
    ));
    assert!(!broken_out.join(BUNDLE_MANIFEST_FILENAME).exists());
    assert!(!broken_out
        .join(host.executable_name(&broken_config.product_name))
        .exists());

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn staged_desktop_launcher_is_the_player_binary() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("desktop_launcher")?;
    let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
    fs::create_dir_all(&scenes_dir)?;
    let main_scene = scenes_dir.join("Main.unity");
    fs::write(&main_scene, sample_unity_scene("MainRoot"))?;

    let player_binary =
        package::locate_player_binary().ok_or("the unitas player binary has not been built")?;
    let player_binary_bytes = fs::read(&player_binary)?;

    for (target, launcher_name) in [
        (BuildTarget::WINDOWS_X86_64, "PackagedGame.exe"),
        (BuildTarget::LINUX_X86_64, "PackagedGame"),
    ] {
        let config = BuildConfiguration::new(
            vec![main_scene.clone()],
            "Unitas",
            "PackagedGame",
            "com.unitas.packaged",
            "1.0.0",
            target,
        );
        let output_dir = workspace.join(format!("out_{}", target.identifier()));
        let staged = stage_bundle_for_host(&config, &output_dir, Some(target))?;

        assert_eq!(
            staged
                .launch_artifact_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some(launcher_name)
        );
        assert_eq!(
            fs::read(&staged.launch_artifact_path)?,
            player_binary_bytes,
            "the staged launcher of {target} is not the player binary"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&staged.launch_artifact_path)?
                .permissions()
                .mode();
            assert_eq!(
                mode & 0o111,
                0o111,
                "the staged launcher of {target} is not executable"
            );
        }
    }

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn script_compile_cache_key_separates_debug_and_non_debug_compiles(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("compile_cache_key")?;
    let scripts_dir = workspace.join("Assets").join("Scripts");
    fs::create_dir_all(&scripts_dir)?;
    fs::write(scripts_dir.join("Player.cs"), "public class Player { }\n")?;

    let release_key = script_compile_cache_key(&workspace, &workspace, false);
    let debug_key = script_compile_cache_key(&workspace, &workspace, true);
    assert_ne!(release_key, debug_key);
    assert_eq!(
        release_key,
        script_compile_cache_key(&workspace, &workspace, false)
    );

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn script_compile_command_passes_the_debug_flag_only_for_debugging_compiles() {
    let release = script_compile_command(
        Path::new("/bridge/Unitas.Scripting.dll"),
        Path::new("/project"),
        Path::new("/staging"),
        false,
    );
    let debug = script_compile_command(
        Path::new("/bridge/Unitas.Scripting.dll"),
        Path::new("/project"),
        Path::new("/staging"),
        true,
    );
    let release_args: Vec<&OsStr> = release.get_args().collect();
    let debug_args: Vec<&OsStr> = debug.get_args().collect();
    assert!(release_args.contains(&OsStr::new("--player")));
    assert!(debug_args.contains(&OsStr::new("compile")));
    assert!(!release_args.contains(&OsStr::new("--debug")));
    assert!(debug_args.contains(&OsStr::new("--debug")));
}

#[test]
fn staging_records_script_host_from_whether_project_carries_scripts(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = temp_dir("script_host_manifest")?;
    let host = BuildTarget::MACOS_ARM64;
    assert_scriptless_staging(&workspace, host)?;
    assert_scripted_staging(&workspace, host)?;
    fs::remove_dir_all(&workspace)?;
    Ok(())
}

fn assert_scriptless_staging(
    workspace: &Path,
    host: BuildTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    let scriptless_scenes = workspace.join("Scriptless").join("Assets").join("Scenes");
    fs::create_dir_all(&scriptless_scenes)?;
    let scriptless_scene = scriptless_scenes.join("Main.unity");
    fs::write(&scriptless_scene, sample_unity_scene("MainRoot"))?;
    let scriptless_config = BuildConfiguration::new(
        vec![scriptless_scene],
        "Unitas",
        "ScriptlessGame",
        "com.unitas.scriptless",
        "1.0.0",
        host,
    );
    let scriptless_out = workspace.join("scriptless_out");
    let scriptless_staged = stage_bundle_for_host(&scriptless_config, &scriptless_out, Some(host))?;
    assert_eq!(
        scriptless_staged.manifest.format_version,
        BUNDLE_MANIFEST_FORMAT_VERSION
    );
    assert_eq!(scriptless_staged.manifest.script_host, ScriptHost::None);
    assert!(fs::read_to_string(&scriptless_staged.manifest_path)?.contains("script_host=none"));
    Ok(())
}

fn assert_scripted_staging(
    workspace: &Path,
    host: BuildTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    let scripted_root = workspace.join("Scripted");
    let scripted_scenes = scripted_root.join("Assets").join("Scenes");
    let scripted_scripts = scripted_root.join("Assets").join("Scripts");
    fs::create_dir_all(&scripted_scenes)?;
    fs::create_dir_all(&scripted_scripts)?;
    let models_dir = scripted_root.join("Assets").join("Models");
    fs::create_dir_all(&models_dir)?;
    fs::write(
        models_dir.join("Quad.obj"),
        "o Quad\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\nvn 0.0 0.0 1.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 1.0 1.0\nvt 0.0 1.0\nf 1/1/1 2/2/1 3/3/1\nf 1/1/1 3/3/1 4/4/1\n",
    )?;
    fs::write(
        models_dir.join("Quad.obj.meta"),
        "fileFormatVersion: 2\nguid: 1234567890abcdef1234567890abcdef\n",
    )?;
    fs::write(models_dir.join("Dummy.gltf"), b"not used")?;
    fs::write(models_dir.join("Dummy.glb"), b"not used")?;
    let scripted_scene = scripted_scenes.join("Main.unity");
    fs::write(&scripted_scene, sample_unity_scene("MainRoot"))?;
    fs::write(
        scripted_scripts.join("Controller.cs"),
        "public class Controller { }\n",
    )?;
    let scripted_config = BuildConfiguration::new(
        vec![scripted_scene],
        "Unitas",
        "ScriptedGame",
        "com.unitas.scripted",
        "1.0.0",
        host,
    );
    let scripted_out = workspace.join("scripted_out");
    let scripted_staged = stage_bundle_for_host(&scripted_config, &scripted_out, Some(host))?;
    assert_eq!(
        scripted_staged.manifest.format_version,
        BUNDLE_MANIFEST_FORMAT_VERSION
    );
    assert_eq!(scripted_staged.manifest.script_host, ScriptHost::CoreClr);
    assert!(fs::read_to_string(&scripted_staged.manifest_path)?.contains("script_host=coreclr"));

    let round_trip = BundleManifest::load_from_dir(&scripted_out)?;
    assert_eq!(round_trip.script_host, ScriptHost::CoreClr);
    assert!(!scripted_out.join("Assets/Models/Quad.obj").exists());
    assert!(!scripted_out.join("Assets/Models/Dummy.gltf").exists());
    assert!(!scripted_out.join("Assets/Models/Dummy.glb").exists());
    assert!(scripted_out.join("Assets/Models/Quad.obj.meta").is_file());
    let cooked_dir = scripted_out.join(crate::cooked::COOKED_MESH_DIR_NAME);
    let mesh_manifest = crate::cooked::CookedMeshManifest::load_from_dir(&cooked_dir)
        .ok_or("expected cooked mesh manifest")?;
    let quad_entry = mesh_manifest
        .get_by_asset_path("Assets/Models/Quad.obj")
        .ok_or("expected Assets/Models/Quad.obj entry")?;
    assert!(cooked_dir.join(&quad_entry.cooked_path).is_file());
    Ok(())
}
