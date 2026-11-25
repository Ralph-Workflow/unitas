use unitas_core::compat::{
    absence_finding, build_report, is_present, least_supported_state, package_of, presence,
    reassess, summarize_package, CompatibilityReport, CompatibilityState, ContentIdentity, Finding,
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
fn compat_package_matrix_reports_supported_urp_and_input_system_and_unknown_for_unlisted_or_preview()
{
    use unitas_core::compat::{
        assess_package, lookup_package_matrix, lookup_unsupported_family,
        RECOGNIZED_UNSUPPORTED_FAMILIES, SUPPORTED_PACKAGE_MATRIX,
    };

    assert_eq!(SUPPORTED_PACKAGE_MATRIX.len(), 29);
    assert_eq!(RECOGNIZED_UNSUPPORTED_FAMILIES.len(), 11);

    let urp = assess_package("com.unity.render-pipelines.universal", "14.0.10", false);
    assert_eq!(urp.state, CompatibilityState::Supported);
    assert_eq!(urp.tier, Some(Tier::Native));
    assert_eq!(urp.owning_spec.as_deref(), Some("rendering-and-animation.md"));
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

    let hdrp = assess_package("com.unity.render-pipelines.high-definition", "14.0.10", false);
    assert_eq!(hdrp.state, CompatibilityState::Unsupported);
    assert_eq!(hdrp.owning_feature_id.as_deref(), Some("REN-006"));

    let visual_scripting = assess_package("com.unity.visualscripting", "1.9.1", false);
    assert_eq!(visual_scripting.state, CompatibilityState::Unsupported);
    assert_eq!(visual_scripting.owning_feature_id.as_deref(), Some("RUN-002"));

    let rider = assess_package("com.unity.ide.rider", "3.0.28", false);
    assert_eq!(rider.state, CompatibilityState::PartiallySupported);
    assert_eq!(rider.owning_feature_id.as_deref(), Some("DIA-002"));

    let collab = assess_package("com.unity.collab-proxy", "2.3.1", false);
    assert_eq!(collab.state, CompatibilityState::PartiallySupported);
    assert_eq!(collab.owning_feature_id.as_deref(), Some("EDT-007"));

    assert!(lookup_package_matrix("com.unity.shadergraph").is_some());
    assert!(lookup_unsupported_family("Enlighten realtime global illumination").is_some());
    assert!(lookup_unsupported_family("Unity-built AssetBundles and Addressables content").is_some());
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
        assert_eq!(urp.owning_spec.as_deref(), Some("rendering-and-animation.md"));
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

    assert!(assessment
        .absent_features
        .contains(&"PHY-004".to_string()));
    assert!(assessment
        .absent_features
        .contains(&"ENV-001".to_string()));
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
fn compat_assess_project_isolates_hdrp_and_visual_scripting_while_urp_scenes_remain_playable_and_updates_on_removal()
{
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
    assert!(initial.playable_scenes.contains(&"Assets/Scenes/GameplayUrp.unity".to_string()));

    let hdrp_finding = initial.report.findings.iter().find(|f| {
        f.content.asset_path.as_deref() == Some("Assets/HDRP/SkyVolume.asset")
    });
    assert!(hdrp_finding.is_some());
    if let Some(f) = hdrp_finding {
        assert_eq!(f.state, CompatibilityState::Unsupported);
        assert_eq!(f.feature_id, "REN-006");
        assert!(f
            .observable_consequence
            .contains("URP and Built-in scenes remain editable and playable"));
    }

    let vs_finding = initial.report.findings.iter().find(|f| {
        f.content.asset_path.as_deref() == Some("Assets/Graphs/EnemyBrain.asset")
    });
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

    let phy_finding = initial.report.findings.iter().find(|f| f.feature_id == "PHY-001");
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
