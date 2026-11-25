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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PackageMatrixEntry {
    pub package_id: &'static str,
    pub display_name: &'static str,
    pub version_line: &'static str,
    pub tier: Tier,
    pub state: CompatibilityState,
    pub owning_spec: &'static str,
    pub owning_feature_id: &'static str,
    pub state_notes: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UnsupportedFamilyEntry {
    pub family: &'static str,
    pub package_id: Option<&'static str>,
    pub recognized_form: &'static str,
    pub state: CompatibilityState,
    pub owning_spec: &'static str,
    pub owning_feature_id: &'static str,
    pub observable_consequence: &'static str,
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

pub const SUPPORTED_PACKAGE_MATRIX: &[PackageMatrixEntry] = &[
    PackageMatrixEntry {
        package_id: "com.unity.render-pipelines.universal",
        display_name: "Universal Render Pipeline",
        version_line: "14.x",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-006",
        state_notes: "Supported, including the Forward, Forward+, and Deferred rendering paths",
    },
    PackageMatrixEntry {
        package_id: "com.unity.shadergraph",
        display_name: "Shader Graph",
        version_line: "14.x",
        tier: Tier::Native,
        state: CompatibilityState::PartiallySupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-006",
        state_notes: "Partially supported: graph targets for HDRP and Visual Effect Graph are Unsupported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.postprocessing",
        display_name: "Post Processing v2",
        version_line: "3.x",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-003",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.sprite",
        display_name: "2D Sprite",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.tilemap",
        display_name: "2D Tilemap Editor",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.tilemap.extras",
        display_name: "2D Tilemap Extras",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.animation",
        display_name: "2D Animation",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.pixel-perfect",
        display_name: "2D Pixel Perfect",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.2d.psdimporter",
        display_name: "2D PSD Importer",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-005",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.cinemachine",
        display_name: "Cinemachine",
        version_line: "2.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "CIN-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.timeline",
        display_name: "Timeline",
        version_line: "1.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "CIN-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.ugui",
        display_name: "uGUI",
        version_line: "1.0.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "ui-audio-input.md",
        owning_feature_id: "UI-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.textmeshpro",
        display_name: "TextMeshPro",
        version_line: "3.0.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "ui-audio-input.md",
        owning_feature_id: "TXT-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.inputsystem",
        display_name: "Input System",
        version_line: "1.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "ui-audio-input.md",
        owning_feature_id: "INP-002",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.localization",
        display_name: "Localization",
        version_line: "1.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "accessibility-localization-persistence.md",
        owning_feature_id: "LOC-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.ai.navigation",
        display_name: "AI Navigation",
        version_line: "1.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "physics-navigation-terrain.md",
        owning_feature_id: "NAV-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.netcode.gameobjects",
        display_name: "Netcode for GameObjects",
        version_line: "1.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "build-platforms-xr-networking.md",
        owning_feature_id: "NET-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.xr.management",
        display_name: "XR Plugin Management",
        version_line: "4.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "build-platforms-xr-networking.md",
        owning_feature_id: "XR-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.xr.interaction.toolkit",
        display_name: "XR Interaction Toolkit",
        version_line: "2.x",
        tier: Tier::Familiar,
        state: CompatibilityState::Supported,
        owning_spec: "build-platforms-xr-networking.md",
        owning_feature_id: "XR-001",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.addressables",
        display_name: "Addressables",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-006",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.burst",
        display_name: "Burst",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Supported: results are equivalent within the declared floating-point mode",
    },
    PackageMatrixEntry {
        package_id: "com.unity.mathematics",
        display_name: "Mathematics",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.collections",
        display_name: "Collections",
        version_line: "Released or verified",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.nuget.newtonsoft-json",
        display_name: "Newtonsoft Json",
        version_line: "3.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-009",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.test-framework",
        display_name: "Unity Test Framework",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Supported,
        owning_spec: "diagnostics-profiling-playtesting.md",
        owning_feature_id: "TST-003",
        state_notes: "Supported",
    },
    PackageMatrixEntry {
        package_id: "com.unity.ide.visualstudio",
        display_name: "Visual Studio Editor",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::PartiallySupported,
        owning_spec: "diagnostics-profiling-playtesting.md",
        owning_feature_id: "DIA-002",
        state_notes: "Partially supported: project-file generation and debugger attachment are provided by DIA-002; the package preference UI is not drawn (RUN-010)",
    },
    PackageMatrixEntry {
        package_id: "com.unity.ide.rider",
        display_name: "JetBrains Rider Editor",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::PartiallySupported,
        owning_spec: "diagnostics-profiling-playtesting.md",
        owning_feature_id: "DIA-002",
        state_notes: "Partially supported: project-file generation and debugger attachment are provided by DIA-002; the package preference UI is not drawn (RUN-010)",
    },
    PackageMatrixEntry {
        package_id: "com.unity.ide.vscode",
        display_name: "VS Code Editor",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::PartiallySupported,
        owning_spec: "diagnostics-profiling-playtesting.md",
        owning_feature_id: "DIA-002",
        state_notes: "Partially supported: project-file generation and debugger attachment are provided by DIA-002; the package preference UI is not drawn (RUN-010)",
    },
    PackageMatrixEntry {
        package_id: "com.unity.collab-proxy",
        display_name: "Version Control",
        version_line: "Released or verified",
        tier: Tier::Native,
        state: CompatibilityState::PartiallySupported,
        owning_spec: "editor-and-authoring.md",
        owning_feature_id: "EDT-007",
        state_notes: "Partially supported: the package and its settings are preserved as project content; no version-control client is provided",
    },
];

pub const RECOGNIZED_UNSUPPORTED_FAMILIES: &[UnsupportedFamilyEntry] = &[
    UnsupportedFamilyEntry {
        family: "High Definition Render Pipeline",
        package_id: Some("com.unity.render-pipelines.high-definition"),
        recognized_form: "com.unity.render-pipelines.high-definition assets, materials, volumes, and settings",
        state: CompatibilityState::Unsupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-006",
        observable_consequence: "HDRP materials, volumes, and pipeline assets do not render; URP and Built-in scenes remain editable and playable",
    },
    UnsupportedFamilyEntry {
        family: "Custom scriptable render pipelines",
        package_id: None,
        recognized_form: "A project render pipeline asset that is neither the Built-in Render Pipeline nor URP",
        state: CompatibilityState::Unsupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-006",
        observable_consequence: "The custom render pipeline asset does not execute; Built-in and URP scenes remain usable",
    },
    UnsupportedFamilyEntry {
        family: "Entities and Entities Graphics",
        package_id: Some("com.unity.entities"),
        recognized_form: "com.unity.entities, com.unity.entities.graphics, and com.unity.rendering.hybrid content",
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        observable_consequence: "ECS subscenes and Entities systems do not simulate or render",
    },
    UnsupportedFamilyEntry {
        family: "Netcode for Entities",
        package_id: Some("com.unity.netcode"),
        recognized_form: "com.unity.netcode content",
        state: CompatibilityState::Unsupported,
        owning_spec: "build-platforms-xr-networking.md",
        owning_feature_id: "NET-001",
        observable_consequence: "Netcode for Entities replication and ghost systems do not run",
    },
    UnsupportedFamilyEntry {
        family: "Visual Effect Graph",
        package_id: Some("com.unity.visualeffectgraph"),
        recognized_form: "com.unity.visualeffectgraph assets",
        state: CompatibilityState::Unsupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-004",
        observable_consequence: "VFX Graph assets do not simulate or render; ParticleSystem effects remain usable",
    },
    UnsupportedFamilyEntry {
        family: "UI Toolkit runtime panels",
        package_id: None,
        recognized_form: "Runtime panel settings, UXML documents, and UIDocument components",
        state: CompatibilityState::Unsupported,
        owning_spec: "ui-audio-input.md",
        owning_feature_id: "UI-001",
        observable_consequence: "UIDocument runtime panels are not drawn; uGUI canvases remain usable",
    },
    UnsupportedFamilyEntry {
        family: "Visual Scripting",
        package_id: Some("com.unity.visualscripting"),
        recognized_form: "com.unity.visualscripting graphs and machines",
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-002",
        observable_consequence: "Script graphs and state machines do not execute; C# MonoBehaviours remain playable",
    },
    UnsupportedFamilyEntry {
        family: "Enlighten realtime global illumination",
        package_id: None,
        recognized_form: "Realtime GI settings and Enlighten realtime lighting data; baked lighting remains Supported content",
        state: CompatibilityState::Unsupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-003",
        observable_consequence: "Enlighten realtime GI is not evaluated; Unity-baked lightmaps and probes remain active",
    },
    UnsupportedFamilyEntry {
        family: "Unity-built AssetBundles and Addressables content",
        package_id: None,
        recognized_form: "AssetBundles and Addressables content built by Unity and loaded in a unitas player; unitas rebuilds that content from the project",
        state: CompatibilityState::Unsupported,
        owning_spec: "build-platforms-xr-networking.md",
        owning_feature_id: "BLD-005",
        observable_consequence: "Pre-built Unity binary bundles are not loaded directly; content must be built by unitas from project sources",
    },
    UnsupportedFamilyEntry {
        family: "Baking inside unitas",
        package_id: None,
        recognized_form: "Requests to bake lightmaps or occlusion culling data; Unity-baked lightmaps, light probes, reflection probe captures, and occlusion data remain Supported content",
        state: CompatibilityState::Unsupported,
        owning_spec: "rendering-and-animation.md",
        owning_feature_id: "REN-003",
        observable_consequence: "In-editor lightmap or occlusion baking cannot be triggered; existing baked lighting data renders normally",
    },
    UnsupportedFamilyEntry {
        family: "Custom editor extension drawing",
        package_id: None,
        recognized_form: "Custom inspectors, property drawers, and editor windows; their commands and content effects remain Supported",
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-010",
        observable_consequence: "Custom IMGUI/UI Toolkit inspector and window chrome is not drawn; menu commands and asset mutations still run",
    },
];

pub fn lookup_package_matrix(package_id: &str) -> Option<&'static PackageMatrixEntry> {
    SUPPORTED_PACKAGE_MATRIX
        .iter()
        .find(|entry| entry.package_id == package_id)
}

pub fn lookup_unsupported_family(name_or_package_id: &str) -> Option<&'static UnsupportedFamilyEntry> {
    RECOGNIZED_UNSUPPORTED_FAMILIES.iter().find(|entry| {
        entry.family == name_or_package_id
            || entry.package_id == Some(name_or_package_id)
            || (name_or_package_id == "com.unity.entities.graphics"
                && entry.package_id == Some("com.unity.entities"))
            || (name_or_package_id == "com.unity.rendering.hybrid"
                && entry.package_id == Some("com.unity.entities"))
    })
}

pub fn assess_package(
    package_id: &str,
    resolved_version: &str,
    is_locally_modified_embedded: bool,
) -> AssessedPackage {
    if is_locally_modified_embedded {
        return AssessedPackage {
            package_id: package_id.to_string(),
            resolved_version: resolved_version.to_string(),
            tier: None,
            state: CompatibilityState::Unknown,
            owning_spec: None,
            owning_feature_id: None,
            notes: "locally modified embedded copy is assessed as third-party content".to_string(),
        };
    }
    if let Some(unsupported) = lookup_unsupported_family(package_id) {
        return AssessedPackage {
            package_id: package_id.to_string(),
            resolved_version: resolved_version.to_string(),
            tier: None,
            state: unsupported.state,
            owning_spec: Some(unsupported.owning_spec.to_string()),
            owning_feature_id: Some(unsupported.owning_feature_id.to_string()),
            notes: unsupported.observable_consequence.to_string(),
        };
    }
    let Some(entry) = lookup_package_matrix(package_id) else {
        return AssessedPackage {
            package_id: package_id.to_string(),
            resolved_version: resolved_version.to_string(),
            tier: None,
            state: CompatibilityState::Unknown,
            owning_spec: None,
            owning_feature_id: None,
            notes: "package is not listed in the Unity 2022.3 LTS supported package matrix"
                .to_string(),
        };
    };
    if is_preview_or_experimental_version(resolved_version)
        || !version_matches_line(resolved_version, entry.version_line)
    {
        return AssessedPackage {
            package_id: package_id.to_string(),
            resolved_version: resolved_version.to_string(),
            tier: Some(entry.tier),
            state: CompatibilityState::Unknown,
            owning_spec: Some(entry.owning_spec.to_string()),
            owning_feature_id: Some(entry.owning_feature_id.to_string()),
            notes: format!(
                "version {resolved_version} is outside the Unity 2022.3 LTS matrix line ({})",
                entry.version_line
            ),
        };
    }
    AssessedPackage {
        package_id: package_id.to_string(),
        resolved_version: resolved_version.to_string(),
        tier: Some(entry.tier),
        state: entry.state,
        owning_spec: Some(entry.owning_spec.to_string()),
        owning_feature_id: Some(entry.owning_feature_id.to_string()),
        notes: entry.state_notes.to_string(),
    }
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
        tier: Tier::Native,
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
