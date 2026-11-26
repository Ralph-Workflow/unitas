use std::collections::{BTreeSet, HashMap};

use unitas_core::EntityId;

use crate::backend::{Backend, BackendHandles};
use crate::body::{
    lerp_vector, slerp_rotation, BodyDesc, BodyRole, CollisionDetectionMode3D, Shape,
};
use crate::error::PhysicsError;
use crate::event::{CollisionEvent, Phase, TriggerEvent};
use crate::matrix::CollisionMatrix;
use crate::query::{
    cast_ray_against, cast_shape_against, normalize, overlaps_at, probe_shape, CastShape, QueryHit,
};

const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];
const DEFAULT_SLEEP_THRESHOLD: f64 = 0.05;
const DEFAULT_TIME_TO_SLEEP: f64 = 0.5;
const MAX_CONVEX_MESH_HULL_TRIANGLES: usize = 255;

type PairSet = BTreeSet<(EntityId, EntityId)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ForceMode {
    Force = 0,
    Impulse = 1,
    Acceleration = 2,
    VelocityChange = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial3D {
    pub friction: f64,
    pub restitution: f64,
}

impl Default for PhysicsMaterial3D {
    fn default() -> Self {
        Self {
            friction: 0.6,
            restitution: 0.0,
        }
    }
}

impl PhysicsMaterial3D {
    pub fn new(friction: f64, restitution: f64) -> Self {
        Self {
            friction: friction.max(0.0),
            restitution: restitution.max(0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshColliderDesc {
    pub object_name: String,
    pub mesh_name: String,
    pub convex: bool,
    pub is_trigger: bool,
    pub triangle_count: usize,
    pub bounds_half_extents: [f64; 3],
}

impl MeshColliderDesc {
    pub fn non_convex(
        object_name: impl Into<String>,
        mesh_name: impl Into<String>,
        bounds_half_extents: [f64; 3],
    ) -> Self {
        Self {
            object_name: object_name.into(),
            mesh_name: mesh_name.into(),
            convex: false,
            is_trigger: false,
            triangle_count: 12,
            bounds_half_extents,
        }
    }

    pub fn convex(
        object_name: impl Into<String>,
        mesh_name: impl Into<String>,
        bounds_half_extents: [f64; 3],
    ) -> Self {
        Self {
            object_name: object_name.into(),
            mesh_name: mesh_name.into(),
            convex: true,
            is_trigger: false,
            triangle_count: 12,
            bounds_half_extents,
        }
    }

    pub fn with_trigger(mut self, is_trigger: bool) -> Self {
        self.is_trigger = is_trigger;
        self
    }

    pub fn with_triangle_count(mut self, triangle_count: usize) -> Self {
        self.triangle_count = triangle_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicsDiagnostic {
    pub entity: EntityId,
    pub object_name: String,
    pub code: &'static str,
    pub message: String,
}

fn speed_of(body: &Body) -> f64 {
    let sum = body.velocity[0] * body.velocity[0]
        + body.velocity[1] * body.velocity[1]
        + body.velocity[2] * body.velocity[2];
    sum.sqrt()
}

fn angular_speed_of(body: &Body) -> f64 {
    let sum = body.angular_velocity[0] * body.angular_velocity[0]
        + body.angular_velocity[1] * body.angular_velocity[1]
        + body.angular_velocity[2] * body.angular_velocity[2];
    sum.sqrt()
}

fn pair_exchanges_messages(first: &Body, second: &Body) -> bool {
    if first.is_trigger || second.is_trigger {
        return first.role != BodyRole::Static || second.role != BodyRole::Static;
    }
    if first.role == BodyRole::Kinematic && second.role != BodyRole::Dynamic {
        return false;
    }
    if second.role == BodyRole::Kinematic && first.role != BodyRole::Dynamic {
        return false;
    }
    first.role != BodyRole::Static || second.role != BodyRole::Static
}

#[derive(Clone, Copy, Debug)]
struct Body {
    entity: EntityId,
    handles: BackendHandles,
    shape: Shape,
    mass: f64,
    role: BodyRole,
    is_trigger: bool,
    position: [f64; 3],
    rotation: [f64; 4],
    query_position: [f64; 3],
    query_rotation: [f64; 4],
    velocity: [f64; 3],
    angular_velocity: [f64; 3],
    layer: u32,
    friction: f64,
    restitution: f64,
    uses_default_material: bool,
    previous_position: [f64; 3],
    previous_rotation: [f64; 4],
    sweep_velocity: [f64; 3],
    pending_force: [f64; 3],
    pending_impulse: [f64; 3],
    pending_acceleration: [f64; 3],
    pending_torque: [f64; 3],
    pending_angular_acceleration: [f64; 3],
    pending_move_position: Option<[f64; 3]>,
    pending_move_rotation: Option<[f64; 4]>,
    sleeping: bool,
    sleep_timer: f64,
    simulated: bool,
}

impl Body {
    fn inverse_mass(&self) -> f64 {
        match self.role {
            BodyRole::Dynamic if self.mass > 0.0 => 1.0 / self.mass,
            BodyRole::Dynamic | BodyRole::Kinematic | BodyRole::Static => 0.0,
        }
    }
}

pub struct World {
    gravity: [f64; 3],
    backend: Backend,
    bodies: Vec<Body>,
    indices: HashMap<EntityId, usize>,
    overlapping: PairSet,
    overlapping_triggers: PairSet,
    events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,
    collision_matrix: CollisionMatrix,
    sleep_threshold: f64,
    time_to_sleep: f64,
    auto_sync_transforms: bool,
    default_material: PhysicsMaterial3D,
    diagnostics: Vec<PhysicsDiagnostic>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self::with_gravity(DEFAULT_GRAVITY)
    }

    pub fn with_gravity(gravity: [f64; 3]) -> Self {
        Self {
            gravity,
            backend: Backend::new(),
            bodies: Vec::new(),
            indices: HashMap::new(),
            overlapping: PairSet::new(),
            overlapping_triggers: PairSet::new(),
            events: Vec::new(),
            trigger_events: Vec::new(),
            collision_matrix: CollisionMatrix::new(),
            sleep_threshold: DEFAULT_SLEEP_THRESHOLD,
            time_to_sleep: DEFAULT_TIME_TO_SLEEP,
            auto_sync_transforms: true,
            default_material: PhysicsMaterial3D::default(),
            diagnostics: Vec::new(),
        }
    }

    pub fn gravity(&self) -> [f64; 3] {
        self.gravity
    }

    pub fn set_collision_matrix(&mut self, matrix: CollisionMatrix) {
        self.collision_matrix = matrix;
    }

    pub fn collision_matrix(&self) -> &CollisionMatrix {
        &self.collision_matrix
    }

    pub fn ignore_layer_collision(&mut self, layer_a: usize, layer_b: usize, ignore: bool) {
        self.collision_matrix
            .ignore_layer_collision(layer_a, layer_b, ignore);
    }

    pub fn get_ignore_layer_collision(&self, layer_a: usize, layer_b: usize) -> bool {
        !self.collision_matrix.is_enabled(layer_a, layer_b)
    }

    pub fn layer_mask_for_layer(&self, layer: usize) -> u32 {
        self.collision_matrix.layer_mask_for_layer(layer)
    }

    pub fn auto_sync_transforms(&self) -> bool {
        self.auto_sync_transforms
    }

    pub fn set_auto_sync_transforms(&mut self, auto_sync: bool) {
        self.auto_sync_transforms = auto_sync;
        if auto_sync {
            self.sync_transforms();
        }
    }

    pub fn sync_transforms(&mut self) {
        for body in &mut self.bodies {
            body.query_position = body.position;
            body.query_rotation = body.rotation;
        }
    }

    pub fn default_material(&self) -> PhysicsMaterial3D {
        self.default_material
    }

    pub fn set_default_material(&mut self, material: PhysicsMaterial3D) {
        let clamped = PhysicsMaterial3D::new(material.friction, material.restitution);
        self.default_material = clamped;
        for body in &mut self.bodies {
            if body.uses_default_material {
                body.friction = clamped.friction;
                body.restitution = clamped.restitution;
                self.backend
                    .set_material(body.handles, clamped.friction, clamped.restitution);
            }
        }
    }

    pub fn body_material(&self, entity: EntityId) -> Option<PhysicsMaterial3D> {
        self.indices.get(&entity).map(|index| {
            let body = &self.bodies[*index];
            PhysicsMaterial3D {
                friction: body.friction,
                restitution: body.restitution,
            }
        })
    }

    pub fn add_body_with_default_material(
        &mut self,
        entity: EntityId,
        mut desc: BodyDesc,
    ) -> Result<(), PhysicsError> {
        desc.friction = self.default_material.friction;
        desc.restitution = self.default_material.restitution;
        self.add_body(entity, desc)?;
        if let Some(index) = self.indices.get(&entity).copied() {
            self.bodies[index].uses_default_material = true;
        }
        Ok(())
    }

    pub fn add_body_with_material(
        &mut self,
        entity: EntityId,
        mut desc: BodyDesc,
        material: Option<PhysicsMaterial3D>,
    ) -> Result<(), PhysicsError> {
        match material {
            Some(mat) => {
                desc.friction = mat.friction;
                desc.restitution = mat.restitution;
                self.add_body(entity, desc)
            }
            None => self.add_body_with_default_material(entity, desc),
        }
    }

    pub fn add_mesh_collider_body(
        &mut self,
        entity: EntityId,
        mut body_desc: BodyDesc,
        mesh_desc: MeshColliderDesc,
    ) -> Result<(), PhysicsError> {
        if self.indices.contains_key(&entity) {
            return Err(PhysicsError::DuplicateEntity(entity));
        }
        body_desc.shape = Shape::Box {
            half_extents: mesh_desc.bounds_half_extents,
        };
        body_desc.is_trigger = mesh_desc.is_trigger;
        if !body_desc.is_valid() {
            return Err(PhysicsError::InvalidBody(entity));
        }
        let effective_role = body_desc.effective_role();
        if !mesh_desc.convex && effective_role == BodyRole::Dynamic {
            let message = format!(
                "Non-convex MeshCollider with non-kinematic Rigidbody is no longer supported since Unity 5. Object: '{}' (mesh '{}', entity {})",
                mesh_desc.object_name, mesh_desc.mesh_name, entity
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name.clone(),
                code: "PHY-MESH-NONCONVEX-DYNAMIC",
                message,
            });
            return Err(PhysicsError::NonConvexMeshColliderOnDynamicBody {
                entity,
                object_name: mesh_desc.object_name,
            });
        }
        if !mesh_desc.convex && mesh_desc.is_trigger {
            let message = format!(
                "A non-convex MeshCollider cannot be used as a trigger on '{}' (mesh '{}', entity {}); enable Convex to use a trigger MeshCollider.",
                mesh_desc.object_name, mesh_desc.mesh_name, entity
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name.clone(),
                code: "PHY-MESH-NONCONVEX-TRIGGER",
                message,
            });
            body_desc.is_trigger = false;
            self.add_body(entity, body_desc)?;
            if let Some(index) = self.indices.get(&entity).copied() {
                self.bodies[index].simulated = false;
                let handles = self.bodies[index].handles;
                self.backend.set_enabled(handles, false);
            }
            return Ok(());
        }
        if mesh_desc.convex && mesh_desc.triangle_count > MAX_CONVEX_MESH_HULL_TRIANGLES {
            let message = format!(
                "Convex MeshCollider on '{}' (mesh '{}', entity {}) has {} triangles, exceeding the 255-triangle hull limit; using reduced convex hull.",
                mesh_desc.object_name, mesh_desc.mesh_name, entity, mesh_desc.triangle_count
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name,
                code: "PHY-MESH-CONVEX-HULL-LIMIT",
                message,
            });
        }
        self.add_body(entity, body_desc)
    }

    pub fn attach_mesh_collider(
        &mut self,
        entity: EntityId,
        mesh_desc: MeshColliderDesc,
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&entity).copied() else {
            return Err(PhysicsError::InvalidBody(entity));
        };
        let new_shape = Shape::Box {
            half_extents: mesh_desc.bounds_half_extents,
        };
        if !new_shape.is_valid() {
            return Err(PhysicsError::InvalidBody(entity));
        }
        let role = self.bodies[index].role;
        if !mesh_desc.convex && role == BodyRole::Dynamic {
            let message = format!(
                "Non-convex MeshCollider with non-kinematic Rigidbody is no longer supported since Unity 5. Object: '{}' (mesh '{}', entity {})",
                mesh_desc.object_name, mesh_desc.mesh_name, entity
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name.clone(),
                code: "PHY-MESH-NONCONVEX-DYNAMIC",
                message,
            });
            self.bodies[index].simulated = false;
            self.bodies[index].velocity = [0.0; 3];
            let handles = self.bodies[index].handles;
            self.backend.set_enabled(handles, false);
            self.backend.set_velocity(handles, [0.0; 3], false);
            self.overlapping
                .retain(|(first, second)| *first != entity && *second != entity);
            self.overlapping_triggers
                .retain(|(first, second)| *first != entity && *second != entity);
            return Err(PhysicsError::NonConvexMeshColliderOnDynamicBody {
                entity,
                object_name: mesh_desc.object_name,
            });
        }
        if !mesh_desc.convex && mesh_desc.is_trigger {
            let message = format!(
                "A non-convex MeshCollider cannot be used as a trigger on '{}' (mesh '{}', entity {}); enable Convex to use a trigger MeshCollider.",
                mesh_desc.object_name, mesh_desc.mesh_name, entity
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name,
                code: "PHY-MESH-NONCONVEX-TRIGGER",
                message,
            });
            self.bodies[index].simulated = false;
            let handles = self.bodies[index].handles;
            self.backend.set_enabled(handles, false);
            return Ok(());
        }
        if mesh_desc.convex && mesh_desc.triangle_count > MAX_CONVEX_MESH_HULL_TRIANGLES {
            let message = format!(
                "Convex MeshCollider on '{}' (mesh '{}', entity {}) has {} triangles, exceeding the 255-triangle hull limit; using reduced convex hull.",
                mesh_desc.object_name, mesh_desc.mesh_name, entity, mesh_desc.triangle_count
            );
            self.diagnostics.push(PhysicsDiagnostic {
                entity,
                object_name: mesh_desc.object_name,
                code: "PHY-MESH-CONVEX-HULL-LIMIT",
                message,
            });
        }
        let handles = self.bodies[index].handles;
        self.backend.set_shape(handles, &new_shape);
        self.backend.set_sensor(handles, mesh_desc.is_trigger);
        self.backend.set_enabled(handles, true);
        self.bodies[index].shape = new_shape;
        self.bodies[index].is_trigger = mesh_desc.is_trigger;
        self.bodies[index].simulated = true;
        Ok(())
    }

    pub fn diagnostics(&self) -> &[PhysicsDiagnostic] {
        &self.diagnostics
    }

    pub fn entity_diagnostics(&self, entity: EntityId) -> Vec<&PhysicsDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diag| diag.entity == entity)
            .collect()
    }

    pub fn drain_diagnostics(&mut self) -> Vec<PhysicsDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn is_body_simulated(&self, entity: EntityId) -> bool {
        self.indices
            .get(&entity)
            .is_some_and(|index| self.bodies[*index].simulated)
    }

    pub fn add_body(&mut self, entity: EntityId, desc: BodyDesc) -> Result<(), PhysicsError> {
        if self.indices.contains_key(&entity) {
            return Err(PhysicsError::DuplicateEntity(entity));
        }
        if !desc.is_valid() {
            return Err(PhysicsError::InvalidBody(entity));
        }
        let role = desc.effective_role();
        let handles = self.backend.add_body(entity, &desc, role);
        let index = self.bodies.len();
        self.bodies.push(Body {
            entity,
            handles,
            shape: desc.shape,
            mass: desc.mass,
            role,
            is_trigger: desc.is_trigger,
            position: desc.position,
            rotation: desc.rotation,
            query_position: desc.position,
            query_rotation: desc.rotation,
            velocity: desc.velocity,
            angular_velocity: [0.0; 3],
            layer: desc.layer,
            friction: desc.friction,
            restitution: desc.restitution,
            uses_default_material: false,
            previous_position: desc.position,
            previous_rotation: desc.rotation,
            sweep_velocity: [0.0; 3],
            pending_force: [0.0; 3],
            pending_impulse: [0.0; 3],
            pending_acceleration: [0.0; 3],
            pending_torque: [0.0; 3],
            pending_angular_acceleration: [0.0; 3],
            pending_move_position: None,
            pending_move_rotation: None,
            sleeping: false,
            sleep_timer: 0.0,
            simulated: true,
        });
        self.indices.insert(entity, index);
        Ok(())
    }

    pub fn step(&mut self, delta_seconds: f64) -> Result<(), PhysicsError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(PhysicsError::InvalidDeltaTime(delta_seconds));
        }
        for body in &mut self.bodies {
            if !body.simulated {
                continue;
            }
            if body.role == BodyRole::Kinematic {
                if body.pending_move_position.is_some() || body.pending_move_rotation.is_some() {
                    if delta_seconds > 0.0 {
                        self.backend.set_next_kinematic_pose(
                            body.handles,
                            body.pending_move_position,
                            body.pending_move_rotation,
                        );
                    } else {
                        let position = body.pending_move_position.unwrap_or(body.position);
                        let rotation = body.pending_move_rotation.unwrap_or(body.rotation);
                        self.backend.set_pose(body.handles, position, rotation);
                    }
                    body.pending_move_position = None;
                    body.pending_move_rotation = None;
                } else if delta_seconds > 0.0
                    && (body.position != body.previous_position
                        || body.rotation != body.previous_rotation)
                {
                    self.backend
                        .set_translation(body.handles, body.previous_position, false);
                    self.backend
                        .set_rotation(body.handles, body.previous_rotation, false);
                    self.backend.set_next_kinematic_pose(
                        body.handles,
                        Some(body.position),
                        Some(body.rotation),
                    );
                }
                continue;
            }
            if body.role != BodyRole::Dynamic {
                continue;
            }
            body.previous_position = body.position;
            body.previous_rotation = body.rotation;
            if body.sleeping {
                continue;
            }
            let inv_mass = body.inverse_mass();
            let mass = if inv_mass > 0.0 { 1.0 / inv_mass } else { 0.0 };
            let force = [
                body.pending_force[0] + body.pending_acceleration[0] * mass,
                body.pending_force[1] + body.pending_acceleration[1] * mass,
                body.pending_force[2] + body.pending_acceleration[2] * mass,
            ];
            let impulse = body.pending_impulse;
            let torque = body.pending_torque;
            let angular_acceleration = body.pending_angular_acceleration;
            body.pending_force = [0.0; 3];
            body.pending_impulse = [0.0; 3];
            body.pending_acceleration = [0.0; 3];
            body.pending_torque = [0.0; 3];
            body.pending_angular_acceleration = [0.0; 3];
            self.backend.apply_forces(body.handles, force, impulse);
            self.backend.apply_torque(body.handles, torque);
            self.backend.apply_angular_acceleration(
                body.handles,
                angular_acceleration,
                delta_seconds,
            );
        }
        if delta_seconds > 0.0 {
            self.backend
                .step(self.gravity, delta_seconds, &self.collision_matrix);
        }
        for body in &mut self.bodies {
            if !body.simulated {
                continue;
            }
            match body.role {
                BodyRole::Dynamic => {
                    body.position = self.backend.position(body.handles);
                    body.rotation = self.backend.rotation(body.handles);
                    if body.sleeping && !self.backend.is_sleeping(body.handles) {
                        body.sleeping = false;
                        body.sleep_timer = 0.0;
                    }
                    if !body.sleeping {
                        body.velocity = self.backend.velocity(body.handles);
                        body.angular_velocity = self.backend.angular_velocity(body.handles);
                    }
                }
                BodyRole::Kinematic => {
                    body.position = self.backend.position(body.handles);
                    body.rotation = self.backend.rotation(body.handles);
                }
                BodyRole::Static => {}
            }
        }
        self.update_sweep_velocities(delta_seconds);
        self.update_sleeping(delta_seconds);
        if delta_seconds > 0.0 {
            self.collect_events();
        } else {
            self.emit_events(&self.overlapping.clone());
            self.emit_trigger_events(&self.overlapping_triggers.clone());
        }
        self.commit_kinematic_poses();
        self.sync_transforms();
        Ok(())
    }

    fn collect_events(&mut self) {
        let (contact_pairs, intersection_pairs) = self.backend.touching_entity_pairs();
        let mut overlapping = PairSet::new();
        let mut overlapping_triggers = PairSet::new();
        for pair in contact_pairs {
            let (Some(first), Some(second)) =
                (self.indices.get(&pair.0), self.indices.get(&pair.1))
            else {
                continue;
            };
            if !self.bodies[*first].simulated || !self.bodies[*second].simulated {
                continue;
            }
            if !pair_exchanges_messages(&self.bodies[*first], &self.bodies[*second]) {
                continue;
            }
            overlapping.insert(pair);
        }
        for pair in intersection_pairs {
            let (Some(first), Some(second)) =
                (self.indices.get(&pair.0), self.indices.get(&pair.1))
            else {
                continue;
            };
            if !self.bodies[*first].simulated || !self.bodies[*second].simulated {
                continue;
            }
            if !pair_exchanges_messages(&self.bodies[*first], &self.bodies[*second]) {
                continue;
            }
            overlapping_triggers.insert(pair);
        }
        self.emit_events(&overlapping);
        self.emit_trigger_events(&overlapping_triggers);
        self.overlapping = overlapping;
        self.overlapping_triggers = overlapping_triggers;
    }

    pub fn drain_events(&mut self) -> Vec<CollisionEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_trigger_events(&mut self) -> Vec<TriggerEvent> {
        std::mem::take(&mut self.trigger_events)
    }

    pub fn position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].position)
    }

    pub fn rotation(&self, entity: EntityId) -> Option<[f64; 4]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].rotation)
    }

    pub fn query_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].query_position)
    }

    pub fn query_rotation(&self, entity: EntityId) -> Option<[f64; 4]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].query_rotation)
    }

    pub fn velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].velocity)
    }

    pub fn angular_velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].angular_velocity)
    }

    pub fn angular_inertia(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices.get(&entity).map(|index| {
            self.backend
                .angular_inertia(self.bodies[*index].handles)
        })
    }

    pub fn interpolated_pose(&self, entity: EntityId, alpha: f64) -> Option<([f64; 3], [f64; 4])> {
        let index = self.indices.get(&entity).copied()?;
        let body = &self.bodies[index];
        let alpha = alpha.clamp(0.0, 1.0);
        Some((
            lerp_vector(body.previous_position, body.position, alpha),
            slerp_rotation(body.previous_rotation, body.rotation, alpha),
        ))
    }

    pub fn effective_velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices.get(&entity).map(|index| {
            let body = &self.bodies[*index];
            match body.role {
                BodyRole::Dynamic => body.velocity,
                BodyRole::Kinematic => {
                    if body.sweep_velocity != [0.0; 3] {
                        body.sweep_velocity
                    } else {
                        body.velocity
                    }
                }
                BodyRole::Static => [0.0; 3],
            }
        })
    }

    pub fn teleport_position(&mut self, body: EntityId, p: [f64; 3]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !p.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let auto_sync = self.auto_sync_transforms;
        let slot = &mut self.bodies[index];
        slot.position = p;
        slot.previous_position = p;
        slot.pending_move_position = None;
        if auto_sync {
            slot.query_position = p;
        }
        slot.sweep_velocity = [0.0; 3];
        slot.sleeping = false;
        slot.sleep_timer = 0.0;
        let handles = slot.handles;
        self.backend.set_translation(handles, p, true);
        self.backend.wake_up(handles);
        Ok(())
    }

    pub fn teleport_rotation(
        &mut self,
        body: EntityId,
        rotation: [f64; 4],
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !rotation.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let auto_sync = self.auto_sync_transforms;
        let slot = &mut self.bodies[index];
        slot.rotation = rotation;
        slot.previous_rotation = rotation;
        slot.pending_move_rotation = None;
        if auto_sync {
            slot.query_rotation = rotation;
        }
        slot.sleeping = false;
        slot.sleep_timer = 0.0;
        let handles = slot.handles;
        self.backend.set_rotation(handles, rotation, true);
        self.backend.wake_up(handles);
        Ok(())
    }

    pub fn move_position(&mut self, body: EntityId, target: [f64; 3]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !target.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        if self.bodies[index].role != BodyRole::Kinematic {
            return Err(PhysicsError::InvalidBody(body));
        }
        self.bodies[index].pending_move_position = Some(target);
        Ok(())
    }

    pub fn move_rotation(&mut self, body: EntityId, target: [f64; 4]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !target.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        if self.bodies[index].role != BodyRole::Kinematic {
            return Err(PhysicsError::InvalidBody(body));
        }
        self.bodies[index].pending_move_rotation = Some(target);
        Ok(())
    }

    pub fn add_force(
        &mut self,
        body: EntityId,
        force: [f64; 3],
        mode: ForceMode,
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !force.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let slot = &mut self.bodies[index];
        if slot.role != BodyRole::Dynamic {
            return Ok(());
        }
        let mut sync_velocity = false;
        match mode {
            ForceMode::Force => {
                slot.pending_force[0] += force[0];
                slot.pending_force[1] += force[1];
                slot.pending_force[2] += force[2];
            }
            ForceMode::Acceleration => {
                slot.pending_acceleration[0] += force[0];
                slot.pending_acceleration[1] += force[1];
                slot.pending_acceleration[2] += force[2];
            }
            ForceMode::Impulse => {
                let inv_mass = slot.inverse_mass();
                slot.velocity[0] += force[0] * inv_mass;
                slot.velocity[1] += force[1] * inv_mass;
                slot.velocity[2] += force[2] * inv_mass;
                sync_velocity = true;
            }
            ForceMode::VelocityChange => {
                slot.velocity[0] += force[0];
                slot.velocity[1] += force[1];
                slot.velocity[2] += force[2];
                sync_velocity = true;
            }
        }
        if force != [0.0; 3] {
            slot.sleeping = false;
            slot.sleep_timer = 0.0;
        }
        let handles = slot.handles;
        let velocity = slot.velocity;
        if sync_velocity {
            self.backend.set_velocity(handles, velocity, true);
        }
        if force != [0.0; 3] {
            self.backend.wake_up(handles);
        }
        Ok(())
    }

    pub fn add_impulse(&mut self, body: EntityId, impulse: [f64; 3]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !impulse.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let slot = &mut self.bodies[index];
        if slot.role == BodyRole::Dynamic {
            slot.pending_impulse[0] += impulse[0];
            slot.pending_impulse[1] += impulse[1];
            slot.pending_impulse[2] += impulse[2];
            if impulse != [0.0; 3] {
                slot.sleeping = false;
                slot.sleep_timer = 0.0;
                let handles = slot.handles;
                self.backend.wake_up(handles);
            }
        }
        Ok(())
    }

    pub fn add_torque(
        &mut self,
        body: EntityId,
        torque: [f64; 3],
        mode: ForceMode,
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !torque.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let slot = &mut self.bodies[index];
        if slot.role != BodyRole::Dynamic {
            return Ok(());
        }
        match mode {
            ForceMode::Force => {
                slot.pending_torque[0] += torque[0];
                slot.pending_torque[1] += torque[1];
                slot.pending_torque[2] += torque[2];
            }
            ForceMode::Acceleration => {
                slot.pending_angular_acceleration[0] += torque[0];
                slot.pending_angular_acceleration[1] += torque[1];
                slot.pending_angular_acceleration[2] += torque[2];
            }
            ForceMode::Impulse => {
                let handles = slot.handles;
                self.backend.apply_torque_impulse(handles, torque);
                slot.angular_velocity = self.backend.angular_velocity(handles);
            }
            ForceMode::VelocityChange => {
                slot.angular_velocity[0] += torque[0];
                slot.angular_velocity[1] += torque[1];
                slot.angular_velocity[2] += torque[2];
                let handles = slot.handles;
                let angular_velocity = slot.angular_velocity;
                self.backend
                    .set_angular_velocity(handles, angular_velocity, false);
            }
        }
        if torque != [0.0; 3] {
            slot.sleeping = false;
            slot.sleep_timer = 0.0;
            let handles = slot.handles;
            self.backend.wake_up(handles);
        }
        Ok(())
    }

    pub fn body_role(&self, entity: EntityId) -> Option<BodyRole> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].role)
    }

    pub fn set_velocity(&mut self, body: EntityId, v: [f64; 3]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !v.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        self.bodies[index].velocity = v;
        let handles = self.bodies[index].handles;
        if self.bodies[index].role == BodyRole::Dynamic {
            self.backend.set_velocity(handles, v, true);
        }
        self.wake_up(body);
        Ok(())
    }

    pub fn set_angular_velocity(
        &mut self,
        body: EntityId,
        v: [f64; 3],
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !v.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        let slot = &mut self.bodies[index];
        slot.angular_velocity = v;
        let handles = slot.handles;
        if slot.role == BodyRole::Dynamic {
            self.backend.set_angular_velocity(handles, v, true);
        }
        self.wake_up(body);
        Ok(())
    }

    pub fn set_collision_detection_mode(
        &mut self,
        body: EntityId,
        mode: CollisionDetectionMode3D,
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        let handles = self.bodies[index].handles;
        self.backend.set_collision_detection_mode(handles, mode);
        Ok(())
    }

    pub fn sleep(&mut self, body: EntityId) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        self.bodies[index].sleeping = true;
        self.bodies[index].sleep_timer = 0.0;
        self.bodies[index].velocity = [0.0; 3];
        self.bodies[index].angular_velocity = [0.0; 3];
        self.bodies[index].pending_force = [0.0; 3];
        self.bodies[index].pending_impulse = [0.0; 3];
        self.bodies[index].pending_acceleration = [0.0; 3];
        self.bodies[index].pending_torque = [0.0; 3];
        self.bodies[index].pending_angular_acceleration = [0.0; 3];
        let handles = self.bodies[index].handles;
        self.backend.sleep(handles);
        Ok(())
    }

    pub fn wake_up(&mut self, body: EntityId) {
        if let Some(index) = self.indices.get(&body).copied() {
            self.bodies[index].sleeping = false;
            self.bodies[index].sleep_timer = 0.0;
            let handles = self.bodies[index].handles;
            self.backend.wake_up(handles);
        }
    }

    pub fn is_sleeping(&self, body: EntityId) -> bool {
        self.indices
            .get(&body)
            .map(|index| self.bodies[*index].sleeping)
            .unwrap_or(false)
    }

    pub fn set_sleep_threshold(&mut self, speed_threshold: f64, time_to_sleep: f64) {
        self.sleep_threshold = speed_threshold.max(0.0);
        self.time_to_sleep = time_to_sleep.max(0.0);
    }

    pub fn set_position(&mut self, body: EntityId, p: [f64; 3]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !p.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        if self.bodies[index].role == BodyRole::Dynamic {
            return Err(PhysicsError::InvalidBody(body));
        }
        self.bodies[index].position = p;
        if self.auto_sync_transforms {
            self.bodies[index].query_position = p;
        }
        if self.bodies[index].role == BodyRole::Kinematic {
            return Ok(());
        }
        let handles = self.bodies[index].handles;
        self.backend.set_translation(handles, p, false);
        Ok(())
    }

    pub fn set_rotation(&mut self, body: EntityId, rotation: [f64; 4]) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&body).copied() else {
            return Err(PhysicsError::InvalidBody(body));
        };
        if !rotation.iter().all(|value| value.is_finite()) {
            return Err(PhysicsError::InvalidBody(body));
        }
        if self.bodies[index].role == BodyRole::Dynamic {
            return Err(PhysicsError::InvalidBody(body));
        }
        self.bodies[index].rotation = rotation;
        if self.auto_sync_transforms {
            self.bodies[index].query_rotation = rotation;
        }
        if self.bodies[index].role == BodyRole::Kinematic {
            return Ok(());
        }
        let handles = self.bodies[index].handles;
        self.backend.set_rotation(handles, rotation, false);
        Ok(())
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn remove_body(&mut self, entity: EntityId) -> bool {
        let Some(index) = self.indices.remove(&entity) else {
            return false;
        };
        let removed = self.bodies.remove(index);
        self.backend.remove_body(removed.handles);
        for slot in self.indices.values_mut() {
            if *slot > index {
                *slot -= 1;
            }
        }
        self.overlapping
            .retain(|(first, second)| *first != entity && *second != entity);
        self.overlapping_triggers
            .retain(|(first, second)| *first != entity && *second != entity);
        true
    }

    pub fn set_gravity(&mut self, gravity: [f64; 3]) {
        self.gravity = gravity;
    }

    pub fn raycast(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
    ) -> Vec<QueryHit> {
        self.raycast_with_triggers(origin, direction, max_distance, layer_mask, false)
    }

    pub fn raycast_with_triggers(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryHit> {
        self.cast(
            CastShape::Point,
            origin,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        )
    }

    pub fn linecast(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryHit> {
        let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
        let Some(direction) = normalize(delta) else {
            return Vec::new();
        };
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        self.cast(
            CastShape::Point,
            start,
            direction,
            distance,
            layer_mask,
            include_triggers,
        )
    }

    pub fn sphere_cast(
        &self,
        origin: [f64; 3],
        radius: f64,
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryHit> {
        if !radius.is_finite() || radius <= 0.0 {
            return Vec::new();
        }
        self.cast(
            CastShape::Sphere { radius },
            origin,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        )
    }

    pub fn box_cast(
        &self,
        origin: [f64; 3],
        half_extents: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryHit> {
        if !half_extents
            .iter()
            .all(|extent| extent.is_finite() && *extent > 0.0)
        {
            return Vec::new();
        }
        self.cast(
            CastShape::Box { half_extents },
            origin,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        )
    }

    pub fn raycast_default_layer(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        query_layer: usize,
    ) -> Vec<QueryHit> {
        let mask = self.layer_mask_for_layer(query_layer);
        self.cast_with_layer_mode(
            CastShape::Point,
            origin,
            direction,
            max_distance,
            (mask, false, true),
        )
    }

    pub fn overlap_sphere_default_layer(
        &self,
        center: [f64; 3],
        radius: f64,
        query_layer: usize,
    ) -> Vec<EntityId> {
        let mask = self.layer_mask_for_layer(query_layer);
        if !radius.is_finite() || radius <= 0.0 {
            return Vec::new();
        }
        let probe = probe_shape(&Shape::Sphere { radius });
        let dispatcher = self.backend.dispatcher();
        let mut entities: Vec<EntityId> = self
            .bodies
            .iter()
            .filter(|body| body.simulated)
            .filter(|body| Self::layer_index_matches_mask(body.layer, mask))
            .filter(|body| !body.is_trigger)
            .filter(|body| {
                overlaps_at(
                    dispatcher,
                    &probe,
                    center,
                    body.query_position,
                    body.query_rotation,
                    &body.shape,
                )
            })
            .map(|body| body.entity)
            .collect();
        entities.sort();
        entities
    }

    pub fn overlap_sphere(&self, center: [f64; 3], radius: f64, layer_mask: u32) -> Vec<EntityId> {
        self.overlap_sphere_with_triggers(center, radius, layer_mask, false)
    }

    pub fn overlap_sphere_with_triggers(
        &self,
        center: [f64; 3],
        radius: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<EntityId> {
        if !radius.is_finite() || radius <= 0.0 {
            return Vec::new();
        }
        let probe = probe_shape(&Shape::Sphere { radius });
        let dispatcher = self.backend.dispatcher();
        let mut entities: Vec<EntityId> = self
            .bodies
            .iter()
            .filter(|body| body.simulated)
            .filter(|body| body.layer & layer_mask != 0)
            .filter(|body| !body.is_trigger || include_triggers)
            .filter(|body| {
                overlaps_at(
                    dispatcher,
                    &probe,
                    center,
                    body.query_position,
                    body.query_rotation,
                    &body.shape,
                )
            })
            .map(|body| body.entity)
            .collect();
        entities.sort();
        entities
    }

    fn layer_index_matches_mask(body_layer: u32, layer_mask: u32) -> bool {
        body_layer < 32 && (layer_mask & (1u32 << body_layer)) != 0
    }

    fn cast(
        &self,
        cast_shape: CastShape,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryHit> {
        self.cast_with_layer_mode(
            cast_shape,
            origin,
            direction,
            max_distance,
            (layer_mask, include_triggers, false),
        )
    }

    fn cast_with_layer_mode(
        &self,
        cast_shape: CastShape,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        filter: (u32, bool, bool),
    ) -> Vec<QueryHit> {
        let (layer_mask, include_triggers, use_layer_index) = filter;
        if max_distance <= 0.0 || max_distance.is_nan() {
            return Vec::new();
        }
        let Some(direction) = normalize(direction) else {
            return Vec::new();
        };
        let dispatcher = self.backend.dispatcher();
        let mut hits = Vec::new();
        for body in &self.bodies {
            if !body.simulated {
                continue;
            }
            let layer_ok = if use_layer_index {
                Self::layer_index_matches_mask(body.layer, layer_mask)
            } else {
                body.layer & layer_mask != 0
            };
            if !layer_ok {
                continue;
            }
            if body.is_trigger && !include_triggers {
                continue;
            }
            let hit = match cast_shape {
                CastShape::Point => cast_ray_against(
                    origin,
                    direction,
                    max_distance,
                    body.query_position,
                    body.query_rotation,
                    &body.shape,
                ),
                CastShape::Sphere { .. } | CastShape::Box { .. } => cast_shape_against(
                    dispatcher,
                    cast_shape,
                    origin,
                    direction,
                    max_distance,
                    (body.query_position, body.query_rotation, &body.shape),
                ),
            };
            let Some(hit) = hit else {
                continue;
            };
            if hit.distance > max_distance {
                continue;
            }
            hits.push(QueryHit {
                entity: body.entity,
                point: hit.point,
                normal: hit.normal,
                distance: hit.distance,
            });
        }
        hits.sort_by(|first, second| {
            first
                .distance
                .total_cmp(&second.distance)
                .then(first.entity.cmp(&second.entity))
        });
        hits
    }

    fn update_sweep_velocities(&mut self, delta_seconds: f64) {
        for body in &mut self.bodies {
            if !body.simulated || body.role != BodyRole::Kinematic {
                continue;
            }
            if delta_seconds <= 0.0 {
                body.sweep_velocity = [0.0; 3];
                continue;
            }
            body.sweep_velocity = [
                (body.position[0] - body.previous_position[0]) / delta_seconds,
                (body.position[1] - body.previous_position[1]) / delta_seconds,
                (body.position[2] - body.previous_position[2]) / delta_seconds,
            ];
        }
    }

    fn commit_kinematic_poses(&mut self) {
        for body in &mut self.bodies {
            if body.role == BodyRole::Dynamic {
                continue;
            }
            body.previous_position = body.position;
            body.previous_rotation = body.rotation;
        }
    }

    fn update_sleeping(&mut self, delta_seconds: f64) {
        let threshold = self.sleep_threshold;
        let time_to_sleep = self.time_to_sleep;
        for body in &mut self.bodies {
            if !body.simulated || body.role != BodyRole::Dynamic || body.sleeping {
                continue;
            }
            if speed_of(body) <= threshold && angular_speed_of(body) <= threshold {
                body.sleep_timer += delta_seconds;
                if body.sleep_timer >= time_to_sleep {
                    body.sleeping = true;
                    body.velocity = [0.0; 3];
                    body.angular_velocity = [0.0; 3];
                    let handles = body.handles;
                    self.backend.sleep(handles);
                }
            } else {
                body.sleep_timer = 0.0;
            }
        }
    }

    fn layer_pair_enabled(&self, pair: &(EntityId, EntityId)) -> bool {
        let (Some(first), Some(second)) = (self.indices.get(&pair.0), self.indices.get(&pair.1))
        else {
            return false;
        };
        self.collision_matrix.is_enabled(
            self.bodies[*first].layer as usize,
            self.bodies[*second].layer as usize,
        )
    }

    fn emit_events(&mut self, overlapping: &BTreeSet<(EntityId, EntityId)>) {
        for pair in overlapping.difference(&self.overlapping) {
            if !self.layer_pair_enabled(pair) {
                continue;
            }
            self.events.push(CollisionEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Enter,
            });
        }
        for pair in &self.overlapping {
            if overlapping.contains(pair) && self.layer_pair_enabled(pair) {
                self.events.push(CollisionEvent {
                    a: pair.0,
                    b: pair.1,
                    phase: Phase::Stay,
                });
            }
        }
        for pair in self.overlapping.difference(overlapping) {
            if !self.layer_pair_enabled(pair) {
                continue;
            }
            self.events.push(CollisionEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Exit,
            });
        }
    }

    fn emit_trigger_events(&mut self, overlapping_triggers: &BTreeSet<(EntityId, EntityId)>) {
        for pair in overlapping_triggers.difference(&self.overlapping_triggers) {
            if !self.layer_pair_enabled(pair) {
                continue;
            }
            self.trigger_events.push(TriggerEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Enter,
            });
        }
        for pair in &self.overlapping_triggers {
            if overlapping_triggers.contains(pair) && self.layer_pair_enabled(pair) {
                self.trigger_events.push(TriggerEvent {
                    a: pair.0,
                    b: pair.1,
                    phase: Phase::Stay,
                });
            }
        }
        for pair in self.overlapping_triggers.difference(overlapping_triggers) {
            if !self.layer_pair_enabled(pair) {
                continue;
            }
            self.trigger_events.push(TriggerEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Exit,
            });
        }
    }
}
