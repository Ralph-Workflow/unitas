use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use unitas_core::{CoreError, EntityId, SceneGraph};

use crate::meta::{GuidIndex, MetaError};
use crate::prefab;
use crate::unity_yaml::{self, Node, Stream, YamlError};

const GAME_OBJECT_CLASS: u32 = 1;
const TRANSFORM_CLASS: u32 = 4;
const RIGIDBODY_CLASS: u32 = 54;
const BOX_COLLIDER_CLASS: u32 = 65;
const MONO_BEHAVIOUR_CLASS: u32 = 114;
const PHYSIC_MATERIAL_CLASS: u32 = 134;
const SPHERE_COLLIDER_CLASS: u32 = 135;
const CAPSULE_COLLIDER_CLASS: u32 = 136;
const PREFAB_INSTANCE_CLASS: u32 = 1001;
const ROOT_FILE_ID: i64 = 0;
const UNIT_QUATERNION_TOLERANCE: f64 = 1e-3;
const DEFAULT_PHYSIC_MATERIAL_FRICTION: f64 = 0.6;
const DEFAULT_PHYSIC_MATERIAL_RESTITUTION: f64 = 0.0;
const DEFAULT_CAPSULE_RADIUS: f64 = 0.5;
const DEFAULT_CAPSULE_HEIGHT: f64 = 2.0;
const DEFAULT_CAPSULE_DIRECTION: u32 = 1;

#[derive(Debug)]
pub enum SceneError {
    MissingGameObject {
        transform_file_id: i64,
        game_object_file_id: i64,
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
            SceneError::MissingGameObject { transform_file_id, game_object_file_id } => write!(
                formatter,
                "transform {transform_file_id} references unknown game object {game_object_file_id}"
            ),
            SceneError::DuplicateTransformOwner { game_object_file_id } => {
                write!(formatter, "more than one transform owns game object {game_object_file_id}")
            }
            SceneError::MissingFather { transform_file_id, father_file_id } => write!(
                formatter,
                "transform {transform_file_id} references unknown father {father_file_id}"
            ),
            SceneError::MissingChild { transform_file_id, child_file_id } => write!(
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
                write!(formatter, "prefab instance {file_id} has no source prefab reference")
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
pub struct PhysicsBinding {
    pub entity: EntityId,
    pub rigidbody: Option<RigidbodyComponent>,
    pub box_collider: Option<BoxColliderComponent>,
    pub sphere_collider: Option<SphereColliderComponent>,
    pub capsule_collider: Option<CapsuleColliderComponent>,
    pub layer: u32,
}

struct DocumentSet {
    game_objects: GameObjectRecords,
    transforms: HashMap<i64, EntityId>,
    root: Option<EntityId>,
    scripts: Vec<ScriptBinding>,
    physics: Vec<PhysicsBinding>,
}

struct PrefabContext<'a> {
    index: Option<&'a GuidIndex>,
    stack: Vec<String>,
}

struct Assembly {
    graph: SceneGraph,
    scripts: Vec<ScriptBinding>,
    physics: Vec<PhysicsBinding>,
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

fn assemble_stream(
    stream: &Stream,
    index: Option<&GuidIndex>,
    expand: bool,
) -> Result<Assembly, SceneError> {
    if !expand {
        check_external_references(stream, index)?;
    }
    let mut graph = SceneGraph::new();
    let mut context = PrefabContext { index, stack: Vec::new() };
    let set = assemble_documents(stream, &mut graph, &mut context, expand)?;
    Ok(Assembly { graph, scripts: set.scripts, physics: set.physics })
}

fn assemble_file(path: &Path, index: Option<&GuidIndex>) -> Result<Assembly, SceneError> {
    let stream = unity_yaml::parse_file(path)?;
    let expand = has_prefab_instances(&stream);
    let resolve_guids = expand || has_material_references(&stream);
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

fn assemble_documents(
    stream: &Stream,
    graph: &mut SceneGraph,
    context: &mut PrefabContext,
    expand: bool,
) -> Result<DocumentSet, SceneError> {
    let mut set = DocumentSet {
        game_objects: HashMap::new(),
        transforms: HashMap::new(),
        root: None,
        scripts: Vec::new(),
        physics: Vec::new(),
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
        set.game_objects
            .insert(document.file_id, GameObjectRecord { entity, name, layer });
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
            return Err(SceneError::DuplicateTransformOwner { game_object_file_id });
        }
        owners.insert(game_object_file_id, document.file_id);
        let father = match reference_id(body.get("m_Father"))? {
            Some(file_id) if file_id != ROOT_FILE_ID => Some(file_id),
            _ => None,
        };
        let children = child_ids(body.get("m_Children"))?;
        apply_local_transform(graph, entity, body);
        set.transforms.insert(document.file_id, entity);
        records.push(TransformRecord { file_id: document.file_id, entity, father, children });
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
        let entity = match set.game_objects.get(&game_object_file_id) {
            Some(record) => record.entity,
            None => {
                if instance_roots.contains(&game_object_file_id) {
                    continue;
                }
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                })
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
        set.scripts.push(ScriptBinding { entity, guid });
    }
    set.physics = collect_physics(stream, &set.game_objects, &instance_roots, context.index)?;
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
                    return Err(SceneError::MissingPrefabSource { file_id: document.file_id })
                }
            };
            expand_instance(&instance, document.file_id, &mut set, graph, context)?;
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
) -> Result<(), SceneError> {
    let path = match context.index.and_then(|index| index.get(&instance.source.guid)) {
        Some(path) => path.to_path_buf(),
        None => {
            return Err(SceneError::UnresolvedExternalReference {
                guid: instance.source.guid.clone(),
                file_id: instance.source.file_id,
            })
        }
    };
    if context.stack.contains(&instance.source.guid) {
        return Err(SceneError::CyclicPrefabReference { guid: instance.source.guid.clone() });
    }
    let stream = unity_yaml::parse_file(&path)?;
    context.stack.push(instance.source.guid.clone());
    let assembled = assemble_documents(&stream, graph, context, true);
    context.stack.pop();
    let prefab_set = assembled?;
    let root = match prefab_set.root {
        Some(root) => root,
        None => {
            return Err(SceneError::PrefabWithoutRoot { guid: instance.source.guid.clone() })
        }
    };
    prefab::apply_modifications(
        instance,
        &prefab_set.game_objects,
        &prefab_set.transforms,
        graph,
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
    Ok(())
}

enum PhysicsComponent {
    Rigidbody(RigidbodyComponent),
    BoxCollider(BoxColliderComponent),
    SphereCollider(SphereColliderComponent),
    CapsuleCollider(CapsuleColliderComponent),
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
                })
            }
        };
        let component = match document.class_id {
            RIGIDBODY_CLASS => PhysicsComponent::Rigidbody(rigidbody_component(body)),
            BOX_COLLIDER_CLASS => {
                PhysicsComponent::BoxCollider(box_collider_component(body, &record.name, index))
            }
            SPHERE_COLLIDER_CLASS => PhysicsComponent::SphereCollider(sphere_collider_component(
                body,
                &record.name,
                index,
            )),
            CAPSULE_COLLIDER_CLASS => PhysicsComponent::CapsuleCollider(
                capsule_collider_component(body, &record.name, index),
            ),
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
        (None, Some(guid)) | (Some(_), Some(guid)) => guid,
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
    if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
        return Err(format!("invalid m_Bounciness {restitution}"));
    }

    Ok(MaterialValues {
        friction,
        restitution,
    })
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
                file_id: source.get("fileID").and_then(Node::as_i64).unwrap_or(ROOT_FILE_ID),
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
