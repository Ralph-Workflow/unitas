use unitas_core::EntityId;
use unitas_physics::matrix::{
    CollisionMatrix, CollisionMessage, MessageDelivery, PhysicsObject, message_is_eligible,
    route_message,
};
use unitas_physics::two_d::effector::BuoyancyEffector2D;
use unitas_physics::two_d::{
    AreaEffector2D, BodyDesc2D, BodyId2D, BodyType2D, ColliderDesc2D, ColliderId2D,
    ContactFilter2D, Effector2D, PlatformEffector2D, PointEffector2D, RaycastHit2D,
    RoutedMessage2D, SurfaceEffector2D, World2D,
};
use unitas_physics::{
    BodyDesc, BodyRole, CollisionDetectionMode3D, CollisionEvent, Phase, PhysicsError, QueryHit,
    TriggerEvent, World,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsWorldSummary {
    pub body_count_3d: usize,
    pub gravity_3d: [f64; 3],
    pub gravity_2d: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodySnapshot3D {
    pub entity: EntityId,
    pub role: BodyRole,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodySnapshot2D {
    pub id: BodyId2D,
    pub game_object: EntityId,
    pub body_type: BodyType2D,
    pub position: [f32; 2],
    pub rotation: f32,
    pub linear_velocity: [f32; 2],
    pub simulated: bool,
    pub sleeping: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactEvent2D {
    pub a: EntityId,
    pub b: EntityId,
    pub trigger: bool,
    pub phase: Phase,
}

#[derive(Default)]
pub struct PhysicsRuntimeWorld {
    world_3d: World,
    world_2d: World2D,
}

impl PhysicsRuntimeWorld {
    pub fn new() -> Self {
        Self {
            world_3d: World::new(),
            world_2d: World2D::new(),
        }
    }

    pub fn world_3d(&self) -> &World {
        &self.world_3d
    }

    pub fn world_3d_mut(&mut self) -> &mut World {
        &mut self.world_3d
    }

    pub fn world_2d(&self) -> &World2D {
        &self.world_2d
    }

    pub fn world_2d_mut(&mut self) -> &mut World2D {
        &mut self.world_2d
    }

    pub fn summary(&self) -> PhysicsWorldSummary {
        PhysicsWorldSummary {
            body_count_3d: self.world_3d.body_count(),
            gravity_3d: self.world_3d.gravity(),
            gravity_2d: self.world_2d.gravity(),
        }
    }

    pub fn add_body_3d(&mut self, entity: EntityId, desc: BodyDesc) -> Result<(), PhysicsError> {
        self.world_3d.add_body(entity, desc)
    }

    pub fn set_collision_detection_mode_3d(
        &mut self,
        entity: EntityId,
        mode: CollisionDetectionMode3D,
    ) -> Result<(), PhysicsError> {
        self.world_3d.set_collision_detection_mode(entity, mode)
    }

    pub fn inspect_body_3d(&self, entity: EntityId) -> Option<BodySnapshot3D> {
        Some(BodySnapshot3D {
            entity,
            role: self.world_3d.body_role(entity)?,
            position: self.world_3d.position(entity)?,
            velocity: self.world_3d.velocity(entity)?,
        })
    }

    pub fn step_3d(
        &mut self,
        delta_seconds: f64,
    ) -> Result<(Vec<CollisionEvent>, Vec<TriggerEvent>), PhysicsError> {
        self.world_3d.step(delta_seconds)?;
        Ok((
            self.world_3d.drain_events(),
            self.world_3d.drain_trigger_events(),
        ))
    }

    pub fn set_layer_collision_3d(&mut self, layer_a: usize, layer_b: usize, enabled: bool) {
        let mut matrix = self.world_3d.collision_matrix().clone();
        matrix.set_enabled(layer_a, layer_b, enabled);
        self.world_3d.set_collision_matrix(matrix);
    }

    pub fn layer_collision_enabled_3d(&self, layer_a: usize, layer_b: usize) -> bool {
        self.world_3d
            .collision_matrix()
            .is_enabled(layer_a, layer_b)
    }

    pub fn raycast_3d(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
    ) -> Vec<QueryHit> {
        self.world_3d
            .raycast(origin, direction, max_distance, layer_mask)
    }

    pub fn route_3d_message(
        &self,
        message: CollisionMessage,
        object_a: &PhysicsObject,
        object_b: &PhysicsObject,
    ) -> Vec<MessageDelivery> {
        if !message_is_eligible(object_a, object_b, self.world_3d.collision_matrix()) {
            return Vec::new();
        }
        route_message(message, object_a, object_b)
    }

    pub fn add_body_2d(&mut self, desc: BodyDesc2D) -> BodyId2D {
        self.world_2d.add_body(desc)
    }

    pub fn add_collider_2d(&mut self, desc: ColliderDesc2D) -> ColliderId2D {
        self.world_2d.add_collider(desc)
    }

    pub fn set_effector_2d(&mut self, collider: ColliderId2D, effector: Effector2D) -> bool {
        if self.world_2d.collider(collider).is_none() {
            return false;
        }
        self.world_2d.clear_effectors(collider);
        self.world_2d.attach_effector(collider, effector);
        true
    }

    pub fn set_platform_effector_2d(
        &mut self,
        collider: ColliderId2D,
        effector: PlatformEffector2D,
    ) -> bool {
        self.set_effector_2d(collider, Effector2D::Platform(effector))
    }

    pub fn set_surface_effector_2d(
        &mut self,
        collider: ColliderId2D,
        effector: SurfaceEffector2D,
    ) -> bool {
        self.set_effector_2d(collider, Effector2D::Surface(effector))
    }

    pub fn set_area_effector_2d(
        &mut self,
        collider: ColliderId2D,
        effector: AreaEffector2D,
    ) -> bool {
        self.set_effector_2d(collider, Effector2D::Area(effector))
    }

    pub fn set_point_effector_2d(
        &mut self,
        collider: ColliderId2D,
        effector: PointEffector2D,
    ) -> bool {
        self.set_effector_2d(collider, Effector2D::Point(effector))
    }

    pub fn set_buoyancy_effector_2d(
        &mut self,
        collider: ColliderId2D,
        effector: BuoyancyEffector2D,
    ) -> bool {
        self.set_effector_2d(collider, Effector2D::Buoyancy(effector))
    }

    pub fn inspect_body_2d(&self, id: BodyId2D) -> Option<BodySnapshot2D> {
        let body = self.world_2d.body(id)?;
        Some(BodySnapshot2D {
            id,
            game_object: body.game_object,
            body_type: body.body_type,
            position: body.position,
            rotation: body.rotation,
            linear_velocity: body.linear_velocity,
            simulated: body.simulated,
            sleeping: body.sleeping,
        })
    }

    pub fn step_2d(&mut self, delta_seconds: f32) -> Vec<RoutedMessage2D> {
        self.world_2d.step(delta_seconds);
        self.world_2d.drain_messages()
    }

    pub fn step_2d_contacts(&mut self, delta_seconds: f32) -> Vec<ContactEvent2D> {
        let messages = self.step_2d(delta_seconds);
        messages
            .iter()
            .map(|message| ContactEvent2D {
                a: message.recipient,
                b: self
                    .world_2d
                    .collider(message.other_collider)
                    .map_or(message.recipient, |collider| collider.game_object),
                trigger: message.kind.is_trigger(),
                phase: message.kind.phase(),
            })
            .collect()
    }

    pub fn ignore_layer_collision_2d(&mut self, layer_a: u8, layer_b: u8, ignore: bool) {
        self.world_2d
            .ignore_layer_collision(layer_a, layer_b, ignore);
    }

    pub fn ignore_collision_2d(
        &mut self,
        collider_a: ColliderId2D,
        collider_b: ColliderId2D,
        ignore: bool,
    ) {
        self.world_2d
            .ignore_collision(collider_a, collider_b, ignore);
    }

    pub fn raycast_2d(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        filter: ContactFilter2D,
    ) -> Vec<RaycastHit2D> {
        self.world_2d
            .raycast(origin, direction, max_distance, filter)
    }

    pub fn matrix_3d(&self) -> &CollisionMatrix {
        self.world_3d.collision_matrix()
    }
}
