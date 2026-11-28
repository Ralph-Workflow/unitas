use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

use crate::body::distance;
use crate::navigation::agent::NavMeshQueryFilter;
use crate::navigation::mesh::{area_matches_mask, ALL_AREAS_MASK};
use crate::navigation::{NavMesh, NavMeshLink};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshHit {
    pub position: [f64; 3],
    pub normal: [f64; 3],
    pub distance: f64,
    pub mask: u32,
    pub hit: bool,
    pub polygon_index: Option<usize>,
}

impl NavMeshHit {
    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    pub fn normal(&self) -> [f64; 3] {
        self.normal
    }

    pub fn distance(&self) -> f64 {
        self.distance
    }

    pub fn mask(&self) -> u32 {
        self.mask
    }

    pub fn hit(&self) -> bool {
        self.hit
    }

    pub fn polygon_index(&self) -> Option<usize> {
        self.polygon_index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathQueryStatus {
    Complete,
    Partial,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnreachableReason {
    StartNotOnNavMesh,
    GoalNotOnNavMesh,
    NoRouteToGoal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PathLinkTraversal {
    pub(crate) entry_corner: usize,
    pub(crate) link: NavMeshLink,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathQueryResult {
    pub status: PathQueryStatus,
    pub corners: Vec<[f64; 3]>,
    pub unreachable_reason: Option<UnreachableReason>,
    pub(crate) link_traversals: Vec<PathLinkTraversal>,
    pub(crate) polygons: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
enum StepKind {
    Adjacent { portal: [f64; 3] },
    Link { link: usize, forward: bool },
}

#[derive(Clone, Copy, Debug)]
struct PathStep {
    from: usize,
    kind: StepKind,
}

#[derive(Clone, Copy, Debug)]
struct OpenEntry {
    priority: f64,
    polygon: usize,
    order: u32,
}

impl PartialEq for OpenEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.order == other.order
    }
}

impl Eq for OpenEntry {}

impl PartialOrd for OpenEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpenEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.order.cmp(&self.order))
    }
}

impl NavMesh {
    pub fn find_path(&self, start: [f64; 3], goal: [f64; 3]) -> PathQueryResult {
        self.find_path_with_mask(start, goal, ALL_AREAS_MASK)
    }

    pub fn find_path_with_mask(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        area_mask: u32,
    ) -> PathQueryResult {
        self.find_path_with_costs(start, goal, area_mask, &BTreeMap::new())
    }

    pub fn calculate_path(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        filter: &NavMeshQueryFilter,
    ) -> PathQueryResult {
        self.find_path_with_costs(start, goal, filter.area_mask, filter.area_costs())
    }

    pub fn find_path_with_costs(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        area_mask: u32,
        area_cost_overrides: &BTreeMap<u32, f64>,
    ) -> PathQueryResult {
        let Some(start_polygon) = self
            .polygon_at(start)
            .filter(|&poly| area_matches_mask(self.polygons[poly].area, area_mask))
        else {
            return invalid_path_result(UnreachableReason::StartNotOnNavMesh);
        };
        let Some(goal_polygon) = self
            .polygon_at(goal)
            .filter(|&poly| area_matches_mask(self.polygons[poly].area, area_mask))
        else {
            return invalid_path_result(UnreachableReason::GoalNotOnNavMesh);
        };
        if start_polygon == goal_polygon {
            return PathQueryResult {
                status: PathQueryStatus::Complete,
                corners: vec![start, goal],
                unreachable_reason: None,
                link_traversals: Vec::new(),
                polygons: vec![start_polygon],
            };
        }
        self.search(
            start,
            goal,
            start_polygon,
            goal_polygon,
            area_mask,
            area_cost_overrides,
        )
    }

    fn effective_area_cost(&self, area: u32, overrides: &BTreeMap<u32, f64>) -> f64 {
        overrides
            .get(&area)
            .copied()
            .unwrap_or_else(|| self.area_cost(area))
    }

    fn effective_min_area_cost(&self, overrides: &BTreeMap<u32, f64>) -> f64 {
        overrides
            .values()
            .copied()
            .fold(self.min_area_cost(), f64::min)
    }

    fn search(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        start_polygon: usize,
        goal_polygon: usize,
        area_mask: u32,
        area_cost_overrides: &BTreeMap<u32, f64>,
    ) -> PathQueryResult {
        let mut cost_so_far = vec![f64::INFINITY; self.polygons.len()];
        let mut came_from: Vec<Option<PathStep>> = vec![None; self.polygons.len()];
        let mut closed = vec![false; self.polygons.len()];
        let mut open = BinaryHeap::new();
        let mut order = 0u32;
        let min_cost = self.effective_min_area_cost(area_cost_overrides);
        cost_so_far[start_polygon] = 0.0;
        let mut closest_distance = self.heuristic(start_polygon, goal, min_cost);
        let mut closest = start_polygon;
        open.push(OpenEntry {
            priority: closest_distance,
            polygon: start_polygon,
            order,
        });
        order += 1;
        while let Some(entry) = open.pop() {
            let current = entry.polygon;
            if closed[current] {
                continue;
            }
            closed[current] = true;
            if current == goal_polygon {
                return self.build_complete_result(start, goal, goal_polygon, &came_from);
            }
            let remaining = self.heuristic(current, goal, min_cost);
            if remaining < closest_distance {
                (closest_distance, closest) = (remaining, current);
            }
            for (next, kind) in self.successors(current, area_mask) {
                if closed[next] {
                    continue;
                }
                let tentative = cost_so_far[current]
                    + self.step_cost(current, next, &kind, area_cost_overrides);
                if tentative < cost_so_far[next] {
                    cost_so_far[next] = tentative;
                    came_from[next] = Some(PathStep {
                        from: current,
                        kind,
                    });
                    open.push(OpenEntry {
                        priority: tentative + self.heuristic(next, goal, min_cost),
                        polygon: next,
                        order,
                    });
                    order += 1;
                }
            }
        }
        self.build_partial_result(start, goal, closest, &came_from)
    }

    fn build_complete_result(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        goal_polygon: usize,
        came_from: &[Option<PathStep>],
    ) -> PathQueryResult {
        let (corners, link_traversals, polygons) =
            self.build_corners(start, goal, goal_polygon, came_from);
        PathQueryResult {
            status: PathQueryStatus::Complete,
            corners,
            unreachable_reason: None,
            link_traversals,
            polygons,
        }
    }

    fn build_partial_result(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        closest: usize,
        came_from: &[Option<PathStep>],
    ) -> PathQueryResult {
        let (mut corners, link_traversals, polygons) =
            self.build_corners(start, goal, closest, came_from);
        corners.pop();
        corners.push(self.polygons[closest].centroid);
        PathQueryResult {
            status: PathQueryStatus::Partial,
            corners,
            unreachable_reason: Some(UnreachableReason::NoRouteToGoal),
            link_traversals,
            polygons,
        }
    }

    fn successors(&self, polygon: usize, area_mask: u32) -> Vec<(usize, StepKind)> {
        let mut successors = Vec::new();
        for neighbor in &self.polygons[polygon].neighbors {
            if !self.is_polygon_carved(neighbor.polygon)
                && area_matches_mask(self.polygons[neighbor.polygon].area, area_mask)
            {
                successors.push((
                    neighbor.polygon,
                    StepKind::Adjacent {
                        portal: neighbor.portal,
                    },
                ));
            }
        }
        for (index, link) in self.links.iter().enumerate() {
            if !link.activated || !area_matches_mask(link.area, area_mask) {
                continue;
            }
            if link.start_polygon == polygon
                && !self.is_polygon_carved(link.end_polygon)
                && area_matches_mask(self.polygons[link.end_polygon].area, area_mask)
            {
                successors.push((
                    link.end_polygon,
                    StepKind::Link {
                        link: index,
                        forward: true,
                    },
                ));
            }
            if link.bidirectional
                && link.end_polygon == polygon
                && !self.is_polygon_carved(link.start_polygon)
                && area_matches_mask(self.polygons[link.start_polygon].area, area_mask)
            {
                successors.push((
                    link.start_polygon,
                    StepKind::Link {
                        link: index,
                        forward: false,
                    },
                ));
            }
        }
        successors
    }

    fn step_cost(
        &self,
        from: usize,
        to: usize,
        kind: &StepKind,
        overrides: &BTreeMap<u32, f64>,
    ) -> f64 {
        match kind {
            StepKind::Adjacent { portal } => {
                let traversal = distance(self.polygons[from].centroid, *portal)
                    + distance(*portal, self.polygons[to].centroid);
                traversal * self.effective_area_cost(self.polygons[to].area, overrides)
            }
            StepKind::Link { link, forward } => {
                let link = &self.links[*link];
                let (entry, exit) = if *forward {
                    (link.start, link.end)
                } else {
                    (link.end, link.start)
                };
                let approach = distance(self.polygons[from].centroid, entry)
                    * self.effective_area_cost(self.polygons[from].area, overrides);
                let link_cost = link.cost_override.unwrap_or_else(|| {
                    distance(entry, exit) * self.effective_area_cost(link.area, overrides)
                });
                let departure = distance(exit, self.polygons[to].centroid)
                    * self.effective_area_cost(self.polygons[to].area, overrides);
                approach + link_cost + departure
            }
        }
    }

    fn heuristic(&self, polygon: usize, goal: [f64; 3], min_cost: f64) -> f64 {
        distance(self.polygons[polygon].centroid, goal) * min_cost
    }

    fn build_corners(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        target: usize,
        came_from: &[Option<PathStep>],
    ) -> (Vec<[f64; 3]>, Vec<PathLinkTraversal>, Vec<usize>) {
        let mut steps = Vec::new();
        let mut polygons = vec![target];
        let mut current = target;
        while let Some(step) = came_from[current] {
            steps.push(step);
            polygons.push(step.from);
            current = step.from;
        }
        steps.reverse();
        polygons.reverse();

        let mut corners = vec![start];
        let mut link_traversals = Vec::new();
        for step in steps {
            match step.kind {
                StepKind::Adjacent { portal } => corners.push(portal),
                StepKind::Link { link, forward } => {
                    let link_data = self.links[link];
                    let entry_corner = corners.len();
                    let oriented = if forward {
                        corners.push(link_data.start);
                        corners.push(link_data.end);
                        link_data
                    } else {
                        corners.push(link_data.end);
                        corners.push(link_data.start);
                        NavMeshLink {
                            start: link_data.end,
                            end: link_data.start,
                            start_polygon: link_data.end_polygon,
                            end_polygon: link_data.start_polygon,
                            ..link_data
                        }
                    };
                    link_traversals.push(PathLinkTraversal {
                        entry_corner,
                        link: oriented,
                    });
                }
            }
        }
        corners.push(goal);
        (corners, link_traversals, polygons)
    }
}

fn invalid_path_result(reason: UnreachableReason) -> PathQueryResult {
    PathQueryResult {
        status: PathQueryStatus::Invalid,
        corners: Vec::new(),
        unreachable_reason: Some(reason),
        link_traversals: Vec::new(),
        polygons: Vec::new(),
    }
}
