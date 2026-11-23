use std::path::{Path, PathBuf};

use unitas_asset::meta::{self, GuidIndex, MetaError};

const PROJECT_DIR: &str = "tests/fixtures/project";
const CAMERA_GUID: &str = "4e9b7d1f2c3a4b5c6d7e8f9a0b1c2d3e";
const MATERIAL_GUID: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f809";
const TEXTURE_GUID: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";

#[test]
fn meta_guid_is_parsed_from_text() -> Result<(), MetaError> {
    let text = "fileFormatVersion: 2\nguid: 4e9b7d1f2c3a4b5c6d7e8f9a0b1c2d3e\nfolderAsset: yes\n";
    let guid = meta::parse_meta_text(text, Path::new("Camera.prefab.meta"))?;
    assert_eq!(guid, CAMERA_GUID);
    Ok(())
}

#[test]
fn meta_guid_is_parsed_from_file() -> Result<(), MetaError> {
    let path = Path::new(PROJECT_DIR).join("Camera.prefab.meta");
    let guid = meta::parse_meta_file(&path)?;
    assert_eq!(guid, CAMERA_GUID);
    Ok(())
}

#[test]
fn meta_missing_guid_is_an_error() -> Result<(), MetaError> {
    let result = meta::parse_meta_text("fileFormatVersion: 2\n", Path::new("empty.meta"));
    assert!(matches!(result, Err(MetaError::MissingGuid { .. })));
    Ok(())
}

#[test]
fn meta_invalid_guid_is_an_error() -> Result<(), MetaError> {
    let result = meta::parse_meta_text("guid: not-a-guid\n", Path::new("bad.meta"));
    assert!(matches!(result, Err(MetaError::InvalidGuid { .. })));
    Ok(())
}

#[test]
fn meta_index_maps_guid_to_asset_path() -> Result<(), MetaError> {
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    assert_eq!(index.len(), 3);
    assert!(!index.is_empty());
    assert_eq!(
        index.get(CAMERA_GUID),
        Some(Path::new("tests/fixtures/project/Camera.prefab"))
    );
    assert_eq!(
        index.get(MATERIAL_GUID),
        Some(Path::new("tests/fixtures/project/Material.mat"))
    );
    assert_eq!(
        index.get(TEXTURE_GUID),
        Some(Path::new("tests/fixtures/project/Textures/Wood.png"))
    );
    assert!(index.contains(CAMERA_GUID));
    Ok(())
}

#[test]
fn meta_index_returns_none_for_unknown_guid() -> Result<(), MetaError> {
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    assert!(index.get("00000000000000000000000000000000").is_none());
    assert!(!index.contains("00000000000000000000000000000000"));
    Ok(())
}

#[test]
fn meta_index_rejects_duplicate_guid() -> Result<(), MetaError> {
    let mut index = GuidIndex::default();
    index.insert(String::from(CAMERA_GUID), PathBuf::from("first.prefab"))?;
    let result = index.insert(String::from(CAMERA_GUID), PathBuf::from("second.prefab"));
    assert!(matches!(result, Err(MetaError::DuplicateGuid { .. })));
    assert_eq!(index.len(), 1);
    Ok(())
}

#[test]
fn meta_index_rejects_invalid_guid() -> Result<(), MetaError> {
    let mut index = GuidIndex::default();
    let result = index.insert(String::from("not-a-guid"), PathBuf::from("first.prefab"));
    assert!(matches!(result, Err(MetaError::InvalidGuid { .. })));
    assert!(index.is_empty());
    Ok(())
}

#[test]
fn meta_index_alias_works() -> Result<(), MetaError> {
    let mut index = unitas_asset::MetaIndex::default();
    index.insert(String::from(CAMERA_GUID), PathBuf::from("Camera.prefab"))?;
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(CAMERA_GUID), Some(Path::new("Camera.prefab")));
    Ok(())
}
