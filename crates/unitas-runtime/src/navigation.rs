use std::collections::BTreeMap;
use std::fmt;

use unitas_core::EntityId;
use unitas_physics::navigation::{
    NavMesh, NavMeshAgent, NavMeshHit, NavMeshLink, NavMeshObstacle, NavMeshQueryFilter,
    NavMeshSurface, NavMeshTriangle, NavigationError, PathQueryResult,
};

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

    pub fn navmesh_mut(&mut self) -> &mut NavMesh {
        &mut self.navmesh
    }

    pub fn set_navmesh(&mut self, navmesh: NavMesh) {
        self.navmesh = navmesh;
    }

    pub fn rebuild_surface(
        &mut self,
        surface: &mut NavMeshSurface,
        triangles: Vec<NavMeshTriangle>,
    ) -> Result<(), NavigationRuntimeError> {
        let built = surface
            .update_navmesh(triangles)
            .map_err(NavigationRuntimeError::Navigation)?;
        self.navmesh = built.clone();
        Ok(())
    }

    pub fn add_link(&mut self, link: NavMeshLink) -> Result<(), NavigationRuntimeError> {
        self.navmesh
            .add_link(link)
            .map_err(NavigationRuntimeError::Navigation)
    }

    pub fn add_obstacle(
        &mut self,
        obstacle: NavMeshObstacle,
    ) -> Result<usize, NavigationRuntimeError> {
        self.navmesh
            .add_obstacle(obstacle)
            .map_err(NavigationRuntimeError::Navigation)
    }

    pub fn set_area_cost(&mut self, area: u32, cost: f64) -> Result<(), NavigationRuntimeError> {
        self.navmesh
            .set_area_cost(area, cost)
            .map_err(NavigationRuntimeError::Navigation)
    }

    pub fn sample_position(
        &self,
        source: [f64; 3],
        max_distance: f64,
        area_mask: u32,
    ) -> Option<NavMeshHit> {
        self.navmesh
            .sample_position(source, max_distance, area_mask)
    }

    pub fn raycast(&self, source: [f64; 3], target: [f64; 3], area_mask: u32) -> NavMeshHit {
        self.navmesh.raycast(source, target, area_mask)
    }

    pub fn calculate_path(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        filter: &NavMeshQueryFilter,
    ) -> PathQueryResult {
        self.navmesh.calculate_path(start, goal, filter)
    }

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

    pub fn position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.agents.get(&entity).map(NavMeshAgent::position)
    }

    pub fn next_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.agents.get(&entity).map(NavMeshAgent::next_position)
    }

    pub fn is_on_navmesh(&self, entity: EntityId) -> bool {
        self.agents
            .get(&entity)
            .is_some_and(|agent| agent.is_on_navmesh(&self.navmesh))
    }

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

    pub fn step(&mut self, delta_seconds: f64) -> Result<(), NavigationRuntimeError> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(NavigationRuntimeError::InvalidDeltaTime(delta_seconds));
        }
        let mut agent_list: Vec<NavMeshAgent> = self.agents.values().cloned().collect();
        NavMeshAgent::step_agents_with_avoidance(&mut agent_list, &self.navmesh, delta_seconds);
        for updated in agent_list {
            self.agents.insert(updated.entity(), updated);
        }
        Ok(())
    }
}
