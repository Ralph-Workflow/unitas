use crate::entity::EntityId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    UnknownEntity(EntityId),
    AttachCycle { child: EntityId, parent: EntityId },
}

pub type SceneGraphError = CoreError;

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownEntity(id) => write!(formatter, "unknown entity {id:?}"),
            Self::AttachCycle { child, parent } => {
                write!(formatter, "attaching {child:?} under {parent:?} creates a cycle")
            }
        }
    }
}

impl std::error::Error for CoreError {}
