//! Unity's 3D collision action matrix and the routing of the six collision and trigger messages.

use crate::event::Phase;

pub use crate::event::{FrameEvents, MessageTracker, TouchingPair};

/// Unity has 32 layers and the project physics settings hold one matrix for them.
pub const LAYER_COUNT: usize = 32;

/// The layer collision matrix from the project physics settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionMatrix {
    enabled: [bool; LAYER_COUNT * LAYER_COUNT],
}

impl CollisionMatrix {
    /// Every layer pair collides, as in a project that never edited the matrix.
    pub fn new() -> Self {
        Self {
            enabled: [true; LAYER_COUNT * LAYER_COUNT],
        }
    }

    /// Sets the cell for a layer pair; Unity keeps the matrix symmetric.
    pub fn set_enabled(&mut self, a: usize, b: usize, enabled: bool) {
        if a >= LAYER_COUNT || b >= LAYER_COUNT {
            return;
        }
        self.enabled[a * LAYER_COUNT + b] = enabled;
        self.enabled[b * LAYER_COUNT + a] = enabled;
    }

    /// Whether the layer pair may collide; a layer outside the 32 Unity layers never does.
    pub fn is_enabled(&self, a: usize, b: usize) -> bool {
        if a >= LAYER_COUNT || b >= LAYER_COUNT {
            return false;
        }
        self.enabled[a * LAYER_COUNT + b]
    }

    /// Removes the layer pair from the matrix, which stops every callback for it.
    pub fn disable_pair(&mut self, a: usize, b: usize) {
        self.set_enabled(a, b, false);
    }

    /// Physics.IgnoreLayerCollision: the layer pair exchanges no messages.
    pub fn ignore_layer_collision(&mut self, a: usize, b: usize, ignore: bool) {
        self.set_enabled(a, b, !ignore);
    }

    /// Returns the bitmask of layers enabled to collide/query against `layer`
    /// under this matrix (0 when `layer >= LAYER_COUNT`).
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

pub use crate::world::{MeshColliderDesc, PhysicsDiagnostic, PhysicsMaterial3D};

impl Default for CollisionMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// The minimal GameObject and Rigidbody model the message rules operate on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicsObject {
    /// The GameObject the description is about.
    pub object_id: u64,
    /// The GameObject owning the collider when it is not `object_id`.
    pub collider_object_id: Option<u64>,
    /// The GameObject owning the Rigidbody when it is not `object_id`.
    pub rigidbody_object_id: Option<u64>,
    /// The layer of the GameObject.
    pub layer: usize,
    /// The layer the collider collides on, when it differs from the GameObject layer.
    pub collider_layer: Option<usize>,
    /// The object has a Rigidbody.
    pub has_rigidbody: bool,
    /// The body is static and never receives collision messages.
    pub is_static: bool,
    /// The body is kinematic and only receives messages from dynamic bodies.
    pub is_kinematic: bool,
    /// The collider is a trigger.
    pub is_trigger: bool,
    /// The GameObject is active in the hierarchy; a deactivated GameObject exchanges nothing.
    pub active: bool,
    /// The receiving MonoBehaviour is enabled; Unity delivers to disabled behaviours too.
    pub behaviour_enabled: bool,
    /// The objects this one ignores collisions with (Physics.IgnoreCollision).
    pub ignored_object_ids: Vec<u64>,
}

impl PhysicsObject {
    /// A collider without a Rigidbody on the given layer.
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

    /// The GameObject that carries the collider.
    pub fn collider_owner(&self) -> u64 {
        self.collider_object_id.unwrap_or(self.object_id)
    }

    /// The GameObject that carries the Rigidbody.
    pub fn rigidbody_owner(&self) -> u64 {
        self.rigidbody_object_id.unwrap_or(self.object_id)
    }

    /// The layer the collider collides on.
    pub fn effective_layer(&self) -> usize {
        self.collider_layer.unwrap_or(self.layer)
    }

    /// A dynamic body: it has a Rigidbody that neither static nor kinematic.
    pub fn is_dynamic(&self) -> bool {
        self.has_rigidbody && !self.is_static && !self.is_kinematic
    }

    /// Whether this object ignores collisions with the other object.
    pub fn ignores(&self, other_object_id: u64) -> bool {
        self.ignored_object_ids.contains(&other_object_id)
    }
}

/// The six collision and trigger messages Unity sends to eligible scripts.
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
    /// Whether the message is one of the three trigger messages.
    pub fn is_trigger(self) -> bool {
        matches!(
            self,
            Self::OnTriggerEnter | Self::OnTriggerStay | Self::OnTriggerExit
        )
    }

    /// The contact phase the message reports.
    pub fn phase(self) -> Phase {
        match self {
            Self::OnCollisionEnter | Self::OnTriggerEnter => Phase::Enter,
            Self::OnCollisionStay | Self::OnTriggerStay => Phase::Stay,
            Self::OnCollisionExit | Self::OnTriggerExit => Phase::Exit,
        }
    }

    /// The message for a contact phase, collision or trigger.
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

/// One message addressed to one GameObject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageDelivery {
    pub message: CollisionMessage,
    pub receiver_object_id: u64,
    pub from_object_id: u64,
}

/// The pair rules that hold whatever the layer matrix allows.
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

/// Whether the pair exchanges any collision or trigger message.
///
/// A message needs a Rigidbody on at least one side, the layer matrix must allow
/// the pair, and two static bodies exchange nothing. A kinematic body only
/// exchanges messages with a dynamic body.
pub fn message_is_eligible(a: &PhysicsObject, b: &PhysicsObject, matrix: &CollisionMatrix) -> bool {
    if !matrix.is_enabled(a.effective_layer(), b.effective_layer()) {
        return false;
    }
    if a.ignores(b.object_id) || b.ignores(a.object_id) {
        return false;
    }
    pair_allowed(a, b)
}

/// The messages one side of a pair receives.
///
/// Collision messages need a Rigidbody on the receiving side, and a kinematic body
/// only receives them from a dynamic body. A trigger collider receives trigger
/// messages even without a Rigidbody of its own.
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

/// The GameObjects a message about this object reaches.
///
/// Unity sends the message to the collider's GameObject and to the GameObject of
/// the attached Rigidbody, and it reaches disabled MonoBehaviours on both.
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

/// Routes one message to the GameObjects of the pair that receive it.
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
