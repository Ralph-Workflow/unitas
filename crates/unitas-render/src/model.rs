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
