use crate::asset_tools::open_project;
use crate::descriptor::Scope;
use crate::dispatch::{AbiError, DispatchTableBuilder, required_string, tool_descriptor};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;
use unitas_asset::dependencies::{AssetImportState, DependencyGraph};
use unitas_asset::project_settings::{self, InputManagerSettings};

pub fn register(builder: DispatchTableBuilder) -> Result<DispatchTableBuilder, AbiError> {
    let builder = builder.tool(
        tool_descriptor(
            "diagnostics.list",
            "List the trust, package, and import diagnostics of the open project",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(list_diagnostics),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "profiler.capture",
            "Capture the timing markers and counters of a project inspection pass",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(capture_profile),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "accessibility.inspect",
            "Inspect the fonts, text assets, sorting layers, and input axes of the open project",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(inspect_accessibility),
    )?;
    Ok(builder)
}

const fn severity_for(state: AssetImportState) -> &'static str {
    match state {
        AssetImportState::Failed => "error",
        AssetImportState::Missing => "warning",
        AssetImportState::BlockedByDependency => "warning",
        AssetImportState::BlockedByTrust => "warning",
        AssetImportState::ReadyWithLimitations => "information",
        AssetImportState::NotYetEvaluated => "information",
        AssetImportState::Processing => "information",
        AssetImportState::Ready => "information",
    }
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

fn list_diagnostics(args: &Value) -> Result<Value, String> {
    let (project, open_report) = open_project(args)?;
    let mut diagnostics: Vec<Value> = Vec::new();
    for blocked in &open_report.trust_gate.blocked_vectors {
        diagnostics.push(json!({
            "severity": "warning",
            "category": "trust",
            "identifier": blocked.kind.label(),
            "message": format!(
                "the {} '{}' is blocked until the project is trusted",
                blocked.kind.label(),
                blocked.identifier
            ),
            "location": blocked.identifier,
        }));
    }
    for modification in &open_report.package_modifications {
        diagnostics.push(json!({
            "severity": "warning",
            "category": "package",
            "identifier": modification.package,
            "message": modification.message,
            "location": modification.package,
        }));
    }
    for entry in &project.inventory().entries {
        let state = project.asset_import_state(entry);
        if state.is_fully_usable() {
            continue;
        }
        diagnostics.push(json!({
            "severity": severity_for(state),
            "category": "import",
            "identifier": entry.guid,
            "message": format!(
                "the asset '{}' is {}",
                entry.path.to_string_lossy(),
                state.label()
            ),
            "location": entry.path.to_string_lossy(),
            "state": state.label(),
        }));
    }
    Ok(json!({
        "diagnostics": diagnostics,
        "count": diagnostics.len(),
    }))
}

fn elapsed_micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn capture_profile(args: &Value) -> Result<Value, String> {
    let root = required_string(args, "projectPath")?;
    let started = Instant::now();
    let open_started = Instant::now();
    let (project, _report) = open_project(args)?;
    let open_micros = elapsed_micros(open_started);
    let graph_started = Instant::now();
    let graph = DependencyGraph::build(&project).map_err(|error| error.to_string())?;
    let graph_micros = elapsed_micros(graph_started);
    let settings_started = Instant::now();
    let settings = project_settings::load_project_settings(Path::new(&root))
        .map_err(|error| error.to_string())?;
    let settings_micros = elapsed_micros(settings_started);
    let cycles = graph.cycles();
    Ok(json!({
        "markers": [
            { "name": "project.open", "durationMicros": open_micros },
            { "name": "dependencyGraph.build", "durationMicros": graph_micros },
            { "name": "projectSettings.load", "durationMicros": settings_micros },
        ],
        "totalMicros": elapsed_micros(started),
        "counters": {
            "assets": project.inventory().entries.len(),
            "cycles": cycles.len(),
            "qualityLevels": settings.quality.levels.len(),
        },
    }))
}

fn inspect_accessibility(args: &Value) -> Result<Value, String> {
    let root = required_string(args, "projectPath")?;
    let (project, _report) = open_project(args)?;
    let mut fonts: Vec<String> = Vec::new();
    let mut text_assets: Vec<String> = Vec::new();
    for entry in &project.inventory().entries {
        let extension = extension_of(&entry.path);
        if matches!(extension.as_str(), "ttf" | "otf") {
            fonts.push(entry.path.to_string_lossy().to_string());
        }
        if matches!(
            extension.as_str(),
            "txt" | "json" | "csv" | "xml" | "html" | "htm" | "yaml" | "fnt"
        ) {
            text_assets.push(entry.path.to_string_lossy().to_string());
        }
    }
    let settings = project_settings::load_project_settings(Path::new(&root))
        .map_err(|error| error.to_string())?;
    let sorting_layers: Vec<String> =
        settings
            .tag_manager
            .as_ref()
            .map_or_else(Vec::new, |manager| {
                manager
                    .sorting_layers
                    .iter()
                    .map(|layer| layer.name.clone())
                    .collect()
            });
    let input = project_settings::parse_input_manager(
        &Path::new(&root)
            .join("ProjectSettings")
            .join("InputManager.asset"),
    )
    .map_err(|error| error.to_string())?
    .unwrap_or_else(|| InputManagerSettings {
        axes: Vec::new(),
        skipped_axes: 0,
    });
    let axes: Vec<Value> = input
        .axes
        .iter()
        .map(|axis| {
            json!({
                "name": axis.name,
                "positiveButton": axis.positive_button,
                "negativeButton": axis.negative_button,
                "altPositiveButton": axis.alt_negative_button,
            })
        })
        .collect();
    Ok(json!({
        "fonts": fonts,
        "textAssets": text_assets,
        "sortingLayers": sorting_layers,
        "inputAxes": axes,
        "skippedInputAxes": input.skipped_axes,
        "notInspected": [
            "accessibility roles, labels, and focus order are owned by the editor scene model",
            "contrast and text-scale readiness are not computed from the project assets",
        ],
    }))
}
