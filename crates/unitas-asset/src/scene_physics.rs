use std::collections::{HashMap, HashSet};
use std::path::Path;

#[path = "scene_joints.rs"]
pub mod scene_joints;

use unitas_core::EntityId;

use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Node, Stream};

use super::{
    reference_id, resolve_mesh_reference_for, vector3_or, yaml_bool, yaml_f64, yaml_u32,
    BounceCombine, BoxColliderComponent, CapsuleColliderComponent, CharacterControllerComponent,
    ColliderComponent, FrictionCombine, GameObjectRecords, MaterialValues, MeshColliderComponent,
    PhysicsBinding, RigidbodyCollisionDetection, RigidbodyComponent, RigidbodyInterpolation,
    SceneError, SphereColliderComponent, BOX_COLLIDER_CLASS, CAPSULE_COLLIDER_CLASS,
    CHARACTER_CONTROLLER_CLASS, CHARACTER_CONTROLLER_FIELD_KEYS, CHARACTER_CONTROLLER_SCRIPT_NAME,
    DEFAULT_CAPSULE_DIRECTION, DEFAULT_CAPSULE_HEIGHT, DEFAULT_CAPSULE_RADIUS,
    DEFAULT_CHARACTER_CONTROLLER_ENABLED, DEFAULT_CHARACTER_CONTROLLER_HEIGHT,
    DEFAULT_CHARACTER_CONTROLLER_MIN_MOVE_DISTANCE, DEFAULT_CHARACTER_CONTROLLER_RADIUS,
    DEFAULT_CHARACTER_CONTROLLER_SKIN_WIDTH, DEFAULT_CHARACTER_CONTROLLER_SLOPE_LIMIT,
    DEFAULT_CHARACTER_CONTROLLER_STEP_OFFSET, DEFAULT_PHYSIC_MATERIAL_FRICTION,
    DEFAULT_PHYSIC_MATERIAL_RESTITUTION, DEFAULT_RIGIDBODY_ANGULAR_DRAG, DEFAULT_RIGIDBODY_DRAG,
    DEFAULT_RIGIDBODY_MAX_ANGULAR_VELOCITY, DEFAULT_RIGIDBODY_MAX_DEPENETRATION_VELOCITY,
    DEFAULT_RIGIDBODY_SLEEP_ANGULAR_THRESHOLD, DEFAULT_RIGIDBODY_SLEEP_THRESHOLD,
    MESH_COLLIDER_CLASS, MONO_BEHAVIOUR_CLASS, PHYSIC_MATERIAL_CLASS, RIGIDBODY_CLASS,
    ROOT_FILE_ID, SPHERE_COLLIDER_CLASS,
};

enum PhysicsComponent {
    Rigidbody(RigidbodyComponent),
    BoxCollider(BoxColliderComponent),
    SphereCollider(SphereColliderComponent),
    CapsuleCollider(CapsuleColliderComponent),
    MeshCollider(MeshColliderComponent),
    CharacterController(CharacterControllerComponent),
}

pub(super) fn collect_physics(
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
        if scene_joints::is_joint_class(document.class_id) {
            let _ = scene_joints::parse_scene_joints_3d(stream);
            let _ = scene_joints::parse_scene_joints_2d(stream);
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
            MESH_COLLIDER_CLASS => match document.root.get("MeshCollider") {
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
                MESH_COLLIDER_CLASS => PhysicsComponent::MeshCollider(mesh_collider_component(
                    body,
                    &record.name,
                    record.entity,
                    index,
                )),
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
                    colliders: Vec::new(),
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
            PhysicsComponent::BoxCollider(value) => {
                binding.colliders.push(ColliderComponent::Box(value));
            }
            PhysicsComponent::SphereCollider(value) => {
                binding.colliders.push(ColliderComponent::Sphere(value));
            }
            PhysicsComponent::CapsuleCollider(value) => {
                binding.colliders.push(ColliderComponent::Capsule(value));
            }
            PhysicsComponent::MeshCollider(value) => {
                binding.colliders.push(ColliderComponent::Mesh(value));
            }
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
        drag: yaml_f64(body.get("m_Drag"), DEFAULT_RIGIDBODY_DRAG, |value| {
            value >= 0.0
        }),
        angular_drag: yaml_f64(
            body.get("m_AngularDrag"),
            DEFAULT_RIGIDBODY_ANGULAR_DRAG,
            |value| value >= 0.0,
        ),
        use_gravity: yaml_bool(body.get("m_UseGravity"), true),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        is_kinematic: yaml_bool(body.get("m_IsKinematic"), false),
        interpolation: rigidbody_interpolation(body.get("m_Interpolate")),
        collision_detection: rigidbody_collision_detection(body.get("m_CollisionDetection")),
        sleep_threshold: yaml_f64(
            body.get("m_SleepThreshold"),
            DEFAULT_RIGIDBODY_SLEEP_THRESHOLD,
            |value| value >= 0.0,
        ),
        sleep_angular_threshold: yaml_f64(
            body.get("m_SleepAngularThreshold"),
            DEFAULT_RIGIDBODY_SLEEP_ANGULAR_THRESHOLD,
            |value| value >= 0.0,
        ),
        solver_iterations: optional_u32(body.get("m_SolverIterations")),
        solver_velocity_iterations: optional_u32(body.get("m_SolverVelocityIterations")),
        max_angular_velocity: yaml_f64(
            body.get("m_MaxAngularVelocity"),
            DEFAULT_RIGIDBODY_MAX_ANGULAR_VELOCITY,
            |value| value >= 0.0,
        ),
        max_depenetration_velocity: yaml_f64(
            body.get("m_MaxDepenetrationVelocity"),
            DEFAULT_RIGIDBODY_MAX_DEPENETRATION_VELOCITY,
            |value| value >= 0.0,
        ),
        include_layers: layer_mask_bits(body.get("m_IncludeLayers")),
        exclude_layers: layer_mask_bits(body.get("m_ExcludeLayers")),
        layer_override_priority: yaml_u32(body.get("m_LayerOverridePriority"), 0),
    }
}

fn rigidbody_interpolation(node: Option<&Node>) -> RigidbodyInterpolation {
    match node.and_then(Node::as_i64) {
        Some(1) => RigidbodyInterpolation::Interpolate,
        Some(2) => RigidbodyInterpolation::Extrapolate,
        _ => RigidbodyInterpolation::None,
    }
}

fn rigidbody_collision_detection(node: Option<&Node>) -> RigidbodyCollisionDetection {
    match node.and_then(Node::as_i64) {
        Some(1) => RigidbodyCollisionDetection::Continuous,
        Some(2) => RigidbodyCollisionDetection::ContinuousDynamic,
        Some(3) => RigidbodyCollisionDetection::ContinuousSpeculative,
        _ => RigidbodyCollisionDetection::Discrete,
    }
}

fn optional_u32(node: Option<&Node>) -> Option<u32> {
    node.and_then(Node::as_i64)
        .and_then(|value| u32::try_from(value).ok())
}

fn layer_mask_bits(node: Option<&Node>) -> Option<u32> {
    let node = node?;
    let bits = node.get("m_Bits").unwrap_or(node);
    bits.as_i64().and_then(|value| u32::try_from(value).ok())
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

fn mesh_collider_component(
    body: &Node,
    game_object_name: &str,
    entity: EntityId,
    index: Option<&GuidIndex>,
) -> MeshColliderComponent {
    let (material, material_diagnostic) =
        resolve_collider_material(body, "MeshCollider", game_object_name, index);
    let owner = format!(
        "MeshCollider on '{game_object_name}' (entity {})",
        entity.as_u64()
    );
    let (mesh_guid, mesh_file_id, mesh_path, mesh_diagnostic) =
        resolve_mesh_reference_for(body, index, &owner);
    let mesh_diagnostic = match mesh_diagnostic {
        Some(reason) => Some(reason),
        None => {
            unresolved_mesh_diagnostic(&owner, mesh_guid.as_deref(), mesh_path.as_deref(), index)
        }
    };
    MeshColliderComponent {
        mesh_guid,
        mesh_file_id,
        mesh_path,
        convex: yaml_bool(body.get("m_Convex"), false),
        center: vector3_or(body.get("m_Center"), [0.0, 0.0, 0.0]),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        material,
        material_diagnostic,
        mesh_diagnostic,
    }
}

fn unresolved_mesh_diagnostic(
    owner: &str,
    guid: Option<&str>,
    path: Option<&Path>,
    index: Option<&GuidIndex>,
) -> Option<String> {
    let guid = guid?;
    if path.is_some() || index.is_none() {
        return None;
    }
    if guid.eq_ignore_ascii_case(crate::primitive::BUILTIN_EXTRA_GUID) {
        return None;
    }
    Some(format!("{owner} references unresolved mesh guid {guid}"))
}

pub(super) fn is_character_controller(body: &Node) -> bool {
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

    match parse_physic_material(path) {
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

pub fn parse_physic_material(path: &Path) -> Result<MaterialValues, String> {
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
        friction_combine: friction_combine(
            body.get("m_FrictionCombine")
                .or_else(|| body.get("frictionCombine")),
        ),
        bounce_combine: bounce_combine(
            body.get("m_BounceCombine")
                .or_else(|| body.get("bounceCombine")),
        ),
    })
}

fn friction_combine(node: Option<&Node>) -> FrictionCombine {
    match node.and_then(Node::as_i64) {
        Some(1) => FrictionCombine::Minimum,
        Some(2) => FrictionCombine::Multiply,
        Some(3) => FrictionCombine::Maximum,
        _ => FrictionCombine::Average,
    }
}

fn bounce_combine(node: Option<&Node>) -> BounceCombine {
    match node.and_then(Node::as_i64) {
        Some(1) => BounceCombine::Minimum,
        Some(2) => BounceCombine::Maximum,
        Some(3) => BounceCombine::Multiply,
        _ => BounceCombine::Average,
    }
}

pub const CIRCLE_COLLIDER2D_CLASS: u32 = 58;
pub const POLYGON_COLLIDER2D_CLASS: u32 = 60;
pub const BOX_COLLIDER2D_CLASS: u32 = 61;
pub const COMPOSITE_COLLIDER2D_CLASS: u32 = 66;
pub const EDGE_COLLIDER2D_CLASS: u32 = 68;
pub const CAPSULE_COLLIDER2D_CLASS: u32 = 70;
pub const AREA_EFFECTOR2D_CLASS: u32 = 249;
pub const POINT_EFFECTOR2D_CLASS: u32 = 250;
pub const PLATFORM_EFFECTOR2D_CLASS: u32 = 251;
pub const SURFACE_EFFECTOR2D_CLASS: u32 = 252;
pub const BUOYANCY_EFFECTOR2D_CLASS: u32 = 253;

#[derive(Debug, Clone, PartialEq)]
pub enum ColliderShape2DDesc {
    Box {
        size: [f32; 2],
    },
    Circle {
        radius: f32,
    },
    Capsule {
        size: [f32; 2],
        direction: u32,
    },
    Polygon {
        vertices: Vec<[f32; 2]>,
    },
    Edge {
        points: Vec<[f32; 2]>,
    },
    Composite {
        geometry_type: u32,
        generation_type: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Collider2DDesc {
    pub entity: EntityId,
    pub layer: u32,
    pub enabled: bool,
    pub is_trigger: bool,
    pub used_by_effector: bool,
    pub used_by_composite: bool,
    pub offset: [f32; 2],
    pub shape: ColliderShape2DDesc,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlatformEffector2DDesc {
    pub use_one_way: bool,
    pub use_one_way_grouping: bool,
    pub surface_arc: f32,
    pub use_side_friction: bool,
    pub use_side_bounce: bool,
    pub side_arc: f32,
    pub rotational_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceEffector2DDesc {
    pub speed: f32,
    pub speed_variation: f32,
    pub force_scale: f32,
    pub use_contact_force: bool,
    pub use_friction: bool,
    pub use_bounce: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaEffector2DDesc {
    pub use_global_angle: bool,
    pub force_angle: f32,
    pub force_magnitude: f32,
    pub force_variation: f32,
    pub force_target: u32,
    pub drag: f32,
    pub angular_drag: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointEffector2DDesc {
    pub force_magnitude: f32,
    pub force_variation: f32,
    pub distance_scale: f32,
    pub force_source: u32,
    pub force_target: u32,
    pub force_mode: u32,
    pub drag: f32,
    pub angular_drag: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancyEffector2DDesc {
    pub surface_level: f32,
    pub density: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
    pub flow_angle: f32,
    pub flow_magnitude: f32,
    pub flow_variation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneEffectorKind2D {
    Platform(PlatformEffector2DDesc),
    Surface(SurfaceEffector2DDesc),
    Area(AreaEffector2DDesc),
    Point(PointEffector2DDesc),
    Buoyancy(BuoyancyEffector2DDesc),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneEffector2DBinding {
    pub entity: EntityId,
    pub enabled: bool,
    pub use_collider_mask: bool,
    pub collider_mask: u32,
    pub kind: SceneEffectorKind2D,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssembledScenePhysics {
    pub colliders: Vec<Collider2DDesc>,
    pub effectors: Vec<SceneEffector2DBinding>,
}

impl AssembledScenePhysics {
    pub fn shift_entity_ids(&mut self, base: u64) {
        for col in &mut self.colliders {
            col.entity = EntityId::new(base + col.entity.as_u64());
        }
        for eff in &mut self.effectors {
            eff.entity = EntityId::new(base + eff.entity.as_u64());
        }
    }
}

pub fn is_effector_or_collider_2d_class(class_id: u32) -> bool {
    matches!(
        class_id,
        CIRCLE_COLLIDER2D_CLASS
            | POLYGON_COLLIDER2D_CLASS
            | BOX_COLLIDER2D_CLASS
            | COMPOSITE_COLLIDER2D_CLASS
            | EDGE_COLLIDER2D_CLASS
            | CAPSULE_COLLIDER2D_CLASS
            | AREA_EFFECTOR2D_CLASS
            | POINT_EFFECTOR2D_CLASS
            | PLATFORM_EFFECTOR2D_CLASS
            | SURFACE_EFFECTOR2D_CLASS
            | BUOYANCY_EFFECTOR2D_CLASS
    )
}

pub(super) fn collect_physics_2d(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
) -> AssembledScenePhysics {
    let mut go_map = HashMap::with_capacity(game_objects.len());
    for (&file_id, record) in game_objects {
        if !instance_roots.contains(&file_id) {
            go_map.insert(file_id, (record.entity, record.layer));
        }
    }
    parse_scene_physics_2d_with_map(stream, &go_map)
}

pub fn parse_scene_physics_2d(stream: &Stream) -> AssembledScenePhysics {
    let go_map = build_go_entity_and_layer_map(stream);
    parse_scene_physics_2d_with_map(stream, &go_map)
}

fn parse_scene_physics_2d_with_map(
    stream: &Stream,
    go_map: &HashMap<i64, (EntityId, u32)>,
) -> AssembledScenePhysics {
    let mut colliders = Vec::new();
    let mut effectors = Vec::new();
    for doc in &stream.documents {
        if doc.stripped {
            continue;
        }
        if let Some(col) = parse_doc_collider_2d(doc.class_id, &doc.root, go_map) {
            colliders.push(col);
        } else if let Some(eff) = parse_doc_effector_2d(doc.class_id, &doc.root, go_map) {
            effectors.push(eff);
        }
    }
    AssembledScenePhysics {
        colliders,
        effectors,
    }
}

fn build_go_entity_and_layer_map(stream: &Stream) -> HashMap<i64, (EntityId, u32)> {
    let mut map = HashMap::new();
    let mut next_id = 0u64;
    for doc in &stream.documents {
        if doc.class_id == 1 && !doc.stripped {
            let layer = doc
                .root
                .get("GameObject")
                .and_then(|b| b.get("m_Layer"))
                .map_or(0, |n| yaml_u32(Some(n), 0));
            map.insert(doc.file_id, (EntityId::new(next_id), layer));
            next_id += 1;
        }
    }
    map
}

fn resolve_go_2d(body: &Node, go_map: &HashMap<i64, (EntityId, u32)>) -> Option<(EntityId, u32)> {
    let go_id = body
        .get("m_GameObject")?
        .get("fileID")?
        .as_i64()
        .filter(|&id| id != 0)?;
    go_map.get(&go_id).copied()
}

fn f32_field(body: &Node, key: &str, default: f32) -> f32 {
    body.get(key)
        .and_then(Node::as_f64)
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

fn vec2_field(node: Option<&Node>, default: [f32; 2]) -> [f32; 2] {
    let Some(n) = node else { return default };
    let (Some(x), Some(y)) = (
        n.get("x").and_then(Node::as_f64),
        n.get("y").and_then(Node::as_f64),
    ) else {
        return default;
    };
    [x as f32, y as f32]
}

fn parse_points_seq(node: Option<&Node>) -> Vec<[f32; 2]> {
    let Some(seq) = node.and_then(Node::as_sequence) else {
        return Vec::new();
    };
    seq.iter()
        .filter_map(|item| {
            Some([
                item.get("x")?.as_f64()? as f32,
                item.get("y")?.as_f64()? as f32,
            ])
        })
        .collect()
}

fn parse_polygon_points(body: &Node) -> Vec<[f32; 2]> {
    let Some(points) = body.get("m_Points") else {
        return Vec::new();
    };
    if let Some(paths) = points.get("m_Paths").and_then(Node::as_sequence) {
        if let Some(first) = paths.first() {
            return parse_points_seq(Some(first));
        }
    }
    parse_points_seq(Some(points))
}

fn parse_doc_collider_2d(
    class_id: u32,
    root: &Node,
    go_map: &HashMap<i64, (EntityId, u32)>,
) -> Option<Collider2DDesc> {
    let (body, shape) = match class_id {
        BOX_COLLIDER2D_CLASS => {
            let b = root.get("BoxCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Box {
                    size: vec2_field(b.get("m_Size"), [1.0, 1.0]),
                },
            )
        }
        CIRCLE_COLLIDER2D_CLASS => {
            let b = root.get("CircleCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Circle {
                    radius: f32_field(b, "m_Radius", 0.5),
                },
            )
        }
        CAPSULE_COLLIDER2D_CLASS => {
            let b = root.get("CapsuleCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Capsule {
                    size: vec2_field(b.get("m_Size"), [0.5, 1.0]),
                    direction: yaml_u32(b.get("m_Direction"), 0),
                },
            )
        }
        POLYGON_COLLIDER2D_CLASS => {
            let b = root.get("PolygonCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Polygon {
                    vertices: parse_polygon_points(b),
                },
            )
        }
        EDGE_COLLIDER2D_CLASS => {
            let b = root.get("EdgeCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Edge {
                    points: parse_points_seq(b.get("m_Points")),
                },
            )
        }
        COMPOSITE_COLLIDER2D_CLASS => {
            let b = root.get("CompositeCollider2D")?;
            (
                b,
                ColliderShape2DDesc::Composite {
                    geometry_type: yaml_u32(b.get("m_GeometryType"), 0),
                    generation_type: yaml_u32(b.get("m_GenerationType"), 0),
                },
            )
        }
        _ => return None,
    };
    let (entity, layer) = resolve_go_2d(body, go_map)?;
    Some(Collider2DDesc {
        entity,
        layer,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        is_trigger: yaml_bool(body.get("m_IsTrigger"), false),
        used_by_effector: yaml_bool(body.get("m_UsedByEffector"), false),
        used_by_composite: yaml_bool(body.get("m_UsedByComposite"), false),
        offset: vec2_field(body.get("m_Offset"), [0.0, 0.0]),
        shape,
    })
}

fn parse_doc_effector_2d(
    class_id: u32,
    root: &Node,
    go_map: &HashMap<i64, (EntityId, u32)>,
) -> Option<SceneEffector2DBinding> {
    let (body, kind) = match class_id {
        PLATFORM_EFFECTOR2D_CLASS => {
            let b = root.get("PlatformEffector2D")?;
            (
                b,
                SceneEffectorKind2D::Platform(PlatformEffector2DDesc {
                    use_one_way: yaml_bool(b.get("m_UseOneWay"), true),
                    use_one_way_grouping: yaml_bool(b.get("m_UseOneWayGrouping"), false),
                    surface_arc: f32_field(b, "m_SurfaceArc", 180.0),
                    use_side_friction: yaml_bool(b.get("m_UseSideFriction"), false),
                    use_side_bounce: yaml_bool(b.get("m_UseSideBounce"), false),
                    side_arc: f32_field(b, "m_SideArc", 0.0),
                    rotational_offset: f32_field(b, "m_RotationalOffset", 0.0),
                }),
            )
        }
        SURFACE_EFFECTOR2D_CLASS => {
            let b = root.get("SurfaceEffector2D")?;
            (
                b,
                SceneEffectorKind2D::Surface(SurfaceEffector2DDesc {
                    speed: f32_field(b, "m_Speed", 1.0),
                    speed_variation: f32_field(b, "m_SpeedVariation", 0.0),
                    force_scale: f32_field(b, "m_ForceScale", 0.1),
                    use_contact_force: yaml_bool(b.get("m_UseContactForce"), false),
                    use_friction: yaml_bool(b.get("m_UseFriction"), true),
                    use_bounce: yaml_bool(b.get("m_UseBounce"), true),
                }),
            )
        }
        AREA_EFFECTOR2D_CLASS => {
            let b = root.get("AreaEffector2D")?;
            (
                b,
                SceneEffectorKind2D::Area(AreaEffector2DDesc {
                    use_global_angle: yaml_bool(b.get("m_UseGlobalAngle"), false),
                    force_angle: f32_field(b, "m_ForceAngle", 0.0),
                    force_magnitude: f32_field(b, "m_ForceMagnitude", 10.0),
                    force_variation: f32_field(b, "m_ForceVariation", 0.0),
                    force_target: yaml_u32(b.get("m_ForceTarget"), 0),
                    drag: f32_field(b, "m_Drag", 0.0),
                    angular_drag: f32_field(b, "m_AngularDrag", 0.0),
                }),
            )
        }
        POINT_EFFECTOR2D_CLASS => {
            let b = root.get("PointEffector2D")?;
            (
                b,
                SceneEffectorKind2D::Point(PointEffector2DDesc {
                    force_magnitude: f32_field(b, "m_ForceMagnitude", -10.0),
                    force_variation: f32_field(b, "m_ForceVariation", 0.0),
                    distance_scale: f32_field(b, "m_DistanceScale", 1.0),
                    force_source: yaml_u32(b.get("m_ForceSource"), 0),
                    force_target: yaml_u32(b.get("m_ForceTarget"), 0),
                    force_mode: yaml_u32(b.get("m_ForceMode"), 0),
                    drag: f32_field(b, "m_Drag", 0.0),
                    angular_drag: f32_field(b, "m_AngularDrag", 0.0),
                }),
            )
        }
        BUOYANCY_EFFECTOR2D_CLASS => {
            let b = root.get("BuoyancyEffector2D")?;
            (
                b,
                SceneEffectorKind2D::Buoyancy(BuoyancyEffector2DDesc {
                    surface_level: f32_field(b, "m_SurfaceLevel", 0.0),
                    density: f32_field(b, "m_Density", 2.0),
                    linear_drag: f32_field(b, "m_LinearDrag", 5.0),
                    angular_drag: f32_field(b, "m_AngularDrag", 5.0),
                    flow_angle: f32_field(b, "m_FlowAngle", 0.0),
                    flow_magnitude: f32_field(b, "m_FlowMagnitude", 0.0),
                    flow_variation: f32_field(b, "m_FlowVariation", 0.0),
                }),
            )
        }
        _ => return None,
    };
    let (entity, _) = resolve_go_2d(body, go_map)?;
    let collider_mask = layer_mask_bits(body.get("m_ColliderMask")).unwrap_or(u32::MAX);
    Some(SceneEffector2DBinding {
        entity,
        enabled: yaml_bool(body.get("m_Enabled"), true),
        use_collider_mask: yaml_bool(body.get("m_UseColliderMask"), true),
        collider_mask,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::scene_joints;
    use crate::unity_yaml;
    use std::path::Path;
    use unitas_core::EntityId;

    const FIVE_EFFECTORS_SCENE: &str = "%YAML 1.1
--- !u!1 &10
GameObject:
  m_Name: OneWayPlatform
  m_Layer: 8
--- !u!61 &11
BoxCollider2D:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_UsedByComposite: 0
  m_Offset: {x: 0.5, y: -0.25}
  m_Size: {x: 6.0, y: 0.4}
--- !u!251 &12
PlatformEffector2D:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_UseColliderMask: 1
  m_ColliderMask:
    m_Bits: 128
  m_RotationalOffset: 15
  m_UseOneWay: 1
  m_UseOneWayGrouping: 1
  m_SurfaceArc: 200
  m_UseSideFriction: 1
  m_UseSideBounce: 1
  m_SideArc: 25
--- !u!1 &20
GameObject:
  m_Name: ConveyorBelt
  m_Layer: 6
--- !u!58 &21
CircleCollider2D:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Radius: 1.5
--- !u!252 &22
SurfaceEffector2D:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_UseColliderMask: 1
  m_ColliderMask:
    m_Bits: 64
  m_Speed: 3.5
  m_SpeedVariation: 0.25
  m_ForceScale: 0.8
  m_UseContactForce: 1
  m_UseFriction: 0
  m_UseBounce: 1
--- !u!1 &30
GameObject:
  m_Name: WindZone
  m_Layer: 0
--- !u!70 &31
CapsuleCollider2D:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 1}
  m_Size: {x: 2.0, y: 4.0}
  m_Direction: 0
--- !u!249 &32
AreaEffector2D:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_UseColliderMask: 0
  m_ColliderMask:
    m_Bits: 3
  m_UseGlobalAngle: 1
  m_ForceAngle: 90
  m_ForceMagnitude: 25
  m_ForceVariation: 2
  m_ForceTarget: 1
  m_Drag: 0.5
  m_AngularDrag: 0.75
--- !u!1 &40
GameObject:
  m_Name: MagnetNode
  m_Layer: 0
--- !u!60 &41
PolygonCollider2D:
  m_GameObject: {fileID: 40}
  m_Enabled: 1
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Points:
    m_Paths:
    - - {x: -1, y: -1}
      - {x: 1, y: -1}
      - {x: 0, y: 1}
--- !u!250 &42
PointEffector2D:
  m_GameObject: {fileID: 40}
  m_Enabled: 0
  m_UseColliderMask: 1
  m_ColliderMask:
    m_Bits: 15
  m_ForceMagnitude: -40
  m_ForceVariation: 5
  m_DistanceScale: 1.25
  m_ForceSource: 1
  m_ForceTarget: 1
  m_ForceMode: 2
  m_Drag: 0.3
  m_AngularDrag: 0.4
--- !u!1 &50
GameObject:
  m_Name: WaterPool
  m_Layer: 4
--- !u!68 &51
EdgeCollider2D:
  m_GameObject: {fileID: 50}
  m_Enabled: 1
  m_IsTrigger: 1
  m_UsedByEffector: 0
  m_Points:
  - {x: -4, y: 0}
  - {x: 4, y: 0}
--- !u!66 &52
CompositeCollider2D:
  m_GameObject: {fileID: 50}
  m_Enabled: 1
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_GeometryType: 1
  m_GenerationType: 0
--- !u!253 &53
BuoyancyEffector2D:
  m_GameObject: {fileID: 50}
  m_Enabled: 1
  m_UseColliderMask: 1
  m_ColliderMask:
    m_Bits: 255
  m_SurfaceLevel: 1.75
  m_Density: 2.5
  m_LinearDrag: 4.0
  m_AngularDrag: 3.0
  m_FlowAngle: 180
  m_FlowMagnitude: 6.0
  m_FlowVariation: 0.5
";

    #[test]
    fn parses_scene_joints_both_3d_and_2d() {
        let yaml = "%YAML 1.1\n--- !u!1 &1\nGameObject:\n  m_Name: A\n--- !u!232 &2\nDistanceJoint2D:\n  m_GameObject: {fileID: 1}\n  m_Distance: 3.5\n";
        let Ok(stream) = unity_yaml::parse(yaml) else {
            return;
        };
        assert!(scene_joints::is_joint_class(
            scene_joints::DISTANCE_JOINT2D_CLASS
        ));
        let j2d = scene_joints::parse_scene_joints_2d(&stream);
        assert_eq!(j2d.len(), 1);
        assert_eq!(j2d[0].kind, scene_joints::SceneJointKind2D::Distance);
        assert!((j2d[0].distance - 3.5).abs() < 1e-5);
    }

    #[test]
    fn parses_all_five_2d_effectors_and_used_by_effector_colliders(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stream = unity_yaml::parse(FIVE_EFFECTORS_SCENE)?;
        let physics_2d = super::parse_scene_physics_2d(&stream);
        assert_eq!(physics_2d.effectors.len(), 5);
        assert_eq!(physics_2d.colliders.len(), 6);

        let plat = &physics_2d.effectors[0];
        assert_eq!(plat.entity, EntityId::new(0));
        assert!(plat.enabled);
        assert!(plat.use_collider_mask);
        assert_eq!(plat.collider_mask, 128);
        assert_eq!(
            plat.kind,
            super::SceneEffectorKind2D::Platform(super::PlatformEffector2DDesc {
                use_one_way: true,
                use_one_way_grouping: true,
                surface_arc: 200.0,
                use_side_friction: true,
                use_side_bounce: true,
                side_arc: 25.0,
                rotational_offset: 15.0,
            })
        );

        let surf = &physics_2d.effectors[1];
        assert_eq!(surf.entity, EntityId::new(1));
        assert!(surf.enabled);
        assert!(surf.use_collider_mask);
        assert_eq!(surf.collider_mask, 64);
        assert_eq!(
            surf.kind,
            super::SceneEffectorKind2D::Surface(super::SurfaceEffector2DDesc {
                speed: 3.5,
                speed_variation: 0.25,
                force_scale: 0.8,
                use_contact_force: true,
                use_friction: false,
                use_bounce: true,
            })
        );

        let area = &physics_2d.effectors[2];
        assert_eq!(area.entity, EntityId::new(2));
        assert!(area.enabled);
        assert!(!area.use_collider_mask);
        assert_eq!(area.collider_mask, 3);
        assert_eq!(
            area.kind,
            super::SceneEffectorKind2D::Area(super::AreaEffector2DDesc {
                use_global_angle: true,
                force_angle: 90.0,
                force_magnitude: 25.0,
                force_variation: 2.0,
                force_target: 1,
                drag: 0.5,
                angular_drag: 0.75,
            })
        );

        let point = &physics_2d.effectors[3];
        assert_eq!(point.entity, EntityId::new(3));
        assert!(!point.enabled);
        assert!(point.use_collider_mask);
        assert_eq!(point.collider_mask, 15);
        assert_eq!(
            point.kind,
            super::SceneEffectorKind2D::Point(super::PointEffector2DDesc {
                force_magnitude: -40.0,
                force_variation: 5.0,
                distance_scale: 1.25,
                force_source: 1,
                force_target: 1,
                force_mode: 2,
                drag: 0.3,
                angular_drag: 0.4,
            })
        );

        let buoy = &physics_2d.effectors[4];
        assert_eq!(buoy.entity, EntityId::new(4));
        assert!(buoy.enabled);
        assert!(buoy.use_collider_mask);
        assert_eq!(buoy.collider_mask, 255);
        assert_eq!(
            buoy.kind,
            super::SceneEffectorKind2D::Buoyancy(super::BuoyancyEffector2DDesc {
                surface_level: 1.75,
                density: 2.5,
                linear_drag: 4.0,
                angular_drag: 3.0,
                flow_angle: 180.0,
                flow_magnitude: 6.0,
                flow_variation: 0.5,
            })
        );

        assert!(physics_2d.colliders[0].used_by_effector);
        assert!(physics_2d.colliders[1].used_by_effector);
        assert!(physics_2d.colliders[2].used_by_effector);
        assert!(physics_2d.colliders[3].used_by_effector);
        assert!(!physics_2d.colliders[4].used_by_effector);
        assert!(physics_2d.colliders[5].used_by_effector);
        Ok(())
    }

    #[test]
    fn rootway_scene_parses_one_way_bridge_platform_effector_and_conveyor_surface_effector(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rootway = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/2d-platformer/Assets/Scenes/Rootway.unity");
        let stream = unity_yaml::parse_file(&rootway)?;
        let physics_2d = super::parse_scene_physics_2d(&stream);
        assert_eq!(physics_2d.effectors.len(), 2);

        let bridge_eff = &physics_2d.effectors[0];
        assert_eq!(bridge_eff.entity, EntityId::new(3));
        assert!(bridge_eff.enabled);
        assert_eq!(
            bridge_eff.kind,
            super::SceneEffectorKind2D::Platform(super::PlatformEffector2DDesc {
                use_one_way: true,
                use_one_way_grouping: false,
                surface_arc: 200.0,
                use_side_friction: false,
                use_side_bounce: false,
                side_arc: 0.0,
                rotational_offset: 0.0,
            })
        );
        let bridge_col = physics_2d
            .colliders
            .iter()
            .find(|c| c.entity == EntityId::new(3))
            .ok_or("OneWayBridge collider not parsed")?;
        assert!(bridge_col.used_by_effector);
        assert_eq!(bridge_col.offset, [0.0, 0.0]);
        assert_eq!(
            bridge_col.shape,
            super::ColliderShape2DDesc::Box { size: [6.0, 0.4] }
        );

        let conveyor_eff = &physics_2d.effectors[1];
        assert_eq!(conveyor_eff.entity, EntityId::new(4));
        assert_eq!(
            conveyor_eff.kind,
            super::SceneEffectorKind2D::Surface(super::SurfaceEffector2DDesc {
                speed: 3.0,
                speed_variation: 0.0,
                force_scale: 1.0,
                use_contact_force: false,
                use_friction: true,
                use_bounce: false,
            })
        );
        Ok(())
    }
}
