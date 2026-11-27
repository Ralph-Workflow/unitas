use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dependencies::{
    AssetImportState, ImportDiagnostic, ImportFingerprint, ImportRecord, UNCHANGED_REASON,
};
use crate::json::{self, JsonValue};

const CACHE_FILE_NAME: &str = "import-cache.json";
const CACHE_FORMAT: &str = "unitas-import-cache";
const CACHE_FORMAT_VERSION: i32 = 1;
const TEMP_SUFFIX: &str = ".tmp";
static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub const LIBRARY_DIRECTORY: &str = "Library";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCacheError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ImportCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot write the import cache at {}: {}",
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for ImportCacheError {}

#[derive(Debug, Clone, Default)]
pub struct ImportCache {
    directory: PathBuf,
    records: BTreeMap<PathBuf, ImportRecord>,
}

impl ImportCache {
    pub fn open(directory: &Path) -> Self {
        let mut cache = ImportCache {
            directory: directory.to_path_buf(),
            records: BTreeMap::new(),
        };
        cache.load();
        cache
    }

    pub fn file_path(&self) -> PathBuf {
        self.directory.join(CACHE_FILE_NAME)
    }

    pub fn record(&self, path: &Path) -> Option<&ImportRecord> {
        self.records.get(path)
    }

    pub fn records(&self) -> impl Iterator<Item = &ImportRecord> + '_ {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn replace(&mut self, records: impl IntoIterator<Item = ImportRecord>) {
        self.records = records
            .into_iter()
            .map(|record| (record.path.clone(), record))
            .collect();
    }

    pub fn save(&self) -> Result<(), ImportCacheError> {
        let path = self.file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| ImportCacheError {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        write_atomic(&path, self.encode().as_bytes())
    }

    fn load(&mut self) {
        let path = self.file_path();
        let Ok(text) = fs::read_to_string(&path) else {
            return;
        };
        let Ok(value) = json::parse(&text) else {
            return;
        };
        if value.get("format").and_then(JsonValue::as_str) != Some(CACHE_FORMAT) {
            return;
        }
        if value.get("formatVersion").and_then(JsonValue::as_i64)
            != Some(i64::from(CACHE_FORMAT_VERSION))
        {
            return;
        }
        let Some(assets) = value.get("assets").and_then(JsonValue::as_array) else {
            return;
        };
        for asset in assets {
            if let Some(record) = decode_record(asset) {
                self.records.insert(record.path.clone(), record);
            }
        }
    }

    fn encode(&self) -> String {
        let assets: Vec<JsonValue> = self.records.values().map(encode_record).collect();
        json::write(&JsonValue::Object(vec![
            (
                "format".to_string(),
                JsonValue::String(CACHE_FORMAT.to_string()),
            ),
            (
                "formatVersion".to_string(),
                JsonValue::Number(f64::from(CACHE_FORMAT_VERSION)),
            ),
            ("assets".to_string(), JsonValue::Array(assets)),
        ]))
    }
}

fn encode_record(record: &ImportRecord) -> JsonValue {
    JsonValue::Object(vec![
        (
            "path".to_string(),
            JsonValue::String(record.path.display().to_string()),
        ),
        ("guid".to_string(), JsonValue::String(record.guid.clone())),
        (
            "source".to_string(),
            JsonValue::String(encode_hash(record.fingerprint.source)),
        ),
        (
            "metadata".to_string(),
            JsonValue::String(encode_hash(record.fingerprint.metadata)),
        ),
        (
            "settings".to_string(),
            JsonValue::String(encode_hash(record.fingerprint.settings)),
        ),
        (
            "importerVersion".to_string(),
            JsonValue::String(encode_hash(record.fingerprint.importer_version)),
        ),
        (
            "dependencies".to_string(),
            JsonValue::String(encode_hash(record.fingerprint.dependencies)),
        ),
        (
            "state".to_string(),
            JsonValue::String(encode_state(record.state).to_string()),
        ),
        (
            "diagnostics".to_string(),
            JsonValue::Array(record.diagnostics.iter().map(encode_diagnostic).collect()),
        ),
    ])
}

fn encode_diagnostic(diagnostic: &ImportDiagnostic) -> JsonValue {
    JsonValue::Object(vec![
        (
            "asset".to_string(),
            JsonValue::String(diagnostic.asset.clone()),
        ),
        (
            "subObject".to_string(),
            encode_optional(&diagnostic.sub_object),
        ),
        (
            "issue".to_string(),
            JsonValue::String(diagnostic.issue.clone()),
        ),
        (
            "consequence".to_string(),
            JsonValue::String(diagnostic.consequence.clone()),
        ),
        (
            "partiallyUsable".to_string(),
            JsonValue::Bool(diagnostic.partially_usable),
        ),
        ("context".to_string(), encode_optional(&diagnostic.context)),
    ])
}

fn encode_optional(value: &Option<String>) -> JsonValue {
    match value {
        Some(text) => JsonValue::String(text.clone()),
        None => JsonValue::Null,
    }
}

fn encode_hash(value: u64) -> String {
    format!("{value:016x}")
}

fn encode_state(state: AssetImportState) -> &'static str {
    match state {
        AssetImportState::NotYetEvaluated => "NotYetEvaluated",
        AssetImportState::Processing => "Processing",
        AssetImportState::Ready => "Ready",
        AssetImportState::ReadyWithLimitations => "ReadyWithLimitations",
        AssetImportState::Failed => "Failed",
        AssetImportState::Missing => "Missing",
        AssetImportState::BlockedByDependency => "BlockedByDependency",
        AssetImportState::BlockedByTrust => "BlockedByTrust",
    }
}

fn decode_record(asset: &JsonValue) -> Option<ImportRecord> {
    let path = asset.get("path")?.as_str()?.to_string();
    let guid = asset.get("guid")?.as_str()?.to_string();
    let fingerprint = ImportFingerprint {
        source: decode_hash(asset.get("source")?)?,
        metadata: decode_hash(asset.get("metadata")?)?,
        settings: decode_hash(asset.get("settings")?)?,
        importer_version: decode_hash(asset.get("importerVersion")?)?,
        dependencies: decode_hash(asset.get("dependencies")?)?,
    };
    let state = decode_state(asset.get("state")?.as_str()?)?;
    let diagnostics = decode_diagnostics(asset.get("diagnostics")?)?;
    Some(ImportRecord {
        path: PathBuf::from(path),
        guid,
        state,
        diagnostics,
        fingerprint,
        reused: false,
        reason: UNCHANGED_REASON.to_string(),
    })
}

fn decode_diagnostics(value: &JsonValue) -> Option<Vec<ImportDiagnostic>> {
    let items = value.as_array()?;
    let mut diagnostics = Vec::with_capacity(items.len());
    for item in items {
        let asset = item.get("asset")?.as_str()?.to_string();
        let sub_object = decode_optional(item.get("subObject")?);
        let issue = item.get("issue")?.as_str()?.to_string();
        let consequence = item.get("consequence")?.as_str()?.to_string();
        let partially_usable = item.get("partiallyUsable")?.as_bool()?;
        let context = decode_optional(item.get("context")?);
        diagnostics.push(ImportDiagnostic {
            asset,
            sub_object,
            issue,
            consequence,
            partially_usable,
            context,
        });
    }
    Some(diagnostics)
}

fn decode_optional(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::Null => None,
        JsonValue::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn decode_hash(value: &JsonValue) -> Option<u64> {
    u64::from_str_radix(value.as_str()?, 16).ok()
}

fn decode_state(name: &str) -> Option<AssetImportState> {
    match name {
        "NotYetEvaluated" => Some(AssetImportState::NotYetEvaluated),
        "Processing" => Some(AssetImportState::Processing),
        "Ready" => Some(AssetImportState::Ready),
        "ReadyWithLimitations" => Some(AssetImportState::ReadyWithLimitations),
        "Failed" => Some(AssetImportState::Failed),
        "Missing" => Some(AssetImportState::Missing),
        "BlockedByDependency" => Some(AssetImportState::BlockedByDependency),
        "BlockedByTrust" => Some(AssetImportState::BlockedByTrust),
        _ => None,
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ImportCacheError> {
    let temp = temp_path(path);
    if let Err(error) = fs::write(&temp, bytes) {
        return Err(ImportCacheError {
            path: temp,
            message: error.to_string(),
        });
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(ImportCacheError {
            path: path.to_path_buf(),
            message: error.to_string(),
        });
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let seq = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut temp = path.file_name().unwrap_or_default().to_os_string();
    temp.push(format!(".{}.{seq}{TEMP_SUFFIX}", std::process::id()));
    path.with_file_name(temp)
}
