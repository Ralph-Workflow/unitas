use unitas_core::EntityId;

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
