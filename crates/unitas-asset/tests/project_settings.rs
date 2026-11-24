use std::error::Error;
use std::path::PathBuf;

use unitas_asset::{
    parse_dynamics_manager, parse_dynamics_manager_stream, parse_time_manager,
    parse_time_manager_stream, unity_yaml, DynamicsSettings, ProjectSettingsError, TimeSettings,
    DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY,
};

type TestResult = Result<(), Box<dyn Error>>;

fn settings_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("project")
        .join("ProjectSettings")
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
