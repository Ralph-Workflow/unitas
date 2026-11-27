use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{BuildConfiguration, BuildMode, BuildValidationError};
use crate::licensing;
use crate::package::{self, PackagedApp};
use crate::player::ScriptHost;
use crate::targets::{BuildTarget, TargetOs};

pub const BUNDLE_MANIFEST_FILENAME: &str = "bundle.manifest";
pub const DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME: &str = "development_diagnostics.marker";
pub const DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE: &str = "UNITAS_DEV_DIAGNOSTICS_ENABLED";
pub const BUNDLE_MANIFEST_FORMAT_VERSION: u32 = 2;
pub const MANAGED_DIR_NAME: &str = "Managed";
pub const PROJECT_SETTINGS_DIR_NAME: &str = "ProjectSettings";
pub const MANAGED_GAME_ASSEMBLY_FILENAME: &str = "Assembly-CSharp.dll";
pub const PLAYER_HOST_ASSEMBLY_FILENAME: &str = "Unitas.PlayerHost.dll";
pub const PLAYER_HOST_RUNTIME_CONFIG_FILENAME: &str = "Unitas.PlayerHost.runtimeconfig.json";
pub const PLAYER_HOST_PAYLOAD_FILENAMES: [&str; 8] = [
    "Unitas.PlayerHost.dll",
    "Unitas.PlayerHost.deps.json",
    "Unitas.PlayerHost.runtimeconfig.json",
    "Unitas.Bridge.dll",
    "Unitas.Scripting.dll",
    "UnityEngine.dll",
    "Microsoft.CodeAnalysis.dll",
    "Microsoft.CodeAnalysis.CSharp.dll",
];

#[cfg(target_os = "windows")]
pub const RUNTIME_LIBRARY_FILENAME: &str = "unitas_runtime.dll";
#[cfg(target_os = "macos")]
pub const RUNTIME_LIBRARY_FILENAME: &str = "libunitas_runtime.dylib";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const RUNTIME_LIBRARY_FILENAME: &str = "libunitas_runtime.so";

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
            schedule[word_index] =
                u32::from_be_bytes([word_bytes[0], word_bytes[1], word_bytes[2], word_bytes[3]]);
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
    pub build_identifier: String,
    pub target: BuildTarget,
    pub mode: BuildMode,
    pub script_host: ScriptHost,
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

    pub fn compute_build_identifier(&self) -> String {
        let hash = content_hash_hex(self.canonical_body_core().as_bytes());
        hash.chars().take(16).collect()
    }

    fn canonical_body_core(&self) -> String {
        let mut lines = Vec::with_capacity(14 + self.entries.len() * 6);
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
        lines.push(format!("script_host={}", self.script_host.as_str()));
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

    fn canonical_body_without_hash(&self) -> String {
        let mut text = self.canonical_body_core();
        text.push_str(&format!("\nbuild_identifier={}", self.build_identifier));
        text
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
            let (key, value) =
                trimmed
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
        let build_identifier = required_field(&map, "build_identifier")?;
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

        let script_host_raw = required_field(&map, "script_host")?;
        let script_host =
            ScriptHost::from_name(&script_host_raw).ok_or_else(|| BundleError::ManifestParse {
                message: format!("unrecognized script_host '{script_host_raw}'"),
            })?;

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
            build_identifier,
            target,
            mode,
            script_host,
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
    raw.parse::<u32>()
        .map_err(|error| BundleError::ManifestParse {
            message: format!("invalid u32 value for '{key}' ('{raw}'): {error}"),
        })
}

fn parse_u64_field(map: &BTreeMap<String, String>, key: &str) -> Result<u64, BundleError> {
    let raw = required_field(map, key)?;
    raw.parse::<u64>()
        .map_err(|error| BundleError::ManifestParse {
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
    pub application_bundle_path: Option<PathBuf>,
    pub diagnostics_marker_path: Option<PathBuf>,
    pub manifest: BundleManifest,
    /// Non-fatal staging problems (for example a managed payload file that is
    /// not built yet); the bundle is complete without them, but the player
    /// cannot run scripts until they are resolved.
    pub diagnostics: Vec<String>,
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
    let host = host.or_else(BuildTarget::current_host);
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
        let scene_graph = unitas_asset::assemble_scene_file(scene_path).map_err(|error| {
            BundleError::SceneAssembly {
                path: scene_path.clone(),
                message: error.to_string(),
            }
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

    let mut diagnostics: Vec<String> = Vec::new();
    let mut script_host = ScriptHost::None;
    if let Some(project_root) = config
        .scenes
        .first()
        .and_then(|scene_path| unity_project_root(scene_path))
    {
        stage_project_assets(&project_root, output_dir)?;
        crate::cooked::stage_project_cooked_textures(&project_root, output_dir)?;
        stage_project_settings(&project_root, output_dir)?;
        if project_has_scripts(&project_root) {
            script_host = ScriptHost::CoreClr;
            stage_runtime_library(
                output_dir,
                &mut entries,
                &mut total_content_bytes,
                &mut diagnostics,
            );
            stage_managed_payload(
                &project_root,
                output_dir,
                &mut entries,
                &mut total_content_bytes,
                &mut diagnostics,
                config.script_debugging,
            )?;
        }
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
        build_identifier: String::new(),
        target: config.target,
        mode: config.mode(),
        script_host,
        development_diagnostics: include_diagnostics,
        diagnostics_marker,
        startup_scene_staged_path,
        entries,
        total_content_bytes,
        manifest_hash: String::new(),
    };
    manifest.build_identifier = manifest.compute_build_identifier();
    manifest.manifest_hash = manifest.compute_manifest_hash();

    let manifest_path = output_dir.join(BUNDLE_MANIFEST_FILENAME);
    let canonical_manifest = manifest.to_canonical_string();
    fs::write(&manifest_path, canonical_manifest).map_err(|error| BundleError::Io {
        path: manifest_path.clone(),
        message: error.to_string(),
    })?;

    let mut application_bundle_path = None;
    let launch_artifact_path = match macos_application(&manifest, output_dir)? {
        Some(application) => {
            application_bundle_path = Some(application.app_path);
            application.executable_path
        }
        None => {
            let Some(player_binary) = package::locate_player_binary() else {
                return Err(BundleError::Io {
                    path: output_dir.to_path_buf(),
                    message: String::from("the unitas player binary has not been built"),
                });
            };
            stage_player_launcher(&manifest, output_dir, &player_binary)?
        }
    };

    Ok(StagedBundle {
        output_dir: output_dir.to_path_buf(),
        manifest_path,
        launch_artifact_path,
        application_bundle_path,
        diagnostics_marker_path,
        manifest,
        diagnostics,
    })
}

fn stage_player_launcher(
    manifest: &BundleManifest,
    output_dir: &Path,
    player_binary: &Path,
) -> Result<PathBuf, BundleError> {
    let path = output_dir.join(manifest.target.executable_name(&manifest.product_name));
    fs::copy(player_binary, &path).map_err(|error| BundleError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    package::make_executable(&path).map_err(|error| BundleError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok(path)
}

fn stage_project_assets(project_root: &Path, output_dir: &Path) -> Result<(), BundleError> {
    crate::cooked::stage_project_assets_without_raw_textures(project_root, output_dir)
}

fn stage_project_settings(project_root: &Path, output_dir: &Path) -> Result<(), BundleError> {
    let source = project_root.join(PROJECT_SETTINGS_DIR_NAME);
    if !source.is_dir() {
        return Ok(());
    }
    let destination = output_dir.join(PROJECT_SETTINGS_DIR_NAME);
    if destination.starts_with(&source) {
        return Ok(());
    }
    copy_directory(&source, &destination)
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), BundleError> {
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
            copy_directory(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|error| BundleError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
        }
    }
    Ok(())
}

/// The Unity-style project root a scene belongs to: the nearest ancestor
/// directory that contains an `Assets` folder. Returns `None` for scenes
/// that do not live inside a project (for example test fixtures).
fn unity_project_root(scene_path: &Path) -> Option<PathBuf> {
    let mut directory = scene_path.parent()?;
    loop {
        if directory.join("Assets").is_dir() {
            return Some(directory.to_path_buf());
        }
        directory = directory.parent()?;
    }
}

/// Whether a Unity-style project directory has any C# scripts under
/// `Assets/` (searched a few levels deep).
fn project_has_scripts(project_root: &Path) -> bool {
    fn dir_contains_cs(directory: &Path, depth: usize) -> bool {
        if depth == 0 {
            return false;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return false;
        };
        entries.flatten().any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                dir_contains_cs(&path, depth - 1)
            } else {
                path.extension().is_some_and(|extension| extension == "cs")
            }
        })
    }
    dir_contains_cs(&project_root.join("Assets"), 8)
}

/// Stages the runtime library the player loads in process so the managed
/// player host and the native launcher bind the same file. The bundle stays
/// self-contained: the library travels with the content it hosts.
fn stage_runtime_library(
    output_dir: &Path,
    entries: &mut Vec<BundleManifestEntry>,
    total_content_bytes: &mut u64,
    diagnostics: &mut Vec<String>,
) {
    let Some(source) = locate_runtime_library() else {
        diagnostics.push(format!(
            "the runtime library '{RUNTIME_LIBRARY_FILENAME}' was not found; the packaged \
             player cannot host scripts without it"
        ));
        return;
    };
    let bytes = match fs::read(&source) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(format!(
                "the runtime library '{}' could not be read: {error}",
                source.display()
            ));
            return;
        }
    };
    let destination = output_dir.join(RUNTIME_LIBRARY_FILENAME);
    if let Err(error) = fs::write(&destination, &bytes) {
        diagnostics.push(format!(
            "the runtime library could not be staged at '{}': {error}",
            destination.display()
        ));
        return;
    }
    *total_content_bytes = total_content_bytes.saturating_add(bytes.len() as u64);
    entries.push(BundleManifestEntry {
        index: entries.len(),
        source_path: source.to_string_lossy().replace('\\', "/"),
        staged_path: RUNTIME_LIBRARY_FILENAME.to_string(),
        content_hash: content_hash_hex(&bytes),
        size_bytes: bytes.len() as u64,
        root_object_count: 0,
    });
}

/// The runtime library of the local build: next to the running binary, in the
/// build dependency directory, or in the workspace target directory.
fn locate_runtime_library() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(RUNTIME_LIBRARY_FILENAME));
            candidates.push(directory.join("deps").join(RUNTIME_LIBRARY_FILENAME));
        }
    }
    if let Some(root) = licensing::workspace_root() {
        for profile in ["debug", "release"] {
            candidates.push(
                root.join("target")
                    .join(profile)
                    .join(RUNTIME_LIBRARY_FILENAME),
            );
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Stages the `Managed/` payload for a project: the game assembly compiled
/// from the project's `Assets` scripts at build time through the existing
/// bridge compile pipeline, plus the `Unitas.PlayerHost` host files. The
/// payload list comes from the build outputs; anything that is not built yet
/// (for example `Unitas.PlayerHost` before its owning unit lands) degrades
/// to a diagnostic instead of failing the build.
fn stage_managed_payload(
    project_root: &Path,
    output_dir: &Path,
    entries: &mut Vec<BundleManifestEntry>,
    total_content_bytes: &mut u64,
    diagnostics: &mut Vec<String>,
    script_debugging: bool,
) -> Result<(), BundleError> {
    let mut sources: Vec<(String, PathBuf)> = Vec::new();

    let build_dir = output_dir.join("_managed_build");
    match compile_project_scripts(project_root, &build_dir, script_debugging) {
        Ok(true) => {
            let mut game_assemblies: Vec<PathBuf> = fs::read_dir(&build_dir)
                .map_err(|error| BundleError::Io {
                    path: build_dir.clone(),
                    message: error.to_string(),
                })?
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && path.extension().is_some_and(|extension| extension == "dll")
                })
                .collect();
            game_assemblies.sort();
            if game_assemblies.is_empty() {
                diagnostics.push(format!(
                    "script compilation for '{}' produced no game assemblies",
                    project_root.display()
                ));
            }
            for assembly in game_assemblies {
                if let Some(name) = assembly.file_name().and_then(|name| name.to_str()) {
                    sources.push((name.to_string(), assembly));
                }
            }
        }
        Ok(false) => diagnostics.push(format!(
            "script compilation for '{}' was skipped: bridge/Unitas.Scripting is not built; \
             run the dotnet-build gate first",
            project_root.display()
        )),
        Err(message) => diagnostics.push(format!(
            "script compilation for '{}' failed: {message}",
            project_root.display()
        )),
    }

    if let Some(workspace_root) = licensing::workspace_root() {
        let host_output_dir = workspace_root
            .join("bridge")
            .join("Unitas.PlayerHost")
            .join("bin")
            .join("Release")
            .join("net8.0");
        for filename in PLAYER_HOST_PAYLOAD_FILENAMES {
            let path = host_output_dir.join(filename);
            if path.is_file() {
                sources.push((filename.to_string(), path));
            } else {
                diagnostics.push(format!(
                    "managed payload '{filename}' is missing at '{}'; the packaged player \
                     cannot host scripts until Unitas.PlayerHost is built",
                    path.display()
                ));
            }
        }
    }

    if sources.is_empty() {
        return Ok(());
    }

    let managed_dir = output_dir.join(MANAGED_DIR_NAME);
    fs::create_dir_all(&managed_dir).map_err(|error| BundleError::Io {
        path: managed_dir.clone(),
        message: error.to_string(),
    })?;
    for (filename, source) in sources {
        let bytes = fs::read(&source).map_err(|error| BundleError::Io {
            path: source.clone(),
            message: error.to_string(),
        })?;
        let staged_path = format!("{MANAGED_DIR_NAME}/{filename}");
        fs::write(managed_dir.join(&filename), &bytes).map_err(|error| BundleError::Io {
            path: managed_dir.join(&filename),
            message: error.to_string(),
        })?;
        *total_content_bytes = total_content_bytes.saturating_add(bytes.len() as u64);
        entries.push(BundleManifestEntry {
            index: entries.len(),
            source_path: source.to_string_lossy().replace('\\', "/"),
            staged_path,
            content_hash: content_hash_hex(&bytes),
            size_bytes: bytes.len() as u64,
            root_object_count: 0,
        });
    }
    let _ = fs::remove_dir_all(&build_dir);
    Ok(())
}

/// Copies the cached game assemblies into the staging output. Returns whether
/// the cache held a complete assembly set.
fn publish_cached_assemblies(cache_dir: &Path, output_dir: &Path) -> Result<bool, String> {
    if !cache_dir.join(".complete").is_file() {
        return Ok(false);
    }
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Ok(false);
    };
    let mut published = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|extension| extension != "dll") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        fs::copy(&path, output_dir.join(name)).map_err(|error| error.to_string())?;
        published = true;
    }
    Ok(published)
}

/// The cache key of a script compile: the project's script sources, the
/// bridge assemblies the compile reads, and whether the compile runs with
/// script debugging. A change to any of them invalidates it, so a debug and a
/// non-debug compile never share a cache entry.
fn script_compile_cache_key(
    workspace_root: &Path,
    project_root: &Path,
    script_debugging: bool,
) -> String {
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    collect_script_sources(&project_root.join("Assets"), "", &mut parts, 8);
    collect_bridge_assemblies(&workspace_root.join("bridge"), &mut parts, 8);
    parts.push((
        String::from("script-debugging"),
        vec![u8::from(script_debugging)],
    ));
    parts.sort_by(|left, right| left.0.cmp(&right.0));
    let mut bytes = Vec::new();
    for (name, content) in parts {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&content);
        bytes.push(0);
    }
    content_hash_hex(&bytes)
}

fn collect_script_sources(
    directory: &Path,
    prefix: &str,
    out: &mut Vec<(String, Vec<u8>)>,
    depth: usize,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let name = if prefix.is_empty() {
            file_name.to_string()
        } else {
            format!("{prefix}/{file_name}")
        };
        if path.is_dir() {
            collect_script_sources(&path, &name, out, depth - 1);
        } else if path.extension().is_some_and(|extension| extension == "cs") {
            if let Ok(content) = fs::read(&path) {
                out.push((format!("script:{name}"), content));
            }
        }
    }
}

/// Collects the published bridge assemblies the compile reads. Only the
/// `Release/net8.0` output folders carry them, and their identity (name, size
/// and modification time) is enough to invalidate a stale cache.
fn collect_bridge_assemblies(directory: &Path, out: &mut Vec<(String, Vec<u8>)>, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "net8.0" {
            collect_published_assemblies(&path, out);
            continue;
        }
        if file_name == "obj" || file_name == "ref" || file_name == "runtimes" {
            continue;
        }
        collect_bridge_assemblies(&path, out, depth - 1);
    }
}

fn collect_published_assemblies(directory: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|extension| extension != "dll") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let mut identity = metadata.len().to_le_bytes().to_vec();
        identity.extend_from_slice(&modified.to_le_bytes());
        out.push((format!("bridge:{name}"), identity));
    }
}

/// The compile command line for a project's scripts. Script debugging adds
/// the `--debug` flag, so the compiled assemblies carry the symbols a
/// debugger needs.
fn script_compile_command(
    scripting_assembly: &Path,
    project_root: &Path,
    staging_dir: &Path,
    script_debugging: bool,
) -> Command {
    let mut command = Command::new("dotnet");
    command
        .arg(scripting_assembly)
        .arg("compile")
        .arg(project_root)
        .arg("--player");
    if script_debugging {
        command.arg("--debug");
    }
    command.arg("--out").arg(staging_dir);
    command
}

/// Compiles a project's scripts through the bridge compile pipeline
/// (`Unitas.Scripting compile <projectRoot> --player [--debug]`). Returns
/// `Ok(false)` when the bridge tooling has not been built yet.
fn compile_project_scripts(
    project_root: &Path,
    output_dir: &Path,
    script_debugging: bool,
) -> Result<bool, String> {
    let Some(workspace_root) = licensing::workspace_root() else {
        return Ok(false);
    };
    let scripting_assembly = workspace_root
        .join("bridge")
        .join("Unitas.Scripting")
        .join("bin")
        .join("Release")
        .join("net8.0")
        .join("Unitas.Scripting.dll");
    if !scripting_assembly.is_file() {
        return Ok(false);
    }
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;
    // The compile is the slowest step of staging and the same project is
    // staged by several gates and tests. Cache the compiled assemblies by the
    // content of the project's script sources and of the bridge assemblies the
    // compile references, so a repeated staging reuses them instead of paying
    // for another compile.
    let cache_dir = workspace_root
        .join("target")
        .join("unitas-script-cache")
        .join(script_compile_cache_key(
            &workspace_root,
            project_root,
            script_debugging,
        ));
    if publish_cached_assemblies(&cache_dir, output_dir)? {
        return Ok(true);
    }
    let staging_dir = std::env::temp_dir().join(format!(
        "unitas-script-compile-{}-{}",
        std::process::id(),
        content_hash_hex(cache_dir.to_string_lossy().as_bytes())
    ));
    let _ = fs::remove_dir_all(&staging_dir);
    fs::create_dir_all(&staging_dir).map_err(|error| error.to_string())?;
    let output = script_compile_command(
        &scripting_assembly,
        project_root,
        &staging_dir,
        script_debugging,
    )
    .output()
    .map_err(|error| format!("failed to launch dotnet: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let _ = fs::create_dir_all(&cache_dir);
    for entry in fs::read_dir(&staging_dir)
        .map_err(|error| error.to_string())?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "dll") {
            if let Some(name) = path.file_name() {
                // Publish atomically: a concurrent staging must never read a
                // half-written assembly.
                let temporary = cache_dir.join(format!(
                    "{}.{}.tmp",
                    name.to_string_lossy(),
                    std::process::id()
                ));
                if fs::copy(&path, &temporary).is_ok() {
                    let _ = fs::rename(&temporary, cache_dir.join(name));
                }
                let _ = fs::remove_file(&temporary);
            }
        }
    }
    let _ = fs::write(cache_dir.join(".complete"), b"ok");
    let _ = fs::remove_dir_all(&staging_dir);
    publish_cached_assemblies(&cache_dir, output_dir)?;
    Ok(true)
}

/// The macOS application bundle a staged bundle is packaged as, when the
/// target is macOS and a unitas player binary has been built.
///
/// Developing-mode stage-only builds (`stage_bundle_for_host`) run in place via
/// `unitas-player <bundle_dir>` and skip duplicating the ~40 MB player binary
/// and running `codesign` inside a nested `.app` bundle unless explicitly
/// requested through `package_macos_app` or release mode.
fn macos_application(
    manifest: &BundleManifest,
    output_dir: &Path,
) -> Result<Option<PackagedApp>, BundleError> {
    if manifest.target.os != TargetOs::MacOs || manifest.mode == BuildMode::Development {
        return Ok(None);
    }
    let Some(player_binary) = package::locate_player_binary() else {
        return Ok(None);
    };
    let app_path = output_dir.join(format!(
        "{}.{}",
        package::bundle_name(&manifest.product_name),
        package::APP_BUNDLE_EXTENSION
    ));
    let licensing = licensing::licensing_record(&manifest.application_identifier);
    let license_text = licensing::workspace_root().map(|root| root.join("LICENSE"));
    let application = package::package_macos_app(
        manifest,
        output_dir,
        &app_path,
        &player_binary,
        &licensing,
        license_text.as_deref(),
    )
    .map_err(|error| BundleError::Io {
        path: app_path.clone(),
        message: error.to_string(),
    })?;
    Ok(Some(application))
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

pub const CONTENT_CATALOG_FILENAME: &str = "content.catalog";
pub const CONTENT_CATALOG_FORMAT_VERSION: u32 = 1;
pub const UNITAS_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn derive_player_compatibility_version(version: &str) -> String {
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return String::from("0.1");
    }
    let mut parts = trimmed.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}")
}

pub fn detect_unity_bundle_signature(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"UnityFS") {
        Some("UnityFS")
    } else if bytes.starts_with(b"UnityWeb") {
        Some("UnityWeb")
    } else if bytes.starts_with(b"UnityRaw") {
        Some("UnityRaw")
    } else if bytes.starts_with(b"UnityArchive") {
        Some("UnityArchive")
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentBuildMode {
    FullCatalog,
    ContentUpdate,
}

impl ContentBuildMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ContentBuildMode::FullCatalog => "full_catalog",
            ContentBuildMode::ContentUpdate => "content_update",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "full_catalog" => Some(ContentBuildMode::FullCatalog),
            "content_update" => Some(ContentBuildMode::ContentUpdate),
            _ => None,
        }
    }
}

impl fmt::Display for ContentBuildMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressablesProfile {
    pub name: String,
    pub variables: BTreeMap<String, String>,
}

impl AddressablesProfile {
    pub fn new(
        name: impl Into<String>,
        build_path: impl Into<String>,
        load_path: impl Into<String>,
    ) -> Self {
        let mut variables = BTreeMap::new();
        variables.insert(String::from("BuildPath"), build_path.into());
        variables.insert(String::from("LoadPath"), load_path.into());
        Self {
            name: name.into(),
            variables,
        }
    }

    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn resolve_template(&self, template: &str, target: BuildTarget) -> String {
        let mut current = template.to_string();
        for _ in 0..8 {
            let mut next = current.replace("[BuildTarget]", &target.identifier());
            next = next.replace("{BuildTarget}", &target.identifier());
            for (key, value) in &self.variables {
                let bracket_token = format!("[{key}]");
                let brace_token = format!("{{{key}}}");
                next = next.replace(&bracket_token, value);
                next = next.replace(&brace_token, value);
            }
            if next == current {
                break;
            }
            current = next;
        }
        current
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressablesGroupConfig {
    pub group_name: String,
    pub scenes: Vec<PathBuf>,
    pub build_path_template: String,
    pub load_path_template: String,
    pub dependencies: Vec<String>,
}

impl AddressablesGroupConfig {
    pub fn new(
        group_name: impl Into<String>,
        scenes: Vec<PathBuf>,
        build_path_template: impl Into<String>,
        load_path_template: impl Into<String>,
        dependencies: Vec<String>,
    ) -> Self {
        Self {
            group_name: group_name.into(),
            scenes,
            build_path_template: build_path_template.into(),
            load_path_template: load_path_template.into(),
            dependencies,
        }
    }
}

pub fn derive_compatible_player_versions(version: &str) -> Vec<String> {
    let compat = derive_player_compatibility_version(version);
    let trimmed = version.trim();
    let wildcard = format!("{compat}.x");
    let mut versions = vec![compat, wildcard];
    if !trimmed.is_empty() && !versions.iter().any(|item| item == trimmed) {
        versions.push(String::from(trimmed));
    }
    versions
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCatalogEntry {
    pub bundle_name: String,
    pub group_name: String,
    pub source_path: String,
    pub staged_path: String,
    pub build_path: String,
    pub load_path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub root_object_count: usize,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCatalog {
    pub format_version: u32,
    pub catalog_id: String,
    pub profile_name: Option<String>,
    pub unitas_version: String,
    pub player_compatibility_version: String,
    pub compatible_player_versions: Vec<String>,
    pub target: BuildTarget,
    pub bundles: Vec<ContentCatalogEntry>,
    pub total_content_bytes: u64,
    pub catalog_hash: String,
}

impl ContentCatalog {
    pub fn compute_catalog_hash(&self) -> String {
        content_hash_hex(self.canonical_body_without_hash().as_bytes())
    }

    pub fn is_compatible_with_player(
        &self,
        player_version: &str,
        player_target: BuildTarget,
    ) -> bool {
        if self.target != player_target {
            return false;
        }
        let trimmed = player_version.trim();
        let player_compat = derive_player_compatibility_version(trimmed);
        let wildcard = format!("{player_compat}.x");
        self.player_compatibility_version == player_compat
            || self.player_compatibility_version == trimmed
            || self
                .compatible_player_versions
                .iter()
                .any(|v| v == trimmed || v == &player_compat || v == &wildcard)
    }

    fn canonical_body_without_hash(&self) -> String {
        let mut lines = Vec::with_capacity(10 + self.bundles.len() * 10);
        lines.push(format!("format_version={}", self.format_version));
        lines.push(format!("catalog_id={}", self.catalog_id));
        lines.push(format!(
            "profile_name={}",
            self.profile_name.as_deref().unwrap_or("")
        ));
        lines.push(format!("unitas_version={}", self.unitas_version));
        lines.push(format!(
            "player_compatibility_version={}",
            self.player_compatibility_version
        ));
        lines.push(format!(
            "compatible_player_versions={}",
            self.compatible_player_versions.join(",")
        ));
        lines.push(format!("target_os={}", self.target.os_name()));
        lines.push(format!("target_arch={}", self.target.arch_id()));
        lines.push(format!("total_content_bytes={}", self.total_content_bytes));
        lines.push(format!("bundle_count={}", self.bundles.len()));

        let mut sorted = self.bundles.clone();
        sorted.sort_by(|left, right| {
            left.bundle_name
                .cmp(&right.bundle_name)
                .then_with(|| left.staged_path.cmp(&right.staged_path))
        });

        for (slot, entry) in sorted.iter().enumerate() {
            lines.push(format!("bundle.{slot}.bundle_name={}", entry.bundle_name));
            lines.push(format!("bundle.{slot}.group_name={}", entry.group_name));
            lines.push(format!("bundle.{slot}.source_path={}", entry.source_path));
            lines.push(format!("bundle.{slot}.staged_path={}", entry.staged_path));
            lines.push(format!("bundle.{slot}.build_path={}", entry.build_path));
            lines.push(format!("bundle.{slot}.load_path={}", entry.load_path));
            lines.push(format!("bundle.{slot}.content_hash={}", entry.content_hash));
            lines.push(format!("bundle.{slot}.size_bytes={}", entry.size_bytes));
            lines.push(format!(
                "bundle.{slot}.root_object_count={}",
                entry.root_object_count
            ));
            lines.push(format!(
                "bundle.{slot}.dependencies={}",
                entry.dependencies.join(",")
            ));
        }

        lines.join("\n")
    }

    pub fn to_canonical_string(&self) -> String {
        let mut text = self.canonical_body_without_hash();
        text.push_str(&format!("\ncatalog_hash={}\n", self.catalog_hash));
        text
    }

    pub fn from_canonical_str(text: &str) -> Result<Self, BundleError> {
        let mut map = BTreeMap::new();
        for (line_number, raw_line) in text.lines().enumerate() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (key, value) =
                trimmed
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
        if format_version != CONTENT_CATALOG_FORMAT_VERSION {
            return Err(BundleError::ManifestParse {
                message: format!("unsupported content catalog format_version '{format_version}'"),
            });
        }

        let unitas_version = required_field(&map, "unitas_version")?;
        let player_compatibility_version = required_field(&map, "player_compatibility_version")?;
        let compatible_player_versions = match map.get("compatible_player_versions") {
            Some(raw) if !raw.trim().is_empty() => raw
                .split(',')
                .map(|part| part.trim().to_string())
                .filter(|part| !part.is_empty())
                .collect(),
            _ => derive_compatible_player_versions(&player_compatibility_version),
        };
        let profile_name = map
            .get("profile_name")
            .map(|val| val.trim().to_string())
            .filter(|val| !val.is_empty());
        let target_os = required_field(&map, "target_os")?;
        let target_arch = required_field(&map, "target_arch")?;
        let target = BuildTarget::from_env(&target_os, &target_arch).ok_or_else(|| {
            BundleError::ManifestParse {
                message: format!("unrecognized target '{target_os}-{target_arch}'"),
            }
        })?;
        let total_content_bytes = parse_u64_field(&map, "total_content_bytes")?;
        let bundle_count = parse_usize_field(&map, "bundle_count")?;
        let catalog_hash = required_field(&map, "catalog_hash")?;
        let catalog_id = map
            .get("catalog_id")
            .cloned()
            .filter(|val| !val.trim().is_empty())
            .unwrap_or_else(|| {
                format!(
                    "catalog-{}-{}",
                    target.identifier(),
                    player_compatibility_version
                )
            });

        let mut bundles = Vec::with_capacity(bundle_count);
        for slot in 0..bundle_count {
            let bundle_name = required_field(&map, &format!("bundle.{slot}.bundle_name"))?;
            let group_name = map
                .get(&format!("bundle.{slot}.group_name"))
                .cloned()
                .filter(|val| !val.trim().is_empty())
                .unwrap_or_else(|| String::from("default"));
            let source_path = required_field(&map, &format!("bundle.{slot}.source_path"))?;
            let staged_path = required_field(&map, &format!("bundle.{slot}.staged_path"))?;
            let build_path = map
                .get(&format!("bundle.{slot}.build_path"))
                .cloned()
                .filter(|val| !val.trim().is_empty())
                .unwrap_or_else(|| staged_path.clone());
            let load_path = map
                .get(&format!("bundle.{slot}.load_path"))
                .cloned()
                .filter(|val| !val.trim().is_empty())
                .unwrap_or_else(|| staged_path.clone());
            let content_hash = required_field(&map, &format!("bundle.{slot}.content_hash"))?;
            let size_bytes = parse_u64_field(&map, &format!("bundle.{slot}.size_bytes"))?;
            let root_object_count =
                parse_usize_field(&map, &format!("bundle.{slot}.root_object_count"))?;
            let dependencies = map
                .get(&format!("bundle.{slot}.dependencies"))
                .map(|raw| {
                    raw.split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            bundles.push(ContentCatalogEntry {
                bundle_name,
                group_name,
                source_path,
                staged_path,
                build_path,
                load_path,
                content_hash,
                size_bytes,
                root_object_count,
                dependencies,
            });
        }

        bundles.sort_by(|left, right| {
            left.bundle_name
                .cmp(&right.bundle_name)
                .then_with(|| left.staged_path.cmp(&right.staged_path))
        });

        let catalog = Self {
            format_version,
            catalog_id,
            profile_name,
            unitas_version,
            player_compatibility_version,
            compatible_player_versions,
            target,
            bundles,
            total_content_bytes,
            catalog_hash,
        };

        let expected_hash = catalog.compute_catalog_hash();
        if catalog.catalog_hash != expected_hash {
            return Err(BundleError::ManifestParse {
                message: format!(
                    "catalog_hash mismatch: expected '{expected_hash}', found '{}'",
                    catalog.catalog_hash
                ),
            });
        }

        Ok(catalog)
    }

    pub fn load_from_file(catalog_path: &Path) -> Result<Self, BundleError> {
        let content = fs::read_to_string(catalog_path).map_err(|error| BundleError::Io {
            path: catalog_path.to_path_buf(),
            message: error.to_string(),
        })?;
        Self::from_canonical_str(&content)
    }

    pub fn load_from_dir(output_dir: &Path) -> Result<Self, BundleError> {
        Self::load_from_file(&output_dir.join(CONTENT_CATALOG_FILENAME))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentRejectionReason {
    UnityBuiltArchive {
        signature: String,
    },
    TargetMismatch {
        bundle_target: BuildTarget,
        expected_target: BuildTarget,
    },
    IncompatiblePlayerVersion {
        bundle_version: String,
        expected_version: String,
    },
    UnreadableScene {
        message: String,
    },
}

impl ContentRejectionReason {
    pub const fn code(&self) -> &'static str {
        match self {
            ContentRejectionReason::UnityBuiltArchive { .. } => "content.unity_built_bundle",
            ContentRejectionReason::TargetMismatch { .. } => "content.target_mismatch",
            ContentRejectionReason::IncompatiblePlayerVersion { .. } => {
                "content.incompatible_player_version"
            }
            ContentRejectionReason::UnreadableScene { .. } => "content.unreadable_bundle",
        }
    }
}

impl fmt::Display for ContentRejectionReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentRejectionReason::UnityBuiltArchive { signature } => {
                write!(
                    formatter,
                    "bundle was built by Unity (detected '{signature}' archive signature); rebuild content from project sources with unitas"
                )
            }
            ContentRejectionReason::TargetMismatch {
                bundle_target,
                expected_target,
            } => {
                write!(
                    formatter,
                    "bundle target '{bundle_target}' does not match player target '{expected_target}'"
                )
            }
            ContentRejectionReason::IncompatiblePlayerVersion {
                bundle_version,
                expected_version,
            } => {
                write!(
                    formatter,
                    "bundle player-compatibility version '{bundle_version}' is incompatible with player compatibility version '{expected_version}'"
                )
            }
            ContentRejectionReason::UnreadableScene { message } => {
                write!(formatter, "bundle content cannot be assembled: {message}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedContentBundle {
    pub bundle_name: String,
    pub source_path: PathBuf,
    pub reason: ContentRejectionReason,
}

impl fmt::Display for RejectedContentBundle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "rejected bundle '{}' ('{}'): {}",
            self.bundle_name,
            self.source_path.display(),
            self.reason
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentBundleInput {
    pub bundle_name: String,
    pub group_name: String,
    pub source_path: PathBuf,
    pub target: BuildTarget,
    pub player_compatibility_version: String,
    pub build_path: Option<String>,
    pub load_path: Option<String>,
    pub dependencies: Vec<String>,
}

impl ContentBundleInput {
    pub fn new(
        bundle_name: impl Into<String>,
        source_path: PathBuf,
        target: BuildTarget,
        player_compatibility_version: impl Into<String>,
    ) -> Self {
        Self {
            bundle_name: bundle_name.into(),
            group_name: String::from("default"),
            source_path,
            target,
            player_compatibility_version: player_compatibility_version.into(),
            build_path: None,
            load_path: None,
            dependencies: Vec::new(),
        }
    }

    pub fn with_group_profile_and_dependencies(
        mut self,
        group_name: impl Into<String>,
        build_path: impl Into<String>,
        load_path: impl Into<String>,
        dependencies: Vec<String>,
    ) -> Self {
        self.group_name = group_name.into();
        self.build_path = Some(build_path.into());
        self.load_path = Some(load_path.into());
        self.dependencies = dependencies;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentBuildResult {
    pub output_dir: PathBuf,
    pub catalog_path: PathBuf,
    pub catalog: ContentCatalog,
    pub emitted_bundles: Vec<ContentCatalogEntry>,
    pub unchanged_bundles: Vec<String>,
    pub rejected_bundles: Vec<RejectedContentBundle>,
}

pub fn build_content_catalog(
    config: &BuildConfiguration,
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    build_content_catalog_for_host(config, output_dir, BuildTarget::current_host())
}

pub fn build_content_catalog_for_host(
    config: &BuildConfiguration,
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<ContentBuildResult, BundleError> {
    build_content_with_optional_baseline(config, None, None, None, output_dir, host)
}

pub fn build_content_update(
    config: &BuildConfiguration,
    prior_catalog: &ContentCatalog,
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    build_content_update_for_host(
        config,
        prior_catalog,
        output_dir,
        BuildTarget::current_host(),
    )
}

pub fn build_content_update_for_host(
    config: &BuildConfiguration,
    prior_catalog: &ContentCatalog,
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<ContentBuildResult, BundleError> {
    build_content_with_optional_baseline(config, None, None, Some(prior_catalog), output_dir, host)
}

pub fn build_addressables_content(
    config: &BuildConfiguration,
    profile: &AddressablesProfile,
    groups: &[AddressablesGroupConfig],
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    build_addressables_content_for_host(
        config,
        profile,
        groups,
        output_dir,
        BuildTarget::current_host(),
    )
}

pub fn build_addressables_content_for_host(
    config: &BuildConfiguration,
    profile: &AddressablesProfile,
    groups: &[AddressablesGroupConfig],
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<ContentBuildResult, BundleError> {
    build_content_with_optional_baseline(
        config,
        Some(profile),
        Some(groups),
        None,
        output_dir,
        host,
    )
}

pub fn build_addressables_content_update(
    config: &BuildConfiguration,
    profile: &AddressablesProfile,
    groups: &[AddressablesGroupConfig],
    prior_catalog: &ContentCatalog,
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    build_addressables_content_update_for_host(
        config,
        profile,
        groups,
        prior_catalog,
        output_dir,
        BuildTarget::current_host(),
    )
}

pub fn build_addressables_content_update_for_host(
    config: &BuildConfiguration,
    profile: &AddressablesProfile,
    groups: &[AddressablesGroupConfig],
    prior_catalog: &ContentCatalog,
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<ContentBuildResult, BundleError> {
    build_content_with_optional_baseline(
        config,
        Some(profile),
        Some(groups),
        Some(prior_catalog),
        output_dir,
        host,
    )
}

fn build_content_with_optional_baseline(
    config: &BuildConfiguration,
    profile: Option<&AddressablesProfile>,
    groups: Option<&[AddressablesGroupConfig]>,
    prior_catalog: Option<&ContentCatalog>,
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<ContentBuildResult, BundleError> {
    config
        .validate_with_host(host)
        .map_err(BundleError::Validation)?;

    let player_compatibility_version = derive_player_compatibility_version(&config.version);
    if let Some(prior) = prior_catalog {
        if prior.target != config.target {
            return Err(BundleError::Validation(vec![BuildValidationError::new(
                "target",
                format!(
                    "content update target '{}' does not match prior catalog target '{}'",
                    config.target, prior.target
                ),
                "build the content update for the same target as the baseline content catalog",
            )]));
        }
        if prior.player_compatibility_version != player_compatibility_version {
            return Err(BundleError::Validation(vec![BuildValidationError::new(
                "version",
                format!(
                    "content update player-compatibility version '{}' does not match prior catalog compatibility version '{}'",
                    player_compatibility_version, prior.player_compatibility_version
                ),
                "keep the same major.minor player-compatibility version for post-release content updates or ship a new full player build",
            )]));
        }
    }

    let mut prior_hashes = BTreeMap::new();
    if let Some(prior) = prior_catalog {
        for entry in &prior.bundles {
            prior_hashes.insert(entry.bundle_name.as_str(), entry.content_hash.as_str());
        }
    }

    let verified_bundles = collect_verified_content_bundles(config, profile, groups)?;

    build_content_result(
        config,
        profile,
        player_compatibility_version,
        verified_bundles,
        &prior_hashes,
        output_dir,
    )
}

fn build_content_result(
    config: &BuildConfiguration,
    profile: Option<&AddressablesProfile>,
    player_compatibility_version: String,
    verified_bundles: Vec<(ContentCatalogEntry, Vec<u8>)>,
    prior_hashes: &BTreeMap<&str, &str>,
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    fs::create_dir_all(output_dir).map_err(|error| BundleError::Io {
        path: output_dir.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut bundles = Vec::with_capacity(verified_bundles.len());
    let mut emitted_bundles = Vec::new();
    let mut unchanged_bundles = Vec::new();
    let mut total_content_bytes = 0u64;

    for (entry, bytes) in verified_bundles {
        total_content_bytes = total_content_bytes.saturating_add(entry.size_bytes);
        let is_unchanged = prior_hashes
            .get(entry.bundle_name.as_str())
            .is_some_and(|prior_hash| *prior_hash == entry.content_hash);

        let dest_path = output_dir.join(&entry.staged_path);
        if is_unchanged {
            if dest_path.exists() {
                fs::remove_file(&dest_path).map_err(|error| BundleError::Io {
                    path: dest_path,
                    message: error.to_string(),
                })?;
            }
            unchanged_bundles.push(entry.bundle_name.clone());
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|error| BundleError::Io {
                    path: parent.to_path_buf(),
                    message: error.to_string(),
                })?;
            }
            fs::write(&dest_path, &bytes).map_err(|error| BundleError::Io {
                path: dest_path,
                message: error.to_string(),
            })?;
            emitted_bundles.push(entry.clone());
        }
        bundles.push(entry);
    }

    bundles.sort_by(|left, right| {
        left.bundle_name
            .cmp(&right.bundle_name)
            .then_with(|| left.staged_path.cmp(&right.staged_path))
    });
    emitted_bundles.sort_by(|left, right| {
        left.bundle_name
            .cmp(&right.bundle_name)
            .then_with(|| left.staged_path.cmp(&right.staged_path))
    });
    unchanged_bundles.sort();

    let compatible_player_versions = derive_compatible_player_versions(&config.version);
    let profile_name = profile.map(|p| p.name.clone());
    let catalog_id = match &profile_name {
        Some(prof) => format!(
            "catalog-{}-{}-{prof}",
            config.application_identifier,
            config.target.identifier()
        ),
        None => format!(
            "catalog-{}-{}",
            config.application_identifier,
            config.target.identifier()
        ),
    };

    let mut catalog = ContentCatalog {
        format_version: CONTENT_CATALOG_FORMAT_VERSION,
        catalog_id,
        profile_name,
        unitas_version: String::from(UNITAS_VERSION),
        player_compatibility_version,
        compatible_player_versions,
        target: config.target,
        bundles,
        total_content_bytes,
        catalog_hash: String::new(),
    };
    catalog.catalog_hash = catalog.compute_catalog_hash();

    let catalog_path = output_dir.join(CONTENT_CATALOG_FILENAME);
    fs::write(&catalog_path, catalog.to_canonical_string()).map_err(|error| BundleError::Io {
        path: catalog_path.clone(),
        message: error.to_string(),
    })?;

    Ok(ContentBuildResult {
        output_dir: output_dir.to_path_buf(),
        catalog_path,
        catalog,
        emitted_bundles,
        unchanged_bundles,
        rejected_bundles: Vec::new(),
    })
}

fn collect_verified_content_bundles(
    config: &BuildConfiguration,
    profile: Option<&AddressablesProfile>,
    groups: Option<&[AddressablesGroupConfig]>,
) -> Result<Vec<(ContentCatalogEntry, Vec<u8>)>, BundleError> {
    let mut verified_bundles = Vec::new();
    if let (Some(active_profile), Some(group_list)) = (profile, groups) {
        if group_list.is_empty() {
            return Err(BundleError::Validation(vec![BuildValidationError::new(
                "addressables_groups",
                "no Addressables groups were provided for content build",
                "configure at least one AddressablesGroupConfig with one or more scene/asset paths",
            )]));
        }
        for group in group_list {
            let resolved_build_dir = active_profile
                .resolve_template(&group.build_path_template, config.target)
                .trim_end_matches('/')
                .to_string();
            let resolved_load_dir = active_profile
                .resolve_template(&group.load_path_template, config.target)
                .trim_end_matches('/')
                .to_string();
            for (index, scene_path) in group.scenes.iter().enumerate() {
                if !scene_path.is_file() {
                    return Err(BundleError::SceneMissing {
                        path: scene_path.clone(),
                    });
                }
                let bytes = fs::read(scene_path).map_err(|error| BundleError::Io {
                    path: scene_path.clone(),
                    message: error.to_string(),
                })?;
                let scene_graph =
                    unitas_asset::assemble_scene_file(scene_path).map_err(|error| {
                        BundleError::SceneAssembly {
                            path: scene_path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                let content_hash = content_hash_hex(&bytes);
                let size_bytes = bytes.len() as u64;
                let root_object_count = scene_graph.roots().len();
                let file_name = scene_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("scene.unity");
                let stem = scene_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("scene");
                let bundle_name = format!("{}_{index:03}_{stem}", group.group_name);
                let build_path = format!("{resolved_build_dir}/{bundle_name}.bundle");
                let load_path = format!("{resolved_load_dir}/{bundle_name}.bundle");
                let staged_path = build_path.clone();
                let source_path = normalize_scene_source_path(scene_path, file_name);

                verified_bundles.push((
                    ContentCatalogEntry {
                        bundle_name,
                        group_name: group.group_name.clone(),
                        source_path,
                        staged_path,
                        build_path,
                        load_path,
                        content_hash,
                        size_bytes,
                        root_object_count,
                        dependencies: group.dependencies.clone(),
                    },
                    bytes,
                ));
            }
        }
    } else {
        for (index, scene_path) in config.scenes.iter().enumerate() {
            if !scene_path.is_file() {
                return Err(BundleError::SceneMissing {
                    path: scene_path.clone(),
                });
            }
            let bytes = fs::read(scene_path).map_err(|error| BundleError::Io {
                path: scene_path.clone(),
                message: error.to_string(),
            })?;
            let scene_graph = unitas_asset::assemble_scene_file(scene_path).map_err(|error| {
                BundleError::SceneAssembly {
                    path: scene_path.clone(),
                    message: error.to_string(),
                }
            })?;
            let content_hash = content_hash_hex(&bytes);
            let size_bytes = bytes.len() as u64;
            let root_object_count = scene_graph.roots().len();
            let file_name = scene_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("scene.unity");
            let stem = scene_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("scene");
            let bundle_name = format!("scene_{index:03}_{stem}");
            let staged_path = format!("ContentBundles/{bundle_name}.bundle");
            let source_path = normalize_scene_source_path(scene_path, file_name);

            verified_bundles.push((
                ContentCatalogEntry {
                    bundle_name,
                    group_name: String::from("default"),
                    source_path,
                    staged_path: staged_path.clone(),
                    build_path: staged_path.clone(),
                    load_path: staged_path,
                    content_hash,
                    size_bytes,
                    root_object_count,
                    dependencies: Vec::new(),
                },
                bytes,
            ));
        }
    }
    Ok(verified_bundles)
}

pub fn validate_content_bundle(
    inputs: &[ContentBundleInput],
    expected_target: BuildTarget,
    expected_player_compatibility_version: &str,
    output_dir: &Path,
) -> Result<ContentBuildResult, BundleError> {
    fs::create_dir_all(output_dir).map_err(|error| BundleError::Io {
        path: output_dir.to_path_buf(),
        message: error.to_string(),
    })?;

    let expected_compat = expected_player_compatibility_version.trim();
    let mut bundles = Vec::new();
    let mut emitted_bundles = Vec::new();
    let mut rejected_bundles = Vec::new();
    let mut total_content_bytes = 0u64;

    for input in inputs {
        if !input.source_path.is_file() {
            return Err(BundleError::SceneMissing {
                path: input.source_path.clone(),
            });
        }
        let bytes = fs::read(&input.source_path).map_err(|error| BundleError::Io {
            path: input.source_path.clone(),
            message: error.to_string(),
        })?;

        let staged_rel = input
            .build_path
            .clone()
            .unwrap_or_else(|| format!("ContentBundles/{}.bundle", input.bundle_name));
        let load_rel = input
            .load_path
            .clone()
            .unwrap_or_else(|| staged_rel.clone());
        let staged_dest = output_dir.join(&staged_rel);

        if let Some(signature) = detect_unity_bundle_signature(&bytes) {
            if staged_dest.exists() {
                fs::remove_file(&staged_dest).map_err(|error| BundleError::Io {
                    path: staged_dest,
                    message: error.to_string(),
                })?;
            }
            rejected_bundles.push(RejectedContentBundle {
                bundle_name: input.bundle_name.clone(),
                source_path: input.source_path.clone(),
                reason: ContentRejectionReason::UnityBuiltArchive {
                    signature: String::from(signature),
                },
            });
            continue;
        }

        if input.target != expected_target {
            if staged_dest.exists() {
                fs::remove_file(&staged_dest).map_err(|error| BundleError::Io {
                    path: staged_dest,
                    message: error.to_string(),
                })?;
            }
            rejected_bundles.push(RejectedContentBundle {
                bundle_name: input.bundle_name.clone(),
                source_path: input.source_path.clone(),
                reason: ContentRejectionReason::TargetMismatch {
                    bundle_target: input.target,
                    expected_target,
                },
            });
            continue;
        }

        let input_compat = input.player_compatibility_version.trim();
        if input_compat != expected_compat
            && derive_player_compatibility_version(input_compat) != expected_compat
        {
            if staged_dest.exists() {
                fs::remove_file(&staged_dest).map_err(|error| BundleError::Io {
                    path: staged_dest,
                    message: error.to_string(),
                })?;
            }
            rejected_bundles.push(RejectedContentBundle {
                bundle_name: input.bundle_name.clone(),
                source_path: input.source_path.clone(),
                reason: ContentRejectionReason::IncompatiblePlayerVersion {
                    bundle_version: input.player_compatibility_version.clone(),
                    expected_version: String::from(expected_compat),
                },
            });
            continue;
        }

        let scene_graph = match unitas_asset::assemble_scene_file(&input.source_path) {
            Ok(graph) => graph,
            Err(error) => {
                if staged_dest.exists() {
                    fs::remove_file(&staged_dest).map_err(|io_err| BundleError::Io {
                        path: staged_dest,
                        message: io_err.to_string(),
                    })?;
                }
                rejected_bundles.push(RejectedContentBundle {
                    bundle_name: input.bundle_name.clone(),
                    source_path: input.source_path.clone(),
                    reason: ContentRejectionReason::UnreadableScene {
                        message: error.to_string(),
                    },
                });
                continue;
            }
        };

        if let Some(parent) = staged_dest.parent() {
            fs::create_dir_all(parent).map_err(|error| BundleError::Io {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        fs::write(&staged_dest, &bytes).map_err(|error| BundleError::Io {
            path: staged_dest,
            message: error.to_string(),
        })?;

        let file_name = input
            .source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("bundle.unity");
        let source_path = normalize_scene_source_path(&input.source_path, file_name);
        let size_bytes = bytes.len() as u64;
        total_content_bytes = total_content_bytes.saturating_add(size_bytes);

        let entry = ContentCatalogEntry {
            bundle_name: input.bundle_name.clone(),
            group_name: input.group_name.clone(),
            source_path,
            staged_path: staged_rel.clone(),
            build_path: staged_rel,
            load_path: load_rel,
            content_hash: content_hash_hex(&bytes),
            size_bytes,
            root_object_count: scene_graph.roots().len(),
            dependencies: input.dependencies.clone(),
        };
        emitted_bundles.push(entry.clone());
        bundles.push(entry);
    }

    bundles.sort_by(|left, right| {
        left.bundle_name
            .cmp(&right.bundle_name)
            .then_with(|| left.staged_path.cmp(&right.staged_path))
    });
    emitted_bundles.sort_by(|left, right| {
        left.bundle_name
            .cmp(&right.bundle_name)
            .then_with(|| left.staged_path.cmp(&right.staged_path))
    });

    let mut catalog = ContentCatalog {
        format_version: CONTENT_CATALOG_FORMAT_VERSION,
        catalog_id: format!(
            "catalog-validated-{}-{}",
            expected_target.identifier(),
            expected_compat
        ),
        profile_name: None,
        unitas_version: String::from(UNITAS_VERSION),
        player_compatibility_version: String::from(expected_compat),
        compatible_player_versions: derive_compatible_player_versions(expected_compat),
        target: expected_target,
        bundles,
        total_content_bytes,
        catalog_hash: String::new(),
    };
    catalog.catalog_hash = catalog.compute_catalog_hash();

    let catalog_path = output_dir.join(CONTENT_CATALOG_FILENAME);
    fs::write(&catalog_path, catalog.to_canonical_string()).map_err(|error| BundleError::Io {
        path: catalog_path.clone(),
        message: error.to_string(),
    })?;

    Ok(ContentBuildResult {
        output_dir: output_dir.to_path_buf(),
        catalog_path,
        catalog,
        emitted_bundles,
        unchanged_bundles: Vec::new(),
        rejected_bundles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
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

    #[test]
    fn staged_desktop_launcher_is_the_player_binary() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("desktop_launcher")?;
        let scenes_dir = workspace.join("Project").join("Assets").join("Scenes");
        fs::create_dir_all(&scenes_dir)?;
        let main_scene = scenes_dir.join("Main.unity");
        fs::write(&main_scene, sample_unity_scene("MainRoot"))?;

        let player_binary =
            package::locate_player_binary().ok_or("the unitas player binary has not been built")?;
        let player_binary_bytes = fs::read(&player_binary)?;

        for (target, launcher_name) in [
            (BuildTarget::WINDOWS_X86_64, "PackagedGame.exe"),
            (BuildTarget::LINUX_X86_64, "PackagedGame"),
        ] {
            let config = BuildConfiguration::new(
                vec![main_scene.clone()],
                "Unitas",
                "PackagedGame",
                "com.unitas.packaged",
                "1.0.0",
                target,
            );
            let output_dir = workspace.join(format!("out_{}", target.identifier()));
            let staged = stage_bundle_for_host(&config, &output_dir, Some(target))?;

            assert_eq!(
                staged
                    .launch_artifact_path
                    .file_name()
                    .and_then(|name| name.to_str()),
                Some(launcher_name)
            );
            assert_eq!(
                fs::read(&staged.launch_artifact_path)?,
                player_binary_bytes,
                "the staged launcher of {target} is not the player binary"
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(&staged.launch_artifact_path)?
                    .permissions()
                    .mode();
                assert_eq!(
                    mode & 0o111,
                    0o111,
                    "the staged launcher of {target} is not executable"
                );
            }
        }

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    fn script_compile_cache_key_separates_debug_and_non_debug_compiles(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("compile_cache_key")?;
        let scripts_dir = workspace.join("Assets").join("Scripts");
        fs::create_dir_all(&scripts_dir)?;
        fs::write(scripts_dir.join("Player.cs"), "public class Player { }\n")?;

        let release_key = script_compile_cache_key(&workspace, &workspace, false);
        let debug_key = script_compile_cache_key(&workspace, &workspace, true);
        assert_ne!(release_key, debug_key);
        assert_eq!(
            release_key,
            script_compile_cache_key(&workspace, &workspace, false)
        );

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    fn script_compile_command_passes_the_debug_flag_only_for_debugging_compiles() {
        let release = script_compile_command(
            Path::new("/bridge/Unitas.Scripting.dll"),
            Path::new("/project"),
            Path::new("/staging"),
            false,
        );
        let debug = script_compile_command(
            Path::new("/bridge/Unitas.Scripting.dll"),
            Path::new("/project"),
            Path::new("/staging"),
            true,
        );
        let release_args: Vec<&OsStr> = release.get_args().collect();
        let debug_args: Vec<&OsStr> = debug.get_args().collect();
        assert!(release_args.contains(&OsStr::new("--player")));
        assert!(debug_args.contains(&OsStr::new("compile")));
        assert!(!release_args.contains(&OsStr::new("--debug")));
        assert!(debug_args.contains(&OsStr::new("--debug")));
    }

    #[test]
    fn staging_records_script_host_from_whether_project_carries_scripts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("script_host_manifest")?;
        let host = BuildTarget::MACOS_ARM64;

        let scriptless_scenes = workspace.join("Scriptless").join("Assets").join("Scenes");
        fs::create_dir_all(&scriptless_scenes)?;
        let scriptless_scene = scriptless_scenes.join("Main.unity");
        fs::write(&scriptless_scene, sample_unity_scene("MainRoot"))?;
        let scriptless_config = BuildConfiguration::new(
            vec![scriptless_scene],
            "Unitas",
            "ScriptlessGame",
            "com.unitas.scriptless",
            "1.0.0",
            host,
        );
        let scriptless_out = workspace.join("scriptless_out");
        let scriptless_staged =
            stage_bundle_for_host(&scriptless_config, &scriptless_out, Some(host))?;
        assert_eq!(
            scriptless_staged.manifest.format_version,
            BUNDLE_MANIFEST_FORMAT_VERSION
        );
        assert_eq!(scriptless_staged.manifest.script_host, ScriptHost::None);
        assert!(fs::read_to_string(&scriptless_staged.manifest_path)?.contains("script_host=none"));

        let scripted_root = workspace.join("Scripted");
        let scripted_scenes = scripted_root.join("Assets").join("Scenes");
        let scripted_scripts = scripted_root.join("Assets").join("Scripts");
        fs::create_dir_all(&scripted_scenes)?;
        fs::create_dir_all(&scripted_scripts)?;
        let scripted_scene = scripted_scenes.join("Main.unity");
        fs::write(&scripted_scene, sample_unity_scene("MainRoot"))?;
        fs::write(
            scripted_scripts.join("Controller.cs"),
            "public class Controller { }\n",
        )?;
        let scripted_config = BuildConfiguration::new(
            vec![scripted_scene],
            "Unitas",
            "ScriptedGame",
            "com.unitas.scripted",
            "1.0.0",
            host,
        );
        let scripted_out = workspace.join("scripted_out");
        let scripted_staged = stage_bundle_for_host(&scripted_config, &scripted_out, Some(host))?;
        assert_eq!(
            scripted_staged.manifest.format_version,
            BUNDLE_MANIFEST_FORMAT_VERSION
        );
        assert_eq!(scripted_staged.manifest.script_host, ScriptHost::CoreClr);
        assert!(fs::read_to_string(&scripted_staged.manifest_path)?.contains("script_host=coreclr"));

        let round_trip = BundleManifest::load_from_dir(&scripted_out)?;
        assert_eq!(round_trip.script_host, ScriptHost::CoreClr);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }
}
