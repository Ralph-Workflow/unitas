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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub company_name: Option<String>,
    pub product_name: Option<String>,
    pub bundle_version: Option<String>,
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

    Ok(DynamicsSettings { gravity: [x, y, z] })
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

pub fn parse_player_settings(path: &Path) -> Result<Option<ProjectIdentity>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_player_settings_stream(&stream).map(Some)
}

pub fn parse_player_settings_stream(
    stream: &Stream,
) -> Result<ProjectIdentity, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("PlayerSettings"))
        .ok_or(ProjectSettingsError::MissingDocument("PlayerSettings"))?;

    Ok(ProjectIdentity {
        company_name: parse_optional_non_empty_string(body, "companyName"),
        product_name: parse_optional_non_empty_string(body, "productName"),
        bundle_version: parse_optional_non_empty_string(body, "bundleVersion"),
    })
}

fn parse_optional_non_empty_string(node: &Node, key: &str) -> Option<String> {
    node.get(key)
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_optional_string(node: &Node, key: &str) -> String {
    match node.get(key).and_then(Node::as_str) {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn parse_optional_f64(node: &Node, key: &str, default: f64) -> Result<f64, ProjectSettingsError> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingBoundary {
    NextRecompilation,
    NextPlaySession,
    NextPhysicsStep,
    NextBuild,
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingValueSource {
    Default,
    TargetOverride,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTargetSetting {
    pub target_group_id: String,
    pub target_name: String,
    pub effective_value: String,
    pub source: SettingValueSource,
    pub boundary: SettingBoundary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProjectSettings {
    pub fixed_timestep: f64,
    pub maximum_allowed_timestep: f64,
    pub time_scale: f64,
    pub gravity: [f64; 3],
    pub layer_collision_matrix: [u32; 32],
    pub gravity_2d: [f64; 2],
    pub layer_collision_matrix_2d: [u32; 32],
    pub identity: ProjectIdentity,
    pub default_scripting_defines: String,
    pub target_scripting_defines: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixedStepBoundaryEvaluation {
    pub fixed_timestep: f64,
    pub fixed_steps_executed: usize,
    pub simulated_seconds: f64,
    pub residual_accumulator_seconds: f64,
    pub gravity_y_displacement_meters: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionBoundaryEvaluation {
    pub layer_a: usize,
    pub layer_b: usize,
    pub collides_3d: bool,
    pub collides_2d: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecompileBoundaryEvaluation {
    pub target_group_id: String,
    pub target_name: String,
    pub active_symbols: Vec<String>,
    pub source: SettingValueSource,
}

impl LoadedProjectSettings {
    pub fn fixed_timestep(&self) -> f64 {
        self.fixed_timestep
    }

    pub fn fixed_timestep_boundary(&self) -> SettingBoundary {
        SettingBoundary::NextPlaySession
    }

    pub fn collision_matrix_boundary(&self) -> SettingBoundary {
        SettingBoundary::NextPhysicsStep
    }

    pub fn layers_collide(&self, layer_a: usize, layer_b: usize) -> bool {
        if layer_a >= 32 || layer_b >= 32 {
            return false;
        }
        (self.layer_collision_matrix[layer_a] & (1u32 << layer_b)) != 0
    }

    pub fn layers_collide_2d(&self, layer_a: usize, layer_b: usize) -> bool {
        if layer_a >= 32 || layer_b >= 32 {
            return false;
        }
        (self.layer_collision_matrix_2d[layer_a] & (1u32 << layer_b)) != 0
    }

    pub fn evaluate_fixed_step_boundary(
        &self,
        frame_delta_seconds: f64,
    ) -> FixedStepBoundaryEvaluation {
        let clamped_delta = frame_delta_seconds
            .max(0.0)
            .min(self.maximum_allowed_timestep);
        let scaled_delta = clamped_delta * self.time_scale.max(0.0);
        let dt = self.fixed_timestep;
        if !dt.is_finite() || dt <= 0.0 {
            return FixedStepBoundaryEvaluation {
                fixed_timestep: dt,
                fixed_steps_executed: 0,
                simulated_seconds: 0.0,
                residual_accumulator_seconds: scaled_delta,
                gravity_y_displacement_meters: 0.0,
            };
        }
        let mut accumulator = scaled_delta;
        let mut steps = 0usize;
        let mut velocity_y = 0.0f64;
        let mut position_y = 0.0f64;
        while accumulator + 1e-12 >= dt {
            accumulator = (accumulator - dt).max(0.0);
            steps += 1;
            velocity_y += self.gravity[1] * dt;
            position_y += velocity_y * dt;
        }
        FixedStepBoundaryEvaluation {
            fixed_timestep: dt,
            fixed_steps_executed: steps,
            simulated_seconds: (steps as f64) * dt,
            residual_accumulator_seconds: accumulator,
            gravity_y_displacement_meters: position_y,
        }
    }

    pub fn evaluate_collision_boundary(
        &self,
        layer_a: usize,
        layer_b: usize,
    ) -> CollisionBoundaryEvaluation {
        CollisionBoundaryEvaluation {
            layer_a,
            layer_b,
            collides_3d: self.layers_collide(layer_a, layer_b),
            collides_2d: self.layers_collide_2d(layer_a, layer_b),
        }
    }

    pub fn evaluate_recompile_boundary(
        &self,
        target_group_id: &str,
        target_name: &str,
    ) -> RecompileBoundaryEvaluation {
        let effective = self.effective_scripting_defines(target_group_id, target_name);
        let active_symbols = effective
            .effective_value
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        RecompileBoundaryEvaluation {
            target_group_id: effective.target_group_id,
            target_name: effective.target_name,
            active_symbols,
            source: effective.source,
        }
    }

    pub fn effective_scripting_defines(
        &self,
        target_group_id: &str,
        target_name: &str,
    ) -> EffectiveTargetSetting {
        if let Some((_, value)) = self
            .target_scripting_defines
            .iter()
            .find(|(k, v)| k == target_group_id && !v.is_empty())
        {
            return EffectiveTargetSetting {
                target_group_id: target_group_id.to_string(),
                target_name: target_name.to_string(),
                effective_value: value.clone(),
                source: SettingValueSource::TargetOverride,
                boundary: SettingBoundary::NextRecompilation,
            };
        }
        EffectiveTargetSetting {
            target_group_id: target_group_id.to_string(),
            target_name: target_name.to_string(),
            effective_value: self.default_scripting_defines.clone(),
            source: SettingValueSource::Default,
            boundary: SettingBoundary::NextRecompilation,
        }
    }
}

pub fn parse_layer_collision_matrix(hex: &str) -> [u32; 32] {
    let mut matrix = [0xFFFF_FFFFu32; 32];
    let trimmed = hex.trim();
    for (layer, chunk) in trimmed.as_bytes().chunks(8).take(32).enumerate() {
        if chunk.len() == 8 {
            if let Ok(hex_str) = std::str::from_utf8(chunk) {
                if let Ok(bytes) = u32::from_str_radix(hex_str, 16) {
                    matrix[layer] = bytes.swap_bytes();
                }
            }
        }
    }
    matrix
}

pub fn format_layer_collision_matrix(matrix: &[u32; 32]) -> String {
    let mut out = String::with_capacity(256);
    for &mask in matrix {
        let swapped = mask.swap_bytes();
        out.push_str(&format!("{swapped:08x}"));
    }
    out
}

pub fn load_project_settings(
    settings_dir: &Path,
) -> Result<LoadedProjectSettings, ProjectSettingsError> {
    let time_path = settings_dir.join("TimeManager.asset");
    let mut fixed_timestep = DEFAULT_FIXED_TIMESTEP;
    let mut maximum_allowed_timestep = 0.33333334;
    let mut time_scale = 1.0;
    if time_path.exists() {
        let stream = unity_yaml::parse_file(&time_path)?;
        if let Ok(t) = parse_time_manager_stream(&stream) {
            fixed_timestep = t.fixed_timestep;
        }
        if let Some(body) = stream
            .documents
            .iter()
            .filter(|d| !d.stripped)
            .find_map(|d| d.root.get("TimeManager"))
        {
            if let Some(max_dt) = body
                .get("m_MaximumAllowedTimestep")
                .and_then(Node::as_f64)
                .filter(|v| v.is_finite() && *v > 0.0)
            {
                maximum_allowed_timestep = max_dt;
            }
            if let Some(scale) = body
                .get("m_TimeScale")
                .and_then(Node::as_f64)
                .filter(|v| v.is_finite() && *v >= 0.0)
            {
                time_scale = scale;
            }
        }
    }

    let dynamics_path = settings_dir.join("DynamicsManager.asset");
    let mut gravity = DEFAULT_GRAVITY;
    let mut layer_collision_matrix = [0xFFFF_FFFFu32; 32];
    if dynamics_path.exists() {
        let stream = unity_yaml::parse_file(&dynamics_path)?;
        if let Ok(dyn_settings) = parse_dynamics_manager_stream(&stream) {
            gravity = dyn_settings.gravity;
        }
        if let Some(body) = stream
            .documents
            .iter()
            .filter(|d| !d.stripped)
            .find_map(|d| {
                d.root
                    .get("PhysicsManager")
                    .or_else(|| d.root.get("DynamicsManager"))
            })
        {
            if let Some(hex) = body.get("m_LayerCollisionMatrix").and_then(Node::as_str) {
                layer_collision_matrix = parse_layer_collision_matrix(hex);
            }
        }
    }

    let physics2d_path = settings_dir.join("Physics2DSettings.asset");
    let mut gravity_2d = [0.0, -9.81];
    let mut layer_collision_matrix_2d = [0xFFFF_FFFFu32; 32];
    if physics2d_path.exists() {
        let stream = unity_yaml::parse_file(&physics2d_path)?;
        if let Some(body) = stream
            .documents
            .iter()
            .filter(|d| !d.stripped)
            .find_map(|d| d.root.get("Physics2DSettings"))
        {
            if let Some(g_node) = body.get("m_Gravity") {
                if let (Some(gx), Some(gy)) =
                    (parse_finite_f64(g_node, "x"), parse_finite_f64(g_node, "y"))
                {
                    gravity_2d = [gx, gy];
                }
            }
            if let Some(hex) = body.get("m_LayerCollisionMatrix").and_then(Node::as_str) {
                layer_collision_matrix_2d = parse_layer_collision_matrix(hex);
            }
        }
    }

    let player_path = settings_dir.join("ProjectSettings.asset");
    let mut identity = ProjectIdentity::default();
    let mut default_scripting_defines = String::new();
    let mut target_scripting_defines = Vec::new();
    if player_path.exists() {
        let stream = unity_yaml::parse_file(&player_path)?;
        if let Ok(id) = parse_player_settings_stream(&stream) {
            identity = id;
        }
        if let Some(body) = stream
            .documents
            .iter()
            .filter(|d| !d.stripped)
            .find_map(|d| d.root.get("PlayerSettings"))
        {
            if let Some(Node::Mapping(entries)) = body.get("scriptingDefineSymbols") {
                for (key, val) in entries {
                    if let Some(text) = val.as_str() {
                        if key == "default" {
                            default_scripting_defines = text.to_string();
                        } else {
                            target_scripting_defines.push((key.clone(), text.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(LoadedProjectSettings {
        fixed_timestep,
        maximum_allowed_timestep,
        time_scale,
        gravity,
        layer_collision_matrix,
        gravity_2d,
        layer_collision_matrix_2d,
        identity,
        default_scripting_defines,
        target_scripting_defines,
    })
}
