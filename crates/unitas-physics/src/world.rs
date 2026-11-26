use std::collections::{BTreeMap, BTreeSet, HashMap};

use unitas_core::EntityId;

use crate::backend::{Backend, BackendHandles, ColliderLayer};
use crate::body::{
    add, dot, inverse_rotation, lerp_vector, multiply_rotation, normalize_rotation, rotate_vector,
    slerp_rotation, subtract, vector_length, BodyDesc, BodyRole, ColliderShape,
    CollisionDetectionMode3D, Shape, IDENTITY_ROTATION,
};
use crate::error::PhysicsError;
use crate::event::{CollisionEvent, Phase, TriggerEvent};
use crate::joint::{
    BrokenJointEvent3D, CharacterJoint3D, ConfigurableJoint3D, ConfigurableJointMotion,
    FixedJoint3D, HingeJoint3D, Joint3D, JointDrive3D, JointId3D, SpringJoint3D,
};
use crate::matrix::CollisionMatrix;
use crate::query::{
    cast_ray_against, cast_shape_against, normalize, overlaps_at, probe_shape, CastShape, QueryHit,
};

const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];
const DEFAULT_SLEEP_THRESHOLD: f64 = 0.05;
const DEFAULT_TIME_TO_SLEEP: f64 = 0.5;
const MAX_CONVEX_MESH_HULL_TRIANGLES: usize = 255;

type PairSet = BTreeSet<(EntityId, EntityId)>;
type BodyState3D = (usize, [f64; 3], [f64; 4], [f64; 3], [f64; 3], f64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ForceMode {
    Force = 0,
    Impulse = 1,
    Acceleration = 2,
    VelocityChange = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrictionCombine {
    #[default]
    Average,
    Minimum,
    Multiply,
    Maximum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BounceCombine {
    #[default]
    Average,
    Minimum,
    Maximum,
    Multiply,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyInterpolation {
    None,
    #[default]
    Interpolate,
    Extrapolate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SolverType {
    #[default]
    Default,
    Tgs,
    TgsSoft,
}

impl SolverType {
    pub fn is_representable(self) -> bool {
        !matches!(self, SolverType::TgsSoft)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverSettings {
    pub num_solver_iterations: usize,
    pub num_additional_friction_iterations: usize,
    pub num_internal_pgs_iterations: usize,
    pub contact_offset: f64,
    pub allowed_linear_error: f64,
    pub max_depenetration_velocity: f64,
    pub deterministic_accumulation: bool,
    pub solver_type: SolverType,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            num_solver_iterations: 4,
            num_additional_friction_iterations: 1,
            num_internal_pgs_iterations: 1,
            contact_offset: 0.02,
            allowed_linear_error: 0.005,
            max_depenetration_velocity: 3.0,
            deterministic_accumulation: true,
            solver_type: SolverType::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionContact {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub impulse: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionPayload {
    pub contacts: Vec<CollisionContact>,
    pub relative_velocity: [f64; 3],
    pub total_impulse: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial3D {
    pub friction: f64,
    pub restitution: f64,
    pub friction_combine: FrictionCombine,
    pub restitution_combine: BounceCombine,
}

impl Default for PhysicsMaterial3D {
    fn default() -> Self {
        Self {
            friction: 0.6,
            restitution: 0.0,
            friction_combine: FrictionCombine::Average,
            restitution_combine: BounceCombine::Average,
        }
    }
}

impl PhysicsMaterial3D {
    pub fn new(friction: f64, restitution: f64) -> Self {
        Self {
            friction: friction.max(0.0),
            restitution: restitution.max(0.0),
            friction_combine: FrictionCombine::Average,
            restitution_combine: BounceCombine::Average,
        }
    }
}

pub fn combine_materials(a: &PhysicsMaterial3D, b: &PhysicsMaterial3D) -> PhysicsMaterial3D {
    let friction_combine =
        if friction_priority(a.friction_combine) >= friction_priority(b.friction_combine) {
            a.friction_combine
        } else {
            b.friction_combine
        };
    let restitution_combine =
        if bounce_priority(a.restitution_combine) >= bounce_priority(b.restitution_combine) {
            a.restitution_combine
        } else {
            b.restitution_combine
        };
    PhysicsMaterial3D {
        friction: combine_friction(a.friction, b.friction, friction_combine),
        restitution: combine_restitution(a.restitution, b.restitution, restitution_combine),
        friction_combine,
        restitution_combine,
    }
}

fn friction_priority(rule: FrictionCombine) -> u8 {
    match rule {
        FrictionCombine::Average => 0,
        FrictionCombine::Minimum => 1,
        FrictionCombine::Multiply => 2,
        FrictionCombine::Maximum => 3,
    }
}

fn bounce_priority(rule: BounceCombine) -> u8 {
    match rule {
        BounceCombine::Average => 0,
        BounceCombine::Minimum => 1,
        BounceCombine::Multiply => 2,
        BounceCombine::Maximum => 3,
    }
}

fn combine_friction(first: f64, second: f64, rule: FrictionCombine) -> f64 {
    match rule {
        FrictionCombine::Average => (first + second) / 2.0,
        FrictionCombine::Minimum => first.min(second).abs(),
        FrictionCombine::Multiply => first * second,
        FrictionCombine::Maximum => first.max(second),
    }
}

fn combine_restitution(first: f64, second: f64, rule: BounceCombine) -> f64 {
    match rule {
        BounceCombine::Average => (first + second) / 2.0,
        BounceCombine::Minimum => first.min(second).abs(),
        BounceCombine::Maximum => first.max(second),
        BounceCombine::Multiply => first * second,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColliderDesc {
    pub shape: ColliderShape,
    pub offset: [f64; 3],
    pub rotation: [f64; 4],
    pub is_trigger: bool,
    pub material: Option<PhysicsMaterial3D>,
    pub layer: u32,
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

fn clamp_angular_velocity(velocity: [f64; 3], max: f64) -> [f64; 3] {
    let speed = vector_length(velocity);
    if speed <= max || speed == 0.0 {
        return velocity;
    }
    let scale = max / speed;
    [
        velocity[0] * scale,
        velocity[1] * scale,
        velocity[2] * scale,
    ]
}

pub(crate) fn ordered_pair(first: EntityId, second: EntityId) -> (EntityId, EntityId) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
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

#[derive(Clone, Debug)]
struct Body {
    entity: EntityId,
    handles: BackendHandles,
    collider_shape: ColliderShape,
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
    friction_combine: FrictionCombine,
    restitution_combine: BounceCombine,
    collision_detection_mode: CollisionDetectionMode3D,
    interpolation: BodyInterpolation,
    max_angular_velocity: Option<f64>,
    linear_drag: f64,
    angular_drag: f64,
    layer_include: Option<u32>,
    layer_exclude: Option<u32>,
    layer_priority: u32,
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

    fn query_shape(&self) -> Option<Shape> {
        match &self.collider_shape {
            ColliderShape::Primitive(shape) => Some(*shape),
            ColliderShape::Mesh { .. } => None,
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
    collision_payloads: BTreeMap<(EntityId, EntityId), CollisionPayload>,
    collision_matrix: CollisionMatrix,
    sleep_threshold: f64,
    time_to_sleep: f64,
    auto_sync_transforms: bool,
    default_material: PhysicsMaterial3D,
    solver_settings: SolverSettings,
    ccd_body_count: usize,
    last_delta_seconds: f64,
    diagnostics: Vec<PhysicsDiagnostic>,
    next_joint_id_3d: u64,
    joints_3d: BTreeMap<JointId3D, Joint3D>,
    broken_joints_3d: Vec<BrokenJointEvent3D>,
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
        let mut world = Self {
            gravity,
            backend: Backend::new(),
            bodies: Vec::new(),
            indices: HashMap::new(),
            overlapping: PairSet::new(),
            overlapping_triggers: PairSet::new(),
            events: Vec::new(),
            trigger_events: Vec::new(),
            collision_payloads: BTreeMap::new(),
            collision_matrix: CollisionMatrix::new(),
            sleep_threshold: DEFAULT_SLEEP_THRESHOLD,
            time_to_sleep: DEFAULT_TIME_TO_SLEEP,
            auto_sync_transforms: true,
            default_material: PhysicsMaterial3D::default(),
            solver_settings: SolverSettings::default(),
            ccd_body_count: 0,
            last_delta_seconds: 0.0,
            diagnostics: Vec::new(),
            next_joint_id_3d: 1,
            joints_3d: BTreeMap::new(),
            broken_joints_3d: Vec::new(),
        };
        world.backend.set_solver_settings(world.solver_settings);
        world
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
        let rules = PhysicsMaterial3D {
            friction_combine: material.friction_combine,
            restitution_combine: material.restitution_combine,
            ..clamped
        };
        self.default_material = rules;
        for body in &mut self.bodies {
            if body.uses_default_material {
                body.friction = rules.friction;
                body.restitution = rules.restitution;
                body.friction_combine = rules.friction_combine;
                body.restitution_combine = rules.restitution_combine;
                self.backend.set_material(
                    body.handles,
                    rules.friction,
                    rules.restitution,
                    rules.friction_combine,
                    rules.restitution_combine,
                );
            }
        }
    }

    pub fn body_material(&self, entity: EntityId) -> Option<PhysicsMaterial3D> {
        self.indices.get(&entity).map(|index| {
            let body = &self.bodies[*index];
            PhysicsMaterial3D {
                friction: body.friction,
                restitution: body.restitution,
                friction_combine: body.friction_combine,
                restitution_combine: body.restitution_combine,
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
            self.bodies[index].friction_combine = self.default_material.friction_combine;
            self.bodies[index].restitution_combine = self.default_material.restitution_combine;
            let handles = self.bodies[index].handles;
            let material = self.default_material;
            self.backend.set_material(
                handles,
                material.friction,
                material.restitution,
                material.friction_combine,
                material.restitution_combine,
            );
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
                self.add_body(entity, desc)?;
                if let Some(index) = self.indices.get(&entity).copied() {
                    self.bodies[index].friction_combine = mat.friction_combine;
                    self.bodies[index].restitution_combine = mat.restitution_combine;
                    let handles = self.bodies[index].handles;
                    self.backend.set_material(
                        handles,
                        mat.friction,
                        mat.restitution,
                        mat.friction_combine,
                        mat.restitution_combine,
                    );
                }
                Ok(())
            }
            None => self.add_body_with_default_material(entity, desc),
        }
    }

    fn refuse_non_convex_mesh(&mut self, entity: EntityId) -> PhysicsError {
        let object_name = entity.to_string();
        let message = format!(
            "Non-convex MeshCollider with non-kinematic Rigidbody is no longer supported since Unity 5. Object: '{object_name}' (entity {entity})"
        );
        self.diagnostics.push(PhysicsDiagnostic {
            entity,
            object_name: object_name.clone(),
            code: "PHY-MESH-NONCONVEX-DYNAMIC",
            message,
        });
        PhysicsError::NonConvexMeshColliderOnDynamicBody {
            entity,
            object_name,
        }
    }

    fn refuse_non_convex_trigger(&mut self, entity: EntityId) -> PhysicsError {
        let message = format!(
            "A non-convex MeshCollider cannot be used as a trigger (entity {entity}); enable Convex to use a trigger MeshCollider."
        );
        self.diagnostics.push(PhysicsDiagnostic {
            entity,
            object_name: entity.to_string(),
            code: "PHY-MESH-NONCONVEX-TRIGGER",
            message,
        });
        PhysicsError::InvalidBody(entity)
    }

    fn note_convex_hull_limit(&mut self, entity: EntityId, shape: &ColliderShape) {
        let ColliderShape::Mesh {
            indices,
            convex: true,
            ..
        } = shape
        else {
            return;
        };
        let triangles = indices.len() / 3;
        if triangles <= MAX_CONVEX_MESH_HULL_TRIANGLES {
            return;
        }
        let message = format!(
            "Convex MeshCollider (entity {entity}) has {triangles} triangles, exceeding the 255-triangle hull limit; using the convex hull of its vertices."
        );
        self.diagnostics.push(PhysicsDiagnostic {
            entity,
            object_name: entity.to_string(),
            code: "PHY-MESH-CONVEX-HULL-LIMIT",
            message,
        });
    }

    pub fn add_collider(
        &mut self,
        entity: EntityId,
        desc: ColliderDesc,
    ) -> Result<(), PhysicsError> {
        let Some(index) = self.indices.get(&entity).copied() else {
            return Err(PhysicsError::InvalidBody(entity));
        };
        if !desc.shape.is_valid()
            || !desc.offset.iter().all(|value| value.is_finite())
            || !desc.rotation.iter().all(|value| value.is_finite())
        {
            return Err(PhysicsError::InvalidBody(entity));
        }
        let role = self.bodies[index].role;
        if desc.shape.is_non_convex_mesh() {
            if role == BodyRole::Dynamic {
                return Err(self.refuse_non_convex_mesh(entity));
            }
            if desc.is_trigger {
                return Err(self.refuse_non_convex_trigger(entity));
            }
        }
        let material = desc.material.unwrap_or(PhysicsMaterial3D {
            friction: self.bodies[index].friction,
            restitution: self.bodies[index].restitution,
            friction_combine: self.bodies[index].friction_combine,
            restitution_combine: self.bodies[index].restitution_combine,
        });
        let layer = ColliderLayer {
            layer: desc.layer as usize,
            include: self.bodies[index].layer_include,
            exclude: self.bodies[index].layer_exclude,
            priority: self.bodies[index].layer_priority,
        };
        let handles = self.bodies[index].handles;
        if self
            .backend
            .add_collider(entity, handles, &desc, material, layer)
            .is_none()
        {
            return Err(PhysicsError::InvalidBody(entity));
        }
        self.note_convex_hull_limit(entity, &desc.shape);
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
        if desc.collider_shape.is_non_convex_mesh() {
            if role == BodyRole::Dynamic {
                return Err(self.refuse_non_convex_mesh(entity));
            }
            if desc.is_trigger {
                return Err(self.refuse_non_convex_trigger(entity));
            }
        }
        let Some(handles) = self.backend.add_body(entity, &desc, role) else {
            return Err(PhysicsError::InvalidBody(entity));
        };
        if desc.collision_detection_mode.uses_ccd_solver() {
            self.ccd_body_count += 1;
        }
        self.note_convex_hull_limit(entity, &desc.collider_shape);
        let index = self.bodies.len();
        self.bodies.push(Body {
            entity,
            handles,
            collider_shape: desc.collider_shape,
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
            friction_combine: FrictionCombine::Average,
            restitution_combine: BounceCombine::Average,
            collision_detection_mode: desc.collision_detection_mode,
            interpolation: BodyInterpolation::default(),
            max_angular_velocity: None,
            linear_drag: 0.0,
            angular_drag: 0.0,
            layer_include: None,
            layer_exclude: None,
            layer_priority: 0,
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
        let sweep_distance = self.kinematic_sweep_distance();
        if sweep_distance > self.solver_settings.contact_offset {
            self.backend.set_prediction_distance(sweep_distance);
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
            self.prepare_collision_detection(delta_seconds);
            self.backend
                .step(self.gravity, delta_seconds, &self.collision_matrix);
            self.collect_collision_payloads();
        }
        if sweep_distance > self.solver_settings.contact_offset {
            self.backend
                .set_prediction_distance(self.solver_settings.contact_offset);
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
                        if let Some(max) = body.max_angular_velocity {
                            let clamped = clamp_angular_velocity(body.angular_velocity, max);
                            if clamped != body.angular_velocity {
                                body.angular_velocity = clamped;
                                self.backend
                                    .set_angular_velocity(body.handles, clamped, false);
                            }
                        }
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
        self.solve_joints_3d(delta_seconds);
        self.update_sleeping(delta_seconds);
        self.last_delta_seconds = delta_seconds;
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

    fn kinematic_sweep_distance(&self) -> f64 {
        let mut distance: f64 = 0.0;
        for body in &self.bodies {
            if !body.simulated || body.role != BodyRole::Kinematic {
                continue;
            }
            let Some(target) = body.pending_move_position else {
                continue;
            };
            let current = self.backend.position(body.handles);
            distance = distance.max(vector_length(subtract(target, current)));
        }
        distance
    }

    fn prepare_collision_detection(&mut self, delta_seconds: f64) {
        self.backend.set_ccd_solver_enabled(self.ccd_body_count > 0);
        let contact_offset = self.solver_settings.contact_offset;
        for body in &mut self.bodies {
            if body.role != BodyRole::Dynamic {
                continue;
            }
            if body.collision_detection_mode.uses_speculative_contacts() {
                let travel = delta_seconds * vector_length(body.velocity);
                let margin = travel.max(contact_offset);
                self.backend.set_soft_ccd_prediction(body.handles, margin);
            }
        }
    }

    fn collect_collision_payloads(&mut self) {
        self.collision_payloads.clear();
        for (first, second, mut payload) in self.backend.collision_payloads() {
            let first_velocity = self.effective_velocity(first).unwrap_or([0.0; 3]);
            let second_velocity = self.effective_velocity(second).unwrap_or([0.0; 3]);
            payload.relative_velocity = subtract(first_velocity, second_velocity);
            if payload.total_impulse <= 0.0 {
                self.derive_collision_impulse(first, second, &mut payload);
            }
            self.collision_payloads.insert((first, second), payload);
        }
    }

    fn derive_collision_impulse(
        &self,
        first: EntityId,
        second: EntityId,
        payload: &mut CollisionPayload,
    ) {
        let Some(normal) = payload.contacts.first().map(|contact| contact.normal) else {
            return;
        };
        let approach = dot(payload.relative_velocity, normal).abs();
        if approach <= 0.0 {
            return;
        }
        let inverse = self.body_inverse_mass(first) + self.body_inverse_mass(second);
        if inverse <= 0.0 {
            return;
        }
        let impulse = approach / inverse;
        let share = impulse / payload.contacts.len() as f64;
        for contact in &mut payload.contacts {
            contact.impulse = share;
        }
        payload.total_impulse = impulse;
    }

    fn body_inverse_mass(&self, entity: EntityId) -> f64 {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].inverse_mass())
            .unwrap_or(0.0)
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
        self.indices
            .get(&entity)
            .map(|index| self.backend.angular_inertia(self.bodies[*index].handles))
    }

    pub fn interpolated_pose(&self, entity: EntityId, alpha: f64) -> Option<([f64; 3], [f64; 4])> {
        let index = self.indices.get(&entity).copied()?;
        let body = &self.bodies[index];
        let alpha = alpha.clamp(0.0, 1.0);
        Some(match body.interpolation {
            BodyInterpolation::None => (body.position, body.rotation),
            BodyInterpolation::Interpolate => (
                lerp_vector(body.previous_position, body.position, alpha),
                slerp_rotation(body.previous_rotation, body.rotation, alpha),
            ),
            BodyInterpolation::Extrapolate => {
                let factor = alpha * self.last_delta_seconds;
                (
                    [
                        body.position[0] + body.velocity[0] * factor,
                        body.position[1] + body.velocity[1] * factor,
                        body.position[2] + body.velocity[2] * factor,
                    ],
                    body.rotation,
                )
            }
        })
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
        let v = match slot.max_angular_velocity {
            Some(max) => clamp_angular_velocity(v, max),
            None => v,
        };
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
        let previous = self.bodies[index].collision_detection_mode;
        if previous.uses_ccd_solver() && !mode.uses_ccd_solver() {
            self.ccd_body_count = self.ccd_body_count.saturating_sub(1);
        }
        if !previous.uses_ccd_solver() && mode.uses_ccd_solver() {
            self.ccd_body_count += 1;
        }
        self.bodies[index].collision_detection_mode = mode;
        let handles = self.bodies[index].handles;
        self.backend.set_collision_detection_mode(handles, mode);
        Ok(())
    }

    pub fn collision_detection_mode(&self, body: EntityId) -> Option<CollisionDetectionMode3D> {
        self.indices
            .get(&body)
            .map(|index| self.bodies[*index].collision_detection_mode)
    }

    pub fn set_solver_settings(&mut self, settings: SolverSettings) {
        self.solver_settings = settings;
        self.backend.set_solver_settings(settings);
    }

    pub fn solver_settings(&self) -> SolverSettings {
        self.solver_settings
    }

    pub fn solver_type(&self) -> SolverType {
        self.solver_settings.solver_type
    }

    pub fn set_body_drag(&mut self, entity: EntityId, linear_drag: f64, angular_drag: f64) {
        let Some(index) = self.indices.get(&entity).copied() else {
            return;
        };
        let linear_drag = if linear_drag.is_finite() {
            linear_drag.max(0.0)
        } else {
            0.0
        };
        let angular_drag = if angular_drag.is_finite() {
            angular_drag.max(0.0)
        } else {
            0.0
        };
        self.bodies[index].linear_drag = linear_drag;
        self.bodies[index].angular_drag = angular_drag;
        let handles = self.bodies[index].handles;
        self.backend.set_drag(handles, linear_drag, angular_drag);
    }

    pub fn set_body_interpolation(&mut self, entity: EntityId, mode: BodyInterpolation) {
        let Some(index) = self.indices.get(&entity).copied() else {
            return;
        };
        self.bodies[index].interpolation = mode;
    }

    pub fn set_body_max_angular_velocity(&mut self, entity: EntityId, max: f64) {
        let Some(index) = self.indices.get(&entity).copied() else {
            return;
        };
        let max = if max.is_finite() {
            max.max(0.0)
        } else {
            f64::MAX
        };
        self.bodies[index].max_angular_velocity = Some(max);
        let clamped = clamp_angular_velocity(self.bodies[index].angular_velocity, max);
        self.bodies[index].angular_velocity = clamped;
        let handles = self.bodies[index].handles;
        self.backend.set_angular_velocity(handles, clamped, false);
    }

    pub fn set_body_layer_overrides(
        &mut self,
        entity: EntityId,
        include: Option<u32>,
        exclude: Option<u32>,
        priority: u32,
    ) {
        let Some(index) = self.indices.get(&entity).copied() else {
            return;
        };
        self.bodies[index].layer_include = include;
        self.bodies[index].layer_exclude = exclude;
        self.bodies[index].layer_priority = priority;
        let handles = self.bodies[index].handles;
        self.backend
            .set_layer_overrides(handles, include, exclude, priority);
    }

    pub fn collision_payload(&self, a: EntityId, b: EntityId) -> Option<&CollisionPayload> {
        self.collision_payloads.get(&ordered_pair(a, b))
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
        if removed.collision_detection_mode.uses_ccd_solver() {
            self.ccd_body_count = self.ccd_body_count.saturating_sub(1);
        }
        self.collision_payloads
            .retain(|(first, second), _| *first != entity && *second != entity);
        for slot in self.indices.values_mut() {
            if *slot > index {
                *slot -= 1;
            }
        }
        self.overlapping
            .retain(|(first, second)| *first != entity && *second != entity);
        self.overlapping_triggers
            .retain(|(first, second)| *first != entity && *second != entity);
        self.joints_3d
            .retain(|_, joint| joint.body_a() != entity && joint.body_b() != Some(entity));
        true
    }

    pub fn add_joint_3d(&mut self, joint: Joint3D) -> Result<JointId3D, PhysicsError> {
        let body_a = joint.body_a();
        if !self.indices.contains_key(&body_a) {
            return Err(PhysicsError::InvalidBody(body_a));
        }
        if let Some(body_b) = joint.body_b() {
            if !self.indices.contains_key(&body_b) || body_b == body_a {
                return Err(PhysicsError::InvalidBody(body_b));
            }
        }
        let id = JointId3D::new(self.next_joint_id_3d);
        self.next_joint_id_3d += 1;
        self.joints_3d.insert(id, joint);
        Ok(id)
    }

    pub fn remove_joint_3d(&mut self, id: JointId3D) -> bool {
        self.joints_3d.remove(&id).is_some()
    }

    pub fn joint_3d(&self, id: JointId3D) -> Option<&Joint3D> {
        self.joints_3d.get(&id)
    }

    pub fn joint_count_3d(&self) -> usize {
        self.joints_3d.len()
    }

    pub fn hinge_angle_3d(&self, id: JointId3D) -> Option<f64> {
        let Joint3D::Hinge(hj) = self.joints_3d.get(&id)? else {
            return None;
        };
        let idx_a = self.indices.get(&hj.body_a).copied()?;
        let rot_a = self.bodies[idx_a].rotation;
        let rot_b = match hj.body_b {
            Some(bid_b) => {
                let idx_b = self.indices.get(&bid_b).copied()?;
                self.bodies[idx_b].rotation
            }
            None => IDENTITY_ROTATION,
        };
        let axis_local = normalize(hj.axis).unwrap_or([1.0, 0.0, 0.0]);
        let rel_rot = match hj.body_b {
            Some(_) => multiply_rotation(inverse_rotation(rot_a), rot_b),
            None => rot_a,
        };
        Some(signed_twist_angle(rel_rot, axis_local))
    }

    pub fn drain_broken_joints_3d(&mut self) -> Vec<BrokenJointEvent3D> {
        std::mem::take(&mut self.broken_joints_3d)
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
                body.query_shape().is_some_and(|shape| {
                    overlaps_at(
                        dispatcher,
                        &probe,
                        center,
                        body.query_position,
                        body.query_rotation,
                        &shape,
                    )
                })
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
                body.query_shape().is_some_and(|shape| {
                    overlaps_at(
                        dispatcher,
                        &probe,
                        center,
                        body.query_position,
                        body.query_rotation,
                        &shape,
                    )
                })
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
                CastShape::Point => body.query_shape().and_then(|shape| {
                    cast_ray_against(
                        origin,
                        direction,
                        max_distance,
                        body.query_position,
                        body.query_rotation,
                        &shape,
                    )
                }),
                CastShape::Sphere { .. } | CastShape::Box { .. } => {
                    body.query_shape().and_then(|shape| {
                        cast_shape_against(
                            dispatcher,
                            cast_shape,
                            origin,
                            direction,
                            max_distance,
                            (body.query_position, body.query_rotation, &shape),
                        )
                    })
                }
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

    fn solve_joints_3d(&mut self, dt: f64) {
        if dt <= 0.0 || self.joints_3d.is_empty() {
            return;
        }
        let joint_entries: Vec<(JointId3D, Joint3D)> = self
            .joints_3d
            .iter()
            .map(|(id, j)| (*id, j.clone()))
            .collect();
        let mut broken: Vec<BrokenJointEvent3D> = Vec::new();
        let mut touched_indices: BTreeSet<usize> = BTreeSet::new();

        for (joint_id, joint) in joint_entries {
            match joint {
                Joint3D::Fixed(fj) => {
                    if let Some(ev) =
                        self.solve_fixed_joint_3d(joint_id, fj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Hinge(hj) => {
                    if let Some(ev) =
                        self.solve_hinge_joint_3d(joint_id, hj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Spring(sj) => {
                    if let Some(ev) =
                        self.solve_spring_joint_3d(joint_id, sj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Character(cj) => {
                    if let Some(ev) =
                        self.solve_character_joint_3d(joint_id, cj, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
                Joint3D::Configurable(cfg) => {
                    if let Some(ev) =
                        self.solve_configurable_joint_3d(joint_id, &cfg, dt, &mut touched_indices)
                    {
                        broken.push(ev);
                    }
                }
            }
        }

        for idx in touched_indices {
            if idx < self.bodies.len() {
                let body = &self.bodies[idx];
                if body.simulated && body.role == BodyRole::Dynamic {
                    let handles = body.handles;
                    self.backend.set_pose(handles, body.position, body.rotation);
                    self.backend.set_velocity(handles, body.velocity, true);
                    self.backend
                        .set_angular_velocity(handles, body.angular_velocity, true);
                }
            }
        }

        for ev in broken {
            self.joints_3d.remove(&ev.joint_id);
            self.broken_joints_3d.push(ev);
        }
    }

    fn body_state_3d(&self, entity: EntityId) -> Option<BodyState3D> {
        let idx = *self.indices.get(&entity)?;
        let b = &self.bodies[idx];
        if !b.simulated {
            return None;
        }
        Some((
            idx,
            b.position,
            b.rotation,
            b.velocity,
            b.angular_velocity,
            b.inverse_mass(),
        ))
    }

    fn solve_fixed_joint_3d(
        &mut self,
        joint_id: JointId3D,
        fj: FixedJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(fj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match fj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, fj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, fj.anchor_b)),
            None => fj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let rel_rot = multiply_rotation(inverse_rotation(rot_a), rot_b);
        let (rot_axis_local, rot_err) = rotation_axis_and_angle(rel_rot);
        let ang_diff = vector_length(subtract(ang_b, ang_a));
        let reaction_torque = (rot_err.abs() / (inv_sum * dt * dt)) + (ang_diff / (inv_sum * dt));

        if let Some(ev) = check_break_3d(
            joint_id,
            fj.body_a,
            reaction_force,
            reaction_torque,
            fj.break_force,
            fj.break_torque,
        ) {
            return Some(ev);
        }

        let w_a = inv_a / inv_sum;
        let w_b = inv_b / inv_sum;
        let avg_vel = scale3(add(scale3(vel_a, w_b), scale3(vel_b, w_a)), 1.0);
        let avg_ang = scale3(add(scale3(ang_a, w_b), scale3(ang_b, w_a)), 1.0);
        let target_rot = slerp_rotation(rot_a, rot_b, w_a);
        let _ = rot_axis_local;

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = target_rot;
            let new_anchor_offset_a = rotate_vector(b_a.rotation, fj.anchor_a);
            let target_anchor_world = add(world_a, scale3(pos_delta, w_a));
            b_a.position = subtract(target_anchor_world, new_anchor_offset_a);
            b_a.velocity = if inv_b == 0.0 { vel_b } else { avg_vel };
            b_a.angular_velocity = if inv_b == 0.0 { ang_b } else { avg_ang };
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = target_rot;
                let new_anchor_offset_b = rotate_vector(b_b.rotation, fj.anchor_b);
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                b_b.position = subtract(target_anchor_world, new_anchor_offset_b);
                b_b.velocity = if inv_a == 0.0 { vel_a } else { avg_vel };
                b_b.angular_velocity = if inv_a == 0.0 { ang_a } else { avg_ang };
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        }
        None
    }

    fn solve_hinge_joint_3d(
        &mut self,
        joint_id: JointId3D,
        hj: HingeJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(hj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match hj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let axis_local = normalize(hj.axis).unwrap_or([1.0, 0.0, 0.0]);
        let world_a = add(pos_a, rotate_vector(rot_a, hj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, hj.anchor_b)),
            None => hj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let ref_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_a
        } else {
            IDENTITY_ROTATION
        };
        let mov_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_b
        } else {
            rot_a
        };
        let axis_world = rotate_vector(ref_rot, axis_local);
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (twist_angle, swing_err) = decompose_twist_swing_angle(rel_rot, axis_local);

        let mov_ang = if opt_idx_b.is_some() && inv_b > 0.0 {
            ang_b
        } else {
            ang_a
        };
        let ref_ang = if opt_idx_b.is_some() && inv_b > 0.0 {
            ang_a
        } else {
            [0.0; 3]
        };
        let rel_ang_vel = dot(subtract(mov_ang, ref_ang), axis_world);
        let mut active_angle = twist_angle;
        let mut active_omega = rel_ang_vel;
        let mut applied_torque = swing_err / (inv_sum * dt * dt);

        let mov_inv = if opt_idx_b.is_some() && inv_b > 0.0 {
            inv_b
        } else {
            inv_a
        };

        if hj.use_spring {
            let spring_torque = hj.spring.spring * (hj.spring.target_position - active_angle)
                - hj.spring.damper * active_omega;
            applied_torque = applied_torque.max(spring_torque.abs());
            active_omega += spring_torque * mov_inv * dt;
            active_angle += spring_torque * mov_inv * dt * dt;
        }

        if hj.use_motor {
            let diff = hj.motor.target_velocity - active_omega;
            let should_drive = if hj.motor.free_spin {
                if hj.motor.target_velocity >= 0.0 {
                    diff > 0.0
                } else {
                    diff < 0.0
                }
            } else {
                true
            };
            if should_drive {
                let max_dw = hj.motor.force.max(0.0) * mov_inv * dt;
                let dw = diff.clamp(-max_dw, max_dw);
                let motor_torque = (dw / (mov_inv * dt)).abs();
                applied_torque = applied_torque.max(motor_torque);
                active_omega += dw;
                active_angle += active_omega * dt;
            }
        }

        if hj.use_limits {
            let low = hj.limits.min.min(hj.limits.max);
            let high = hj.limits.min.max(hj.limits.max);
            if active_angle < low {
                let limit_err = low - active_angle;
                applied_torque = applied_torque.max(limit_err / (mov_inv * dt * dt));
                active_angle = low;
                if active_omega < 0.0 {
                    active_omega = 0.0;
                }
            } else if active_angle > high {
                let limit_err = active_angle - high;
                applied_torque = applied_torque.max(limit_err / (mov_inv * dt * dt));
                active_angle = high;
                if active_omega > 0.0 {
                    active_omega = 0.0;
                }
            }
        }

        if let Some(ev) = check_break_3d(
            joint_id,
            hj.body_a,
            reaction_force,
            applied_torque,
            hj.break_force,
            hj.break_torque,
        ) {
            return Some(ev);
        }

        let desired_rel_rot = quat_from_axis_angle(axis_local, active_angle);
        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, desired_rel_rot));
        let desired_mov_ang = add(ref_ang, scale3(axis_world, active_omega));

        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let w_a = inv_a / inv_sum;
                let w_b = inv_b / inv_sum;
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                if inv_a > 0.0 {
                    let b_a = &mut self.bodies[idx_a];
                    let anchor_a_world = rotate_vector(b_a.rotation, hj.anchor_a);
                    b_a.position = subtract(add(world_a, scale3(pos_delta, w_a)), anchor_a_world);
                    b_a.sleeping = false;
                    b_a.sleep_timer = 0.0;
                    touched.insert(idx_a);
                }
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let new_anchor_b_world = rotate_vector(b_b.rotation, hj.anchor_b);
                b_b.position = subtract(target_anchor_world, new_anchor_b_world);
                b_b.angular_velocity = desired_mov_ang;
                let arm_b = new_anchor_b_world;
                b_b.velocity = add(vel_a, cross3(b_b.angular_velocity, scale3(arm_b, -1.0)));
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
                return None;
            }
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let new_anchor_a_world = rotate_vector(b_a.rotation, hj.anchor_a);
            b_a.position = subtract(world_b, new_anchor_a_world);
            b_a.angular_velocity = desired_mov_ang;
            b_a.velocity = cross3(b_a.angular_velocity, scale3(new_anchor_a_world, -1.0));
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }

    fn solve_spring_joint_3d(
        &mut self,
        joint_id: JointId3D,
        sj: SpringJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(sj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) = match sj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, sj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, sj.anchor_b)),
            None => sj.anchor_b,
        };
        let delta = subtract(world_b, world_a);
        let dist = vector_length(delta);
        let dir = normalize(delta).unwrap_or([1.0, 0.0, 0.0]);
        let min_d = sj.min_distance.max(0.0);
        let max_d = sj.max_distance.max(min_d);
        let tol = sj.tolerance.max(0.0);

        let stretch = if dist > max_d + tol {
            dist - max_d
        } else if dist < (min_d - tol).max(0.0) {
            dist - min_d
        } else {
            0.0
        };

        let rel_vel_along = dot(subtract(vel_b, vel_a), dir);
        let force_mag = sj.spring.max(0.0) * stretch + sj.damper.max(0.0) * rel_vel_along;

        if let Some(ev) = check_break_3d(
            joint_id,
            sj.body_a,
            force_mag.abs(),
            0.0,
            sj.break_force,
            sj.break_torque,
        ) {
            return Some(ev);
        }

        if force_mag.abs() <= 1e-12 {
            return None;
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            let dv_a = force_mag * inv_a * dt;
            b_a.velocity = add(b_a.velocity, scale3(dir, dv_a));
            b_a.position = add(b_a.position, scale3(dir, dv_a * dt));
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let b_b = &mut self.bodies[idx_b];
                let dv_b = -force_mag * inv_b * dt;
                b_b.velocity = add(b_b.velocity, scale3(dir, dv_b));
                b_b.position = add(b_b.position, scale3(dir, dv_b * dt));
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        }
        None
    }

    fn solve_character_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cj: CharacterJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, _ang_a, inv_a) = self.body_state_3d(cj.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, _ang_b, inv_b) = match cj.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let world_a = add(pos_a, rotate_vector(rot_a, cj.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, cj.anchor_b)),
            None => cj.anchor_b,
        };
        let pos_delta = subtract(world_b, world_a);
        let pos_err = vector_length(pos_delta);
        let vel_diff = vector_length(subtract(vel_b, vel_a));
        let reaction_force = (pos_err / (inv_sum * dt * dt)) + (vel_diff / (inv_sum * dt));

        let ref_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_a
        } else {
            IDENTITY_ROTATION
        };
        let mov_rot = if opt_idx_b.is_some() && inv_b > 0.0 {
            rot_b
        } else {
            rot_a
        };
        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (clamped_rel, clamped_angle_err) = clamp_character_rotation(
            rel_rot,
            cj.axis,
            cj.swing_axis,
            (cj.twist_limit_low, cj.twist_limit_high),
            (cj.swing1_limit, cj.swing2_limit),
        );
        let reaction_torque = clamped_angle_err / (inv_sum * dt * dt);

        if let Some(ev) = check_break_3d(
            joint_id,
            cj.body_a,
            reaction_force,
            reaction_torque,
            cj.break_force,
            cj.break_torque,
        ) {
            return Some(ev);
        }

        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, clamped_rel));

        if let Some(idx_b) = opt_idx_b {
            if inv_b > 0.0 {
                let w_a = inv_a / inv_sum;
                let w_b = inv_b / inv_sum;
                let target_anchor_world = subtract(world_b, scale3(pos_delta, w_b));
                if inv_a > 0.0 {
                    let b_a = &mut self.bodies[idx_a];
                    let anchor_a_world = rotate_vector(b_a.rotation, cj.anchor_a);
                    b_a.position = subtract(add(world_a, scale3(pos_delta, w_a)), anchor_a_world);
                    b_a.sleeping = false;
                    b_a.sleep_timer = 0.0;
                    touched.insert(idx_a);
                }
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let new_anchor_b_world = rotate_vector(b_b.rotation, cj.anchor_b);
                b_b.position = subtract(target_anchor_world, new_anchor_b_world);
                if clamped_angle_err > 1e-6 {
                    b_b.angular_velocity = [0.0; 3];
                }
                b_b.velocity = vel_a;
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
                return None;
            }
        }

        if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let new_anchor_a_world = rotate_vector(b_a.rotation, cj.anchor_a);
            b_a.position = subtract(world_b, new_anchor_a_world);
            if clamped_angle_err > 1e-6 {
                b_a.angular_velocity = [0.0; 3];
            }
            b_a.velocity = [0.0; 3];
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }

    fn solve_configurable_joint_3d(
        &mut self,
        joint_id: JointId3D,
        cfg: &ConfigurableJoint3D,
        dt: f64,
        touched: &mut BTreeSet<usize>,
    ) -> Option<BrokenJointEvent3D> {
        let (idx_a, pos_a, rot_a, vel_a, ang_a, inv_a) = self.body_state_3d(cfg.body_a)?;
        let (opt_idx_b, pos_b, rot_b, vel_b, ang_b, inv_b) = match cfg.body_b {
            Some(bid_b) => {
                let (ib, pb, rb, vb, ab, invb) = self.body_state_3d(bid_b)?;
                (Some(ib), pb, rb, vb, ab, invb)
            }
            None => (None, [0.0; 3], IDENTITY_ROTATION, [0.0; 3], [0.0; 3], 0.0),
        };
        let inv_sum = inv_a + inv_b;
        if inv_sum <= 0.0 {
            return None;
        }
        let mov_is_b = opt_idx_b.is_some() && inv_b > 0.0;
        let ref_rot = if mov_is_b { rot_a } else { IDENTITY_ROTATION };
        let mov_rot = if mov_is_b { rot_b } else { rot_a };
        let ref_vel = if mov_is_b { vel_a } else { [0.0; 3] };
        let mov_vel = if mov_is_b { vel_b } else { vel_a };
        let ref_ang = if mov_is_b { ang_a } else { [0.0; 3] };
        let mov_ang = if mov_is_b { ang_b } else { ang_a };
        let mov_inv = if mov_is_b { inv_b } else { inv_a };

        let world_a = add(pos_a, rotate_vector(rot_a, cfg.anchor_a));
        let world_b = match opt_idx_b {
            Some(_) => add(pos_b, rotate_vector(rot_b, cfg.anchor_b)),
            None => cfg.anchor_b,
        };
        let ref_anchor = if mov_is_b { world_a } else { world_b };
        let mov_anchor = if mov_is_b { world_b } else { world_a };

        let (basis_x_local, basis_y_local, basis_z_local) =
            orthonormal_frame(cfg.axis, cfg.secondary_axis);
        let basis_x = rotate_vector(ref_rot, basis_x_local);
        let basis_y = rotate_vector(ref_rot, basis_y_local);
        let basis_z = rotate_vector(ref_rot, basis_z_local);
        let bases = [basis_x, basis_y, basis_z];

        let rel_offset_world = subtract(mov_anchor, ref_anchor);
        let rel_vel_world = subtract(mov_vel, ref_vel);

        let mut coords = [
            dot(rel_offset_world, basis_x),
            dot(rel_offset_world, basis_y),
            dot(rel_offset_world, basis_z),
        ];
        let mut v_coords = [
            dot(rel_vel_world, basis_x),
            dot(rel_vel_world, basis_y),
            dot(rel_vel_world, basis_z),
        ];

        let motions = [cfg.x_motion, cfg.y_motion, cfg.z_motion];
        let drives = [cfg.x_drive, cfg.y_drive, cfg.z_drive];
        let lin_lim = cfg.linear_limit.max(0.0);
        let mut total_force_sq = 0.0f64;

        for axis_i in 0..3 {
            let drive = drives[axis_i];
            if motions[axis_i] != ConfigurableJointMotion::Locked
                && (drive.position_spring > 0.0 || drive.position_damper > 0.0)
            {
                let pos_err = cfg.target_position[axis_i] - coords[axis_i];
                let vel_err = cfg.target_velocity[axis_i] - v_coords[axis_i];
                let mut f = drive.position_spring * pos_err + drive.position_damper * vel_err;
                if drive.maximum_force.is_finite() {
                    let max_f = drive.maximum_force.max(0.0);
                    f = f.clamp(-max_f, max_f);
                }
                total_force_sq += f * f;
                v_coords[axis_i] += f * mov_inv * dt;
                coords[axis_i] += v_coords[axis_i] * dt;
            }

            match motions[axis_i] {
                ConfigurableJointMotion::Locked => {
                    let err = coords[axis_i].abs();
                    let f = (err / (mov_inv * dt * dt)) + (v_coords[axis_i].abs() / (mov_inv * dt));
                    total_force_sq += f * f;
                    coords[axis_i] = 0.0;
                    v_coords[axis_i] = 0.0;
                }
                ConfigurableJointMotion::Limited => {
                    if coords[axis_i] < -lin_lim {
                        let err = -lin_lim - coords[axis_i];
                        let f = err / (mov_inv * dt * dt);
                        total_force_sq += f * f;
                        coords[axis_i] = -lin_lim;
                        if v_coords[axis_i] < 0.0 {
                            v_coords[axis_i] = 0.0;
                        }
                    } else if coords[axis_i] > lin_lim {
                        let err = coords[axis_i] - lin_lim;
                        let f = err / (mov_inv * dt * dt);
                        total_force_sq += f * f;
                        coords[axis_i] = lin_lim;
                        if v_coords[axis_i] > 0.0 {
                            v_coords[axis_i] = 0.0;
                        }
                    }
                }
                ConfigurableJointMotion::Free => {}
            }
        }

        let rel_rot = multiply_rotation(inverse_rotation(ref_rot), mov_rot);
        let (clamped_rel_rot, ang_err, zero_ang) = clamp_configurable_rotation(
            rel_rot,
            (basis_x_local, basis_y_local, basis_z_local),
            (
                cfg.angular_x_motion,
                cfg.angular_y_motion,
                cfg.angular_z_motion,
            ),
            (
                cfg.angular_x_limit_low,
                cfg.angular_x_limit_high,
                cfg.angular_y_limit,
                cfg.angular_z_limit,
            ),
        );
        let reaction_torque = ang_err / (mov_inv * dt * dt);
        let reaction_force = total_force_sq.sqrt();

        if let Some(ev) = check_break_3d(
            joint_id,
            cfg.body_a,
            reaction_force,
            reaction_torque,
            cfg.break_force,
            cfg.break_torque,
        ) {
            return Some(ev);
        }

        let new_offset_world = add(
            add(scale3(bases[0], coords[0]), scale3(bases[1], coords[1])),
            scale3(bases[2], coords[2]),
        );
        let new_rel_vel_world = add(
            add(scale3(bases[0], v_coords[0]), scale3(bases[1], v_coords[1])),
            scale3(bases[2], v_coords[2]),
        );
        let desired_mov_anchor = add(ref_anchor, new_offset_world);
        let desired_mov_vel = add(ref_vel, new_rel_vel_world);
        let desired_mov_rot = normalize_rotation(multiply_rotation(ref_rot, clamped_rel_rot));
        let desired_mov_ang = if zero_ang { ref_ang } else { mov_ang };
        let _ = JointDrive3D::default();

        if mov_is_b {
            if let Some(idx_b) = opt_idx_b {
                let b_b = &mut self.bodies[idx_b];
                b_b.rotation = desired_mov_rot;
                let anchor_b_world = rotate_vector(b_b.rotation, cfg.anchor_b);
                b_b.position = subtract(desired_mov_anchor, anchor_b_world);
                b_b.velocity = desired_mov_vel;
                b_b.angular_velocity = desired_mov_ang;
                b_b.sleeping = false;
                b_b.sleep_timer = 0.0;
                touched.insert(idx_b);
            }
        } else if inv_a > 0.0 {
            let b_a = &mut self.bodies[idx_a];
            b_a.rotation = desired_mov_rot;
            let anchor_a_world = rotate_vector(b_a.rotation, cfg.anchor_a);
            b_a.position = subtract(desired_mov_anchor, anchor_a_world);
            b_a.velocity = desired_mov_vel;
            b_a.angular_velocity = desired_mov_ang;
            b_a.sleeping = false;
            b_a.sleep_timer = 0.0;
            touched.insert(idx_a);
        }
        None
    }
}

fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn quat_from_axis_angle(axis: [f64; 3], angle: f64) -> [f64; 4] {
    let Some(unit) = normalize(axis) else {
        return IDENTITY_ROTATION;
    };
    let half = 0.5 * angle;
    let s = half.sin();
    normalize_rotation([unit[0] * s, unit[1] * s, unit[2] * s, half.cos()])
}

fn rotation_axis_and_angle(q: [f64; 4]) -> ([f64; 3], f64) {
    let mut unit = normalize_rotation(q);
    if unit[3] < 0.0 {
        unit = [-unit[0], -unit[1], -unit[2], -unit[3]];
    }
    let vec_part = [unit[0], unit[1], unit[2]];
    let sin_half = vector_length(vec_part);
    if sin_half <= 1e-12 {
        return ([1.0, 0.0, 0.0], 0.0);
    }
    let angle = 2.0 * sin_half.atan2(unit[3].clamp(-1.0, 1.0));
    (scale3(vec_part, 1.0 / sin_half), angle)
}

fn signed_twist_angle(rel_rot: [f64; 4], axis_local: [f64; 3]) -> f64 {
    let mut unit = normalize_rotation(rel_rot);
    if unit[3] < 0.0 {
        unit = [-unit[0], -unit[1], -unit[2], -unit[3]];
    }
    let proj = dot([unit[0], unit[1], unit[2]], axis_local);
    2.0 * proj.atan2(unit[3])
}

fn decompose_twist_swing_angle(rel_rot: [f64; 4], axis_local: [f64; 3]) -> (f64, f64) {
    let mut unit = normalize_rotation(rel_rot);
    if unit[3] < 0.0 {
        unit = [-unit[0], -unit[1], -unit[2], -unit[3]];
    }
    let proj = dot([unit[0], unit[1], unit[2]], axis_local);
    let twist_vec = scale3(axis_local, proj);
    let twist_len = (dot(twist_vec, twist_vec) + unit[3] * unit[3]).sqrt();
    let twist_q = if twist_len > 1e-12 {
        [
            twist_vec[0] / twist_len,
            twist_vec[1] / twist_len,
            twist_vec[2] / twist_len,
            unit[3] / twist_len,
        ]
    } else {
        IDENTITY_ROTATION
    };
    let twist_angle = 2.0 * proj.atan2(unit[3]);
    let swing_q = multiply_rotation(unit, inverse_rotation(twist_q));
    let (_, swing_angle) = rotation_axis_and_angle(swing_q);
    (twist_angle, swing_angle.abs())
}

fn orthonormal_frame(primary: [f64; 3], secondary: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let x = normalize(primary).unwrap_or([1.0, 0.0, 0.0]);
    let sec_orth = subtract(secondary, scale3(x, dot(secondary, x)));
    let y = normalize(sec_orth).unwrap_or_else(|| {
        let fallback = if x[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let f_orth = subtract(fallback, scale3(x, dot(fallback, x)));
        normalize(f_orth).unwrap_or([0.0, 1.0, 0.0])
    });
    let z = cross3(x, y);
    (x, y, z)
}

fn clamp_character_rotation(
    rel_rot: [f64; 4],
    axis: [f64; 3],
    swing_axis: [f64; 3],
    twist_limits: (f64, f64),
    swing_limits: (f64, f64),
) -> ([f64; 4], f64) {
    let (x_axis, y_axis, z_axis) = orthonormal_frame(axis, swing_axis);
    let mut unit = normalize_rotation(rel_rot);
    if unit[3] < 0.0 {
        unit = [-unit[0], -unit[1], -unit[2], -unit[3]];
    }
    let v = [unit[0], unit[1], unit[2]];
    let proj_x = dot(v, x_axis);
    let twist_raw = [
        x_axis[0] * proj_x,
        x_axis[1] * proj_x,
        x_axis[2] * proj_x,
        unit[3],
    ];
    let twist_q = normalize_rotation(twist_raw);
    let swing_q = normalize_rotation(multiply_rotation(unit, inverse_rotation(twist_q)));

    let twist_angle = 2.0 * proj_x.atan2(unit[3]);
    let low_t = twist_limits.0.min(twist_limits.1);
    let high_t = twist_limits.0.max(twist_limits.1);
    let clamped_twist = twist_angle.clamp(low_t, high_t);
    let twist_err = (twist_angle - clamped_twist).abs();

    let (swing_rot_axis, swing_angle) = rotation_axis_and_angle(swing_q);
    let swing_vec = scale3(swing_rot_axis, swing_angle);
    let s1 = dot(swing_vec, y_axis);
    let s2 = dot(swing_vec, z_axis);
    let lim1 = swing_limits.0.max(0.0);
    let lim2 = swing_limits.1.max(0.0);

    let clamped_s1 = s1.clamp(-lim1, lim1);
    let clamped_s2 = s2.clamp(-lim2, lim2);
    let swing_err = ((s1 - clamped_s1).powi(2) + (s2 - clamped_s2).powi(2)).sqrt();

    let new_swing_vec = add(scale3(y_axis, clamped_s1), scale3(z_axis, clamped_s2));
    let new_swing_angle = vector_length(new_swing_vec);
    let new_swing_q = if new_swing_angle > 1e-12 {
        quat_from_axis_angle(new_swing_vec, new_swing_angle)
    } else {
        IDENTITY_ROTATION
    };
    let new_twist_q = quat_from_axis_angle(x_axis, clamped_twist);
    let combined = normalize_rotation(multiply_rotation(new_swing_q, new_twist_q));
    (combined, twist_err + swing_err)
}

fn clamp_configurable_rotation(
    rel_rot: [f64; 4],
    axes: ([f64; 3], [f64; 3], [f64; 3]),
    motions: (
        ConfigurableJointMotion,
        ConfigurableJointMotion,
        ConfigurableJointMotion,
    ),
    limits: (f64, f64, f64, f64),
) -> ([f64; 4], f64, bool) {
    let (x_axis, y_axis, z_axis) = axes;
    let (rot_axis, rot_angle) = rotation_axis_and_angle(rel_rot);
    let rot_vec = scale3(rot_axis, rot_angle);
    let mut rx = dot(rot_vec, x_axis);
    let mut ry = dot(rot_vec, y_axis);
    let mut rz = dot(rot_vec, z_axis);
    let orig = (rx, ry, rz);
    let mut clamped_any = false;

    match motions.0 {
        ConfigurableJointMotion::Locked => {
            if rx.abs() > 1e-6 {
                clamped_any = true;
            }
            rx = 0.0;
        }
        ConfigurableJointMotion::Limited => {
            let low = limits.0.min(limits.1);
            let high = limits.0.max(limits.1);
            let c = rx.clamp(low, high);
            if (c - rx).abs() > 1e-6 {
                clamped_any = true;
            }
            rx = c;
        }
        ConfigurableJointMotion::Free => {}
    }

    match motions.1 {
        ConfigurableJointMotion::Locked => {
            if ry.abs() > 1e-6 {
                clamped_any = true;
            }
            ry = 0.0;
        }
        ConfigurableJointMotion::Limited => {
            let lim = limits.2.max(0.0);
            let c = ry.clamp(-lim, lim);
            if (c - ry).abs() > 1e-6 {
                clamped_any = true;
            }
            ry = c;
        }
        ConfigurableJointMotion::Free => {}
    }

    match motions.2 {
        ConfigurableJointMotion::Locked => {
            if rz.abs() > 1e-6 {
                clamped_any = true;
            }
            rz = 0.0;
        }
        ConfigurableJointMotion::Limited => {
            let lim = limits.3.max(0.0);
            let c = rz.clamp(-lim, lim);
            if (c - rz).abs() > 1e-6 {
                clamped_any = true;
            }
            rz = c;
        }
        ConfigurableJointMotion::Free => {}
    }

    let err = ((rx - orig.0).powi(2) + (ry - orig.1).powi(2) + (rz - orig.2).powi(2)).sqrt();
    let new_vec = add(
        add(scale3(x_axis, rx), scale3(y_axis, ry)),
        scale3(z_axis, rz),
    );
    let new_angle = vector_length(new_vec);
    let new_q = if new_angle > 1e-12 {
        quat_from_axis_angle(new_vec, new_angle)
    } else {
        IDENTITY_ROTATION
    };
    let all_locked = motions.0 == ConfigurableJointMotion::Locked
        && motions.1 == ConfigurableJointMotion::Locked
        && motions.2 == ConfigurableJointMotion::Locked;
    (new_q, err, clamped_any || all_locked)
}

fn check_break_3d(
    joint_id: JointId3D,
    entity: EntityId,
    reaction_force: f64,
    reaction_torque: f64,
    break_force: f64,
    break_torque: f64,
) -> Option<BrokenJointEvent3D> {
    if break_force.is_finite() && reaction_force > break_force {
        return Some(BrokenJointEvent3D {
            joint_id,
            entity,
            break_force: reaction_force,
        });
    }
    if break_torque.is_finite() && reaction_torque > break_torque {
        return Some(BrokenJointEvent3D {
            joint_id,
            entity,
            break_force: reaction_torque,
        });
    }
    None
}
