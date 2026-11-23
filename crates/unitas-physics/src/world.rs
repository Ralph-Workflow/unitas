use std::collections::{BTreeSet, HashMap};

use unitas_core::EntityId;

use crate::body::{BodyDesc, Shape};
use crate::error::PhysicsError;
use crate::event::{CollisionEvent, Phase, TriggerEvent};

const GRAVITY: f64 = 9.81;

type PairSet = BTreeSet<(EntityId, EntityId)>;

#[derive(Clone, Copy, Debug)]
struct Body {
    entity: EntityId,
    shape: Shape,
    mass: f64,
    use_gravity: bool,
    is_trigger: bool,
    position: [f64; 3],
    velocity: [f64; 3],
}

#[derive(Default)]
pub struct World {
    bodies: Vec<Body>,
    indices: HashMap<EntityId, usize>,
    overlapping: BTreeSet<(EntityId, EntityId)>,
    overlapping_triggers: BTreeSet<(EntityId, EntityId)>,
    events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
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
            velocity: desc.velocity,
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

    fn integrate(&mut self, delta_seconds: f64) {
        for body in &mut self.bodies {
            if body.mass <= 0.0 {
                continue;
            }
            if body.use_gravity {
                body.velocity[1] -= GRAVITY * delta_seconds;
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
                if first.shape.overlaps(first.position, &second.shape, second.position) {
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
