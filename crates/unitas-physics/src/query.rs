use unitas_core::EntityId;

use crate::body::{
    add, capsule_local_segment, closest_point, closest_point_on_segment,
    closest_point_segment_aabb, distance, dot, inverse_rotation, rotate_vector, subtract, Shape,
};

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
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}

pub(crate) fn cast_against(
    origin: [f64; 3],
    direction: [f64; 3],
    position: [f64; 3],
    rotation: [f64; 4],
    shape: &Shape,
    cast: CastShape,
) -> Option<CastHit> {
    match shape {
        Shape::Sphere { radius } => cast_against_sphere(origin, direction, position, *radius, cast),
        Shape::Box { half_extents } => {
            cast_against_box(origin, direction, position, rotation, *half_extents, cast)
        }
        Shape::Capsule {
            radius,
            height,
            direction: axis_direction,
        } => cast_against_capsule(
            origin,
            direction,
            position,
            rotation,
            (*radius, *height, *axis_direction),
            cast,
        ),
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
        CastShape::Sphere {
            radius: cast_radius,
        } => {
            let expanded = radius + cast_radius;
            if distance(origin, center) < expanded {
                let normal = normalize(subtract(origin, center)).unwrap_or([
                    -direction[0],
                    -direction[1],
                    -direction[2],
                ]);
                return Some(CastHit {
                    distance: 0.0,
                    point: [
                        center[0] + normal[0] * radius,
                        center[1] + normal[1] * radius,
                        center[2] + normal[2] * radius,
                    ],
                    normal,
                });
            }
            let (distance, normal) = ray_sphere_entry_hit(origin, direction, center, expanded)?;
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
            if point_strictly_inside_aabb(origin, center, expanded) {
                let normal = normalize(subtract(origin, center)).unwrap_or([
                    -direction[0],
                    -direction[1],
                    -direction[2],
                ]);
                return Some(CastHit {
                    distance: 0.0,
                    point: [
                        center[0] + normal[0] * radius,
                        center[1] + normal[1] * radius,
                        center[2] + normal[2] * radius,
                    ],
                    normal,
                });
            }
            let (distance, _) = ray_box_entry_hit(origin, direction, center, expanded)?;
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
    rotation: [f64; 4],
    half_extents: [f64; 3],
    cast: CastShape,
) -> Option<CastHit> {
    let inverse = inverse_rotation(rotation);
    let local_origin = rotate_vector(inverse, subtract(origin, center));
    let local_direction = normalize(rotate_vector(inverse, direction))?;
    match cast {
        CastShape::Point => {
            let (distance, local_normal) =
                ray_box_hit(local_origin, local_direction, [0.0; 3], half_extents)?;
            Some(CastHit {
                distance,
                point: travel(origin, direction, distance),
                normal: rotate_vector(rotation, local_normal),
            })
        }
        CastShape::Sphere { radius } => {
            let expanded = [
                half_extents[0] + radius,
                half_extents[1] + radius,
                half_extents[2] + radius,
            ];
            if point_strictly_inside_aabb(local_origin, [0.0; 3], expanded) {
                let closest = closest_point([0.0; 3], half_extents, local_origin);
                let local_normal = normalize(subtract(local_origin, closest))
                    .unwrap_or_else(|| aabb_outward_normal(local_origin, half_extents));
                return Some(CastHit {
                    distance: 0.0,
                    point: add(center, rotate_vector(rotation, closest)),
                    normal: rotate_vector(rotation, local_normal),
                });
            }
            let (distance, local_normal) =
                ray_box_entry_hit(local_origin, local_direction, [0.0; 3], expanded)?;
            Some(CastHit {
                distance,
                point: add(
                    center,
                    rotate_vector(
                        rotation,
                        closest_point(
                            [0.0; 3],
                            half_extents,
                            travel(local_origin, local_direction, distance),
                        ),
                    ),
                ),
                normal: rotate_vector(rotation, local_normal),
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
            if point_strictly_inside_aabb(local_origin, [0.0; 3], expanded) {
                let closest = closest_point([0.0; 3], half_extents, local_origin);
                let local_normal = aabb_outward_normal(local_origin, expanded);
                return Some(CastHit {
                    distance: 0.0,
                    point: add(center, rotate_vector(rotation, closest)),
                    normal: rotate_vector(rotation, local_normal),
                });
            }
            let (distance, local_normal) =
                ray_box_entry_hit(local_origin, local_direction, [0.0; 3], expanded)?;
            Some(CastHit {
                distance,
                point: add(
                    center,
                    rotate_vector(
                        rotation,
                        closest_point(
                            [0.0; 3],
                            half_extents,
                            travel(local_origin, local_direction, distance),
                        ),
                    ),
                ),
                normal: rotate_vector(rotation, local_normal),
            })
        }
    }
}

pub(crate) fn cast_against_capsule(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    rotation: [f64; 4],
    capsule: (f64, f64, u8),
    cast: CastShape,
) -> Option<CastHit> {
    let (radius, height, axis_direction) = capsule;
    let inverse = inverse_rotation(rotation);
    let local_origin = rotate_vector(inverse, subtract(origin, center));
    let local_direction = normalize(rotate_vector(inverse, direction))?;
    let (seg_a, seg_b) = capsule_local_segment(radius, height, axis_direction);
    match cast {
        CastShape::Point => {
            let (distance, local_normal) =
                ray_capsule_hit(local_origin, local_direction, seg_a, seg_b, radius)?;
            Some(CastHit {
                distance,
                point: travel(origin, direction, distance),
                normal: rotate_vector(rotation, local_normal),
            })
        }
        CastShape::Sphere {
            radius: cast_radius,
        } => {
            let expanded = radius + cast_radius;
            let closest_start = closest_point_on_segment(seg_a, seg_b, local_origin);
            if distance(local_origin, closest_start) < expanded {
                let local_normal = normalize(subtract(local_origin, closest_start)).unwrap_or([
                    -local_direction[0],
                    -local_direction[1],
                    -local_direction[2],
                ]);
                let local_point = [
                    closest_start[0] + local_normal[0] * radius,
                    closest_start[1] + local_normal[1] * radius,
                    closest_start[2] + local_normal[2] * radius,
                ];
                return Some(CastHit {
                    distance: 0.0,
                    point: add(center, rotate_vector(rotation, local_point)),
                    normal: rotate_vector(rotation, local_normal),
                });
            }
            let (distance, local_normal) =
                ray_capsule_entry_hit(local_origin, local_direction, seg_a, seg_b, expanded)?;
            let swept_center = travel(local_origin, local_direction, distance);
            let seg_pt = closest_point_on_segment(seg_a, seg_b, swept_center);
            let local_point = [
                seg_pt[0] + local_normal[0] * radius,
                seg_pt[1] + local_normal[1] * radius,
                seg_pt[2] + local_normal[2] * radius,
            ];
            Some(CastHit {
                distance,
                point: add(center, rotate_vector(rotation, local_point)),
                normal: rotate_vector(rotation, local_normal),
            })
        }
        CastShape::Box { half_extents } => {
            let local_seg_a = subtract(seg_a, local_origin);
            let local_seg_b = subtract(seg_b, local_origin);
            let (seg_pt_start, box_pt_start) =
                closest_point_segment_aabb(local_seg_a, local_seg_b, half_extents);
            if distance(seg_pt_start, box_pt_start) < radius {
                let seg_in_capsule = add(local_origin, seg_pt_start);
                let local_normal = normalize(subtract(box_pt_start, seg_pt_start)).unwrap_or([
                    -local_direction[0],
                    -local_direction[1],
                    -local_direction[2],
                ]);
                let local_point = [
                    seg_in_capsule[0] + local_normal[0] * radius,
                    seg_in_capsule[1] + local_normal[1] * radius,
                    seg_in_capsule[2] + local_normal[2] * radius,
                ];
                return Some(CastHit {
                    distance: 0.0,
                    point: add(center, rotate_vector(rotation, local_point)),
                    normal: rotate_vector(rotation, local_normal),
                });
            }
            let mut half_box = half_extents;
            let axis_index = match axis_direction {
                0 => 0usize,
                2 => 2usize,
                _ => 1usize,
            };
            let half_seg = seg_b[axis_index].abs();
            for item in &mut half_box {
                *item += radius;
            }
            half_box[axis_index] += half_seg;
            let (distance, _) =
                ray_box_entry_hit(local_origin, local_direction, [0.0; 3], half_box)?;
            let hit_center = travel(local_origin, local_direction, distance);
            let rel_a = subtract(seg_a, hit_center);
            let rel_b = subtract(seg_b, hit_center);
            let (seg_rel, box_rel) = closest_point_segment_aabb(rel_a, rel_b, half_extents);
            let seg_pt = add(hit_center, seg_rel);
            let local_normal = normalize(subtract(box_rel, seg_rel)).unwrap_or([
                -local_direction[0],
                -local_direction[1],
                -local_direction[2],
            ]);
            let local_point = [
                seg_pt[0] + local_normal[0] * radius,
                seg_pt[1] + local_normal[1] * radius,
                seg_pt[2] + local_normal[2] * radius,
            ];
            Some(CastHit {
                distance,
                point: add(center, rotate_vector(rotation, local_point)),
                normal: rotate_vector(rotation, local_normal),
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
    let offset = subtract(origin, center);
    let half_b = dot(offset, direction);
    let c = dot(offset, offset) - radius * radius;
    let discriminant = half_b * half_b - c;
    if discriminant <= 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    if c < 0.0 {
        let exit = -half_b + root;
        if exit <= 0.0 {
            return None;
        }
        let point = travel(origin, direction, exit);
        let outward = normalize(subtract(point, center))?;
        return Some((exit, [-outward[0], -outward[1], -outward[2]]));
    }
    let entry = -half_b - root;
    if entry <= 0.0 {
        return None;
    }
    let point = travel(origin, direction, entry);
    let normal = normalize(subtract(point, center))?;
    Some((entry, normal))
}

fn ray_sphere_entry_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let offset = subtract(origin, center);
    let half_b = dot(offset, direction);
    let c = dot(offset, offset) - radius * radius;
    let discriminant = half_b * half_b - c;
    if discriminant <= 0.0 {
        return None;
    }
    let entry = -half_b - discriminant.sqrt();
    if entry <= 0.0 {
        return None;
    }
    let point = travel(origin, direction, entry);
    let normal = normalize(subtract(point, center))?;
    Some((entry, normal))
}

pub(crate) fn ray_box_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    half_extents: [f64; 3],
) -> Option<(f64, [f64; 3])> {
    if point_strictly_inside_aabb(origin, center, half_extents) {
        let mut exit = f64::INFINITY;
        let mut exit_axis = 0usize;
        let mut exit_sign = -1.0f64;
        for axis in 0..3 {
            let minimum = center[axis] - half_extents[axis];
            let maximum = center[axis] + half_extents[axis];
            let component = direction[axis];
            if component == 0.0 {
                continue;
            }
            let first = (minimum - origin[axis]) / component;
            let second = (maximum - origin[axis]) / component;
            let (far, inward_sign) = if first <= second {
                (second, -1.0)
            } else {
                (first, 1.0)
            };
            if far < exit {
                exit = far;
                exit_axis = axis;
                exit_sign = inward_sign;
            }
        }
        if !exit.is_finite() || exit <= 0.0 {
            return None;
        }
        let mut normal = [0.0; 3];
        normal[exit_axis] = exit_sign;
        return Some((exit, normal));
    }
    ray_box_entry_hit(origin, direction, center, half_extents)
}

fn ray_box_entry_hit(
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

fn ray_capsule_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let closest_start = closest_point_on_segment(seg_a, seg_b, origin);
    if distance(origin, closest_start) < radius {
        return ray_capsule_exit_hit(origin, direction, seg_a, seg_b, radius);
    }
    ray_capsule_entry_hit(origin, direction, seg_a, seg_b, radius)
}

fn ray_capsule_entry_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let mut best: Option<(f64, [f64; 3])> = None;
    let mut consider = |candidate: Option<(f64, [f64; 3])>| {
        if let Some((dist, normal)) = candidate {
            if dist > 0.0 && best.is_none_or(|(best_dist, _)| dist < best_dist) {
                best = Some((dist, normal));
            }
        }
    };
    consider(ray_sphere_entry_hit(origin, direction, seg_a, radius));
    consider(ray_sphere_entry_hit(origin, direction, seg_b, radius));
    consider(ray_cylinder_segment_hit(
        origin, direction, seg_a, seg_b, radius, false,
    ));
    best
}

fn ray_capsule_exit_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let mut best: Option<(f64, [f64; 3])> = None;
    let mut consider = |candidate: Option<(f64, [f64; 3])>| {
        if let Some((dist, normal)) = candidate {
            if dist > 0.0 && best.is_none_or(|(best_dist, _)| dist > best_dist) {
                best = Some((dist, normal));
            }
        }
    };
    consider(ray_sphere_exit_candidate(origin, direction, seg_a, radius));
    consider(ray_sphere_exit_candidate(origin, direction, seg_b, radius));
    consider(ray_cylinder_segment_hit(
        origin, direction, seg_a, seg_b, radius, true,
    ));
    best
}

fn ray_sphere_exit_candidate(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    radius: f64,
) -> Option<(f64, [f64; 3])> {
    let offset = subtract(origin, center);
    let half_b = dot(offset, direction);
    let c = dot(offset, offset) - radius * radius;
    let discriminant = half_b * half_b - c;
    if discriminant <= 0.0 {
        return None;
    }
    let exit = -half_b + discriminant.sqrt();
    if exit <= 0.0 {
        return None;
    }
    let point = travel(origin, direction, exit);
    let outward = normalize(subtract(point, center))?;
    Some((exit, [-outward[0], -outward[1], -outward[2]]))
}

fn ray_cylinder_segment_hit(
    origin: [f64; 3],
    direction: [f64; 3],
    seg_a: [f64; 3],
    seg_b: [f64; 3],
    radius: f64,
    exit_root: bool,
) -> Option<(f64, [f64; 3])> {
    let ab = subtract(seg_b, seg_a);
    let len_sq = dot(ab, ab);
    if len_sq <= 1e-18 {
        return None;
    }
    let axis = normalize(ab)?;
    let half_len = len_sq.sqrt() * 0.5;
    let mid = [
        0.5 * (seg_a[0] + seg_b[0]),
        0.5 * (seg_a[1] + seg_b[1]),
        0.5 * (seg_a[2] + seg_b[2]),
    ];
    let rel_origin = subtract(origin, mid);
    let dir_axis = dot(direction, axis);
    let orig_axis = dot(rel_origin, axis);
    let d_perp = [
        direction[0] - axis[0] * dir_axis,
        direction[1] - axis[1] * dir_axis,
        direction[2] - axis[2] * dir_axis,
    ];
    let o_perp = [
        rel_origin[0] - axis[0] * orig_axis,
        rel_origin[1] - axis[1] * orig_axis,
        rel_origin[2] - axis[2] * orig_axis,
    ];
    let a = dot(d_perp, d_perp);
    if a <= 1e-18 {
        return None;
    }
    let half_b = dot(o_perp, d_perp);
    let c = dot(o_perp, o_perp) - radius * radius;
    let disc = half_b * half_b - a * c;
    if disc <= 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let t = if exit_root {
        (-half_b + root) / a
    } else {
        (-half_b - root) / a
    };
    if t <= 0.0 {
        return None;
    }
    let hit_axis = orig_axis + dir_axis * t;
    if hit_axis.abs() > half_len {
        return None;
    }
    let radial = [
        o_perp[0] + d_perp[0] * t,
        o_perp[1] + d_perp[1] * t,
        o_perp[2] + d_perp[2] * t,
    ];
    let outward = normalize(radial)?;
    let normal = if exit_root {
        [-outward[0], -outward[1], -outward[2]]
    } else {
        outward
    };
    Some((t, normal))
}

fn point_strictly_inside_aabb(point: [f64; 3], center: [f64; 3], half_extents: [f64; 3]) -> bool {
    (point[0] - center[0]).abs() < half_extents[0]
        && (point[1] - center[1]).abs() < half_extents[1]
        && (point[2] - center[2]).abs() < half_extents[2]
}

fn aabb_outward_normal(local_point: [f64; 3], half_extents: [f64; 3]) -> [f64; 3] {
    let mut best_axis = 0usize;
    let mut min_clearance = half_extents[0] - local_point[0].abs();
    for axis in 1..3 {
        let clearance = half_extents[axis] - local_point[axis].abs();
        if clearance < min_clearance {
            min_clearance = clearance;
            best_axis = axis;
        }
    }
    let mut normal = [0.0; 3];
    normal[best_axis] = if local_point[best_axis] >= 0.0 {
        1.0
    } else {
        -1.0
    };
    normal
}

fn travel(origin: [f64; 3], direction: [f64; 3], distance: f64) -> [f64; 3] {
    [
        origin[0] + direction[0] * distance,
        origin[1] + direction[1] * distance,
        origin[2] + direction[2] * distance,
    ]
}
