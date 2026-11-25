use std::collections::BTreeMap;

use crate::body::{closest_point_on_segment, distance, dot, subtract, vector_length};
use crate::navigation::path::NavMeshHit;
use crate::navigation::NavigationError;

const VERTEX_EPSILON: f64 = 1e-6;
const AREA_EPSILON: f64 = 1e-9;
const HEIGHT_TOLERANCE: f64 = 0.25;
const POINT_EPSILON: f64 = 1e-9;

pub const ALL_AREAS_MASK: u32 = u32::MAX;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshNeighbor {
    pub polygon: usize,
    pub portal: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct NavMeshPolygon {
    pub vertices: [[f64; 3]; 3],
    pub area: u32,
    pub centroid: [f64; 3],
    pub neighbors: Vec<NavMeshNeighbor>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavMeshLink {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub bidirectional: bool,
    pub area: u32,
    pub cost_override: Option<f64>,
    pub activated: bool,
    pub(crate) start_polygon: usize,
    pub(crate) end_polygon: usize,
}

impl NavMeshLink {
    pub fn new(start: [f64; 3], end: [f64; 3], bidirectional: bool) -> Self {
        Self {
            start,
            end,
            bidirectional,
            area: 0,
            cost_override: None,
            activated: true,
            start_polygon: 0,
            end_polygon: 0,
        }
    }

    pub fn with_area(mut self, area: u32) -> Self {
        self.area = area;
        self
    }

    pub fn with_cost_override(mut self, cost: f64) -> Self {
        self.cost_override = Some(cost);
        self
    }

    pub fn with_activated(mut self, activated: bool) -> Self {
        self.activated = activated;
        self
    }

    pub fn start_polygon(&self) -> usize {
        self.start_polygon
    }

    pub fn end_polygon(&self) -> usize {
        self.end_polygon
    }
}

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
    pub const ALL_AREAS: u32 = ALL_AREAS_MASK;

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

    pub fn add_link(&mut self, link: NavMeshLink) -> Result<(), NavigationError> {
        if !link.start.iter().all(|component| component.is_finite())
            || !link.end.iter().all(|component| component.is_finite())
        {
            return Err(NavigationError::InvalidPosition {
                position: link.start,
            });
        }
        if let Some(cost) = link.cost_override {
            if !cost.is_finite() || cost <= 0.0 {
                return Err(NavigationError::InvalidAreaCost {
                    area: link.area,
                    cost,
                });
            }
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

    pub fn set_link_activated(&mut self, index: usize, activated: bool) -> bool {
        if let Some(link) = self.links.get_mut(index) {
            link.activated = activated;
            true
        } else {
            false
        }
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

    pub fn area_cost(&self, area: u32) -> f64 {
        self.area_costs.get(&area).copied().unwrap_or(1.0)
    }

    pub fn polygon_at(&self, point: [f64; 3]) -> Option<usize> {
        self.polygons
            .iter()
            .position(|polygon| point_in_triangle(point, &polygon.vertices))
    }

    pub fn sample_position(
        &self,
        source: [f64; 3],
        max_distance: f64,
        area_mask: u32,
    ) -> Option<NavMeshHit> {
        if !source.iter().all(|component| component.is_finite())
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return None;
        }
        let mut best: Option<NavMeshHit> = None;
        for (index, polygon) in self.polygons.iter().enumerate() {
            if !area_matches_mask(polygon.area, area_mask) {
                continue;
            }
            let closest = closest_point_on_triangle(source, &polygon.vertices);
            let dist = distance(source, closest);
            if dist <= max_distance + VERTEX_EPSILON
                && best.as_ref().is_none_or(|current| dist < current.distance)
            {
                best = Some(NavMeshHit {
                    position: closest,
                    normal: triangle_normal(&polygon.vertices),
                    distance: dist.min(max_distance),
                    mask: area_bit(polygon.area),
                    hit: true,
                    polygon_index: Some(index),
                });
            }
        }
        best
    }

    pub fn find_closest_edge(&self, source: [f64; 3], area_mask: u32) -> Option<NavMeshHit> {
        if !source.iter().all(|component| component.is_finite()) {
            return None;
        }
        let mut best: Option<NavMeshHit> = None;
        for (index, polygon) in self.polygons.iter().enumerate() {
            if !area_matches_mask(polygon.area, area_mask) {
                continue;
            }
            for edge_index in 0..3 {
                let edge_start = polygon.vertices[edge_index];
                let edge_end = polygon.vertices[(edge_index + 1) % 3];
                if self.is_shared_masked_edge(index, edge_start, edge_end, area_mask) {
                    continue;
                }
                let closest = closest_point_on_segment(edge_start, edge_end, source);
                let dist = distance(source, closest);
                if best.as_ref().is_none_or(|current| dist < current.distance) {
                    best = Some(NavMeshHit {
                        position: closest,
                        normal: boundary_edge_normal(&polygon.vertices, edge_start, edge_end),
                        distance: dist,
                        mask: area_bit(polygon.area),
                        hit: true,
                        polygon_index: Some(index),
                    });
                }
            }
        }
        best
    }

    pub fn raycast(&self, source: [f64; 3], target: [f64; 3], area_mask: u32) -> NavMeshHit {
        if !source.iter().all(|component| component.is_finite())
            || !target.iter().all(|component| component.is_finite())
        {
            return NavMeshHit {
                position: source,
                normal: [0.0; 3],
                distance: 0.0,
                mask: 0,
                hit: true,
                polygon_index: None,
            };
        }

        let start_poly = self.polygons.iter().enumerate().find(|(_, poly)| {
            area_matches_mask(poly.area, area_mask) && point_in_triangle(source, &poly.vertices)
        });
        let Some((start_index, start_polygon)) = start_poly else {
            return NavMeshHit {
                position: source,
                normal: [0.0; 3],
                distance: 0.0,
                mask: 0,
                hit: true,
                polygon_index: None,
            };
        };

        let total_distance = distance(source, target);
        if total_distance <= VERTEX_EPSILON {
            return NavMeshHit {
                position: target,
                normal: [0.0; 3],
                distance: 0.0,
                mask: area_bit(start_polygon.area),
                hit: false,
                polygon_index: Some(start_index),
            };
        }

        let mut boundary_hits = Vec::new();
        for (index, polygon) in self.polygons.iter().enumerate() {
            if !area_matches_mask(polygon.area, area_mask) {
                continue;
            }
            for edge_index in 0..3 {
                let edge_start = polygon.vertices[edge_index];
                let edge_end = polygon.vertices[(edge_index + 1) % 3];
                if self.is_shared_masked_edge(index, edge_start, edge_end, area_mask) {
                    continue;
                }
                let outward = boundary_edge_normal(&polygon.vertices, edge_start, edge_end);
                let ray_dir = subtract(target, source);
                if dot(ray_dir, outward) <= AREA_EPSILON {
                    continue;
                }
                if let Some((t, point)) =
                    intersect_segments_xz(source, target, edge_start, edge_end)
                {
                    boundary_hits.push((t, point, outward, index, polygon.area));
                }
            }
        }

        boundary_hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, point, normal, poly_index, area)) = boundary_hits.first().copied() {
            return NavMeshHit {
                position: point,
                normal,
                distance: distance(source, point),
                mask: area_bit(area),
                hit: true,
                polygon_index: Some(poly_index),
            };
        }

        let target_poly = self.polygons.iter().enumerate().find(|(_, poly)| {
            area_matches_mask(poly.area, area_mask) && point_in_triangle(target, &poly.vertices)
        });
        if let Some((end_index, end_polygon)) = target_poly {
            NavMeshHit {
                position: target,
                normal: [0.0; 3],
                distance: total_distance,
                mask: area_bit(end_polygon.area),
                hit: false,
                polygon_index: Some(end_index),
            }
        } else {
            NavMeshHit {
                position: source,
                normal: [0.0; 3],
                distance: 0.0,
                mask: area_bit(start_polygon.area),
                hit: true,
                polygon_index: Some(start_index),
            }
        }
    }

    pub(crate) fn min_area_cost(&self) -> f64 {
        self.min_area_cost
    }

    fn is_shared_masked_edge(
        &self,
        polygon_index: usize,
        edge_start: [f64; 3],
        edge_end: [f64; 3],
        area_mask: u32,
    ) -> bool {
        self.polygons
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                other_index != polygon_index
                    && area_matches_mask(other.area, area_mask)
                    && polygon_has_edge(&other.vertices, edge_start, edge_end)
            })
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

pub(crate) fn area_bit(area: u32) -> u32 {
    if area < 32 {
        1u32 << area
    } else {
        0
    }
}

pub(crate) fn area_matches_mask(area: u32, area_mask: u32) -> bool {
    (area_bit(area) & area_mask) != 0
}

fn polygon_has_edge(vertices: &[[f64; 3]; 3], start: [f64; 3], end: [f64; 3]) -> bool {
    let has_start = vertices
        .iter()
        .any(|vertex| distance(*vertex, start) <= VERTEX_EPSILON);
    let has_end = vertices
        .iter()
        .any(|vertex| distance(*vertex, end) <= VERTEX_EPSILON);
    has_start && has_end
}

fn triangle_normal(vertices: &[[f64; 3]; 3]) -> [f64; 3] {
    let edge_one = subtract(vertices[1], vertices[0]);
    let edge_two = subtract(vertices[2], vertices[0]);
    let mut normal = cross(edge_one, edge_two);
    let len = vector_length(normal);
    if len <= AREA_EPSILON {
        return [0.0, 1.0, 0.0];
    }
    normal = [normal[0] / len, normal[1] / len, normal[2] / len];
    if normal[1] < 0.0 {
        normal = [-normal[0], -normal[1], -normal[2]];
    }
    normal
}

fn boundary_edge_normal(
    vertices: &[[f64; 3]; 3],
    edge_start: [f64; 3],
    edge_end: [f64; 3],
) -> [f64; 3] {
    let centroid = triangle_centroid(vertices);
    let edge_dir = subtract(edge_end, edge_start);
    let edge_len = vector_length(edge_dir);
    if edge_len <= AREA_EPSILON {
        return [0.0, 0.0, 1.0];
    }
    let unit_edge = [
        edge_dir[0] / edge_len,
        edge_dir[1] / edge_len,
        edge_dir[2] / edge_len,
    ];
    let to_edge = subtract(midpoint(edge_start, edge_end), centroid);
    let proj = dot(to_edge, unit_edge);
    let perp = [
        to_edge[0] - unit_edge[0] * proj,
        to_edge[1] - unit_edge[1] * proj,
        to_edge[2] - unit_edge[2] * proj,
    ];
    let perp_len = vector_length(perp);
    if perp_len <= AREA_EPSILON {
        return [0.0, 0.0, 1.0];
    }
    [
        perp[0] / perp_len,
        perp[1] / perp_len,
        perp[2] / perp_len,
    ]
}

fn intersect_segments_xz(
    ray_start: [f64; 3],
    ray_end: [f64; 3],
    edge_start: [f64; 3],
    edge_end: [f64; 3],
) -> Option<(f64, [f64; 3])> {
    let rx = ray_end[0] - ray_start[0];
    let rz = ray_end[2] - ray_start[2];
    let sx = edge_end[0] - edge_start[0];
    let sz = edge_end[2] - edge_start[2];
    let denom = rx * sz - rz * sx;
    if denom.abs() <= AREA_EPSILON {
        return None;
    }
    let qpx = edge_start[0] - ray_start[0];
    let qpz = edge_start[2] - ray_start[2];
    let t = (qpx * sz - qpz * sx) / denom;
    let u = (qpx * rz - qpz * rx) / denom;
    if (-POINT_EPSILON..=1.0 + POINT_EPSILON).contains(&t)
        && (-POINT_EPSILON..=1.0 + POINT_EPSILON).contains(&u)
    {
        let clamped_t = t.clamp(0.0, 1.0);
        let clamped_u = u.clamp(0.0, 1.0);
        let point = [
            ray_start[0] + rx * clamped_t,
            edge_start[1] + (edge_end[1] - edge_start[1]) * clamped_u,
            ray_start[2] + rz * clamped_t,
        ];
        Some((clamped_t, point))
    } else {
        None
    }
}

fn closest_point_on_triangle(point: [f64; 3], vertices: &[[f64; 3]; 3]) -> [f64; 3] {
    let a = vertices[0];
    let b = vertices[1];
    let c = vertices[2];
    let ab = subtract(b, a);
    let ac = subtract(c, a);
    let ap = subtract(point, a);

    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = subtract(point, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [a[0] + v * ab[0], a[1] + v * ab[1], a[2] + v * ab[2]];
    }

    let cp = subtract(point, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [a[0] + w * ac[0], a[1] + w * ac[1], a[2] + w * ac[2]];
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = subtract(c, b);
        return [b[0] + w * bc[0], b[1] + w * bc[1], b[2] + w * bc[2]];
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    [
        a[0] + ab[0] * v + ac[0] * w,
        a[1] + ab[1] * v + ac[1] * w,
        a[2] + ab[2] * v + ac[2] * w,
    ]
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
