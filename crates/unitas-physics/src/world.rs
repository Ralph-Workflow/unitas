use std::collections::{BTreeSet, HashMap};

use unitas_core::EntityId;

use crate::body::{BodyDesc, Shape, IDENTITY_ROTATION};
use crate::error::PhysicsError;
use crate::event::{CollisionEvent, Phase, TriggerEvent};
use crate::query::{cast_against, normalize, CastShape, QueryHit};

const DEFAULT_GRAVITY: [f64; 3] = [0.0, -9.81, 0.0];

type PairSet = BTreeSet<(EntityId, EntityId)>;

#[derive(Clone, Copy, Debug)]
struct Body {
    entity: EntityId,
    shape: Shape,
    mass: f64,
    use_gravity: bool,
    is_trigger: bool,
    position: [f64; 3],
    rotation: [f64; 4],
    velocity: [f64; 3],
    layer: u32,
}

pub struct World {
    gravity: [f64; 3],
    bodies: Vec<Body>,
    indices: HashMap<EntityId, usize>,
    overlapping: BTreeSet<(EntityId, EntityId)>,
    overlapping_triggers: BTreeSet<(EntityId, EntityId)>,
    events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,
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
            overlapping_triggers: BTreeSet::new(),
            events: Vec::new(),
            trigger_events: Vec::new(),
        }
    }

    pub fn gravity(&self) -> [f64; 3] {
        self.gravity
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
            use_gravity: desc.use_gravity,
            is_trigger: desc.is_trigger,
            position: desc.position,
            rotation: desc.rotation,
            velocity: desc.velocity,
            layer: desc.layer,
        });
        self.indices.insert(entity, index);
        Ok(())
    }

    pub fn step(&mut self, delta_seconds: f64) -> Result<(), PhysicsError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(PhysicsError::InvalidDeltaTime(delta_seconds));
        }
        self.integrate(delta_seconds);
        let (overlapping, overlapping_triggers) = self.overlapping_pairs();
        self.emit_events(&overlapping);
        self.emit_trigger_events(&overlapping_triggers);
        self.overlapping = overlapping;
        self.overlapping_triggers = overlapping_triggers;
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
        let delta = [
            end[0] - start[0],
            end[1] - start[1],
            end[2] - start[2],
        ];
        let Some(direction) = normalize(delta) else {
            return Vec::new();
        };
        let distance =
            (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
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
        self.bodies
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
            .collect()
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
                cast_shape
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
            if body.mass <= 0.0 {
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

    fn overlapping_pairs(&self) -> (PairSet, PairSet) {
        let mut pairs = BTreeSet::new();
        let mut trigger_pairs = BTreeSet::new();
        for (index, first) in self.bodies.iter().enumerate() {
            for second in self.bodies.iter().skip(index + 1) {
                if first.shape.overlaps(
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

    fn emit_events(&mut self, overlapping: &BTreeSet<(EntityId, EntityId)>) {
        for pair in overlapping.difference(&self.overlapping) {
            self.events.push(CollisionEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Enter,
            });
        }
        for pair in &self.overlapping {
            if overlapping.contains(pair) {
                self.events.push(CollisionEvent {
                    a: pair.0,
                    b: pair.1,
                    phase: Phase::Stay,
                });
            }
        }
        for pair in self.overlapping.difference(overlapping) {
            self.events.push(CollisionEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Exit,
            });
        }
    }

    fn emit_trigger_events(&mut self, overlapping_triggers: &BTreeSet<(EntityId, EntityId)>) {
        for pair in overlapping_triggers.difference(&self.overlapping_triggers) {
            self.trigger_events.push(TriggerEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Enter,
            });
        }
        for pair in &self.overlapping_triggers {
            if overlapping_triggers.contains(pair) {
                self.trigger_events.push(TriggerEvent {
                    a: pair.0,
                    b: pair.1,
                    phase: Phase::Stay,
                });
            }
        }
        for pair in self.overlapping_triggers.difference(overlapping_triggers) {
            self.trigger_events.push(TriggerEvent {
                a: pair.0,
                b: pair.1,
                phase: Phase::Exit,
            });
        }
    }
}
