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
        other: &Self,
        other_position: [f64; 3],
    ) -> bool {
        match (self, other) {
            (Self::Sphere { radius }, Self::Sphere { radius: other_radius }) => {
                distance(position, other_position) < radius + other_radius
            }
            (Self::Sphere { radius }, Self::Box { half_extents }) => {
                let closest = closest_point(other_position, *half_extents, position);
                distance(position, closest) < *radius
            }
            (Self::Box { half_extents }, Self::Sphere { radius }) => {
                let closest = closest_point(position, *half_extents, other_position);
                distance(other_position, closest) < *radius
            }
            (Self::Box { half_extents }, Self::Box { half_extents: other_half_extents }) => {
                let overlap_x = (position[0] - other_position[0]).abs()
                    < half_extents[0] + other_half_extents[0];
                let overlap_y = (position[1] - other_position[1]).abs()
                    < half_extents[1] + other_half_extents[1];
                let overlap_z = (position[2] - other_position[2]).abs()
                    < half_extents[2] + other_half_extents[2];
                overlap_x && overlap_y && overlap_z
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDesc {
    pub position: [f64; 3],
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

fn distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    let dx = first[0] - second[0];
    let dy = first[1] - second[1];
    let dz = first[2] - second[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn closest_point(center: [f64; 3], half_extents: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    [
        point[0].clamp(center[0] - half_extents[0], center[0] + half_extents[0]),
        point[1].clamp(center[1] - half_extents[1], center[1] + half_extents[1]),
        point[2].clamp(center[2] - half_extents[2], center[2] + half_extents[2]),
    ]
}
