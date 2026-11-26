use crate::event::Phase;

pub use crate::event::{FrameEvents, MessageTracker, TouchingPair};

pub const LAYER_COUNT: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionMatrix {
    enabled: [bool; LAYER_COUNT * LAYER_COUNT],
}

impl CollisionMatrix {
    pub fn new() -> Self {
        Self {
            enabled: [true; LAYER_COUNT * LAYER_COUNT],
        }
    }

    pub fn set_enabled(&mut self, a: usize, b: usize, enabled: bool) {
        if a >= LAYER_COUNT || b >= LAYER_COUNT {
            return;
        }
        self.enabled[a * LAYER_COUNT + b] = enabled;
        self.enabled[b * LAYER_COUNT + a] = enabled;
    }

    pub fn is_enabled(&self, a: usize, b: usize) -> bool {
        if a >= LAYER_COUNT || b >= LAYER_COUNT {
            return false;
        }
        self.enabled[a * LAYER_COUNT + b]
    }

    pub fn disable_pair(&mut self, a: usize, b: usize) {
        self.set_enabled(a, b, false);
    }

    pub fn ignore_layer_collision(&mut self, a: usize, b: usize, ignore: bool) {
        self.set_enabled(a, b, !ignore);
    }

    pub fn layer_mask_for_layer(&self, layer: usize) -> u32 {
        if layer >= LAYER_COUNT {
            return 0;
        }
        let mut mask = 0u32;
        for other in 0..LAYER_COUNT {
            if self.is_enabled(layer, other) {
                mask |= 1u32 << other;
            }
        }
        mask
    }
}

pub use crate::world::{PhysicsDiagnostic, PhysicsMaterial3D};

impl Default for CollisionMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicsObject {
    pub object_id: u64,
    pub collider_object_id: Option<u64>,
    pub rigidbody_object_id: Option<u64>,
    pub layer: usize,
    pub collider_layer: Option<usize>,
    pub has_rigidbody: bool,
    pub is_static: bool,
    pub is_kinematic: bool,
    pub is_trigger: bool,
    pub active: bool,
    pub behaviour_enabled: bool,
    pub ignored_object_ids: Vec<u64>,
}

impl PhysicsObject {
    pub fn new(object_id: u64, layer: usize) -> Self {
        Self {
            object_id,
            collider_object_id: None,
            rigidbody_object_id: None,
            layer,
            collider_layer: None,
            has_rigidbody: false,
            is_static: false,
            is_kinematic: false,
            is_trigger: false,
            active: true,
            behaviour_enabled: true,
            ignored_object_ids: Vec::new(),
        }
    }

    pub fn collider_owner(&self) -> u64 {
        self.collider_object_id.unwrap_or(self.object_id)
    }

    pub fn rigidbody_owner(&self) -> u64 {
        self.rigidbody_object_id.unwrap_or(self.object_id)
    }

    pub fn effective_layer(&self) -> usize {
        self.collider_layer.unwrap_or(self.layer)
    }

    pub fn is_dynamic(&self) -> bool {
        self.has_rigidbody && !self.is_static && !self.is_kinematic
    }

    pub fn ignores(&self, other_object_id: u64) -> bool {
        self.ignored_object_ids.contains(&other_object_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionMessage {
    OnCollisionEnter,
    OnCollisionStay,
    OnCollisionExit,
    OnTriggerEnter,
    OnTriggerStay,
    OnTriggerExit,
}

impl CollisionMessage {
    pub fn is_trigger(self) -> bool {
        matches!(
            self,
            Self::OnTriggerEnter | Self::OnTriggerStay | Self::OnTriggerExit
        )
    }

    pub fn phase(self) -> Phase {
        match self {
            Self::OnCollisionEnter | Self::OnTriggerEnter => Phase::Enter,
            Self::OnCollisionStay | Self::OnTriggerStay => Phase::Stay,
            Self::OnCollisionExit | Self::OnTriggerExit => Phase::Exit,
        }
    }

    pub fn from_phase(phase: Phase, is_trigger: bool) -> Self {
        match (phase, is_trigger) {
            (Phase::Enter, false) => Self::OnCollisionEnter,
            (Phase::Stay, false) => Self::OnCollisionStay,
            (Phase::Exit, false) => Self::OnCollisionExit,
            (Phase::Enter, true) => Self::OnTriggerEnter,
            (Phase::Stay, true) => Self::OnTriggerStay,
            (Phase::Exit, true) => Self::OnTriggerExit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageDelivery {
    pub message: CollisionMessage,
    pub receiver_object_id: u64,
    pub from_object_id: u64,
}

fn pair_allowed(a: &PhysicsObject, b: &PhysicsObject) -> bool {
    if !a.active || !b.active {
        return false;
    }
    if !(a.has_rigidbody || b.has_rigidbody) {
        return false;
    }
    if a.is_static && b.is_static {
        return false;
    }
    if a.is_kinematic && !b.is_dynamic() {
        return false;
    }
    if b.is_kinematic && !a.is_dynamic() {
        return false;
    }
    true
}

pub fn message_is_eligible(a: &PhysicsObject, b: &PhysicsObject, matrix: &CollisionMatrix) -> bool {
    if !matrix.is_enabled(a.effective_layer(), b.effective_layer()) {
        return false;
    }
    if a.ignores(b.object_id) || b.ignores(a.object_id) {
        return false;
    }
    pair_allowed(a, b)
}

fn receives(receiver: &PhysicsObject, other: &PhysicsObject, is_trigger: bool) -> bool {
    if !pair_allowed(receiver, other) {
        return false;
    }
    if is_trigger && receiver.is_trigger {
        return true;
    }
    if !receiver.has_rigidbody || receiver.is_static {
        return false;
    }
    true
}

fn recipients(object: &PhysicsObject) -> Vec<u64> {
    let mut owners = vec![object.collider_owner()];
    if object.has_rigidbody {
        let rigidbody_owner = object.rigidbody_owner();
        if !owners.contains(&rigidbody_owner) {
            owners.push(rigidbody_owner);
        }
    }
    owners
}

pub fn route_message(
    message: CollisionMessage,
    a: &PhysicsObject,
    b: &PhysicsObject,
) -> Vec<MessageDelivery> {
    let is_trigger = message.is_trigger();
    let mut deliveries = Vec::new();
    if receives(a, b, is_trigger) {
        deliveries.extend(
            recipients(a)
                .into_iter()
                .map(|receiver_object_id| MessageDelivery {
                    message,
                    receiver_object_id,
                    from_object_id: b.object_id,
                }),
        );
    }
    if receives(b, a, is_trigger) {
        deliveries.extend(
            recipients(b)
                .into_iter()
                .map(|receiver_object_id| MessageDelivery {
                    message,
                    receiver_object_id,
                    from_object_id: a.object_id,
                }),
        );
    }
    deliveries
}
