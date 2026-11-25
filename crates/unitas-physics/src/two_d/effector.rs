use crate::two_d::collider::{dot2, normalize2, rotate_vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformEffector2D {
    pub use_one_way: bool,
    pub surface_arc: f32,
    pub rotational_offset: f32,
    pub side_arc: f32,
    pub use_side_friction: bool,
    pub use_side_bounce: bool,
    pub collider_mask: u32,
}

impl Default for PlatformEffector2D {
    fn default() -> Self {
        Self {
            use_one_way: true,
            surface_arc: 180.0,
            rotational_offset: 0.0,
            side_arc: 0.0,
            use_side_friction: false,
            use_side_bounce: false,
            collider_mask: u32::MAX,
        }
    }
}

impl PlatformEffector2D {
    pub fn new(surface_arc_degrees: f32) -> Self {
        Self {
            surface_arc: surface_arc_degrees,
            ..Self::default()
        }
    }

    pub fn with_collider_mask(mut self, mask: u32) -> Self {
        self.collider_mask = mask;
        self
    }

    pub fn with_rotational_offset(mut self, offset_degrees: f32) -> Self {
        self.rotational_offset = offset_degrees;
        self
    }

    pub fn with_one_way(mut self, use_one_way: bool) -> Self {
        self.use_one_way = use_one_way;
        self
    }

    pub fn allows_layer(&self, layer: u8) -> bool {
        layer < 32 && (self.collider_mask & (1u32 << layer)) != 0
    }

    pub(crate) fn surface_up(&self, platform_angle_radians: f32) -> [f32; 2] {
        let total_angle = platform_angle_radians + self.rotational_offset.to_radians();
        rotate_vec2([0.0, 1.0], total_angle)
    }

    pub(crate) fn should_pass_through(
        &self,
        platform_angle_radians: f32,
        outward_normal_from_platform_to_other: [f32; 2],
        relative_velocity_other_minus_platform: [f32; 2],
    ) -> bool {
        if !self.use_one_way {
            return false;
        }
        let up = self.surface_up(platform_angle_radians);
        let half_arc_rad = (self.surface_arc.clamp(0.0, 360.0) * 0.5).to_radians();
        let cos_threshold = half_arc_rad.cos();
        let normal =
            normalize2(outward_normal_from_platform_to_other).unwrap_or([0.0, 1.0]);
        let normal_alignment = dot2(normal, up);
        if normal_alignment < cos_threshold - 1e-5 {
            return true;
        }
        let vel_along_up = dot2(relative_velocity_other_minus_platform, up);
        if vel_along_up > 1e-5 {
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceEffector2D {
    pub speed: f32,
    pub speed_variation: f32,
    pub force_scale: f32,
    pub use_contact_force: bool,
    pub use_friction: bool,
    pub use_bounce: bool,
    pub collider_mask: u32,
}

impl Default for SurfaceEffector2D {
    fn default() -> Self {
        Self {
            speed: 1.0,
            speed_variation: 0.0,
            force_scale: 1.0,
            use_contact_force: false,
            use_friction: true,
            use_bounce: true,
            collider_mask: u32::MAX,
        }
    }
}

impl SurfaceEffector2D {
    pub fn new(speed: f32, force_scale: f32) -> Self {
        Self {
            speed,
            force_scale,
            ..Self::default()
        }
    }

    pub fn with_collider_mask(mut self, mask: u32) -> Self {
        self.collider_mask = mask;
        self
    }

    pub fn allows_layer(&self, layer: u8) -> bool {
        layer < 32 && (self.collider_mask & (1u32 << layer)) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaEffector2D {
    pub force_angle: f32,
    pub force_magnitude: f32,
    pub force_variation: f32,
    pub drag: f32,
    pub angular_drag: f32,
    pub use_global_angle: bool,
    pub collider_mask: u32,
}

impl Default for AreaEffector2D {
    fn default() -> Self {
        Self {
            force_angle: 0.0,
            force_magnitude: 10.0,
            force_variation: 0.0,
            drag: 0.0,
            angular_drag: 0.0,
            use_global_angle: false,
            collider_mask: u32::MAX,
        }
    }
}

impl AreaEffector2D {
    pub fn new(force_angle_degrees: f32, force_magnitude: f32, drag: f32) -> Self {
        Self {
            force_angle: force_angle_degrees,
            force_magnitude,
            drag,
            ..Self::default()
        }
    }

    pub fn with_collider_mask(mut self, mask: u32) -> Self {
        self.collider_mask = mask;
        self
    }

    pub fn with_global_angle(mut self, use_global_angle: bool) -> Self {
        self.use_global_angle = use_global_angle;
        self
    }

    pub fn allows_layer(&self, layer: u8) -> bool {
        layer < 32 && (self.collider_mask & (1u32 << layer)) != 0
    }

    pub(crate) fn force_vector(&self, collider_angle_radians: f32) -> [f32; 2] {
        let base_rad = self.force_angle.to_radians();
        let angle = if self.use_global_angle {
            base_rad
        } else {
            base_rad + collider_angle_radians
        };
        let (s, c) = angle.sin_cos();
        [c * self.force_magnitude, s * self.force_magnitude]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointEffector2D {
    pub force_magnitude: f32,
    pub force_variation: f32,
    pub distance_scale: f32,
    pub drag: f32,
    pub angular_drag: f32,
    pub collider_mask: u32,
}

impl Default for PointEffector2D {
    fn default() -> Self {
        Self {
            force_magnitude: -10.0,
            force_variation: 0.0,
            distance_scale: 1.0,
            drag: 0.0,
            angular_drag: 0.0,
            collider_mask: u32::MAX,
        }
    }
}

impl PointEffector2D {
    pub fn new(force_magnitude: f32, distance_scale: f32) -> Self {
        Self {
            force_magnitude,
            distance_scale,
            ..Self::default()
        }
    }

    pub fn with_collider_mask(mut self, mask: u32) -> Self {
        self.collider_mask = mask;
        self
    }

    pub fn with_drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    pub fn allows_layer(&self, layer: u8) -> bool {
        layer < 32 && (self.collider_mask & (1u32 << layer)) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effector2D {
    Platform(PlatformEffector2D),
    Surface(SurfaceEffector2D),
    Area(AreaEffector2D),
    Point(PointEffector2D),
}

impl From<PlatformEffector2D> for Effector2D {
    fn from(effector: PlatformEffector2D) -> Self {
        Self::Platform(effector)
    }
}

impl From<SurfaceEffector2D> for Effector2D {
    fn from(effector: SurfaceEffector2D) -> Self {
        Self::Surface(effector)
    }
}

impl From<AreaEffector2D> for Effector2D {
    fn from(effector: AreaEffector2D) -> Self {
        Self::Area(effector)
    }
}

impl From<PointEffector2D> for Effector2D {
    fn from(effector: PointEffector2D) -> Self {
        Self::Point(effector)
    }
}
