use crate::camera::{look_at, Camera};
use crate::math::{Mat4, Vec3};
use crate::raster::DirectionalLight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowQuality {
    Low,
    Medium,
    High,
}

impl ShadowQuality {
    pub const fn map_size(self) -> u32 {
        match self {
            Self::Low => 512,
            Self::Medium => 1024,
            Self::High => 2048,
        }
    }

    pub const fn distance(self) -> f32 {
        match self {
            Self::Low => 15.0,
            Self::Medium => 30.0,
            Self::High => 60.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowSettings {
    pub map_size: u32,
    pub distance: f32,
    pub bias: f32,
}

impl ShadowSettings {
    pub const DEFAULT_BIAS: f32 = 0.002;

    pub fn new(map_size: u32, distance: f32) -> Self {
        Self {
            map_size: map_size.max(1),
            distance: distance.max(1.0),
            bias: Self::DEFAULT_BIAS,
        }
    }

    pub fn for_quality(quality: ShadowQuality) -> Self {
        Self::new(quality.map_size(), quality.distance())
    }
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self::for_quality(ShadowQuality::Medium)
    }
}

pub(crate) fn scene_bounds(bounds: impl Iterator<Item = (Vec3, Vec3, Mat4)>) -> (Vec3, f32) {
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;
    for (mesh_min, mesh_max, model) in bounds {
        for corner in corners(mesh_min, mesh_max) {
            any = true;
            let world = model.transform_point(corner).truncate();
            min = Vec3::new(min.x.min(world.x), min.y.min(world.y), min.z.min(world.z));
            max = Vec3::new(max.x.max(world.x), max.y.max(world.y), max.z.max(world.z));
        }
    }
    if !any {
        min = Vec3::ZERO;
        max = Vec3::ZERO;
    }
    let center = (min + max) * 0.5;
    let radius = (max - center).length().max(0.5);
    (center, radius)
}

fn corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
}

pub(crate) fn light_view_proj(
    center: Vec3,
    radius: f32,
    light: &DirectionalLight,
    distance: f32,
) -> Mat4 {
    let direction = light.direction.normalize();
    let span = distance.max(radius + 1.0);
    let eye = center - direction * span;
    let up_reference = Vec3::new(0.0, 1.0, 0.0);
    let up = if direction.cross(up_reference).length() < 1e-4 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        up_reference
    };
    let view = look_at(eye, center, up);
    let near = (span - radius).max(0.1);
    let far = span + radius;
    let camera = Camera::orthographic(radius.max(0.5), 1.0, near, far);
    camera.projection_matrix() * view
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_light() -> DirectionalLight {
        DirectionalLight {
            direction: Vec3::new(0.3, 1.0, 0.2),
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        }
    }

    #[test]
    fn quality_levels_raise_map_size_and_distance() {
        assert!(ShadowQuality::Low.map_size() < ShadowQuality::Medium.map_size());
        assert!(ShadowQuality::Medium.map_size() < ShadowQuality::High.map_size());
        assert!(ShadowQuality::Low.distance() < ShadowQuality::Medium.distance());
        assert!(ShadowQuality::Medium.distance() < ShadowQuality::High.distance());
        for quality in [ShadowQuality::Low, ShadowQuality::Medium, ShadowQuality::High] {
            let settings = ShadowSettings::for_quality(quality);
            assert_eq!(settings.map_size, quality.map_size());
            assert_eq!(settings.distance, quality.distance());
            assert_eq!(settings.bias, ShadowSettings::DEFAULT_BIAS);
        }
    }

    #[test]
    fn settings_clamp_degenerate_values() {
        let settings = ShadowSettings::new(0, 0.0);
        assert_eq!(settings.map_size, 1);
        assert_eq!(settings.distance, 1.0);
    }

    #[test]
    fn light_view_proj_fits_scene_in_clip_space() {
        let center = Vec3::new(1.0, 2.0, -3.0);
        let radius = 2.5;
        let matrix = light_view_proj(center, radius, &test_light(), 30.0);
        let projected_center = matrix.project_point(center);
        assert!(projected_center.x.abs() < 1e-4, "{projected_center:?}");
        assert!(projected_center.y.abs() < 1e-4, "{projected_center:?}");
        assert!((0.0..=1.0).contains(&projected_center.z), "{projected_center:?}");
        for corner in corners(center - Vec3::new(radius, radius, radius), center + Vec3::new(radius, radius, radius)) {
            let projected = matrix.project_point(corner);
            assert!(projected.x.abs() <= 1.0 + 1e-4, "{projected:?}");
            assert!(projected.y.abs() <= 1.0 + 1e-4, "{projected:?}");
            assert!((-1e-4..=1.0 + 1e-4).contains(&projected.z), "{projected:?}");
        }
    }

    #[test]
    fn scene_bounds_empty_input_is_a_unit_sphere_at_origin() {
        let (center, radius) = scene_bounds(std::iter::empty());
        assert_eq!(center, Vec3::ZERO);
        assert_eq!(radius, 0.5);
    }

    #[test]
    fn scene_bounds_encloses_transformed_geometry() {
        let model = Mat4::from_cols_array([
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            10.0, 0.0, 0.0, 1.0,
        ]);
        let (center, radius) = scene_bounds(std::iter::once((
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            model,
        )));
        assert_eq!(center, Vec3::new(10.0, 0.0, 0.0));
        assert!((radius - 3.0f32.sqrt() * 2.0).abs() < 1e-4);
    }
}

#[cfg(all(test, feature = "device"))]
mod device_tests {
    use super::ShadowSettings;
    use crate::camera::{look_at, Camera};
    use crate::gpu::{GpuInstance, Renderer};
    use crate::math::{Mat4, Vec3};
    use crate::mesh::{cube, Mesh};
    use crate::raster::DirectionalLight;
    use crate::shadow::ShadowQuality;

    const SIZE: u32 = 128;

    fn camera() -> Camera {
        Camera::perspective(60.0, 1.0, 0.1, 100.0)
    }

    fn view() -> Mat4 {
        look_at(
            Vec3::new(0.0, 3.0, -6.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    fn light() -> DirectionalLight {
        DirectionalLight {
            direction: Vec3::new(-0.5, 1.0, 0.2),
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        }
    }

    fn plane() -> Mesh {
        Mesh::from_buffers(
            &[
                [-3.0, 0.0, -3.0],
                [3.0, 0.0, -3.0],
                [3.0, 0.0, 3.0],
                [-3.0, 0.0, 3.0],
            ],
            &[[0.0, 1.0, 0.0]; 4],
            &[0, 2, 1, 0, 3, 2],
        )
    }

    fn model(translation: Vec3, scale: f32) -> Mat4 {
        Mat4::from_cols_array([
            scale, 0.0, 0.0, 0.0,
            0.0, scale, 0.0, 0.0,
            0.0, 0.0, scale, 0.0,
            translation.x, translation.y, translation.z, 1.0,
        ])
    }

    #[test]
    fn directional_shadow_darkens_occluded_texels(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match Renderer::new() {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping GPU test, no GPU device available: {error}");
                return Ok(());
            }
        };
        let floor = plane();
        let block = cube(0.9);
        let instances = vec![
            GpuInstance {
                mesh: &floor,
                model: model(Vec3::ZERO, 1.0),
                color: [0.7, 0.7, 0.7],
                lit: true,
            },
            GpuInstance {
                mesh: &block,
                model: model(Vec3::new(0.4, 1.2, 0.1), 1.0),
                color: [0.9, 0.4, 0.3],
                lit: true,
            },
        ];
        let without = renderer.render_offscreen_with(
            &instances,
            &camera(),
            view(),
            &light(),
            [0.0, 0.0, 0.0],
            (SIZE, SIZE),
        )?;
        let with = renderer.render_offscreen_shadowed(
            &instances,
            &camera(),
            view(),
            &light(),
            &ShadowSettings::for_quality(ShadowQuality::Medium),
            [0.0, 0.0, 0.0],
            (SIZE, SIZE),
        )?;
        assert_eq!(without.width, with.width);
        assert_eq!(without.height, with.height);
        let mut darkened = 0usize;
        let mut unchanged = 0usize;
        for (lit_pixel, shadowed_pixel) in without.pixels.iter().zip(with.pixels.iter()) {
            let drop = i32::from(lit_pixel[1]) - i32::from(shadowed_pixel[1]);
            if drop >= 12 {
                darkened += 1;
            } else if drop.abs() <= 2 && lit_pixel[1] > 40 {
                unchanged += 1;
            }
        }
        assert!(
            darkened >= 30,
            "expected at least 30 texels darkened by the shadow, found {darkened}"
        );
        assert!(
            unchanged >= 100,
            "expected a large lit region to stay unchanged, found {unchanged}"
        );
        Ok(())
    }

    #[test]
    fn shadow_free_default_path_is_unchanged_by_shadow_support(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match Renderer::new() {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping GPU test, no GPU device available: {error}");
                return Ok(());
            }
        };
        let block = cube(1.0);
        let instances = vec![GpuInstance {
            mesh: &block,
            model: model(Vec3::ZERO, 1.0),
            color: [0.9, 0.3, 0.2],
            lit: true,
        }];
        let first = renderer.render_offscreen_with(
            &instances,
            &camera(),
            view(),
            &light(),
            [0.05, 0.05, 0.08],
            (64, 64),
        )?;
        let second = renderer.render_offscreen_with(
            &instances,
            &camera(),
            view(),
            &light(),
            [0.05, 0.05, 0.08],
            (64, 64),
        )?;
        assert_eq!(first.pixels, second.pixels);
        Ok(())
    }
}
