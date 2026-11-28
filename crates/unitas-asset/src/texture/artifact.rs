use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::dependencies::{
    ImportCandidate, ImportDiagnostic, ImportFingerprint, ImportOutcome, ImportSession,
    ImportSessionReport,
};
use crate::import_cache::{ImportCache, ImportCacheError};
use crate::json::{self, JsonValue};
use crate::meta::guid_from_meta_text;
use crate::texture::import::{
    import_texture, import_texture_for_target, texture_to_rgba8, TextureFormat, TextureImportError,
    TextureImportResult, TextureImportSettings, TexturePlatformFormat,
};

pub const COOKED_DIR_NAME: &str = "Cooked";
pub const COOKED_MANIFEST_FILENAME: &str = "manifest.json";
pub const COOKED_TEXTURE_MAGIC: [u8; 4] = *b"UTEX";
pub const COOKED_TEXTURE_FORMAT_VERSION: u32 = 1;
pub const COOKED_MANIFEST_FORMAT: &str = "unitas-cooked-texture-manifest";
pub const COOKED_MANIFEST_FORMAT_VERSION: u32 = 1;
pub const TEXTURE_IMPORTER_VERSION: &str = "unitas-texture-cook-v1";

const HEADER_BYTES: usize = 20;
const RGBA8_CHANNELS: u8 = 4;
const BUNDLE_MANIFEST_FILENAME: &str = "bundle.manifest";
static COOK_TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static TEXTURE_COOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_texture_cook() -> std::sync::MutexGuard<'static, ()> {
    TEXTURE_COOK_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let seq = COOK_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut temp = path.file_name().unwrap_or_default().to_os_string();
    temp.push(format!(".{}.{seq}.tmp", std::process::id()));
    let temp_path = path.with_file_name(temp);
    fs::write(&temp_path, bytes)?;
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn copy_file_atomic(src: &Path, dst: &Path) -> std::io::Result<()> {
    let seq = COOK_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut temp = dst.file_name().unwrap_or_default().to_os_string();
    temp.push(format!(".{}.{seq}.tmp", std::process::id()));
    let temp_path = dst.with_file_name(temp);
    fs::copy(src, &temp_path)?;
    if let Err(error) = fs::rename(&temp_path, dst) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureArtifact {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub pixels: Vec<u8>,
    pub srgb: bool,
    pub normal_map: bool,
    pub generate_mips: bool,
    pub settings: TextureImportSettings,
    pub encoded_format: TexturePlatformFormat,
    pub compressed_data: Option<Vec<u8>>,
    pub compressed_mip_data: Vec<Vec<u8>>,
}

impl TextureArtifact {
    pub fn from_import(result: &TextureImportResult) -> Self {
        TextureArtifact {
            width: result.texture.width,
            height: result.texture.height,
            format: result.texture.format,
            pixels: result.texture.pixels.clone(),
            srgb: result.settings.srgb,
            normal_map: result.settings.is_normal_map(),
            generate_mips: result.settings.mipmaps,
            settings: result.settings.clone(),
            encoded_format: result.encoded_format,
            compressed_data: result.compressed_data.clone(),
            compressed_mip_data: result.compressed_mip_data.clone(),
        }
    }
}

pub fn texture_artifact(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
) -> Result<TextureArtifact, TextureImportError> {
    let result = import_texture(bytes, file_name, settings)?;
    Ok(TextureArtifact::from_import(&result))
}

pub fn texture_artifact_for_target(
    bytes: &[u8],
    file_name: &str,
    settings: &TextureImportSettings,
    build_target: Option<&str>,
) -> Result<TextureArtifact, TextureImportError> {
    let result = import_texture_for_target(bytes, file_name, settings, build_target)?;
    Ok(TextureArtifact::from_import(&result))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedTexture {
    pub format_version: u32,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub srgb: bool,
    pub normal_map: bool,
    pub rgba: Vec<u8>,
}

impl CookedTexture {
    pub fn from_import(result: &TextureImportResult) -> Self {
        CookedTexture {
            format_version: COOKED_TEXTURE_FORMAT_VERSION,
            width: result.texture.width,
            height: result.texture.height,
            channels: RGBA8_CHANNELS,
            srgb: !result.settings.is_linear(),
            normal_map: result.settings.is_normal_map(),
            rgba: texture_to_rgba8(&result.texture),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let payload_len = u32::try_from(self.rgba.len()).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(HEADER_BYTES + self.rgba.len());
        out.extend_from_slice(&COOKED_TEXTURE_MAGIC);
        out.extend_from_slice(&self.format_version.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.push(self.channels);
        out.push(u8::from(self.srgb));
        out.push(u8::from(self.normal_map));
        out.push(0);
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&self.rgba);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, TextureImportError> {
        if bytes.len() < 24 {
            return Err(TextureImportError::Truncated);
        }
        if bytes.get(0..4) != Some(&COOKED_TEXTURE_MAGIC) {
            return Err(TextureImportError::InvalidSignature);
        }
        let version = read_u32_le(bytes, 4)?;
        if version != COOKED_TEXTURE_FORMAT_VERSION {
            return Err(TextureImportError::UnsupportedFormat(format!(
                "cooked texture version {version}"
            )));
        }
        let width = read_u32_le(bytes, 8)?;
        let height = read_u32_le(bytes, 12)?;
        let channels = bytes.get(16).copied().unwrap_or(0);
        if channels != RGBA8_CHANNELS {
            return Err(TextureImportError::UnsupportedColorType(channels));
        }
        let srgb = bytes.get(17).copied().unwrap_or(0) != 0;
        let normal_map = bytes.get(18).copied().unwrap_or(0) != 0;
        let payload_len = read_u32_le(bytes, 20)? as usize;
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(RGBA8_CHANNELS as usize))
            .ok_or_else(|| {
                TextureImportError::CorruptData("cooked dimensions overflow".to_string())
            })?;
        if payload_len != expected_len {
            return Err(TextureImportError::CorruptData(
                "cooked payload length does not match dimensions".to_string(),
            ));
        }
        let Some(payload) = bytes.get(24..24 + payload_len) else {
            return Err(TextureImportError::Truncated);
        };
        if bytes.len() != 24 + payload_len {
            return Err(TextureImportError::CorruptData(
                "cooked trailing bytes".to_string(),
            ));
        }
        Ok(CookedTexture {
            format_version: version,
            width,
            height,
            channels,
            srgb,
            normal_map,
            rgba: payload.to_vec(),
        })
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, TextureImportError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(TextureImportError::Truncated)?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(slice);
    Ok(u32::from_le_bytes(raw))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedTextureEntry {
    pub asset_path: String,
    pub guid: String,
    pub cooked_path: String,
    pub content_hash: String,
    pub format_version: u32,
    pub width: u32,
    pub height: u32,
    pub srgb: bool,
    pub normal_map: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookedTextureManifest {
    pub format_version: u32,
    pub entries: BTreeMap<String, CookedTextureEntry>,
}

impl CookedTextureManifest {
    pub fn new() -> Self {
        CookedTextureManifest {
            format_version: COOKED_MANIFEST_FORMAT_VERSION,
            entries: BTreeMap::new(),
        }
    }

    pub fn get_by_asset_path(&self, asset_path: &str) -> Option<&CookedTextureEntry> {
        let normalized = normalize_slashes(asset_path);
        self.entries.get(&normalized)
    }

    pub fn get_by_guid(&self, guid: &str) -> Option<&CookedTextureEntry> {
        self.entries.values().find(|entry| entry.guid == guid)
    }

    pub fn load_from_dir(cooked_dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(cooked_dir.join(COOKED_MANIFEST_FILENAME)).ok()?;
        Self::decode(&text)
    }

    pub fn save_to_dir(&self, cooked_dir: &Path) -> Result<PathBuf, ImportCacheError> {
        fs::create_dir_all(cooked_dir).map_err(|error| ImportCacheError {
            path: cooked_dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = cooked_dir.join(COOKED_MANIFEST_FILENAME);
        let preserved_audio: Vec<JsonValue> = fs::read_to_string(&path)
            .ok()
            .and_then(|text| json::parse(&text).ok())
            .and_then(|val| {
                val.get("audioEntries")
                    .and_then(JsonValue::as_array)
                    .map(<[JsonValue]>::to_vec)
            })
            .unwrap_or_default();
        let encoded = if preserved_audio.is_empty() {
            self.encode()
        } else {
            self.encode_with_audio_entries(preserved_audio)
        };
        write_file_atomic(&path, encoded.as_bytes()).map_err(|error| ImportCacheError {
            path: path.clone(),
            message: error.to_string(),
        })?;
        Ok(path)
    }

    pub fn encode(&self) -> String {
        self.encode_with_audio_entries(Vec::new())
    }

    fn encode_with_audio_entries(&self, audio_entries: Vec<JsonValue>) -> String {
        let entries: Vec<JsonValue> = self.entries.values().map(encode_manifest_entry).collect();
        let mut fields = vec![
            (
                "format".to_string(),
                JsonValue::String(COOKED_MANIFEST_FORMAT.to_string()),
            ),
            (
                "formatVersion".to_string(),
                JsonValue::Number(f64::from(self.format_version)),
            ),
            ("entries".to_string(), JsonValue::Array(entries)),
        ];
        if !audio_entries.is_empty() {
            fields.push(("audioEntries".to_string(), JsonValue::Array(audio_entries)));
        }
        json::write(&JsonValue::Object(fields))
    }

    pub fn decode(text: &str) -> Option<Self> {
        let value = json::parse(text).ok()?;
        if value.get("format").and_then(JsonValue::as_str) != Some(COOKED_MANIFEST_FORMAT) {
            return None;
        }
        let version = u32::try_from(value.get("formatVersion").and_then(JsonValue::as_i64)?)
            .ok()
            .filter(|v| *v == COOKED_MANIFEST_FORMAT_VERSION)?;
        let items = value.get("entries").and_then(JsonValue::as_array)?;
        let mut entries = BTreeMap::new();
        for item in items {
            let entry = decode_manifest_entry(item)?;
            entries.insert(entry.asset_path.clone(), entry);
        }
        Some(CookedTextureManifest {
            format_version: version,
            entries,
        })
    }
}

fn encode_manifest_entry(entry: &CookedTextureEntry) -> JsonValue {
    JsonValue::Object(vec![
        (
            "assetPath".to_string(),
            JsonValue::String(entry.asset_path.clone()),
        ),
        ("guid".to_string(), JsonValue::String(entry.guid.clone())),
        (
            "cookedPath".to_string(),
            JsonValue::String(entry.cooked_path.clone()),
        ),
        (
            "contentHash".to_string(),
            JsonValue::String(entry.content_hash.clone()),
        ),
        (
            "formatVersion".to_string(),
            JsonValue::Number(f64::from(entry.format_version)),
        ),
        (
            "width".to_string(),
            JsonValue::Number(f64::from(entry.width)),
        ),
        (
            "height".to_string(),
            JsonValue::Number(f64::from(entry.height)),
        ),
        ("srgb".to_string(), JsonValue::Bool(entry.srgb)),
        ("normalMap".to_string(), JsonValue::Bool(entry.normal_map)),
    ])
}

fn decode_manifest_entry(value: &JsonValue) -> Option<CookedTextureEntry> {
    Some(CookedTextureEntry {
        asset_path: value.get("assetPath")?.as_str()?.to_string(),
        guid: value.get("guid")?.as_str()?.to_string(),
        cooked_path: value.get("cookedPath")?.as_str()?.to_string(),
        content_hash: value.get("contentHash")?.as_str()?.to_string(),
        format_version: u32::try_from(value.get("formatVersion")?.as_i64()?).ok()?,
        width: u32::try_from(value.get("width")?.as_i64()?).ok()?,
        height: u32::try_from(value.get("height")?.as_i64()?).ok()?,
        srgb: value.get("srgb")?.as_bool()?,
        normal_map: value.get("normalMap")?.as_bool()?,
    })
}

pub fn is_texture_source_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "tga" | "targa" | "gif" | "psd" | "psb" | "exr" | "hdr"
    )
}

pub fn cook_project_textures(
    project_root: &Path,
    cache_dir: &Path,
    cooked_out_dir: &Path,
) -> Result<(CookedTextureManifest, ImportSessionReport), ImportCacheError> {
    let assets_dir = project_root.join("Assets");
    let mut source_paths = Vec::new();
    if assets_dir.is_dir() {
        collect_texture_files(&assets_dir, &mut source_paths)?;
        source_paths.sort();
    }
    let mut candidates = Vec::with_capacity(source_paths.len());
    for source_path in &source_paths {
        let rel = source_path
            .strip_prefix(project_root)
            .unwrap_or(source_path);
        let rel_key = normalize_slashes(&rel.to_string_lossy());
        candidates.push(build_texture_candidate(
            source_path,
            PathBuf::from(rel_key),
        )?);
    }
    cook_texture_candidates(&candidates, cache_dir, Some(cooked_out_dir))
}

pub fn cook_texture_candidates(
    candidates: &[ImportCandidate],
    cache_dir: &Path,
    cooked_out_dir: Option<&Path>,
) -> Result<(CookedTextureManifest, ImportSessionReport), ImportCacheError> {
    let _cook_guard = lock_texture_cook();
    let cache_cooked_dir = cache_dir.join(COOKED_DIR_NAME);
    fs::create_dir_all(&cache_cooked_dir).map_err(|error| ImportCacheError {
        path: cache_cooked_dir.clone(),
        message: error.to_string(),
    })?;
    let cached_manifest =
        CookedTextureManifest::load_from_dir(&cache_cooked_dir).unwrap_or_default();
    let mut cache = ImportCache::open(cache_dir);
    prune_missing_cached_artifacts(&mut cache, &cache_cooked_dir, &cached_manifest, candidates);
    let mut session = ImportSession::with_cache(cache);
    let mut freshly_cooked: BTreeMap<String, CookedTextureEntry> = BTreeMap::new();

    let report = session.run(candidates, &mut |candidate| {
        cook_single_candidate(candidate, &cache_cooked_dir, &mut freshly_cooked)
    });
    if let Some(error) = session.cache_error() {
        return Err(error.clone());
    }

    let mut manifest = CookedTextureManifest::new();
    for (key, entry) in &cached_manifest.entries {
        manifest.entries.insert(key.clone(), entry.clone());
    }
    for candidate in candidates {
        let key = normalize_slashes(&candidate.path.to_string_lossy());
        if let Some(entry) = freshly_cooked.remove(&key) {
            manifest.entries.insert(key, entry);
        }
    }
    manifest.save_to_dir(&cache_cooked_dir)?;
    if let Some(out_dir) = cooked_out_dir {
        let mut requested_manifest = CookedTextureManifest::new();
        for candidate in candidates {
            let key = normalize_slashes(&candidate.path.to_string_lossy());
            if let Some(entry) = manifest.entries.get(&key) {
                requested_manifest.entries.insert(key, entry.clone());
            }
        }
        stage_cooked_outputs(&cache_cooked_dir, out_dir, &requested_manifest)?;
        return Ok((requested_manifest, report));
    }
    Ok((manifest, report))
}

fn prune_missing_cached_artifacts(
    cache: &mut ImportCache,
    cache_cooked_dir: &Path,
    cached_manifest: &CookedTextureManifest,
    candidates: &[ImportCandidate],
) {
    let mut missing = Vec::new();
    for candidate in candidates {
        let key = normalize_slashes(&candidate.path.to_string_lossy());
        let artifact_exists = cached_manifest
            .get_by_asset_path(&key)
            .map(|entry| cache_cooked_dir.join(&entry.cooked_path).is_file())
            .unwrap_or(false);
        if !artifact_exists {
            missing.push(candidate.path.clone());
        }
    }
    if !missing.is_empty() {
        let retained: Vec<_> = cache
            .records()
            .filter(|record| !missing.contains(&record.path))
            .cloned()
            .collect();
        cache.replace(retained);
    }
}

fn cook_single_candidate(
    candidate: &ImportCandidate,
    cache_cooked_dir: &Path,
    freshly_cooked: &mut BTreeMap<String, CookedTextureEntry>,
) -> ImportOutcome {
    let asset_key = normalize_slashes(&candidate.path.to_string_lossy());
    let file_name = candidate
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("texture.png");
    let settings = TextureImportSettings::from_meta_text(&candidate.metadata).unwrap_or_default();
    let imported = match import_texture(&candidate.source, file_name, &settings) {
        Ok(imported) => imported,
        Err(error) => {
            return ImportOutcome::failed(ImportDiagnostic::new(
                asset_key,
                error.to_string(),
                "the texture stays failed",
            ));
        }
    };
    let cooked = CookedTexture::from_import(&imported);
    let encoded = cooked.encode();
    let content_hash = format!("{:016x}", fnv1a_hash(&encoded));
    let cooked_filename = format!("{}_{}.utex", sanitize_id(&candidate.guid), content_hash);
    let target_path = cache_cooked_dir.join(&cooked_filename);
    if let Err(error) = write_file_atomic(&target_path, &encoded) {
        return ImportOutcome::failed(ImportDiagnostic::new(
            asset_key,
            error.to_string(),
            "the cooked texture could not be written",
        ));
    }
    freshly_cooked.insert(
        asset_key.clone(),
        CookedTextureEntry {
            asset_path: asset_key,
            guid: candidate.guid.clone(),
            cooked_path: cooked_filename,
            content_hash,
            format_version: cooked.format_version,
            width: cooked.width,
            height: cooked.height,
            srgb: cooked.srgb,
            normal_map: cooked.normal_map,
        },
    );
    ImportOutcome::ready()
}

fn stage_cooked_outputs(
    cache_cooked_dir: &Path,
    cooked_out_dir: &Path,
    manifest: &CookedTextureManifest,
) -> Result<(), ImportCacheError> {
    if cache_cooked_dir == cooked_out_dir {
        return Ok(());
    }
    fs::create_dir_all(cooked_out_dir).map_err(|error| ImportCacheError {
        path: cooked_out_dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in manifest.entries.values() {
        let src = cache_cooked_dir.join(&entry.cooked_path);
        let dst = cooked_out_dir.join(&entry.cooked_path);
        if src != dst {
            copy_file_atomic(&src, &dst).map_err(|error| ImportCacheError {
                path: dst,
                message: error.to_string(),
            })?;
        }
    }
    manifest.save_to_dir(cooked_out_dir)?;
    Ok(())
}

pub fn build_texture_candidate(
    source_path: &Path,
    logical_path: PathBuf,
) -> Result<ImportCandidate, ImportCacheError> {
    let source = fs::read(source_path).map_err(|error| ImportCacheError {
        path: source_path.to_path_buf(),
        message: error.to_string(),
    })?;
    let meta_path = meta_path_for(source_path);
    let metadata = fs::read_to_string(&meta_path).unwrap_or_default();
    let settings_parsed = TextureImportSettings::from_meta_text(&metadata).unwrap_or_default();
    let settings = settings_parsed.to_meta_text();
    let guid = guid_from_meta_text(&metadata).unwrap_or_else(|| {
        format!(
            "{:016x}",
            fnv1a_hash(normalize_slashes(&logical_path.to_string_lossy()).as_bytes())
        )
    });
    Ok(ImportCandidate {
        path: logical_path,
        guid,
        source,
        metadata,
        settings,
        importer_version: TEXTURE_IMPORTER_VERSION.to_string(),
        dependencies: Vec::new(),
    })
}

pub fn load_or_cook_texture(path: &Path) -> Option<CookedTexture> {
    load_or_cook_texture_with_cache(path, None)
}

pub fn load_or_cook_texture_with_cache(
    path: &Path,
    explicit_cache_dir: Option<&Path>,
) -> Option<CookedTexture> {
    if let Some((root, cooked_dir, manifest)) = find_ancestor_cooked_manifest(path) {
        if let Ok(rel) = path.strip_prefix(&root) {
            let rel_key = normalize_slashes(&rel.to_string_lossy());
            if let Some(entry) = manifest.get_by_asset_path(&rel_key) {
                let bytes = fs::read(cooked_dir.join(&entry.cooked_path)).ok()?;
                return CookedTexture::decode(&bytes).ok();
            }
        }
        if let Some(guid) = read_sidecar_guid(path) {
            if let Some(entry) = manifest.get_by_guid(&guid) {
                let bytes = fs::read(cooked_dir.join(&entry.cooked_path)).ok()?;
                return CookedTexture::decode(&bytes).ok();
            }
        }
        if root.join(BUNDLE_MANIFEST_FILENAME).is_file() {
            return None;
        }
    }
    if is_inside_staged_bundle(path) && !path.is_file() {
        return None;
    }
    let logical_path = logical_asset_path(path);
    let candidate = build_texture_candidate(path, logical_path).ok()?;
    let cache_dir = explicit_cache_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_texture_cache_dir_for_path(path, &candidate.fingerprint()));
    let (manifest, _) =
        cook_texture_candidates(std::slice::from_ref(&candidate), &cache_dir, None).ok()?;
    let key = normalize_slashes(&candidate.path.to_string_lossy());
    let entry = manifest.get_by_asset_path(&key)?;
    let cooked_bytes = fs::read(cache_dir.join(COOKED_DIR_NAME).join(&entry.cooked_path)).ok()?;
    CookedTexture::decode(&cooked_bytes).ok()
}

pub fn default_texture_cache_dir(project_root: &Path) -> PathBuf {
    if !project_root.as_os_str().is_empty() && project_root.is_dir() {
        return project_root
            .join("Library")
            .join("unitas")
            .join("cache")
            .join("texture");
    }
    let hash = fnv1a_hash(project_root.to_string_lossy().as_bytes());
    std::env::temp_dir()
        .join("unitas_texture_import_cache")
        .join(format!("{hash:016x}"))
}

fn default_texture_cache_dir_for_path(path: &Path, fingerprint: &ImportFingerprint) -> PathBuf {
    if let Some(root) = find_project_root(path) {
        return default_texture_cache_dir(&root);
    }
    let parent_key = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{:016x}", fingerprint.source));
    let hash = fnv1a_hash(parent_key.as_bytes());
    std::env::temp_dir()
        .join("unitas_texture_import_cache")
        .join(format!("{hash:016x}"))
}

fn find_ancestor_cooked_manifest(path: &Path) -> Option<(PathBuf, PathBuf, CookedTextureManifest)> {
    let mut current = path.parent()?;
    loop {
        let cooked_dir = current.join(COOKED_DIR_NAME);
        if let Some(manifest) = CookedTextureManifest::load_from_dir(&cooked_dir) {
            return Some((current.to_path_buf(), cooked_dir, manifest));
        }
        current = current.parent()?;
    }
}

fn is_inside_staged_bundle(path: &Path) -> bool {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(BUNDLE_MANIFEST_FILENAME).is_file() {
            return true;
        }
        current = dir.parent();
    }
    false
}

fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn logical_asset_path(path: &Path) -> PathBuf {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.file_name().and_then(|n| n.to_str()) == Some("Assets") {
            if let Some(root) = dir.parent() {
                if let Ok(rel) = path.strip_prefix(root) {
                    return PathBuf::from(normalize_slashes(&rel.to_string_lossy()));
                }
            }
        }
        current = dir.parent();
    }
    path.file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
}

fn read_sidecar_guid(path: &Path) -> Option<String> {
    let text = fs::read_to_string(meta_path_for(path)).ok()?;
    guid_from_meta_text(&text)
}

fn meta_path_for(path: &Path) -> PathBuf {
    let mut meta = path.as_os_str().to_os_string();
    meta.push(".meta");
    PathBuf::from(meta)
}

fn collect_texture_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ImportCacheError> {
    let entries = fs::read_dir(dir).map_err(|error| ImportCacheError {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ImportCacheError {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_texture_files(&path, out)?;
        } else if is_texture_source_path(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

fn sanitize_id(raw: &str) -> String {
    let filtered: String = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();
    if filtered.is_empty() {
        "texture".to_string()
    } else {
        filtered
    }
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
