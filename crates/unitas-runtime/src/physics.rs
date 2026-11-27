use std::collections::HashMap;

use unitas_core::EntityId;
use unitas_physics::matrix::{
    CollisionMatrix, CollisionMessage, MessageDelivery, PhysicsObject, message_is_eligible,
    route_message,
};
use unitas_physics::two_d::body::{CollisionDetectionMode2D, RigidbodyInterpolation2D};
use unitas_physics::two_d::effector::BuoyancyEffector2D;
use unitas_physics::two_d::{
    AreaEffector2D, BodyDesc2D, BodyId2D, BodyType2D, ColliderDesc2D, ColliderId2D,
    ContactFilter2D, ContactPoint2D, Effector2D, PlatformEffector2D, PointEffector2D, RaycastHit2D,
    RoutedMessage2D, Shape2D, SurfaceEffector2D, World2D,
};
use unitas_physics::{
    BodyDesc, BodyRole, CollisionDetectionMode3D, CollisionEvent, Phase, PhysicsError, QueryHit,
    SolverSettings, TriggerEvent, World,
};

const FREEZE_ROTATION_CONSTRAINT_2D: u32 = 4;

pub(crate) fn layer_bits(layer: u32) -> u32 {
    if layer >= 32 {
        return layer;
    }
    1 << layer
}

fn encoded_layer_index(layer: u32) -> usize {
    layer_bits(layer) as usize
}

fn encoded_matrix(source: &CollisionMatrix) -> CollisionMatrix {
    let mut matrix = CollisionMatrix::new();
    for layer in 0..32usize {
        for other in 0..32usize {
            matrix.set_enabled(
                encoded_layer_index(layer as u32),
                encoded_layer_index(other as u32),
                source.is_enabled(layer, other),
            );
        }
    }
    matrix
}

fn encoded_object(object: &PhysicsObject) -> PhysicsObject {
    let mut encoded = object.clone();
    encoded.layer = layer_bits(object.layer as u32) as usize;
    encoded.collider_layer = object
        .collider_layer
        .map(|layer| layer_bits(layer as u32) as usize);
    encoded
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct ContactEvent {
    pub a: EntityId,
    pub b: EntityId,
    pub trigger: bool,
    pub phase: Phase,
    pub is_2d: bool,
    pub contacts: Vec<ContactPoint2D>,
    pub relative_velocity: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionPayload2D {
    pub impulse: f32,
    pub relative_velocity: [f32; 2],
    pub contacts: Vec<ContactPoint2D>,
}

#[derive(Default)]
pub struct PhysicsRuntimeWorld {
    world_3d: World,
    world_2d: World2D,
    interpolation_alpha_3d: f64,
    bodies_2d: HashMap<EntityId, BodyId2D>,
    colliders_2d: HashMap<EntityId, Vec<ColliderId2D>>,
}

impl PhysicsRuntimeWorld {
    pub fn new() -> Self {
        Self {
            world_3d: World::new(),
            world_2d: World2D::new(),
            interpolation_alpha_3d: 0.0,
            bodies_2d: HashMap::new(),
            colliders_2d: HashMap::new(),
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

    pub fn clear_2d(&mut self) {
        self.world_2d.clear_entities();
        self.bodies_2d.clear();
        self.colliders_2d.clear();
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

    pub fn interpolation_alpha_3d(&self) -> f64 {
        self.interpolation_alpha_3d
    }

    pub fn set_interpolation_alpha_3d(&mut self, alpha: f64) {
        self.interpolation_alpha_3d = alpha.clamp(0.0, 1.0);
    }

    pub fn presented_pose_3d(&self, entity: EntityId) -> Option<([f64; 3], [f64; 4])> {
        self.world_3d
            .interpolated_pose(entity, self.interpolation_alpha_3d)
    }

    pub fn set_layer_collision_3d(&mut self, layer_a: usize, layer_b: usize, enabled: bool) {
        let mut matrix = self.world_3d.collision_matrix().clone();
        matrix.set_enabled(
            encoded_layer_index(layer_a as u32),
            encoded_layer_index(layer_b as u32),
            enabled,
        );
        self.world_3d.set_collision_matrix(matrix);
    }

    pub fn set_layer_collision_matrix_3d(&mut self, source: &CollisionMatrix) {
        self.world_3d.set_collision_matrix(encoded_matrix(source));
    }

    pub fn layer_collision_enabled_3d(&self, layer_a: usize, layer_b: usize) -> bool {
        self.world_3d.collision_matrix().is_enabled(
            encoded_layer_index(layer_a as u32),
            encoded_layer_index(layer_b as u32),
        )
    }

    pub fn set_solver_settings_3d(&mut self, settings: SolverSettings) {
        self.world_3d.set_solver_settings(settings);
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
        if !message_is_eligible(
            &encoded_object(object_a),
            &encoded_object(object_b),
            self.world_3d.collision_matrix(),
        ) {
            return Vec::new();
        }
        route_message(message, object_a, object_b)
    }

    pub fn add_body_2d(&mut self, desc: BodyDesc2D) -> BodyId2D {
        let entity = desc.game_object;
        let id = self.world_2d.add_body(desc);
        self.bodies_2d.insert(entity, id);
        id
    }

    pub fn add_collider_2d(&mut self, desc: ColliderDesc2D) -> ColliderId2D {
        let entity = desc.game_object;
        let id = self.world_2d.add_collider(desc);
        self.colliders_2d.entry(entity).or_default().push(id);
        id
    }

    pub fn set_capsule_collider_2d(&mut self, entity: EntityId, desc: ColliderDesc2D) -> bool {
        let existing = self.colliders_2d.get(&entity).and_then(|ids| {
            ids.iter().copied().find(|id| {
                self.world_2d
                    .collider(*id)
                    .is_some_and(|collider| matches!(&collider.shape, Shape2D::Capsule { .. }))
            })
        });
        if let Some(existing) = existing {
            let Some(collider) = self.world_2d.collider_mut(existing) else {
                return false;
            };
            collider.shape = desc.shape;
            collider.is_trigger = desc.is_trigger;
            collider.layer = desc.layer;
            collider.attached_body = desc.attached_body;
            let _ = self.world_2d.set_collider_offset(existing, desc.offset);
            return true;
        }
        self.add_collider_2d(desc);
        true
    }

    pub fn set_polygon_collider_2d(&mut self, entity: EntityId, desc: ColliderDesc2D) -> bool {
        let existing = self.colliders_2d.get(&entity).and_then(|ids| {
            ids.iter().copied().find(|id| {
                self.world_2d
                    .collider(*id)
                    .is_some_and(|collider| matches!(&collider.shape, Shape2D::Polygon { .. }))
            })
        });
        if let Some(existing) = existing {
            let Some(collider) = self.world_2d.collider_mut(existing) else {
                return false;
            };
            collider.shape = desc.shape;
            collider.is_trigger = desc.is_trigger;
            collider.layer = desc.layer;
            collider.attached_body = desc.attached_body;
            let _ = self.world_2d.set_collider_offset(existing, desc.offset);
            return true;
        }
        self.add_collider_2d(desc);
        true
    }

    pub fn set_edge_collider_2d(&mut self, entity: EntityId, desc: ColliderDesc2D) -> bool {
        let existing = self.colliders_2d.get(&entity).and_then(|ids| {
            ids.iter().copied().find(|id| {
                self.world_2d.collider(*id).is_some_and(|collider| {
                    matches!(&collider.shape, Shape2D::Edge { .. })
                        || matches!(
                            &collider.shape,
                            Shape2D::Composite { shapes }
                                if !shapes.is_empty()
                                    && shapes.iter().all(|s| matches!(s, Shape2D::Edge { .. }))
                        )
                })
            })
        });
        if let Some(existing) = existing {
            let Some(collider) = self.world_2d.collider_mut(existing) else {
                return false;
            };
            collider.shape = desc.shape;
            collider.is_trigger = desc.is_trigger;
            collider.layer = desc.layer;
            collider.attached_body = desc.attached_body;
            let _ = self.world_2d.set_collider_offset(existing, desc.offset);
            return true;
        }
        self.add_collider_2d(desc);
        true
    }

    pub fn body_id_2d(&self, entity: EntityId) -> Option<BodyId2D> {
        self.bodies_2d.get(&entity).copied()
    }

    pub fn collider_ids_2d(&self, entity: EntityId) -> &[ColliderId2D] {
        self.colliders_2d
            .get(&entity)
            .map_or(&[][..], |ids| ids.as_slice())
    }

    pub fn collider_game_object_2d(&self, collider: ColliderId2D) -> Option<EntityId> {
        self.world_2d
            .collider(collider)
            .map(|collider| collider.game_object)
    }

    pub fn body_position_2d(&self, entity: EntityId) -> Option<[f32; 2]> {
        let id = self.body_id_2d(entity)?;
        self.world_2d.body(id).map(|body| body.position)
    }

    pub fn body_velocity_2d(&self, entity: EntityId) -> Option<[f32; 2]> {
        let id = self.body_id_2d(entity)?;
        self.world_2d.body(id).map(|body| body.linear_velocity)
    }

    pub fn set_body_velocity_2d(&mut self, entity: EntityId, velocity: [f32; 2]) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.linear_velocity = velocity;
        true
    }

    pub fn move_body_position_2d(&mut self, entity: EntityId, target: [f32; 2]) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.move_position(target);
        self.stick_touching_riders_to_kinematic_carrier_2d(id);
        true
    }

    fn stick_touching_riders_to_kinematic_carrier_2d(&mut self, carrier_id: BodyId2D) {
        let is_kinematic = self
            .world_2d
            .body(carrier_id)
            .is_some_and(|b| b.is_kinematic());
        if !is_kinematic {
            return;
        }
        let carrier_colliders: Vec<ColliderId2D> = self
            .colliders_2d
            .values()
            .flatten()
            .copied()
            .filter(|cid| {
                self.world_2d
                    .collider(*cid)
                    .is_some_and(|col| col.attached_body == Some(carrier_id))
            })
            .collect();
        let mut riders_to_stick = Vec::new();
        for other_id in self.bodies_2d.values().copied() {
            if other_id == carrier_id {
                continue;
            }
            let is_dynamic = self.world_2d.body(other_id).is_some_and(|b| b.is_dynamic());
            if !is_dynamic {
                continue;
            }
            let rider_colliders: Vec<ColliderId2D> = self
                .colliders_2d
                .values()
                .flatten()
                .copied()
                .filter(|cid| {
                    self.world_2d
                        .collider(*cid)
                        .is_some_and(|col| col.attached_body == Some(other_id))
                })
                .collect();
            let supported_from_below = rider_colliders.iter().any(|rc| {
                carrier_colliders
                    .iter()
                    .any(|cc| self.world_2d.is_touching(*rc, *cc))
                    && self
                        .world_2d
                        .get_contacts(*rc)
                        .iter()
                        .any(|c| c.normal[1] > 0.7)
            });
            if supported_from_below && self.world_2d.rider_carrier(other_id) != Some(carrier_id) {
                riders_to_stick.push(other_id);
            }
        }
        for rider_id in riders_to_stick {
            self.world_2d.stick_rider(rider_id, carrier_id);
        }
    }

    pub fn set_body_simulated_2d(&mut self, entity: EntityId, simulated: bool) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.simulated = simulated;
        true
    }

    pub fn set_body_type_2d(&mut self, entity: EntityId, body_type: BodyType2D) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.body_type = body_type;
        true
    }

    pub fn set_body_interpolation_2d(
        &mut self,
        entity: EntityId,
        interpolation: RigidbodyInterpolation2D,
    ) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.interpolation = interpolation;
        true
    }

    pub fn set_body_collision_detection_mode_2d(
        &mut self,
        entity: EntityId,
        mode: CollisionDetectionMode2D,
    ) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.collision_detection_mode = mode;
        true
    }

    pub fn set_body_constraints_2d(&mut self, entity: EntityId, constraints: u32) -> bool {
        let Some(id) = self.body_id_2d(entity) else {
            return false;
        };
        let Some(body) = self.world_2d.body_mut(id) else {
            return false;
        };
        body.freeze_rotation = constraints & FREEZE_ROTATION_CONSTRAINT_2D != 0;
        true
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

    pub fn step_2d_contacts(&mut self, delta_seconds: f32) -> Vec<ContactEvent> {
        let messages = self.step_2d(delta_seconds);
        messages
            .iter()
            .map(|message| ContactEvent {
                a: message.recipient,
                b: self
                    .world_2d
                    .collider(message.other_collider)
                    .map_or(message.recipient, |collider| collider.game_object),
                trigger: message.kind.is_trigger(),
                phase: message.kind.phase(),
                is_2d: true,
                contacts: message.contacts.clone(),
                relative_velocity: message.relative_velocity,
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

    pub fn box_cast_2d(
        &self,
        center: [f32; 2],
        half_extents: [f32; 2],
        angle: f32,
        direction: [f32; 2],
        distance: f32,
        filter: ContactFilter2D,
    ) -> Vec<RaycastHit2D> {
        self.world_2d
            .box_cast(center, half_extents, angle, direction, distance, filter)
    }

    pub fn overlap_point_2d(&self, point: [f32; 2], filter: ContactFilter2D) -> Vec<ColliderId2D> {
        self.world_2d.overlap_point(point, filter)
    }

    pub fn overlap_circle_2d(
        &self,
        center: [f32; 2],
        radius: f32,
        filter: ContactFilter2D,
    ) -> Vec<ColliderId2D> {
        self.world_2d.overlap_circle(center, radius, filter)
    }

    pub fn overlap_box_2d(
        &self,
        center: [f32; 2],
        half_extents: [f32; 2],
        angle: f32,
        filter: ContactFilter2D,
    ) -> Vec<ColliderId2D> {
        self.world_2d
            .overlap_box(center, half_extents, angle, filter)
    }

    pub fn matrix_3d(&self) -> &CollisionMatrix {
        self.world_3d.collision_matrix()
    }
}
