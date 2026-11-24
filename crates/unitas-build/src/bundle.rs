use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{BuildConfiguration, BuildMode, BuildValidationError};
use crate::targets::BuildTarget;

pub const BUNDLE_MANIFEST_FILENAME: &str = "bundle.manifest";
pub const DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME: &str = "development_diagnostics.marker";
pub const DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE: &str = "UNITAS_DEV_DIAGNOSTICS_ENABLED";
pub const BUNDLE_MANIFEST_FORMAT_VERSION: u32 = 1;

const SHA256_INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

pub fn content_hash_hex(bytes: &[u8]) -> String {
    let mut state = SHA256_INITIAL_STATE;
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((bytes.len() + 9).div_ceil(64) * 64);
    padded.extend_from_slice(bytes);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (word_index, word_bytes) in chunk.chunks_exact(4).enumerate() {
            schedule[word_index] = u32::from_be_bytes([
                word_bytes[0],
                word_bytes[1],
                word_bytes[2],
                word_bytes[3],
            ]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_ROUND_CONSTANTS[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut hex = String::with_capacity(64);
    for word in state {
        for byte in word.to_be_bytes() {
            let high = char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0');
            let low = char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0');
            hex.push(high);
            hex.push(low);
        }
    }
    hex
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifestEntry {
    pub index: usize,
    pub source_path: String,
    pub staged_path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub root_object_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    pub format_version: u32,
    pub company_name: String,
    pub product_name: String,
    pub application_identifier: String,
    pub version: String,
    pub target: BuildTarget,
    pub mode: BuildMode,
    pub development_diagnostics: bool,
    pub diagnostics_marker: Option<String>,
    pub startup_scene_staged_path: String,
    pub entries: Vec<BundleManifestEntry>,
    pub total_content_bytes: u64,
    pub manifest_hash: String,
}

impl BundleManifest {
    pub fn compute_manifest_hash(&self) -> String {
        content_hash_hex(self.canonical_body_without_hash().as_bytes())
    }

    fn canonical_body_without_hash(&self) -> String {
        let mut lines = Vec::with_capacity(13 + self.entries.len() * 6);
        lines.push(format!("format_version={}", self.format_version));
        lines.push(format!("company_name={}", self.company_name));
        lines.push(format!("product_name={}", self.product_name));
        lines.push(format!(
            "application_identifier={}",
            self.application_identifier
        ));
        lines.push(format!("version={}", self.version));
        lines.push(format!("target_os={}", self.target.os_name()));
        lines.push(format!("target_arch={}", self.target.arch_id()));
        lines.push(format!("mode={}", self.mode.as_str()));
        lines.push(format!(
            "development_diagnostics={}",
            self.development_diagnostics
        ));
        lines.push(format!(
            "diagnostics_marker={}",
            self.diagnostics_marker.as_deref().unwrap_or("")
        ));
        lines.push(format!(
            "startup_scene_staged_path={}",
            self.startup_scene_staged_path
        ));
        lines.push(format!("total_content_bytes={}", self.total_content_bytes));
        lines.push(format!("entry_count={}", self.entries.len()));

        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|left, right| {
            left.staged_path
                .cmp(&right.staged_path)
                .then_with(|| left.index.cmp(&right.index))
        });

        for (slot, entry) in sorted_entries.iter().enumerate() {
            lines.push(format!("entry.{slot}.index={}", entry.index));
            lines.push(format!("entry.{slot}.source_path={}", entry.source_path));
            lines.push(format!("entry.{slot}.staged_path={}", entry.staged_path));
            lines.push(format!("entry.{slot}.content_hash={}", entry.content_hash));
            lines.push(format!("entry.{slot}.size_bytes={}", entry.size_bytes));
            lines.push(format!(
                "entry.{slot}.root_object_count={}",
                entry.root_object_count
            ));
        }

        lines.join("\n")
    }

    pub fn to_canonical_string(&self) -> String {
        let mut text = self.canonical_body_without_hash();
        text.push_str(&format!("\nmanifest_hash={}\n", self.manifest_hash));
        text
    }

    pub fn from_canonical_str(text: &str) -> Result<Self, BundleError> {
        let mut map = BTreeMap::new();
        for (line_number, raw_line) in text.lines().enumerate() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| BundleError::ManifestParse {
                    message: format!(
                        "line {} is missing '=' separator: '{trimmed}'",
                        line_number + 1
                    ),
                })?;
            map.insert(key.to_string(), value.to_string());
        }

        let format_version = parse_u32_field(&map, "format_version")?;
        if format_version != BUNDLE_MANIFEST_FORMAT_VERSION {
            return Err(BundleError::ManifestParse {
                message: format!("unsupported manifest format_version '{format_version}'"),
            });
        }

        let company_name = required_field(&map, "company_name")?;
        let product_name = required_field(&map, "product_name")?;
        let application_identifier = required_field(&map, "application_identifier")?;
        let version = required_field(&map, "version")?;
        let target_os = required_field(&map, "target_os")?;
        let target_arch = required_field(&map, "target_arch")?;
        let target = BuildTarget::from_env(&target_os, &target_arch).ok_or_else(|| {
            BundleError::ManifestParse {
                message: format!("unrecognized target '{target_os}-{target_arch}'"),
            }
        })?;

        let mode_raw = required_field(&map, "mode")?;
        let mode = match mode_raw.as_str() {
            "development" => BuildMode::Development,
            "release" => BuildMode::Release,
            _ => {
                return Err(BundleError::ManifestParse {
                    message: format!("unrecognized build mode '{mode_raw}'"),
                });
            }
        };

        let development_diagnostics = parse_bool_field(&map, "development_diagnostics")?;
        let diagnostics_marker_raw = required_field(&map, "diagnostics_marker")?;
        let diagnostics_marker = if diagnostics_marker_raw.is_empty() {
            None
        } else {
            Some(diagnostics_marker_raw)
        };

        let startup_scene_staged_path = required_field(&map, "startup_scene_staged_path")?;
        let total_content_bytes = parse_u64_field(&map, "total_content_bytes")?;
        let entry_count = parse_usize_field(&map, "entry_count")?;
        let manifest_hash = required_field(&map, "manifest_hash")?;

        let mut entries = Vec::with_capacity(entry_count);
        for slot in 0..entry_count {
            let index = parse_usize_field(&map, &format!("entry.{slot}.index"))?;
            let source_path = required_field(&map, &format!("entry.{slot}.source_path"))?;
            let staged_path = required_field(&map, &format!("entry.{slot}.staged_path"))?;
            let content_hash = required_field(&map, &format!("entry.{slot}.content_hash"))?;
            let size_bytes = parse_u64_field(&map, &format!("entry.{slot}.size_bytes"))?;
            let root_object_count =
                parse_usize_field(&map, &format!("entry.{slot}.root_object_count"))?;
            entries.push(BundleManifestEntry {
                index,
                source_path,
                staged_path,
                content_hash,
                size_bytes,
                root_object_count,
            });
        }

        entries.sort_by(|left, right| {
            left.staged_path
                .cmp(&right.staged_path)
                .then_with(|| left.index.cmp(&right.index))
        });

        let manifest = Self {
            format_version,
            company_name,
            product_name,
            application_identifier,
            version,
            target,
            mode,
            development_diagnostics,
            diagnostics_marker,
            startup_scene_staged_path,
            entries,
            total_content_bytes,
            manifest_hash,
        };

        let expected_hash = manifest.compute_manifest_hash();
        if manifest.manifest_hash != expected_hash {
            return Err(BundleError::ManifestParse {
                message: format!(
                    "manifest_hash mismatch: expected '{expected_hash}', found '{}'",
                    manifest.manifest_hash
                ),
            });
        }

        Ok(manifest)
    }

    pub fn load_from_file(manifest_path: &Path) -> Result<Self, BundleError> {
        let content = fs::read_to_string(manifest_path).map_err(|error| BundleError::Io {
            path: manifest_path.to_path_buf(),
            message: error.to_string(),
        })?;
        Self::from_canonical_str(&content)
    }

    pub fn load_from_dir(output_dir: &Path) -> Result<Self, BundleError> {
        Self::load_from_file(&output_dir.join(BUNDLE_MANIFEST_FILENAME))
    }
}

fn required_field(map: &BTreeMap<String, String>, key: &str) -> Result<String, BundleError> {
    map.get(key)
        .cloned()
        .ok_or_else(|| BundleError::ManifestParse {
            message: format!("missing required field '{key}'"),
        })
}

fn parse_u32_field(map: &BTreeMap<String, String>, key: &str) -> Result<u32, BundleError> {
    let raw = required_field(map, key)?;
    raw.parse::<u32>().map_err(|error| BundleError::ManifestParse {
        message: format!("invalid u32 value for '{key}' ('{raw}'): {error}"),
    })
}

fn parse_u64_field(map: &BTreeMap<String, String>, key: &str) -> Result<u64, BundleError> {
    let raw = required_field(map, key)?;
    raw.parse::<u64>().map_err(|error| BundleError::ManifestParse {
        message: format!("invalid u64 value for '{key}' ('{raw}'): {error}"),
    })
}

fn parse_usize_field(map: &BTreeMap<String, String>, key: &str) -> Result<usize, BundleError> {
    let raw = required_field(map, key)?;
    raw.parse::<usize>()
        .map_err(|error| BundleError::ManifestParse {
            message: format!("invalid usize value for '{key}' ('{raw}'): {error}"),
        })
}

fn parse_bool_field(map: &BTreeMap<String, String>, key: &str) -> Result<bool, BundleError> {
    let raw = required_field(map, key)?;
    match raw.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(BundleError::ManifestParse {
            message: format!("invalid boolean value for '{key}' ('{raw}')"),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedBundle {
    pub output_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub launch_artifact_path: PathBuf,
    pub diagnostics_marker_path: Option<PathBuf>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    Validation(Vec<BuildValidationError>),
    SceneMissing { path: PathBuf },
    SceneAssembly { path: PathBuf, message: String },
    Io { path: PathBuf, message: String },
    ManifestParse { message: String },
}

impl fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::Validation(errors) => {
                let joined = errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(formatter, "build configuration validation failed: {joined}")
            }
            BundleError::SceneMissing { path } => {
                write!(formatter, "scene file does not exist: '{}'", path.display())
            }
            BundleError::SceneAssembly { path, message } => {
                write!(
                    formatter,
                    "failed to assemble scene '{}': {message}",
                    path.display()
                )
            }
            BundleError::Io { path, message } => {
                write!(formatter, "I/O error at '{}': {message}", path.display())
            }
            BundleError::ManifestParse { message } => {
                write!(formatter, "invalid bundle manifest: {message}")
            }
        }
    }
}

impl std::error::Error for BundleError {}

struct VerifiedSceneInput {
    index: usize,
    source_path: String,
    staged_path: String,
    bytes: Vec<u8>,
    content_hash: String,
    size_bytes: u64,
    root_object_count: usize,
}

pub fn stage_bundle(
    config: &BuildConfiguration,
    output_dir: &Path,
) -> Result<StagedBundle, BundleError> {
    stage_bundle_for_host(config, output_dir, BuildTarget::current_host())
}

pub fn stage_bundle_for_host(
    config: &BuildConfiguration,
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<StagedBundle, BundleError> {
    config
        .validate_with_host(host)
        .map_err(BundleError::Validation)?;

    let mut verified_scenes = Vec::with_capacity(config.scenes.len());
    for (index, scene_path) in config.scenes.iter().enumerate() {
        if !scene_path.is_file() {
            return Err(BundleError::SceneMissing {
                path: scene_path.clone(),
            });
        }
        let scene_graph =
            unitas_asset::assemble_scene_file(scene_path).map_err(|error| BundleError::SceneAssembly {
                path: scene_path.clone(),
                message: error.to_string(),
            })?;
        let bytes = fs::read(scene_path).map_err(|error| BundleError::Io {
            path: scene_path.clone(),
            message: error.to_string(),
        })?;
        let content_hash = content_hash_hex(&bytes);
        let size_bytes = bytes.len() as u64;
        let root_object_count = scene_graph.roots().len();
        let file_name = scene_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("scene.unity");
        let staged_path = format!("Assets/scene_{index:03}_{file_name}");
        let source_path = normalize_scene_source_path(scene_path, file_name);

        verified_scenes.push(VerifiedSceneInput {
            index,
            source_path,
            staged_path,
            bytes,
            content_hash,
            size_bytes,
            root_object_count,
        });
    }

    let assets_dir = output_dir.join("Assets");
    fs::create_dir_all(&assets_dir).map_err(|error| BundleError::Io {
        path: assets_dir.clone(),
        message: error.to_string(),
    })?;

    let mut startup_scene_staged_path = String::new();
    let mut total_content_bytes = 0u64;
    let mut entries = Vec::with_capacity(verified_scenes.len());

    for verified in verified_scenes {
        let dest_path = output_dir.join(&verified.staged_path);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|error| BundleError::Io {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        fs::write(&dest_path, &verified.bytes).map_err(|error| BundleError::Io {
            path: dest_path,
            message: error.to_string(),
        })?;

        if verified.index == 0 {
            startup_scene_staged_path.clone_from(&verified.staged_path);
        }
        total_content_bytes = total_content_bytes.saturating_add(verified.size_bytes);
        entries.push(BundleManifestEntry {
            index: verified.index,
            source_path: verified.source_path,
            staged_path: verified.staged_path,
            content_hash: verified.content_hash,
            size_bytes: verified.size_bytes,
            root_object_count: verified.root_object_count,
        });
    }

    entries.sort_by(|left, right| {
        left.staged_path
            .cmp(&right.staged_path)
            .then_with(|| left.index.cmp(&right.index))
    });

    let include_diagnostics = config.include_diagnostics();
    let marker_file_path = output_dir.join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME);
    let (diagnostics_marker, diagnostics_marker_path) = if include_diagnostics {
        let marker_value = DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE.to_string();
        let marker_content = format!(
            "marker={marker_value}\nproduct_name={}\napplication_identifier={}\nmode={}\n",
            config.product_name,
            config.application_identifier,
            config.mode().as_str()
        );
        fs::write(&marker_file_path, marker_content).map_err(|error| BundleError::Io {
            path: marker_file_path.clone(),
            message: error.to_string(),
        })?;
        (Some(marker_value), Some(marker_file_path))
    } else {
        if marker_file_path.exists() {
            fs::remove_file(&marker_file_path).map_err(|error| BundleError::Io {
                path: marker_file_path.clone(),
                message: error.to_string(),
            })?;
        }
        (None, None)
    };

    let mut manifest = BundleManifest {
        format_version: BUNDLE_MANIFEST_FORMAT_VERSION,
        company_name: config.company_name.clone(),
        product_name: config.product_name.clone(),
        application_identifier: config.application_identifier.clone(),
        version: config.version.clone(),
        target: config.target,
        mode: config.mode(),
        development_diagnostics: include_diagnostics,
        diagnostics_marker,
        startup_scene_staged_path,
        entries,
        total_content_bytes,
        manifest_hash: String::new(),
    };
    manifest.manifest_hash = manifest.compute_manifest_hash();

    let manifest_path = output_dir.join(BUNDLE_MANIFEST_FILENAME);
    let canonical_manifest = manifest.to_canonical_string();
    fs::write(&manifest_path, canonical_manifest).map_err(|error| BundleError::Io {
        path: manifest_path.clone(),
        message: error.to_string(),
    })?;

    let launch_artifact_path = output_dir.join(config.target.executable_name(&config.product_name));
    let launcher_descriptor = format!(
        "unitas_player_launcher=v1\n\
         company_name={}\n\
         product_name={}\n\
         application_identifier={}\n\
         version={}\n\
         target={}\n\
         mode={}\n\
         development_diagnostics={}\n\
         manifest={BUNDLE_MANIFEST_FILENAME}\n\
         manifest_hash={}\n\
         startup_scene={}\n",
        manifest.company_name,
        manifest.product_name,
        manifest.application_identifier,
        manifest.version,
        manifest.target.identifier(),
        manifest.mode.as_str(),
        manifest.development_diagnostics,
        manifest.manifest_hash,
        manifest.startup_scene_staged_path
    );
    fs::write(&launch_artifact_path, launcher_descriptor).map_err(|error| BundleError::Io {
        path: launch_artifact_path.clone(),
        message: error.to_string(),
    })?;

    Ok(StagedBundle {
        output_dir: output_dir.to_path_buf(),
        manifest_path,
        launch_artifact_path,
        diagnostics_marker_path,
        manifest,
    })
}

fn normalize_scene_source_path(scene_path: &Path, fallback_name: &str) -> String {
    let raw = scene_path.to_string_lossy().replace('\\', "/");
    if let Some((_, tail)) = raw.rsplit_once("/Assets/") {
        return format!("Assets/{tail}");
    }
    if raw.starts_with("Assets/") {
        return raw;
    }
    fallback_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "unitas_build_bundle_test_{label}_{}_{id}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn sample_unity_scene(root_name: &str) -> String {
        format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: {root_name}\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_Father: {{fileID: 0}}\n  m_Children: []\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n"
        )
    }

    #[test]
    fn content_hash_hex_matches_sha256_test_vectors() {
        let expected_empty = [
            "e3b0c442", "98fc1c14", "9afbf4c8", "996fb924", "27ae41e4", "649b934c", "a495991b",
            "7852b855",
        ]
        .concat();
        let expected_abc = [
            "ba7816bf", "8f01cfea", "414140de", "5dae2223", "b00361a3", "96177a9c", "b410ff61",
            "f20015ad",
        ]
        .concat();
        assert_eq!(content_hash_hex(b""), expected_empty);
        assert_eq!(content_hash_hex(b"abc"), expected_abc);
    }

    #[test]
    fn repeated_builds_produce_byte_identical_manifests_and_hashes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("determinism")?;
        let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
        fs::create_dir_all(&scenes_dir)?;
        let main_scene = scenes_dir.join("Main.unity");
        let level_scene = scenes_dir.join("LevelTwo.unity");
        fs::write(&main_scene, sample_unity_scene("MainRoot"))?;
        fs::write(&level_scene, sample_unity_scene("LevelRoot"))?;

        let host = BuildTarget::MACOS_ARM64;
        let config = BuildConfiguration::new(
            vec![main_scene, level_scene],
            "Unitas",
            "DeterministicGame",
            "com.unitas.deterministic",
            "1.2.3",
            host,
        );

        let out_one = workspace.join("out_one");
        let out_two = workspace.join("out_two");

        let bundle_one = stage_bundle_for_host(&config, &out_one, Some(host))?;
        let bundle_two = stage_bundle_for_host(&config, &out_two, Some(host))?;

        let manifest_bytes_one = fs::read(&bundle_one.manifest_path)?;
        let manifest_bytes_two = fs::read(&bundle_two.manifest_path)?;
        assert_eq!(manifest_bytes_one, manifest_bytes_two);
        assert_eq!(bundle_one.manifest, bundle_two.manifest);
        assert_eq!(
            bundle_one.manifest.manifest_hash,
            bundle_two.manifest.manifest_hash
        );
        assert_eq!(bundle_one.manifest.manifest_hash.len(), 64);
        assert_eq!(bundle_one.manifest.entries.len(), 2);
        assert_eq!(
            bundle_one.manifest.startup_scene_staged_path,
            "Assets/scene_000_Main.unity"
        );
        for entry in &bundle_one.manifest.entries {
            assert_eq!(entry.root_object_count, 1);
            assert_eq!(entry.content_hash.len(), 64);
            assert!(out_one.join(&entry.staged_path).is_file());
        }
        assert!(bundle_one.launch_artifact_path.is_file());

        let round_trip = BundleManifest::load_from_dir(&out_one)?;
        assert_eq!(round_trip, bundle_one.manifest);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    fn development_and_release_builds_differ_by_diagnostics_marker_and_clean_stale_marker(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("dev_vs_release")?;
        let scene_path = workspace.join("Main.unity");
        fs::write(&scene_path, sample_unity_scene("PlayerRoot"))?;

        let host = BuildTarget::MACOS_ARM64;
        let out_dir = workspace.join("build_out");

        let dev_config = BuildConfiguration::new(
            vec![scene_path.clone()],
            "Unitas",
            "DiagGame",
            "com.unitas.diag",
            "0.5.0",
            host,
        )
        .with_development_build(true);

        let dev_bundle = stage_bundle_for_host(&dev_config, &out_dir, Some(host))?;
        assert!(dev_bundle.manifest.development_diagnostics);
        assert_eq!(
            dev_bundle.manifest.diagnostics_marker,
            Some(DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE.to_string())
        );
        let marker_path = out_dir.join(DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME);
        assert_eq!(
            dev_bundle.diagnostics_marker_path,
            Some(marker_path.clone())
        );
        assert!(marker_path.is_file());

        let release_config = dev_config.with_development_build(false);
        let release_bundle = stage_bundle_for_host(&release_config, &out_dir, Some(host))?;
        assert!(!release_bundle.manifest.development_diagnostics);
        assert_eq!(release_bundle.manifest.diagnostics_marker, None);
        assert_eq!(release_bundle.diagnostics_marker_path, None);
        assert!(!marker_path.exists());
        assert_ne!(
            dev_bundle.manifest.manifest_hash,
            release_bundle.manifest.manifest_hash
        );

        let loaded_release = BundleManifest::load_from_dir(&out_dir)?;
        assert_eq!(loaded_release, release_bundle.manifest);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    fn missing_or_malformed_scene_fails_without_writing_manifest_or_launch_artifact(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("invalid_scenes")?;
        let host = BuildTarget::MACOS_ARM64;

        let missing_scene = workspace.join("DoesNotExist.unity");
        let missing_out = workspace.join("missing_out");
        let missing_config = BuildConfiguration::new(
            vec![missing_scene],
            "Unitas",
            "MissingSceneGame",
            "com.unitas.missing",
            "1.0.0",
            host,
        );

        let missing_result = stage_bundle_for_host(&missing_config, &missing_out, Some(host));
        assert!(matches!(
            missing_result,
            Err(BundleError::SceneMissing { .. })
        ));
        assert!(!missing_out.join(BUNDLE_MANIFEST_FILENAME).exists());
        assert!(!missing_out
            .join(host.executable_name(&missing_config.product_name))
            .exists());

        let broken_scene = workspace.join("Broken.unity");
        fs::write(
            &broken_scene,
            "%YAML 1.1\n\
%TAG !u! tag:unity3d.com,2011:\n\
--- !u!4 &201\n\
Transform:\n\
  m_GameObject: {fileID: 999}\n",
        )?;
        let broken_out = workspace.join("broken_out");
        let broken_config = BuildConfiguration::new(
            vec![broken_scene],
            "Unitas",
            "BrokenSceneGame",
            "com.unitas.broken",
            "1.0.0",
            host,
        );

        let broken_result = stage_bundle_for_host(&broken_config, &broken_out, Some(host));
        assert!(matches!(
            broken_result,
            Err(BundleError::SceneAssembly { .. })
        ));
        assert!(!broken_out.join(BUNDLE_MANIFEST_FILENAME).exists());
        assert!(!broken_out
            .join(host.executable_name(&broken_config.product_name))
            .exists());

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }
}
