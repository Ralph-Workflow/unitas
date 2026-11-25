use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const GUID_KEY: &str = "guid:";
const META_SUFFIX: &str = ".meta";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaError {
    Read {
        path: PathBuf,
        message: String,
    },
    MissingGuid {
        path: PathBuf,
    },
    InvalidGuid {
        path: PathBuf,
        value: String,
    },
    DuplicateGuid {
        guid: String,
        path: PathBuf,
        existing: PathBuf,
    },
}

impl fmt::Display for MetaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaError::Read { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            MetaError::MissingGuid { path } => {
                write!(formatter, "{} has no guid field", path.display())
            }
            MetaError::InvalidGuid { path, value } => {
                write!(formatter, "{} has invalid guid {}", path.display(), value)
            }
            MetaError::DuplicateGuid {
                guid,
                path,
                existing,
            } => {
                write!(
                    formatter,
                    "guid {} in {} duplicates {}",
                    guid,
                    path.display(),
                    existing.display()
                )
            }
        }
    }
}

impl std::error::Error for MetaError {}

pub fn guid_from_meta_text(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.strip_prefix(GUID_KEY)?.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

pub fn is_valid_guid(guid: &str) -> bool {
    !guid.is_empty() && guid.chars().all(|character| character.is_ascii_hexdigit())
}

pub fn parse_meta_text(text: &str, path: &Path) -> Result<String, MetaError> {
    let value = match guid_from_meta_text(text) {
        Some(value) => value,
        None => {
            return Err(MetaError::MissingGuid {
                path: path.to_path_buf(),
            })
        }
    };
    if !is_valid_guid(&value) {
        return Err(MetaError::InvalidGuid {
            path: path.to_path_buf(),
            value,
        });
    }
    Ok(value)
}

pub fn parse_meta_file(path: &Path) -> Result<String, MetaError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            return Err(MetaError::Read {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        }
    };
    parse_meta_text(&text, path)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuidIndex {
    entries: BTreeMap<String, PathBuf>,
}

pub type MetaIndex = GuidIndex;

impl GuidIndex {
    pub fn build(root: &Path) -> Result<Self, MetaError> {
        let mut meta_files = Vec::new();
        collect_meta_files(root, &mut meta_files)?;
        meta_files.sort();
        let mut index = GuidIndex::default();
        for meta_file in meta_files {
            let guid = parse_meta_file(&meta_file)?;
            index.insert(guid, asset_path_for(&meta_file))?;
        }
        Ok(index)
    }

    pub fn insert(&mut self, guid: String, path: PathBuf) -> Result<(), MetaError> {
        if !is_valid_guid(&guid) {
            return Err(MetaError::InvalidGuid { path, value: guid });
        }
        if let Some(existing) = self.entries.get(&guid) {
            return Err(MetaError::DuplicateGuid {
                guid,
                path,
                existing: existing.clone(),
            });
        }
        self.entries.insert(guid, path);
        Ok(())
    }

    pub fn get(&self, guid: &str) -> Option<&Path> {
        self.entries.get(guid).map(PathBuf::as_path)
    }

    pub fn contains(&self, guid: &str) -> bool {
        self.entries.contains_key(guid)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Path)> {
        self.entries
            .iter()
            .map(|(guid, path)| (guid.as_str(), path.as_path()))
    }
}

fn collect_meta_files(directory: &Path, collected: &mut Vec<PathBuf>) -> Result<(), MetaError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(MetaError::Read {
                path: directory.to_path_buf(),
                message: error.to_string(),
            })
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return Err(MetaError::Read {
                    path: directory.to_path_buf(),
                    message: error.to_string(),
                })
            }
        };
        let path = entry.path();
        if path.is_dir() {
            collect_meta_files(&path, collected)?;
        } else if is_meta_file(&path) {
            collected.push(path);
        }
    }
    Ok(())
}

fn is_meta_file(path: &Path) -> bool {
    match path.file_name() {
        Some(name) => name.to_string_lossy().ends_with(META_SUFFIX),
        None => false,
    }
}

fn asset_path_for(meta_path: &Path) -> PathBuf {
    let mut asset = meta_path.to_path_buf();
    let name = match asset.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_string(),
        None => return asset,
    };
    if let Some(stripped) = name.strip_suffix(META_SUFFIX) {
        asset.set_file_name(stripped);
    }
    asset
}
