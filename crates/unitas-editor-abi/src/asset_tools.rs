use crate::descriptor::Scope;
use crate::dispatch::{
    AbiError, DispatchTableBuilder, optional_bool, optional_string, optional_u64, required_string,
    tool_descriptor,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use unitas_asset::database::AssetFilter;
use unitas_asset::dependencies::{AssetImportState, DependencyGraph};
use unitas_asset::project::{AssetKind, OpenReport, Project};
use unitas_asset::scene_save;
use unitas_asset::trust::{TrustStore, per_user_trust_store_path};
use unitas_asset::unity_yaml::{self, Node};

static SESSIONS: OnceLock<Mutex<HashMap<PathBuf, Project>>> = OnceLock::new();

pub fn register(builder: DispatchTableBuilder) -> Result<DispatchTableBuilder, AbiError> {
    let builder = builder.tool(
        tool_descriptor(
            "assets.search",
            "Search the project asset database by name, class, label, folder, and import state",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "nameContains": { "type": "string" },
                "class": { "type": "string" },
                "label": { "type": "string" },
                "folder": { "type": "string" },
                "state": { "type": "string" },
                "includeSubAssets": { "type": "boolean" },
                "limit": { "type": "integer" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(search_assets),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.preview",
            "Preview one project asset with its class, size, import state, and summary",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "path": { "type": "string" },
            }),
            &["projectPath", "path"],
            Scope::ReadProject,
        ),
        Box::new(preview_asset),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.move",
            "Move a project asset and its meta file to another project path",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "from": { "type": "string" },
                "to": { "type": "string" },
            }),
            &["projectPath", "from", "to"],
            Scope::EditContent,
        ),
        Box::new(move_asset),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.delete",
            "Delete a project asset into the project trash so it can be restored",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "path": { "type": "string" },
            }),
            &["projectPath", "path"],
            Scope::EditContent,
        ),
        Box::new(delete_asset),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.restore",
            "Restore a deleted project asset from the project trash",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "path": { "type": "string" },
            }),
            &["projectPath", "path"],
            Scope::EditContent,
        ),
        Box::new(restore_asset),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.duplicate",
            "Duplicate a project asset under a new path with a fresh GUID",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "from": { "type": "string" },
                "to": { "type": "string" },
            }),
            &["projectPath", "from", "to"],
            Scope::EditContent,
        ),
        Box::new(duplicate_asset),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.favorites",
            "List, add, or remove the favorite assets of the open project",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "action": { "type": "string" },
                "guid": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(favorites),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.recents",
            "List the recently used assets of the open project",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(recents),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "assets.dependencies",
            "Report the dependencies, reverse dependencies, and cycles of a project asset",
            json!({
                "projectPath": { "type": "string" },
                "trustStorePath": { "type": "string" },
                "guid": { "type": "string" },
                "transitive": { "type": "boolean" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(dependencies),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "settings.inspect",
            "Inspect the project settings families and their effective values",
            json!({
                "projectPath": { "type": "string" },
            }),
            &["projectPath"],
            Scope::ReadProject,
        ),
        Box::new(inspect_settings),
    )?;
    let builder = builder.tool(
        tool_descriptor(
            "settings.update",
            "Write one project setting value back into its ProjectSettings file",
            json!({
                "projectPath": { "type": "string" },
                "file": { "type": "string" },
                "key": { "type": "string" },
                "value": { "type": "string" },
            }),
            &["projectPath", "file", "key", "value"],
            Scope::ChangePackagesAndSettings,
        ),
        Box::new(update_setting),
    )?;
    Ok(builder)
}

fn trust_store(args: &Value) -> TrustStore {
    match optional_string(args, "trustStorePath") {
        Some(path) => TrustStore::new(path),
        None => TrustStore::new(per_user_trust_store_path()),
    }
}

pub fn open_project(args: &Value) -> Result<(Project, OpenReport), String> {
    let root = required_string(args, "projectPath")?;
    Project::open(Path::new(&root), &trust_store(args))
        .map_err(|error| format!("cannot open the project at '{root}': {error}"))
}

fn sessions() -> &'static Mutex<HashMap<PathBuf, Project>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_project<T>(
    args: &Value,
    action: impl FnOnce(&mut Project) -> Result<T, String>,
) -> Result<T, String> {
    let root = required_string(args, "projectPath")?;
    let key = PathBuf::from(&root);
    let mut guard = sessions()
        .lock()
        .map_err(|_| "the project session lock is poisoned".to_string())?;
    if !guard.contains_key(&key) {
        let (project, _report) = Project::open(Path::new(&root), &trust_store(args))
            .map_err(|error| format!("cannot open the project at '{root}': {error}"))?;
        guard.insert(key.clone(), project);
    }
    let project = guard
        .get_mut(&key)
        .ok_or_else(|| "the project session is missing".to_string())?;
    action(project)
}

fn import_state(label: &str) -> Result<AssetImportState, String> {
    match label {
        "not yet evaluated" => Ok(AssetImportState::NotYetEvaluated),
        "processing" => Ok(AssetImportState::Processing),
        "ready" => Ok(AssetImportState::Ready),
        "ready with limitations" => Ok(AssetImportState::ReadyWithLimitations),
        "failed" => Ok(AssetImportState::Failed),
        "missing" => Ok(AssetImportState::Missing),
        "blocked by dependency" => Ok(AssetImportState::BlockedByDependency),
        "blocked by trust" => Ok(AssetImportState::BlockedByTrust),
        other => Err(format!("unknown asset import state '{other}'")),
    }
}

fn asset_filter(args: &Value) -> Result<AssetFilter, String> {
    let state = match optional_string(args, "state") {
        Some(state) => Some(import_state(&state)?),
        None => None,
    };
    Ok(AssetFilter {
        name_contains: optional_string(args, "nameContains"),
        class: optional_string(args, "class"),
        label: optional_string(args, "label"),
        folder: optional_string(args, "folder").map(PathBuf::from),
        state,
        include_sub_assets: optional_bool(args, "includeSubAssets"),
    })
}

fn search_assets(args: &Value) -> Result<Value, String> {
    let filter = asset_filter(args)?;
    let limit =
        optional_u64(args, "limit").map_or(200usize, |value| usize::try_from(value).unwrap_or(200));
    with_project(args, |project| {
        let entries = project.search_assets(&filter);
        let assets: Vec<Value> = entries
            .iter()
            .take(limit)
            .map(|entry| {
                json!({
                    "guid": entry.guid,
                    "path": entry.path.to_string_lossy(),
                    "kind": if entry.kind == AssetKind::Main { "main" } else { "sub" },
                    "class": entry.class,
                    "labels": entry.labels,
                })
            })
            .collect();
        Ok(json!({
            "assets": assets,
            "count": entries.len(),
            "truncated": entries.len() > limit,
        }))
    })
}

fn preview_asset(args: &Value) -> Result<Value, String> {
    let path = required_string(args, "path")?;
    with_project(args, |project| {
        let preview = project
            .preview_asset(Path::new(&path))
            .map_err(|error| error.to_string())?;
        Ok(json!({
            "guid": preview.guid,
            "path": preview.path.to_string_lossy(),
            "family": preview.family.map(|family| family.family_label()),
            "className": preview.class_name,
            "byteSize": preview.byte_size,
            "state": preview.state.label(),
            "summary": preview.summary,
        }))
    })
}

fn move_asset(args: &Value) -> Result<Value, String> {
    let from = required_string(args, "from")?;
    let to = required_string(args, "to")?;
    with_project(args, |project| {
        project
            .move_asset(Path::new(&from), Path::new(&to))
            .map_err(|error| error.to_string())?;
        Ok(json!({"moved": true, "from": from, "to": to}))
    })
}

fn delete_asset(args: &Value) -> Result<Value, String> {
    let path = required_string(args, "path")?;
    with_project(args, |project| {
        project
            .delete_asset(Path::new(&path))
            .map_err(|error| error.to_string())?;
        Ok(json!({"deleted": true, "path": path}))
    })
}

fn restore_asset(args: &Value) -> Result<Value, String> {
    let path = required_string(args, "path")?;
    with_project(args, |project| {
        project
            .restore_asset(Path::new(&path))
            .map_err(|error| error.to_string())?;
        Ok(json!({"restored": true, "path": path}))
    })
}

fn duplicate_asset(args: &Value) -> Result<Value, String> {
    let from = required_string(args, "from")?;
    let to = required_string(args, "to")?;
    with_project(args, |project| {
        let guid = project
            .duplicate_asset(Path::new(&from), Path::new(&to))
            .map_err(|error| error.to_string())?;
        Ok(json!({"duplicated": true, "guid": guid, "from": from, "to": to}))
    })
}

fn favorites(args: &Value) -> Result<Value, String> {
    let action = optional_string(args, "action").unwrap_or_else(|| "list".to_string());
    let guid = optional_string(args, "guid");
    with_project(args, |project| match action.as_str() {
        "add" => {
            let guid = guid.ok_or_else(|| {
                "assets.favorites needs a 'guid' argument to add a favorite".to_string()
            })?;
            let added = project.add_favorite(&guid);
            Ok(json!({"added": added, "favorites": project.favorites()}))
        }
        "remove" => {
            let guid = guid.ok_or_else(|| {
                "assets.favorites needs a 'guid' argument to remove a favorite".to_string()
            })?;
            let removed = project.remove_favorite(&guid);
            Ok(json!({"removed": removed, "favorites": project.favorites()}))
        }
        "list" => Ok(json!({"favorites": project.favorites()})),
        other => Err(format!("unknown assets.favorites action '{other}'")),
    })
}

fn recents(args: &Value) -> Result<Value, String> {
    with_project(args, |project| {
        Ok(json!({"recent": project.recent_assets()}))
    })
}

fn dependencies(args: &Value) -> Result<Value, String> {
    let transitive = optional_bool(args, "transitive");
    let guid = optional_string(args, "guid");
    with_project(args, |project| {
        let graph = DependencyGraph::build(project).map_err(|error| error.to_string())?;
        let cycles = graph.cycles();
        match guid {
            Some(guid) => Ok(json!({
                "guid": guid,
                "dependencies": graph.dependencies_of(&guid),
                "reverseDependencies": graph.reverse_dependencies_of(&guid, transitive),
                "cycles": cycles,
            })),
            None => Ok(json!({
                "assetCount": project.inventory().entries.len(),
                "cycles": cycles,
            })),
        }
    })
}

fn inspect_settings(args: &Value) -> Result<Value, String> {
    let root = required_string(args, "projectPath")?;
    let settings = unitas_asset::project_settings::load_project_settings(Path::new(&root))
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
    let tags: Vec<String> = settings
        .tag_manager
        .as_ref()
        .map_or_else(Vec::new, |manager| manager.tags.clone());
    let quality_levels: Vec<String> = settings
        .quality
        .levels
        .iter()
        .map(|level| level.name.clone())
        .collect();
    let target_defines: Vec<Value> = settings
        .target_scripting_defines
        .iter()
        .map(|(target, defines)| json!({"target": target, "defines": defines}))
        .collect();
    Ok(json!({
        "fixedTimestep": settings.fixed_timestep,
        "maximumAllowedTimestep": settings.maximum_allowed_timestep,
        "timeScale": settings.time_scale,
        "gravity": settings.gravity,
        "gravity2d": settings.gravity_2d,
        "colorSpace": settings.identity.active_color_space.code(),
        "companyName": settings.identity.company_name,
        "productName": settings.identity.product_name,
        "bundleVersion": settings.identity.bundle_version,
        "currentQuality": settings.quality.current_quality,
        "qualityLevels": quality_levels,
        "defaultScriptingDefines": settings.default_scripting_defines,
        "targetScriptingDefines": target_defines,
        "tags": tags,
        "sortingLayers": sorting_layers,
    }))
}

fn set_scalar(node: &mut Node, keys: &[&str], value: &str) -> bool {
    let Some((key, rest)) = keys.split_first() else {
        return false;
    };
    match node {
        Node::Mapping(entries) => {
            for (entry_key, entry_value) in entries.iter_mut() {
                if entry_key != key {
                    continue;
                }
                if rest.is_empty() {
                    *entry_value = Node::Scalar(value.to_string());
                    return true;
                }
                return set_scalar(entry_value, rest, value);
            }
            false
        }
        _ => false,
    }
}

fn update_setting(args: &Value) -> Result<Value, String> {
    let root = required_string(args, "projectPath")?;
    let file = required_string(args, "file")?;
    let key = required_string(args, "key")?;
    let value = required_string(args, "value")?;
    let path = Path::new(&root).join("ProjectSettings").join(&file);
    if !path.is_file() {
        return Err(format!(
            "the project settings file '{}' does not exist",
            path.display()
        ));
    }
    let mut stream = unity_yaml::parse_file(&path).map_err(|error| format!("{error}"))?;
    let keys: Vec<&str> = key.split('.').collect();
    let applied = stream
        .documents
        .iter_mut()
        .any(|document| set_scalar(&mut document.root, &keys, &value));
    if !applied {
        return Err(format!(
            "the setting '{key}' is not present in '{}'",
            path.display()
        ));
    }
    scene_save::save_stream(&path, &stream).map_err(|error| error.to_string())?;
    Ok(json!({"updated": true, "file": file, "key": key, "value": value}))
}
