use std::collections::{BTreeSet, HashMap};

use unitas_core::EntityId;

use crate::body::{BodyDesc, BodyRole, Shape, IDENTITY_ROTATION};
use crate::error::PhysicsError;
use crate::event::{CollisionEvent, Phase, TriggerEvent};
use crate::matrix::CollisionMatrix;
use crate::query::{cast_against, normalize, CastShape, QueryHit};

const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];
const SOLVER_ITERATIONS: usize = 4;

type PairSet = BTreeSet<(EntityId, EntityId)>;

#[derive(Clone, Copy, Debug)]
struct Body {
    entity: EntityId,
    shape: Shape,
    mass: f64,
    role: BodyRole,
    use_gravity: bool,
    is_trigger: bool,
    position: [f64; 3],
    rotation: [f64; 4],
    velocity: [f64; 3],
    layer: u32,
    friction: f64,
    restitution: f64,
    previous_position: [f64; 3],
    sweep_velocity: [f64; 3],
}

impl Body {
    fn inverse_mass(&self) -> f64 {
        match self.role {
            BodyRole::Dynamic if self.mass > 0.0 => 1.0 / self.mass,
            BodyRole::Dynamic | BodyRole::Kinematic | BodyRole::Static => 0.0,
        }
    }

    fn effective_velocity(&self) -> [f64; 3] {
        match self.role {
            BodyRole::Dynamic => self.velocity,
            BodyRole::Kinematic => self.sweep_velocity,
            BodyRole::Static => [0.0; 3],
        }
    }
}

pub struct World {
    gravity: [f64; 3],
    bodies: Vec<Body>,
    indices: HashMap<EntityId, usize>,
    overlapping: BTreeSet<(EntityId, EntityId)>,
    resolved_contacts: BTreeSet<(EntityId, EntityId)>,
    overlapping_triggers: BTreeSet<(EntityId, EntityId)>,
    events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,
    collision_matrix: CollisionMatrix,
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
            bodies: Vec::new(),
            indices: HashMap::new(),
            overlapping: BTreeSet::new(),
            resolved_contacts: BTreeSet::new(),
            overlapping_triggers: BTreeSet::new(),
            events: Vec::new(),
            trigger_events: Vec::new(),
            collision_matrix: CollisionMatrix::new(),
        }
    }

    pub fn gravity(&self) -> [f64; 3] {
        self.gravity
    }

    /// Replaces the layer collision matrix.
    pub fn set_collision_matrix(&mut self, matrix: CollisionMatrix) {
        self.collision_matrix = matrix;
    }

    /// The layer collision matrix that gates the collision and trigger callbacks.
    pub fn collision_matrix(&self) -> &CollisionMatrix {
        &self.collision_matrix
    }

    pub fn add_body(&mut self, entity: EntityId, desc: BodyDesc) -> Result<(), PhysicsError> {
        if self.indices.contains_key(&entity) {
            return Err(PhysicsError::DuplicateEntity(entity));
        }
        if !desc.is_valid() {
            return Err(PhysicsError::InvalidBody(entity));
        }
        let index = self.bodies.len();
        self.bodies.push(Body {
            entity,
            shape: desc.shape,
            mass: desc.mass,
            role: desc.effective_role(),
            use_gravity: desc.use_gravity,
            is_trigger: desc.is_trigger,
            position: desc.position,
            rotation: desc.rotation,
            velocity: desc.velocity,
            layer: desc.layer,
            friction: desc.friction,
            restitution: desc.restitution,
            previous_position: desc.position,
            sweep_velocity: [0.0; 3],
        });
        self.indices.insert(entity, index);
        Ok(())
    }

    pub fn step(&mut self, delta_seconds: f64) -> Result<(), PhysicsError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(PhysicsError::InvalidDeltaTime(delta_seconds));
        }
        self.integrate(delta_seconds);
        self.update_sweep_velocities(delta_seconds);
        let (mut overlapping, mut overlapping_triggers) = self.overlapping_pairs();
        let resolved = self.resolve_contacts();
        let (post_overlapping, post_triggers) = self.overlapping_pairs();
        overlapping.extend(post_overlapping);
        overlapping_triggers.extend(post_triggers);
        let mut active = overlapping.clone();
        active.extend(self.resolved_contacts.iter().copied());
        self.emit_events(&active);
        self.emit_trigger_events(&overlapping_triggers);
        let mut next_resolved = BTreeSet::new();
        for pair in resolved {
            if !self.overlapping.contains(&pair) {
                next_resolved.insert(pair);
            }
        }
        self.overlapping = active;
        self.resolved_contacts = next_resolved;
        self.overlapping_triggers = overlapping_triggers;
        self.commit_positions();
        Ok(())
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

    pub fn velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.indices
            .get(&entity)
            .map(|index| self.bodies[*index].velocity)
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
        Ok(())
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
        Ok(())
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn remove_body(&mut self, entity: EntityId) -> bool {
        let Some(index) = self.indices.remove(&entity) else {
            return false;
        };
        self.bodies.remove(index);
        for slot in self.indices.values_mut() {
            if *slot > index {
                *slot -= 1;
            }
        }
        self.overlapping
            .retain(|(first, second)| *first != entity && *second != entity);
        self.resolved_contacts
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
        let probe = Shape::Sphere { radius };
        let mut entities: Vec<EntityId> = self
            .bodies
            .iter()
            .filter(|body| body.layer & layer_mask != 0)
            .filter(|body| !body.is_trigger || include_triggers)
            .filter(|body| {
                probe.overlaps(
                    center,
                    IDENTITY_ROTATION,
                    &body.shape,
                    body.position,
                    body.rotation,
                )
            })
            .map(|body| body.entity)
            .collect();
        entities.sort();
        entities
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
        if max_distance <= 0.0 || max_distance.is_nan() {
            return Vec::new();
        }
        let Some(direction) = normalize(direction) else {
            return Vec::new();
        };
        let mut hits = Vec::new();
        for body in &self.bodies {
            if body.layer & layer_mask == 0 {
                continue;
            }
            if body.is_trigger && !include_triggers {
                continue;
            }
            let Some(hit) = cast_against(
                origin,
                direction,
                body.position,
                body.rotation,
                &body.shape,
                cast_shape,
            ) else {
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

    fn integrate(&mut self, delta_seconds: f64) {
        for body in &mut self.bodies {
            if body.role != BodyRole::Dynamic {
                continue;
            }
            if body.use_gravity {
                body.velocity[0] += self.gravity[0] * delta_seconds;
                body.velocity[1] += self.gravity[1] * delta_seconds;
                body.velocity[2] += self.gravity[2] * delta_seconds;
            }
            body.position[0] += body.velocity[0] * delta_seconds;
            body.position[1] += body.velocity[1] * delta_seconds;
            body.position[2] += body.velocity[2] * delta_seconds;
        }
    }

    fn update_sweep_velocities(&mut self, delta_seconds: f64) {
        for body in &mut self.bodies {
            if body.role != BodyRole::Kinematic {
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

    fn commit_positions(&mut self) {
        for body in &mut self.bodies {
            body.previous_position = body.position;
        }
    }

    fn resolve_contacts(&mut self) -> PairSet {
        let mut resolved = BTreeSet::new();
        let count = self.bodies.len();
        for _ in 0..SOLVER_ITERATIONS {
            for first_index in 0..count {
                for second_index in (first_index + 1)..count {
                    let first = self.bodies[first_index];
                    let second = self.bodies[second_index];
                    if first.is_trigger || second.is_trigger {
                        continue;
                    }
                    let inv_a = first.inverse_mass();
                    let inv_b = second.inverse_mass();
                    let inv_sum = inv_a + inv_b;
                    if inv_sum <= 0.0 {
                        continue;
                    }
                    let Some(contact) = first.shape.contact(
                        first.position,
                        first.rotation,
                        &second.shape,
                        second.position,
                        second.rotation,
                    ) else {
                        continue;
                    };
                    let move_a = contact.depth * (inv_a / inv_sum);
                    let move_b = contact.depth * (inv_b / inv_sum);
                    self.bodies[first_index].position[0] -= contact.normal[0] * move_a;
                    self.bodies[first_index].position[1] -= contact.normal[1] * move_a;
                    self.bodies[first_index].position[2] -= contact.normal[2] * move_a;
                    self.bodies[second_index].position[0] += contact.normal[0] * move_b;
                    self.bodies[second_index].position[1] += contact.normal[1] * move_b;
                    self.bodies[second_index].position[2] += contact.normal[2] * move_b;

                    let first_velocity = self.bodies[first_index].effective_velocity();
                    let second_velocity = self.bodies[second_index].effective_velocity();
                    let relative_velocity = [
                        second_velocity[0] - first_velocity[0],
                        second_velocity[1] - first_velocity[1],
                        second_velocity[2] - first_velocity[2],
                    ];
                    let normal_speed = relative_velocity[0] * contact.normal[0]
                        + relative_velocity[1] * contact.normal[1]
                        + relative_velocity[2] * contact.normal[2];
                    if normal_speed < 0.0 {
                        let restitution = (first.restitution + second.restitution) / 2.0;
                        let friction = (first.friction + second.friction) / 2.0;
                        let impulse = -normal_speed / inv_sum * (1.0 + restitution);
                        self.bodies[first_index].velocity[0] -= contact.normal[0] * impulse * inv_a;
                        self.bodies[first_index].velocity[1] -= contact.normal[1] * impulse * inv_a;
                        self.bodies[first_index].velocity[2] -= contact.normal[2] * impulse * inv_a;
                        self.bodies[second_index].velocity[0] +=
                            contact.normal[0] * impulse * inv_b;
                        self.bodies[second_index].velocity[1] +=
                            contact.normal[1] * impulse * inv_b;
                        self.bodies[second_index].velocity[2] +=
                            contact.normal[2] * impulse * inv_b;
                        let tangential = [
                            relative_velocity[0] - contact.normal[0] * normal_speed,
                            relative_velocity[1] - contact.normal[1] * normal_speed,
                            relative_velocity[2] - contact.normal[2] * normal_speed,
                        ];
                        let tangential_speed = (tangential[0] * tangential[0]
                            + tangential[1] * tangential[1]
                            + tangential[2] * tangential[2])
                            .sqrt();
                        if tangential_speed > 0.0 {
                            let friction_impulse =
                                (tangential_speed / inv_sum).min(friction * impulse);
                            let direction = [
                                tangential[0] / tangential_speed,
                                tangential[1] / tangential_speed,
                                tangential[2] / tangential_speed,
                            ];
                            self.bodies[first_index].velocity[0] +=
                                direction[0] * friction_impulse * inv_a;
                            self.bodies[first_index].velocity[1] +=
                                direction[1] * friction_impulse * inv_a;
                            self.bodies[first_index].velocity[2] +=
                                direction[2] * friction_impulse * inv_a;
                            self.bodies[second_index].velocity[0] -=
                                direction[0] * friction_impulse * inv_b;
                            self.bodies[second_index].velocity[1] -=
                                direction[1] * friction_impulse * inv_b;
                            self.bodies[second_index].velocity[2] -=
                                direction[2] * friction_impulse * inv_b;
                        }
                        let pair = if first.entity < second.entity {
                            (first.entity, second.entity)
                        } else {
                            (second.entity, first.entity)
                        };
                        resolved.insert(pair);
                    }
                }
            }
        }
        resolved
    }

    fn overlapping_pairs(&self) -> (PairSet, PairSet) {
        let mut pairs = BTreeSet::new();
        let mut trigger_pairs = BTreeSet::new();
        for (index, first) in self.bodies.iter().enumerate() {
            for second in self.bodies.iter().skip(index + 1) {
                if first.shape.in_contact(
                    first.position,
                    first.rotation,
                    &second.shape,
                    second.position,
                    second.rotation,
                ) {
                    let pair = if first.entity < second.entity {
                        (first.entity, second.entity)
                    } else {
                        (second.entity, first.entity)
                    };
                    if first.is_trigger || second.is_trigger {
                        trigger_pairs.insert(pair);
                    } else {
                        pairs.insert(pair);
                    }
                }
            }
        }
        (pairs, trigger_pairs)
    }

    fn layer_pair_enabled(&self, pair: &(EntityId, EntityId)) -> bool {
        let (Some(first), Some(second)) = (self.indices.get(&pair.0), self.indices.get(&pair.1))
        else {
            return false;
        };
        self.collision_matrix
            .is_enabled(self.bodies[*first].layer as usize, self.bodies[*second].layer as usize)
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
