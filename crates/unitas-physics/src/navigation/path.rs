use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::body::distance;
use crate::navigation::NavMesh;

/// The outcome of a path query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathQueryStatus {
    /// The path reaches the destination.
    Complete,
    /// The path gets as close to the destination as the navmesh allows.
    Partial,
    /// The query names no usable start or destination.
    Invalid,
}

/// Why a path query could not reach the destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnreachableReason {
    StartNotOnNavMesh,
    GoalNotOnNavMesh,
    NoRouteToGoal,
}

/// The corners of a path query and the outcome that produced them.
#[derive(Clone, Debug, PartialEq)]
pub struct PathQueryResult {
    pub status: PathQueryStatus,
    pub corners: Vec<[f64; 3]>,
    pub unreachable_reason: Option<UnreachableReason>,
}

/// How one polygon was reached from the previous one.
#[derive(Clone, Copy, Debug)]
enum StepKind {
    Adjacent { portal: [f64; 3] },
    Link { link: usize, forward: bool },
}

/// The polygon a step came from and the way it was taken.
#[derive(Clone, Copy, Debug)]
struct PathStep {
    from: usize,
    kind: StepKind,
}

/// One polygon waiting in the open set of the search.
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
    /// Searches the navmesh for the cheapest path from the start to the goal.
    pub fn find_path(&self, start: [f64; 3], goal: [f64; 3]) -> PathQueryResult {
        let Some(start_polygon) = self.polygon_at(start) else {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::StartNotOnNavMesh),
            };
        };
        let Some(goal_polygon) = self.polygon_at(goal) else {
            return PathQueryResult {
                status: PathQueryStatus::Invalid,
                corners: Vec::new(),
                unreachable_reason: Some(UnreachableReason::GoalNotOnNavMesh),
            };
        };
        if start_polygon == goal_polygon {
            return PathQueryResult {
                status: PathQueryStatus::Complete,
                corners: vec![start, goal],
                unreachable_reason: None,
            };
        }
        self.search(start, goal, start_polygon, goal_polygon)
    }

    fn search(
        &self,
        start: [f64; 3],
        goal: [f64; 3],
        start_polygon: usize,
        goal_polygon: usize,
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
                return PathQueryResult {
                    status: PathQueryStatus::Complete,
                    corners: self.build_corners(start, goal, goal_polygon, &came_from),
                    unreachable_reason: None,
                };
            }
            let remaining = self.heuristic(current, goal);
            if remaining < closest_distance {
                closest_distance = remaining;
                closest = current;
            }
            for (next, kind) in self.successors(current) {
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

        let mut corners = self.build_corners(start, goal, closest, &came_from);
        corners.pop();
        corners.push(self.polygons[closest].centroid);
        PathQueryResult {
            status: PathQueryStatus::Partial,
            corners,
            unreachable_reason: Some(UnreachableReason::NoRouteToGoal),
        }
    }

    fn successors(&self, polygon: usize) -> Vec<(usize, StepKind)> {
        let mut successors = Vec::new();
        for neighbor in &self.polygons[polygon].neighbors {
            successors.push((
                neighbor.polygon,
                StepKind::Adjacent {
                    portal: neighbor.portal,
                },
            ));
        }
        for (index, link) in self.links.iter().enumerate() {
            if link.start_polygon == polygon {
                successors.push((
                    link.end_polygon,
                    StepKind::Link {
                        link: index,
                        forward: true,
                    },
                ));
            }
            if link.bidirectional && link.end_polygon == polygon {
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
        let traversal = match kind {
            StepKind::Adjacent { portal } => {
                distance(self.polygons[from].centroid, *portal)
                    + distance(*portal, self.polygons[to].centroid)
            }
            StepKind::Link { link, forward } => {
                let link = &self.links[*link];
                let (entry, exit) = if *forward {
                    (link.start, link.end)
                } else {
                    (link.end, link.start)
                };
                distance(self.polygons[from].centroid, entry) + distance(entry, exit)
            }
        };
        traversal * self.area_cost(self.polygons[to].area)
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
    ) -> Vec<[f64; 3]> {
        let mut steps = Vec::new();
        let mut current = target;
        while let Some(step) = came_from[current] {
            steps.push(step);
            current = step.from;
        }
        steps.reverse();

        let mut corners = vec![start];
        for step in steps {
            match step.kind {
                StepKind::Adjacent { portal } => corners.push(portal),
                StepKind::Link { link, forward } => {
                    let link = &self.links[link];
                    if forward {
                        corners.push(link.start);
                        corners.push(link.end);
                    } else {
                        corners.push(link.end);
                        corners.push(link.start);
                    }
                }
            }
        }
        corners.push(goal);
        corners
    }
}
