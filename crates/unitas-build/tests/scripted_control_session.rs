use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use unitas_build::bundle::{MANAGED_DIR_NAME, PLAYER_HOST_ASSEMBLY_FILENAME};
use unitas_build::config::BuildConfiguration;
use unitas_build::report::BuildReport;
use unitas_build::targets::BuildTarget;
use unitas_build::{BundleManifest, BUILD_REPORT_FILENAME, BUNDLE_MANIFEST_FILENAME};

type TestResult = Result<(), Box<dyn Error>>;

const TRUST_REQUIRED_EXIT_CODE: i32 = 3;

fn repository_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn has_managed_host() -> bool {
    let Some(root) = repository_root() else {
        return false;
    };
    let host_dll = root
        .join("bridge")
        .join("Unitas.PlayerHost")
        .join("bin")
        .join("Release")
        .join("net8.0")
        .join(PLAYER_HOST_ASSEMBLY_FILENAME);
    let staged_dll = root
        .join("target")
        .join("editor-test-bundle")
        .join(MANAGED_DIR_NAME)
        .join(PLAYER_HOST_ASSEMBLY_FILENAME);
    if !host_dll.is_file() && !staged_dll.is_file() {
        eprintln!(
            "skipping: no managed player host payload at '{}'",
            host_dll.display()
        );
        return false;
    }
    true
}

fn private_bundle(name: &str) -> Result<Option<(PathBuf, PathBuf)>, Box<dyn Error>> {
    if !has_managed_host() {
        return Ok(None);
    }
    let root = temp_dir(name)?;
    let bundle = root.join("bundle");
    unitas_build::sample::stage_marble_rush_bundle(&bundle)?;
    Ok(Some((root, bundle)))
}

#[test]
fn the_control_session_reports_the_coreclr_script_host() -> TestResult {
    let Some((root, bundle)) = private_bundle("scripted-control-session")? else {
        return Ok(());
    };
    let (line, status, stderr) = run_control_state_query(&root, &bundle)?;
    if line.trim().is_empty() {
        return Err(format!(
            "the control session answered no state request (exit code {:?}); stderr: {stderr}",
            status.code()
        )
        .into());
    }
    let payload: Value = serde_json::from_str(line.trim())?;
    assert_eq!(
        payload["ok"],
        Value::Bool(true),
        "the control session refused the state request: {payload}"
    );
    assert_eq!(
        payload["script_host"],
        Value::String(String::from("coreclr")),
        "the control session did not host the game's scripts on CoreCLR: {payload}"
    );
    assert_eq!(
        status.code(),
        Some(0),
        "the control session must exit cleanly once its stdin closes"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

fn run_control_state_query(
    root: &Path,
    bundle: &Path,
) -> Result<(String, std::process::ExitStatus, String), Box<dyn Error>> {
    let stderr_path = root.join("player-stderr.log");
    let stderr_file = fs::File::create(&stderr_path)?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_unitas-player"))
        .arg(bundle)
        .arg("--control-stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    {
        let mut stdin = child.stdin.take().ok_or("the player stdin was not piped")?;
        writeln!(stdin, "{{\"id\":1,\"cmd\":\"state\"}}")?;
        stdin.flush()?;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or("the player stdout was not piped")?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let status = child.wait()?;
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    Ok((line, status, stderr))
}

#[test]
fn an_untrusted_project_is_refused_on_the_control_path() -> TestResult {
    let Some((root, bundle)) = private_bundle("scripted-control-trust")? else {
        return Ok(());
    };

    let staged = root.join("untrusted-bundle");
    fs::create_dir_all(&staged)?;
    let manifest = BundleManifest::load_from_dir(&bundle)?;
    fs::copy(
        bundle.join(BUNDLE_MANIFEST_FILENAME),
        staged.join(BUNDLE_MANIFEST_FILENAME),
    )?;
    let scene = staged.join(&manifest.startup_scene_staged_path);
    fs::create_dir_all(scene.parent().ok_or("the startup scene has no parent")?)?;
    fs::copy(bundle.join(&manifest.startup_scene_staged_path), &scene)?;

    let config = BuildConfiguration::new(
        vec![scene],
        "Unitas",
        "ScriptedControl",
        "com.unitas.scriptedcontrol",
        "1.0.0",
        BuildTarget::host(),
    );
    let report = BuildReport::succeeded(
        &config,
        0,
        staged.clone(),
        staged.join("unitas-player"),
        0,
        Vec::new(),
        Vec::new(),
    );
    report.write_to_file(&staged.join(BUILD_REPORT_FILENAME))?;

    let output = Command::new(env!("CARGO_BIN_EXE_unitas-player"))
        .arg(&staged)
        .arg("--control-stdio")
        .env("UNITAS_TRUST_STORE", root.join("trust-store.json"))
        .stdin(Stdio::null())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(TRUST_REQUIRED_EXIT_CODE),
        "an untrusted project must be refused with the trust exit code: {stderr}"
    );
    assert!(
        stderr.contains("refused to run project code"),
        "the refusal must name the trust requirement: {stderr}"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
