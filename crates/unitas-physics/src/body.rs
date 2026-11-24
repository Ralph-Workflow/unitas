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
pub(crate) struct Contact {
    pub(crate) normal: [f64; 3],
    pub(crate) depth: f64,
}

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

    pub(crate) fn in_contact(
        &self,
        position: [f64; 3],
        rotation: [f64; 4],
        other: &Self,
        other_position: [f64; 3],
        other_rotation: [f64; 4],
    ) -> bool {
        const CONTACT_SLOP: f64 = 1e-8;
        match (self, other) {
            (Self::Sphere { radius }, Self::Sphere { radius: other_radius }) => {
                distance(position, other_position) < radius + other_radius
            }
            (Self::Sphere { radius }, Self::Box { half_extents }) => {
                let local = to_local(other_rotation, other_position, position);
                let closest = closest_point([0.0; 3], *half_extents, local);
                distance(local, closest) <= *radius + CONTACT_SLOP
            }
            (Self::Box { half_extents }, Self::Sphere { radius }) => {
                let local = to_local(rotation, position, other_position);
                let closest = closest_point([0.0; 3], *half_extents, local);
                distance(local, closest) <= *radius + CONTACT_SLOP
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

    pub(crate) fn contact(
        &self,
        position: [f64; 3],
        rotation: [f64; 4],
        other: &Self,
        other_position: [f64; 3],
        other_rotation: [f64; 4],
    ) -> Option<Contact> {
        match (self, other) {
            (Self::Sphere { radius }, Self::Sphere { radius: other_radius }) => {
                sphere_sphere_contact(position, *radius, other_position, *other_radius)
            }
            (Self::Sphere { radius }, Self::Box { half_extents }) => {
                sphere_box_contact(position, *radius, other_position, other_rotation, *half_extents)
            }
            (Self::Box { half_extents }, Self::Sphere { radius }) => sphere_box_contact(
                other_position,
                *radius,
                position,
                rotation,
                *half_extents,
            )
            .map(|contact| Contact {
                normal: [-contact.normal[0], -contact.normal[1], -contact.normal[2]],
                depth: contact.depth,
            }),
            (Self::Box { half_extents }, Self::Box { half_extents: other_half_extents }) => {
                box_box_contact(
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
    pub role: BodyRole,
}

impl BodyDesc {
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
    box_box_contact(
        position,
        rotation,
        half_extents,
        other_position,
        other_rotation,
        other_half_extents,
    )
    .is_some()
}

fn sphere_sphere_contact(
    position: [f64; 3],
    radius: f64,
    other_position: [f64; 3],
    other_radius: f64,
) -> Option<Contact> {
    let delta = subtract(other_position, position);
    let dist = vector_length(delta);
    let sum = radius + other_radius;
    if dist >= sum {
        return None;
    }
    let normal = if dist > OVERLAP_EPSILON {
        [delta[0] / dist, delta[1] / dist, delta[2] / dist]
    } else {
        [0.0, 1.0, 0.0]
    };
    Some(Contact {
        normal,
        depth: sum - dist,
    })
}

fn sphere_box_contact(
    sphere_position: [f64; 3],
    radius: f64,
    box_position: [f64; 3],
    box_rotation: [f64; 4],
    half_extents: [f64; 3],
) -> Option<Contact> {
    let local = to_local(box_rotation, box_position, sphere_position);
    let closest = closest_point([0.0; 3], half_extents, local);
    let delta = subtract(closest, local);
    let dist = vector_length(delta);
    if dist >= radius {
        return None;
    }
    if dist > OVERLAP_EPSILON {
        let local_normal = [delta[0] / dist, delta[1] / dist, delta[2] / dist];
        return Some(Contact {
            normal: rotate_vector(box_rotation, local_normal),
            depth: radius - dist,
        });
    }
    let mut best_axis = 0usize;
    let mut min_clearance = half_extents[0] - local[0].abs();
    for axis in 1..3 {
        let clearance = half_extents[axis] - local[axis].abs();
        if clearance < min_clearance {
            min_clearance = clearance;
            best_axis = axis;
        }
    }
    let mut local_normal = [0.0; 3];
    local_normal[best_axis] = if local[best_axis] >= 0.0 { -1.0 } else { 1.0 };
    Some(Contact {
        normal: rotate_vector(box_rotation, local_normal),
        depth: radius + min_clearance,
    })
}

fn box_box_contact(
    position: [f64; 3],
    rotation: [f64; 4],
    half_extents: [f64; 3],
    other_position: [f64; 3],
    other_rotation: [f64; 4],
    other_half_extents: [f64; 3],
) -> Option<Contact> {
    let axes = basis(rotation);
    let other_axes = basis(other_rotation);
    let offset = subtract(other_position, position);
    let reach = |axis: [f64; 3]| {
        projected_radius(axis, &axes, half_extents)
            + projected_radius(axis, &other_axes, other_half_extents)
    };
    let mut min_depth = f64::INFINITY;
    let mut best_normal = [0.0, 1.0, 0.0];
    for axis in axes.iter().chain(other_axes.iter()) {
        let span = reach(*axis);
        let proj = dot(offset, *axis);
        if proj.abs() >= span - OVERLAP_EPSILON {
            return None;
        }
        let depth = span - proj.abs();
        if depth < min_depth {
            min_depth = depth;
            best_normal = if proj >= 0.0 {
                *axis
            } else {
                [-axis[0], -axis[1], -axis[2]]
            };
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
            let span = reach(axis);
            let proj = dot(offset, axis);
            if proj.abs() >= span - OVERLAP_EPSILON {
                return None;
            }
            let depth = span - proj.abs();
            if depth < min_depth {
                min_depth = depth;
                best_normal = if proj >= 0.0 {
                    axis
                } else {
                    [-axis[0], -axis[1], -axis[2]]
                };
            }
        }
    }
    Some(Contact {
        normal: best_normal,
        depth: min_depth,
    })
}

fn basis(rotation: [f64; 4]) -> [[f64; 3]; 3] {
    [
        rotate_vector(rotation, [1.0, 0.0, 0.0]),
        rotate_vector(rotation, [0.0, 1.0, 0.0]),
        rotate_vector(rotation, [0.0, 0.0, 1.0]),
    ]
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
