use crate::asset_tools::open_project;
use crate::descriptor::Scope;
use crate::dispatch::{AbiError, DispatchTableBuilder, required_string, tool_descriptor};
use serde_json::{Value, json};
use std::path::Path;
use unitas_asset::project::{CompatibilityFinding, MigrationStatus, build_compatibility_report};
use unitas_core::compat::{
    CompatibilityState, RECOGNIZED_UNSUPPORTED_FAMILIES, SUPPORTED_PACKAGE_MATRIX, Tier,
    probe_owned_features,
};

pub fn register(builder: DispatchTableBuilder) -> Result<DispatchTableBuilder, AbiError> {
    let builder = builder.tool(
        tool_descriptor(
            "compat.report",
            "Report the project compatibility findings and the state of every owned feature",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(report),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "compat.package_matrix",
            "Report the supported package matrix and the recognized unsupported families",
            json!({}),
            &[],
            Scope::ReadProject,
        ),
        Box::new(package_matrix),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "compat.version_info",
            "Report the declared Unity version, the effective behavior version, and the migration state",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(version_info),
    )?;
    Ok(builder)
}

const fn tier_label(tier: Tier) -> &'static str {
    match tier {
        Tier::Exact => "exact",
        Tier::Semantic => "semantic",
        Tier::Familiar => "familiar",
        Tier::Native => "native",
    }
}

const fn state_label(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "supported",
        CompatibilityState::PartiallySupported => "partially-supported",
        CompatibilityState::Unsupported => "unsupported",
        CompatibilityState::Unknown => "unknown",
        CompatibilityState::PlatformLimited => "platform-limited",
        CompatibilityState::ExternallyUnavailable => "externally-unavailable",
    }
}

fn findings(items: &[CompatibilityFinding]) -> Vec<Value> {
    items
        .iter()
        .map(|finding| {
            json!({
                "identity": finding.identity,
                "category": finding.category,
                "catalogId": finding.catalog_id,
                "tier": finding.tier,
                "state": finding.state,
            })
        })
        .collect()
}

fn migration_label(migration: &MigrationStatus) -> String {
    match migration {
        MigrationStatus::Current => "current".to_string(),
        MigrationStatus::RequiresUpgrade(preview) => format!(
            "requires-upgrade:assets={}:settings={}:packages={}",
            preview.assets.len(),
            preview.settings.len(),
            preview.packages.len()
        ),
        MigrationStatus::Unsupported { reason, .. } => format!("unsupported:{reason}"),
    }
}

fn report(args: &Value) -> Result<Value, String> {
    let root = required_string(args, "projectPath")?;
    let (_project, open_report) = open_project(args)?;
    let compatibility = build_compatibility_report(Path::new(&root), &open_report);
    let owned: Vec<Value> = probe_owned_features()
        .iter()
        .map(|state| {
            json!({
                "featureId": state.feature_id,
                "tier": tier_label(state.tier),
                "state": state_label(state.state),
                "capability": state.capability,
                "gap": state.gap,
            })
        })
        .collect();
    Ok(json!({
        "unitasVersion": compatibility.unitas_version,
        "declaredUnityVersion": open_report.declared_version,
        "effectiveBehaviorVersion": open_report.effective_behavior_version,
        "assetFindings": findings(&compatibility.asset_findings),
        "packageFindings": findings(&compatibility.package_findings),
        "settingFindings": findings(&compatibility.setting_findings),
        "ownedFeatures": owned,
    }))
}

fn package_matrix(_args: &Value) -> Result<Value, String> {
    let packages: Vec<Value> = SUPPORTED_PACKAGE_MATRIX
        .iter()
        .map(|entry| {
            json!({
                "packageId": entry.package_id,
                "displayName": entry.display_name,
                "versionLine": entry.version_line,
                "tier": tier_label(entry.tier),
                "state": state_label(entry.state),
                "owningFeatureId": entry.owning_feature_id,
                "stateNotes": entry.state_notes,
            })
        })
        .collect();
    let unsupported: Vec<Value> = RECOGNIZED_UNSUPPORTED_FAMILIES
        .iter()
        .map(|entry| {
            json!({
                "family": entry.family,
                "packageId": entry.package_id,
                "recognizedForm": entry.recognized_form,
                "state": state_label(entry.state),
                "owningFeatureId": entry.owning_feature_id,
                "observableConsequence": entry.observable_consequence,
            })
        })
        .collect();
    Ok(json!({
        "packages": packages,
        "unsupportedFamilies": unsupported,
    }))
}

fn version_info(args: &Value) -> Result<Value, String> {
    let (_project, open_report) = open_project(args)?;
    Ok(json!({
        "unitasVersion": env!("CARGO_PKG_VERSION"),
        "declaredUnityVersion": open_report.declared_version,
        "effectiveBehaviorVersion": open_report.effective_behavior_version,
        "migration": migration_label(&open_report.migration),
        "recognizedPackages": open_report.recognized_packages,
        "recognizedSettings": open_report.recognized_settings,
    }))
}
