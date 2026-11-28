use unitas_core::compat::{
    assess_package, assess_project, build_familiar_finding, build_semantic_finding, is_present,
    lookup_familiar_behavior_differences, lookup_package_matrix, lookup_unsupported_family,
    parse_canonical, render_canonical, submit_finding, submit_findings, AssessedPackage,
    CanonicalReportParseError, CompatibilityReport, CompatibilityState, ContentIdentity, Finding,
    FindingInput, FindingRejection, FindingSubmissionError, OwnedCapabilityPart,
    OwnedFeatureCapability, PackageSource, PackageSummary, ProjectAssetEntry,
    ProjectCompatibilityAssessment, ProjectContent, ProjectPackageEntry, ProjectSettingEntry, Tier,
    RECOGNIZED_UNSUPPORTED_FAMILIES, SUPPORTED_PACKAGE_MATRIX,
};

pub fn content(path: &str) -> ContentIdentity {
    ContentIdentity {
        guid: Some(format!("guid-{path}")),
        asset_path: Some(path.to_string()),
        sub_asset_id: None,
        object_name: None,
    }
}

pub fn finding(feature_id: &str, tier: Tier, state: CompatibilityState, path: &str) -> Finding {
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

pub fn summary(
    package_name: &str,
    state: CompatibilityState,
    feature_ids: &[&str],
) -> PackageSummary {
    PackageSummary {
        package_name: package_name.to_string(),
        state,
        feature_ids: feature_ids.iter().map(|id| (*id).to_string()).collect(),
    }
}

pub fn version_in_line(line: &str) -> String {
    if line == "Released or verified" {
        return "1.0.0".to_string();
    }
    match line.strip_suffix(".x") {
        Some(prefix) => format!("{prefix}.0"),
        None => line.to_string(),
    }
}

pub fn workspace_file(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

pub fn declares_test(source: &str, test_name: &str) -> bool {
    source.contains(&format!("fn {test_name}("))
        || source.contains(&format!("void {test_name}("))
        || source.contains(&format!("Task {test_name}("))
}

pub fn empty_project() -> ProjectContent {
    ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: Vec::new(),
        assets: Vec::new(),
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    }
}

pub fn is_feature_id(cell: &str) -> bool {
    let mut parts = cell.split('-');
    let prefix = parts.next().unwrap_or_default();
    let number = parts.next().unwrap_or_default();
    parts.next().is_none()
        && (2..=3).contains(&prefix.len())
        && prefix
            .chars()
            .all(|character| character.is_ascii_uppercase())
        && !number.is_empty()
        && number.chars().all(|character| character.is_ascii_digit())
}

pub fn feature_catalog_rows() -> Vec<(String, String, String)> {
    let source = std::fs::read_to_string(workspace_file(
        "docs/product-specification/feature-catalog.md",
    ))
    .unwrap_or_default();
    let mut rows = Vec::new();
    for line in source.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 6 || !is_feature_id(cells[1]) {
            continue;
        }
        let tier = cells
            .iter()
            .copied()
            .find(|cell| matches!(*cell, "Exact" | "Semantic" | "Familiar" | "Native"))
            .unwrap_or(cells[3]);
        let spec_cell = cells
            .iter()
            .copied()
            .find(|cell| cell.contains(".md"))
            .unwrap_or(cells[4]);
        let spec = spec_cell
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map_or_else(
                || spec_cell.trim_matches('`').to_string(),
                |(path, _)| path.trim_matches('`').to_string(),
            );
        rows.push((cells[1].to_string(), tier.to_string(), spec));
    }
    rows
}

pub fn assert_supported_package_matrix_entries() {
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
}

pub fn assert_specific_package_assessments() {
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
}

pub fn assert_unsupported_and_partial_packages() {
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

fn pkg_entry(
    package_id: &str,
    version: &str,
    source: PackageSource,
    is_locally_modified_embedded: bool,
) -> ProjectPackageEntry {
    ProjectPackageEntry {
        package_id: package_id.to_string(),
        version: version.to_string(),
        source,
        is_locally_modified_embedded,
    }
}

fn asset_entry(
    family: &str,
    guid: &str,
    path: &str,
    name: &str,
    owning_feature_id: Option<&str>,
    tier: Option<Tier>,
    state: Option<CompatibilityState>,
) -> ProjectAssetEntry {
    ProjectAssetEntry {
        family: family.to_string(),
        content: ContentIdentity {
            guid: Some(guid.to_string()),
            asset_path: Some(path.to_string()),
            sub_asset_id: None,
            object_name: Some(name.to_string()),
        },
        owning_feature_id: owning_feature_id.map(str::to_string),
        tier,
        state,
        observable_consequence: None,
    }
}

pub fn build_story_6_project_content() -> ProjectContent {
    ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![
            pkg_entry(
                "com.unity.render-pipelines.universal",
                "14.0.10",
                PackageSource::Registry,
                false,
            ),
            pkg_entry(
                "com.unity.inputsystem",
                "1.7.0",
                PackageSource::Registry,
                false,
            ),
            pkg_entry(
                "com.vendor.unlisted",
                "2.0.0",
                PackageSource::ScopedRegistry,
                false,
            ),
            pkg_entry(
                "com.unity.addressables",
                "1.22.0-pre.1",
                PackageSource::Registry,
                false,
            ),
        ],
        assets: vec![asset_entry(
            "URP Scene",
            "guid-main-scene",
            "Assets/Scenes/Main.unity",
            "Main",
            Some("SER-002"),
            Some(Tier::Exact),
            Some(CompatibilityState::Supported),
        )],
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
    }
}

pub fn assert_story_6_assessed_packages(packages: &[AssessedPackage]) {
    let urp = packages
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

    let input = packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.inputsystem");
    assert!(input.is_some());
    if let Some(input) = input {
        assert_eq!(input.state, CompatibilityState::Supported);
        assert_eq!(input.tier, Some(Tier::Familiar));
        assert_eq!(input.owning_spec.as_deref(), Some("ui-audio-input.md"));
        assert_eq!(input.owning_feature_id.as_deref(), Some("INP-002"));
    }

    let unlisted = packages
        .iter()
        .find(|pkg| pkg.package_id == "com.vendor.unlisted");
    assert!(unlisted.is_some());
    if let Some(unlisted) = unlisted {
        assert_eq!(unlisted.state, CompatibilityState::Unknown);
    }

    let preview = packages
        .iter()
        .find(|pkg| pkg.package_id == "com.unity.addressables");
    assert!(preview.is_some());
    if let Some(preview) = preview {
        assert_eq!(preview.state, CompatibilityState::Unknown);
    }
}

fn story_5_project_with_assets(
    package: ProjectPackageEntry,
    assets: Vec<ProjectAssetEntry>,
) -> ProjectCompatibilityAssessment {
    assess_project(&ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![package],
        assets,
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    })
}

pub fn build_story_5_reassessment_pair() -> (
    ProjectCompatibilityAssessment,
    ProjectCompatibilityAssessment,
) {
    let urp_scene = asset_entry(
        "URP Scene",
        "guid-playable-scene",
        "Assets/Scenes/Playable.unity",
        "Playable",
        Some("REN-006"),
        Some(Tier::Native),
        Some(CompatibilityState::Supported),
    );
    let hdrp_asset = asset_entry(
        "High Definition Render Pipeline",
        "guid-hdrp-volume",
        "Assets/HDRP/SkyVolume.asset",
        "SkyVolume",
        None,
        None,
        None,
    );
    let vendor_asset = asset_entry(
        "Unassessed vendor content",
        "guid-vendor-plugin",
        "Assets/Vendor/plugin.asset",
        "plugin",
        None,
        None,
        None,
    );
    let package = pkg_entry(
        "com.unity.render-pipelines.universal",
        "14.0.10",
        PackageSource::Registry,
        false,
    );

    let before = story_5_project_with_assets(
        package.clone(),
        vec![urp_scene.clone(), hdrp_asset, vendor_asset.clone()],
    );
    let after = story_5_project_with_assets(package, vec![urp_scene, vendor_asset]);
    (before, after)
}

fn assert_unaffected_findings_preserved(
    before: &ProjectCompatibilityAssessment,
    after: &ProjectCompatibilityAssessment,
) {
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
}

pub fn assert_story_5_reassessment_outcomes(
    before: &ProjectCompatibilityAssessment,
    after: &ProjectCompatibilityAssessment,
) {
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
    assert_unaffected_findings_preserved(before, after);
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

pub fn build_story_8_modified_embedded_content() -> ProjectContent {
    ProjectContent {
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
    }
}

fn story_7_initial_assets() -> (ProjectAssetEntry, Vec<ProjectAssetEntry>) {
    let urp_scene = asset_entry(
        "URP Scene",
        "guid-urp-scene",
        "Assets/Scenes/GameplayUrp.unity",
        "GameplayUrp",
        Some("REN-006"),
        Some(Tier::Native),
        Some(CompatibilityState::Supported),
    );
    let hdrp_asset = asset_entry(
        "High Definition Render Pipeline",
        "guid-hdrp-mat",
        "Assets/HDRP/SkyVolume.asset",
        "SkyVolume",
        None,
        None,
        None,
    );
    let vs_graph = asset_entry(
        "Visual Scripting",
        "guid-vs-graph",
        "Assets/Graphs/EnemyBrain.asset",
        "EnemyBrain",
        None,
        None,
        None,
    );
    (urp_scene.clone(), vec![urp_scene, hdrp_asset, vs_graph])
}

pub fn build_story_7_projects() -> (ProjectContent, ProjectContent) {
    let (urp_scene, initial_assets) = story_7_initial_assets();
    let urp_pkg = pkg_entry(
        "com.unity.render-pipelines.universal",
        "14.0.10",
        PackageSource::Registry,
        false,
    );
    let phy_rb = finding(
        "PHY-001",
        Tier::Semantic,
        CompatibilityState::Supported,
        "Assets/Prefabs/PlayerRB.prefab",
    );

    let initial_content = ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2021.3.35f1".to_string()),
        packages: vec![
            urp_pkg.clone(),
            pkg_entry(
                "com.unity.ide.rider",
                "3.0.28",
                PackageSource::Registry,
                false,
            ),
        ],
        assets: initial_assets,
        settings: vec![ProjectSettingEntry {
            setting_name: "ProjectSettings/LightingSettings.lighting".to_string(),
            family: "Enlighten realtime global illumination".to_string(),
            owning_feature_id: None,
            tier: None,
            state: None,
            observable_consequence: None,
        }],
        additional_findings: vec![phy_rb.clone()],
        track_absent_catalog_features: vec!["NET-001".to_string()],
    };
    let cleaned_content = ProjectContent {
        unitas_version: "0.1.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![urp_pkg],
        assets: vec![urp_scene],
        settings: Vec::new(),
        additional_findings: vec![phy_rb],
        track_absent_catalog_features: vec!["NET-001".to_string()],
    };
    (initial_content, cleaned_content)
}

pub fn assert_story_7_initial_assessment(initial: &ProjectCompatibilityAssessment) {
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
}

pub fn build_external_worktree_inputs() -> (FindingInput, FindingInput, FindingInput) {
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

    (scripting_input, rendering_input, editor_input)
}

pub fn assert_merged_external_worktree_report(report: &CompatibilityReport) {
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

pub fn assert_invalid_feature_and_tier_rejections(base_report: &CompatibilityReport) {
    let empty_feature = FindingInput::new(
        "   ",
        Tier::Semantic,
        CompatibilityState::Supported,
        content("Assets/Scripts/A.cs"),
        "transform hierarchy",
        "behaves as documented",
    );
    assert_eq!(
        submit_finding(base_report, empty_feature),
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
        submit_finding(base_report, unknown_feature),
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
        submit_finding(base_report, tier_mismatch),
        Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: "REN-001".to_string(),
            expected: Tier::Native,
            submitted: Tier::Exact,
        })
    );
}

pub fn assert_invalid_content_and_message_rejections(base_report: &CompatibilityReport) {
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
        submit_finding(base_report, empty_content),
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
        submit_finding(base_report, empty_behavior),
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
        submit_finding(base_report, empty_consequence),
        Err(FindingSubmissionError::EmptyObservableConsequence)
    );
}

pub fn assert_unity_diff_and_duplicate_rejections(base_report: &CompatibilityReport) {
    let duplicate_existing = FindingInput::new(
        "RUN-001",
        Tier::Semantic,
        CompatibilityState::PartiallySupported,
        content("Assets/Scripts/Bootstrap.cs"),
        "GameObject activation",
        "differs on order",
    );
    assert_eq!(
        submit_finding(base_report, duplicate_existing),
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
        submit_finding(base_report, supported_with_unity_diff),
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
        submit_finding(base_report, familiar_without_diff_on_partial),
        Err(FindingSubmissionError::MissingReliedUponUnityDifference {
            feature_id: "UI-001".to_string(),
            tier: Tier::Familiar,
            state: CompatibilityState::PartiallySupported,
        })
    );
}

pub fn assert_batch_duplicate_rejection(base_report: &CompatibilityReport) {
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
    let batch_res = submit_findings(base_report, vec![batch_dup_a, batch_dup_b]);
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

pub fn build_canonical_round_trip_inputs() -> (FindingInput, FindingInput, FindingInput) {
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

    (input_scr, input_ren, input_phy)
}

pub fn assert_canonical_render_and_round_trip(
    report_a: &CompatibilityReport,
    report_b: &CompatibilityReport,
) {
    assert_eq!(report_a, report_b);

    let rendered_a = report_a.to_canonical_text();
    let rendered_b = report_b.to_canonical_text();
    assert_eq!(rendered_a, rendered_b);
    assert_eq!(render_canonical(report_a), rendered_a);
    assert!(rendered_a.starts_with("UNITAS_COMPAT_REPORT v1\n"));
    assert!(rendered_a.contains("unitas_version: 0.3.0\n"));
    assert!(rendered_a.contains("overall_state: Unsupported\n"));

    let parsed = CompatibilityReport::from_canonical_text(&rendered_a);
    assert!(parsed.is_ok());
    if let Ok(round_tripped) = parsed {
        assert_eq!(&round_tripped, report_a);
        assert_eq!(round_tripped.to_canonical_text(), rendered_a);
        assert_eq!(parse_canonical(&rendered_a), Ok(round_tripped));
    }
}

pub fn assert_malformed_canonical_text_rejected() {
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
}

pub fn build_story_16_migrated_project_content() -> ProjectContent {
    ProjectContent {
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
    }
}

pub fn assert_semantic_finding_cites_cmp_001() {
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
}

pub fn assert_familiar_catalog_entry_and_finding(
    feature_id: &str,
    expected_spec: &str,
    expected_count: usize,
) {
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

pub fn build_story_20_project_content() -> ProjectContent {
    ProjectContent {
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
    }
}

pub fn build_story_1_family_project_content(family: &str) -> ProjectContent {
    ProjectContent {
        unitas_version: "0.4.0".to_string(),
        declared_unity_version: Some("2022.3.20f1".to_string()),
        packages: vec![pkg_entry(
            "com.unity.render-pipelines.universal",
            "14.0.10",
            PackageSource::Registry,
            false,
        )],
        assets: vec![
            asset_entry(
                family,
                "guid-unsupported-case",
                "Assets/Unsupported/case.asset",
                "case",
                None,
                None,
                None,
            ),
            asset_entry(
                "Unassessed vendor content",
                "guid-unassessed-case",
                "Assets/Vendor/unassessed.asset",
                "unassessed",
                None,
                None,
                None,
            ),
            asset_entry(
                "URP Scene",
                "guid-playable-scene",
                "Assets/Scenes/Playable.unity",
                "Playable",
                Some("REN-006"),
                Some(Tier::Native),
                Some(CompatibilityState::Supported),
            ),
        ],
        settings: Vec::new(),
        additional_findings: Vec::new(),
        track_absent_catalog_features: Vec::new(),
    }
}

pub fn assert_story_1_finding_completeness(findings: &[Finding]) {
    for finding in findings {
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

pub fn build_story_19_project_content() -> ProjectContent {
    ProjectContent {
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
    }
}

pub fn assert_capability_state_matches_parts(capability: &OwnedFeatureCapability) {
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
}

fn assert_evidence_entry_exists(feature_id: &str, evidence: &str) {
    let mut split = evidence.splitn(2, "::");
    let file = split.next().unwrap_or_default();
    let test_name = split.next().unwrap_or_default();
    assert!(
        !test_name.is_empty(),
        "CMP-002: the evidence '{evidence}' of '{feature_id}' must be written as path::test_name"
    );
    let path = workspace_file(file);
    assert!(
        path.is_file(),
        "CMP-002: the evidence file '{file}' of '{feature_id}' must exist in the workspace"
    );
    let source = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        !source.is_empty(),
        "CMP-002: the evidence file '{file}' of '{feature_id}' must be readable"
    );
    let declaration = if file.ends_with(".cs") {
        format!("void {test_name}(")
    } else if file.ends_with(".json") {
        format!("\"{test_name}\"")
    } else {
        format!("fn {test_name}(")
    };
    assert!(
        source.contains(&declaration) || declares_test(&source, test_name),
        "CMP-002: the evidence '{evidence}' of '{feature_id}' must name a test that exists in the workspace"
    );
}

pub fn assert_capability_part_evidence_exists(feature_id: &str, part: &OwnedCapabilityPart) {
    assert!(
        workspace_file(part.location).is_file(),
        "CMP-002: the capability source '{}' of '{}' must exist in the workspace",
        part.location,
        feature_id
    );
    if part.evidence.is_empty() {
        assert!(
            part.missing.is_some(),
            "CMP-002: the missing capability '{}' of '{}' must name the gap",
            part.behavior,
            feature_id
        );
        return;
    }
    for evidence in part.evidence {
        assert_evidence_entry_exists(feature_id, evidence);
    }
}

pub fn build_probe_state_test_content() -> ProjectContent {
    ProjectContent {
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
    }
}

fn assert_probe_collab_finding(findings: &[Finding]) {
    let collab = findings
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

pub fn assert_probe_state_findings(findings: &[Finding]) {
    let texture = findings
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
            CompatibilityState::Supported,
            "CMP-002: the texture import is supported because the animation clip importer and the platform texture encoders are implemented"
        );
        assert!(
            texture
                .observable_consequence
                .contains("content is editable and playable"),
            "CMP-002: the texture finding must report that the imported content is usable: {}",
            texture.observable_consequence
        );
    }

    let quality = findings
        .iter()
        .find(|f| f.content.asset_path.as_deref() == Some("ProjectSettings/QualitySettings.asset"));
    assert!(
        quality.is_some(),
        "CMP-002: the project setting must produce a finding"
    );
    if let Some(quality) = quality {
        assert_eq!(quality.feature_id, "PRJ-007");
        assert_eq!(
            quality.state,
            CompatibilityState::Supported,
            "CMP-002: a project setting is supported because every ProjectSettings family is represented"
        );
        assert!(
            quality
                .observable_consequence
                .contains("project setting is represented and honored"),
            "CMP-002: the setting finding must report that the setting is represented and honored: {}",
            quality.observable_consequence
        );
    }

    assert_probe_collab_finding(findings);
}

pub fn build_unsupported_family_gap_content() -> ProjectContent {
    ProjectContent {
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
    }
}

pub fn assert_sav_001_capability_parts(capability: &OwnedFeatureCapability) {
    assert_eq!(
        capability.owning_spec, "accessibility-localization-persistence.md",
        "CMP-002: SAV-001 must name the specification that owns it"
    );

    let demonstrated = capability
        .parts
        .iter()
        .filter(|part| !part.evidence.is_empty())
        .count();
    let gaps = capability
        .parts
        .iter()
        .filter(|part| part.evidence.is_empty())
        .count();
    assert!(
        demonstrated >= 6,
        "CMP-002: SAV-001 must record the acceptance items it demonstrates: {demonstrated}"
    );
    assert!(
        gaps >= 5,
        "CMP-002: SAV-001 must name the acceptance items it does not demonstrate: {gaps}"
    );
    for part in capability
        .parts
        .iter()
        .filter(|part| part.evidence.is_empty())
    {
        assert!(
            !part.missing.unwrap_or_default().trim().is_empty(),
            "CMP-002: the SAV-001 gap '{}' must name the missing behaviour",
            part.behavior
        );
    }
}

pub fn parse_catalog_tier(tier: &str) -> Option<Tier> {
    match tier {
        "Exact" => Some(Tier::Exact),
        "Semantic" => Some(Tier::Semantic),
        "Familiar" => Some(Tier::Familiar),
        "Native" => Some(Tier::Native),
        _ => None,
    }
}
