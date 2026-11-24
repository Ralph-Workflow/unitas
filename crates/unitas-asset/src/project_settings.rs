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

fn parse_finite_f64(node: &Node, key: &str) -> Option<f64> {
    node.get(key)
        .and_then(Node::as_f64)
        .filter(|value| value.is_finite())
}

#[cfg(test)]
#[path = "../tests/project_settings.rs"]
mod tests;
