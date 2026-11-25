use std::error::Error;
use std::path::PathBuf;

use unitas_build::bundle::{
    ContentBuildMode, ContentBuildResult, ContentCatalog, ContentRejectionReason,
    RejectedContentBundle, CONTENT_CATALOG_FORMAT_VERSION,
};
use unitas_build::config::BuildConfiguration;
use unitas_build::report::{BuildReport, ContentBuildReport};
use unitas_build::targets::BuildTarget;
use unitas_core::compat::{
    CompatibilityReport, CompatibilityState, ContentIdentity, Finding, Tier,
};

type TestResult = Result<(), Box<dyn Error>>;

fn sample_config() -> BuildConfiguration {
    BuildConfiguration::new(
        vec![PathBuf::from("Assets/Scenes/Boot.unity")],
        "Unitas Studio",
        "SpaceRunner",
        "com.unitas.spacerunner",
        "1.4.2",
        BuildTarget::MACOS_ARM64,
    )
}

fn finding(feature_id: &str, tier: Tier, state: CompatibilityState, asset_path: &str) -> Finding {
    Finding {
        feature_id: feature_id.to_string(),
        tier,
        state,
        content: ContentIdentity {
            guid: Some(format!("guid-{asset_path}")),
            asset_path: Some(asset_path.to_string()),
            sub_asset_id: None,
            object_name: None,
        },
        behavior: "the assessed behavior".to_string(),
        relied_upon_unity_differences: vec![
            "the Unity difference the content relies on".to_string()
        ],
        observable_consequence: "the observable consequence".to_string(),
        conditions: vec!["the conditions under which the finding applies".to_string()],
        related_findings: Vec::new(),
    }
}

fn content_catalog() -> ContentCatalog {
    ContentCatalog {
        format_version: CONTENT_CATALOG_FORMAT_VERSION,
        unitas_version: "0.1.0".to_string(),
        player_compatibility_version: "1.4".to_string(),
        target: BuildTarget::MACOS_ARM64,
        bundles: Vec::new(),
        total_content_bytes: 0,
        catalog_hash: "cataloghash".to_string(),
    }
}

fn content_build_result(rejected: RejectedContentBundle) -> ContentBuildResult {
    ContentBuildResult {
        output_dir: PathBuf::from("/tmp/unitas-content"),
        catalog_path: PathBuf::from("/tmp/unitas-content/content.catalog"),
        catalog: content_catalog(),
        emitted_bundles: Vec::new(),
        unchanged_bundles: Vec::new(),
        rejected_bundles: vec![rejected],
    }
}

#[test]
fn build_report_surfaces_compatibility_findings_and_round_trips_json() -> TestResult {
    let compatibility = CompatibilityReport {
        produced_by_unitas_version: "0.1.0".to_string(),
        findings: vec![
            finding(
                "REN-006",
                Tier::Exact,
                CompatibilityState::Unsupported,
                "Assets/Render/HDRP.asset",
            ),
            finding(
                "CMP-002",
                Tier::Native,
                CompatibilityState::PartiallySupported,
                "Assets/Scenes/Boot.unity",
            ),
        ],
        packages: Vec::new(),
    };

    let report = BuildReport::succeeded(
        &sample_config(),
        1200,
        PathBuf::from("/tmp/unitas-builds/SpaceRunner"),
        PathBuf::from("/tmp/unitas-builds/SpaceRunner/SpaceRunner"),
        4096,
        Vec::new(),
        Vec::new(),
    )
    .with_compatibility_report(&compatibility);

    assert_eq!(report.compatibility_findings.len(), 2);
    let unsupported = &report.compatibility_findings[0];
    assert_eq!(unsupported.feature_id, "REN-006");
    assert_eq!(unsupported.tier, Tier::Exact);
    assert_eq!(unsupported.state, CompatibilityState::Unsupported);
    assert_eq!(
        unsupported.asset_path.as_deref(),
        Some("Assets/Render/HDRP.asset")
    );
    assert_eq!(
        unsupported.guid.as_deref(),
        Some("guid-Assets/Render/HDRP.asset")
    );
    assert_eq!(unsupported.behavior, "the assessed behavior");
    assert_eq!(
        unsupported.observable_consequence,
        "the observable consequence"
    );
    assert_eq!(
        unsupported.relied_upon_unity_differences,
        vec!["the Unity difference the content relies on".to_string()]
    );
    assert_eq!(
        unsupported.conditions,
        vec!["the conditions under which the finding applies".to_string()]
    );
    assert!(unsupported.related_findings.is_empty());
    let partial = &report.compatibility_findings[1];
    assert_eq!(partial.feature_id, "CMP-002");
    assert_eq!(partial.tier, Tier::Native);
    assert_eq!(partial.state, CompatibilityState::PartiallySupported);
    assert_eq!(
        partial.asset_path.as_deref(),
        Some("Assets/Scenes/Boot.unity")
    );

    let json = report.to_json();
    assert!(json.contains("\"feature_id\": \"REN-006\""));
    assert!(json.contains("\"state\": \"unsupported\""));
    assert!(json.contains("\"tier\": \"exact\""));
    let parsed = BuildReport::from_json(&json)?;
    assert_eq!(parsed, report);
    Ok(())
}

#[test]
fn content_build_report_surfaces_the_unity_built_content_exclusion() -> TestResult {
    let result = content_build_result(RejectedContentBundle {
        bundle_name: "level".to_string(),
        source_path: PathBuf::from("/tmp/unitas-content/level.bundle"),
        reason: ContentRejectionReason::UnityBuiltArchive {
            signature: "UnityFS".to_string(),
        },
    });

    let report = ContentBuildReport::from_build_result(&result, ContentBuildMode::FullCatalog, 40);

    assert_eq!(report.compatibility_findings.len(), 1);
    let exclusion = &report.compatibility_findings[0];
    assert_eq!(exclusion.feature_id, "BLD-005");
    assert_eq!(exclusion.tier, Tier::Native);
    assert_eq!(exclusion.state, CompatibilityState::Unsupported);
    assert_eq!(
        exclusion.asset_path.as_deref(),
        Some("/tmp/unitas-content/level.bundle")
    );
    assert!(exclusion
        .behavior
        .contains("AssetBundles and Addressables content built by Unity"));
    assert!(exclusion
        .observable_consequence
        .contains("content must be built by unitas"));

    let json = report.to_json();
    assert!(json.contains("\"feature_id\": \"BLD-005\""));
    let parsed = ContentBuildReport::from_json(&json)?;
    assert_eq!(parsed, report);
    Ok(())
}

#[test]
fn content_build_report_has_no_compatibility_findings_without_unity_built_content() -> TestResult {
    let result = content_build_result(RejectedContentBundle {
        bundle_name: "level".to_string(),
        source_path: PathBuf::from("/tmp/unitas-content/level.bundle"),
        reason: ContentRejectionReason::TargetMismatch {
            bundle_target: BuildTarget::LINUX_X86_64,
            expected_target: BuildTarget::MACOS_ARM64,
        },
    });

    let report = ContentBuildReport::from_build_result(&result, ContentBuildMode::FullCatalog, 40);

    assert!(
        report.compatibility_findings.is_empty(),
        "a bundle rejected for another reason is not a Unity-built content exclusion"
    );
    Ok(())
}
