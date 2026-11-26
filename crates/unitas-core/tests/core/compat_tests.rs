use unitas_core::compat::{
    absence_finding, build_report, is_present, least_supported_state, lookup_catalog_feature,
    merge_findings, package_of, parse_canonical, presence, reassess, render_canonical,
    submit_finding, submit_findings, summarize_package, CompatibilityReport, CompatibilityState,
    ContentIdentity, Finding, FindingInput, FindingRejection, FindingSubmissionError,
    PackageSummary, Presence, Tier,
};

fn content(path: &str) -> ContentIdentity {
    ContentIdentity {
        guid: Some(format!("guid-{path}")),
        asset_path: Some(path.to_string()),
        sub_asset_id: None,
        object_name: None,
    }
}

fn finding(feature_id: &str, tier: Tier, state: CompatibilityState, path: &str) -> Finding {
    Finding {
        feature_id: feature_id.to_string(),
        tier,
        state,
        content: content(path),
        behavior: "the assessed behavior".to_string(),
        relied_upon_unity_differences: vec![
            "the Unity difference the content relies on".to_string()
        ],
        observable_consequence: "the observable consequence".to_string(),
        conditions: vec!["the conditions under which the finding applies".to_string()],
        related_findings: Vec::new(),
    }
}

fn summary(package_name: &str, state: CompatibilityState, feature_ids: &[&str]) -> PackageSummary {
    PackageSummary {
        package_name: package_name.to_string(),
        state,
        feature_ids: feature_ids.iter().map(|id| (*id).to_string()).collect(),
    }
}

fn version_in_line(line: &str) -> String {
    if line == "Released or verified" {
        return "1.0.0".to_string();
    }
    match line.strip_suffix(".x") {
        Some(prefix) => format!("{prefix}.0"),
        None => line.to_string(),
    }
}

#[test]
fn compat_least_supported_state_without_assessments_is_unknown() {
    assert_eq!(least_supported_state(&[]), CompatibilityState::Unknown);
}

#[test]
fn compat_least_supported_state_reads_supported_only_when_every_state_is_supported() {
    assert_eq!(
        least_supported_state(&[CompatibilityState::Supported]),
        CompatibilityState::Supported
    );
    assert_eq!(
        least_supported_state(&[CompatibilityState::Supported, CompatibilityState::Supported]),
        CompatibilityState::Supported
    );
    assert_eq!(
        least_supported_state(&[CompatibilityState::Supported, CompatibilityState::Unknown]),
        CompatibilityState::Unknown
    );
}

#[test]
fn compat_least_supported_state_takes_the_worst_state() {
    assert_eq!(
        least_supported_state(&[
            CompatibilityState::Supported,
            CompatibilityState::PartiallySupported
        ]),
        CompatibilityState::PartiallySupported
    );
    assert_eq!(
        least_supported_state(&[
            CompatibilityState::Supported,
            CompatibilityState::PlatformLimited
        ]),
        CompatibilityState::PlatformLimited
    );
    assert_eq!(
        least_supported_state(&[
            CompatibilityState::PartiallySupported,
            CompatibilityState::Unknown
        ]),
        CompatibilityState::Unknown
    );
    assert_eq!(
        least_supported_state(&[CompatibilityState::Unknown, CompatibilityState::Unsupported]),
        CompatibilityState::Unsupported
    );
    assert_eq!(
        least_supported_state(&[
            CompatibilityState::Unsupported,
            CompatibilityState::ExternallyUnavailable
        ]),
        CompatibilityState::ExternallyUnavailable
    );
}

#[test]
fn compat_state_ordering_is_total_and_matches_the_documented_order() {
    let mut states = [
        CompatibilityState::Supported,
        CompatibilityState::PartiallySupported,
        CompatibilityState::Unsupported,
        CompatibilityState::Unknown,
        CompatibilityState::PlatformLimited,
        CompatibilityState::ExternallyUnavailable,
    ];
    states.sort();

    assert_eq!(
        states,
        [
            CompatibilityState::Supported,
            CompatibilityState::PartiallySupported,
            CompatibilityState::PlatformLimited,
            CompatibilityState::Unknown,
            CompatibilityState::Unsupported,
            CompatibilityState::ExternallyUnavailable,
        ]
    );
    assert!(CompatibilityState::Supported < CompatibilityState::PartiallySupported);
    assert!(CompatibilityState::PartiallySupported < CompatibilityState::PlatformLimited);
    assert!(CompatibilityState::PlatformLimited < CompatibilityState::Unknown);
    assert!(CompatibilityState::Unknown < CompatibilityState::Unsupported);
    assert!(CompatibilityState::Unsupported < CompatibilityState::ExternallyUnavailable);
}

#[test]
fn compat_summarize_package_reports_least_supported_state_and_feature_ids() {
    let findings = vec![
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/texture.png",
        ),
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::PartiallySupported,
            "Assets/model.asset",
        ),
        finding(
            "REN-006",
            Tier::Native,
            CompatibilityState::Supported,
            "Assets/material.mat",
        ),
    ];

    let summarized = summarize_package("PRJ", &findings);

    assert_eq!(summarized.package_name, "PRJ");
    assert_eq!(summarized.state, CompatibilityState::PartiallySupported);
    assert_eq!(
        summarized.feature_ids,
        vec!["PRJ-004".to_string(), "REN-006".to_string()]
    );
}

#[test]
fn compat_summarize_package_keeps_one_entry_per_feature_id() {
    let findings = vec![
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/first.png",
        ),
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/second.png",
        ),
    ];

    let summarized = summarize_package("PRJ", &findings);

    assert_eq!(summarized.feature_ids, vec!["PRJ-004".to_string()]);
    assert_eq!(summarized.state, CompatibilityState::Supported);
}

#[test]
fn compat_build_report_records_the_version_and_derives_package_summaries() {
    let findings = vec![
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/texture.png",
        ),
        finding(
            "REN-006",
            Tier::Native,
            CompatibilityState::Unsupported,
            "Assets/shader.shader",
        ),
    ];

    let report = build_report("0.1.0", findings);

    assert_eq!(report.produced_by_unitas_version, "0.1.0");
    assert_eq!(report.findings.len(), 2);
    assert_eq!(
        report.packages,
        vec![
            summary("PRJ", CompatibilityState::Supported, &["PRJ-004"]),
            summary("REN", CompatibilityState::Unsupported, &["REN-006"]),
        ]
    );
    assert_eq!(package_of(&report.findings[0]), "PRJ");
    assert_eq!(package_of(&report.findings[1]), "REN");
}

#[test]
fn cmp_002_story_5_reassess_updates_only_the_affected_finding() {
    let assessed = finding(
        "PRJ-004",
        Tier::Semantic,
        CompatibilityState::Supported,
        "Assets/texture.png",
    );
    let untouched = finding(
        "REN-006",
        Tier::Native,
        CompatibilityState::Supported,
        "Assets/shader.shader",
    );
    let report = build_report("0.1.0", vec![assessed.clone(), untouched.clone()]);

    let mut updated = assessed.clone();
    updated.state = CompatibilityState::Unsupported;
    let reassessed = reassess(&report, vec![updated]);

    assert_eq!(reassessed.produced_by_unitas_version, "0.1.0");
    assert_eq!(reassessed.findings.len(), 2);
    assert_eq!(
        reassessed.findings[0].state,
        CompatibilityState::Unsupported
    );
    assert_eq!(
        reassessed.findings[1], untouched,
        "CMP-002 Story 5: reassessment must not change findings the project change did not affect"
    );
    assert_eq!(
        reassessed.packages,
        vec![
            summary("PRJ", CompatibilityState::Unsupported, &["PRJ-004"]),
            summary("REN", CompatibilityState::Supported, &["REN-006"]),
        ]
    );
    assert_eq!(
        reassessed.summary().overall_state,
        CompatibilityState::Unsupported,
        "CMP-002 Story 5: the reassessed report must state the new project state"
    );
}

#[test]
fn cmp_002_story_5_reassess_appends_a_finding_for_newly_present_content() {
    let assessed = finding(
        "PRJ-004",
        Tier::Semantic,
        CompatibilityState::Supported,
        "Assets/first.png",
    );
    let report = build_report("0.1.0", vec![assessed]);

    let unseen = finding(
        "PRJ-004",
        Tier::Semantic,
        CompatibilityState::PartiallySupported,
        "Assets/second.png",
    );
    let reassessed = reassess(&report, vec![unseen]);

    assert_eq!(reassessed.findings.len(), 2);
    assert_eq!(
        reassessed.findings[1].content.asset_path.as_deref(),
        Some("Assets/second.png")
    );
    assert_eq!(
        reassessed.packages,
        vec![summary(
            "PRJ",
            CompatibilityState::PartiallySupported,
            &["PRJ-004"]
        )]
    );
}

#[test]
fn compat_absence_is_distinguishable_and_never_counted_as_supported() {
    let absent = absence_finding("CMP-002");
    let present = finding(
        "PRJ-004",
        Tier::Semantic,
        CompatibilityState::Supported,
        "Assets/texture.png",
    );

    assert_eq!(absent.state, CompatibilityState::Unknown);
    assert_eq!(presence(&absent), Presence::NotPresent);
    assert!(!is_present(&absent));
    assert!(absent.content.guid.is_none());
    assert!(absent.content.asset_path.is_none());

    assert_eq!(presence(&present), Presence::Present);
    assert!(is_present(&present));

    let summarized = summarize_package("PRJ", &[present, absent]);

    assert_eq!(summarized.state, CompatibilityState::Unknown);
    assert_eq!(
        summarized.feature_ids,
        vec!["PRJ-004".to_string(), "CMP-002".to_string()]
    );
}

#[test]
fn compat_report_keeps_the_findings_it_was_built_from() {
    let findings = vec![
        finding(
            "PRJ-004",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/texture.png",
        ),
        finding(
            "PRJ-005",
            Tier::Exact,
            CompatibilityState::PartiallySupported,
            "Assets/model.asset",
        ),
    ];
    let report: CompatibilityReport = build_report("0.2.0", findings.clone());

    assert_eq!(report.findings, findings);
    assert_eq!(
        report.packages,
        vec![summary(
            "PRJ",
            CompatibilityState::PartiallySupported,
            &["PRJ-004", "PRJ-005"]
        )]
    );
}

#[test]
fn cmp_002_story_6_supported_package_matrix_reports_urp_and_input_system_supported_and_unlisted_or_preview_unknown(
) {
    use unitas_core::compat::{
        assess_package, lookup_package_matrix, lookup_unsupported_family,
        RECOGNIZED_UNSUPPORTED_FAMILIES, SUPPORTED_PACKAGE_MATRIX,
    };

    assert_eq!(SUPPORTED_PACKAGE_MATRIX.len(), 29);
    assert_eq!(RECOGNIZED_UNSUPPORTED_FAMILIES.len(), 11);

    for entry in SUPPORTED_PACKAGE_MATRIX {
        assert!(
            !entry.version_line.trim().is_empty(),
            "CMP-002 Story 6: matrix entry '{}' must carry a version line",
            entry.package_id
        );
        assert!(
            !entry.owning_spec.trim().is_empty(),
            "CMP-002 Story 6: matrix entry '{}' must carry an owning specification",
            entry.package_id
        );
        assert!(
            !entry.state_notes.trim().is_empty(),
            "CMP-002 Story 6: matrix entry '{}' must state its matrix state",
            entry.package_id
        );
        let version = version_in_line(entry.version_line);
        let assessed = assess_package(entry.package_id, &version, false);
        assert_eq!(
            assessed.resolved_version, version,
            "CMP-002 Story 6: the report must state the resolved version of '{}'",
            entry.package_id
        );
        assert_eq!(
            assessed.tier,
            Some(entry.tier),
            "CMP-002 Story 6: the report must state the tier of '{}'",
            entry.package_id
        );
        assert_eq!(
            assessed.state, entry.state,
            "CMP-002 Story 6: the report must state the matrix state of '{}'",
            entry.package_id
        );
        assert_eq!(
            assessed.owning_spec.as_deref(),
            Some(entry.owning_spec),
            "CMP-002 Story 6: the report must state the owning specification of '{}'",
            entry.package_id
        );
        assert_eq!(
            assessed.owning_feature_id.as_deref(),
            Some(entry.owning_feature_id),
            "CMP-002 Story 6: the report must state the owning feature of '{}'",
            entry.package_id
        );
    }

    let urp = assess_package("com.unity.render-pipelines.universal", "14.0.10", false);
    assert_eq!(urp.state, CompatibilityState::Supported);
    assert_eq!(urp.tier, Some(Tier::Native));
    assert_eq!(
        urp.owning_spec.as_deref(),
        Some("rendering-and-animation.md")
    );
    assert_eq!(urp.owning_feature_id.as_deref(), Some("REN-006"));

    let input = assess_package("com.unity.inputsystem", "1.7.0", false);
    assert_eq!(input.state, CompatibilityState::Supported);
    assert_eq!(input.tier, Some(Tier::Familiar));
    assert_eq!(input.owning_spec.as_deref(), Some("ui-audio-input.md"));
    assert_eq!(input.owning_feature_id.as_deref(), Some("INP-002"));

    let unlisted = assess_package("com.vendor.custom-toolkit", "2.1.0", false);
    assert_eq!(
        unlisted.state,
        CompatibilityState::Unknown,
        "CMP-002 Story 6: a package the matrix does not list must be reported Unknown"
    );
    assert!(unlisted.tier.is_none());

    let preview = assess_package("com.unity.inputsystem", "1.8.0-pre.1", false);
    assert_eq!(
        preview.state,
        CompatibilityState::Unknown,
        "CMP-002 Story 6: a preview version of a listed package must be reported Unknown"
    );

    let modified_embedded = assess_package("com.unity.ugui", "1.0.0", true);
    assert_eq!(modified_embedded.state, CompatibilityState::Unknown);

    let hdrp = assess_package(
        "com.unity.render-pipelines.high-definition",
        "14.0.10",
        false,
    );
    assert_eq!(hdrp.state, CompatibilityState::Unsupported);
    assert_eq!(hdrp.owning_feature_id.as_deref(), Some("REN-006"));

    let visual_scripting = assess_package("com.unity.visualscripting", "1.9.1", false);
    assert_eq!(visual_scripting.state, CompatibilityState::Unsupported);
    assert_eq!(
        visual_scripting.owning_feature_id.as_deref(),
        Some("RUN-002")
    );

    let rider = assess_package("com.unity.ide.rider", "3.0.28", false);
    assert_eq!(rider.state, CompatibilityState::PartiallySupported);
    assert_eq!(rider.owning_feature_id.as_deref(), Some("DIA-002"));

    let collab = assess_package("com.unity.collab-proxy", "2.3.1", false);
    assert_eq!(collab.state, CompatibilityState::PartiallySupported);
    assert_eq!(collab.owning_feature_id.as_deref(), Some("EDT-007"));

    assert!(lookup_package_matrix("com.unity.shadergraph").is_some());
    assert!(lookup_unsupported_family("Enlighten realtime global illumination").is_some());
    assert!(
        lookup_unsupported_family("Unity-built AssetBundles and Addressables content").is_some()
    );
}

#[test]
fn cmp_002_story_6_project_report_reports_urp_and_input_system_with_tiers_and_owning_specs_and_unknown_for_unlisted_or_preview(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        ProjectSettingEntry,
    };

    let content = ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![
            ProjectPackageEntry {
                package_id: "com.unity.render-pipelines.universal".to_string(),
                version: "14.0.10".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.unity.inputsystem".to_string(),
                version: "1.7.0".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.vendor.unlisted".to_string(),
                version: "2.0.0".to_string(),
                source: PackageSource::ScopedRegistry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.unity.addressables".to_string(),
                version: "1.22.0-pre.1".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
        ],
        assets: vec![ProjectAssetEntry {
            family: "URP Scene".to_string(),
            content: ContentIdentity {
                guid: Some("guid-main-scene".to_string()),
                asset_path: Some("Assets/Scenes/Main.unity".to_string()),
                sub_asset_id: None,
                object_name: Some("Main".to_string()),
            },
            owning_feature_id: Some("SER-002".to_string()),
            tier: Some(Tier::Exact),
            state: Some(CompatibilityState::Supported),
            observable_consequence: None,
        }],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/GraphicsSettings.asset".to_string(),
            family: "Graphics".to_string(),
            owning_feature_id: Some("REN-006".to_string()),
            tier: Some(Tier::Native),
            state: Some(CompatibilityState::Supported),
            observable_consequence: None,
        }],
        additional_findings: Vec::new(),
        track_absent_catalog_features: vec!["PHY-004".to_string(), "ENV-001".to_string()],
    };

    let assessment = assess_project(&content);

    assert_eq!(assessment.report.produced_by_unitas_version, "0.1.0");
    assert_eq!(
        assessment.effective_baseline_unity_version,
        "2022.3 LTS".to_string()
    );
    assert!(!assessment.migrated_from_earlier_lts);

    let urp = assessment
        .assessed_packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.render-pipelines.universal");
    assert!(urp.is_some());
    if let Some(urp) = urp {
        assert_eq!(urp.state, CompatibilityState::Supported);
        assert_eq!(urp.tier, Some(Tier::Native));
        assert_eq!(
            urp.owning_spec.as_deref(),
            Some("rendering-and-animation.md")
        );
        assert_eq!(urp.owning_feature_id.as_deref(), Some("REN-006"));
    }

    let input = assessment
        .assessed_packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.inputsystem");
    assert!(input.is_some());
    if let Some(input) = input {
        assert_eq!(input.state, CompatibilityState::Supported);
        assert_eq!(input.tier, Some(Tier::Familiar));
        assert_eq!(input.owning_spec.as_deref(), Some("ui-audio-input.md"));
        assert_eq!(input.owning_feature_id.as_deref(), Some("INP-002"));
    }

    let unlisted = assessment
        .assessed_packages
        .iter()
        .find(|pkg| pkg.package_id == "com.vendor.unlisted");
    assert!(unlisted.is_some());
    if let Some(unlisted) = unlisted {
        assert_eq!(unlisted.state, CompatibilityState::Unknown);
    }

    let preview = assessment
        .assessed_packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.addressables");
    assert!(preview.is_some());
    if let Some(preview) = preview {
        assert_eq!(preview.state, CompatibilityState::Unknown);
    }

    assert!(assessment.absent_features.contains(&"PHY-004".to_string()));
    assert!(assessment.absent_features.contains(&"ENV-001".to_string()));
    assert!(!assessment
        .present_supported_features
        .contains(&"PHY-004".to_string()));
    assert!(assessment
        .present_supported_features
        .contains(&"REN-006".to_string()));
    assert!(assessment
        .present_supported_features
        .contains(&"INP-002".to_string()));
}

#[test]
fn cmp_002_story_5_reassessment_after_a_project_change_updates_only_affected_findings_and_makes_the_project_state_clear(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
    };

    let urp_scene = ProjectAssetEntry {
        family: "URP Scene".to_string(),
        content: ContentIdentity {
            guid: Some("guid-playable-scene".to_string()),
            asset_path: Some("Assets/Scenes/Playable.unity".to_string()),
            sub_asset_id: None,
            object_name: Some("Playable".to_string()),
        },
        owning_feature_id: Some("REN-006".to_string()),
        tier: Some(Tier::Native),
        state: Some(CompatibilityState::Supported),
        observable_consequence: None,
    };
    let hdrp_asset = ProjectAssetEntry {
        family: "High Definition Render Pipeline".to_string(),
        content: ContentIdentity {
            guid: Some("guid-hdrp-volume".to_string()),
            asset_path: Some("Assets/HDRP/SkyVolume.asset".to_string()),
            sub_asset_id: None,
            object_name: Some("SkyVolume".to_string()),
        },
        owning_feature_id: None,
        tier: None,
        state: None,
        observable_consequence: None,
    };
    let vendor_asset = ProjectAssetEntry {
        family: "Unassessed vendor content".to_string(),
        content: ContentIdentity {
            guid: Some("guid-vendor-plugin".to_string()),
            asset_path: Some("Assets/Vendor/plugin.asset".to_string()),
            sub_asset_id: None,
            object_name: Some("plugin".to_string()),
        },
        owning_feature_id: None,
        tier: None,
        state: None,
        observable_consequence: None,
    };
    let package = ProjectPackageEntry {
        package_id: "com.unity.render-pipelines.universal".to_string(),
        version: "14.0.10".to_string(),
        source: PackageSource::Registry,
        is_locally_modified_embedded: false,
    };

    let before = assess_project(&ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![package.clone()],
        assets: vec![urp_scene.clone(), hdrp_asset.clone(), vendor_asset.clone()],
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    });

    let after = assess_project(&ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![package],
        assets: vec![urp_scene, vendor_asset],
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    });

    assert!(
        before
            .report
            .findings
            .iter()
            .any(|finding| finding.content.asset_path.as_deref()
                == Some("Assets/HDRP/SkyVolume.asset")),
        "CMP-002 Story 5: the project before the change must report the HDRP content"
    );
    assert!(
        !after
            .report
            .findings
            .iter()
            .any(|finding| finding.content.asset_path.as_deref()
                == Some("Assets/HDRP/SkyVolume.asset")),
        "CMP-002 Story 5: content removed by the project change must not keep a finding"
    );

    for path in ["Assets/Scenes/Playable.unity", "Assets/Vendor/plugin.asset"] {
        let before_finding = before
            .report
            .findings
            .iter()
            .find(|finding| finding.content.asset_path.as_deref() == Some(path));
        let after_finding = after
            .report
            .findings
            .iter()
            .find(|finding| finding.content.asset_path.as_deref() == Some(path));
        assert!(
            before_finding.is_some() && after_finding.is_some(),
            "CMP-002 Story 5: unaffected content must keep its finding after reassessment"
        );
        if let (Some(before_finding), Some(after_finding)) = (before_finding, after_finding) {
            assert_eq!(
                before_finding, after_finding,
                "CMP-002 Story 5: reassessment must update only the findings the project change affected"
            );
        }
    }

    assert_eq!(before.overall_state, CompatibilityState::Unsupported);
    assert_eq!(
        after.overall_state,
        CompatibilityState::Unknown,
        "CMP-002 Story 5: the reassessed report must state the new project state"
    );
    assert!(before.unsupported_features.contains(&"REN-006".to_string()));
    assert!(
        !after.unsupported_features.contains(&"REN-006".to_string()),
        "CMP-002 Story 5: the removed content must no longer be reported unsupported"
    );
    assert!(after
        .present_supported_features
        .contains(&"REN-006".to_string()));
    assert_eq!(after.effective_baseline_unity_version, "2022.3 LTS");
    assert_eq!(
        after.report.produced_by_unitas_version, "0.4.0",
        "CMP-002 Story 5: the reassessed report must record the unitas version that produced it"
    );
}

#[test]
fn cmp_002_story_8_locally_modified_embedded_package_is_not_matched_and_is_assessed_as_third_party_content(
) {
    use unitas_core::compat::{
        assess_package, assess_project, PackageSource, ProjectContent, ProjectPackageEntry,
    };

    let modified = assess_package("com.unity.ugui", "1.0.0", true);
    assert_eq!(
        modified.state,
        CompatibilityState::Unknown,
        "CMP-002 Story 8: a locally modified embedded copy must not match the released version"
    );
    assert!(modified.tier.is_none());
    assert!(modified.owning_spec.is_none());
    assert!(
        modified.notes.contains("third-party"),
        "CMP-002 Story 8: a locally modified embedded copy must be assessed as third-party content"
    );

    let released = assess_package("com.unity.ugui", "1.0.0", false);
    assert_eq!(
        released.state,
        CompatibilityState::Supported,
        "CMP-002 Story 8: an embedded copy whose content matches the released version is matched"
    );
    assert_eq!(released.tier, Some(Tier::Familiar));

    let content = ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![ProjectPackageEntry {
            package_id: "com.unity.ugui".to_string(),
            version: "1.0.0".to_string(),
            source: PackageSource::Embedded,
            is_locally_modified_embedded: true,
        }],
        assets: Vec::new(),
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    };

    let assessment = assess_project(&content);
    let finding = assessment
        .report
        .findings
        .iter()
        .find(|finding| finding.feature_id == "PRJ-006");
    assert!(finding.is_some());
    if let Some(finding) = finding {
        assert_eq!(
            finding.state,
            CompatibilityState::Unknown,
            "CMP-002 Story 8: the project report must report the modified copy as third-party content"
        );
        assert!(finding.observable_consequence.contains("third-party"));
    }
}

#[test]
fn cmp_002_story_7_hdrp_and_visual_scripting_are_unsupported_with_their_observable_consequence_while_urp_scenes_remain_playable(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        ProjectSettingEntry,
    };

    let urp_scene = ProjectAssetEntry {
        family: "URP Scene".to_string(),
        content: ContentIdentity {
            guid: Some("guid-urp-scene".to_string()),
            asset_path: Some("Assets/Scenes/GameplayUrp.unity".to_string()),
            sub_asset_id: None,
            object_name: Some("GameplayUrp".to_string()),
        },
        owning_feature_id: Some("REN-006".to_string()),
        tier: Some(Tier::Native),
        state: Some(CompatibilityState::Supported),
        observable_consequence: None,
    };
    let hdrp_asset = ProjectAssetEntry {
        family: "High Definition Render Pipeline".to_string(),
        content: ContentIdentity {
            guid: Some("guid-hdrp-mat".to_string()),
            asset_path: Some("Assets/HDRP/SkyVolume.asset".to_string()),
            sub_asset_id: None,
            object_name: Some("SkyVolume".to_string()),
        },
        owning_feature_id: None,
        tier: None,
        state: None,
        observable_consequence: None,
    };
    let vs_graph = ProjectAssetEntry {
        family: "Visual Scripting".to_string(),
        content: ContentIdentity {
            guid: Some("guid-vs-graph".to_string()),
            asset_path: Some("Assets/Graphs/EnemyBrain.asset".to_string()),
            sub_asset_id: None,
            object_name: Some("EnemyBrain".to_string()),
        },
        owning_feature_id: None,
        tier: None,
        state: None,
        observable_consequence: None,
    };

    let initial_content = ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2021.3.35f1".to_string()),
        packages: vec![
            ProjectPackageEntry {
                package_id: "com.unity.render-pipelines.universal".to_string(),
                version: "14.0.10".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.unity.ide.rider".to_string(),
                version: "3.0.28".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
        ],
        assets: vec![urp_scene.clone(), hdrp_asset, vs_graph],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/LightingSettings.lighting".to_string(),
            family: "Enlighten realtime global illumination".to_string(),
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        additional_findings: vec![finding(
            "PHY-001",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/Prefabs/PlayerRB.prefab",
        )],
        track_absent_catalog_features: vec!["NET-001".to_string()],
    };

    let initial = assess_project(&initial_content);

    assert!(initial.migrated_from_earlier_lts);
    assert_eq!(initial.overall_state, CompatibilityState::Unsupported);
    assert!(initial
        .playable_scenes
        .contains(&"Assets/Scenes/GameplayUrp.unity".to_string()));

    let hdrp_finding = initial
        .report
        .findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("Assets/HDRP/SkyVolume.asset"));
    assert!(hdrp_finding.is_some());
    if let Some(f) = hdrp_finding {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "REN-006");
        assert!(f
            .observable_consequence
            .contains("URP and Built-in scenes remain editable and playable"));
    }

    let vs_finding = initial
        .report
        .findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("Assets/Graphs/EnemyBrain.asset"));
    assert!(vs_finding.is_some());
    if let Some(f) = vs_finding {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "RUN-002");
        assert!(f
            .observable_consequence
            .contains("C# MonoBehaviours remain playable"));
    }

    let enlighten_finding = initial.report.findings.iter().find(|f| {
        f.content.asset_path.as_deref() == Some("ProjectSettings/LightingSettings.lighting")
    });
    assert!(enlighten_finding.is_some());
    if let Some(f) = enlighten_finding {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "REN-003");
    }

    let phy_finding = initial
        .report
        .findings
        .iter()
        .find(|f| f.feature_id == "PHY-001");
    assert!(phy_finding.is_some());

    let cleaned_content = ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![ProjectPackageEntry {
            package_id: "com.unity.render-pipelines.universal".to_string(),
            version: "14.0.10".to_string(),
            source: PackageSource::Registry,
            is_locally_modified_embedded: false,
        }],
        assets: vec![urp_scene],
        settings: Vec::new(),
        additional_findings: vec![finding(
            "PHY-001",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/Prefabs/PlayerRB.prefab",
        )],
        track_absent_catalog_features: vec!["NET-001".to_string()],
    };

    let cleaned = assess_project(&cleaned_content);
    assert_eq!(cleaned.overall_state, CompatibilityState::Supported);
    assert!(cleaned
        .report
        .findings
        .iter()
        .all(|f| f.state == CompatibilityState::Supported || !is_present(f)));
    assert!(cleaned.absent_features.contains(&"NET-001".to_string()));
}

#[test]
fn compat_submit_findings_ingests_external_worktree_catalog_ids_and_updates_summaries() {
    let base_report = build_report(
        "0.2.0",
        vec![finding(
            "PHY-001",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/Prefabs/Crate.prefab",
        )],
    );

    let run_entry = lookup_catalog_feature("RUN-001");
    assert!(run_entry.is_some());
    if let Some(entry) = run_entry {
        assert_eq!(entry.tier, Tier::Semantic);
    }

    let scr_alias = lookup_catalog_feature("SCR-001");
    assert!(scr_alias.is_some());
    if let Some(entry) = scr_alias {
        assert_eq!(entry.tier, Tier::Semantic);
    }

    let scripting_input = FindingInput::new(
        "SCR-001",
        Tier::Semantic,
        CompatibilityState::PartiallySupported,
        content("Assets/Scripts/PlayerController.cs"),
        "MonoBehaviour OnAnimatorIK message callback",
        "IK pass callback is not invoked during fixed step",
    )
    .with_condition("script defines OnAnimatorIK");

    let rendering_input = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Unsupported,
        content("Assets/Cameras/SecurityCam.prefab"),
        "Camera custom stereo target eye",
        "secondary stereo eye target is not rendered",
    )
    .with_unity_difference("Native camera pipeline uses single-pass target binding")
    .with_condition("Camera.stereoTargetEye is non-default")
    .with_related_finding("XR-001");

    let editor_input = FindingInput::new(
        "EDT-002",
        Tier::Native,
        CompatibilityState::Supported,
        content("Assets/Materials/Wood.mat"),
        "Inspector material property editing",
        "properties edit and serialize cleanly",
    );

    let after_one = submit_finding(&base_report, scripting_input.clone());
    assert!(after_one.is_ok());
    if let Ok(report_one) = after_one {
        assert_eq!(report_one.findings.len(), 2);
        let summary_counts = report_one.summary();
        assert_eq!(summary_counts.total_findings, 2);
        assert_eq!(summary_counts.supported_count, 1);
        assert_eq!(summary_counts.partially_supported_count, 1);
        assert_eq!(
            summary_counts.overall_state,
            CompatibilityState::PartiallySupported
        );
    }

    let merged = submit_findings(
        &base_report,
        vec![rendering_input, editor_input, scripting_input],
    );
    assert!(merged.is_ok());
    if let Ok(report) = merged {
        assert_eq!(report.findings.len(), 4);
        assert_eq!(report.findings[0].feature_id, "EDT-002");
        assert_eq!(report.findings[1].feature_id, "PHY-001");
        assert_eq!(report.findings[2].feature_id, "REN-001");
        assert_eq!(report.findings[3].feature_id, "SCR-001");

        assert_eq!(report.packages.len(), 4);
        assert_eq!(report.packages[0].package_name, "EDT");
        assert_eq!(report.packages[0].state, CompatibilityState::Supported);
        assert_eq!(report.packages[1].package_name, "PHY");
        assert_eq!(report.packages[1].state, CompatibilityState::Supported);
        assert_eq!(report.packages[2].package_name, "REN");
        assert_eq!(report.packages[2].state, CompatibilityState::Unsupported);
        assert_eq!(report.packages[3].package_name, "SCR");
        assert_eq!(
            report.packages[3].state,
            CompatibilityState::PartiallySupported
        );

        let counts = report.summary();
        assert_eq!(counts.total_findings, 4);
        assert_eq!(counts.present_findings, 4);
        assert_eq!(counts.absent_findings, 0);
        assert_eq!(counts.supported_count, 2);
        assert_eq!(counts.partially_supported_count, 1);
        assert_eq!(counts.unsupported_count, 1);
        assert_eq!(counts.overall_state, CompatibilityState::Unsupported);
    }
}

#[test]
fn compat_submit_finding_rejects_invalid_feature_tier_content_message_and_duplicates() {
    let base_report = build_report(
        "0.2.0",
        vec![finding(
            "RUN-001",
            Tier::Semantic,
            CompatibilityState::Supported,
            "Assets/Scripts/Bootstrap.cs",
        )],
    );

    let empty_feature = FindingInput::new(
        "   ",
        Tier::Semantic,
        CompatibilityState::Supported,
        content("Assets/Scripts/A.cs"),
        "transform hierarchy",
        "behaves as documented",
    );
    assert_eq!(
        submit_finding(&base_report, empty_feature),
        Err(FindingSubmissionError::EmptyFeatureId)
    );

    let unknown_feature = FindingInput::new(
        "XYZ-999",
        Tier::Semantic,
        CompatibilityState::Supported,
        content("Assets/Scripts/A.cs"),
        "unknown subsystem",
        "none",
    );
    assert_eq!(
        submit_finding(&base_report, unknown_feature),
        Err(FindingSubmissionError::UnknownCatalogFeatureId {
            feature_id: "XYZ-999".to_string(),
        })
    );

    let tier_mismatch = FindingInput::new(
        "REN-001",
        Tier::Exact,
        CompatibilityState::Supported,
        content("Assets/Cameras/Main.prefab"),
        "camera projection",
        "renders cleanly",
    );
    assert_eq!(
        submit_finding(&base_report, tier_mismatch),
        Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: "REN-001".to_string(),
            expected: Tier::Native,
            submitted: Tier::Exact,
        })
    );

    let empty_content = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Supported,
        ContentIdentity {
            guid: Some("   ".to_string()),
            asset_path: None,
            sub_asset_id: None,
            object_name: None,
        },
        "camera projection",
        "renders cleanly",
    );
    assert_eq!(
        submit_finding(&base_report, empty_content),
        Err(FindingSubmissionError::MissingContentIdentity)
    );

    let empty_behavior = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Supported,
        content("Assets/Cameras/Main.prefab"),
        "  ",
        "renders cleanly",
    );
    assert_eq!(
        submit_finding(&base_report, empty_behavior),
        Err(FindingSubmissionError::EmptyBehavior)
    );

    let empty_consequence = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Supported,
        content("Assets/Cameras/Main.prefab"),
        "camera projection",
        "",
    );
    assert_eq!(
        submit_finding(&base_report, empty_consequence),
        Err(FindingSubmissionError::EmptyObservableConsequence)
    );

    let duplicate_existing = FindingInput::new(
        "RUN-001",
        Tier::Semantic,
        CompatibilityState::PartiallySupported,
        content("Assets/Scripts/Bootstrap.cs"),
        "GameObject activation",
        "differs on order",
    );
    assert_eq!(
        submit_finding(&base_report, duplicate_existing),
        Err(FindingSubmissionError::DuplicateFindingIdentity {
            feature_id: "RUN-001".to_string(),
            content: Box::new(content("Assets/Scripts/Bootstrap.cs")),
        })
    );

    let supported_with_unity_diff = FindingInput::new(
        "PHY-001",
        Tier::Semantic,
        CompatibilityState::Supported,
        content("Assets/Prefabs/Box.prefab"),
        "Rigidbody box collider contact points",
        "contact order differs from Unity 2022.3 LTS",
    )
    .with_unity_difference("contact points are emitted in manifold index order");
    assert_eq!(
        submit_finding(&base_report, supported_with_unity_diff),
        Err(
            FindingSubmissionError::SupportedFindingClaimsUnityDifference {
                feature_id: "PHY-001".to_string(),
                tier: Tier::Semantic,
            }
        )
    );

    let familiar_without_diff_on_partial = FindingInput::new(
        "UI-001",
        Tier::Familiar,
        CompatibilityState::PartiallySupported,
        content("Assets/UI/Hud.prefab"),
        "Canvas scaler screen match mode",
        "shrink mode falls back to expand",
    );
    assert_eq!(
        submit_finding(&base_report, familiar_without_diff_on_partial),
        Err(FindingSubmissionError::MissingReliedUponUnityDifference {
            feature_id: "UI-001".to_string(),
            tier: Tier::Familiar,
            state: CompatibilityState::PartiallySupported,
        })
    );

    let batch_dup_a = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Supported,
        content("Assets/Cameras/CamA.prefab"),
        "camera depth",
        "supported",
    );
    let batch_dup_b = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Unsupported,
        content("Assets/Cameras/CamA.prefab"),
        "camera hdr",
        "unsupported",
    );
    let batch_res = submit_findings(&base_report, vec![batch_dup_a, batch_dup_b]);
    assert_eq!(
        batch_res,
        Err(FindingRejection {
            index: 1,
            error: FindingSubmissionError::DuplicateFindingIdentity {
                feature_id: "REN-001".to_string(),
                content: Box::new(content("Assets/Cameras/CamA.prefab")),
            },
        })
    );
}

#[test]
fn compat_canonical_rendering_is_byte_identical_across_insertion_orders_and_round_trips() {
    let base_empty = build_report("0.3.0", Vec::new());

    let input_scr = FindingInput::new(
        "SCR-001",
        Tier::Semantic,
        CompatibilityState::PartiallySupported,
        ContentIdentity {
            guid: Some("guid-script-1".to_string()),
            asset_path: Some("Assets/Scripts/Mover.cs".to_string()),
            sub_asset_id: Some("11500000".to_string()),
            object_name: Some("Mover".to_string()),
        },
        "MonoBehaviour FixedUpdate scheduling",
        "script FixedUpdate runs in deterministic step order",
    )
    .with_unity_difference("execution order among equal priorities is sorted by GUID")
    .with_condition("script attached to active GameObject")
    .with_related_finding("PHY-001");

    let input_ren = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::Unsupported,
        ContentIdentity {
            guid: Some("guid-cam-1".to_string()),
            asset_path: Some("Assets/Prefabs/Cam.prefab".to_string()),
            sub_asset_id: None,
            object_name: Some("MainCamera".to_string()),
        },
        "Camera physical sensor gate fit",
        "gate fit lens shift is ignored; standard perspective FOV is used",
    )
    .with_unity_difference("physical camera lens shift is not emulated")
    .with_condition("Camera.usePhysicalProperties is true");

    let input_phy = FindingInput::new(
        "PHY-001",
        Tier::Semantic,
        CompatibilityState::Supported,
        ContentIdentity {
            guid: Some("guid-rb-1".to_string()),
            asset_path: Some("Assets/Prefabs/Ball.prefab".to_string()),
            sub_asset_id: None,
            object_name: Some("Ball".to_string()),
        },
        "Rigidbody sphere collision response",
        "collides and resolves impulses deterministically",
    );

    let forward = submit_findings(
        &base_empty,
        vec![input_scr.clone(), input_ren.clone(), input_phy.clone()],
    );
    let reverse = submit_findings(
        &base_empty,
        vec![input_phy, input_ren.clone(), input_scr.clone()],
    );
    assert!(forward.is_ok());
    assert!(reverse.is_ok());

    if let (Ok(report_a), Ok(report_b)) = (forward, reverse) {
        assert_eq!(report_a, report_b);

        let rendered_a = report_a.to_canonical_text();
        let rendered_b = report_b.to_canonical_text();
        assert_eq!(rendered_a, rendered_b);
        assert_eq!(render_canonical(&report_a), rendered_a);
        assert!(rendered_a.starts_with("UNITAS_COMPAT_REPORT v1\n"));
        assert!(rendered_a.contains("unitas_version: 0.3.0\n"));
        assert!(rendered_a.contains("overall_state: Unsupported\n"));

        let parsed = CompatibilityReport::from_canonical_text(&rendered_a);
        assert!(parsed.is_ok());
        if let Ok(round_tripped) = parsed {
            assert_eq!(round_tripped, report_a);
            assert_eq!(round_tripped.to_canonical_text(), rendered_a);
            assert_eq!(parse_canonical(&rendered_a), Ok(round_tripped));
        }

        let part_one = submit_findings(&base_empty, vec![input_scr]);
        let part_two = submit_findings(&base_empty, vec![input_ren]);
        assert!(part_one.is_ok());
        assert!(part_two.is_ok());
        if let (Ok(r1), Ok(r2)) = (part_one, part_two) {
            let merged = merge_findings(&r1, &r2.findings);
            assert!(merged.is_ok());
            if let Ok(merged_report) = merged {
                assert_eq!(merged_report.findings.len(), 2);
                assert_eq!(
                    merged_report.summary().overall_state,
                    CompatibilityState::Unsupported
                );
            }
        }
    }
}

#[test]
fn compat_canonical_parser_rejects_malformed_text_and_escapes_special_characters() {
    use unitas_core::compat::CanonicalReportParseError;

    assert_eq!(
        CompatibilityReport::from_canonical_text(""),
        Err(CanonicalReportParseError::MissingHeader)
    );
    assert_eq!(
        CompatibilityReport::from_canonical_text("WRONG_HEADER\n"),
        Err(CanonicalReportParseError::MissingHeader)
    );
    assert_eq!(
        CompatibilityReport::from_canonical_text("UNITAS_COMPAT_REPORT v1\n"),
        Err(CanonicalReportParseError::MissingVersion)
    );
    assert_eq!(
        CompatibilityReport::from_canonical_text(
            "UNITAS_COMPAT_REPORT v1\nmalformed_line_no_colon\n"
        ),
        Err(CanonicalReportParseError::MalformedLine {
            line_number: 2,
            line: "malformed_line_no_colon".to_string(),
        })
    );
    assert_eq!(
        CompatibilityReport::from_canonical_text(
            "UNITAS_COMPAT_REPORT v1\nunitas_version: 0.1.0\n[finding]\nfeature_id: REN-001\ntier: BogusTier\n"
        ),
        Err(CanonicalReportParseError::InvalidTier("BogusTier".to_string()))
    );
    assert_eq!(
        CompatibilityReport::from_canonical_text(
            "UNITAS_COMPAT_REPORT v1\nunitas_version: 0.1.0\n[finding]\nfeature_id: REN-001\ntier: Native\nstate: BogusState\n"
        ),
        Err(CanonicalReportParseError::InvalidState("BogusState".to_string()))
    );

    let base = build_report("0.3.0", Vec::new());
    let special = FindingInput::new(
        "REN-001",
        Tier::Native,
        CompatibilityState::PartiallySupported,
        ContentIdentity {
            guid: Some("guid\\1".to_string()),
            asset_path: Some("Assets/Cameras/Cam\t1.prefab".to_string()),
            sub_asset_id: None,
            object_name: Some("Cam\nName".to_string()),
        },
        "Camera line1\nline2\tbehavior\\check",
        "Consequence with \\n and \r\n newlines",
    )
    .with_unity_difference("diff\twith\ttabs")
    .with_condition("cond\nwith\nnewlines")
    .with_related_finding("XR-001");

    let report_res = base.submit_finding(special);
    assert!(report_res.is_ok());
    if let Ok(report) = report_res {
        let text = report.to_canonical_text();
        let parsed = CompatibilityReport::from_canonical_text(&text);
        assert_eq!(parsed, Ok(report));
    }
}

#[test]
fn cmp_002_story_16_migrates_unity_2021_3_urp_12_x_against_2022_3_lts_urp_14_x_baseline_without_2021_3_claims(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        ProjectSettingEntry,
    };

    let content = ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2021.3.38f1".to_string()),
        packages: vec![ProjectPackageEntry {
            package_id: "com.unity.render-pipelines.universal".to_string(),
            version: "12.1.14".to_string(),
            source: PackageSource::Registry,
            is_locally_modified_embedded: false,
        }],
        assets: vec![ProjectAssetEntry {
            family: "URP Forward Scene".to_string(),
            content: ContentIdentity {
                guid: Some("guid-migrated-urp-scene".to_string()),
                asset_path: Some("Assets/Scenes/MigratedLevel.unity".to_string()),
                sub_asset_id: None,
                object_name: Some("MigratedLevel".to_string()),
            },
            owning_feature_id: Some("REN-006".to_string()),
            tier: Some(Tier::Native),
            state: Some(CompatibilityState::Supported),
            observable_consequence: None,
        }],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/ProjectVersion.txt".to_string(),
            family: "Serialized project version".to_string(),
            owning_feature_id: Some("SER-005".to_string()),
            tier: Some(Tier::Exact),
            state: Some(CompatibilityState::Supported),
            observable_consequence: None,
        }],
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    };

    let assessment = assess_project(&content);
    assert!(assessment.migrated_from_earlier_lts);
    assert_eq!(assessment.effective_baseline_unity_version, "2022.3 LTS");
    assert_eq!(assessment.overall_state, CompatibilityState::Supported);

    let urp = assessment
        .assessed_packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.render-pipelines.universal");
    assert!(urp.is_some());
    if let Some(urp_pkg) = urp {
        assert_eq!(
            urp_pkg.state,
            CompatibilityState::Supported,
            "CMP-002 Story 16: URP 12.x in a 2021.3 LTS project must migrate and assess as Supported against URP 14.x"
        );
        assert_eq!(urp_pkg.tier, Some(Tier::Native));
        assert_eq!(
            urp_pkg.owning_spec.as_deref(),
            Some("rendering-and-animation.md")
        );
        assert_eq!(urp_pkg.owning_feature_id.as_deref(), Some("REN-006"));
        assert!(urp_pkg.notes.contains("14.x"));
        assert!(!urp_pkg.notes.contains("2021.3-specific"));
    }

    for f in &assessment.report.findings {
        assert!(
            !f.behavior.contains("2021.3") && !f.observable_consequence.contains("2021.3-specific"),
            "CMP-002 Story 16: no finding may claim 2021.3-specific behavior"
        );
    }
}

#[test]
fn cmp_002_story_17_and_rendering_outcome_34_semantic_cites_cmp_001_and_familiar_catalog_populates_all_differences(
) {
    use unitas_core::compat::{
        build_familiar_finding, build_semantic_finding, lookup_familiar_behavior_differences,
        FAMILIAR_BEHAVIOR_DIFFERENCES,
    };

    let semantic = build_semantic_finding(
        "PHY-001",
        CompatibilityState::Supported,
        content("Assets/Prefabs/DynamicCrate.prefab"),
        "Rigidbody continuous collision detection and friction impulse solver",
        "CMP-001::phy_001_rigidbody_stack_benchmark",
        None,
    );
    assert!(semantic.is_ok());
    if let Ok(semantic_finding) = semantic {
        assert_eq!(semantic_finding.tier, Tier::Semantic);
        assert!(
            semantic_finding
                .observable_consequence
                .contains("Unity 2022.3 LTS"),
            "CMP-002 Story 17: Semantic finding must cite Unity 2022.3 LTS"
        );
        assert!(
            semantic_finding.observable_consequence.contains("CMP-001"),
            "CMP-002 Story 17: Semantic finding must cite its CMP-001 benchmark"
        );
        assert!(
            semantic_finding
                .conditions
                .iter()
                .any(|c| c.contains("CMP-001::phy_001_rigidbody_stack_benchmark")),
            "CMP-002 Story 17: Semantic finding conditions must record the CMP-001 benchmark ID"
        );
    }

    let expected_counts = [
        ("ANI-001", "rendering-and-animation.md", 5usize),
        ("ANI-002", "rendering-and-animation.md", 3usize),
        ("CIN-001", "rendering-and-animation.md", 2usize),
        ("REN-004", "rendering-and-animation.md", 11usize),
        ("PHY-004", "physics-navigation-terrain.md", 3usize),
        ("NAV-001", "physics-navigation-terrain.md", 4usize),
        ("ENV-001", "physics-navigation-terrain.md", 2usize),
    ];

    assert_eq!(FAMILIAR_BEHAVIOR_DIFFERENCES.len(), 7);

    for (feature_id, expected_spec, expected_count) in expected_counts {
        let entry = lookup_familiar_behavior_differences(feature_id);
        assert!(
            entry.is_some(),
            "CMP-002 Story 17 / Outcome 34: missing Familiar difference entry for {feature_id}"
        );
        if let Some(catalog_diffs) = entry {
            assert_eq!(catalog_diffs.owning_spec, expected_spec);
            assert_eq!(
                catalog_diffs.differences.len(),
                expected_count,
                "CMP-002 Story 17 / Outcome 34: unexpected difference count for {feature_id}"
            );
        }

        let built = build_familiar_finding(
            feature_id,
            CompatibilityState::Supported,
            content(&format!("Assets/Test/{feature_id}.asset")),
            &format!("{feature_id} behavior assessment"),
            &[],
        );
        assert!(built.is_ok());
        if let Ok(familiar_finding) = built {
            assert_eq!(familiar_finding.tier, Tier::Familiar);
            assert_eq!(
                familiar_finding.relied_upon_unity_differences.len(),
                expected_count,
                "CMP-002 Story 17 / Outcome 34: Familiar finding for {feature_id} must list all {expected_count} canonical differences"
            );
            assert!(
                familiar_finding
                    .observable_consequence
                    .contains(expected_spec),
                "CMP-002 Story 17: Familiar finding for {feature_id} must cite its owning spec {expected_spec}"
            );
            assert!(
                familiar_finding
                    .observable_consequence
                    .contains("AUT-006"),
                "Outcome 34: Familiar finding for {feature_id} must reference the AUT-006 API reference"
            );
        }
    }
}

#[test]
fn cmp_002_story_18_translated_material_accepts_against_unitas_reference_and_rejects_unity_screenshot_pixel_diff_failure(
) {
    use unitas_core::compat::assess_translated_material;

    let accepted = assess_translated_material(
        content("Assets/Materials/HeroLit.mat"),
        "Standard -> URP/Lit translated material",
        Some("Refs/TST-002/hero_lit_approved.png"),
        true,
        true,
    );
    assert_eq!(accepted.feature_id, "REN-002");
    assert_eq!(accepted.tier, Tier::Native);
    assert_eq!(
        accepted.state,
        CompatibilityState::Supported,
        "CMP-002 Story 18: translated material matching unitas-approved reference image is Supported even when pixels differ from a Unity screenshot"
    );
    assert!(accepted.observable_consequence.contains("TST-002"));
    assert!(accepted
        .observable_consequence
        .contains("Refs/TST-002/hero_lit_approved.png"));
    assert!(accepted
        .observable_consequence
        .contains("never a compatibility failure"));

    let base_report = build_report("0.4.0", Vec::new());
    let bogus_pixel_failure = FindingInput::new(
        "REN-002",
        Tier::Native,
        CompatibilityState::Unsupported,
        content("Assets/Materials/HeroLit.mat"),
        "Translated material rendering",
        "pixel difference from a Unity screenshot exceeded threshold",
    );
    let rejected = submit_finding(&base_report, bogus_pixel_failure);
    assert!(
        rejected.is_err(),
        "CMP-002 Story 18: report must never present a pixel difference from a Unity screenshot as a compatibility failure"
    );
}

#[test]
fn cmp_002_story_19_unspecified_supported_package_behavior_is_unknown_and_package_matrix_tiers_match_feature_catalog(
) {
    use unitas_core::compat::{
        assess_package_behavior, PackageBehaviorClassification, SUPPORTED_PACKAGE_MATRIX,
    };

    for entry in SUPPORTED_PACKAGE_MATRIX {
        let catalog = lookup_catalog_feature(entry.owning_feature_id);
        assert!(
            catalog.is_some(),
            "CMP-002 Story 19: every package matrix entry must reference a valid catalog feature"
        );
        if let Some(cat) = catalog {
            assert_eq!(
                entry.tier, cat.tier,
                "CMP-002 Story 19: package {} tier must equal owning catalog feature {} tier",
                entry.package_id, entry.owning_feature_id
            );
            assert_eq!(
                entry.owning_spec, cat.owning_spec,
                "CMP-002 Story 19: package {} owning spec must equal owning catalog feature {} spec",
                entry.package_id, entry.owning_feature_id
            );
        }
    }

    let unspecified = assess_package_behavior(
        "com.unity.inputsystem",
        "1.7.0",
        content("Assets/Input/PlayerControls.inputactions"),
        "InputActionRebindingExtensions.LoadBindingOverridesFromJson undocumented experimental hook",
        PackageBehaviorClassification::NeitherNamedNorExcluded,
    );
    assert_eq!(
        unspecified.state,
        CompatibilityState::Unknown,
        "CMP-002 Story 19: behavior in a Supported package neither named nor excluded by the owning spec must be reported Unknown"
    );
    assert_eq!(unspecified.feature_id, "INP-002");
    assert_eq!(unspecified.tier, Tier::Familiar);
}

#[test]
fn cmp_002_story_19_undemonstrated_package_behaviors_are_reported_with_their_real_compatibility_state(
) {
    use unitas_core::compat::assess_package;

    let undemonstrated = [
        (
            "com.unity.localization",
            "1.5.1",
            CompatibilityState::Unsupported,
        ),
        (
            "com.unity.addressables",
            "1.21.21",
            CompatibilityState::Unsupported,
        ),
        ("com.unity.burst", "1.8.7", CompatibilityState::Unsupported),
        (
            "com.unity.collections",
            "2.1.4",
            CompatibilityState::Unsupported,
        ),
        (
            "com.unity.mathematics",
            "1.2.6",
            CompatibilityState::Unknown,
        ),
        (
            "com.unity.nuget.newtonsoft-json",
            "3.2.1",
            CompatibilityState::Unknown,
        ),
    ];

    for (package_id, version, expected_state) in undemonstrated {
        let assessed = assess_package(package_id, version, false);
        assert_eq!(
            assessed.state, expected_state,
            "CMP-002 Story 19: package '{package_id}' must be reported with its real compatibility state, never Supported"
        );
        assert_ne!(
            assessed.state,
            CompatibilityState::Supported,
            "CMP-002 Story 19: package '{package_id}' is not demonstrated and must never be reported Supported"
        );
        let expected_reason = match expected_state {
            CompatibilityState::Unsupported => "Unsupported: ",
            _ => "Unknown: ",
        };
        assert!(
            assessed.notes.starts_with(expected_reason),
            "CMP-002 Story 19: package '{package_id}' must record why it is {expected_state:?}"
        );
    }
}

#[test]
fn cmp_002_story_20_enlighten_gi_and_unity_built_assetbundles_are_unsupported_and_rider_and_collab_are_partially_supported_with_exclusions(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        ProjectSettingEntry,
    };

    let content = ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![
            ProjectPackageEntry {
                package_id: "com.unity.ide.rider".to_string(),
                version: "3.0.28".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.unity.collab-proxy".to_string(),
                version: "2.3.1".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
        ],
        assets: vec![ProjectAssetEntry {
            family: "Unity-built AssetBundles and Addressables content".to_string(),
            content: ContentIdentity {
                guid: Some("guid-unity-bundle".to_string()),
                asset_path: Some("Assets/StreamingAssets/level1.bundle".to_string()),
                sub_asset_id: None,
                object_name: Some("level1.bundle".to_string()),
            },
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/LightingSettings.lighting".to_string(),
            family: "Enlighten realtime global illumination".to_string(),
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    };

    let assessment = assess_project(&content);

    let enlighten = assessment.report.findings.iter().find(|f| {
        f.content.asset_path.as_deref() == Some("ProjectSettings/LightingSettings.lighting")
    });
    assert!(enlighten.is_some());
    if let Some(f) = enlighten {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "REN-003");
    }

    let bundle =
        assessment.report.findings.iter().find(|f| {
            f.content.asset_path.as_deref() == Some("Assets/StreamingAssets/level1.bundle")
        });
    assert!(bundle.is_some());
    if let Some(f) = bundle {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "BLD-005");
    }

    let rider = assessment
        .assessed_packages
        .iter()
        .find(|p| p.package_id == "com.unity.ide.rider");
    assert!(rider.is_some());
    if let Some(p) = rider {
        assert_eq!(p.state, CompatibilityState::PartiallySupported);
        assert_eq!(p.owning_feature_id.as_deref(), Some("DIA-002"));
        assert!(p.notes.contains("preference"));
        assert!(p.notes.contains("RUN-010"));
    }

    let collab = assessment
        .assessed_packages
        .iter()
        .find(|p| p.package_id == "com.unity.collab-proxy");
    assert!(collab.is_some());
    if let Some(p) = collab {
        assert_eq!(p.state, CompatibilityState::PartiallySupported);
        assert_eq!(p.owning_feature_id.as_deref(), Some("EDT-007"));
        assert!(p.notes.contains("no version-control client is provided"));
    }
}

#[test]
fn cmp_002_story_22_shader_graph_unrepresentable_node_reports_graph_and_node_and_hdrp_target_is_unsupported(
) {
    use unitas_core::compat::{assess_shader_graph, ShaderGraphNodeIssue, ShaderGraphTarget};

    let unrepresentable = assess_shader_graph(
        content("Assets/Shaders/WaterSurface.shadergraph"),
        ShaderGraphTarget::Universal,
        &[ShaderGraphNodeIssue {
            node_type: "CustomHDSceneColorNode".to_string(),
            node_id: "node-98a1".to_string(),
        }],
        &["Assets/Materials/OceanWater.mat".to_string()],
    );
    assert_eq!(unrepresentable.feature_id, "REN-006");
    assert_eq!(unrepresentable.tier, Tier::Native);
    assert_eq!(unrepresentable.state, CompatibilityState::Unsupported);
    assert_eq!(
        unrepresentable.content.asset_path.as_deref(),
        Some("Assets/Shaders/WaterSurface.shadergraph")
    );
    assert_eq!(
        unrepresentable.content.sub_asset_id.as_deref(),
        Some("node-98a1")
    );
    assert!(unrepresentable
        .behavior
        .contains("WaterSurface.shadergraph"));
    assert!(unrepresentable.behavior.contains("CustomHDSceneColorNode"));
    assert!(unrepresentable.behavior.contains("node-98a1"));
    assert!(unrepresentable
        .observable_consequence
        .contains("Assets/Materials/OceanWater.mat"));

    let hdrp_target = assess_shader_graph(
        content("Assets/Shaders/HdrpDecal.shadergraph"),
        ShaderGraphTarget::HighDefinition,
        &[],
        &["Assets/Materials/HdrpDecal.mat".to_string()],
    );
    assert_eq!(hdrp_target.feature_id, "REN-006");
    assert_eq!(hdrp_target.tier, Tier::Native);
    assert_eq!(
        hdrp_target.state,
        CompatibilityState::Unsupported,
        "CMP-002 Story 22: HDRP Shader Graph target must be reported Unsupported"
    );
    assert!(hdrp_target.behavior.contains("HDRP"));
}

#[test]
fn cmp_002_story_1_report_identifies_every_recognized_unsupported_behavior_and_keeps_unassessed_content_explicit(
) {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        RECOGNIZED_UNSUPPORTED_FAMILIES,
    };

    for entry in RECOGNIZED_UNSUPPORTED_FAMILIES {
        let content = ProjectContent {
            unitas_version: "0.4.0".to_string(),
            declared_unity_version: Some("2022.3.20f1".to_string()),
            packages: vec![ProjectPackageEntry {
                package_id: "com.unity.render-pipelines.universal".to_string(),
                version: "14.0.10".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            }],
            assets: vec![
                ProjectAssetEntry {
                    family: entry.family.to_string(),
                    content: ContentIdentity {
                        guid: Some("guid-unsupported-case".to_string()),
                        asset_path: Some("Assets/Unsupported/case.asset".to_string()),
                        sub_asset_id: None,
                        object_name: Some("case".to_string()),
                    },
                    owning_feature_id: None,
                    tier: None,
                    state: None,
                    observable_consequence: None,
                },
                ProjectAssetEntry {
                    family: "Unassessed vendor content".to_string(),
                    content: ContentIdentity {
                        guid: Some("guid-unassessed-case".to_string()),
                        asset_path: Some("Assets/Vendor/unassessed.asset".to_string()),
                        sub_asset_id: None,
                        object_name: Some("unassessed".to_string()),
                    },
                    owning_feature_id: None,
                    tier: None,
                    state: None,
                    observable_consequence: None,
                },
                ProjectAssetEntry {
                    family: "URP Scene".to_string(),
                    content: ContentIdentity {
                        guid: Some("guid-playable-scene".to_string()),
                        asset_path: Some("Assets/Scenes/Playable.unity".to_string()),
                        sub_asset_id: None,
                        object_name: Some("Playable".to_string()),
                    },
                    owning_feature_id: Some("REN-006".to_string()),
                    tier: Some(Tier::Native),
                    state: Some(CompatibilityState::Supported),
                    observable_consequence: None,
                },
            ],
            settings: Vec::new(),
            additional_findings: Vec::new(),
            track_absent_catalog_features: Vec::new(),
        };

        let assessment = assess_project(&content);

        let unsupported = assessment.report.findings.iter().find(|finding| {
            finding.content.asset_path.as_deref() == Some("Assets/Unsupported/case.asset")
        });
        assert!(
            unsupported.is_some(),
            "CMP-002 Story 1: recognized family '{}' must produce a finding",
            entry.family
        );
        if let Some(finding) = unsupported {
            assert_eq!(
                finding.state,
                CompatibilityState::Unsupported,
                "CMP-002 Story 1: recognized family '{}' must be reported Unsupported",
                entry.family
            );
            assert_eq!(finding.feature_id, entry.owning_feature_id);
            assert_eq!(finding.observable_consequence, entry.observable_consequence);
        }

        let unassessed = assessment.report.findings.iter().find(|finding| {
            finding.content.asset_path.as_deref() == Some("Assets/Vendor/unassessed.asset")
        });
        assert!(
            unassessed.is_some(),
            "CMP-002 Story 1: unassessed content must stay explicit"
        );
        if let Some(finding) = unassessed {
            assert_eq!(
                finding.state,
                CompatibilityState::Unknown,
                "CMP-002 Story 1: unassessed content must never be reported Supported"
            );
        }

        assert!(
            assessment
                .playable_scenes
                .contains(&"Assets/Scenes/Playable.unity".to_string()),
            "CMP-002 Story 1: the rest of the project must remain editable and playable"
        );
        assert_eq!(assessment.overall_state, CompatibilityState::Unsupported);

        for finding in &assessment.report.findings {
            assert!(
                !finding.behavior.trim().is_empty(),
                "CMP-002 Story 1: every finding must name the assessed behavior"
            );
            assert!(
                !finding.observable_consequence.trim().is_empty(),
                "CMP-002 Story 1: every finding must state its observable consequence"
            );
            assert!(
                !finding.conditions.is_empty(),
                "CMP-002 Story 1: every finding must state the conditions under which it applies"
            );
            assert!(
                is_present(finding),
                "CMP-002 Story 1: every present finding must name the affected content"
            );
        }
    }
}

#[test]
fn cmp_002_story_19_report_findings_carry_the_compatibility_tier_of_their_owning_catalog_feature() {
    use unitas_core::compat::{
        assess_project, PackageSource, ProjectAssetEntry, ProjectContent, ProjectPackageEntry,
        ProjectSettingEntry,
    };

    let content = ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![
            ProjectPackageEntry {
                package_id: "com.vendor.unlisted".to_string(),
                version: "1.0.0".to_string(),
                source: PackageSource::ScopedRegistry,
                is_locally_modified_embedded: false,
            },
            ProjectPackageEntry {
                package_id: "com.unity.visualscripting".to_string(),
                version: "1.9.1".to_string(),
                source: PackageSource::Registry,
                is_locally_modified_embedded: false,
            },
        ],
        assets: vec![ProjectAssetEntry {
            family: "Enlighten realtime global illumination".to_string(),
            content: ContentIdentity {
                guid: Some("guid-enlighten-settings".to_string()),
                asset_path: Some("Assets/Enlighten/RealtimeLighting.lighting".to_string()),
                sub_asset_id: None,
                object_name: Some("RealtimeLighting".to_string()),
            },
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/QualitySettings.asset".to_string(),
            family: "Quality".to_string(),
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        additional_findings: Vec::new(),
        track_absent_catalog_features: vec!["PHY-004".to_string()],
    };

    let assessment = assess_project(&content);

    assert!(!assessment.report.findings.is_empty());
    for finding in &assessment.report.findings {
        let catalog = lookup_catalog_feature(&finding.feature_id);
        assert!(
            catalog.is_some(),
            "CMP-002 Story 19: finding feature '{}' must name a catalog feature",
            finding.feature_id
        );
        if let Some(catalog) = catalog {
            assert_eq!(
                finding.tier, catalog.tier,
                "CMP-002 Story 19: finding for '{}' must carry the compatibility tier of its owning catalog feature",
                finding.feature_id
            );
        }
    }
}

const OWNED_FEATURE_IDS: [&str; 26] = [
    "PRJ-001", "PRJ-002", "PRJ-003", "PRJ-004", "PRJ-005", "PRJ-006", "PRJ-007", "PRJ-008",
    "SER-001", "SER-002", "SER-003", "SER-004", "SER-005", "PHY-001", "PHY-002", "PHY-003",
    "PHY-004", "NAV-001", "ENV-001", "ANI-001", "ANI-002", "CIN-001", "REN-004", "EDT-007",
    "CMP-002", "BLD-005",
];

fn workspace_file(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

fn empty_project() -> unitas_core::compat::ProjectContent {
    unitas_core::compat::ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: Vec::new(),
        assets: Vec::new(),
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    }
}

#[test]
fn cmp_002_every_owned_feature_reports_a_real_state_from_assess_project() {
    use unitas_core::compat::{assess_project, CompatibilityState};

    let assessment = assess_project(&empty_project());

    let reported: Vec<_> = assessment
        .owned_feature_states
        .iter()
        .filter(|state| OWNED_FEATURE_IDS.contains(&state.feature_id.as_str()))
        .collect();

    assert_eq!(
        reported.len(),
        OWNED_FEATURE_IDS.len(),
        "CMP-002: assess_project must report a state for every owned feature id"
    );

    for state in reported {
        assert!(
            matches!(
                state.state,
                CompatibilityState::Supported
                    | CompatibilityState::PartiallySupported
                    | CompatibilityState::Unsupported
            ),
            "CMP-002: '{}' must report a real state, never Unknown or an external state",
            state.feature_id
        );
        assert!(
            !state.capability.trim().is_empty(),
            "CMP-002: '{}' must name the capability it assessed",
            state.feature_id
        );
        if state.state != CompatibilityState::Supported {
            let gap = state.gap.as_deref().unwrap_or_default();
            assert!(
                !gap.trim().is_empty(),
                "CMP-002: '{}' must name the gap that keeps it from Supported",
                state.feature_id
            );
            assert!(
                gap.contains(".rs"),
                "CMP-002: the gap for '{}' must name the module where the capability is missing: {}",
                state.feature_id,
                gap
            );
        }
    }
}

#[test]
fn cmp_002_owned_capability_sources_exist_in_the_workspace_and_drive_the_state() {
    use unitas_core::compat::{
        lookup_owned_feature_capability, CompatibilityState, OWNED_FEATURE_CAPABILITIES,
    };

    for capability in OWNED_FEATURE_CAPABILITIES {
        assert!(
            !capability.parts.is_empty(),
            "CMP-002: '{}' must name the capabilities it probes",
            capability.feature_id
        );
        assert!(
            lookup_owned_feature_capability(capability.feature_id).is_some(),
            "CMP-002: '{}' must be reachable by id",
            capability.feature_id
        );

        let demonstrated = capability
            .parts
            .iter()
            .filter(|part| !part.evidence.is_empty())
            .count();
        let missing = capability.parts.len() - demonstrated;
        if demonstrated == capability.parts.len() {
            assert_eq!(
                capability.state(),
                CompatibilityState::Supported,
                "CMP-002: '{}' demonstrates every probed capability",
                capability.feature_id
            );
        } else if demonstrated == 0 {
            assert_eq!(
                capability.state(),
                CompatibilityState::Unsupported,
                "CMP-002: '{}' demonstrates no probed capability",
                capability.feature_id
            );
        } else {
            assert!(
                missing > 0,
                "CMP-002: '{}' demonstrates part of its probed capability",
                capability.feature_id
            );
            assert_eq!(
                capability.state(),
                CompatibilityState::PartiallySupported,
                "CMP-002: '{}' demonstrates part of its probed capability",
                capability.feature_id
            );
        }

        for part in capability.parts {
            assert!(
                workspace_file(part.location).is_file(),
                "CMP-002: the capability source '{}' of '{}' must exist in the workspace",
                part.location,
                capability.feature_id
            );
            if part.evidence.is_empty() {
                assert!(
                    part.missing.is_some(),
                    "CMP-002: the missing capability '{}' of '{}' must name the gap",
                    part.behavior,
                    capability.feature_id
                );
            } else {
                for evidence in part.evidence {
                    let mut split = evidence.splitn(2, "::");
                    let file = split.next().unwrap_or_default();
                    let test_name = split.next().unwrap_or_default();
                    assert!(
                        !test_name.is_empty(),
                        "CMP-002: the evidence '{}' of '{}' must be written as path::test_name",
                        evidence,
                        capability.feature_id
                    );
                    let path = workspace_file(file);
                    assert!(
                        path.is_file(),
                        "CMP-002: the evidence file '{}' of '{}' must exist in the workspace",
                        file,
                        capability.feature_id
                    );
                    let source = std::fs::read_to_string(&path).unwrap_or_default();
                    assert!(
                        !source.is_empty(),
                        "CMP-002: the evidence file '{}' of '{}' must be readable",
                        file,
                        capability.feature_id
                    );
                    assert!(
                        source.contains(&format!("fn {test_name}(")),
                        "CMP-002: the evidence '{}' of '{}' must name a test that exists in the workspace",
                        evidence,
                        capability.feature_id
                    );
                }
            }
        }
    }
}

#[test]
fn cmp_002_owned_capabilities_match_the_catalog_and_the_familiar_differences() {
    use unitas_core::compat::{
        lookup_catalog_feature, lookup_familiar_behavior_differences, Tier,
        OWNED_FEATURE_CAPABILITIES,
    };

    for capability in OWNED_FEATURE_CAPABILITIES {
        let catalog = lookup_catalog_feature(capability.feature_id);
        assert!(
            catalog.is_some(),
            "CMP-002: owned feature '{}' must be a catalog feature",
            capability.feature_id
        );
        if let Some(catalog) = catalog {
            assert_eq!(
                capability.owning_spec, catalog.owning_spec,
                "CMP-002: owned feature '{}' must name the spec that owns it",
                capability.feature_id
            );
            if catalog.tier == Tier::Familiar {
                assert!(
                    lookup_familiar_behavior_differences(capability.feature_id).is_some(),
                    "CMP-002: a Familiar owned feature '{}' must list its differences from Unity 2022.3 LTS",
                    capability.feature_id
                );
            }
        }
    }
}

#[test]
fn cmp_002_project_findings_take_the_probe_state_and_name_the_gap_and_its_location() {
    use unitas_core::compat::{
        assess_project, CompatibilityState, PackageSource, ProjectAssetEntry, ProjectContent,
        ProjectPackageEntry, ProjectSettingEntry,
    };

    let content = ProjectContent {
        packages: vec![ProjectPackageEntry {
            package_id: "com.unity.collab-proxy".to_string(),
            version: "2.3.1".to_string(),
            source: PackageSource::Registry,
            is_locally_modified_embedded: false,
        }],
        assets: vec![ProjectAssetEntry {
            family: "Texture2D".to_string(),
            content: ContentIdentity {
                guid: Some("guid-texture".to_string()),
                asset_path: Some("Assets/Textures/Stone.png".to_string()),
                sub_asset_id: None,
                object_name: Some("Stone".to_string()),
            },
            owning_feature_id: Some("PRJ-004".to_string()),
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/QualitySettings.asset".to_string(),
            family: "Quality".to_string(),
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        ..empty_project()
    };

    let assessment = assess_project(&content);

    let texture = assessment
        .report
        .findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("Assets/Textures/Stone.png"));
    assert!(
        texture.is_some(),
        "CMP-002: the imported texture must produce a finding"
    );
    if let Some(texture) = texture {
        assert_eq!(texture.feature_id, "PRJ-004");
        assert_eq!(
            texture.state,
            CompatibilityState::PartiallySupported,
            "CMP-002: the texture import is partially supported because no compressed texture encoder exists"
        );
        assert!(
            texture
                .observable_consequence
                .contains("no BCn, ASTC, or ETC2 texture encoder"),
            "CMP-002: the texture finding must name the precise gap: {}",
            texture.observable_consequence
        );
        assert!(
            texture
                .observable_consequence
                .contains("crates/unitas-asset/src/texture/import.rs"),
            "CMP-002: the texture finding must name where the gap is: {}",
            texture.observable_consequence
        );
    }

    let quality =
        assessment.report.findings.iter().find(|f| {
            f.content.asset_path.as_deref() == Some("ProjectSettings/QualitySettings.asset")
        });
    assert!(
        quality.is_some(),
        "CMP-002: the project setting must produce a finding"
    );
    if let Some(quality) = quality {
        assert_eq!(quality.feature_id, "PRJ-007");
        assert_eq!(
            quality.state,
            CompatibilityState::PartiallySupported,
            "CMP-002: a project setting is partially supported because only some settings are represented"
        );
        assert!(
            quality.observable_consequence.contains("tags and layers"),
            "CMP-002: the setting finding must name the precise gap: {}",
            quality.observable_consequence
        );
        assert!(
            quality
                .observable_consequence
                .contains("crates/unitas-asset/src/project_settings.rs"),
            "CMP-002: the setting finding must name where the gap is: {}",
            quality.observable_consequence
        );
    }

    let collab = assessment
        .report
        .findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("Packages/com.unity.collab-proxy"));
    assert!(
        collab.is_some(),
        "CMP-002: the package must produce a finding"
    );
    if let Some(collab) = collab {
        assert_eq!(collab.feature_id, "EDT-007");
        assert_eq!(
            collab.state,
            CompatibilityState::PartiallySupported,
            "CMP-002: the version control package is partially supported because no version-control client is provided"
        );
        assert!(
            collab
                .observable_consequence
                .contains("no version-control client is provided"),
            "CMP-002: the package finding must name the precise gap: {}",
            collab.observable_consequence
        );
        assert!(
            collab
                .observable_consequence
                .contains("crates/unitas-asset/src/merge.rs"),
            "CMP-002: the package finding must name where the gap is: {}",
            collab.observable_consequence
        );
    }
}

#[test]
fn cmp_002_a_project_using_a_recognized_unsupported_family_reports_unsupported_with_its_gap() {
    use unitas_core::compat::{
        assess_project, CompatibilityState, ProjectAssetEntry, ProjectContent,
    };

    let content = ProjectContent {
        assets: vec![
            ProjectAssetEntry {
                family: "High Definition Render Pipeline".to_string(),
                content: ContentIdentity {
                    guid: Some("guid-hdrp".to_string()),
                    asset_path: Some("Assets/HDRP/SkyVolume.asset".to_string()),
                    sub_asset_id: None,
                    object_name: Some("SkyVolume".to_string()),
                },
                owning_feature_id: None,
                tier: None,
                state: None,
                observable_consequence: None,
            },
            ProjectAssetEntry {
                family: "URP Scene".to_string(),
                content: ContentIdentity {
                    guid: Some("guid-urp".to_string()),
                    asset_path: Some("Assets/Scenes/Main.unity".to_string()),
                    sub_asset_id: None,
                    object_name: Some("Main".to_string()),
                },
                owning_feature_id: Some("REN-006".to_string()),
                tier: Some(Tier::Native),
                state: Some(CompatibilityState::Supported),
                observable_consequence: None,
            },
        ],
        ..empty_project()
    };

    let assessment = assess_project(&content);

    let hdrp = assessment
        .report
        .findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("Assets/HDRP/SkyVolume.asset"));
    assert!(
        hdrp.is_some(),
        "CMP-002: recognized content must produce a finding"
    );
    if let Some(hdrp) = hdrp {
        assert_eq!(
            hdrp.state,
            CompatibilityState::Unsupported,
            "CMP-002: the compatibility contract defines Unsupported as recognized content whose required behavior is unavailable"
        );
        assert!(
            hdrp.observable_consequence
                .contains("HDRP materials, volumes, and pipeline assets do not render"),
            "CMP-002: the finding must name the excluded behavior: {}",
            hdrp.observable_consequence
        );
        assert!(
            hdrp.observable_consequence
                .contains("URP and Built-in scenes remain editable and playable"),
            "CMP-002: the finding must name what stays usable: {}",
            hdrp.observable_consequence
        );
    }

    assert!(
        assessment
            .playable_scenes
            .contains(&"Assets/Scenes/Main.unity".to_string()),
        "CMP-002: unrelated content stays playable"
    );
    assert!(
        !assessment
            .present_supported_features
            .contains(&"REN-006".to_string())
            || assessment.overall_state != CompatibilityState::Supported,
        "CMP-002: the project state must reflect the unsupported content"
    );
}
