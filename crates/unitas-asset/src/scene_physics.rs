use std::collections::{HashMap, HashSet};
use std::path::Path;

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
