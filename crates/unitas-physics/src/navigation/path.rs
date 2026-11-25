use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::body::distance;
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
        let Some(start_polygon) = self.polygon_at(start) else {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::StartNotOnNavMesh),
                link_traversals: Vec::new(),
            };
        };
        if !area_matches_mask(self.polygons[start_polygon].area, area_mask) {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::StartNotOnNavMesh),
                link_traversals: Vec::new(),
            };
        };
        let Some(goal_polygon) = self.polygon_at(goal) else {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::GoalNotOnNavMesh),
                link_traversals: Vec::new(),
            };
        };
        if !area_matches_mask(self.polygons[goal_polygon].area, area_mask) {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::GoalNotOnNavMesh),
                link_traversals: Vec::new(),
            };
        };
        if start_polygon == goal_polygon {
            return PathQueryResult {
                status: PathQueryStatus::Complete,
                corners: vec![start, goal],
                unreachable_reason: None,
                link_traversals: Vec::new(),
            };
        }
        self.search(start, goal, start_polygon, goal_polygon, area_mask)
    }

    fn search(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        start_polygon: usize,
        goal_polygon: usize,
        area_mask: u32,
    ) -> PathQueryResult {
        let mut cost_so_far = vec![f64::INFINITY; self.polygons.len()];
        let mut came_from: Vec<Option<PathStep>> = vec![None; self.polygons.len()];
        let mut closed = vec![false; self.polygons.len()];
        let mut open = BinaryHeap::new();
        let mut order = 0u32;

        cost_so_far[start_polygon] = 0.0;
        open.push(OpenEntry {
            priority: self.heuristic(start_polygon, goal),
            polygon: start_polygon,
            order,
        });
        order += 1;

        let mut closest = start_polygon;
        let mut closest_distance = self.heuristic(start_polygon, goal);

        while let Some(entry) = open.pop() {
            let current = entry.polygon;
            if closed[current] {
                continue;
            }
            closed[current] = true;
            if current == goal_polygon {
                let (corners, link_traversals) =
                    self.build_corners(start, goal, goal_polygon, &came_from);
                return PathQueryResult {
                    status: PathQueryStatus::Complete,
                    corners,
                    unreachable_reason: None,
                    link_traversals,
                };
            }
            let remaining = self.heuristic(current, goal);
            if remaining < closest_distance {
                closest_distance = remaining;
                closest = current;
            }
            for (next, kind) in self.successors(current, area_mask) {
                if closed[next] {
                    continue;
                }
                let tentative = cost_so_far[current] + self.step_cost(current, next, &kind);
                if tentative < cost_so_far[next] {
                    cost_so_far[next] = tentative;
                    came_from[next] = Some(PathStep {
                        from: current,
                        kind,
                    });
                    open.push(OpenEntry {
                        priority: tentative + self.heuristic(next, goal),
                        polygon: next,
                        order,
                    });
                    order += 1;
                }
            }
        }

        let (mut corners, link_traversals) = self.build_corners(start, goal, closest, &came_from);
        corners.pop();
        corners.push(self.polygons[closest].centroid);
        PathQueryResult {
            status: PathQueryStatus::Partial,
            corners,
            unreachable_reason: Some(UnreachableReason::NoRouteToGoal),
            link_traversals,
        }
    }

    fn successors(&self, polygon: usize, area_mask: u32) -> Vec<(usize, StepKind)> {
        let mut successors = Vec::new();
        for neighbor in &self.polygons[polygon].neighbors {
            if area_matches_mask(self.polygons[neighbor.polygon].area, area_mask) {
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

    fn step_cost(&self, from: usize, to: usize, kind: &StepKind) -> f64 {
        match kind {
            StepKind::Adjacent { portal } => {
                let traversal = distance(self.polygons[from].centroid, *portal)
                    + distance(*portal, self.polygons[to].centroid);
                traversal * self.area_cost(self.polygons[to].area)
            }
            StepKind::Link { link, forward } => {
                let link = &self.links[*link];
                let (entry, exit) = if *forward {
                    (link.start, link.end)
                } else {
                    (link.end, link.start)
                };
                let approach =
                    distance(self.polygons[from].centroid, entry) * self.area_cost(self.polygons[from].area);
                let link_cost = link
                    .cost_override
                    .unwrap_or_else(|| distance(entry, exit) * self.area_cost(link.area));
                let departure =
                    distance(exit, self.polygons[to].centroid) * self.area_cost(self.polygons[to].area);
                approach + link_cost + departure
            }
        }
    }

    fn heuristic(&self, polygon: usize, goal: [f64; 3]) -> f64 {
        distance(self.polygons[polygon].centroid, goal) * self.min_area_cost()
    }

    fn build_corners(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        target: usize,
        came_from: &[Option<PathStep>],
    ) -> (Vec<[f64; 3]>, Vec<PathLinkTraversal>) {
        let mut steps = Vec::new();
        let mut current = target;
        while let Some(step) = came_from[current] {
            steps.push(step);
            current = step.from;
        }
        steps.reverse();

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
        (corners, link_traversals)
    }
}
