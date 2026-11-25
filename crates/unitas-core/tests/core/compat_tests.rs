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
fn compat_reassess_updates_only_the_matching_finding() {
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
    assert_eq!(reassessed.findings[1], untouched);
    assert_eq!(
        reassessed.packages,
        vec![
            summary("PRJ", CompatibilityState::Unsupported, &["PRJ-004"]),
            summary("REN", CompatibilityState::Supported, &["REN-006"]),
        ]
    );
}

#[test]
fn compat_reassess_appends_a_finding_for_unseen_content() {
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
fn compat_package_matrix_reports_supported_urp_and_input_system_and_unknown_for_unlisted_or_preview(
) {
    use unitas_core::compat::{
        assess_package, lookup_package_matrix, lookup_unsupported_family,
        RECOGNIZED_UNSUPPORTED_FAMILIES, SUPPORTED_PACKAGE_MATRIX,
    };

    assert_eq!(SUPPORTED_PACKAGE_MATRIX.len(), 29);
    assert_eq!(RECOGNIZED_UNSUPPORTED_FAMILIES.len(), 11);

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
    assert_eq!(unlisted.state, CompatibilityState::Unknown);
    assert!(unlisted.tier.is_none());

    let preview = assess_package("com.unity.inputsystem", "1.8.0-pre.1", false);
    assert_eq!(preview.state, CompatibilityState::Unknown);

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
fn compat_assess_project_reports_supported_urp_and_input_system_and_unknown_unlisted_or_preview() {
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
fn compat_assess_project_isolates_hdrp_and_visual_scripting_while_urp_scenes_remain_playable_and_updates_on_removal(
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
        Err(FindingSubmissionError::SupportedFindingClaimsUnityDifference {
            feature_id: "PHY-001".to_string(),
            tier: Tier::Semantic,
        })
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
                assert_eq!(merged_report.summary().overall_state, CompatibilityState::Unsupported);
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
        CompatibilityReport::from_canonical_text("UNITAS_COMPAT_REPORT v1\nmalformed_line_no_colon\n"),
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


