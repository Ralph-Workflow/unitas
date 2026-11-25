use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use unitas_core::{CoreError, EntityId, SceneGraph};

use crate::meta::{GuidIndex, MetaError};
use crate::prefab;
use crate::unity_yaml::{self, Node, Stream, YamlError};

pub const AUDIO_SOURCE_CLASS: u32 = 82;
pub const AUDIO_CLIP_CLASS: u32 = 83;
pub const CAMERA_CLASS: u32 = 20;
pub const MESH_RENDERER_CLASS: u32 = 23;
pub const MESH_FILTER_CLASS: u32 = 33;
pub const LIGHT_CLASS: u32 = 108;
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
const DEFAULT_LIGHT_DIRECTION: [f32; 3] = [0.0, -1.0, 0.0];
const DIRECTIONAL_LIGHT_TYPE: i64 = 1;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialValues {
    pub friction: f64,
    pub restitution: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RigidbodyComponent {
    pub mass: f64,
    pub use_gravity: bool,
    pub is_trigger: bool,
    pub is_kinematic: bool,
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
    pub box_collider: Option<BoxColliderComponent>,
    pub sphere_collider: Option<SphereColliderComponent>,
    pub capsule_collider: Option<CapsuleColliderComponent>,
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
    pub mesh_path: Option<PathBuf>,
    pub mesh_diagnostic: Option<String>,
    pub material_guid: Option<AssetGuid>,
    pub material_path: Option<PathBuf>,
    pub material_diagnostic: Option<String>,
    pub material_color: [f32; 3],
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderBinding {
    pub cameras: Vec<CameraComponent>,
    pub renderers: Vec<MeshRendererComponent>,
    pub lights: Vec<LightComponent>,
}

#[derive(Debug)]
pub struct AssembledScene {
    pub graph: SceneGraph,
    pub scripts: Vec<ScriptBinding>,
    pub physics: Vec<PhysicsBinding>,
    pub audio_bindings: Vec<AudioBinding>,
    pub render: RenderBinding,
    pub diagnostics: Vec<(EntityId, String)>,
}

struct DocumentSet {
    game_objects: GameObjectRecords,
    transforms: HashMap<i64, EntityId>,
    root: Option<EntityId>,
    scripts: Vec<ScriptBinding>,
    physics: Vec<PhysicsBinding>,
    audio_bindings: Vec<AudioBinding>,
    render: RenderBinding,
}

struct PrefabContext<'a> {
    index: Option<&'a GuidIndex>,
    stack: Vec<String>,
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
    };
    let set = assemble_documents(stream, &mut graph, &mut context, expand)?;
    let diagnostics = collect_scene_diagnostics(&set.physics, &set.audio_bindings, &set.render);
    Ok(AssembledScene {
        graph,
        scripts: set.scripts,
        physics: set.physics,
        audio_bindings: set.audio_bindings,
        render: set.render,
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
            MESH_RENDERER_CLASS => document
                .root
                .get("MeshRenderer")
                .and_then(|node| node.get("m_Materials"))
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
        audio_bindings: Vec::new(),
        render: RenderBinding::default(),
    };
    let instance_roots = prefab_instance_roots(stream);
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
    set.audio_bindings = collect_audio(stream, &set.game_objects, &instance_roots, context.index)?;
    set.render = collect_render(stream, &set.game_objects, &instance_roots, context.index)?;
    if expand {
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
                expand_instance(&instance, document.file_id, &mut set, graph, context)?;
            if set.root.is_none() && instance.transform_parent == ROOT_FILE_ID {
                set.root = Some(expanded_root);
            }
        }
    }
    Ok(set)
}

fn expand_instance(
    instance: &prefab::PrefabInstance,
    file_id: i64,
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
    let stream = unity_yaml::parse_file(&path)?;
    context.stack.push(instance.source.guid.clone());
    let assembled = assemble_documents(&stream, graph, context, true);
    context.stack.pop();
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
    prefab::apply_modifications(
        instance,
        &prefab_set.game_objects,
        &prefab_set.transforms,
        graph,
    );
    apply_overridden_bindings(instance, &stream, &mut prefab_set, context.index);
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
    set.audio_bindings.extend(prefab_set.audio_bindings);
    set.render.cameras.extend(prefab_set.render.cameras);
    set.render.renderers.extend(prefab_set.render.renderers);
    set.render.lights.extend(prefab_set.render.lights);
    Ok(root)
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
    instance: &prefab::PrefabInstance,
    stream: &Stream,
    set: &mut DocumentSet,
    index: Option<&GuidIndex>,
) {
    for modification in &instance.modifications {
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
            apply_material_override(modification, entity, set, index);
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
        renderer.material_color = resolve_material_color(path, index);
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

enum PhysicsComponent {
    Rigidbody(RigidbodyComponent),
    BoxCollider(BoxColliderComponent),
    SphereCollider(SphereColliderComponent),
    CapsuleCollider(CapsuleColliderComponent),
    CharacterController(CharacterControllerComponent),
}

fn collect_physics(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Result<Vec<PhysicsBinding>, SceneError> {
    let mut bindings: Vec<PhysicsBinding> = Vec::new();
    let mut positions: HashMap<EntityId, usize> = HashMap::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        let body = match document.class_id {
            RIGIDBODY_CLASS => match document.root.get("Rigidbody") {
                Some(body) => body,
                None => continue,
            },
            BOX_COLLIDER_CLASS => match document.root.get("BoxCollider") {
                Some(body) => body,
                None => continue,
            },
            SPHERE_COLLIDER_CLASS => match document.root.get("SphereCollider") {
                Some(body) => body,
                None => continue,
            },
            CAPSULE_COLLIDER_CLASS => match document.root.get("CapsuleCollider") {
                Some(body) => body,
                None => continue,
            },
            CHARACTER_CONTROLLER_CLASS => match document.root.get("CharacterController") {
                Some(body) => body,
                None => continue,
            },
            MONO_BEHAVIOUR_CLASS => match document.root.get("MonoBehaviour") {
                Some(body) if is_character_controller(body) => body,
                _ => continue,
            },
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
        let component =
            match document.class_id {
                RIGIDBODY_CLASS => PhysicsComponent::Rigidbody(rigidbody_component(body)),
                BOX_COLLIDER_CLASS => {
                    PhysicsComponent::BoxCollider(box_collider_component(body, &record.name, index))
                }
                SPHERE_COLLIDER_CLASS => PhysicsComponent::SphereCollider(
                    sphere_collider_component(body, &record.name, index),
                ),
                CAPSULE_COLLIDER_CLASS => PhysicsComponent::CapsuleCollider(
                    capsule_collider_component(body, &record.name, index),
                ),
                CHARACTER_CONTROLLER_CLASS | MONO_BEHAVIOUR_CLASS => {
                    PhysicsComponent::CharacterController(character_controller_component(body))
                }
                _ => continue,
            };
        let entity = record.entity;
        let layer = record.layer;
        let position = match positions.get(&entity) {
            Some(position) => *position,
            None => {
                bindings.push(PhysicsBinding {
                    entity,
                    rigidbody: None,
                    box_collider: None,
                    sphere_collider: None,
                    capsule_collider: None,
                    character_controller: None,
                    layer,
                });
                let position = bindings.len() - 1;
                positions.insert(entity, position);
                position
            }
        };
        let binding = &mut bindings[position];
        match component {
            PhysicsComponent::Rigidbody(value) => binding.rigidbody = Some(value),
            PhysicsComponent::BoxCollider(value) => binding.box_collider = Some(value),
            PhysicsComponent::SphereCollider(value) => binding.sphere_collider = Some(value),
            PhysicsComponent::CapsuleCollider(value) => binding.capsule_collider = Some(value),
            PhysicsComponent::CharacterController(value) => {
                binding.character_controller = Some(value);
            }
        }
    }
    Ok(bindings)
}

fn rigidbody_component(body: &Node) -> RigidbodyComponent {
    RigidbodyComponent {
        mass: body.get("m_Mass").and_then(Node::as_f64).unwrap_or(1.0),
        use_gravity: yaml_bool(body.get("m_UseGravity"), true),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        is_kinematic: yaml_bool(body.get("m_IsKinematic"), false),
    }
}

fn box_collider_component(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> BoxColliderComponent {
    let (material, material_diagnostic) =
        resolve_collider_material(body, "BoxCollider", game_object_name, index);
    BoxColliderComponent {
        size: vector3_or(body.get("m_Size"), [1.0, 1.0, 1.0]),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        material,
        material_diagnostic,
    }
}

fn sphere_collider_component(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> SphereColliderComponent {
    let (material, material_diagnostic) =
        resolve_collider_material(body, "SphereCollider", game_object_name, index);
    SphereColliderComponent {
        radius: body.get("m_Radius").and_then(Node::as_f64).unwrap_or(0.5),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        material,
        material_diagnostic,
    }
}

fn capsule_collider_component(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> CapsuleColliderComponent {
    let (material, material_diagnostic) =
        resolve_collider_material(body, "CapsuleCollider", game_object_name, index);
    let mut geometry_notes: Vec<String> = Vec::new();
    let radius = match body.get("m_Radius") {
        Some(node) => match node.as_f64() {
            Some(value) if value.is_finite() && value > 0.0 => value,
            Some(value) => {
                geometry_notes.push(format!(
                    "invalid m_Radius {value} clamped to {DEFAULT_CAPSULE_RADIUS}"
                ));
                DEFAULT_CAPSULE_RADIUS
            }
            None => {
                geometry_notes.push(format!(
                    "invalid m_Radius clamped to {DEFAULT_CAPSULE_RADIUS}"
                ));
                DEFAULT_CAPSULE_RADIUS
            }
        },
        None => DEFAULT_CAPSULE_RADIUS,
    };
    let raw_height = match body.get("m_Height") {
        Some(node) => match node.as_f64() {
            Some(value) if value.is_finite() && value > 0.0 => value,
            Some(value) => {
                let fallback = DEFAULT_CAPSULE_HEIGHT.max(2.0 * radius);
                geometry_notes.push(format!("invalid m_Height {value} clamped to {fallback}"));
                fallback
            }
            None => {
                let fallback = DEFAULT_CAPSULE_HEIGHT.max(2.0 * radius);
                geometry_notes.push(format!("invalid m_Height clamped to {fallback}"));
                fallback
            }
        },
        None => DEFAULT_CAPSULE_HEIGHT,
    };
    let radius = if raw_height < 2.0 * radius {
        let clamped_radius = raw_height / 2.0;
        geometry_notes.push(format!(
            "m_Radius {radius} exceeds half of m_Height {raw_height}; clamped to {clamped_radius}"
        ));
        clamped_radius
    } else {
        radius
    };
    let direction = match body.get("m_Direction") {
        Some(node) => match node.as_i64() {
            Some(0) => 0,
            Some(1) => 1,
            Some(2) => 2,
            Some(value) => {
                geometry_notes.push(format!(
                    "invalid m_Direction {value} defaulted to {DEFAULT_CAPSULE_DIRECTION}"
                ));
                DEFAULT_CAPSULE_DIRECTION
            }
            None => {
                geometry_notes.push(format!(
                    "invalid m_Direction defaulted to {DEFAULT_CAPSULE_DIRECTION}"
                ));
                DEFAULT_CAPSULE_DIRECTION
            }
        },
        None => DEFAULT_CAPSULE_DIRECTION,
    };
    let geometry_diagnostic = if geometry_notes.is_empty() {
        None
    } else {
        Some(format!(
            "CapsuleCollider on '{game_object_name}': {}",
            geometry_notes.join("; ")
        ))
    };
    CapsuleColliderComponent {
        radius,
        height: raw_height,
        direction,
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        material,
        material_diagnostic,
        geometry_diagnostic,
    }
}

fn is_character_controller(body: &Node) -> bool {
    match script_name(body) {
        Some(name) => name == CHARACTER_CONTROLLER_SCRIPT_NAME,
        None => CHARACTER_CONTROLLER_FIELD_KEYS
            .iter()
            .all(|key| body.get(key).is_some()),
    }
}

fn script_name(body: &Node) -> Option<&str> {
    body.get("m_EditorClassIdentifier")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.rsplit("::").next().unwrap_or(value))
}

fn character_controller_component(body: &Node) -> CharacterControllerComponent {
    CharacterControllerComponent {
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_CHARACTER_CONTROLLER_ENABLED),
        radius: yaml_f64(
            body.get("m_Radius"),
            DEFAULT_CHARACTER_CONTROLLER_RADIUS,
            |value| value > 0.0,
        ),
        height: yaml_f64(
            body.get("m_Height"),
            DEFAULT_CHARACTER_CONTROLLER_HEIGHT,
            |value| value > 0.0,
        ),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        slope_limit: yaml_f64(
            body.get("m_SlopeLimit"),
            DEFAULT_CHARACTER_CONTROLLER_SLOPE_LIMIT,
            |value| (0.0..=90.0).contains(&value),
        ),
        step_offset: yaml_f64(
            body.get("m_StepOffset"),
            DEFAULT_CHARACTER_CONTROLLER_STEP_OFFSET,
            |value| value >= 0.0,
        ),
        skin_width: yaml_f64(
            body.get("m_SkinWidth"),
            DEFAULT_CHARACTER_CONTROLLER_SKIN_WIDTH,
            |value| value >= 0.0,
        ),
        min_move_distance: yaml_f64(
            body.get("m_MinMoveDistance"),
            DEFAULT_CHARACTER_CONTROLLER_MIN_MOVE_DISTANCE,
            |value| value >= 0.0,
        ),
    }
}

fn resolve_collider_material(
    body: &Node,
    collider_kind: &str,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> (Option<MaterialValues>, Option<String>) {
    let material_node = match body.get("m_Material") {
        Some(node) => node,
        None => return (None, None),
    };
    let file_id = material_node.get("fileID").and_then(Node::as_i64);
    let guid = material_node
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let guid = match (file_id, guid) {
        (Some(0) | None, None) => return (None, None),
        (Some(id), None) => {
            return (
                None,
                Some(format!(
                    "{collider_kind} on '{game_object_name}' references m_Material fileID {id} without a guid"
                )),
            )
        }
        (None | Some(_), Some(guid)) => guid,
    };

    let path = match index.and_then(|guids| guids.get(guid)) {
        Some(path) => path,
        None => {
            return (
                None,
                Some(format!(
                    "{collider_kind} on '{game_object_name}' references unresolved PhysicMaterial guid {guid}"
                )),
            )
        }
    };

    match load_physic_material(path) {
        Ok(values) => (Some(values), None),
        Err(reason) => (
            None,
            Some(format!(
                "{collider_kind} on '{game_object_name}' PhysicMaterial guid {guid} ({}): {reason}",
                path.display()
            )),
        ),
    }
}

fn load_physic_material(path: &Path) -> Result<MaterialValues, String> {
    let stream = unity_yaml::parse_file(path).map_err(|error| error.to_string())?;
    let body = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| {
            if document.class_id == PHYSIC_MATERIAL_CLASS {
                document.root.get("PhysicMaterial")
            } else {
                None
            }
        })
        .or_else(|| {
            stream
                .documents
                .iter()
                .filter(|document| !document.stripped)
                .find_map(|document| document.root.get("PhysicMaterial"))
        })
        .ok_or_else(|| String::from("missing PhysicMaterial document"))?;

    let friction = match body.get("m_DynamicFriction") {
        Some(node) => node
            .as_f64()
            .ok_or_else(|| String::from("invalid m_DynamicFriction"))?,
        None => DEFAULT_PHYSIC_MATERIAL_FRICTION,
    };
    if !friction.is_finite() || friction < 0.0 {
        return Err(format!("invalid m_DynamicFriction {friction}"));
    }

    let restitution = match body.get("m_Bounciness") {
        Some(node) => node
            .as_f64()
            .ok_or_else(|| String::from("invalid m_Bounciness"))?,
        None => DEFAULT_PHYSIC_MATERIAL_RESTITUTION,
    };
    if !restitution.is_finite() || restitution < 0.0 {
        return Err(format!("invalid m_Bounciness {restitution}"));
    }

    Ok(MaterialValues {
        friction,
        restitution,
    })
}

fn collect_audio(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Result<Vec<AudioBinding>, SceneError> {
    let mut bindings = Vec::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        if document.class_id == AUDIO_CLIP_CLASS {
            continue;
        }
        if document.class_id != AUDIO_SOURCE_CLASS {
            continue;
        }
        let body = match document.root.get("AudioSource") {
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
        let component = audio_source_component(body, &record.name, index);
        bindings.push((record.entity, component));
    }
    Ok(bindings)
}

fn audio_source_component(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> AudioSourceComponent {
    let (clip_guid, clip_path, clip_diagnostic) = resolve_audio_clip(body, game_object_name, index);
    let volume = yaml_f32(body.get("m_Volume"), DEFAULT_AUDIO_VOLUME, |value| {
        (0.0..=1.0).contains(&value)
    });
    let pitch = yaml_f32(body.get("m_Pitch"), DEFAULT_AUDIO_PITCH, |value| {
        value.is_finite()
    });
    let looping = yaml_bool(
        body.get("m_Loop").or_else(|| body.get("Loop")),
        DEFAULT_AUDIO_LOOPING,
    );
    let play_on_awake = yaml_bool(body.get("m_PlayOnAwake"), DEFAULT_AUDIO_PLAY_ON_AWAKE);
    let priority = yaml_u8(
        body.get("m_Priority").or_else(|| body.get("Priority")),
        DEFAULT_AUDIO_PRIORITY,
    );
    let spatial_blend_node = body.get("m_SpatialBlend").or_else(|| {
        body.get("panLevelCustomCurve").and_then(|curve| {
            curve
                .get("m_Curve")
                .and_then(Node::as_sequence)
                .and_then(|seq| seq.first())
                .and_then(|point| point.get("value"))
                .or(Some(curve))
        })
    });
    let spatial_blend = yaml_f32(spatial_blend_node, DEFAULT_AUDIO_SPATIAL_BLEND, |value| {
        (0.0..=1.0).contains(&value)
    });
    let min_distance = yaml_f32(
        body.get("m_MinDistance")
            .or_else(|| body.get("MinDistance")),
        DEFAULT_AUDIO_MIN_DISTANCE,
        |value| value >= 0.0,
    );
    let max_distance = yaml_f32(
        body.get("m_MaxDistance")
            .or_else(|| body.get("MaxDistance")),
        DEFAULT_AUDIO_MAX_DISTANCE,
        |value| value > 0.0,
    );
    let mute = yaml_bool(
        body.get("m_Mute").or_else(|| body.get("Mute")),
        DEFAULT_AUDIO_MUTE,
    );
    AudioSourceComponent {
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_AUDIO_ENABLED),
        clip_guid,
        clip_path,
        clip_diagnostic,
        volume,
        pitch,
        looping,
        play_on_awake,
        priority,
        spatial_blend,
        min_distance,
        max_distance,
        mute,
    }
}

fn resolve_audio_clip(
    body: &Node,
    game_object_name: &str,
    index: Option<&GuidIndex>,
) -> (Option<AssetGuid>, Option<PathBuf>, Option<String>) {
    let clip_node = match body.get("m_audioClip").or_else(|| body.get("m_AudioClip")) {
        Some(node) => node,
        None => return (None, None, None),
    };
    let file_id = clip_node.get("fileID").and_then(Node::as_i64);
    let guid = clip_node
        .get("guid")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let guid = match (file_id, guid) {
        (Some(0) | None, None) => return (None, None, None),
        (Some(id), None) => {
            return (
                None,
                None,
                Some(format!(
                    "AudioSource on '{game_object_name}' references m_audioClip fileID {id} without a guid"
                )),
            )
        }
        (None | Some(_), Some(guid)) => guid.to_string(),
    };

    let path = match index.and_then(|guids| guids.get(&guid)) {
        Some(path) => path.to_path_buf(),
        None => {
            return (
                Some(guid.clone()),
                None,
                Some(format!(
                "AudioSource on '{game_object_name}' references unresolved AudioClip guid {guid}"
            )),
            )
        }
    };

    let supported = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    if !supported {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        return (
            Some(guid.clone()),
            Some(path.clone()),
            Some(format!(
                "AudioSource on '{game_object_name}' AudioClip guid {guid} ({}): unsupported audio format .{ext} (only .wav is supported)",
                path.display()
            )),
        );
    }

    (Some(guid), Some(path), None)
}

fn collect_render(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
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
                binding
                    .renderers
                    .push(mesh_renderer_component(entity, body, mesh_filter, index));
            }
            LIGHT_CLASS => binding.lights.push(light_component(entity, body)),
            _ => {}
        }
    }
    Ok(binding)
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

fn mesh_renderer_component(
    entity: EntityId,
    body: &Node,
    mesh_filter: Option<&Node>,
    index: Option<&GuidIndex>,
) -> MeshRendererComponent {
    let (mesh_guid, mesh_path, mesh_diagnostic) = match mesh_filter {
        Some(filter) => resolve_mesh_reference(entity, filter, index),
        None => (
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
    let material_color = match (&material_path, index) {
        (Some(path), Some(index)) => resolve_material_color(path, index),
        _ => DEFAULT_MATERIAL_COLOR,
    };
    MeshRendererComponent {
        entity,
        mesh_guid,
        mesh_path,
        mesh_diagnostic,
        material_guid,
        material_path,
        material_diagnostic,
        material_color,
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_RENDERER_ENABLED),
    }
}

fn resolve_mesh_reference(
    entity: EntityId,
    filter: &Node,
    index: Option<&GuidIndex>,
) -> (Option<AssetGuid>, Option<PathBuf>, Option<String>) {
    let mesh_node = match filter.get("m_Mesh") {
        Some(node) => node,
        None => {
            return (
                None,
                None,
                Some(format!(
                    "MeshFilter on entity {} has no m_Mesh reference",
                    entity.as_u64()
                )),
            )
        }
    };
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
                None,
                Some(format!(
                    "MeshFilter on entity {} references m_Mesh without a guid",
                    entity.as_u64()
                )),
            )
        }
    };
    let path = index
        .and_then(|guids| guids.get(&guid))
        .map(Path::to_path_buf);
    (Some(guid), path, None)
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

fn resolve_material_color(path: &Path, index: &GuidIndex) -> [f32; 3] {
    match crate::material::parse_material_with_guids(path, index) {
        Ok(material) => [material.color[0], material.color[1], material.color[2]],
        Err(_) => DEFAULT_MATERIAL_COLOR,
    }
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
        if let Some(sphere) = &binding.sphere_collider {
            if let Some(diag) = &sphere.material_diagnostic {
                diagnostics.push((binding.entity, diag.clone()));
            }
        }
        if let Some(collider) = &binding.box_collider {
            if let Some(diag) = &collider.material_diagnostic {
                diagnostics.push((binding.entity, diag.clone()));
            }
        }
        if let Some(capsule) = &binding.capsule_collider {
            if let Some(diag) = &capsule.material_diagnostic {
                diagnostics.push((binding.entity, diag.clone()));
            }
            if let Some(diag) = &capsule.geometry_diagnostic {
                diagnostics.push((binding.entity, diag.clone()));
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
