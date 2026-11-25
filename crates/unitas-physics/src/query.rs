use rapier3d::geometry::{ContactData, ContactManifoldData, Ray};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::{PersistentQueryDispatcher, ShapeCastOptions};
use rapier3d::parry::shape::{Ball, Capsule, Cuboid};
use unitas_core::EntityId;

use crate::body::{capsule_half_segment, Shape, IDENTITY_ROTATION};

pub(crate) type Dispatcher<'a> =
    &'a dyn PersistentQueryDispatcher<ContactManifoldData, ContactData>;

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

pub(crate) enum ProbeShape {
    Ball(Ball),
    Cuboid(Cuboid),
    Capsule(Capsule),
}

impl ProbeShape {
    fn as_shape(&self) -> &dyn rapier3d::parry::shape::Shape {
        match self {
            Self::Ball(shape) => shape,
            Self::Cuboid(shape) => shape,
            Self::Capsule(shape) => shape,
        }
    }
}

pub(crate) fn probe_shape(shape: &Shape) -> ProbeShape {
    match shape {
        Shape::Sphere { radius } => ProbeShape::Ball(Ball::new(*radius as f32)),
        Shape::Box { half_extents } => ProbeShape::Cuboid(Cuboid::new(Vector::new(
            half_extents[0] as f32,
            half_extents[1] as f32,
            half_extents[2] as f32,
        ))),
        Shape::Capsule {
            radius,
            height,
            direction,
        } => {
            let half_segment = capsule_half_segment(*radius, *height) as f32;
            let radius = *radius as f32;
            ProbeShape::Capsule(match direction {
                0 => Capsule::new_x(half_segment, radius),
                2 => Capsule::new_z(half_segment, radius),
                _ => Capsule::new_y(half_segment, radius),
            })
        }
    }
}

pub(crate) fn cast_probe_shape(cast: CastShape) -> Option<ProbeShape> {
    match cast {
        CastShape::Point => None,
        CastShape::Sphere { radius } => Some(ProbeShape::Ball(Ball::new(radius as f32))),
        CastShape::Box { half_extents } => Some(ProbeShape::Cuboid(Cuboid::new(Vector::new(
            half_extents[0] as f32,
            half_extents[1] as f32,
            half_extents[2] as f32,
        )))),
    }
}

pub(crate) fn unit_rotation(rotation: [f64; 4]) -> Rotation {
    let quaternion = Rotation::from_xyzw(
        rotation[0] as f32,
        rotation[1] as f32,
        rotation[2] as f32,
        rotation[3] as f32,
    );
    if quaternion.is_finite() && quaternion.length_squared() > 0.0 {
        quaternion.normalize()
    } else {
        Rotation::IDENTITY
    }
}

pub(crate) fn pose_of(position: [f64; 3], rotation: [f64; 4]) -> Pose {
    Pose::from_parts(
        Vector::new(position[0] as f32, position[1] as f32, position[2] as f32),
        unit_rotation(rotation),
    )
}

pub(crate) fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}

pub(crate) fn cast_ray_against(
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    position: [f64; 3],
    rotation: [f64; 4],
    shape: &Shape,
) -> Option<CastHit> {
    let ray = Ray::new(
        Vector::new(origin[0] as f32, origin[1] as f32, origin[2] as f32),
        Vector::new(
            direction[0] as f32,
            direction[1] as f32,
            direction[2] as f32,
        ),
    );
    let pose = pose_of(position, rotation);
    let probe = probe_shape(shape);
    let intersection =
        probe
            .as_shape()
            .cast_ray_and_get_normal(&pose, &ray, max_distance as f32, false)?;
    let point = ray.point_at(intersection.time_of_impact);
    Some(CastHit {
        distance: f64::from(intersection.time_of_impact),
        point: [f64::from(point.x), f64::from(point.y), f64::from(point.z)],
        normal: [
            f64::from(intersection.normal.x),
            f64::from(intersection.normal.y),
            f64::from(intersection.normal.z),
        ],
    })
}

pub(crate) fn cast_shape_against(
    dispatcher: Dispatcher<'_>,
    cast: CastShape,
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    target: ([f64; 3], [f64; 4], &Shape),
) -> Option<CastHit> {
    let (position, rotation, shape) = target;
    let probe = cast_probe_shape(cast)?;
    let target_pose = pose_of(position, rotation);
    let cast_pose = pose_of(origin, IDENTITY_ROTATION);
    let pos12 = target_pose.inv_mul(&cast_pose);
    let velocity = target_pose.rotation.inverse()
        * Vector::new(
            direction[0] as f32,
            direction[1] as f32,
            direction[2] as f32,
        );
    let target = probe_shape(shape);
    let hit = dispatcher
        .cast_shapes(
            &pos12,
            velocity,
            target.as_shape(),
            probe.as_shape(),
            ShapeCastOptions {
                max_time_of_impact: max_distance as f32,
                target_distance: 0.0,
                stop_at_penetration: true,
                compute_impact_geometry_on_penetration: true,
            },
        )
        .ok()??;
    let point = target_pose.transform_point(hit.witness1);
    let normal = target_pose.rotation * hit.normal1;
    Some(CastHit {
        distance: f64::from(hit.time_of_impact),
        point: [f64::from(point.x), f64::from(point.y), f64::from(point.z)],
        normal: [
            f64::from(normal.x),
            f64::from(normal.y),
            f64::from(normal.z),
        ],
    })
}

pub(crate) fn overlaps_at(
    dispatcher: Dispatcher<'_>,
    probe: &ProbeShape,
    center: [f64; 3],
    position: [f64; 3],
    rotation: [f64; 4],
    shape: &Shape,
) -> bool {
    let probe_pose = pose_of(center, IDENTITY_ROTATION);
    let target_pose = pose_of(position, rotation);
    let pos12 = probe_pose.inv_mul(&target_pose);
    let target = probe_shape(shape);
    dispatcher
        .intersection_test(&pos12, probe.as_shape(), target.as_shape())
        .is_ok_and(|intersection| intersection.intersecting)
}
