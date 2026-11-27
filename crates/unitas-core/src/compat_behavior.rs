use super::assess_package;
use super::compat_catalog::lookup_catalog_feature;
use super::compat_familiar::lookup_familiar_behavior_differences;
use super::compat_finding::sanitize_content_identity;
use super::{CompatibilityState, ContentIdentity, Finding, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackageBehaviorClassification {
    NamedSupported,
    NamedExcluded(&'static str),
    NeitherNamedNorExcluded,
}

pub fn assess_package_behavior(
    package_id: &str,
    resolved_version: &str,
    content: ContentIdentity,
    behavior_description: &str,
    classification: PackageBehaviorClassification,
) -> Finding {
    let assessed_pkg = assess_package(package_id, resolved_version, false);
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

    let (state, consequence) = if assessed_pkg.state != CompatibilityState::Supported
        && assessed_pkg.state != CompatibilityState::PartiallySupported
    {
        (assessed_pkg.state, assessed_pkg.notes)
    } else {
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
    };

    let relied_upon_unity_differences = if tier == Tier::Familiar {
        lookup_familiar_behavior_differences(&feature_id).map_or_else(Vec::new, |entry| {
            entry
                .differences
                .iter()
                .map(|diff| (*diff).to_string())
                .collect()
        })
    } else {
        Vec::new()
    };

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
