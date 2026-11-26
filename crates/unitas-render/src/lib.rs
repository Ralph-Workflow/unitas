pub mod camera;
#[cfg(feature = "device")]
pub mod capture;
#[cfg(feature = "device")]
pub mod glyphs;
pub mod image;
pub mod math;
pub mod mesh;
pub mod raster;

#[cfg(feature = "device")]
pub mod gpu;
#[cfg(feature = "device")]
pub mod shadow;

pub use camera::{look_at, Camera};
pub use image::{Image, Rgba8};
pub use math::{Mat4, Vec3, Vec4};
pub use mesh::{cube, uv_sphere, Mesh};
pub use raster::{render, render_scene, render_with_view, DirectionalLight, MeshInstance};

#[cfg(feature = "device")]
pub use capture::{
    render_offscreen_with_ids, CaptureFrame, CaptureInstance, CaptureObject, ScreenBounds,
};

#[cfg(feature = "device")]
pub use gpu::{
    GpuInstance, GpuRenderer, RenderError, RenderScene, Renderer, TextOverlayItem,
    GPU_CPU_MEAN_ERROR_TOLERANCE,
};
#[cfg(feature = "device")]
pub use shadow::{ShadowQuality, ShadowSettings};
