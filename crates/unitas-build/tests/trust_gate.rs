use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::trust::{TrustOrigin, TrustStore};
use unitas_build::config::BuildConfiguration;
use unitas_build::report::BuildReport;
use unitas_build::targets::BuildTarget;
use unitas_build::BUILD_REPORT_FILENAME;

type TestResult = Result<(), Box<dyn Error>>;

const TRUST_REQUIRED_EXIT_CODE: i32 = 3;

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn bundle_with_project_report(root: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
    let project = root.join("project");
    fs::create_dir_all(project.join("Assets").join("Scenes"))?;
    let scene = project.join("Assets").join("Scenes").join("Main.unity");
    fs::write(&scene, "%YAML 1.1\n")?;
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle)?;
    let config = BuildConfiguration::new(
        vec![scene],
        "Unitas",
        "TrustGate",
        "com.unitas.trustgate",
        "1.0.0",
        BuildTarget::host(),
    );
    let report = BuildReport::succeeded(
        &config,
        1,
        bundle.clone(),
        bundle.join("unitas-player"),
        0,
        Vec::new(),
        Vec::new(),
    );
    report.write_to_file(&bundle.join(BUILD_REPORT_FILENAME))?;
    Ok((bundle, project))
}

fn run_player(bundle: &Path, store_path: &Path) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_unitas-player"))
        .arg(bundle)
        .arg("--frames")
        .arg("1")
        .env("UNITAS_TRUST_STORE", store_path)
        .output()
}

#[test]
fn untrusted_project_cannot_run_its_code() -> TestResult {
    let root = temp_dir("player-trust-gate")?;
    let (bundle, project) = bundle_with_project_report(&root)?;
    let store_path = root.join("user-trust.json");
    let store = TrustStore::new(&store_path);

    let refused = run_player(&bundle, &store_path)?;
    let refused_stderr = String::from_utf8_lossy(&refused.stderr);
    assert_eq!(
        refused.status.code(),
        Some(TRUST_REQUIRED_EXIT_CODE),
        "an untrusted project must be refused with the trust exit code, stderr: {refused_stderr}"
    );
    assert!(
        refused_stderr.contains("trust-required"),
        "the refusal must name the trust requirement, stderr: {refused_stderr}"
    );

    store.trust_project(&project.to_string_lossy(), TrustOrigin::Person)?;

    let permitted = run_player(&bundle, &store_path)?;
    let permitted_stderr = String::from_utf8_lossy(&permitted.stderr);
    assert_ne!(
        permitted.status.code(),
        Some(TRUST_REQUIRED_EXIT_CODE),
        "a trusted project must not be refused for trust, stderr: {permitted_stderr}"
    );
    assert!(
        !permitted_stderr.contains("trust-required"),
        "a trusted project must not report a trust requirement, stderr: {permitted_stderr}"
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}
