use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use unitas_asset::meta::{self, GuidIndex, MetaError};

const META_SUFFIX: &str = ".meta";
const CS_SUFFIX: &str = ".cs";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptingError {
    Meta(MetaError),
    InvalidGuid(String),
    Io { path: PathBuf, message: String },
    HandleNotFound(u64),
    Poisoned,
}

impl fmt::Display for ScriptingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptingError::Meta(error) => write!(formatter, "{error}"),
            ScriptingError::InvalidGuid(guid) => write!(formatter, "invalid 128-bit guid {guid}"),
            ScriptingError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            ScriptingError::HandleNotFound(handle) => {
                write!(formatter, "scripting index handle {handle} not found")
            }
            ScriptingError::Poisoned => write!(formatter, "scripting registry mutex poisoned"),
        }
    }
}

impl std::error::Error for ScriptingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScriptingError::Meta(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MetaError> for ScriptingError {
    fn from(error: MetaError) -> Self {
        ScriptingError::Meta(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptGuidRecord {
    pub guid: String,
    pub guid_hi: u64,
    pub guid_lo: u64,
    pub path: PathBuf,
    pub relative_path: String,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPlatformConfig {
    pub platform: String,
    pub enabled: bool,
    pub cpu: Option<String>,
    pub os: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadataRecord {
    pub guid: String,
    pub guid_hi: u64,
    pub guid_lo: u64,
    pub path: PathBuf,
    pub relative_path: String,
    pub library_name: String,
    pub is_native: bool,
    pub any_platform_enabled: bool,
    pub editor_enabled: bool,
    pub editor_cpu: Option<String>,
    pub editor_os: Option<String>,
    pub define_constraints: Vec<String>,
    pub auto_referenced: bool,
    pub validate_references: bool,
    pub platforms: Vec<PluginPlatformConfig>,
}

impl PluginMetadataRecord {
    pub fn check_compatibility(
        &self,
        target_platform: &str,
        target_arch: &str,
        is_editor: bool,
    ) -> Result<(), String> {
        if is_editor {
            if !self.editor_enabled && !self.any_platform_enabled {
                return Err(format!(
                    "plugin '{}' is excluded for target 'Editor' (architecture '{}')",
                    self.library_name, target_arch
                ));
            }
            if let Some(cpu) = &self.editor_cpu
                && !arch_matches(cpu, target_arch)
            {
                return Err(format!(
                    "plugin '{}' targets architecture '{}' on target 'Editor', which is incompatible with running architecture '{}'",
                    self.library_name, cpu, target_arch
                ));
            }
        }

        let matched_platform = self
            .platforms
            .iter()
            .find(|entry| platform_matches(&entry.platform, target_platform));

        if let Some(config) = matched_platform {
            if !config.enabled && !self.any_platform_enabled {
                return Err(format!(
                    "plugin '{}' is excluded on target '{}' (architecture '{}')",
                    self.library_name, config.platform, target_arch
                ));
            }
            if let Some(cpu) = &config.cpu
                && !arch_matches(cpu, target_arch)
            {
                return Err(format!(
                    "plugin '{}' targets platform '{}' with architecture '{}', which is incompatible with target architecture '{}'",
                    self.library_name, config.platform, cpu, target_arch
                ));
            }
            return Ok(());
        }

        for config in &self.platforms {
            if config.enabled
                && let Some(cpu) = &config.cpu
                && !arch_matches(cpu, target_arch)
            {
                return Err(format!(
                    "plugin '{}' targets platform '{}' with architecture '{}', which is incompatible with target '{}' and architecture '{}'",
                    self.library_name, config.platform, cpu, target_platform, target_arch
                ));
            }
        }

        if !is_editor && !self.any_platform_enabled && !self.platforms.is_empty() {
            return Err(format!(
                "plugin '{}' is not enabled for target '{}' (architecture '{}')",
                self.library_name, target_platform, target_arch
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectScriptIndex {
    root: PathBuf,
    all_guids: GuidIndex,
    scripts_by_guid: BTreeMap<String, ScriptGuidRecord>,
    scripts_by_rel_path: BTreeMap<String, String>,
    scripts_by_name: BTreeMap<String, Vec<String>>,
    plugins_by_guid: BTreeMap<String, PluginMetadataRecord>,
    plugins_by_rel_path: BTreeMap<String, String>,
}

impl ProjectScriptIndex {
    pub fn build(project_root: &Path) -> Result<Self, ScriptingError> {
        let mut index = ProjectScriptIndex {
            root: project_root.to_path_buf(),
            ..ProjectScriptIndex::default()
        };

        let mut scan_roots = Vec::new();
        let assets_dir = project_root.join("Assets");
        let packages_dir = project_root.join("Packages");
        if assets_dir.is_dir() {
            scan_roots.push(assets_dir);
        }
        if packages_dir.is_dir() {
            scan_roots.push(packages_dir);
        }
        if scan_roots.is_empty() && project_root.is_dir() {
            scan_roots.push(project_root.to_path_buf());
        }

        let mut meta_files = Vec::new();
        for scan_root in &scan_roots {
            collect_meta_files(scan_root, &mut meta_files)?;
        }
        meta_files.sort();

        let mut guid_index = GuidIndex::default();
        for meta_path in meta_files {
            let text = match fs::read_to_string(&meta_path) {
                Ok(content) => content,
                Err(error) => {
                    return Err(ScriptingError::Io {
                        path: meta_path,
                        message: error.to_string(),
                    });
                }
            };
            let guid = meta::parse_meta_text(&text, &meta_path)?;
            let asset_path = asset_path_for_meta(&meta_path);
            guid_index.insert(guid.clone(), asset_path.clone())?;
            let (guid_hi, guid_lo) = parse_guid_u128(&guid)?;
            let relative_path = normalize_relative_path(project_root, &asset_path);

            if is_csharp_script(&asset_path) {
                let script_name = asset_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_string();
                let record = ScriptGuidRecord {
                    guid: guid.clone(),
                    guid_hi,
                    guid_lo,
                    path: asset_path,
                    relative_path: relative_path.clone(),
                    script_name: script_name.clone(),
                };
                index
                    .scripts_by_rel_path
                    .insert(relative_path, guid.clone());
                index
                    .scripts_by_name
                    .entry(script_name)
                    .or_default()
                    .push(guid.clone());
                index.scripts_by_guid.insert(guid, record);
            } else if is_plugin_file(&asset_path) {
                let plugin = parse_plugin_meta_text(
                    &text,
                    &guid,
                    guid_hi,
                    guid_lo,
                    &asset_path,
                    &relative_path,
                );
                index
                    .plugins_by_rel_path
                    .insert(relative_path, guid.clone());
                index.plugins_by_guid.insert(guid, plugin);
            }
        }

        index.all_guids = guid_index;
        Ok(index)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn scripts(&self) -> impl Iterator<Item = &ScriptGuidRecord> {
        self.scripts_by_guid.values()
    }

    pub fn script_count(&self) -> usize {
        self.scripts_by_guid.len()
    }

    pub fn script_by_guid(&self, guid: &str) -> Option<&ScriptGuidRecord> {
        self.scripts_by_guid.get(guid)
    }

    pub fn script_by_guid_pair(&self, guid_hi: u64, guid_lo: u64) -> Option<&ScriptGuidRecord> {
        let formatted = format_guid_u128(guid_hi, guid_lo);
        self.scripts_by_guid.get(&formatted)
    }

    pub fn script_by_relative_path(&self, relative_path: &str) -> Option<&ScriptGuidRecord> {
        let normalized = relative_path.replace('\\', "/");
        let guid = self.scripts_by_rel_path.get(&normalized)?;
        self.scripts_by_guid.get(guid)
    }

    pub fn scripts_by_class_name(&self, class_name: &str) -> Vec<&ScriptGuidRecord> {
        match self.scripts_by_name.get(class_name) {
            Some(guids) => guids
                .iter()
                .filter_map(|guid| self.scripts_by_guid.get(guid))
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn plugins(&self) -> impl Iterator<Item = &PluginMetadataRecord> {
        self.plugins_by_guid.values()
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins_by_guid.len()
    }

    pub fn plugin_by_guid(&self, guid: &str) -> Option<&PluginMetadataRecord> {
        self.plugins_by_guid.get(guid)
    }

    pub fn plugin_by_relative_path(&self, relative_path: &str) -> Option<&PluginMetadataRecord> {
        let normalized = relative_path.replace('\\', "/");
        let guid = self.plugins_by_rel_path.get(&normalized)?;
        self.plugins_by_guid.get(guid)
    }

    pub fn guid_index(&self) -> &GuidIndex {
        &self.all_guids
    }

    pub fn to_manifest_tsv(&self) -> String {
        let mut lines = Vec::new();
        for (guid, path) in self.all_guids.iter() {
            let rel = normalize_relative_path(&self.root, path);
            lines.push(format!("GUID\t{guid}\t{rel}"));
        }
        for record in self.scripts_by_guid.values() {
            lines.push(format!(
                "SCRIPT\t{}\t{:016x}\t{:016x}\t{}\t{}",
                record.guid,
                record.guid_hi,
                record.guid_lo,
                record.script_name,
                record.relative_path
            ));
        }
        for plugin in self.plugins_by_guid.values() {
            let mut platform_specs = Vec::new();
            for p in &plugin.platforms {
                let cpu = p.cpu.as_deref().unwrap_or("AnyCPU");
                let os = p.os.as_deref().unwrap_or("AnyOS");
                let enabled_flag = if p.enabled { "1" } else { "0" };
                platform_specs.push(format!("{}:{}:{}:{}", p.platform, enabled_flag, cpu, os));
            }
            let editor_cpu = plugin.editor_cpu.as_deref().unwrap_or("AnyCPU");
            let editor_os = plugin.editor_os.as_deref().unwrap_or("AnyOS");
            lines.push(format!(
                "PLUGIN\t{}\t{:016x}\t{:016x}\t{}\t{}\t{}\t{}\t{}\t{editor_cpu}:{editor_os}\t{}\t{}\t{}",
                plugin.guid,
                plugin.guid_hi,
                plugin.guid_lo,
                plugin.library_name,
                plugin.relative_path,
                if plugin.is_native { "native" } else { "managed" },
                if plugin.any_platform_enabled { "1" } else { "0" },
                if plugin.editor_enabled { "1" } else { "0" },
                if plugin.auto_referenced { "1" } else { "0" },
                platform_specs.join(","),
                plugin.define_constraints.join(",")
            ));
        }
        lines.join("\n")
    }
}

pub fn parse_guid_u128(guid: &str) -> Result<(u64, u64), ScriptingError> {
    let trimmed = guid.trim();
    if trimmed.len() != 32 || !meta::is_valid_guid(trimmed) {
        return Err(ScriptingError::InvalidGuid(trimmed.to_string()));
    }
    let hi = match u64::from_str_radix(&trimmed[..16], 16) {
        Ok(value) => value,
        Err(_) => return Err(ScriptingError::InvalidGuid(trimmed.to_string())),
    };
    let lo = match u64::from_str_radix(&trimmed[16..], 16) {
        Ok(value) => value,
        Err(_) => return Err(ScriptingError::InvalidGuid(trimmed.to_string())),
    };
    Ok((hi, lo))
}

pub fn format_guid_u128(guid_hi: u64, guid_lo: u64) -> String {
    format!("{guid_hi:016x}{guid_lo:016x}")
}

pub fn parse_plugin_meta_file(
    meta_path: &Path,
    project_root: &Path,
) -> Result<PluginMetadataRecord, ScriptingError> {
    let text = match fs::read_to_string(meta_path) {
        Ok(content) => content,
        Err(error) => {
            return Err(ScriptingError::Io {
                path: meta_path.to_path_buf(),
                message: error.to_string(),
            });
        }
    };
    let guid = meta::parse_meta_text(&text, meta_path)?;
    let (guid_hi, guid_lo) = parse_guid_u128(&guid)?;
    let asset_path = asset_path_for_meta(meta_path);
    let relative_path = normalize_relative_path(project_root, &asset_path);
    Ok(parse_plugin_meta_text(
        &text,
        &guid,
        guid_hi,
        guid_lo,
        &asset_path,
        &relative_path,
    ))
}

pub fn parse_plugin_meta_text(
    text: &str,
    guid: &str,
    guid_hi: u64,
    guid_lo: u64,
    asset_path: &Path,
    relative_path: &str,
) -> PluginMetadataRecord {
    let library_name = asset_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    let extension = asset_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let is_native_ext = matches!(
        extension.as_str(),
        "dylib" | "so" | "bundle" | "a" | "framework" | "aar" | "jslib" | "jspre"
    );

    let mut is_native = is_native_ext;
    let mut auto_referenced = true;
    let mut validate_references = true;
    let mut define_constraints: Vec<String> = Vec::new();
    let mut acc = PluginMetaAccumulator {
        editor_enabled: true,
        ..PluginMetaAccumulator::default()
    };

    let mut in_settings = false;
    let mut in_define_constraints = false;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let indent = raw_line.len() - raw_line.trim_start().len();

        if in_define_constraints {
            if let Some(item) = trimmed.strip_prefix("- ") {
                let constraint = unquote(item.trim());
                if !constraint.is_empty() {
                    define_constraints.push(constraint);
                }
                continue;
            } else if indent <= 4 {
                in_define_constraints = false;
            }
        }

        if trimmed == "defineConstraints:" {
            in_define_constraints = true;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("defineConstraints:") {
            let rest_trimmed = rest.trim();
            if rest_trimmed.starts_with('[') && rest_trimmed.ends_with(']') {
                let inner = &rest_trimmed[1..rest_trimmed.len() - 1];
                for part in inner.split(',') {
                    let clean = unquote(part.trim());
                    if !clean.is_empty() {
                        define_constraints.push(clean);
                    }
                }
            }
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("isPreloaded:") {
            if val.trim() == "1" || val.trim().eq_ignore_ascii_case("true") {
                is_native = true;
            }
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("autoReferenced:") {
            auto_referenced = parse_bool_flag(val.trim(), true);
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("validateReferences:") {
            validate_references = parse_bool_flag(val.trim(), true);
            continue;
        }

        if let Some(first_rest) = trimmed.strip_prefix("- first:") {
            acc.flush();
            in_settings = false;
            let inline = first_rest.trim();
            if let Some(platform_name) = extract_inline_mapping_target(inline) {
                acc.current_platform = Some(platform_name);
            }
            continue;
        }

        if trimmed == "first:" {
            acc.flush();
            in_settings = false;
            continue;
        }

        if acc.current_platform.is_none()
            && indent >= 4
            && !in_settings
            && let Some((key, val)) = split_yaml_kv(trimmed)
        {
            if key == "Any" {
                acc.current_platform = Some(String::from("Any"));
                continue;
            } else if key != "second"
                && key != "enabled"
                && key != "settings"
                && key != "platformData"
                && key != "PluginImporter"
                && key != "externalObjects"
                && key != "serializedVersion"
                && key != "iconMap"
                && key != "executionOrder"
            {
                let target = if val.is_empty() { key } else { val };
                acc.current_platform = Some(target);
                continue;
            }
        }

        if let Some(platform_data_key) = trimmed.strip_prefix("- ")
            && let Some((key, val)) = split_yaml_kv(platform_data_key)
            && (key == "Any" || key.starts_with("Editor") || key.starts_with("Standalone"))
        {
            acc.flush();
            let target = if val.is_empty() { key } else { val };
            acc.current_platform = Some(target);
            continue;
        }

        if trimmed == "settings:" {
            in_settings = true;
            continue;
        }
        if trimmed.starts_with("settings:") {
            in_settings = false;
            continue;
        }

        if acc.current_platform.is_some() {
            if let Some(enabled_str) = trimmed.strip_prefix("enabled:") {
                acc.current_enabled = Some(parse_bool_flag(enabled_str.trim(), false));
                continue;
            }
            if in_settings && let Some((key, val)) = split_yaml_kv(trimmed) {
                if key.eq_ignore_ascii_case("CPU") && !val.is_empty() {
                    if !val.eq_ignore_ascii_case("None") {
                        acc.current_cpu = Some(val);
                    }
                } else if key.eq_ignore_ascii_case("OS")
                    && !val.is_empty()
                    && !val.eq_ignore_ascii_case("None")
                {
                    acc.current_os = Some(val);
                }
            }
        }
    }

    acc.flush();

    if !acc.saw_any_platform && acc.platforms.is_empty() && !acc.saw_editor_enabled {
        acc.any_platform_enabled = true;
        acc.editor_enabled = true;
    } else if !acc.saw_editor_enabled {
        acc.editor_enabled = acc.any_platform_enabled;
    }

    for p in &acc.platforms {
        if let Some(cpu) = &p.cpu
            && !cpu.eq_ignore_ascii_case("AnyCPU")
        {
            is_native = true;
        }
    }
    if let Some(cpu) = &acc.editor_cpu
        && !cpu.eq_ignore_ascii_case("AnyCPU")
    {
        is_native = true;
    }

    PluginMetadataRecord {
        guid: guid.to_string(),
        guid_hi,
        guid_lo,
        path: asset_path.to_path_buf(),
        relative_path: relative_path.to_string(),
        library_name,
        is_native,
        any_platform_enabled: acc.any_platform_enabled,
        editor_enabled: acc.editor_enabled,
        editor_cpu: acc.editor_cpu,
        editor_os: acc.editor_os,
        define_constraints,
        auto_referenced,
        validate_references,
        platforms: acc.platforms,
    }
}

static SCRIPT_INDEX_REGISTRY: OnceLock<Mutex<BTreeMap<u64, ProjectScriptIndex>>> = OnceLock::new();
static NEXT_SCRIPT_INDEX_HANDLE: AtomicU64 = AtomicU64::new(1);

fn script_index_registry() -> &'static Mutex<BTreeMap<u64, ProjectScriptIndex>> {
    SCRIPT_INDEX_REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub fn register_project_script_index(project_root: &Path) -> Result<u64, ScriptingError> {
    let index = ProjectScriptIndex::build(project_root)?;
    let mut guard = match script_index_registry().lock() {
        Ok(guard) => guard,
        Err(_) => return Err(ScriptingError::Poisoned),
    };
    let handle = NEXT_SCRIPT_INDEX_HANDLE.fetch_add(1, Ordering::Relaxed);
    guard.insert(handle, index);
    Ok(handle)
}

pub fn destroy_project_script_index(handle: u64) -> Result<(), ScriptingError> {
    let mut guard = match script_index_registry().lock() {
        Ok(guard) => guard,
        Err(_) => return Err(ScriptingError::Poisoned),
    };
    if guard.remove(&handle).is_none() {
        return Err(ScriptingError::HandleNotFound(handle));
    }
    Ok(())
}

pub fn with_project_script_index<F, R>(handle: u64, callback: F) -> Result<R, ScriptingError>
where
    F: FnOnce(&ProjectScriptIndex) -> R,
{
    let guard = match script_index_registry().lock() {
        Ok(guard) => guard,
        Err(_) => return Err(ScriptingError::Poisoned),
    };
    let index = match guard.get(&handle) {
        Some(index) => index,
        None => return Err(ScriptingError::HandleNotFound(handle)),
    };
    Ok(callback(index))
}

#[derive(Default)]
struct PluginMetaAccumulator {
    current_platform: Option<String>,
    current_enabled: Option<bool>,
    current_cpu: Option<String>,
    current_os: Option<String>,
    any_platform_enabled: bool,
    saw_any_platform: bool,
    editor_enabled: bool,
    saw_editor_enabled: bool,
    editor_cpu: Option<String>,
    editor_os: Option<String>,
    platforms: Vec<PluginPlatformConfig>,
}

impl PluginMetaAccumulator {
    fn flush(&mut self) {
        let platform = match self.current_platform.take() {
            Some(name) => name,
            None => return,
        };
        let enabled = self.current_enabled.take().unwrap_or(false);
        let cpu = self.current_cpu.take();
        let os = self.current_os.take();

        if platform.eq_ignore_ascii_case("Any") {
            self.any_platform_enabled = enabled;
            self.saw_any_platform = true;
        } else if platform.eq_ignore_ascii_case("Editor") {
            self.editor_enabled = enabled;
            self.saw_editor_enabled = true;
            if cpu.is_some() {
                self.editor_cpu = cpu.clone();
            }
            if os.is_some() {
                self.editor_os = os.clone();
            }
            self.platforms.push(PluginPlatformConfig {
                platform,
                enabled,
                cpu,
                os,
            });
        } else {
            self.platforms.push(PluginPlatformConfig {
                platform,
                enabled,
                cpu,
                os,
            });
        }
    }
}

fn extract_inline_mapping_target(inline: &str) -> Option<String> {
    if !inline.starts_with('{') || !inline.ends_with('}') {
        return None;
    }
    let inner = inline[1..inline.len() - 1].trim();
    let (key, val) = split_yaml_kv(inner)?;
    if key == "Any" {
        Some(String::from("Any"))
    } else if !val.is_empty() {
        Some(val)
    } else {
        Some(key)
    }
}

fn split_yaml_kv(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = unquote(line[..colon].trim());
    let val = unquote(line[colon + 1..].trim());
    Some((key, val))
}

fn unquote(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_bool_flag(text: &str, default: bool) -> bool {
    match text {
        "1" | "true" | "True" | "TRUE" => true,
        "0" | "false" | "False" | "FALSE" => false,
        _ => default,
    }
}

fn arch_matches(configured: &str, target: &str) -> bool {
    let conf = normalize_arch(configured);
    let targ = normalize_arch(target);
    if conf == "anycpu" || conf == "none" || conf.is_empty() {
        return true;
    }
    conf == targ
}

fn normalize_arch(arch: &str) -> String {
    let lower = arch.trim().to_ascii_lowercase();
    match lower.as_str() {
        "x64" | "x86_64" | "amd64" => String::from("x86_64"),
        "arm64" | "aarch64" => String::from("arm64"),
        "x86" | "i386" | "i686" => String::from("x86"),
        "anycpu" | "any" => String::from("anycpu"),
        _ => lower,
    }
}

fn platform_matches(configured: &str, target: &str) -> bool {
    if configured.eq_ignore_ascii_case(target) {
        return true;
    }
    let conf = configured.to_ascii_lowercase();
    let targ = target.to_ascii_lowercase();
    if (conf == "standaloneosx" || conf == "standaloneosxuniversal" || conf == "osx")
        && (targ == "standaloneosx" || targ == "osx" || targ == "macos")
    {
        return true;
    }
    if (conf == "standalonewindows64" || conf == "standalonewindows" || conf == "windows")
        && (targ == "standalonewindows64" || targ == "standalonewindows" || targ == "windows")
    {
        return true;
    }
    if (conf == "standalonelinux64" || conf == "linux")
        && (targ == "standalonelinux64" || targ == "linux")
    {
        return true;
    }
    false
}

fn collect_meta_files(
    directory: &Path,
    collected: &mut Vec<PathBuf>,
) -> Result<(), ScriptingError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(ScriptingError::Io {
                path: directory.to_path_buf(),
                message: error.to_string(),
            });
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return Err(ScriptingError::Io {
                    path: directory.to_path_buf(),
                    message: error.to_string(),
                });
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

fn is_csharp_script(path: &Path) -> bool {
    match path.file_name() {
        Some(name) => name.to_string_lossy().ends_with(CS_SUFFIX),
        None => false,
    }
}

fn is_plugin_file(path: &Path) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.to_ascii_lowercase(),
        None => return false,
    };
    matches!(
        ext.as_str(),
        "dll" | "dylib" | "so" | "bundle" | "a" | "framework" | "aar" | "jar" | "jslib" | "jspre"
    )
}

fn asset_path_for_meta(meta_path: &Path) -> PathBuf {
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

fn normalize_relative_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("unitas_scripting_{prefix}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn builds_script_guid_map_and_plugin_metadata() {
        let root = unique_temp_dir("guid_map");
        let scripts_dir = root.join("Assets").join("Scripts");
        let plugins_dir = root.join("Assets").join("Plugins").join("x86_64");
        let _ = fs::create_dir_all(&scripts_dir);
        let _ = fs::create_dir_all(&plugins_dir);

        let script_path = scripts_dir.join("Mover.cs");
        let _ = fs::write(&script_path, "public class Mover {}");
        let script_meta = scripts_dir.join("Mover.cs.meta");
        let _ = fs::write(
            &script_meta,
            "fileFormatVersion: 2\nguid: 00000000000000000000000000000001\nMonoImporter:\n  serializedVersion: 2\n",
        );

        let plugin_path = plugins_dir.join("libfastmath.dylib");
        let _ = fs::write(&plugin_path, "");
        let plugin_meta = plugins_dir.join("libfastmath.dylib.meta");
        let _ = fs::write(
            &plugin_meta,
            "fileFormatVersion: 2\nguid: 11112222333344445555666677778888\nPluginImporter:\n  serializedVersion: 2\n  platformData:\n  - first:\n      Any:\n    second:\n      enabled: 0\n  - first:\n      Editor: Editor\n    second:\n      enabled: 1\n      settings:\n        CPU: x86_64\n        OS: OSX\n  - first:\n      Standalone: StandaloneOSX\n    second:\n      enabled: 1\n      settings:\n        CPU: x86_64\n",
        );

        let built = ProjectScriptIndex::build(&root);
        assert!(built.is_ok());
        if let Ok(index) = built {
            assert_eq!(index.script_count(), 1);
            let script = index.script_by_guid("00000000000000000000000000000001");
            assert!(script.is_some());
            if let Some(record) = script {
                assert_eq!(record.guid_hi, 0);
                assert_eq!(record.guid_lo, 1);
                assert_eq!(record.script_name, "Mover");
                assert_eq!(record.relative_path, "Assets/Scripts/Mover.cs");
            }

            assert_eq!(index.plugin_count(), 1);
            let plugin = index.plugin_by_guid("11112222333344445555666677778888");
            assert!(plugin.is_some());
            if let Some(p) = plugin {
                assert!(p.is_native);
                assert_eq!(p.library_name, "libfastmath");
                assert!(
                    p.check_compatibility("StandaloneOSX", "x86_64", true)
                        .is_ok()
                );
                let arm_check = p.check_compatibility("StandaloneOSX", "ARM64", true);
                assert!(arm_check.is_err());
                if let Err(diag) = arm_check {
                    assert!(diag.contains("libfastmath"));
                    assert!(diag.contains("x86_64"));
                    assert!(diag.contains("ARM64"));
                }
            }
        }

        let _ = fs::remove_dir_all(&root);
    }
}
