use unitas_core::EntityId;

use crate::body::{add, rotate_vector, subtract, vector_length, IDENTITY_ROTATION};
use crate::navigation::mesh::ALL_AREAS_MASK;
use crate::navigation::path::PathLinkTraversal;
use crate::navigation::{
    NavMesh, NavMeshLink, NavigationError, PathQueryResult, PathQueryStatus, UnreachableReason,
};

const DEFAULT_SPEED: f64 = 3.5;
const POSITION_EPSILON: f64 = 1e-9;

#[derive(Clone, Debug)]
pub struct NavMeshAgent {
    pub speed: f64,
    pub update_position: bool,
    pub update_rotation: bool,
    pub auto_traverse_off_mesh_link: bool,
    pub auto_repath: bool,
    pub area_mask: u32,
    entity: EntityId,
    name: String,
    position: [f64; 3],
    next_position: [f64; 3],
    rotation: [f64; 4],
    velocity: [f64; 3],
    destination: Option<[f64; 3]>,
    path_status: Option<PathQueryStatus>,
    corners: Vec<[f64; 3]>,
    corner_index: usize,
    link_traversals: Vec<PathLinkTraversal>,
    active_off_mesh_link: Option<NavMeshLink>,
    waiting_on_off_mesh_link: bool,
}

impl NavMeshAgent {
    pub fn new(entity: EntityId, name: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            speed: DEFAULT_SPEED,
            update_position: true,
            update_rotation: true,
            auto_traverse_off_mesh_link: true,
            auto_repath: true,
            area_mask: ALL_AREAS_MASK,
            entity,
            name: name.into(),
            position,
            next_position: position,
            rotation: IDENTITY_ROTATION,
            velocity: [0.0; 3],
            destination: None,
            path_status: None,
            corners: Vec::new(),
            corner_index: 0,
            link_traversals: Vec::new(),
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

    pub fn destination(&self) -> Option<[f64; 3]> {
        self.destination
    }

    pub fn path_status(&self) -> Option<PathQueryStatus> {
        self.path_status
    }

    pub fn has_path(&self) -> bool {
        self.corner_index < self.corners.len() || self.waiting_on_off_mesh_link
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
        let result = navmesh.find_path_with_mask(self.next_position, target, self.area_mask);
        match result.status {
            PathQueryStatus::Complete | PathQueryStatus::Partial => {
                self.destination = Some(target);
                self.path_status = Some(result.status);
                self.corners = result.corners.clone();
                self.corner_index = usize::from(self.corners.len() > 1);
                self.link_traversals = result.link_traversals.clone();
                self.active_off_mesh_link = None;
                self.waiting_on_off_mesh_link = false;
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

    pub fn warp(&mut self, position: [f64; 3]) -> Result<(), NavigationError> {
        if !position.iter().all(|component| component.is_finite()) {
            return Err(NavigationError::InvalidPosition { position });
        }
        self.position = position;
        self.next_position = position;
        self.velocity = [0.0; 3];
        self.destination = None;
        self.path_status = None;
        self.corners.clear();
        self.corner_index = 0;
        self.link_traversals.clear();
        self.active_off_mesh_link = None;
        self.waiting_on_off_mesh_link = false;
        Ok(())
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
            || self.has_invalidated_link(navmesh);
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
        if self.waiting_on_off_mesh_link {
            self.velocity = [0.0; 3];
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
            if length <= POSITION_EPSILON {
                let reached_index = self.corner_index;
                self.corner_index += 1;
                self.after_corner_reached(reached_index);
                if self.waiting_on_off_mesh_link {
                    break;
                }
                continue;
            }
            if length <= budget {
                current = target;
                budget -= length;
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

        self.next_position = current;
        let displacement = subtract(current, previous);
        self.velocity = [
            displacement[0] / delta_seconds,
            displacement[1] / delta_seconds,
            displacement[2] / delta_seconds,
        ];
        if self.update_position {
            self.position = current;
        }
        if self.update_rotation {
            if let Some(rot) = look_rotation_xz(displacement) {
                self.rotation = rot;
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

fn look_rotation_xz(direction: [f64; 3]) -> Option<[f64; 4]> {
    let planar_len = (direction[0] * direction[0] + direction[2] * direction[2]).sqrt();
    if planar_len <= POSITION_EPSILON {
        return None;
    }
    let yaw = direction[0].atan2(direction[2]);
    let half = 0.5 * yaw;
    Some([0.0, half.sin(), 0.0, half.cos()])
}
