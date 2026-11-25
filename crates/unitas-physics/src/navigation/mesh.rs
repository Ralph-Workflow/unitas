use std::collections::BTreeMap;

use crate::body::{distance, dot, subtract, vector_length};
use crate::navigation::NavigationError;

/// The distance below which two vertices are the same point.
const VERTEX_EPSILON: f64 = 1e-6;
/// The smallest triangle area the navmesh accepts.
const AREA_EPSILON: f64 = 1e-9;
/// The distance from a triangle plane a point may sit and still count as on the mesh.
const HEIGHT_TOLERANCE: f64 = 0.25;
/// The tolerance of the barycentric containment test.
const POINT_EPSILON: f64 = 1e-9;

/// One walkable triangle of the source mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshTriangle {
    pub vertices: [[f64; 3]; 3],
    pub area: u32,
}

impl NavMeshTriangle {
    pub fn new(vertices: [[f64; 3]; 3], area: u32) -> Self {
        Self { vertices, area }
    }
}

/// The traversal cost of one navmesh area; areas without an entry cost 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshArea {
    pub id: u32,
    pub cost: f64,
}

impl NavMeshArea {
    pub fn new(id: u32, cost: f64) -> Self {
        Self { id, cost }
    }
}

/// One neighbour of a navmesh polygon and the portal the two share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshNeighbor {
    pub polygon: usize,
    pub portal: [f64; 3],
}

/// One walkable polygon of the navmesh, built from one source triangle.
#[derive(Clone, Debug, PartialEq)]
pub struct NavMeshPolygon {
    pub vertices: [[f64; 3]; 3],
    pub area: u32,
    pub centroid: [f64; 3],
    pub neighbors: Vec<NavMeshNeighbor>,
}

/// An explicit connection between two navmesh areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshLink {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub bidirectional: bool,
    pub(crate) start_polygon: usize,
    pub(crate) end_polygon: usize,
}

impl NavMeshLink {
    pub fn new(start: [f64; 3], end: [f64; 3], bidirectional: bool) -> Self {
        Self {
            start,
            end,
            bidirectional,
            start_polygon: 0,
            end_polygon: 0,
        }
    }
}

/// The navmesh: walkable polygons, the areas they belong to, and explicit links.
#[derive(Clone, Debug)]
pub struct NavMesh {
    pub(crate) polygons: Vec<NavMeshPolygon>,
    pub(crate) links: Vec<NavMeshLink>,
    area_costs: BTreeMap<u32, f64>,
    min_area_cost: f64,
}

impl Default for NavMesh {
    fn default() -> Self {
        Self {
            polygons: Vec::new(),
            links: Vec::new(),
            area_costs: BTreeMap::new(),
            min_area_cost: 1.0,
        }
    }
}

impl NavMesh {
    /// Builds the navmesh from a walkable triangle soup and the cost of its areas.
    pub fn build(
        triangles: &[NavMeshTriangle],
        areas: &[NavMeshArea],
    ) -> Result<Self, NavigationError> {
        let mut area_costs = BTreeMap::new();
        let mut min_area_cost: f64 = 1.0;
        for area in areas {
            if !area.cost.is_finite() || area.cost <= 0.0 {
                return Err(NavigationError::InvalidAreaCost {
                    area: area.id,
                    cost: area.cost,
                });
            }
            area_costs.insert(area.id, area.cost);
            min_area_cost = min_area_cost.min(area.cost);
        }

        let mut polygons = Vec::with_capacity(triangles.len());
        for (index, triangle) in triangles.iter().enumerate() {
            if !triangle
                .vertices
                .iter()
                .flatten()
                .all(|component| component.is_finite())
            {
                return Err(NavigationError::InvalidPosition {
                    position: triangle.vertices[0],
                });
            }
            if triangle_area(&triangle.vertices) <= AREA_EPSILON {
                return Err(NavigationError::DegenerateTriangle { index });
            }
            polygons.push(NavMeshPolygon {
                vertices: triangle.vertices,
                area: triangle.area,
                centroid: triangle_centroid(&triangle.vertices),
                neighbors: Vec::new(),
            });
        }

        let mut navmesh = Self {
            polygons,
            links: Vec::new(),
            area_costs,
            min_area_cost,
        };
        navmesh.link_neighbors();
        Ok(navmesh)
    }

    /// Adds an explicit link between two navmesh areas.
    pub fn add_link(&mut self, link: NavMeshLink) -> Result<(), NavigationError> {
        if !link.start.iter().all(|component| component.is_finite())
            || !link.end.iter().all(|component| component.is_finite())
        {
            return Err(NavigationError::InvalidPosition {
                position: link.start,
            });
        }
        let start_polygon =
            self.polygon_at(link.start)
                .ok_or(NavigationError::LinkEndpointOffNavMesh {
                    position: link.start,
                })?;
        let end_polygon = self
            .polygon_at(link.end)
            .ok_or(NavigationError::LinkEndpointOffNavMesh { position: link.end })?;
        self.links.push(NavMeshLink {
            start_polygon,
            end_polygon,
            ..link
        });
        Ok(())
    }

    pub fn polygons(&self) -> &[NavMeshPolygon] {
        &self.polygons
    }

    pub fn links(&self) -> &[NavMeshLink] {
        &self.links
    }

    pub fn polygon_count(&self) -> usize {
        self.polygons.len()
    }

    /// The traversal cost of one area; areas without an entry cost 1.
    pub fn area_cost(&self, area: u32) -> f64 {
        self.area_costs.get(&area).copied().unwrap_or(1.0)
    }

    /// The polygon the point sits on, if any.
    pub fn polygon_at(&self, point: [f64; 3]) -> Option<usize> {
        self.polygons
            .iter()
            .position(|polygon| point_in_triangle(point, &polygon.vertices))
    }

    pub(crate) fn min_area_cost(&self) -> f64 {
        self.min_area_cost
    }

    fn link_neighbors(&mut self) {
        let mut pairs = Vec::new();
        for first in 0..self.polygons.len() {
            for second in (first + 1)..self.polygons.len() {
                let shared = shared_edge(
                    &self.polygons[first].vertices,
                    &self.polygons[second].vertices,
                );
                if let Some((start, end)) = shared {
                    pairs.push((first, second, midpoint(start, end)));
                }
            }
        }
        for (first, second, portal) in pairs {
            self.polygons[first].neighbors.push(NavMeshNeighbor {
                polygon: second,
                portal,
            });
            self.polygons[second].neighbors.push(NavMeshNeighbor {
                polygon: first,
                portal,
            });
        }
    }
}

fn triangle_area(vertices: &[[f64; 3]; 3]) -> f64 {
    let edge_one = subtract(vertices[1], vertices[0]);
    let edge_two = subtract(vertices[2], vertices[0]);
    0.5 * vector_length(cross(edge_one, edge_two))
}

fn triangle_centroid(vertices: &[[f64; 3]; 3]) -> [f64; 3] {
    [
        (vertices[0][0] + vertices[1][0] + vertices[2][0]) / 3.0,
        (vertices[0][1] + vertices[1][1] + vertices[2][1]) / 3.0,
        (vertices[0][2] + vertices[1][2] + vertices[2][2]) / 3.0,
    ]
}

fn cross(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn midpoint(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        (first[0] + second[0]) / 2.0,
        (first[1] + second[1]) / 2.0,
        (first[2] + second[2]) / 2.0,
    ]
}

fn shared_edge(first: &[[f64; 3]; 3], second: &[[f64; 3]; 3]) -> Option<([f64; 3], [f64; 3])> {
    let mut shared = Vec::new();
    for vertex in first {
        if second
            .iter()
            .any(|other| distance(*vertex, *other) <= VERTEX_EPSILON)
        {
            shared.push(*vertex);
        }
    }
    if shared.len() == 2 {
        Some((shared[0], shared[1]))
    } else {
        None
    }
}

fn point_in_triangle(point: [f64; 3], vertices: &[[f64; 3]; 3]) -> bool {
    let edge_one = subtract(vertices[1], vertices[0]);
    let edge_two = subtract(vertices[2], vertices[0]);
    let normal = cross(edge_one, edge_two);
    let normal_length = vector_length(normal);
    if normal_length <= AREA_EPSILON {
        return false;
    }
    let unit_normal = [
        normal[0] / normal_length,
        normal[1] / normal_length,
        normal[2] / normal_length,
    ];
    let offset = subtract(point, vertices[0]);
    if dot(offset, unit_normal).abs() > HEIGHT_TOLERANCE {
        return false;
    }
    let d00 = dot(edge_one, edge_one);
    let d01 = dot(edge_one, edge_two);
    let d11 = dot(edge_two, edge_two);
    let d20 = dot(offset, edge_one);
    let d21 = dot(offset, edge_two);
    let denominator = d00 * d11 - d01 * d01;
    if denominator <= f64::EPSILON {
        return false;
    }
    let first = (d11 * d20 - d01 * d21) / denominator;
    let second = (d00 * d21 - d01 * d20) / denominator;
    first >= -POINT_EPSILON && second >= -POINT_EPSILON && first + second <= 1.0 + POINT_EPSILON
}
