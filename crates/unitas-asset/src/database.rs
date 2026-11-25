use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::project::{build_inventory, AssetEntry, AssetKind, Project};

const TRASH_DIRECTORY: &str = ".unitas/trash";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetFilter {
    pub name_contains: Option<String>,
    pub class: Option<String>,
    pub label: Option<String>,
    pub include_sub_assets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseError {
    NotFound { path: PathBuf },
    DestinationExists { path: PathBuf },
    Io { path: PathBuf, message: String },
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::NotFound { path } => {
                write!(f, "asset not found: {}", path.display())
            }
            DatabaseError::DestinationExists { path } => {
                write!(f, "destination already exists: {}", path.display())
            }
            DatabaseError::Io { path, message } => {
                write!(f, "cannot access {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

impl Project {
    pub fn search_assets(&self, filter: &AssetFilter) -> Vec<&AssetEntry> {
        self.inventory
            .entries
            .iter()
            .filter(|entry| matches_filter(entry, filter))
            .collect()
    }

    pub fn move_asset(&mut self, from: &Path, to: &Path) -> Result<(), DatabaseError> {
        let from_rel = self.relative_path(from);
        let to_rel = self.relative_path(to);
        let from_asset = self.root.join(&from_rel);
        let from_meta = meta_path_for(&from_asset);
        let to_asset = self.root.join(&to_rel);
        let to_meta = meta_path_for(&to_asset);

        if !from_asset.is_file() || !from_meta.is_file() {
            return Err(DatabaseError::NotFound { path: from_rel });
        }
        if to_asset.exists() || to_meta.exists() {
            return Err(DatabaseError::DestinationExists { path: to_rel });
        }

        relocate_pair(&from_asset, &from_meta, &to_asset, &to_meta)?;
        self.refresh_inventory()
    }

    pub fn delete_asset(&mut self, path: &Path) -> Result<(), DatabaseError> {
        let rel = self.relative_path(path);
        let asset_path = self.root.join(&rel);
        let meta_path = meta_path_for(&asset_path);

        if !asset_path.is_file() || !meta_path.is_file() {
            return Err(DatabaseError::NotFound { path: rel });
        }

        let trash_asset = self.trash_root().join(&rel);
        let trash_meta = meta_path_for(&trash_asset);
        if trash_asset.exists() {
            let _ = fs::remove_file(&trash_asset);
        }
        if trash_meta.exists() {
            let _ = fs::remove_file(&trash_meta);
        }

        relocate_pair(&asset_path, &meta_path, &trash_asset, &trash_meta)?;
        self.refresh_inventory()
    }

    pub fn restore_asset(&mut self, path: &Path) -> Result<(), DatabaseError> {
        let rel = self.relative_path(path);
        let trash_asset = self.trash_root().join(&rel);
        let trash_meta = meta_path_for(&trash_asset);
        let dest_asset = self.root.join(&rel);
        let dest_meta = meta_path_for(&dest_asset);

        if !trash_asset.is_file() || !trash_meta.is_file() {
            return Err(DatabaseError::NotFound { path: rel });
        }
        if dest_asset.exists() || dest_meta.exists() {
            return Err(DatabaseError::DestinationExists { path: rel });
        }

        relocate_pair(&trash_asset, &trash_meta, &dest_asset, &dest_meta)?;
        self.refresh_inventory()
    }

    fn relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf())
    }

    fn trash_root(&self) -> PathBuf {
        self.root.join(TRASH_DIRECTORY)
    }

    fn refresh_inventory(&mut self) -> Result<(), DatabaseError> {
        self.inventory = build_inventory(&self.root).map_err(|error| DatabaseError::Io {
            path: self.root.clone(),
            message: error.to_string(),
        })?;
        Ok(())
    }
}

fn meta_path_for(asset_path: &Path) -> PathBuf {
    let mut name = asset_path.as_os_str().to_os_string();
    name.push(".meta");
    PathBuf::from(name)
}

fn relocate_pair(
    from_asset: &Path,
    from_meta: &Path,
    to_asset: &Path,
    to_meta: &Path,
) -> Result<(), DatabaseError> {
    if let Some(parent) = to_asset.parent() {
        fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::rename(from_asset, to_asset).map_err(|error| DatabaseError::Io {
        path: from_asset.to_path_buf(),
        message: error.to_string(),
    })?;
    if let Err(error) = fs::rename(from_meta, to_meta) {
        let _ = fs::rename(to_asset, from_asset);
        return Err(DatabaseError::Io {
            path: from_meta.to_path_buf(),
            message: error.to_string(),
        });
    }
    Ok(())
}

fn matches_filter(entry: &AssetEntry, filter: &AssetFilter) -> bool {
    if !filter.include_sub_assets && entry.kind != AssetKind::Main {
        return false;
    }
    if let Some(expected_class) = &filter.class {
        if entry.class.as_deref() != Some(expected_class.as_str()) {
            return false;
        }
    }
    if let Some(expected_label) = &filter.label {
        if !entry.labels.iter().any(|label| label == expected_label) {
            return false;
        }
    }
    if let Some(needle) = &filter.name_contains {
        if !matches_name(entry, needle) {
            return false;
        }
    }
    true
}

fn matches_name(entry: &AssetEntry, needle: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    match entry.kind {
        AssetKind::Main => entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().contains(&needle_lower)),
        AssetKind::Sub => {
            entry
                .labels
                .iter()
                .any(|label| label.to_ascii_lowercase().contains(&needle_lower))
                || entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().contains(&needle_lower))
        }
    }
}
