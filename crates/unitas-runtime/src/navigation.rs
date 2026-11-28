use std::collections::BTreeMap;
use std::fmt;

use unitas_asset::scene::NavMeshObstacleShape;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentExtraProps {
    pub enabled: bool,
    pub acceleration: f64,
    pub angular_speed: f64,
    pub stopping_distance: f64,
    pub height: f64,
    pub auto_braking: bool,
}

impl Default for AgentExtraProps {
    fn default() -> Self {
        Self {
            enabled: true,
            acceleration: 8.0,
            angular_speed: 120.0,
            stopping_distance: 0.0,
            height: 2.0,
            auto_braking: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeObstacleEntry {
    pub enabled: bool,
    pub shape: NavMeshObstacleShape,
    pub center: [f64; 3],
    pub size: [f64; 3],
    pub radius: f64,
    pub height: f64,
    pub carving: bool,
    pub carve_only_stationary: bool,
    pub is_stationary: bool,
    pub navmesh_index: Option<usize>,
}

impl Default for RuntimeObstacleEntry {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: NavMeshObstacleShape::Box,
            center: [0.0, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            radius: 0.5,
            height: 1.0,
            carving: false,
            carve_only_stationary: true,
            is_stationary: true,
            navmesh_index: None,
        }
    }
}

impl RuntimeObstacleEntry {
    pub fn half_extents(&self) -> [f64; 3] {
        match self.shape {
            NavMeshObstacleShape::Capsule => {
                let r = self.radius.max(0.01);
                let h = (self.height * 0.5).max(0.01);
                [r, h, r]
            }
            NavMeshObstacleShape::Box => [
                (self.size[0] * 0.5).max(0.01),
                (self.size[1] * 0.5).max(0.01),
                (self.size[2] * 0.5).max(0.01),
            ],
        }
    }
}

#[derive(Default)]
pub struct NavigationRuntimeWorld {
    navmesh: NavMesh,
    agents: BTreeMap<EntityId, NavMeshAgent>,
    agent_props: BTreeMap<EntityId, AgentExtraProps>,
    obstacles: BTreeMap<EntityId, RuntimeObstacleEntry>,
    surface_triangles: Vec<NavMeshTriangle>,
}

impl NavigationRuntimeWorld {
    pub fn new(navmesh: NavMesh) -> Self {
        Self {
            navmesh,
            agents: BTreeMap::new(),
            agent_props: BTreeMap::new(),
            obstacles: BTreeMap::new(),
            surface_triangles: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.navmesh = NavMesh::default();
        self.agents.clear();
        self.agent_props.clear();
        self.obstacles.clear();
        self.surface_triangles.clear();
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

    pub fn surface_triangles(&self) -> &[NavMeshTriangle] {
        &self.surface_triangles
    }

    pub fn set_surface_triangles(
        &mut self,
        triangles: Vec<NavMeshTriangle>,
    ) -> Result<(), NavigationRuntimeError> {
        let mut surface = NavMeshSurface::new();
        self.surface_triangles.clone_from(&triangles);
        self.rebuild_surface(&mut surface, triangles)
    }

    pub fn append_surface_triangles(
        &mut self,
        triangles: &[NavMeshTriangle],
    ) -> Result<(), NavigationRuntimeError> {
        if triangles.is_empty() {
            return Ok(());
        }
        self.surface_triangles.extend_from_slice(triangles);
        let all = self.surface_triangles.clone();
        let mut surface = NavMeshSurface::new();
        self.rebuild_surface(&mut surface, all)
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
        self.reapply_entity_obstacles();
        Ok(())
    }

    fn reapply_entity_obstacles(&mut self) {
        let entities: Vec<EntityId> = self.obstacles.keys().copied().collect();
        for entity in entities {
            if let Some(entry) = self.obstacles.get(&entity).copied() {
                let obstacle = NavMeshObstacle::carving_box(entry.center, entry.half_extents())
                    .with_carving(entry.carving)
                    .with_carve_only_stationary(entry.carve_only_stationary, entry.is_stationary)
                    .with_enabled(entry.enabled && entry.carving);
                if let Ok(index) = self.navmesh.add_obstacle(obstacle) {
                    if let Some(slot) = self.obstacles.get_mut(&entity) {
                        slot.navmesh_index = Some(index);
                    }
                }
            }
        }
    }

    pub fn upsert_entity_obstacle(
        &mut self,
        entity: EntityId,
        mut entry: RuntimeObstacleEntry,
    ) -> Result<usize, NavigationRuntimeError> {
        if let Some(existing) = self.obstacles.get(&entity) {
            if let Some(old_index) = existing.navmesh_index {
                self.navmesh.set_obstacle_enabled(old_index, false);
            }
        }
        let obstacle = NavMeshObstacle::carving_box(entry.center, entry.half_extents())
            .with_carving(entry.carving)
            .with_carve_only_stationary(entry.carve_only_stationary, entry.is_stationary)
            .with_enabled(entry.enabled && entry.carving);
        let index = self.add_obstacle(obstacle)?;
        entry.navmesh_index = Some(index);
        self.obstacles.insert(entity, entry);
        Ok(index)
    }

    pub fn entity_obstacle(&self, entity: EntityId) -> Option<&RuntimeObstacleEntry> {
        self.obstacles.get(&entity)
    }

    pub fn remove_entity_obstacle(&mut self, entity: EntityId) -> bool {
        let Some(removed) = self.obstacles.remove(&entity) else {
            return false;
        };
        if let Some(index) = removed.navmesh_index {
            self.navmesh.set_obstacle_enabled(index, false);
        }
        true
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
        self.agent_props.entry(entity).or_default();
        Ok(())
    }

    pub fn ensure_agent(&mut self, entity: EntityId, position: [f64; 3]) -> &mut NavMeshAgent {
        self.agent_props.entry(entity).or_default();
        self.agents
            .entry(entity)
            .or_insert_with(|| NavMeshAgent::new(entity, format!("Agent_{entity}"), position))
    }

    pub fn has_agent(&self, entity: EntityId) -> bool {
        self.agents.contains_key(&entity)
    }

    pub fn remove_agent(&mut self, entity: EntityId) -> bool {
        self.agent_props.remove(&entity);
        self.agents.remove(&entity).is_some()
    }

    pub fn agent(&self, entity: EntityId) -> Option<&NavMeshAgent> {
        self.agents.get(&entity)
    }

    pub fn agent_mut(&mut self, entity: EntityId) -> Option<&mut NavMeshAgent> {
        self.agents.get_mut(&entity)
    }

    pub fn agent_props(&self, entity: EntityId) -> Option<&AgentExtraProps> {
        self.agent_props.get(&entity)
    }

    pub fn agent_props_mut(&mut self, entity: EntityId) -> Option<&mut AgentExtraProps> {
        if !self.agents.contains_key(&entity) {
            return None;
        }
        Some(self.agent_props.entry(entity).or_default())
    }

    pub fn agent_entities(&self) -> Vec<EntityId> {
        self.agents.keys().copied().collect()
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
        let mut agent_list: Vec<NavMeshAgent> = self
            .agents
            .values()
            .filter(|agent| {
                self.agent_props
                    .get(&agent.entity())
                    .is_none_or(|props| props.enabled)
            })
            .cloned()
            .collect();
        NavMeshAgent::step_agents_with_avoidance(&mut agent_list, &self.navmesh, delta_seconds);
        for mut updated in agent_list {
            let stop_dist = self
                .agent_props
                .get(&updated.entity())
                .map_or(0.0, |props| props.stopping_distance);
            if stop_dist > 0.0 && updated.has_path() && updated.remaining_distance() <= stop_dist {
                updated.reset_path();
            }
            self.agents.insert(updated.entity(), updated);
        }
        Ok(())
    }
}
