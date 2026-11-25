use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use crate::json::{parse, write as write_json, JsonValue};

const BUILTIN_PREFIX: &str = "com.unity.modules.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    Registry,
    ScopedRegistry(String),
    Git(String),
    Local(String),
    Embedded,
    Tarball(String),
    BuiltIn,
}

impl PackageSource {
    pub fn kind(&self) -> &'static str {
        match self {
            PackageSource::Registry => "registry",
            PackageSource::ScopedRegistry(_) => "scoped-registry",
            PackageSource::Git(_) => "git",
            PackageSource::Local(_) => "local",
            PackageSource::Embedded => "embedded",
            PackageSource::Tarball(_) => "tarball",
            PackageSource::BuiltIn => "builtin",
        }
    }

    pub fn is_editable(&self) -> bool {
        matches!(self, PackageSource::Embedded | PackageSource::Local(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageEditOutcome {
    Committed {
        package: String,
        path: String,
    },
    NotCommitted {
        package: String,
        source: PackageSource,
        path: String,
        remediation: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageModificationReason {
    ReadOnlyCacheModified,
    EmbeddedDivergesFromRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageAssessmentClassification {
    ReleasedVersion,
    ThirdPartyContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageModificationFinding {
    pub package: String,
    pub version: Option<String>,
    pub source: PackageSource,
    pub modified_paths: Vec<String>,
    pub committed: bool,
    pub reason: PackageModificationReason,
    pub classification: PackageAssessmentClassification,
    pub message: String,
}

pub fn attempt_package_edit(
    manifest: &PackageManifest,
    package_name: &str,
    relative_path: &str,
) -> PackageEditOutcome {
    let source = manifest
        .dependencies
        .iter()
        .find(|d| d.name == package_name)
        .map(|d| d.source.clone())
        .unwrap_or(PackageSource::Registry);
    if source.is_editable() {
        PackageEditOutcome::Committed {
            package: package_name.to_string(),
            path: relative_path.to_string(),
        }
    } else {
        PackageEditOutcome::NotCommitted {
            package: package_name.to_string(),
            source,
            path: relative_path.to_string(),
            remediation: "embed the package under Packages/ to edit its content".to_string(),
        }
    }
}

pub fn detect_package_modifications(
    project_root: &Path,
    manifest: &PackageManifest,
    lock: Option<&PackageLock>,
) -> Vec<PackageModificationFinding> {
    let mut findings = Vec::new();
    let locked_by_name: BTreeMap<&str, &LockedPackage> = lock
        .map(|l| {
            l.dependencies
                .iter()
                .map(|entry| (entry.name.as_str(), entry))
                .collect()
        })
        .unwrap_or_default();

    let cache_dir = project_root.join("Library").join("PackageCache");
    for dep in &manifest.dependencies {
        let locked = locked_by_name.get(dep.name.as_str()).copied();
        let version = dep
            .version
            .clone()
            .or_else(|| locked.and_then(|l| l.version.clone()));
        match &dep.source {
            PackageSource::Registry
            | PackageSource::ScopedRegistry(_)
            | PackageSource::Git(_)
            | PackageSource::Tarball(_)
            | PackageSource::BuiltIn => {
                if let Some(pkg_cache_dir) =
                    find_cached_package_dir(&cache_dir, &dep.name, version.as_deref())
                {
                    let modified_paths = detect_modified_files_in_dir(&pkg_cache_dir, locked);
                    if !modified_paths.is_empty() {
                        findings.push(PackageModificationFinding {
                            package: dep.name.clone(),
                            version: version.clone(),
                            source: dep.source.clone(),
                            modified_paths,
                            committed: false,
                            reason: PackageModificationReason::ReadOnlyCacheModified,
                            classification: PackageAssessmentClassification::ThirdPartyContent,
                            message: format!(
                                "edit to read-only {} package {} was not committed; embed the package under Packages/ to modify it",
                                dep.source.kind(),
                                dep.name
                            ),
                        });
                    }
                }
            }
            PackageSource::Embedded => {
                let folder_name = dep.path.as_deref().unwrap_or(&dep.name);
                let embedded_dir = project_root.join("Packages").join(folder_name);
                if embedded_dir.is_dir() {
                    let modified_paths = detect_embedded_divergence(&embedded_dir, locked);
                    if !modified_paths.is_empty() {
                        findings.push(PackageModificationFinding {
                            package: dep.name.clone(),
                            version,
                            source: PackageSource::Embedded,
                            modified_paths,
                            committed: true,
                            reason: PackageModificationReason::EmbeddedDivergesFromRelease,
                            classification: PackageAssessmentClassification::ThirdPartyContent,
                            message: format!(
                                "embedded package {} is locally modified from the released version and is assessed as third-party content",
                                dep.name
                            ),
                        });
                    }
                }
            }
            PackageSource::Local(_) => {}
        }
    }
    findings.sort_by(|a, b| a.package.cmp(&b.package));
    findings
}

fn find_cached_package_dir(
    cache_dir: &Path,
    package_name: &str,
    version: Option<&str>,
) -> Option<std::path::PathBuf> {
    if !cache_dir.is_dir() {
        return None;
    }
    if let Some(ver) = version {
        let exact = cache_dir.join(format!("{package_name}@{ver}"));
        if exact.is_dir() {
            return Some(exact);
        }
    }
    let prefix = format!("{package_name}@");
    let entries = fs::read_dir(cache_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) || name == package_name {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn detect_modified_files_in_dir(dir: &Path, locked: Option<&LockedPackage>) -> Vec<String> {
    let mut files = Vec::new();
    collect_relative_files(dir, dir, &mut files);
    files.sort();
    let non_manifest: Vec<String> = files
        .into_iter()
        .filter(|rel| rel != "package.json" && rel != "package.json.meta")
        .collect();
    if !non_manifest.is_empty() {
        return non_manifest;
    }
    if let Some(lock_entry) = locked {
        if let Ok(pkg_json_text) = fs::read_to_string(dir.join("package.json")) {
            if let Ok(root) = parse(&pkg_json_text) {
                let disk_version = root.get("version").and_then(|v| v.as_str());
                if let (Some(disk_v), Some(lock_v)) = (disk_version, lock_entry.version.as_deref())
                {
                    if disk_v != lock_v {
                        return vec!["package.json".to_string()];
                    }
                }
            }
        }
    }
    Vec::new()
}

fn detect_embedded_divergence(dir: &Path, locked: Option<&LockedPackage>) -> Vec<String> {
    let mut files = Vec::new();
    collect_relative_files(dir, dir, &mut files);
    files.sort();
    if let Ok(pkg_json_text) = fs::read_to_string(dir.join("package.json")) {
        if let Ok(root) = parse(&pkg_json_text) {
            let disk_version = root.get("version").and_then(|v| v.as_str());
            if let Some(lock_entry) = locked {
                if let (Some(disk_v), Some(lock_v)) = (disk_version, lock_entry.version.as_deref())
                {
                    if disk_v != lock_v {
                        return files;
                    }
                }
                if lock_entry.hash.is_some() {
                    let non_manifest: Vec<String> = files
                        .iter()
                        .filter(|rel| *rel != "package.json" && *rel != "package.json.meta")
                        .cloned()
                        .collect();
                    if !non_manifest.is_empty() {
                        return non_manifest;
                    }
                }
            } else if !files.is_empty() {
                return files;
            }
        }
    }
    Vec::new()
}

fn collect_relative_files(root: &Path, current: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRequirement {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDependency {
    pub name: String,
    pub source: PackageSource,
    pub version: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,
    pub scoped_registry: Option<String>,
    pub hash: Option<String>,
    pub dependencies: Vec<PackageRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedRegistry {
    pub name: String,
    pub url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub dependencies: Vec<PackageDependency>,
    pub scoped_registries: Vec<ScopedRegistry>,
    pub testables: Vec<String>,
    pub resolution_strategy: String,
    pub enable_lock_file: bool,
}

impl Default for PackageManifest {
    fn default() -> Self {
        PackageManifest {
            dependencies: Vec::new(),
            scoped_registries: Vec::new(),
            testables: Vec::new(),
            resolution_strategy: "highest".to_string(),
            enable_lock_file: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub depth: u32,
    pub source: PackageSource,
    pub version: Option<String>,
    pub hash: Option<String>,
    pub dependencies: Vec<PackageRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLock {
    pub dependencies: Vec<LockedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    InvalidJson(String),
    InvalidValue {
        context: String,
        field: String,
        value: String,
    },
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::InvalidJson(message) => write!(f, "invalid package json: {message}"),
            PackageError::InvalidValue {
                context,
                field,
                value,
            } => write!(f, "invalid {field} in {context}: {value}"),
        }
    }
}

impl std::error::Error for PackageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequest {
    pub version: String,
    pub chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionConflict {
    VersionMismatch {
        package: String,
        requests: Vec<VersionRequest>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedReason {
    MissingFromLock,
    MissingVersion,
    VersionConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedPackage {
    pub name: String,
    pub source: Option<PackageSource>,
    pub reason: UnresolvedReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Option<String>,
    pub source: PackageSource,
    pub depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolutionReport {
    pub resolved: Vec<ResolvedPackage>,
    pub conflicts: Vec<ResolutionConflict>,
    pub unresolved: Vec<UnresolvedPackage>,
}

impl ResolutionReport {
    pub fn resolved_package(&self, name: &str) -> Option<&ResolvedPackage> {
        self.resolved.iter().find(|p| p.name == name)
    }

    pub fn unresolved_package(&self, name: &str) -> Option<&UnresolvedPackage> {
        self.unresolved.iter().find(|p| p.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    package: String,
    version: Option<String>,
    chain: Vec<String>,
}

pub fn parse_manifest(text: &str) -> Result<PackageManifest, PackageError> {
    let root = parse(text).map_err(|e| PackageError::InvalidJson(format!("{e:?}")))?;
    let mut manifest = PackageManifest::default();

    if let Some(registries) = root.get("scopedRegistries") {
        let entries = registries
            .as_array()
            .ok_or_else(|| PackageError::InvalidValue {
                context: "manifest.json".to_string(),
                field: "scopedRegistries".to_string(),
                value: "not an array".to_string(),
            })?;
        for entry in entries {
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let url = entry
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let scopes = entry
                .get("scopes")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();
            manifest
                .scoped_registries
                .push(ScopedRegistry { name, url, scopes });
        }
    }

    if let Some(testables) = root.get("testables") {
        let entries = testables
            .as_array()
            .ok_or_else(|| PackageError::InvalidValue {
                context: "manifest.json".to_string(),
                field: "testables".to_string(),
                value: "not an array".to_string(),
            })?;
        manifest.testables = entries
            .iter()
            .filter_map(|item| item.as_str())
            .map(|s| s.to_string())
            .collect();
    }

    if let Some(strategy) = root.get("resolutionStrategy") {
        manifest.resolution_strategy = match strategy {
            JsonValue::String(text) => text.clone(),
            JsonValue::Object(_) => strategy
                .get("strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("highest")
                .to_string(),
            _ => "highest".to_string(),
        };
    }

    if let Some(enabled) = root.get("enableLockFile") {
        manifest.enable_lock_file = enabled.as_bool().unwrap_or(true);
    }

    if let Some(dependencies) = root.get("dependencies") {
        let entries = dependencies
            .as_object()
            .ok_or_else(|| PackageError::InvalidValue {
                context: "manifest.json".to_string(),
                field: "dependencies".to_string(),
                value: "not an object".to_string(),
            })?;
        for (name, value) in entries {
            manifest.dependencies.push(parse_dependency(name, value)?);
        }
    }

    for dependency in &mut manifest.dependencies {
        if dependency.source != PackageSource::Registry {
            continue;
        }
        if let Some(registry) = manifest.scoped_registries.iter().find(|registry| {
            registry
                .scopes
                .iter()
                .any(|scope| scope == &dependency.name)
        }) {
            dependency.source = PackageSource::ScopedRegistry(registry.url.clone());
            dependency.scoped_registry = Some(registry.url.clone());
        }
    }

    Ok(manifest)
}

pub fn parse_lock(text: &str) -> Result<PackageLock, PackageError> {
    let root = parse(text).map_err(|e| PackageError::InvalidJson(format!("{e:?}")))?;
    let mut lock = PackageLock {
        dependencies: Vec::new(),
    };
    let dependencies = match root.get("dependencies") {
        Some(value) => value
            .as_object()
            .ok_or_else(|| PackageError::InvalidValue {
                context: "packages-lock.json".to_string(),
                field: "dependencies".to_string(),
                value: "not an object".to_string(),
            })?,
        None => return Ok(lock),
    };
    for (name, entry) in dependencies {
        let url = entry
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let kind = entry
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("registry");
        let source = match kind {
            "registry" => PackageSource::Registry,
            "scoped-registry" => PackageSource::ScopedRegistry(url.clone().unwrap_or_default()),
            "git" => PackageSource::Git(url.clone().unwrap_or_default()),
            "local" => PackageSource::Local(url.clone().unwrap_or_default()),
            "embedded" => PackageSource::Embedded,
            "tarball" => PackageSource::Tarball(url.clone().unwrap_or_default()),
            "builtin" => PackageSource::BuiltIn,
            other => {
                return Err(PackageError::InvalidValue {
                    context: "packages-lock.json".to_string(),
                    field: "source".to_string(),
                    value: other.to_string(),
                })
            }
        };
        let requirements = match entry.get("dependencies") {
            Some(value) => parse_requirements(value)?,
            None => Vec::new(),
        };
        lock.dependencies.push(LockedPackage {
            name: name.clone(),
            depth: entry
                .get("depth")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0) as u32,
            source,
            version: entry
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            hash: entry
                .get("hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            dependencies: requirements,
        });
    }
    Ok(lock)
}

fn parse_requirements(value: &JsonValue) -> Result<Vec<PackageRequirement>, PackageError> {
    let entries = value
        .as_object()
        .ok_or_else(|| PackageError::InvalidValue {
            context: "package dependencies".to_string(),
            field: "dependencies".to_string(),
            value: "not an object".to_string(),
        })?;
    let mut requirements = Vec::new();
    for (name, entry) in entries {
        let version = match entry {
            JsonValue::String(text) => Some(text.clone()),
            JsonValue::Object(_) => entry
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        };
        requirements.push(PackageRequirement {
            name: name.clone(),
            version,
        });
    }
    Ok(requirements)
}

fn parse_dependency(name: &str, value: &JsonValue) -> Result<PackageDependency, PackageError> {
    let mut dependency = PackageDependency {
        name: name.to_string(),
        source: PackageSource::Registry,
        version: None,
        url: None,
        path: None,
        scoped_registry: None,
        hash: None,
        dependencies: Vec::new(),
    };
    match value {
        JsonValue::String(version) => dependency.version = Some(version.clone()),
        JsonValue::Object(_) => {
            if let Some(version) = value.get("version").and_then(|v| v.as_str()) {
                dependency.version = Some(version.to_string());
            }
            if let Some(url) = value.get("git").and_then(|v| v.as_str()) {
                dependency.url = Some(url.to_string());
                dependency.source = PackageSource::Git(url.to_string());
            }
            if let Some(hash) = value.get("hash").and_then(|v| v.as_str()) {
                dependency.hash = Some(hash.to_string());
            }
            if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                if dependency.source == PackageSource::Registry {
                    dependency.url = Some(url.to_string());
                    dependency.source = PackageSource::Tarball(url.to_string());
                }
            }
            if let Some(path) = value.get("file").and_then(|v| v.as_str()) {
                dependency.source = source_for_file(path);
                match &dependency.source {
                    PackageSource::Tarball(url) => dependency.url = Some(url.clone()),
                    PackageSource::Local(local) => dependency.path = Some(local.clone()),
                    PackageSource::Embedded => dependency.path = Some(path.to_string()),
                    _ => {}
                }
            }
            if let Some(url) = value.get("scopedRegistry").and_then(|v| v.as_str()) {
                dependency.scoped_registry = Some(url.to_string());
                if dependency.source == PackageSource::Registry {
                    dependency.source = PackageSource::ScopedRegistry(url.to_string());
                }
            }
            if let Some(requirements) = value.get("dependencies") {
                dependency.dependencies = parse_requirements(requirements)?;
            }
        }
        _ => {
            return Err(PackageError::InvalidValue {
                context: "manifest.json".to_string(),
                field: name.to_string(),
                value: "not a version string or object".to_string(),
            })
        }
    }
    if dependency.source == PackageSource::Registry && name.starts_with(BUILTIN_PREFIX) {
        dependency.source = PackageSource::BuiltIn;
    }
    Ok(dependency)
}

fn source_for_file(path: &str) -> PackageSource {
    let trimmed = path.strip_prefix("file:").unwrap_or(path);
    if trimmed.ends_with(".tgz") || trimmed.ends_with(".tar.gz") {
        PackageSource::Tarball(trimmed.to_string())
    } else if trimmed.contains('/') || trimmed.contains('\\') {
        PackageSource::Local(trimmed.to_string())
    } else {
        PackageSource::Embedded
    }
}

impl PackageManifest {
    pub fn to_json(&self) -> JsonValue {
        let mut dependencies = Vec::new();
        for dependency in &self.dependencies {
            let mut fields: Vec<(String, JsonValue)> = Vec::new();
            match &dependency.source {
                PackageSource::Registry | PackageSource::BuiltIn => {}
                PackageSource::ScopedRegistry(url) => {
                    fields.push(("scopedRegistry".to_string(), JsonValue::String(url.clone())));
                }
                PackageSource::Git(url) => {
                    fields.push(("git".to_string(), JsonValue::String(url.clone())));
                }
                PackageSource::Local(path) => {
                    fields.push(("file".to_string(), JsonValue::String(path.clone())));
                }
                PackageSource::Embedded => {
                    fields.push((
                        "file".to_string(),
                        JsonValue::String(dependency.name.clone()),
                    ));
                }
                PackageSource::Tarball(url) => {
                    fields.push(("url".to_string(), JsonValue::String(url.clone())));
                }
            }
            if let Some(version) = &dependency.version {
                fields.push(("version".to_string(), JsonValue::String(version.clone())));
            }
            if let Some(hash) = &dependency.hash {
                fields.push(("hash".to_string(), JsonValue::String(hash.clone())));
            }
            if !dependency.dependencies.is_empty() {
                let mut requirements = Vec::new();
                for requirement in &dependency.dependencies {
                    let version = requirement.version.clone().unwrap_or_default();
                    requirements.push((requirement.name.clone(), JsonValue::String(version)));
                }
                fields.push(("dependencies".to_string(), JsonValue::Object(requirements)));
            }
            dependencies.push((dependency.name.clone(), JsonValue::Object(fields)));
        }
        let mut root: Vec<(String, JsonValue)> =
            vec![("dependencies".to_string(), JsonValue::Object(dependencies))];
        if !self.scoped_registries.is_empty() {
            let mut registries = Vec::new();
            for registry in &self.scoped_registries {
                let scopes = registry
                    .scopes
                    .iter()
                    .map(|scope| JsonValue::String(scope.clone()))
                    .collect();
                registries.push(JsonValue::Object(vec![
                    ("name".to_string(), JsonValue::String(registry.name.clone())),
                    ("url".to_string(), JsonValue::String(registry.url.clone())),
                    ("scopes".to_string(), JsonValue::Array(scopes)),
                ]));
            }
            root.push(("scopedRegistries".to_string(), JsonValue::Array(registries)));
        }
        if !self.testables.is_empty() {
            let testables = self
                .testables
                .iter()
                .map(|name| JsonValue::String(name.clone()))
                .collect();
            root.push(("testables".to_string(), JsonValue::Array(testables)));
        }
        root.push((
            "resolutionStrategy".to_string(),
            JsonValue::String(self.resolution_strategy.clone()),
        ));
        root.push((
            "enableLockFile".to_string(),
            JsonValue::Bool(self.enable_lock_file),
        ));
        JsonValue::Object(root)
    }

    pub fn write(&self) -> String {
        write_json(&self.to_json())
    }
}

impl PackageLock {
    pub fn to_json(&self) -> JsonValue {
        let mut dependencies = Vec::new();
        for entry in &self.dependencies {
            let mut fields: Vec<(String, JsonValue)> = Vec::new();
            if let Some(version) = &entry.version {
                fields.push(("version".to_string(), JsonValue::String(version.clone())));
            }
            fields.push(("depth".to_string(), JsonValue::Number(entry.depth as f64)));
            fields.push((
                "source".to_string(),
                JsonValue::String(entry.source.kind().to_string()),
            ));
            match &entry.source {
                PackageSource::ScopedRegistry(url)
                | PackageSource::Git(url)
                | PackageSource::Tarball(url)
                | PackageSource::Local(url) => {
                    fields.push(("url".to_string(), JsonValue::String(url.clone())));
                }
                PackageSource::Registry | PackageSource::Embedded | PackageSource::BuiltIn => {}
            }
            if let Some(hash) = &entry.hash {
                fields.push(("hash".to_string(), JsonValue::String(hash.clone())));
            }
            if !entry.dependencies.is_empty() {
                let mut requirements = Vec::new();
                for requirement in &entry.dependencies {
                    let version = requirement.version.clone().unwrap_or_default();
                    requirements.push((requirement.name.clone(), JsonValue::String(version)));
                }
                fields.push(("dependencies".to_string(), JsonValue::Object(requirements)));
            }
            dependencies.push((entry.name.clone(), JsonValue::Object(fields)));
        }
        JsonValue::Object(vec![(
            "dependencies".to_string(),
            JsonValue::Object(dependencies),
        )])
    }

    pub fn write(&self) -> String {
        write_json(&self.to_json())
    }
}

pub fn resolve(manifest: &PackageManifest, lock: Option<&PackageLock>) -> ResolutionReport {
    let mut report = ResolutionReport::default();
    let requests = collect_requests(manifest, lock);
    let groups = group_requests(&requests);
    let direct: BTreeMap<&str, &PackageDependency> = manifest
        .dependencies
        .iter()
        .map(|dependency| (dependency.name.as_str(), dependency))
        .collect();
    let locked: BTreeMap<&str, &LockedPackage> = lock
        .map(|lock| {
            lock.dependencies
                .iter()
                .map(|entry| (entry.name.as_str(), entry))
                .collect()
        })
        .unwrap_or_default();

    let conflicted = detect_conflicts(&groups, &direct, &mut report);

    for dependency in &manifest.dependencies {
        let version = dependency.version.clone().or_else(|| {
            locked
                .get(dependency.name.as_str())
                .and_then(|e| e.version.clone())
        });
        match version {
            Some(version) => report.resolved.push(ResolvedPackage {
                name: dependency.name.clone(),
                version: Some(version),
                source: dependency.source.clone(),
                depth: 0,
            }),
            None => report.unresolved.push(UnresolvedPackage {
                name: dependency.name.clone(),
                source: Some(dependency.source.clone()),
                reason: UnresolvedReason::MissingVersion,
            }),
        }
    }

    if let Some(lock) = lock {
        for entry in &lock.dependencies {
            if direct.contains_key(entry.name.as_str()) {
                continue;
            }
            if conflicted.contains(entry.name.as_str()) {
                report.unresolved.push(UnresolvedPackage {
                    name: entry.name.clone(),
                    source: Some(entry.source.clone()),
                    reason: UnresolvedReason::VersionConflict,
                });
                continue;
            }
            report.resolved.push(ResolvedPackage {
                name: entry.name.clone(),
                version: entry.version.clone(),
                source: entry.source.clone(),
                depth: entry.depth,
            });
        }
    }

    for name in groups.keys() {
        if direct.contains_key(*name) || locked.contains_key(*name) {
            continue;
        }
        let reason = if conflicted.contains(*name) {
            UnresolvedReason::VersionConflict
        } else {
            UnresolvedReason::MissingFromLock
        };
        report.unresolved.push(UnresolvedPackage {
            name: (*name).to_string(),
            source: None,
            reason,
        });
    }

    sort_report(&mut report);
    report
}

pub fn open_offline(manifest: &PackageManifest, lock: &PackageLock) -> ResolutionReport {
    let mut report = ResolutionReport::default();
    let requests = collect_requests(manifest, Some(lock));
    let groups = group_requests(&requests);
    let direct: BTreeMap<&str, &PackageDependency> = manifest
        .dependencies
        .iter()
        .map(|dependency| (dependency.name.as_str(), dependency))
        .collect();
    let conflicted = detect_conflicts(&groups, &direct, &mut report);

    for entry in &lock.dependencies {
        if conflicted.contains(entry.name.as_str()) {
            report.unresolved.push(UnresolvedPackage {
                name: entry.name.clone(),
                source: Some(entry.source.clone()),
                reason: UnresolvedReason::VersionConflict,
            });
            continue;
        }
        report.resolved.push(ResolvedPackage {
            name: entry.name.clone(),
            version: entry.version.clone(),
            source: entry.source.clone(),
            depth: entry.depth,
        });
    }

    for dependency in &manifest.dependencies {
        let locked = lock
            .dependencies
            .iter()
            .any(|entry| entry.name == dependency.name);
        if !locked {
            report.unresolved.push(UnresolvedPackage {
                name: dependency.name.clone(),
                source: Some(dependency.source.clone()),
                reason: UnresolvedReason::MissingFromLock,
            });
        }
    }

    sort_report(&mut report);
    report
}

fn collect_requests(manifest: &PackageManifest, lock: Option<&PackageLock>) -> Vec<Request> {
    let mut requests = Vec::new();
    for dependency in &manifest.dependencies {
        requests.push(Request {
            package: dependency.name.clone(),
            version: dependency.version.clone(),
            chain: vec![dependency.name.clone()],
        });
        for requirement in &dependency.dependencies {
            requests.push(Request {
                package: requirement.name.clone(),
                version: requirement.version.clone(),
                chain: vec![dependency.name.clone(), requirement.name.clone()],
            });
        }
    }
    if let Some(lock) = lock {
        let paths = lock_paths(lock);
        for entry in &lock.dependencies {
            for requirement in &entry.dependencies {
                let mut chain = paths
                    .get(entry.name.as_str())
                    .cloned()
                    .unwrap_or_else(|| vec![entry.name.clone()]);
                chain.push(requirement.name.clone());
                requests.push(Request {
                    package: requirement.name.clone(),
                    version: requirement.version.clone(),
                    chain,
                });
            }
        }
    }
    requests
}

fn lock_paths(lock: &PackageLock) -> BTreeMap<&str, Vec<String>> {
    let mut entries: Vec<&LockedPackage> = lock.dependencies.iter().collect();
    entries.sort_by(|left, right| (left.depth, &left.name).cmp(&(right.depth, &right.name)));
    let mut paths: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for entry in entries {
        let parent_path = paths
            .get(entry.name.as_str())
            .cloned()
            .unwrap_or_else(|| vec![entry.name.clone()]);
        paths
            .entry(entry.name.as_str())
            .or_insert(parent_path.clone());
        for requirement in &entry.dependencies {
            let mut child_path = parent_path.clone();
            child_path.push(requirement.name.clone());
            paths.entry(requirement.name.as_str()).or_insert(child_path);
        }
    }
    paths
}

fn group_requests(requests: &[Request]) -> BTreeMap<&str, Vec<&Request>> {
    let mut groups: BTreeMap<&str, Vec<&Request>> = BTreeMap::new();
    for request in requests {
        groups
            .entry(request.package.as_str())
            .or_default()
            .push(request);
    }
    groups
}

fn detect_conflicts(
    groups: &BTreeMap<&str, Vec<&Request>>,
    direct: &BTreeMap<&str, &PackageDependency>,
    report: &mut ResolutionReport,
) -> BTreeSet<String> {
    let mut conflicted = BTreeSet::new();
    for (name, group) in groups {
        if direct.contains_key(name) {
            continue;
        }
        let mut versions: Vec<&str> = group
            .iter()
            .filter_map(|request| request.version.as_deref())
            .collect();
        versions.sort_unstable();
        versions.dedup();
        if versions.len() < 2 {
            continue;
        }
        let mut requests: Vec<VersionRequest> = group
            .iter()
            .filter_map(|request| {
                request.version.as_ref().map(|version| VersionRequest {
                    version: version.clone(),
                    chain: request.chain.clone(),
                })
            })
            .collect();
        requests.sort_by(|left, right| left.version.cmp(&right.version));
        report.conflicts.push(ResolutionConflict::VersionMismatch {
            package: (*name).to_string(),
            requests,
        });
        conflicted.insert((*name).to_string());
    }
    conflicted
}

fn sort_report(report: &mut ResolutionReport) {
    report
        .resolved
        .sort_by(|left, right| (left.depth, &left.name).cmp(&(right.depth, &right.name)));
    report
        .unresolved
        .sort_by(|left, right| left.name.cmp(&right.name));
}
