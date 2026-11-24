use std::fmt;
use std::path::Path;

use crate::unity_yaml::{self, Node, Stream, YamlError};

pub const DEFAULT_FIXED_TIMESTEP: f64 = 0.02;
pub const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSettings {
    pub fixed_timestep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicsSettings {
    pub gravity: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputAxis {
    pub name: String,
    pub negative_button: String,
    pub positive_button: String,
    pub alt_negative_button: String,
    pub alt_positive_button: String,
    pub gravity: f64,
    pub dead: f64,
    pub sensitivity: f64,
    pub snap: bool,
    pub invert: bool,
    pub axis_type: u32,
    pub axis: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputManagerSettings {
    pub axes: Vec<InputAxis>,
    pub skipped_axes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSettingsError {
    MissingDocument(&'static str),
    Yaml { error: YamlError },
}

impl fmt::Display for ProjectSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectSettingsError::MissingDocument(document) => {
                write!(formatter, "stream contains no {document} document")
            }
            ProjectSettingsError::Yaml { error } => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ProjectSettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectSettingsError::Yaml { error } => Some(error),
            ProjectSettingsError::MissingDocument(_) => None,
        }
    }
}

impl From<YamlError> for ProjectSettingsError {
    fn from(error: YamlError) -> Self {
        ProjectSettingsError::Yaml { error }
    }
}

pub fn parse_time_manager(path: &Path) -> Result<Option<TimeSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_time_manager_stream(&stream).map(Some)
}

pub fn parse_time_manager_stream(stream: &Stream) -> Result<TimeSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("TimeManager"))
        .ok_or(ProjectSettingsError::MissingDocument("TimeManager"))?;

    let fixed_timestep = body
        .get("m_FixedTimestep")
        .and_then(Node::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or(ProjectSettingsError::MissingDocument("TimeManager"))?;

    Ok(TimeSettings { fixed_timestep })
}

pub fn parse_dynamics_manager(
    path: &Path,
) -> Result<Option<DynamicsSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_dynamics_manager_stream(&stream).map(Some)
}

pub fn parse_dynamics_manager_stream(
    stream: &Stream,
) -> Result<DynamicsSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| {
            document
                .root
                .get("PhysicsManager")
                .or_else(|| document.root.get("DynamicsManager"))
        })
        .ok_or(ProjectSettingsError::MissingDocument("PhysicsManager"))?;

    let gravity_node = body
        .get("m_Gravity")
        .ok_or(ProjectSettingsError::MissingDocument("PhysicsManager"))?;

    let x = parse_finite_f64(gravity_node, "x")
        .ok_or(ProjectSettingsError::MissingDocument("PhysicsManager"))?;
    let y = parse_finite_f64(gravity_node, "y")
        .ok_or(ProjectSettingsError::MissingDocument("PhysicsManager"))?;
    let z = parse_finite_f64(gravity_node, "z")
        .ok_or(ProjectSettingsError::MissingDocument("PhysicsManager"))?;

    Ok(DynamicsSettings {
        gravity: [x, y, z],
    })
}

pub fn parse_input_manager(
    path: &Path,
) -> Result<Option<InputManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_input_manager_stream(&stream).map(Some)
}

pub fn parse_input_manager_stream(
    stream: &Stream,
) -> Result<InputManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("InputManager"))
        .ok_or(ProjectSettingsError::MissingDocument("InputManager"))?;

    let entries = body
        .get("m_Axes")
        .and_then(Node::as_sequence)
        .ok_or(ProjectSettingsError::MissingDocument("InputManager"))?;

    let mut axes = Vec::new();
    let mut skipped_axes = 0usize;

    for entry in entries {
        let name = entry
            .get("m_Name")
            .and_then(Node::as_str)
            .ok_or(ProjectSettingsError::MissingDocument("InputManager"))?
            .to_string();
        let axis_type = parse_optional_u32(entry, "type", "m_Type", 0)?;
        if axis_type >= 2 {
            skipped_axes += 1;
            continue;
        }
        let axis = parse_optional_u32(entry, "axis", "m_Axis", 0)?;
        let gravity = parse_optional_f64(entry, "gravity", 0.0)?;
        let dead = parse_optional_f64(entry, "dead", 0.0)?;
        let sensitivity = parse_optional_f64(entry, "sensitivity", 1.0)?;
        let snap = parse_optional_bool(entry, "snap", false)?;
        let invert = parse_optional_bool(entry, "invert", false)?;

        axes.push(InputAxis {
            name,
            negative_button: parse_optional_string(entry, "negativeButton"),
            positive_button: parse_optional_string(entry, "positiveButton"),
            alt_negative_button: parse_optional_string(entry, "altNegativeButton"),
            alt_positive_button: parse_optional_string(entry, "altPositiveButton"),
            gravity,
            dead,
            sensitivity,
            snap,
            invert,
            axis_type,
            axis,
        });
    }

    Ok(InputManagerSettings { axes, skipped_axes })
}

fn parse_optional_string(node: &Node, key: &str) -> String {
    match node.get(key).and_then(Node::as_str) {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn parse_optional_f64(
    node: &Node,
    key: &str,
    default: f64,
) -> Result<f64, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => child
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(ProjectSettingsError::MissingDocument("InputManager")),
        None => Ok(default),
    }
}

fn parse_optional_u32(
    node: &Node,
    primary_key: &str,
    fallback_key: &str,
    default: u32,
) -> Result<u32, ProjectSettingsError> {
    match node.get(primary_key).or_else(|| node.get(fallback_key)) {
        Some(child) => child
            .as_i64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ProjectSettingsError::MissingDocument("InputManager")),
        None => Ok(default),
    }
}

fn parse_optional_bool(
    node: &Node,
    key: &str,
    default: bool,
) -> Result<bool, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => {
            if let Some(value) = child.as_i64() {
                Ok(value != 0)
            } else if let Some(text) = child.as_str() {
                match text.trim() {
                    "true" | "True" | "TRUE" => Ok(true),
                    "false" | "False" | "FALSE" => Ok(false),
                    _ => Err(ProjectSettingsError::MissingDocument("InputManager")),
                }
            } else {
                Err(ProjectSettingsError::MissingDocument("InputManager"))
            }
        }
        None => Ok(default),
    }
}

fn parse_finite_f64(node: &Node, key: &str) -> Option<f64> {
    node.get(key)
        .and_then(Node::as_f64)
        .filter(|value| value.is_finite())
}

#[cfg(test)]
#[path = "../tests/project_settings.rs"]
mod tests;
