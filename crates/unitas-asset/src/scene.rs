use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use unitas_core::{CoreError, EntityId, SceneGraph};

use crate::material::Material;
use crate::meta::{GuidIndex, MetaError};
use crate::particle::{parse_particle_system, ParticleSystemBinding};
use crate::prefab;
use crate::prefab_variant::{
    ScenePrefabInstance, SourceObjectId, StructuralOverrides, TargetedModification,
};
use crate::trail::{trail_renderer_component, TrailRendererBinding};
use crate::unity_yaml::{self, Document, Node, Stream, YamlError};

use self::scene_audio::collect_audio;
use self::scene_physics::{
    collect_physics, collect_physics_2d, is_character_controller, AssembledScenePhysics,
};

pub use crate::particle::PARTICLE_SYSTEM_CLASS;
pub use crate::trail::TRAIL_RENDERER_CLASS;
pub use scene_audio::{AudioFilterComponent, AudioFilterKind};
pub use scene_physics::parse_physic_material;

#[path = "scene_anim.rs"]
mod scene_anim;
#[path = "scene_audio.rs"]
mod scene_audio;
#[path = "scene_physics.rs"]
pub mod scene_physics;
#[path = "scene_sprite.rs"]
mod scene_sprite;

use self::scene_sprite::{sprite_border, ui_image_draw_mode};

pub const AUDIO_SOURCE_CLASS: u32 = 82;
pub const AUDIO_CLIP_CLASS: u32 = 83;
pub const CAMERA_CLASS: u32 = 20;
pub const MESH_RENDERER_CLASS: u32 = 23;
pub const MESH_FILTER_CLASS: u32 = 33;
pub const MESH_COLLIDER_CLASS: u32 = 64;
pub const ANIMATOR_CLASS: u32 = 95;
pub const LIGHT_CLASS: u32 = 108;
pub const SPRITE_RENDERER_CLASS: u32 = 212;
pub const CANVAS_CLASS: u32 = 223;
pub const RECT_TRANSFORM_CLASS: u32 = 224;
pub const IMAGE_SCRIPT_GUID: &str = "fe87c0e1cc204ed48ad3b37840f39efc";
pub const TEXT_SCRIPT_GUID: &str = "5f7201a12d95ffc409449d95f23cf332";
pub const BUTTON_SCRIPT_GUID: &str = "4e29b1a8efbd4b44bb3f3716e73f07ff";
pub const CANVAS_SCALER_SCRIPT_GUID: &str = "0cd44c1031e13a943bb63640046fad76";
pub const HORIZONTAL_LAYOUT_GROUP_SCRIPT_GUID: &str = "306cc8c2b49d7114eaa3623786fc2126";
pub const VERTICAL_LAYOUT_GROUP_SCRIPT_GUID: &str = "59f8146938fff824cb5fd77236b75775";
pub const CONTENT_SIZE_FITTER_SCRIPT_GUID: &str = "3245ec927659c4140ac4f8d17403cc18";
pub type AssetGuid = String;

const GAME_OBJECT_CLASS: u32 = 1;
const TRANSFORM_CLASS: u32 = 4;
const RIGIDBODY_CLASS: u32 = 54;
const BOX_COLLIDER_CLASS: u32 = 65;
const MONO_BEHAVIOUR_CLASS: u32 = 114;
const PHYSIC_MATERIAL_CLASS: u32 = 134;
const SPHERE_COLLIDER_CLASS: u32 = 135;
const CAPSULE_COLLIDER_CLASS: u32 = 136;
const CHARACTER_CONTROLLER_CLASS: u32 = 143;
const PREFAB_INSTANCE_CLASS: u32 = 1001;
const ROOT_FILE_ID: i64 = 0;
const UNIT_QUATERNION_TOLERANCE: f64 = 1e-3;
const DEFAULT_PHYSIC_MATERIAL_FRICTION: f64 = 0.6;
const DEFAULT_PHYSIC_MATERIAL_RESTITUTION: f64 = 0.0;
const DEFAULT_RIGIDBODY_DRAG: f64 = 0.0;
const DEFAULT_RIGIDBODY_ANGULAR_DRAG: f64 = 0.05;
const DEFAULT_RIGIDBODY_SLEEP_THRESHOLD: f64 = 0.005;
const DEFAULT_RIGIDBODY_SLEEP_ANGULAR_THRESHOLD: f64 = 0.02;
const DEFAULT_RIGIDBODY_MAX_ANGULAR_VELOCITY: f64 = 7.0;
const DEFAULT_RIGIDBODY_MAX_DEPENETRATION_VELOCITY: f64 = 10.0;
const DEFAULT_CAPSULE_RADIUS: f64 = 0.5;
const DEFAULT_CAPSULE_HEIGHT: f64 = 2.0;
const DEFAULT_CAPSULE_DIRECTION: u32 = 1;
const DEFAULT_CHARACTER_CONTROLLER_ENABLED: bool = true;
const DEFAULT_CHARACTER_CONTROLLER_RADIUS: f64 = 0.5;
const DEFAULT_CHARACTER_CONTROLLER_HEIGHT: f64 = 2.0;
const DEFAULT_CHARACTER_CONTROLLER_SLOPE_LIMIT: f64 = 45.0;
const DEFAULT_CHARACTER_CONTROLLER_STEP_OFFSET: f64 = 0.3;
const DEFAULT_CHARACTER_CONTROLLER_SKIN_WIDTH: f64 = 0.08;
const DEFAULT_CHARACTER_CONTROLLER_MIN_MOVE_DISTANCE: f64 = 0.0;
const DEFAULT_AUDIO_ENABLED: bool = true;
const DEFAULT_AUDIO_VOLUME: f32 = 1.0;
const DEFAULT_AUDIO_PITCH: f32 = 1.0;
const DEFAULT_AUDIO_LOOPING: bool = false;
const DEFAULT_AUDIO_PLAY_ON_AWAKE: bool = true;
const DEFAULT_AUDIO_PRIORITY: u8 = 128;
const DEFAULT_AUDIO_SPATIAL_BLEND: f32 = 0.0;
const DEFAULT_AUDIO_MIN_DISTANCE: f32 = 1.0;
const DEFAULT_AUDIO_MAX_DISTANCE: f32 = 500.0;
const DEFAULT_AUDIO_MUTE: bool = false;
const DEFAULT_CAMERA_FIELD_OF_VIEW: f32 = 60.0;
const DEFAULT_CAMERA_NEAR_CLIP_PLANE: f32 = 0.3;
const DEFAULT_CAMERA_FAR_CLIP_PLANE: f32 = 1000.0;
const DEFAULT_CAMERA_ORTHOGRAPHIC_SIZE: f32 = 5.0;
const DEFAULT_CAMERA_BACKGROUND_COLOR: [f32; 3] = [0.19215687, 0.3019608, 0.4745098];
const DEFAULT_RENDERER_ENABLED: bool = true;
const DEFAULT_MATERIAL_COLOR: [f32; 3] = [0.5, 0.5, 0.5];
const DEFAULT_LIGHT_INTENSITY: f32 = 1.0;
const DEFAULT_LIGHT_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const DEFAULT_SPRITE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_LIGHT_DIRECTION: [f32; 3] = [0.0, -1.0, 0.0];
const DIRECTIONAL_LIGHT_TYPE: i64 = 1;
const DEFAULT_UI_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_UI_FONT_SIZE: f32 = 14.0;
const DEFAULT_UI_ENABLED: bool = true;
const DEFAULT_REFERENCE_RESOLUTION: [f32; 2] = [800.0, 600.0];
const DEFAULT_MATCH_WIDTH_OR_HEIGHT: f32 = 0.0;
const CHARACTER_CONTROLLER_SCRIPT_NAME: &str = "CharacterController";
const CHARACTER_CONTROLLER_FIELD_KEYS: [&str; 4] = [
    "m_SlopeLimit",
    "m_StepOffset",
    "m_SkinWidth",
    "m_MinMoveDistance",
];

#[derive(Debug)]
pub enum SceneError {
    MissingGameObject {
        transform_file_id: i64,
        game_object_file_id: i64,
    },
    DuplicateFileId {
        file_id: i64,
    },
    DuplicateTransformOwner {
        game_object_file_id: i64,
    },
    MissingFather {
        transform_file_id: i64,
        father_file_id: i64,
    },
    MissingChild {
        transform_file_id: i64,
        child_file_id: i64,
    },
    InvalidFileId {
        file_id: String,
    },
    UnresolvedExternalReference {
        guid: String,
        file_id: i64,
    },
    CyclicPrefabReference {
        guid: String,
    },
    MissingPrefabSource {
        file_id: i64,
    },
    PrefabWithoutRoot {
        guid: String,
    },
    Meta {
        error: MetaError,
    },
    Yaml {
        error: YamlError,
    },
    Core {
        error: CoreError,
    },
}

impl fmt::Display for SceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SceneError::MissingGameObject {
                transform_file_id,
                game_object_file_id,
            } => write!(
                formatter,
                "transform {transform_file_id} references unknown game object {game_object_file_id}"
            ),
            SceneError::DuplicateFileId { file_id } => {
                write!(formatter, "duplicate local file id {file_id}")
            }
            SceneError::DuplicateTransformOwner {
                game_object_file_id,
            } => {
                write!(
                    formatter,
                    "more than one transform owns game object {game_object_file_id}"
                )
            }
            SceneError::MissingFather {
                transform_file_id,
                father_file_id,
            } => write!(
                formatter,
                "transform {transform_file_id} references unknown father {father_file_id}"
            ),
            SceneError::MissingChild {
                transform_file_id,
                child_file_id,
            } => write!(
                formatter,
                "transform {transform_file_id} references unknown child {child_file_id}"
            ),
            SceneError::InvalidFileId { file_id } => write!(formatter, "invalid file id {file_id}"),
            SceneError::UnresolvedExternalReference { guid, file_id } => write!(
                formatter,
                "external reference {file_id} with guid {guid} is not in the guid index"
            ),
            SceneError::CyclicPrefabReference { guid } => {
                write!(formatter, "prefab with guid {guid} refers to itself")
            }
            SceneError::MissingPrefabSource { file_id } => {
                write!(
                    formatter,
                    "prefab instance {file_id} has no source prefab reference"
                )
            }
            SceneError::PrefabWithoutRoot { guid } => {
                write!(formatter, "prefab with guid {guid} has no root transform")
            }
            SceneError::Meta { error } => write!(formatter, "{error}"),
            SceneError::Yaml { error } => write!(formatter, "{error}"),
            SceneError::Core { error } => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SceneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SceneError::Yaml { error } => Some(error),
            SceneError::Core { error } => Some(error),
            SceneError::Meta { error } => Some(error),
            _ => None,
        }
    }
}

impl From<YamlError> for SceneError {
    fn from(error: YamlError) -> Self {
        SceneError::Yaml { error }
    }
}

impl From<CoreError> for SceneError {
    fn from(error: CoreError) -> Self {
        SceneError::Core { error }
    }
}

impl From<MetaError> for SceneError {
    fn from(error: MetaError) -> Self {
        SceneError::Meta { error }
    }
}

struct TransformRecord {
    file_id: i64,
    entity: EntityId,
    father: Option<i64>,
    children: Vec<i64>,
}

type GameObjectRecords = HashMap<i64, GameObjectRecord>;

pub(crate) struct GameObjectRecord {
    pub(crate) entity: EntityId,
    pub(crate) name: String,
    pub(crate) layer: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBinding {
    pub entity: EntityId,
    pub guid: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigidbodyInterpolation {
    None,
    Interpolate,
    Extrapolate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigidbodyCollisionDetection {
    Discrete,
    Continuous,
    ContinuousDynamic,
    ContinuousSpeculative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrictionCombine {
    Average,
    Minimum,
    Multiply,
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BounceCombine {
    Average,
    Minimum,
    Maximum,
    Multiply,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialValues {
    pub friction: f64,
    pub restitution: f64,
    pub friction_combine: FrictionCombine,
    pub bounce_combine: BounceCombine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RigidbodyComponent {
    pub mass: f64,
    pub drag: f64,
    pub angular_drag: f64,
    pub use_gravity: bool,
    pub is_trigger: bool,
    pub is_kinematic: bool,
    pub interpolation: RigidbodyInterpolation,
    pub collision_detection: RigidbodyCollisionDetection,
    pub sleep_threshold: f64,
    pub sleep_angular_threshold: f64,
    pub solver_iterations: Option<u32>,
    pub solver_velocity_iterations: Option<u32>,
    pub max_angular_velocity: f64,
    pub max_depenetration_velocity: f64,
    pub include_layers: Option<u32>,
    pub exclude_layers: Option<u32>,
    pub layer_override_priority: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoxColliderComponent {
    pub size: [f64; 3],
    pub center: [f64; 3],
    pub is_trigger: bool,
    pub material: Option<MaterialValues>,
    pub material_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SphereColliderComponent {
    pub radius: f64,
    pub center: [f64; 3],
    pub is_trigger: bool,
    pub material: Option<MaterialValues>,
    pub material_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapsuleColliderComponent {
    pub radius: f64,
    pub height: f64,
    pub direction: u32,
    pub center: [f64; 3],
    pub is_trigger: bool,
    pub material: Option<MaterialValues>,
    pub material_diagnostic: Option<String>,
    pub geometry_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshColliderComponent {
    pub mesh_guid: Option<AssetGuid>,
    pub mesh_file_id: Option<i64>,
    pub mesh_path: Option<PathBuf>,
    pub convex: bool,
    pub center: [f64; 3],
    pub is_trigger: bool,
    pub material: Option<MaterialValues>,
    pub material_diagnostic: Option<String>,
    pub mesh_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColliderComponent {
    Box(BoxColliderComponent),
    Sphere(SphereColliderComponent),
    Capsule(CapsuleColliderComponent),
    Mesh(MeshColliderComponent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterControllerComponent {
    pub enabled: bool,
    pub radius: f64,
    pub height: f64,
    pub center: [f64; 3],
    pub slope_limit: f64,
    pub step_offset: f64,
    pub skin_width: f64,
    pub min_move_distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsBinding {
    pub entity: EntityId,
    pub rigidbody: Option<RigidbodyComponent>,
    pub colliders: Vec<ColliderComponent>,
    pub character_controller: Option<CharacterControllerComponent>,
    pub layer: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioSourceComponent {
    pub enabled: bool,
    pub clip_guid: Option<AssetGuid>,
    pub clip_path: Option<PathBuf>,
    pub clip_diagnostic: Option<String>,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub play_on_awake: bool,
    pub priority: u8,
    pub spatial_blend: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub mute: bool,
    pub filters: Vec<AudioFilterComponent>,
}

impl Default for AudioSourceComponent {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_AUDIO_ENABLED,
            clip_guid: None,
            clip_path: None,
            clip_diagnostic: None,
            volume: DEFAULT_AUDIO_VOLUME,
            pitch: DEFAULT_AUDIO_PITCH,
            looping: DEFAULT_AUDIO_LOOPING,
            play_on_awake: DEFAULT_AUDIO_PLAY_ON_AWAKE,
            priority: DEFAULT_AUDIO_PRIORITY,
            spatial_blend: DEFAULT_AUDIO_SPATIAL_BLEND,
            min_distance: DEFAULT_AUDIO_MIN_DISTANCE,
            max_distance: DEFAULT_AUDIO_MAX_DISTANCE,
            mute: DEFAULT_AUDIO_MUTE,
            filters: Vec::new(),
        }
    }
}

pub type AudioBinding = (EntityId, AudioSourceComponent);

#[derive(Debug, Clone, PartialEq)]
pub struct CameraComponent {
    pub entity: EntityId,
    pub orthographic: bool,
    pub orthographic_size: f32,
    pub field_of_view: f32,
    pub near_clip_plane: f32,
    pub far_clip_plane: f32,
    pub background_color: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshRendererComponent {
    pub entity: EntityId,
    pub mesh_guid: Option<AssetGuid>,
    pub mesh_file_id: Option<i64>,
    pub mesh_path: Option<PathBuf>,
    pub mesh_diagnostic: Option<String>,
    pub material_guid: Option<AssetGuid>,
    pub material_path: Option<PathBuf>,
    pub material_diagnostic: Option<String>,
    pub material_color: [f32; 3],
    pub albedo_tex_path: Option<PathBuf>,
    pub normal_tex_path: Option<PathBuf>,
    pub emission_tex_path: Option<PathBuf>,
    pub metallic: Option<f32>,
    pub smoothness: Option<f32>,
    pub emission_color: Option<[f32; 4]>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightComponent {
    pub entity: EntityId,
    pub directional: bool,
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
    pub unsupported_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteDrawMode {
    Simple,
    Sliced,
    Tiled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRendererComponent {
    pub entity: EntityId,
    pub sprite_guid: Option<AssetGuid>,
    pub sprite_file_id: Option<i64>,
    pub sprite_path: Option<PathBuf>,
    pub color: [f32; 4],
    pub sorting_layer: i32,
    pub sorting_order: i32,
    pub draw_mode: SpriteDrawMode,
    pub flip_x: bool,
    pub flip_y: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderBinding {
    pub cameras: Vec<CameraComponent>,
    pub renderers: Vec<MeshRendererComponent>,
    pub lights: Vec<LightComponent>,
    pub sprites: Vec<SpriteRendererComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRenderMode {
    ScreenSpaceOverlay,
    ScreenSpaceCamera,
    WorldSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasScaleMode {
    ConstantPixelSize,
    ScaleWithScreenSize,
    ConstantPhysicalSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasMatchMode {
    MatchWidthOrHeight,
    Shrink,
    Expand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasScalerComponent {
    pub ui_scale_mode: CanvasScaleMode,
    pub reference_resolution: [f32; 2],
    pub screen_match_mode: CanvasMatchMode,
    pub match_width_or_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasComponent {
    pub entity: EntityId,
    pub render_mode: CanvasRenderMode,
    pub sorting_order: i64,
    pub scaler: Option<CanvasScalerComponent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectTransformComponent {
    pub entity: EntityId,
    pub parent: Option<EntityId>,
    pub anchor_min: [f32; 2],
    pub anchor_max: [f32; 2],
    pub anchored_position: [f32; 2],
    pub size_delta: [f32; 2],
    pub pivot: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiImageComponent {
    pub entity: EntityId,
    pub color: [f32; 4],
    pub sprite_guid: Option<AssetGuid>,
    pub enabled: bool,
    pub draw_mode: SpriteDrawMode,
    /// The nine-slice border of the sprite in sprite pixels: left, bottom, right, top.
    pub sprite_border: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiTextComponent {
    pub entity: EntityId,
    pub text: String,
    pub color: [f32; 4],
    pub font_size: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiButtonComponent {
    pub entity: EntityId,
    pub target_graphic: Option<EntityId>,
    pub interactable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiPadding {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutGroupAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiLayoutGroupComponent {
    pub entity: EntityId,
    pub axis: LayoutGroupAxis,
    pub padding: UiPadding,
    pub spacing: f32,
    pub child_alignment: u32,
    pub child_control_width: bool,
    pub child_control_height: bool,
    pub child_force_expand_width: bool,
    pub child_force_expand_height: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentSizeFitMode {
    #[default]
    Unconstrained = 0,
    MinSize = 1,
    PreferredSize = 2,
}

impl ContentSizeFitMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Unconstrained),
            1 => Some(Self::MinSize),
            2 => Some(Self::PreferredSize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiContentSizeFitterComponent {
    pub entity: EntityId,
    pub horizontal_fit: ContentSizeFitMode,
    pub vertical_fit: ContentSizeFitMode,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiElementComponent {
    Image(UiImageComponent),
    Text(UiTextComponent),
    Button(UiButtonComponent),
}

impl UiElementComponent {
    pub fn entity(&self) -> EntityId {
        match self {
            Self::Image(image) => image.entity,
            Self::Text(text) => text.entity,
            Self::Button(button) => button.entity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiBinding {
    pub canvases: Vec<CanvasComponent>,
    pub rect_transforms: Vec<RectTransformComponent>,
    pub elements: Vec<UiElementComponent>,
    pub layout_groups: Vec<UiLayoutGroupComponent>,
    pub content_size_fitters: Vec<UiContentSizeFitterComponent>,
}

impl UiBinding {
    pub fn shift_entity_ids(&mut self, base: u64) {
        for canvas in &mut self.canvases {
            canvas.entity = EntityId::new(base + canvas.entity.as_u64());
        }
        for rect in &mut self.rect_transforms {
            rect.entity = EntityId::new(base + rect.entity.as_u64());
            rect.parent = rect
                .parent
                .map(|parent| EntityId::new(base + parent.as_u64()));
        }
        for element in &mut self.elements {
            match element {
                UiElementComponent::Image(image) => {
                    image.entity = EntityId::new(base + image.entity.as_u64());
                }
                UiElementComponent::Text(text) => {
                    text.entity = EntityId::new(base + text.entity.as_u64());
                }
                UiElementComponent::Button(button) => {
                    button.entity = EntityId::new(base + button.entity.as_u64());
                    button.target_graphic = button
                        .target_graphic
                        .map(|target| EntityId::new(base + target.as_u64()));
                }
            }
        }
        for group in &mut self.layout_groups {
            group.entity = EntityId::new(base + group.entity.as_u64());
        }
        for fitter in &mut self.content_size_fitters {
            fitter.entity = EntityId::new(base + fitter.entity.as_u64());
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBinding {
    pub entity: EntityId,
    pub binding: ParticleSystemBinding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrailBinding {
    pub entity: EntityId,
    pub binding: TrailRendererBinding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorBinding {
    pub entity: EntityId,
    pub controller_guid: Option<String>,
    pub enabled: bool,
    pub apply_root_motion: bool,
    pub culling_mode: i32,
    pub update_mode: i32,
}

#[derive(Debug)]
pub struct AssembledScene {
    pub graph: SceneGraph,
    pub scripts: Vec<ScriptBinding>,
    pub physics: Vec<PhysicsBinding>,
    pub physics_2d: AssembledScenePhysics,
    pub audio_bindings: Vec<AudioBinding>,
    pub render: RenderBinding,
    pub particles: Vec<ParticleBinding>,
    pub trails: Vec<TrailBinding>,
    pub animators: Vec<AnimatorBinding>,
    pub ui: UiBinding,
    pub diagnostics: Vec<(EntityId, String)>,
}

struct DocumentSet {
    game_objects: GameObjectRecords,
    transforms: HashMap<i64, EntityId>,
    root: Option<EntityId>,
    scripts: Vec<ScriptBinding>,
    physics: Vec<PhysicsBinding>,
    physics_2d: AssembledScenePhysics,
    audio_bindings: Vec<AudioBinding>,
    render: RenderBinding,
    particles: Vec<ParticleBinding>,
    trails: Vec<TrailBinding>,
    animators: Vec<AnimatorBinding>,
    ui: UiBinding,
    unsupported_classes: Vec<(EntityId, u32)>,
}

struct PrefabContext<'a> {
    index: Option<&'a GuidIndex>,
    stack: Vec<String>,
    composed_modifications: Vec<(String, TargetedModification)>,
    composed_structural: Vec<(String, StructuralOverrides)>,
    prefab_streams: HashMap<PathBuf, Stream>,
    material_properties: HashMap<PathBuf, MaterialProperties>,
}

pub fn assemble_scene(stream: &Stream) -> Result<SceneGraph, SceneError> {
    Ok(assemble_stream(stream, None, false)?.graph)
}

pub fn assemble_scene_with_guids(
    stream: &Stream,
    index: &GuidIndex,
) -> Result<SceneGraph, SceneError> {
    Ok(assemble_stream(stream, Some(index), false)?.graph)
}

pub fn assemble_scene_file(path: &Path) -> Result<SceneGraph, SceneError> {
    Ok(assemble_file(path, None)?.graph)
}

pub fn assemble_scene_file_with_guids(
    path: &Path,
    index: &GuidIndex,
) -> Result<SceneGraph, SceneError> {
    Ok(assemble_file(path, Some(index))?.graph)
}

pub fn assemble_scene_with_physics(
    stream: &Stream,
) -> Result<(SceneGraph, Vec<ScriptBinding>, Vec<PhysicsBinding>), SceneError> {
    let assembly = assemble_stream(stream, None, false)?;
    Ok((assembly.graph, assembly.scripts, assembly.physics))
}

pub fn assemble_scene_file_with_physics(
    path: &Path,
) -> Result<(SceneGraph, Vec<ScriptBinding>, Vec<PhysicsBinding>), SceneError> {
    let assembly = assemble_file(path, None)?;
    Ok((assembly.graph, assembly.scripts, assembly.physics))
}

pub fn assemble_scene_bundle(stream: &Stream) -> Result<AssembledScene, SceneError> {
    assemble_stream(stream, None, false)
}

pub fn assemble_scene_bundle_with_guids(
    stream: &Stream,
    index: &GuidIndex,
) -> Result<AssembledScene, SceneError> {
    assemble_stream(stream, Some(index), false)
}

pub fn assemble_scene_with_index(
    stream: &Stream,
    index: &GuidIndex,
) -> Result<AssembledScene, SceneError> {
    assemble_stream(stream, Some(index), false)
}

pub fn assemble_scene_file_bundle(path: &Path) -> Result<AssembledScene, SceneError> {
    assemble_file(path, None)
}

pub fn assemble_scene_file_bundle_with_guids(
    path: &Path,
    index: &GuidIndex,
) -> Result<AssembledScene, SceneError> {
    assemble_file(path, Some(index))
}

pub fn assemble_scene_file_with_index(
    path: &Path,
    index: &GuidIndex,
) -> Result<AssembledScene, SceneError> {
    assemble_file(path, Some(index))
}

pub fn assemble_scene_with_audio(
    stream: &Stream,
    index: Option<&GuidIndex>,
) -> Result<AssembledScene, SceneError> {
    assemble_stream(stream, index, false)
}

pub fn assemble_scene_file_with_audio(path: &Path) -> Result<AssembledScene, SceneError> {
    assemble_file(path, None)
}

fn assemble_stream(
    stream: &Stream,
    index: Option<&GuidIndex>,
    expand: bool,
) -> Result<AssembledScene, SceneError> {
    if !expand {
        check_external_references(stream, index)?;
    }
    let mut graph = SceneGraph::new();
    let mut context = PrefabContext {
        index,
        stack: Vec::new(),
        composed_modifications: Vec::new(),
        composed_structural: Vec::new(),
        prefab_streams: HashMap::new(),
        material_properties: HashMap::new(),
    };
    let set = assemble_documents(stream, &mut graph, &mut context, expand)?;
    let mut diagnostics = collect_scene_diagnostics(&set.physics, &set.audio_bindings, &set.render);
    append_unsupported_class_diagnostics(&mut diagnostics, &set.unsupported_classes);
    Ok(AssembledScene {
        graph,
        scripts: set.scripts,
        physics: set.physics,
        physics_2d: set.physics_2d,
        audio_bindings: set.audio_bindings,
        render: set.render,
        particles: set.particles,
        trails: set.trails,
        animators: set.animators,
        ui: set.ui,
        diagnostics,
    })
}

fn assemble_file(path: &Path, index: Option<&GuidIndex>) -> Result<AssembledScene, SceneError> {
    let stream = unity_yaml::parse_file(path)?;
    let expand = has_prefab_instances(&stream);
    let resolve_guids = expand
        || has_material_references(&stream)
        || has_audio_references(&stream)
        || has_render_references(&stream);
    let discovered = match index {
        Some(_) => None,
        None => {
            if resolve_guids {
                match prefab::guid_index(path)? {
                    Some(found) => Some(found),
                    None => path
                        .parent()
                        .and_then(|parent| GuidIndex::build(parent).ok()),
                }
            } else {
                None
            }
        }
    };
    let index = match index {
        Some(index) => Some(index),
        None => discovered.as_ref(),
    };
    assemble_stream(&stream, index, expand)
}

fn prefab_instance_roots(stream: &Stream) -> HashSet<i64> {
    let instance_file_ids: HashSet<i64> = stream
        .documents
        .iter()
        .filter(|document| document.class_id == PREFAB_INSTANCE_CLASS && !document.stripped)
        .map(|document| document.file_id)
        .collect();
    let mut roots = HashSet::new();
    for document in &stream.documents {
        if document.class_id != GAME_OBJECT_CLASS || document.stripped {
            continue;
        }
        let file_id = document
            .root
            .get("GameObject")
            .and_then(|body| body.get("m_PrefabInstance"))
            .and_then(|node| node.get("fileID"))
            .and_then(Node::as_i64)
            .unwrap_or(ROOT_FILE_ID);
        if instance_file_ids.contains(&file_id) {
            roots.insert(document.file_id);
        }
    }
    roots
}

fn has_prefab_instances(stream: &Stream) -> bool {
    stream
        .documents
        .iter()
        .any(|document| document.class_id == PREFAB_INSTANCE_CLASS && !document.stripped)
}

fn has_material_references(stream: &Stream) -> bool {
    stream.documents.iter().any(|document| {
        if document.stripped {
            return false;
        }
        let body = match document.class_id {
            BOX_COLLIDER_CLASS => document.root.get("BoxCollider"),
            SPHERE_COLLIDER_CLASS => document.root.get("SphereCollider"),
            CAPSULE_COLLIDER_CLASS => document.root.get("CapsuleCollider"),
            MESH_COLLIDER_CLASS => document.root.get("MeshCollider"),
            _ => None,
        };
        body.and_then(|node| node.get("m_Material")).is_some()
    })
}

fn has_audio_references(stream: &Stream) -> bool {
    stream.documents.iter().any(|document| {
        if document.stripped || document.class_id != AUDIO_SOURCE_CLASS {
            return false;
        }
        document
            .root
            .get("AudioSource")
            .and_then(|node| node.get("m_audioClip").or_else(|| node.get("m_AudioClip")))
            .is_some()
    })
}

fn has_render_references(stream: &Stream) -> bool {
    stream.documents.iter().any(|document| {
        if document.stripped {
            return false;
        }
        match document.class_id {
            MESH_FILTER_CLASS => document
                .root
                .get("MeshFilter")
                .and_then(|node| node.get("m_Mesh"))
                .is_some(),
            MESH_COLLIDER_CLASS => document
                .root
                .get("MeshCollider")
                .and_then(|node| node.get("m_Mesh"))
                .is_some(),
            MESH_RENDERER_CLASS => document
                .root
                .get("MeshRenderer")
                .and_then(|node| node.get("m_Materials"))
                .is_some(),
            SPRITE_RENDERER_CLASS => document
                .root
                .get("SpriteRenderer")
                .and_then(|node| node.get("m_Sprite"))
                .is_some(),
            _ => false,
        }
    })
}

fn assemble_documents(
    stream: &Stream,
    graph: &mut SceneGraph,
    context: &mut PrefabContext,
    expand: bool,
) -> Result<DocumentSet, SceneError> {
    let mut seen_file_ids: HashSet<i64> = HashSet::new();
    for document in &stream.documents {
        if !seen_file_ids.insert(document.file_id) {
            return Err(SceneError::DuplicateFileId {
                file_id: document.file_id,
            });
        }
    }
    let mut set = DocumentSet {
        game_objects: HashMap::new(),
        transforms: HashMap::new(),
        root: None,
        scripts: Vec::new(),
        physics: Vec::new(),
        physics_2d: AssembledScenePhysics::default(),
        audio_bindings: Vec::new(),
        render: RenderBinding::default(),
        particles: Vec::new(),
        trails: Vec::new(),
        animators: Vec::new(),
        ui: UiBinding::default(),
        unsupported_classes: Vec::new(),
    };
    let mut instance_roots = prefab_instance_roots(stream);
    instance_roots.extend(added_game_object_ids(stream));
    for document in &stream.documents {
        if document.class_id != GAME_OBJECT_CLASS || document.stripped {
            continue;
        }
        if instance_roots.contains(&document.file_id) {
            continue;
        }
        let body = document.root.get("GameObject");
        let name = match body
            .and_then(|body| body.get("m_Name"))
            .and_then(Node::as_str)
        {
            Some(name) => name.to_string(),
            None => String::from("GameObject"),
        };
        let entity = graph.add_entity(name.clone());
        let layer = yaml_u32(body.and_then(|body| body.get("m_Layer")), 0);
        set.game_objects.insert(
            document.file_id,
            GameObjectRecord {
                entity,
                name,
                layer,
            },
        );
    }
    build_transforms(stream, graph, &instance_roots, &mut set)?;
    for document in &stream.documents {
        if document.class_id != MONO_BEHAVIOUR_CLASS || document.stripped {
            continue;
        }
        let body = match document.root.get("MonoBehaviour") {
            Some(body) if !is_character_controller(body) => body,
            _ => continue,
        };
        let game_object_file_id = match reference_id(body.get("m_GameObject"))? {
            Some(file_id) => file_id,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id: ROOT_FILE_ID,
                })
            }
        };
        let entity = match set.game_objects.get(&game_object_file_id) {
            Some(record) => record.entity,
            None => {
                if instance_roots.contains(&game_object_file_id) {
                    continue;
                }
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                });
            }
        };
        let script_node = match body.get("m_Script") {
            Some(node) => node,
            None => continue,
        };
        let guid = match script_node.get("guid").and_then(Node::as_str) {
            Some(guid) => guid.to_string(),
            None => continue,
        };
        set.scripts.push(ScriptBinding {
            entity,
            guid,
            fields: serialized_fields(body),
        });
    }
    set.physics = collect_physics(stream, &set.game_objects, &instance_roots, context.index)?;
    set.physics_2d = collect_physics_2d(stream, &set.game_objects, &instance_roots);
    set.audio_bindings = collect_audio(stream, &set.game_objects, &instance_roots, context.index)?;
    let index = context.index;
    let material_properties = &mut context.material_properties;
    set.render = collect_render(
        stream,
        &set.game_objects,
        &instance_roots,
        index,
        material_properties,
    )?;
    set.ui = collect_ui(stream, &set.game_objects, &instance_roots, index);
    let (particles, trails) =
        collect_particles_and_trails(stream, &set.game_objects, &instance_roots);
    set.particles = particles;
    set.trails = trails;
    set.animators = scene_anim::collect_animators(stream, &set.game_objects, &instance_roots);
    set.unsupported_classes =
        collect_unsupported_components(stream, &set.game_objects, &instance_roots);
    if expand {
        expand_prefab_instances(stream, &mut set, graph, context)?;
    }
    Ok(set)
}

fn build_transforms(
    stream: &Stream,
    graph: &mut SceneGraph,
    instance_roots: &HashSet<i64>,
    set: &mut DocumentSet,
) -> Result<(), SceneError> {
    let mut owners: HashMap<i64, i64> = HashMap::new();
    let mut records: Vec<TransformRecord> = Vec::new();
    for document in &stream.documents {
        if document.class_id != TRANSFORM_CLASS || document.stripped {
            continue;
        }
        let body = match document.root.get("Transform") {
            Some(body) => body,
            None => continue,
        };
        let game_object_file_id = match reference_id(body.get("m_GameObject"))? {
            Some(file_id) => file_id,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id: ROOT_FILE_ID,
                })
            }
        };
        if instance_roots.contains(&game_object_file_id) {
            continue;
        }
        let entity = match set.game_objects.get(&game_object_file_id) {
            Some(record) => record.entity,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                })
            }
        };
        if owners.contains_key(&game_object_file_id) {
            return Err(SceneError::DuplicateTransformOwner {
                game_object_file_id,
            });
        }
        owners.insert(game_object_file_id, document.file_id);
        let father = match reference_id(body.get("m_Father"))? {
            Some(file_id) if file_id != ROOT_FILE_ID => Some(file_id),
            _ => None,
        };
        let children = child_ids(body.get("m_Children"))?;
        apply_local_transform(graph, entity, body);
        set.transforms.insert(document.file_id, entity);
        records.push(TransformRecord {
            file_id: document.file_id,
            entity,
            father,
            children,
        });
    }
    for record in &records {
        if let Some(father_file_id) = record.father {
            let parent = match set.transforms.get(&father_file_id) {
                Some(parent) => *parent,
                None => {
                    return Err(SceneError::MissingFather {
                        transform_file_id: record.file_id,
                        father_file_id,
                    })
                }
            };
            graph.attach(record.entity, parent)?;
        }
        for child_file_id in &record.children {
            if !set.transforms.contains_key(child_file_id) {
                return Err(SceneError::MissingChild {
                    transform_file_id: record.file_id,
                    child_file_id: *child_file_id,
                });
            }
        }
    }
    for record in &records {
        let ordered: Vec<EntityId> = record
            .children
            .iter()
            .filter_map(|file_id| set.transforms.get(file_id).copied())
            .collect();
        order_children(graph, record.entity, &ordered);
    }
    set.root = records
        .iter()
        .find(|record| record.father.is_none())
        .map(|record| record.entity);
    Ok(())
}

fn expand_prefab_instances(
    stream: &Stream,
    set: &mut DocumentSet,
    graph: &mut SceneGraph,
    context: &mut PrefabContext,
) -> Result<(), SceneError> {
    for document in &stream.documents {
        if document.class_id != PREFAB_INSTANCE_CLASS || document.stripped {
            continue;
        }
        let body = match document.root.get("PrefabInstance") {
            Some(body) => body,
            None => continue,
        };
        let instance = match prefab::parse_instance(body) {
            Some(instance) => instance,
            None => {
                return Err(SceneError::MissingPrefabSource {
                    file_id: document.file_id,
                })
            }
        };
        let expanded_root =
            expand_instance(&instance, document.file_id, stream, set, graph, context)?;
        if set.root.is_none() && instance.transform_parent == ROOT_FILE_ID {
            set.root = Some(expanded_root);
        }
    }
    Ok(())
}

fn expand_instance(
    instance: &prefab::PrefabInstance,
    file_id: i64,
    containing: &Stream,
    set: &mut DocumentSet,
    graph: &mut SceneGraph,
    context: &mut PrefabContext,
) -> Result<EntityId, SceneError> {
    let path = match context
        .index
        .and_then(|index| index.get(&instance.source.guid))
    {
        Some(path) => path.to_path_buf(),
        None => {
            return Err(SceneError::UnresolvedExternalReference {
                guid: instance.source.guid.clone(),
                file_id: instance.source.file_id,
            })
        }
    };
    if context.stack.contains(&instance.source.guid) {
        return Err(SceneError::CyclicPrefabReference {
            guid: instance.source.guid.clone(),
        });
    }
    let mut stream = match context.prefab_streams.get(&path) {
        Some(cached) => cached.clone(),
        None => {
            let parsed = unity_yaml::parse_file(&path)?;
            context.prefab_streams.insert(path.clone(), parsed.clone());
            parsed
        }
    };
    let guid = instance.source.guid.clone();
    let scene_instance =
        ScenePrefabInstance::from_stream(containing, format!("prefab-instance-{file_id}"), file_id);
    let targeted = scene_instance
        .as_ref()
        .map(|entry| entry.targeted_modifications.clone())
        .unwrap_or_default();
    let structural = scene_instance
        .as_ref()
        .map(|entry| entry.structural_overrides.clone())
        .unwrap_or_default();

    let mut direct: Vec<TargetedModification> = Vec::new();
    let mut composed: Vec<TargetedModification> = Vec::new();
    for modification in targeted {
        let target_guid = modification.target.source_guid.clone();
        if target_guid.is_empty() || target_guid == guid {
            direct.push(modification);
        } else {
            composed.push(modification);
        }
    }

    let inherited: Vec<TargetedModification> = context
        .composed_modifications
        .iter()
        .filter(|(target_guid, _)| *target_guid == guid)
        .map(|(_, modification)| modification.clone())
        .collect();

    for modification in &direct {
        if apply_modification_to_stream(&mut stream, modification) {
            continue;
        }
        if let Some((routed_guid, routed_file_id)) = routed_target(&stream, modification) {
            composed.push(TargetedModification::new(
                SourceObjectId::new(routed_guid, routed_file_id),
                modification.property_path.clone(),
                modification.value.clone(),
            ));
        }
    }
    for modification in &inherited {
        apply_modification_to_stream(&mut stream, modification);
    }

    let inherited_structural: Vec<StructuralOverrides> = context
        .composed_structural
        .iter()
        .filter(|(target_guid, _)| *target_guid == guid)
        .map(|(_, overrides)| overrides.clone())
        .collect();
    let structural_mark = context.composed_structural.len();
    for (target_guid, overrides) in group_structural_by_guid(&structural) {
        if target_guid.is_empty() || target_guid == guid {
            apply_structural_overrides(&mut stream, containing, &overrides);
        } else {
            context.composed_structural.push((target_guid, overrides));
        }
    }
    for overrides in &inherited_structural {
        apply_structural_overrides(&mut stream, containing, overrides);
    }

    let composed_mark = context.composed_modifications.len();
    for modification in composed {
        context
            .composed_modifications
            .push((modification.target.source_guid.clone(), modification));
    }

    context.stack.push(instance.source.guid.clone());
    let assembled = assemble_documents(&stream, graph, context, true);
    context.stack.pop();
    context.composed_modifications.truncate(composed_mark);
    context.composed_structural.truncate(structural_mark);
    let prefab_set = assembled?;
    let root = match prefab_set.root {
        Some(root) => root,
        None => {
            return Err(SceneError::PrefabWithoutRoot {
                guid: instance.source.guid.clone(),
            })
        }
    };
    let mut prefab_set = prefab_set;
    let entity_modifications: Vec<prefab::Modification> = direct
        .iter()
        .chain(inherited.iter())
        .map(TargetedModification::to_modification)
        .collect();
    prefab::apply_modifications(
        &entity_modifications,
        &prefab_set.game_objects,
        &prefab_set.transforms,
        graph,
    );
    apply_overridden_bindings(
        &entity_modifications,
        &stream,
        &mut prefab_set,
        context.index,
        &mut context.material_properties,
    );
    if instance.transform_parent != ROOT_FILE_ID {
        let parent = match set.transforms.get(&instance.transform_parent) {
            Some(parent) => *parent,
            None => {
                return Err(SceneError::MissingFather {
                    transform_file_id: file_id,
                    father_file_id: instance.transform_parent,
                })
            }
        };
        graph.attach(root, parent)?;
    }
    set.scripts.extend(prefab_set.scripts);
    set.physics.extend(prefab_set.physics);
    set.physics_2d
        .colliders
        .extend(prefab_set.physics_2d.colliders);
    set.physics_2d
        .effectors
        .extend(prefab_set.physics_2d.effectors);
    set.audio_bindings.extend(prefab_set.audio_bindings);
    set.render.cameras.extend(prefab_set.render.cameras);
    set.render.renderers.extend(prefab_set.render.renderers);
    set.render.lights.extend(prefab_set.render.lights);
    set.particles.extend(prefab_set.particles);
    set.trails.extend(prefab_set.trails);
    set.unsupported_classes
        .extend(prefab_set.unsupported_classes);
    Ok(root)
}

fn added_game_object_ids(stream: &Stream) -> HashSet<i64> {
    let mut ids = HashSet::new();
    for document in &stream.documents {
        if document.class_id != PREFAB_INSTANCE_CLASS || document.stripped {
            continue;
        }
        let body = match document.root.get("PrefabInstance") {
            Some(body) => body,
            None => continue,
        };
        let items = match body
            .get("m_Modification")
            .and_then(|modification| modification.get("m_AddedGameObjects"))
            .and_then(Node::as_sequence)
        {
            Some(items) => items,
            None => continue,
        };
        for item in items {
            if let Some(file_id) = item
                .get("addedObject")
                .and_then(|node| node.get("fileID"))
                .and_then(Node::as_i64)
            {
                ids.insert(file_id);
            }
        }
    }
    ids
}

fn apply_modification_to_stream(stream: &mut Stream, modification: &TargetedModification) -> bool {
    let index = match stream.documents.iter().position(|document| {
        document.file_id == modification.target.target_file_id && !document.stripped
    }) {
        Some(index) => index,
        None => return false,
    };
    let value = match &modification.object_reference {
        Some(reference) if modification.value.is_empty() => reference.to_encoded_string(),
        _ => modification.value.clone(),
    };
    prefab::apply_modification_to_document(
        &mut stream.documents[index],
        &modification.property_path,
        &value,
    )
}

fn routed_target(stream: &Stream, modification: &TargetedModification) -> Option<(String, i64)> {
    for document in &stream.documents {
        if !document.stripped {
            continue;
        }
        let (guid, file_id) = match prefab::corresponding_source(document) {
            Some(entry) => entry,
            None => continue,
        };
        if guid.is_empty() || file_id != modification.target.target_file_id {
            continue;
        }
        return Some((guid, file_id));
    }
    None
}

fn group_structural_by_guid(overrides: &StructuralOverrides) -> Vec<(String, StructuralOverrides)> {
    let mut groups: Vec<(String, StructuralOverrides)> = Vec::new();
    for removed in &overrides.removed_components {
        let group = structural_group(&mut groups, &removed.source_guid);
        group.removed_components.push(removed.clone());
    }
    for removed in &overrides.removed_children {
        let group = structural_group(&mut groups, &removed.source_guid);
        group.removed_children.push(removed.clone());
    }
    for added in &overrides.added_children {
        let group = structural_group(&mut groups, &added.parent_target.source_guid);
        group.added_children.push(added.clone());
    }
    for added in &overrides.added_components {
        let group = structural_group(&mut groups, &added.owner_target.source_guid);
        group.added_components.push(added.clone());
    }
    for (target, order) in &overrides.added_sibling_order {
        let group = structural_group(&mut groups, &target.source_guid);
        group
            .added_sibling_order
            .insert(target.clone(), order.clone());
    }
    groups
}

fn structural_group<'a>(
    groups: &'a mut Vec<(String, StructuralOverrides)>,
    guid: &str,
) -> &'a mut StructuralOverrides {
    if !groups.iter().any(|(entry, _)| entry == guid) {
        groups.push((guid.to_string(), StructuralOverrides::default()));
    }
    let index = groups
        .iter()
        .position(|(entry, _)| entry == guid)
        .unwrap_or(0);
    &mut groups[index].1
}

fn apply_structural_overrides(
    stream: &mut Stream,
    containing: &Stream,
    overrides: &StructuralOverrides,
) {
    let removed_components: Vec<i64> = overrides
        .removed_components
        .iter()
        .map(|removed| removed.target_file_id)
        .collect();
    let removed_children: Vec<i64> = overrides
        .removed_children
        .iter()
        .map(|removed| removed.target_file_id)
        .collect();
    prefab::remove_components(stream, &removed_components);
    for child in removed_children {
        prefab::remove_game_object(stream, child);
    }
    for added in &overrides.added_children {
        let parent =
            match prefab::resolve_transform_file_id(stream, added.parent_target.target_file_id) {
                Some(parent) => parent,
                None => continue,
            };
        prefab::copy_added_game_object(
            stream,
            containing,
            added.child.file_id,
            parent,
            added.sibling_index,
        );
    }
    for added in &overrides.added_components {
        let owner =
            match prefab::resolve_game_object_file_id(stream, added.owner_target.target_file_id) {
                Some(owner) => owner,
                None => continue,
            };
        prefab::copy_added_component(stream, containing, added.component.file_id, owner);
    }
}

const SCRIPT_RESERVED_KEYS: [&str; 7] = [
    "m_ObjectHideFlags",
    "m_GameObject",
    "m_Script",
    "m_PrefabInstance",
    "m_CorrespondingSourceObject",
    "m_PrefabAsset",
    "serializedVersion",
];

fn serialized_fields(body: &Node) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    if let Some(entries) = body.as_mapping() {
        for (key, value) in entries {
            if SCRIPT_RESERVED_KEYS.contains(&key.as_str()) {
                continue;
            }
            if let Some(scalar) = value.as_str() {
                fields.insert(key.clone(), scalar.to_string());
            }
        }
    }
    fields
}

fn apply_overridden_bindings(
    modifications: &[prefab::Modification],
    stream: &Stream,
    set: &mut DocumentSet,
    index: Option<&GuidIndex>,
    material_properties: &mut HashMap<PathBuf, MaterialProperties>,
) {
    for modification in modifications {
        let document =
            match stream.documents.iter().find(|document| {
                document.file_id == modification.target_file_id && !document.stripped
            }) {
                Some(document) => document,
                None => continue,
            };
        let body = match document.class_id {
            MESH_RENDERER_CLASS => document.root.get("MeshRenderer"),
            MONO_BEHAVIOUR_CLASS => document.root.get("MonoBehaviour"),
            _ => continue,
        };
        let body = match body {
            Some(body) => body,
            None => continue,
        };
        let game_object_file_id = match reference_id(body.get("m_GameObject")) {
            Ok(Some(file_id)) => file_id,
            _ => continue,
        };
        let entity = match set.game_objects.get(&game_object_file_id) {
            Some(record) => record.entity,
            None => continue,
        };
        if document.class_id == MESH_RENDERER_CLASS {
            apply_material_override(modification, entity, set, index, material_properties);
        } else {
            apply_script_field_override(modification, entity, set);
        }
    }
}

fn material_slot(property_path: &str) -> Option<usize> {
    let rest = property_path.strip_prefix("m_Materials.Array.data[")?;
    let slot = rest.strip_suffix(']')?;
    slot.parse::<usize>().ok()
}

fn apply_material_override(
    modification: &prefab::Modification,
    entity: EntityId,
    set: &mut DocumentSet,
    index: Option<&GuidIndex>,
    material_properties: &mut HashMap<PathBuf, MaterialProperties>,
) {
    if material_slot(&modification.property_path) != Some(0) {
        return;
    }
    let guid = match modification
        .object_reference
        .as_ref()
        .and_then(|reference| reference.guid.clone())
        .filter(|guid| !guid.is_empty())
    {
        Some(guid) => guid,
        None => return,
    };
    let renderer = match set
        .render
        .renderers
        .iter_mut()
        .find(|renderer| renderer.entity == entity)
    {
        Some(renderer) => renderer,
        None => return,
    };
    let path = index
        .and_then(|guids| guids.get(&guid))
        .map(Path::to_path_buf);
    if let (Some(path), Some(index)) = (&path, index) {
        let properties = resolve_material_properties(path, index, material_properties);
        apply_material_properties(renderer, &properties);
    }
    renderer.material_guid = Some(guid);
    renderer.material_path = path;
    renderer.material_diagnostic = None;
}

fn apply_script_field_override(
    modification: &prefab::Modification,
    entity: EntityId,
    set: &mut DocumentSet,
) {
    if modification.property_path.contains('.') || modification.property_path.contains('[') {
        return;
    }
    let binding = match set
        .scripts
        .iter_mut()
        .find(|binding| binding.entity == entity)
    {
        Some(binding) => binding,
        None => return,
    };
    let value = match &modification.object_reference {
        Some(reference) if modification.value.is_empty() => reference.to_encoded_string(),
        _ => modification.value.clone(),
    };
    binding
        .fields
        .insert(modification.property_path.clone(), value);
}

fn collect_render(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
    material_properties: &mut HashMap<PathBuf, MaterialProperties>,
) -> Result<RenderBinding, SceneError> {
    let mut binding = RenderBinding::default();
    let mut mesh_filters: HashMap<EntityId, &Node> = HashMap::new();
    let mut components: Vec<(u32, EntityId, &Node)> = Vec::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        let body = match document.class_id {
            CAMERA_CLASS => document.root.get("Camera"),
            MESH_FILTER_CLASS => document.root.get("MeshFilter"),
            MESH_RENDERER_CLASS => document.root.get("MeshRenderer"),
            LIGHT_CLASS => document.root.get("Light"),
            SPRITE_RENDERER_CLASS => document.root.get("SpriteRenderer"),
            _ => None,
        };
        let body = match body {
            Some(body) => body,
            None => continue,
        };
        let game_object_file_id = match reference_id(body.get("m_GameObject"))? {
            Some(file_id) => file_id,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id: ROOT_FILE_ID,
                })
            }
        };
        let record = match game_objects.get(&game_object_file_id) {
            Some(record) => record,
            None => {
                if instance_roots.contains(&game_object_file_id) {
                    continue;
                }
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                });
            }
        };
        if document.class_id == MESH_FILTER_CLASS {
            mesh_filters.insert(record.entity, body);
            continue;
        }
        components.push((document.class_id, record.entity, body));
    }
    for (class_id, entity, body) in components {
        match class_id {
            CAMERA_CLASS => binding.cameras.push(camera_component(entity, body)),
            MESH_RENDERER_CLASS => {
                let mesh_filter = mesh_filters.get(&entity).copied();
                binding.renderers.push(mesh_renderer_component(
                    entity,
                    body,
                    mesh_filter,
                    index,
                    material_properties,
                ));
            }
            LIGHT_CLASS => binding.lights.push(light_component(entity, body)),
            SPRITE_RENDERER_CLASS => {
                binding
                    .sprites
                    .push(sprite_renderer_component(entity, body, index));
            }
            _ => {}
        }
    }
    Ok(binding)
}

fn collect_particles_and_trails(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> (Vec<ParticleBinding>, Vec<TrailBinding>) {
    let mut particles = Vec::new();
    let mut trails = Vec::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        match document.class_id {
            PARTICLE_SYSTEM_CLASS => {
                let binding = match parse_particle_system(document) {
                    Ok(binding) => binding,
                    Err(_) => continue,
                };
                let body = match document.root.get("ParticleSystem") {
                    Some(body) => body,
                    None => continue,
                };
                if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                    particles.push(ParticleBinding { entity, binding });
                }
            }
            TRAIL_RENDERER_CLASS => {
                let body = match document.root.get("TrailRenderer") {
                    Some(body) => body,
                    None => continue,
                };
                if let Some(entity) = component_entity(body, game_objects, instance_roots) {
                    trails.push(TrailBinding {
                        entity,
                        binding: trail_renderer_component(entity, body),
                    });
                }
            }
            _ => {}
        }
    }
    (particles, trails)
}

fn collect_ui(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> UiBinding {
    let mut binding = UiBinding::default();
    let mut collectors = UiCollectors::default();
    let mut rect_entities: HashMap<i64, EntityId> = HashMap::new();
    let mut rect_fathers: HashMap<i64, i64> = HashMap::new();
    let mut rects: Vec<(i64, RectTransformComponent)> = Vec::new();
    for doc in &stream.documents {
        if doc.stripped {
            continue;
        }
        if doc.class_id == RECT_TRANSFORM_CLASS {
            let Some(body) = doc.root.get("RectTransform") else {
                continue;
            };
            let Some(entity) = component_entity(body, game_objects, instance_roots) else {
                continue;
            };
            rect_entities.insert(doc.file_id, entity);
            if let Ok(Some(father)) = reference_id(body.get("m_Father")) {
                rect_fathers.insert(doc.file_id, father);
            }
            rects.push((doc.file_id, parse_rect_transform(entity, body)));
        } else if doc.class_id == CANVAS_CLASS {
            let Some(body) = doc.root.get("Canvas") else {
                continue;
            };
            let Some(entity) = component_entity(body, game_objects, instance_roots) else {
                continue;
            };
            binding.canvases.push(CanvasComponent {
                entity,
                render_mode: canvas_render_mode(body.get("m_RenderMode")),
                sorting_order: body
                    .get("m_SortingOrder")
                    .and_then(Node::as_i64)
                    .unwrap_or(0),
                scaler: None,
            });
        }
    }
    for (file_id, rect) in &mut rects {
        rect.parent = rect_fathers
            .get(file_id)
            .and_then(|f| rect_entities.get(f))
            .copied();
    }
    binding.rect_transforms = rects.into_iter().map(|(_, rect)| rect).collect();
    for doc in &stream.documents {
        if doc.stripped || doc.class_id != MONO_BEHAVIOUR_CLASS {
            continue;
        }
        let Some(body) = doc.root.get("MonoBehaviour") else {
            continue;
        };
        let Some(guid) = body
            .get("m_Script")
            .and_then(|n| n.get("guid"))
            .and_then(Node::as_str)
            .map(str::trim)
        else {
            continue;
        };
        let Some(entity) = component_entity(body, game_objects, instance_roots) else {
            continue;
        };
        collect_ui_mono_behaviour(
            doc.file_id,
            entity,
            guid,
            body,
            index,
            &mut binding,
            &mut collectors,
        );
    }
    for (slot, target) in collectors.button_targets {
        if let Some(entity) = collectors.graphics.get(&target).copied() {
            if let Some(UiElementComponent::Button(button)) = binding.elements.get_mut(slot) {
                button.target_graphic = Some(entity);
            }
        }
    }
    binding
}

/// The graphics and button targets the UI parser collects while it walks the
/// scene's `MonoBehaviour` components.
#[derive(Default)]
struct UiCollectors {
    graphics: HashMap<i64, EntityId>,
    button_targets: Vec<(usize, i64)>,
}

fn parse_rect_transform(entity: EntityId, body: &Node) -> RectTransformComponent {
    RectTransformComponent {
        entity,
        parent: None,
        anchor_min: vector2_or(body.get("m_AnchorMin"), [0.0, 0.0]),
        anchor_max: vector2_or(body.get("m_AnchorMax"), [1.0, 1.0]),
        anchored_position: vector2_or(body.get("m_AnchoredPosition"), [0.0, 0.0]),
        size_delta: vector2_or(body.get("m_SizeDelta"), [0.0, 0.0]),
        pivot: vector2_or(body.get("m_Pivot"), [0.5, 0.5]),
    }
}

fn collect_ui_mono_behaviour(
    file_id: i64,
    entity: EntityId,
    guid: &str,
    body: &Node,
    index: Option<&GuidIndex>,
    binding: &mut UiBinding,
    collectors: &mut UiCollectors,
) {
    match guid {
        IMAGE_SCRIPT_GUID => {
            collectors.graphics.insert(file_id, entity);
            let sprite = body.get("m_Sprite");
            let sprite_guid = sprite_guid(sprite);
            binding
                .elements
                .push(UiElementComponent::Image(UiImageComponent {
                    entity,
                    color: color4_or(body.get("m_Color"), DEFAULT_UI_COLOR),
                    sprite_border: sprite_border(index, sprite_guid.as_deref()),
                    sprite_guid,
                    enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
                    draw_mode: ui_image_draw_mode(body.get("m_Type")),
                }));
        }
        TEXT_SCRIPT_GUID => binding
            .elements
            .push(UiElementComponent::Text(UiTextComponent {
                entity,
                text: body
                    .get("m_Text")
                    .and_then(Node::as_str)
                    .unwrap_or_default()
                    .to_string(),
                color: color4_or(body.get("m_Color"), DEFAULT_UI_COLOR),
                font_size: yaml_f32(body.get("m_FontSize"), DEFAULT_UI_FONT_SIZE, |v| v > 0.0),
                enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
            })),
        BUTTON_SCRIPT_GUID => {
            let index = binding.elements.len();
            binding
                .elements
                .push(UiElementComponent::Button(UiButtonComponent {
                    entity,
                    target_graphic: None,
                    interactable: yaml_bool(body.get("m_Interactable"), DEFAULT_UI_ENABLED),
                }));
            if let Ok(Some(target)) = reference_id(body.get("m_TargetGraphic")) {
                collectors.button_targets.push((index, target));
            }
        }
        CANVAS_SCALER_SCRIPT_GUID => {
            if let Some(canvas) = binding.canvases.iter_mut().find(|c| c.entity == entity) {
                canvas.scaler = Some(CanvasScalerComponent {
                    ui_scale_mode: canvas_scale_mode(body.get("m_UiScaleMode")),
                    reference_resolution: vector2_or(
                        body.get("m_ReferenceResolution"),
                        DEFAULT_REFERENCE_RESOLUTION,
                    ),
                    screen_match_mode: canvas_match_mode(body.get("m_ScreenMatchMode")),
                    match_width_or_height: yaml_f32(
                        body.get("m_MatchWidthOrHeight"),
                        DEFAULT_MATCH_WIDTH_OR_HEIGHT,
                        |v| (0.0..=1.0).contains(&v),
                    ),
                });
            }
        }
        HORIZONTAL_LAYOUT_GROUP_SCRIPT_GUID => binding.layout_groups.push(parse_layout_group(
            entity,
            LayoutGroupAxis::Horizontal,
            body,
        )),
        VERTICAL_LAYOUT_GROUP_SCRIPT_GUID => {
            binding
                .layout_groups
                .push(parse_layout_group(entity, LayoutGroupAxis::Vertical, body));
        }
        CONTENT_SIZE_FITTER_SCRIPT_GUID => binding
            .content_size_fitters
            .push(parse_content_size_fitter(entity, body)),
        _ => {}
    }
}

fn parse_layout_group(
    entity: EntityId,
    axis: LayoutGroupAxis,
    body: &Node,
) -> UiLayoutGroupComponent {
    let padding = body
        .get("m_Padding")
        .map_or_else(UiPadding::default, |n| UiPadding {
            left: yaml_i32(n.get("m_Left"), 0),
            right: yaml_i32(n.get("m_Right"), 0),
            top: yaml_i32(n.get("m_Top"), 0),
            bottom: yaml_i32(n.get("m_Bottom"), 0),
        });
    UiLayoutGroupComponent {
        entity,
        axis,
        padding,
        spacing: yaml_f32(body.get("m_Spacing"), 0.0, f32::is_finite),
        child_alignment: yaml_u32(body.get("m_ChildAlignment"), 0).min(8),
        child_control_width: yaml_bool(body.get("m_ChildControlWidth"), true),
        child_control_height: yaml_bool(body.get("m_ChildControlHeight"), true),
        child_force_expand_width: yaml_bool(body.get("m_ChildForceExpandWidth"), true),
        child_force_expand_height: yaml_bool(body.get("m_ChildForceExpandHeight"), true),
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
    }
}

fn parse_content_size_fitter(entity: EntityId, body: &Node) -> UiContentSizeFitterComponent {
    UiContentSizeFitterComponent {
        entity,
        horizontal_fit: ContentSizeFitMode::from_u32(yaml_u32(body.get("m_HorizontalFit"), 0))
            .unwrap_or(ContentSizeFitMode::Unconstrained),
        vertical_fit: ContentSizeFitMode::from_u32(yaml_u32(body.get("m_VerticalFit"), 0))
            .unwrap_or(ContentSizeFitMode::Unconstrained),
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
    }
}

fn yaml_i32(node: Option<&Node>, default: i32) -> i32 {
    node.and_then(Node::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(default)
}

fn canvas_render_mode(node: Option<&Node>) -> CanvasRenderMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasRenderMode::ScreenSpaceCamera,
        2 => CanvasRenderMode::WorldSpace,
        _ => CanvasRenderMode::ScreenSpaceOverlay,
    }
}

fn canvas_scale_mode(node: Option<&Node>) -> CanvasScaleMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasScaleMode::ScaleWithScreenSize,
        2 => CanvasScaleMode::ConstantPhysicalSize,
        _ => CanvasScaleMode::ConstantPixelSize,
    }
}

fn canvas_match_mode(node: Option<&Node>) -> CanvasMatchMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasMatchMode::Shrink,
        2 => CanvasMatchMode::Expand,
        _ => CanvasMatchMode::MatchWidthOrHeight,
    }
}

fn sprite_guid(node: Option<&Node>) -> Option<AssetGuid> {
    let node = node?;
    let guid = node
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(guid.to_string())
}

fn color4_or(node: Option<&Node>, default: [f32; 4]) -> [f32; 4] {
    let node = match node {
        Some(node) => node,
        None => return default,
    };
    let red = node.get("r").and_then(Node::as_f64);
    let green = node.get("g").and_then(Node::as_f64);
    let blue = node.get("b").and_then(Node::as_f64);
    match (red, green, blue) {
        (Some(red), Some(green), Some(blue)) => {
            let alpha = node
                .get("a")
                .and_then(Node::as_f64)
                .filter(|value| value.is_finite())
                .unwrap_or(1.0);
            [red as f32, green as f32, blue as f32, alpha as f32]
        }
        _ => default,
    }
}

fn vector2_or(node: Option<&Node>, default: [f32; 2]) -> [f32; 2] {
    let node = match node {
        Some(node) => node,
        None => return default,
    };
    let x = node.get("x").and_then(Node::as_f64);
    let y = node.get("y").and_then(Node::as_f64);
    match (x, y) {
        (Some(x), Some(y)) => [x as f32, y as f32],
        _ => default,
    }
}

fn collect_unsupported_components(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> Vec<(EntityId, u32)> {
    let mut unsupported = Vec::new();
    for document in &stream.documents {
        if document.stripped || handled_component_class(document.class_id) {
            continue;
        }
        let body = match document_body(document) {
            Some(body) => body,
            None => continue,
        };
        if let Some(entity) = component_entity(body, game_objects, instance_roots) {
            unsupported.push((entity, document.class_id));
        }
    }
    unsupported
}

fn append_unsupported_class_diagnostics(
    diagnostics: &mut Vec<(EntityId, String)>,
    unsupported: &[(EntityId, u32)],
) {
    let mut reported: HashSet<u32> = HashSet::new();
    for (entity, class_id) in unsupported {
        if !reported.insert(*class_id) {
            continue;
        }
        diagnostics.push((*entity, format!("unsupported component class {class_id}")));
    }
}

fn component_entity(
    body: &Node,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> Option<EntityId> {
    let file_id = match reference_id(body.get("m_GameObject")) {
        Ok(Some(file_id)) => file_id,
        _ => return None,
    };
    if instance_roots.contains(&file_id) {
        return None;
    }
    game_objects.get(&file_id).map(|record| record.entity)
}

fn document_body(document: &Document) -> Option<&Node> {
    let entries = document.root.as_mapping()?;
    let (_, body) = entries.first()?;
    Some(body)
}

fn handled_component_class(class_id: u32) -> bool {
    scene_physics::scene_joints::is_joint_class(class_id)
        || matches!(
            class_id,
            GAME_OBJECT_CLASS
                | TRANSFORM_CLASS
                | CAMERA_CLASS
                | MESH_RENDERER_CLASS
                | MESH_FILTER_CLASS
                | LIGHT_CLASS
                | RIGIDBODY_CLASS
                | BOX_COLLIDER_CLASS
                | SPHERE_COLLIDER_CLASS
                | CAPSULE_COLLIDER_CLASS
                | MESH_COLLIDER_CLASS
                | CHARACTER_CONTROLLER_CLASS
                | MONO_BEHAVIOUR_CLASS
                | AUDIO_SOURCE_CLASS
                | AUDIO_CLIP_CLASS
                | PHYSIC_MATERIAL_CLASS
                | PARTICLE_SYSTEM_CLASS
                | TRAIL_RENDERER_CLASS
                | ANIMATOR_CLASS
                | PREFAB_INSTANCE_CLASS
                | CANVAS_CLASS
                | RECT_TRANSFORM_CLASS
                | SPRITE_RENDERER_CLASS
        )
}

fn camera_component(entity: EntityId, body: &Node) -> CameraComponent {
    CameraComponent {
        entity,
        orthographic: yaml_bool(body.get("orthographic"), false),
        orthographic_size: yaml_f32(
            body.get("orthographic size"),
            DEFAULT_CAMERA_ORTHOGRAPHIC_SIZE,
            |value| value > 0.0,
        ),
        field_of_view: yaml_f32(
            body.get("field of view"),
            DEFAULT_CAMERA_FIELD_OF_VIEW,
            |value| value > 0.0,
        ),
        near_clip_plane: yaml_f32(
            body.get("near clip plane"),
            DEFAULT_CAMERA_NEAR_CLIP_PLANE,
            |value| value >= 0.0,
        ),
        far_clip_plane: yaml_f32(
            body.get("far clip plane"),
            DEFAULT_CAMERA_FAR_CLIP_PLANE,
            |value| value > 0.0,
        ),
        background_color: color3_or(
            body.get("m_BackGroundColor"),
            DEFAULT_CAMERA_BACKGROUND_COLOR,
        ),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MaterialProperties {
    color: [f32; 3],
    albedo_tex_path: Option<PathBuf>,
    normal_tex_path: Option<PathBuf>,
    emission_tex_path: Option<PathBuf>,
    metallic: Option<f32>,
    smoothness: Option<f32>,
    emission_color: Option<[f32; 4]>,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        MaterialProperties {
            color: DEFAULT_MATERIAL_COLOR,
            albedo_tex_path: None,
            normal_tex_path: None,
            emission_tex_path: None,
            metallic: None,
            smoothness: None,
            emission_color: None,
        }
    }
}

impl MaterialProperties {
    fn from_material(material: &Material, index: &GuidIndex) -> MaterialProperties {
        MaterialProperties {
            color: [material.color[0], material.color[1], material.color[2]],
            albedo_tex_path: texture_path(material.main_tex_guid.as_deref(), index),
            normal_tex_path: texture_path(material.bump_tex_guid.as_deref(), index),
            emission_tex_path: texture_path(material.emission_tex_guid.as_deref(), index),
            metallic: material.metallic,
            smoothness: material.glossiness,
            emission_color: material.emission_color,
        }
    }
}

fn texture_path(guid: Option<&str>, index: &GuidIndex) -> Option<PathBuf> {
    guid.and_then(|guid| index.get(guid)).map(Path::to_path_buf)
}

fn apply_material_properties(
    renderer: &mut MeshRendererComponent,
    properties: &MaterialProperties,
) {
    renderer.material_color = properties.color;
    renderer.albedo_tex_path = properties.albedo_tex_path.clone();
    renderer.normal_tex_path = properties.normal_tex_path.clone();
    renderer.emission_tex_path = properties.emission_tex_path.clone();
    renderer.metallic = properties.metallic;
    renderer.smoothness = properties.smoothness;
    renderer.emission_color = properties.emission_color;
}

fn mesh_renderer_component(
    entity: EntityId,
    body: &Node,
    mesh_filter: Option<&Node>,
    index: Option<&GuidIndex>,
    material_properties: &mut HashMap<PathBuf, MaterialProperties>,
) -> MeshRendererComponent {
    let (mesh_guid, mesh_file_id, mesh_path, mesh_diagnostic) = match mesh_filter {
        Some(filter) => resolve_mesh_reference(entity, filter, index),
        None => (
            None,
            None,
            None,
            Some(format!(
                "MeshRenderer on entity {} has no MeshFilter to resolve the mesh reference",
                entity.as_u64()
            )),
        ),
    };
    let (material_guid, material_path, material_diagnostic) =
        resolve_material_reference(entity, body, index);
    let properties = match (&material_path, index) {
        (Some(path), Some(index)) => resolve_material_properties(path, index, material_properties),
        _ => MaterialProperties::default(),
    };
    MeshRendererComponent {
        entity,
        mesh_guid,
        mesh_file_id,
        mesh_path,
        mesh_diagnostic,
        material_guid,
        material_path,
        material_diagnostic,
        material_color: properties.color,
        albedo_tex_path: properties.albedo_tex_path,
        normal_tex_path: properties.normal_tex_path,
        emission_tex_path: properties.emission_tex_path,
        metallic: properties.metallic,
        smoothness: properties.smoothness,
        emission_color: properties.emission_color,
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_RENDERER_ENABLED),
    }
}

fn resolve_mesh_reference(
    entity: EntityId,
    filter: &Node,
    index: Option<&GuidIndex>,
) -> (
    Option<AssetGuid>,
    Option<i64>,
    Option<PathBuf>,
    Option<String>,
) {
    resolve_mesh_reference_for(
        filter,
        index,
        &format!("MeshFilter on entity {}", entity.as_u64()),
    )
}

fn resolve_mesh_reference_for(
    source: &Node,
    index: Option<&GuidIndex>,
    owner: &str,
) -> (
    Option<AssetGuid>,
    Option<i64>,
    Option<PathBuf>,
    Option<String>,
) {
    let mesh_node = match source.get("m_Mesh") {
        Some(node) => node,
        None => {
            return (
                None,
                None,
                None,
                Some(format!("{owner} has no m_Mesh reference")),
            )
        }
    };
    let file_id = mesh_node.get("fileID").and_then(Node::as_i64);
    let guid = mesh_node
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let guid = match guid {
        Some(guid) => guid.to_string(),
        None => {
            return (
                None,
                file_id,
                None,
                Some(format!("{owner} references m_Mesh without a guid")),
            )
        }
    };
    let path = index
        .and_then(|guids| guids.get(&guid))
        .map(Path::to_path_buf);
    (Some(guid), file_id, path, None)
}

fn resolve_material_reference(
    entity: EntityId,
    body: &Node,
    index: Option<&GuidIndex>,
) -> (Option<AssetGuid>, Option<PathBuf>, Option<String>) {
    let entry = body
        .get("m_Materials")
        .and_then(Node::as_sequence)
        .and_then(|items| items.first());
    let entry = match entry {
        Some(entry) => entry,
        None => return (None, None, None),
    };
    let file_id = entry.get("fileID").and_then(Node::as_i64);
    let guid = entry
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let guid = match guid {
        Some(guid) => guid.to_string(),
        None => {
            if file_id == Some(0) {
                return (None, None, None);
            }
            return (
                None,
                None,
                Some(format!(
                    "MeshRenderer on entity {} references material fileID {} without a guid",
                    entity.as_u64(),
                    file_id.unwrap_or(0)
                )),
            );
        }
    };
    let path = index
        .and_then(|guids| guids.get(&guid))
        .map(Path::to_path_buf);
    (Some(guid), path, None)
}

fn resolve_material_properties(
    path: &Path,
    index: &GuidIndex,
    cache: &mut HashMap<PathBuf, MaterialProperties>,
) -> MaterialProperties {
    if let Some(properties) = cache.get(path) {
        return properties.clone();
    }
    let properties = match crate::material::parse_material_with_guids(path, index) {
        Ok(material) => MaterialProperties::from_material(&material, index),
        Err(_) => MaterialProperties::default(),
    };
    cache.insert(path.to_path_buf(), properties.clone());
    properties
}

fn light_component(entity: EntityId, body: &Node) -> LightComponent {
    let light_type = body.get("m_Type").and_then(Node::as_i64).unwrap_or(0);
    let directional = light_type == DIRECTIONAL_LIGHT_TYPE;
    let unsupported_diagnostic = if directional {
        None
    } else {
        Some(format!(
            "{} light on entity {} is not supported by the renderer",
            light_type_name(light_type),
            entity.as_u64()
        ))
    };
    LightComponent {
        entity,
        directional,
        color: color3_or(body.get("m_Color"), DEFAULT_LIGHT_COLOR),
        intensity: yaml_f32(body.get("m_Intensity"), DEFAULT_LIGHT_INTENSITY, |value| {
            value.is_finite()
        }),
        direction: DEFAULT_LIGHT_DIRECTION,
        unsupported_diagnostic,
    }
}

fn sprite_renderer_component(
    entity: EntityId,
    body: &Node,
    index: Option<&GuidIndex>,
) -> SpriteRendererComponent {
    let sprite = body.get("m_Sprite");
    let sprite_file_id = sprite
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64);
    let sprite_guid = sprite
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let sprite_path = sprite_guid
        .as_ref()
        .and_then(|guid| index.and_then(|guids| guids.get(guid)))
        .map(Path::to_path_buf);
    let draw_mode = match body.get("m_DrawMode").and_then(Node::as_i64).unwrap_or(0) {
        1 => SpriteDrawMode::Sliced,
        2 => SpriteDrawMode::Tiled,
        _ => SpriteDrawMode::Simple,
    };
    SpriteRendererComponent {
        entity,
        sprite_guid,
        sprite_file_id,
        sprite_path,
        color: color4_or(body.get("m_Color"), DEFAULT_SPRITE_COLOR),
        sorting_layer: body
            .get("m_SortingLayerID")
            .and_then(Node::as_i64)
            .unwrap_or(0) as i32,
        sorting_order: body
            .get("m_SortingOrder")
            .and_then(Node::as_i64)
            .unwrap_or(0) as i32,
        draw_mode,
        flip_x: yaml_bool(body.get("m_FlipX"), false),
        flip_y: yaml_bool(body.get("m_FlipY"), false),
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_RENDERER_ENABLED),
    }
}

fn light_type_name(light_type: i64) -> &'static str {
    match light_type {
        0 => "spot",
        1 => "directional",
        2 => "point",
        3 => "area",
        4 => "disc",
        _ => "unknown",
    }
}

fn color3_or(node: Option<&Node>, default: [f32; 3]) -> [f32; 3] {
    let node = match node {
        Some(node) => node,
        None => return default,
    };
    let red = node.get("r").and_then(Node::as_f64);
    let green = node.get("g").and_then(Node::as_f64);
    let blue = node.get("b").and_then(Node::as_f64);
    match (red, green, blue) {
        (Some(red), Some(green), Some(blue)) => [red as f32, green as f32, blue as f32],
        _ => default,
    }
}

fn collect_scene_diagnostics(
    physics: &[PhysicsBinding],
    audio_bindings: &[AudioBinding],
    render: &RenderBinding,
) -> Vec<(EntityId, String)> {
    let mut diagnostics = Vec::new();
    for binding in physics {
        for collider in &binding.colliders {
            match collider {
                ColliderComponent::Box(c) => {
                    if let Some(d) = &c.material_diagnostic {
                        diagnostics.push((binding.entity, d.clone()));
                    }
                }
                ColliderComponent::Sphere(c) => {
                    if let Some(d) = &c.material_diagnostic {
                        diagnostics.push((binding.entity, d.clone()));
                    }
                }
                ColliderComponent::Capsule(c) => {
                    for d in [&c.material_diagnostic, &c.geometry_diagnostic]
                        .into_iter()
                        .flatten()
                    {
                        diagnostics.push((binding.entity, d.clone()));
                    }
                }
                ColliderComponent::Mesh(c) => {
                    for d in [&c.material_diagnostic, &c.mesh_diagnostic]
                        .into_iter()
                        .flatten()
                    {
                        diagnostics.push((binding.entity, d.clone()));
                    }
                }
            }
        }
    }
    for (entity, audio) in audio_bindings {
        if let Some(diag) = &audio.clip_diagnostic {
            diagnostics.push((*entity, diag.clone()));
        }
    }
    for renderer in &render.renderers {
        if let Some(diag) = &renderer.mesh_diagnostic {
            diagnostics.push((renderer.entity, diag.clone()));
        }
        if let Some(diag) = &renderer.material_diagnostic {
            diagnostics.push((renderer.entity, diag.clone()));
        }
    }
    for light in &render.lights {
        if let Some(diag) = &light.unsupported_diagnostic {
            diagnostics.push((light.entity, diag.clone()));
        }
    }
    diagnostics
}

fn yaml_u8(node: Option<&Node>, default: u8) -> u8 {
    match node.and_then(Node::as_i64) {
        Some(value) if (0..=255).contains(&value) => u8::try_from(value).unwrap_or(default),
        _ => default,
    }
}

fn yaml_f32(node: Option<&Node>, default: f32, valid: impl Fn(f32) -> bool) -> f32 {
    match node.and_then(Node::as_f64) {
        Some(value) if value.is_finite() => {
            let value = value as f32;
            if value.is_finite() && valid(value) {
                value
            } else {
                default
            }
        }
        _ => default,
    }
}

fn yaml_u32(node: Option<&Node>, default: u32) -> u32 {
    match node.and_then(Node::as_i64) {
        Some(value) if value < 0 => 0,
        Some(value) => u32::try_from(value).unwrap_or(default),
        None => default,
    }
}

fn yaml_bool(node: Option<&Node>, default: bool) -> bool {
    match node.and_then(Node::as_f64) {
        Some(value) => value != 0.0,
        None => match node.and_then(Node::as_str) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        },
    }
}

fn yaml_f64(node: Option<&Node>, default: f64, valid: impl Fn(f64) -> bool) -> f64 {
    match node.and_then(Node::as_f64) {
        Some(value) if value.is_finite() && valid(value) => value,
        _ => default,
    }
}

fn check_external_references(stream: &Stream, index: Option<&GuidIndex>) -> Result<(), SceneError> {
    let index = match index {
        Some(index) => index,
        None => return Ok(()),
    };
    for document in &stream.documents {
        if document.class_id != PREFAB_INSTANCE_CLASS || document.stripped {
            continue;
        }
        let body = match document.root.get("PrefabInstance") {
            Some(body) => body,
            None => continue,
        };
        let source = match body.get("m_SourcePrefab") {
            Some(source) => source,
            None => continue,
        };
        let guid = match source.get("guid").and_then(Node::as_str) {
            Some(guid) => guid,
            None => continue,
        };
        if !index.contains(guid) {
            return Err(SceneError::UnresolvedExternalReference {
                guid: guid.to_string(),
                file_id: source
                    .get("fileID")
                    .and_then(Node::as_i64)
                    .unwrap_or(ROOT_FILE_ID),
            });
        }
    }
    Ok(())
}

fn apply_local_transform(graph: &mut SceneGraph, entity: EntityId, body: &Node) {
    if let Some(position) = body.get("m_LocalPosition").and_then(vector3) {
        if let Some(transform) = graph.transform_mut(entity) {
            transform.translation = position.map(|value| value as f32);
        }
    }
    if let Some(rotation) = body.get("m_LocalRotation").and_then(quaternion4) {
        if let Some(transform) = graph.transform_mut(entity) {
            transform.rotation = rotation.map(|value| value as f32);
        }
    }
    if let Some(scale) = body.get("m_LocalScale").and_then(vector3) {
        if let Some(transform) = graph.transform_mut(entity) {
            transform.scale = scale.map(|value| value as f32);
        }
    }
}

fn order_children(graph: &mut SceneGraph, parent: EntityId, ordered: &[EntityId]) {
    let mut result: Vec<EntityId> = Vec::new();
    for child in ordered {
        if graph.parent(*child) == Some(parent) {
            result.push(*child);
        }
    }
    let extras: Vec<EntityId> = graph
        .children(parent)
        .iter()
        .copied()
        .filter(|child| !result.contains(child))
        .collect();
    result.extend(extras);
    if let Some(transform) = graph.transform_mut(parent) {
        transform.children = result;
    }
}

fn reference_id(node: Option<&Node>) -> Result<Option<i64>, SceneError> {
    let node = match node {
        Some(node) => node,
        None => return Ok(None),
    };
    let value = match node.get("fileID") {
        Some(value) => value,
        None => return Ok(None),
    };
    match value.as_i64() {
        Some(file_id) => Ok(Some(file_id)),
        None => {
            let text = match value.as_scalar() {
                Some(text) => text.to_string(),
                None => String::new(),
            };
            Err(SceneError::InvalidFileId { file_id: text })
        }
    }
}

fn child_ids(node: Option<&Node>) -> Result<Vec<i64>, SceneError> {
    let node = match node {
        Some(node) => node,
        None => return Ok(Vec::new()),
    };
    let items = match node.as_sequence() {
        Some(items) => items,
        None => return Ok(Vec::new()),
    };
    let mut ids = Vec::new();
    for item in items {
        if let Some(file_id) = reference_id(Some(item))? {
            ids.push(file_id);
        }
    }
    Ok(ids)
}

fn vector3(node: &Node) -> Option<[f64; 3]> {
    let x = node.get("x").and_then(Node::as_f64)?;
    let y = node.get("y").and_then(Node::as_f64)?;
    let z = node.get("z").and_then(Node::as_f64)?;
    Some([x, y, z])
}

fn vector3_or(node: Option<&Node>, default: [f64; 3]) -> [f64; 3] {
    match node.and_then(vector3) {
        Some(vector) => vector,
        None => default,
    }
}

fn quaternion4(node: &Node) -> Option<[f64; 4]> {
    let x = node.get("x").and_then(Node::as_f64)?;
    let y = node.get("y").and_then(Node::as_f64)?;
    let z = node.get("z").and_then(Node::as_f64)?;
    let w = node.get("w").and_then(Node::as_f64)?;
    if !x.is_finite() || !y.is_finite() || !z.is_finite() || !w.is_finite() {
        return None;
    }
    let length = (x * x + y * y + z * z + w * w).sqrt();
    if !length.is_finite() || (length - 1.0).abs() > UNIT_QUATERNION_TOLERANCE {
        return None;
    }
    Some([x, y, z, w])
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
