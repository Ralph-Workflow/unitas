use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project::Project;
use unitas_asset::trust::{
    BlockedVectorKind, TrustError, TrustOrigin, TrustOutcome, TrustRefusal, TrustStore,
};

type TestResult = Result<(), Box<dyn Error>>;

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("unitas-{name}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn project_dir(root: &Path) -> std::io::Result<PathBuf> {
    let project = root.join("project");
    fs::create_dir_all(&project)?;
    Ok(project)
}

fn path_text(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path.to_str().ok_or("path is not text")?.to_string())
}

#[test]
fn untrusted_project_is_restricted() -> TestResult {
    let root = temp_dir("trust-restricted")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));
    let project_path = path_text(&project)?;

    assert!(!store.is_trusted(&project_path));
    let capabilities = store.capabilities(&project_path);
    assert!(capabilities.restricted);
    assert!(!capabilities.may_run_code);
    assert!(!capabilities.may_fetch);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn trust_record_is_written_outside_the_project() -> TestResult {
    let root = temp_dir("trust-location")?;
    let project = project_dir(&root)?;
    let store_path = root.join("user-trust.json");
    assert!(!store_path.starts_with(&project));
    let store = TrustStore::new(&store_path);
    let project_path = path_text(&project)?;

    let outcome = store.trust_project(&project_path, TrustOrigin::Person)?;
    assert_eq!(outcome, TrustOutcome::Granted);
    assert!(store.is_trusted(&project_path));
    assert!(store_path.exists());
    assert!(!store_path.starts_with(&project));
    let project_entries = fs::read_dir(&project)?.count();
    assert_eq!(project_entries, 0);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn trust_grants_run_but_not_fetch_until_consent_is_given() -> TestResult {
    let root = temp_dir("trust-consent")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));
    let project_path = path_text(&project)?;

    store.trust_project(&project_path, TrustOrigin::Person)?;
    let capabilities = store.capabilities(&project_path);
    assert!(capabilities.may_run_code);
    assert!(!capabilities.may_fetch);
    assert!(!capabilities.restricted);

    let outcome = store.grant_fetch_consent(&project_path, TrustOrigin::Person)?;
    assert_eq!(outcome, TrustOutcome::Granted);
    let capabilities = store.capabilities(&project_path);
    assert!(capabilities.may_run_code);
    assert!(capabilities.may_fetch);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn consent_without_trust_does_not_enable_running_code() -> TestResult {
    let root = temp_dir("consent-only")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));
    let project_path = path_text(&project)?;

    store.grant_fetch_consent(&project_path, TrustOrigin::Person)?;
    assert!(!store.is_trusted(&project_path));
    let capabilities = store.capabilities(&project_path);
    assert!(!capabilities.may_run_code);
    assert!(!capabilities.may_fetch);
    assert!(capabilities.restricted);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn revoking_trust_restores_the_restricted_capability_set() -> TestResult {
    let root = temp_dir("trust-revoke")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));
    let project_path = path_text(&project)?;

    store.trust_project(&project_path, TrustOrigin::Person)?;
    store.grant_fetch_consent(&project_path, TrustOrigin::Person)?;
    assert!(store.capabilities(&project_path).may_run_code);

    store.revoke_trust(&project_path)?;
    assert!(!store.is_trusted(&project_path));
    let capabilities = store.capabilities(&project_path);
    assert!(capabilities.restricted);
    assert!(!capabilities.may_run_code);
    assert!(!capabilities.may_fetch);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn automation_requested_trust_grant_is_refused() -> TestResult {
    let root = temp_dir("trust-automation")?;
    let project = project_dir(&root)?;
    let store_path = root.join("user-trust.json");
    let store = TrustStore::new(&store_path);
    let project_path = path_text(&project)?;

    let outcome = store.request_trust_from_automation(&project_path);
    assert_eq!(
        outcome,
        TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant)
    );
    let outcome = store.trust_project(&project_path, TrustOrigin::Automation)?;
    assert_eq!(
        outcome,
        TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant)
    );
    assert!(!store.is_trusted(&project_path));
    assert!(!store_path.exists());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn a_store_inside_the_project_is_rejected() -> TestResult {
    let root = temp_dir("trust-inside")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(project.join("trust.json"));
    let project_path = path_text(&project)?;

    let error = match store.trust_project(&project_path, TrustOrigin::Person) {
        Err(error) => error,
        Ok(_) => return Err("expected the store inside the project to be rejected".into()),
    };
    assert!(matches!(error, TrustError::StoreInsideProject { .. }));
    assert!(!store.is_trusted(&project_path));
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn write_project_file(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

#[test]
fn untrusted_open_blocks_initialize_on_load_source_generator_and_git_fetch_while_assets_stay_inspectable(
) -> TestResult {
    let root = temp_dir("trust-vectors")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));

    write_project_file(
        &project.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.16f1\n",
    )?;
    write_project_file(
        &project.join("Assets/Scenes/Main.unity"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!29 &1\nScene:\n  m_Name: Main\n",
    )?;
    write_project_file(
        &project.join("Assets/Scenes/Main.unity.meta"),
        "fileFormatVersion: 2\nguid: 11111111111111111111111111111111\nNativeFormatImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Assets/Materials/Stone.mat"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: Stone\n",
    )?;
    write_project_file(
        &project.join("Assets/Materials/Stone.mat.meta"),
        "fileFormatVersion: 2\nguid: 22222222222222222222222222222222\nNativeFormatImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Assets/Editor/AutoRun.cs"),
        "using UnityEditor;\n[InitializeOnLoad]\npublic static class AutoRun {\n    static AutoRun() { System.IO.File.WriteAllText(\"pwned.txt\", \"ran\"); }\n}\n",
    )?;
    write_project_file(
        &project.join("Assets/Editor/AutoRun.cs.meta"),
        "fileFormatVersion: 2\nguid: 33333333333333333333333333333333\nMonoImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Assets/Analyzers/EvilGenerator.dll"),
        "MZ-fake-roslyn-generator",
    )?;
    write_project_file(
        &project.join("Assets/Analyzers/EvilGenerator.dll.meta"),
        "fileFormatVersion: 2\nguid: 44444444444444444444444444444444\nlabels:\n- RoslynAnalyzer\nPluginImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Packages/manifest.json"),
        r#"{
  "dependencies": {
    "com.example.remote-git": {
      "git": "https://example.invalid/evil.git",
      "hash": "0000000000000000000000000000000000000001"
    }
  }
}"#,
    )?;

    let (opened, report) = Project::open(&project, &store)?;

    assert!(report.capabilities.restricted);
    assert!(!report.capabilities.may_run_code);
    assert!(!report.capabilities.may_fetch);
    assert!(!project.join("pwned.txt").exists());
    assert!(!project.join("Library/PackageCache").exists());

    assert!(
        report.trust_gate.has_blocked_vector(
            BlockedVectorKind::InitializeOnLoad,
            "Assets/Editor/AutoRun.cs"
        ),
        "expected InitializeOnLoad vector to be blocked: {:?}",
        report.trust_gate.blocked_vectors
    );
    assert!(
        report.trust_gate.has_blocked_vector(
            BlockedVectorKind::SourceGenerator,
            "Assets/Analyzers/EvilGenerator.dll"
        ),
        "expected SourceGenerator vector to be blocked: {:?}",
        report.trust_gate.blocked_vectors
    );
    assert!(
        report.trust_gate.has_blocked_vector(
            BlockedVectorKind::GitPackageFetch,
            "com.example.remote-git"
        ),
        "expected GitPackageFetch vector to be blocked: {:?}",
        report.trust_gate.blocked_vectors
    );

    assert!(opened
        .inventory()
        .entries
        .iter()
        .any(|entry| entry.path == Path::new("Assets/Scenes/Main.unity")));
    assert!(opened
        .inventory()
        .entries
        .iter()
        .any(|entry| entry.path == Path::new("Assets/Materials/Stone.mat")));
    assert!(opened
        .inventory()
        .entries
        .iter()
        .any(|entry| entry.path == Path::new("Assets/Editor/AutoRun.cs")));

    let project_path = path_text(&project)?;
    assert_eq!(
        store.request_trust_from_automation(&project_path),
        TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant)
    );

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn trusted_open_with_fetch_consent_moves_vectors_to_permitted() -> TestResult {
    let root = temp_dir("trust-vectors-permitted")?;
    let project = project_dir(&root)?;
    let store = TrustStore::new(root.join("user-trust.json"));
    let project_path = path_text(&project)?;

    write_project_file(
        &project.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.16f1\n",
    )?;
    write_project_file(
        &project.join("Assets/Editor/Hook.cs"),
        "using UnityEditor;\npublic static class Hook {\n    [InitializeOnLoadMethod]\n    static void Init() {}\n}\n",
    )?;
    write_project_file(
        &project.join("Assets/Editor/Hook.cs.meta"),
        "fileFormatVersion: 2\nguid: 55555555555555555555555555555555\nMonoImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Assets/Gen/MySourceGenerator.cs"),
        "using Microsoft.CodeAnalysis;\n[Generator]\npublic class MySourceGenerator {}\n",
    )?;
    write_project_file(
        &project.join("Assets/Gen/MySourceGenerator.cs.meta"),
        "fileFormatVersion: 2\nguid: 66666666666666666666666666666666\nMonoImporter:\n  externalObjects: {}\n",
    )?;
    write_project_file(
        &project.join("Packages/manifest.json"),
        r#"{
  "dependencies": {
    "com.example.git-pkg": {
      "git": "https://example.invalid/pkg.git"
    }
  }
}"#,
    )?;

    store.trust_project(&project_path, TrustOrigin::Person)?;
    let (_, trusted_without_fetch) = Project::open(&project, &store)?;
    assert!(trusted_without_fetch
        .trust_gate
        .blocked_vectors
        .iter()
        .all(|v| v.kind != BlockedVectorKind::InitializeOnLoad
            && v.kind != BlockedVectorKind::SourceGenerator));
    assert!(trusted_without_fetch.trust_gate.has_blocked_vector(
        BlockedVectorKind::GitPackageFetch,
        "com.example.git-pkg"
    ));

    store.grant_fetch_consent(&project_path, TrustOrigin::Person)?;
    let (_, fully_consented) = Project::open(&project, &store)?;
    assert!(fully_consented.trust_gate.blocked_vectors.is_empty());
    assert_eq!(fully_consented.trust_gate.permitted_vectors.len(), 3);

    fs::remove_dir_all(&root)?;
    Ok(())
}
