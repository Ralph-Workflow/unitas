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
use crate::model::{import_model, import_model_sub_assets, ModelImportError};
use crate::primitive::{MeshData, SubMesh};

pub const COOKED_MESH_DIR_NAME: &str = "Cooked";
pub const COOKED_MESH_MANIFEST_FILENAME: &str = "mesh-manifest.json";
pub const COOKED_MESH_MAGIC: [u8; 4] = *b"UMSH";
pub const COOKED_MESH_FORMAT_VERSION: u16 = 1;
pub const COOKED_MESH_IMPORTER: &str = "mesh-v1";
pub const COOKED_MESH_EXTENSION: &str = "umsh";
pub const COOKED_MESH_MANIFEST_FORMAT: &str = "unitas-cooked-mesh-manifest";
pub const COOKED_MESH_MANIFEST_FORMAT_VERSION: u32 = 1;
pub const MESH_IMPORTER_VERSION: &str = COOKED_MESH_IMPORTER;
const BUNDLE_MANIFEST_FILENAME: &str = "bundle.manifest";
static COOK_MESH_TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static MESH_COOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_mesh_cook() -> std::sync::MutexGuard<'static, ()> {
    MESH_COOK_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CookedMesh {
    pub format_version: u16,
    pub mesh: MeshData,
    pub sub_assets: Vec<MeshData>,
}

impl CookedMesh {
    pub fn from_import(mesh: MeshData) -> Self {
        Self::from_import_with_sub_assets(mesh.clone(), vec![mesh])
    }
    pub fn from_import_with_sub_assets(mesh: MeshData, sub_assets: Vec<MeshData>) -> Self {
        CookedMesh {
            format_version: COOKED_MESH_FORMAT_VERSION,
            mesh,
            sub_assets,
        }
    }
    pub fn from_mesh(mesh: MeshData) -> Self {
        Self::from_import(mesh)
    }
    pub fn into_mesh(self) -> MeshData {
        self.mesh
    }
    pub fn as_mesh(&self) -> &MeshData {
        &self.mesh
    }
    pub fn sub_assets(&self) -> &[MeshData] {
        &self.sub_assets
    }
    pub fn name(&self) -> &str {
        &self.mesh.name
    }
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.mesh.positions
    }
    pub fn normals(&self) -> &[[f32; 3]] {
        &self.mesh.normals
    }
    pub fn uvs(&self) -> &[[f32; 2]] {
        &self.mesh.uvs
    }
    pub fn tangents(&self) -> &[[f32; 4]] {
        &self.mesh.tangents
    }
    pub fn indices(&self) -> &[u32] {
        &self.mesh.indices
    }
    pub fn sub_meshes(&self) -> &[SubMesh] {
        &self.mesh.sub_meshes
    }
    pub fn vertex_count(&self) -> u32 {
        u32::try_from(self.mesh.positions.len()).unwrap_or(u32::MAX)
    }
    pub fn index_count(&self) -> u32 {
        u32::try_from(self.mesh.indices.len()).unwrap_or(u32::MAX)
    }
    pub fn sub_mesh_count(&self) -> u32 {
        u32::try_from(self.mesh.sub_meshes.len()).unwrap_or(u32::MAX)
    }
    pub fn encode(&self) -> Vec<u8> {
        encode_cooked_mesh(self)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ModelImportError> {
        decode_cooked_mesh(bytes)
    }
    pub fn manifest_entry(
        &self,
        asset_path: impl Into<String>,
        guid: impl Into<String>,
        cooked_path: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> CookedMeshEntry {
        manifest_entry(asset_path, guid, cooked_path, content_hash, self)
    }
}

pub fn encode_cooked_mesh(cooked: &CookedMesh) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&COOKED_MESH_MAGIC);
    out.extend_from_slice(&cooked.format_version.to_le_bytes());
    write_mesh(&mut out, &cooked.mesh);
    write_u32(&mut out, cooked.sub_assets.len());
    for sub in &cooked.sub_assets {
        write_mesh(&mut out, sub);
    }
    out
}

pub fn decode_cooked_mesh(bytes: &[u8]) -> Result<CookedMesh, ModelImportError> {
    if bytes.len() < 6 {
        return Err(ModelImportError::TruncatedCookedMesh);
    }
    if bytes.get(0..4) != Some(&COOKED_MESH_MAGIC) {
        return Err(ModelImportError::InvalidCookedSignature);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != COOKED_MESH_FORMAT_VERSION {
        return Err(ModelImportError::UnsupportedCookedVersion(version));
    }
    let mut pos = 6;
    let mesh = read_mesh(bytes, &mut pos)?;
    let count = read_u32(bytes, &mut pos)? as usize;
    let mut sub_assets = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        sub_assets.push(read_mesh(bytes, &mut pos)?);
    }
    if pos != bytes.len() {
        return Err(ModelImportError::CorruptCookedMesh(
            "trailing bytes".to_string(),
        ));
    }
    Ok(CookedMesh {
        format_version: version,
        mesh,
        sub_assets,
    })
}

fn write_u32(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&u32::try_from(value).unwrap_or(u32::MAX).to_le_bytes());
}

fn write_str(out: &mut Vec<u8>, text: &str) {
    write_u32(out, text.len());
    out.extend_from_slice(text.as_bytes());
}

fn write_f32s<const N: usize>(out: &mut Vec<u8>, items: &[[f32; N]]) {
    write_u32(out, items.len());
    for item in items {
        for v in item {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

fn write_mesh(out: &mut Vec<u8>, mesh: &MeshData) {
    write_str(out, &mesh.name);
    write_f32s(out, &mesh.positions);
    write_f32s(out, &mesh.normals);
    write_f32s(out, &mesh.uvs);
    write_f32s(out, &mesh.tangents);
    write_u32(out, mesh.indices.len());
    for idx in &mesh.indices {
        out.extend_from_slice(&idx.to_le_bytes());
    }
    write_u32(out, mesh.sub_meshes.len());
    for sub in &mesh.sub_meshes {
        write_str(out, &sub.name);
        out.extend_from_slice(&sub.index_offset.to_le_bytes());
        out.extend_from_slice(&sub.index_count.to_le_bytes());
        write_str(out, &sub.sub_asset_id);
    }
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, ModelImportError> {
    let end = pos
        .checked_add(4)
        .ok_or(ModelImportError::TruncatedCookedMesh)?;
    let s = bytes
        .get(*pos..end)
        .ok_or(ModelImportError::TruncatedCookedMesh)?;
    *pos = end;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn read_str(bytes: &[u8], pos: &mut usize) -> Result<String, ModelImportError> {
    let len = read_u32(bytes, pos)? as usize;
    let end = pos
        .checked_add(len)
        .ok_or(ModelImportError::TruncatedCookedMesh)?;
    let s = bytes
        .get(*pos..end)
        .ok_or(ModelImportError::TruncatedCookedMesh)?;
    *pos = end;
    std::str::from_utf8(s)
        .map(str::to_string)
        .map_err(|e| ModelImportError::CorruptCookedMesh(e.to_string()))
}

fn read_f32s<const N: usize>(
    bytes: &[u8],
    pos: &mut usize,
) -> Result<Vec<[f32; N]>, ModelImportError> {
    let count = read_u32(bytes, pos)? as usize;
    let need = count
        .checked_mul(N * 4)
        .ok_or(ModelImportError::TruncatedCookedMesh)?;
    if bytes.len().saturating_sub(*pos) < need {
        return Err(ModelImportError::TruncatedCookedMesh);
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let mut item = [0.0f32; N];
        for slot in &mut item {
            *slot = f32::from_bits(read_u32(bytes, pos)?);
        }
        out.push(item);
    }
    Ok(out)
}

fn read_mesh(bytes: &[u8], pos: &mut usize) -> Result<MeshData, ModelImportError> {
    let name = read_str(bytes, pos)?;
    let (positions, normals) = (read_f32s::<3>(bytes, pos)?, read_f32s::<3>(bytes, pos)?);
    let (uvs, tangents) = (read_f32s::<2>(bytes, pos)?, read_f32s::<4>(bytes, pos)?);
    let idx_count = read_u32(bytes, pos)? as usize;
    if bytes.len().saturating_sub(*pos) < idx_count.saturating_mul(4) {
        return Err(ModelImportError::TruncatedCookedMesh);
    }
    let mut indices = Vec::with_capacity(idx_count);
    for _ in 0..idx_count {
        indices.push(read_u32(bytes, pos)?);
    }
    let sub_count = read_u32(bytes, pos)? as usize;
    let mut sub_meshes = Vec::with_capacity(sub_count.min(1024));
    for _ in 0..sub_count {
        sub_meshes.push(SubMesh {
            name: read_str(bytes, pos)?,
            index_offset: read_u32(bytes, pos)?,
            index_count: read_u32(bytes, pos)?,
            sub_asset_id: read_str(bytes, pos)?,
        });
    }
    Ok(MeshData {
        name,
        positions,
        normals,
        uvs,
        tangents,
        indices,
        sub_meshes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedMeshEntry {
    pub asset_path: String,
    pub guid: String,
    pub cooked_path: String,
    pub content_hash: String,
    pub format_version: u16,
    pub vertex_count: u32,
    pub index_count: u32,
    pub sub_mesh_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookedMeshManifest {
    pub format_version: u32,
    pub entries: BTreeMap<String, CookedMeshEntry>,
}

impl CookedMeshManifest {
    pub fn new() -> Self {
        CookedMeshManifest {
            format_version: COOKED_MESH_MANIFEST_FORMAT_VERSION,
            entries: BTreeMap::new(),
        }
    }
    pub fn get_by_asset_path(&self, asset_path: &str) -> Option<&CookedMeshEntry> {
        self.entries.get(&asset_path.replace('\\', "/"))
    }
    pub fn get_by_guid(&self, guid: &str) -> Option<&CookedMeshEntry> {
        self.entries.values().find(|entry| entry.guid == guid)
    }
    pub fn load_from_dir(cooked_dir: &Path) -> Option<Self> {
        Self::decode(&fs::read_to_string(cooked_dir.join(COOKED_MESH_MANIFEST_FILENAME)).ok()?)
    }
    pub fn save_to_dir(&self, cooked_dir: &Path) -> Result<PathBuf, ImportCacheError> {
        fs::create_dir_all(cooked_dir).map_err(|e| ImportCacheError {
            path: cooked_dir.to_path_buf(),
            message: e.to_string(),
        })?;
        let path = cooked_dir.join(COOKED_MESH_MANIFEST_FILENAME);
        write_file_atomic(&path, self.encode().as_bytes()).map_err(|e| ImportCacheError {
            path: path.clone(),
            message: e.to_string(),
        })?;
        Ok(path)
    }
    pub fn encode(&self) -> String {
        let entries: Vec<JsonValue> = self.entries.values().map(encode_entry).collect();
        json::write(&JsonValue::Object(vec![
            (
                "format".to_string(),
                JsonValue::String(COOKED_MESH_MANIFEST_FORMAT.to_string()),
            ),
            (
                "formatVersion".to_string(),
                JsonValue::Number(f64::from(self.format_version)),
            ),
            ("entries".to_string(), JsonValue::Array(entries)),
        ]))
    }
    pub fn decode(text: &str) -> Option<Self> {
        let val = json::parse(text).ok()?;
        if val.get("format").and_then(JsonValue::as_str) != Some(COOKED_MESH_MANIFEST_FORMAT) {
            return None;
        }
        let version = u32::try_from(val.get("formatVersion").and_then(JsonValue::as_i64)?)
            .ok()
            .filter(|v| *v == COOKED_MESH_MANIFEST_FORMAT_VERSION)?;
        let mut entries = BTreeMap::new();
        for item in val.get("entries").and_then(JsonValue::as_array)? {
            let entry = decode_entry(item)?;
            entries.insert(entry.asset_path.clone(), entry);
        }
        Some(CookedMeshManifest {
            format_version: version,
            entries,
        })
    }
}

fn encode_entry(e: &CookedMeshEntry) -> JsonValue {
    JsonValue::Object(vec![
        (
            "assetPath".to_string(),
            JsonValue::String(e.asset_path.clone()),
        ),
        ("guid".to_string(), JsonValue::String(e.guid.clone())),
        (
            "cookedPath".to_string(),
            JsonValue::String(e.cooked_path.clone()),
        ),
        (
            "contentHash".to_string(),
            JsonValue::String(e.content_hash.clone()),
        ),
        (
            "formatVersion".to_string(),
            JsonValue::Number(f64::from(e.format_version)),
        ),
        (
            "vertexCount".to_string(),
            JsonValue::Number(f64::from(e.vertex_count)),
        ),
        (
            "indexCount".to_string(),
            JsonValue::Number(f64::from(e.index_count)),
        ),
        (
            "subMeshCount".to_string(),
            JsonValue::Number(f64::from(e.sub_mesh_count)),
        ),
    ])
}

fn decode_entry(v: &JsonValue) -> Option<CookedMeshEntry> {
    Some(CookedMeshEntry {
        asset_path: v.get("assetPath")?.as_str()?.to_string(),
        guid: v.get("guid")?.as_str()?.to_string(),
        cooked_path: v.get("cookedPath")?.as_str()?.to_string(),
        content_hash: v.get("contentHash")?.as_str()?.to_string(),
        format_version: u16::try_from(v.get("formatVersion")?.as_i64()?).ok()?,
        vertex_count: u32::try_from(v.get("vertexCount")?.as_i64()?).ok()?,
        index_count: u32::try_from(v.get("indexCount")?.as_i64()?).ok()?,
        sub_mesh_count: u32::try_from(v.get("subMeshCount")?.as_i64()?).ok()?,
    })
}

pub fn manifest_entry(
    asset_path: impl Into<String>,
    guid: impl Into<String>,
    cooked_path: impl Into<String>,
    content_hash: impl Into<String>,
    cooked: &CookedMesh,
) -> CookedMeshEntry {
    CookedMeshEntry {
        asset_path: asset_path.into().replace('\\', "/"),
        guid: guid.into(),
        cooked_path: cooked_path.into(),
        content_hash: content_hash.into(),
        format_version: cooked.format_version,
        vertex_count: cooked.vertex_count(),
        index_count: cooked.index_count(),
        sub_mesh_count: cooked.sub_mesh_count(),
    }
}

pub fn cooked_mesh_relative_path(guid: &str, content_hash: &str) -> String {
    format!(
        "{}_{}.{COOKED_MESH_EXTENSION}",
        sanitize_id(guid),
        content_hash
    )
}

pub fn is_mesh_source_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "obj" | "fbx" | "gltf" | "glb"
        )
    })
}

pub fn is_model_source_path(path: &Path) -> bool {
    is_mesh_source_path(path)
}

pub fn build_mesh_candidate(
    source_path: &Path,
    logical_path: PathBuf,
) -> Result<ImportCandidate, ImportCacheError> {
    let source = fs::read(source_path).map_err(|e| ImportCacheError {
        path: source_path.to_path_buf(),
        message: e.to_string(),
    })?;
    let metadata = fs::read_to_string(meta_path_for(source_path)).unwrap_or_default();
    let guid = guid_from_meta_text(&metadata).unwrap_or_else(|| {
        format!(
            "{:016x}",
            fnv1a_hash(logical_path.to_string_lossy().replace('\\', "/").as_bytes())
        )
    });
    Ok(ImportCandidate {
        path: logical_path,
        guid,
        source,
        metadata: metadata.clone(),
        settings: metadata,
        importer_version: COOKED_MESH_IMPORTER.to_string(),
        dependencies: Vec::new(),
    })
}

pub fn cook_project_meshes(
    project_root: &Path,
    cache_dir: &Path,
    cooked_out_dir: &Path,
) -> Result<(CookedMeshManifest, ImportSessionReport), ImportCacheError> {
    let assets_dir = project_root.join("Assets");
    let mut source_paths = Vec::new();
    if assets_dir.is_dir() {
        collect_mesh_files(&assets_dir, &mut source_paths)?;
        source_paths.sort();
    }
    let mut candidates = Vec::with_capacity(source_paths.len());
    for src in &source_paths {
        let rel = src
            .strip_prefix(project_root)
            .unwrap_or(src)
            .to_string_lossy()
            .replace('\\', "/");
        candidates.push(build_mesh_candidate(src, PathBuf::from(rel))?);
    }
    cook_mesh_candidates(&candidates, cache_dir, Some(cooked_out_dir))
}

pub fn cook_mesh_candidates(
    candidates: &[ImportCandidate],
    cache_dir: &Path,
    cooked_out_dir: Option<&Path>,
) -> Result<(CookedMeshManifest, ImportSessionReport), ImportCacheError> {
    let _cook_guard = lock_mesh_cook();
    let cache_cooked_dir = cache_dir.join(COOKED_MESH_DIR_NAME);
    fs::create_dir_all(&cache_cooked_dir).map_err(|e| ImportCacheError {
        path: cache_cooked_dir.clone(),
        message: e.to_string(),
    })?;
    let cached_manifest = CookedMeshManifest::load_from_dir(&cache_cooked_dir).unwrap_or_default();
    let mut cache = ImportCache::open(cache_dir);
    prune_missing(&mut cache, &cache_cooked_dir, &cached_manifest, candidates);
    let mut session = ImportSession::with_cache(cache);
    let mut fresh: BTreeMap<String, CookedMeshEntry> = BTreeMap::new();
    let report = session.run(candidates, &mut |c| {
        cook_single(c, &cache_cooked_dir, &mut fresh)
    });
    if let Some(error) = session.cache_error() {
        return Err(error.clone());
    }
    let mut manifest = CookedMeshManifest::new();
    for (k, v) in &cached_manifest.entries {
        manifest.entries.insert(k.clone(), v.clone());
    }
    for c in candidates {
        let key = c.path.to_string_lossy().replace('\\', "/");
        if let Some(entry) = fresh.remove(&key) {
            manifest.entries.insert(key, entry);
        }
    }
    manifest.save_to_dir(&cache_cooked_dir)?;
    if let Some(out_dir) = cooked_out_dir {
        let mut req = CookedMeshManifest::new();
        for c in candidates {
            let key = c.path.to_string_lossy().replace('\\', "/");
            if let Some(e) = manifest.entries.get(&key) {
                req.entries.insert(key, e.clone());
            }
        }
        stage_outputs(&cache_cooked_dir, out_dir, &req)?;
        return Ok((req, report));
    }
    Ok((manifest, report))
}

fn prune_missing(
    cache: &mut ImportCache,
    dir: &Path,
    manifest: &CookedMeshManifest,
    candidates: &[ImportCandidate],
) {
    let missing: Vec<_> = candidates
        .iter()
        .filter(|c| {
            !manifest
                .get_by_asset_path(&c.path.to_string_lossy())
                .is_some_and(|e| dir.join(&e.cooked_path).is_file())
        })
        .map(|c| c.path.clone())
        .collect();
    if !missing.is_empty() {
        let retained: Vec<_> = cache
            .records()
            .filter(|r| !missing.contains(&r.path))
            .cloned()
            .collect();
        cache.replace(retained);
    }
}

fn cook_single(
    c: &ImportCandidate,
    dir: &Path,
    fresh: &mut BTreeMap<String, CookedMeshEntry>,
) -> ImportOutcome {
    let key = c.path.to_string_lossy().replace('\\', "/");
    let name = c
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mesh.obj");
    let (Ok(combined), Ok(subs)) = (
        import_model(&c.source, name),
        import_model_sub_assets(&c.source, name),
    ) else {
        let err = import_model(&c.source, name)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        return ImportOutcome::failed(ImportDiagnostic::new(key, err, "the mesh stays failed"));
    };
    let cooked = CookedMesh::from_import_with_sub_assets(combined, subs);
    let encoded = cooked.encode();
    let hash = format!("{:016x}", fnv1a_hash(&encoded));
    let fname = cooked_mesh_relative_path(&c.guid, &hash);
    if let Err(e) = write_file_atomic(&dir.join(&fname), &encoded) {
        return ImportOutcome::failed(ImportDiagnostic::new(
            key,
            e.to_string(),
            "the cooked mesh could not be written",
        ));
    }
    fresh.insert(
        key.clone(),
        manifest_entry(key, c.guid.clone(), fname, hash, &cooked),
    );
    ImportOutcome::ready()
}

fn copy_file_atomic(src: &Path, dst: &Path) -> std::io::Result<()> {
    let seq = COOK_MESH_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

fn stage_outputs(
    src_dir: &Path,
    dst_dir: &Path,
    manifest: &CookedMeshManifest,
) -> Result<(), ImportCacheError> {
    if src_dir == dst_dir {
        return Ok(());
    }
    fs::create_dir_all(dst_dir).map_err(|e| ImportCacheError {
        path: dst_dir.to_path_buf(),
        message: e.to_string(),
    })?;
    for entry in manifest.entries.values() {
        let (src, dst) = (
            src_dir.join(&entry.cooked_path),
            dst_dir.join(&entry.cooked_path),
        );
        if src != dst {
            copy_file_atomic(&src, &dst).map_err(|e| ImportCacheError {
                path: dst,
                message: e.to_string(),
            })?;
        }
    }
    manifest.save_to_dir(dst_dir)?;
    Ok(())
}

pub fn load_or_cook_mesh(path: &Path) -> Option<CookedMesh> {
    load_or_cook_mesh_with_cache(path, None)
}

pub fn load_or_cook_mesh_with_cache(
    path: &Path,
    explicit_cache_dir: Option<&Path>,
) -> Option<CookedMesh> {
    if let Some((root, cooked_dir, manifest)) = find_ancestor_manifest(path) {
        if let Ok(rel) = path.strip_prefix(&root) {
            if let Some(entry) = manifest.get_by_asset_path(&rel.to_string_lossy()) {
                return CookedMesh::decode(&fs::read(cooked_dir.join(&entry.cooked_path)).ok()?)
                    .ok();
            }
        }
        if let Some(guid) = fs::read_to_string(meta_path_for(path))
            .ok()
            .and_then(|t| guid_from_meta_text(&t))
        {
            if let Some(entry) = manifest.get_by_guid(&guid) {
                return CookedMesh::decode(&fs::read(cooked_dir.join(&entry.cooked_path)).ok()?)
                    .ok();
            }
        }
        if root.join(BUNDLE_MANIFEST_FILENAME).is_file() {
            return None;
        }
    }
    if has_ancestor_file(path, BUNDLE_MANIFEST_FILENAME) && !path.is_file() {
        return None;
    }
    let candidate = build_mesh_candidate(path, logical_asset_path(path)).ok()?;
    let cache_dir = explicit_cache_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_mesh_cache_dir_for_path(path, &candidate.fingerprint()));
    let (manifest, _) =
        cook_mesh_candidates(std::slice::from_ref(&candidate), &cache_dir, None).ok()?;
    let entry = manifest.get_by_asset_path(&candidate.path.to_string_lossy())?;
    CookedMesh::decode(
        &fs::read(
            cache_dir
                .join(COOKED_MESH_DIR_NAME)
                .join(&entry.cooked_path),
        )
        .ok()?,
    )
    .ok()
}

pub fn default_mesh_cache_dir(project_root: &Path) -> PathBuf {
    if !project_root.as_os_str().is_empty() && project_root.is_dir() {
        return project_root
            .join("Library")
            .join("unitas")
            .join("cache")
            .join("mesh");
    }
    let hash = fnv1a_hash(project_root.to_string_lossy().as_bytes());
    std::env::temp_dir()
        .join("unitas_mesh_import_cache")
        .join(format!("{hash:016x}"))
}

pub fn default_mesh_cache_dir_for_path(path: &Path, fingerprint: &ImportFingerprint) -> PathBuf {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir() {
            return default_mesh_cache_dir(dir);
        }
        cur = dir.parent();
    }
    let key = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{:016x}", fingerprint.source));
    std::env::temp_dir()
        .join("unitas_mesh_import_cache")
        .join(format!("{:016x}", fnv1a_hash(key.as_bytes())))
}

fn find_ancestor_manifest(path: &Path) -> Option<(PathBuf, PathBuf, CookedMeshManifest)> {
    let mut cur = path.parent()?;
    loop {
        let dir = cur.join(COOKED_MESH_DIR_NAME);
        if let Some(m) = CookedMeshManifest::load_from_dir(&dir) {
            return Some((cur.to_path_buf(), dir, m));
        }
        cur = cur.parent()?;
    }
}

fn has_ancestor_file(path: &Path, file_name: &str) -> bool {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir.join(file_name).is_file() {
            return true;
        }
        cur = dir.parent();
    }
    false
}

fn logical_asset_path(path: &Path) -> PathBuf {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir.file_name().and_then(|n| n.to_str()) == Some("Assets") {
            if let Some(root) = dir.parent() {
                if let Ok(rel) = path.strip_prefix(root) {
                    return PathBuf::from(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        cur = dir.parent();
    }
    path.file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
}

fn meta_path_for(path: &Path) -> PathBuf {
    let mut meta = path.as_os_str().to_os_string();
    meta.push(".meta");
    PathBuf::from(meta)
}

fn collect_mesh_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ImportCacheError> {
    for entry in fs::read_dir(dir).map_err(|e| ImportCacheError {
        path: dir.to_path_buf(),
        message: e.to_string(),
    })? {
        let p = entry
            .map_err(|e| ImportCacheError {
                path: dir.to_path_buf(),
                message: e.to_string(),
            })?
            .path();
        if p.is_dir() {
            collect_mesh_files(&p, out)?;
        } else if is_mesh_source_path(&p) {
            out.push(p);
        }
    }
    Ok(())
}

fn sanitize_id(raw: &str) -> String {
    let s: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if s.is_empty() {
        "mesh".to_string()
    } else {
        s
    }
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let seq = COOK_MESH_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{import_model, import_model_sub_assets};

    const SAMPLE_OBJ: &str = "o Hero\nusemtl Steel\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 1.0 1.0\nvt 0.0 1.0\nvn 0.0 0.0 1.0\ng Front\nf 1/1/1 2/2/1 3/3/1\nf 1/1/1 3/3/1 4/4/1\ng Back\nusemtl Brass\nf 2/2/1 1/1/1 4/4/1\nf 2/2/1 4/4/1 3/3/1\n";

    fn temp_test_dir(tag: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!(
            "unitas_cooked_mesh_test_{tag}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn cooked_mesh_round_trip_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
        let combined = import_model(SAMPLE_OBJ.as_bytes(), "Hero.obj")?;
        let sub_assets = import_model_sub_assets(SAMPLE_OBJ.as_bytes(), "Hero.obj")?;
        let cooked = CookedMesh::from_import_with_sub_assets(combined.clone(), sub_assets.clone());
        assert_eq!(cooked.format_version, COOKED_MESH_FORMAT_VERSION);
        assert_eq!(cooked.name(), "Hero");
        assert_eq!(cooked.positions(), combined.positions.as_slice());
        assert_eq!(cooked.normals(), combined.normals.as_slice());
        assert_eq!(cooked.uvs(), combined.uvs.as_slice());
        assert_eq!(cooked.tangents(), combined.tangents.as_slice());
        assert_eq!(cooked.indices(), combined.indices.as_slice());
        assert_eq!(cooked.sub_meshes(), combined.sub_meshes.as_slice());
        assert_eq!(cooked.sub_assets(), sub_assets.as_slice());
        let encoded = encode_cooked_mesh(&cooked);
        assert_eq!(encoded.get(0..4), Some(COOKED_MESH_MAGIC.as_slice()));
        let decoded = decode_cooked_mesh(&encoded)?;
        assert_eq!(decoded, cooked);
        assert_eq!(decoded.as_mesh(), &combined);
        assert_eq!(decoded.sub_assets(), sub_assets.as_slice());
        Ok(())
    }

    #[test]
    fn cooked_mesh_rejects_corrupt_magic_version_and_truncated_payload(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let combined = import_model(SAMPLE_OBJ.as_bytes(), "Hero.obj")?;
        let encoded = CookedMesh::from_import(combined).encode();
        let mut bad_magic = encoded.clone();
        bad_magic[0] = b'X';
        assert_eq!(
            CookedMesh::decode(&bad_magic),
            Err(ModelImportError::InvalidCookedSignature)
        );
        assert!(ModelImportError::InvalidCookedSignature
            .to_string()
            .contains("UMSH"));
        let mut bad_version = encoded.clone();
        bad_version[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            CookedMesh::decode(&bad_version),
            Err(ModelImportError::UnsupportedCookedVersion(99))
        );
        assert!(ModelImportError::UnsupportedCookedVersion(99)
            .to_string()
            .contains("99"));
        assert_eq!(
            CookedMesh::decode(&encoded[..6]),
            Err(ModelImportError::TruncatedCookedMesh)
        );
        assert_eq!(
            CookedMesh::decode(&encoded[..encoded.len().saturating_sub(2)]),
            Err(ModelImportError::TruncatedCookedMesh)
        );
        let mut trailing = encoded;
        trailing.push(0xFF);
        assert!(matches!(
            CookedMesh::decode(&trailing),
            Err(ModelImportError::CorruptCookedMesh(_))
        ));
        Ok(())
    }

    #[test]
    fn load_or_cook_mesh_caches_artifact_and_recooks_when_source_changes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_test_dir("cache_round_trip")?;
        let project = root.join("Project");
        let models_dir = project.join("Assets").join("Models");
        fs::create_dir_all(&models_dir)?;
        fs::create_dir_all(project.join("ProjectSettings"))?;
        let obj_path = models_dir.join("Hero.obj");
        fs::write(&obj_path, SAMPLE_OBJ.as_bytes())?;
        fs::write(
            models_dir.join("Hero.obj.meta"),
            "fileFormatVersion: 2\nguid: 11112222333344445555666677778888\n",
        )?;
        let cache_dir = root.join("Cache");
        let first = load_or_cook_mesh_with_cache(&obj_path, Some(&cache_dir))
            .ok_or("expected first load_or_cook_mesh_with_cache")?;
        assert_eq!(
            (
                first.positions().len(),
                first.indices().len(),
                first.sub_meshes().len()
            ),
            (4, 12, 2)
        );
        let cooked_dir = cache_dir.join(COOKED_MESH_DIR_NAME);
        let manifest_one =
            CookedMeshManifest::load_from_dir(&cooked_dir).ok_or("expected mesh manifest")?;
        let entry_one = manifest_one
            .get_by_asset_path("Assets/Models/Hero.obj")
            .ok_or("missing Hero.obj entry")?;
        assert!(
            entry_one.cooked_path.ends_with(".umsh")
                && cooked_dir.join(&entry_one.cooked_path).is_file()
        );
        let (_, report_cached) =
            cook_project_meshes(&project, &cache_dir, &root.join("OutCached"))?;
        assert_eq!((report_cached.imported, report_cached.reused), (0, 1));
        let second = load_or_cook_mesh_with_cache(&obj_path, Some(&cache_dir))
            .ok_or("expected cached load_or_cook_mesh_with_cache")?;
        assert_eq!(second, first);
        let modified = "v 0.0 0.0 0.0\nv 2.0 0.0 0.0\nv 0.0 2.0 0.0\nvn 0.0 0.0 1.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 0.0 1.0\ng Tri\nf 1/1/1 2/2/1 3/3/1\n";
        fs::write(&obj_path, modified.as_bytes())?;
        let third = load_or_cook_mesh_with_cache(&obj_path, Some(&cache_dir))
            .ok_or("expected recooked load_or_cook_mesh_with_cache")?;
        assert_eq!(
            (
                third.positions().len(),
                third.indices().len(),
                third.sub_meshes().len()
            ),
            (3, 3, 1)
        );
        assert_ne!(third, first);
        fs::remove_dir_all(&root)?;
        Ok(())
    }
}
