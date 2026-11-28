use unitas_asset::scene::{
    NavMeshAgentComponent, NavMeshObstacleComponent, NavMeshSurfaceComponent, NavigationBinding,
    OffMeshLinkComponent,
};
use unitas_core::{EntityId, SceneGraph};
use unitas_physics::navigation::{NavMeshLink, NavMeshTriangle};

use super::Runtime;
use crate::navigation::{AgentExtraProps, RuntimeObstacleEntry};

impl Runtime {
    pub(crate) fn register_scene_navigation(
        &mut self,
        navigation: &NavigationBinding,
        entity_base: u64,
        graph: &SceneGraph,
    ) -> Vec<EntityId> {
        self.register_scene_navmesh_surfaces(&navigation.surfaces);
        self.register_scene_navmesh_obstacles(&navigation.obstacles, entity_base, graph);
        self.register_scene_off_mesh_links(&navigation.links, entity_base, graph);
        self.register_scene_navmesh_agents(&navigation.agents, entity_base, graph)
    }

    fn register_scene_navmesh_surfaces(&mut self, surfaces: &[NavMeshSurfaceComponent]) {
        let mut triangles = Vec::new();
        for surface in surfaces {
            if !surface.enabled {
                continue;
            }
            for tri in &surface.triangles {
                triangles.push(NavMeshTriangle::new(tri.vertices, tri.area));
            }
        }
        if !triangles.is_empty() {
            let _ = self.navigation.append_surface_triangles(&triangles);
        }
    }

    fn register_scene_navmesh_obstacles(
        &mut self,
        obstacles: &[NavMeshObstacleComponent],
        entity_base: u64,
        graph: &SceneGraph,
    ) {
        for obstacle in obstacles {
            let entity = EntityId::new(entity_base + obstacle.entity.as_u64());
            let world_tf = graph.world_transform(entity).unwrap_or_default();
            let world_center = [
                f64::from(world_tf.translation[0]) + obstacle.center[0],
                f64::from(world_tf.translation[1]) + obstacle.center[1],
                f64::from(world_tf.translation[2]) + obstacle.center[2],
            ];
            let entry = RuntimeObstacleEntry {
                enabled: obstacle.enabled,
                shape: obstacle.shape,
                center: world_center,
                size: obstacle.size,
                radius: obstacle.radius,
                height: obstacle.height,
                carving: obstacle.carving,
                carve_only_stationary: obstacle.carve_only_stationary,
                is_stationary: true,
                navmesh_index: None,
            };
            let _ = self.navigation.upsert_entity_obstacle(entity, entry);
        }
    }

    fn register_scene_off_mesh_links(
        &mut self,
        links: &[OffMeshLinkComponent],
        entity_base: u64,
        graph: &SceneGraph,
    ) {
        for link in links {
            if !link.enabled || !link.activated {
                continue;
            }
            let (Some(start_raw), Some(end_raw)) = (link.start, link.end) else {
                continue;
            };
            let start_entity = EntityId::new(entity_base + start_raw.as_u64());
            let end_entity = EntityId::new(entity_base + end_raw.as_u64());
            let (Some(start_tf), Some(end_tf)) = (
                graph.world_transform(start_entity),
                graph.world_transform(end_entity),
            ) else {
                continue;
            };
            let start_pos = [
                f64::from(start_tf.translation[0]),
                f64::from(start_tf.translation[1]),
                f64::from(start_tf.translation[2]),
            ];
            let end_pos = [
                f64::from(end_tf.translation[0]),
                f64::from(end_tf.translation[1]),
                f64::from(end_tf.translation[2]),
            ];
            let mut nav_link = NavMeshLink::new(start_pos, end_pos, link.bi_directional)
                .with_area(link.area_index)
                .with_activated(link.activated);
            if link.cost_override > 0.0 {
                nav_link = nav_link.with_cost_override(link.cost_override);
            }
            let _ = self.navigation.add_link(nav_link);
        }
    }

    fn register_scene_navmesh_agents(
        &mut self,
        agents: &[NavMeshAgentComponent],
        entity_base: u64,
        graph: &SceneGraph,
    ) -> Vec<EntityId> {
        let mut registered = Vec::with_capacity(agents.len());
        for comp in agents {
            let entity = EntityId::new(entity_base + comp.entity.as_u64());
            let world_tf = graph.world_transform(entity).unwrap_or_default();
            let pos = [
                f64::from(world_tf.translation[0]),
                f64::from(world_tf.translation[1]),
                f64::from(world_tf.translation[2]),
            ];
            let name = graph
                .entity(entity)
                .map_or_else(|| format!("Agent_{entity}"), |e| e.name.clone());
            if self.navigation.add_agent(entity, name, pos).is_ok() {
                if let Some(agent) = self.navigation.agent_mut(entity) {
                    agent.speed = comp.speed;
                    agent.radius = comp.radius;
                    agent.avoidance_priority = comp.avoidance_priority;
                    agent.auto_traverse_off_mesh_link = comp.auto_traverse_off_mesh_link;
                    agent.auto_repath = comp.auto_repath;
                    agent.area_mask = comp.walkable_mask;
                }
                if let Some(props) = self.navigation.agent_props_mut(entity) {
                    *props = AgentExtraProps {
                        enabled: comp.enabled,
                        acceleration: comp.acceleration,
                        angular_speed: comp.angular_speed,
                        stopping_distance: comp.stopping_distance,
                        height: comp.height,
                        auto_braking: comp.auto_braking,
                    };
                }
                registered.push(entity);
            }
        }
        registered
    }

    pub(crate) fn write_back_navigation_transforms(&mut self) {
        let entities = self.navigation.agent_entities();
        for entity in entities {
            if let Some(agent) = self.navigation.agent(entity) {
                if agent.update_position {
                    let pos = agent.position();
                    self.write_back_entity_transform(entity, pos);
                }
            }
        }
    }

    pub fn ensure_navmesh_agent(&mut self, entity: EntityId) {
        let pos = self.entity_transform(entity).map_or([0.0; 3], |tf| {
            [
                f64::from(tf.translation[0]),
                f64::from(tf.translation[1]),
                f64::from(tf.translation[2]),
            ]
        });
        let _ = self.navigation.ensure_agent(entity, pos);
    }

    pub fn warp_navmesh_agent(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
    ) -> Result<(), crate::navigation::NavigationRuntimeError> {
        self.ensure_navmesh_agent(entity);
        self.navigation.warp(entity, position)?;
        self.write_back_entity_transform(entity, position);
        Ok(())
    }

    pub fn move_navmesh_agent(&mut self, entity: EntityId, offset: [f64; 3]) -> bool {
        if !offset.iter().all(|c| c.is_finite()) {
            return false;
        }
        self.ensure_navmesh_agent(entity);
        let Some(current) = self.navigation.position(entity) else {
            return false;
        };
        let candidate = [
            current[0] + offset[0],
            current[1] + offset[1],
            current[2] + offset[2],
        ];
        let area_mask = self
            .navigation
            .agent(entity)
            .map_or(u32::MAX, |a| a.area_mask);
        let resolved = if self.navigation.navmesh().polygon_at(candidate).is_some() {
            candidate
        } else if let Some(hit) = self.navigation.sample_position(candidate, 1.0, area_mask) {
            hit.position
        } else {
            candidate
        };
        if let Some(agent) = self.navigation.agent_mut(entity) {
            let dest = agent.destination();
            let _ = agent.warp(resolved);
            if let Some(target) = dest {
                let navmesh = self.navigation.navmesh().clone();
                if let Some(agent_again) = self.navigation.agent_mut(entity) {
                    let _ = agent_again.set_destination(&navmesh, target);
                }
            }
        }
        self.write_back_entity_transform(entity, resolved);
        true
    }
}
