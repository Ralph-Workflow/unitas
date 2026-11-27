use std::collections::HashMap;
use std::path::Path;

use unitas_asset::collision::{CollisionMesh, cook_builtin_collision_mesh, cook_collision_mesh};
use unitas_asset::scene::{
    CharacterControllerComponent, ColliderComponent, MeshColliderComponent, PhysicsBinding,
    RigidbodyComponent,
};
use unitas_core::{EntityId, IDENTITY_ROTATION, SceneGraph, normalize_rotation, rotate_vector};
use unitas_physics::{
    BodyDesc, BodyRole, CharacterController, ColliderDesc, ColliderShape, CollisionDetectionMode3D,
    Phase, PhysicsMaterial3D, Shape, World,
};

use super::Runtime;
use crate::error::RuntimeError;
use crate::physics::layer_bits;
use crate::physics_settings::{collision_detection_of, interpolation_of, material_of};

pub(super) struct BodySpec {
    desc: BodyDesc,
    colliders: Vec<ColliderDesc>,
    offset: [f32; 3],
    material: Option<PhysicsMaterial3D>,
}

pub(super) type BodySpecs = Vec<(EntityId, BodySpec)>;
pub(super) type ColliderDiagnostics = Vec<(EntityId, String)>;

pub(super) type ShiftedBodies = (
    Vec<EntityId>,
    Vec<(EntityId, [f32; 3])>,
    Vec<(EntityId, Vec<ColliderDesc>)>,
);

pub(super) fn shift_scene_bodies(
    descs: &[(EntityId, BodySpec)],
    entity_base: u64,
) -> ShiftedBodies {
    let bodies = descs
        .iter()
        .map(|(entity, _)| EntityId::new(entity_base + entity.as_u64()))
        .collect::<Vec<_>>();
    let body_offsets = descs
        .iter()
        .map(|(entity, spec)| (EntityId::new(entity_base + entity.as_u64()), spec.offset))
        .collect::<Vec<_>>();
    let body_colliders = descs
        .iter()
        .map(|(entity, spec)| {
            (
                EntityId::new(entity_base + entity.as_u64()),
                spec.colliders.clone(),
            )
        })
        .collect::<Vec<_>>();
    (bodies, body_offsets, body_colliders)
}

pub(super) fn body_descs(
    graph: &SceneGraph,
    bindings: &[PhysicsBinding],
) -> (BodySpecs, ColliderDiagnostics) {
    let rigid_entities: std::collections::BTreeSet<EntityId> = bindings
        .iter()
        .filter(|binding| binding.rigidbody.is_some())
        .map(|binding| binding.entity)
        .collect();
    let mut descs = Vec::new();
    let mut diagnostics = Vec::new();
    for binding in bindings {
        let spec = match body_spec(graph, binding, &rigid_entities, &mut diagnostics) {
            Some(spec) => spec,
            None => continue,
        };
        descs.push((binding.entity, spec));
    }
    (descs, diagnostics)
}

pub(super) fn validate_bodies(descs: &[(EntityId, BodySpec)]) -> Result<(), RuntimeError> {
    let mut world = World::new();
    for (entity, spec) in descs {
        world.add_body(*entity, spec.desc.clone())?;
        for collider in spec.colliders.iter().skip(1) {
            world.add_collider(*entity, collider.clone())?;
        }
    }
    Ok(())
}

pub(super) fn collect_scene_rigidbodies(
    bindings: &[PhysicsBinding],
    entity_base: u64,
) -> HashMap<EntityId, RigidbodyComponent> {
    bindings
        .iter()
        .filter_map(|binding| {
            binding.rigidbody.as_ref().map(|rigidbody| {
                (
                    EntityId::new(entity_base + binding.entity.as_u64()),
                    rigidbody.clone(),
                )
            })
        })
        .collect::<HashMap<EntityId, RigidbodyComponent>>()
}

fn layer_override_mask(mask: Option<u32>) -> Option<u32> {
    mask.filter(|bits| *bits != 0)
}

pub(super) fn collect_character_controllers(
    bindings: &[PhysicsBinding],
    entity_base: u64,
) -> Vec<(EntityId, CharacterControllerComponent)> {
    let mut controllers = Vec::new();
    for binding in bindings {
        if let Some(controller) = &binding.character_controller {
            let entity = EntityId::new(entity_base + binding.entity.as_u64());
            controllers.push((entity, controller.clone()));
        }
    }
    controllers
}

pub(super) fn controller_from_component(
    entity: Option<EntityId>,
    component: &CharacterControllerComponent,
) -> CharacterController {
    let mut controller = CharacterController::default();
    controller.radius = component.radius;
    controller.height = component.height;
    controller.center = component.center;
    controller.slope_limit_deg = component.slope_limit;
    controller.step_offset = component.step_offset;
    controller.skin_width = component.skin_width;
    controller.min_move_distance = if component.min_move_distance > 0.0 {
        component.min_move_distance
    } else {
        0.001
    };
    controller.layer_mask = u32::MAX;
    controller.enabled = component.enabled;
    controller.excluded_entity = entity;
    if controller.validate().is_err() {
        controller = CharacterController::default();
        controller.excluded_entity = entity;
    }
    controller
}

pub(super) fn collect_material_diagnostics(
    bindings: &[PhysicsBinding],
    entity_base: u64,
) -> Vec<(EntityId, String)> {
    let mut diagnostics = Vec::new();
    for binding in bindings {
        let entity = EntityId::new(entity_base + binding.entity.as_u64());
        for collider in &binding.colliders {
            let (material_diagnostic, geometry_diagnostic) = match collider {
                ColliderComponent::Box(component) => (component.material_diagnostic.as_ref(), None),
                ColliderComponent::Sphere(component) => {
                    (component.material_diagnostic.as_ref(), None)
                }
                ColliderComponent::Capsule(component) => (
                    component.material_diagnostic.as_ref(),
                    component.geometry_diagnostic.as_ref(),
                ),
                ColliderComponent::Mesh(component) => {
                    (component.material_diagnostic.as_ref(), None)
                }
            };
            if let Some(diag) = material_diagnostic {
                diagnostics.push((entity, diag.clone()));
            }
            if let Some(diag) = geometry_diagnostic {
                diagnostics.push((entity, diag.clone()));
            }
        }
    }
    diagnostics
}

pub(super) fn has_rigidbody_ancestor(
    graph: &SceneGraph,
    entity: EntityId,
    rigid_entities: &std::collections::BTreeSet<EntityId>,
) -> bool {
    let mut current = graph.parent(entity);
    while let Some(ancestor) = current {
        if rigid_entities.contains(&ancestor) {
            return true;
        }
        current = graph.parent(ancestor);
    }
    false
}

/// One Unity collider component, resolved into the shape the world needs.
struct ColliderPlan {
    shape: ColliderShape,
    center: [f64; 3],
    is_trigger: bool,
    material: Option<PhysicsMaterial3D>,
}

/// The colliders of one GameObject, in the order the scene lists them. A
/// collider whose mesh cannot be cooked is dropped with a diagnostic.
fn collider_plans(
    graph: &SceneGraph,
    binding: &PhysicsBinding,
    scale: [f32; 3],
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Vec<ColliderPlan> {
    let mut plans = Vec::new();
    for collider in &binding.colliders {
        let plan = match collider {
            ColliderComponent::Box(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Box {
                    half_extents: [
                        component.size[0] / 2.0,
                        component.size[1] / 2.0,
                        component.size[2] / 2.0,
                    ],
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Sphere(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Sphere {
                    radius: component.radius,
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Capsule(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Capsule {
                    radius: component.radius,
                    height: component.height,
                    direction: component.direction as u8,
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Mesh(component) => {
                match mesh_shape(graph, binding.entity, component, scale, diagnostics) {
                    Some(shape) => ColliderPlan {
                        shape,
                        center: component.center,
                        is_trigger: component.is_trigger,
                        material: component.material.as_ref().map(material_of),
                    },
                    None => continue,
                }
            }
        };
        plans.push(plan);
    }
    plans
}

/// Cook the geometry a mesh collider names: Unity's built-in primitive mesh
/// by identity, otherwise the referenced asset file.
fn mesh_shape(
    graph: &SceneGraph,
    entity: EntityId,
    component: &MeshColliderComponent,
    scale: [f32; 3],
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Option<ColliderShape> {
    let cooked = match &component.mesh_path {
        Some(path) => match cook_file_mesh(path, component.mesh_file_id) {
            Ok(mesh) => Some(mesh),
            Err(reason) => {
                diagnostics.push((
                    entity,
                    mesh_diagnostic_text(graph, entity, component, Some(&reason)),
                ));
                return None;
            }
        },
        None => component
            .mesh_file_id
            .zip(component.mesh_guid.as_deref())
            .and_then(|(file_id, guid)| cook_builtin_collision_mesh(file_id, guid)),
    };
    let mesh = match cooked {
        Some(mesh) => mesh,
        None => {
            diagnostics.push((entity, mesh_diagnostic_text(graph, entity, component, None)));
            return None;
        }
    };
    Some(ColliderShape::Mesh {
        vertices: mesh
            .vertices
            .iter()
            .map(|vertex| {
                [
                    f64::from(vertex[0]) * f64::from(scale[0]),
                    f64::from(vertex[1]) * f64::from(scale[1]),
                    f64::from(vertex[2]) * f64::from(scale[2]),
                ]
            })
            .collect(),
        indices: mesh.indices,
        convex: component.convex,
    })
}

fn cook_file_mesh(path: &Path, file_id: Option<i64>) -> Result<CollisionMesh, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    cook_collision_mesh(&bytes, file_name, file_id).map_err(|error| error.to_string())
}

/// The diagnostic a mesh collider that names no cookable geometry produces.
/// It always names the object and the asset it could not resolve.
fn mesh_diagnostic_text(
    graph: &SceneGraph,
    entity: EntityId,
    component: &MeshColliderComponent,
    reason: Option<&str>,
) -> String {
    if let Some(diagnostic) = &component.mesh_diagnostic {
        return diagnostic.clone();
    }
    let name = graph
        .entity(entity)
        .map_or("?", |entity| entity.name.as_str());
    let owner = format!("MeshCollider on '{name}' (entity {})", entity.as_u64());
    match (&component.mesh_path, component.mesh_guid.as_deref(), reason) {
        (Some(path), _, Some(reason)) => {
            format!(
                "{owner} references mesh '{}' that could not be cooked: {reason}",
                path.display()
            )
        }
        (Some(path), _, None) => {
            format!(
                "{owner} references mesh '{}' that could not be cooked",
                path.display()
            )
        }
        (None, Some(guid), Some(reason)) => {
            format!("{owner} references mesh guid {guid} that could not be cooked: {reason}")
        }
        (None, Some(guid), None) => {
            format!("{owner} references mesh guid {guid} that could not be cooked")
        }
        (None, None, _) => format!("{owner} has no mesh reference"),
    }
}

/// The offset of a collider from the body, in the body's own frame. The body
/// sits at the centre of the first collider, so the first collider's offset is
/// the origin and every other collider keeps its distance from it.
fn collider_offset(center: [f64; 3], first_center: [f64; 3], scale: [f32; 3]) -> [f64; 3] {
    [
        ((center[0] as f32 - first_center[0] as f32) * scale[0]) as f64,
        ((center[1] as f32 - first_center[1] as f32) * scale[1]) as f64,
        ((center[2] as f32 - first_center[2] as f32) * scale[2]) as f64,
    ]
}

fn body_spec(
    graph: &SceneGraph,
    binding: &PhysicsBinding,
    rigid_entities: &std::collections::BTreeSet<EntityId>,
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Option<BodySpec> {
    let (translation, rotation, scale) = match graph.world_transform(binding.entity) {
        Some(world) => (world.translation, world.rotation, world.scale),
        None => ([0.0f32; 3], IDENTITY_ROTATION, [1.0f32; 3]),
    };
    let plans = collider_plans(graph, binding, scale, diagnostics);
    let first_center = plans.first()?.center;
    let (mass, use_gravity, rigidbody_trigger, role) = match &binding.rigidbody {
        Some(rigidbody) if rigidbody.is_kinematic => (
            rigidbody.mass,
            rigidbody.use_gravity,
            rigidbody.is_trigger,
            BodyRole::Kinematic,
        ),
        Some(rigidbody) => (
            rigidbody.mass,
            rigidbody.use_gravity,
            rigidbody.is_trigger,
            BodyRole::Dynamic,
        ),
        None if has_rigidbody_ancestor(graph, binding.entity, rigid_entities) => {
            (0.0, false, false, BodyRole::Kinematic)
        }
        None => (0.0, false, false, BodyRole::Static),
    };
    let collision_detection_mode = binding
        .rigidbody
        .as_ref()
        .map_or(CollisionDetectionMode3D::Discrete, |rigidbody| {
            collision_detection_of(rigidbody.collision_detection)
        });
    let layer = layer_bits(binding.layer);
    let colliders: Vec<ColliderDesc> = plans
        .into_iter()
        .enumerate()
        .map(|(index, plan)| ColliderDesc {
            shape: plan.shape,
            offset: if index == 0 {
                [0.0; 3]
            } else {
                collider_offset(plan.center, first_center, scale)
            },
            rotation: [0.0, 0.0, 0.0, 1.0],
            is_trigger: plan.is_trigger || rigidbody_trigger,
            material: plan.material,
            layer,
        })
        .collect();
    let first = colliders.first()?;
    let material = first.material;
    let friction = material.map_or(0.6, |m| m.friction);
    let restitution = material.map_or(0.0, |m| m.restitution);
    let scaled_center = [
        first_center[0] as f32 * scale[0],
        first_center[1] as f32 * scale[1],
        first_center[2] as f32 * scale[2],
    ];
    let offset = rotate_vector(rotation, scaled_center);
    Some(BodySpec {
        desc: BodyDesc {
            position: [
                translation[0] as f64 + offset[0] as f64,
                translation[1] as f64 + offset[1] as f64,
                translation[2] as f64 + offset[2] as f64,
            ],
            rotation: [
                rotation[0] as f64,
                rotation[1] as f64,
                rotation[2] as f64,
                rotation[3] as f64,
            ],
            mass,
            velocity: [0.0; 3],
            use_gravity,
            is_trigger: first.is_trigger,
            collider_shape: first.shape.clone(),
            layer,
            role,
            friction,
            restitution,
            collision_detection_mode,
        },
        colliders,
        offset,
        material,
    })
}

pub(super) fn normalized(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}

pub(super) fn rotations_equivalent(a: [f64; 4], b: [f64; 4]) -> bool {
    let a = normalize_rotation([a[0] as f32, a[1] as f32, a[2] as f32, a[3] as f32]);
    let b = normalize_rotation([b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32]);
    let dot = f64::from(a[0]) * f64::from(b[0])
        + f64::from(a[1]) * f64::from(b[1])
        + f64::from(a[2]) * f64::from(b[2])
        + f64::from(a[3]) * f64::from(b[3]);
    (1.0 - dot.abs()) <= 1e-6
}

pub(super) fn ordered_pair(first: u64, second: u64) -> (u64, u64) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn phase_code(phase: Phase) -> u32 {
    match phase {
        Phase::Enter => 0,
        Phase::Stay => 1,
        Phase::Exit => 2,
    }
}

pub const CONTACT_PHASE_2D_FLAG: u32 = 0x100;

pub(super) fn contact_phase_code(phase: Phase, is_2d: bool) -> u32 {
    let code = phase_code(phase);
    if is_2d {
        code | CONTACT_PHASE_2D_FLAG
    } else {
        code
    }
}

impl Runtime {
    pub(super) fn register_scene_bodies(
        &mut self,
        bodies: &[EntityId],
        descs: &[(EntityId, BodySpec)],
        rigidbodies: &HashMap<EntityId, RigidbodyComponent>,
    ) -> Result<(), RuntimeError> {
        for (body, (_, spec)) in bodies.iter().zip(descs.iter()) {
            self.physics.world_3d_mut().add_body_with_material(
                *body,
                spec.desc.clone(),
                spec.material,
            )?;
            // The body's own shape already is the first Unity collider, so
            // only the remaining ones become extra colliders. Adding the
            // first one again would put a second copy of it in the world.
            for collider in spec.colliders.iter().skip(1) {
                self.physics
                    .world_3d_mut()
                    .add_collider(*body, collider.clone())?;
            }
            if let Some(rigidbody) = rigidbodies.get(body) {
                let world = self.physics.world_3d_mut();
                world.set_body_drag(*body, rigidbody.drag, rigidbody.angular_drag);
                world.set_body_interpolation(*body, interpolation_of(rigidbody.interpolation));
                world.set_body_layer_overrides(
                    *body,
                    layer_override_mask(rigidbody.include_layers),
                    layer_override_mask(rigidbody.exclude_layers),
                    rigidbody.layer_override_priority,
                );
            }
        }
        Ok(())
    }
}
