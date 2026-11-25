//! Navigation data and agent locomotion: a navmesh built from a walkable
//! triangle soup, A* path queries over it, and the agents that walk the paths.

mod agent;
mod mesh;
mod path;

use std::fmt;

use unitas_core::EntityId;

pub use agent::NavMeshAgent;
pub use mesh::{
    NavMesh, NavMeshArea, NavMeshLink, NavMeshNeighbor, NavMeshPolygon, NavMeshTriangle,
};
pub use path::{PathQueryResult, PathQueryStatus, UnreachableReason};

/// The failures the navigation data and the agents report.
#[derive(Clone, Debug, PartialEq)]
pub enum NavigationError {
    DegenerateTriangle { index: usize },
    InvalidAreaCost { area: u32, cost: f64 },
    InvalidPosition { position: [f64; 3] },
    LinkEndpointOffNavMesh { position: [f64; 3] },
    DestinationOffNavMesh { position: [f64; 3] },
    AgentOffNavMesh { entity: EntityId, name: String },
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
                "navmesh agent '{name}' (entity {entity}) is not on the navmesh"
            ),
        }
    }
}

impl std::error::Error for NavigationError {}
