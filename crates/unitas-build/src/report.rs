use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use unitas_core::compat::{
    CompatibilityReport, CompatibilityState, Finding, Tier, RECOGNIZED_UNSUPPORTED_FAMILIES,
};

use crate::bundle::{
    ContentBuildMode, ContentBuildResult, ContentRejectionReason, RejectedContentBundle,
};
use crate::config::{BuildConfiguration, BuildMode, BuildValidationError};
use crate::licensing::{licensing_record, LicensingRecord, ThirdPartyNotice};
use crate::targets::{BuildTarget, TargetArch, TargetOs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

impl BuildOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            BuildOutcome::Succeeded => "succeeded",
            BuildOutcome::Failed => "failed",
            BuildOutcome::Cancelled => "cancelled",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "succeeded" => Some(BuildOutcome::Succeeded),
            "failed" => Some(BuildOutcome::Failed),
            "cancelled" => Some(BuildOutcome::Cancelled),
            _ => None,
        }
    }
}

impl fmt::Display for BuildOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDiagnostic {
    pub code: String,
    pub message: String,
    pub source: Option<String>,
    pub remedy: Option<String>,
}

impl BuildDiagnostic {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        source: Option<String>,
        remedy: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            source,
            remedy,
        }
    }

    pub fn from_validation_error(error: &BuildValidationError) -> Self {
        Self {
            code: format!("setting.{}", error.setting),
            message: error.consequence.clone(),
            source: Some(error.setting.clone()),
            remedy: Some(error.remedy.clone()),
        }
    }

    pub fn for_setting(
        setting: impl Into<String>,
        consequence: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        let setting_name = setting.into();
        Self {
            code: format!("setting.{setting_name}"),
            message: consequence.into(),
            source: Some(setting_name),
            remedy: Some(remedy.into()),
        }
    }

    pub fn for_content(
        code: impl Into<String>,
        content_path: impl Into<String>,
        message: impl Into<String>,
        remedy: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            source: Some(content_path.into()),
            remedy,
        }
    }

    pub fn from_rejected_content_bundle(rejected: &RejectedContentBundle) -> Self {
        let remedy = match &rejected.reason {
            ContentRejectionReason::UnityBuiltArchive { .. } => {
                "rebuild the AssetBundle or Addressables group from project sources with unitas"
            }
            ContentRejectionReason::TargetMismatch { .. } => {
                "rebuild the content bundle for the active player target"
            }
            ContentRejectionReason::IncompatiblePlayerVersion { .. } => {
                "rebuild the content bundle against the shipped player's compatibility version"
            }
            ContentRejectionReason::UnreadableScene { .. } => {
                "fix the corrupted or unsupported serialized scene/asset data and rebuild"
            }
        };
        Self {
            code: String::from(rejected.reason.code()),
            message: format!(
                "bundle '{}' rejected: {}",
                rejected.bundle_name, rejected.reason
            ),
            source: Some(rejected.bundle_name.clone()),
            remedy: Some(String::from(remedy)),
        }
    }
}

impl From<BuildValidationError> for BuildDiagnostic {
    fn from(error: BuildValidationError) -> Self {
        Self::from_validation_error(&error)
    }
}

impl From<&BuildValidationError> for BuildDiagnostic {
    fn from(error: &BuildValidationError) -> Self {
        Self::from_validation_error(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityFinding {
    pub feature_id: String,
    pub tier: Tier,
    pub state: CompatibilityState,
    pub guid: Option<String>,
    pub asset_path: Option<String>,
    pub sub_asset_id: Option<String>,
    pub object_name: Option<String>,
    pub behavior: String,
    pub relied_upon_unity_differences: Vec<String>,
    pub observable_consequence: String,
    pub conditions: Vec<String>,
    pub related_findings: Vec<String>,
}

impl CompatibilityFinding {
    pub fn from_finding(finding: &Finding) -> Self {
        Self {
            feature_id: finding.feature_id.clone(),
            tier: finding.tier,
            state: finding.state,
            guid: finding.content.guid.clone(),
            asset_path: finding.content.asset_path.clone(),
            sub_asset_id: finding.content.sub_asset_id.clone(),
            object_name: finding.content.object_name.clone(),
            behavior: finding.behavior.clone(),
            relied_upon_unity_differences: finding.relied_upon_unity_differences.clone(),
            observable_consequence: finding.observable_consequence.clone(),
            conditions: finding.conditions.clone(),
            related_findings: finding.related_findings.clone(),
        }
    }

    pub fn unity_built_content(content: &str) -> Option<Self> {
        let family = RECOGNIZED_UNSUPPORTED_FAMILIES
            .iter()
            .find(|entry| entry.owning_feature_id == "BLD-005")?;
        Some(Self {
            feature_id: family.owning_feature_id.to_string(),
            tier: Tier::Native,
            state: family.state,
            guid: None,
            asset_path: Some(content.to_string()),
            sub_asset_id: None,
            object_name: None,
            behavior: family.recognized_form.to_string(),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: family.observable_consequence.to_string(),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        })
    }
}

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Exact => "exact",
        Tier::Semantic => "semantic",
        Tier::Familiar => "familiar",
        Tier::Native => "native",
    }
}

fn parse_tier(raw: &str) -> Result<Tier, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "exact" => Ok(Tier::Exact),
        "semantic" => Ok(Tier::Semantic),
        "familiar" => Ok(Tier::Familiar),
        "native" => Ok(Tier::Native),
        other => Err(ReportParseError::new(format!(
            "unrecognized compatibility tier '{other}'"
        ))),
    }
}

fn state_name(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "supported",
        CompatibilityState::PartiallySupported => "partially_supported",
        CompatibilityState::Unsupported => "unsupported",
        CompatibilityState::Unknown => "unknown",
        CompatibilityState::PlatformLimited => "platform_limited",
        CompatibilityState::ExternallyUnavailable => "externally_unavailable",
    }
}

fn parse_state(raw: &str) -> Result<CompatibilityState, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "supported" => Ok(CompatibilityState::Supported),
        "partially_supported" => Ok(CompatibilityState::PartiallySupported),
        "unsupported" => Ok(CompatibilityState::Unsupported),
        "unknown" => Ok(CompatibilityState::Unknown),
        "platform_limited" => Ok(CompatibilityState::PlatformLimited),
        "externally_unavailable" => Ok(CompatibilityState::ExternallyUnavailable),
        other => Err(ReportParseError::new(format!(
            "unrecognized compatibility state '{other}'"
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeCategoryEntry {
    pub category: String,
    pub size_bytes: u64,
}

impl SizeCategoryEntry {
    pub fn new(category: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            category: category.into(),
            size_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeContributor {
    pub path: String,
    pub category: String,
    pub size_bytes: u64,
    pub content_hash: String,
}

impl SizeContributor {
    pub fn new(
        path: impl Into<String>,
        category: impl Into<String>,
        size_bytes: u64,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            category: category.into(),
            size_bytes,
            content_hash: content_hash.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportParseError {
    pub message: String,
}

impl ReportParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReportParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to parse build report: {}", self.message)
    }
}

impl std::error::Error for ReportParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub target: BuildTarget,
    pub architecture: TargetArch,
    pub configuration: BuildMode,
    pub company_name: String,
    pub product_name: String,
    pub application_identifier: String,
    pub version: String,
    pub included_scenes: Vec<String>,
    pub outcome: BuildOutcome,
    pub elapsed_ms: u64,
    pub errors: Vec<BuildDiagnostic>,
    pub warnings: Vec<BuildDiagnostic>,
    pub output_location: Option<PathBuf>,
    pub launch_artifact: Option<PathBuf>,
    pub total_size_bytes: u64,
    pub category_sizes: Vec<SizeCategoryEntry>,
    pub major_contributors: Vec<SizeContributor>,
    pub development_diagnostics_included: bool,
    pub unsupported_behaviors: Vec<String>,
    pub compatibility_findings: Vec<CompatibilityFinding>,
    pub licensing: Option<LicensingRecord>,
}

impl BuildReport {
    pub fn succeeded(
        config: &BuildConfiguration,
        elapsed_ms: u64,
        output_location: PathBuf,
        launch_artifact: PathBuf,
        total_size_bytes: u64,
        category_sizes: Vec<SizeCategoryEntry>,
        major_contributors: Vec<SizeContributor>,
    ) -> Self {
        let mut sorted_contributors = major_contributors;
        sorted_contributors.sort_by(|left, right| {
            right
                .size_bytes
                .cmp(&left.size_bytes)
                .then_with(|| left.path.cmp(&right.path))
        });
        Self {
            target: config.target,
            architecture: config.target.arch(),
            configuration: config.mode(),
            company_name: config.company_name.clone(),
            product_name: config.product_name.clone(),
            application_identifier: config.application_identifier.clone(),
            version: config.version.clone(),
            included_scenes: config
                .scenes
                .iter()
                .map(|scene| scene.to_string_lossy().into_owned())
                .collect(),
            outcome: BuildOutcome::Succeeded,
            elapsed_ms,
            errors: Vec::new(),
            warnings: Vec::new(),
            output_location: Some(output_location),
            launch_artifact: Some(launch_artifact),
            total_size_bytes,
            category_sizes,
            major_contributors: sorted_contributors,
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: Some(licensing_record(&config.application_identifier)),
        }
    }

    pub fn failed(
        config: &BuildConfiguration,
        elapsed_ms: u64,
        errors: Vec<BuildDiagnostic>,
    ) -> Self {
        Self {
            target: config.target,
            architecture: config.target.arch(),
            configuration: config.mode(),
            company_name: config.company_name.clone(),
            product_name: config.product_name.clone(),
            application_identifier: config.application_identifier.clone(),
            version: config.version.clone(),
            included_scenes: config
                .scenes
                .iter()
                .map(|scene| scene.to_string_lossy().into_owned())
                .collect(),
            outcome: BuildOutcome::Failed,
            elapsed_ms,
            errors,
            warnings: Vec::new(),
            output_location: None,
            launch_artifact: None,
            total_size_bytes: 0,
            category_sizes: Vec::new(),
            major_contributors: Vec::new(),
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: None,
        }
    }

    pub fn failed_validation(
        config: &BuildConfiguration,
        elapsed_ms: u64,
        validation_errors: &[BuildValidationError],
    ) -> Self {
        let diagnostics = validation_errors
            .iter()
            .map(BuildDiagnostic::from_validation_error)
            .collect();
        Self::failed(config, elapsed_ms, diagnostics)
    }

    pub fn cancelled(config: &BuildConfiguration, elapsed_ms: u64) -> Self {
        Self {
            target: config.target,
            architecture: config.target.arch(),
            configuration: config.mode(),
            company_name: config.company_name.clone(),
            product_name: config.product_name.clone(),
            application_identifier: config.application_identifier.clone(),
            version: config.version.clone(),
            included_scenes: config
                .scenes
                .iter()
                .map(|scene| scene.to_string_lossy().into_owned())
                .collect(),
            outcome: BuildOutcome::Cancelled,
            elapsed_ms,
            errors: Vec::new(),
            warnings: Vec::new(),
            output_location: None,
            launch_artifact: None,
            total_size_bytes: 0,
            category_sizes: Vec::new(),
            major_contributors: Vec::new(),
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: None,
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<BuildDiagnostic>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn with_unsupported_behaviors(mut self, behaviors: Vec<String>) -> Self {
        self.unsupported_behaviors = behaviors;
        self
    }

    pub fn with_compatibility_report(mut self, report: &CompatibilityReport) -> Self {
        self.compatibility_findings = report
            .findings
            .iter()
            .map(CompatibilityFinding::from_finding)
            .collect();
        self
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        push_kv_str(&mut out, 2, "target", &self.target.identifier(), true);
        push_kv_str(
            &mut out,
            2,
            "architecture",
            self.architecture.as_str(),
            true,
        );
        push_kv_str(
            &mut out,
            2,
            "configuration",
            self.configuration.as_str(),
            true,
        );
        push_kv_str(&mut out, 2, "company_name", &self.company_name, true);
        push_kv_str(&mut out, 2, "product_name", &self.product_name, true);
        push_kv_str(
            &mut out,
            2,
            "application_identifier",
            &self.application_identifier,
            true,
        );
        push_kv_str(&mut out, 2, "version", &self.version, true);
        push_kv_string_array(&mut out, 2, "included_scenes", &self.included_scenes, true);
        push_kv_str(&mut out, 2, "outcome", self.outcome.as_str(), true);
        push_kv_u64(&mut out, 2, "elapsed_ms", self.elapsed_ms, true);
        push_kv_diagnostics(&mut out, 2, "errors", &self.errors, true);
        push_kv_diagnostics(&mut out, 2, "warnings", &self.warnings, true);
        push_kv_opt_path(
            &mut out,
            2,
            "output_location",
            self.output_location.as_deref(),
            true,
        );
        push_kv_opt_path(
            &mut out,
            2,
            "launch_artifact",
            self.launch_artifact.as_deref(),
            true,
        );
        push_kv_u64(&mut out, 2, "total_size_bytes", self.total_size_bytes, true);
        push_kv_category_sizes(&mut out, 2, "category_sizes", &self.category_sizes, true);
        push_kv_contributors(
            &mut out,
            2,
            "major_contributors",
            &self.major_contributors,
            true,
        );
        push_kv_bool(
            &mut out,
            2,
            "development_diagnostics_included",
            self.development_diagnostics_included,
            true,
        );
        push_kv_string_array(
            &mut out,
            2,
            "unsupported_behaviors",
            &self.unsupported_behaviors,
            true,
        );
        push_kv_compatibility_findings(
            &mut out,
            2,
            "compatibility_findings",
            &self.compatibility_findings,
            true,
        );
        push_kv_licensing(&mut out, 2, "licensing", self.licensing.as_ref(), false);
        out.push_str("}\n");
        out
    }

    pub fn from_json(json: &str) -> Result<Self, ReportParseError> {
        let root = JsonParser::parse(json)?;
        let obj = root.as_object("root")?;

        let target_str = get_required_str(obj, "target")?;
        let architecture_str = get_required_str(obj, "architecture")?;
        let architecture = parse_target_arch(architecture_str)?;
        let target = parse_build_target(target_str, architecture)?;

        let configuration_str = get_required_str(obj, "configuration")?;
        let configuration = parse_build_mode(configuration_str)?;

        let company_name = get_required_str(obj, "company_name")?.to_string();
        let product_name = get_required_str(obj, "product_name")?.to_string();
        let application_identifier = get_required_str(obj, "application_identifier")?.to_string();
        let version = get_required_str(obj, "version")?.to_string();
        let included_scenes = get_required_string_list(obj, "included_scenes")?;

        let outcome_str = get_required_str(obj, "outcome")?;
        let outcome = BuildOutcome::from_name(outcome_str).ok_or_else(|| {
            ReportParseError::new(format!("unrecognized build outcome '{outcome_str}'"))
        })?;

        let elapsed_ms = get_required_u64(obj, "elapsed_ms")?;
        let errors = get_required_diagnostics(obj, "errors")?;
        let warnings = get_required_diagnostics(obj, "warnings")?;
        let output_location = get_optional_path(obj, "output_location")?;
        let launch_artifact = get_optional_path(obj, "launch_artifact")?;
        let total_size_bytes = get_required_u64(obj, "total_size_bytes")?;
        let category_sizes = get_required_category_sizes(obj, "category_sizes")?;
        let major_contributors = get_required_contributors(obj, "major_contributors")?;
        let development_diagnostics_included =
            get_required_bool(obj, "development_diagnostics_included")?;
        let unsupported_behaviors = get_required_string_list(obj, "unsupported_behaviors")?;
        let compatibility_findings =
            get_required_compatibility_findings(obj, "compatibility_findings")?;
        let licensing = get_optional_licensing(obj)?;

        Ok(Self {
            target,
            architecture,
            configuration,
            company_name,
            product_name,
            application_identifier,
            version,
            included_scenes,
            outcome,
            elapsed_ms,
            errors,
            warnings,
            output_location,
            launch_artifact,
            total_size_bytes,
            category_sizes,
            major_contributors,
            development_diagnostics_included,
            unsupported_behaviors,
            compatibility_findings,
            licensing,
        })
    }

    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        std::fs::write(path, self.to_json())
    }

    pub fn read_from_file(path: &Path) -> Result<Self, ReportParseError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| ReportParseError::new(format!("io error reading report: {err}")))?;
        Self::from_json(&content)
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn push_escaped_json_str(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            control if (control as u32) < 0x20 => {
                let code = control as u32;
                out.push_str(&format!("\\u{code:04x}"));
            }
            normal => out.push(normal),
        }
    }
    out.push('"');
}

fn push_trailing(out: &mut String, trailing_comma: bool) {
    if trailing_comma {
        out.push_str(",\n");
    } else {
        out.push('\n');
    }
}

fn push_kv_str(out: &mut String, indent: usize, key: &str, value: &str, trailing_comma: bool) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    push_escaped_json_str(out, value);
    push_trailing(out, trailing_comma);
}

fn push_kv_opt_str(
    out: &mut String,
    indent: usize,
    key: &str,
    value: Option<&str>,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    match value {
        Some(text) => push_escaped_json_str(out, text),
        None => out.push_str("null"),
    }
    push_trailing(out, trailing_comma);
}

fn push_kv_opt_path(
    out: &mut String,
    indent: usize,
    key: &str,
    value: Option<&Path>,
    trailing_comma: bool,
) {
    let text = value.map(|path| path.to_string_lossy().into_owned());
    push_kv_opt_str(out, indent, key, text.as_deref(), trailing_comma);
}

fn push_kv_u64(out: &mut String, indent: usize, key: &str, value: u64, trailing_comma: bool) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    out.push_str(&value.to_string());
    push_trailing(out, trailing_comma);
}

fn push_kv_bool(out: &mut String, indent: usize, key: &str, value: bool, trailing_comma: bool) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    out.push_str(": ");
    out.push_str(if value { "true" } else { "false" });
    push_trailing(out, trailing_comma);
}

fn push_kv_string_array(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[String],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        push_escaped_json_str(out, item);
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_diagnostics(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[BuildDiagnostic],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "code", &item.code, true);
        push_kv_str(out, indent + 4, "message", &item.message, true);
        push_kv_opt_str(out, indent + 4, "source", item.source.as_deref(), true);
        push_kv_opt_str(out, indent + 4, "remedy", item.remedy.as_deref(), false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_compatibility_findings(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[CompatibilityFinding],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "feature_id", &item.feature_id, true);
        push_kv_str(out, indent + 4, "tier", tier_name(item.tier), true);
        push_kv_str(out, indent + 4, "state", state_name(item.state), true);
        push_kv_opt_str(out, indent + 4, "guid", item.guid.as_deref(), true);
        push_kv_opt_str(
            out,
            indent + 4,
            "asset_path",
            item.asset_path.as_deref(),
            true,
        );
        push_kv_opt_str(
            out,
            indent + 4,
            "sub_asset_id",
            item.sub_asset_id.as_deref(),
            true,
        );
        push_kv_opt_str(
            out,
            indent + 4,
            "object_name",
            item.object_name.as_deref(),
            true,
        );
        push_kv_str(out, indent + 4, "behavior", &item.behavior, true);
        push_kv_string_array(
            out,
            indent + 4,
            "relied_upon_unity_differences",
            &item.relied_upon_unity_differences,
            true,
        );
        push_kv_str(
            out,
            indent + 4,
            "observable_consequence",
            &item.observable_consequence,
            true,
        );
        push_kv_string_array(out, indent + 4, "conditions", &item.conditions, true);
        push_kv_string_array(
            out,
            indent + 4,
            "related_findings",
            &item.related_findings,
            false,
        );
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_category_sizes(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[SizeCategoryEntry],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "category", &item.category, true);
        push_kv_u64(out, indent + 4, "size_bytes", item.size_bytes, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_contributors(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[SizeContributor],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "path", &item.path, true);
        push_kv_str(out, indent + 4, "category", &item.category, true);
        push_kv_u64(out, indent + 4, "size_bytes", item.size_bytes, true);
        push_kv_str(out, indent + 4, "content_hash", &item.content_hash, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn push_kv_licensing(
    out: &mut String,
    indent: usize,
    key: &str,
    licensing: Option<&LicensingRecord>,
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    let Some(record) = licensing else {
        out.push_str(": null");
        push_trailing(out, trailing_comma);
        return;
    };
    out.push_str(": {\n");
    push_kv_str(
        out,
        indent + 2,
        "application_identifier",
        &record.application_identifier,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "governing_license",
        &record.governing_license,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "governing_license_name",
        &record.governing_license_name,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "conveyance_statement",
        &record.conveyance_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "corresponding_source_obligation",
        &record.corresponding_source_obligation,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "corresponding_source_availability",
        &record.corresponding_source_availability,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "linking_exception_statement",
        &record.linking_exception_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "commercial_license_statement",
        &record.commercial_license_statement,
        true,
    );
    push_kv_str(
        out,
        indent + 2,
        "not_legal_advice_statement",
        &record.not_legal_advice_statement,
        true,
    );
    push_kv_string_array(
        out,
        indent + 2,
        "license_texts",
        &record.license_texts,
        true,
    );
    push_kv_notices(
        out,
        indent + 2,
        "third_party_notices",
        &record.third_party_notices,
        false,
    );
    push_indent(out, indent);
    out.push('}');
    push_trailing(out, trailing_comma);
}

fn push_kv_notices(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[ThirdPartyNotice],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "ecosystem", &item.ecosystem, true);
        push_kv_str(out, indent + 4, "component", &item.component, true);
        push_kv_str(out, indent + 4, "version", &item.version, true);
        push_kv_str(out, indent + 4, "license", &item.license, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub(crate) fn as_object(
        &self,
        context: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ReportParseError> {
        match self {
            JsonValue::Object(map) => Ok(map),
            _ => Err(ReportParseError::new(format!(
                "expected JSON object for {context}"
            ))),
        }
    }

    pub(crate) fn as_array(&self, context: &str) -> Result<&[JsonValue], ReportParseError> {
        match self {
            JsonValue::Array(items) => Ok(items.as_slice()),
            _ => Err(ReportParseError::new(format!(
                "expected JSON array for {context}"
            ))),
        }
    }

    pub(crate) fn as_str(&self, context: &str) -> Result<&str, ReportParseError> {
        match self {
            JsonValue::String(text) => Ok(text.as_str()),
            _ => Err(ReportParseError::new(format!(
                "expected JSON string for {context}"
            ))),
        }
    }

    fn as_u64(&self, context: &str) -> Result<u64, ReportParseError> {
        match self {
            JsonValue::Number(num) => Ok(*num),
            _ => Err(ReportParseError::new(format!(
                "expected JSON unsigned integer for {context}"
            ))),
        }
    }

    fn as_bool(&self, context: &str) -> Result<bool, ReportParseError> {
        match self {
            JsonValue::Bool(flag) => Ok(*flag),
            _ => Err(ReportParseError::new(format!(
                "expected JSON boolean for {context}"
            ))),
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

/// Parses a JSON document into the report's JSON value model.
pub(crate) fn parse_json_document(input: &str) -> Result<JsonValue, ReportParseError> {
    JsonParser::parse(input)
}

impl<'a> JsonParser<'a> {
    fn parse(input: &'a str) -> Result<JsonValue, ReportParseError> {
        let mut parser = Self {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.pos != parser.bytes.len() {
            return Err(ReportParseError::new(format!(
                "unexpected trailing JSON content at byte {}",
                parser.pos
            )));
        }
        Ok(value)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<JsonValue, ReportParseError> {
        self.skip_whitespace();
        let byte = self.peek().ok_or_else(|| {
            ReportParseError::new(format!("unexpected end of JSON at byte {}", self.pos))
        })?;
        match byte {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' | b'f' => self.parse_bool().map(JsonValue::Bool),
            b'n' => self.parse_null(),
            b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            other => Err(ReportParseError::new(format!(
                "unexpected JSON token '{}' at byte {}",
                char::from(other),
                self.pos
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, ReportParseError> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(ReportParseError::new(format!(
                    "expected object key string at byte {}",
                    self.pos
                )));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(ReportParseError::new(format!(
                    "expected ':' after key '{key}' at byte {}",
                    self.pos
                )));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(map));
                }
                _ => {
                    return Err(ReportParseError::new(format!(
                        "expected ',' or '}}' in object at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, ReportParseError> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }

        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => {
                    return Err(ReportParseError::new(format!(
                        "expected ',' or ']' in array at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, ReportParseError> {
        self.pos += 1;
        let mut raw = Vec::new();
        while let Some(&byte) = self.bytes.get(self.pos) {
            self.pos += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(raw).map_err(|err| {
                        ReportParseError::new(format!("invalid UTF-8 in JSON string: {err}"))
                    });
                }
                b'\\' => {
                    let escaped = self.bytes.get(self.pos).copied().ok_or_else(|| {
                        ReportParseError::new("unexpected end of JSON escape sequence")
                    })?;
                    self.pos += 1;
                    match escaped {
                        b'"' => raw.push(b'"'),
                        b'\\' => raw.push(b'\\'),
                        b'/' => raw.push(b'/'),
                        b'n' => raw.push(b'\n'),
                        b'r' => raw.push(b'\r'),
                        b't' => raw.push(b'\t'),
                        b'b' => raw.push(0x08),
                        b'f' => raw.push(0x0C),
                        b'u' => {
                            let hex_slice =
                                self.bytes.get(self.pos..self.pos + 4).ok_or_else(|| {
                                    ReportParseError::new("incomplete \\uXXXX escape sequence")
                                })?;
                            let hex_str = std::str::from_utf8(hex_slice).map_err(|_| {
                                ReportParseError::new("invalid hex digits in \\uXXXX escape")
                            })?;
                            let code = u32::from_str_radix(hex_str, 16).map_err(|_| {
                                ReportParseError::new("invalid hex value in \\uXXXX escape")
                            })?;
                            let ch = char::from_u32(code).ok_or_else(|| {
                                ReportParseError::new("invalid Unicode scalar in \\uXXXX escape")
                            })?;
                            let mut buf = [0u8; 4];
                            raw.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            self.pos += 4;
                        }
                        other => {
                            return Err(ReportParseError::new(format!(
                                "unsupported JSON escape '\\{}'",
                                char::from(other)
                            )));
                        }
                    }
                }
                normal => raw.push(normal),
            }
        }
        Err(ReportParseError::new("unterminated JSON string"))
    }

    fn parse_bool(&mut self) -> Result<bool, ReportParseError> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"true") {
            self.pos += 4;
            Ok(true)
        } else if self.bytes.get(self.pos..self.pos + 5) == Some(b"false") {
            self.pos += 5;
            Ok(false)
        } else {
            Err(ReportParseError::new(format!(
                "invalid boolean literal at byte {}",
                self.pos
            )))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, ReportParseError> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(ReportParseError::new(format!(
                "invalid null literal at byte {}",
                self.pos
            )))
        }
    }

    fn parse_number(&mut self) -> Result<u64, ReportParseError> {
        let start = self.pos;
        while let Some(&byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let slice = self
            .bytes
            .get(start..self.pos)
            .ok_or_else(|| ReportParseError::new("invalid numeric slice"))?;
        let text = std::str::from_utf8(slice)
            .map_err(|_| ReportParseError::new("invalid UTF-8 in number"))?;
        text.parse::<u64>()
            .map_err(|err| ReportParseError::new(format!("invalid u64 number '{text}': {err}")))
    }
}

fn get_field<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a JsonValue, ReportParseError> {
    obj.get(key)
        .ok_or_else(|| ReportParseError::new(format!("missing required field '{key}'")))
}

fn get_required_str<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, ReportParseError> {
    get_field(obj, key)?.as_str(key)
}

fn get_optional_string(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<String>, ReportParseError> {
    match get_field(obj, key)? {
        JsonValue::Null => Ok(None),
        value => Ok(Some(value.as_str(key)?.to_string())),
    }
}

fn get_optional_path(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<PathBuf>, ReportParseError> {
    Ok(get_optional_string(obj, key)?.map(PathBuf::from))
}

fn get_required_u64(obj: &BTreeMap<String, JsonValue>, key: &str) -> Result<u64, ReportParseError> {
    get_field(obj, key)?.as_u64(key)
}

fn get_required_bool(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<bool, ReportParseError> {
    get_field(obj, key)?.as_bool(key)
}

fn get_required_string_list(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<String>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        list.push(item.as_str(key)?.to_string());
    }
    Ok(list)
}

fn get_required_diagnostics(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<BuildDiagnostic>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let diag_obj = item.as_object(key)?;
        let code = get_required_str(diag_obj, "code")?.to_string();
        let message = get_required_str(diag_obj, "message")?.to_string();
        let source = get_optional_string(diag_obj, "source")?;
        let remedy = get_optional_string(diag_obj, "remedy")?;
        list.push(BuildDiagnostic {
            code,
            message,
            source,
            remedy,
        });
    }
    Ok(list)
}

fn get_required_compatibility_findings(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<CompatibilityFinding>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let feature_id = get_required_str(entry_obj, "feature_id")?.to_string();
        let tier = parse_tier(get_required_str(entry_obj, "tier")?)?;
        let state = parse_state(get_required_str(entry_obj, "state")?)?;
        let guid = get_optional_string(entry_obj, "guid")?;
        let asset_path = get_optional_string(entry_obj, "asset_path")?;
        let sub_asset_id = get_optional_string(entry_obj, "sub_asset_id")?;
        let object_name = get_optional_string(entry_obj, "object_name")?;
        let behavior = get_required_str(entry_obj, "behavior")?.to_string();
        let relied_upon_unity_differences =
            get_required_string_list(entry_obj, "relied_upon_unity_differences")?;
        let observable_consequence =
            get_required_str(entry_obj, "observable_consequence")?.to_string();
        let conditions = get_required_string_list(entry_obj, "conditions")?;
        let related_findings = get_required_string_list(entry_obj, "related_findings")?;
        list.push(CompatibilityFinding {
            feature_id,
            tier,
            state,
            guid,
            asset_path,
            sub_asset_id,
            object_name,
            behavior,
            relied_upon_unity_differences,
            observable_consequence,
            conditions,
            related_findings,
        });
    }
    Ok(list)
}

fn get_required_category_sizes(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<SizeCategoryEntry>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let category = get_required_str(entry_obj, "category")?.to_string();
        let size_bytes = get_required_u64(entry_obj, "size_bytes")?;
        list.push(SizeCategoryEntry {
            category,
            size_bytes,
        });
    }
    Ok(list)
}

fn get_required_contributors(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<SizeContributor>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let path = get_required_str(entry_obj, "path")?.to_string();
        let category = get_required_str(entry_obj, "category")?.to_string();
        let size_bytes = get_required_u64(entry_obj, "size_bytes")?;
        let content_hash = get_required_str(entry_obj, "content_hash")?.to_string();
        list.push(SizeContributor {
            path,
            category,
            size_bytes,
            content_hash,
        });
    }
    Ok(list)
}

fn get_optional_licensing(
    obj: &BTreeMap<String, JsonValue>,
) -> Result<Option<LicensingRecord>, ReportParseError> {
    match obj.get("licensing") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let record = value.as_object("licensing")?;
            Ok(Some(LicensingRecord {
                application_identifier: get_required_str(record, "application_identifier")?
                    .to_string(),
                governing_license: get_required_str(record, "governing_license")?.to_string(),
                governing_license_name: get_required_str(record, "governing_license_name")?
                    .to_string(),
                conveyance_statement: get_required_str(record, "conveyance_statement")?.to_string(),
                corresponding_source_obligation: get_required_str(
                    record,
                    "corresponding_source_obligation",
                )?
                .to_string(),
                corresponding_source_availability: get_required_str(
                    record,
                    "corresponding_source_availability",
                )?
                .to_string(),
                linking_exception_statement: get_required_str(
                    record,
                    "linking_exception_statement",
                )?
                .to_string(),
                commercial_license_statement: get_required_str(
                    record,
                    "commercial_license_statement",
                )?
                .to_string(),
                not_legal_advice_statement: get_required_str(record, "not_legal_advice_statement")?
                    .to_string(),
                license_texts: get_required_string_list(record, "license_texts")?,
                third_party_notices: get_required_notices(record, "third_party_notices")?,
            }))
        }
    }
}

fn get_required_notices(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<ThirdPartyNotice>, ReportParseError> {
    let array = get_field(obj, key)?.as_array(key)?;
    let mut notices = Vec::with_capacity(array.len());
    for item in array {
        let entry = item.as_object(key)?;
        notices.push(ThirdPartyNotice {
            ecosystem: get_required_str(entry, "ecosystem")?.to_string(),
            component: get_required_str(entry, "component")?.to_string(),
            version: get_required_str(entry, "version")?.to_string(),
            license: get_required_str(entry, "license")?.to_string(),
        });
    }
    Ok(notices)
}

fn parse_target_arch(raw: &str) -> Result<TargetArch, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "x86_64" | "x86-64" => Ok(TargetArch::X86_64),
        "aarch64" | "arm64" => Ok(TargetArch::Arm64),
        _ => Err(ReportParseError::new(format!(
            "unrecognized target architecture '{raw}'"
        ))),
    }
}

fn parse_build_target(
    raw: &str,
    fallback_arch: TargetArch,
) -> Result<BuildTarget, ReportParseError> {
    for candidate in BuildTarget::all() {
        if candidate.identifier().eq_ignore_ascii_case(raw)
            || candidate.to_string().eq_ignore_ascii_case(raw)
        {
            return Ok(*candidate);
        }
    }
    let os = match raw.to_ascii_lowercase().as_str() {
        "windows" => Some(TargetOs::Windows),
        "macos" => Some(TargetOs::MacOs),
        "linux" => Some(TargetOs::Linux),
        _ => None,
    };
    if let Some(target_os) = os {
        return Ok(BuildTarget::new(target_os, fallback_arch));
    }
    Err(ReportParseError::new(format!(
        "unrecognized build target '{raw}'"
    )))
}

fn parse_build_mode(raw: &str) -> Result<BuildMode, ReportParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "development" => Ok(BuildMode::Development),
        "release" => Ok(BuildMode::Release),
        _ => Err(ReportParseError::new(format!(
            "unrecognized build configuration mode '{raw}'"
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedBundleReportEntry {
    pub bundle_name: String,
    pub source_path: String,
    pub code: String,
    pub reason: String,
}

impl RejectedBundleReportEntry {
    pub fn from_rejected_bundle(rejected: &RejectedContentBundle) -> Self {
        Self {
            bundle_name: rejected.bundle_name.clone(),
            source_path: rejected.source_path.to_string_lossy().into_owned(),
            code: String::from(rejected.reason.code()),
            reason: rejected.reason.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentBuildReport {
    pub mode: ContentBuildMode,
    pub unitas_version: String,
    pub player_compatibility_version: String,
    pub target: BuildTarget,
    pub architecture: TargetArch,
    pub outcome: BuildOutcome,
    pub elapsed_ms: u64,
    pub catalog_path: Option<PathBuf>,
    pub catalog_hash: String,
    pub emitted_bundles: Vec<String>,
    pub unchanged_bundles: Vec<String>,
    pub rejected_bundles: Vec<RejectedBundleReportEntry>,
    pub errors: Vec<BuildDiagnostic>,
    pub compatibility_findings: Vec<CompatibilityFinding>,
}

impl ContentBuildReport {
    pub fn from_build_result(
        result: &ContentBuildResult,
        mode: ContentBuildMode,
        elapsed_ms: u64,
    ) -> Self {
        let outcome = if result.rejected_bundles.is_empty() {
            BuildOutcome::Succeeded
        } else {
            BuildOutcome::Failed
        };
        let emitted_bundles = result
            .emitted_bundles
            .iter()
            .map(|entry| entry.bundle_name.clone())
            .collect();
        let rejected_bundles = result
            .rejected_bundles
            .iter()
            .map(RejectedBundleReportEntry::from_rejected_bundle)
            .collect();
        let errors = result
            .rejected_bundles
            .iter()
            .map(BuildDiagnostic::from_rejected_content_bundle)
            .collect();
        let mut compatibility_findings = Vec::new();
        for rejected in &result.rejected_bundles {
            if matches!(
                &rejected.reason,
                ContentRejectionReason::UnityBuiltArchive { .. }
            ) {
                let source_path = rejected.source_path.to_string_lossy().into_owned();
                if let Some(finding) = CompatibilityFinding::unity_built_content(&source_path) {
                    compatibility_findings.push(finding);
                }
            }
        }

        Self {
            mode,
            unitas_version: result.catalog.unitas_version.clone(),
            player_compatibility_version: result.catalog.player_compatibility_version.clone(),
            target: result.catalog.target,
            architecture: result.catalog.target.arch(),
            outcome,
            elapsed_ms,
            catalog_path: Some(result.catalog_path.clone()),
            catalog_hash: result.catalog.catalog_hash.clone(),
            emitted_bundles,
            unchanged_bundles: result.unchanged_bundles.clone(),
            rejected_bundles,
            errors,
            compatibility_findings,
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        push_kv_str(&mut out, 2, "mode", self.mode.as_str(), true);
        push_kv_str(&mut out, 2, "unitas_version", &self.unitas_version, true);
        push_kv_str(
            &mut out,
            2,
            "player_compatibility_version",
            &self.player_compatibility_version,
            true,
        );
        push_kv_str(&mut out, 2, "target", &self.target.identifier(), true);
        push_kv_str(
            &mut out,
            2,
            "architecture",
            self.architecture.as_str(),
            true,
        );
        push_kv_str(&mut out, 2, "outcome", self.outcome.as_str(), true);
        push_kv_u64(&mut out, 2, "elapsed_ms", self.elapsed_ms, true);
        push_kv_opt_path(
            &mut out,
            2,
            "catalog_path",
            self.catalog_path.as_deref(),
            true,
        );
        push_kv_str(&mut out, 2, "catalog_hash", &self.catalog_hash, true);
        push_kv_string_array(&mut out, 2, "emitted_bundles", &self.emitted_bundles, true);
        push_kv_string_array(
            &mut out,
            2,
            "unchanged_bundles",
            &self.unchanged_bundles,
            true,
        );
        push_kv_rejected_bundles(
            &mut out,
            2,
            "rejected_bundles",
            &self.rejected_bundles,
            true,
        );
        push_kv_diagnostics(&mut out, 2, "errors", &self.errors, true);
        push_kv_compatibility_findings(
            &mut out,
            2,
            "compatibility_findings",
            &self.compatibility_findings,
            false,
        );
        out.push_str("}\n");
        out
    }

    pub fn from_json(json: &str) -> Result<Self, ReportParseError> {
        let root = JsonParser::parse(json)?;
        let obj = root.as_object("root")?;

        let mode_str = get_required_str(obj, "mode")?;
        let mode = ContentBuildMode::from_name(mode_str).ok_or_else(|| {
            ReportParseError::new(format!("unrecognized content build mode '{mode_str}'"))
        })?;
        let unitas_version = get_required_str(obj, "unitas_version")?.to_string();
        let player_compatibility_version =
            get_required_str(obj, "player_compatibility_version")?.to_string();
        let target_str = get_required_str(obj, "target")?;
        let architecture_str = get_required_str(obj, "architecture")?;
        let architecture = parse_target_arch(architecture_str)?;
        let target = parse_build_target(target_str, architecture)?;
        let outcome_str = get_required_str(obj, "outcome")?;
        let outcome = BuildOutcome::from_name(outcome_str).ok_or_else(|| {
            ReportParseError::new(format!("unrecognized build outcome '{outcome_str}'"))
        })?;
        let elapsed_ms = get_required_u64(obj, "elapsed_ms")?;
        let catalog_path = get_optional_path(obj, "catalog_path")?;
        let catalog_hash = get_required_str(obj, "catalog_hash")?.to_string();
        let emitted_bundles = get_required_string_list(obj, "emitted_bundles")?;
        let unchanged_bundles = get_required_string_list(obj, "unchanged_bundles")?;
        let rejected_bundles = get_required_rejected_bundles(obj, "rejected_bundles")?;
        let errors = get_required_diagnostics(obj, "errors")?;
        let compatibility_findings =
            get_required_compatibility_findings(obj, "compatibility_findings")?;

        Ok(Self {
            mode,
            unitas_version,
            player_compatibility_version,
            target,
            architecture,
            outcome,
            elapsed_ms,
            catalog_path,
            catalog_hash,
            emitted_bundles,
            unchanged_bundles,
            rejected_bundles,
            errors,
            compatibility_findings,
        })
    }
}

fn push_kv_rejected_bundles(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[RejectedBundleReportEntry],
    trailing_comma: bool,
) {
    push_indent(out, indent);
    push_escaped_json_str(out, key);
    if items.is_empty() {
        out.push_str(": []");
        push_trailing(out, trailing_comma);
        return;
    }
    out.push_str(": [\n");
    for (index, item) in items.iter().enumerate() {
        push_indent(out, indent + 2);
        out.push_str("{\n");
        push_kv_str(out, indent + 4, "bundle_name", &item.bundle_name, true);
        push_kv_str(out, indent + 4, "source_path", &item.source_path, true);
        push_kv_str(out, indent + 4, "code", &item.code, true);
        push_kv_str(out, indent + 4, "reason", &item.reason, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn get_required_rejected_bundles(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<RejectedBundleReportEntry>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let bundle_name = get_required_str(entry_obj, "bundle_name")?.to_string();
        let source_path = get_required_str(entry_obj, "source_path")?.to_string();
        let code = get_required_str(entry_obj, "code")?.to_string();
        let reason = get_required_str(entry_obj, "reason")?.to_string();
        list.push(RejectedBundleReportEntry {
            bundle_name,
            source_path,
            code,
            reason,
        });
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> BuildConfiguration {
        BuildConfiguration::new(
            vec![
                PathBuf::from("Assets/Scenes/Boot.unity"),
                PathBuf::from("Assets/Scenes/Gameplay.unity"),
            ],
            "Unitas Studio",
            "SpaceRunner",
            "com.unitas.spacerunner",
            "1.4.2",
            BuildTarget::MACOS_ARM64,
        )
    }

    #[test]
    fn succeeded_report_records_the_licensing_record() -> Result<(), Box<dyn std::error::Error>> {
        let config = sample_config();
        let report = BuildReport::succeeded(
            &config,
            10,
            PathBuf::from("/tmp/unitas-builds/SpaceRunner"),
            PathBuf::from(
                "/tmp/unitas-builds/SpaceRunner/SpaceRunner.app/Contents/MacOS/SpaceRunner",
            ),
            1024,
            Vec::new(),
            Vec::new(),
        );
        let licensing = report
            .licensing
            .as_ref()
            .ok_or("a succeeded report records the governing license")?;
        assert_eq!(
            licensing.application_identifier,
            config.application_identifier
        );
        assert_eq!(licensing.governing_license, "GPL-3.0-only");
        assert!(licensing.conveyance_statement.contains("GPLv3"));
        assert!(licensing
            .corresponding_source_obligation
            .contains("whole conveyed work"));
        assert!(licensing
            .linking_exception_statement
            .contains("no linking exception"));
        assert!(licensing
            .not_legal_advice_statement
            .contains("not legal advice"));
        assert!(!licensing.third_party_notices.is_empty());

        let json = report.to_json();
        assert!(json.contains("\"licensing\""));
        let parsed = BuildReport::from_json(&json)?;
        assert_eq!(parsed, report);
        Ok(())
    }

    #[test]
    fn succeeded_report_contains_all_spec_fields_and_round_trips_json(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = sample_config().with_development_build(true);
        let category_sizes = vec![
            SizeCategoryEntry::new("scenes", 4096),
            SizeCategoryEntry::new("metadata", 512),
            SizeCategoryEntry::new("launcher", 8192),
            SizeCategoryEntry::new("diagnostics", 256),
        ];
        let major_contributors = vec![
            SizeContributor::new("Assets/Scenes/Boot.unity", "scenes", 1024, "a1b2c3d4"),
            SizeContributor::new("SpaceRunner", "launcher", 8192, "e5f60718"),
            SizeContributor::new("Assets/Scenes/Gameplay.unity", "scenes", 3072, "99887766"),
        ];
        let warning = BuildDiagnostic::for_content(
            "content.unused_meta",
            "Assets/Scenes/Boot.unity.meta",
            "meta file contains custom \"userData\" not used by desktop target",
            Some("remove unused userData if not needed".to_string()),
        );

        let report = BuildReport::succeeded(
            &config,
            1350,
            PathBuf::from("/tmp/unitas-builds/SpaceRunner"),
            PathBuf::from("/tmp/unitas-builds/SpaceRunner/SpaceRunner"),
            13056,
            category_sizes.clone(),
            major_contributors,
        )
        .with_warnings(vec![warning.clone()])
        .with_unsupported_behaviors(vec![
            "mobile.orientation_lock is ignored on desktop targets".to_string(),
        ]);

        assert_eq!(report.target, BuildTarget::MACOS_ARM64);
        assert_eq!(report.architecture, TargetArch::Arm64);
        assert_eq!(report.configuration, BuildMode::Development);
        assert_eq!(report.company_name, "Unitas Studio");
        assert_eq!(report.product_name, "SpaceRunner");
        assert_eq!(report.application_identifier, "com.unitas.spacerunner");
        assert_eq!(report.version, "1.4.2");
        assert_eq!(
            report.included_scenes,
            vec![
                "Assets/Scenes/Boot.unity".to_string(),
                "Assets/Scenes/Gameplay.unity".to_string()
            ]
        );
        assert_eq!(report.outcome, BuildOutcome::Succeeded);
        assert_eq!(report.elapsed_ms, 1350);
        assert!(report.errors.is_empty());
        assert_eq!(report.warnings, vec![warning]);
        assert_eq!(
            report.output_location,
            Some(PathBuf::from("/tmp/unitas-builds/SpaceRunner"))
        );
        assert_eq!(
            report.launch_artifact,
            Some(PathBuf::from("/tmp/unitas-builds/SpaceRunner/SpaceRunner"))
        );
        assert_eq!(report.total_size_bytes, 13056);
        assert_eq!(report.category_sizes, category_sizes);
        assert_eq!(report.major_contributors.len(), 3);
        assert_eq!(report.major_contributors[0].path, "SpaceRunner");
        assert_eq!(
            report.major_contributors[1].path,
            "Assets/Scenes/Gameplay.unity"
        );
        assert_eq!(
            report.major_contributors[2].path,
            "Assets/Scenes/Boot.unity"
        );
        assert!(report.development_diagnostics_included);
        assert_eq!(
            report.unsupported_behaviors,
            vec!["mobile.orientation_lock is ignored on desktop targets".to_string()]
        );

        let json = report.to_json();
        for required_key in [
            "\"target\"",
            "\"architecture\"",
            "\"configuration\"",
            "\"company_name\"",
            "\"product_name\"",
            "\"application_identifier\"",
            "\"version\"",
            "\"included_scenes\"",
            "\"outcome\"",
            "\"elapsed_ms\"",
            "\"errors\"",
            "\"warnings\"",
            "\"output_location\"",
            "\"launch_artifact\"",
            "\"total_size_bytes\"",
            "\"category_sizes\"",
            "\"major_contributors\"",
            "\"development_diagnostics_included\"",
            "\"unsupported_behaviors\"",
        ] {
            assert!(
                json.contains(required_key),
                "missing key {required_key} in JSON"
            );
        }

        let round_tripped = BuildReport::from_json(&json)?;
        assert_eq!(round_tripped, report);
        Ok(())
    }

    #[test]
    fn failed_report_from_validation_errors_clears_artifacts_and_round_trips(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut invalid_config = sample_config();
        invalid_config.scenes.clear();
        invalid_config.product_name = String::new();

        let validation_errors =
            match invalid_config.validate_with_host(Some(BuildTarget::MACOS_ARM64)) {
                Ok(()) => return Err("expected validation failure".into()),
                Err(errs) => errs,
            };

        let report = BuildReport::failed_validation(&invalid_config, 42, &validation_errors);
        assert_eq!(report.outcome, BuildOutcome::Failed);
        assert_eq!(report.elapsed_ms, 42);
        assert_eq!(report.output_location, None);
        assert_eq!(report.launch_artifact, None);
        assert_eq!(report.total_size_bytes, 0);
        assert!(report.errors.len() >= 2);
        assert!(report.errors.iter().any(|diag| {
            diag.code == "setting.scenes"
                && diag.source.as_deref() == Some("scenes")
                && !diag.message.is_empty()
                && diag.remedy.as_deref().is_some_and(|r| !r.is_empty())
        }));

        let json = report.to_json();
        let parsed = BuildReport::from_json(&json)?;
        assert_eq!(parsed, report);
        Ok(())
    }

    #[test]
    fn cancelled_report_has_no_launch_artifact_and_round_trips(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = sample_config();
        let report = BuildReport::cancelled(&config, 19);

        assert_eq!(report.outcome, BuildOutcome::Cancelled);
        assert_eq!(report.elapsed_ms, 19);
        assert_eq!(report.output_location, None);
        assert_eq!(report.launch_artifact, None);
        assert_eq!(report.total_size_bytes, 0);
        assert!(report.errors.is_empty());

        let json = report.to_json();
        let parsed = BuildReport::from_json(&json)?;
        assert_eq!(parsed, report);
        Ok(())
    }

    #[test]
    fn development_and_release_reports_distinguish_configuration_and_diagnostics(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let release_config = sample_config().with_mode(BuildMode::Release);
        let dev_config = sample_config().with_mode(BuildMode::Development);

        let release_report = BuildReport::succeeded(
            &release_config,
            100,
            PathBuf::from("/tmp/out/rel"),
            PathBuf::from("/tmp/out/rel/SpaceRunner"),
            2048,
            vec![SizeCategoryEntry::new("scenes", 2048)],
            Vec::new(),
        );
        let dev_report = BuildReport::succeeded(
            &dev_config,
            120,
            PathBuf::from("/tmp/out/dev"),
            PathBuf::from("/tmp/out/dev/SpaceRunner"),
            2304,
            vec![
                SizeCategoryEntry::new("scenes", 2048),
                SizeCategoryEntry::new("diagnostics", 256),
            ],
            Vec::new(),
        );

        assert_eq!(release_report.configuration, BuildMode::Release);
        assert!(!release_report.development_diagnostics_included);
        assert_eq!(dev_report.configuration, BuildMode::Development);
        assert!(dev_report.development_diagnostics_included);

        assert_eq!(
            BuildReport::from_json(&release_report.to_json())?,
            release_report
        );
        assert_eq!(BuildReport::from_json(&dev_report.to_json())?, dev_report);
        Ok(())
    }

    #[test]
    fn file_round_trip_writes_and_reads_report() -> Result<(), Box<dyn std::error::Error>> {
        let config = sample_config();
        let report = BuildReport::succeeded(
            &config,
            88,
            PathBuf::from("/tmp/unitas-out"),
            PathBuf::from("/tmp/unitas-out/SpaceRunner"),
            1024,
            vec![SizeCategoryEntry::new("scenes", 1024)],
            vec![SizeContributor::new(
                "Assets/Scenes/Boot.unity",
                "scenes",
                1024,
                "deadbeef",
            )],
        );

        let temp_path = std::env::temp_dir().join(format!(
            "unitas-build-report-test-{}.json",
            std::process::id()
        ));
        report.write_to_file(&temp_path)?;
        let loaded = BuildReport::read_from_file(&temp_path)?;
        let _ = std::fs::remove_file(&temp_path);
        assert_eq!(loaded, report);
        Ok(())
    }
}
