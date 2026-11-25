//! Per-user project trust and fetch consent, stored outside the project.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::json::{parse, write as write_json, JsonValue};

/// Who asked for a trust or consent grant; automation can never be granted one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustOrigin {
    Person,
    Automation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustRefusal {
    AutomationCannotGrant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustOutcome {
    Granted,
    Refused(TrustRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRecord {
    pub project_path: String,
    pub trusted: bool,
    pub fetch_consent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectCapabilities {
    pub may_run_code: bool,
    pub may_fetch: bool,
    pub restricted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    StoreInsideProject {
        project_path: String,
        store_path: String,
    },
    Io {
        path: String,
        message: String,
    },
    InvalidStore(String),
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustError::StoreInsideProject {
                project_path,
                store_path,
            } => write!(
                f,
                "trust store {store_path} must live outside the project {project_path}"
            ),
            TrustError::Io { path, message } => write!(f, "trust store {path}: {message}"),
            TrustError::InvalidStore(message) => write!(f, "invalid trust store: {message}"),
        }
    }
}

impl std::error::Error for TrustError {}

/// Per-user trust records keyed by absolute project path.
#[derive(Debug, Clone)]
pub struct TrustStore {
    store_path: PathBuf,
}

impl TrustStore {
    pub fn new(store_path: impl Into<PathBuf>) -> Self {
        TrustStore {
            store_path: store_path.into(),
        }
    }

    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    /// Grants trust for a project. An automation request is refused and changes nothing.
    pub fn trust_project(
        &self,
        project_path: &str,
        origin: TrustOrigin,
    ) -> Result<TrustOutcome, TrustError> {
        if origin == TrustOrigin::Automation {
            return Ok(TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant));
        }
        let key = self.key(project_path)?;
        let mut records = self.records()?;
        let record = records.entry(key.clone()).or_insert(TrustRecord {
            project_path: key,
            trusted: false,
            fetch_consent: false,
        });
        record.trusted = true;
        self.write_records(&records)?;
        Ok(TrustOutcome::Granted)
    }

    /// The automation path: a request reaches the person and never grants trust.
    pub fn request_trust_from_automation(&self, project_path: &str) -> TrustOutcome {
        match self.trust_project(project_path, TrustOrigin::Automation) {
            Ok(outcome) => outcome,
            Err(_) => TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant),
        }
    }

    pub fn revoke_trust(&self, project_path: &str) -> Result<(), TrustError> {
        let key = self.key(project_path)?;
        let mut records = self.records()?;
        if let Some(record) = records.get_mut(&key) {
            record.trusted = false;
        }
        self.write_records(&records)?;
        Ok(())
    }

    /// Grants fetch consent. Consent never implies trust.
    pub fn grant_fetch_consent(
        &self,
        project_path: &str,
        origin: TrustOrigin,
    ) -> Result<TrustOutcome, TrustError> {
        if origin == TrustOrigin::Automation {
            return Ok(TrustOutcome::Refused(TrustRefusal::AutomationCannotGrant));
        }
        let key = self.key(project_path)?;
        let mut records = self.records()?;
        let record = records.entry(key.clone()).or_insert(TrustRecord {
            project_path: key,
            trusted: false,
            fetch_consent: false,
        });
        record.fetch_consent = true;
        self.write_records(&records)?;
        Ok(TrustOutcome::Granted)
    }

    pub fn revoke_fetch_consent(&self, project_path: &str) -> Result<(), TrustError> {
        let key = self.key(project_path)?;
        let mut records = self.records()?;
        if let Some(record) = records.get_mut(&key) {
            record.fetch_consent = false;
        }
        self.write_records(&records)?;
        Ok(())
    }

    pub fn is_trusted(&self, project_path: &str) -> bool {
        self.record(project_path)
            .map(|record| record.trusted)
            .unwrap_or(false)
    }

    pub fn has_fetch_consent(&self, project_path: &str) -> bool {
        self.record(project_path)
            .map(|record| record.fetch_consent)
            .unwrap_or(false)
    }

    pub fn record(&self, project_path: &str) -> Option<TrustRecord> {
        let key = self.key(project_path).ok()?;
        self.records().ok().and_then(|records| records.get(&key).cloned())
    }

    /// Capabilities of a project from the per-user records alone.
    pub fn capabilities(&self, project_path: &str) -> ProjectCapabilities {
        let trusted = self.is_trusted(project_path);
        let consent = self.has_fetch_consent(project_path);
        ProjectCapabilities {
            may_run_code: trusted,
            may_fetch: trusted && consent,
            restricted: !trusted,
        }
    }

    /// Capabilities when the invocation itself carries an explicit fetch consent.
    pub fn capabilities_with_fetch_consent(
        &self,
        project_path: &str,
        invocation_consent: bool,
    ) -> ProjectCapabilities {
        let trusted = self.is_trusted(project_path);
        ProjectCapabilities {
            may_run_code: trusted,
            may_fetch: invocation_consent || (trusted && self.has_fetch_consent(project_path)),
            restricted: !trusted,
        }
    }

    fn key(&self, project_path: &str) -> Result<String, TrustError> {
        let project = normalize(project_path);
        let store = normalize(&self.store_path.to_string_lossy());
        if Path::new(&store).starts_with(Path::new(&project)) {
            return Err(TrustError::StoreInsideProject {
                project_path: project,
                store_path: store,
            });
        }
        Ok(project)
    }

    fn records(&self) -> Result<BTreeMap<String, TrustRecord>, TrustError> {
        let text = match fs::read_to_string(&self.store_path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new())
            }
            Err(error) => {
                return Err(TrustError::Io {
                    path: self.store_path.to_string_lossy().to_string(),
                    message: error.to_string(),
                })
            }
        };
        if text.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        let root = parse(&text).map_err(|e| TrustError::InvalidStore(format!("{e:?}")))?;
        let projects = match root.get("projects") {
            Some(value) => value.as_object().ok_or_else(|| {
                TrustError::InvalidStore("projects is not an object".to_string())
            })?,
            None => return Ok(BTreeMap::new()),
        };
        let mut records = BTreeMap::new();
        for (path, entry) in projects {
            records.insert(
                path.clone(),
                TrustRecord {
                    project_path: path.clone(),
                    trusted: entry.get("trusted").and_then(|v| v.as_bool()).unwrap_or(false),
                    fetch_consent: entry
                        .get("fetchConsent")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                },
            );
        }
        Ok(records)
    }

    fn write_records(&self, records: &BTreeMap<String, TrustRecord>) -> Result<(), TrustError> {
        let mut projects = Vec::new();
        for (path, record) in records {
            projects.push((
                path.clone(),
                JsonValue::Object(vec![
                    ("trusted".to_string(), JsonValue::Bool(record.trusted)),
                    ("fetchConsent".to_string(), JsonValue::Bool(record.fetch_consent)),
                ]),
            ));
        }
        let root = JsonValue::Object(vec![
            ("version".to_string(), JsonValue::Number(1.0)),
            ("projects".to_string(), JsonValue::Object(projects)),
        ]);
        if let Some(parent) = self.store_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| TrustError::Io {
                    path: parent.to_string_lossy().to_string(),
                    message: error.to_string(),
                })?;
            }
        }
        fs::write(&self.store_path, write_json(&root)).map_err(|error| TrustError::Io {
            path: self.store_path.to_string_lossy().to_string(),
            message: error.to_string(),
        })
    }
}

/// Lexically normalizes a path so trust records key on one absolute location.
fn normalize(path: &str) -> String {
    let raw = Path::new(path);
    let absolute = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(current) => current.join(raw),
            Err(_) => raw.to_path_buf(),
        }
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::RootDir => normalized.push("/"),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized.to_string_lossy().to_string()
}
