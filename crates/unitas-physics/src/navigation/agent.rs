use unitas_core::EntityId;

use crate::body::{add, subtract, vector_length};
use crate::navigation::{
    NavMesh, NavigationError, PathQueryResult, PathQueryStatus, UnreachableReason,
};

/// The walking speed of an agent that does not set one, in metres per second.
const DEFAULT_SPEED: f64 = 3.5;
/// The distance below which two positions are the same point.
const POSITION_EPSILON: f64 = 1e-9;

/// An agent that walks the navmesh towards a destination.
#[derive(Clone, Debug)]
pub struct NavMeshAgent {
    /// The walking speed in metres per second.
    pub speed: f64,
    /// Whether the agent moves its own transform while it walks.
    pub update_position: bool,
    entity: EntityId,
    name: String,
    position: [f64; 3],
    next_position: [f64; 3],
    velocity: [f64; 3],
    corners: Vec<[f64; 3]>,
    corner_index: usize,
}

impl NavMeshAgent {
    pub fn new(entity: EntityId, name: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            speed: DEFAULT_SPEED,
            update_position: true,
            entity,
            name: name.into(),
            position,
            next_position: position,
            velocity: [0.0; 3],
            corners: Vec::new(),
            corner_index: 0,
        }
    }

    pub fn entity(&self) -> EntityId {
        self.entity
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The position of the agent transform.
    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    /// The position the agent reaches next; it moves on even when the transform stays.
    pub fn next_position(&self) -> [f64; 3] {
        self.next_position
    }

    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    /// Whether the agent still has corners left to walk.
    pub fn has_path(&self) -> bool {
        self.corner_index < self.corners.len()
    }

    /// Whether the agent stands on the navmesh.
    pub fn is_on_navmesh(&self, navmesh: &NavMesh) -> bool {
        navmesh.polygon_at(self.position).is_some()
    }

    /// Sets the destination the agent walks to.
    pub fn set_destination(
        &mut self,
        navmesh: &NavMesh,
        target: [f64; 3],
    ) -> Result<PathQueryResult, NavigationError> {
        if !target.iter().all(|component| component.is_finite()) {
            return Err(NavigationError::InvalidPosition { position: target });
        }
        if !self.is_on_navmesh(navmesh) {
            return Err(NavigationError::AgentOffNavMesh {
                entity: self.entity,
                name: self.name.clone(),
            });
        }
        let result = navmesh.find_path(self.next_position, target);
        match result.status {
            PathQueryStatus::Complete | PathQueryStatus::Partial => {
                self.corners = result.corners.clone();
                self.corner_index = 1;
                Ok(result)
            }
            PathQueryStatus::Invalid => {
                if result.unreachable_reason == Some(UnreachableReason::StartNotOnNavMesh) {
                    Err(NavigationError::AgentOffNavMesh {
                        entity: self.entity,
                        name: self.name.clone(),
                    })
                } else {
                    Err(NavigationError::DestinationOffNavMesh { position: target })
                }
            }
        }
    }

    /// Moves the agent to a position without needing a path.
    pub fn warp(&mut self, position: [f64; 3]) -> Result<(), NavigationError> {
        if !position.iter().all(|component| component.is_finite()) {
            return Err(NavigationError::InvalidPosition { position });
        }
        self.position = position;
        self.next_position = position;
        self.velocity = [0.0; 3];
        self.corners.clear();
        self.corner_index = 0;
        Ok(())
    }

    /// Advances the agent along its path by one tick.
    pub fn update(&mut self, delta_seconds: f64) {
        if !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || !self.speed.is_finite()
            || self.speed <= 0.0
        {
            return;
        }
        if !self.has_path() {
            self.velocity = [0.0; 3];
            return;
        }

        let previous = self.next_position;
        let mut budget = self.speed * delta_seconds;
        let mut current = previous;
        while budget > 0.0 && self.corner_index < self.corners.len() {
            let target = self.corners[self.corner_index];
            let offset = subtract(target, current);
            let length = vector_length(offset);
            if length <= POSITION_EPSILON {
                self.corner_index += 1;
                continue;
            }
            if length <= budget {
                current = target;
                budget -= length;
                self.corner_index += 1;
            } else {
                current = add(
                    current,
                    [
                        offset[0] / length * budget,
                        offset[1] / length * budget,
                        offset[2] / length * budget,
                    ],
                );
                budget = 0.0;
            }
        }

        self.next_position = current;
        self.velocity = [
            (current[0] - previous[0]) / delta_seconds,
            (current[1] - previous[1]) / delta_seconds,
            (current[2] - previous[2]) / delta_seconds,
        ];
        if self.update_position {
            self.position = current;
        }
    }
}
