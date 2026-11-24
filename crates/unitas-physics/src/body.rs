pub(crate) const IDENTITY_ROTATION: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

const OVERLAP_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Sphere { radius: f64 },
    Box { half_extents: [f64; 3] },
}

impl Shape {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Sphere { radius } => radius.is_finite() && *radius > 0.0,
            Self::Box { half_extents } => half_extents
                .iter()
                .all(|extent| extent.is_finite() && *extent > 0.0),
        }
    }

    pub(crate) fn overlaps(
        &self,
        position: [f64; 3],
        rotation: [f64; 4],
        other: &Self,
        other_position: [f64; 3],
        other_rotation: [f64; 4],
    ) -> bool {
        match (self, other) {
            (Self::Sphere { radius }, Self::Sphere { radius: other_radius }) => {
                distance(position, other_position) < radius + other_radius
            }
            (Self::Sphere { radius }, Self::Box { half_extents }) => {
                let local = to_local(other_rotation, other_position, position);
                let closest = closest_point([0.0; 3], *half_extents, local);
                distance(local, closest) < *radius
            }
            (Self::Box { half_extents }, Self::Sphere { radius }) => {
                let local = to_local(rotation, position, other_position);
                let closest = closest_point([0.0; 3], *half_extents, local);
                distance(local, closest) < *radius
            }
            (Self::Box { half_extents }, Self::Box { half_extents: other_half_extents }) => {
                boxes_overlap(
                    position,
                    rotation,
                    *half_extents,
                    other_position,
                    other_rotation,
                    *other_half_extents,
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDesc {
    pub position: [f64; 3],
    pub rotation: [f64; 4],
    pub mass: f64,
    pub velocity: [f64; 3],
    pub use_gravity: bool,
    pub is_trigger: bool,
    pub shape: Shape,
    pub layer: u32,
}

impl BodyDesc {
    pub(crate) fn is_valid(&self) -> bool {
        self.mass.is_finite()
            && self.position.iter().all(|value| value.is_finite())
            && self.velocity.iter().all(|value| value.is_finite())
            && self.shape.is_valid()
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

pub(crate) fn closest_point(center: [f64; 3], half_extents: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    [
        point[0].clamp(center[0] - half_extents[0], center[0] + half_extents[0]),
        point[1].clamp(center[1] - half_extents[1], center[1] + half_extents[1]),
        point[2].clamp(center[2] - half_extents[2], center[2] + half_extents[2]),
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

fn boxes_overlap(
    position: [f64; 3],
    rotation: [f64; 4],
    half_extents: [f64; 3],
    other_position: [f64; 3],
    other_rotation: [f64; 4],
    other_half_extents: [f64; 3],
) -> bool {
    let axes = basis(rotation);
    let other_axes = basis(other_rotation);
    let offset = subtract(other_position, position);
    let reach = |axis: [f64; 3]| {
        projected_radius(axis, &axes, half_extents)
            + projected_radius(axis, &other_axes, other_half_extents)
    };
    for axis in axes.iter().chain(other_axes.iter()) {
        if separates(*axis, offset, reach(*axis)) {
            return false;
        }
    }
    for first in &axes {
        for second in &other_axes {
            let cross = cross_product(*first, *second);
            let length = vector_length(cross);
            if length <= OVERLAP_EPSILON {
                continue;
            }
            let axis = [cross[0] / length, cross[1] / length, cross[2] / length];
            if separates(axis, offset, reach(axis)) {
                return false;
            }
        }
    }
    true
}

fn basis(rotation: [f64; 4]) -> [[f64; 3]; 3] {
    [
        rotate_vector(rotation, [1.0, 0.0, 0.0]),
        rotate_vector(rotation, [0.0, 1.0, 0.0]),
        rotate_vector(rotation, [0.0, 0.0, 1.0]),
    ]
}

fn separates(axis: [f64; 3], offset: [f64; 3], reach: f64) -> bool {
    dot(offset, axis).abs() >= reach - OVERLAP_EPSILON
}

fn projected_radius(axis: [f64; 3], axes: &[[f64; 3]; 3], half_extents: [f64; 3]) -> f64 {
    half_extents[0] * dot(axis, axes[0]).abs()
        + half_extents[1] * dot(axis, axes[1]).abs()
        + half_extents[2] * dot(axis, axes[2]).abs()
}

fn cross_product(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn vector_length(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}

fn distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    let dx = first[0] - second[0];
    let dy = first[1] - second[1];
    let dz = first[2] - second[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
