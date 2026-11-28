use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::camera::Camera;
use crate::capture::CaptureInstance;
use crate::cull::{visible, Aabb, Frustum};
use crate::glyphs::{
    FontLibrary, GlyphAtlas, GlyphResources, HorizontalAlign, Rect, SwashCache, TextFrame,
    TextPipeline, TexturedQuad, VerticalAlign,
};
use crate::image::{Image, Rgba8};
use crate::light_2d::{create_lights_2d_buffer, pack_lights_2d, LIGHTS_2D_BYTES};
use crate::math::{Mat4, Vec3};
use crate::mesh::{Mesh, MeshAttributes};
use crate::model::{
    effective_alpha, BlendMode, DirectionalLight, Light2D, Material, MaterialInstance,
    MeshInstance, MAX_LIGHTS_2D,
};
use crate::render_graph::{PostChain, RenderGraph, RenderPassKind};
use crate::shadow::{light_view_projection, shadow_map_size, ShadowSettings};
use crate::sprite::{sprite_quad, texture_runs, Sprite};

use gpu_pipelines::{
    build_shadow_pipeline, create_capture_pipelines, create_default_material_bind,
    create_global_layout, create_instance_layout, create_material_layout, create_material_sampler,
    create_pipelines, create_scene_pipeline_layout, create_scene_shader, create_shadow_layout,
    create_shadow_pipeline_layout, create_shadow_resources, create_shadow_sampler,
};

#[path = "gpu_capture.rs"]
mod gpu_capture;
#[path = "gpu_pipelines.rs"]
mod gpu_pipelines;
#[path = "gpu_shader.rs"]
mod gpu_shader;
#[path = "gpu_surface.rs"]
mod gpu_surface;
#[path = "post.rs"]
pub mod post;

pub use crate::model::{GpuLight, GpuLightKind, MAX_GPU_LIGHTS};
pub use gpu_surface::{present_mode_for_v_sync_count, SurfaceFrame};

const VERTEX_FLOATS: usize = 15;
const VERTEX_STRIDE: u64 = 60;
const UNIFORM_SLOT: u64 = 256;
const GLOBAL_SLOT: u64 = 1024;
const GLOBAL_FLOATS: usize = 180;
const INSTANCE_FLOATS: usize = 24;
const SLOT_FLOATS: usize = (UNIFORM_SLOT / 4) as usize;
const COPY_ROW_ALIGNMENT: u64 = 256;
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const INITIAL_SHADOW_TEXTURE_SIZE: u32 = 64;
const MAX_CACHED_RESOURCES: usize = 4096;

pub const GPU_CPU_MEAN_ERROR_TOLERANCE: f32 = 4.0;

#[derive(Debug)]
pub enum RenderError {
    AdapterUnavailable(String),
    DeviceRequestFailed(String),
    SurfaceUnavailable(String),
    SurfaceNotConfigured,
    InvalidSize(u32, u32),
    BufferCast(&'static str),
    Readback(String),
    Poll(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::AdapterUnavailable(message) => {
                write!(formatter, "no GPU adapter available: {message}")
            }
            RenderError::DeviceRequestFailed(message) => {
                write!(formatter, "GPU device request failed: {message}")
            }
            RenderError::SurfaceUnavailable(message) => {
                write!(formatter, "surface frame unavailable: {message}")
            }
            RenderError::SurfaceNotConfigured => {
                write!(formatter, "surface was not configured for this renderer")
            }
            RenderError::InvalidSize(width, height) => {
                write!(formatter, "invalid render target size {width}x{height}")
            }
            RenderError::BufferCast(kind) => {
                write!(formatter, "failed to cast {kind} data for GPU upload")
            }
            RenderError::Readback(message) => write!(formatter, "readback failed: {message}"),
            RenderError::Poll(message) => write!(formatter, "device poll failed: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuInstance<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
    pub lit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuMaterialInstance<'a> {
    pub mesh: &'a Mesh,
    pub attributes: Option<&'a MeshAttributes>,
    pub model: Mat4,
    pub color: [f32; 3],
    pub material: &'a Material,
}

impl<'a> From<MaterialInstance<'a>> for GpuMaterialInstance<'a> {
    fn from(instance: MaterialInstance<'a>) -> Self {
        Self {
            mesh: instance.mesh,
            attributes: instance.attributes,
            model: instance.model,
            color: instance.color,
            material: instance.material,
        }
    }
}

impl<'a, 'b> From<&'a MaterialInstance<'b>> for GpuMaterialInstance<'b>
where
    'b: 'a,
{
    fn from(instance: &'a MaterialInstance<'b>) -> Self {
        Self {
            mesh: instance.mesh,
            attributes: instance.attributes,
            model: instance.model,
            color: instance.color,
            material: instance.material,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderScene<'a> {
    pub camera: &'a Camera,
    pub view: Mat4,
    pub light: &'a DirectionalLight,
    pub background: [f32; 3],
}

pub type GpuRenderer = Renderer;

#[derive(Debug, Clone, PartialEq)]
pub struct TextOverlayItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size_px: f32,
    pub color: [f32; 4],
    pub rect: [f32; 4],
    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,
    pub wrap: bool,
    pub max_width: Option<f32>,
    pub line_spacing: f32,
    pub clip_rect: Option<[f32; 4]>,
}

impl Default for TextOverlayItem {
    fn default() -> Self {
        Self {
            text: String::new(),
            x: 0.0,
            y: 0.0,
            size_px: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            rect: [0.0; 4],
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            wrap: false,
            max_width: None,
            line_spacing: 1.0,
            clip_rect: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRect {
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub sprite: Option<Rgba8>,
    pub uv_rect: [f32; 4],
    pub clip_rect: Option<[f32; 4]>,
}

impl Default for CanvasRect {
    fn default() -> Self {
        Self {
            rect: [0.0; 4],
            color: [1.0, 1.0, 1.0, 1.0],
            sprite: None,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect: None,
        }
    }
}

impl<'a, 'b> From<&'a MeshInstance<'b>> for GpuInstance<'b>
where
    'b: 'a,
{
    fn from(instance: &'a MeshInstance<'b>) -> Self {
        GpuInstance {
            mesh: instance.mesh,
            model: instance.model,
            color: instance.color,
            lit: true,
        }
    }
}

#[derive(Clone)]
struct Pipelines {
    lit: wgpu::RenderPipeline,
    unlit: wgpu::RenderPipeline,
    lit_blend: wgpu::RenderPipeline,
    unlit_blend: wgpu::RenderPipeline,
    sprite: wgpu::RenderPipeline,
}

struct UniformState {
    buffer: wgpu::Buffer,
    capacity: usize,
    global_bind_group: wgpu::BindGroup,
    instance_bind_group: wgpu::BindGroup,
    lights_2d_buffer: wgpu::Buffer,
}

struct SurfaceState {
    pipelines: Pipelines,
    format: wgpu::TextureFormat,
    config_size: (u32, u32),
    depth: wgpu::Texture,
    depth_size: (u32, u32),
}

struct TextTarget {
    pipeline: TextPipeline,
    resources: GlyphResources,
    uploaded: usize,
}

struct TextState {
    fonts: FontLibrary,
    cache: SwashCache,
    atlas: GlyphAtlas,
    targets: HashMap<wgpu::TextureFormat, TextTarget>,
    image_targets: HashMap<wgpu::TextureFormat, ImageTarget>,
    solid_targets: HashMap<wgpu::TextureFormat, ImageTarget>,
}

struct ImageTarget {
    pipeline: TextPipeline,
    resources: Option<GlyphResources>,
    size: (u32, u32),
}

struct ShadowState {
    settings: ShadowSettings,
    current_size: u32,
    texture_size: u32,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

struct SceneLightingState {
    lights: Vec<GpuLight>,
    ambient: [f32; 3],
}

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    global_layout: wgpu::BindGroupLayout,
    instance_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    offscreen_pipelines: Pipelines,
    capture_pipelines: Pipelines,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_sampler: wgpu::Sampler,
    shadow: Mutex<ShadowState>,
    uniforms: Mutex<UniformState>,
    cache: Mutex<ResourceCache>,
    material_sampler: wgpu::Sampler,
    material_bind: wgpu::BindGroup,
    surface: Mutex<Option<SurfaceState>>,
    text: Mutex<TextState>,
    sprites: Mutex<Vec<Sprite>>,
    lights_2d: Mutex<Vec<Light2D>>,
    lighting: Mutex<SceneLightingState>,
    sprite_quad: (Mesh, MeshAttributes),
    v_sync_count: AtomicU32,
    post: Mutex<post::PostState>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

fn solid_texture(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("solid"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn image_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &Image,
    fallback: [u8; 4],
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    let width = image.width;
    let height = image.height;
    if width == 0 || height == 0 {
        return solid_texture(device, queue, fallback);
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("material-image"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for &[r, g, b] in &image.pixels {
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn camera_world_position(view: Mat4) -> Vec3 {
    let m = view.as_array();
    Vec3::new(
        -(m[0] * m[12] + m[1] * m[13] + m[2] * m[14]),
        -(m[4] * m[12] + m[5] * m[13] + m[6] * m[14]),
        -(m[8] * m[12] + m[9] * m[13] + m[10] * m[14]),
    )
}

struct FramePasses<'a> {
    graph: &'a RenderGraph,
    items: &'a [DrawItem<'a>],
    prepared: &'a [PreparedDraw],
    prepared_sprites: &'a [PreparedSprite],
    view: Mat4,
    overlay: &'a [TextOverlayItem],
    post_enabled: bool,
}

struct OffscreenTarget {
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    scene_view: wgpu::TextureView,
    pipelines: Pipelines,
    background: [f32; 3],
    size: (u32, u32),
}

struct SceneResources {
    shader: wgpu::ShaderModule,
    global_layout: wgpu::BindGroupLayout,
    instance_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    offscreen_pipelines: Pipelines,
    capture_pipelines: Pipelines,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_sampler: wgpu::Sampler,
}

impl Renderer {
    pub fn new() -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(|error| RenderError::AdapterUnavailable(error.to_string()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| RenderError::DeviceRequestFailed(error.to_string()))?;
        Self::from_parts(instance, adapter, device, queue)
    }

    fn create_scene_resources(device: &wgpu::Device) -> SceneResources {
        let shader = create_scene_shader(device);
        let global_layout = create_global_layout(device);
        let instance_layout = create_instance_layout(device);
        let material_layout = create_material_layout(device);
        let shadow_layout = create_shadow_layout(device);
        let pipeline_layout = create_scene_pipeline_layout(
            device,
            &global_layout,
            &instance_layout,
            &material_layout,
            &shadow_layout,
        );
        let shadow_pipeline_layout =
            create_shadow_pipeline_layout(device, &global_layout, &instance_layout);
        let offscreen_pipelines =
            create_pipelines(device, &pipeline_layout, &shader, OFFSCREEN_FORMAT);
        let capture_pipelines = create_capture_pipelines(
            device,
            &pipeline_layout,
            &shader,
            OFFSCREEN_FORMAT,
            ID_FORMAT,
        );
        let shadow_pipeline = build_shadow_pipeline(device, &shadow_pipeline_layout, &shader);
        let shadow_sampler = create_shadow_sampler(device);
        SceneResources {
            shader,
            global_layout,
            instance_layout,
            material_layout,
            shadow_layout,
            pipeline_layout,
            offscreen_pipelines,
            capture_pipelines,
            shadow_pipeline,
            shadow_sampler,
        }
    }

    fn create_shadow_state(
        device: &wgpu::Device,
        shadow_layout: &wgpu::BindGroupLayout,
        shadow_sampler: &wgpu::Sampler,
    ) -> ShadowState {
        let (shadow_view, shadow_bind_group) = create_shadow_resources(
            device,
            shadow_layout,
            shadow_sampler,
            INITIAL_SHADOW_TEXTURE_SIZE,
        );
        ShadowState {
            settings: ShadowSettings::disabled(),
            current_size: 0,
            texture_size: INITIAL_SHADOW_TEXTURE_SIZE,
            view: shadow_view,
            bind_group: shadow_bind_group,
        }
    }

    fn create_text_state() -> TextState {
        let mut fonts = FontLibrary::new();
        fonts.load_bundled_font();
        fonts.load_system_fonts();
        TextState {
            fonts,
            cache: SwashCache::new(),
            atlas: GlyphAtlas::new(1024, 1024),
            targets: HashMap::new(),
            image_targets: HashMap::new(),
            solid_targets: HashMap::new(),
        }
    }

    fn default_lighting_state() -> SceneLightingState {
        SceneLightingState {
            lights: Vec::new(),
            ambient: [0.0, 0.0, 0.0],
        }
    }

    fn from_parts(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Result<Self, RenderError> {
        let scene = Self::create_scene_resources(&device);
        let shadow =
            Self::create_shadow_state(&device, &scene.shadow_layout, &scene.shadow_sampler);
        let material_sampler = create_material_sampler(&device);
        let material_bind = create_default_material_bind(
            &device,
            &queue,
            &scene.material_layout,
            &material_sampler,
        );
        let uniforms = create_uniforms(&device, &scene.global_layout, &scene.instance_layout, 1);
        let post = post::PostState::new(&device);
        let text = Self::create_text_state();
        Ok(Renderer {
            instance,
            adapter,
            device,
            queue,
            global_layout: scene.global_layout,
            instance_layout: scene.instance_layout,
            material_layout: scene.material_layout,
            shadow_layout: scene.shadow_layout,
            pipeline_layout: scene.pipeline_layout,
            shader: scene.shader,
            offscreen_pipelines: scene.offscreen_pipelines,
            capture_pipelines: scene.capture_pipelines,
            shadow_pipeline: scene.shadow_pipeline,
            shadow_sampler: scene.shadow_sampler,
            shadow: Mutex::new(shadow),
            uniforms: Mutex::new(uniforms),
            cache: Mutex::new(ResourceCache::default()),
            material_sampler,
            material_bind,
            surface: Mutex::new(None),
            text: Mutex::new(text),
            sprites: Mutex::new(Vec::new()),
            lights_2d: Mutex::new(Vec::new()),
            lighting: Mutex::new(Self::default_lighting_state()),
            sprite_quad: sprite_quad(),
            v_sync_count: AtomicU32::new(1),
            post: Mutex::new(post),
        })
    }

    pub fn set_shadow_settings(&self, settings: ShadowSettings) {
        let mut shadow = lock(&self.shadow);
        if !settings.enabled {
            shadow.current_size = 0;
        }
        shadow.settings = settings;
    }

    pub fn shadow_settings(&self) -> ShadowSettings {
        lock(&self.shadow).settings
    }

    pub fn set_v_sync_count(&self, v_sync_count: u32) {
        self.v_sync_count.store(v_sync_count, Ordering::Relaxed);
    }

    pub fn v_sync_count(&self) -> u32 {
        self.v_sync_count.load(Ordering::Relaxed)
    }

    pub fn shadow_map_size(&self) -> u32 {
        let shadow = lock(&self.shadow);
        if shadow.settings.enabled {
            shadow.current_size
        } else {
            0
        }
    }

    pub fn invalidate_resource_cache(&self) {
        let mut cache = lock(&self.cache);
        cache.meshes.clear();
        cache.materials.clear();
        cache.sprite_textures.clear();
    }

    pub fn set_sprites(&self, sprites: Vec<Sprite>) {
        let mut state = lock(&self.sprites);
        *state = sprites;
    }

    pub fn sprites(&self) -> Vec<Sprite> {
        lock(&self.sprites).clone()
    }

    pub fn set_scene_lighting(&self, lights: &[GpuLight], ambient: [f32; 3]) {
        let mut state = lock(&self.lighting);
        state.lights.clear();
        state.lights.extend_from_slice(lights);
        state.ambient = ambient;
    }

    pub fn set_lights_2d(&self, lights: &[Light2D]) {
        let mut state = lock(&self.lights_2d);
        state.clear();
        state.extend(lights.iter().take(MAX_LIGHTS_2D).copied());
    }

    pub fn scene_lights(&self) -> Vec<GpuLight> {
        lock(&self.lighting).lights.clone()
    }

    pub fn scene_ambient(&self) -> [f32; 3] {
        lock(&self.lighting).ambient
    }

    pub fn mesh_upload_count(&self) -> u32 {
        lock(&self.cache).mesh_uploads
    }

    pub fn material_bind_group_count(&self) -> u32 {
        lock(&self.cache).material_uploads
    }

    fn prepare_shadow_map(&self, pixel_size: u32) -> bool {
        let mut shadow = lock(&self.shadow);
        if !shadow.settings.enabled {
            shadow.current_size = 0;
            return false;
        }
        let max_texels = if shadow.settings.resolution_index >= 3 {
            4096
        } else {
            2048
        };
        let map_size = shadow_map_size(pixel_size, shadow.settings.resolution_index, max_texels);
        if shadow.texture_size != map_size {
            let (view, bind_group) = create_shadow_resources(
                &self.device,
                &self.shadow_layout,
                &self.shadow_sampler,
                map_size,
            );
            shadow.view = view;
            shadow.bind_group = bind_group;
            shadow.texture_size = map_size;
        }
        shadow.current_size = map_size;
        true
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn has_overlay_fonts(&self) -> bool {
        lock(&self.text).fonts.has_fonts()
    }

    pub fn render_offscreen(
        &self,
        instances: &[MeshInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
        size: (u32, u32),
    ) -> Result<Image, RenderError> {
        let converted: Vec<GpuInstance<'_>> = instances.iter().map(GpuInstance::from).collect();
        self.render_offscreen_with(&converted, camera, view, light, background, size)
    }

    pub fn render_offscreen_scene(
        &self,
        instances: &[MeshInstance<'_>],
        scene: RenderScene<'_>,
        width: u32,
        height: u32,
    ) -> Result<Image, RenderError> {
        let converted: Vec<GpuInstance<'_>> = instances.iter().map(GpuInstance::from).collect();
        self.render_offscreen_with_overlay(&converted, scene, (width, height), &[])
    }

    pub fn render_offscreen_scene_lights(
        &self,
        instances: &[MeshInstance<'_>],
        scene: RenderScene<'_>,
        lights: &[GpuLight],
        ambient: [f32; 3],
        width: u32,
        height: u32,
    ) -> Result<Image, RenderError> {
        self.set_scene_lighting(lights, ambient);
        let converted: Vec<GpuInstance<'_>> = instances.iter().map(GpuInstance::from).collect();
        self.render_offscreen_items(plain_draw_items(&converted), scene, (width, height), &[])
            .map(Image::from_rgba8)
    }

    pub fn render_offscreen_with(
        &self,
        instances: &[GpuInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
        size: (u32, u32),
    ) -> Result<Image, RenderError> {
        let scene = RenderScene {
            camera,
            view,
            light,
            background,
        };
        self.render_offscreen_with_overlay(instances, scene, size, &[])
    }

    pub fn render_offscreen_with_overlay(
        &self,
        instances: &[GpuInstance<'_>],
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Image, RenderError> {
        let rgba = self.render_offscreen_rgba_with_overlay(instances, scene, size, overlay)?;
        Ok(Image::from_rgba8(rgba))
    }

    pub fn render_offscreen_rgba_with_overlay(
        &self,
        instances: &[GpuInstance<'_>],
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Rgba8, RenderError> {
        self.render_offscreen_items(plain_draw_items(instances), scene, size, overlay)
    }

    pub fn render_offscreen_materials(
        &self,
        instances: &[GpuMaterialInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
        size: (u32, u32),
    ) -> Result<Image, RenderError> {
        let scene = RenderScene {
            camera,
            view,
            light,
            background,
        };
        let rgba = self.render_offscreen_items(material_draw_items(instances), scene, size, &[])?;
        Ok(Image::from_rgba8(rgba))
    }

    pub fn render_offscreen_materials_with_overlay(
        &self,
        instances: &[GpuMaterialInstance<'_>],
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Image, RenderError> {
        let rgba =
            self.render_offscreen_materials_rgba_with_overlay(instances, scene, size, overlay)?;
        Ok(Image::from_rgba8(rgba))
    }

    pub fn render_offscreen_materials_rgba_with_overlay(
        &self,
        instances: &[GpuMaterialInstance<'_>],
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Rgba8, RenderError> {
        self.render_offscreen_items(material_draw_items(instances), scene, size, overlay)
    }

    fn create_offscreen_target(
        &self,
        width: u32,
        height: u32,
        background: [f32; 3],
    ) -> Result<OffscreenTarget, RenderError> {
        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen-color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth_texture = self.create_depth_texture(width, height);
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (scene_view, pipelines) = self.post_scene_target((width, height), &color_view)?;
        Ok(OffscreenTarget {
            color_texture,
            color_view,
            depth_view,
            scene_view,
            pipelines,
            background,
            size: (width, height),
        })
    }

    fn encode_offscreen_opaque_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("offscreen-opaque"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.scene_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(background_color(target.background)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let indices = opaque_order(items);
        self.draw_pass_indices(&mut pass, &target.pipelines, items, prepared, &indices);
    }

    fn encode_offscreen_transparent_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
        indices: &[usize],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("offscreen-transparent"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.scene_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.draw_pass_indices(&mut pass, &target.pipelines, items, prepared, indices);
    }

    fn encode_offscreen_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        passes: &FramePasses<'_>,
    ) {
        for graph_pass in passes.graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(encoder, passes.items, passes.prepared);
                }
                RenderPassKind::Opaque => {
                    self.encode_offscreen_opaque_pass(
                        encoder,
                        target,
                        passes.items,
                        passes.prepared,
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(passes.items, passes.view);
                    if !indices.is_empty() {
                        self.encode_offscreen_transparent_pass(
                            encoder,
                            target,
                            passes.items,
                            passes.prepared,
                            &indices,
                        );
                    }
                }
                RenderPassKind::Sprites => {
                    self.encode_sprite_pass(
                        encoder,
                        &[&target.scene_view],
                        &target.depth_view,
                        passes.prepared_sprites,
                        &target.pipelines.sprite,
                        passes.items.len(),
                    );
                }
                RenderPassKind::UiOverlay => {
                    if !passes.post_enabled {
                        self.draw_text_overlay(
                            encoder,
                            &target.color_view,
                            OFFSCREEN_FORMAT,
                            target.size,
                            1.0,
                            passes.overlay,
                        );
                    }
                }
                RenderPassKind::CaptureIds => {}
            }
        }
    }

    fn encode_offscreen_post(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        graph: &RenderGraph,
        target: &OffscreenTarget,
        overlay: &[TextOverlayItem],
    ) -> Result<(), RenderError> {
        self.encode_post_chain(
            encoder,
            graph.post_passes(),
            &target.color_view,
            OFFSCREEN_FORMAT,
            target.size,
        )?;
        self.draw_text_overlay(
            encoder,
            &target.color_view,
            OFFSCREEN_FORMAT,
            target.size,
            1.0,
            overlay,
        );
        Ok(())
    }

    fn read_offscreen_frame(
        &self,
        mut encoder: wgpu::CommandEncoder,
        target: &OffscreenTarget,
        width: u32,
        height: u32,
    ) -> Result<Rgba8, RenderError> {
        let bytes_per_row = align_up(u64::from(width) * 4, COPY_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: bytes_per_row * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let data = self.read_buffer(&readback)?;
        Ok(rgba_from_readback(&data, width, height, bytes_per_row))
    }

    fn render_offscreen_items(
        &self,
        items: Vec<DrawItem<'_>>,
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Rgba8, RenderError> {
        let (width, height) = size;
        if width == 0 || height == 0 {
            return Ok(Rgba8::new(width, height));
        }
        let shadows_enabled = self.prepare_shadow_map(width.max(height));
        let sprites = self.sprites();
        let post_enabled = self.post_settings().enabled;
        let graph = RenderGraph::offscreen(!overlay.is_empty(), shadows_enabled)
            .with_sprites(!sprites.is_empty())
            .with_post(if post_enabled {
                PostChain::bloom_tonemap()
            } else {
                PostChain::default()
            });
        let target = self.create_offscreen_target(width, height, scene.background)?;
        let (prepared, prepared_sprites) =
            self.prepare_draw_items(&items, &sprites, scene.camera, scene.view, scene.light)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen"),
            });
        let passes = FramePasses {
            graph: &graph,
            items: &items,
            prepared: &prepared,
            prepared_sprites: &prepared_sprites,
            view: scene.view,
            overlay,
            post_enabled,
        };
        self.encode_offscreen_passes(&mut encoder, &target, &passes);
        if post_enabled {
            self.encode_offscreen_post(&mut encoder, &graph, &target, overlay)?;
        }
        self.read_offscreen_frame(encoder, &target, width, height)
    }

    pub fn draw_rgba_to_view(
        &self,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        image: &Rgba8,
    ) -> Result<(), RenderError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("image"),
            });
        self.draw_rgba_overlay(&mut encoder, view, format, size, image)?;
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    fn image_target<'a>(
        &self,
        text: &'a mut TextState,
        format: wgpu::TextureFormat,
        image: &Rgba8,
    ) -> &'a mut ImageTarget {
        let target = text
            .image_targets
            .entry(format)
            .or_insert_with(|| ImageTarget {
                pipeline: TextPipeline::new(&self.device, format),
                resources: None,
                size: (0, 0),
            });
        if target.size != (image.width, image.height) {
            target.resources = Some(
                target
                    .pipeline
                    .upload_image(&self.device, &self.queue, image),
            );
            target.size = (image.width, image.height);
        } else if let Some(resources) = &target.resources {
            TextPipeline::write_image(&self.queue, resources, image);
        }
        target
    }

    fn draw_rgba_overlay(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        image: &Rgba8,
    ) -> Result<(), RenderError> {
        if size.0 == 0 || size.1 == 0 || image.width == 0 || image.height == 0 {
            return Ok(());
        }
        let mut text = lock(&self.text);
        let target = self.image_target(&mut text, format, image);
        let Some(resources) = &target.resources else {
            return Ok(());
        };
        let quads = [TexturedQuad {
            x: 0.0,
            y: 0.0,
            width: size.0 as f32,
            height: size.1 as f32,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }];
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: size.0 as f32,
            height: size.1 as f32,
        };
        let mut target_frame = TextFrame {
            encoder,
            view,
            frame,
            clip: frame,
        };
        target.pipeline.draw(
            &self.device,
            &self.queue,
            resources,
            &mut target_frame,
            &quads,
        );
        Ok(())
    }

    fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn upload_mesh(
        &self,
        mesh: &Mesh,
        attributes: Option<&MeshAttributes>,
    ) -> Result<MeshBuffers, RenderError> {
        let mut vertex_data: Vec<f32> = Vec::with_capacity(mesh.positions.len() * VERTEX_FLOATS);
        for (index, position) in mesh.positions.iter().enumerate() {
            vertex_data.extend_from_slice(&[position.x, position.y, position.z]);
            let normal = mesh.normals.get(index).copied().unwrap_or(Vec3::ZERO);
            vertex_data.extend_from_slice(&[normal.x, normal.y, normal.z]);
            let uv = attributes
                .and_then(|attrs| attrs.uvs.get(index).copied())
                .unwrap_or([0.0, 0.0]);
            vertex_data.extend_from_slice(&[uv[0], uv[1]]);
            let tangent = attributes
                .and_then(|attrs| attrs.tangents.get(index).copied())
                .unwrap_or(Vec3::ZERO);
            vertex_data.extend_from_slice(&[tangent.x, tangent.y, tangent.z]);
            vertex_data.extend_from_slice(&[1.0, 1.0, 1.0, 1.0]);
        }
        let vertex_bytes = bytemuck::try_cast_slice::<f32, u8>(&vertex_data)
            .map_err(|_| RenderError::BufferCast("vertex"))?;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh-vertices"),
            size: vertex_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&vertex_buffer, 0, vertex_bytes);
        let index_bytes = bytemuck::try_cast_slice::<u32, u8>(&mesh.indices)
            .map_err(|_| RenderError::BufferCast("index"))?;
        let index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh-indices"),
            size: index_bytes.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&index_buffer, 0, index_bytes);
        Ok(MeshBuffers {
            vertices: vertex_buffer,
            indices: index_buffer,
            index_count: mesh.indices.len() as u32,
            bounds: Aabb::from_positions(&mesh.positions),
        })
    }

    fn cached_mesh(
        &self,
        mesh: &Mesh,
        attributes: Option<&MeshAttributes>,
    ) -> Result<Option<MeshBuffers>, RenderError> {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            return Ok(None);
        }
        let key = MeshKey {
            mesh: std::ptr::from_ref(mesh) as usize,
            attributes: attributes.map_or(0, |attrs| std::ptr::from_ref(attrs) as usize),
            vertex_count: mesh.positions.len(),
            index_count: mesh.indices.len(),
            probe: mesh_probe(mesh, attributes),
        };
        let mut cache = lock(&self.cache);
        if let Some(cached) = cache.meshes.get(&key) {
            return Ok(Some(cached.clone()));
        }
        if cache.meshes.len() >= MAX_CACHED_RESOURCES {
            cache.meshes.clear();
        }
        let buffers = self.upload_mesh(mesh, attributes)?;
        cache.meshes.insert(key, buffers.clone());
        cache.mesh_uploads += 1;
        Ok(Some(buffers))
    }

    fn cached_material_bind_group(&self, material: &Material) -> wgpu::BindGroup {
        let key = material_key(material);
        let mut cache = lock(&self.cache);
        if let Some(cached) = cache.materials.get(&key) {
            return cached.clone();
        }
        if cache.materials.len() >= MAX_CACHED_RESOURCES {
            cache.materials.clear();
        }
        let bind_group = self.create_material_bind_group(material);
        cache.materials.insert(key, bind_group.clone());
        cache.material_uploads += 1;
        bind_group
    }

    fn create_material_bind_group(&self, material: &Material) -> wgpu::BindGroup {
        let albedo = match &material.albedo {
            Some(image) => image_texture(
                &self.device,
                &self.queue,
                image,
                [255, 255, 255, 255],
                wgpu::TextureFormat::Rgba8Unorm,
            ),
            None => solid_texture(&self.device, &self.queue, [255, 255, 255, 255]),
        };
        let normal = match &material.normal {
            Some(image) => image_texture(
                &self.device,
                &self.queue,
                image,
                [128, 128, 255, 255],
                wgpu::TextureFormat::Rgba8Unorm,
            ),
            None => solid_texture(&self.device, &self.queue, [128, 128, 255, 255]),
        };
        let emissive = match &material.emissive {
            Some(image) => image_texture(
                &self.device,
                &self.queue,
                image,
                [0, 0, 0, 255],
                wgpu::TextureFormat::Rgba8Unorm,
            ),
            None => solid_texture(&self.device, &self.queue, [0, 0, 0, 255]),
        };
        let metallic = material.metallic.clamp(0.0, 1.0);
        let smoothness = material.smoothness.clamp(0.0, 1.0);
        let alpha = effective_alpha(material);
        self.create_texture_bind_group(albedo, normal, emissive, [metallic, smoothness, alpha, 0.0])
    }

    fn cached_sprite_bind_group(&self, image: &Image) -> wgpu::BindGroup {
        let key = image_key(image);
        let mut cache = lock(&self.cache);
        if let Some(cached) = cache.sprite_textures.get(&key) {
            return cached.clone();
        }
        if cache.sprite_textures.len() >= MAX_CACHED_RESOURCES {
            cache.sprite_textures.clear();
        }
        let bind_group = self.create_sprite_bind_group(image);
        cache.sprite_textures.insert(key, bind_group.clone());
        bind_group
    }

    fn create_sprite_bind_group(&self, image: &Image) -> wgpu::BindGroup {
        let albedo = image_texture(
            &self.device,
            &self.queue,
            image,
            [255, 255, 255, 255],
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let normal = solid_texture(&self.device, &self.queue, [128, 128, 255, 255]);
        let emissive = solid_texture(&self.device, &self.queue, [0, 0, 0, 255]);
        self.create_texture_bind_group(albedo, normal, emissive, [0.0, 0.0, 1.0, 0.0])
    }

    fn create_texture_bind_group(
        &self,
        albedo: wgpu::TextureView,
        normal: wgpu::TextureView,
        emissive: wgpu::TextureView,
        params: [f32; 4],
    ) -> wgpu::BindGroup {
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material-params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&params_buffer, 0, bytemuck::cast_slice::<f32, u8>(&params));
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&emissive),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.material_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn prepare_meshes(&self, items: &[DrawItem<'_>]) -> Result<Vec<PreparedDraw>, RenderError> {
        let mut prepared = Vec::with_capacity(items.len());
        for item in items {
            let mesh = self.cached_mesh(item.instance.mesh, item.attributes)?;
            let material_bind = item
                .material
                .map(|material| self.cached_material_bind_group(material));
            prepared.push(PreparedDraw {
                mesh,
                material_bind,
                visible: true,
            });
        }
        Ok(prepared)
    }

    fn prepare_sprite_runs(&self, sprites: &[Sprite]) -> Result<Vec<PreparedSprite>, RenderError> {
        let mut runs = Vec::with_capacity(sprites.len());
        for range in texture_runs(sprites) {
            let mesh = self.upload_sprite_run(&sprites[range.clone()])?;
            let bind = self.cached_sprite_bind_group(&sprites[range.start].texture);
            runs.push(PreparedSprite { mesh, bind });
        }
        Ok(runs)
    }

    fn upload_sprite_run(&self, run: &[Sprite]) -> Result<MeshBuffers, RenderError> {
        let (quad, quad_attributes) = &self.sprite_quad;
        let mut vertex_data: Vec<f32> = Vec::with_capacity(run.len() * 4 * VERTEX_FLOATS);
        let mut indices: Vec<u32> = Vec::with_capacity(run.len() * 6);
        for (quad_index, sprite) in run.iter().enumerate() {
            let base = (quad_index * 4) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            for corner in 0..4 {
                let world = sprite.model.transform_point(quad.positions[corner]);
                vertex_data.extend_from_slice(&[world.x, world.y, world.z]);
                vertex_data.extend_from_slice(&[0.0, 0.0, 0.0]);
                let uv = quad_attributes
                    .uvs
                    .get(corner)
                    .copied()
                    .unwrap_or([0.0, 0.0]);
                vertex_data.extend_from_slice(&uv);
                vertex_data.extend_from_slice(&[0.0, 0.0, 0.0]);
                vertex_data.extend_from_slice(&sprite.color);
            }
        }
        let vertex_bytes = bytemuck::try_cast_slice::<f32, u8>(&vertex_data)
            .map_err(|_| RenderError::BufferCast("vertex"))?;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite-run-vertices"),
            size: vertex_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&vertex_buffer, 0, vertex_bytes);
        let index_bytes = bytemuck::try_cast_slice::<u32, u8>(&indices)
            .map_err(|_| RenderError::BufferCast("index"))?;
        let index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite-run-indices"),
            size: index_bytes.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&index_buffer, 0, index_bytes);
        Ok(MeshBuffers {
            vertices: vertex_buffer,
            indices: index_buffer,
            index_count: indices.len() as u32,
            bounds: None,
        })
    }

    fn pack_globals(
        &self,
        view_proj: Mat4,
        view: Mat4,
        light: &DirectionalLight,
    ) -> [f32; GLOBAL_FLOATS] {
        let (shadow_settings, shadow_size) = {
            let shadow = lock(&self.shadow);
            (shadow.settings, shadow.current_size)
        };
        let (lights, ambient) = {
            let lighting = lock(&self.lighting);
            (lighting.lights.clone(), lighting.ambient)
        };
        pack_globals_uniform(
            view_proj,
            camera_world_position(view),
            light,
            &lights,
            ambient,
            shadow_settings,
            shadow_size,
        )
    }

    fn write_globals(
        &self,
        state: &UniformState,
        view_proj: Mat4,
        view: Mat4,
        light: &DirectionalLight,
    ) {
        let globals = self.pack_globals(view_proj, view, light);
        self.queue
            .write_buffer(&state.buffer, 0, bytemuck::cast_slice::<f32, u8>(&globals));
        let lights_2d = lock(&self.lights_2d).clone();
        let packed_lights_2d = pack_lights_2d(&lights_2d);
        self.queue.write_buffer(
            &state.lights_2d_buffer,
            0,
            bytemuck::cast_slice::<f32, u8>(&packed_lights_2d),
        );
    }

    fn prepare_draw_items(
        &self,
        items: &[DrawItem<'_>],
        sprites: &[Sprite],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
    ) -> Result<(Vec<PreparedDraw>, Vec<PreparedSprite>), RenderError> {
        let view_proj = camera.projection_matrix() * view;
        let mut prepared = self.prepare_meshes(items)?;
        let prepared_sprites = self.prepare_sprite_runs(sprites)?;
        let frustum = Frustum::from_view_projection(&view_proj);
        for (item, draw) in items.iter().zip(prepared.iter_mut()) {
            draw.visible = visible(
                draw.mesh.as_ref().and_then(|mesh| mesh.bounds.as_ref()),
                &item.instance.model,
                &frustum,
            );
        }
        let total = items.len() + sprites.len();
        let mut state = lock(&self.uniforms);
        if state.capacity < total {
            let capacity = total.max(state.capacity * 2).max(1);
            *state = create_uniforms(
                &self.device,
                &self.global_layout,
                &self.instance_layout,
                capacity,
            );
        }
        self.write_globals(&state, view_proj, view, light);

        let mut instance_data: Vec<f32> = vec![0.0; SLOT_FLOATS * total];
        for (index, item) in items.iter().enumerate() {
            let start = index * SLOT_FLOATS;
            instance_data[start..start + 16].copy_from_slice(item.instance.model.as_array());
            instance_data[start + 16..start + 20].copy_from_slice(&[
                item.instance.color[0],
                item.instance.color[1],
                item.instance.color[2],
                item.alpha,
            ]);
            instance_data[start + 20] = item.object_id as f32;
        }
        if !sprites.is_empty() {
            let start = items.len() * SLOT_FLOATS;
            instance_data[start..start + 16].copy_from_slice(Mat4::identity().as_array());
            instance_data[start + 16..start + 20].copy_from_slice(&[1.0, 1.0, 1.0, 1.0]);
        }
        if !instance_data.is_empty() {
            self.queue.write_buffer(
                &state.buffer,
                GLOBAL_SLOT,
                bytemuck::cast_slice::<f32, u8>(&instance_data),
            );
        }
        Ok((prepared, prepared_sprites))
    }

    fn encode_shadow_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
    ) {
        let shadow = lock(&self.shadow);
        let state = lock(&self.uniforms);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.shadow_pipeline);
        pass.set_bind_group(0, &state.global_bind_group, &[]);
        for index in opaque_order(items) {
            let mesh = match &prepared[index].mesh {
                Some(mesh) => mesh,
                None => continue,
            };
            pass.set_bind_group(
                1,
                &state.instance_bind_group,
                &[((GLOBAL_SLOT + UNIFORM_SLOT * index as u64) as u32)],
            );
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }

    fn draw_pass_indices(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipelines: &Pipelines,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
        indices: &[usize],
    ) {
        let state = lock(&self.uniforms);
        let shadow = lock(&self.shadow);
        pass.set_bind_group(0, &state.global_bind_group, &[]);
        pass.set_bind_group(3, &shadow.bind_group, &[]);
        for &index in indices {
            let item = &items[index];
            let mesh = match &prepared[index].mesh {
                Some(mesh) => mesh,
                None => continue,
            };
            if !prepared[index].visible {
                continue;
            }
            pass.set_bind_group(
                1,
                &state.instance_bind_group,
                &[((GLOBAL_SLOT + UNIFORM_SLOT * index as u64) as u32)],
            );
            let material_bind = prepared[index]
                .material_bind
                .as_ref()
                .unwrap_or(&self.material_bind);
            pass.set_bind_group(2, material_bind, &[]);
            let pipeline = match (item.instance.lit, item.alpha < 1.0) {
                (true, true) => &pipelines.lit_blend,
                (true, false) => &pipelines.lit,
                (false, true) => &pipelines.unlit_blend,
                (false, false) => &pipelines.unlit,
            };
            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }

    fn encode_sprite_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_views: &[&wgpu::TextureView],
        depth_view: &wgpu::TextureView,
        prepared: &[PreparedSprite],
        pipeline: &wgpu::RenderPipeline,
        instance_base: usize,
    ) {
        if prepared.is_empty() {
            return;
        }
        let state = lock(&self.uniforms);
        let shadow = lock(&self.shadow);
        let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = color_views
            .iter()
            .map(|view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })
            })
            .collect();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sprites"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &state.global_bind_group, &[]);
        pass.set_bind_group(3, &shadow.bind_group, &[]);
        let slot = (GLOBAL_SLOT + UNIFORM_SLOT * instance_base as u64) as u32;
        pass.set_bind_group(1, &state.instance_bind_group, &[slot]);
        for sprite in prepared {
            pass.set_bind_group(2, &sprite.bind, &[]);
            pass.set_vertex_buffer(0, sprite.mesh.vertices.slice(..));
            pass.set_index_buffer(sprite.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..sprite.mesh.index_count, 0, 0..1);
        }
    }

    fn read_buffer(&self, buffer: &wgpu::Buffer) -> Result<Vec<u8>, RenderError> {
        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| RenderError::Poll(error.to_string()))?;
        let mapped = receiver
            .recv()
            .map_err(|_| RenderError::Readback("map callback was dropped".to_string()))?;
        mapped.map_err(|_| RenderError::Readback("buffer map was rejected".to_string()))?;
        let view = slice
            .get_mapped_range()
            .map_err(|error| RenderError::Readback(error.to_string()))?;
        let data = view.to_vec();
        drop(view);
        buffer.unmap();
        Ok(data)
    }
}

#[derive(Clone)]
struct MeshBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    bounds: Option<Aabb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MeshKey {
    mesh: usize,
    attributes: usize,
    vertex_count: usize,
    index_count: usize,
    probe: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ImageKey {
    width: u32,
    height: u32,
    pixels: usize,
    len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MaterialKey {
    albedo: Option<ImageKey>,
    normal: Option<ImageKey>,
    emissive: Option<ImageKey>,
    metallic: u32,
    smoothness: u32,
    alpha: u32,
    blend: u8,
}

struct PreparedDraw {
    mesh: Option<MeshBuffers>,
    material_bind: Option<wgpu::BindGroup>,
    visible: bool,
}

struct PreparedSprite {
    mesh: MeshBuffers,
    bind: wgpu::BindGroup,
}

#[derive(Default)]
struct ResourceCache {
    meshes: HashMap<MeshKey, MeshBuffers>,
    materials: HashMap<MaterialKey, wgpu::BindGroup>,
    sprite_textures: HashMap<ImageKey, wgpu::BindGroup>,
    mesh_uploads: u32,
    material_uploads: u32,
}

fn image_key(image: &Image) -> ImageKey {
    ImageKey {
        width: image.width,
        height: image.height,
        pixels: image.pixels.as_ptr() as usize,
        len: image.pixels.len(),
    }
}

fn material_key(material: &Material) -> MaterialKey {
    MaterialKey {
        albedo: material.albedo.as_ref().map(image_key),
        normal: material.normal.as_ref().map(image_key),
        emissive: material.emissive.as_ref().map(image_key),
        metallic: material.metallic.clamp(0.0, 1.0).to_bits(),
        smoothness: material.smoothness.clamp(0.0, 1.0).to_bits(),
        alpha: effective_alpha(material).to_bits(),
        blend: match &material.blend {
            BlendMode::Opaque => 0,
            BlendMode::Blend => 1,
        },
    }
}

fn hash_probe(hash: u64, values: &[f32]) -> u64 {
    let mut hash = hash;
    for value in values {
        hash ^= u64::from(value.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn mesh_probe(mesh: &Mesh, attributes: Option<&MeshAttributes>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    if let Some(position) = mesh.positions.first() {
        hash = hash_probe(hash, &[position.x, position.y, position.z]);
    }
    if let Some(position) = mesh.positions.last() {
        hash = hash_probe(hash, &[position.x, position.y, position.z]);
    }
    hash ^= u64::from(mesh.indices.first().copied().unwrap_or(0));
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= u64::from(mesh.indices.last().copied().unwrap_or(0));
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    if let Some(attributes) = attributes {
        if let Some(uv) = attributes.uvs.first() {
            hash = hash_probe(hash, uv);
        }
        if let Some(uv) = attributes.uvs.last() {
            hash = hash_probe(hash, uv);
        }
    }
    hash
}

struct DrawItem<'a> {
    instance: GpuInstance<'a>,
    attributes: Option<&'a MeshAttributes>,
    material: Option<&'a Material>,
    alpha: f32,
    object_id: u32,
}

fn plain_draw_items<'a>(instances: &[GpuInstance<'a>]) -> Vec<DrawItem<'a>> {
    instances
        .iter()
        .map(|instance| DrawItem {
            instance: *instance,
            attributes: None,
            material: None,
            alpha: 1.0,
            object_id: 0,
        })
        .collect()
}

fn material_draw_items<'a>(instances: &[GpuMaterialInstance<'a>]) -> Vec<DrawItem<'a>> {
    instances
        .iter()
        .map(|instance| DrawItem {
            instance: GpuInstance {
                mesh: instance.mesh,
                model: instance.model,
                color: instance.color,
                lit: true,
            },
            attributes: instance.attributes,
            material: Some(instance.material),
            alpha: effective_alpha(instance.material),
            object_id: 0,
        })
        .collect()
}

fn capture_draw_items<'a>(instances: &[CaptureInstance<'a>]) -> Vec<DrawItem<'a>> {
    instances
        .iter()
        .enumerate()
        .map(|(index, instance)| DrawItem {
            instance: GpuInstance {
                mesh: instance.mesh,
                model: instance.model,
                color: instance.color,
                lit: instance.lit,
            },
            attributes: None,
            material: None,
            alpha: 1.0,
            object_id: (index + 1) as u32,
        })
        .collect()
}

fn opaque_order(items: &[DrawItem<'_>]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.alpha >= 1.0).then_some(index))
        .collect()
}

fn transparent_order(items: &[DrawItem<'_>], view: Mat4) -> Vec<usize> {
    let mut translucent: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.alpha < 1.0).then_some(index))
        .collect();
    translucent.sort_by(|&a, &b| {
        let depth_a = view
            .mul_mat4(&items[a].instance.model)
            .transform_point(Vec3::ZERO)
            .z;
        let depth_b = view
            .mul_mat4(&items[b].instance.model)
            .transform_point(Vec3::ZERO)
            .z;
        depth_a
            .partial_cmp(&depth_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    translucent
}

fn pack_light_uniforms(globals: &mut [f32; GLOBAL_FLOATS], lights: &[GpuLight]) {
    for (index, item) in lights.iter().take(MAX_GPU_LIGHTS).enumerate() {
        let base = 52 + index * 16;
        globals[base] = item.position.x;
        globals[base + 1] = item.position.y;
        globals[base + 2] = item.position.z;
        globals[base + 3] = item.range.max(0.0);
        let dir = item.direction.normalize();
        globals[base + 4] = dir.x;
        globals[base + 5] = dir.y;
        globals[base + 6] = dir.z;
        globals[base + 7] = match item.kind {
            GpuLightKind::Directional => 0.0,
            GpuLightKind::Point => 1.0,
            GpuLightKind::Spot => 2.0,
        };
        globals[base + 8] = item.color[0];
        globals[base + 9] = item.color[1];
        globals[base + 10] = item.color[2];
        globals[base + 11] = item.intensity.max(0.0);
        let outer_half = (item.spot_angle_deg.clamp(1.0, 179.0) * 0.5).to_radians();
        let inner_half = (item
            .inner_spot_angle_deg
            .clamp(0.0, item.spot_angle_deg.clamp(1.0, 179.0))
            * 0.5)
            .to_radians();
        let cos_outer = outer_half.cos();
        let cos_inner = inner_half.cos().max(cos_outer + 1e-4);
        globals[base + 12] = cos_outer;
        globals[base + 13] = cos_inner;
    }
}

fn pack_globals_uniform(
    view_proj: Mat4,
    camera_pos: Vec3,
    light: &DirectionalLight,
    lights: &[GpuLight],
    ambient: [f32; 3],
    shadow_settings: ShadowSettings,
    shadow_size: u32,
) -> [f32; GLOBAL_FLOATS] {
    let mut globals = [0.0f32; GLOBAL_FLOATS];
    globals[0..16].copy_from_slice(view_proj.as_array());
    let direction = light.direction.normalize();
    let light_vp = light_view_projection(direction, camera_pos, shadow_settings.distance);
    globals[16..32].copy_from_slice(light_vp.as_array());
    globals[32] = direction.x;
    globals[33] = direction.y;
    globals[34] = direction.z;
    globals[35] = light.intensity.clamp(0.0, 1.0);
    globals[36] = light.color[0];
    globals[37] = light.color[1];
    globals[38] = light.color[2];
    globals[39] = 1.0 - globals[35];
    globals[40] = camera_pos.x;
    globals[41] = camera_pos.y;
    globals[42] = camera_pos.z;
    let shadow_active = shadow_settings.enabled && shadow_size > 0 && globals[35] > 0.0;
    globals[44] = if shadow_active { 1.0 } else { 0.0 };
    globals[45] = if shadow_settings.soft { 1.0 } else { 0.0 };
    globals[46] = if shadow_size > 0 {
        1.0 / (shadow_size as f32)
    } else {
        0.0
    };
    globals[47] = 0.0015;
    let count = lights.len().min(MAX_GPU_LIGHTS);
    globals[48] = ambient[0].max(0.0);
    globals[49] = ambient[1].max(0.0);
    globals[50] = ambient[2].max(0.0);
    globals[51] = count as f32;
    pack_light_uniforms(&mut globals, lights);
    globals
}

fn create_uniforms(
    device: &wgpu::Device,
    global_layout: &wgpu::BindGroupLayout,
    instance_layout: &wgpu::BindGroupLayout,
    capacity: usize,
) -> UniformState {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene-uniforms"),
        size: GLOBAL_SLOT + UNIFORM_SLOT * (capacity as u64).max(1),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let lights_2d_buffer = create_lights_2d_buffer(device);
    let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("globals"),
        layout: global_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new((GLOBAL_FLOATS * 4) as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &lights_2d_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(LIGHTS_2D_BYTES),
                }),
            },
        ],
    });
    let instance_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("instance"),
        layout: instance_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: wgpu::BufferSize::new((INSTANCE_FLOATS * 4) as u64),
            }),
        }],
    });
    UniformState {
        buffer,
        capacity,
        global_bind_group,
        instance_bind_group,
        lights_2d_buffer,
    }
}

fn background_color(background: [f32; 3]) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(background[0]),
        g: f64::from(background[1]),
        b: f64::from(background[2]),
        a: 1.0,
    }
}

fn rgba_from_readback(data: &[u8], width: u32, height: u32, bytes_per_row: u64) -> Rgba8 {
    let mut image = Rgba8::new(width, height);
    let width = width as usize;
    let height = height as usize;
    let bytes_per_row = bytes_per_row as usize;
    let row_bytes = width * 4;
    let pixels = &mut image.pixels;
    for y in 0..height {
        let row_start = y * bytes_per_row;
        if row_start >= data.len() {
            break;
        }
        let available = ((data.len() - row_start).min(row_bytes)) / 4 * 4;
        let source = &data[row_start..row_start + available];
        let target = &mut pixels[y * row_bytes..y * row_bytes + available];
        for (pixel, texel) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
            pixel[0] = texel[0];
            pixel[1] = texel[1];
            pixel[2] = texel[2];
            pixel[3] = 255;
        }
    }
    image
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;

#[cfg(test)]
mod cull_tests {
    use super::*;
    use crate::camera::look_at;
    use crate::mesh::cube;

    #[test]
    fn prepare_draw_items_culls_offscreen_instances() -> Result<(), RenderError> {
        let Ok(renderer) = Renderer::new() else {
            eprintln!("skipping GPU test, no GPU device available");
            return Ok(());
        };
        let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
        let view = look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let light = DirectionalLight {
            direction: Vec3::new(0.0, 0.0, -1.0),
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        };
        let mesh = cube(0.5);
        let instances = [
            GpuInstance {
                mesh: &mesh,
                model: Mat4::identity(),
                color: [1.0, 1.0, 1.0],
                lit: false,
            },
            GpuInstance {
                mesh: &mesh,
                model: Mat4::from_cols_array([
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0e6,
                    1.0,
                ]),
                color: [1.0, 1.0, 1.0],
                lit: false,
            },
        ];
        let items = plain_draw_items(&instances);
        let (prepared, _) = renderer.prepare_draw_items(&items, &[], &camera, view, &light)?;
        assert!(prepared[0].visible);
        assert!(!prepared[1].visible);
        Ok(())
    }
}
