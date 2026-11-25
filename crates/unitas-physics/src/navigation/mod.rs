mod agent;
mod mesh;
mod path;

use std::fmt;

use unitas_core::EntityId;

pub use agent::{NavMeshAgent, NavMeshQueryFilter};
pub use mesh::{
    NavMesh, NavMeshArea, NavMeshLink, NavMeshNeighbor, NavMeshObstacle, NavMeshPolygon,
    NavMeshSurface, NavMeshTriangle,
};
pub use path::{NavMeshHit, PathQueryResult, PathQueryStatus, UnreachableReason};

#[derive(Clone, Debug, PartialEq)]
pub enum NavigationError {
    DegenerateTriangle {
        index: usize,
    },
    InvalidAreaCost {
        area: u32,
        cost: f64,
    },
    InvalidPosition {
        position: [f64; 3],
    },
    InvalidObstacle {
        center: [f64; 3],
        half_extents: [f64; 3],
    },
    LinkEndpointOffNavMesh {
        position: [f64; 3],
    },
    DestinationOffNavMesh {
        position: [f64; 3],
    },
    AgentOffNavMesh {
        entity: EntityId,
        name: String,
    },
}

impl fmt::Display for NavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DegenerateTriangle { index } => {
                write!(formatter, "navmesh triangle {index} has no area")
            }
            Self::InvalidAreaCost { area, cost } => {
                write!(formatter, "navmesh area {area} has an invalid cost {cost}")
            }
            Self::InvalidPosition { position } => {
                write!(formatter, "navmesh position {position:?} is not finite")
            }
            Self::InvalidObstacle {
                center,
                half_extents,
            } => write!(
                formatter,
                "navmesh obstacle at {center:?} has invalid half-extents {half_extents:?}"
            ),
            Self::LinkEndpointOffNavMesh { position } => {
                write!(
                    formatter,
                    "navmesh link endpoint {position:?} is not on the navmesh"
                )
            }
            Self::DestinationOffNavMesh { position } => {
                write!(
                    formatter,
                    "navmesh destination {position:?} is not on the navmesh"
                )
            }
            Self::AgentOffNavMesh { entity, name } => write!(
                formatter,
                "navmesh agent '{name}' (entity {entity}) is not on the navmesh for SetDestination"
            ),
        }
    }
}

impl std::error::Error for NavigationError {}
