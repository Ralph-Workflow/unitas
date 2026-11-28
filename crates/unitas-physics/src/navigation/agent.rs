use std::cmp::Ordering;
use std::collections::BTreeMap;

use unitas_core::EntityId;

use crate::body::{add, distance, dot, rotate_vector, subtract, vector_length, IDENTITY_ROTATION};
use crate::navigation::mesh::{area_matches_mask, ALL_AREAS_MASK};
use crate::navigation::path::PathLinkTraversal;
use crate::navigation::{
    NavMesh, NavMeshLink, NavigationError, PathQueryResult, PathQueryStatus, UnreachableReason,
};

const DEFAULT_SPEED: f64 = 3.5;
const DEFAULT_RADIUS: f64 = 0.5;
const DEFAULT_AVOIDANCE_PRIORITY: u32 = 50;
const POSITION_EPSILON: f64 = 1e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct NavMeshQueryFilter {
    pub area_mask: u32,
    area_costs: BTreeMap<u32, f64>,
}

impl Default for NavMeshQueryFilter {
    fn default() -> Self {
        Self::new(ALL_AREAS_MASK)
    }
}

impl NavMeshQueryFilter {
    pub fn new(area_mask: u32) -> Self {
        Self {
            area_mask,
            area_costs: BTreeMap::new(),
        }
    }

    pub fn set_area_cost(&mut self, area: u32, cost: f64) -> Result<(), NavigationError> {
        if !cost.is_finite() || cost <= 0.0 {
            return Err(NavigationError::InvalidAreaCost { area, cost });
        }
        self.area_costs.insert(area, cost);
        Ok(())
    }

    pub fn get_area_cost(&self, area: u32) -> Option<f64> {
        self.area_costs.get(&area).copied()
    }

    pub fn area_costs(&self) -> &BTreeMap<u32, f64> {
        &self.area_costs
    }
}

#[derive(Clone, Debug)]
pub struct NavMeshAgent {
    pub speed: f64,
    pub radius: f64,
    pub avoidance_priority: u32,
    pub is_stopped: bool,
    pub update_position: bool,
    pub update_rotation: bool,
    pub update_up_axis: bool,
    pub auto_traverse_off_mesh_link: bool,
    pub auto_repath: bool,
    pub area_mask: u32,
    area_costs: BTreeMap<u32, f64>,
    entity: EntityId,
    name: String,
    position: [f64; 3],
    next_position: [f64; 3],
    rotation: [f64; 4],
    velocity: [f64; 3],
    desired_velocity: [f64; 3],
    destination: Option<[f64; 3]>,
    path_status: Option<PathQueryStatus>,
    unreachable_reason: Option<UnreachableReason>,
    corners: Vec<[f64; 3]>,
    corner_index: usize,
    link_traversals: Vec<PathLinkTraversal>,
    path_polygons: Vec<usize>,
    active_off_mesh_link: Option<NavMeshLink>,
    waiting_on_off_mesh_link: bool,
}

impl NavMeshAgent {
    pub fn new(entity: EntityId, name: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            speed: DEFAULT_SPEED,
            radius: DEFAULT_RADIUS,
            avoidance_priority: DEFAULT_AVOIDANCE_PRIORITY,
            is_stopped: false,
            update_position: true,
            update_rotation: true,
            update_up_axis: true,
            auto_traverse_off_mesh_link: true,
            auto_repath: true,
            area_mask: ALL_AREAS_MASK,
            area_costs: BTreeMap::new(),
            entity,
            name: name.into(),
            position,
            next_position: position,
            rotation: IDENTITY_ROTATION,
            velocity: [0.0; 3],
            desired_velocity: [0.0; 3],
            destination: None,
            path_status: None,
            unreachable_reason: None,
            corners: Vec::new(),
            corner_index: 0,
            link_traversals: Vec::new(),
            path_polygons: Vec::new(),
            active_off_mesh_link: None,
            waiting_on_off_mesh_link: false,
        }
    }

    pub fn entity(&self) -> EntityId {
        self.entity
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    pub fn next_position(&self) -> [f64; 3] {
        self.next_position
    }

    pub fn rotation(&self) -> [f64; 4] {
        self.rotation
    }

    pub fn set_rotation(&mut self, rotation: [f64; 4]) {
        self.rotation = rotation;
    }

    pub fn forward(&self) -> [f64; 3] {
        rotate_vector(self.rotation, [0.0, 0.0, 1.0])
    }

    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    pub fn desired_velocity(&self) -> [f64; 3] {
        self.desired_velocity
    }

    pub fn destination(&self) -> Option<[f64; 3]> {
        self.destination
    }

    pub fn path_status(&self) -> Option<PathQueryStatus> {
        self.path_status
    }

    pub fn unreachable_reason(&self) -> Option<UnreachableReason> {
        self.unreachable_reason
    }

    pub fn corners(&self) -> &[[f64; 3]] {
        &self.corners
    }

    pub fn remaining_distance(&self) -> f64 {
        if !self.has_path() || self.corner_index >= self.corners.len() {
            return 0.0;
        }
        let mut total = distance(self.next_position, self.corners[self.corner_index]);
        for idx in self.corner_index..(self.corners.len() - 1) {
            total += distance(self.corners[idx], self.corners[idx + 1]);
        }
        total
    }

    pub fn set_area_cost(&mut self, area: u32, cost: f64) -> Result<(), NavigationError> {
        if !cost.is_finite() || cost <= 0.0 {
            return Err(NavigationError::InvalidAreaCost { area, cost });
        }
        self.area_costs.insert(area, cost);
        Ok(())
    }

    pub fn get_area_cost(&self, navmesh: &NavMesh, area: u32) -> f64 {
        self.area_costs
            .get(&area)
            .copied()
            .unwrap_or_else(|| navmesh.area_cost(area))
    }

    pub fn reset_path(&mut self) {
        self.destination = None;
        self.path_status = None;
        self.unreachable_reason = None;
        self.corners.clear();
        self.corner_index = 0;
        self.link_traversals.clear();
        self.path_polygons.clear();
        self.active_off_mesh_link = None;
        self.waiting_on_off_mesh_link = false;
        self.velocity = [0.0; 3];
        self.desired_velocity = [0.0; 3];
    }

    pub fn has_path(&self) -> bool {
        self.corner_index < self.corners.len() || self.waiting_on_off_mesh_link
    }

    pub fn is_path_stale(&self, navmesh: &NavMesh) -> bool {
        self.has_invalidated_link(navmesh)
            || self.path_polygons.iter().any(|poly| {
                navmesh.is_polygon_carved(*poly)
                    || navmesh
                        .polygons()
                        .get(*poly)
                        .is_none_or(|p| !area_matches_mask(p.area, self.area_mask))
            })
    }

    pub fn is_on_navmesh(&self, navmesh: &NavMesh) -> bool {
        navmesh.polygon_at(self.position).is_some()
    }

    pub fn is_on_off_mesh_link(&self) -> bool {
        self.active_off_mesh_link.is_some()
    }

    pub fn current_off_mesh_link(&self) -> Option<NavMeshLink> {
        self.active_off_mesh_link
    }

    pub fn complete_off_mesh_link(&mut self) -> bool {
        let Some(link) = self.active_off_mesh_link.take() else {
            return false;
        };
        self.waiting_on_off_mesh_link = false;
        self.next_position = link.end;
        if self.update_position {
            self.position = link.end;
        }
        if self.update_rotation {
            let dir = subtract(link.end, link.start);
            if let Some(rot) = look_rotation_xz(dir) {
                self.rotation = rot;
            }
        }
        while self.corner_index < self.corners.len() {
            let corner = self.corners[self.corner_index];
            self.corner_index += 1;
            if vector_length(subtract(corner, link.end)) <= POSITION_EPSILON {
                break;
            }
        }
        true
    }

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
        let result = navmesh.find_path_with_costs(
            self.next_position,
            target,
            self.area_mask,
            &self.area_costs,
        );
        match result.status {
            PathQueryStatus::Complete | PathQueryStatus::Partial => {
                self.destination = Some(target);
                self.path_status = Some(result.status);
                self.unreachable_reason = result.unreachable_reason;
                self.corners = result.corners.clone();
                self.corner_index = usize::from(self.corners.len() > 1);
                self.link_traversals = result.link_traversals.clone();
                self.path_polygons = result.polygons.clone();
                self.active_off_mesh_link = None;
                self.waiting_on_off_mesh_link = false;
                Ok(result)
            }
            PathQueryStatus::Invalid => {
                self.path_status = Some(PathQueryStatus::Invalid);
                self.unreachable_reason = result.unreachable_reason;
                self.corners.clear();
                self.corner_index = 0;
                self.link_traversals.clear();
                self.path_polygons.clear();
                self.active_off_mesh_link = None;
                self.waiting_on_off_mesh_link = false;
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

    pub fn warp(&mut self, position: [f64; 3]) -> Result<(), NavigationError> {
        if !position.iter().all(|component| component.is_finite()) {
            return Err(NavigationError::InvalidPosition { position });
        }
        self.position = position;
        self.next_position = position;
        self.reset_path();
        Ok(())
    }

    pub fn warp_on_navmesh(&mut self, navmesh: &NavMesh, position: [f64; 3]) -> bool {
        if !position.iter().all(|component| component.is_finite()) {
            return false;
        }
        let Some(hit) = navmesh.sample_position(position, self.radius.max(0.5), self.area_mask)
        else {
            return false;
        };
        let _ = self.warp(hit.position);
        true
    }

    pub fn step_with_navmesh(&mut self, navmesh: &NavMesh, delta_seconds: f64) {
        if self.auto_repath && !self.waiting_on_off_mesh_link {
            self.try_auto_repath(navmesh);
        }
        self.update(delta_seconds);
        if self.auto_repath && !self.waiting_on_off_mesh_link && !self.has_path() {
            self.try_auto_repath(navmesh);
        }
    }

    fn try_auto_repath(&mut self, navmesh: &NavMesh) {
        let Some(target) = self.destination else {
            return;
        };
        let needs_repath = self.path_status == Some(PathQueryStatus::Partial)
            || (!self.has_path()
                && vector_length(subtract(target, self.next_position)) > POSITION_EPSILON)
            || self.is_path_stale(navmesh);
        if needs_repath {
            let _ = self.set_destination(navmesh, target);
        }
    }

    fn has_invalidated_link(&self, navmesh: &NavMesh) -> bool {
        self.link_traversals
            .iter()
            .filter(|traversal| traversal.entry_corner + 1 >= self.corner_index)
            .any(|traversal| {
                !navmesh.links().iter().any(|candidate| {
                    candidate.activated
                        && ((vector_length(subtract(candidate.start, traversal.link.start))
                            <= POSITION_EPSILON
                            && vector_length(subtract(candidate.end, traversal.link.end))
                                <= POSITION_EPSILON)
                            || (candidate.bidirectional
                                && vector_length(subtract(candidate.end, traversal.link.start))
                                    <= POSITION_EPSILON
                                && vector_length(subtract(candidate.start, traversal.link.end))
                                    <= POSITION_EPSILON))
                })
            })
    }

    pub fn update(&mut self, delta_seconds: f64) {
        if !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || !self.speed.is_finite()
            || self.speed <= 0.0
        {
            return;
        }
        if self.is_stopped || self.waiting_on_off_mesh_link || !self.has_path() {
            self.velocity = [0.0; 3];
            self.desired_velocity = [0.0; 3];
            return;
        }

        let previous = self.next_position;
        let current = self.advance_along_corners(previous, self.speed * delta_seconds);
        self.apply_agent_displacement(previous, current, delta_seconds);
    }

    fn advance_along_corners(&mut self, start: [f64; 3], mut budget: f64) -> [f64; 3] {
        let mut current = start;
        while budget > 0.0 && self.corner_index < self.corners.len() {
            if let Some(traversal) = self
                .link_traversals
                .iter()
                .find(|item| item.entry_corner + 1 == self.corner_index)
                .copied()
            {
                self.active_off_mesh_link = Some(traversal.link);
                if !self.auto_traverse_off_mesh_link {
                    self.waiting_on_off_mesh_link = true;
                    break;
                }
            }

            let target = self.corners[self.corner_index];
            let offset = subtract(target, current);
            let length = vector_length(offset);
            if length <= budget {
                current = if length <= POSITION_EPSILON {
                    current
                } else {
                    target
                };
                budget = (budget - length).max(0.0);
                let reached_index = self.corner_index;
                self.corner_index += 1;
                self.after_corner_reached(reached_index);
                if self.waiting_on_off_mesh_link {
                    break;
                }
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
        current
    }

    fn apply_agent_displacement(
        &mut self,
        previous: [f64; 3],
        current: [f64; 3],
        delta_seconds: f64,
    ) {
        self.next_position = current;
        let displacement = subtract(current, previous);
        self.velocity = [
            displacement[0] / delta_seconds,
            displacement[1] / delta_seconds,
            displacement[2] / delta_seconds,
        ];
        self.desired_velocity = self.velocity;
        if self.update_position {
            self.position = current;
        }
        if self.update_rotation {
            if let Some(rot) = look_rotation_xz(displacement) {
                self.rotation = rot;
            }
        }
    }

    pub fn step_agents_with_avoidance(
        agents: &mut [NavMeshAgent],
        navmesh: &NavMesh,
        delta_seconds: f64,
    ) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        let previous_positions: Vec<[f64; 3]> =
            agents.iter().map(NavMeshAgent::next_position).collect();
        for agent in agents.iter_mut() {
            agent.step_with_navmesh(navmesh, delta_seconds);
        }

        let count = agents.len();
        for i in 0..count {
            for j in (i + 1)..count {
                resolve_agent_pair_avoidance(
                    agents,
                    navmesh,
                    &previous_positions,
                    (i, j),
                    delta_seconds,
                );
            }
        }
    }

    fn after_corner_reached(&mut self, reached_index: usize) {
        if self
            .link_traversals
            .iter()
            .any(|item| item.entry_corner + 1 == reached_index)
        {
            self.active_off_mesh_link = None;
            self.waiting_on_off_mesh_link = false;
        }
        if let Some(traversal) = self
            .link_traversals
            .iter()
            .find(|item| item.entry_corner == reached_index)
            .copied()
        {
            self.active_off_mesh_link = Some(traversal.link);
            if !self.auto_traverse_off_mesh_link {
                self.waiting_on_off_mesh_link = true;
            }
        }
    }
}

fn resolve_agent_pair_avoidance(
    agents: &mut [NavMeshAgent],
    navmesh: &NavMesh,
    previous_positions: &[[f64; 3]],
    pair: (usize, usize),
    delta_seconds: f64,
) {
    let (i, j) = pair;
    let pos_i = agents[i].next_position;
    let pos_j = agents[j].next_position;
    let min_dist = agents[i].radius.max(0.0) + agents[j].radius.max(0.0);
    if min_dist <= POSITION_EPSILON {
        return;
    }
    let Some((overlap, unit_push)) = compute_avoidance_push(
        (pos_i, previous_positions[i]),
        (pos_j, previous_positions[j]),
        min_dist,
    ) else {
        return;
    };
    let (weight_i, weight_j) = match agents[i]
        .avoidance_priority
        .cmp(&agents[j].avoidance_priority)
    {
        Ordering::Less => (0.0, 1.0),
        Ordering::Greater => (1.0, 0.0),
        Ordering::Equal => (0.5, 0.5),
    };
    if weight_i > 0.0 {
        let candidate_i = [
            pos_i[0] - unit_push[0] * overlap * weight_i,
            pos_i[1],
            pos_i[2] - unit_push[2] * overlap * weight_i,
        ];
        apply_avoidance_displacement(
            &mut agents[i],
            navmesh,
            previous_positions[i],
            candidate_i,
            delta_seconds,
        );
    }
    if weight_j > 0.0 {
        let candidate_j = [
            pos_j[0] + unit_push[0] * overlap * weight_j,
            pos_j[1],
            pos_j[2] + unit_push[2] * overlap * weight_j,
        ];
        apply_avoidance_displacement(
            &mut agents[j],
            navmesh,
            previous_positions[j],
            candidate_j,
            delta_seconds,
        );
    }
}

fn compute_avoidance_push(
    agent_i: ([f64; 3], [f64; 3]),
    agent_j: ([f64; 3], [f64; 3]),
    min_dist: f64,
) -> Option<(f64, [f64; 3])> {
    let (pos_i, prev_i) = agent_i;
    let (pos_j, prev_j) = agent_j;
    let mut diff = [pos_j[0] - pos_i[0], 0.0, pos_j[2] - pos_i[2]];
    let mut dist = vector_length(diff);
    if dist >= min_dist {
        return None;
    }
    if dist <= POSITION_EPSILON {
        let travel_i = subtract(pos_i, prev_i);
        let len_i = (travel_i[0] * travel_i[0] + travel_i[2] * travel_i[2]).sqrt();
        diff = if len_i > POSITION_EPSILON {
            [-travel_i[2] / len_i, 0.0, travel_i[0] / len_i]
        } else {
            [0.0, 0.0, 1.0]
        };
        dist = 0.0;
    } else {
        diff = [diff[0] / dist, 0.0, diff[2] / dist];
    }
    let lateral = [-diff[2], 0.0, diff[0]];
    let vel_i = subtract(pos_i, prev_i);
    let vel_j = subtract(pos_j, prev_j);
    let head_on = dot(vel_i, diff) > 0.0 && dot(vel_j, diff) < 0.0;
    let push_dir = if head_on {
        [
            diff[0] * 0.6 + lateral[0] * 0.8,
            0.0,
            diff[2] * 0.6 + lateral[2] * 0.8,
        ]
    } else {
        diff
    };
    let push_len = vector_length(push_dir).max(POSITION_EPSILON);
    Some((
        min_dist - dist,
        [push_dir[0] / push_len, 0.0, push_dir[2] / push_len],
    ))
}

fn apply_avoidance_displacement(
    agent: &mut NavMeshAgent,
    navmesh: &NavMesh,
    previous: [f64; 3],
    candidate: [f64; 3],
    delta_seconds: f64,
) {
    let resolved = if navmesh.polygon_at(candidate).is_some() {
        candidate
    } else if let Some(hit) =
        navmesh.sample_position(candidate, agent.radius.max(1.0), agent.area_mask)
    {
        hit.position
    } else {
        candidate
    };
    agent.next_position = resolved;
    let displacement = subtract(resolved, previous);
    agent.velocity = [
        displacement[0] / delta_seconds,
        displacement[1] / delta_seconds,
        displacement[2] / delta_seconds,
    ];
    if agent.update_position {
        agent.position = resolved;
    }
    if agent.update_rotation {
        if let Some(rot) = look_rotation_xz(displacement) {
            agent.rotation = rot;
        }
    }
}

fn look_rotation_xz(direction: [f64; 3]) -> Option<[f64; 4]> {
    let planar_len = (direction[0] * direction[0] + direction[2] * direction[2]).sqrt();
    if planar_len <= POSITION_EPSILON {
        return None;
    }
    let yaw = direction[0].atan2(direction[2]);
    let half = 0.5 * yaw;
    Some([0.0, half.sin(), 0.0, half.cos()])
}
