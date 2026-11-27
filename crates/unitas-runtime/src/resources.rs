use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_asset::meta::{MetaError, parse_meta_file};

const RESOURCES_DIR_LOWER: &str = "resources";
const META_SUFFIX_LOWER: &str = ".meta";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Text,
    Prefab,
    Scene,
    Material,
    AudioClip,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEntry {
    pub resource_path: String,
    pub canonical_key: String,
    pub file_path: PathBuf,
    pub extension: String,
    pub kind: ResourceKind,
    pub guid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourcesError {
    Read { path: PathBuf, message: String },
    Meta(MetaError),
}

impl fmt::Display for ResourcesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourcesError::Read { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            ResourcesError::Meta(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ResourcesError {}

impl From<MetaError> for ResourcesError {
    fn from(error: MetaError) -> Self {
        ResourcesError::Meta(error)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourcesIndex {
    entries: BTreeMap<String, Vec<ResourceEntry>>,
}

impl ResourcesIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan(root: &Path) -> Result<Self, ResourcesError> {
        let mut index = ResourcesIndex::new();
        if !root.exists() {
            return Ok(index);
        }
        let mut resources_dirs = Vec::new();
        collect_resources_dirs(root, &mut resources_dirs)?;
        resources_dirs.sort();
        for resources_dir in resources_dirs {
            let mut files = Vec::new();
            collect_resource_files(&resources_dir, &mut files)?;
            files.sort();
            for file_path in files {
                if let Some(entry) = build_resource_entry(&resources_dir, &file_path) {
                    index.insert(entry);
                }
            }
        }
        Ok(index)
    }

    pub fn insert(&mut self, entry: ResourceEntry) {
        self.entries
            .entry(entry.canonical_key.clone())
            .or_default()
            .push(entry);
    }

    pub fn load(&self, path: &str) -> Option<&ResourceEntry> {
        let key = normalize_resource_key(path);
        if key.is_empty() {
            return None;
        }
        self.entries.get(&key).and_then(|list| list.first())
    }

    pub fn load_with_kind(&self, path: &str, kind: ResourceKind) -> Option<&ResourceEntry> {
        let key = normalize_resource_key(path);
        if key.is_empty() {
            return None;
        }
        self.entries
            .get(&key)
            .and_then(|list| list.iter().find(|entry| entry.kind == kind))
    }

    pub fn load_all(&self, path: &str) -> Vec<&ResourceEntry> {
        let prefix = normalize_resource_key(path);
        let mut matches = Vec::new();
        for (key, list) in &self.entries {
            let included = if prefix.is_empty() || *key == prefix {
                true
            } else {
                key.starts_with(&prefix) && key.as_bytes().get(prefix.len()) == Some(&b'/')
            };
            if included {
                for entry in list {
                    matches.push(entry);
                }
            }
        }
        matches
    }

    pub fn load_all_with_kind(&self, path: &str, kind: ResourceKind) -> Vec<&ResourceEntry> {
        self.load_all(path)
            .into_iter()
            .filter(|entry| entry.kind == kind)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.entries.values().flat_map(|list| list.iter())
    }
}

pub fn normalize_resource_path(path: &str) -> String {
    let replaced = path.trim().replace('\\', "/");
    let mut segments = Vec::new();
    for segment in replaced.split('/') {
        let trimmed = segment.trim();
        if !trimmed.is_empty() && trimmed != "." {
            segments.push(trimmed);
        }
    }
    segments.join("/")
}

pub fn normalize_resource_key(path: &str) -> String {
    normalize_resource_path(path).to_ascii_lowercase()
}

pub fn classify_resource_extension(extension: &str) -> ResourceKind {
    match extension.to_ascii_lowercase().as_str() {
        "txt" | "json" | "xml" | "csv" | "yaml" | "yml" | "htm" | "html" | "fnt" | "md"
        | "bytes" => ResourceKind::Text,
        "prefab" => ResourceKind::Prefab,
        "unity" => ResourceKind::Scene,
        "mat" | "physicmaterial" => ResourceKind::Material,
        "wav" | "ogg" | "mp3" | "aiff" | "aif" => ResourceKind::AudioClip,
        _ => ResourceKind::Binary,
    }
}

pub fn read_resource_bytes(entry: &ResourceEntry) -> Result<Vec<u8>, ResourcesError> {
    fs::read(&entry.file_path).map_err(|error| ResourcesError::Read {
        path: entry.file_path.clone(),
        message: error.to_string(),
    })
}

pub fn read_resource_text(entry: &ResourceEntry) -> Result<String, ResourcesError> {
    fs::read_to_string(&entry.file_path).map_err(|error| ResourcesError::Read {
        path: entry.file_path.clone(),
        message: error.to_string(),
    })
}

fn collect_resources_dirs(
    directory: &Path,
    collected: &mut Vec<PathBuf>,
) -> Result<(), ResourcesError> {
    let entries = fs::read_dir(directory).map_err(|error| ResourcesError::Read {
        path: directory.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut subdirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| ResourcesError::Read {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        }
    }
    subdirs.sort();
    for path in subdirs {
        let is_resources = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(RESOURCES_DIR_LOWER))
            .unwrap_or(false);
        if is_resources {
            collected.push(path.clone());
        }
        collect_resources_dirs(&path, collected)?;
    }
    Ok(())
}

fn collect_resource_files(
    directory: &Path,
    collected: &mut Vec<PathBuf>,
) -> Result<(), ResourcesError> {
    let entries = fs::read_dir(directory).map_err(|error| ResourcesError::Read {
        path: directory.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| ResourcesError::Read {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        if path.is_dir() {
            let is_nested_resources = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case(RESOURCES_DIR_LOWER))
                .unwrap_or(false);
            if !is_nested_resources {
                collect_resource_files(&path, collected)?;
            }
        } else if !is_ignored_file(&path) {
            collected.push(path);
        }
    }
    Ok(())
}

fn is_ignored_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if name.starts_with('.') {
        return true;
    }
    name.to_ascii_lowercase().ends_with(META_SUFFIX_LOWER)
}

fn build_resource_entry(resources_dir: &Path, file_path: &Path) -> Option<ResourceEntry> {
    let relative = file_path.strip_prefix(resources_dir).ok()?;
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string();
    let without_ext = relative.with_extension("");
    let mut segments = Vec::new();
    for component in without_ext.components() {
        let text = component.as_os_str().to_str()?;
        if !text.is_empty() {
            segments.push(text);
        }
    }
    if segments.is_empty() {
        return None;
    }
    let resource_path = segments.join("/");
    let canonical_key = resource_path.to_ascii_lowercase();
    let kind = classify_resource_extension(&extension);
    let meta_path = meta_path_for(file_path);
    let guid = if meta_path.is_file() {
        parse_meta_file(&meta_path).ok()
    } else {
        None
    };
    Some(ResourceEntry {
        resource_path,
        canonical_key,
        file_path: file_path.to_path_buf(),
        extension,
        kind,
        guid,
    })
}

fn meta_path_for(file_path: &Path) -> PathBuf {
    let mut os = file_path.as_os_str().to_os_string();
    os.push(".meta");
    PathBuf::from(os)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "unitas_resources_{}_{}_{}",
            label,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn scan_discovers_nested_resources_strips_extensions_and_ignores_meta()
    -> Result<(), Box<dyn Error>> {
        let root = unique_temp_dir("scan")?;
        let res_a = root.join("Assets").join("Resources").join("Config");
        let res_b = root
            .join("Assets")
            .join("Gameplay")
            .join("Resources")
            .join("Prefabs");
        fs::create_dir_all(&res_a)?;
        fs::create_dir_all(&res_b)?;

        let json_path = res_a.join("balance.json");
        let meta_path = res_a.join("balance.json.meta");
        let prefab_path = res_b.join("Enemy.prefab");
        let outside_path = root.join("Assets").join("Gameplay").join("Ignored.txt");

        fs::write(&json_path, "{\"hp\":100}")?;
        fs::write(
            &meta_path,
            "fileFormatVersion: 2\nguid: 10000000000000000000000000000001\n",
        )?;
        fs::write(
            &prefab_path,
            "%YAML 1.1\n--- !u!1 &1\nGameObject:\n  m_Name: Enemy\n",
        )?;
        fs::write(&outside_path, "not in resources")?;

        let index = ResourcesIndex::scan(&root)?;
        let _ = fs::remove_dir_all(&root);

        assert_eq!(index.len(), 2);

        let Some(balance) = index.load("Config/balance") else {
            return Err("expected Config/balance to resolve".into());
        };
        assert_eq!(balance.resource_path, "Config/balance");
        assert_eq!(balance.kind, ResourceKind::Text);
        assert_eq!(
            balance.guid.as_deref(),
            Some("10000000000000000000000000000001")
        );

        let Some(balance_backslash) = index.load("config\\BALANCE") else {
            return Err("expected case-insensitive backslash lookup to resolve".into());
        };
        assert_eq!(balance_backslash.file_path, json_path);

        let Some(enemy) = index.load("Prefabs/Enemy") else {
            return Err("expected Prefabs/Enemy to resolve".into());
        };
        assert_eq!(enemy.kind, ResourceKind::Prefab);
        assert_eq!(enemy.guid, None);

        assert!(index.load("Missing/Path").is_none());
        assert!(index.load("").is_none());
        assert!(
            index
                .load_with_kind("Config/balance", ResourceKind::Prefab)
                .is_none()
        );
        assert!(
            index
                .load_with_kind("Config/balance", ResourceKind::Text)
                .is_some()
        );

        let all_in_config = index.load_all("Config");
        assert_eq!(all_in_config.len(), 1);
        let all_root = index.load_all("");
        assert_eq!(all_root.len(), 2);

        Ok(())
    }

    #[test]
    fn read_resource_text_and_bytes_return_file_contents() -> Result<(), Box<dyn Error>> {
        let root = unique_temp_dir("read")?;
        let res_dir = root.join("Assets").join("Resources");
        fs::create_dir_all(&res_dir)?;
        let file_path = res_dir.join("greeting.txt");
        fs::write(&file_path, "hello unitas")?;

        let index = ResourcesIndex::scan(&root)?;
        let Some(entry) = index.load("greeting") else {
            let _ = fs::remove_dir_all(&root);
            return Err("expected greeting resource".into());
        };
        let text = read_resource_text(entry)?;
        let bytes = read_resource_bytes(entry)?;
        let _ = fs::remove_dir_all(&root);

        assert_eq!(text, "hello unitas");
        assert_eq!(bytes, b"hello unitas");
        Ok(())
    }
}
