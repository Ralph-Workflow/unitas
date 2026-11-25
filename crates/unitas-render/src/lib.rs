pub mod camera;
pub mod image;
pub mod math;
pub mod mesh;
pub mod raster;

#[cfg(feature = "device")]
pub mod gpu;

pub use camera::{look_at, Camera};
pub use image::Image;
pub use math::{Mat4, Vec3, Vec4};
pub use mesh::{cube, uv_sphere, Mesh};
pub use raster::{render, render_scene, render_with_view, DirectionalLight, MeshInstance};

#[cfg(feature = "device")]
pub use gpu::{
    GpuInstance, GpuRenderer, RenderError, RenderScene, Renderer, GPU_CPU_MEAN_ERROR_TOLERANCE,
};
