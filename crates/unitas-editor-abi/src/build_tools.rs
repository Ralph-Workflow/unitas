use crate::descriptor::{Scope, ToolDescriptor};
use crate::dispatch::{AbiError, DispatchTableBuilder};

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use unitas_build::licensing::{LicensingRecord, licensing_record};
use unitas_build::{
    BUILD_REPORT_FILENAME, BuildConfiguration, BuildTarget, execute_build_for_host,
};

pub const LICENSING_RECORD_FILENAME: &str = "licensing-record.json";

pub fn register(builder: DispatchTableBuilder) -> Result<DispatchTableBuilder, AbiError> {
    builder.tool(
        ToolDescriptor {
            name: "build.player".to_string(),
            description: "Package a project directory's .unity scenes into a player bundle for a build target".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "projectPath": { "type": "string" },
                    "buildTarget": { "type": "string" },
                },
                "required": ["projectPath", "buildTarget"],
            }),
            required_scope: Scope::Build,
            origin: "unitas://native/build".to_string(),
        },
        Box::new(build_player),
    )
}

fn build_player(args: &Value) -> Result<Value, String> {
    let project_path = required_string(args, "projectPath")?;
    let target_name = required_string(args, "buildTarget")?;
    let target = parse_target(&target_name)?;
    let project_dir = PathBuf::from(&project_path);
    if !project_dir.is_dir() {
        return Err(format!("project directory does not exist: {project_path}"));
    }

    let config = build_player_config(&project_dir, target)?;
    let output_dir = project_dir.join("builds").join(target.identifier());
    let (bundle, mut report, elapsed_ms) = run_player_build(&config, &output_dir, target)?;

    report.elapsed_ms = elapsed_ms;
    let report_path = output_dir.join(BUILD_REPORT_FILENAME);
    report.write_to_file(&report_path).map_err(|error| {
        format!(
            "cannot write the build report at '{}': {error}",
            report_path.display()
        )
    })?;

    let licensing_path = write_licensing_record(&output_dir, &config.application_identifier)?;
    let build_id = bundle.manifest.manifest_hash.clone();
    Ok(json!({
        "bundlePath": bundle.output_dir.to_string_lossy(),
        "reportPath": report_path.to_string_lossy(),
        "buildId": build_id,
        "licensingRecordPath": licensing_path.to_string_lossy(),
    }))
}

fn build_player_config(
    project_dir: &Path,
    target: BuildTarget,
) -> Result<BuildConfiguration, String> {
    let scenes = project_scenes(project_dir)?;
    let product_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unitas-player")
        .to_string();
    let mut config = BuildConfiguration::for_host(scenes, product_name);
    config.target = target;
    Ok(config)
}

fn run_player_build(
    config: &BuildConfiguration,
    output_dir: &Path,
    target: BuildTarget,
) -> Result<(unitas_build::StagedBundle, unitas_build::BuildReport, u64), String> {
    let started = Instant::now();
    let outcome = execute_build_for_host(config, output_dir, 0, BuildTarget::current_host());
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let Some(bundle) = outcome.bundle else {
        let reason = outcome
            .report
            .errors
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "build failed for target {}: {reason}",
            target.identifier()
        ));
    };
    Ok((bundle, outcome.report, elapsed_ms))
}

fn required_string(args: &Value, key: &str) -> Result<String, String> {
    match args.get(key).and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Ok(text.to_string()),
        _ => Err(format!(
            "build.player requires a non-empty '{key}' argument"
        )),
    }
}

fn parse_target(name: &str) -> Result<BuildTarget, String> {
    let expected =
        "expected '<os>-<arch>' with os in windows|macos|linux and arch in x86_64|aarch64";
    let Some((os, arch)) = name.split_once('-') else {
        return Err(format!("unknown build target '{name}': {expected}"));
    };

    BuildTarget::from_env(os, arch)
        .ok_or_else(|| format!("unknown build target '{name}': {expected}"))
}

fn project_scenes(project_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut scenes = Vec::new();
    collect_scenes(project_dir, &mut scenes)?;
    scenes.sort();
    if scenes.is_empty() {
        return Err(format!(
            "the project directory '{}' contains no '.unity' scene to build",
            project_dir.display()
        ));
    }

    Ok(scenes)
}

fn collect_scenes(dir: &Path, scenes: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|error| {
        format!(
            "cannot read the project directory '{}': {error}",
            dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "cannot read the project directory '{}': {error}",
                dir.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_scenes(&path, scenes)?;
            continue;
        }
        let is_scene = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("unity"));
        if is_scene {
            scenes.push(path);
        }
    }
    Ok(())
}

fn write_licensing_record(
    output_dir: &Path,
    application_identifier: &str,
) -> Result<PathBuf, String> {
    let record = licensing_record(application_identifier);
    let path = output_dir.join(LICENSING_RECORD_FILENAME);
    let text = serde_json::to_string_pretty(&licensing_record_value(&record))
        .map_err(|error| format!("cannot encode the licensing record: {error}"))?;
    fs::write(&path, text).map_err(|error| {
        format!(
            "cannot write the licensing record at '{}': {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn licensing_record_value(record: &LicensingRecord) -> Value {
    let notices = record
        .third_party_notices
        .iter()
        .map(|notice| {
            json!({
                "ecosystem": notice.ecosystem,
                "component": notice.component,
                "version": notice.version,
                "license": notice.license,
            })
        })
        .collect::<Vec<Value>>();
    json!({
        "application_identifier": record.application_identifier,
        "governing_license": record.governing_license,
        "governing_license_name": record.governing_license_name,
        "conveyance_statement": record.conveyance_statement,
        "corresponding_source_obligation": record.corresponding_source_obligation,
        "corresponding_source_availability": record.corresponding_source_availability,
        "linking_exception_statement": record.linking_exception_statement,
        "commercial_license_statement": record.commercial_license_statement,
        "not_legal_advice_statement": record.not_legal_advice_statement,
        "license_texts": &record.license_texts,
        "third_party_notices": notices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_build::sample::marble_rush_scene_yaml;

    fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "unitas-editor-abi-{label}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn project_with_scene(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let project = temp_dir(label)?;
        let scenes = project.join("Assets").join("Scenes");
        std::fs::create_dir_all(&scenes)?;
        let scene = scenes.join("Main.unity");
        std::fs::write(&scene, marble_rush_scene_yaml())?;
        Ok(project)
    }

    fn host_target() -> Result<BuildTarget, Box<dyn std::error::Error>> {
        BuildTarget::current_host().ok_or_else(|| {
            std::io::Error::other("the current host is not a supported build target").into()
        })
    }

    fn string_field(value: &Value, key: &str) -> String {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    fn failure_of(result: Result<Value, String>) -> String {
        match result {
            Ok(_) => String::from("the tool reported success"),
            Err(error) => error,
        }
    }

    #[test]
    fn builtin_table_registers_build_player_with_the_build_scope()
    -> Result<(), Box<dyn std::error::Error>> {
        let table = crate::builtin_dispatch_table()?;
        let manifest = table.manifest();
        let Some(descriptor) = manifest
            .tools
            .iter()
            .find(|tool| tool.name == "build.player")
        else {
            return Err(std::io::Error::other("build.player is not registered").into());
        };
        assert_eq!(descriptor.required_scope, Scope::Build);
        assert_eq!(descriptor.origin, "unitas://native/build");
        assert_eq!(
            descriptor.input_schema["properties"]["buildTarget"]["type"],
            json!("string")
        );
        Ok(())
    }

    #[test]
    fn build_player_stages_a_bundle_and_writes_the_licensing_record()
    -> Result<(), Box<dyn std::error::Error>> {
        let project = project_with_scene("player-ok")?;
        let result = build_player(&json!({
            "projectPath": project.to_string_lossy(),
            "buildTarget": host_target()?.identifier(),
        }))?;

        let bundle_path = PathBuf::from(string_field(&result, "bundlePath"));
        let report_path = PathBuf::from(string_field(&result, "reportPath"));
        let licensing_path = PathBuf::from(string_field(&result, "licensingRecordPath"));
        let build_id = string_field(&result, "buildId");

        assert!(bundle_path.is_dir(), "bundle directory is missing");
        assert!(report_path.is_file(), "build report is missing");
        assert!(licensing_path.is_file(), "licensing record is missing");
        assert_eq!(build_id.len(), 64, "build id is not a content hash");

        let record_text = std::fs::read_to_string(&licensing_path)?;
        let record: Value = serde_json::from_str(&record_text)?;
        assert_eq!(record["governing_license"], json!("GPL-3.0-only"));
        let notices = record["third_party_notices"].as_array().map_or(0, Vec::len);
        assert!(notices > 0, "third-party notices are missing");
        assert!(
            record["commercial_license_statement"]
                .as_str()
                .is_some_and(|statement| statement.contains("commercial license")),
            "commercial license statement is missing"
        );

        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    #[test]
    fn build_player_names_a_missing_project_an_unknown_target_and_an_empty_project()
    -> Result<(), Box<dyn std::error::Error>> {
        let missing = temp_dir("player-missing")?;
        let missing_error = failure_of(build_player(&json!({
            "projectPath": missing.join("absent").to_string_lossy(),
            "buildTarget": host_target()?.identifier(),
        })));
        assert!(
            missing_error.contains("project directory does not exist"),
            "unexpected error: {missing_error}"
        );

        let project = project_with_scene("player-target")?;
        let target_error = failure_of(build_player(&json!({
            "projectPath": project.to_string_lossy(),
            "buildTarget": "commodore-64".to_string(),
        })));
        assert!(
            target_error.contains("unknown build target"),
            "unexpected error: {target_error}"
        );

        let empty = temp_dir("player-empty")?;
        let empty_error = failure_of(build_player(&json!({
            "projectPath": empty.to_string_lossy(),
            "buildTarget": host_target()?.identifier(),
        })));
        assert!(
            empty_error.contains("no '.unity' scene"),
            "unexpected error: {empty_error}"
        );

        let _ = std::fs::remove_dir_all(&missing);
        let _ = std::fs::remove_dir_all(&project);
        let _ = std::fs::remove_dir_all(&empty);
        Ok(())
    }
}
