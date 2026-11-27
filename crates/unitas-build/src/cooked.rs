use std::fs;
use std::path::Path;

pub use unitas_asset::audio_import::{
    cook_audio_candidates, cook_project_audio, default_audio_cache_dir, is_audio_source_path,
    load_or_cook_audio, load_or_cook_audio_with_cache, CookedAudio, CookedAudioEntry,
    CookedAudioManifest, AUDIO_IMPORTER_VERSION, COOKED_AUDIO_FORMAT_VERSION, COOKED_AUDIO_MAGIC,
};
use unitas_asset::dependencies::ImportSessionReport;
use unitas_asset::import_cache::ImportCacheError;
pub use unitas_asset::texture::{
    cook_project_textures, cook_texture_candidates, default_texture_cache_dir,
    is_texture_source_path, load_or_cook_texture, load_or_cook_texture_with_cache, CookedTexture,
    CookedTextureEntry, CookedTextureManifest, COOKED_DIR_NAME, COOKED_MANIFEST_FILENAME,
    COOKED_MANIFEST_FORMAT, COOKED_MANIFEST_FORMAT_VERSION, COOKED_TEXTURE_FORMAT_VERSION,
    COOKED_TEXTURE_MAGIC,
};

use crate::bundle::BundleError;

pub fn stage_project_assets_without_raw_textures(
    project_root: &Path,
    output_dir: &Path,
) -> Result<(), BundleError> {
    let source = project_root.join("Assets");
    if !source.is_dir() {
        return Ok(());
    }
    let destination = output_dir.join("Assets");
    if destination.starts_with(&source) {
        return Ok(());
    }
    copy_non_texture_directory(&source, &destination)
}

pub fn stage_project_cooked_textures(
    project_root: &Path,
    output_dir: &Path,
) -> Result<(CookedTextureManifest, ImportSessionReport), BundleError> {
    let cache_dir = default_texture_cache_dir(project_root);
    stage_project_cooked_textures_with_cache(project_root, &cache_dir, output_dir)
}

pub fn stage_project_cooked_textures_with_cache(
    project_root: &Path,
    cache_dir: &Path,
    output_dir: &Path,
) -> Result<(CookedTextureManifest, ImportSessionReport), BundleError> {
    let cooked_dir = output_dir.join(COOKED_DIR_NAME);
    if cooked_dir.exists() {
        fs::remove_dir_all(&cooked_dir).map_err(|error| BundleError::Io {
            path: cooked_dir.clone(),
            message: error.to_string(),
        })?;
    }
    cook_project_textures(project_root, cache_dir, &cooked_dir).map_err(map_cache_error)
}

pub fn stage_project_cooked_audio(
    project_root: &Path,
    output_dir: &Path,
) -> Result<(CookedAudioManifest, ImportSessionReport), BundleError> {
    let cache_dir = default_audio_cache_dir(project_root);
    stage_project_cooked_audio_with_cache(project_root, &cache_dir, output_dir)
}

pub fn stage_project_cooked_audio_with_cache(
    project_root: &Path,
    cache_dir: &Path,
    output_dir: &Path,
) -> Result<(CookedAudioManifest, ImportSessionReport), BundleError> {
    let cooked_dir = output_dir.join(COOKED_DIR_NAME);
    cook_project_audio(project_root, cache_dir, &cooked_dir).map_err(map_cache_error)
}

fn map_cache_error(error: ImportCacheError) -> BundleError {
    BundleError::Io {
        path: error.path,
        message: error.message,
    }
}

fn copy_non_texture_directory(source: &Path, destination: &Path) -> Result<(), BundleError> {
    let entries = fs::read_dir(source).map_err(|error| BundleError::Io {
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::create_dir_all(destination).map_err(|error| BundleError::Io {
        path: destination.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| BundleError::Io {
            path: source.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_non_texture_directory(&path, &target)?;
        } else if is_texture_source_path(&path) || is_audio_source_path(&path) {
            if target.is_file() {
                let _ = fs::remove_file(&target);
            }
        } else {
            fs::copy(&path, &target).map_err(|error| BundleError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
        }
    }
    Ok(())
}
