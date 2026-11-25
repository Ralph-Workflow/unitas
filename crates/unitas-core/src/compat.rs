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
        state_notes: "Partially supported: graph targets for HDRP and Visual Effect Graph are Unsupported; a graph that uses a node unitas cannot represent is reported per graph and node under REN-006",
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
        state: CompatibilityState::Unsupported,
        owning_spec: "accessibility-localization-persistence.md",
        owning_feature_id: "LOC-001",
        state_notes: "Unsupported: no localization import, string table, or locale selection exists; localized content is not imported",
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
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-006",
        state_notes: "Unsupported: no addressable catalog, key or label load, or handle API exists; content cannot be loaded by address",
    },
    PackageMatrixEntry {
        package_id: "com.unity.burst",
        display_name: "Burst",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Unsupported: no job system and no Burst compilation exist; jobs do not schedule and Burst-compiled code does not run",
    },
    PackageMatrixEntry {
        package_id: "com.unity.mathematics",
        display_name: "Mathematics",
        version_line: "1.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Unknown,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Unknown: not assessed; no test demonstrates the Unity.Mathematics types and functions under unitas",
    },
    PackageMatrixEntry {
        package_id: "com.unity.collections",
        display_name: "Collections",
        version_line: "Released or verified",
        tier: Tier::Semantic,
        state: CompatibilityState::Unsupported,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-011",
        state_notes: "Unsupported: no native container allocator, disposal, or job safety system exists; the Collections API is unavailable",
    },
    PackageMatrixEntry {
        package_id: "com.unity.nuget.newtonsoft-json",
        display_name: "Newtonsoft Json",
        version_line: "3.x",
        tier: Tier::Semantic,
        state: CompatibilityState::Unknown,
        owning_spec: "scripting-runtime.md",
        owning_feature_id: "RUN-009",
        state_notes: "Unknown: not assessed; no test demonstrates Newtonsoft Json serialization under unitas",
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

pub fn lookup_unsupported_family(
    name_or_package_id: &str,
) -> Option<&'static UnsupportedFamilyEntry> {
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
    assess_package_for_baseline(package_id, resolved_version, is_locally_modified_embedded, false)
}

pub fn assess_package_for_baseline(
    package_id: &str,
    resolved_version: &str,
    is_locally_modified_embedded: bool,
    migrated_from_earlier_lts: bool,
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
    if is_preview_or_experimental_version(resolved_version) {
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
    let matches_direct = version_matches_line(resolved_version, entry.version_line);
    let matches_migrated = !matches_direct
        && migrated_from_earlier_lts
        && version_migrates_from_earlier_lts_line(resolved_version, entry.version_line);
    if !matches_direct && !matches_migrated {
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
    let notes = if matches_migrated {
        format!(
            "migrated under SER-005 and assessed against Unity 2022.3 LTS matrix line ({}); {}",
            entry.version_line, entry.state_notes
        )
    } else {
        entry.state_notes.to_string()
    };
    AssessedPackage {
        package_id: package_id.to_string(),
        resolved_version: resolved_version.to_string(),
        tier: Some(entry.tier),
        state: entry.state,
        owning_spec: Some(entry.owning_spec.to_string()),
        owning_feature_id: Some(entry.owning_feature_id.to_string()),
        notes,
    }
}

fn version_migrates_from_earlier_lts_line(version: &str, line: &str) -> bool {
    let Some(target_prefix) = line.strip_suffix(".x") else {
        return false;
    };
    let target_major = match target_prefix.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
        Some(v) => v,
        None => return false,
    };
    let resolved_major = match version.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
        Some(v) => v,
        None => return false,
    };
    resolved_major > 0 && resolved_major < target_major
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PackageSource {
    Registry,
    ScopedRegistry,
    Git,
    LocalPath,
    Embedded,
    Tarball,
    BuiltInModule,
    AssetStoreFolder,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectPackageEntry {
    pub package_id: String,
    pub version: String,
    pub source: PackageSource,
    pub is_locally_modified_embedded: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectAssetEntry {
    pub family: String,
    pub content: ContentIdentity,
    pub owning_feature_id: Option<String>,
    pub tier: Option<Tier>,
    pub state: Option<CompatibilityState>,
    pub observable_consequence: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectSettingEntry {
    pub setting_name: String,
    pub family: String,
    pub owning_feature_id: Option<String>,
    pub tier: Option<Tier>,
    pub state: Option<CompatibilityState>,
    pub observable_consequence: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectContent {
    pub unitas_version: String,
    pub declared_unity_version: Option<String>,
    pub packages: Vec<ProjectPackageEntry>,
    pub assets: Vec<ProjectAssetEntry>,
    pub settings: Vec<ProjectSettingEntry>,
    pub additional_findings: Vec<Finding>,
    pub track_absent_catalog_features: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectCompatibilityAssessment {
    pub report: CompatibilityReport,
    pub effective_baseline_unity_version: String,
    pub migrated_from_earlier_lts: bool,
    pub overall_state: CompatibilityState,
    pub assessed_packages: Vec<AssessedPackage>,
    pub present_supported_features: Vec<String>,
    pub partially_supported_features: Vec<String>,
    pub unsupported_features: Vec<String>,
    pub unknown_features: Vec<String>,
    pub absent_features: Vec<String>,
    pub playable_scenes: Vec<String>,
}

pub fn assess_project(content: &ProjectContent) -> ProjectCompatibilityAssessment {
    let migrated_from_earlier_lts = content
        .declared_unity_version
        .as_deref()
        .is_some_and(|version| version.starts_with("2020.3") || version.starts_with("2021.3"));

    let mut assessed_packages: Vec<AssessedPackage> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    let mut playable_scenes: Vec<String> = Vec::new();

    for pkg in &content.packages {
        let assessed = if pkg.source == PackageSource::BuiltInModule
            || pkg.package_id.starts_with("com.unity.modules.")
        {
            AssessedPackage {
                package_id: pkg.package_id.clone(),
                resolved_version: pkg.version.clone(),
                tier: Some(Tier::Semantic),
                state: CompatibilityState::Supported,
                owning_spec: Some("scripting-runtime.md".to_string()),
                owning_feature_id: Some("RUN-002".to_string()),
                notes: "built-in engine module".to_string(),
            }
        } else {
            assess_package_for_baseline(
                &pkg.package_id,
                &pkg.version,
                pkg.is_locally_modified_embedded,
                migrated_from_earlier_lts,
            )
        };
        let feature_id = assessed
            .owning_feature_id
            .clone()
            .unwrap_or_else(|| "PRJ-006".to_string());
        let tier = assessed.tier.unwrap_or(Tier::Native);
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
        findings.push(Finding {
            feature_id,
            tier,
            state: assessed.state,
            content: ContentIdentity {
                guid: None,
                asset_path: Some(format!("Packages/{}", pkg.package_id)),
                sub_asset_id: None,
                object_name: Some(format!("{}@{}", pkg.package_id, pkg.version)),
            },
            behavior: format!("package {} ({})", pkg.package_id, pkg.version),
            relied_upon_unity_differences,
            observable_consequence: assessed.notes.clone(),
            conditions: vec!["package resolved in project manifest".to_string()],
            related_findings: Vec::new(),
        });
        assessed_packages.push(assessed);
    }

    for asset in &content.assets {
        let unsupported_entry = lookup_unsupported_family(&asset.family);
        let state = asset.state.unwrap_or_else(|| {
            unsupported_entry.map_or(CompatibilityState::Unknown, |entry| entry.state)
        });
        let feature_id = asset.owning_feature_id.clone().unwrap_or_else(|| {
            unsupported_entry.map_or_else(
                || "PRJ-004".to_string(),
                |entry| entry.owning_feature_id.to_string(),
            )
        });
        let tier = asset
            .tier
            .or_else(|| lookup_catalog_feature(&feature_id).map(|cat| cat.tier))
            .unwrap_or(Tier::Native);
        let consequence = asset.observable_consequence.clone().unwrap_or_else(|| {
            unsupported_entry.map_or_else(
                || match state {
                    CompatibilityState::Supported => "content is editable and playable".to_string(),
                    CompatibilityState::PartiallySupported => {
                        "content is usable with stated exclusions".to_string()
                    }
                    CompatibilityState::Unsupported => {
                        "recognized content is unsupported; unrelated content remains usable"
                            .to_string()
                    }
                    CompatibilityState::Unknown
                    | CompatibilityState::PlatformLimited
                    | CompatibilityState::ExternallyUnavailable => {
                        "content compatibility requires target or behavioral classification"
                            .to_string()
                    }
                },
                |entry| entry.observable_consequence.to_string(),
            )
        });
        if state == CompatibilityState::Supported {
            if let Some(path) = &asset.content.asset_path {
                if path.ends_with(".unity") && !playable_scenes.contains(path) {
                    playable_scenes.push(path.clone());
                }
            }
        }
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
        findings.push(Finding {
            feature_id,
            tier,
            state,
            content: asset.content.clone(),
            behavior: format!("asset family {}", asset.family),
            relied_upon_unity_differences,
            observable_consequence: consequence,
            conditions: vec!["asset is present in project".to_string()],
            related_findings: Vec::new(),
        });
    }

    for setting in &content.settings {
        let unsupported_entry = lookup_unsupported_family(&setting.family);
        let state = setting.state.unwrap_or_else(|| {
            unsupported_entry.map_or(CompatibilityState::Supported, |entry| entry.state)
        });
        let feature_id = setting.owning_feature_id.clone().unwrap_or_else(|| {
            unsupported_entry.map_or_else(
                || "PRJ-007".to_string(),
                |entry| entry.owning_feature_id.to_string(),
            )
        });
        let tier = setting.tier.unwrap_or(Tier::Exact);
        let consequence = setting.observable_consequence.clone().unwrap_or_else(|| {
            unsupported_entry.map_or_else(
                || "project setting is represented and honored".to_string(),
                |entry| entry.observable_consequence.to_string(),
            )
        });
        findings.push(Finding {
            feature_id,
            tier,
            state,
            content: ContentIdentity {
                guid: None,
                asset_path: Some(setting.setting_name.clone()),
                sub_asset_id: None,
                object_name: Some(setting.family.clone()),
            },
            behavior: format!("project setting {}", setting.family),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: consequence,
            conditions: vec!["setting is configured in ProjectSettings".to_string()],
            related_findings: Vec::new(),
        });
    }

    for extra in &content.additional_findings {
        if is_present(extra) && extra.state == CompatibilityState::Supported {
            if let Some(path) = &extra.content.asset_path {
                if path.ends_with(".unity") && !playable_scenes.contains(path) {
                    playable_scenes.push(path.clone());
                }
            }
        }
        let mut enriched = extra.clone();
        if enriched.tier == Tier::Familiar && enriched.relied_upon_unity_differences.is_empty() {
            if let Some(entry) = lookup_familiar_behavior_differences(&enriched.feature_id) {
                enriched.relied_upon_unity_differences = entry
                    .differences
                    .iter()
                    .map(|diff| (*diff).to_string())
                    .collect();
            }
        }
        findings.push(enriched);
    }

    let mut present_supported_features: Vec<String> = Vec::new();
    let mut partially_supported_features: Vec<String> = Vec::new();
    let mut unsupported_features: Vec<String> = Vec::new();
    let mut unknown_features: Vec<String> = Vec::new();
    let mut absent_features: Vec<String> = Vec::new();
    let mut present_states: Vec<CompatibilityState> = Vec::new();

    for finding in &findings {
        if !is_present(finding) {
            push_unique(&mut absent_features, &finding.feature_id);
            continue;
        }
        present_states.push(finding.state);
        match finding.state {
            CompatibilityState::Supported => {
                push_unique(&mut present_supported_features, &finding.feature_id);
            }
            CompatibilityState::PartiallySupported => {
                push_unique(&mut partially_supported_features, &finding.feature_id);
            }
            CompatibilityState::Unsupported => {
                push_unique(&mut unsupported_features, &finding.feature_id);
            }
            CompatibilityState::Unknown
            | CompatibilityState::PlatformLimited
            | CompatibilityState::ExternallyUnavailable => {
                push_unique(&mut unknown_features, &finding.feature_id);
            }
        }
    }

    for tracked in &content.track_absent_catalog_features {
        let is_any_present = findings
            .iter()
            .any(|f| is_present(f) && &f.feature_id == tracked);
        if !is_any_present {
            push_unique(&mut absent_features, tracked);
            let already_has_absence = findings
                .iter()
                .any(|f| !is_present(f) && &f.feature_id == tracked);
            if !already_has_absence {
                findings.push(absence_finding(tracked));
            }
        }
    }

    let overall_state = least_supported_state(&present_states);
    let report = build_report(&content.unitas_version, findings);

    ProjectCompatibilityAssessment {
        report,
        effective_baseline_unity_version: "2022.3 LTS".to_string(),
        migrated_from_earlier_lts,
        overall_state,
        assessed_packages,
        present_supported_features,
        partially_supported_features,
        unsupported_features,
        unknown_features,
        absent_features,
        playable_scenes,
    }
}

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|existing| existing == item) {
        list.push(item.to_string());
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CatalogFeatureEntry {
    pub feature_id: &'static str,
    pub tier: Tier,
    pub owning_spec: &'static str,
}

pub const FEATURE_CATALOG: &[CatalogFeatureEntry] = &[
    CatalogFeatureEntry {
        feature_id: "PRJ-001",
        tier: Tier::Exact,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-002",
        tier: Tier::Native,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-003",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-004",
        tier: Tier::Semantic,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-005",
        tier: Tier::Exact,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-006",
        tier: Tier::Semantic,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-007",
        tier: Tier::Exact,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRJ-008",
        tier: Tier::Native,
        owning_spec: "project-and-assets.md",
    },
    CatalogFeatureEntry {
        feature_id: "SER-001",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "SER-002",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "SER-003",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "SER-004",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "SER-005",
        tier: Tier::Exact,
        owning_spec: "serialization-and-prefabs.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-001",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-002",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-003",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-004",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-005",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-006",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-007",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "EDT-008",
        tier: Tier::Native,
        owning_spec: "editor-and-authoring.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-001",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-002",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-003",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-004",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-005",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-006",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-007",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-008",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-009",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-010",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "RUN-011",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-001",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-002",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-003",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-004",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-005",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-006",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-007",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-008",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-009",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-010",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "SCR-011",
        tier: Tier::Semantic,
        owning_spec: "scripting-runtime.md",
    },
    CatalogFeatureEntry {
        feature_id: "PHY-001",
        tier: Tier::Semantic,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "PHY-002",
        tier: Tier::Semantic,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "PHY-003",
        tier: Tier::Semantic,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "PHY-004",
        tier: Tier::Familiar,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "NAV-001",
        tier: Tier::Familiar,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "ENV-001",
        tier: Tier::Familiar,
        owning_spec: "physics-navigation-terrain.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-001",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-002",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-003",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-004",
        tier: Tier::Familiar,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-005",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-006",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "REN-007",
        tier: Tier::Native,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "ANI-001",
        tier: Tier::Familiar,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "ANI-002",
        tier: Tier::Familiar,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "CIN-001",
        tier: Tier::Familiar,
        owning_spec: "rendering-and-animation.md",
    },
    CatalogFeatureEntry {
        feature_id: "UI-001",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "UI-002",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "TXT-001",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "INP-001",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "INP-002",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUD-001",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "MED-001",
        tier: Tier::Familiar,
        owning_spec: "ui-audio-input.md",
    },
    CatalogFeatureEntry {
        feature_id: "BLD-001",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "BLD-002",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "BLD-003",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "BLD-004",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "BLD-005",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "PLT-001",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "XR-001",
        tier: Tier::Familiar,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "NET-001",
        tier: Tier::Familiar,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "LIC-001",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRD-001",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRD-002",
        tier: Tier::Native,
        owning_spec: "build-platforms-xr-networking.md",
    },
    CatalogFeatureEntry {
        feature_id: "LOC-001",
        tier: Tier::Familiar,
        owning_spec: "accessibility-localization-persistence.md",
    },
    CatalogFeatureEntry {
        feature_id: "ACC-001",
        tier: Tier::Native,
        owning_spec: "accessibility-localization-persistence.md",
    },
    CatalogFeatureEntry {
        feature_id: "SAV-001",
        tier: Tier::Semantic,
        owning_spec: "accessibility-localization-persistence.md",
    },
    CatalogFeatureEntry {
        feature_id: "DIA-001",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "DIA-002",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "DIA-003",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRF-001",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "PRF-002",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "TST-001",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "TST-002",
        tier: Tier::Native,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "TST-003",
        tier: Tier::Semantic,
        owning_spec: "diagnostics-profiling-playtesting.md",
    },
    CatalogFeatureEntry {
        feature_id: "SMP-001",
        tier: Tier::Native,
        owning_spec: "sample-games.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-001",
        tier: Tier::Native,
        owning_spec: "sample-games.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-002",
        tier: Tier::Native,
        owning_spec: "compatibility-contract.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-003",
        tier: Tier::Native,
        owning_spec: "compatibility-contract.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-004",
        tier: Tier::Native,
        owning_spec: "compatibility-contract.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-005",
        tier: Tier::Native,
        owning_spec: "compatibility-contract.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-001",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-002",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-003",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-004",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-005",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-006",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-007",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
    CatalogFeatureEntry {
        feature_id: "AUT-008",
        tier: Tier::Native,
        owning_spec: "automation-and-mcp.md",
    },
];

pub fn lookup_catalog_feature(feature_id: &str) -> Option<&'static CatalogFeatureEntry> {
    FEATURE_CATALOG
        .iter()
        .find(|entry| entry.feature_id == feature_id)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FamiliarBehaviorDifferenceEntry {
    pub feature_id: &'static str,
    pub owning_spec: &'static str,
    pub differences: &'static [&'static str],
}

pub const FAMILIAR_BEHAVIOR_DIFFERENCES: &[FamiliarBehaviorDifferenceEntry] = &[
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ANI-001",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "AnimationClip::sample caps event firing across multi-cycle steps in a single frame to at most 64 loop/ping-pong cycles (max_cycles = 64), whereas Unity 2022.3 LTS does not specify a 64-cycle cap.",
            "PropertyCurve::Quaternion evaluation and pose blending use shortest-path component-aligned Hermite/Linear interpolation followed by unit normalization (nlerp_shortest) rather than Unity's internal slerp/squad curve resampling.",
            "StateMotion::BlendTree2D computes child blend weights via inverse-squared Euclidean distance in (parameter_x, parameter_y) space across SimpleDirectional, FreeformDirectional, and FreeformCartesian modes rather than polar-angle/gradient-band Delaunay triangulation.",
            "Within a single frame, animation events across different animators fire in deterministic ascending EntityId evaluation order rather than native job-thread completion order.",
            "Controller default property values for Write Defaults are captured from the controller's curves evaluated at t = 0.0 upon binding rather than snapshotting live scene values at OnEnable.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ANI-002",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "Humanoid muscle space models 19 HumanBones (7 required core bones) and 17 primary normalized MuscleChannel axes with analytical twist extraction and linear hips_scale root retargeting rather than Unity's 55-muscle + finger-DoF solver.",
            "OnAnimatorIK limb solving uses a deterministic closed-form analytical two-bone law-of-cosines solver (solve_two_bone_ik) with pole-vector projection and clamped slerp look-at distribution rather than iterative cyclic coordinate descent (CCD).",
            "PlayableGraph invokes ScriptPlayable callbacks in deterministic ascending node-creation index order among reachable nodes (all PrepareFrame before any ProcessFrame) rather than depth-first topological traversal from outputs.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "CIN-001",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "TimelineWorld::drive_animation drives bound AnimationWorld entities by gating animator playback speed and tracking clip local time rather than constructing an internal AnimationLayerMixerPlayable sub-graph per track.",
            "CinemachineWorld resolves equal-priority active virtual cameras deterministically by a monotonically increasing activation counter and reports normalized linear blend progress elapsed / duration rather than custom AnimationCurve tangents.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "REN-004",
        owning_spec: "rendering-and-animation.md",
        differences: &[
            "Particle emission and shape sampling use a platform-independent SplitMix64 PRNG with Marsaglia sphere rejection and Newton-Raphson cube root so fixed seeds produce identical particle counts and positions across all hosts/targets, whereas numeric sample positions for a given seed differ from Unity 2022.3 LTS's internal generator.",
            "ParticleShape::Mesh samples uniformly inside the mesh's axis-aligned bounding box rather than barycentric triangle/vertex/edge surface weighting.",
            "Property-over-lifetime modules evaluate two-keyframe linear curves (Curve<N>) at normalized age age / lifetime.",
            "3D and 2D particle collision and trigger queries evaluate sphere/circle overlaps of radius particle.size * 0.5 with radial contact normals rather than sub-step raycast trajectory deflection.",
            "ParticleSystemForceField evaluates Sphere and Box containment with directional, gravity, radial gravity_focus, and linear drag forces without 3D vector-field textures or curl-noise tables.",
            "Visual Effect Graph (com.unity.visualeffectgraph) is excluded and reported as Unsupported under REN-004.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "PHY-004",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "WheelCollider uses unitas's deterministic sphere-cast suspension and piecewise-linear slip solver rather than PhysX 4.1's vehicle SDK solver; trajectories are repeatable per target but not bit-for-bit identical to Unity 2022.3 LTS.",
            "ArticulationBody steps hierarchical Revolute and Prismatic joint chains deterministically in reduced coordinates with PD target drives clamped to [lower, upper].",
            "Cloth simulates a deterministic Verlet particle grid with distance constraints, gravity, damping, ClearTransformMotion, and collision resolution exclusively against assigned ClothCollider capsules.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "NAV-001",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "Calling SetDestination or path-dependent members on an off-NavMesh NavMeshAgent (isOnNavMesh == false) raises an explicit error naming the agent and member (NavigationError::AgentOffNavMesh) rather than failing silently.",
            "Path queries and agents expose structured UnreachableReason (StartNotOnNavMesh, GoalNotOnNavMesh, NoRouteToGoal) alongside PathComplete, PathPartial, and PathInvalid.",
            "Saved navigation data that no longer matches its source geometry or settings is marked and reported is_stale() == true and is never silently rebuilt on entering play mode.",
            "Navigation mesh building, NavMeshObstacle polygon carving, portal A* pathfinding, and reciprocal priority-weighted avoidance use unitas's deterministic solver rather than Recast/Detour's internal voxel/crowd numeric output.",
        ],
    },
    FamiliarBehaviorDifferenceEntry {
        feature_id: "ENV-001",
        owning_spec: "physics-navigation-terrain.md",
        differences: &[
            "SpeedTree 7 (.spm) and SpeedTree 8 (.st) geometry, materials, and imported mesh LODs are supported (PartiallySupported), whereas SpeedTree wind animation is excluded and reported on each affected prototype (wind_excluded == true) while Tree Creator trees and terrain grass wave with WindZone.",
            "World positions remain 32-bit single-precision without silent origin rebasing; objects at >= 100,000 units report a structured LargeWorldImpact identifying the affected systems (physics, rendering, animation, navigation).",
        ],
    },
];

pub fn lookup_familiar_behavior_differences(
    feature_id: &str,
) -> Option<&'static FamiliarBehaviorDifferenceEntry> {
    FAMILIAR_BEHAVIOR_DIFFERENCES
        .iter()
        .find(|entry| entry.feature_id == feature_id)
}

pub fn build_semantic_finding(
    feature_id: &str,
    state: CompatibilityState,
    content: ContentIdentity,
    behavior: &str,
    cmp_001_benchmark_id: &str,
    found_difference: Option<&str>,
) -> Result<Finding, FindingSubmissionError> {
    let trimmed_id = feature_id.trim();
    if trimmed_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(trimmed_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId {
            feature_id: trimmed_id.to_string(),
        });
    };
    if catalog_entry.tier != Tier::Semantic {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: trimmed_id.to_string(),
            expected: catalog_entry.tier,
            submitted: Tier::Semantic,
        });
    }
    let benchmark = cmp_001_benchmark_id.trim();
    if benchmark.is_empty() {
        return Err(FindingSubmissionError::EmptyObservableConsequence);
    }
    let consequence = match found_difference {
        Some(diff) if !diff.trim().is_empty() => format!(
            "assessed against Unity 2022.3 LTS reference ({}) and backed by CMP-001 benchmark {}; difference: {}",
            catalog_entry.owning_spec,
            benchmark,
            diff.trim()
        ),
        _ => format!(
            "matches Unity 2022.3 LTS documented observable behavior ({}) as verified by CMP-001 benchmark {}",
            catalog_entry.owning_spec, benchmark
        ),
    };
    let mut input = FindingInput::new(
        trimmed_id,
        Tier::Semantic,
        state,
        content,
        behavior,
        consequence,
    )
    .with_condition(format!("reference: Unity 2022.3 LTS ({})", catalog_entry.owning_spec))
    .with_condition(format!("benchmark: {benchmark}"));
    if let Some(diff) = found_difference {
        if !diff.trim().is_empty() {
            input = input.with_unity_difference(diff.trim());
        }
    }
    validate_finding_input(input, &[])
}

pub fn build_familiar_finding(
    feature_id: &str,
    state: CompatibilityState,
    content: ContentIdentity,
    behavior: &str,
    additional_differences: &[&str],
) -> Result<Finding, FindingSubmissionError> {
    let trimmed_id = feature_id.trim();
    if trimmed_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(trimmed_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId {
            feature_id: trimmed_id.to_string(),
        });
    };
    if catalog_entry.tier != Tier::Familiar {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id: trimmed_id.to_string(),
            expected: catalog_entry.tier,
            submitted: Tier::Familiar,
        });
    }
    let mut differences: Vec<String> = lookup_familiar_behavior_differences(trimmed_id)
        .map(|entry| {
            entry
                .differences
                .iter()
                .map(|diff| (*diff).to_string())
                .collect()
        })
        .unwrap_or_default();
    for extra in additional_differences {
        let trimmed = extra.trim();
        if !trimmed.is_empty() && !differences.iter().any(|existing| existing == trimmed) {
            differences.push(trimmed.to_string());
        }
    }
    let consequence = format!(
        "behavior defined by {} ({}) and listed in the AUT-006 API reference with {} Unity 2022.3 LTS difference(s)",
        catalog_entry.owning_spec,
        trimmed_id,
        differences.len()
    );
    let mut input = FindingInput::new(
        trimmed_id,
        Tier::Familiar,
        state,
        content,
        behavior,
        consequence,
    )
    .with_condition(format!(
        "owning specification: {} ({})",
        catalog_entry.owning_spec, trimmed_id
    ))
    .with_condition("listed in AUT-006 API reference".to_string());
    for diff in differences {
        input = input.with_unity_difference(diff);
    }
    validate_finding_input(input, &[])
}

pub fn assess_translated_material(
    content: ContentIdentity,
    material_description: &str,
    unitas_reference_image: Option<&str>,
    passes_unitas_reference: bool,
    differs_from_unity_screenshot: bool,
) -> Finding {
    let sanitized = sanitize_content_identity(&content);
    let (state, consequence) = match (unitas_reference_image, passes_unitas_reference) {
        (Some(ref_img), true) => (
            CompatibilityState::Supported,
            format!(
                "translated material passes visual validation against unitas-approved reference image '{ref_img}' (TST-002); pixel difference from a Unity screenshot (observed={differs_from_unity_screenshot}) is never a compatibility failure"
            ),
        ),
        (Some(ref_img), false) => (
            CompatibilityState::PartiallySupported,
            format!(
                "translated material diverges from unitas-approved reference image '{ref_img}' (TST-002)"
            ),
        ),
        (None, _) => (
            CompatibilityState::Unknown,
            "translated material has not yet been compared against a unitas-approved reference image (TST-002)".to_string(),
        ),
    };
    Finding {
        feature_id: "REN-002".to_string(),
        tier: Tier::Native,
        state,
        content: sanitized,
        behavior: material_description.trim().to_string(),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: consequence,
        conditions: vec![
            "visual acceptance uses unitas-approved references (TST-002), not Unity screenshots"
                .to_string(),
        ],
        related_findings: Vec::new(),
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderGraphTarget {
    BuiltIn,
    Universal,
    HighDefinition,
    VisualEffect,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShaderGraphNodeIssue {
    pub node_type: String,
    pub node_id: String,
}

pub fn assess_shader_graph(
    graph_content: ContentIdentity,
    target: ShaderGraphTarget,
    unrepresentable_nodes: &[ShaderGraphNodeIssue],
    affected_materials: &[String],
) -> Finding {
    let mut content = sanitize_content_identity(&graph_content);
    let graph_label = content
        .asset_path
        .clone()
        .or_else(|| content.object_name.clone())
        .or_else(|| content.guid.clone())
        .unwrap_or_else(|| "ShaderGraph".to_string());
    let materials_clause = if affected_materials.is_empty() {
        "no affected materials".to_string()
    } else {
        format!("affected materials: {}", affected_materials.join(", "))
    };

    match target {
        ShaderGraphTarget::HighDefinition => Finding {
            feature_id: "REN-006".to_string(),
            tier: Tier::Native,
            state: CompatibilityState::Unsupported,
            content,
            behavior: format!("Shader Graph '{graph_label}' with HDRP graph target"),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: format!(
                "Shader Graph HDRP graph target is Unsupported under REN-006; {materials_clause}"
            ),
            conditions: vec!["Shader Graph target is HDRP".to_string()],
            related_findings: Vec::new(),
        },
        ShaderGraphTarget::VisualEffect => Finding {
            feature_id: "REN-006".to_string(),
            tier: Tier::Native,
            state: CompatibilityState::Unsupported,
            content,
            behavior: format!(
                "Shader Graph '{graph_label}' with Visual Effect Graph target"
            ),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: format!(
                "Shader Graph Visual Effect Graph target is Unsupported under REN-006; {materials_clause}"
            ),
            conditions: vec!["Shader Graph target is Visual Effect Graph".to_string()],
            related_findings: vec!["REN-004".to_string()],
        },
        ShaderGraphTarget::BuiltIn | ShaderGraphTarget::Universal => {
            if let Some(first_issue) = unrepresentable_nodes.first() {
                if content.sub_asset_id.is_none() {
                    content.sub_asset_id = Some(first_issue.node_id.clone());
                }
                let nodes_summary: Vec<String> = unrepresentable_nodes
                    .iter()
                    .map(|issue| format!("{} ({})", issue.node_type, issue.node_id))
                    .collect();
                Finding {
                    feature_id: "REN-006".to_string(),
                    tier: Tier::Native,
                    state: CompatibilityState::Unsupported,
                    content,
                    behavior: format!(
                        "Shader Graph '{graph_label}' uses unrepresentable node(s): {}",
                        nodes_summary.join(", ")
                    ),
                    relied_upon_unity_differences: Vec::new(),
                    observable_consequence: format!(
                        "graph '{graph_label}' cannot be represented due to node(s) {} and renders as a failed shader; {materials_clause}",
                        nodes_summary.join(", ")
                    ),
                    conditions: vec![
                        "Shader Graph contains unrepresentable node".to_string(),
                    ],
                    related_findings: Vec::new(),
                }
            } else {
                Finding {
                    feature_id: "REN-006".to_string(),
                    tier: Tier::Native,
                    state: CompatibilityState::Supported,
                    content,
                    behavior: format!("Shader Graph '{graph_label}'"),
                    relied_upon_unity_differences: Vec::new(),
                    observable_consequence: format!(
                        "Shader Graph '{graph_label}' imports and translates cleanly; {materials_clause}"
                    ),
                    conditions: vec![
                        "Shader Graph target is URP or Built-in with representable nodes"
                            .to_string(),
                    ],
                    related_findings: Vec::new(),
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompatibilitySummary {
    pub overall_state: CompatibilityState,
    pub total_findings: usize,
    pub present_findings: usize,
    pub absent_findings: usize,
    pub supported_count: usize,
    pub partially_supported_count: usize,
    pub platform_limited_count: usize,
    pub unknown_count: usize,
    pub unsupported_count: usize,
    pub externally_unavailable_count: usize,
}

pub fn summarize_report(report: &CompatibilityReport) -> CompatibilitySummary {
    let mut present_findings = 0;
    let mut absent_findings = 0;
    let mut supported_count = 0;
    let mut partially_supported_count = 0;
    let mut platform_limited_count = 0;
    let mut unknown_count = 0;
    let mut unsupported_count = 0;
    let mut externally_unavailable_count = 0;
    let mut effective_states: Vec<CompatibilityState> = Vec::new();

    for finding in &report.findings {
        if is_present(finding) {
            present_findings += 1;
            effective_states.push(finding.state);
            match finding.state {
                CompatibilityState::Supported => supported_count += 1,
                CompatibilityState::PartiallySupported => partially_supported_count += 1,
                CompatibilityState::PlatformLimited => platform_limited_count += 1,
                CompatibilityState::Unknown => unknown_count += 1,
                CompatibilityState::Unsupported => unsupported_count += 1,
                CompatibilityState::ExternallyUnavailable => externally_unavailable_count += 1,
            }
        } else {
            absent_findings += 1;
            unknown_count += 1;
            effective_states.push(CompatibilityState::Unknown);
        }
    }

    CompatibilitySummary {
        overall_state: least_supported_state(&effective_states),
        total_findings: report.findings.len(),
        present_findings,
        absent_findings,
        supported_count,
        partially_supported_count,
        platform_limited_count,
        unknown_count,
        unsupported_count,
        externally_unavailable_count,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FindingInput {
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

impl FindingInput {
    pub fn new(
        feature_id: impl Into<String>,
        tier: Tier,
        state: CompatibilityState,
        content: ContentIdentity,
        behavior: impl Into<String>,
        observable_consequence: impl Into<String>,
    ) -> Self {
        Self {
            feature_id: feature_id.into(),
            tier,
            state,
            content,
            behavior: behavior.into(),
            relied_upon_unity_differences: Vec::new(),
            observable_consequence: observable_consequence.into(),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_unity_difference(mut self, difference: impl Into<String>) -> Self {
        self.relied_upon_unity_differences.push(difference.into());
        self
    }

    #[must_use]
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }

    #[must_use]
    pub fn with_related_finding(mut self, related: impl Into<String>) -> Self {
        self.related_findings.push(related.into());
        self
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FindingSubmissionError {
    EmptyFeatureId,
    UnknownCatalogFeatureId {
        feature_id: String,
    },
    CatalogTierMismatch {
        feature_id: String,
        expected: Tier,
        submitted: Tier,
    },
    MissingContentIdentity,
    EmptyBehavior,
    EmptyObservableConsequence,
    SupportedFindingClaimsUnityDifference {
        feature_id: String,
        tier: Tier,
    },
    MissingReliedUponUnityDifference {
        feature_id: String,
        tier: Tier,
        state: CompatibilityState,
    },
    UnityScreenshotPixelDifferenceIsNotCompatibilityFailure {
        feature_id: String,
    },
    DuplicateFindingIdentity {
        feature_id: String,
        content: Box<ContentIdentity>,
    },
}

impl std::fmt::Display for FindingSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFeatureId => write!(formatter, "finding feature_id must not be empty"),
            Self::UnknownCatalogFeatureId { feature_id } => {
                write!(formatter, "unknown catalog feature_id '{feature_id}'")
            }
            Self::CatalogTierMismatch {
                feature_id,
                expected,
                submitted,
            } => write!(
                formatter,
                "catalog tier mismatch for '{feature_id}': expected {expected:?}, got {submitted:?}"
            ),
            Self::MissingContentIdentity => {
                write!(formatter, "finding content identity must name at least one non-empty field")
            }
            Self::EmptyBehavior => write!(formatter, "finding behavior must not be empty"),
            Self::EmptyObservableConsequence => {
                write!(formatter, "finding observable_consequence must not be empty")
            }
            Self::SupportedFindingClaimsUnityDifference { feature_id, tier } => write!(
                formatter,
                "finding for '{feature_id}' ({tier:?}) cannot claim Supported while listing a Unity difference"
            ),
            Self::MissingReliedUponUnityDifference {
                feature_id,
                tier,
                state,
            } => write!(
                formatter,
                "finding for '{feature_id}' ({tier:?}) with state {state:?} must list at least one relied-upon Unity difference"
            ),
            Self::UnityScreenshotPixelDifferenceIsNotCompatibilityFailure { feature_id } => write!(
                formatter,
                "finding for '{feature_id}' cannot report a pixel difference from a Unity screenshot as a compatibility failure"
            ),
            Self::DuplicateFindingIdentity {
                feature_id,
                content,
            } => write!(
                formatter,
                "duplicate finding identity for feature '{feature_id}' and content {content:?}"
            ),
        }
    }
}

impl std::error::Error for FindingSubmissionError {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FindingRejection {
    pub index: usize,
    pub error: FindingSubmissionError,
}

impl std::fmt::Display for FindingRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "finding submission rejected at index {}: {}",
            self.index, self.error
        )
    }
}

impl std::error::Error for FindingRejection {}

fn sanitize_optional_field(field: Option<&str>) -> Option<String> {
    field
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn sanitize_content_identity(content: &ContentIdentity) -> ContentIdentity {
    ContentIdentity {
        guid: sanitize_optional_field(content.guid.as_deref()),
        asset_path: sanitize_optional_field(content.asset_path.as_deref()),
        sub_asset_id: sanitize_optional_field(content.sub_asset_id.as_deref()),
        object_name: sanitize_optional_field(content.object_name.as_deref()),
    }
}

fn validate_finding_input(
    input: FindingInput,
    existing: &[Finding],
) -> Result<Finding, FindingSubmissionError> {
    let feature_id = input.feature_id.trim().to_string();
    if feature_id.is_empty() {
        return Err(FindingSubmissionError::EmptyFeatureId);
    }
    let Some(catalog_entry) = lookup_catalog_feature(&feature_id) else {
        return Err(FindingSubmissionError::UnknownCatalogFeatureId { feature_id });
    };
    if catalog_entry.tier != input.tier {
        return Err(FindingSubmissionError::CatalogTierMismatch {
            feature_id,
            expected: catalog_entry.tier,
            submitted: input.tier,
        });
    }
    let content = sanitize_content_identity(&input.content);
    if !content.names_content() {
        return Err(FindingSubmissionError::MissingContentIdentity);
    }
    let behavior = input.behavior.trim().to_string();
    if behavior.is_empty() {
        return Err(FindingSubmissionError::EmptyBehavior);
    }
    let observable_consequence = input.observable_consequence.trim().to_string();
    if observable_consequence.is_empty() {
        return Err(FindingSubmissionError::EmptyObservableConsequence);
    }
    let relied_upon_unity_differences: Vec<String> = input
        .relied_upon_unity_differences
        .into_iter()
        .map(|diff| diff.trim().to_string())
        .filter(|diff| !diff.is_empty())
        .collect();
    if matches!(input.tier, Tier::Exact | Tier::Semantic)
        && input.state == CompatibilityState::Supported
        && !relied_upon_unity_differences.is_empty()
    {
        return Err(
            FindingSubmissionError::SupportedFindingClaimsUnityDifference {
                feature_id,
                tier: input.tier,
            },
        );
    }
    if input.tier == Tier::Familiar
        && matches!(
            input.state,
            CompatibilityState::PartiallySupported | CompatibilityState::Unsupported
        )
        && relied_upon_unity_differences.is_empty()
    {
        return Err(FindingSubmissionError::MissingReliedUponUnityDifference {
            feature_id,
            tier: input.tier,
            state: input.state,
        });
    }
    let lower_consequence = observable_consequence.to_ascii_lowercase();
    let lower_behavior = behavior.to_ascii_lowercase();
    if input.tier == Tier::Native
        && matches!(
            input.state,
            CompatibilityState::PartiallySupported | CompatibilityState::Unsupported
        )
        && ((lower_consequence.contains("pixel") && lower_consequence.contains("unity screenshot"))
            || (lower_behavior.contains("pixel") && lower_behavior.contains("unity screenshot")))
    {
        return Err(
            FindingSubmissionError::UnityScreenshotPixelDifferenceIsNotCompatibilityFailure {
                feature_id,
            },
        );
    }
    let is_duplicate = existing.iter().any(|item| {
        item.feature_id == feature_id && sanitize_content_identity(&item.content) == content
    });
    if is_duplicate {
        return Err(FindingSubmissionError::DuplicateFindingIdentity {
            feature_id,
            content: Box::new(content),
        });
    }
    Ok(Finding {
        feature_id,
        tier: input.tier,
        state: input.state,
        content,
        behavior,
        relied_upon_unity_differences,
        observable_consequence,
        conditions: input.conditions,
        related_findings: input.related_findings,
    })
}

fn compare_content_identity(left: &ContentIdentity, right: &ContentIdentity) -> Ordering {
    left.asset_path
        .as_deref()
        .unwrap_or("")
        .cmp(right.asset_path.as_deref().unwrap_or(""))
        .then_with(|| {
            left.guid
                .as_deref()
                .unwrap_or("")
                .cmp(right.guid.as_deref().unwrap_or(""))
        })
        .then_with(|| {
            left.sub_asset_id
                .as_deref()
                .unwrap_or("")
                .cmp(right.sub_asset_id.as_deref().unwrap_or(""))
        })
        .then_with(|| {
            left.object_name
                .as_deref()
                .unwrap_or("")
                .cmp(right.object_name.as_deref().unwrap_or(""))
        })
}

fn tier_rank(tier: Tier) -> u8 {
    match tier {
        Tier::Exact => 0,
        Tier::Semantic => 1,
        Tier::Familiar => 2,
        Tier::Native => 3,
    }
}

fn compare_findings(left: &Finding, right: &Finding) -> Ordering {
    left.feature_id
        .cmp(&right.feature_id)
        .then_with(|| compare_content_identity(&left.content, &right.content))
        .then_with(|| left.behavior.cmp(&right.behavior))
        .then_with(|| left.state.cmp(&right.state))
        .then_with(|| tier_rank(left.tier).cmp(&tier_rank(right.tier)))
        .then_with(|| {
            left.observable_consequence
                .cmp(&right.observable_consequence)
        })
        .then_with(|| {
            left.relied_upon_unity_differences
                .cmp(&right.relied_upon_unity_differences)
        })
        .then_with(|| left.conditions.cmp(&right.conditions))
        .then_with(|| left.related_findings.cmp(&right.related_findings))
}

fn build_canonical_report(unitas_version: &str, mut findings: Vec<Finding>) -> CompatibilityReport {
    findings.sort_by(compare_findings);
    let mut report = build_report(unitas_version, findings);
    for package in &mut report.packages {
        package.feature_ids.sort();
    }
    report
        .packages
        .sort_by(|left, right| left.package_name.cmp(&right.package_name));
    report
}

pub fn submit_finding(
    report: &CompatibilityReport,
    input: FindingInput,
) -> Result<CompatibilityReport, FindingSubmissionError> {
    let validated = validate_finding_input(input, &report.findings)?;
    let mut findings = report.findings.clone();
    findings.push(validated);
    Ok(build_canonical_report(
        &report.produced_by_unitas_version,
        findings,
    ))
}

pub fn submit_findings<I>(
    report: &CompatibilityReport,
    inputs: I,
) -> Result<CompatibilityReport, FindingRejection>
where
    I: IntoIterator<Item = FindingInput>,
{
    let mut findings = report.findings.clone();
    for (index, input) in inputs.into_iter().enumerate() {
        match validate_finding_input(input, &findings) {
            Ok(validated) => findings.push(validated),
            Err(error) => return Err(FindingRejection { index, error }),
        }
    }
    Ok(build_canonical_report(
        &report.produced_by_unitas_version,
        findings,
    ))
}

pub fn merge_findings(
    report: &CompatibilityReport,
    external_findings: &[Finding],
) -> Result<CompatibilityReport, FindingRejection> {
    let inputs = external_findings
        .iter()
        .cloned()
        .map(|finding| FindingInput {
            feature_id: finding.feature_id,
            tier: finding.tier,
            state: finding.state,
            content: finding.content,
            behavior: finding.behavior,
            relied_upon_unity_differences: finding.relied_upon_unity_differences,
            observable_consequence: finding.observable_consequence,
            conditions: finding.conditions,
            related_findings: finding.related_findings,
        });
    submit_findings(report, inputs)
}

impl CompatibilityReport {
    pub fn summary(&self) -> CompatibilitySummary {
        summarize_report(self)
    }

    pub fn submit_finding(
        &self,
        input: FindingInput,
    ) -> Result<CompatibilityReport, FindingSubmissionError> {
        submit_finding(self, input)
    }

    pub fn submit_findings<I>(&self, inputs: I) -> Result<CompatibilityReport, FindingRejection>
    where
        I: IntoIterator<Item = FindingInput>,
    {
        submit_findings(self, inputs)
    }

    pub fn merge_findings(
        &self,
        external_findings: &[Finding],
    ) -> Result<CompatibilityReport, FindingRejection> {
        merge_findings(self, external_findings)
    }

    pub fn render_canonical(&self) -> String {
        render_canonical(self)
    }

    pub fn parse_canonical(text: &str) -> Result<CompatibilityReport, CanonicalReportParseError> {
        parse_canonical(text)
    }

    pub fn to_canonical_text(&self) -> String {
        render_canonical(self)
    }

    pub fn from_canonical_text(
        text: &str,
    ) -> Result<CompatibilityReport, CanonicalReportParseError> {
        parse_canonical(text)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CanonicalReportParseError {
    MissingHeader,
    MissingVersion,
    MalformedLine { line_number: usize, line: String },
    InvalidTier(String),
    InvalidState(String),
    InvalidEscapeSequence(String),
    UnexpectedSectionLine { line_number: usize, line: String },
}

impl std::fmt::Display for CanonicalReportParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHeader => write!(formatter, "missing UNITAS_COMPAT_REPORT v1 header"),
            Self::MissingVersion => write!(formatter, "missing unitas_version field"),
            Self::MalformedLine { line_number, line } => {
                write!(formatter, "malformed line {line_number}: '{line}'")
            }
            Self::InvalidTier(value) => write!(formatter, "invalid compatibility tier '{value}'"),
            Self::InvalidState(value) => write!(formatter, "invalid compatibility state '{value}'"),
            Self::InvalidEscapeSequence(value) => {
                write!(formatter, "invalid escape sequence in '{value}'")
            }
            Self::UnexpectedSectionLine { line_number, line } => {
                write!(
                    formatter,
                    "unexpected line {line_number} outside section: '{line}'"
                )
            }
        }
    }
}

impl std::error::Error for CanonicalReportParseError {}

fn format_tier(tier: Tier) -> &'static str {
    match tier {
        Tier::Exact => "Exact",
        Tier::Semantic => "Semantic",
        Tier::Familiar => "Familiar",
        Tier::Native => "Native",
    }
}

fn parse_tier(value: &str) -> Result<Tier, CanonicalReportParseError> {
    match value {
        "Exact" => Ok(Tier::Exact),
        "Semantic" => Ok(Tier::Semantic),
        "Familiar" => Ok(Tier::Familiar),
        "Native" => Ok(Tier::Native),
        other => Err(CanonicalReportParseError::InvalidTier(other.to_string())),
    }
}

fn format_state(state: CompatibilityState) -> &'static str {
    match state {
        CompatibilityState::Supported => "Supported",
        CompatibilityState::PartiallySupported => "PartiallySupported",
        CompatibilityState::PlatformLimited => "PlatformLimited",
        CompatibilityState::Unknown => "Unknown",
        CompatibilityState::Unsupported => "Unsupported",
        CompatibilityState::ExternallyUnavailable => "ExternallyUnavailable",
    }
}

fn parse_state(value: &str) -> Result<CompatibilityState, CanonicalReportParseError> {
    match value {
        "Supported" => Ok(CompatibilityState::Supported),
        "PartiallySupported" => Ok(CompatibilityState::PartiallySupported),
        "PlatformLimited" => Ok(CompatibilityState::PlatformLimited),
        "Unknown" => Ok(CompatibilityState::Unknown),
        "Unsupported" => Ok(CompatibilityState::Unsupported),
        "ExternallyUnavailable" => Ok(CompatibilityState::ExternallyUnavailable),
        other => Err(CanonicalReportParseError::InvalidState(other.to_string())),
    }
}

fn escape_canonical_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn unescape_canonical_str(value: &str) -> Result<String, CanonicalReportParseError> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                _ => {
                    return Err(CanonicalReportParseError::InvalidEscapeSequence(
                        value.to_string(),
                    ))
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn format_optional_str(value: Option<&str>) -> String {
    match value {
        Some(inner) => format!("Some({})", escape_canonical_str(inner)),
        None => "None".to_string(),
    }
}

fn parse_optional_str(value: &str) -> Result<Option<String>, CanonicalReportParseError> {
    if value == "None" {
        return Ok(None);
    }
    if let Some(rest) = value.strip_prefix("Some(") {
        if let Some(inner) = rest.strip_suffix(')') {
            return Ok(Some(unescape_canonical_str(inner)?));
        }
    }
    Err(CanonicalReportParseError::InvalidEscapeSequence(
        value.to_string(),
    ))
}

pub fn render_canonical(report: &CompatibilityReport) -> String {
    let canonical =
        build_canonical_report(&report.produced_by_unitas_version, report.findings.clone());
    let summary = canonical.summary();
    let mut out = String::new();
    out.push_str("UNITAS_COMPAT_REPORT v1\n");
    out.push_str(&format!(
        "unitas_version: {}\n",
        escape_canonical_str(&canonical.produced_by_unitas_version)
    ));
    out.push_str(&format!(
        "overall_state: {}\n",
        format_state(summary.overall_state)
    ));
    out.push_str(&format!("total_findings: {}\n", summary.total_findings));
    out.push_str(&format!("present_findings: {}\n", summary.present_findings));
    out.push_str(&format!("absent_findings: {}\n", summary.absent_findings));
    out.push_str(&format!("supported_count: {}\n", summary.supported_count));
    out.push_str(&format!(
        "partially_supported_count: {}\n",
        summary.partially_supported_count
    ));
    out.push_str(&format!(
        "platform_limited_count: {}\n",
        summary.platform_limited_count
    ));
    out.push_str(&format!("unknown_count: {}\n", summary.unknown_count));
    out.push_str(&format!(
        "unsupported_count: {}\n",
        summary.unsupported_count
    ));
    out.push_str(&format!(
        "externally_unavailable_count: {}\n",
        summary.externally_unavailable_count
    ));
    out.push_str(&format!("packages_count: {}\n", canonical.packages.len()));
    for package in &canonical.packages {
        out.push_str("[package]\n");
        out.push_str(&format!(
            "package_name: {}\n",
            escape_canonical_str(&package.package_name)
        ));
        out.push_str(&format!("state: {}\n", format_state(package.state)));
        for feature_id in &package.feature_ids {
            out.push_str(&format!(
                "feature_id: {}\n",
                escape_canonical_str(feature_id)
            ));
        }
    }
    out.push_str(&format!("findings_count: {}\n", canonical.findings.len()));
    for finding in &canonical.findings {
        out.push_str("[finding]\n");
        out.push_str(&format!(
            "feature_id: {}\n",
            escape_canonical_str(&finding.feature_id)
        ));
        out.push_str(&format!("tier: {}\n", format_tier(finding.tier)));
        out.push_str(&format!("state: {}\n", format_state(finding.state)));
        out.push_str(&format!(
            "content.guid: {}\n",
            format_optional_str(finding.content.guid.as_deref())
        ));
        out.push_str(&format!(
            "content.asset_path: {}\n",
            format_optional_str(finding.content.asset_path.as_deref())
        ));
        out.push_str(&format!(
            "content.sub_asset_id: {}\n",
            format_optional_str(finding.content.sub_asset_id.as_deref())
        ));
        out.push_str(&format!(
            "content.object_name: {}\n",
            format_optional_str(finding.content.object_name.as_deref())
        ));
        out.push_str(&format!(
            "behavior: {}\n",
            escape_canonical_str(&finding.behavior)
        ));
        out.push_str(&format!(
            "observable_consequence: {}\n",
            escape_canonical_str(&finding.observable_consequence)
        ));
        for difference in &finding.relied_upon_unity_differences {
            out.push_str(&format!(
                "unity_difference: {}\n",
                escape_canonical_str(difference)
            ));
        }
        for condition in &finding.conditions {
            out.push_str(&format!("condition: {}\n", escape_canonical_str(condition)));
        }
        for related in &finding.related_findings {
            out.push_str(&format!(
                "related_finding: {}\n",
                escape_canonical_str(related)
            ));
        }
    }
    out
}

struct PendingFinding {
    feature_id: Option<String>,
    tier: Option<Tier>,
    state: Option<CompatibilityState>,
    guid: Option<String>,
    asset_path: Option<String>,
    sub_asset_id: Option<String>,
    object_name: Option<String>,
    behavior: Option<String>,
    observable_consequence: Option<String>,
    relied_upon_unity_differences: Vec<String>,
    conditions: Vec<String>,
    related_findings: Vec<String>,
}

impl PendingFinding {
    fn new() -> Self {
        Self {
            feature_id: None,
            tier: None,
            state: None,
            guid: None,
            asset_path: None,
            sub_asset_id: None,
            object_name: None,
            behavior: None,
            observable_consequence: None,
            relied_upon_unity_differences: Vec::new(),
            conditions: Vec::new(),
            related_findings: Vec::new(),
        }
    }

    fn finish(self, line_number: usize) -> Result<Finding, CanonicalReportParseError> {
        let Some(feature_id) = self.feature_id else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing feature_id in [finding]".to_string(),
            });
        };
        let Some(tier) = self.tier else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing tier in [finding]".to_string(),
            });
        };
        let Some(state) = self.state else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing state in [finding]".to_string(),
            });
        };
        let Some(behavior) = self.behavior else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing behavior in [finding]".to_string(),
            });
        };
        let Some(observable_consequence) = self.observable_consequence else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: "missing observable_consequence in [finding]".to_string(),
            });
        };
        Ok(Finding {
            feature_id,
            tier,
            state,
            content: ContentIdentity {
                guid: self.guid,
                asset_path: self.asset_path,
                sub_asset_id: self.sub_asset_id,
                object_name: self.object_name,
            },
            behavior,
            relied_upon_unity_differences: self.relied_upon_unity_differences,
            observable_consequence,
            conditions: self.conditions,
            related_findings: self.related_findings,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseSection {
    Header,
    Package,
    Finding,
}

pub fn parse_canonical(text: &str) -> Result<CompatibilityReport, CanonicalReportParseError> {
    let mut lines = text.lines().enumerate();
    let Some((_, first_line)) = lines.next() else {
        return Err(CanonicalReportParseError::MissingHeader);
    };
    if first_line.trim() != "UNITAS_COMPAT_REPORT v1" {
        return Err(CanonicalReportParseError::MissingHeader);
    }

    let mut unitas_version: Option<String> = None;
    let mut section = ParseSection::Header;
    let mut findings: Vec<Finding> = Vec::new();
    let mut current_finding: Option<PendingFinding> = None;

    for (zero_index, raw_line) in lines {
        let line_number = zero_index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "[package]" {
            if let Some(pending) = current_finding.take() {
                findings.push(pending.finish(line_number)?);
            }
            section = ParseSection::Package;
            continue;
        }
        if trimmed == "[finding]" {
            if let Some(pending) = current_finding.take() {
                findings.push(pending.finish(line_number)?);
            }
            section = ParseSection::Finding;
            current_finding = Some(PendingFinding::new());
            continue;
        }
        let Some((key, raw_value)) = raw_line.split_once(": ") else {
            return Err(CanonicalReportParseError::MalformedLine {
                line_number,
                line: raw_line.to_string(),
            });
        };
        let key = key.trim();
        if key == "findings_count" || key == "packages_count" {
            continue;
        }
        match section {
            ParseSection::Header => match key {
                "unitas_version" => {
                    unitas_version = Some(unescape_canonical_str(raw_value)?);
                }
                "overall_state"
                | "total_findings"
                | "present_findings"
                | "absent_findings"
                | "supported_count"
                | "partially_supported_count"
                | "platform_limited_count"
                | "unknown_count"
                | "unsupported_count"
                | "externally_unavailable_count" => {}
                _ => {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                }
            },
            ParseSection::Package => match key {
                "package_name" | "state" | "feature_id" => {}
                _ => {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                }
            },
            ParseSection::Finding => {
                let Some(pending) = current_finding.as_mut() else {
                    return Err(CanonicalReportParseError::UnexpectedSectionLine {
                        line_number,
                        line: raw_line.to_string(),
                    });
                };
                match key {
                    "feature_id" => pending.feature_id = Some(unescape_canonical_str(raw_value)?),
                    "tier" => pending.tier = Some(parse_tier(raw_value)?),
                    "state" => pending.state = Some(parse_state(raw_value)?),
                    "content.guid" => pending.guid = parse_optional_str(raw_value)?,
                    "content.asset_path" => pending.asset_path = parse_optional_str(raw_value)?,
                    "content.sub_asset_id" => pending.sub_asset_id = parse_optional_str(raw_value)?,
                    "content.object_name" => pending.object_name = parse_optional_str(raw_value)?,
                    "behavior" => pending.behavior = Some(unescape_canonical_str(raw_value)?),
                    "observable_consequence" => {
                        pending.observable_consequence = Some(unescape_canonical_str(raw_value)?);
                    }
                    "unity_difference" => pending
                        .relied_upon_unity_differences
                        .push(unescape_canonical_str(raw_value)?),
                    "condition" => pending.conditions.push(unescape_canonical_str(raw_value)?),
                    "related_finding" => pending
                        .related_findings
                        .push(unescape_canonical_str(raw_value)?),
                    _ => {
                        return Err(CanonicalReportParseError::UnexpectedSectionLine {
                            line_number,
                            line: raw_line.to_string(),
                        });
                    }
                }
            }
        }
    }

    if let Some(pending) = current_finding.take() {
        findings.push(pending.finish(text.lines().count())?);
    }

    let Some(version) = unitas_version else {
        return Err(CanonicalReportParseError::MissingVersion);
    };

    Ok(build_canonical_report(&version, findings))
}
