#[path = "compat_behavior.rs"]
mod compat_behavior;
#[path = "compat_canonical.rs"]
mod compat_canonical;
#[path = "compat_catalog.rs"]
mod compat_catalog;
#[path = "compat_familiar.rs"]
mod compat_familiar;
#[path = "compat_finding.rs"]
mod compat_finding;
#[path = "compat_matrix.rs"]
mod compat_matrix;
#[path = "compat_owned.rs"]
mod compat_owned;
#[path = "compat_shader.rs"]
mod compat_shader;

pub use compat_behavior::*;
pub use compat_canonical::*;
pub use compat_catalog::*;
pub use compat_familiar::*;
pub use compat_finding::*;
pub use compat_matrix::*;
pub use compat_owned::*;
pub use compat_shader::*;

use compat_catalog::owning_feature_tier;
use compat_owned::owned_capability_gap_suffix;

use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompatibilityState {
    Supported,
    PartiallySupported,
    Unsupported,
    Unknown,
    PlatformLimited,
    ExternallyUnavailable,
}

impl CompatibilityState {
    fn severity(self) -> u8 {
        match self {
            Self::Supported => 0,
            Self::PartiallySupported => 1,
            Self::PlatformLimited => 2,
            Self::Unknown => 3,
            Self::Unsupported => 4,
            Self::ExternallyUnavailable => 5,
        }
    }
}

impl PartialOrd for CompatibilityState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompatibilityState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.severity().cmp(&other.severity())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Exact,
    Semantic,
    Familiar,
    Native,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ContentIdentity {
    pub guid: Option<String>,
    pub asset_path: Option<String>,
    pub sub_asset_id: Option<String>,
    pub object_name: Option<String>,
}

impl ContentIdentity {
    fn names_content(&self) -> bool {
        self.guid.is_some()
            || self.asset_path.is_some()
            || self.sub_asset_id.is_some()
            || self.object_name.is_some()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Finding {
    pub feature_id: String,
    pub tier: Tier,
    pub state: CompatibilityState,
    pub content: ContentIdentity,
    pub behavior: String,
    pub relied_upon_unity_differences: Vec<String>,
    pub observable_consequence: String,
    pub conditions: Vec<String>,
    pub related_findings: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PackageSummary {
    pub package_name: String,
    pub state: CompatibilityState,
    pub feature_ids: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompatibilityReport {
    pub produced_by_unitas_version: String,
    pub findings: Vec<Finding>,
    pub packages: Vec<PackageSummary>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presence {
    Present,
    NotPresent,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssessedPackage {
    pub package_id: String,
    pub resolved_version: String,
    pub tier: Option<Tier>,
    pub state: CompatibilityState,
    pub owning_spec: Option<String>,
    pub owning_feature_id: Option<String>,
    pub notes: String,
}

pub fn assess_package(
    package_id: &str,
    resolved_version: &str,
    is_locally_modified_embedded: bool,
) -> AssessedPackage {
    assess_package_for_baseline(
        package_id,
        resolved_version,
        is_locally_modified_embedded,
        false,
    )
}

fn unknown_assessment(
    package_id: &str,
    resolved_version: &str,
    tier: Option<Tier>,
    owning_spec: Option<&str>,
    owning_feature_id: Option<&str>,
    notes: String,
) -> AssessedPackage {
    AssessedPackage {
        package_id: package_id.to_string(),
        resolved_version: resolved_version.to_string(),
        tier,
        state: CompatibilityState::Unknown,
        owning_spec: owning_spec.map(str::to_string),
        owning_feature_id: owning_feature_id.map(str::to_string),
        notes,
    }
}

fn unsupported_family_assessment(
    package_id: &str,
    resolved_version: &str,
    unsupported: &UnsupportedFamilyEntry,
) -> AssessedPackage {
    AssessedPackage {
        package_id: package_id.to_string(),
        resolved_version: resolved_version.to_string(),
        tier: None,
        state: unsupported.state,
        owning_spec: Some(unsupported.owning_spec.to_string()),
        owning_feature_id: Some(unsupported.owning_feature_id.to_string()),
        notes: unsupported.observable_consequence.to_string(),
    }
}

pub fn assess_package_for_baseline(
    package_id: &str,
    resolved_version: &str,
    is_locally_modified_embedded: bool,
    migrated_from_earlier_lts: bool,
) -> AssessedPackage {
    if is_locally_modified_embedded {
        return unknown_assessment(
            package_id,
            resolved_version,
            None,
            None,
            None,
            "locally modified embedded copy is assessed as third-party content".to_string(),
        );
    }
    if let Some(unsupported) = lookup_unsupported_family(package_id) {
        return unsupported_family_assessment(package_id, resolved_version, unsupported);
    }
    let Some(entry) = lookup_package_matrix(package_id) else {
        return unknown_assessment(
            package_id,
            resolved_version,
            None,
            None,
            None,
            "package is not listed in the Unity 2022.3 LTS supported package matrix".to_string(),
        );
    };
    assess_against_matrix_entry(
        package_id,
        resolved_version,
        entry,
        migrated_from_earlier_lts,
    )
}

fn assess_against_matrix_entry(
    package_id: &str,
    resolved_version: &str,
    entry: &PackageMatrixEntry,
    migrated_from_earlier_lts: bool,
) -> AssessedPackage {
    let matches_direct = version_matches_line(resolved_version, entry.version_line);
    let matches_migrated = !matches_direct
        && migrated_from_earlier_lts
        && version_migrates_from_earlier_lts_line(resolved_version, entry.version_line);
    if is_preview_or_experimental_version(resolved_version)
        || (!matches_direct && !matches_migrated)
    {
        return unknown_assessment(
            package_id,
            resolved_version,
            Some(entry.tier),
            Some(entry.owning_spec),
            Some(entry.owning_feature_id),
            format!(
                "version {resolved_version} is outside the Unity 2022.3 LTS matrix line ({})",
                entry.version_line
            ),
        );
    }
    let notes = if matches_migrated {
        format!(
            "migrated under SER-005 and assessed against Unity 2022.3 LTS matrix line ({}); {}",
            entry.version_line, entry.state_notes
        )
    } else {
        entry.state_notes.to_string()
    };
    AssessedPackage {
        package_id: package_id.to_string(),
        resolved_version: resolved_version.to_string(),
        tier: Some(entry.tier),
        state: entry.state,
        owning_spec: Some(entry.owning_spec.to_string()),
        owning_feature_id: Some(entry.owning_feature_id.to_string()),
        notes,
    }
}

fn version_migrates_from_earlier_lts_line(version: &str, line: &str) -> bool {
    let Some(target_prefix) = line.strip_suffix(".x") else {
        return false;
    };
    let target_major = match target_prefix
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
    {
        Some(v) => v,
        None => return false,
    };
    let resolved_major = match version
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
    {
        Some(v) => v,
        None => return false,
    };
    resolved_major > 0 && resolved_major < target_major
}

fn is_preview_or_experimental_version(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    lower.contains("-pre")
        || lower.contains("-exp")
        || lower.contains("preview")
        || lower.contains("experimental")
        || lower.contains("-alpha")
        || lower.contains("-beta")
}

fn version_matches_line(version: &str, line: &str) -> bool {
    if line == "Released or verified" {
        return !version.trim().is_empty();
    }
    if let Some(prefix) = line.strip_suffix(".x") {
        let expected_with_dot = format!("{prefix}.");
        return version == prefix || version.starts_with(&expected_with_dot);
    }
    version == line
}

pub fn presence(finding: &Finding) -> Presence {
    if finding.content.names_content() {
        Presence::Present
    } else {
        Presence::NotPresent
    }
}

pub fn is_present(finding: &Finding) -> bool {
    presence(finding) == Presence::Present
}

pub fn absence_finding(feature_id: &str) -> Finding {
    Finding {
        feature_id: feature_id.to_string(),
        tier: owning_feature_tier(feature_id, None),
        state: CompatibilityState::Unknown,
        content: ContentIdentity {
            guid: None,
            asset_path: None,
            sub_asset_id: None,
            object_name: None,
        },
        behavior: "the feature is not present in this project".to_string(),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: "none; the project does not use the feature".to_string(),
        conditions: vec!["the feature is absent from the project".to_string()],
        related_findings: Vec::new(),
    }
}

pub fn least_supported_state(states: &[CompatibilityState]) -> CompatibilityState {
    match states.iter().copied().max() {
        Some(state) => state,
        None => CompatibilityState::Unknown,
    }
}

pub fn summarize_package(package_name: &str, findings: &[Finding]) -> PackageSummary {
    let indices: Vec<usize> = (0..findings.len()).collect();
    summarize(package_name, findings, &indices)
}

pub fn package_of(finding: &Finding) -> &str {
    package_key(&finding.feature_id)
}

pub fn build_report(unitas_version: &str, findings: Vec<Finding>) -> CompatibilityReport {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, finding) in findings.iter().enumerate() {
        let key = package_key(&finding.feature_id);
        match groups.iter_mut().find(|(name, _)| name == key) {
            Some((_, indices)) => indices.push(index),
            None => groups.push((key.to_string(), vec![index])),
        }
    }
    let packages = groups
        .iter()
        .map(|(name, indices)| summarize(name, &findings, indices))
        .collect();
    CompatibilityReport {
        produced_by_unitas_version: unitas_version.to_string(),
        findings,
        packages,
    }
}

pub fn reassess(report: &CompatibilityReport, updated: Vec<Finding>) -> CompatibilityReport {
    let mut findings = report.findings.clone();
    for update in updated {
        let replaced = findings.iter().position(|finding| {
            finding.feature_id == update.feature_id && finding.content == update.content
        });
        match replaced {
            Some(index) => findings[index] = update,
            None => findings.push(update),
        }
    }
    build_report(&report.produced_by_unitas_version, findings)
}

fn summarize(package_name: &str, findings: &[Finding], indices: &[usize]) -> PackageSummary {
    let mut feature_ids: Vec<String> = Vec::new();
    let mut states: Vec<CompatibilityState> = Vec::new();
    for index in indices {
        let finding = &findings[*index];
        if !feature_ids.contains(&finding.feature_id) {
            feature_ids.push(finding.feature_id.clone());
        }
        states.push(match presence(finding) {
            Presence::Present => finding.state,
            Presence::NotPresent => CompatibilityState::Unknown,
        });
    }
    PackageSummary {
        package_name: package_name.to_string(),
        state: least_supported_state(&states),
        feature_ids,
    }
}

fn package_key(feature_id: &str) -> &str {
    match feature_id.split_once('-') {
        Some((family, _)) => family,
        None => feature_id,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PackageSource {
    Registry,
    ScopedRegistry,
    Git,
    LocalPath,
    Embedded,
    Tarball,
    BuiltInModule,
    AssetStoreFolder,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectPackageEntry {
    pub package_id: String,
    pub version: String,
    pub source: PackageSource,
    pub is_locally_modified_embedded: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectAssetEntry {
    pub family: String,
    pub content: ContentIdentity,
    pub owning_feature_id: Option<String>,
    pub tier: Option<Tier>,
    pub state: Option<CompatibilityState>,
    pub observable_consequence: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectSettingEntry {
    pub setting_name: String,
    pub family: String,
    pub owning_feature_id: Option<String>,
    pub tier: Option<Tier>,
    pub state: Option<CompatibilityState>,
    pub observable_consequence: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectContent {
    pub unitas_version: String,
    pub declared_unity_version: Option<String>,
    pub packages: Vec<ProjectPackageEntry>,
    pub assets: Vec<ProjectAssetEntry>,
    pub settings: Vec<ProjectSettingEntry>,
    pub additional_findings: Vec<Finding>,
    pub track_absent_catalog_features: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectCompatibilityAssessment {
    pub report: CompatibilityReport,
    pub effective_baseline_unity_version: String,
    pub migrated_from_earlier_lts: bool,
    pub overall_state: CompatibilityState,
    pub assessed_packages: Vec<AssessedPackage>,
    pub present_supported_features: Vec<String>,
    pub partially_supported_features: Vec<String>,
    pub unsupported_features: Vec<String>,
    pub unknown_features: Vec<String>,
    pub absent_features: Vec<String>,
    pub playable_scenes: Vec<String>,
    pub owned_feature_states: Vec<OwnedFeatureState>,
}

pub fn assess_project(content: &ProjectContent) -> ProjectCompatibilityAssessment {
    let migrated_from_earlier_lts = content
        .declared_unity_version
        .as_deref()
        .is_some_and(|version| version.starts_with("2020.3") || version.starts_with("2021.3"));

    let mut assessed_packages: Vec<AssessedPackage> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    let mut playable_scenes: Vec<String> = Vec::new();

    assess_project_packages(
        content,
        migrated_from_earlier_lts,
        &mut assessed_packages,
        &mut findings,
    );
    assess_project_assets(content, &mut findings, &mut playable_scenes);
    assess_project_settings(content, &mut findings);
    assess_project_additional_findings(content, &mut findings, &mut playable_scenes);

    let mut feature_states = summarize_project_findings(&findings);
    apply_tracked_absent_features(content, &mut findings, &mut feature_states.absent);

    let overall_state = least_supported_state(&feature_states.present_states);
    let report = build_report(&content.unitas_version, findings);

    ProjectCompatibilityAssessment {
        report,
        effective_baseline_unity_version: "2022.3 LTS".to_string(),
        migrated_from_earlier_lts,
        overall_state,
        assessed_packages,
        present_supported_features: feature_states.present_supported,
        partially_supported_features: feature_states.partially_supported,
        unsupported_features: feature_states.unsupported,
        unknown_features: feature_states.unknown,
        absent_features: feature_states.absent,
        playable_scenes,
        owned_feature_states: probe_owned_features(),
    }
}

fn assess_project_packages(
    content: &ProjectContent,
    migrated_from_earlier_lts: bool,
    assessed_packages: &mut Vec<AssessedPackage>,
    findings: &mut Vec<Finding>,
) {
    for pkg in &content.packages {
        let (assessed, finding) = project_package_finding(pkg, migrated_from_earlier_lts);
        assessed_packages.push(assessed);
        findings.push(finding);
    }
}

fn assess_project_assets(
    content: &ProjectContent,
    findings: &mut Vec<Finding>,
    playable_scenes: &mut Vec<String>,
) {
    for asset in &content.assets {
        findings.push(project_asset_finding(asset, playable_scenes));
    }
}

fn assess_project_settings(content: &ProjectContent, findings: &mut Vec<Finding>) {
    for setting in &content.settings {
        findings.push(project_setting_finding(setting));
    }
}

fn assess_project_additional_findings(
    content: &ProjectContent,
    findings: &mut Vec<Finding>,
    playable_scenes: &mut Vec<String>,
) {
    for extra in &content.additional_findings {
        findings.push(project_additional_finding(extra, playable_scenes));
    }
}

struct ProjectFeatureStates {
    present_supported: Vec<String>,
    partially_supported: Vec<String>,
    unsupported: Vec<String>,
    unknown: Vec<String>,
    absent: Vec<String>,
    present_states: Vec<CompatibilityState>,
}

fn summarize_project_findings(findings: &[Finding]) -> ProjectFeatureStates {
    let mut present_supported_features: Vec<String> = Vec::new();
    let mut partially_supported_features: Vec<String> = Vec::new();
    let mut unsupported_features: Vec<String> = Vec::new();
    let mut unknown_features: Vec<String> = Vec::new();
    let mut absent_features: Vec<String> = Vec::new();
    let mut present_states: Vec<CompatibilityState> = Vec::new();
    for finding in findings {
        if !is_present(finding) {
            push_unique(&mut absent_features, &finding.feature_id);
            continue;
        }
        present_states.push(finding.state);
        match finding.state {
            CompatibilityState::Supported => {
                push_unique(&mut present_supported_features, &finding.feature_id);
            }
            CompatibilityState::PartiallySupported => {
                push_unique(&mut partially_supported_features, &finding.feature_id);
            }
            CompatibilityState::Unsupported => {
                push_unique(&mut unsupported_features, &finding.feature_id);
            }
            CompatibilityState::Unknown
            | CompatibilityState::PlatformLimited
            | CompatibilityState::ExternallyUnavailable => {
                push_unique(&mut unknown_features, &finding.feature_id);
            }
        }
    }

    ProjectFeatureStates {
        present_supported: present_supported_features,
        partially_supported: partially_supported_features,
        unsupported: unsupported_features,
        unknown: unknown_features,
        absent: absent_features,
        present_states,
    }
}

fn apply_tracked_absent_features(
    content: &ProjectContent,
    findings: &mut Vec<Finding>,
    absent_features: &mut Vec<String>,
) {
    for tracked in &content.track_absent_catalog_features {
        let is_any_present = findings
            .iter()
            .any(|f| is_present(f) && &f.feature_id == tracked);
        if !is_any_present {
            push_unique(absent_features, tracked);
            let already_has_absence = findings
                .iter()
                .any(|f| !is_present(f) && &f.feature_id == tracked);
            if !already_has_absence {
                findings.push(absence_finding(tracked));
            }
        }
    }
}

fn familiar_differences(feature_id: &str, tier: Tier) -> Vec<String> {
    if tier == Tier::Familiar {
        lookup_familiar_behavior_differences(feature_id).map_or_else(Vec::new, |entry| {
            entry
                .differences
                .iter()
                .map(|diff| (*diff).to_string())
                .collect()
        })
    } else {
        Vec::new()
    }
}

fn consequence_with_capability_gap(
    consequence: String,
    state: CompatibilityState,
    capability: Option<&'static OwnedFeatureCapability>,
) -> String {
    if state == CompatibilityState::PartiallySupported {
        format!(
            "{}{}",
            consequence,
            owned_capability_gap_suffix(&consequence, capability)
        )
    } else {
        consequence
    }
}

fn built_in_module_assessment(pkg: &ProjectPackageEntry) -> AssessedPackage {
    AssessedPackage {
        package_id: pkg.package_id.clone(),
        resolved_version: pkg.version.clone(),
        tier: Some(Tier::Semantic),
        state: CompatibilityState::Supported,
        owning_spec: Some("scripting-runtime.md".to_string()),
        owning_feature_id: Some("RUN-002".to_string()),
        notes: "built-in engine module".to_string(),
    }
}

fn project_package_finding(
    pkg: &ProjectPackageEntry,
    migrated_from_earlier_lts: bool,
) -> (AssessedPackage, Finding) {
    let assessed = if pkg.source == PackageSource::BuiltInModule
        || pkg.package_id.starts_with("com.unity.modules.")
    {
        built_in_module_assessment(pkg)
    } else {
        assess_package_for_baseline(
            &pkg.package_id,
            &pkg.version,
            pkg.is_locally_modified_embedded,
            migrated_from_earlier_lts,
        )
    };
    let feature_id = assessed
        .owning_feature_id
        .clone()
        .unwrap_or_else(|| "PRJ-006".to_string());
    let capability = lookup_owned_feature_capability(&feature_id);
    let tier = owning_feature_tier(&feature_id, assessed.tier);
    let relied_upon_unity_differences = familiar_differences(&feature_id, tier);
    let finding = Finding {
        feature_id,
        tier,
        state: assessed.state,
        content: ContentIdentity {
            guid: None,
            asset_path: Some(format!("Packages/{}", pkg.package_id)),
            sub_asset_id: None,
            object_name: Some(format!("{}@{}", pkg.package_id, pkg.version)),
        },
        behavior: format!("package {} ({})", pkg.package_id, pkg.version),
        relied_upon_unity_differences,
        observable_consequence: consequence_with_capability_gap(
            assessed.notes.clone(),
            assessed.state,
            capability,
        ),
        conditions: vec!["package resolved in project manifest".to_string()],
        related_findings: Vec::new(),
    };
    (assessed, finding)
}

fn default_asset_state(
    asset: &ProjectAssetEntry,
    unsupported_entry: Option<&UnsupportedFamilyEntry>,
    capability: Option<&'static OwnedFeatureCapability>,
) -> CompatibilityState {
    unsupported_entry.map_or_else(
        || match (asset.owning_feature_id.as_deref(), capability) {
            (Some(_), Some(capability)) => capability.state(),
            _ => CompatibilityState::Unknown,
        },
        |entry| entry.state,
    )
}

fn default_asset_consequence(
    state: CompatibilityState,
    unsupported_entry: Option<&UnsupportedFamilyEntry>,
) -> String {
    unsupported_entry.map_or_else(
        || match state {
            CompatibilityState::Supported => "content is editable and playable".to_string(),
            CompatibilityState::PartiallySupported => {
                "content is usable with stated exclusions".to_string()
            }
            CompatibilityState::Unsupported => {
                "recognized content is unsupported; unrelated content remains usable".to_string()
            }
            CompatibilityState::Unknown
            | CompatibilityState::PlatformLimited
            | CompatibilityState::ExternallyUnavailable => {
                "content compatibility requires target or behavioral classification".to_string()
            }
        },
        |entry| entry.observable_consequence.to_string(),
    )
}

fn collect_playable_scene(asset: &ProjectAssetEntry, playable_scenes: &mut Vec<String>) {
    if let Some(path) = &asset.content.asset_path {
        if path.ends_with(".unity") && !playable_scenes.contains(path) {
            playable_scenes.push(path.clone());
        }
    }
}

fn project_asset_finding(asset: &ProjectAssetEntry, playable_scenes: &mut Vec<String>) -> Finding {
    let unsupported_entry = lookup_unsupported_family(&asset.family);
    let feature_id = asset.owning_feature_id.clone().unwrap_or_else(|| {
        unsupported_entry.map_or_else(
            || "PRJ-004".to_string(),
            |entry| entry.owning_feature_id.to_string(),
        )
    });
    let capability = lookup_owned_feature_capability(&feature_id);
    let state = asset
        .state
        .unwrap_or_else(|| default_asset_state(asset, unsupported_entry, capability));
    let tier = owning_feature_tier(&feature_id, asset.tier);
    let consequence = asset
        .observable_consequence
        .clone()
        .unwrap_or_else(|| default_asset_consequence(state, unsupported_entry));
    let consequence = consequence_with_capability_gap(consequence, state, capability);
    if state == CompatibilityState::Supported {
        collect_playable_scene(asset, playable_scenes);
    }
    let relied_upon_unity_differences = familiar_differences(&feature_id, tier);
    Finding {
        feature_id,
        tier,
        state,
        content: asset.content.clone(),
        behavior: format!("asset family {}", asset.family),
        relied_upon_unity_differences,
        observable_consequence: consequence,
        conditions: vec!["asset is present in project".to_string()],
        related_findings: Vec::new(),
    }
}

fn default_setting_state(
    unsupported_entry: Option<&UnsupportedFamilyEntry>,
    capability: Option<&'static OwnedFeatureCapability>,
) -> CompatibilityState {
    unsupported_entry.map_or_else(
        || capability.map_or(CompatibilityState::Unknown, |capability| capability.state()),
        |entry| entry.state,
    )
}

fn default_setting_consequence(
    state: CompatibilityState,
    unsupported_entry: Option<&UnsupportedFamilyEntry>,
) -> String {
    unsupported_entry.map_or_else(
        || match state {
            CompatibilityState::Unknown
            | CompatibilityState::PlatformLimited
            | CompatibilityState::ExternallyUnavailable => {
                "project setting compatibility requires target or behavioral classification"
                    .to_string()
            }
            _ => "project setting is represented and honored".to_string(),
        },
        |entry| entry.observable_consequence.to_string(),
    )
}

fn project_setting_finding(setting: &ProjectSettingEntry) -> Finding {
    let unsupported_entry = lookup_unsupported_family(&setting.family);
    let feature_id = setting.owning_feature_id.clone().unwrap_or_else(|| {
        unsupported_entry.map_or_else(
            || "PRJ-007".to_string(),
            |entry| entry.owning_feature_id.to_string(),
        )
    });
    let capability = lookup_owned_feature_capability(&feature_id);
    let state = setting
        .state
        .unwrap_or_else(|| default_setting_state(unsupported_entry, capability));
    let tier = owning_feature_tier(&feature_id, setting.tier);
    let consequence = setting
        .observable_consequence
        .clone()
        .unwrap_or_else(|| default_setting_consequence(state, unsupported_entry));
    let consequence = consequence_with_capability_gap(consequence, state, capability);
    Finding {
        feature_id,
        tier,
        state,
        content: ContentIdentity {
            guid: None,
            asset_path: Some(setting.setting_name.clone()),
            sub_asset_id: None,
            object_name: Some(setting.family.clone()),
        },
        behavior: format!("project setting {}", setting.family),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: consequence,
        conditions: vec!["setting is configured in ProjectSettings".to_string()],
        related_findings: Vec::new(),
    }
}

fn project_additional_finding(extra: &Finding, playable_scenes: &mut Vec<String>) -> Finding {
    if is_present(extra) && extra.state == CompatibilityState::Supported {
        if let Some(path) = &extra.content.asset_path {
            if path.ends_with(".unity") && !playable_scenes.contains(path) {
                playable_scenes.push(path.clone());
            }
        }
    }
    let mut enriched = extra.clone();
    enriched.tier = owning_feature_tier(&enriched.feature_id, Some(enriched.tier));
    if enriched.tier == Tier::Familiar && enriched.relied_upon_unity_differences.is_empty() {
        if let Some(entry) = lookup_familiar_behavior_differences(&enriched.feature_id) {
            enriched.relied_upon_unity_differences = entry
                .differences
                .iter()
                .map(|diff| (*diff).to_string())
                .collect();
        }
    }
    enriched
}

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|existing| existing == item) {
        list.push(item.to_string());
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompatibilitySummary {
    pub overall_state: CompatibilityState,
    pub total_findings: usize,
    pub present_findings: usize,
    pub absent_findings: usize,
    pub supported_count: usize,
    pub partially_supported_count: usize,
    pub platform_limited_count: usize,
    pub unknown_count: usize,
    pub unsupported_count: usize,
    pub externally_unavailable_count: usize,
}

pub fn summarize_report(report: &CompatibilityReport) -> CompatibilitySummary {
    let mut present_findings = 0;
    let mut absent_findings = 0;
    let mut supported_count = 0;
    let mut partially_supported_count = 0;
    let mut platform_limited_count = 0;
    let mut unknown_count = 0;
    let mut unsupported_count = 0;
    let mut externally_unavailable_count = 0;
    let mut effective_states: Vec<CompatibilityState> = Vec::new();

    for finding in &report.findings {
        if is_present(finding) {
            present_findings += 1;
            effective_states.push(finding.state);
            match finding.state {
                CompatibilityState::Supported => supported_count += 1,
                CompatibilityState::PartiallySupported => partially_supported_count += 1,
                CompatibilityState::PlatformLimited => platform_limited_count += 1,
                CompatibilityState::Unknown => unknown_count += 1,
                CompatibilityState::Unsupported => unsupported_count += 1,
                CompatibilityState::ExternallyUnavailable => externally_unavailable_count += 1,
            }
        } else {
            absent_findings += 1;
            unknown_count += 1;
            effective_states.push(CompatibilityState::Unknown);
        }
    }

    CompatibilitySummary {
        overall_state: least_supported_state(&effective_states),
        total_findings: report.findings.len(),
        present_findings,
        absent_findings,
        supported_count,
        partially_supported_count,
        platform_limited_count,
        unknown_count,
        unsupported_count,
        externally_unavailable_count,
    }
}

impl CompatibilityReport {
    pub fn summary(&self) -> CompatibilitySummary {
        summarize_report(self)
    }

    pub fn submit_finding(
        &self,
        input: FindingInput,
    ) -> Result<CompatibilityReport, FindingSubmissionError> {
        submit_finding(self, input)
    }

    pub fn submit_findings<I>(&self, inputs: I) -> Result<CompatibilityReport, FindingRejection>
    where
        I: IntoIterator<Item = FindingInput>,
    {
        submit_findings(self, inputs)
    }

    pub fn merge_findings(
        &self,
        external_findings: &[Finding],
    ) -> Result<CompatibilityReport, FindingRejection> {
        merge_findings(self, external_findings)
    }

    pub fn render_canonical(&self) -> String {
        render_canonical(self)
    }

    pub fn parse_canonical(text: &str) -> Result<CompatibilityReport, CanonicalReportParseError> {
        parse_canonical(text)
    }

    pub fn to_canonical_text(&self) -> String {
        render_canonical(self)
    }

    pub fn from_canonical_text(
        text: &str,
    ) -> Result<CompatibilityReport, CanonicalReportParseError> {
        parse_canonical(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    const PRESENTATION_PLAYER_OWNED_IDS: [&str; 23] = [
        "REN-001", "REN-002", "REN-003", "REN-004", "REN-005", "REN-006", "REN-007", "UI-001",
        "UI-002", "TXT-001", "INP-001", "INP-002", "AUD-001", "MED-001", "BLD-001", "BLD-004",
        "BLD-005", "PLT-001", "LIC-001", "PRD-001", "XR-001", "NET-001", "DIA-003",
    ];

    #[test]
    fn presentation_player_catalog_ids_are_present_in_owned_capabilities_and_reported() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from);
        assert!(workspace_root.is_some());
        let Some(root) = workspace_root else {
            return;
        };

        let probed = probe_owned_features();
        let assessment = assess_project(&ProjectContent {
            unitas_version: "0.1.0".to_string(),
            declared_unity_version: Some("2022.3.20f1".to_string()),
            packages: Vec::new(),
            assets: Vec::new(),
            settings: Vec::new(),
            additional_findings: Vec::new(),
            track_absent_catalog_features: Vec::new(),
        });

        for feature_id in PRESENTATION_PLAYER_OWNED_IDS {
            let Some(capability) = assert_catalog_capability_and_state(feature_id) else {
                continue;
            };
            let derived_state = capability.state();
            assert_capability_parts_are_evidenced(feature_id, capability, &root);

            assert!(
                probed
                    .iter()
                    .any(|entry| entry.feature_id == feature_id && entry.state == derived_state),
                "probe_owned_features missing {feature_id}"
            );
            assert!(
                assessment
                    .owned_feature_states
                    .iter()
                    .any(|entry| entry.feature_id == feature_id && entry.state == derived_state),
                "assess_project.owned_feature_states missing {feature_id}"
            );
        }
    }

    fn assert_catalog_capability_and_state(
        feature_id: &str,
    ) -> Option<&'static OwnedFeatureCapability> {
        let catalog_entry = lookup_catalog_feature(feature_id);
        assert!(
            catalog_entry.is_some(),
            "catalog entry missing for {feature_id}"
        );
        let catalog = catalog_entry?;

        let capability_entry = lookup_owned_feature_capability(feature_id);
        assert!(
            capability_entry.is_some(),
            "owned feature capability missing for {feature_id}"
        );
        let capability = capability_entry?;

        assert_eq!(capability.owning_spec, catalog.owning_spec);
        assert!(!capability.capability.trim().is_empty());
        assert!(!capability.parts.is_empty());

        let derived_state = capability.state();
        assert!(
            matches!(
                derived_state,
                CompatibilityState::PartiallySupported | CompatibilityState::Unsupported
            ),
            "feature {feature_id} derived unexpected state {derived_state:?}"
        );
        assert!(capability.gap().is_some());

        if catalog.tier == Tier::Familiar {
            assert!(
                lookup_familiar_behavior_differences(feature_id).is_some(),
                "Familiar owned feature {feature_id} must list differences"
            );
        }
        Some(capability)
    }

    fn assert_capability_parts_are_evidenced(
        feature_id: &str,
        capability: &OwnedFeatureCapability,
        root: &Path,
    ) {
        for part in capability.parts {
            assert!(!part.behavior.trim().is_empty());
            assert!(
                root.join(part.location).is_file(),
                "location {} does not exist for {feature_id}",
                part.location
            );
            if part.evidence.is_empty() {
                assert!(
                    part.missing.is_some_and(|m| !m.trim().is_empty()),
                    "missing text required for undemonstrated part in {feature_id}"
                );
            } else {
                for item in part.evidence {
                    assert_evidence_item_exists(feature_id, item, root);
                }
            }
        }
    }

    fn assert_evidence_item_exists(feature_id: &str, item: &str, root: &Path) {
        let split = item.split_once("::");
        assert!(
            split.is_some(),
            "invalid evidence format {item} in {feature_id}"
        );
        let Some((file_rel, test_name)) = split else {
            return;
        };
        let file_path = root.join(file_rel);
        assert!(
            file_path.is_file(),
            "evidence file {file_rel} does not exist for {feature_id}"
        );
        let text = fs::read_to_string(&file_path).unwrap_or_default();
        let found = if file_rel.ends_with(".rs") {
            text.contains(&format!("fn {test_name}("))
        } else if file_rel.ends_with(".cs") {
            text.contains(&format!("void {test_name}("))
                || text.contains(&format!("Task {test_name}("))
        } else if file_rel.ends_with(".json") {
            text.contains(&format!("\"{test_name}\""))
        } else {
            false
        };
        assert!(
            found,
            "evidence test {test_name} not found in {file_rel} for {feature_id}"
        );
    }
}
