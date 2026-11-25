use std::collections::BTreeMap;
use std::fmt;

use unitas_core::EntityId;
use unitas_physics::navigation::{NavMesh, NavMeshAgent, NavigationError, PathQueryResult};

/// The failures the navigation runtime reports.
#[derive(Clone, Debug, PartialEq)]
pub enum NavigationRuntimeError {
    UnknownAgent(EntityId),
    DuplicateAgent(EntityId),
    InvalidDeltaTime(f64),
    Navigation(NavigationError),
}

impl fmt::Display for NavigationRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAgent(entity) => {
                write!(formatter, "no navmesh agent for entity {entity}")
            }
            Self::DuplicateAgent(entity) => {
                write!(formatter, "entity {entity} already has a navmesh agent")
            }
            Self::InvalidDeltaTime(delta_seconds) => write!(
                formatter,
                "delta seconds {delta_seconds} is not a finite non-negative number"
            ),
            Self::Navigation(error) => write!(formatter, "navigation error: {error}"),
        }
    }
}

impl std::error::Error for NavigationRuntimeError {}

impl From<NavigationError> for NavigationRuntimeError {
    fn from(error: NavigationError) -> Self {
        Self::Navigation(error)
    }
}

/// The navmesh and the agents bound to the scene entities that walk it.
#[derive(Default)]
pub struct NavigationRuntimeWorld {
    navmesh: NavMesh,
    agents: BTreeMap<EntityId, NavMeshAgent>,
}

impl NavigationRuntimeWorld {
    pub fn new(navmesh: NavMesh) -> Self {
        Self {
            navmesh,
            agents: BTreeMap::new(),
        }
    }

    pub fn navmesh(&self) -> &NavMesh {
        &self.navmesh
    }

    /// Binds an entity to a new agent that starts at the given position.
    pub fn add_agent(
        &mut self,
        entity: EntityId,
        name: impl Into<String>,
        position: [f64; 3],
    ) -> Result<(), NavigationRuntimeError> {
        if self.agents.contains_key(&entity) {
            return Err(NavigationRuntimeError::DuplicateAgent(entity));
        }
        self.agents
            .insert(entity, NavMeshAgent::new(entity, name, position));
        Ok(())
    }

    pub fn remove_agent(&mut self, entity: EntityId) -> bool {
        self.agents.remove(&entity).is_some()
    }

    pub fn agent(&self, entity: EntityId) -> Option<&NavMeshAgent> {
        self.agents.get(&entity)
    }

    pub fn agent_mut(&mut self, entity: EntityId) -> Option<&mut NavMeshAgent> {
        self.agents.get_mut(&entity)
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// The position of the entity transform.
    pub fn position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.agents.get(&entity).map(NavMeshAgent::position)
    }

    /// The position the entity reaches next.
    pub fn next_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.agents.get(&entity).map(NavMeshAgent::next_position)
    }

    pub fn is_on_navmesh(&self, entity: EntityId) -> bool {
        self.agents
            .get(&entity)
            .is_some_and(|agent| agent.is_on_navmesh(&self.navmesh))
    }

    /// Sets the destination of the agent bound to the entity.
    pub fn set_destination(
        &mut self,
        entity: EntityId,
        target: [f64; 3],
    ) -> Result<PathQueryResult, NavigationRuntimeError> {
        let agent = self
            .agents
            .get_mut(&entity)
            .ok_or(NavigationRuntimeError::UnknownAgent(entity))?;
        agent
            .set_destination(&self.navmesh, target)
            .map_err(NavigationRuntimeError::Navigation)
    }

    /// Moves the agent bound to the entity without needing a path.
    pub fn warp(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
    ) -> Result<(), NavigationRuntimeError> {
        let agent = self
            .agents
            .get_mut(&entity)
            .ok_or(NavigationRuntimeError::UnknownAgent(entity))?;
        agent
            .warp(position)
            .map_err(NavigationRuntimeError::Navigation)
    }

    /// Steps every agent by one tick.
    pub fn step(&mut self, delta_seconds: f64) -> Result<(), NavigationRuntimeError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(NavigationRuntimeError::InvalidDeltaTime(delta_seconds));
        }
        for agent in self.agents.values_mut() {
            agent.update(delta_seconds);
        }
        Ok(())
    }
}
