use std::collections::{BTreeMap, BTreeSet};

use unitas_core::EntityId;

use crate::matrix::{
    message_is_eligible, route_message, CollisionMatrix, CollisionMessage, MessageDelivery,
    PhysicsObject,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Enter,
    Stay,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionEvent {
    pub a: EntityId,
    pub b: EntityId,
    pub phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEvent {
    pub a: EntityId,
    pub b: EntityId,
    pub phase: Phase,
}

/// A pair of colliders that are touching in the current frame.
#[derive(Clone, Copy, Debug)]
pub struct TouchingPair<'a> {
    pub a: &'a PhysicsObject,
    pub b: &'a PhysicsObject,
    pub is_trigger: bool,
}

/// The frame events that suppress the transitions Unity does not report.
#[derive(Clone, Debug, Default)]
pub struct FrameEvents {
    /// Objects that moved by an instantaneous position change.
    pub teleported: BTreeSet<u64>,
    /// Objects whose GameObject was deactivated in this frame.
    pub deactivated: BTreeSet<u64>,
    /// Objects whose GameObject was activated in this frame.
    pub activated: BTreeSet<u64>,
}

impl FrameEvents {
    /// Whether the pair reports nothing in this frame.
    ///
    /// A teleported body is not swept, a deactivated GameObject ends its contacts
    /// without an exit message, and an activated GameObject does not start one.
    pub fn suppresses(&self, a: u64, b: u64) -> bool {
        [a, b].iter().any(|object_id| {
            self.teleported.contains(object_id)
                || self.deactivated.contains(object_id)
                || self.activated.contains(object_id)
        })
    }
}

#[derive(Clone, Debug)]
struct TrackedPair {
    a: PhysicsObject,
    b: PhysicsObject,
    is_trigger: bool,
}

/// Tracks the touching pairs so the six messages fire on transitions only.
///
/// A pair that keeps touching reports a stay message, so a collider shape change
/// never re-fires the enter message of a pair that is already touching.
#[derive(Clone, Debug, Default)]
pub struct MessageTracker {
    touching: BTreeMap<(u64, u64), TrackedPair>,
}

impl MessageTracker {
    /// A tracker that has seen no contact yet.
    pub fn new() -> Self {
        Self {
            touching: BTreeMap::new(),
        }
    }

    /// Applies one frame of contacts and returns the messages the transition implies.
    pub fn update(
        &mut self,
        pairs: &[TouchingPair<'_>],
        matrix: &CollisionMatrix,
        events: &FrameEvents,
    ) -> Vec<MessageDelivery> {
        let mut deliveries = Vec::new();
        let mut current: BTreeMap<(u64, u64), TrackedPair> = BTreeMap::new();

        for pair in pairs {
            let (a, b) = (pair.a, pair.b);
            if !message_is_eligible(a, b, matrix) {
                continue;
            }
            let key = pair_key(a.object_id, b.object_id);
            let was_touching = self.touching.contains_key(&key);
            current.insert(
                key,
                TrackedPair {
                    a: a.clone(),
                    b: b.clone(),
                    is_trigger: pair.is_trigger,
                },
            );
            if events.suppresses(a.object_id, b.object_id) {
                continue;
            }
            let phase = if was_touching { Phase::Stay } else { Phase::Enter };
            deliveries.extend(route_message(
                CollisionMessage::from_phase(phase, pair.is_trigger),
                a,
                b,
            ));
        }

        for (key, tracked) in &self.touching {
            if current.contains_key(key) {
                continue;
            }
            if events.suppresses(key.0, key.1) {
                continue;
            }
            if !message_is_eligible(&tracked.a, &tracked.b, matrix) {
                continue;
            }
            deliveries.extend(route_message(
                CollisionMessage::from_phase(Phase::Exit, tracked.is_trigger),
                &tracked.a,
                &tracked.b,
            ));
        }

        self.touching = current;
        deliveries
    }
}

fn pair_key(a: u64, b: u64) -> (u64, u64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
