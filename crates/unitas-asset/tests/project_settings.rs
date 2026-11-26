use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project_settings::{
    load_project_settings, parse_audio_manager, parse_color_space, parse_editor_build_settings,
    parse_editor_settings, parse_graphics_settings, parse_mono_manager, parse_nav_mesh_areas,
    parse_package_manager_settings, parse_preset_manager, parse_quality_settings,
    parse_quality_settings_stream, parse_tag_manager, parse_version_control_settings,
    AudioManagerSettings, BuildSceneEntry, ColorSpaceSetting, EditorBuildSettings, EditorSettings,
    GraphicsSettings, MonoManagerSettings, NavMeshAreaEntry, NavMeshAreaSettings,
    PackageManagerSettings, PlayerSettings, PresetManagerSettings, QualityLevel, QualitySettings,
    SettingBoundary, SettingValueSource, SortingLayerEntry, TagManagerSettings,
    VersionControlSettings,
};
use unitas_asset::scene_save;
use unitas_asset::unity_yaml::Node;
use unitas_asset::{
    parse_dynamics_manager, parse_dynamics_manager_stream, parse_input_manager,
    parse_input_manager_stream, parse_player_settings, parse_player_settings_stream,
    parse_time_manager, parse_time_manager_stream, unity_yaml, DynamicsSettings, InputAxis,
    InputManagerSettings, ProjectIdentity, ProjectSettingsError, TimeSettings,
    DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY,
};

type TestResult = Result<(), Box<dyn Error>>;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn settings_dir() -> PathBuf {
    fixtures_dir().join("project").join("ProjectSettings")
}

#[test]
fn default_constants_match_unity_defaults() {
    assert_eq!(DEFAULT_FIXED_TIMESTEP, 0.02);
    assert_eq!(DEFAULT_GRAVITY, [0.0, -9.81, 0.0]);
}

#[test]
fn parses_time_manager_asset() -> TestResult {
    let path = settings_dir().join("TimeManager.asset");
    let settings = parse_time_manager(&path)?;
    assert_eq!(
        settings,
        Some(TimeSettings {
            fixed_timestep: 0.03333333333333333
        })
    );
    Ok(())
}

#[test]
fn parses_dynamics_manager_asset() -> TestResult {
    let path = settings_dir().join("DynamicsManager.asset");
    let settings = parse_dynamics_manager(&path)?;
    assert_eq!(
        settings,
        Some(DynamicsSettings {
            gravity: [0.0, -2.0, 0.0]
        })
    );
    Ok(())
}

#[test]
fn missing_files_return_ok_none() {
    let missing_time = settings_dir().join("MissingTimeManager.asset");
    let missing_dynamics = settings_dir().join("MissingDynamicsManager.asset");
    assert_eq!(parse_time_manager(&missing_time), Ok(None));
    assert_eq!(parse_dynamics_manager(&missing_dynamics), Ok(None));
}

#[test]
fn malformed_file_returns_yaml_error() {
    let broken = settings_dir().join("BrokenTimeManager.asset");
    let result = parse_time_manager(&broken);
    assert!(matches!(result, Err(ProjectSettingsError::Yaml { .. })));
}

#[test]
fn stream_variants_match_file_variants_and_reject_empty_stream() -> TestResult {
    let time_stream = unity_yaml::parse_file(&settings_dir().join("TimeManager.asset"))?;
    let dynamics_stream = unity_yaml::parse_file(&settings_dir().join("DynamicsManager.asset"))?;

    assert_eq!(
        parse_time_manager_stream(&time_stream),
        Ok(TimeSettings {
            fixed_timestep: 0.03333333333333333
        })
    );
    assert_eq!(
        parse_dynamics_manager_stream(&dynamics_stream),
        Ok(DynamicsSettings {
            gravity: [0.0, -2.0, 0.0]
        })
    );

    let empty = unity_yaml::parse("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n")?;
    assert_eq!(
        parse_time_manager_stream(&empty),
        Err(ProjectSettingsError::MissingDocument("TimeManager"))
    );
    assert_eq!(
        parse_dynamics_manager_stream(&empty),
        Err(ProjectSettingsError::MissingDocument("PhysicsManager"))
    );
    Ok(())
}

#[test]
fn dynamics_manager_stream_supports_dynamics_manager_root_key_and_rejects_invalid_values(
) -> TestResult {
    let dynamics_key_stream = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nDynamicsManager:\n  m_Gravity: {x: 1, y: -3.5, z: 0.25}\n",
    )?;
    assert_eq!(
        parse_dynamics_manager_stream(&dynamics_key_stream),
        Ok(DynamicsSettings {
            gravity: [1.0, -3.5, 0.25]
        })
    );

    let non_positive_time = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_FixedTimestep: 0\n",
    )?;
    assert_eq!(
        parse_time_manager_stream(&non_positive_time),
        Err(ProjectSettingsError::MissingDocument("TimeManager"))
    );
    Ok(())
}

#[test]
fn parses_input_manager_asset_and_skips_joystick_axes() -> TestResult {
    let path = fixtures_dir().join("InputManager.asset");
    let settings = parse_input_manager(&path)?;
    let expected = InputManagerSettings {
        axes: vec![
            InputAxis {
                name: String::from("Horizontal"),
                negative_button: String::from("left"),
                positive_button: String::from("right"),
                alt_negative_button: String::from("a"),
                alt_positive_button: String::from("d"),
                gravity: 3.0,
                dead: 0.001,
                sensitivity: 3.0,
                snap: true,
                invert: false,
                axis_type: 0,
                axis: 0,
            },
            InputAxis {
                name: String::from("Vertical"),
                negative_button: String::from("down"),
                positive_button: String::from("up"),
                alt_negative_button: String::from("s"),
                alt_positive_button: String::from("w"),
                gravity: 3.0,
                dead: 0.001,
                sensitivity: 3.0,
                snap: false,
                invert: true,
                axis_type: 0,
                axis: 0,
            },
            InputAxis {
                name: String::from("Fire1"),
                negative_button: String::new(),
                positive_button: String::from("mouse 0"),
                alt_negative_button: String::new(),
                alt_positive_button: String::from("left ctrl"),
                gravity: 1000.0,
                dead: 0.001,
                sensitivity: 1000.0,
                snap: false,
                invert: false,
                axis_type: 0,
                axis: 0,
            },
            InputAxis {
                name: String::from("Mouse X"),
                negative_button: String::new(),
                positive_button: String::new(),
                alt_negative_button: String::new(),
                alt_positive_button: String::new(),
                gravity: 0.0,
                dead: 0.0,
                sensitivity: 0.1,
                snap: false,
                invert: false,
                axis_type: 1,
                axis: 0,
            },
            InputAxis {
                name: String::from("Mouse Y"),
                negative_button: String::new(),
                positive_button: String::new(),
                alt_negative_button: String::new(),
                alt_positive_button: String::new(),
                gravity: 0.0,
                dead: 0.0,
                sensitivity: 0.1,
                snap: false,
                invert: false,
                axis_type: 1,
                axis: 1,
            },
            InputAxis {
                name: String::from("Mouse ScrollWheel"),
                negative_button: String::new(),
                positive_button: String::new(),
                alt_negative_button: String::new(),
                alt_positive_button: String::new(),
                gravity: 0.0,
                dead: 0.0,
                sensitivity: 0.1,
                snap: false,
                invert: false,
                axis_type: 1,
                axis: 2,
            },
        ],
        skipped_axes: 1,
    };
    assert_eq!(settings, Some(expected.clone()));

    let stream = unity_yaml::parse_file(&path)?;
    assert_eq!(parse_input_manager_stream(&stream), Ok(expected));
    Ok(())
}

#[test]
fn input_manager_handles_missing_file_and_m_type_aliases_and_rejects_missing_document() -> TestResult
{
    let missing = settings_dir().join("MissingInputManager.asset");
    assert_eq!(parse_input_manager(&missing), Ok(None));

    let empty = unity_yaml::parse("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n")?;
    assert_eq!(
        parse_input_manager_stream(&empty),
        Err(ProjectSettingsError::MissingDocument("InputManager"))
    );

    let m_type_stream = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!13 &1\nInputManager:\n  m_Axes:\n  - m_Name: Jump\n    positiveButton: space\n    m_Type: 0\n    m_Axis: 0\n  - m_Name: PadJump\n    m_Type: 2\n    m_Axis: 3\n",
    )?;
    assert_eq!(
        parse_input_manager_stream(&m_type_stream),
        Ok(InputManagerSettings {
            axes: vec![InputAxis {
                name: String::from("Jump"),
                negative_button: String::new(),
                positive_button: String::from("space"),
                alt_negative_button: String::new(),
                alt_positive_button: String::new(),
                gravity: 0.0,
                dead: 0.0,
                sensitivity: 1.0,
                snap: false,
                invert: false,
                axis_type: 0,
                axis: 0,
            }],
            skipped_axes: 1,
        })
    );
    Ok(())
}

#[test]
fn parses_player_settings_from_prefab_fixture_and_inline_streams() -> TestResult {
    let prefab_settings = fixtures_dir()
        .join("prefab_project")
        .join("ProjectSettings")
        .join("ProjectSettings.asset");
    let parsed = parse_player_settings(&prefab_settings)?;
    assert_eq!(
        parsed,
        Some(ProjectIdentity {
            company_name: Some(String::from("Unitas")),
            product_name: Some(String::from("PrefabFixture")),
            bundle_version: None,
            active_color_space: ColorSpaceSetting::Gamma,
        })
    );

    let full_stream = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  companyName: UnitasCorp\n  productName: UnitasGame\n  bundleVersion: 1.2.3\n",
    )?;
    assert_eq!(
        parse_player_settings_stream(&full_stream),
        Ok(ProjectIdentity {
            company_name: Some(String::from("UnitasCorp")),
            product_name: Some(String::from("UnitasGame")),
            bundle_version: Some(String::from("1.2.3")),
            active_color_space: ColorSpaceSetting::Gamma,
        })
    );

    let blank_fields = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  companyName:\n  productName: \"\"\n  bundleVersion: 0.5.0\n",
    )?;
    assert_eq!(
        parse_player_settings_stream(&blank_fields),
        Ok(ProjectIdentity {
            company_name: None,
            product_name: None,
            bundle_version: Some(String::from("0.5.0")),
            active_color_space: ColorSpaceSetting::Gamma,
        })
    );
    Ok(())
}

#[test]
fn player_settings_handles_missing_file_and_missing_document() -> TestResult {
    let missing = settings_dir().join("MissingProjectSettings.asset");
    assert_eq!(parse_player_settings(&missing), Ok(None));

    let empty = unity_yaml::parse("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n")?;
    assert_eq!(
        parse_player_settings_stream(&empty),
        Err(ProjectSettingsError::MissingDocument("PlayerSettings"))
    );
    Ok(())
}

fn temp_settings_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_project_settings_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn set_settings_scalar(node: &mut Node, keys: &[&str], value: &str) -> bool {
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
                return set_settings_scalar(entry_value, rest, value);
            }
            false
        }
        _ => false,
    }
}

#[test]
fn project_settings_unchanged_save_byte_identity_and_edit_save_reload_round_trip() -> TestResult {
    let root = temp_settings_dir("round_trip")?;
    let time_path = root.join("TimeManager.asset");
    let dynamics_path = root.join("DynamicsManager.asset");
    let input_path = root.join("InputManager.asset");
    let player_path = root.join("ProjectSettings.asset");

    let time_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_FixedTimestep: 0.02\n  m_MaximumAllowedTimestep: 0.33333334\n  m_TimeScale: 1\n  m_MaximumParticleTimestep: 0.03\n";
    let dynamics_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nPhysicsManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 13\n  m_Gravity: {x: 0, y: -9.81, z: 0}\n  m_DefaultSolverIterations: 6\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n";
    let input_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!13 &1\nInputManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_UsePhysicalKeys: 0\n  m_Axes:\n  - serializedVersion: 3\n    m_Name: Fire1\n    negativeButton:\n    positiveButton: mouse 0\n    altNegativeButton:\n    altPositiveButton: left ctrl\n    gravity: 1000\n    dead: 0.001\n    sensitivity: 1000\n    snap: 0\n    invert: 0\n    type: 0\n    axis: 0\n";
    let player_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 26\n  companyName: UnitasStudio\n  productName: UnitasSim\n  bundleVersion: 1.0.0\n  scriptingDefineSymbols:\n    1: FEATURE_ALPHA;ENABLE_TELEMETRY\n  customPlayerKey: preserved_value\n";

    fs::write(&time_path, time_yaml)?;
    fs::write(&dynamics_path, dynamics_yaml)?;
    fs::write(&input_path, input_yaml)?;
    fs::write(&player_path, player_yaml)?;

    for (path, expected_yaml) in [
        (&time_path, time_yaml),
        (&dynamics_path, dynamics_yaml),
        (&input_path, input_yaml),
        (&player_path, player_yaml),
    ] {
        let parsed = unity_yaml::parse(expected_yaml)?;
        assert_eq!(scene_save::emit_stream(&parsed), expected_yaml);
        scene_save::save_stream(path, &parsed)?;
        assert_eq!(fs::read_to_string(path)?, expected_yaml);
    }

    let mut time_stream = unity_yaml::parse_file(&time_path)?;
    assert!(set_settings_scalar(
        &mut time_stream.documents[0].root,
        &["TimeManager", "m_FixedTimestep"],
        "0.01"
    ));
    scene_save::save_stream(&time_path, &time_stream)?;

    let mut dynamics_stream = unity_yaml::parse_file(&dynamics_path)?;
    assert!(set_settings_scalar(
        &mut dynamics_stream.documents[0].root,
        &["PhysicsManager", "m_Gravity", "y"],
        "-19.62"
    ));
    assert!(set_settings_scalar(
        &mut dynamics_stream.documents[0].root,
        &["PhysicsManager", "m_LayerCollisionMatrix"],
        "0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff"
    ));
    scene_save::save_stream(&dynamics_path, &dynamics_stream)?;

    let mut player_stream = unity_yaml::parse_file(&player_path)?;
    assert!(set_settings_scalar(
        &mut player_stream.documents[0].root,
        &["PlayerSettings", "productName"],
        "UnitasSimPro"
    ));
    assert!(set_settings_scalar(
        &mut player_stream.documents[0].root,
        &["PlayerSettings", "scriptingDefineSymbols", "1"],
        "FEATURE_ALPHA;FEATURE_BETA"
    ));
    scene_save::save_stream(&player_path, &player_stream)?;

    assert_eq!(
        parse_time_manager(&time_path)?,
        Some(TimeSettings {
            fixed_timestep: 0.01
        })
    );
    let reloaded_time_stream = unity_yaml::parse_file(&time_path)?;
    assert_eq!(
        reloaded_time_stream.documents[0]
            .root
            .get("TimeManager")
            .and_then(|n| n.get("m_MaximumAllowedTimestep"))
            .and_then(Node::as_str),
        Some("0.33333334")
    );
    assert_eq!(
        reloaded_time_stream.documents[0]
            .root
            .get("TimeManager")
            .and_then(|n| n.get("m_MaximumParticleTimestep"))
            .and_then(Node::as_str),
        Some("0.03")
    );

    assert_eq!(
        parse_dynamics_manager(&dynamics_path)?,
        Some(DynamicsSettings {
            gravity: [0.0, -19.62, 0.0]
        })
    );
    let reloaded_dynamics_stream = unity_yaml::parse_file(&dynamics_path)?;
    assert_eq!(
        reloaded_dynamics_stream.documents[0]
            .root
            .get("PhysicsManager")
            .and_then(|n| n.get("m_LayerCollisionMatrix"))
            .and_then(Node::as_str),
        Some("0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff0000ffff")
    );
    assert_eq!(
        reloaded_dynamics_stream.documents[0]
            .root
            .get("PhysicsManager")
            .and_then(|n| n.get("m_DefaultSolverIterations"))
            .and_then(Node::as_str),
        Some("6")
    );

    assert_eq!(
        parse_player_settings(&player_path)?,
        Some(ProjectIdentity {
            company_name: Some(String::from("UnitasStudio")),
            product_name: Some(String::from("UnitasSimPro")),
            bundle_version: Some(String::from("1.0.0")),
            active_color_space: ColorSpaceSetting::Gamma,
        })
    );
    let reloaded_player_stream = unity_yaml::parse_file(&player_path)?;
    assert_eq!(
        reloaded_player_stream.documents[0]
            .root
            .get("PlayerSettings")
            .and_then(|n| n.get("scriptingDefineSymbols"))
            .and_then(|n| n.get("1"))
            .and_then(Node::as_str),
        Some("FEATURE_ALPHA;FEATURE_BETA")
    );
    assert_eq!(
        reloaded_player_stream.documents[0]
            .root
            .get("PlayerSettings")
            .and_then(|n| n.get("customPlayerKey"))
            .and_then(Node::as_str),
        Some("preserved_value")
    );

    let reloaded_input_stream = unity_yaml::parse_file(&input_path)?;
    assert_eq!(
        reloaded_input_stream.documents[0]
            .root
            .get("InputManager")
            .and_then(|n| n.get("m_UsePhysicalKeys"))
            .and_then(Node::as_str),
        Some("0")
    );
    let parsed_input = parse_input_manager(&input_path)?.ok_or("missing input manager")?;
    assert_eq!(parsed_input.axes.len(), 1);
    assert_eq!(parsed_input.axes[0].name, "Fire1");

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EffectiveTargetSetting {
    target_group_id: &'static str,
    target_name: &'static str,
    effective_value: String,
    source: &'static str,
}

fn resolve_per_target_scripting_defines(
    player_body: &Node,
    target_group_id: &'static str,
    target_name: &'static str,
    default_fallback: &str,
) -> EffectiveTargetSetting {
    let defines_map = player_body.get("scriptingDefineSymbols");
    if let Some(override_val) = defines_map
        .and_then(|m| m.get(target_group_id))
        .and_then(Node::as_str)
        .filter(|s| !s.is_empty())
    {
        return EffectiveTargetSetting {
            target_group_id,
            target_name,
            effective_value: override_val.to_string(),
            source: "target_override",
        };
    }
    let default_val = defines_map
        .and_then(|m| m.get("default"))
        .and_then(Node::as_str)
        .unwrap_or(default_fallback);
    EffectiveTargetSetting {
        target_group_id,
        target_name,
        effective_value: default_val.to_string(),
        source: "default",
    }
}

#[test]
fn prj007_outcome10_per_target_scripting_defines_collision_matrix_and_fixed_timestep_effective_target_resolution(
) -> TestResult {
    let root = temp_settings_dir("prj007_outcome10")?;
    let time_path = root.join("TimeManager.asset");
    let dynamics_path = root.join("DynamicsManager.asset");
    let player_path = root.join("ProjectSettings.asset");

    let initial_time = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_FixedTimestep: 0.02\n  m_MaximumAllowedTimestep: 0.33333334\n  m_TimeScale: 1\n";
    let initial_dynamics = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nPhysicsManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 13\n  m_Gravity: {x: 0, y: -9.81, z: 0}\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n";
    let initial_player = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 26\n  companyName: UnitasStudio\n  productName: TargetMatrixApp\n  bundleVersion: 2.0.0\n  scriptingDefineSymbols:\n    default: GLOBAL_DEFAULT\n    1: STANDALONE_BUILD; desktop_input\n    4: IOS_BUILD;MOBILE_INPUT\n    7: ANDROID_BUILD;MOBILE_INPUT\n  unrecognizedVendorSetting: retained_verbatim\n";

    fs::write(&time_path, initial_time)?;
    fs::write(&dynamics_path, initial_dynamics)?;
    fs::write(&player_path, initial_player)?;

    for (path, expected) in [
        (&time_path, initial_time),
        (&dynamics_path, initial_dynamics),
        (&player_path, initial_player),
    ] {
        let stream = unity_yaml::parse_file(path)?;
        scene_save::save_stream(path, &stream)?;
        assert_eq!(fs::read_to_string(path)?, expected);
    }

    let mut time_stream = unity_yaml::parse_file(&time_path)?;
    assert!(set_settings_scalar(
        &mut time_stream.documents[0].root,
        &["TimeManager", "m_FixedTimestep"],
        "0.008333334"
    ));
    scene_save::save_stream(&time_path, &time_stream)?;

    let mut dynamics_stream = unity_yaml::parse_file(&dynamics_path)?;
    assert!(set_settings_scalar(
        &mut dynamics_stream.documents[0].root,
        &["PhysicsManager", "m_LayerCollisionMatrix"],
        "ffffffff00000000ffffffff00000000ffffffff00000000ffffffff00000000"
    ));
    scene_save::save_stream(&dynamics_path, &dynamics_stream)?;

    let mut player_stream = unity_yaml::parse_file(&player_path)?;
    assert!(set_settings_scalar(
        &mut player_stream.documents[0].root,
        &["PlayerSettings", "scriptingDefineSymbols", "7"],
        "ANDROID_BUILD;MOBILE_INPUT;VULKAN_BACKEND"
    ));
    scene_save::save_stream(&player_path, &player_stream)?;

    let reloaded_time = parse_time_manager(&time_path)?.ok_or("missing TimeManager")?;
    assert!((reloaded_time.fixed_timestep - 0.008333334).abs() < 1e-9);

    let reloaded_dynamics_stream = unity_yaml::parse_file(&dynamics_path)?;
    assert_eq!(
        reloaded_dynamics_stream.documents[0]
            .root
            .get("PhysicsManager")
            .and_then(|n| n.get("m_LayerCollisionMatrix"))
            .and_then(Node::as_str),
        Some("ffffffff00000000ffffffff00000000ffffffff00000000ffffffff00000000")
    );

    let reloaded_player_stream = unity_yaml::parse_file(&player_path)?;
    let player_body = reloaded_player_stream.documents[0]
        .root
        .get("PlayerSettings")
        .ok_or("missing PlayerSettings")?;

    let standalone_eff =
        resolve_per_target_scripting_defines(player_body, "1", "Standalone", "GLOBAL_DEFAULT");
    assert_eq!(
        standalone_eff,
        EffectiveTargetSetting {
            target_group_id: "1",
            target_name: "Standalone",
            effective_value: String::from("STANDALONE_BUILD; desktop_input"),
            source: "target_override",
        }
    );

    let ios_eff = resolve_per_target_scripting_defines(player_body, "4", "iOS", "GLOBAL_DEFAULT");
    assert_eq!(
        ios_eff,
        EffectiveTargetSetting {
            target_group_id: "4",
            target_name: "iOS",
            effective_value: String::from("IOS_BUILD;MOBILE_INPUT"),
            source: "target_override",
        }
    );

    let android_eff =
        resolve_per_target_scripting_defines(player_body, "7", "Android", "GLOBAL_DEFAULT");
    assert_eq!(
        android_eff,
        EffectiveTargetSetting {
            target_group_id: "7",
            target_name: "Android",
            effective_value: String::from("ANDROID_BUILD;MOBILE_INPUT;VULKAN_BACKEND"),
            source: "target_override",
        }
    );

    let webgl_eff =
        resolve_per_target_scripting_defines(player_body, "13", "WebGL", "GLOBAL_DEFAULT");
    assert_eq!(
        webgl_eff,
        EffectiveTargetSetting {
            target_group_id: "13",
            target_name: "WebGL",
            effective_value: String::from("GLOBAL_DEFAULT"),
            source: "default",
        }
    );

    assert_eq!(
        player_body
            .get("unrecognizedVendorSetting")
            .and_then(Node::as_str),
        Some("retained_verbatim")
    );

    let typed_settings = unitas_asset::project_settings::load_project_settings(&root)?;
    assert_eq!(
        typed_settings.fixed_timestep(),
        0.008333334,
        "PRJ-007 Outcome 10: typed fixed timestep reflects saved setting"
    );
    assert_eq!(
        typed_settings.fixed_timestep_boundary(),
        unitas_asset::project_settings::SettingBoundary::NextPlaySession,
        "PRJ-007 Outcome 10: fixed timestep declares next play session boundary (RUN-004)"
    );
    assert!(
        !typed_settings.layers_collide(1, 0),
        "PRJ-007 Outcome 10: layer collision matrix bit disables collision between layer 1 and layer 0 (PHY-001)"
    );
    assert!(
        typed_settings.layers_collide(0, 0),
        "PRJ-007 Outcome 10: layer collision matrix bit keeps collision between layer 0 and layer 0 (PHY-001)"
    );
    assert_eq!(
        typed_settings.collision_matrix_boundary(),
        unitas_asset::project_settings::SettingBoundary::NextPhysicsStep,
        "PRJ-007 Outcome 10: layer collision matrix declares physics simulation boundary (PHY-001)"
    );
    let android_effective = typed_settings.effective_scripting_defines("7", "Android");
    assert_eq!(
        android_effective.effective_value, "ANDROID_BUILD;MOBILE_INPUT;VULKAN_BACKEND",
        "PRJ-007 Outcome 10: effective per-target scripting define for Android"
    );
    assert_eq!(
        android_effective.source,
        unitas_asset::project_settings::SettingValueSource::TargetOverride,
        "PRJ-007 Outcome 10: Android scripting define source is TargetOverride"
    );
    assert_eq!(
        android_effective.boundary,
        unitas_asset::project_settings::SettingBoundary::NextRecompilation,
        "PRJ-007 Outcome 10: scripting defines take effect at next recompilation (RUN-007)"
    );
    let webgl_effective = typed_settings.effective_scripting_defines("13", "WebGL");
    assert_eq!(
        webgl_effective.effective_value, "GLOBAL_DEFAULT",
        "PRJ-007 Outcome 10: unoverridden target falls back to default scripting defines"
    );
    assert_eq!(
        webgl_effective.source,
        unitas_asset::project_settings::SettingValueSource::Default
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj007_outcome10_fixed_timestep_collision_matrix_and_scripting_defines_change_behavior_at_feature_boundaries(
) -> TestResult {
    let root = temp_settings_dir("prj007_outcome10_boundaries")?;
    let time_path = root.join("TimeManager.asset");
    let dynamics_path = root.join("DynamicsManager.asset");
    let physics2d_path = root.join("Physics2DSettings.asset");
    let player_path = root.join("ProjectSettings.asset");

    let initial_time = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_FixedTimestep: 0.02\n  m_MaximumAllowedTimestep: 0.33333334\n  m_TimeScale: 1\n";
    let initial_dynamics = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nPhysicsManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 13\n  m_Gravity: {x: 0, y: -10, z: 0}\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n";
    let initial_physics2d = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!19 &1\nPhysics2DSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 5\n  m_Gravity: {x: 0, y: -9.81}\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n";
    let initial_player = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 26\n  companyName: UnitasStudio\n  productName: BoundaryApp\n  bundleVersion: 1.0.0\n  scriptingDefineSymbols:\n    default: GLOBAL_DEFAULT\n    1: STANDALONE_BUILD\n    7: ANDROID_BUILD;MOBILE_INPUT\n";

    fs::write(&time_path, initial_time)?;
    fs::write(&dynamics_path, initial_dynamics)?;
    fs::write(&physics2d_path, initial_physics2d)?;
    fs::write(&player_path, initial_player)?;

    let baseline_settings = unitas_asset::project_settings::load_project_settings(&root)?;
    let baseline_time_eval = baseline_settings.evaluate_fixed_step_boundary(0.10);
    assert_eq!(baseline_time_eval.fixed_timestep, 0.02);
    assert_eq!(baseline_time_eval.fixed_steps_executed, 5);
    assert!((baseline_time_eval.simulated_seconds - 0.10).abs() < 1e-12);
    assert!(baseline_time_eval.residual_accumulator_seconds < 1e-12);
    assert!((baseline_time_eval.gravity_y_displacement_meters - (-0.06)).abs() < 1e-12);

    let baseline_collision_0_1 = baseline_settings.evaluate_collision_boundary(0, 1);
    assert!(baseline_collision_0_1.collides_3d);
    assert!(baseline_collision_0_1.collides_2d);
    let baseline_collision_2_3 = baseline_settings.evaluate_collision_boundary(2, 3);
    assert!(baseline_collision_2_3.collides_3d);
    assert!(baseline_collision_2_3.collides_2d);

    let baseline_android_recompile = baseline_settings.evaluate_recompile_boundary("7", "Android");
    assert_eq!(
        baseline_android_recompile.active_symbols,
        vec![String::from("ANDROID_BUILD"), String::from("MOBILE_INPUT")]
    );
    assert_eq!(
        baseline_android_recompile.source,
        unitas_asset::project_settings::SettingValueSource::TargetOverride
    );

    let mut time_stream = unity_yaml::parse_file(&time_path)?;
    assert!(set_settings_scalar(
        &mut time_stream.documents[0].root,
        &["TimeManager", "m_FixedTimestep"],
        "0.01"
    ));
    scene_save::save_stream(&time_path, &time_stream)?;

    let mut updated_matrix_3d = baseline_settings.layer_collision_matrix;
    updated_matrix_3d[0] &= !(1u32 << 1);
    updated_matrix_3d[1] &= !(1u32 << 0);
    let updated_hex_3d =
        unitas_asset::project_settings::format_layer_collision_matrix(&updated_matrix_3d);
    let mut dynamics_stream = unity_yaml::parse_file(&dynamics_path)?;
    assert!(set_settings_scalar(
        &mut dynamics_stream.documents[0].root,
        &["PhysicsManager", "m_LayerCollisionMatrix"],
        &updated_hex_3d
    ));
    scene_save::save_stream(&dynamics_path, &dynamics_stream)?;

    let mut updated_matrix_2d = baseline_settings.layer_collision_matrix_2d;
    updated_matrix_2d[2] &= !(1u32 << 3);
    updated_matrix_2d[3] &= !(1u32 << 2);
    let updated_hex_2d =
        unitas_asset::project_settings::format_layer_collision_matrix(&updated_matrix_2d);
    let mut physics2d_stream = unity_yaml::parse_file(&physics2d_path)?;
    assert!(set_settings_scalar(
        &mut physics2d_stream.documents[0].root,
        &["Physics2DSettings", "m_LayerCollisionMatrix"],
        &updated_hex_2d
    ));
    scene_save::save_stream(&physics2d_path, &physics2d_stream)?;

    let mut player_stream = unity_yaml::parse_file(&player_path)?;
    assert!(set_settings_scalar(
        &mut player_stream.documents[0].root,
        &["PlayerSettings", "scriptingDefineSymbols", "7"],
        "ANDROID_BUILD;MOBILE_INPUT;VULKAN_BACKEND"
    ));
    scene_save::save_stream(&player_path, &player_stream)?;

    let active_session_eval = baseline_settings.evaluate_fixed_step_boundary(0.10);
    assert_eq!(
        active_session_eval.fixed_steps_executed, 5,
        "PRJ-007 Outcome 10: active play session keeps prior fixed timestep until NextPlaySession boundary"
    );

    let reloaded_settings = unitas_asset::project_settings::load_project_settings(&root)?;
    assert_eq!(
        reloaded_settings.fixed_timestep_boundary(),
        unitas_asset::project_settings::SettingBoundary::NextPlaySession
    );
    let next_session_eval = reloaded_settings.evaluate_fixed_step_boundary(0.10);
    assert_eq!(next_session_eval.fixed_timestep, 0.01);
    assert_eq!(next_session_eval.fixed_steps_executed, 10);
    assert_ne!(
        next_session_eval.fixed_steps_executed,
        baseline_time_eval.fixed_steps_executed
    );
    assert!((next_session_eval.simulated_seconds - 0.10).abs() < 1e-12);
    assert!(
        (next_session_eval.gravity_y_displacement_meters - (-0.055)).abs() < 1e-12,
        "PRJ-007 Outcome 10: changing fixed timestep from 0.02 to 0.01 changes semi-implicit Euler integration trajectory at NextPlaySession boundary"
    );
    assert_ne!(
        next_session_eval.gravity_y_displacement_meters,
        baseline_time_eval.gravity_y_displacement_meters
    );

    assert_eq!(
        reloaded_settings.collision_matrix_boundary(),
        unitas_asset::project_settings::SettingBoundary::NextPhysicsStep
    );
    let updated_collision_0_1 = reloaded_settings.evaluate_collision_boundary(0, 1);
    assert!(!updated_collision_0_1.collides_3d);
    assert!(updated_collision_0_1.collides_2d);
    let updated_collision_1_0 = reloaded_settings.evaluate_collision_boundary(1, 0);
    assert!(!updated_collision_1_0.collides_3d);
    assert_ne!(
        updated_collision_0_1.collides_3d,
        baseline_collision_0_1.collides_3d
    );

    let updated_collision_2_3 = reloaded_settings.evaluate_collision_boundary(2, 3);
    assert!(updated_collision_2_3.collides_3d);
    assert!(!updated_collision_2_3.collides_2d);
    let updated_collision_3_2 = reloaded_settings.evaluate_collision_boundary(3, 2);
    assert!(!updated_collision_3_2.collides_2d);
    assert_ne!(
        updated_collision_2_3.collides_2d,
        baseline_collision_2_3.collides_2d
    );

    let updated_android_recompile = reloaded_settings.evaluate_recompile_boundary("7", "Android");
    assert_eq!(
        updated_android_recompile.active_symbols,
        vec![
            String::from("ANDROID_BUILD"),
            String::from("MOBILE_INPUT"),
            String::from("VULKAN_BACKEND")
        ]
    );
    assert_ne!(
        updated_android_recompile.active_symbols,
        baseline_android_recompile.active_symbols
    );

    for path in [&time_path, &dynamics_path, &physics2d_path, &player_path] {
        let before = fs::read_to_string(path)?;
        let parsed = unity_yaml::parse(&before)?;
        scene_save::save_stream(path, &parsed)?;
        assert_eq!(fs::read_to_string(path)?, before);
    }

    fs::remove_dir_all(&root)?;
    Ok(())
}

const QUALITY_SETTINGS_YAML: &str = r"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!47 &1
QualitySettings:
  m_ObjectHideFlags: 0
  serializedVersion: 5
  m_CurrentQuality: 1
  m_QualitySettings:
  - serializedVersion: 2
    name: Low
    pixelLightCount: 1
    shadows: 0
    shadowResolution: 0
    shadowProjection: 1
    shadowCascades: 1
    shadowDistance: 40
    shadowNearPlaneOffset: 3
    shadowCascade2Split: 0.33333334
    shadowCascade4Split: {x: 0.06666667, y: 0.2, z: 0.46666667}
    shadowmaskMode: 0
    skinWeights: 2
    textureQuality: 0
    anisotropicTextures: 0
    antiAliasing: 0
    softParticles: 0
    softVegetation: 0
    realtimeReflectionProbes: 0
    billboardsFaceCameraPosition: 0
    vSyncCount: 0
    lodBias: 0.7
    maximumLODLevel: 0
    enableLODCrossFade: 1
    streamingMipmapsActive: 0
    streamingMipmapsAddAllCameras: 1
    streamingMipmapsMemoryBudget: 512
    streamingMipmapsRenderersPerFrame: 512
    streamingMipmapsMaxLevelReduction: 2
    streamingMipmapsMaxFileIORequests: 1024
    particleRaycastBudget: 4
    asyncUploadTimeSlice: 2
    asyncUploadBufferSize: 16
    asyncUploadPersistentBuffer: 1
    resolutionScalingFixedDPIFactor: 1
    customRenderPipeline: {fileID: 0}
    terrainQualityOverrides: 0
    reflectionQualityOverrides: 0
    unrecognizedVendorQualityKey: retained_verbatim
  - serializedVersion: 2
    name: High
    pixelLightCount: 4
    shadows: 2
    shadowResolution: 2
    shadowProjection: 1
    shadowCascades: 4
    shadowDistance: 50
    shadowNearPlaneOffset: 3
    shadowCascade2Split: 0.33333334
    shadowCascade4Split: {x: 0.06666667, y: 0.2, z: 0.46666667}
    shadowmaskMode: 0
    skinWeights: 2
    textureQuality: 0
    anisotropicTextures: 1
    antiAliasing: 2
    softParticles: 0
    softVegetation: 1
    realtimeReflectionProbes: 1
    billboardsFaceCameraPosition: 0
    vSyncCount: 1
    lodBias: 1
    maximumLODLevel: 0
    enableLODCrossFade: 1
    streamingMipmapsActive: 0
    streamingMipmapsAddAllCameras: 1
    streamingMipmapsMemoryBudget: 512
    streamingMipmapsRenderersPerFrame: 512
    streamingMipmapsMaxLevelReduction: 2
    streamingMipmapsMaxFileIORequests: 1024
    particleRaycastBudget: 4
    asyncUploadTimeSlice: 2
    asyncUploadBufferSize: 16
    asyncUploadPersistentBuffer: 1
    resolutionScalingFixedDPIFactor: 1
    customRenderPipeline: {fileID: 0}
    terrainQualityOverrides: 0
    reflectionQualityOverrides: 0
  m_PerPlatformDefaultQuality:
    Android: 1
    Standalone: 0
  m_PerPlatformCustomRenderPipeline: {}
";

fn expected_quality_settings() -> QualitySettings {
    QualitySettings {
        current_quality: 1,
        levels: vec![
            QualityLevel {
                name: String::from("Low"),
                pixel_light_count: 1,
                shadows: 0,
                shadow_resolution: 0,
                shadow_distance: 40.0,
                texture_quality: 0,
                anisotropic_textures: 0,
                anti_aliasing: 0,
                v_sync_count: 0,
                lod_bias: 0.7,
            },
            QualityLevel {
                name: String::from("High"),
                pixel_light_count: 4,
                shadows: 2,
                shadow_resolution: 2,
                shadow_distance: 50.0,
                texture_quality: 0,
                anisotropic_textures: 1,
                anti_aliasing: 2,
                v_sync_count: 1,
                lod_bias: 1.0,
            },
        ],
        per_platform_default_quality: vec![
            (String::from("Android"), 1),
            (String::from("Standalone"), 0),
        ],
    }
}

#[test]
fn parses_quality_settings_asset_and_reports_per_target_effective_value() -> TestResult {
    let root = temp_settings_dir("quality_parse")?;
    let path = root.join("QualitySettings.asset");
    fs::write(&path, QUALITY_SETTINGS_YAML)?;

    let parsed = parse_quality_settings(&path)?;
    assert_eq!(parsed, Some(expected_quality_settings()));

    let stream = unity_yaml::parse_file(&path)?;
    assert_eq!(
        parse_quality_settings_stream(&stream),
        Ok(expected_quality_settings())
    );

    let settings = parsed.ok_or("missing quality settings")?;
    let standalone = settings.effective_quality("Standalone", "Standalone");
    assert_eq!(
        standalone.effective_value, "Low",
        "PRJ-007 Outcome 10: the per-target quality default of Standalone names its level"
    );
    assert_eq!(standalone.target_group_id, "Standalone");
    assert_eq!(standalone.target_name, "Standalone");
    assert_eq!(
        standalone.source,
        SettingValueSource::TargetOverride,
        "PRJ-007 Outcome 10: a per-target quality default is reported as a target override"
    );
    assert_eq!(
        standalone.boundary,
        SettingBoundary::NextPlaySession,
        "PRJ-007 Outcome 10: the quality level of a target applies at the next play session (REN-003)"
    );

    let android = settings.effective_quality("Android", "Android");
    assert_eq!(android.effective_value, "High");
    assert_eq!(android.source, SettingValueSource::TargetOverride);

    let ios = settings.effective_quality("iOS", "iOS");
    assert_eq!(
        ios.effective_value, "High",
        "PRJ-007 Outcome 10: a target without a quality default falls back to the project quality"
    );
    assert_eq!(
        ios.source,
        SettingValueSource::Default,
        "PRJ-007 Outcome 10: the project quality is reported as the default source"
    );
    assert_eq!(ios.boundary, SettingBoundary::NextPlaySession);

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn prj007_quality_settings_round_trips_byte_exactly_and_preserves_unknown_keys() -> TestResult {
    let root = temp_settings_dir("quality_round_trip")?;
    let path = root.join("QualitySettings.asset");
    fs::write(&path, QUALITY_SETTINGS_YAML)?;

    let parsed = unity_yaml::parse(QUALITY_SETTINGS_YAML)?;
    assert_eq!(
        scene_save::emit_stream(&parsed),
        QUALITY_SETTINGS_YAML,
        "PRJ-007 Outcome 10: an unchanged QualitySettings file re-emits byte for byte"
    );
    scene_save::save_stream(&path, &parsed)?;
    assert_eq!(
        fs::read_to_string(&path)?,
        QUALITY_SETTINGS_YAML,
        "PRJ-007 Outcome 10: saving an unchanged QualitySettings file keeps every byte"
    );

    let mut stream = unity_yaml::parse_file(&path)?;
    assert!(set_settings_scalar(
        &mut stream.documents[0].root,
        &["QualitySettings", "m_CurrentQuality"],
        "0"
    ));
    scene_save::save_stream(&path, &stream)?;

    let reloaded = parse_quality_settings(&path)?.ok_or("missing quality settings")?;
    assert_eq!(reloaded.current_quality, 0);
    assert_eq!(
        reloaded.effective_quality("iOS", "iOS").effective_value,
        "Low",
        "PRJ-007 Outcome 10: the saved quality default is the effective value for the target"
    );
    assert_eq!(
        reloaded.effective_quality("iOS", "iOS").source,
        SettingValueSource::Default
    );

    let reloaded_stream = unity_yaml::parse_file(&path)?;
    let aggregate = unitas_asset::project_settings::load_project_settings(&root)?;
    assert_eq!(
        aggregate
            .effective_quality("Standalone", "Standalone")
            .effective_value,
        "Low",
        "PRJ-007 Outcome 10: the loaded project settings report the per-target quality level"
    );
    assert_eq!(
        aggregate
            .effective_quality("Standalone", "Standalone")
            .source,
        SettingValueSource::TargetOverride
    );
    assert_eq!(
        aggregate.effective_quality("iOS", "iOS").source,
        SettingValueSource::Default
    );
    let level = reloaded_stream.documents[0]
        .root
        .get("QualitySettings")
        .and_then(|body| body.get("m_QualitySettings"))
        .and_then(Node::as_sequence)
        .and_then(|levels| levels.first())
        .ok_or("missing quality level")?;
    assert_eq!(
        level
            .get("unrecognizedVendorQualityKey")
            .and_then(Node::as_str),
        Some("retained_verbatim"),
        "PRJ-007 Outcome 22: a quality key no Unity 2022.3 level defines survives the save"
    );
    assert_eq!(
        level
            .get("shadowCascade4Split")
            .and_then(|node| node.get("z"))
            .and_then(Node::as_str),
        Some("0.46666667"),
        "PRJ-007 Outcome 22: a quality setting the model does not represent keeps its on-disk form"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn quality_settings_handles_missing_file_missing_document_and_invalid_level_index() -> TestResult {
    let missing = settings_dir().join("MissingQualitySettings.asset");
    assert_eq!(parse_quality_settings(&missing), Ok(None));

    let empty = unity_yaml::parse("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n")?;
    assert_eq!(
        parse_quality_settings_stream(&empty),
        Err(ProjectSettingsError::MissingDocument("QualitySettings"))
    );

    let out_of_range = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!47 &1\nQualitySettings:\n  m_CurrentQuality: 7\n  m_QualitySettings:\n  - name: Low\n",
    )?;
    assert_eq!(
        parse_quality_settings_stream(&out_of_range),
        Err(ProjectSettingsError::MissingDocument("QualitySettings"))
    );

    let without_levels = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!47 &1\nQualitySettings:\n  m_CurrentQuality: 0\n",
    )?;
    assert_eq!(
        parse_quality_settings_stream(&without_levels),
        Err(ProjectSettingsError::MissingDocument("QualitySettings"))
    );

    Ok(())
}

#[test]
fn sample_quality_settings_asset_is_unity_2022_3_on_disk_and_reports_per_target_levels(
) -> TestResult {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/roll-a-ball/ProjectSettings/QualitySettings.asset");
    let text = fs::read_to_string(&path)?;
    let parsed = unity_yaml::parse(&text)?;
    assert_eq!(
        scene_save::emit_stream(&parsed),
        text,
        "PRJ-007 Outcome 10: the sample QualitySettings file is byte identical to its re-emitted form"
    );

    let settings = parse_quality_settings(&path)?.ok_or("missing sample quality settings")?;
    assert_eq!(settings.levels.len(), 3);
    assert_eq!(settings.levels[0].name, "Low");
    assert_eq!(settings.levels[1].name, "Medium");
    assert_eq!(settings.levels[2].name, "High");
    assert_eq!(settings.current_quality, 2);

    let standalone = settings.effective_quality("Standalone", "Standalone");
    assert_eq!(standalone.effective_value, "High");
    assert_eq!(standalone.source, SettingValueSource::TargetOverride);
    assert_eq!(standalone.boundary, SettingBoundary::NextPlaySession);

    let android = settings.effective_quality("Android", "Android");
    assert_eq!(android.effective_value, "Medium");
    assert_eq!(android.source, SettingValueSource::TargetOverride);

    let server = settings.effective_quality("Server", "Server");
    assert_eq!(server.effective_value, "Low");
    assert_eq!(server.source, SettingValueSource::TargetOverride);

    Ok(())
}

#[test]
fn malformed_quality_settings_file_reports_a_diagnostic_naming_the_file() -> TestResult {
    let root = temp_settings_dir("quality_malformed")?;
    let path = root.join("QualitySettings.asset");

    fs::write(
        &path,
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!47 &1\nQualitySettingsTypo:\n  m_CurrentQuality: 0\n  m_QualitySettings:\n  - name: Low\n",
    )?;
    let error = unitas_asset::project_settings::load_project_settings(&root)
        .err()
        .ok_or("expected a diagnostic for a malformed QualitySettings file")?;
    assert!(
        matches!(&error, ProjectSettingsError::MalformedFile { .. }),
        "PRJ-007 Outcome 10: a malformed settings file reports a file naming diagnostic"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "{}: stream contains no QualitySettings document",
            path.display()
        ),
        "PRJ-007 Outcome 10: the diagnostic names the malformed QualitySettings file"
    );

    fs::write(
        &path,
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!47 &1\nQualitySettings:\n  m_CurrentQuality: 9\n  m_QualitySettings:\n  - name: Low\n",
    )?;
    let error = unitas_asset::project_settings::load_project_settings(&root)
        .err()
        .ok_or("expected a diagnostic for an out of range quality index")?;
    assert!(
        error
            .to_string()
            .contains(&path.to_string_lossy().to_string()),
        "PRJ-007 Outcome 10: the diagnostic names the file with the out of range quality index: {error}"
    );

    fs::write(&path, "%YAML 1.1\nnot a unity settings document\n")?;
    let error = unitas_asset::project_settings::load_project_settings(&root)
        .err()
        .ok_or("expected a diagnostic for an unparseable QualitySettings file")?;
    assert_eq!(
        error.to_string(),
        format!(
            "{}: line 2: unexpected content not a unity settings document",
            path.display()
        ),
        "PRJ-007 Outcome 10: the diagnostic names the unparseable QualitySettings file"
    );

    fs::remove_file(&path)?;
    let settings = unitas_asset::project_settings::load_project_settings(&root)?;
    assert_eq!(
        settings.quality,
        QualitySettings::default(),
        "PRJ-007 Outcome 10: a missing QualitySettings file keeps the Unity-like silent default"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn parses_all_project_settings_families_and_round_trips_edits() -> TestResult {
    let project_root = temp_settings_dir("all_families")?;
    let root = project_root.join("ProjectSettings");
    fs::create_dir_all(&root)?;

    let time_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_FixedTimestep: 0.02\n  m_MaximumAllowedTimestep: 0.33333334\n  m_TimeScale: 1\n  unknownTimeField: preserved_time\n";
    let dynamics_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nPhysicsManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 13\n  m_Gravity: {x: 0, y: -9.81, z: 0}\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n  unknownPhysicsField: preserved_physics\n";
    let physics2d_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!19 &1\nPhysics2DSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 5\n  m_Gravity: {x: 0, y: -9.81}\n  m_LayerCollisionMatrix: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\n  unknownPhysics2DField: preserved_physics2d\n";
    let input_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!13 &1\nInputManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_Axes:\n  - serializedVersion: 3\n    m_Name: Jump\n    negativeButton:\n    positiveButton: space\n    altNegativeButton:\n    altPositiveButton:\n    gravity: 1000\n    dead: 0.001\n    sensitivity: 1000\n    snap: 0\n    invert: 0\n    type: 0\n    axis: 0\n  unknownInputField: preserved_input\n";
    let player_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!129 &1\nPlayerSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 26\n  companyName: UnitasStudio\n  productName: AllFamiliesGame\n  bundleVersion: 1.5.0\n  m_ActiveColorSpace: 1\n  scriptingDefineSymbols:\n    default: BASE_DEF\n    1: STANDALONE_DEF\n  unknownPlayerField: preserved_player\n";
    let tag_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!78 &1\nTagManager:\n  serializedVersion: 2\n  tags:\n  - Pickup\n  - Hazard\n  layers:\n  - Default\n  - TransparentFX\n  - Ignore Raycast\n  - \n  - Water\n  - UI\n  - \n  - \n  - PlayerLayer\n  m_SortingLayers:\n  - name: Default\n    uniqueID: 0\n    locked: 0\n  - name: Foreground\n    uniqueID: 42\n    locked: 1\n  unknownTagField: preserved_tag\n";
    let graphics_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!30 &1\nGraphicsSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 15\n  m_CustomRenderPipeline: {fileID: 11400000, guid: a1b2c3d4e5f60718293a4b5c6d7e8f90, type: 2}\n  m_TransparencySortMode: 2\n  m_AlwaysIncludedShaders:\n  - {fileID: 48, guid: 0000000000000000f000000000000000, type: 0}\n  - {fileID: 10753, guid: 0000000000000000f000000000000000, type: 0}\n  m_TierSettings:\n  - m_BuildTarget: 1\n    m_Tier: 0\n  unknownGraphicsField: preserved_graphics\n";
    let audio_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!89 &1\nAudioManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_Volume: 0.75\n  Rolloff Scale: 1.25\n  Doppler Factor: 0.5\n  Default Speaker Mode: 2\n  m_DSPBufferSize: 512\n  m_VirtualVoiceCount: 256\n  m_RealVoiceCount: 16\n  m_DisableAudio: 0\n  unknownAudioField: preserved_audio\n";
    let nav_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!126 &1\nNavMeshAreas:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  areas:\n  - name: Walkable\n    cost: 1\n  - name: Not Walkable\n    cost: 1\n  - name: Mud\n    cost: 3.5\n  unknownNavField: preserved_nav\n";
    let editor_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!159 &1\nEditorSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 9\n  m_SerializationMode: 2\n  m_DefaultBehaviorMode: 1\n  m_LineEndingsForNewScripts: 2\n  unknownEditorField: preserved_editor\n";
    let mono_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!116 &1\nMonoManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_ExecutionOrder:\n    11111111222222223333333344444444: -100\n    55555555666666667777777788888888: 50\n  unknownMonoField: preserved_mono\n";
    let preset_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1386491679 &1\nPresetManager:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_DefaultPresets:\n  - type: 1\n    presetList: []\n  - type: 2\n    presetList: []\n  unknownPresetField: preserved_preset\n";
    let pkg_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &1\nPackageManagerSettings:\n  m_ObjectHideFlags: 0\n  m_EnablePreReleasePackages: 1\n  m_ScopedRegistries:\n  - m_Name: UnitasRegistry\n    m_Url: \"https://packages.unitas.example\"\n    m_Scopes:\n    - com.unitas\n  unknownPkgField: preserved_pkg\n";
    let vcs_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!890905787 &1\nVersionControlSettings:\n  m_ObjectHideFlags: 0\n  m_Mode: Visible Meta Files\n  unknownVcsField: preserved_vcs\n";
    let build_raw = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1045 &1\nEditorBuildSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_Scenes:\n  - enabled: 1\n    path: Assets/Scenes/Main.unity\n    guid: 5ca1ab1e000000000000000000000001\n  - enabled: 0\n    path: Assets/Scenes/Bonus.unity\n    guid: 5ca1ab1e000000000000000000000002\n  m_configObjects: {}\n  unknownBuildField: preserved_build\n";

    let files = [
        ("TimeManager.asset", time_raw),
        ("DynamicsManager.asset", dynamics_raw),
        ("Physics2DSettings.asset", physics2d_raw),
        ("InputManager.asset", input_raw),
        ("QualitySettings.asset", QUALITY_SETTINGS_YAML),
        ("ProjectSettings.asset", player_raw),
        ("TagManager.asset", tag_raw),
        ("GraphicsSettings.asset", graphics_raw),
        ("AudioManager.asset", audio_raw),
        ("NavMeshAreas.asset", nav_raw),
        ("EditorSettings.asset", editor_raw),
        ("MonoManager.asset", mono_raw),
        ("PresetManager.asset", preset_raw),
        ("PackageManagerSettings.asset", pkg_raw),
        ("VersionControlSettings.asset", vcs_raw),
        ("EditorBuildSettings.asset", build_raw),
    ];

    for (name, raw) in files {
        let path = root.join(name);
        fs::write(&path, raw)?;
        let parsed = unity_yaml::parse(raw)?;
        let out = scene_save::emit_stream(&parsed);
        assert_eq!(out, raw);
    }

    let tag_path = root.join("TagManager.asset");
    let mut tag_settings = TagManagerSettings::load(&tag_path)?.ok_or("missing TagManager")?;
    assert_eq!(tag_settings.emit(), tag_raw);
    tag_settings.save(&tag_path)?;
    assert_eq!(fs::read_to_string(&tag_path)?, tag_raw);
    assert_eq!(
        tag_settings.tags(),
        vec![String::from("Pickup"), String::from("Hazard")]
    );
    assert_eq!(tag_settings.layers()[0], "Default");
    assert_eq!(tag_settings.layers()[8], "PlayerLayer");
    assert_eq!(
        tag_settings.sorting_layers(),
        vec![
            SortingLayerEntry {
                name: String::from("Default"),
                unique_id: 0,
                locked: false,
            },
            SortingLayerEntry {
                name: String::from("Foreground"),
                unique_id: 42,
                locked: true,
            },
        ]
    );
    tag_settings.add_tag("Checkpoint");
    tag_settings.set_layer(9, "EnemyLayer")?;
    tag_settings.save(&tag_path)?;
    let reloaded_tags = parse_tag_manager(&tag_path)?.ok_or("missing reloaded TagManager")?;
    assert_eq!(
        reloaded_tags.tags(),
        vec![
            String::from("Pickup"),
            String::from("Hazard"),
            String::from("Checkpoint")
        ]
    );
    assert_eq!(reloaded_tags.layers()[9], "EnemyLayer");
    let reloaded_tag_stream = unity_yaml::parse_file(&tag_path)?;
    assert_eq!(
        reloaded_tag_stream.documents[0]
            .root
            .get("TagManager")
            .and_then(|n| n.get("unknownTagField"))
            .and_then(Node::as_str),
        Some("preserved_tag")
    );

    let graphics_path = root.join("GraphicsSettings.asset");
    let mut graphics = GraphicsSettings::load(&graphics_path)?.ok_or("missing GraphicsSettings")?;
    assert_eq!(graphics.emit(), graphics_raw);
    graphics.save(&graphics_path)?;
    assert_eq!(fs::read_to_string(&graphics_path)?, graphics_raw);
    assert_eq!(
        graphics.custom_render_pipeline_guid(),
        Some(String::from("a1b2c3d4e5f60718293a4b5c6d7e8f90"))
    );
    assert_eq!(graphics.transparency_sort_mode(), 2);
    assert_eq!(graphics.always_included_shader_count(), 2);
    assert_eq!(graphics.tier_settings_count, 1);
    graphics.set_transparency_sort_mode(1)?;
    graphics.save(&graphics_path)?;
    let reloaded_graphics =
        parse_graphics_settings(&graphics_path)?.ok_or("missing reloaded GraphicsSettings")?;
    assert_eq!(reloaded_graphics.transparency_sort_mode(), 1);
    assert_eq!(reloaded_graphics.always_included_shader_count(), 2);
    let reloaded_graphics_stream = unity_yaml::parse_file(&graphics_path)?;
    assert_eq!(
        reloaded_graphics_stream.documents[0]
            .root
            .get("GraphicsSettings")
            .and_then(|n| n.get("unknownGraphicsField"))
            .and_then(Node::as_str),
        Some("preserved_graphics")
    );

    let player_path = root.join("ProjectSettings.asset");
    assert_eq!(
        parse_color_space(&player_path)?,
        Some(ColorSpaceSetting::Linear)
    );
    let mut player_settings =
        PlayerSettings::load(&player_path)?.ok_or("missing PlayerSettings")?;
    assert_eq!(
        player_settings.active_color_space(),
        ColorSpaceSetting::Linear
    );
    player_settings.set_active_color_space(ColorSpaceSetting::Gamma);
    assert_eq!(
        player_settings.active_color_space(),
        ColorSpaceSetting::Gamma
    );
    let mut player_stream = unity_yaml::parse_file(&player_path)?;
    player_settings.apply_to_stream(&mut player_stream)?;
    scene_save::save_stream(&player_path, &player_stream)?;
    assert_eq!(
        parse_color_space(&player_path)?,
        Some(ColorSpaceSetting::Gamma)
    );
    let reloaded_player = parse_player_settings(&player_path)?.ok_or("missing reloaded player")?;
    assert_eq!(
        reloaded_player.active_color_space(),
        ColorSpaceSetting::Gamma
    );
    let reloaded_player_stream = unity_yaml::parse_file(&player_path)?;
    assert_eq!(
        reloaded_player_stream.documents[0]
            .root
            .get("PlayerSettings")
            .and_then(|n| n.get("unknownPlayerField"))
            .and_then(Node::as_str),
        Some("preserved_player")
    );

    let audio_path = root.join("AudioManager.asset");
    let mut audio = AudioManagerSettings::load(&audio_path)?.ok_or("missing AudioManager")?;
    assert_eq!(audio.emit(), audio_raw);
    audio.save(&audio_path)?;
    assert_eq!(fs::read_to_string(&audio_path)?, audio_raw);
    assert!((audio.volume() - 0.75).abs() < 1e-12);
    assert!((audio.rolloff_scale() - 1.25).abs() < 1e-12);
    assert!((audio.doppler_factor() - 0.5).abs() < 1e-12);
    assert_eq!(audio.default_speaker_mode(), 2);
    assert_eq!(audio.dsp_buffer_size(), 512);
    assert_eq!(audio.virtual_voice_count(), 256);
    assert_eq!(audio.real_voice_count(), 16);
    assert!(!audio.disable_audio());
    audio.set_volume(0.4)?;
    audio.save(&audio_path)?;
    let reloaded_audio = parse_audio_manager(&audio_path)?.ok_or("missing reloaded audio")?;
    assert!((reloaded_audio.volume() - 0.4).abs() < 1e-12);
    let reloaded_audio_stream = unity_yaml::parse_file(&audio_path)?;
    assert_eq!(
        reloaded_audio_stream.documents[0]
            .root
            .get("AudioManager")
            .and_then(|n| n.get("unknownAudioField"))
            .and_then(Node::as_str),
        Some("preserved_audio")
    );

    let nav_path = root.join("NavMeshAreas.asset");
    let mut nav = NavMeshAreaSettings::load(&nav_path)?.ok_or("missing NavMeshAreas")?;
    assert_eq!(nav.emit(), nav_raw);
    nav.save(&nav_path)?;
    assert_eq!(fs::read_to_string(&nav_path)?, nav_raw);
    assert_eq!(
        nav.areas(),
        vec![
            NavMeshAreaEntry {
                name: String::from("Walkable"),
                cost: 1.0,
            },
            NavMeshAreaEntry {
                name: String::from("Not Walkable"),
                cost: 1.0,
            },
            NavMeshAreaEntry {
                name: String::from("Mud"),
                cost: 3.5,
            },
        ]
    );
    nav.set_area_cost(2, 5.25)?;
    nav.save(&nav_path)?;
    let reloaded_nav = parse_nav_mesh_areas(&nav_path)?.ok_or("missing reloaded nav")?;
    assert!((reloaded_nav.areas()[2].cost - 5.25).abs() < 1e-12);
    let reloaded_nav_stream = unity_yaml::parse_file(&nav_path)?;
    assert_eq!(
        reloaded_nav_stream.documents[0]
            .root
            .get("NavMeshAreas")
            .and_then(|n| n.get("unknownNavField"))
            .and_then(Node::as_str),
        Some("preserved_nav")
    );

    let editor_path = root.join("EditorSettings.asset");
    let mut editor = EditorSettings::load(&editor_path)?.ok_or("missing EditorSettings")?;
    assert_eq!(editor.emit(), editor_raw);
    editor.save(&editor_path)?;
    assert_eq!(fs::read_to_string(&editor_path)?, editor_raw);
    assert_eq!(editor.serialization_mode(), 2);
    assert_eq!(editor.default_behavior_mode(), 1);
    assert_eq!(editor.line_endings_for_new_scripts(), 2);
    editor.set_serialization_mode(1)?;
    editor.save(&editor_path)?;
    let reloaded_editor = parse_editor_settings(&editor_path)?.ok_or("missing reloaded editor")?;
    assert_eq!(reloaded_editor.serialization_mode(), 1);
    let reloaded_editor_stream = unity_yaml::parse_file(&editor_path)?;
    assert_eq!(
        reloaded_editor_stream.documents[0]
            .root
            .get("EditorSettings")
            .and_then(|n| n.get("unknownEditorField"))
            .and_then(Node::as_str),
        Some("preserved_editor")
    );

    let mono_path = root.join("MonoManager.asset");
    let mut mono = MonoManagerSettings::load(&mono_path)?.ok_or("missing MonoManager")?;
    assert_eq!(mono.emit(), mono_raw);
    mono.save(&mono_path)?;
    assert_eq!(fs::read_to_string(&mono_path)?, mono_raw);
    assert_eq!(
        mono.execution_orders(),
        vec![
            (String::from("11111111222222223333333344444444"), -100),
            (String::from("55555555666666667777777788888888"), 50),
        ]
    );
    mono.set_execution_order("55555555666666667777777788888888", 120);
    mono.set_execution_order("99999999aaaabbbbccccddddeeeeffff", -25);
    mono.save(&mono_path)?;
    let reloaded_mono = parse_mono_manager(&mono_path)?.ok_or("missing reloaded mono")?;
    assert_eq!(
        reloaded_mono.execution_orders(),
        vec![
            (String::from("11111111222222223333333344444444"), -100),
            (String::from("55555555666666667777777788888888"), 120),
            (String::from("99999999aaaabbbbccccddddeeeeffff"), -25),
        ]
    );
    let reloaded_mono_stream = unity_yaml::parse_file(&mono_path)?;
    assert_eq!(
        reloaded_mono_stream.documents[0]
            .root
            .get("MonoManager")
            .and_then(|n| n.get("unknownMonoField"))
            .and_then(Node::as_str),
        Some("preserved_mono")
    );

    let preset_path = root.join("PresetManager.asset");
    let preset = PresetManagerSettings::load(&preset_path)?.ok_or("missing PresetManager")?;
    assert_eq!(preset.emit(), preset_raw);
    preset.save(&preset_path)?;
    assert_eq!(fs::read_to_string(&preset_path)?, preset_raw);
    assert_eq!(preset.default_preset_type_count(), 2);
    let reloaded_preset = parse_preset_manager(&preset_path)?.ok_or("missing reloaded preset")?;
    assert_eq!(reloaded_preset.default_preset_type_count(), 2);

    let pkg_path = root.join("PackageManagerSettings.asset");
    let mut pkg =
        PackageManagerSettings::load(&pkg_path)?.ok_or("missing PackageManagerSettings")?;
    assert_eq!(pkg.emit(), pkg_raw);
    pkg.save(&pkg_path)?;
    assert_eq!(fs::read_to_string(&pkg_path)?, pkg_raw);
    assert!(pkg.enable_pre_release_packages());
    assert_eq!(pkg.scoped_registries_count(), 1);
    pkg.set_enable_pre_release_packages(false)?;
    pkg.save(&pkg_path)?;
    let reloaded_pkg =
        parse_package_manager_settings(&pkg_path)?.ok_or("missing reloaded package manager")?;
    assert!(!reloaded_pkg.enable_pre_release_packages());
    assert_eq!(reloaded_pkg.scoped_registries_count(), 1);
    let reloaded_pkg_stream = unity_yaml::parse_file(&pkg_path)?;
    assert_eq!(
        reloaded_pkg_stream.documents[0]
            .root
            .get("PackageManagerSettings")
            .and_then(|n| n.get("unknownPkgField"))
            .and_then(Node::as_str),
        Some("preserved_pkg")
    );

    let vcs_path = root.join("VersionControlSettings.asset");
    let mut vcs =
        VersionControlSettings::load(&vcs_path)?.ok_or("missing VersionControlSettings")?;
    assert_eq!(vcs.emit(), vcs_raw);
    vcs.save(&vcs_path)?;
    assert_eq!(fs::read_to_string(&vcs_path)?, vcs_raw);
    assert_eq!(vcs.mode()?, "Visible Meta Files");
    vcs.set_mode("Hidden Meta Files")?;
    vcs.save(&vcs_path)?;
    let reloaded_vcs =
        parse_version_control_settings(&vcs_path)?.ok_or("missing reloaded version control")?;
    assert_eq!(reloaded_vcs.mode()?, "Hidden Meta Files");
    let reloaded_vcs_stream = unity_yaml::parse_file(&vcs_path)?;
    assert_eq!(
        reloaded_vcs_stream.documents[0]
            .root
            .get("VersionControlSettings")
            .and_then(|n| n.get("unknownVcsField"))
            .and_then(Node::as_str),
        Some("preserved_vcs")
    );

    let build_path = root.join("EditorBuildSettings.asset");
    let mut build = EditorBuildSettings::load(&build_path)?.ok_or("missing EditorBuildSettings")?;
    assert_eq!(build.emit(), build_raw);
    build.save(&build_path)?;
    assert_eq!(fs::read_to_string(&build_path)?, build_raw);
    assert_eq!(
        build.scenes(),
        vec![
            BuildSceneEntry {
                enabled: true,
                path: String::from("Assets/Scenes/Main.unity"),
                guid: String::from("5ca1ab1e000000000000000000000001"),
            },
            BuildSceneEntry {
                enabled: false,
                path: String::from("Assets/Scenes/Bonus.unity"),
                guid: String::from("5ca1ab1e000000000000000000000002"),
            },
        ]
    );
    build.set_scene_enabled("Assets/Scenes/Bonus.unity", true)?;
    build.save(&build_path)?;
    let reloaded_build =
        parse_editor_build_settings(&build_path)?.ok_or("missing reloaded build settings")?;
    assert!(reloaded_build.scenes()[1].enabled);
    let reloaded_build_stream = unity_yaml::parse_file(&build_path)?;
    assert_eq!(
        reloaded_build_stream.documents[0]
            .root
            .get("EditorBuildSettings")
            .and_then(|n| n.get("unknownBuildField"))
            .and_then(Node::as_str),
        Some("preserved_build")
    );

    let loaded = load_project_settings(&project_root)?;
    assert_eq!(loaded.fixed_timestep(), 0.02);
    assert_eq!(loaded.gravity, [0.0, -9.81, 0.0]);
    assert_eq!(loaded.gravity_2d, [0.0, -9.81]);
    assert_eq!(
        loaded.identity.product_name.as_deref(),
        Some("AllFamiliesGame")
    );
    assert_eq!(
        loaded.identity.active_color_space(),
        ColorSpaceSetting::Gamma
    );
    assert_eq!(loaded.color_space, ColorSpaceSetting::Gamma);
    assert_eq!(loaded.quality.levels.len(), 2);

    let loaded_tags = loaded.tag_manager.ok_or("expected Some(tag_manager)")?;
    assert_eq!(loaded_tags.tags().len(), 3);
    assert_eq!(loaded_tags.layers()[9], "EnemyLayer");

    let loaded_graphics = loaded.graphics.ok_or("expected Some(graphics)")?;
    assert_eq!(loaded_graphics.transparency_sort_mode(), 1);
    assert_eq!(loaded_graphics.always_included_shader_count(), 2);

    let loaded_audio = loaded.audio.ok_or("expected Some(audio)")?;
    assert!((loaded_audio.volume() - 0.4).abs() < 1e-12);

    let loaded_nav = loaded
        .nav_mesh_areas
        .ok_or("expected Some(nav_mesh_areas)")?;
    assert!((loaded_nav.areas()[2].cost - 5.25).abs() < 1e-12);

    let loaded_editor = loaded.editor.ok_or("expected Some(editor)")?;
    assert_eq!(loaded_editor.serialization_mode(), 1);

    let loaded_mono = loaded.mono_manager.ok_or("expected Some(mono_manager)")?;
    assert_eq!(loaded_mono.execution_orders().len(), 3);

    let loaded_preset = loaded
        .preset_manager
        .ok_or("expected Some(preset_manager)")?;
    assert_eq!(loaded_preset.default_preset_type_count(), 2);

    let loaded_pkg = loaded
        .package_manager
        .ok_or("expected Some(package_manager)")?;
    assert!(!loaded_pkg.enable_pre_release_packages());
    assert_eq!(loaded_pkg.scoped_registries_count(), 1);

    let loaded_vcs = loaded
        .version_control
        .ok_or("expected Some(version_control)")?;
    assert_eq!(loaded_vcs.mode()?, "Hidden Meta Files");

    let loaded_build = loaded.editor_build.ok_or("expected Some(editor_build)")?;
    assert_eq!(loaded_build.scenes().len(), 2);
    assert!(loaded_build.scenes()[1].enabled);

    fs::remove_dir_all(&project_root)?;
    Ok(())
}
