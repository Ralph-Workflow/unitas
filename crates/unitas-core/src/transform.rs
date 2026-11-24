use crate::entity::EntityId;

pub const IDENTITY_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

const MIN_ROTATION_LENGTH: f64 = 1e-6;
const UNIT_ROTATION_TOLERANCE: f64 = 1e-3;

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub parent: Option<EntityId>,
    pub children: Vec<EntityId>,
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parent(&self) -> Option<EntityId> {
        self.parent
    }

    pub fn children(&self) -> &[EntityId] {
        &self.children
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: IDENTITY_ROTATION,
            scale: [1.0; 3],
            parent: None,
            children: Vec::new(),
        }
    }
}

pub fn normalize_rotation(rotation: [f32; 4]) -> [f32; 4] {
    if !is_usable_rotation(rotation) {
        return IDENTITY_ROTATION;
    }
    let length = rotation_length(rotation);
    [
        (f64::from(rotation[0]) / length) as f32,
        (f64::from(rotation[1]) / length) as f32,
        (f64::from(rotation[2]) / length) as f32,
        (f64::from(rotation[3]) / length) as f32,
    ]
}

pub fn multiply_rotation(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    if !is_usable_rotation(a) || !is_usable_rotation(b) {
        return IDENTITY_ROTATION;
    }
    let (ax, ay, az, aw) = (a[0], a[1], a[2], a[3]);
    let (bx, by, bz, bw) = (b[0], b[1], b[2], b[3]);
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub fn rotate_vector(rotation: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let rotation = unit_rotation(rotation);
    if !vector.iter().all(|value| value.is_finite()) {
        return vector;
    }
    let axis = [rotation[0], rotation[1], rotation[2]];
    let twice_cross = scale_by(2.0, cross_product(axis, vector));
    add_vector(
        vector,
        add_vector(
            scale_by(rotation[3], twice_cross),
            cross_product(axis, twice_cross),
        ),
    )
}

pub fn compose_world(parent_world: &Transform, local: &Transform) -> Transform {
    let parent_rotation = usable_rotation(parent_world.rotation);
    let local_rotation = usable_rotation(local.rotation);
    let scaled = scale_vector(parent_world.scale, local.translation);
    Transform {
        translation: add_vector(
            parent_world.translation,
            rotate_vector(parent_rotation, scaled),
        ),
        rotation: multiply_rotation(parent_rotation, local_rotation),
        scale: scale_vector(parent_world.scale, local.scale),
        parent: None,
        children: Vec::new(),
    }
}

pub(crate) fn usable_rotation(rotation: [f32; 4]) -> [f32; 4] {
    unit_rotation(rotation)
}

fn unit_rotation(rotation: [f32; 4]) -> [f32; 4] {
    if !is_usable_rotation(rotation) {
        return IDENTITY_ROTATION;
    }
    if (rotation_length(rotation) - 1.0).abs() <= UNIT_ROTATION_TOLERANCE {
        return rotation;
    }
    normalize_rotation(rotation)
}

fn is_usable_rotation(rotation: [f32; 4]) -> bool {
    rotation.iter().all(|value| value.is_finite())
        && rotation_length(rotation) >= MIN_ROTATION_LENGTH
}

fn rotation_length(rotation: [f32; 4]) -> f64 {
    rotation
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt()
}

fn scale_vector(scale: [f32; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        scale[0] * vector[0],
        scale[1] * vector[1],
        scale[2] * vector[2],
    ]
}

fn scale_by(factor: f32, vector: [f32; 3]) -> [f32; 3] {
    [factor * vector[0], factor * vector[1], factor * vector[2]]
}

fn add_vector(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
