use std::collections::BTreeMap;

use crate::two_d::collider::Shape2D;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompositeGeometryType2D {
    #[default]
    Outlines,
    Polygons,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompositeGenerationType2D {
    #[default]
    Synchronous,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositeCollider2D {
    pub geometry_type: CompositeGeometryType2D,
    pub generation_type: CompositeGenerationType2D,
    pub vertex_distance: f32,
    pub offset_distance: f32,
    pub edge_radius: f32,
}

impl Default for CompositeCollider2D {
    fn default() -> Self {
        Self {
            geometry_type: CompositeGeometryType2D::Outlines,
            generation_type: CompositeGenerationType2D::Synchronous,
            vertex_distance: 0.000_05,
            offset_distance: 0.000_005,
            edge_radius: 0.0,
        }
    }
}

impl CompositeCollider2D {
    pub fn new(
        geometry_type: CompositeGeometryType2D,
        generation_type: CompositeGenerationType2D,
    ) -> Self {
        Self {
            geometry_type,
            generation_type,
            ..Self::default()
        }
    }

    pub fn with_geometry_type(mut self, geometry_type: CompositeGeometryType2D) -> Self {
        self.geometry_type = geometry_type;
        self
    }

    pub fn with_generation_type(mut self, generation_type: CompositeGenerationType2D) -> Self {
        self.generation_type = generation_type;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TileColliderType2D {
    None,
    Grid,
    Sprite { vertices: Vec<[f32; 2]> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TilemapCollider2D {
    pub cell_size: [f32; 2],
    pub used_by_composite: bool,
    tiles: BTreeMap<[i32; 2], TileColliderType2D>,
}

impl Default for TilemapCollider2D {
    fn default() -> Self {
        Self::new([1.0, 1.0])
    }
}

impl TilemapCollider2D {
    pub fn new(cell_size: [f32; 2]) -> Self {
        Self {
            cell_size,
            used_by_composite: false,
            tiles: BTreeMap::new(),
        }
    }

    pub fn with_used_by_composite(mut self, used_by_composite: bool) -> Self {
        self.used_by_composite = used_by_composite;
        self
    }

    pub fn set_tile(&mut self, cell: [i32; 2], collider_type: TileColliderType2D) {
        if matches!(collider_type, TileColliderType2D::None) {
            self.tiles.remove(&cell);
        } else {
            self.tiles.insert(cell, collider_type);
        }
    }

    pub fn remove_tile(&mut self, cell: [i32; 2]) -> bool {
        self.tiles.remove(&cell).is_some()
    }

    pub fn tile(&self, cell: [i32; 2]) -> Option<&TileColliderType2D> {
        self.tiles.get(&cell)
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn build_shapes(&self) -> Vec<Shape2D> {
        let cw = self.cell_size[0].max(0.0);
        let ch = self.cell_size[1].max(0.0);
        if cw <= 0.0 || ch <= 0.0 {
            return Vec::new();
        }
        let mut grid_rows: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
        let mut shapes = Vec::new();
        for (cell, kind) in &self.tiles {
            match kind {
                TileColliderType2D::None => {}
                TileColliderType2D::Grid => {
                    grid_rows.entry(cell[1]).or_default().push(cell[0]);
                }
                TileColliderType2D::Sprite { vertices } => {
                    if vertices.len() >= 3 {
                        let base_x = (cell[0] as f32) * cw;
                        let base_y = (cell[1] as f32) * ch;
                        let world_verts = vertices
                            .iter()
                            .map(|v| [base_x + v[0] * cw, base_y + v[1] * ch])
                            .collect();
                        shapes.push(Shape2D::Polygon {
                            vertices: world_verts,
                        });
                    }
                }
            }
        }
        for (y, mut xs) in grid_rows {
            xs.sort_unstable();
            xs.dedup();
            let mut idx = 0usize;
            while idx < xs.len() {
                let start_x = xs[idx];
                let mut end_x = start_x;
                while idx + 1 < xs.len() && xs[idx + 1] == end_x + 1 {
                    idx += 1;
                    end_x = xs[idx];
                }
                let min_x = (start_x as f32) * cw;
                let max_x = ((end_x + 1) as f32) * cw;
                let min_y = (y as f32) * ch;
                let max_y = ((y + 1) as f32) * ch;
                shapes.push(Shape2D::Polygon {
                    vertices: vec![
                        [min_x, min_y],
                        [max_x, min_y],
                        [max_x, max_y],
                        [min_x, max_y],
                    ],
                });
                idx += 1;
            }
        }
        shapes
    }
}

pub fn merge_composite_outlines(shapes: Vec<Shape2D>, tolerance: f32) -> Vec<Shape2D> {
    let eps = tolerance.max(1e-4);
    let mut polys: Vec<Vec<[f32; 2]>> = Vec::new();
    let mut others: Vec<Shape2D> = Vec::new();
    for shape in shapes {
        match shape {
            Shape2D::Polygon { vertices } if vertices.len() >= 3 => {
                polys.push(normalize_ccw_polygon(vertices, eps));
            }
            Shape2D::Box { half_extents } if half_extents[0] > 0.0 && half_extents[1] > 0.0 => {
                let hx = half_extents[0];
                let hy = half_extents[1];
                polys.push(vec![[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]);
            }
            other => others.push(other),
        }
    }
    loop {
        let mut merged_pair = false;
        'outer: for i in 0..polys.len() {
            for j in (i + 1)..polys.len() {
                if let Some(union_poly) = try_merge_convex_polygons(&polys[i], &polys[j], eps) {
                    polys[i] = union_poly;
                    polys.remove(j);
                    merged_pair = true;
                    break 'outer;
                }
            }
        }
        if !merged_pair {
            break;
        }
    }
    let mut out: Vec<Shape2D> = polys
        .into_iter()
        .filter(|vertices| vertices.len() >= 3)
        .map(|vertices| Shape2D::Polygon { vertices })
        .collect();
    out.extend(others);
    out
}

fn polygon_signed_area(verts: &[[f32; 2]]) -> f32 {
    let mut area = 0.0f32;
    let n = verts.len();
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        area += a[0] * b[1] - b[0] * a[1];
    }
    0.5 * area
}

fn normalize_ccw_polygon(mut verts: Vec<[f32; 2]>, eps: f32) -> Vec<[f32; 2]> {
    if polygon_signed_area(&verts) < 0.0 {
        verts.reverse();
    }
    simplify_collinear_polygon(verts, eps)
}

fn simplify_collinear_polygon(verts: Vec<[f32; 2]>, eps: f32) -> Vec<[f32; 2]> {
    let mut deduped: Vec<[f32; 2]> = Vec::with_capacity(verts.len());
    for v in verts {
        if deduped
            .last()
            .is_none_or(|prev| !points_close(*prev, v, eps))
        {
            deduped.push(v);
        }
    }
    if deduped.len() >= 2 {
        if let (Some(first), Some(last)) = (deduped.first(), deduped.last()) {
            if points_close(*first, *last, eps) {
                deduped.pop();
            }
        }
    }
    if deduped.len() < 3 {
        return deduped;
    }
    let mut cleaned = Vec::with_capacity(deduped.len());
    let n = deduped.len();
    for i in 0..n {
        let prev = deduped[(i + n - 1) % n];
        let curr = deduped[i];
        let next = deduped[(i + 1) % n];
        let e1 = [curr[0] - prev[0], curr[1] - prev[1]];
        let e2 = [next[0] - curr[0], next[1] - curr[1]];
        let cross = e1[0] * e2[1] - e1[1] * e2[0];
        let dot = e1[0] * e2[0] + e1[1] * e2[1];
        let len1 = (e1[0] * e1[0] + e1[1] * e1[1]).sqrt();
        let len2 = (e2[0] * e2[0] + e2[1] * e2[1]).sqrt();
        let denom = (len1 * len2).max(1e-6);
        if (cross.abs() / denom) <= eps && dot >= 0.0 {
            continue;
        }
        cleaned.push(curr);
    }
    cleaned
}

fn points_close(a: [f32; 2], b: [f32; 2], eps: f32) -> bool {
    (a[0] - b[0]).abs() <= eps && (a[1] - b[1]).abs() <= eps
}

fn is_strictly_convex_ccw(verts: &[[f32; 2]], eps: f32) -> bool {
    let n = verts.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        let c = verts[(i + 2) % n];
        let e1 = [b[0] - a[0], b[1] - a[1]];
        let e2 = [c[0] - b[0], c[1] - b[1]];
        let cross = e1[0] * e2[1] - e1[1] * e2[0];
        if cross < -eps {
            return false;
        }
    }
    polygon_signed_area(verts) > eps * eps
}

fn axis_aligned_bounds(verts: &[[f32; 2]], eps: f32) -> Option<([f32; 2], [f32; 2])> {
    if verts.len() != 4 {
        return None;
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for v in verts {
        min_x = min_x.min(v[0]);
        min_y = min_y.min(v[1]);
        max_x = max_x.max(v[0]);
        max_y = max_y.max(v[1]);
    }
    if max_x - min_x <= eps || max_y - min_y <= eps {
        return None;
    }
    let corners = [
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    for c in corners {
        if !verts.iter().any(|v| points_close(*v, c, eps)) {
            return None;
        }
    }
    Some(([min_x, min_y], [max_x, max_y]))
}

fn try_merge_axis_aligned_boxes(a: &[[f32; 2]], b: &[[f32; 2]], eps: f32) -> Option<Vec<[f32; 2]>> {
    let (min_a, max_a) = axis_aligned_bounds(a, eps)?;
    let (min_b, max_b) = axis_aligned_bounds(b, eps)?;
    let same_y = (min_a[1] - min_b[1]).abs() <= eps && (max_a[1] - max_b[1]).abs() <= eps;
    let touch_x = max_a[0] >= min_b[0] - eps && min_a[0] <= max_b[0] + eps;
    if same_y && touch_x {
        let min_x = min_a[0].min(min_b[0]);
        let max_x = max_a[0].max(max_b[0]);
        let min_y = 0.5 * (min_a[1] + min_b[1]);
        let max_y = 0.5 * (max_a[1] + max_b[1]);
        return Some(vec![
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ]);
    }
    let same_x = (min_a[0] - min_b[0]).abs() <= eps && (max_a[0] - max_b[0]).abs() <= eps;
    let touch_y = max_a[1] >= min_b[1] - eps && min_a[1] <= max_b[1] + eps;
    if same_x && touch_y {
        let min_x = 0.5 * (min_a[0] + min_b[0]);
        let max_x = 0.5 * (max_a[0] + max_b[0]);
        let min_y = min_a[1].min(min_b[1]);
        let max_y = max_a[1].max(max_b[1]);
        return Some(vec![
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ]);
    }
    None
}

fn try_merge_convex_polygons(a: &[[f32; 2]], b: &[[f32; 2]], eps: f32) -> Option<Vec<[f32; 2]>> {
    if let Some(merged_box) = try_merge_axis_aligned_boxes(a, b, eps) {
        return Some(merged_box);
    }
    let na = a.len();
    let nb = b.len();
    for ia in 0..na {
        let a0 = a[ia];
        let a1 = a[(ia + 1) % na];
        for ib in 0..nb {
            let b0 = b[ib];
            let b1 = b[(ib + 1) % nb];
            if points_close(a0, b1, eps) && points_close(a1, b0, eps) {
                let mut combined = Vec::with_capacity(na + nb - 2);
                for k in 1..na {
                    combined.push(a[(ia + k) % na]);
                }
                for k in 1..(nb - 1) {
                    combined.push(b[(ib + 1 + k) % nb]);
                }
                let simplified = simplify_collinear_polygon(combined, eps);
                if is_strictly_convex_ccw(&simplified, eps) {
                    return Some(simplified);
                }
            }
        }
    }
    None
}
