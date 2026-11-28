use crate::math::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_positions(positions: &[Vec3]) -> Option<Self> {
        let mut rest = positions.iter();
        let first = *rest.next()?;
        let mut min = first;
        let mut max = first;
        for position in rest {
            min = Vec3::new(
                min.x.min(position.x),
                min.y.min(position.y),
                min.z.min(position.z),
            );
            max = Vec3::new(
                max.x.max(position.x),
                max.y.max(position.y),
                max.z.max(position.z),
            );
        }
        Some(Self { min, max })
    }

    pub fn transform(&self, model: &Mat4) -> Self {
        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for corner in 0..8 {
            let point = Vec3::new(
                if corner & 1 == 0 {
                    self.min.x
                } else {
                    self.max.x
                },
                if corner & 2 == 0 {
                    self.min.y
                } else {
                    self.max.y
                },
                if corner & 4 == 0 {
                    self.min.z
                } else {
                    self.max.z
                },
            );
            let world = model.transform_point(point).truncate();
            min = Vec3::new(min.x.min(world.x), min.y.min(world.y), min.z.min(world.z));
            max = Vec3::new(max.x.max(world.x), max.y.max(world.y), max.z.max(world.z));
        }
        Self { min, max }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Plane {
    normal: Vec3,
    offset: f32,
}

impl Plane {
    fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_view_projection(view_proj: &Mat4) -> Self {
        let row = |r: usize| {
            [
                view_proj.get(0, r),
                view_proj.get(1, r),
                view_proj.get(2, r),
                view_proj.get(3, r),
            ]
        };
        let combine = |a: [f32; 4], b: [f32; 4], sign: f32| {
            [
                a[0] + sign * b[0],
                a[1] + sign * b[1],
                a[2] + sign * b[2],
                a[3] + sign * b[3],
            ]
        };
        let r0 = row(0);
        let r1 = row(1);
        let r2 = row(2);
        let r3 = row(3);
        let plane = |coeffs: [f32; 4]| Plane {
            normal: Vec3::new(coeffs[0], coeffs[1], coeffs[2]),
            offset: coeffs[3],
        };
        Self {
            planes: [
                plane(combine(r3, r0, 1.0)),
                plane(combine(r3, r0, -1.0)),
                plane(combine(r3, r1, 1.0)),
                plane(combine(r3, r1, -1.0)),
                plane(r2),
                plane(combine(r3, r2, -1.0)),
            ],
        }
    }

    pub fn intersects(&self, bounds: &Aabb) -> bool {
        self.planes.iter().all(|plane| {
            let positive = Vec3::new(
                if plane.normal.x >= 0.0 {
                    bounds.max.x
                } else {
                    bounds.min.x
                },
                if plane.normal.y >= 0.0 {
                    bounds.max.y
                } else {
                    bounds.min.y
                },
                if plane.normal.z >= 0.0 {
                    bounds.max.z
                } else {
                    bounds.min.z
                },
            );
            plane.signed_distance(positive) >= 0.0
        })
    }
}

pub fn visible(bounds: Option<&Aabb>, model: &Mat4, frustum: &Frustum) -> bool {
    match bounds {
        Some(bounds) => frustum.intersects(&bounds.transform(model)),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{look_at, Camera};
    use crate::math::Vec3;

    fn identity_frustum() -> Frustum {
        Frustum::from_view_projection(&Mat4::identity())
    }

    fn cube_bounds(half: f32, center: Vec3) -> Aabb {
        Aabb {
            min: center + Vec3::new(-half, -half, -half),
            max: center + Vec3::new(half, half, half),
        }
    }

    fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        Mat4::from_cols_array([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
        ])
    }

    #[test]
    fn from_positions_takes_componentwise_extremes() {
        let positions = [
            Vec3::new(1.0, -2.0, 3.0),
            Vec3::new(-4.0, 5.0, -6.0),
            Vec3::new(0.5, 0.25, -0.75),
        ];
        let bounds = Aabb::from_positions(&positions);
        assert_eq!(
            bounds,
            Some(Aabb {
                min: Vec3::new(-4.0, -2.0, -6.0),
                max: Vec3::new(1.0, 5.0, 3.0),
            })
        );
    }

    #[test]
    fn from_positions_is_none_without_vertices() {
        assert_eq!(Aabb::from_positions(&[]), None);
    }

    #[test]
    fn transform_applies_translation_to_extremes() {
        let bounds = Aabb {
            min: Vec3::new(-1.0, 0.0, 2.0),
            max: Vec3::new(1.0, 3.0, 4.0),
        };
        let moved = bounds.transform(&translation(10.0, -2.0, 0.5));
        assert_eq!(
            moved,
            Aabb {
                min: Vec3::new(9.0, -2.0, 2.5),
                max: Vec3::new(11.0, 1.0, 4.5),
            }
        );
    }

    #[test]
    fn bounds_inside_unit_cube_ndc_are_visible() {
        let frustum = identity_frustum();
        let bounds = Aabb {
            min: Vec3::new(-0.5, -0.5, 0.25),
            max: Vec3::new(0.5, 0.5, 0.75),
        };
        assert!(frustum.intersects(&bounds));
    }

    #[test]
    fn bounds_beyond_right_plane_are_outside() {
        let frustum = identity_frustum();
        let bounds = Aabb {
            min: Vec3::new(3.0, -0.5, 0.25),
            max: Vec3::new(4.0, 0.5, 0.75),
        };
        assert!(!frustum.intersects(&bounds));
    }

    #[test]
    fn bounds_beyond_near_plane_are_outside() {
        let frustum = identity_frustum();
        let bounds = Aabb {
            min: Vec3::new(-0.5, -0.5, -2.0),
            max: Vec3::new(0.5, 0.5, -1.0),
        };
        assert!(!frustum.intersects(&bounds));
    }

    #[test]
    fn bounds_crossing_a_plane_intersect() {
        let frustum = identity_frustum();
        let bounds = Aabb {
            min: Vec3::new(0.5, -0.5, 0.25),
            max: Vec3::new(2.0, 0.5, 0.75),
        };
        assert!(frustum.intersects(&bounds));
    }

    #[test]
    fn perspective_frustum_keeps_center_and_rejects_behind_camera() {
        let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
        let view = look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let frustum = Frustum::from_view_projection(&(camera.projection_matrix() * view));
        let half_height_at_distance = 5.0 * (30.0f32.to_radians().tan());
        assert!(half_height_at_distance > 2.8 && half_height_at_distance < 2.9);
        let on_screen = cube_bounds(0.5, Vec3::new(0.0, 0.0, 0.0));
        assert!(frustum.intersects(&on_screen));
        let behind = cube_bounds(0.5, Vec3::new(0.0, 0.0, 10.0));
        assert!(!frustum.intersects(&behind));
    }

    #[test]
    fn bounds_less_instances_are_always_visible() {
        let frustum = identity_frustum();
        let model = translation(1.0e6, 0.0, 0.0);
        assert!(visible(None, &model, &frustum));
    }

    #[test]
    fn prepared_visibility_excludes_offscreen_and_retains_onscreen() {
        let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
        let view = look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let frustum = Frustum::from_view_projection(&(camera.projection_matrix() * view));
        let bounds = Some(cube_bounds(0.5, Vec3::ZERO));
        let on_screen = visible(bounds.as_ref(), &Mat4::identity(), &frustum);
        let off_screen = visible(bounds.as_ref(), &translation(0.0, 0.0, 1.0e6), &frustum);
        assert!(on_screen);
        assert!(!off_screen);
    }
}
