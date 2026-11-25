use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::binary_ser::SerializedAsset;
use crate::prefab_variant::{PrefabVariantDefinition, ScenePrefabInstance};
use crate::project::BASELINE_UNITY_VERSION;
use crate::unity_yaml::{Node, Stream};

const MAX_FLOW_ENTRIES: usize = 4;
const FLOW_REFERENCE_KEY: &str = "fileID";
const VECTOR_COMPONENTS: &str = "xyzwrgba";
const TEMP_SUFFIX: &str = ".tmp";
const SERIALIZED_VERSION_KEY: &str = "serializedVersion";

pub fn emit_stream(stream: &Stream) -> String {
    let mut out = String::new();
    if let Some(header) = &stream.header {
        out.push_str(&format!("%YAML {}\n", header.yaml_version));
        if let (Some(handle), Some(prefix)) = (&header.tag_handle, &header.tag_prefix) {
            out.push_str(&format!("%TAG {handle} {prefix}\n"));
        }
    }
    for document in &stream.documents {
        out.push_str(&format!(
            "--- !u!{} &{}",
            document.class_id, document.file_id
        ));
        if document.stripped {
            out.push_str(" stripped");
        }
        out.push('\n');
        write_root(&mut out, &document.root);
    }
    out
}

#[derive(Debug, Clone)]
pub struct SceneFile {
    pub path: PathBuf,
    pub stream: Stream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveOutcome {
    pub path: PathBuf,
    pub written: bool,
    pub upgraded: bool,
}

#[derive(Debug)]
pub enum SceneSaveError {
    Io { path: PathBuf, message: String },
    UnknownScene { path: PathBuf },
}

impl fmt::Display for SceneSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SceneSaveError::Io { path, message } => {
                write!(formatter, "cannot write {}: {}", path.display(), message)
            }
            SceneSaveError::UnknownScene { path } => {
                write!(formatter, "scene {} is not part of the set", path.display())
            }
        }
    }
}

impl std::error::Error for SceneSaveError {}

pub fn save_stream(path: &Path, stream: &Stream) -> Result<(), SceneSaveError> {
    save_text(path, stream, false).map(|_| ())
}

pub fn save_asset(path: &Path, asset: &SerializedAsset) -> Result<(), SceneSaveError> {
    save_asset_form(path, asset, false).map(|_| ())
}

pub fn save_asset_upgraded(
    path: &Path,
    asset: &SerializedAsset,
) -> Result<SaveOutcome, SceneSaveError> {
    save_asset_form(path, asset, true)
}

pub fn emit_baseline_stream(stream: &Stream) -> String {
    let mut baseline = stream.clone();
    for document in &mut baseline.documents {
        remove_serialized_version(&mut document.root);
    }
    emit_stream(&baseline)
}

pub fn upgrade_notice(assets: &[String]) -> String {
    format!(
        "Saving upgraded assets {assets:?} in Unity {BASELINE_UNITY_VERSION} form; these assets will require Unity {BASELINE_UNITY_VERSION} LTS to open in Unity."
    )
}

fn save_asset_form(
    path: &Path,
    asset: &SerializedAsset,
    upgrade: bool,
) -> Result<SaveOutcome, SceneSaveError> {
    match asset {
        SerializedAsset::Text { stream, .. } => save_text(path, stream, upgrade),
        SerializedAsset::Binary(_) => {
            let bytes = asset.write_back();
            let written = write_if_changed(path, &bytes)?;
            Ok(SaveOutcome {
                path: path.to_path_buf(),
                written,
                upgraded: false,
            })
        }
    }
}

fn save_text(
    path: &Path,
    stream: &Stream,
    upgrade: bool,
) -> Result<SaveOutcome, SceneSaveError> {
    let rendered = emit_stream(stream);
    if fs::read(path).ok().as_deref() == Some(rendered.as_bytes()) {
        return Ok(SaveOutcome {
            path: path.to_path_buf(),
            written: false,
            upgraded: false,
        });
    }
    let upgraded = upgrade && has_serialized_version(stream);
    let text = match upgraded {
        true => emit_baseline_stream(stream),
        false => rendered,
    };
    write_atomic(path, text.as_bytes())?;
    Ok(SaveOutcome {
        path: path.to_path_buf(),
        written: true,
        upgraded,
    })
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool, SceneSaveError> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(false);
    }
    write_atomic(path, bytes)?;
    Ok(true)
}

fn has_serialized_version(stream: &Stream) -> bool {
    stream
        .documents
        .iter()
        .any(|document| contains_serialized_version(&document.root))
}

fn contains_serialized_version(node: &Node) -> bool {
    match node {
        Node::Mapping(entries) => {
            entries.iter().any(|(key, value)| {
                key == SERIALIZED_VERSION_KEY || contains_serialized_version(value)
            })
        }
        Node::Sequence(items) => items.iter().any(contains_serialized_version),
        Node::Scalar(_) => false,
    }
}

fn remove_serialized_version(node: &mut Node) {
    match node {
        Node::Mapping(entries) => {
            entries.retain(|(key, _)| key != SERIALIZED_VERSION_KEY);
            for (_, value) in entries.iter_mut() {
                remove_serialized_version(value);
            }
        }
        Node::Sequence(items) => {
            for item in items.iter_mut() {
                remove_serialized_version(item);
            }
        }
        Node::Scalar(_) => {}
    }
}

pub fn save_scene_set(scenes: &[SceneFile], edited: &Path) -> Result<(), SceneSaveError> {
    let scene = match scenes.iter().find(|scene| scene.path == edited) {
        Some(scene) => scene,
        None => {
            return Err(SceneSaveError::UnknownScene {
                path: edited.to_path_buf(),
            })
        }
    };
    save_stream(&scene.path, &scene.stream)
}

pub fn save_prefab_variant(
    path: &Path,
    definition: &PrefabVariantDefinition,
) -> Result<(), SceneSaveError> {
    definition.save_to_file(path)
}

pub fn save_scene_prefab_instance(
    path: &Path,
    stream: &mut Stream,
    instance: &ScenePrefabInstance,
    instance_file_id: i64,
) -> Result<(), SceneSaveError> {
    instance.apply_to_stream(stream, instance_file_id);
    save_stream(path, stream)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SceneSaveError> {
    let temp = temp_path(path)?;
    if let Err(error) = fs::write(&temp, bytes) {
        return Err(SceneSaveError::Io {
            path: temp,
            message: error.to_string(),
        });
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(SceneSaveError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        });
    }
    Ok(())
}

fn temp_path(path: &Path) -> Result<PathBuf, SceneSaveError> {
    let name = match path.file_name() {
        Some(name) => name,
        None => {
            return Err(SceneSaveError::Io {
                path: path.to_path_buf(),
                message: String::from("path has no file name"),
            })
        }
    };
    let mut temp = name.to_os_string();
    temp.push(TEMP_SUFFIX);
    Ok(path.with_file_name(temp))
}

fn write_root(out: &mut String, root: &Node) {
    match root {
        Node::Mapping(entries) => write_mapping_entries(out, entries, 0),
        Node::Sequence(items) => write_sequence(out, items, 0),
        Node::Scalar(text) => {
            write_scalar(out, text);
            out.push('\n');
        }
    }
}

fn write_mapping_entries(out: &mut String, entries: &[(String, Node)], indent: usize) {
    for (key, value) in entries {
        out.push_str(&" ".repeat(indent));
        write_key_value(out, key, value, indent);
    }
}

fn write_key_value(out: &mut String, key: &str, value: &Node, indent: usize) {
    write_scalar(out, key);
    out.push(':');
    match value {
        Node::Mapping(entries) => {
            if entries.is_empty() {
                out.push_str(" {}\n");
            } else if is_flow_mapping(entries) {
                out.push(' ');
                write_flow_mapping(out, entries);
                out.push('\n');
            } else {
                out.push('\n');
                write_mapping_entries(out, entries, indent + 2);
            }
        }
        Node::Sequence(items) => {
            if items.is_empty() {
                out.push_str(" []\n");
            } else {
                out.push('\n');
                write_sequence(out, items, indent);
            }
        }
        Node::Scalar(scalar) => {
            if scalar.is_empty() {
                out.push('\n');
            } else {
                out.push(' ');
                write_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn write_sequence(out: &mut String, items: &[Node], indent: usize) {
    for item in items {
        match item {
            Node::Mapping(entries) => {
                if entries.is_empty() {
                    out.push_str(&format!("{}- {{}}\n", " ".repeat(indent)));
                    continue;
                }
                if is_flow_mapping(entries) {
                    out.push_str(&format!("{}- ", " ".repeat(indent)));
                    write_flow_mapping(out, entries);
                    out.push('\n');
                    continue;
                }
                let mut remaining = entries.iter();
                if let Some((key, value)) = remaining.next() {
                    out.push_str(&format!("{}- ", " ".repeat(indent)));
                    write_key_value(out, key, value, indent + 2);
                }
                for (key, value) in remaining {
                    out.push_str(&" ".repeat(indent + 2));
                    write_key_value(out, key, value, indent + 2);
                }
            }
            Node::Sequence(inner) => {
                if inner.is_empty() {
                    out.push_str(&format!("{}- []\n", " ".repeat(indent)));
                } else {
                    out.push_str(&format!("{}-\n", " ".repeat(indent)));
                    write_sequence(out, inner, indent + 2);
                }
            }
            Node::Scalar(scalar) => {
                out.push_str(&format!("{}- ", " ".repeat(indent)));
                write_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn is_flow_mapping(entries: &[(String, Node)]) -> bool {
    if entries.is_empty() || entries.len() > MAX_FLOW_ENTRIES {
        return false;
    }
    if !entries
        .iter()
        .all(|(_, value)| matches!(value, Node::Scalar(_)))
    {
        return false;
    }
    if entries.iter().any(|(key, _)| key == FLOW_REFERENCE_KEY) {
        return true;
    }
    entries.iter().all(|(key, _)| {
        key.len() == 1
            && key
                .chars()
                .all(|character| VECTOR_COMPONENTS.contains(character))
    })
}

fn write_flow_mapping(out: &mut String, entries: &[(String, Node)]) {
    out.push('{');
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_scalar(out, key);
        out.push_str(": ");
        if let Node::Scalar(scalar) = value {
            write_scalar(out, scalar);
        }
    }
    out.push('}');
}

fn write_scalar(out: &mut String, text: &str) {
    if !needs_quotes(text) {
        out.push_str(text);
        return;
    }
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}

fn needs_quotes(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.trim() != text {
        return true;
    }
    if text.starts_with("- ")
        || text == "-"
        || text.starts_with("? ")
        || text == "?"
        || text.starts_with("! ")
        || text == "!"
        || text.starts_with('&')
        || text.starts_with('*')
        || text.starts_with('%')
        || text.starts_with('@')
        || text.starts_with('`')
    {
        return true;
    }
    text.chars().any(|character| {
        matches!(
            character,
            ':' | '#'
                | '{'
                | '}'
                | '['
                | ']'
                | ','
                | '"'
                | '\''
                | '\\'
                | '\n'
                | '\r'
                | '\t'
                | '|'
                | '>'
        )
    })
}
