use unitas_core::EntityId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhysicsError {
    DuplicateEntity(EntityId),
    InvalidBody(EntityId),
    InvalidDeltaTime(f64),
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateEntity(entity) => write!(formatter, "entity {entity} already has a body"),
            Self::InvalidBody(entity) => {
                write!(formatter, "entity {entity} has an invalid body description")
            }
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite non-negative number"
            ),
        }
    }
}

impl std::error::Error for PhysicsError {}
