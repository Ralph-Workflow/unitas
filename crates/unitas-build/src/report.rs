use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use unitas_asset::project::CompatibilityFinding as ProjectCompatibilityFinding;
use unitas_core::compat::{
    CompatibilityReport, CompatibilityState, Finding, Tier, RECOGNIZED_UNSUPPORTED_FAMILIES,
};

use crate::bundle::{
    BundleManifest, ContentBuildMode, ContentBuildResult, ContentCatalogEntry,
    ContentRejectionReason, RejectedContentBundle,
};
use crate::config::{BuildConfiguration, BuildMode, BuildValidationError};
use crate::licensing::{licensing_record, LicensingRecord};
use crate::targets::{BuildTarget, TargetArch};
use crate::toolchain::{TargetAvailability, UnavailableToolchain};

#[path = "report_json.rs"]
pub(crate) mod report_json;

use report_json::{
    get_field, get_optional_contributors, get_optional_desktop_packaging, get_optional_licensing,
    get_optional_path, get_optional_startup_time, get_optional_string,
    get_optional_target_availability, get_required_bool, get_required_category_sizes,
    get_required_compatibility_findings, get_required_contributors, get_required_diagnostics,
    get_required_str, get_required_string_list, get_required_u64, parse_build_mode,
    parse_build_target, parse_target_arch, push_escaped_json_str, push_indent, push_kv_bool,
    push_kv_category_sizes, push_kv_compatibility_findings, push_kv_contributors,
    push_kv_desktop_packaging, push_kv_diagnostics, push_kv_licensing, push_kv_opt_path,
    push_kv_opt_str, push_kv_startup_time, push_kv_str, push_kv_string_array,
    push_kv_target_availability, push_kv_u64, push_trailing, JsonParser,
};
pub(crate) use report_json::{parse_json_document, JsonValue};

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

    /// Maps a project-level compatibility entry, produced by opening the
    /// Unity project the built scenes belong to, into a build-report finding.
    /// Entries whose tier or state the report schema does not recognize are
    /// skipped.
    pub fn from_project_entry(entry: &ProjectCompatibilityFinding) -> Option<Self> {
        let tier = parse_tier(&entry.tier).ok()?;
        let state = parse_state(&entry.state).ok()?;
        Some(Self {
            feature_id: entry.catalog_id.clone(),
            tier,
            state,
            guid: None,
            asset_path: (entry.category == "asset").then(|| entry.identity.clone()),
            sub_asset_id: None,
            object_name: None,
            behavior: format!(
                "project {} '{}' present at build time",
                entry.category, entry.identity
            ),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: format!(
                "the build report records the {} '{}' compatibility state '{}'",
                entry.category, entry.identity, entry.state
            ),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        })
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

pub(super) fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Exact => "exact",
        Tier::Semantic => "semantic",
        Tier::Familiar => "familiar",
        Tier::Native => "native",
    }
}

pub(super) fn parse_tier(raw: &str) -> Result<Tier, ReportParseError> {
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

pub(super) fn state_name(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "supported",
        CompatibilityState::PartiallySupported => "partially_supported",
        CompatibilityState::Unsupported => "unsupported",
        CompatibilityState::Unknown => "unknown",
        CompatibilityState::PlatformLimited => "platform_limited",
        CompatibilityState::ExternallyUnavailable => "externally_unavailable",
    }
}

pub(super) fn parse_state(raw: &str) -> Result<CompatibilityState, ReportParseError> {
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

pub const STARTUP_BASIS: &str =
    "time from process launch until the player answered its first control request";

/// Whether a build report carries a measured player startup time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStatus {
    Measured,
    Unavailable,
}

impl StartupStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            StartupStatus::Measured => "measured",
            StartupStatus::Unavailable => "unavailable",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "measured" => Some(StartupStatus::Measured),
            "unavailable" => Some(StartupStatus::Unavailable),
            _ => None,
        }
    }
}

/// The player startup time a build report records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupMeasurement {
    pub status: StartupStatus,
    pub duration_ms: Option<u64>,
    pub basis: String,
    pub reason: Option<String>,
}

impl StartupMeasurement {
    pub fn measured(duration_ms: u64, basis: impl Into<String>) -> Self {
        Self {
            status: StartupStatus::Measured,
            duration_ms: Some(duration_ms),
            basis: basis.into(),
            reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>, basis: impl Into<String>) -> Self {
        Self {
            status: StartupStatus::Unavailable,
            duration_ms: None,
            basis: basis.into(),
            reason: Some(reason.into()),
        }
    }

    pub fn is_measured(&self) -> bool {
        self.status == StartupStatus::Measured
    }
}

pub const DESKTOP_PACKAGE_STATE_PACKAGED: &str = "packaged";
pub const DESKTOP_PACKAGE_STATE_EXTERNALLY_UNAVAILABLE: &str =
    crate::toolchain::TOOLCHAIN_EXTERNALLY_UNAVAILABLE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPackageOutcome {
    pub target: BuildTarget,
    pub state: String,
    pub app_path: Option<PathBuf>,
    pub launch_artifact: Option<PathBuf>,
    pub build_report_path: PathBuf,
    pub launched: bool,
    pub unavailable: Option<UnavailableToolchain>,
}

impl DesktopPackageOutcome {
    pub fn is_packaged(&self) -> bool {
        self.state == DESKTOP_PACKAGE_STATE_PACKAGED && self.unavailable.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildReport {
    pub target: BuildTarget,
    pub architecture: TargetArch,
    pub configuration: BuildMode,
    pub company_name: String,
    pub product_name: String,
    pub application_identifier: String,
    pub version: String,
    pub build_identifier: String,
    pub assets: Vec<String>,
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
    pub managed_payload: Vec<SizeContributor>,
    pub development_diagnostics_included: bool,
    pub unsupported_behaviors: Vec<String>,
    pub compatibility_findings: Vec<CompatibilityFinding>,
    pub licensing: Option<LicensingRecord>,
    pub startup_time: StartupMeasurement,
    pub target_availability: Vec<TargetAvailability>,
    pub desktop_packaging: Vec<DesktopPackageOutcome>,
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
        let mut report = Self {
            target: config.target,
            architecture: config.target.arch(),
            configuration: config.mode(),
            company_name: config.company_name.clone(),
            product_name: config.product_name.clone(),
            application_identifier: config.application_identifier.clone(),
            version: config.version.clone(),
            build_identifier: String::new(),
            assets: Vec::new(),
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
            managed_payload: Vec::new(),
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: Some(licensing_record(&config.application_identifier)),
            startup_time: StartupMeasurement::unavailable(
                "the build has not launched the player it produced",
                STARTUP_BASIS,
            ),
            target_availability: Vec::new(),
            desktop_packaging: Vec::new(),
        };
        if let Some(output_dir) = report.output_location.clone() {
            report.stamp_from_staged_manifest(&output_dir);
        }
        report
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
            build_identifier: String::new(),
            assets: Vec::new(),
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
            managed_payload: Vec::new(),
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: None,
            startup_time: StartupMeasurement::unavailable(
                "the build did not produce a player to launch",
                STARTUP_BASIS,
            ),
            target_availability: Vec::new(),
            desktop_packaging: Vec::new(),
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
            build_identifier: String::new(),
            assets: Vec::new(),
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
            managed_payload: Vec::new(),
            development_diagnostics_included: config.include_diagnostics(),
            unsupported_behaviors: Vec::new(),
            compatibility_findings: Vec::new(),
            licensing: None,
            startup_time: StartupMeasurement::unavailable(
                "the build was cancelled before it produced a player to launch",
                STARTUP_BASIS,
            ),
            target_availability: Vec::new(),
            desktop_packaging: Vec::new(),
        }
    }

    /// Stamps the content-derived build identifier from the staged bundle
    /// manifest onto the report.
    pub fn with_build_identifier(mut self, build_identifier: impl Into<String>) -> Self {
        self.build_identifier = build_identifier.into();
        self
    }

    /// Records the staged asset paths included in the bundle.
    pub fn with_assets(mut self, assets: Vec<String>) -> Self {
        self.assets = assets;
        self
    }

    /// Fills `build_identifier` and `assets` from the staged bundle manifest
    /// in the build output directory, so the report shares the bundle's
    /// content identity; explicit stamps made via `with_build_identifier` or
    /// `with_assets` win.
    fn stamp_from_staged_manifest(&mut self, output_location: &Path) {
        let Ok(manifest) = BundleManifest::load_from_dir(output_location) else {
            return;
        };
        if self.build_identifier.is_empty() {
            self.build_identifier = manifest.build_identifier.clone();
        }
        if self.assets.is_empty() {
            self.assets = manifest
                .entries
                .iter()
                .map(|entry| entry.staged_path.clone())
                .collect();
        }
    }

    /// Records the staged `Managed/` payload with per-file sizes.
    pub fn with_managed_payload(mut self, payload: Vec<SizeContributor>) -> Self {
        self.managed_payload = payload;
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<BuildDiagnostic>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn with_unsupported_behaviors(mut self, behaviors: Vec<String>) -> Self {
        self.unsupported_behaviors = behaviors;
        self
    }

    pub fn with_compatibility_findings(mut self, findings: Vec<CompatibilityFinding>) -> Self {
        self.compatibility_findings = findings;
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

    pub fn with_startup_time(mut self, startup_time: StartupMeasurement) -> Self {
        self.startup_time = startup_time;
        self
    }

    pub fn with_target_availability(mut self, availability: Vec<TargetAvailability>) -> Self {
        self.target_availability = availability;
        self
    }

    pub fn with_desktop_packaging(mut self, outcomes: Vec<DesktopPackageOutcome>) -> Self {
        self.desktop_packaging = outcomes;
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
        push_kv_str(
            &mut out,
            2,
            "build_identifier",
            &self.build_identifier,
            true,
        );
        push_kv_string_array(&mut out, 2, "included_scenes", &self.included_scenes, true);
        push_kv_string_array(&mut out, 2, "assets", &self.assets, true);
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
        push_kv_contributors(&mut out, 2, "managed_payload", &self.managed_payload, true);
        push_kv_startup_time(&mut out, 2, "startup_time", &self.startup_time, true);
        push_kv_target_availability(
            &mut out,
            2,
            "target_availability",
            &self.target_availability,
            true,
        );
        push_kv_desktop_packaging(
            &mut out,
            2,
            "desktop_packaging",
            &self.desktop_packaging,
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
        let build_identifier = match obj.get("build_identifier") {
            None | Some(JsonValue::Null) => String::new(),
            Some(value) => value.as_str("build_identifier")?.to_string(),
        };
        let included_scenes = get_required_string_list(obj, "included_scenes")?;
        let assets = match obj.get("assets") {
            None | Some(JsonValue::Null) => Vec::new(),
            Some(value) => value
                .as_array("assets")?
                .iter()
                .map(|item| item.as_str("assets").map(str::to_string))
                .collect::<Result<Vec<String>, ReportParseError>>()?,
        };

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
        let managed_payload = get_optional_contributors(obj, "managed_payload")?;
        let development_diagnostics_included =
            get_required_bool(obj, "development_diagnostics_included")?;
        let unsupported_behaviors = get_required_string_list(obj, "unsupported_behaviors")?;
        let compatibility_findings =
            get_required_compatibility_findings(obj, "compatibility_findings")?;
        let licensing = get_optional_licensing(obj)?;
        let startup_time = get_optional_startup_time(obj)?;
        let target_availability = get_optional_target_availability(obj)?;
        let desktop_packaging = get_optional_desktop_packaging(obj)?;

        Ok(Self {
            target,
            architecture,
            configuration,
            company_name,
            product_name,
            application_identifier,
            version,
            build_identifier,
            assets,
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
            managed_payload,
            development_diagnostics_included,
            unsupported_behaviors,
            compatibility_findings,
            licensing,
            startup_time,
            target_availability,
            desktop_packaging,
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
pub struct ContentBundleReportEntry {
    pub bundle_name: String,
    pub group_name: String,
    pub build_path: String,
    pub load_path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub dependencies: Vec<String>,
}

impl ContentBundleReportEntry {
    pub fn from_catalog_entry(entry: &ContentCatalogEntry) -> Self {
        Self {
            bundle_name: entry.bundle_name.clone(),
            group_name: entry.group_name.clone(),
            build_path: entry.build_path.clone(),
            load_path: entry.load_path.clone(),
            content_hash: entry.content_hash.clone(),
            size_bytes: entry.size_bytes,
            dependencies: entry.dependencies.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentBuildReport {
    pub mode: ContentBuildMode,
    pub catalog_id: String,
    pub profile_name: Option<String>,
    pub unitas_version: String,
    pub player_compatibility_version: String,
    pub compatible_player_versions: Vec<String>,
    pub target: BuildTarget,
    pub architecture: TargetArch,
    pub outcome: BuildOutcome,
    pub elapsed_ms: u64,
    pub catalog_path: Option<PathBuf>,
    pub catalog_hash: String,
    pub bundles: Vec<ContentBundleReportEntry>,
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
        let bundles = result
            .catalog
            .bundles
            .iter()
            .map(ContentBundleReportEntry::from_catalog_entry)
            .collect();
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
            catalog_id: result.catalog.catalog_id.clone(),
            profile_name: result.catalog.profile_name.clone(),
            unitas_version: result.catalog.unitas_version.clone(),
            player_compatibility_version: result.catalog.player_compatibility_version.clone(),
            compatible_player_versions: result.catalog.compatible_player_versions.clone(),
            target: result.catalog.target,
            architecture: result.catalog.target.arch(),
            outcome,
            elapsed_ms,
            catalog_path: Some(result.catalog_path.clone()),
            catalog_hash: result.catalog.catalog_hash.clone(),
            bundles,
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
        push_kv_str(&mut out, 2, "catalog_id", &self.catalog_id, true);
        push_kv_opt_str(
            &mut out,
            2,
            "profile_name",
            self.profile_name.as_deref(),
            true,
        );
        push_kv_str(&mut out, 2, "unitas_version", &self.unitas_version, true);
        push_kv_str(
            &mut out,
            2,
            "player_compatibility_version",
            &self.player_compatibility_version,
            true,
        );
        push_kv_string_array(
            &mut out,
            2,
            "compatible_player_versions",
            &self.compatible_player_versions,
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
        push_kv_content_bundles(&mut out, 2, "bundles", &self.bundles, true);
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
        let catalog_id = get_required_str(obj, "catalog_id")?.to_string();
        let profile_name = get_optional_string(obj, "profile_name")?;
        let unitas_version = get_required_str(obj, "unitas_version")?.to_string();
        let player_compatibility_version =
            get_required_str(obj, "player_compatibility_version")?.to_string();
        let compatible_player_versions =
            get_required_string_list(obj, "compatible_player_versions")?;
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
        let bundles = get_required_content_bundles(obj, "bundles")?;
        let emitted_bundles = get_required_string_list(obj, "emitted_bundles")?;
        let unchanged_bundles = get_required_string_list(obj, "unchanged_bundles")?;
        let rejected_bundles = get_required_rejected_bundles(obj, "rejected_bundles")?;
        let errors = get_required_diagnostics(obj, "errors")?;
        let compatibility_findings =
            get_required_compatibility_findings(obj, "compatibility_findings")?;

        Ok(Self {
            mode,
            catalog_id,
            profile_name,
            unitas_version,
            player_compatibility_version,
            compatible_player_versions,
            target,
            architecture,
            outcome,
            elapsed_ms,
            catalog_path,
            catalog_hash,
            bundles,
            emitted_bundles,
            unchanged_bundles,
            rejected_bundles,
            errors,
            compatibility_findings,
        })
    }
}

fn push_kv_content_bundles(
    out: &mut String,
    indent: usize,
    key: &str,
    items: &[ContentBundleReportEntry],
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
        push_kv_str(out, indent + 4, "group_name", &item.group_name, true);
        push_kv_str(out, indent + 4, "build_path", &item.build_path, true);
        push_kv_str(out, indent + 4, "load_path", &item.load_path, true);
        push_kv_str(out, indent + 4, "content_hash", &item.content_hash, true);
        push_kv_u64(out, indent + 4, "size_bytes", item.size_bytes, true);
        push_kv_string_array(out, indent + 4, "dependencies", &item.dependencies, false);
        push_indent(out, indent + 2);
        out.push('}');
        push_trailing(out, index + 1 < items.len());
    }
    push_indent(out, indent);
    out.push(']');
    push_trailing(out, trailing_comma);
}

fn get_required_content_bundles(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<ContentBundleReportEntry>, ReportParseError> {
    let arr = get_field(obj, key)?.as_array(key)?;
    let mut list = Vec::with_capacity(arr.len());
    for item in arr {
        let entry_obj = item.as_object(key)?;
        let bundle_name = get_required_str(entry_obj, "bundle_name")?.to_string();
        let group_name = get_required_str(entry_obj, "group_name")?.to_string();
        let build_path = get_required_str(entry_obj, "build_path")?.to_string();
        let load_path = get_required_str(entry_obj, "load_path")?.to_string();
        let content_hash = get_required_str(entry_obj, "content_hash")?.to_string();
        let size_bytes = get_required_u64(entry_obj, "size_bytes")?;
        let dependencies = get_required_string_list(entry_obj, "dependencies")?;
        list.push(ContentBundleReportEntry {
            bundle_name,
            group_name,
            build_path,
            load_path,
            content_hash,
            size_bytes,
            dependencies,
        });
    }
    Ok(list)
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
    use crate::bundle::stage_bundle_for_host;

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
            "\"assets\"",
            "\"build_identifier\"",
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
    fn succeeded_report_stamps_build_identifier_and_assets_from_the_staged_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("the crate directory has no repository root")?;
        let output_dir = root.join("target").join(format!(
            "unitas-build-report-tests/stamped_report_{}",
            std::process::id()
        ));
        if output_dir.exists() {
            std::fs::remove_dir_all(&output_dir)?;
        }
        std::fs::create_dir_all(&output_dir)?;
        let scene = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scene_alpha.unity");
        let config = BuildConfiguration::new(
            vec![scene],
            "Unitas",
            "StampedGame",
            "com.unitas.stamped",
            "1.0.0",
            BuildTarget::MACOS_ARM64,
        );
        let staged = stage_bundle_for_host(&config, &output_dir, Some(BuildTarget::MACOS_ARM64))?;

        let report = BuildReport::succeeded(
            &config,
            25,
            staged.output_dir.clone(),
            staged.launch_artifact_path.clone(),
            staged.manifest.total_content_bytes,
            Vec::new(),
            Vec::new(),
        );

        assert!(!report.build_identifier.is_empty());
        assert_eq!(report.build_identifier, staged.manifest.build_identifier);
        let expected_assets: Vec<String> = staged
            .manifest
            .entries
            .iter()
            .map(|entry| entry.staged_path.clone())
            .collect();
        assert!(!expected_assets.is_empty());
        assert_eq!(report.assets, expected_assets);

        let json = report.to_json();
        for required_key in [
            "\"included_scenes\"",
            "\"assets\"",
            "\"total_size_bytes\"",
            "\"build_identifier\"",
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
