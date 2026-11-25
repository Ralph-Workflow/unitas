use crate::entity::EntityId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Created,
    Destroyed,
    Reparented,
    TransformChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneChange {
    pub entity: EntityId,
    pub kind: ChangeKind,
    pub version: u64,
    pub parent: Option<EntityId>,
}
