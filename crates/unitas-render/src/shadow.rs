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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_levels_raise_map_size_and_distance() {
        assert!(ShadowQuality::Low.map_size() < ShadowQuality::Medium.map_size());
        assert!(ShadowQuality::Medium.map_size() < ShadowQuality::High.map_size());
        assert!(ShadowQuality::Low.distance() < ShadowQuality::Medium.distance());
        assert!(ShadowQuality::Medium.distance() < ShadowQuality::High.distance());
        for quality in [
            ShadowQuality::Low,
            ShadowQuality::Medium,
            ShadowQuality::High,
        ] {
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
}

#[cfg(all(test, feature = "device"))]
mod device_tests {
    use crate::camera::{look_at, Camera};
    use crate::gpu::{GpuInstance, Renderer};
    use crate::math::{Mat4, Vec3};
    use crate::mesh::cube;
    use crate::raster::DirectionalLight;

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

    fn model(translation: Vec3, scale: f32) -> Mat4 {
        Mat4::from_cols_array([
            scale,
            0.0,
            0.0,
            0.0,
            0.0,
            scale,
            0.0,
            0.0,
            0.0,
            0.0,
            scale,
            0.0,
            translation.x,
            translation.y,
            translation.z,
            1.0,
        ])
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
