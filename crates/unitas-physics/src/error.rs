use unitas_core::EntityId;

#[derive(Clone, Debug, PartialEq)]
pub enum PhysicsError {
    DuplicateEntity(EntityId),
    InvalidBody(EntityId),
    InvalidDeltaTime(f64),
    InvalidCharacterController,
    NonConvexMeshColliderOnDynamicBody {
        entity: EntityId,
        object_name: String,
    },
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateEntity(entity) => {
                write!(formatter, "entity {entity} already has a body")
            }
            Self::InvalidBody(entity) => {
                write!(formatter, "entity {entity} has an invalid body description")
            }
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite non-negative number"
            ),
            Self::InvalidCharacterController => {
                write!(formatter, "invalid character controller configuration")
            }
            Self::NonConvexMeshColliderOnDynamicBody {
                entity,
                object_name,
            } => write!(
                formatter,
                "Non-convex MeshCollider with non-kinematic Rigidbody is no longer supported since Unity 5. Object: '{object_name}' (entity {entity})"
            ),
        }
    }
}

impl std::error::Error for PhysicsError {}
