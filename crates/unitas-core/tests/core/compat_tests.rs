use super::compat_helpers::{
    assert_batch_duplicate_rejection, assert_canonical_render_and_round_trip,
    assert_capability_part_evidence_exists, assert_capability_state_matches_parts,
    assert_familiar_catalog_entry_and_finding, assert_invalid_content_and_message_rejections,
    assert_invalid_feature_and_tier_rejections, assert_malformed_canonical_text_rejected,
    assert_merged_external_worktree_report, assert_probe_state_findings,
    assert_sav_001_capability_parts, assert_semantic_finding_cites_cmp_001,
    assert_specific_package_assessments, assert_story_1_finding_completeness,
    assert_story_5_reassessment_outcomes, assert_story_6_assessed_packages,
    assert_story_7_initial_assessment, assert_supported_package_matrix_entries,
    assert_unity_diff_and_duplicate_rejections, assert_unsupported_and_partial_packages,
    build_canonical_round_trip_inputs, build_external_worktree_inputs,
    build_probe_state_test_content, build_story_16_migrated_project_content,
    build_story_19_project_content, build_story_1_family_project_content,
    build_story_20_project_content, build_story_5_reassessment_pair, build_story_6_project_content,
    build_story_7_projects, build_story_8_modified_embedded_content,
    build_unsupported_family_gap_content, content, empty_project, feature_catalog_rows, finding,
    parse_catalog_tier, summary, workspace_file,
};
use unitas_core::compat::{
    absence_finding, assess_package, assess_project, build_report, is_present,
    least_supported_state, lookup_catalog_feature, lookup_familiar_behavior_differences,
    lookup_owned_feature_capability, merge_findings, package_of, presence, probe_owned_feature,
    reassess, submit_finding, submit_findings, summarize_package, CompatibilityReport,
    CompatibilityState, ContentIdentity, FindingInput, Presence, Tier,
    FAMILIAR_BEHAVIOR_DIFFERENCES, OWNED_FEATURE_CAPABILITIES, RECOGNIZED_UNSUPPORTED_FAMILIES,
};

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

    let mut updated = assessed;
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
    assert_supported_package_matrix_entries();
    assert_specific_package_assessments();
    assert_unsupported_and_partial_packages();
}

#[test]
fn cmp_002_story_6_project_report_reports_urp_and_input_system_with_tiers_and_owning_specs_and_unknown_for_unlisted_or_preview(
) {
    let content = build_story_6_project_content();
    let assessment = assess_project(&content);

    assert_eq!(assessment.report.produced_by_unitas_version, "0.1.0");
    assert_eq!(
        assessment.effective_baseline_unity_version,
        "2022.3 LTS".to_string()
    );
    assert!(!assessment.migrated_from_earlier_lts);

    assert_story_6_assessed_packages(&assessment.assessed_packages);

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
    let (before, after) = build_story_5_reassessment_pair();
    assert_story_5_reassessment_outcomes(&before, &after);
}

#[test]
fn cmp_002_story_8_locally_modified_embedded_package_is_not_matched_and_is_assessed_as_third_party_content(
) {
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

    let assessment = assess_project(&build_story_8_modified_embedded_content());
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
    let (initial_content, cleaned_content) = build_story_7_projects();
    let initial = assess_project(&initial_content);
    assert_story_7_initial_assessment(&initial);

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

    assert_eq!(
        lookup_catalog_feature("RUN-001").map(|e| e.tier),
        Some(Tier::Semantic)
    );
    assert_eq!(
        lookup_catalog_feature("SCR-001").map(|e| e.tier),
        Some(Tier::Semantic)
    );

    let (scripting_input, rendering_input, editor_input) = build_external_worktree_inputs();

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
        assert_merged_external_worktree_report(&report);
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

    assert_invalid_feature_and_tier_rejections(&base_report);
    assert_invalid_content_and_message_rejections(&base_report);
    assert_unity_diff_and_duplicate_rejections(&base_report);
    assert_batch_duplicate_rejection(&base_report);
}

#[test]
fn compat_canonical_rendering_is_byte_identical_across_insertion_orders_and_round_trips() {
    let base_empty = build_report("0.3.0", Vec::new());
    let (input_scr, input_ren, input_phy) = build_canonical_round_trip_inputs();

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
        assert_canonical_render_and_round_trip(&report_a, &report_b);

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
    assert_malformed_canonical_text_rejected();

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
    let assessment = assess_project(&build_story_16_migrated_project_content());
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
    assert_semantic_finding_cites_cmp_001();

    let expected_counts = [
        ("ANI-001", "rendering-and-animation.md", 5usize),
        ("ANI-002", "rendering-and-animation.md", 3usize),
        ("CIN-001", "rendering-and-animation.md", 2usize),
        ("REN-004", "rendering-and-animation.md", 11usize),
        ("PHY-004", "physics-navigation-terrain.md", 3usize),
        ("NAV-001", "physics-navigation-terrain.md", 4usize),
        ("ENV-001", "physics-navigation-terrain.md", 2usize),
    ];

    assert_eq!(FAMILIAR_BEHAVIOR_DIFFERENCES.len(), 16);

    for (feature_id, expected_spec, expected_count) in expected_counts {
        assert_familiar_catalog_entry_and_finding(feature_id, expected_spec, expected_count);
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
    let assessment = assess_project(&build_story_20_project_content());

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
    for entry in RECOGNIZED_UNSUPPORTED_FAMILIES {
        let assessment = assess_project(&build_story_1_family_project_content(entry.family));

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

        assert_story_1_finding_completeness(&assessment.report.findings);
    }
}

#[test]
fn cmp_002_story_19_report_findings_carry_the_compatibility_tier_of_their_owning_catalog_feature() {
    let assessment = assess_project(&build_story_19_project_content());

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

const OWNED_FEATURE_IDS: [&str; 71] = [
    "PRJ-001", "PRJ-002", "PRJ-003", "PRJ-004", "PRJ-005", "PRJ-006", "PRJ-007", "PRJ-008",
    "SER-001", "SER-002", "SER-003", "SER-004", "SER-005", "PHY-001", "PHY-002", "PHY-003",
    "PHY-004", "NAV-001", "ENV-001", "ANI-001", "ANI-002", "CIN-001", "REN-001", "REN-002",
    "REN-003", "REN-004", "REN-005", "REN-006", "REN-007", "UI-001", "UI-002", "TXT-001",
    "INP-001", "INP-002", "AUD-001", "MED-001", "BLD-001", "BLD-004", "BLD-005", "PLT-001",
    "LIC-001", "PRD-001", "XR-001", "NET-001", "DIA-003", "EDT-004", "EDT-005", "EDT-006",
    "EDT-007", "EDT-008", "AUT-001", "AUT-002", "AUT-007", "AUT-008", "CMP-002", "CMP-004",
    "RUN-001", "RUN-002", "RUN-003", "RUN-004", "RUN-005", "RUN-006", "RUN-007", "RUN-008",
    "RUN-009", "RUN-010", "RUN-011", "DIA-001", "PRF-001", "ACC-001", "SAV-001",
];

#[test]
fn cmp_002_every_owned_feature_reports_a_real_state_from_assess_project() {
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
                gap.contains(".rs") || gap.contains(".cs"),
                "CMP-002: the gap for '{}' must name the module where the capability is missing: {}",
                state.feature_id,
                gap
            );
        }
    }
}

#[test]
fn cmp_002_owned_capability_sources_exist_in_the_workspace_and_drive_the_state() {
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

        assert_capability_state_matches_parts(capability);

        for part in capability.parts {
            assert_capability_part_evidence_exists(capability.feature_id, part);
        }
    }
}

#[test]
fn cmp_002_owned_capabilities_match_the_catalog_and_the_familiar_differences() {
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
    let assessment = assess_project(&build_probe_state_test_content());

    assert_probe_state_findings(&assessment.report.findings);

    for (feature_id, gap_location) in [
        ("EDT-007", "crates/unitas-asset/src/merge.rs"),
        ("BLD-005", "crates/unitas-build/src/bundle.rs"),
        ("ENV-001", "crates/unitas-physics/src/terrain/mod.rs"),
        ("REN-004", "crates/unitas-physics/src/particles.rs"),
    ] {
        let probed = assessment
            .owned_feature_states
            .iter()
            .find(|state| state.feature_id == feature_id);
        assert!(
            probed.is_some(),
            "CMP-002: '{feature_id}' must report a probed state"
        );
        if let Some(probed) = probed {
            assert_eq!(
                probed.state,
                CompatibilityState::PartiallySupported,
                "CMP-002: '{feature_id}' keeps a gap and stays partially supported"
            );
            let gap = probed.gap.as_deref().unwrap_or_default();
            assert!(
                gap.contains(gap_location),
                "CMP-002: the gap for '{feature_id}' must name where the capability is missing: {gap}"
            );
        }
    }
}

#[test]
fn cmp_002_a_project_using_a_recognized_unsupported_family_reports_unsupported_with_its_gap() {
    let assessment = assess_project(&build_unsupported_family_gap_content());

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

#[test]
fn cmp_002_the_owned_feature_ids_are_exactly_the_owned_set() {
    let owned: Vec<&str> = OWNED_FEATURE_CAPABILITIES
        .iter()
        .map(|capability| capability.feature_id)
        .collect();

    assert_eq!(
        owned.len(),
        OWNED_FEATURE_IDS.len(),
        "CMP-002: the owned capability table must hold one entry per owned feature id"
    );
    for id in OWNED_FEATURE_IDS {
        assert!(
            owned.contains(&id),
            "CMP-002: '{id}' must own a capability entry"
        );
    }
    for id in owned {
        assert!(
            OWNED_FEATURE_IDS.contains(&id),
            "CMP-002: '{id}' is not one of the owned feature ids"
        );
    }
}

#[test]
fn cmp_002_sav_001_records_the_demonstrated_parts_and_names_every_remaining_gap() {
    let capability = lookup_owned_feature_capability("SAV-001");
    assert!(
        capability.is_some(),
        "CMP-002: SAV-001 must own a capability entry"
    );
    if let Some(capability) = capability {
        assert_sav_001_capability_parts(capability);
    }

    let state = probe_owned_feature("SAV-001");
    assert!(
        state.is_some(),
        "CMP-002: SAV-001 must report a probed owned feature state"
    );
    if let Some(state) = state {
        assert_eq!(
            state.state,
            CompatibilityState::PartiallySupported,
            "CMP-002: SAV-001 demonstrates some of its behaviour and names the rest as gaps, so it stays Partially supported"
        );

        let gap = state.gap.unwrap_or_default();
        for named in [
            "IndexedDB",
            "JsonUtility",
            "XmlSerializer",
            "BinaryFormatter",
            "registry",
            "storage quota",
            "browser target",
        ] {
            assert!(
                gap.contains(named),
                "CMP-002: the SAV-001 gap must name '{named}': {gap}"
            );
        }
    }
}

#[test]
fn cmp_002_the_owned_feature_tiers_and_specs_match_the_authoritative_feature_catalog() {
    let rows = feature_catalog_rows();
    assert!(
        !rows.is_empty(),
        "CMP-002: the authoritative feature catalog must be readable"
    );

    for capability in OWNED_FEATURE_CAPABILITIES {
        let row = rows
            .iter()
            .find(|(id, _, _)| id == capability.feature_id)
            .cloned();
        assert!(
            row.is_some(),
            "CMP-002: '{}' must be listed in the authoritative feature catalog",
            capability.feature_id
        );
        let (_, tier, spec) = row.unwrap_or_default();
        assert_eq!(
            spec, capability.owning_spec,
            "CMP-002: '{}' must name the specification that owns it",
            capability.feature_id
        );
        let catalog = lookup_catalog_feature(capability.feature_id);
        assert!(
            catalog.is_some(),
            "CMP-002: '{}' must be a catalog feature",
            capability.feature_id
        );
        if let Some(catalog) = catalog {
            assert_eq!(
                catalog.owning_spec, spec,
                "CMP-002: '{}' must name the specification that owns it",
                capability.feature_id
            );
            let expected_tier = parse_catalog_tier(tier.as_str());
            assert!(
                expected_tier.is_some(),
                "CMP-002: the catalog tier '{}' of '{}' must be a compatibility tier",
                tier,
                capability.feature_id
            );
            if let Some(expected_tier) = expected_tier {
                assert_eq!(
                    catalog.tier, expected_tier,
                    "CMP-002: '{}' must keep the tier the authoritative catalog gives it",
                    capability.feature_id
                );
            }
        }
    }
}

#[test]
fn cmp_002_every_owning_spec_named_by_the_compatibility_data_resolves_to_a_specification() {
    use unitas_core::compat::{FEATURE_CATALOG, SUPPORTED_PACKAGE_MATRIX};

    let mut specs: Vec<&str> = FEATURE_CATALOG
        .iter()
        .map(|entry| entry.owning_spec)
        .collect();
    specs.extend(
        SUPPORTED_PACKAGE_MATRIX
            .iter()
            .map(|entry| entry.owning_spec),
    );
    specs.extend(
        RECOGNIZED_UNSUPPORTED_FAMILIES
            .iter()
            .map(|entry| entry.owning_spec),
    );
    specs.extend(
        OWNED_FEATURE_CAPABILITIES
            .iter()
            .map(|entry| entry.owning_spec),
    );

    for spec in specs {
        let path = workspace_file(&format!("docs/product-specification/{spec}"));
        assert!(
            path.is_file(),
            "CMP-002: the owning specification '{spec}' must exist in the workspace"
        );
    }
}

#[test]
fn cmp_002_a_setting_owned_by_a_feature_unitas_does_not_own_is_never_reported_supported() {
    use unitas_core::compat::{ProjectContent, ProjectSettingEntry};

    let content = ProjectContent {
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/TagManager.asset".to_string(),
            family: "Tags and layers".to_string(),
            owning_feature_id: Some("EDT-001".to_string()),
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        ..empty_project()
    };

    let assessment = assess_project(&content);

    let finding = assessment.report.findings.iter().find(|finding| {
        finding.content.asset_path.as_deref() == Some("ProjectSettings/TagManager.asset")
    });
    assert!(
        finding.is_some(),
        "CMP-002: the setting must produce a finding"
    );
    if let Some(finding) = finding {
        assert_eq!(finding.feature_id, "EDT-001");
        assert_eq!(
            finding.state,
            CompatibilityState::Unknown,
            "CMP-002: a setting owned by a feature unitas does not own must never be reported Supported"
        );
        assert!(
            finding.observable_consequence.contains("classification"),
            "CMP-002: the finding must say the setting could not be classified: {}",
            finding.observable_consequence
        );
    }
    assert!(
        assessment.unknown_features.contains(&"EDT-001".to_string()),
        "CMP-002: the unclassified setting must stay in the unknown features"
    );
    assert!(
        !assessment
            .present_supported_features
            .contains(&"EDT-001".to_string()),
        "CMP-002: the unclassified setting must never read Supported"
    );
}

#[test]
fn cmp_002_the_texture_import_capability_cites_tests_that_decode_psd_and_hdr() {
    let capability = lookup_owned_feature_capability("PRJ-004");
    assert!(
        capability.is_some(),
        "CMP-002: PRJ-004 must own a capability entry"
    );
    let evidence: Vec<&str> = capability
        .map(|capability| {
            capability
                .parts
                .iter()
                .flat_map(|part| part.evidence.iter().copied())
                .collect()
        })
        .unwrap_or_default();

    let dispatch =
        "crates/unitas-asset/tests/texture_decoders_tests.rs::u4_psd_and_hdr_dispatch_through_the_shared_texture_decoder";
    assert!(
        evidence.contains(&dispatch),
        "CMP-002: the texture import capability must cite the test that decodes PSD and HDR through the shared decoder"
    );

    let (file, test_name) = dispatch.split_once("::").unwrap_or_default();
    let source = std::fs::read_to_string(workspace_file(file)).unwrap_or_default();
    let start = source.find(&format!("fn {test_name}(")).unwrap_or_default();
    assert!(
        start > 0,
        "CMP-002: the cited test '{test_name}' must exist in '{file}'"
    );
    let body = &source[start..];
    let end = body.find("\n}").unwrap_or(body.len());
    let body = &body[..end];
    assert!(
        body.contains("decode_texture"),
        "CMP-002: the cited test must decode through the shared texture decoder"
    );
    assert!(
        body.contains(".PSD") && body.contains(".HDR"),
        "CMP-002: the cited test must decode a PSD file name and an HDR file name"
    );
}

#[test]
fn cmp_003_runtime_scripting_owned_features_report_honest_partially_supported_state_and_gaps() {
    for feature_id in ["RUN-001", "RUN-003", "RUN-004", "RUN-005", "RUN-009"] {
        let probed = probe_owned_feature(feature_id);
        assert!(
            probed.is_some(),
            "CMP-003: '{feature_id}' must return a probed owned feature state"
        );
        if let Some(probed) = probed {
            assert_eq!(probed.feature_id, feature_id);
            assert_eq!(probed.tier, Tier::Semantic);
            assert_eq!(
                probed.state,
                CompatibilityState::PartiallySupported,
                "CMP-003: '{feature_id}' has undemonstrated behavior and must report PartiallySupported, never Supported"
            );
            let gap = probed.gap.unwrap_or_default();
            assert!(
                gap.contains("missing:"),
                "CMP-003: '{feature_id}' must name its undemonstrated gap: {gap}"
            );
        }

        let capability = lookup_owned_feature_capability(feature_id);
        assert!(capability.is_some());
        if let Some(capability) = capability {
            assert_eq!(capability.owning_spec, "scripting-runtime.md");
            assert!(
                capability
                    .parts
                    .iter()
                    .any(|part| !part.evidence.is_empty()),
                "CMP-003: '{feature_id}' must record at least one demonstrated part"
            );
            assert!(
                capability
                    .parts
                    .iter()
                    .any(|part| part.evidence.is_empty() && part.missing.is_some()),
                "CMP-003: '{feature_id}' must record at least one undemonstrated gap part"
            );
        }
    }
}
