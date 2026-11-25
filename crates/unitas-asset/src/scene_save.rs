//! Save path for scenes and prefabs: renders a parsed stream back to Unity
//! YAML, writes a document without losing the previous file, and saves one
//! scene of an additive scene set without touching its neighbours.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::binary_ser::SerializedAsset;
use crate::unity_yaml::{Node, Stream};

const MAX_FLOW_ENTRIES: usize = 4;
const FLOW_REFERENCE_KEY: &str = "fileID";
const VECTOR_COMPONENTS: &str = "xyzwrgba";
const TEMP_SUFFIX: &str = ".tmp";

/// Renders a parsed stream as Unity YAML: the `%YAML` and `%TAG` directives,
/// then one `--- !u!<class id> &<file id>` document per parsed document with
/// its body written as plain scalars, flow mappings for references and
/// vectors, and block sequences.
pub fn emit_stream(stream: &Stream) -> String {
    let mut out = String::new();
    if let Some(header) = &stream.header {
        out.push_str(&format!("%YAML {}\n", header.yaml_version));
        if let (Some(handle), Some(prefix)) = (&header.tag_handle, &header.tag_prefix) {
            out.push_str(&format!("%TAG {handle} {prefix}\n"));
        }
    }
    for document in &stream.documents {
        out.push_str(&format!("--- !u!{} &{}", document.class_id, document.file_id));
        if document.stripped {
            out.push_str(" stripped");
        }
        out.push('\n');
        write_root(&mut out, &document.root);
    }
    out
}

/// One scene of an additive scene set: the file it was read from and its
/// parsed content.
#[derive(Debug, Clone)]
pub struct SceneFile {
    pub path: PathBuf,
    pub stream: Stream,
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

/// Writes a text scene or prefab, keeping the previous file content when the
/// write fails.
pub fn save_stream(path: &Path, stream: &Stream) -> Result<(), SceneSaveError> {
    write_atomic(path, emit_stream(stream).as_bytes())
}

/// Writes a document in the form it was read in: a text document is written
/// back as text and a binary document is written back as binary.
pub fn save_asset(path: &Path, asset: &SerializedAsset) -> Result<(), SceneSaveError> {
    match asset {
        SerializedAsset::Text { stream, .. } => save_stream(path, stream),
        SerializedAsset::Binary(_) => write_atomic(path, &asset.write_back()),
    }
}

/// Saves the edited scene of an additive scene set and leaves every other
/// scene of the set untouched.
pub fn save_scene_set(scenes: &[SceneFile], edited: &Path) -> Result<(), SceneSaveError> {
    let scene = match scenes.iter().find(|scene| scene.path == edited) {
        Some(scene) => scene,
        None => return Err(SceneSaveError::UnknownScene { path: edited.to_path_buf() }),
    };
    save_stream(&scene.path, &scene.stream)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SceneSaveError> {
    let temp = temp_path(path)?;
    if let Err(error) = fs::write(&temp, bytes) {
        return Err(SceneSaveError::Io { path: temp, message: error.to_string() });
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(SceneSaveError::Io { path: path.to_path_buf(), message: error.to_string() });
    }
    Ok(())
}

// shortcut: one fixed temp name per target, so concurrent saves of the same
// file clash; use a unique name when saves can overlap.
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
    if !entries.iter().all(|(_, value)| matches!(value, Node::Scalar(_))) {
        return false;
    }
    if entries.iter().any(|(key, _)| key == FLOW_REFERENCE_KEY) {
        return true;
    }
    entries.iter().all(|(key, _)| {
        key.len() == 1 && key.chars().all(|character| VECTOR_COMPONENTS.contains(character))
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
