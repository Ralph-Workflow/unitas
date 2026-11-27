use std::fmt;
use std::path::{Path, PathBuf};

use crate::scene_save;
use crate::unity_yaml::{self, Document, Header, Node, Stream, YamlError};

pub const DEFAULT_FIXED_TIMESTEP: f64 = 0.02;
pub const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];
pub const DEFAULT_GRAVITY_2D: [f64; 2] = [0.0, -9.81];
pub const DEFAULT_PHYSICS2D_VELOCITY_ITERATIONS: u32 = 8;
pub const DEFAULT_PHYSICS2D_POSITION_ITERATIONS: u32 = 3;

const DEFAULT_SOLVER_ITERATIONS: u32 = 6;
const DEFAULT_SOLVER_VELOCITY_ITERATIONS: u32 = 2;
const DEFAULT_BOUNCE_THRESHOLD: f64 = 2.0;
const DEFAULT_CONTACT_OFFSET: f64 = 0.01;
const DEFAULT_SLEEP_THRESHOLD: f64 = 0.005;
const DEFAULT_SLEEP_ANGULAR_THRESHOLD: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSettings {
    pub fixed_timestep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicsSettings {
    pub gravity: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsSolverType {
    Default,
    Tgs,
    TgsSoft,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsProjectSettings {
    pub gravity: [f64; 3],
    pub default_solver_iterations: u32,
    pub default_solver_velocity_iterations: u32,
    pub bounce_threshold: f64,
    pub default_contact_offset: f64,
    pub sleep_threshold: f64,
    pub sleep_angular_threshold: f64,
    pub solver_type: PhysicsSolverType,
    pub enhanced_determinism: bool,
    pub layer_collision_matrix: [u32; 32],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Physics2DProjectSettings {
    pub gravity: [f64; 2],
    pub velocity_iterations: u32,
    pub position_iterations: u32,
    pub queries_hit_triggers: bool,
    pub queries_start_in_colliders: bool,
    pub callbacks_on_disable: bool,
    pub auto_sync_transforms: bool,
    pub layer_collision_matrix: [u32; 32],
}

impl Default for Physics2DProjectSettings {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY_2D,
            velocity_iterations: DEFAULT_PHYSICS2D_VELOCITY_ITERATIONS,
            position_iterations: DEFAULT_PHYSICS2D_POSITION_ITERATIONS,
            queries_hit_triggers: true,
            queries_start_in_colliders: true,
            callbacks_on_disable: true,
            auto_sync_transforms: true,
            layer_collision_matrix: [0xFFFF_FFFFu32; 32],
        }
    }
}

impl Physics2DProjectSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_physics2d_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_physics2d_settings(path)
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct QualityLevel {
    pub name: String,
    pub pixel_light_count: u32,
    pub shadows: u32,
    pub shadow_resolution: u32,
    pub shadow_distance: f64,
    pub texture_quality: u32,
    pub anisotropic_textures: u32,
    pub anti_aliasing: u32,
    pub v_sync_count: u32,
    pub lod_bias: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct QualitySettings {
    pub current_quality: usize,
    pub levels: Vec<QualityLevel>,
    pub per_platform_default_quality: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub company_name: Option<String>,
    pub product_name: Option<String>,
    pub bundle_version: Option<String>,
    pub active_color_space: ColorSpaceSetting,
}

pub type PlayerSettings = ProjectIdentity;

impl ProjectIdentity {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_player_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_player_settings(path)
    }

    pub fn active_color_space(&self) -> ColorSpaceSetting {
        self.active_color_space
    }

    pub fn set_active_color_space(&mut self, color_space: ColorSpaceSetting) {
        self.active_color_space = color_space;
    }

    pub fn apply_to_stream(&self, stream: &mut Stream) -> Result<(), ProjectSettingsError> {
        let entries = root_mapping_mut(stream, "PlayerSettings")?;
        if let Some(company) = &self.company_name {
            set_mapping_scalar(entries, "companyName", company);
        }
        if let Some(product) = &self.product_name {
            set_mapping_scalar(entries, "productName", product);
        }
        if let Some(version) = &self.bundle_version {
            set_mapping_scalar(entries, "bundleVersion", version);
        }
        set_mapping_scalar(
            entries,
            "m_ActiveColorSpace",
            &self.active_color_space.code().to_string(),
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SortingLayerEntry {
    pub name: String,
    pub unique_id: i64,
    pub locked: bool,
}

pub type SortingLayer = SortingLayerEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagManagerSettings {
    pub tags: Vec<String>,
    pub layers: Vec<String>,
    pub sorting_layers: Vec<SortingLayerEntry>,
    stream: Option<Stream>,
}

impl Default for TagManagerSettings {
    fn default() -> Self {
        TagManagerSettings {
            tags: Vec::new(),
            layers: vec![String::new(); 32],
            sorting_layers: Vec::new(),
            stream: None,
        }
    }
}

impl TagManagerSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_tag_manager_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_tag_manager(path)
    }

    pub fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.iter().any(|existing| existing == tag) {
            self.tags.push(tag.to_string());
        }
        if let Some(stream) = &mut self.stream {
            if let Ok(entries) = root_mapping_mut(stream, "TagManager") {
                let tag_nodes = self
                    .tags
                    .iter()
                    .map(|value| Node::Scalar(value.clone()))
                    .collect();
                set_mapping_node(entries, "tags", Node::Sequence(tag_nodes));
            }
        }
    }

    pub fn layers(&self) -> Vec<String> {
        self.layers.clone()
    }

    pub fn set_layer(&mut self, index: usize, name: &str) -> Result<(), ProjectSettingsError> {
        if index >= 32 {
            return Err(ProjectSettingsError::InvalidValue("TagManager"));
        }
        if self.layers.len() <= index {
            self.layers.resize(index + 1, String::new());
        }
        self.layers[index] = name.to_string();
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "TagManager")?;
            let layer_nodes = self
                .layers
                .iter()
                .map(|value| Node::Scalar(value.clone()))
                .collect();
            set_mapping_node(entries, "layers", Node::Sequence(layer_nodes));
        }
        Ok(())
    }

    pub fn sorting_layers(&self) -> Vec<SortingLayerEntry> {
        self.sorting_layers.clone()
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        let tag_nodes = self
            .tags
            .iter()
            .map(|value| Node::Scalar(value.clone()))
            .collect();
        let layer_nodes = self
            .layers
            .iter()
            .map(|value| Node::Scalar(value.clone()))
            .collect();
        let sorting_nodes = self
            .sorting_layers
            .iter()
            .map(|entry| {
                Node::Mapping(vec![
                    (String::from("name"), Node::Scalar(entry.name.clone())),
                    (
                        String::from("uniqueID"),
                        Node::Scalar(entry.unique_id.to_string()),
                    ),
                    (
                        String::from("locked"),
                        Node::Scalar(if entry.locked { "1" } else { "0" }.to_string()),
                    ),
                ])
            })
            .collect();
        default_single_doc_stream(
            78,
            "TagManager",
            vec![
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (String::from("tags"), Node::Sequence(tag_nodes)),
                (String::from("layers"), Node::Sequence(layer_nodes)),
                (
                    String::from("m_SortingLayers"),
                    Node::Sequence(sorting_nodes),
                ),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphicsSettings {
    pub custom_render_pipeline_guid: Option<String>,
    pub transparency_sort_mode: i64,
    pub always_included_shader_count: usize,
    pub tier_settings_count: usize,
    stream: Option<Stream>,
}

impl GraphicsSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_graphics_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_graphics_settings(path)
    }

    pub fn custom_render_pipeline_guid(&self) -> Option<String> {
        self.custom_render_pipeline_guid.clone()
    }

    pub fn transparency_sort_mode(&self) -> i64 {
        self.transparency_sort_mode
    }

    pub fn set_transparency_sort_mode(&mut self, mode: i64) -> Result<(), ProjectSettingsError> {
        if mode < 0 {
            return Err(ProjectSettingsError::InvalidValue("GraphicsSettings"));
        }
        self.transparency_sort_mode = mode;
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "GraphicsSettings")?;
            set_mapping_scalar(entries, "m_TransparencySortMode", &mode.to_string());
        }
        Ok(())
    }

    pub fn always_included_shader_count(&self) -> usize {
        self.always_included_shader_count
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        let pipeline = match &self.custom_render_pipeline_guid {
            Some(guid) => Node::Mapping(vec![
                (
                    String::from("fileID"),
                    Node::Scalar(String::from("11400000")),
                ),
                (String::from("guid"), Node::Scalar(guid.clone())),
                (String::from("type"), Node::Scalar(String::from("2"))),
            ]),
            None => Node::Mapping(vec![(
                String::from("fileID"),
                Node::Scalar(String::from("0")),
            )]),
        };
        default_single_doc_stream(
            30,
            "GraphicsSettings",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("15")),
                ),
                (String::from("m_CustomRenderPipeline"), pipeline),
                (
                    String::from("m_TransparencySortMode"),
                    Node::Scalar(self.transparency_sort_mode.to_string()),
                ),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorSpaceSetting {
    #[default]
    Gamma,
    Linear,
}

impl ColorSpaceSetting {
    pub fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(ColorSpaceSetting::Gamma),
            1 => Some(ColorSpaceSetting::Linear),
            _ => None,
        }
    }

    pub fn code(self) -> i64 {
        match self {
            ColorSpaceSetting::Gamma => 0,
            ColorSpaceSetting::Linear => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioManagerSettings {
    pub volume: f64,
    pub rolloff_scale: f64,
    pub doppler_factor: f64,
    pub default_speaker_mode: i64,
    pub dsp_buffer_size: i64,
    pub virtual_voice_count: i64,
    pub real_voice_count: i64,
    pub disable_audio: bool,
    stream: Option<Stream>,
}

impl Default for AudioManagerSettings {
    fn default() -> Self {
        AudioManagerSettings {
            volume: 1.0,
            rolloff_scale: 1.0,
            doppler_factor: 1.0,
            default_speaker_mode: 2,
            dsp_buffer_size: 1024,
            virtual_voice_count: 512,
            real_voice_count: 32,
            disable_audio: false,
            stream: None,
        }
    }
}

impl AudioManagerSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_audio_manager_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_audio_manager(path)
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<(), ProjectSettingsError> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(ProjectSettingsError::InvalidValue("AudioManager"));
        }
        self.volume = volume;
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "AudioManager")?;
            set_mapping_scalar(entries, "m_Volume", &format_float_scalar(volume));
        }
        Ok(())
    }

    pub fn rolloff_scale(&self) -> f64 {
        self.rolloff_scale
    }

    pub fn doppler_factor(&self) -> f64 {
        self.doppler_factor
    }

    pub fn default_speaker_mode(&self) -> i64 {
        self.default_speaker_mode
    }

    pub fn dsp_buffer_size(&self) -> i64 {
        self.dsp_buffer_size
    }

    pub fn virtual_voice_count(&self) -> i64 {
        self.virtual_voice_count
    }

    pub fn real_voice_count(&self) -> i64 {
        self.real_voice_count
    }

    pub fn disable_audio(&self) -> bool {
        self.disable_audio
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        default_single_doc_stream(
            89,
            "AudioManager",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (
                    String::from("m_Volume"),
                    Node::Scalar(format_float_scalar(self.volume)),
                ),
                (
                    String::from("Rolloff Scale"),
                    Node::Scalar(format_float_scalar(self.rolloff_scale)),
                ),
                (
                    String::from("Doppler Factor"),
                    Node::Scalar(format_float_scalar(self.doppler_factor)),
                ),
                (
                    String::from("Default Speaker Mode"),
                    Node::Scalar(self.default_speaker_mode.to_string()),
                ),
                (
                    String::from("m_DSPBufferSize"),
                    Node::Scalar(self.dsp_buffer_size.to_string()),
                ),
                (
                    String::from("m_VirtualVoiceCount"),
                    Node::Scalar(self.virtual_voice_count.to_string()),
                ),
                (
                    String::from("m_RealVoiceCount"),
                    Node::Scalar(self.real_voice_count.to_string()),
                ),
                (
                    String::from("m_DisableAudio"),
                    Node::Scalar(if self.disable_audio { "1" } else { "0" }.to_string()),
                ),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NavMeshAreaEntry {
    pub name: String,
    pub cost: f64,
}

pub type NavMeshArea = NavMeshAreaEntry;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NavMeshAreaSettings {
    pub areas: Vec<NavMeshAreaEntry>,
    stream: Option<Stream>,
}

impl NavMeshAreaSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_nav_mesh_areas_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_nav_mesh_areas(path)
    }

    pub fn areas(&self) -> Vec<NavMeshAreaEntry> {
        self.areas.clone()
    }

    pub fn set_area_cost(&mut self, index: usize, cost: f64) -> Result<(), ProjectSettingsError> {
        if !cost.is_finite() || cost < 0.0 || index >= self.areas.len() {
            return Err(ProjectSettingsError::InvalidValue("NavMeshAreas"));
        }
        self.areas[index].cost = cost;
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "NavMeshAreas")?;
            let seq_key = if entries.iter().any(|(k, _)| k == "areas") {
                "areas"
            } else {
                "m_Settings"
            };
            if let Some((_, Node::Sequence(items))) = entries.iter_mut().find(|(k, _)| k == seq_key)
            {
                if let Some(Node::Mapping(item_entries)) = items.get_mut(index) {
                    set_mapping_scalar(item_entries, "cost", &format_float_scalar(cost));
                }
            }
        }
        Ok(())
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        let area_nodes = self
            .areas
            .iter()
            .map(|area| {
                Node::Mapping(vec![
                    (String::from("name"), Node::Scalar(area.name.clone())),
                    (
                        String::from("cost"),
                        Node::Scalar(format_float_scalar(area.cost)),
                    ),
                ])
            })
            .collect();
        default_single_doc_stream(
            126,
            "NavMeshAreas",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (String::from("areas"), Node::Sequence(area_nodes)),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSettings {
    pub serialization_mode: i64,
    pub default_behavior_mode: i64,
    pub line_endings_for_new_scripts: i64,
    stream: Option<Stream>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            serialization_mode: 2,
            default_behavior_mode: 0,
            line_endings_for_new_scripts: 0,
            stream: None,
        }
    }
}

impl EditorSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_editor_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_editor_settings(path)
    }

    pub fn serialization_mode(&self) -> i64 {
        self.serialization_mode
    }

    pub fn set_serialization_mode(&mut self, mode: i64) -> Result<(), ProjectSettingsError> {
        if !(0..=2).contains(&mode) {
            return Err(ProjectSettingsError::InvalidValue("EditorSettings"));
        }
        self.serialization_mode = mode;
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "EditorSettings")?;
            set_mapping_scalar(entries, "m_SerializationMode", &mode.to_string());
        }
        Ok(())
    }

    pub fn default_behavior_mode(&self) -> i64 {
        self.default_behavior_mode
    }

    pub fn line_endings_for_new_scripts(&self) -> i64 {
        self.line_endings_for_new_scripts
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        default_single_doc_stream(
            159,
            "EditorSettings",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("9")),
                ),
                (
                    String::from("m_SerializationMode"),
                    Node::Scalar(self.serialization_mode.to_string()),
                ),
                (
                    String::from("m_DefaultBehaviorMode"),
                    Node::Scalar(self.default_behavior_mode.to_string()),
                ),
                (
                    String::from("m_LineEndingsForNewScripts"),
                    Node::Scalar(self.line_endings_for_new_scripts.to_string()),
                ),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonoManagerSettings {
    pub execution_orders: Vec<(String, i64)>,
    stream: Option<Stream>,
}

impl MonoManagerSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_mono_manager_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_mono_manager(path)
    }

    pub fn execution_orders(&self) -> Vec<(String, i64)> {
        self.execution_orders.clone()
    }

    pub fn set_execution_order(&mut self, script_guid: &str, order: i64) {
        if let Some(existing) = self
            .execution_orders
            .iter_mut()
            .find(|(key, _)| key == script_guid)
        {
            existing.1 = order;
        } else {
            self.execution_orders.push((script_guid.to_string(), order));
        }

        if let Some(stream) = &mut self.stream {
            if let Ok(entries) = root_mapping_mut(stream, "MonoManager") {
                if let Some((_, Node::Mapping(map_entries))) =
                    entries.iter_mut().find(|(k, _)| k == "m_ExecutionOrder")
                {
                    set_mapping_scalar(map_entries, script_guid, &order.to_string());
                    return;
                }
                let seq_key = if entries.iter().any(|(k, _)| k == "m_ExecutionOrder") {
                    "m_ExecutionOrder"
                } else if entries.iter().any(|(k, _)| k == "m_ScriptExecutionOrders") {
                    "m_ScriptExecutionOrders"
                } else {
                    set_mapping_node(
                        entries,
                        "m_ExecutionOrder",
                        Node::Mapping(
                            self.execution_orders
                                .iter()
                                .map(|(k, v)| (k.clone(), Node::Scalar(v.to_string())))
                                .collect(),
                        ),
                    );
                    return;
                };
                if let Some((_, Node::Sequence(items))) =
                    entries.iter_mut().find(|(k, _)| k == seq_key)
                {
                    for item in items.iter_mut() {
                        if let Some((key, _)) = parse_script_execution_order(item) {
                            if key == script_guid {
                                if let Node::Mapping(item_entries) = item {
                                    if item_entries.iter().any(|(k, _)| k == "order") {
                                        set_mapping_scalar(
                                            item_entries,
                                            "order",
                                            &order.to_string(),
                                        );
                                    } else {
                                        set_mapping_scalar(
                                            item_entries,
                                            "second",
                                            &order.to_string(),
                                        );
                                    }
                                }
                                return;
                            }
                        }
                    }
                    items.push(Node::Mapping(vec![
                        (String::from("guid"), Node::Scalar(script_guid.to_string())),
                        (String::from("order"), Node::Scalar(order.to_string())),
                    ]));
                }
            }
        }
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        let map_entries = self
            .execution_orders
            .iter()
            .map(|(k, v)| (k.clone(), Node::Scalar(v.to_string())))
            .collect();
        default_single_doc_stream(
            116,
            "MonoManager",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (String::from("m_ExecutionOrder"), Node::Mapping(map_entries)),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresetManagerSettings {
    pub default_preset_type_count: usize,
    stream: Option<Stream>,
}

impl PresetManagerSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_preset_manager_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_preset_manager(path)
    }

    pub fn default_preset_type_count(&self) -> usize {
        self.default_preset_type_count
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        default_single_doc_stream(
            1386491679,
            "PresetManager",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (String::from("m_DefaultPresets"), Node::Sequence(Vec::new())),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageManagerSettings {
    pub enable_pre_release_packages: bool,
    pub scoped_registries_count: usize,
    stream: Option<Stream>,
}

impl PackageManagerSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_package_manager_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_package_manager_settings(path)
    }

    pub fn enable_pre_release_packages(&self) -> bool {
        self.enable_pre_release_packages
    }

    pub fn set_enable_pre_release_packages(
        &mut self,
        enabled: bool,
    ) -> Result<(), ProjectSettingsError> {
        self.enable_pre_release_packages = enabled;
        if let Some(stream) = &mut self.stream {
            let entries = package_manager_mapping_mut(stream)?;
            let key = if entries.iter().any(|(k, _)| k == "enablePreReleasePackages") {
                "enablePreReleasePackages"
            } else {
                "m_EnablePreReleasePackages"
            };
            set_mapping_scalar(entries, key, if enabled { "1" } else { "0" });
        }
        Ok(())
    }

    pub fn scoped_registries_count(&self) -> usize {
        self.scoped_registries_count
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        default_single_doc_stream(
            114,
            "PackageManagerSettings",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("m_EnablePreReleasePackages"),
                    Node::Scalar(
                        if self.enable_pre_release_packages {
                            "1"
                        } else {
                            "0"
                        }
                        .to_string(),
                    ),
                ),
                (
                    String::from("m_ScopedRegistries"),
                    Node::Sequence(Vec::new()),
                ),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionControlSettings {
    pub mode: String,
    stream: Option<Stream>,
}

impl Default for VersionControlSettings {
    fn default() -> Self {
        VersionControlSettings {
            mode: String::from("Visible Meta Files"),
            stream: None,
        }
    }
}

impl VersionControlSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_version_control_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_version_control_settings(path)
    }

    pub fn mode(&self) -> Result<&str, ProjectSettingsError> {
        if self.mode.is_empty() {
            return Err(ProjectSettingsError::MissingDocument(
                "VersionControlSettings",
            ));
        }
        Ok(self.mode.as_str())
    }

    pub fn set_mode(&mut self, mode: &str) -> Result<(), ProjectSettingsError> {
        let trimmed = mode.trim();
        if trimmed.is_empty() {
            return Err(ProjectSettingsError::InvalidValue("VersionControlSettings"));
        }
        self.mode = trimmed.to_string();
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "VersionControlSettings")?;
            let key = if entries.iter().any(|(k, _)| k == "mode") {
                "mode"
            } else {
                "m_Mode"
            };
            set_mapping_scalar(entries, key, trimmed);
        }
        Ok(())
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        default_single_doc_stream(
            890905787,
            "VersionControlSettings",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (String::from("m_Mode"), Node::Scalar(self.mode.clone())),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildSceneEntry {
    pub enabled: bool,
    pub path: String,
    pub guid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorBuildSettings {
    pub scenes: Vec<BuildSceneEntry>,
    stream: Option<Stream>,
}

impl EditorBuildSettings {
    pub fn from_stream(stream: &Stream) -> Result<Self, ProjectSettingsError> {
        parse_editor_build_settings_stream(stream)
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectSettingsError> {
        parse_editor_build_settings(path)
    }

    pub fn scenes(&self) -> Vec<BuildSceneEntry> {
        self.scenes.clone()
    }

    pub fn set_scene_enabled(
        &mut self,
        path: &str,
        enabled: bool,
    ) -> Result<(), ProjectSettingsError> {
        let scene = self
            .scenes
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or(ProjectSettingsError::InvalidValue("EditorBuildSettings"))?;
        scene.enabled = enabled;
        if let Some(stream) = &mut self.stream {
            let entries = root_mapping_mut(stream, "EditorBuildSettings")?;
            if let Some((_, Node::Sequence(items))) =
                entries.iter_mut().find(|(k, _)| k == "m_Scenes")
            {
                for item in items.iter_mut() {
                    let matches_path = item
                        .get("path")
                        .and_then(Node::as_str)
                        .is_some_and(|candidate| candidate == path);
                    if matches_path {
                        if let Node::Mapping(item_entries) = item {
                            set_mapping_scalar(
                                item_entries,
                                "enabled",
                                if enabled { "1" } else { "0" },
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn to_stream(&self) -> Stream {
        if let Some(stream) = &self.stream {
            return stream.clone();
        }
        let scene_nodes = self
            .scenes
            .iter()
            .map(|scene| {
                Node::Mapping(vec![
                    (
                        String::from("enabled"),
                        Node::Scalar(if scene.enabled { "1" } else { "0" }.to_string()),
                    ),
                    (String::from("path"), Node::Scalar(scene.path.clone())),
                    (String::from("guid"), Node::Scalar(scene.guid.clone())),
                ])
            })
            .collect();
        default_single_doc_stream(
            1045,
            "EditorBuildSettings",
            vec![
                (
                    String::from("m_ObjectHideFlags"),
                    Node::Scalar(String::from("0")),
                ),
                (
                    String::from("serializedVersion"),
                    Node::Scalar(String::from("2")),
                ),
                (String::from("m_Scenes"), Node::Sequence(scene_nodes)),
            ],
        )
    }

    pub fn emit(&self) -> String {
        scene_save::emit_stream(&self.to_stream())
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectSettingsError> {
        let stream = self.to_stream();
        scene_save::save_stream(path, &stream).map_err(|error| ProjectSettingsError::Yaml {
            error: YamlError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSettingsError {
    MissingDocument(&'static str),
    InvalidValue(&'static str),
    Yaml {
        error: YamlError,
    },
    MalformedFile {
        path: PathBuf,
        error: Box<ProjectSettingsError>,
    },
}

impl fmt::Display for ProjectSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectSettingsError::MissingDocument(document) => {
                write!(formatter, "stream contains no {document} document")
            }
            ProjectSettingsError::InvalidValue(document) => {
                write!(formatter, "invalid setting value in {document}")
            }
            ProjectSettingsError::Yaml { error } => write!(formatter, "{error}"),
            ProjectSettingsError::MalformedFile { path, error } => {
                write!(formatter, "{}: {}", path.display(), error)
            }
        }
    }
}

impl std::error::Error for ProjectSettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectSettingsError::Yaml { error } => Some(error),
            ProjectSettingsError::MissingDocument(_) | ProjectSettingsError::InvalidValue(_) => {
                None
            }
            ProjectSettingsError::MalformedFile { error, .. } => Some(error.as_ref()),
        }
    }
}

impl From<YamlError> for ProjectSettingsError {
    fn from(error: YamlError) -> Self {
        ProjectSettingsError::Yaml { error }
    }
}

fn default_single_doc_stream(
    class_id: u32,
    root_key: &str,
    entries: Vec<(String, Node)>,
) -> Stream {
    Stream {
        header: Some(Header {
            yaml_version: String::from("1.1"),
            tag_handle: Some(String::from("!u!")),
            tag_prefix: Some(String::from("tag:unity3d.com,2011:")),
        }),
        documents: vec![Document {
            class_id,
            file_id: 1,
            stripped: false,
            root: Node::Mapping(vec![(root_key.to_string(), Node::Mapping(entries))]),
        }],
    }
}

fn root_mapping_mut<'a>(
    stream: &'a mut Stream,
    root_key: &'static str,
) -> Result<&'a mut Vec<(String, Node)>, ProjectSettingsError> {
    let doc = stream
        .documents
        .iter_mut()
        .filter(|document| !document.stripped)
        .find(|document| document.root.get(root_key).is_some())
        .ok_or(ProjectSettingsError::MissingDocument(root_key))?;
    if let Node::Mapping(root_entries) = &mut doc.root {
        if let Some((_, Node::Mapping(entries))) =
            root_entries.iter_mut().find(|(k, _)| k == root_key)
        {
            return Ok(entries);
        }
    }
    Err(ProjectSettingsError::MissingDocument(root_key))
}

fn package_manager_mapping_mut(
    stream: &mut Stream,
) -> Result<&mut Vec<(String, Node)>, ProjectSettingsError> {
    let doc = stream
        .documents
        .iter_mut()
        .filter(|document| !document.stripped)
        .find(|document| {
            document.root.get("PackageManagerSettings").is_some()
                || document.root.get("m_EnablePreReleasePackages").is_some()
                || document.root.get("enablePreReleasePackages").is_some()
                || document.root.get("m_ScopedRegistries").is_some()
                || document.root.get("scopedRegistries").is_some()
        })
        .ok_or(ProjectSettingsError::MissingDocument(
            "PackageManagerSettings",
        ))?;
    if let Node::Mapping(root_entries) = &mut doc.root {
        let pos = root_entries
            .iter()
            .position(|(k, v)| k == "PackageManagerSettings" && matches!(v, Node::Mapping(_)));
        return match pos {
            Some(index) => match &mut root_entries[index].1 {
                Node::Mapping(entries) => Ok(entries),
                _ => Err(ProjectSettingsError::MissingDocument(
                    "PackageManagerSettings",
                )),
            },
            None => Ok(root_entries),
        };
    }
    Err(ProjectSettingsError::MissingDocument(
        "PackageManagerSettings",
    ))
}

fn set_mapping_scalar(entries: &mut Vec<(String, Node)>, key: &str, value: &str) {
    set_mapping_node(entries, key, Node::Scalar(value.to_string()));
}

fn set_mapping_node(entries: &mut Vec<(String, Node)>, key: &str, value: Node) {
    if let Some((_, existing)) = entries.iter_mut().find(|(entry_key, _)| entry_key == key) {
        *existing = value;
    } else {
        entries.push((key.to_string(), value));
    }
}

fn format_float_scalar(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
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

pub fn parse_physics_manager(
    path: &Path,
) -> Result<Option<PhysicsProjectSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_physics_manager_stream(&stream).map(Some)
}

pub fn parse_physics_manager_stream(
    stream: &Stream,
) -> Result<PhysicsProjectSettings, ProjectSettingsError> {
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

    let gravity = match body.get("m_Gravity") {
        Some(node) => [
            parse_finite_f64(node, "x")
                .ok_or(ProjectSettingsError::InvalidValue("PhysicsManager"))?,
            parse_finite_f64(node, "y")
                .ok_or(ProjectSettingsError::InvalidValue("PhysicsManager"))?,
            parse_finite_f64(node, "z")
                .ok_or(ProjectSettingsError::InvalidValue("PhysicsManager"))?,
        ],
        None => DEFAULT_GRAVITY,
    };

    let layer_collision_matrix = match body.get("m_LayerCollisionMatrix") {
        Some(node) => parse_layer_collision_matrix_node(node, "PhysicsManager")?,
        None => [0xFFFF_FFFFu32; 32],
    };

    Ok(PhysicsProjectSettings {
        gravity,
        default_solver_iterations: parse_optional_u32(
            body,
            "m_DefaultSolverIterations",
            DEFAULT_SOLVER_ITERATIONS,
            "PhysicsManager",
        )?,
        default_solver_velocity_iterations: parse_optional_u32(
            body,
            "m_DefaultSolverVelocityIterations",
            DEFAULT_SOLVER_VELOCITY_ITERATIONS,
            "PhysicsManager",
        )?,
        bounce_threshold: parse_optional_f64(
            body,
            "m_BounceThreshold",
            DEFAULT_BOUNCE_THRESHOLD,
            "PhysicsManager",
        )?,
        default_contact_offset: parse_optional_f64(
            body,
            "m_DefaultContactOffset",
            DEFAULT_CONTACT_OFFSET,
            "PhysicsManager",
        )?,
        sleep_threshold: parse_optional_f64_alias(
            body,
            "m_DefaultSleepThreshold",
            "m_SleepThreshold",
            DEFAULT_SLEEP_THRESHOLD,
            "PhysicsManager",
        )?,
        sleep_angular_threshold: parse_optional_f64_alias(
            body,
            "m_SleepAngularThreshold",
            "m_DefaultSleepAngularThreshold",
            DEFAULT_SLEEP_ANGULAR_THRESHOLD,
            "PhysicsManager",
        )?,
        solver_type: physics_solver_type(body.get("m_SolverType")),
        enhanced_determinism: parse_optional_bool(
            body,
            "m_EnableEnhancedDeterminism",
            false,
            "PhysicsManager",
        )?,
        layer_collision_matrix,
    })
}

fn physics_solver_type(node: Option<&Node>) -> PhysicsSolverType {
    match node.and_then(Node::as_i64) {
        Some(1) => PhysicsSolverType::Tgs,
        Some(2) => PhysicsSolverType::TgsSoft,
        _ => PhysicsSolverType::Default,
    }
}

pub fn parse_physics2d_settings(
    path: &Path,
) -> Result<Option<Physics2DProjectSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_physics2d_settings_stream(&stream).map(Some)
}

pub fn parse_physics2d_settings_stream(
    stream: &Stream,
) -> Result<Physics2DProjectSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| {
            document
                .root
                .get("Physics2DSettings")
                .or_else(|| document.root.get("Physics2DManager"))
        })
        .ok_or(ProjectSettingsError::MissingDocument("Physics2DSettings"))?;

    let gravity = match body.get("m_Gravity") {
        Some(node) => [
            parse_finite_f64(node, "x")
                .ok_or(ProjectSettingsError::InvalidValue("Physics2DSettings"))?,
            parse_finite_f64(node, "y")
                .ok_or(ProjectSettingsError::InvalidValue("Physics2DSettings"))?,
        ],
        None => DEFAULT_GRAVITY_2D,
    };

    let layer_collision_matrix = match body.get("m_LayerCollisionMatrix") {
        Some(node) => parse_layer_collision_matrix_node(node, "Physics2DSettings")?,
        None => [0xFFFF_FFFFu32; 32],
    };

    Ok(Physics2DProjectSettings {
        gravity,
        velocity_iterations: parse_optional_u32(
            body,
            "m_VelocityIterations",
            DEFAULT_PHYSICS2D_VELOCITY_ITERATIONS,
            "Physics2DSettings",
        )?,
        position_iterations: parse_optional_u32(
            body,
            "m_PositionIterations",
            DEFAULT_PHYSICS2D_POSITION_ITERATIONS,
            "Physics2DSettings",
        )?,
        queries_hit_triggers: parse_optional_bool(
            body,
            "m_QueriesHitTriggers",
            true,
            "Physics2DSettings",
        )?,
        queries_start_in_colliders: parse_optional_bool(
            body,
            "m_QueriesStartInColliders",
            true,
            "Physics2DSettings",
        )?,
        callbacks_on_disable: parse_optional_bool(
            body,
            "m_CallbacksOnDisable",
            true,
            "Physics2DSettings",
        )?,
        auto_sync_transforms: parse_optional_bool(
            body,
            "m_AutoSyncTransforms",
            true,
            "Physics2DSettings",
        )?,
        layer_collision_matrix,
    })
}

fn parse_layer_collision_matrix_node(
    node: &Node,
    document: &'static str,
) -> Result<[u32; 32], ProjectSettingsError> {
    if let Some(hex) = node.as_str() {
        return Ok(parse_layer_collision_matrix(hex));
    }
    if let Some(entries) = node.as_sequence() {
        let mut matrix = [0xFFFF_FFFFu32; 32];
        for (layer, entry) in entries.iter().take(32).enumerate() {
            if let Some(val) = entry.as_i64() {
                if let Ok(u) = u32::try_from(val) {
                    matrix[layer] = u;
                } else if let Ok(s) = i32::try_from(val) {
                    matrix[layer] = s as u32;
                } else {
                    return Err(ProjectSettingsError::InvalidValue(document));
                }
            } else if let Some(text) = entry.as_str() {
                let trimmed = text.trim();
                let parsed = if let Some(hex_part) = trimmed
                    .strip_prefix("0x")
                    .or_else(|| trimmed.strip_prefix("0X"))
                {
                    u32::from_str_radix(hex_part, 16).ok()
                } else {
                    trimmed.parse::<u32>().ok()
                };
                if let Some(mask) = parsed {
                    matrix[layer] = mask;
                } else {
                    return Err(ProjectSettingsError::InvalidValue(document));
                }
            } else {
                return Err(ProjectSettingsError::InvalidValue(document));
            }
        }
        return Ok(matrix);
    }
    Err(ProjectSettingsError::InvalidValue(document))
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
        let axis_type = parse_optional_u32_alias(entry, "type", "m_Type", 0, "InputManager")?;
        if axis_type >= 2 {
            skipped_axes += 1;
            continue;
        }
        let axis = parse_optional_u32_alias(entry, "axis", "m_Axis", 0, "InputManager")?;
        let gravity = parse_optional_f64(entry, "gravity", 0.0, "InputManager")?;
        let dead = parse_optional_f64(entry, "dead", 0.0, "InputManager")?;
        let sensitivity = parse_optional_f64(entry, "sensitivity", 1.0, "InputManager")?;
        let snap = parse_optional_bool(entry, "snap", false, "InputManager")?;
        let invert = parse_optional_bool(entry, "invert", false, "InputManager")?;

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

    let active_color_space = parse_color_space_stream(stream)?;

    Ok(ProjectIdentity {
        company_name: parse_optional_non_empty_string(body, "companyName"),
        product_name: parse_optional_non_empty_string(body, "productName"),
        bundle_version: parse_optional_non_empty_string(body, "bundleVersion"),
        active_color_space,
    })
}

pub fn parse_quality_settings(
    path: &Path,
) -> Result<Option<QualitySettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_quality_settings_stream(&stream).map(Some)
}

pub fn parse_quality_settings_stream(
    stream: &Stream,
) -> Result<QualitySettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("QualitySettings"))
        .ok_or(ProjectSettingsError::MissingDocument("QualitySettings"))?;

    let entries = body
        .get("m_QualitySettings")
        .and_then(Node::as_sequence)
        .ok_or(ProjectSettingsError::MissingDocument("QualitySettings"))?;

    let mut levels = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry
            .get("name")
            .and_then(Node::as_str)
            .ok_or(ProjectSettingsError::MissingDocument("QualitySettings"))?
            .to_string();
        levels.push(QualityLevel {
            name,
            pixel_light_count: parse_optional_u32(entry, "pixelLightCount", 1, "QualitySettings")?,
            shadows: parse_optional_u32(entry, "shadows", 1, "QualitySettings")?,
            shadow_resolution: parse_optional_u32(entry, "shadowResolution", 2, "QualitySettings")?,
            shadow_distance: parse_optional_f64(entry, "shadowDistance", 50.0, "QualitySettings")?,
            texture_quality: parse_optional_u32(entry, "textureQuality", 0, "QualitySettings")?,
            anisotropic_textures: parse_optional_u32(
                entry,
                "anisotropicTextures",
                1,
                "QualitySettings",
            )?,
            anti_aliasing: parse_optional_u32(entry, "antiAliasing", 0, "QualitySettings")?,
            v_sync_count: parse_optional_u32(entry, "vSyncCount", 0, "QualitySettings")?,
            lod_bias: parse_optional_f64(entry, "lodBias", 1.0, "QualitySettings")?,
        });
    }

    let current_quality = quality_level_index(body.get("m_CurrentQuality"), levels.len())?;

    let mut per_platform_default_quality = Vec::new();
    if let Some(Node::Mapping(platforms)) = body.get("m_PerPlatformDefaultQuality") {
        for (platform, value) in platforms {
            per_platform_default_quality.push((
                platform.clone(),
                quality_level_index(Some(value), levels.len())?,
            ));
        }
    }

    Ok(QualitySettings {
        current_quality,
        levels,
        per_platform_default_quality,
    })
}

pub fn parse_tag_manager(path: &Path) -> Result<Option<TagManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_tag_manager_stream(&stream).map(Some)
}

pub fn parse_tag_manager_stream(
    stream: &Stream,
) -> Result<TagManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("TagManager"))
        .ok_or(ProjectSettingsError::MissingDocument("TagManager"))?;

    let mut sorting_layers = Vec::new();
    if let Some(child) = body.get("m_SortingLayers") {
        let entries = child
            .as_sequence()
            .ok_or(ProjectSettingsError::MissingDocument("TagManager"))?;
        for entry in entries {
            sorting_layers.push(SortingLayerEntry {
                name: parse_optional_string(entry, "name"),
                unique_id: parse_optional_i64(entry, "uniqueID", 0, "TagManager")?,
                locked: parse_optional_bool(entry, "locked", false, "TagManager")?,
            });
        }
    }

    Ok(TagManagerSettings {
        tags: parse_string_sequence(body, "tags", "TagManager")?,
        layers: parse_string_sequence(body, "layers", "TagManager")?,
        sorting_layers,
        stream: Some(stream.clone()),
    })
}

pub fn parse_graphics_settings(
    path: &Path,
) -> Result<Option<GraphicsSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_graphics_settings_stream(&stream).map(Some)
}

pub fn parse_graphics_settings_stream(
    stream: &Stream,
) -> Result<GraphicsSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("GraphicsSettings"))
        .ok_or(ProjectSettingsError::MissingDocument("GraphicsSettings"))?;

    let custom_render_pipeline_guid = body
        .get("m_CustomRenderPipeline")
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|guid| !guid.is_empty())
        .map(str::to_string);

    let always_included_shader_count = body
        .get("m_AlwaysIncludedShaders")
        .and_then(Node::as_sequence)
        .map_or(0, |entries| entries.len());

    let tier_settings_count = body
        .get("m_TierSettings")
        .and_then(Node::as_sequence)
        .map_or(0, |entries| entries.len());

    Ok(GraphicsSettings {
        custom_render_pipeline_guid,
        transparency_sort_mode: parse_optional_i64(
            body,
            "m_TransparencySortMode",
            0,
            "GraphicsSettings",
        )?,
        always_included_shader_count,
        tier_settings_count,
        stream: Some(stream.clone()),
    })
}

pub fn parse_color_space(path: &Path) -> Result<Option<ColorSpaceSetting>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_color_space_stream(&stream).map(Some)
}

pub fn parse_color_space_stream(
    stream: &Stream,
) -> Result<ColorSpaceSetting, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("PlayerSettings"))
        .ok_or(ProjectSettingsError::MissingDocument("PlayerSettings"))?;

    let code = parse_optional_i64(body, "m_ActiveColorSpace", 0, "PlayerSettings")?;
    ColorSpaceSetting::from_code(code)
        .ok_or(ProjectSettingsError::MissingDocument("PlayerSettings"))
}

pub fn parse_audio_manager(
    path: &Path,
) -> Result<Option<AudioManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_audio_manager_stream(&stream).map(Some)
}

pub fn parse_audio_manager_stream(
    stream: &Stream,
) -> Result<AudioManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("AudioManager"))
        .ok_or(ProjectSettingsError::MissingDocument("AudioManager"))?;

    Ok(AudioManagerSettings {
        volume: parse_optional_f64(body, "m_Volume", 1.0, "AudioManager")?,
        rolloff_scale: parse_optional_f64(body, "Rolloff Scale", 1.0, "AudioManager")?,
        doppler_factor: parse_optional_f64(body, "Doppler Factor", 1.0, "AudioManager")?,
        default_speaker_mode: parse_optional_i64(body, "Default Speaker Mode", 2, "AudioManager")?,
        dsp_buffer_size: parse_optional_i64(body, "m_DSPBufferSize", 1024, "AudioManager")?,
        virtual_voice_count: parse_optional_i64(body, "m_VirtualVoiceCount", 512, "AudioManager")?,
        real_voice_count: parse_optional_i64(body, "m_RealVoiceCount", 32, "AudioManager")?,
        disable_audio: parse_optional_bool(body, "m_DisableAudio", false, "AudioManager")?,
        stream: Some(stream.clone()),
    })
}

pub fn parse_nav_mesh_areas(
    path: &Path,
) -> Result<Option<NavMeshAreaSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_nav_mesh_areas_stream(&stream).map(Some)
}

pub fn parse_nav_mesh_areas_stream(
    stream: &Stream,
) -> Result<NavMeshAreaSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("NavMeshAreas"))
        .ok_or(ProjectSettingsError::MissingDocument("NavMeshAreas"))?;

    let mut areas = Vec::new();
    if let Some(child) = body.get("areas").or_else(|| body.get("m_Settings")) {
        let entries = child
            .as_sequence()
            .ok_or(ProjectSettingsError::MissingDocument("NavMeshAreas"))?;
        for entry in entries {
            areas.push(NavMeshAreaEntry {
                name: parse_optional_string(entry, "name"),
                cost: parse_optional_f64(entry, "cost", 1.0, "NavMeshAreas")?,
            });
        }
    }

    Ok(NavMeshAreaSettings {
        areas,
        stream: Some(stream.clone()),
    })
}

pub fn parse_editor_settings(path: &Path) -> Result<Option<EditorSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_editor_settings_stream(&stream).map(Some)
}

pub fn parse_editor_settings_stream(
    stream: &Stream,
) -> Result<EditorSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("EditorSettings"))
        .ok_or(ProjectSettingsError::MissingDocument("EditorSettings"))?;

    Ok(EditorSettings {
        serialization_mode: parse_optional_i64(body, "m_SerializationMode", 2, "EditorSettings")?,
        default_behavior_mode: parse_optional_i64(
            body,
            "m_DefaultBehaviorMode",
            0,
            "EditorSettings",
        )?,
        line_endings_for_new_scripts: parse_optional_i64(
            body,
            "m_LineEndingsForNewScripts",
            0,
            "EditorSettings",
        )?,
        stream: Some(stream.clone()),
    })
}

pub fn parse_mono_manager(
    path: &Path,
) -> Result<Option<MonoManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_mono_manager_stream(&stream).map(Some)
}

pub fn parse_mono_manager_stream(
    stream: &Stream,
) -> Result<MonoManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("MonoManager"))
        .ok_or(ProjectSettingsError::MissingDocument("MonoManager"))?;

    let mut execution_orders = Vec::new();
    if let Some(child) = body
        .get("m_ExecutionOrder")
        .or_else(|| body.get("m_ScriptExecutionOrders"))
    {
        if let Some(entries) = child.as_mapping() {
            for (key, val) in entries {
                let order = val
                    .as_i64()
                    .ok_or(ProjectSettingsError::MissingDocument("MonoManager"))?;
                execution_orders.push((key.clone(), order));
            }
        } else if let Some(entries) = child.as_sequence() {
            for entry in entries {
                if let Some(order) = parse_script_execution_order(entry) {
                    execution_orders.push(order);
                }
            }
        } else if child.as_str().is_none_or(|s| !s.is_empty()) {
            return Err(ProjectSettingsError::MissingDocument("MonoManager"));
        }
    }

    Ok(MonoManagerSettings {
        execution_orders,
        stream: Some(stream.clone()),
    })
}

pub fn parse_preset_manager(
    path: &Path,
) -> Result<Option<PresetManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_preset_manager_stream(&stream).map(Some)
}

pub fn parse_preset_manager_stream(
    stream: &Stream,
) -> Result<PresetManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("PresetManager"))
        .ok_or(ProjectSettingsError::MissingDocument("PresetManager"))?;

    let default_preset_type_count = match body.get("m_DefaultPresets") {
        Some(Node::Sequence(entries)) => entries.len(),
        Some(Node::Mapping(entries)) => entries.len(),
        _ => 0,
    };

    Ok(PresetManagerSettings {
        default_preset_type_count,
        stream: Some(stream.clone()),
    })
}

pub fn parse_package_manager_settings(
    path: &Path,
) -> Result<Option<PackageManagerSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_package_manager_settings_stream(&stream).map(Some)
}

pub fn parse_package_manager_settings_stream(
    stream: &Stream,
) -> Result<PackageManagerSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| {
            if let Some(body) = document.root.get("PackageManagerSettings") {
                return Some(body);
            }
            let root = &document.root;
            if root.get("m_EnablePreReleasePackages").is_some()
                || root.get("enablePreReleasePackages").is_some()
                || root.get("m_ScopedRegistries").is_some()
                || root.get("scopedRegistries").is_some()
            {
                Some(root)
            } else {
                None
            }
        })
        .ok_or(ProjectSettingsError::MissingDocument(
            "PackageManagerSettings",
        ))?;

    Ok(PackageManagerSettings {
        enable_pre_release_packages: parse_optional_bool_alias(
            body,
            "m_EnablePreReleasePackages",
            "enablePreReleasePackages",
            false,
            "PackageManagerSettings",
        )?,
        scoped_registries_count: body
            .get("m_ScopedRegistries")
            .or_else(|| body.get("scopedRegistries"))
            .and_then(Node::as_sequence)
            .map_or(0, |entries| entries.len()),
        stream: Some(stream.clone()),
    })
}

pub fn parse_version_control_settings(
    path: &Path,
) -> Result<Option<VersionControlSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_version_control_settings_stream(&stream).map(Some)
}

pub fn parse_version_control_settings_stream(
    stream: &Stream,
) -> Result<VersionControlSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("VersionControlSettings"))
        .ok_or(ProjectSettingsError::MissingDocument(
            "VersionControlSettings",
        ))?;

    let mode = body
        .get("m_Mode")
        .or_else(|| body.get("mode"))
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map_or_else(|| VersionControlSettings::default().mode, str::to_string);

    Ok(VersionControlSettings {
        mode,
        stream: Some(stream.clone()),
    })
}

pub fn parse_editor_build_settings(
    path: &Path,
) -> Result<Option<EditorBuildSettings>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path)?;
    parse_editor_build_settings_stream(&stream).map(Some)
}

pub fn parse_editor_build_settings_stream(
    stream: &Stream,
) -> Result<EditorBuildSettings, ProjectSettingsError> {
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| document.root.get("EditorBuildSettings"))
        .ok_or(ProjectSettingsError::MissingDocument("EditorBuildSettings"))?;

    let mut scenes = Vec::new();
    if let Some(child) = body.get("m_Scenes") {
        let entries = child
            .as_sequence()
            .ok_or(ProjectSettingsError::MissingDocument("EditorBuildSettings"))?;
        for entry in entries {
            scenes.push(BuildSceneEntry {
                enabled: parse_optional_bool(entry, "enabled", false, "EditorBuildSettings")?,
                path: parse_optional_string(entry, "path"),
                guid: parse_optional_string(entry, "guid"),
            });
        }
    }

    Ok(EditorBuildSettings {
        scenes,
        stream: Some(stream.clone()),
    })
}

fn quality_level_index(
    node: Option<&Node>,
    level_count: usize,
) -> Result<usize, ProjectSettingsError> {
    node.and_then(Node::as_i64)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|index| *index < level_count)
        .ok_or(ProjectSettingsError::MissingDocument("QualitySettings"))
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

fn parse_optional_f64(
    node: &Node,
    key: &str,
    default: f64,
    document: &'static str,
) -> Result<f64, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => child
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(ProjectSettingsError::MissingDocument(document)),
        None => Ok(default),
    }
}

fn parse_optional_i64(
    node: &Node,
    key: &str,
    default: i64,
    document: &'static str,
) -> Result<i64, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => child
            .as_i64()
            .ok_or(ProjectSettingsError::MissingDocument(document)),
        None => Ok(default),
    }
}

fn parse_optional_u32(
    node: &Node,
    key: &str,
    default: u32,
    document: &'static str,
) -> Result<u32, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => child
            .as_i64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ProjectSettingsError::MissingDocument(document)),
        None => Ok(default),
    }
}

fn parse_optional_u32_alias(
    node: &Node,
    primary_key: &str,
    fallback_key: &str,
    default: u32,
    document: &'static str,
) -> Result<u32, ProjectSettingsError> {
    match node.get(primary_key).or_else(|| node.get(fallback_key)) {
        Some(child) => child
            .as_i64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ProjectSettingsError::MissingDocument(document)),
        None => Ok(default),
    }
}

fn parse_optional_f64_alias(
    node: &Node,
    primary_key: &str,
    fallback_key: &str,
    default: f64,
    document: &'static str,
) -> Result<f64, ProjectSettingsError> {
    match node.get(primary_key).or_else(|| node.get(fallback_key)) {
        Some(child) => child
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(ProjectSettingsError::MissingDocument(document)),
        None => Ok(default),
    }
}

fn parse_optional_bool(
    node: &Node,
    key: &str,
    default: bool,
    document: &'static str,
) -> Result<bool, ProjectSettingsError> {
    match node.get(key) {
        Some(child) => parse_bool_node(child, document),
        None => Ok(default),
    }
}

fn parse_optional_bool_alias(
    node: &Node,
    primary_key: &str,
    fallback_key: &str,
    default: bool,
    document: &'static str,
) -> Result<bool, ProjectSettingsError> {
    match node.get(primary_key).or_else(|| node.get(fallback_key)) {
        Some(child) => parse_bool_node(child, document),
        None => Ok(default),
    }
}

fn parse_bool_node(node: &Node, document: &'static str) -> Result<bool, ProjectSettingsError> {
    if let Some(value) = node.as_i64() {
        Ok(value != 0)
    } else if let Some(text) = node.as_str() {
        match text.trim() {
            "true" | "True" | "TRUE" => Ok(true),
            "false" | "False" | "FALSE" => Ok(false),
            _ => Err(ProjectSettingsError::MissingDocument(document)),
        }
    } else {
        Err(ProjectSettingsError::MissingDocument(document))
    }
}

fn parse_string_sequence(
    node: &Node,
    key: &str,
    document: &'static str,
) -> Result<Vec<String>, ProjectSettingsError> {
    match node.get(key) {
        None => Ok(Vec::new()),
        Some(child) => {
            if child.as_str() == Some("") {
                return Ok(Vec::new());
            }
            let entries = child
                .as_sequence()
                .ok_or(ProjectSettingsError::MissingDocument(document))?;
            Ok(entries
                .iter()
                .filter_map(|entry| entry.as_str().map(str::to_string))
                .collect())
        }
    }
}

fn parse_script_execution_order(entry: &Node) -> Option<(String, i64)> {
    let order = entry
        .get("second")
        .or_else(|| entry.get("order"))
        .and_then(Node::as_i64)?;
    if let Some(guid) = entry.get("guid").and_then(Node::as_str) {
        return Some((guid.to_string(), order));
    }
    let script = entry
        .get("first")
        .or_else(|| entry.get("script"))
        .or_else(|| entry.get("m_Name"))
        .or_else(|| entry.get("name"))?;
    let name = match script {
        Node::Scalar(text) => text.clone(),
        _ => script
            .get("guid")
            .or_else(|| script.get("m_Name"))
            .or_else(|| script.get("name"))
            .and_then(Node::as_str)
            .unwrap_or_default()
            .to_string(),
    };
    Some((name, order))
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
    pub physics_2d: Physics2DProjectSettings,
    pub identity: ProjectIdentity,
    pub quality: QualitySettings,
    pub default_scripting_defines: String,
    pub target_scripting_defines: Vec<(String, String)>,
    pub color_space: ColorSpaceSetting,
    pub tag_manager: Option<TagManagerSettings>,
    pub graphics: Option<GraphicsSettings>,
    pub audio: Option<AudioManagerSettings>,
    pub nav_mesh_areas: Option<NavMeshAreaSettings>,
    pub editor: Option<EditorSettings>,
    pub mono_manager: Option<MonoManagerSettings>,
    pub preset_manager: Option<PresetManagerSettings>,
    pub package_manager: Option<PackageManagerSettings>,
    pub version_control: Option<VersionControlSettings>,
    pub editor_build: Option<EditorBuildSettings>,
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

impl QualitySettings {
    pub fn effective_quality(
        &self,
        target_group_id: &str,
        target_name: &str,
    ) -> EffectiveTargetSetting {
        let (index, source) = match self
            .per_platform_default_quality
            .iter()
            .find(|(platform, _)| platform == target_group_id)
        {
            Some((_, index)) => (*index, SettingValueSource::TargetOverride),
            None => (self.current_quality, SettingValueSource::Default),
        };
        EffectiveTargetSetting {
            target_group_id: target_group_id.to_string(),
            target_name: target_name.to_string(),
            effective_value: self
                .levels
                .get(index)
                .map(|level| level.name.clone())
                .unwrap_or_default(),
            source,
            boundary: SettingBoundary::NextPlaySession,
        }
    }
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

    pub fn effective_quality(
        &self,
        target_group_id: &str,
        target_name: &str,
    ) -> EffectiveTargetSetting {
        self.quality.effective_quality(target_group_id, target_name)
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

fn malformed_file(path: &Path, error: impl Into<ProjectSettingsError>) -> ProjectSettingsError {
    ProjectSettingsError::MalformedFile {
        path: path.to_path_buf(),
        error: Box::new(error.into()),
    }
}

fn load_optional_settings_family<T>(
    path: &Path,
    parse: impl Fn(&Stream) -> Result<T, ProjectSettingsError>,
) -> Result<Option<T>, ProjectSettingsError> {
    if !path.exists() {
        return Ok(None);
    }
    let stream = unity_yaml::parse_file(path).map_err(|error| malformed_file(path, error))?;
    parse(&stream)
        .map(Some)
        .map_err(|error| malformed_file(path, error))
}

fn resolve_settings_dir(project_or_settings_dir: &Path) -> PathBuf {
    let candidate = project_or_settings_dir.join("ProjectSettings");
    if candidate.is_dir() {
        candidate
    } else {
        project_or_settings_dir.to_path_buf()
    }
}

pub fn load_project_settings(
    project_or_settings_dir: &Path,
) -> Result<LoadedProjectSettings, ProjectSettingsError> {
    let settings_dir = resolve_settings_dir(project_or_settings_dir);
    let time_path = settings_dir.join("TimeManager.asset");
    let mut fixed_timestep = DEFAULT_FIXED_TIMESTEP;
    let mut maximum_allowed_timestep = 0.33333334;
    let mut time_scale = 1.0;
    if time_path.exists() {
        let stream = unity_yaml::parse_file(&time_path)
            .map_err(|error| malformed_file(&time_path, error))?;
        fixed_timestep = parse_time_manager_stream(&stream)
            .map_err(|error| malformed_file(&time_path, error))?
            .fixed_timestep;
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
        let stream = unity_yaml::parse_file(&dynamics_path)
            .map_err(|error| malformed_file(&dynamics_path, error))?;
        gravity = parse_dynamics_manager_stream(&stream)
            .map_err(|error| malformed_file(&dynamics_path, error))?
            .gravity;
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
    let mut physics_2d = Physics2DProjectSettings::default();
    if physics2d_path.exists() {
        let stream = unity_yaml::parse_file(&physics2d_path)
            .map_err(|error| malformed_file(&physics2d_path, error))?;
        physics_2d = parse_physics2d_settings_stream(&stream)
            .map_err(|error| malformed_file(&physics2d_path, error))?;
    }
    let gravity_2d = physics_2d.gravity;
    let layer_collision_matrix_2d = physics_2d.layer_collision_matrix;

    let quality_path = settings_dir.join("QualitySettings.asset");
    let mut quality = QualitySettings::default();
    if quality_path.exists() {
        let stream = unity_yaml::parse_file(&quality_path)
            .map_err(|error| malformed_file(&quality_path, error))?;
        quality = parse_quality_settings_stream(&stream)
            .map_err(|error| malformed_file(&quality_path, error))?;
    }

    let player_path = settings_dir.join("ProjectSettings.asset");
    let mut identity = ProjectIdentity::default();
    let mut default_scripting_defines = String::new();
    let mut target_scripting_defines = Vec::new();
    let mut color_space = ColorSpaceSetting::default();
    if player_path.exists() {
        let stream = unity_yaml::parse_file(&player_path)
            .map_err(|error| malformed_file(&player_path, error))?;
        identity = parse_player_settings_stream(&stream)
            .map_err(|error| malformed_file(&player_path, error))?;
        color_space = identity.active_color_space;
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

    let tag_manager = load_optional_settings_family(
        &settings_dir.join("TagManager.asset"),
        parse_tag_manager_stream,
    )?;
    let graphics = load_optional_settings_family(
        &settings_dir.join("GraphicsSettings.asset"),
        parse_graphics_settings_stream,
    )?;
    let audio = load_optional_settings_family(
        &settings_dir.join("AudioManager.asset"),
        parse_audio_manager_stream,
    )?;
    let nav_mesh_areas = load_optional_settings_family(
        &settings_dir.join("NavMeshAreas.asset"),
        parse_nav_mesh_areas_stream,
    )?;
    let editor = load_optional_settings_family(
        &settings_dir.join("EditorSettings.asset"),
        parse_editor_settings_stream,
    )?;
    let mono_manager = load_optional_settings_family(
        &settings_dir.join("MonoManager.asset"),
        parse_mono_manager_stream,
    )?;
    let preset_manager = load_optional_settings_family(
        &settings_dir.join("PresetManager.asset"),
        parse_preset_manager_stream,
    )?;
    let package_manager = load_optional_settings_family(
        &settings_dir.join("PackageManagerSettings.asset"),
        parse_package_manager_settings_stream,
    )?;
    let version_control = load_optional_settings_family(
        &settings_dir.join("VersionControlSettings.asset"),
        parse_version_control_settings_stream,
    )?;
    let editor_build = load_optional_settings_family(
        &settings_dir.join("EditorBuildSettings.asset"),
        parse_editor_build_settings_stream,
    )?;

    Ok(LoadedProjectSettings {
        fixed_timestep,
        maximum_allowed_timestep,
        time_scale,
        gravity,
        layer_collision_matrix,
        gravity_2d,
        layer_collision_matrix_2d,
        physics_2d,
        identity,
        quality,
        default_scripting_defines,
        target_scripting_defines,
        color_space,
        tag_manager,
        graphics,
        audio,
        nav_mesh_areas,
        editor,
        mono_manager,
        preset_manager,
        package_manager,
        version_control,
        editor_build,
    })
}
