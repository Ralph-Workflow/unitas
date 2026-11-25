use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_build::player::{render_failure, window_failure, STARTUP_FAILURE_EXIT_CODE};
use unitas_build::{build_project, BuildConfiguration, PlayerStartupError};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn player_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_unitas-player"))
}

fn scratch_dir(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "unitas_windowed_startup_failure_{label}_{}_{id}",
        std::process::id()
    ))
}

fn fixture_scene(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn windowed_launch_without_a_bundle_manifest_fails_loudly() -> Result<(), Box<dyn std::error::Error>>
{
    let workspace = scratch_dir("missing_manifest");
    let _ = fs::remove_dir_all(&workspace);
    fs::create_dir_all(&workspace)?;
    let bundle_dir = workspace.join("bundle");
    fs::create_dir_all(&bundle_dir)?;

    let output = Command::new(player_binary())
        .arg(&bundle_dir)
        .arg("--windowed")
        .output()?;

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        output.status.code(),
        Some(i32::from(STARTUP_FAILURE_EXIT_CODE))
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("unitas-player startup failed"),
        "the failed launch did not report a startup failure: {stderr}"
    );
    assert!(
        stderr.contains("failed to load bundle manifest"),
        "the failed launch did not name the missing manifest: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        !stdout.contains("launched_product"),
        "the failed launch still reported a successful startup: {stdout}"
    );

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn windowed_launch_with_a_missing_startup_scene_fails_loudly(
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = scratch_dir("missing_scene");
    let _ = fs::remove_dir_all(&workspace);
    fs::create_dir_all(&workspace)?;
    let bundle_dir = workspace.join("bundle");

    let config = BuildConfiguration::for_host(
        vec![fixture_scene("scene_alpha.unity")],
        "StartupFailureGame",
    );
    let (staged, _report) = match build_project(&config, &bundle_dir, 10) {
        Ok(pair) => pair,
        Err(boxed) => return Err(Box::new(boxed.0)),
    };
    fs::remove_file(bundle_dir.join(&staged.manifest.startup_scene_staged_path))?;

    let output = Command::new(player_binary())
        .arg(&bundle_dir)
        .arg("--windowed")
        .output()?;

    assert_ne!(output.status.code(), Some(0));
    assert_eq!(
        output.status.code(),
        Some(i32::from(STARTUP_FAILURE_EXIT_CODE))
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("unitas-player startup failed"),
        "the failed launch did not report a startup failure: {stderr}"
    );
    assert!(
        stderr.contains("startup scene does not exist"),
        "the failed launch did not name the missing startup scene: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        !stdout.contains("launched_product"),
        "the failed launch still reported a successful startup: {stdout}"
    );

    fs::remove_dir_all(&workspace)?;
    Ok(())
}

#[test]
fn windowed_startup_failures_name_their_reason_and_exit_nonzero() {
    assert_ne!(STARTUP_FAILURE_EXIT_CODE, 0);

    let window_error = window_failure("window server unavailable");
    assert!(matches!(
        &window_error,
        PlayerStartupError::WindowFailed { .. }
    ));
    let window_text = window_error.to_string();
    assert!(
        window_text.contains("windowed player failed"),
        "the window failure did not name the windowed player stage: {window_text}"
    );
    assert!(
        window_text.contains("window server unavailable"),
        "the window failure did not name the reason: {window_text}"
    );

    let render_error = render_failure("no suitable GPU adapter");
    assert!(matches!(
        &render_error,
        PlayerStartupError::RenderFailed { .. }
    ));
    let render_text = render_error.to_string();
    assert!(
        render_text.contains("GPU renderer failed"),
        "the render failure did not name the renderer stage: {render_text}"
    );
    assert!(
        render_text.contains("no suitable GPU adapter"),
        "the render failure did not name the reason: {render_text}"
    );
}
