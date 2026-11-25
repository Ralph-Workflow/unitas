use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
fn input_manager_handles_missing_file_and_m_type_aliases_and_rejects_missing_document(
) -> TestResult {
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

    fs::remove_dir_all(&root)?;
    Ok(())
}
