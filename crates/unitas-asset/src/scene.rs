use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use unitas_core::{CoreError, EntityId, SceneGraph};

use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Node, Stream, YamlError};

const GAME_OBJECT_CLASS: u32 = 1;
const TRANSFORM_CLASS: u32 = 4;
const RIGIDBODY_CLASS: u32 = 54;
const BOX_COLLIDER_CLASS: u32 = 65;
const MONO_BEHAVIOUR_CLASS: u32 = 114;
const SPHERE_COLLIDER_CLASS: u32 = 135;
const PREFAB_INSTANCE_CLASS: u32 = 1001;
const ROOT_FILE_ID: i64 = 0;

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

struct TransformRecord {
    file_id: i64,
    entity: EntityId,
    father: Option<i64>,
    children: Vec<i64>,
}

type GameObjectEntities = HashMap<i64, EntityId>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBinding {
    pub entity: EntityId,
    pub guid: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RigidbodyComponent {
    pub mass: f64,
    pub use_gravity: bool,
    pub is_trigger: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoxColliderComponent {
    pub size: [f64; 3],
    pub center: [f64; 3],
    pub is_trigger: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SphereColliderComponent {
    pub radius: f64,
    pub center: [f64; 3],
    pub is_trigger: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsBinding {
    pub entity: EntityId,
    pub rigidbody: Option<RigidbodyComponent>,
    pub box_collider: Option<BoxColliderComponent>,
    pub sphere_collider: Option<SphereColliderComponent>,
}

pub fn assemble_scene(stream: &Stream) -> Result<SceneGraph, SceneError> {
    assemble(stream, None).map(|(graph, _, _)| graph)
}

pub fn assemble_scene_with_guids(
    stream: &Stream,
    index: &GuidIndex,
) -> Result<SceneGraph, SceneError> {
    assemble(stream, Some(index)).map(|(graph, _, _)| graph)
}

pub fn assemble_scene_file(path: &Path) -> Result<SceneGraph, SceneError> {
    let stream = unity_yaml::parse_file(path)?;
    assemble_scene(&stream)
}

pub fn assemble_scene_file_with_guids(
    path: &Path,
    index: &GuidIndex,
) -> Result<SceneGraph, SceneError> {
    let stream = unity_yaml::parse_file(path)?;
    assemble_scene_with_guids(&stream, index)
}

pub fn assemble_scene_with_physics(
    stream: &Stream,
) -> Result<(SceneGraph, Vec<ScriptBinding>, Vec<PhysicsBinding>), SceneError> {
    let (graph, scripts, game_objects) = assemble(stream, None)?;
    let physics = collect_physics(stream, &game_objects)?;
    Ok((graph, scripts, physics))
}

pub fn assemble_scene_file_with_physics(
    path: &Path,
) -> Result<(SceneGraph, Vec<ScriptBinding>, Vec<PhysicsBinding>), SceneError> {
    let stream = unity_yaml::parse_file(path)?;
    assemble_scene_with_physics(&stream)
}

fn assemble(
    stream: &Stream,
    index: Option<&GuidIndex>,
) -> Result<(SceneGraph, Vec<ScriptBinding>, GameObjectEntities), SceneError> {
    check_external_references(stream, index)?;
    let mut graph = SceneGraph::new();
    let mut game_objects: GameObjectEntities = HashMap::new();
    for document in &stream.documents {
        if document.class_id != GAME_OBJECT_CLASS || document.stripped {
            continue;
        }
        let name = match document
            .root
            .get("GameObject")
            .and_then(|body| body.get("m_Name"))
            .and_then(Node::as_str)
        {
            Some(name) => name.to_string(),
            None => String::from("GameObject"),
        };
        game_objects.insert(document.file_id, graph.add_entity(name));
    }
    let mut transforms: HashMap<i64, EntityId> = HashMap::new();
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
        let entity = match game_objects.get(&game_object_file_id) {
            Some(entity) => *entity,
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
        apply_local_transform(&mut graph, entity, body);
        transforms.insert(document.file_id, entity);
        records.push(TransformRecord { file_id: document.file_id, entity, father, children });
    }
    for record in &records {
        if let Some(father_file_id) = record.father {
            let parent = match transforms.get(&father_file_id) {
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
            if !transforms.contains_key(child_file_id) {
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
            .filter_map(|file_id| transforms.get(file_id).copied())
            .collect();
        order_children(&mut graph, record.entity, &ordered);
    }
    let mut scripts: Vec<ScriptBinding> = Vec::new();
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
        let entity = match game_objects.get(&game_object_file_id) {
            Some(entity) => *entity,
            None => {
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
        scripts.push(ScriptBinding { entity, guid });
    }
    Ok((graph, scripts, game_objects))
}

enum PhysicsComponent {
    Rigidbody(RigidbodyComponent),
    BoxCollider(BoxColliderComponent),
    SphereCollider(SphereColliderComponent),
}

fn collect_physics(
    stream: &Stream,
    game_objects: &GameObjectEntities,
) -> Result<Vec<PhysicsBinding>, SceneError> {
    let mut bindings: Vec<PhysicsBinding> = Vec::new();
    let mut positions: HashMap<EntityId, usize> = HashMap::new();
    for document in &stream.documents {
        if document.stripped {
            continue;
        }
        let (body, component) = match document.class_id {
            RIGIDBODY_CLASS => match document.root.get("Rigidbody") {
                Some(body) => (body, PhysicsComponent::Rigidbody(rigidbody_component(body))),
                None => continue,
            },
            BOX_COLLIDER_CLASS => match document.root.get("BoxCollider") {
                Some(body) => (body, PhysicsComponent::BoxCollider(box_collider_component(body))),
                None => continue,
            },
            SPHERE_COLLIDER_CLASS => match document.root.get("SphereCollider") {
                Some(body) => (body, PhysicsComponent::SphereCollider(sphere_collider_component(body))),
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
        let entity = match game_objects.get(&game_object_file_id) {
            Some(entity) => *entity,
            None => {
                return Err(SceneError::MissingGameObject {
                    transform_file_id: document.file_id,
                    game_object_file_id,
                })
            }
        };
        let position = match positions.get(&entity) {
            Some(position) => *position,
            None => {
                bindings.push(PhysicsBinding {
                    entity,
                    rigidbody: None,
                    box_collider: None,
                    sphere_collider: None,
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
        }
    }
    Ok(bindings)
}

fn rigidbody_component(body: &Node) -> RigidbodyComponent {
    RigidbodyComponent {
        mass: body.get("m_Mass").and_then(Node::as_f64).unwrap_or(1.0),
        use_gravity: yaml_bool(body.get("m_UseGravity"), true),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
    }
}

fn box_collider_component(body: &Node) -> BoxColliderComponent {
    BoxColliderComponent {
        size: vector3_or(body.get("m_Size"), [1.0, 1.0, 1.0]),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
    }
}

fn sphere_collider_component(body: &Node) -> SphereColliderComponent {
    SphereColliderComponent {
        radius: body.get("m_Radius").and_then(Node::as_f64).unwrap_or(0.5),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
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
