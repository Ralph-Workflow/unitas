use unitas_core::EntityId;

use crate::two_d::body::BodyId2D;

const EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColliderId2D(u64);

impl ColliderId2D {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for ColliderId2D {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ColliderId2D> for u64 {
    fn from(id: ColliderId2D) -> Self {
        id.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape2D {
    Box { half_extents: [f32; 2] },
    Circle { radius: f32 },
    Edge { start: [f32; 2], end: [f32; 2] },
    Polygon { vertices: Vec<[f32; 2]> },
    Composite { shapes: Vec<Shape2D> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial2D {
    pub friction: f32,
    pub bounciness: f32,
}

impl Default for PhysicsMaterial2D {
    fn default() -> Self {
        Self {
            friction: 0.4,
            bounciness: 0.0,
        }
    }
}

impl PhysicsMaterial2D {
    pub const fn new(friction: f32, bounciness: f32) -> Self {
        Self {
            friction,
            bounciness,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColliderDesc2D {
    pub shape: Shape2D,
    pub offset: [f32; 2],
    pub rotation: f32,
    pub is_trigger: bool,
    pub material: Option<PhysicsMaterial2D>,
    pub used_by_effector: bool,
    pub used_by_composite: bool,
    pub game_object: EntityId,
    pub attached_body: Option<BodyId2D>,
    pub layer: u8,
    pub enabled: bool,
    pub active: bool,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Default for ColliderDesc2D {
    fn default() -> Self {
        Self {
            shape: Shape2D::Box {
                half_extents: [0.5, 0.5],
            },
            offset: [0.0, 0.0],
            rotation: 0.0,
            is_trigger: false,
            material: None,
            used_by_effector: false,
            used_by_composite: false,
            game_object: EntityId::new(0),
            attached_body: None,
            layer: 0,
            enabled: true,
            active: true,
            min_depth: 0.0,
            max_depth: 0.0,
        }
    }
}

impl ColliderDesc2D {
    pub fn box_collider(game_object: EntityId, half_extents: [f32; 2]) -> Self {
        Self {
            shape: Shape2D::Box { half_extents },
            game_object,
            ..Self::default()
        }
    }

    pub fn circle_collider(game_object: EntityId, radius: f32) -> Self {
        Self {
            shape: Shape2D::Circle { radius },
            game_object,
            ..Self::default()
        }
    }

    pub fn edge_collider(game_object: EntityId, start: [f32; 2], end: [f32; 2]) -> Self {
        Self {
            shape: Shape2D::Edge { start, end },
            game_object,
            ..Self::default()
        }
    }

    pub fn polygon_collider(game_object: EntityId, vertices: Vec<[f32; 2]>) -> Self {
        Self {
            shape: Shape2D::Polygon { vertices },
            game_object,
            ..Self::default()
        }
    }

    pub fn with_attached_body(mut self, body: BodyId2D) -> Self {
        self.attached_body = Some(body);
        self
    }

    pub fn with_offset(mut self, offset: [f32; 2]) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_trigger(mut self, is_trigger: bool) -> Self {
        self.is_trigger = is_trigger;
        self
    }

    pub fn with_material(mut self, material: PhysicsMaterial2D) -> Self {
        self.material = Some(material);
        self
    }

    pub fn with_used_by_effector(mut self, used_by_effector: bool) -> Self {
        self.used_by_effector = used_by_effector;
        self
    }

    pub fn with_used_by_composite(mut self, used_by_composite: bool) -> Self {
        self.used_by_composite = used_by_composite;
        self
    }

    pub fn with_layer(mut self, layer: u8) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_depth(mut self, min_depth: f32, max_depth: f32) -> Self {
        self.min_depth = min_depth;
        self.max_depth = max_depth;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Collider2D {
    pub id: ColliderId2D,
    pub shape: Shape2D,
    pub offset: [f32; 2],
    pub rotation: f32,
    pub is_trigger: bool,
    pub material: PhysicsMaterial2D,
    pub uses_default_material: bool,
    pub used_by_effector: bool,
    pub used_by_composite: bool,
    pub game_object: EntityId,
    pub attached_body: Option<BodyId2D>,
    pub layer: u8,
    pub enabled: bool,
    pub active: bool,
    pub min_depth: f32,
    pub max_depth: f32,
    pub(crate) query_offset: [f32; 2],
    pub(crate) query_rotation: f32,
}

impl Collider2D {
    pub fn from_desc(id: ColliderId2D, desc: ColliderDesc2D) -> Self {
        let (material, uses_default_material) = match desc.material {
            Some(material) => (material, false),
            None => (PhysicsMaterial2D::default(), true),
        };
        Self {
            id,
            shape: desc.shape,
            offset: desc.offset,
            rotation: desc.rotation,
            is_trigger: desc.is_trigger,
            material,
            uses_default_material,
            used_by_effector: desc.used_by_effector,
            used_by_composite: desc.used_by_composite,
            game_object: desc.game_object,
            attached_body: desc.attached_body,
            layer: desc.layer,
            enabled: desc.enabled,
            active: desc.active,
            min_depth: desc.min_depth,
            max_depth: desc.max_depth,
            query_offset: desc.offset,
            query_rotation: desc.rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RawContact2D {
    pub(crate) point: [f32; 2],
    pub(crate) normal: [f32; 2],
    pub(crate) depth: f32,
}

pub(crate) fn rotate_vec2(v: [f32; 2], angle: f32) -> [f32; 2] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

pub(crate) fn transform_point2(local: [f32; 2], center: [f32; 2], angle: f32) -> [f32; 2] {
    let r = rotate_vec2(local, angle);
    [center[0] + r[0], center[1] + r[1]]
}

pub(crate) fn inverse_transform_point2(world: [f32; 2], center: [f32; 2], angle: f32) -> [f32; 2] {
    let rel = [world[0] - center[0], world[1] - center[1]];
    rotate_vec2(rel, -angle)
}

pub(crate) fn dot2(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

pub(crate) fn length2(v: [f32; 2]) -> f32 {
    dot2(v, v).sqrt()
}

pub(crate) fn normalize2(v: [f32; 2]) -> Option<[f32; 2]> {
    let len = length2(v);
    if !len.is_finite() || len <= EPSILON {
        None
    } else {
        Some([v[0] / len, v[1] / len])
    }
}

pub(crate) fn normal_angle_degrees(normal: [f32; 2]) -> f32 {
    let deg = normal[1].atan2(normal[0]).to_degrees();
    if deg < 0.0 {
        deg + 360.0
    } else {
        deg
    }
}

pub(crate) fn surface_normal_at_point(
    shape: &Shape2D,
    center: [f32; 2],
    angle: f32,
    point: [f32; 2],
) -> Option<[f32; 2]> {
    match shape {
        Shape2D::Circle { .. } => Some(normalize2(sub2(point, center)).unwrap_or([1.0, 0.0])),
        Shape2D::Box { half_extents } => {
            let local = inverse_transform_point2(point, center, angle);
            let pen_x = half_extents[0] - local[0].abs();
            let pen_y = half_extents[1] - local[1].abs();
            let local_normal = if pen_x <= pen_y {
                [if local[0] < 0.0 { -1.0 } else { 1.0 }, 0.0]
            } else {
                [0.0, if local[1] < 0.0 { -1.0 } else { 1.0 }]
            };
            Some(rotate_vec2(local_normal, angle))
        }
        Shape2D::Edge { start, end } => {
            let a = transform_point2(*start, center, angle);
            let b = transform_point2(*end, center, angle);
            let cp = closest_point_on_segment2(a, b, point);
            if let Some(n) = normalize2(sub2(point, cp)) {
                return Some(n);
            }
            let tangent = sub2(b, a);
            normalize2([-tangent[1], tangent[0]])
        }
        Shape2D::Polygon { .. } => {
            let verts = shape_vertices_world(shape, center, angle)?;
            let mut centroid = [0.0, 0.0];
            for v in &verts {
                centroid = add2(centroid, *v);
            }
            centroid = scale2(centroid, 1.0 / verts.len() as f32);
            let mut best: Option<([f32; 2], [f32; 2], f32)> = None;
            for i in 0..verts.len() {
                let a = verts[i];
                let b = verts[(i + 1) % verts.len()];
                let cp = closest_point_on_segment2(a, b, point);
                let d = sub2(point, cp);
                let d2 = dot2(d, d);
                if best.is_none_or(|(_, _, best_d2)| d2 < best_d2) {
                    best = Some((cp, sub2(b, a), d2));
                }
            }
            let (cp, edge, _) = best?;
            let raw = normalize2(sub2(point, cp)).or_else(|| normalize2([-edge[1], edge[0]]))?;
            // Orient outward, away from the centroid.
            if dot2(raw, sub2(cp, centroid)) >= 0.0 {
                Some(raw)
            } else {
                Some([-raw[0], -raw[1]])
            }
        }
        Shape2D::Composite { shapes } => shapes
            .iter()
            .find_map(|sub| surface_normal_at_point(sub, center, angle, point)),
    }
}

pub(crate) fn sub2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

pub(crate) fn add2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

pub(crate) fn scale2(v: [f32; 2], s: f32) -> [f32; 2] {
    [v[0] * s, v[1] * s]
}

fn shape_vertices_world(shape: &Shape2D, center: [f32; 2], angle: f32) -> Option<Vec<[f32; 2]>> {
    match shape {
        Shape2D::Box { half_extents } => {
            let hx = half_extents[0];
            let hy = half_extents[1];
            let locals = [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]];
            Some(
                locals
                    .iter()
                    .map(|p| transform_point2(*p, center, angle))
                    .collect(),
            )
        }
        Shape2D::Edge { start, end } => Some(vec![
            transform_point2(*start, center, angle),
            transform_point2(*end, center, angle),
        ]),
        Shape2D::Polygon { vertices } => {
            if vertices.len() < 2 {
                return None;
            }
            Some(
                vertices
                    .iter()
                    .map(|p| transform_point2(*p, center, angle))
                    .collect(),
            )
        }
        Shape2D::Circle { .. } | Shape2D::Composite { .. } => None,
    }
}

pub(crate) fn shape_to_local_shapes_in_frame(
    shape: &Shape2D,
    rel_offset: [f32; 2],
    rel_angle: f32,
) -> Vec<Shape2D> {
    match shape {
        Shape2D::Box { half_extents } => {
            if half_extents[0] <= 0.0 || half_extents[1] <= 0.0 {
                return Vec::new();
            }
            let hx = half_extents[0];
            let hy = half_extents[1];
            let locals = [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]];
            let vertices = locals
                .iter()
                .map(|p| transform_point2(*p, rel_offset, rel_angle))
                .collect();
            vec![Shape2D::Polygon { vertices }]
        }
        Shape2D::Edge { start, end } => vec![Shape2D::Edge {
            start: transform_point2(*start, rel_offset, rel_angle),
            end: transform_point2(*end, rel_offset, rel_angle),
        }],
        Shape2D::Polygon { vertices } => {
            if vertices.len() < 2 {
                return Vec::new();
            }
            let mapped = vertices
                .iter()
                .map(|p| transform_point2(*p, rel_offset, rel_angle))
                .collect();
            vec![Shape2D::Polygon { vertices: mapped }]
        }
        Shape2D::Circle { radius } => {
            if *radius <= 0.0 {
                return Vec::new();
            }
            if rel_offset[0].abs() <= EPSILON && rel_offset[1].abs() <= EPSILON {
                vec![Shape2D::Circle { radius: *radius }]
            } else {
                let mut vertices = Vec::with_capacity(12);
                for k in 0..12 {
                    let theta = (k as f32) * (std::f32::consts::TAU / 12.0);
                    let (s, c) = theta.sin_cos();
                    vertices.push([rel_offset[0] + c * radius, rel_offset[1] + s * radius]);
                }
                vec![Shape2D::Polygon { vertices }]
            }
        }
        Shape2D::Composite { shapes } => {
            let mut out = Vec::new();
            for sub in shapes {
                out.extend(shape_to_local_shapes_in_frame(sub, rel_offset, rel_angle));
            }
            out
        }
    }
}

pub(crate) fn closest_point_on_segment2(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> [f32; 2] {
    let ab = sub2(b, a);
    let len_sq = dot2(ab, ab);
    if len_sq <= EPSILON * EPSILON {
        return a;
    }
    let t = (dot2(sub2(p, a), ab) / len_sq).clamp(0.0, 1.0);
    [a[0] + ab[0] * t, a[1] + ab[1] * t]
}

fn point_inside_convex_polygon(point: [f32; 2], verts: &[[f32; 2]]) -> bool {
    if verts.len() < 3 {
        return false;
    }
    let mut pos = false;
    let mut neg = false;
    for i in 0..verts.len() {
        let a = verts[i];
        let b = verts[(i + 1) % verts.len()];
        let edge = sub2(b, a);
        let to_p = sub2(point, a);
        let cross = edge[0] * to_p[1] - edge[1] * to_p[0];
        if cross > EPSILON {
            pos = true;
        } else if cross < -EPSILON {
            neg = true;
        }
        if pos && neg {
            return false;
        }
    }
    true
}

fn closest_point_on_polygon_boundary(
    point: [f32; 2],
    verts: &[[f32; 2]],
) -> ([f32; 2], [f32; 2], f32) {
    let mut best_pt = verts[0];
    let mut best_dist_sq = f32::INFINITY;
    let mut best_normal = [0.0, 1.0];
    let n = verts.len();
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        let cp = closest_point_on_segment2(a, b, point);
        let d = sub2(point, cp);
        let dist_sq = dot2(d, d);
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_pt = cp;
            let edge = sub2(b, a);
            best_normal = normalize2([edge[1], -edge[0]]).unwrap_or([0.0, 1.0]);
        }
    }
    (best_pt, best_normal, best_dist_sq.sqrt())
}

pub(crate) fn compute_contact2d(
    shape_a: &Shape2D,
    center_a: [f32; 2],
    angle_a: f32,
    shape_b: &Shape2D,
    center_b: [f32; 2],
    angle_b: f32,
) -> Option<RawContact2D> {
    match (shape_a, shape_b) {
        (Shape2D::Composite { shapes }, _) => {
            let mut best: Option<RawContact2D> = None;
            for sub in shapes {
                if let Some(c) =
                    compute_contact2d(sub, center_a, angle_a, shape_b, center_b, angle_b)
                {
                    if best.is_none_or(|b| c.depth > b.depth) {
                        best = Some(c);
                    }
                }
            }
            best
        }
        (_, Shape2D::Composite { shapes }) => {
            let mut best: Option<RawContact2D> = None;
            for sub in shapes {
                if let Some(c) =
                    compute_contact2d(shape_a, center_a, angle_a, sub, center_b, angle_b)
                {
                    if best.is_none_or(|b| c.depth > b.depth) {
                        best = Some(c);
                    }
                }
            }
            best
        }
        (Shape2D::Circle { radius: ra }, Shape2D::Circle { radius: rb }) => {
            circle_circle_contact(center_a, *ra, center_b, *rb)
        }
        (Shape2D::Circle { radius }, _) => {
            let verts_b = shape_vertices_world(shape_b, center_b, angle_b)?;
            circle_polygon_contact(center_a, *radius, center_b, &verts_b)
        }
        (_, Shape2D::Circle { radius }) => {
            let verts_a = shape_vertices_world(shape_a, center_a, angle_a)?;
            let contact = circle_polygon_contact(center_b, *radius, center_a, &verts_a)?;
            Some(RawContact2D {
                point: contact.point,
                normal: [-contact.normal[0], -contact.normal[1]],
                depth: contact.depth,
            })
        }
        _ => {
            let verts_a = shape_vertices_world(shape_a, center_a, angle_a)?;
            let verts_b = shape_vertices_world(shape_b, center_b, angle_b)?;
            polygon_polygon_contact(center_a, &verts_a, center_b, &verts_b)
        }
    }
}

fn circle_circle_contact(
    center_a: [f32; 2],
    radius_a: f32,
    center_b: [f32; 2],
    radius_b: f32,
) -> Option<RawContact2D> {
    let delta = sub2(center_b, center_a);
    let dist = length2(delta);
    let sum = radius_a + radius_b;
    if dist > sum {
        return None;
    }
    let normal = normalize2(delta).unwrap_or([0.0, 1.0]);
    let point = [
        center_a[0] + normal[0] * radius_a,
        center_a[1] + normal[1] * radius_a,
    ];
    Some(RawContact2D {
        point,
        normal,
        depth: sum - dist,
    })
}

fn circle_polygon_contact(
    circle_center: [f32; 2],
    radius: f32,
    poly_center: [f32; 2],
    verts: &[[f32; 2]],
) -> Option<RawContact2D> {
    if verts.len() == 2 {
        let cp = closest_point_on_segment2(verts[0], verts[1], circle_center);
        let delta = sub2(cp, circle_center);
        let dist = length2(delta);
        if dist > radius {
            return None;
        }
        let normal = if dist > EPSILON {
            [delta[0] / dist, delta[1] / dist]
        } else {
            let edge = sub2(verts[1], verts[0]);
            normalize2([-edge[1], edge[0]]).unwrap_or([0.0, -1.0])
        };
        return Some(RawContact2D {
            point: cp,
            normal,
            depth: radius - dist,
        });
    }
    let inside = point_inside_convex_polygon(circle_center, verts);
    let (cp, edge_outward, dist) = closest_point_on_polygon_boundary(circle_center, verts);
    if inside {
        let to_poly = sub2(poly_center, circle_center);
        let normal = normalize2(sub2(circle_center, cp))
            .map(|n| [-n[0], -n[1]])
            .or_else(|| normalize2(to_poly))
            .unwrap_or([-edge_outward[0], -edge_outward[1]]);
        return Some(RawContact2D {
            point: cp,
            normal,
            depth: radius + dist,
        });
    }
    if dist > radius {
        return None;
    }
    let delta = sub2(cp, circle_center);
    let normal = normalize2(delta)
        .or_else(|| normalize2(sub2(poly_center, circle_center)))
        .unwrap_or([0.0, -1.0]);
    Some(RawContact2D {
        point: cp,
        normal,
        depth: radius - dist,
    })
}

fn project_polygon(verts: &[[f32; 2]], axis: [f32; 2]) -> (f32, f32) {
    let mut min_p = f32::INFINITY;
    let mut max_p = f32::NEG_INFINITY;
    for v in verts {
        let p = dot2(*v, axis);
        if p < min_p {
            min_p = p;
        }
        if p > max_p {
            max_p = p;
        }
    }
    (min_p, max_p)
}

fn candidate_axes(verts: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if verts.len() == 2 {
        let edge = sub2(verts[1], verts[0]);
        let mut axes = Vec::new();
        if let Some(n) = normalize2([-edge[1], edge[0]]) {
            axes.push(n);
        }
        if let Some(t) = normalize2(edge) {
            axes.push(t);
        }
        return axes;
    }
    let mut axes = Vec::with_capacity(verts.len());
    for i in 0..verts.len() {
        let a = verts[i];
        let b = verts[(i + 1) % verts.len()];
        let edge = sub2(b, a);
        if let Some(n) = normalize2([-edge[1], edge[0]]) {
            axes.push(n);
        }
    }
    axes
}

fn polygon_polygon_contact(
    center_a: [f32; 2],
    verts_a: &[[f32; 2]],
    center_b: [f32; 2],
    verts_b: &[[f32; 2]],
) -> Option<RawContact2D> {
    let mut min_overlap = f32::INFINITY;
    let mut best_axis = [0.0, 1.0];
    let axes_a = candidate_axes(verts_a);
    let axes_b = candidate_axes(verts_b);
    for axis in axes_a.iter().chain(axes_b.iter()) {
        let (min_a, max_a) = project_polygon(verts_a, *axis);
        let (min_b, max_b) = project_polygon(verts_b, *axis);
        if max_a < min_b - EPSILON || max_b < min_a - EPSILON {
            return None;
        }
        let overlap = max_a.min(max_b) - min_a.max(min_b);
        if overlap < min_overlap {
            min_overlap = overlap.max(0.0);
            best_axis = *axis;
        }
    }
    let ab = sub2(center_b, center_a);
    if dot2(ab, best_axis) < 0.0 {
        best_axis = [-best_axis[0], -best_axis[1]];
    }
    let mut support_a = verts_a[0];
    let mut best_proj_a = dot2(support_a, best_axis);
    for v in verts_a.iter().skip(1) {
        let p = dot2(*v, best_axis);
        if p > best_proj_a {
            best_proj_a = p;
            support_a = *v;
        }
    }
    let mut support_b = verts_b[0];
    let mut best_proj_b = dot2(support_b, best_axis);
    for v in verts_b.iter().skip(1) {
        let p = dot2(*v, best_axis);
        if p < best_proj_b {
            best_proj_b = p;
            support_b = *v;
        }
    }
    let point = [
        0.5 * (support_a[0] + support_b[0]),
        0.5 * (support_a[1] + support_b[1]),
    ];
    Some(RawContact2D {
        point,
        normal: best_axis,
        depth: min_overlap,
    })
}

pub(crate) fn point_in_shape2d(
    point: [f32; 2],
    shape: &Shape2D,
    center: [f32; 2],
    angle: f32,
) -> bool {
    match shape {
        Shape2D::Circle { radius } => {
            let d = sub2(point, center);
            dot2(d, d) <= radius * radius
        }
        Shape2D::Box { half_extents } => {
            let local = inverse_transform_point2(point, center, angle);
            local[0].abs() <= half_extents[0] && local[1].abs() <= half_extents[1]
        }
        Shape2D::Edge { start, end } => {
            let a = transform_point2(*start, center, angle);
            let b = transform_point2(*end, center, angle);
            let cp = closest_point_on_segment2(a, b, point);
            length2(sub2(point, cp)) <= 1e-4
        }
        Shape2D::Polygon { .. } => {
            let Some(verts) = shape_vertices_world(shape, center, angle) else {
                return false;
            };
            point_inside_convex_polygon(point, &verts)
        }
        Shape2D::Composite { shapes } => shapes
            .iter()
            .any(|sub| point_in_shape2d(point, sub, center, angle)),
    }
}

pub(crate) fn raycast_shape2d(
    origin: [f32; 2],
    direction: [f32; 2],
    max_distance: f32,
    shape: &Shape2D,
    center: [f32; 2],
    angle: f32,
    queries_start_in_colliders: bool,
) -> Option<(f32, [f32; 2], [f32; 2])> {
    let dir = normalize2(direction)?;
    if max_distance < 0.0 || !max_distance.is_finite() {
        return None;
    }
    if let Shape2D::Composite { shapes } = shape {
        let mut best: Option<(f32, [f32; 2], [f32; 2])> = None;
        for sub in shapes {
            if let Some(hit) = raycast_shape2d(
                origin,
                dir,
                max_distance,
                sub,
                center,
                angle,
                queries_start_in_colliders,
            ) {
                if best.is_none_or(|(best_t, _, _)| hit.0 < best_t) {
                    best = Some(hit);
                }
            }
        }
        return best;
    }
    if point_in_shape2d(origin, shape, center, angle) {
        if !queries_start_in_colliders {
            return None;
        }
        return Some((0.0, origin, [-dir[0], -dir[1]]));
    }
    match shape {
        Shape2D::Circle { radius } => {
            let offset = sub2(origin, center);
            let half_b = dot2(offset, dir);
            let c = dot2(offset, offset) - radius * radius;
            let disc = half_b * half_b - c;
            if disc < 0.0 {
                return None;
            }
            let t = -half_b - disc.sqrt();
            if t < 0.0 || t > max_distance {
                return None;
            }
            let pt = add2(origin, scale2(dir, t));
            let normal = normalize2(sub2(pt, center)).unwrap_or([-dir[0], -dir[1]]);
            Some((t, pt, normal))
        }
        Shape2D::Box { half_extents } => {
            let local_orig = inverse_transform_point2(origin, center, angle);
            let local_dir = rotate_vec2(dir, -angle);
            let mut t_min = f32::NEG_INFINITY;
            let mut t_max = f32::INFINITY;
            let mut normal_local = [0.0, 0.0];
            for axis in 0..2 {
                let min_v = -half_extents[axis];
                let max_v = half_extents[axis];
                if local_dir[axis].abs() <= EPSILON {
                    if local_orig[axis] < min_v || local_orig[axis] > max_v {
                        return None;
                    }
                } else {
                    let t1 = (min_v - local_orig[axis]) / local_dir[axis];
                    let t2 = (max_v - local_orig[axis]) / local_dir[axis];
                    let (near, far, sign) = if t1 <= t2 {
                        (t1, t2, -1.0)
                    } else {
                        (t2, t1, 1.0)
                    };
                    if near > t_min {
                        t_min = near;
                        normal_local = [0.0, 0.0];
                        normal_local[axis] = sign;
                    }
                    t_max = t_max.min(far);
                }
            }
            if t_max < t_min || t_min < 0.0 || t_min > max_distance {
                return None;
            }
            let pt = add2(origin, scale2(dir, t_min));
            let world_normal = rotate_vec2(normal_local, angle);
            Some((t_min, pt, world_normal))
        }
        Shape2D::Edge { start, end } => {
            let a = transform_point2(*start, center, angle);
            let b = transform_point2(*end, center, angle);
            ray_segment_intersect(origin, dir, max_distance, a, b)
        }
        Shape2D::Polygon { .. } => {
            let verts = shape_vertices_world(shape, center, angle)?;
            let mut best: Option<(f32, [f32; 2], [f32; 2])> = None;
            for i in 0..verts.len() {
                let a = verts[i];
                let b = verts[(i + 1) % verts.len()];
                if let Some((t, pt, n)) = ray_segment_intersect(origin, dir, max_distance, a, b) {
                    if best.is_none_or(|(best_t, _, _)| t < best_t) {
                        best = Some((t, pt, n));
                    }
                }
            }
            best
        }
        Shape2D::Composite { .. } => None,
    }
}

fn ray_segment_intersect(
    origin: [f32; 2],
    dir: [f32; 2],
    max_distance: f32,
    a: [f32; 2],
    b: [f32; 2],
) -> Option<(f32, [f32; 2], [f32; 2])> {
    let v1 = sub2(origin, a);
    let v2 = sub2(b, a);
    let v3 = [-dir[1], dir[0]];
    let denom = dot2(v2, v3);
    if denom.abs() <= EPSILON {
        return None;
    }
    let t1 = (v2[0] * v1[1] - v2[1] * v1[0]) / denom;
    let t2 = dot2(v1, v3) / denom;
    if t1 >= 0.0 && t1 <= max_distance && (0.0..=1.0).contains(&t2) {
        let pt = add2(origin, scale2(dir, t1));
        let mut normal = normalize2([-v2[1], v2[0]]).unwrap_or([-dir[0], -dir[1]]);
        if dot2(normal, dir) > 0.0 {
            normal = [-normal[0], -normal[1]];
        }
        Some((t1, pt, normal))
    } else {
        None
    }
}
