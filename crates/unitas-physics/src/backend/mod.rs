use std::collections::HashMap;

use rapier3d::dynamics::{
    CCDSolver, CoefficientCombineRule, ImpulseJointSet, IntegrationParameters, IslandManager,
    MultibodyJointSet, RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, SoftBodySet,
};
use rapier3d::geometry::{
    ActiveCollisionTypes, BroadPhaseBvh, ColliderBuilder, ColliderHandle, ColliderSet, NarrowPhase,
    SharedShape, SolverFlags,
};
use rapier3d::math::{Real, Rotation, Vector};
use rapier3d::pipeline::{
    ActiveHooks, CollisionPipeline, PairFilterContext, PhysicsHooks, PhysicsPipeline,
};
use unitas_core::EntityId;

use crate::body::{capsule_half_segment, BodyDesc, BodyRole, CollisionDetectionMode3D, Shape};
use crate::matrix::CollisionMatrix;
use crate::query::{pose_of, unit_rotation, Dispatcher};

type EntityPairs = Vec<(EntityId, EntityId)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BackendHandles {
    pub(crate) body: RigidBodyHandle,
    pub(crate) collider: ColliderHandle,
}

struct LayerHooks<'a> {
    layers: &'a HashMap<ColliderHandle, usize>,
    matrix: &'a CollisionMatrix,
}

impl LayerHooks<'_> {
    fn enabled(&self, context: &PairFilterContext) -> bool {
        let (Some(first), Some(second)) = (
            self.layers.get(&context.collider1),
            self.layers.get(&context.collider2),
        ) else {
            return true;
        };
        self.matrix.is_enabled(*first, *second)
    }
}

impl PhysicsHooks for LayerHooks<'_> {
    fn filter_contact_pair(&self, context: &PairFilterContext) -> Option<SolverFlags> {
        if self.enabled(context) {
            Some(SolverFlags::COMPUTE_RIGID_IMPULSES)
        } else {
            None
        }
    }

    fn filter_intersection_pair(&self, context: &PairFilterContext) -> bool {
        self.enabled(context)
    }
}

pub(crate) struct Backend {
    pipeline: PhysicsPipeline,
    collision_pipeline: CollisionPipeline,
    islands: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    soft_bodies: SoftBodySet,
    ccd_solver: CCDSolver,
    integration: IntegrationParameters,
    layers: HashMap<ColliderHandle, usize>,
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

fn collider_shape(shape: &Shape) -> SharedShape {
    match shape {
        Shape::Sphere { radius } => SharedShape::ball(*radius as Real),
        Shape::Box { half_extents } => SharedShape::cuboid(
            half_extents[0] as Real,
            half_extents[1] as Real,
            half_extents[2] as Real,
        ),
        Shape::Capsule {
            radius,
            height,
            direction,
        } => {
            let half_segment = capsule_half_segment(*radius, *height) as Real;
            let radius = *radius as Real;
            match direction {
                0 => SharedShape::capsule_x(half_segment, radius),
                2 => SharedShape::capsule_z(half_segment, radius),
                _ => SharedShape::capsule_y(half_segment, radius),
            }
        }
    }
}

fn rotation_of(quaternion: &Rotation) -> [f64; 4] {
    [
        f64::from(quaternion.x),
        f64::from(quaternion.y),
        f64::from(quaternion.z),
        f64::from(quaternion.w),
    ]
}

fn vector_of(vector: Vector) -> [f64; 3] {
    [
        f64::from(vector.x),
        f64::from(vector.y),
        f64::from(vector.z),
    ]
}

// Dynamic bodies start rotation-locked so resting and carried bodies stay put;
// the first angular actuation unlocks the body's angular degrees of freedom.
fn unlock_rotations(body: &mut RigidBody) {
    body.lock_rotations(false, false);
}

fn vector3(value: [f64; 3]) -> Vector {
    Vector::new(value[0] as Real, value[1] as Real, value[2] as Real)
}

impl Backend {
    pub(crate) fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            collision_pipeline: CollisionPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            soft_bodies: SoftBodySet::new(),
            ccd_solver: CCDSolver::new(),
            integration: IntegrationParameters::default(),
            layers: HashMap::new(),
        }
    }

    pub(crate) fn add_body(
        &mut self,
        entity: EntityId,
        desc: &BodyDesc,
        role: BodyRole,
    ) -> BackendHandles {
        let builder = match role {
            BodyRole::Dynamic => RigidBodyBuilder::dynamic(),
            BodyRole::Kinematic => RigidBodyBuilder::kinematic_position_based(),
            BodyRole::Static => RigidBodyBuilder::fixed(),
        };
        let mut builder = builder
            .pose(pose_of(desc.position, desc.rotation))
            .gravity_scale(if desc.use_gravity { 1.0 } else { 0.0 })
            .can_sleep(false)
            .ccd_enabled(desc.collision_detection_mode == CollisionDetectionMode3D::Continuous);
        if role == BodyRole::Dynamic {
            builder = builder.linvel(vector3(desc.velocity)).lock_rotations();
        }
        let body = self.bodies.insert(builder.build());
        let mut collider = ColliderBuilder::new(collider_shape(&desc.shape))
            .friction(desc.friction as Real)
            .restitution(desc.restitution as Real)
            .friction_combine_rule(CoefficientCombineRule::Average)
            .restitution_combine_rule(CoefficientCombineRule::Average)
            .sensor(desc.is_trigger)
            .active_hooks(ActiveHooks::FILTER_CONTACT_PAIRS | ActiveHooks::FILTER_INTERSECTION_PAIR)
            .user_data(u128::from(entity.as_u64()));
        if desc.is_trigger {
            collider = collider.active_collision_types(ActiveCollisionTypes::all());
        }
        if role == BodyRole::Dynamic {
            collider = collider.mass(desc.mass as Real);
        }
        let collider = self
            .colliders
            .insert_with_parent(collider, body, &mut self.bodies);
        self.layers.insert(collider, desc.layer as usize);
        BackendHandles { body, collider }
    }

    pub(crate) fn remove_body(&mut self, handles: BackendHandles) {
        self.layers.remove(&handles.collider);
        self.bodies.remove(
            handles.body,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.soft_bodies,
            true,
        );
    }

    pub(crate) fn step(&mut self, gravity: [f64; 3], delta_seconds: f64, matrix: &CollisionMatrix) {
        self.integration.dt = delta_seconds as Real;
        let hooks = LayerHooks {
            layers: &self.layers,
            matrix,
        };
        self.pipeline.step(
            vector3(gravity),
            &self.integration,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.soft_bodies,
            &mut self.ccd_solver,
            &hooks,
            &(),
        );
        let prediction_distance =
            self.integration.normalized_prediction_distance * self.integration.length_unit;
        self.collision_pipeline.step(
            prediction_distance,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &hooks,
            &(),
        );
    }

    pub(crate) fn touching_entity_pairs(&self) -> (EntityPairs, EntityPairs) {
        let entity_of = |handle: ColliderHandle| -> Option<EntityId> {
            self.colliders
                .get(handle)
                .map(|collider| EntityId::new(collider.user_data as u64))
        };
        let mut contacts = Vec::new();
        for pair in self.narrow_phase.contact_pairs() {
            if !pair.has_any_active_contact() {
                continue;
            }
            if let (Some(first), Some(second)) =
                (entity_of(pair.collider1), entity_of(pair.collider2))
            {
                contacts.push(if first <= second {
                    (first, second)
                } else {
                    (second, first)
                });
            }
        }
        let mut intersections = Vec::new();
        for (first, second, intersecting) in self.narrow_phase.intersection_pairs() {
            if !intersecting {
                continue;
            }
            if let (Some(a), Some(b)) = (entity_of(first), entity_of(second)) {
                intersections.push(if a <= b { (a, b) } else { (b, a) });
            }
        }
        (contacts, intersections)
    }

    pub(crate) fn dispatcher(&self) -> Dispatcher<'_> {
        self.narrow_phase.query_dispatcher()
    }

    pub(crate) fn position(&self, handles: BackendHandles) -> [f64; 3] {
        vector_of(self.bodies[handles.body].translation())
    }

    pub(crate) fn rotation(&self, handles: BackendHandles) -> [f64; 4] {
        rotation_of(self.bodies[handles.body].rotation())
    }

    pub(crate) fn velocity(&self, handles: BackendHandles) -> [f64; 3] {
        vector_of(self.bodies[handles.body].linvel())
    }

    pub(crate) fn angular_velocity(&self, handles: BackendHandles) -> [f64; 3] {
        vector_of(self.bodies[handles.body].angvel())
    }

    pub(crate) fn angular_inertia(&self, handles: BackendHandles) -> [f64; 3] {
        vector_of(
            self.bodies[handles.body]
                .mass_properties()
                .local_mprops
                .principal_inertia(),
        )
    }

    pub(crate) fn set_angular_velocity(
        &mut self,
        handles: BackendHandles,
        angular_velocity: [f64; 3],
        wake: bool,
    ) {
        if angular_velocity != [0.0; 3] {
            unlock_rotations(&mut self.bodies[handles.body]);
        }
        self.bodies[handles.body].set_angvel(vector3(angular_velocity), wake);
    }

    pub(crate) fn set_translation(
        &mut self,
        handles: BackendHandles,
        position: [f64; 3],
        wake: bool,
    ) {
        self.bodies[handles.body].set_translation(vector3(position), wake);
    }

    pub(crate) fn set_rotation(&mut self, handles: BackendHandles, rotation: [f64; 4], wake: bool) {
        self.bodies[handles.body].set_rotation(unit_rotation(rotation), wake);
    }

    pub(crate) fn set_pose(
        &mut self,
        handles: BackendHandles,
        position: [f64; 3],
        rotation: [f64; 4],
    ) {
        self.bodies[handles.body].set_position(pose_of(position, rotation), true);
    }

    pub(crate) fn set_next_kinematic_pose(
        &mut self,
        handles: BackendHandles,
        position: Option<[f64; 3]>,
        rotation: Option<[f64; 4]>,
    ) {
        let body = &mut self.bodies[handles.body];
        if let Some(target) = position {
            body.set_next_kinematic_translation(vector3(target));
        }
        if let Some(target) = rotation {
            body.set_next_kinematic_rotation(unit_rotation(target));
        }
    }

    pub(crate) fn set_velocity(&mut self, handles: BackendHandles, velocity: [f64; 3], wake: bool) {
        self.bodies[handles.body].set_linvel(vector3(velocity), wake);
    }

    pub(crate) fn set_collision_detection_mode(
        &mut self,
        handles: BackendHandles,
        mode: CollisionDetectionMode3D,
    ) {
        self.bodies[handles.body].enable_ccd(mode == CollisionDetectionMode3D::Continuous);
    }

    pub(crate) fn apply_forces(
        &mut self,
        handles: BackendHandles,
        force: [f64; 3],
        impulse: [f64; 3],
    ) {
        let body = &mut self.bodies[handles.body];
        body.reset_forces(false);
        if force != [0.0; 3] {
            body.add_force(vector3(force), false);
        }
        if impulse != [0.0; 3] {
            body.apply_impulse(vector3(impulse), false);
        }
    }

    pub(crate) fn apply_torque(&mut self, handles: BackendHandles, torque: [f64; 3]) {
        let body = &mut self.bodies[handles.body];
        body.reset_torques(false);
        if torque != [0.0; 3] {
            unlock_rotations(body);
            body.add_torque(vector3(torque), false);
        }
    }

    pub(crate) fn apply_torque_impulse(&mut self, handles: BackendHandles, impulse: [f64; 3]) {
        if impulse != [0.0; 3] {
            let body = &mut self.bodies[handles.body];
            unlock_rotations(body);
            body.apply_torque_impulse(vector3(impulse), false);
        }
    }

    pub(crate) fn apply_angular_acceleration(
        &mut self,
        handles: BackendHandles,
        acceleration: [f64; 3],
        delta_seconds: f64,
    ) {
        if acceleration == [0.0; 3] {
            return;
        }
        let body = &mut self.bodies[handles.body];
        unlock_rotations(body);
        let angular_velocity = body.angvel() + vector3(acceleration) * delta_seconds as Real;
        body.set_angvel(angular_velocity, false);
    }

    pub(crate) fn sleep(&mut self, handles: BackendHandles) {
        let body = &mut self.bodies[handles.body];
        body.set_linvel(Vector::ZERO, false);
        body.sleep();
    }

    pub(crate) fn wake_up(&mut self, handles: BackendHandles) {
        self.bodies[handles.body].wake_up(true);
    }

    pub(crate) fn is_sleeping(&self, handles: BackendHandles) -> bool {
        self.bodies[handles.body].is_sleeping()
    }

    pub(crate) fn set_shape(&mut self, handles: BackendHandles, shape: &Shape) {
        self.colliders[handles.collider].set_shape(collider_shape(shape));
    }

    pub(crate) fn set_sensor(&mut self, handles: BackendHandles, is_sensor: bool) {
        let collider = &mut self.colliders[handles.collider];
        collider.set_sensor(is_sensor);
        if is_sensor {
            collider.set_active_collision_types(ActiveCollisionTypes::all());
        }
    }

    pub(crate) fn set_enabled(&mut self, handles: BackendHandles, enabled: bool) {
        self.colliders[handles.collider].set_enabled(enabled);
    }

    pub(crate) fn set_material(
        &mut self,
        handles: BackendHandles,
        friction: f64,
        restitution: f64,
    ) {
        let collider = &mut self.colliders[handles.collider];
        collider.set_friction(friction as Real);
        collider.set_restitution(restitution as Real);
    }
}

#[cfg(test)]
mod tests {
    use unitas_core::EntityId;

    use crate::body::{BodyDesc, BodyRole, CollisionDetectionMode3D, Shape};
    use crate::world::World;

    fn run_boosted_sphere(delta_seconds: f64) -> Result<[f64; 3], crate::PhysicsError> {
        let mut world = World::new();
        world.add_body(
            EntityId::new(1),
            BodyDesc {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 0.0,
                velocity: [0.0; 3],
                use_gravity: false,
                is_trigger: false,
                shape: Shape::Box {
                    half_extents: [0.05, 2.0, 2.0],
                },
                layer: 0,
                role: BodyRole::Static,
                friction: 0.0,
                restitution: 0.0,
                collision_detection_mode: CollisionDetectionMode3D::Discrete,
            },
        )?;
        world.add_body(
            EntityId::new(2),
            BodyDesc {
                position: [-3.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 1.0,
                velocity: [200.0, 0.0, 0.0],
                use_gravity: false,
                is_trigger: false,
                shape: Shape::Sphere { radius: 0.25 },
                layer: 0,
                role: BodyRole::Dynamic,
                friction: 0.0,
                restitution: 0.0,
                collision_detection_mode: CollisionDetectionMode3D::Continuous,
            }
            .with_collision_detection_mode(CollisionDetectionMode3D::Continuous),
        )?;
        for _ in 0..5 {
            world.step(delta_seconds)?;
        }
        world
            .position(EntityId::new(2))
            .ok_or(crate::PhysicsError::InvalidBody(EntityId::new(2)))
    }

    #[test]
    fn continuous_sphere_does_not_tunnel_through_thin_wall() -> Result<(), crate::PhysicsError> {
        for delta_seconds in [0.02, 0.01] {
            let position = run_boosted_sphere(delta_seconds)?;
            assert!(
                position[0] < 0.0,
                "continuous sphere tunneled through the wall at dt {delta_seconds}: x = {}",
                position[0]
            );
        }
        Ok(())
    }
}
