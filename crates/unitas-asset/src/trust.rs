use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::json::{parse, write as write_json, JsonValue};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockedVectorKind {
    InitializeOnLoad,
    SourceGenerator,
    GitPackageFetch,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockedVector {
    pub kind: BlockedVectorKind,
    pub identifier: String,
    pub executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrustGateReport {
    pub blocked_vectors: Vec<BlockedVector>,
    pub permitted_vectors: Vec<BlockedVector>,
}

impl TrustGateReport {
    pub fn has_blocked_vector(&self, kind: BlockedVectorKind, identifier: &str) -> bool {
        self.blocked_vectors
            .iter()
            .any(|v| v.kind == kind && v.identifier == identifier && !v.executed)
    }

    pub fn has_permitted_vector(&self, kind: BlockedVectorKind, identifier: &str) -> bool {
        self.permitted_vectors
            .iter()
            .any(|v| v.kind == kind && v.identifier == identifier && !v.executed)
    }
}

pub fn scan_project_vectors(
    root: &Path,
    capabilities: ProjectCapabilities,
    git_package_names: &[String],
) -> TrustGateReport {
    let mut code_vectors = Vec::new();
    let assets_dir = root.join("Assets");
    if assets_dir.is_dir() {
        collect_code_vectors(root, &assets_dir, &mut code_vectors);
    }
    let packages_dir = root.join("Packages");
    if packages_dir.is_dir() {
        collect_code_vectors(root, &packages_dir, &mut code_vectors);
    }
    code_vectors.sort();
    code_vectors.dedup();

    let mut blocked_vectors = Vec::new();
    let mut permitted_vectors = Vec::new();

    for vector in code_vectors {
        if capabilities.may_run_code {
            permitted_vectors.push(vector);
        } else {
            blocked_vectors.push(vector);
        }
    }

    let mut git_names: Vec<String> = git_package_names.to_vec();
    git_names.sort();
    git_names.dedup();
    for pkg in git_names {
        let vector = BlockedVector {
            kind: BlockedVectorKind::GitPackageFetch,
            identifier: pkg,
            executed: false,
        };
        if capabilities.may_fetch {
            permitted_vectors.push(vector);
        } else {
            blocked_vectors.push(vector);
        }
    }

    TrustGateReport {
        blocked_vectors,
        permitted_vectors,
    }
}

fn collect_code_vectors(root: &Path, directory: &Path, out: &mut Vec<BlockedVector>) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_code_vectors(root, &path, out);
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if name.ends_with(".meta") {
            if let Ok(meta_text) = fs::read_to_string(&path) {
                if is_roslyn_analyzer_meta(&meta_text) {
                    let asset_rel = rel.strip_suffix(".meta").unwrap_or(&rel).to_string();
                    out.push(BlockedVector {
                        kind: BlockedVectorKind::SourceGenerator,
                        identifier: asset_rel,
                        executed: false,
                    });
                }
            }
            continue;
        }
        if name.ends_with(".cs") {
            if let Ok(source) = fs::read_to_string(&path) {
                if has_initialize_on_load(&source) {
                    out.push(BlockedVector {
                        kind: BlockedVectorKind::InitializeOnLoad,
                        identifier: rel.clone(),
                        executed: false,
                    });
                }
                if is_source_generator_script(&source) {
                    out.push(BlockedVector {
                        kind: BlockedVectorKind::SourceGenerator,
                        identifier: rel,
                        executed: false,
                    });
                }
            }
        } else if name.ends_with(".dll") && is_source_generator_dll_name(name, &rel) {
            out.push(BlockedVector {
                kind: BlockedVectorKind::SourceGenerator,
                identifier: rel,
                executed: false,
            });
        }
    }
}

fn is_roslyn_analyzer_meta(meta_text: &str) -> bool {
    meta_text.contains("RoslynAnalyzer")
}

fn has_initialize_on_load(source: &str) -> bool {
    source.contains("[InitializeOnLoad]")
        || source.contains("[InitializeOnLoadMethod")
        || source.contains("InitializeOnLoadAttribute")
        || source.contains("InitializeOnLoadMethodAttribute")
}

fn is_source_generator_script(source: &str) -> bool {
    source.contains("[Generator")
        || source.contains("ISourceGenerator")
        || source.contains("IIncrementalGenerator")
        || source.contains("DiagnosticAnalyzer")
}

fn is_source_generator_dll_name(name: &str, rel_path: &str) -> bool {
    let lower_name = name.to_ascii_lowercase();
    let lower_path = rel_path.to_ascii_lowercase();
    lower_name.contains("generator")
        || lower_name.contains("analyzer")
        || lower_path.contains("/analyzers/")
        || lower_path.contains("/sourcegenerators/")
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
        self.records()
            .ok()
            .and_then(|records| records.get(&key).cloned())
    }

    pub fn capabilities(&self, project_path: &str) -> ProjectCapabilities {
        let trusted = self.is_trusted(project_path);
        let consent = self.has_fetch_consent(project_path);
        ProjectCapabilities {
            may_run_code: trusted,
            may_fetch: trusted && consent,
            restricted: !trusted,
        }
    }

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
            Some(value) => value
                .as_object()
                .ok_or_else(|| TrustError::InvalidStore("projects is not an object".to_string()))?,
            None => return Ok(BTreeMap::new()),
        };
        let mut records = BTreeMap::new();
        for (path, entry) in projects {
            records.insert(
                path.clone(),
                TrustRecord {
                    project_path: path.clone(),
                    trusted: entry
                        .get("trusted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
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
                    (
                        "fetchConsent".to_string(),
                        JsonValue::Bool(record.fetch_consent),
                    ),
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
