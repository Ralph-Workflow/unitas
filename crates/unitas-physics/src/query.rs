use unitas_core::EntityId;

use crate::body::{closest_point, Shape};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryHit {
    pub entity: EntityId,
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub distance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CastHit {
    pub(crate) distance: f64,
    pub(crate) point: [f64; 3],
    pub(crate) normal: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CastShape {
    Point,
    Sphere { radius: f64 },
    Box { half_extents: [f64; 3] },
}

pub(crate) fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length =
        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([
        vector[0] / length,
        vector[1] / length,
        vector[2] / length,
    ])
}

pub(crate) fn cast_against(
    origin: [f64; 3],
    direction: [f64; 3],
    position: [f64; 3],
    shape: &Shape,
    cast: CastShape,
) -> Option<CastHit> {
    match shape {
        Shape::Sphere { radius } => {
            cast_against_sphere(origin, direction, position, *radius, cast)
        }
        Shape::Box { half_extents } => {
            cast_against_box(origin, direction, position, *half_extents, cast)
        }
    }
}

pub(crate) fn cast_against_sphere(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    radius: f64,
    cast: CastShape,
) -> Option<CastHit> {
    match cast {
        CastShape::Point => {
            let (distance, normal) = ray_sphere_hit(origin, direction, center, radius)?;
            Some(CastHit {
                distance,
                point: travel(origin, direction, distance),
                normal,
            })
        }
        CastShape::Sphere { radius: cast_radius } => {
            let (distance, normal) =
                ray_sphere_hit(origin, direction, center, radius + cast_radius)?;
            Some(CastHit {
                distance,
                point: [
                    center[0] + normal[0] * radius,
                    center[1] + normal[1] * radius,
                    center[2] + normal[2] * radius,
                ],
                normal,
            })
        }
        CastShape::Box { half_extents } => {
            let expanded = [
                radius + half_extents[0],
                radius + half_extents[1],
                radius + half_extents[2],
            ];
            let (distance, _) = ray_box_hit(origin, direction, center, expanded)?;
            let contact = travel(origin, direction, distance);
            let normal = normalize([
                contact[0] - center[0],
                contact[1] - center[1],
                contact[2] - center[2],
            ])?;
            Some(CastHit {
                distance,
                point: [
                    center[0] + normal[0] * radius,
                    center[1] + normal[1] * radius,
                    center[2] + normal[2] * radius,
                ],
                normal,
            })
        }
    }
}

pub(crate) fn cast_against_box(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    half_extents: [f64; 3],
    cast: CastShape,
) -> Option<CastHit> {
    match cast {
        CastShape::Point => {
            let (distance, normal) = ray_box_hit(origin, direction, center, half_extents)?;
            Some(CastHit {
                distance,
                point: travel(origin, direction, distance),
                normal,
            })
        }
        CastShape::Sphere { radius } => {
            let expanded = [
                half_extents[0] + radius,
                half_extents[1] + radius,
                half_extents[2] + radius,
            ];
            let (distance, normal) = ray_box_hit(origin, direction, center, expanded)?;
            Some(CastHit {
                distance,
                point: closest_point(
                    center,
                    half_extents,
                    travel(origin, direction, distance),
                ),
                normal,
            })
        }
        CastShape::Box {
            half_extents: cast_half_extents,
        } => {
            let expanded = [
                half_extents[0] + cast_half_extents[0],
                half_extents[1] + cast_half_extents[1],
                half_extents[2] + cast_half_extents[2],
            ];
            let (distance, normal) = ray_box_hit(origin, direction, center, expanded)?;
            Some(CastHit {
                distance,
                point: closest_point(
                    center,
                    half_extents,
                    travel(origin, direction, distance),
                ),
                normal,
            })
        }
    }
}

pub(crate) fn ray_sphere_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let offset = [
        origin[0] - center[0],
        origin[1] - center[1],
        origin[2] - center[2],
    ];
    let half_b = offset[0] * direction[0] + offset[1] * direction[1] + offset[2] * direction[2];
    let c = offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2] - radius * radius;
    let discriminant = half_b * half_b - c;
    if discriminant <= 0.0 {
        return None;
    }
    let entry = -half_b - discriminant.sqrt();
    if entry <= 0.0 {
        return None;
    }
    let point = travel(origin, direction, entry);
    let normal = normalize([
        point[0] - center[0],
        point[1] - center[1],
        point[2] - center[2],
    ])?;
    Some((entry, normal))
}

pub(crate) fn ray_box_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    half_extents: [f64; 3],
) -> Option<(f64, [f64; 3])> {
    let mut entry = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut entry_axis = 0usize;
    let mut entry_sign = -1.0f64;
    for axis in 0..3 {
        let minimum = center[axis] - half_extents[axis];
        let maximum = center[axis] + half_extents[axis];
        let component = direction[axis];
        if component == 0.0 {
            if origin[axis] <= minimum || origin[axis] >= maximum {
                return None;
            }
            continue;
        }
        let first = (minimum - origin[axis]) / component;
        let second = (maximum - origin[axis]) / component;
        let (near, far, sign) = if first <= second {
            (first, second, -1.0)
        } else {
            (second, first, 1.0)
        };
        if near > entry {
            entry = near;
            entry_axis = axis;
            entry_sign = sign;
        }
        exit = exit.min(far);
    }
    if exit <= entry || entry <= 0.0 {
        return None;
    }
    let mut normal = [0.0; 3];
    normal[entry_axis] = entry_sign;
    Some((entry, normal))
}

fn travel(origin: [f64; 3], direction: [f64; 3], distance: f64) -> [f64; 3] {
    [
        origin[0] + direction[0] * distance,
        origin[1] + direction[1] * distance,
        origin[2] + direction[2] * distance,
    ]
}
