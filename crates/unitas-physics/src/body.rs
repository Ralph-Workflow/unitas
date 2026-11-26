pub(crate) const IDENTITY_ROTATION: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

const OVERLAP_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyRole {
    #[default]
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Sphere {
        radius: f64,
    },
    Box {
        half_extents: [f64; 3],
    },
    Capsule {
        radius: f64,
        height: f64,
        direction: u8,
    },
}

impl Shape {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Sphere { radius } => radius.is_finite() && *radius > 0.0,
            Self::Box { half_extents } => half_extents
                .iter()
                .all(|extent| extent.is_finite() && *extent > 0.0),
            Self::Capsule {
                radius,
                height,
                direction,
            } => {
                radius.is_finite()
                    && *radius > 0.0
                    && height.is_finite()
                    && *height >= 0.0
                    && *direction <= 2
            }
        }
    }

    pub(crate) fn bounding_radius(&self) -> f64 {
        match self {
            Self::Sphere { radius } => *radius,
            Self::Box { half_extents } => (half_extents[0] * half_extents[0]
                + half_extents[1] * half_extents[1]
                + half_extents[2] * half_extents[2])
                .sqrt(),
            Self::Capsule { radius, height, .. } => {
                capsule_half_segment(*radius, *height) + *radius
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColliderShape {
    Primitive(Shape),
    Mesh {
        vertices: Vec<[f64; 3]>,
        indices: Vec<u32>,
        convex: bool,
    },
}

impl ColliderShape {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Primitive(shape) => shape.is_valid(),
            Self::Mesh {
                vertices, indices, ..
            } => {
                vertices.len() >= 3
                    && vertices
                        .iter()
                        .all(|vertex| vertex.iter().all(|value| value.is_finite()))
                    && !indices.is_empty()
                    && indices.len() % 3 == 0
                    && indices.iter().all(|index| {
                        usize::try_from(*index).is_ok_and(|index| index < vertices.len())
                    })
            }
        }
    }

    pub(crate) fn is_non_convex_mesh(&self) -> bool {
        matches!(self, Self::Mesh { convex: false, .. })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionDetectionMode3D {
    #[default]
    Discrete,
    Continuous,
    ContinuousDynamic,
    ContinuousSpeculative,
}

impl CollisionDetectionMode3D {
    pub(crate) fn uses_ccd_solver(self) -> bool {
        matches!(self, Self::Continuous | Self::ContinuousDynamic)
    }

    pub(crate) fn uses_speculative_contacts(self) -> bool {
        matches!(self, Self::ContinuousSpeculative)
    }

    pub(crate) fn uses_dynamic_ccd(self) -> bool {
        matches!(self, Self::ContinuousDynamic)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyDesc {
    pub position: [f64; 3],
    pub rotation: [f64; 4],
    pub mass: f64,
    pub velocity: [f64; 3],
    pub use_gravity: bool,
    pub is_trigger: bool,
    pub collider_shape: ColliderShape,
    pub layer: u32,
    pub role: BodyRole,
    pub friction: f64,
    pub restitution: f64,
    pub collision_detection_mode: CollisionDetectionMode3D,
}

impl BodyDesc {
    pub fn with_collision_detection_mode(mut self, mode: CollisionDetectionMode3D) -> Self {
        self.collision_detection_mode = mode;
        self
    }

    pub fn effective_role(&self) -> BodyRole {
        match self.role {
            BodyRole::Dynamic if self.mass <= 0.0 => BodyRole::Static,
            role => role,
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.mass.is_finite()
            && self.position.iter().all(|value| value.is_finite())
            && self.velocity.iter().all(|value| value.is_finite())
            && self.friction.is_finite()
            && self.friction >= 0.0
            && self.restitution.is_finite()
            && self.restitution >= 0.0
            && self.collider_shape.is_valid()
    }
}

pub(crate) fn normalize_rotation(rotation: [f64; 4]) -> [f64; 4] {
    let unit = rotation_or_identity(rotation);
    let length = rotation_length(unit);
    [
        unit[0] / length,
        unit[1] / length,
        unit[2] / length,
        unit[3] / length,
    ]
}

pub(crate) fn multiply_rotation(first: [f64; 4], second: [f64; 4]) -> [f64; 4] {
    let a = rotation_or_identity(first);
    let b = rotation_or_identity(second);
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

pub(crate) fn rotate_vector(rotation: [f64; 4], vector: [f64; 3]) -> [f64; 3] {
    let unit = normalize_rotation(rotation);
    let rotated = multiply_rotation(
        multiply_rotation(unit, [vector[0], vector[1], vector[2], 0.0]),
        inverse_rotation(unit),
    );
    [rotated[0], rotated[1], rotated[2]]
}

pub(crate) fn inverse_rotation(rotation: [f64; 4]) -> [f64; 4] {
    let unit = normalize_rotation(rotation);
    [-unit[0], -unit[1], -unit[2], unit[3]]
}

pub(crate) fn to_local(rotation: [f64; 4], center: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    rotate_vector(inverse_rotation(rotation), subtract(point, center))
}

pub(crate) fn lerp_vector(first: [f64; 3], second: [f64; 3], alpha: f64) -> [f64; 3] {
    [
        first[0] + (second[0] - first[0]) * alpha,
        first[1] + (second[1] - first[1]) * alpha,
        first[2] + (second[2] - first[2]) * alpha,
    ]
}

pub(crate) fn slerp_rotation(first: [f64; 4], second: [f64; 4], alpha: f64) -> [f64; 4] {
    let start = normalize_rotation(first);
    let mut end = normalize_rotation(second);
    let mut cosine = start[0] * end[0] + start[1] * end[1] + start[2] * end[2] + start[3] * end[3];
    if cosine < 0.0 {
        cosine = -cosine;
        end = [-end[0], -end[1], -end[2], -end[3]];
    }
    if cosine > 0.9995 {
        return normalize_rotation([
            start[0] + (end[0] - start[0]) * alpha,
            start[1] + (end[1] - start[1]) * alpha,
            start[2] + (end[2] - start[2]) * alpha,
            start[3] + (end[3] - start[3]) * alpha,
        ]);
    }
    let angle = cosine.clamp(-1.0, 1.0).acos();
    let sine = angle.sin();
    let start_weight = ((1.0 - alpha) * angle).sin() / sine;
    let end_weight = (alpha * angle).sin() / sine;
    normalize_rotation([
        start[0] * start_weight + end[0] * end_weight,
        start[1] * start_weight + end[1] * end_weight,
        start[2] * start_weight + end[2] * end_weight,
        start[3] * start_weight + end[3] * end_weight,
    ])
}

pub(crate) fn add(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] + second[0],
        first[1] + second[1],
        first[2] + second[2],
    ]
}

pub(crate) fn subtract(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn rotation_or_identity(rotation: [f64; 4]) -> [f64; 4] {
    let length = rotation_length(rotation);
    if !length.is_finite() || length <= 0.0 {
        return IDENTITY_ROTATION;
    }
    rotation
}

fn rotation_length(rotation: [f64; 4]) -> f64 {
    (rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3])
        .sqrt()
}

pub(crate) fn dot(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

pub(crate) fn vector_length(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}

pub(crate) fn distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    let dx = first[0] - second[0];
    let dy = first[1] - second[1];
    let dz = first[2] - second[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(crate) fn capsule_local_axis(direction: u8) -> [f64; 3] {
    match direction {
        0 => [1.0, 0.0, 0.0],
        2 => [0.0, 0.0, 1.0],
        _ => [0.0, 1.0, 0.0],
    }
}

pub(crate) fn capsule_half_segment(radius: f64, height: f64) -> f64 {
    (0.5 * height - radius).max(0.0)
}

pub(crate) fn capsule_local_segment(
    radius: f64,
    height: f64,
    direction: u8,
) -> ([f64; 3], [f64; 3]) {
    let half = capsule_half_segment(radius, height);
    let axis = capsule_local_axis(direction);
    (
        [-axis[0] * half, -axis[1] * half, -axis[2] * half],
        [axis[0] * half, axis[1] * half, axis[2] * half],
    )
}

pub(crate) fn capsule_world_segment(
    position: [f64; 3],
    rotation: [f64; 4],
    radius: f64,
    height: f64,
    direction: u8,
) -> ([f64; 3], [f64; 3]) {
    let (local_a, local_b) = capsule_local_segment(radius, height, direction);
    (
        add(position, rotate_vector(rotation, local_a)),
        add(position, rotate_vector(rotation, local_b)),
    )
}

pub(crate) fn closest_point_on_segment(a: [f64; 3], b: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    let ab = subtract(b, a);
    let len_sq = dot(ab, ab);
    if len_sq <= OVERLAP_EPSILON * OVERLAP_EPSILON {
        return a;
    }
    let t = (dot(subtract(point, a), ab) / len_sq).clamp(0.0, 1.0);
    [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]
}
