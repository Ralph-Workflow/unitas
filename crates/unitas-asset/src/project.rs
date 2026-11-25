
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
use crate::trust::{scan_project_vectors, ProjectCapabilities, TrustGateReport, TrustStore};
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
        if let Some(duplicate) = self.duplicate_guids.iter().find(|record| record.guid == guid) {
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
        let capabilities = trust.capabilities(&root.to_string_lossy());
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
    entries.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.kind.cmp(&right.kind))
    });
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
    let document = stream.documents.iter().find(|document| !document.stripped)?;
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
    let manifest =
        parse_manifest(&text).map_err(|error| ProjectError::Packages { message: error.to_string() })?;
    let lock_path = directory.join("packages-lock.json");
    let lock = if lock_path.is_file() {
        let text = fs::read_to_string(&lock_path).map_err(|error| ProjectError::Io {
            path: lock_path.clone(),
            message: error.to_string(),
        })?;
        Some(
            parse_lock(&text)
                .map_err(|error| ProjectError::Packages { message: error.to_string() })?,
        )
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
