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

    pub(crate) fn overlaps(
        &self,
        position: [f64; 3],
        rotation: [f64; 4],
        other: &Self,
        other_position: [f64; 3],
        other_rotation: [f64; 4],
    ) -> bool {
        match (self, other) {
            (
                Self::Sphere { radius },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => distance(position, other_position) < radius + other_radius,
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
            (
                Self::Box { half_extents },
                Self::Box {
                    half_extents: other_half_extents,
                },
            ) => boxes_overlap(
                position,
                rotation,
                *half_extents,
                other_position,
                other_rotation,
                *other_half_extents,
            ),
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => {
                let (a, b) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let closest = closest_point_on_segment(a, b, other_position);
                distance(closest, other_position) < radius + other_radius
            }
            (
                Self::Sphere { radius },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a, b) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let closest = closest_point_on_segment(a, b, position);
                distance(position, closest) < radius + other_radius
            }
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Box { half_extents },
            ) => capsule_box_contact(
                position,
                rotation,
                (*radius, *height, *direction),
                other_position,
                other_rotation,
                *half_extents,
            )
            .is_some(),
            (
                Self::Box { half_extents },
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
            ) => capsule_box_contact(
                other_position,
                other_rotation,
                (*radius, *height, *direction),
                position,
                rotation,
                *half_extents,
            )
            .is_some(),
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a0, a1) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let (b0, b1) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let (pa, pb) = closest_points_on_segments(a0, a1, b0, b1);
                distance(pa, pb) < radius + other_radius
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
            (
                Self::Sphere { radius },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => distance(position, other_position) < radius + other_radius,
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
            (
                Self::Box { half_extents },
                Self::Box {
                    half_extents: other_half_extents,
                },
            ) => boxes_overlap(
                position,
                rotation,
                *half_extents,
                other_position,
                other_rotation,
                *other_half_extents,
            ),
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => {
                let (a, b) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let closest = closest_point_on_segment(a, b, other_position);
                distance(closest, other_position) < radius + other_radius
            }
            (
                Self::Sphere { radius },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a, b) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let closest = closest_point_on_segment(a, b, position);
                distance(position, closest) < radius + other_radius
            }
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Box { half_extents },
            ) => {
                let (a, b) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let local_a = to_local(other_rotation, other_position, a);
                let local_b = to_local(other_rotation, other_position, b);
                let (seg_pt, box_pt) = closest_point_segment_aabb(local_a, local_b, *half_extents);
                distance(seg_pt, box_pt) <= *radius + CONTACT_SLOP
            }
            (
                Self::Box { half_extents },
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
            ) => {
                let (a, b) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *radius,
                    *height,
                    *direction,
                );
                let local_a = to_local(rotation, position, a);
                let local_b = to_local(rotation, position, b);
                let (seg_pt, box_pt) = closest_point_segment_aabb(local_a, local_b, *half_extents);
                distance(seg_pt, box_pt) <= *radius + CONTACT_SLOP
            }
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a0, a1) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let (b0, b1) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let (pa, pb) = closest_points_on_segments(a0, a1, b0, b1);
                distance(pa, pb) < radius + other_radius
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
            (
                Self::Sphere { radius },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => sphere_sphere_contact(position, *radius, other_position, *other_radius),
            (Self::Sphere { radius }, Self::Box { half_extents }) => sphere_box_contact(
                position,
                *radius,
                other_position,
                other_rotation,
                *half_extents,
            ),
            (Self::Box { half_extents }, Self::Sphere { radius }) => {
                sphere_box_contact(other_position, *radius, position, rotation, *half_extents).map(
                    |contact| Contact {
                        normal: [-contact.normal[0], -contact.normal[1], -contact.normal[2]],
                        depth: contact.depth,
                    },
                )
            }
            (
                Self::Box { half_extents },
                Self::Box {
                    half_extents: other_half_extents,
                },
            ) => box_box_contact(
                position,
                rotation,
                *half_extents,
                other_position,
                other_rotation,
                *other_half_extents,
            ),
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Sphere {
                    radius: other_radius,
                },
            ) => {
                let (a, b) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let closest = closest_point_on_segment(a, b, other_position);
                sphere_sphere_contact(closest, *radius, other_position, *other_radius)
            }
            (
                Self::Sphere { radius },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a, b) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let closest = closest_point_on_segment(a, b, position);
                sphere_sphere_contact(position, *radius, closest, *other_radius)
            }
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Box { half_extents },
            ) => capsule_box_contact(
                position,
                rotation,
                (*radius, *height, *direction),
                other_position,
                other_rotation,
                *half_extents,
            ),
            (
                Self::Box { half_extents },
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
            ) => capsule_box_contact(
                other_position,
                other_rotation,
                (*radius, *height, *direction),
                position,
                rotation,
                *half_extents,
            )
            .map(|contact| Contact {
                normal: [-contact.normal[0], -contact.normal[1], -contact.normal[2]],
                depth: contact.depth,
            }),
            (
                Self::Capsule {
                    radius,
                    height,
                    direction,
                },
                Self::Capsule {
                    radius: other_radius,
                    height: other_height,
                    direction: other_direction,
                },
            ) => {
                let (a0, a1) =
                    capsule_world_segment(position, rotation, *radius, *height, *direction);
                let (b0, b1) = capsule_world_segment(
                    other_position,
                    other_rotation,
                    *other_radius,
                    *other_height,
                    *other_direction,
                );
                let (pa, pb) = closest_points_on_segments(a0, a1, b0, b1);
                sphere_sphere_contact(pa, *radius, pb, *other_radius)
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
    pub friction: f64,
    pub restitution: f64,
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
            && self.friction.is_finite()
            && self.friction >= 0.0
            && self.restitution.is_finite()
            && self.restitution >= 0.0
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

pub(crate) fn closest_points_on_segments(
    p1: [f64; 3],
    q1: [f64; 3],
    p2: [f64; 3],
    q2: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let d1 = subtract(q1, p1);
    let d2 = subtract(q2, p2);
    let r = subtract(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let eps = OVERLAP_EPSILON * OVERLAP_EPSILON;
    let (s, t) = if a <= eps && e <= eps {
        (0.0, 0.0)
    } else if a <= eps {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= eps {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s = if denom != 0.0 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (
        [p1[0] + d1[0] * s, p1[1] + d1[1] * s, p1[2] + d1[2] * s],
        [p2[0] + d2[0] * t, p2[1] + d2[1] * t, p2[2] + d2[2] * t],
    )
}

pub(crate) fn closest_point_segment_aabb(
    a: [f64; 3],
    b: [f64; 3],
    half_extents: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let ab = subtract(b, a);
    let mut candidates = Vec::with_capacity(8);
    candidates.push(0.0);
    candidates.push(1.0);
    for axis in 0..3 {
        if ab[axis].abs() > OVERLAP_EPSILON {
            let t1 = (-half_extents[axis] - a[axis]) / ab[axis];
            let t2 = (half_extents[axis] - a[axis]) / ab[axis];
            if (0.0..=1.0).contains(&t1) {
                candidates.push(t1);
            }
            if (0.0..=1.0).contains(&t2) {
                candidates.push(t2);
            }
        }
    }
    candidates.sort_by(|x, y| x.total_cmp(y));

    let evaluate = |t: f64| -> ([f64; 3], [f64; 3], f64) {
        let pt = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
        let clamped = closest_point([0.0; 3], half_extents, pt);
        let diff = subtract(pt, clamped);
        (pt, clamped, dot(diff, diff))
    };

    let mut best_t = 0.0;
    let (mut best_seg, mut best_box, mut best_dist_sq) = evaluate(0.0);
    let eps_sq = OVERLAP_EPSILON * OVERLAP_EPSILON;
    let ab_len_sq = dot(ab, ab);
    let target_t = if ab_len_sq > eps_sq {
        (-dot(a, ab) / ab_len_sq).clamp(0.0, 1.0)
    } else {
        0.5
    };

    let mut consider = |t: f64| {
        let (seg_pt, box_pt, dist_sq) = evaluate(t);
        if dist_sq + eps_sq < best_dist_sq
            || ((dist_sq - best_dist_sq).abs() <= eps_sq
                && (t - target_t).abs() < (best_t - target_t).abs())
        {
            best_t = t;
            best_dist_sq = dist_sq;
            best_seg = seg_pt;
            best_box = box_pt;
        }
    };

    for index in 0..candidates.len() {
        let t = candidates[index];
        consider(t);
        if index + 1 < candidates.len() {
            let t_next = candidates[index + 1];
            if t_next - t > OVERLAP_EPSILON {
                let mid = 0.5 * (t + t_next);
                consider(mid);
                if (t..=t_next).contains(&target_t) {
                    consider(target_t);
                }
                let mid_pt = [a[0] + ab[0] * mid, a[1] + ab[1] * mid, a[2] + ab[2] * mid];
                let mut num = 0.0;
                let mut den = 0.0;
                for axis in 0..3 {
                    if mid_pt[axis] < -half_extents[axis] {
                        num += (-half_extents[axis] - a[axis]) * ab[axis];
                        den += ab[axis] * ab[axis];
                    } else if mid_pt[axis] > half_extents[axis] {
                        num += (half_extents[axis] - a[axis]) * ab[axis];
                        den += ab[axis] * ab[axis];
                    }
                }
                if den > eps_sq {
                    consider((num / den).clamp(t, t_next));
                }
            }
        }
    }
    (best_seg, best_box)
}

fn capsule_box_contact(
    capsule_position: [f64; 3],
    capsule_rotation: [f64; 4],
    capsule: (f64, f64, u8),
    box_position: [f64; 3],
    box_rotation: [f64; 4],
    half_extents: [f64; 3],
) -> Option<Contact> {
    let (radius, height, direction) = capsule;
    let (world_a, world_b) = capsule_world_segment(
        capsule_position,
        capsule_rotation,
        radius,
        height,
        direction,
    );
    let local_a = to_local(box_rotation, box_position, world_a);
    let local_b = to_local(box_rotation, box_position, world_b);
    let (seg_pt, _) = closest_point_segment_aabb(local_a, local_b, half_extents);
    let world_seg_pt = add(box_position, rotate_vector(box_rotation, seg_pt));
    sphere_box_contact(
        world_seg_pt,
        radius,
        box_position,
        box_rotation,
        half_extents,
    )
}
