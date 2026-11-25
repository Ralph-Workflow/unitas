use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::trust::{
    TrustError, TrustOrigin, TrustOutcome, TrustRefusal, TrustStore,
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
