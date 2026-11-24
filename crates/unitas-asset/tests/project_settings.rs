use std::error::Error;
use std::path::PathBuf;

use unitas_asset::{
    parse_dynamics_manager, parse_dynamics_manager_stream, parse_input_manager,
    parse_input_manager_stream, parse_time_manager, parse_time_manager_stream, unity_yaml,
    DynamicsSettings, InputAxis, InputManagerSettings, ProjectSettingsError, TimeSettings,
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
