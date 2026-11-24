use unitas_core::EntityId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycastHit {
    pub entity: EntityId,
    pub distance: f64,
    pub point: [f64; 3],
}

pub(crate) fn normalized_direction(direction: [f64; 3]) -> Option<[f64; 3]> {
    let length =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
            .sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([
        direction[0] / length,
        direction[1] / length,
        direction[2] / length,
    ])
}

pub(crate) fn ray_sphere_distance(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    radius: f64,
) -> Option<f64> {
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
    let root = discriminant.sqrt();
    let entry = -half_b - root;
    let exit = -half_b + root;
    if entry > 0.0 {
        Some(entry)
    } else if exit > 0.0 {
        Some(0.0)
    } else {
        None
    }
}

pub(crate) fn ray_box_distance(
    origin: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    half_extents: [f64; 3],
) -> Option<f64> {
    let mut entry = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
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
        entry = entry.max(first.min(second));
        exit = exit.min(first.max(second));
    }
    if exit <= entry {
        return None;
    }
    if entry > 0.0 {
        Some(entry)
    } else if exit > 0.0 {
        Some(0.0)
    } else {
        None
    }
}
