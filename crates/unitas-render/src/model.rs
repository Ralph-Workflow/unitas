use crate::image::Image;
use crate::math::{Mat4, Vec3};
use crate::mesh::{Mesh, MeshAttributes};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshInstance<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialInstance<'a> {
    pub mesh: &'a Mesh,
    pub attributes: Option<&'a MeshAttributes>,
    pub model: Mat4,
    pub color: [f32; 3],
    pub material: &'a Material,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
}

pub const MAX_GPU_LIGHTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuLightKind {
    Directional,
    Point,
    Spot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuLight {
    pub kind: GpuLightKind,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub inner_spot_angle_deg: f32,
    pub spot_angle_deg: f32,
}

impl GpuLight {
    pub fn directional(direction: Vec3, color: [f32; 3], intensity: f32) -> Self {
        Self {
            kind: GpuLightKind::Directional,
            position: Vec3::ZERO,
            direction,
            color,
            intensity,
            range: 0.0,
            inner_spot_angle_deg: 0.0,
            spot_angle_deg: 0.0,
        }
    }

    pub fn point(position: Vec3, color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            kind: GpuLightKind::Point,
            position,
            direction: Vec3::new(0.0, 1.0, 0.0),
            color,
            intensity,
            range,
            inner_spot_angle_deg: 0.0,
            spot_angle_deg: 0.0,
        }
    }

    pub fn spot(
        position: Vec3,
        direction: Vec3,
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_spot_angle_deg: f32,
        spot_angle_deg: f32,
    ) -> Self {
        Self {
            kind: GpuLightKind::Spot,
            position,
            direction,
            color,
            intensity,
            range,
            inner_spot_angle_deg,
            spot_angle_deg,
        }
    }
}

/// The shape of a 2D light applied to the sprite pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Light2DKind {
    #[default]
    Global,
    Point,
    Spot,
}

/// A 2D light in the world XY plane, applied to the sprite pass.
///
/// A global light tints every sprite with `color * intensity`. A point light
/// falls off linearly from full strength inside `inner_radius` to nothing at
/// `outer_radius`, measured in world units from `position`. A spot light adds
/// a cone around `direction` whose half angles are `inner_angle_deg` and
/// `outer_angle_deg`, so the light is at full strength inside the inner cone
/// and fades to nothing at the outer cone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Light2D {
    pub kind: Light2DKind,
    pub color: [f32; 3],
    pub intensity: f32,
    pub position: [f32; 2],
    pub direction: [f32; 2],
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub inner_angle_deg: f32,
    pub outer_angle_deg: f32,
}

pub const MAX_LIGHTS_2D: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Opaque,
    Blend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub albedo: Option<Image>,
    pub normal: Option<Image>,
    pub emissive: Option<Image>,
    pub metallic: f32,
    pub smoothness: f32,
    pub alpha: f32,
    pub blend: BlendMode,
}

pub static DEFAULT_MATERIAL: Material = Material::plain();

impl Material {
    pub const fn plain() -> Self {
        Self {
            albedo: None,
            normal: None,
            emissive: None,
            metallic: 0.0,
            smoothness: 0.0,
            alpha: 1.0,
            blend: BlendMode::Opaque,
        }
    }

    pub fn is_textured(&self) -> bool {
        self.albedo.is_some() || self.normal.is_some() || self.emissive.is_some()
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::plain()
    }
}

pub fn effective_alpha(material: &Material) -> f32 {
    match material.blend {
        BlendMode::Opaque => 1.0,
        BlendMode::Blend => material.alpha.clamp(0.0, 1.0),
    }
}
