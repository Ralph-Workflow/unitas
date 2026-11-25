use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::binary_ser::{is_text_serialized, read_serialized_file};
use crate::meta::parse_meta_text;
use crate::packages::{
    detect_package_modifications, parse_lock, parse_manifest, resolve, PackageLock,
    PackageManifest, PackageModificationFinding, PackageSource,
};
use crate::project_settings::{
    parse_dynamics_manager, parse_input_manager, parse_player_settings, parse_time_manager,
};
use crate::trust::{
    scan_project_vectors, InvocationTrust, ProjectCapabilities, TrustGateReport, TrustStore,
};
use crate::unity_yaml::{self, Node};

pub const BASELINE_UNITY_VERSION: &str = "2022.3";

pub const ASSESSED_ON_EDIT: &str = "assessed on edit";

const MIGRATABLE_VERSIONS: [&str; 2] = ["2020.3", "2021.3"];

const EARLIEST_MIGRATABLE_VERSION: &str = "2020.3";

const RECOGNIZED_SETTINGS: [&str; 4] = [
    "ProjectSettings.asset",
    "TimeManager.asset",
    "DynamicsManager.asset",
    "InputManager.asset",
];

const UNITY_GENERATED_FOLDERS: [&str; 7] = [
    "Library",
    "Temp",
    "Logs",
    "obj",
    "UserSettings",
    ".git",
    ".unitas",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    Current,
    RequiresUpgrade(UpgradePreview),
    Unsupported { declared: String, reason: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpgradePreview {
    pub assets: Vec<String>,
    pub settings: Vec<String>,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetKind {
    Main,
    Sub,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry {
    pub guid: String,
    pub path: PathBuf,
    pub kind: AssetKind,
    pub class: Option<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGuid {
    pub guid: String,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetInventory {
    pub entries: Vec<AssetEntry>,
    pub duplicate_guids: Vec<DuplicateGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidLookup<'a> {
    Asset(&'a AssetEntry),
    Duplicate(&'a DuplicateGuid),
    Missing,
}

impl AssetInventory {
    pub fn lookup(&self, guid: &str) -> GuidLookup<'_> {
        if let Some(duplicate) = self
            .duplicate_guids
            .iter()
            .find(|record| record.guid == guid)
        {
            return GuidLookup::Duplicate(duplicate);
        }
        match self.entries.iter().find(|entry| entry.guid == guid) {
            Some(entry) => GuidLookup::Asset(entry),
            None => GuidLookup::Missing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    pub declared_version: Option<String>,
    pub effective_behavior_version: String,
    pub migration: MigrationStatus,
    pub inventory: AssetInventory,
    pub capabilities: ProjectCapabilities,
    pub trust_gate: TrustGateReport,
    pub recognized_packages: Vec<String>,
    pub package_modifications: Vec<PackageModificationFinding>,
    pub recognized_settings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub(crate) root: PathBuf,
    pub(crate) inventory: AssetInventory,
    capabilities: ProjectCapabilities,
    trust_gate: TrustGateReport,
}

impl Project {
    pub fn open(root: &Path, trust: &TrustStore) -> Result<(Project, OpenReport), ProjectError> {
        let capabilities = trust.capabilities(&root.to_string_lossy());
        Self::open_with_capabilities(root, capabilities)
    }

    pub fn open_with_invocation(
        root: &Path,
        trust: &TrustStore,
        invocation: InvocationTrust,
    ) -> Result<(Project, OpenReport), ProjectError> {
        let capabilities =
            trust.capabilities_with_invocation(&root.to_string_lossy(), invocation);
        Self::open_with_capabilities(root, capabilities)
    }

    fn open_with_capabilities(
        root: &Path,
        capabilities: ProjectCapabilities,
    ) -> Result<(Project, OpenReport), ProjectError> {
        let missing = missing_signals(root);
        if !missing.is_empty() {
            return Err(ProjectError::NotRecognizable { missing });
        }
        let declared_version = read_declared_version(root)?;
        let inventory = build_inventory(root)?;
        let recognized_settings = read_settings(root);
        let (manifest, lock) = load_manifest_and_lock(root)?;
        let recognized_packages = recognized_package_names(manifest.as_ref(), lock.as_ref());
        let git_packages = git_package_names(manifest.as_ref(), lock.as_ref());
        let trust_gate = scan_project_vectors(root, capabilities, &git_packages);
        let package_modifications = match &manifest {
            Some(manifest) => detect_package_modifications(root, manifest, lock.as_ref()),
            None => Vec::new(),
        };
        let migration = match classify_version(declared_version.as_deref()) {
            VersionClass::Current => MigrationStatus::Current,
            VersionClass::Migratable => MigrationStatus::RequiresUpgrade(upgrade_preview(
                root,
                &inventory,
                &recognized_settings,
                &recognized_packages,
            )),
            VersionClass::Unsupported(reason) => MigrationStatus::Unsupported {
                declared: declared_version.clone().unwrap_or_default(),
                reason,
            },
        };
        let report = OpenReport {
            declared_version,
            effective_behavior_version: BASELINE_UNITY_VERSION.to_string(),
            migration,
            inventory: inventory.clone(),
            capabilities,
            trust_gate: trust_gate.clone(),
            recognized_packages,
            package_modifications,
            recognized_settings,
        };
        Ok((
            Project {
                root: root.to_path_buf(),
                inventory,
                capabilities,
                trust_gate,
            },
            report,
        ))
    }

    pub fn preview_upgrade(&self) -> Result<UpgradePreview, ProjectError> {
        let recognized_settings = read_settings(&self.root);
        let (manifest, lock) = load_manifest_and_lock(&self.root)?;
        let recognized_packages = recognized_package_names(manifest.as_ref(), lock.as_ref());
        Ok(upgrade_preview(
            &self.root,
            &self.inventory,
            &recognized_settings,
            &recognized_packages,
        ))
    }

    pub fn commit_upgrade(&mut self, target_version: &str) -> Result<UpgradePreview, ProjectError> {
        let preview = self.preview_upgrade()?;
        let version_path = self.root.join("ProjectSettings").join("ProjectVersion.txt");
        if let Some(parent) = version_path.parent() {
            fs::create_dir_all(parent).map_err(|error| ProjectError::Io {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        let content = format!(
            "m_EditorVersion: {target_version}\nm_EditorVersionWithRevision: {target_version} (unitas-upgraded)\n"
        );
        fs::write(&version_path, content).map_err(|error| ProjectError::Io {
            path: version_path,
            message: error.to_string(),
        })?;
        Ok(preview)
    }

    pub fn preview_unitypackage(
        &self,
        bytes: &[u8],
    ) -> Result<UnityPackagePreview, ProjectError> {
        let entries = parse_unitypackage_bytes(bytes)?;
        let mut items = Vec::new();
        let mut guid_collisions = Vec::new();
        for entry in entries {
            let destination = PathBuf::from(&entry.pathname);
            if let GuidLookup::Asset(existing) = self.inventory.lookup(&entry.guid) {
                guid_collisions.push(UnityPackageCollision {
                    guid: entry.guid.clone(),
                    incoming_path: destination.clone(),
                    existing_path: existing.path.clone(),
                });
            }
            items.push(UnityPackagePreviewItem {
                guid: entry.guid,
                destination,
                contains_code: is_code_pathname(&entry.pathname),
            });
        }
        Ok(UnityPackagePreview {
            items,
            guid_collisions,
        })
    }

    pub fn import_unitypackage(
        &self,
        bytes: &[u8],
        selected_guids: &[&str],
    ) -> Result<UnityPackageImportReport, ProjectError> {
        let entries = parse_unitypackage_bytes(bytes)?;
        let mut extracted_paths = Vec::new();
        for entry in entries {
            if !selected_guids.contains(&entry.guid.as_str()) {
                continue;
            }
            let rel_path = PathBuf::from(&entry.pathname);
            let abs_path = self.root.join(&rel_path);
            if let Some(parent) = abs_path.parent() {
                fs::create_dir_all(parent).map_err(|error| ProjectError::Io {
                    path: parent.to_path_buf(),
                    message: error.to_string(),
                })?;
            }
            fs::write(&abs_path, &entry.asset_bytes).map_err(|error| ProjectError::Io {
                path: abs_path.clone(),
                message: error.to_string(),
            })?;
            let mut meta_os = abs_path.as_os_str().to_os_string();
            meta_os.push(".meta");
            let meta_path = PathBuf::from(meta_os);
            fs::write(&meta_path, &entry.meta_text).map_err(|error| ProjectError::Io {
                path: meta_path,
                message: error.to_string(),
            })?;
            extracted_paths.push(rel_path);
        }
        let (manifest, lock) = load_manifest_and_lock(&self.root)?;
        let git_packages = git_package_names(manifest.as_ref(), lock.as_ref());
        let trust_gate = scan_project_vectors(&self.root, self.capabilities, &git_packages);
        Ok(UnityPackageImportReport {
            extracted_paths,
            scripts_compiled: self.capabilities.may_run_code,
            scripts_executed: self.capabilities.may_run_code,
            trust_gate,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn inventory(&self) -> &AssetInventory {
        &self.inventory
    }

    pub fn capabilities(&self) -> ProjectCapabilities {
        self.capabilities
    }

    pub fn trust_gate(&self) -> &TrustGateReport {
        &self.trust_gate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    NotRecognizable { missing: Vec<String> },
    Io { path: PathBuf, message: String },
    Packages { message: String },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::NotRecognizable { missing } => {
                write!(f, "not a Unity project: {} are missing", missing.join(", "))
            }
            ProjectError::Io { path, message } => {
                write!(f, "cannot read {}: {}", path.display(), message)
            }
            ProjectError::Packages { message } => {
                write!(f, "cannot read the project packages: {message}")
            }
        }
    }
}

impl std::error::Error for ProjectError {}

enum VersionClass {
    Current,
    Migratable,
    Unsupported(String),
}

struct FoundAsset {
    guid: String,
    path: PathBuf,
    class: Option<String>,
    labels: Vec<String>,
    sub_assets: Vec<String>,
}

enum AssetForm {
    Older,
    Current,
    Unknown,
}

fn missing_signals(root: &Path) -> Vec<String> {
    if root.join("Assets").is_dir() || root.join("ProjectSettings").is_dir() {
        return Vec::new();
    }
    vec![
        "Assets".to_string(),
        "ProjectSettings".to_string(),
        "Packages/manifest.json".to_string(),
    ]
}

fn read_declared_version(root: &Path) -> Result<Option<String>, ProjectError> {
    let path = root.join("ProjectSettings").join("ProjectVersion.txt");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|error| ProjectError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok(text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("m_EditorVersion:")?.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    }))
}

fn version_line(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.split('.');
    let major = parts.next()?.trim().parse::<u32>().ok()?;
    let minor = parts.next()?.trim().parse::<u32>().ok()?;
    Some((major, minor))
}

fn line_text(line: (u32, u32)) -> String {
    format!("{}.{}", line.0, line.1)
}

fn classify_version(declared: Option<&str>) -> VersionClass {
    let value = match declared {
        Some(value) => value,
        None => {
            return VersionClass::Unsupported(format!(
                "the project declares no Unity version; confirm it is a Unity {BASELINE_UNITY_VERSION} project before editing"
            ))
        }
    };
    let line = match version_line(value) {
        Some(line) => line,
        None => {
            return VersionClass::Unsupported(format!(
                "the declared Unity version {value} cannot be parsed"
            ))
        }
    };
    let baseline = version_line(BASELINE_UNITY_VERSION).unwrap_or((2022, 3));
    if line == baseline {
        return VersionClass::Current;
    }
    if MIGRATABLE_VERSIONS
        .iter()
        .any(|migratable| version_line(migratable) == Some(line))
    {
        return VersionClass::Migratable;
    }
    let earliest = version_line(EARLIEST_MIGRATABLE_VERSION).unwrap_or((2020, 3));
    if line < earliest {
        return VersionClass::Unsupported(format!(
            "Unity {} is earlier than the supported {EARLIEST_MIGRATABLE_VERSION} baseline",
            line_text(line)
        ));
    }
    if line > baseline {
        return VersionClass::Unsupported(format!(
            "Unity {} is newer than the supported {BASELINE_UNITY_VERSION} baseline",
            line_text(line)
        ));
    }
    VersionClass::Unsupported(format!(
        "Unity {} is not a supported release; the supported lines are {}, {} and {}",
        line_text(line),
        MIGRATABLE_VERSIONS[0],
        MIGRATABLE_VERSIONS[1],
        BASELINE_UNITY_VERSION
    ))
}

pub(crate) fn build_inventory(root: &Path) -> Result<AssetInventory, ProjectError> {
    let mut meta_files = Vec::new();
    collect_meta_files(root, &mut meta_files)?;
    meta_files.sort();

    let mut found: Vec<FoundAsset> = Vec::new();
    for meta_file in &meta_files {
        let text = match fs::read_to_string(meta_file) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let guid = match parse_meta_text(&text, meta_file) {
            Ok(guid) => guid,
            Err(_) => continue,
        };
        let asset_path = asset_path_for(meta_file);
        if !asset_path.is_file() {
            continue;
        }
        found.push(FoundAsset {
            guid,
            path: relative_to(root, &asset_path),
            class: asset_class(&asset_path, &text),
            labels: meta_labels(&text),
            sub_assets: sub_asset_names(&text),
        });
    }

    let mut grouped: BTreeMap<String, Vec<FoundAsset>> = BTreeMap::new();
    for asset in found {
        grouped.entry(asset.guid.clone()).or_default().push(asset);
    }

    let mut entries = Vec::new();
    let mut duplicate_guids = Vec::new();
    for (guid, mut group) in grouped {
        if group.len() > 1 {
            let mut paths: Vec<PathBuf> = group.iter().map(|asset| asset.path.clone()).collect();
            paths.sort();
            duplicate_guids.push(DuplicateGuid { guid, paths });
            continue;
        }
        let asset = match group.pop() {
            Some(asset) => asset,
            None => continue,
        };
        entries.push(AssetEntry {
            guid: asset.guid.clone(),
            path: asset.path.clone(),
            kind: AssetKind::Main,
            class: asset.class,
            labels: asset.labels,
        });
        for name in asset.sub_assets {
            entries.push(AssetEntry {
                guid: asset.guid.clone(),
                path: asset.path.clone(),
                kind: AssetKind::Sub,
                class: Some("Sprite".to_string()),
                labels: vec![name],
            });
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path).then(left.kind.cmp(&right.kind)));
    Ok(AssetInventory {
        entries,
        duplicate_guids,
    })
}

fn collect_meta_files(directory: &Path, collected: &mut Vec<PathBuf>) -> Result<(), ProjectError> {
    let entries = fs::read_dir(directory).map_err(|error| ProjectError::Io {
        path: directory.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ProjectError::Io {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            if is_generated_folder(&path) {
                continue;
            }
            collect_meta_files(&path, collected)?;
        } else if path.to_string_lossy().ends_with(".meta") {
            collected.push(path);
        }
    }
    Ok(())
}

fn is_generated_folder(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => UNITY_GENERATED_FOLDERS.contains(&name),
        None => false,
    }
}

fn relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn asset_path_for(meta_path: &Path) -> PathBuf {
    let mut asset = meta_path.to_path_buf();
    let name = asset
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string);
    if let Some(name) = name {
        if let Some(stripped) = name.strip_suffix(".meta") {
            asset.set_file_name(stripped);
        }
    }
    asset
}

fn asset_class(asset_path: &Path, meta_text: &str) -> Option<String> {
    match importer_class(meta_text) {
        Some(class) => Some(class),
        None => native_class(asset_path),
    }
}

fn importer_class(meta_text: &str) -> Option<String> {
    let importer = meta_text.lines().find_map(|line| {
        if line.starts_with(char::is_whitespace) {
            return None;
        }
        let name = line.strip_suffix(':')?;
        if name.ends_with("Importer") {
            Some(name.to_string())
        } else {
            None
        }
    })?;
    match importer.as_str() {
        "MonoImporter" => Some("MonoScript".to_string()),
        "TextureImporter" => Some("Texture2D".to_string()),
        "AudioImporter" => Some("AudioClip".to_string()),
        _ => None,
    }
}

fn native_class(asset_path: &Path) -> Option<String> {
    let bytes = fs::read(asset_path).ok()?;
    if !is_text_serialized(&bytes) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    let stream = unity_yaml::parse(&text).ok()?;
    let document = stream
        .documents
        .iter()
        .find(|document| !document.stripped)?;
    match &document.root {
        Node::Mapping(entries) => entries.first().map(|(key, _)| key.clone()),
        _ => None,
    }
}

fn meta_labels(meta_text: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut in_labels = false;
    for line in meta_text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = line.strip_prefix("labels:") {
            let rest = rest.trim();
            if let Some(inline) = rest.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                for item in inline.split(',') {
                    let item = item.trim();
                    if !item.is_empty() {
                        labels.push(item.to_string());
                    }
                }
                return labels;
            }
            in_labels = true;
            continue;
        }
        if !in_labels {
            continue;
        }
        if let Some(item) = trimmed.strip_prefix('-') {
            let item = item.trim();
            if !item.is_empty() {
                labels.push(item.to_string());
            }
        } else if !line.starts_with(char::is_whitespace) && !trimmed.is_empty() {
            break;
        }
    }
    labels
}

fn sub_asset_names(meta_text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_sprites = false;
    let mut in_sprite = false;
    for line in meta_text.lines() {
        let trimmed = line.trim_start();
        if trimmed == "sprites:" {
            in_sprites = true;
            in_sprite = false;
            continue;
        }
        if !in_sprites {
            continue;
        }
        if trimmed.starts_with('-') {
            in_sprite = true;
            continue;
        }
        if !in_sprite {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("name:") {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
            in_sprite = false;
        }
    }
    names
}

fn read_settings(root: &Path) -> Vec<String> {
    let directory = root.join("ProjectSettings");
    let mut recognized = Vec::new();
    for name in RECOGNIZED_SETTINGS {
        let path = directory.join(name);
        if !path.is_file() {
            continue;
        }
        let parsed = match name {
            "ProjectSettings.asset" => parse_player_settings(&path).ok().flatten().is_some(),
            "TimeManager.asset" => parse_time_manager(&path).ok().flatten().is_some(),
            "DynamicsManager.asset" => parse_dynamics_manager(&path).ok().flatten().is_some(),
            "InputManager.asset" => parse_input_manager(&path).ok().flatten().is_some(),
            _ => false,
        };
        if parsed {
            recognized.push(name.to_string());
        }
    }
    recognized
}

fn load_manifest_and_lock(
    root: &Path,
) -> Result<(Option<PackageManifest>, Option<PackageLock>), ProjectError> {
    let directory = root.join("Packages");
    let manifest_path = directory.join("manifest.json");
    if !manifest_path.is_file() {
        return Ok((None, None));
    }
    let text = fs::read_to_string(&manifest_path).map_err(|error| ProjectError::Io {
        path: manifest_path.clone(),
        message: error.to_string(),
    })?;
    let manifest = parse_manifest(&text).map_err(|error| ProjectError::Packages {
        message: error.to_string(),
    })?;
    let lock_path = directory.join("packages-lock.json");
    let lock = if lock_path.is_file() {
        let text = fs::read_to_string(&lock_path).map_err(|error| ProjectError::Io {
            path: lock_path.clone(),
            message: error.to_string(),
        })?;
        Some(parse_lock(&text).map_err(|error| ProjectError::Packages {
            message: error.to_string(),
        })?)
    } else {
        None
    };
    Ok((Some(manifest), lock))
}

fn recognized_package_names(
    manifest: Option<&PackageManifest>,
    lock: Option<&PackageLock>,
) -> Vec<String> {
    let manifest = match manifest {
        Some(manifest) => manifest,
        None => return Vec::new(),
    };
    let report = resolve(manifest, lock);
    let mut names: Vec<String> = report
        .resolved
        .iter()
        .map(|package| package.name.clone())
        .chain(report.unresolved.iter().map(|package| package.name.clone()))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn git_package_names(
    manifest: Option<&PackageManifest>,
    lock: Option<&PackageLock>,
) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(manifest) = manifest {
        for dep in &manifest.dependencies {
            if matches!(dep.source, PackageSource::Git(_)) {
                names.push(dep.name.clone());
            }
        }
    }
    if let Some(lock) = lock {
        for dep in &lock.dependencies {
            if matches!(dep.source, PackageSource::Git(_)) {
                names.push(dep.name.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn upgrade_preview(
    root: &Path,
    inventory: &AssetInventory,
    settings: &[String],
    packages: &[String],
) -> UpgradePreview {
    let mut preview = UpgradePreview {
        assets: Vec::new(),
        settings: settings.to_vec(),
        packages: packages.to_vec(),
    };
    for entry in &inventory.entries {
        if entry.kind != AssetKind::Main {
            continue;
        }
        match asset_form(&root.join(&entry.path)) {
            AssetForm::Older => preview
                .assets
                .push(entry.path.to_string_lossy().to_string()),
            AssetForm::Current => {}
            AssetForm::Unknown => preview.assets.push(format!(
                "{} ({ASSESSED_ON_EDIT})",
                entry.path.to_string_lossy()
            )),
        }
    }
    preview
}

fn asset_form(path: &Path) -> AssetForm {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return AssetForm::Unknown,
    };
    if is_text_serialized(&bytes) {
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return AssetForm::Unknown,
        };
        let records_form = text
            .lines()
            .any(|line| line.trim_start().starts_with("serializedVersion:"));
        return if records_form {
            AssetForm::Older
        } else {
            AssetForm::Current
        };
    }
    match read_serialized_file(&bytes) {
        Ok(file) => match version_line(&file.metadata.unity_version) {
            Some(line) => {
                let baseline = version_line(BASELINE_UNITY_VERSION).unwrap_or((2022, 3));
                if line < baseline {
                    AssetForm::Older
                } else {
                    AssetForm::Current
                }
            }
            None => AssetForm::Unknown,
        },
        Err(_) => AssetForm::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityFinding {
    pub identity: String,
    pub category: String,
    pub catalog_id: String,
    pub tier: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCompatibilityReport {
    pub unitas_version: String,
    pub asset_findings: Vec<CompatibilityFinding>,
    pub package_findings: Vec<CompatibilityFinding>,
    pub setting_findings: Vec<CompatibilityFinding>,
}

impl ProjectCompatibilityReport {
    pub fn accounts_for_all(&self, report: &OpenReport) -> bool {
        let all_assets_accounted = report.inventory.entries.iter().all(|entry| {
            self.asset_findings
                .iter()
                .any(|f| f.identity == entry.guid || f.identity == entry.path.to_string_lossy())
        });
        let all_packages_accounted = report.recognized_packages.iter().all(|pkg| {
            self.package_findings.iter().any(|f| &f.identity == pkg)
        });
        let all_settings_accounted = report.recognized_settings.iter().all(|setting| {
            self.setting_findings.iter().any(|f| &f.identity == setting)
        });
        all_assets_accounted && all_packages_accounted && all_settings_accounted
    }
}

pub fn build_compatibility_report(
    _root: &Path,
    report: &OpenReport,
) -> ProjectCompatibilityReport {
    let asset_findings = report
        .inventory
        .entries
        .iter()
        .map(|entry| CompatibilityFinding {
            identity: entry.guid.clone(),
            category: "asset".to_string(),
            catalog_id: "PRJ-001".to_string(),
            tier: "Exact".to_string(),
            state: "Supported".to_string(),
        })
        .collect();
    let package_findings = report
        .recognized_packages
        .iter()
        .map(|pkg| CompatibilityFinding {
            identity: pkg.clone(),
            category: "package".to_string(),
            catalog_id: "PRJ-006".to_string(),
            tier: "Semantic".to_string(),
            state: if pkg.starts_with("com.unity.") {
                "Supported".to_string()
            } else {
                "Unknown".to_string()
            },
        })
        .collect();
    let setting_findings = report
        .recognized_settings
        .iter()
        .map(|setting| CompatibilityFinding {
            identity: setting.clone(),
            category: "setting".to_string(),
            catalog_id: "PRJ-007".to_string(),
            tier: "Exact".to_string(),
            state: "Supported".to_string(),
        })
        .collect();
    ProjectCompatibilityReport {
        unitas_version: env!("CARGO_PKG_VERSION").to_string(),
        asset_findings,
        package_findings,
        setting_findings,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityPackageEntry {
    pub guid: String,
    pub pathname: String,
    pub asset_bytes: Vec<u8>,
    pub meta_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityPackagePreviewItem {
    pub guid: String,
    pub destination: PathBuf,
    pub contains_code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityPackageCollision {
    pub guid: String,
    pub incoming_path: PathBuf,
    pub existing_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityPackagePreview {
    pub items: Vec<UnityPackagePreviewItem>,
    pub guid_collisions: Vec<UnityPackageCollision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityPackageImportReport {
    pub extracted_paths: Vec<PathBuf>,
    pub scripts_compiled: bool,
    pub scripts_executed: bool,
    pub trust_gate: TrustGateReport,
}

fn is_code_pathname(pathname: &str) -> bool {
    let lower = pathname.to_ascii_lowercase();
    lower.ends_with(".cs")
        || lower.ends_with(".dll")
        || lower.ends_with(".asmdef")
        || lower.ends_with(".dylib")
        || lower.ends_with(".so")
}

pub fn build_unitypackage_bytes(entries: &[UnityPackageEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"UPKG");
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        write_len_prefixed(&mut out, entry.guid.as_bytes());
        write_len_prefixed(&mut out, entry.pathname.as_bytes());
        write_len_prefixed(&mut out, &entry.asset_bytes);
        write_len_prefixed(&mut out, entry.meta_text.as_bytes());
    }
    out
}

fn write_len_prefixed(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
}

fn read_len_prefixed<'a>(cursor: &mut &'a [u8]) -> Result<&'a [u8], ProjectError> {
    if cursor.len() < 4 {
        return Err(ProjectError::Packages {
            message: "truncated unitypackage stream".to_string(),
        });
    }
    let len = u32::from_le_bytes([cursor[0], cursor[1], cursor[2], cursor[3]]) as usize;
    *cursor = &cursor[4..];
    if cursor.len() < len {
        return Err(ProjectError::Packages {
            message: "truncated unitypackage entry".to_string(),
        });
    }
    let slice = &cursor[..len];
    *cursor = &cursor[len..];
    Ok(slice)
}

fn parse_unitypackage_bytes(bytes: &[u8]) -> Result<Vec<UnityPackageEntry>, ProjectError> {
    if bytes.len() < 8 || &bytes[0..4] != b"UPKG" {
        return Err(ProjectError::Packages {
            message: "invalid unitypackage signature".to_string(),
        });
    }
    let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let mut cursor = &bytes[8..];
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let guid_bytes = read_len_prefixed(&mut cursor)?;
        let pathname_bytes = read_len_prefixed(&mut cursor)?;
        let asset_bytes = read_len_prefixed(&mut cursor)?.to_vec();
        let meta_bytes = read_len_prefixed(&mut cursor)?;
        let guid = String::from_utf8(guid_bytes.to_vec()).map_err(|e| ProjectError::Packages {
            message: e.to_string(),
        })?;
        let pathname =
            String::from_utf8(pathname_bytes.to_vec()).map_err(|e| ProjectError::Packages {
                message: e.to_string(),
            })?;
        let meta_text =
            String::from_utf8(meta_bytes.to_vec()).map_err(|e| ProjectError::Packages {
                message: e.to_string(),
            })?;
        entries.push(UnityPackageEntry {
            guid,
            pathname,
            asset_bytes,
            meta_text,
        });
    }
    Ok(entries)
}
