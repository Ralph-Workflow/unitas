use super::assess_package;
use super::compat_catalog::lookup_catalog_feature;
use super::compat_finding::sanitize_content_identity;
use super::familiar_differences;
use super::{AssessedPackage, CompatibilityState, ContentIdentity, Finding, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackageBehaviorClassification {
    NamedSupported,
    NamedExcluded(&'static str),
    NeitherNamedNorExcluded,
}

fn package_behavior_context(assessed_pkg: &AssessedPackage) -> (String, Tier, String) {
    let feature_id = assessed_pkg
        .owning_feature_id
        .clone()
        .unwrap_or_else(|| "PRJ-006".to_string());
    let tier = assessed_pkg
        .tier
        .or_else(|| lookup_catalog_feature(&feature_id).map(|entry| entry.tier))
        .unwrap_or(Tier::Native);
    let owning_spec = assessed_pkg
        .owning_spec
        .clone()
        .or_else(|| lookup_catalog_feature(&feature_id).map(|entry| entry.owning_spec.to_string()))
        .unwrap_or_else(|| "compatibility-contract.md".to_string());
    (feature_id, tier, owning_spec)
}

fn behavior_outcome(
    package_id: &str,
    resolved_version: &str,
    owning_spec: &str,
    feature_id: &str,
    classification: PackageBehaviorClassification,
) -> (CompatibilityState, String) {
    match classification {
        PackageBehaviorClassification::NamedSupported => (
            CompatibilityState::Supported,
            format!(
                "package {package_id} ({resolved_version}) behavior is named and supported by {owning_spec} ({feature_id})"
            ),
        ),
        PackageBehaviorClassification::NamedExcluded(exclusion) => (
            CompatibilityState::PartiallySupported,
            format!(
                "package {package_id} ({resolved_version}) behavior is excluded by {owning_spec} ({feature_id}): {exclusion}"
            ),
        ),
        PackageBehaviorClassification::NeitherNamedNorExcluded => (
            CompatibilityState::Unknown,
            format!(
                "behavior in Supported package {package_id} ({resolved_version}) is neither named nor excluded by {owning_spec} ({feature_id}) and is Unknown until assessed"
            ),
        ),
    }
}

pub fn assess_package_behavior(
    package_id: &str,
    resolved_version: &str,
    content: ContentIdentity,
    behavior_description: &str,
    classification: PackageBehaviorClassification,
) -> Finding {
    let assessed_pkg = assess_package(package_id, resolved_version, false);
    let (feature_id, tier, owning_spec) = package_behavior_context(&assessed_pkg);

    let (state, consequence) = if assessed_pkg.state != CompatibilityState::Supported
        && assessed_pkg.state != CompatibilityState::PartiallySupported
    {
        (assessed_pkg.state, assessed_pkg.notes)
    } else {
        behavior_outcome(
            package_id,
            resolved_version,
            &owning_spec,
            &feature_id,
            classification,
        )
    };

    let relied_upon_unity_differences = familiar_differences(&feature_id, tier);

    Finding {
        feature_id,
        tier,
        state,
        content: sanitize_content_identity(&content),
        behavior: behavior_description.trim().to_string(),
        relied_upon_unity_differences,
        observable_consequence: consequence,
        conditions: vec![format!("package {package_id}@{resolved_version}")],
        related_findings: Vec::new(),
    }
}
