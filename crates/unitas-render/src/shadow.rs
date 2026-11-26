use crate::camera::{look_at, Camera};
use crate::math::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowSettings {
    pub enabled: bool,
    pub soft: bool,
    pub resolution_index: u32,
    pub distance: f32,
}

impl ShadowSettings {
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            soft: false,
            resolution_index: 0,
            distance: 0.0,
        }
    }
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

#[must_use]
pub fn next_power_of_two(value: u32) -> u32 {
    if value <= 1 {
        1
    } else if value > (1_u32 << 31) {
        1_u32 << 31
    } else {
        value.next_power_of_two()
    }
}

#[must_use]
pub fn shadow_map_size(pixel_size: u32, resolution_index: u32, max_texels: u32) -> u32 {
    let multiplier = match resolution_index {
        0 => 0.125_f64,
        1 => 0.25_f64,
        2 => 0.5_f64,
        _ => 1.0_f64,
    };
    let scaled = (f64::from(pixel_size) * multiplier * 3.8).max(0.0);
    let raw = if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        scaled as u32
    };
    let cap = max_texels.max(64);
    next_power_of_two(raw).clamp(64, cap)
}

#[must_use]
pub fn light_view_projection(light_dir: Vec3, camera_position: Vec3, distance: f32) -> Mat4 {
    let half_extent = if distance > 0.0 {
        distance.max(1.0)
    } else {
        1.0
    };
    let dir = {
        let normalized = light_dir.normalize();
        if normalized.length_squared() > 1e-8 {
            normalized
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        }
    };
    let up = if dir.y.abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    };
    let eye = camera_position + dir.scale(half_extent);
    let view = look_at(eye, camera_position, up);
    let proj = Camera::orthographic(half_extent, 1.0, 0.0, 2.0 * half_extent).projection_matrix();
    proj * view
}

#[cfg(test)]
mod tests {
    use super::{light_view_projection, next_power_of_two, shadow_map_size, ShadowSettings};
    use crate::math::Vec3;

    #[test]
    fn shadow_settings_disabled_is_off() {
        let disabled = ShadowSettings::disabled();
        assert!(!disabled.enabled);
        assert!(!disabled.soft);
        assert_eq!(disabled.resolution_index, 0);
        assert!((disabled.distance - 0.0).abs() < 1e-6);
        assert_eq!(ShadowSettings::default(), disabled);
    }

    #[test]
    fn next_power_of_two_handles_boundaries() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(2), 2);
        assert_eq!(next_power_of_two(3), 4);
        assert_eq!(next_power_of_two(63), 64);
        assert_eq!(next_power_of_two(64), 64);
        assert_eq!(next_power_of_two(65), 128);
        assert_eq!(next_power_of_two(1024), 1024);
        assert_eq!(next_power_of_two((1_u32 << 31) + 1), 1_u32 << 31);
    }

    #[test]
    fn shadow_map_size_matches_unity_multiplier_table() {
        assert_eq!(shadow_map_size(256, 0, 4096), 128);
        assert_eq!(shadow_map_size(256, 1, 4096), 256);
        assert_eq!(shadow_map_size(256, 2, 4096), 512);
        assert_eq!(shadow_map_size(256, 3, 4096), 1024);
        assert_eq!(shadow_map_size(256, 4, 4096), 1024);
        assert_eq!(shadow_map_size(512, 2, 512), 512);
        assert_eq!(shadow_map_size(4, 0, 4096), 64);
    }

    #[test]
    fn light_view_projection_maps_volume_into_wgpu_ndc() {
        let light_dir = Vec3::new(0.4, 0.8, 0.2).normalize();
        let camera_pos = Vec3::new(2.0, 1.0, -3.0);
        let distance = 20.0;
        let vp = light_view_projection(light_dir, camera_pos, distance);

        let center_ndc = vp.project_point(camera_pos);
        assert!(center_ndc.x.abs() < 1e-5);
        assert!(center_ndc.y.abs() < 1e-5);
        assert!((center_ndc.z - 0.5).abs() < 1e-5);

        let inside_near = vp.project_point(camera_pos + light_dir.scale(10.0));
        assert!(inside_near.z >= 0.0 && inside_near.z <= 1.0);
        assert!(inside_near.z < center_ndc.z);

        let inside_far = vp.project_point(camera_pos - light_dir.scale(10.0));
        assert!(inside_far.z >= 0.0 && inside_far.z <= 1.0);
        assert!(inside_far.z > center_ndc.z);

        let outside_near = vp.project_point(camera_pos + light_dir.scale(25.0));
        assert!(outside_near.z < 0.0);

        let outside_far = vp.project_point(camera_pos - light_dir.scale(25.0));
        assert!(outside_far.z > 1.0);
    }
}
