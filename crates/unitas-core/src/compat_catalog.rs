use super::Tier;

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
        owning_spec: "sample-games/INDEX.md",
    },
    CatalogFeatureEntry {
        feature_id: "CMP-001",
        tier: Tier::Native,
        owning_spec: "sample-games/INDEX.md",
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

pub(super) fn owning_feature_tier(feature_id: &str, declared: Option<Tier>) -> Tier {
    lookup_catalog_feature(feature_id)
        .map(|entry| entry.tier)
        .or(declared)
        .unwrap_or(Tier::Native)
}
