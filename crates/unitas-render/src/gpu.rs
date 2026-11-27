use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::camera::Camera;
use crate::capture::CaptureInstance;
use crate::glyphs::{
    layout_text, shape_glyph_quads, FontLibrary, GlyphAtlas, GlyphResources, Rect, SwashCache,
    TextFrame, TextPipeline, TextStyle, TexturedQuad,
};
use crate::image::{Image, Rgba8};
use crate::math::{Mat4, Vec3};
use crate::mesh::{Mesh, MeshAttributes};
use crate::model::{
    effective_alpha, BlendMode, DirectionalLight, Material, MaterialInstance, MeshInstance,
};
use crate::render_graph::{RenderGraph, RenderPassKind};
use crate::shadow::{light_view_projection, shadow_map_size, ShadowSettings};
use crate::sprite::{sprite_quad, Sprite};

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

pub use gpu_surface::present_mode_for_v_sync_count;

const VERTEX_FLOATS: usize = 11;
const VERTEX_STRIDE: u64 = 44;
const UNIFORM_SLOT: u64 = 256;
const GLOBAL_FLOATS: usize = 48;
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
}

/// The textured quad that draws a composed frame over a render target.
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
    sprite_quad: (Mesh, MeshAttributes),
    v_sync_count: AtomicU32,
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

    fn from_parts(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Result<Self, RenderError> {
        let shader = create_scene_shader(&device);
        let global_layout = create_global_layout(&device);
        let instance_layout = create_instance_layout(&device);
        let material_layout = create_material_layout(&device);
        let shadow_layout = create_shadow_layout(&device);
        let pipeline_layout = create_scene_pipeline_layout(
            &device,
            &global_layout,
            &instance_layout,
            &material_layout,
            &shadow_layout,
        );
        let shadow_pipeline_layout =
            create_shadow_pipeline_layout(&device, &global_layout, &instance_layout);
        let offscreen_pipelines =
            create_pipelines(&device, &pipeline_layout, &shader, OFFSCREEN_FORMAT);
        let capture_pipelines = create_capture_pipelines(
            &device,
            &pipeline_layout,
            &shader,
            OFFSCREEN_FORMAT,
            ID_FORMAT,
        );
        let shadow_pipeline = build_shadow_pipeline(&device, &shadow_pipeline_layout, &shader);
        let shadow_sampler = create_shadow_sampler(&device);
        let (shadow_view, shadow_bind_group) = create_shadow_resources(
            &device,
            &shadow_layout,
            &shadow_sampler,
            INITIAL_SHADOW_TEXTURE_SIZE,
        );
        let shadow = ShadowState {
            settings: ShadowSettings::disabled(),
            current_size: 0,
            texture_size: INITIAL_SHADOW_TEXTURE_SIZE,
            view: shadow_view,
            bind_group: shadow_bind_group,
        };
        let material_sampler = create_material_sampler(&device);
        let material_bind =
            create_default_material_bind(&device, &queue, &material_layout, &material_sampler);
        let uniforms = create_uniforms(&device, &global_layout, &instance_layout, 0);
        let mut fonts = FontLibrary::new();
        fonts.load_system_fonts();
        let text = TextState {
            fonts,
            cache: SwashCache::new(),
            atlas: GlyphAtlas::new(1024, 1024),
            targets: HashMap::new(),
            image_targets: HashMap::new(),
        };
        Ok(Renderer {
            instance,
            adapter,
            device,
            queue,
            global_layout,
            instance_layout,
            material_layout,
            shadow_layout,
            pipeline_layout,
            shader,
            offscreen_pipelines,
            capture_pipelines,
            shadow_pipeline,
            shadow_sampler,
            shadow: Mutex::new(shadow),
            uniforms: Mutex::new(uniforms),
            cache: Mutex::new(ResourceCache::default()),
            material_sampler,
            material_bind,
            surface: Mutex::new(None),
            text: Mutex::new(text),
            sprites: Mutex::new(Vec::new()),
            sprite_quad: sprite_quad(),
            v_sync_count: AtomicU32::new(1),
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

    /// Drops the cached mesh buffers and material bind groups; call it when
    /// the scene is reloaded.
    pub fn invalidate_resource_cache(&self) {
        let mut cache = lock(&self.cache);
        cache.meshes.clear();
        cache.materials.clear();
        cache.sprite_textures.clear();
    }

    /// The sprites the next frame draws, in the order the sprite pass draws
    /// them: the sorting layer first, the order in the layer second.
    pub fn set_sprites(&self, sprites: Vec<Sprite>) {
        let mut state = lock(&self.sprites);
        *state = sprites;
    }

    pub fn sprites(&self) -> Vec<Sprite> {
        lock(&self.sprites).clone()
    }

    /// The mesh buffers uploaded into the cache since the renderer was created.
    pub fn mesh_upload_count(&self) -> u32 {
        lock(&self.cache).mesh_uploads
    }

    /// The material bind groups created since the renderer was created.
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
        self.render_offscreen(
            instances,
            scene.camera,
            scene.view,
            scene.light,
            scene.background,
            (width, height),
        )
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

    /// The frame of the same render pass in RGBA, so a caller that composes
    /// the interface onto it does not pay a colour format round trip.
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
        let RenderScene {
            camera,
            view,
            light,
            background,
        } = scene;
        let shadows_enabled = self.prepare_shadow_map(width.max(height));
        let sprites = self.sprites();
        let graph = RenderGraph::offscreen(!overlay.is_empty(), shadows_enabled)
            .with_sprites(!sprites.is_empty());
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

        let (prepared, prepared_sprites) =
            self.prepare_draw_items(&items, &sprites, camera, view, light)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen"),
            });
        for graph_pass in graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(&mut encoder, &items, &prepared);
                }
                RenderPassKind::Opaque => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("offscreen-opaque"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &color_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(background_color(background)),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
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
                    self.draw_pass_indices(
                        &mut pass,
                        &self.offscreen_pipelines,
                        &items,
                        &prepared,
                        &opaque_order(&items),
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(&items, view);
                    if !indices.is_empty() {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("offscreen-transparent"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &color_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &depth_view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        self.draw_pass_indices(
                            &mut pass,
                            &self.offscreen_pipelines,
                            &items,
                            &prepared,
                            &indices,
                        );
                    }
                }
                RenderPassKind::Sprites => {
                    self.encode_sprite_pass(
                        &mut encoder,
                        &[&color_view],
                        &depth_view,
                        &prepared_sprites,
                        &self.offscreen_pipelines.sprite,
                        items.len(),
                    );
                }
                RenderPassKind::UiOverlay => {
                    self.draw_text_overlay(
                        &mut encoder,
                        &color_view,
                        OFFSCREEN_FORMAT,
                        (width, height),
                        1.0,
                        overlay,
                    );
                }
                RenderPassKind::CaptureIds => {}
            }
        }

        let bytes_per_row = align_up(u64::from(width) * 4, COPY_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: bytes_per_row * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color_texture,
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

    /// Draws a composed frame over a render target of the given size.
    pub fn draw_rgba_to_view(
        &self,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        image: &Rgba8,
    ) -> Result<(), RenderError> {
        if size.0 == 0 || size.1 == 0 || image.width == 0 || image.height == 0 {
            return Ok(());
        }
        let mut text = lock(&self.text);
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
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("image"),
            });
        let mut target_frame = TextFrame {
            encoder: &mut encoder,
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
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    fn draw_text_overlay(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        scale: f32,
        overlay: &[TextOverlayItem],
    ) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let mut text = lock(&self.text);
        let TextState {
            fonts,
            cache,
            atlas,
            targets,
            ..
        } = &mut *text;
        let mut quads = Vec::new();
        for item in overlay {
            if item.text.is_empty() || item.size_px <= 0.0 {
                continue;
            }
            let scaled = item.size_px * scale;
            let style = TextStyle::new(scaled, scaled, item.color);
            let layout = layout_text(fonts, &item.text, None, &style);
            let mut item_quads = shape_glyph_quads(fonts, cache, atlas, &layout);
            let dx = item.x * scale;
            let dy = item.y * scale;
            for quad in &mut item_quads {
                quad.x += dx;
                quad.y += dy;
            }
            quads.extend(item_quads);
        }
        let target = targets.entry(format).or_insert_with(|| {
            let pipeline = TextPipeline::new(&self.device, format);
            let resources = pipeline.upload_atlas(&self.device, &self.queue, atlas);
            TextTarget {
                pipeline,
                resources,
                uploaded: atlas.region_count(),
            }
        });
        if target.uploaded != atlas.region_count() {
            target.resources = target
                .pipeline
                .upload_atlas(&self.device, &self.queue, atlas);
            target.uploaded = atlas.region_count();
        }
        if quads.is_empty() {
            return;
        }
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
            &target.resources,
            &mut target_frame,
            &quads,
        );
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
        })
    }

    /// The uploaded buffers for a mesh, taken from the resource cache so a mesh
    /// drawn by many instances is uploaded once.
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

    /// The bind group for a material, taken from the resource cache so a
    /// material shared by many instances is uploaded once.
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

    fn prepare_draw_items(
        &self,
        items: &[DrawItem<'_>],
        sprites: &[Sprite],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
    ) -> Result<(Vec<PreparedDraw>, Vec<PreparedSprite>), RenderError> {
        let view_proj = camera.projection_matrix() * view;
        let mut prepared = Vec::with_capacity(items.len());
        for item in items {
            let mesh = self.cached_mesh(item.instance.mesh, item.attributes)?;
            let material_bind = item
                .material
                .map(|material| self.cached_material_bind_group(material));
            prepared.push(PreparedDraw {
                mesh,
                material_bind,
            });
        }
        let mut prepared_sprites = Vec::with_capacity(sprites.len());
        let (quad, quad_attributes) = &self.sprite_quad;
        for sprite in sprites {
            let mesh = self.cached_mesh(quad, Some(quad_attributes))?;
            let bind = self.cached_sprite_bind_group(&sprite.texture);
            prepared_sprites.push(PreparedSprite { mesh, bind });
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

        let mut globals = [0.0f32; GLOBAL_FLOATS];
        globals[0..16].copy_from_slice(view_proj.as_array());
        let direction = light.direction.normalize();
        let (shadow_settings, shadow_size) = {
            let shadow = lock(&self.shadow);
            (shadow.settings, shadow.current_size)
        };
        let light_vp = light_view_projection(
            direction,
            camera_world_position(view),
            shadow_settings.distance,
        );
        globals[16..32].copy_from_slice(light_vp.as_array());
        globals[32] = direction.x;
        globals[33] = direction.y;
        globals[34] = direction.z;
        globals[35] = light.intensity.clamp(0.0, 1.0);
        globals[36] = light.color[0];
        globals[37] = light.color[1];
        globals[38] = light.color[2];
        globals[39] = 1.0 - globals[35];
        let shadow_active = shadow_settings.enabled && shadow_size > 0 && globals[35] > 0.0;
        globals[44] = if shadow_active { 1.0 } else { 0.0 };
        globals[45] = if shadow_settings.soft { 1.0 } else { 0.0 };
        globals[46] = if shadow_size > 0 {
            1.0 / (shadow_size as f32)
        } else {
            0.0
        };
        globals[47] = 0.0015;
        self.queue
            .write_buffer(&state.buffer, 0, bytemuck::cast_slice::<f32, u8>(&globals));

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
        for (index, sprite) in sprites.iter().enumerate() {
            let start = (items.len() + index) * SLOT_FLOATS;
            instance_data[start..start + 16].copy_from_slice(sprite.model.as_array());
            instance_data[start + 16..start + 20].copy_from_slice(&sprite.color);
            instance_data[start + 20] = 0.0;
        }
        if !instance_data.is_empty() {
            self.queue.write_buffer(
                &state.buffer,
                UNIFORM_SLOT,
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
                &[((UNIFORM_SLOT * (index as u64 + 1)) as u32)],
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
            pass.set_bind_group(
                1,
                &state.instance_bind_group,
                &[((UNIFORM_SLOT * (index as u64 + 1)) as u32)],
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
        for (index, sprite) in prepared.iter().enumerate() {
            let mesh = match &sprite.mesh {
                Some(mesh) => mesh,
                None => continue,
            };
            let slot = UNIFORM_SLOT * (instance_base + index + 1) as u64;
            pass.set_bind_group(1, &state.instance_bind_group, &[slot as u32]);
            pass.set_bind_group(2, &sprite.bind, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
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
}

/// The identity of a cached mesh upload: its allocation plus a probe of its
/// contents, so a recycled allocation never serves a stale buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MeshKey {
    mesh: usize,
    attributes: usize,
    vertex_count: usize,
    index_count: usize,
    probe: u64,
}

/// The identity of a cached texture: the allocation the image owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ImageKey {
    width: u32,
    height: u32,
    pixels: usize,
    len: usize,
}

/// The identity of a cached material: its textures and its scalar parameters.
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
}

struct PreparedSprite {
    mesh: Option<MeshBuffers>,
    bind: wgpu::BindGroup,
}

/// The mesh buffers and material bind groups a scene reuses across frames.
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

fn create_uniforms(
    device: &wgpu::Device,
    global_layout: &wgpu::BindGroupLayout,
    instance_layout: &wgpu::BindGroupLayout,
    capacity: usize,
) -> UniformState {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene-uniforms"),
        size: UNIFORM_SLOT * (capacity as u64 + 1),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("globals"),
        layout: global_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: wgpu::BufferSize::new((GLOBAL_FLOATS * 4) as u64),
            }),
        }],
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
mod tests {
    use super::*;
    use crate::camera::look_at;
    use crate::mesh::cube;
    use crate::model::BlendMode;
    use crate::render_graph::RenderTargetKind;

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 128;
    const BACKGROUND: [f32; 3] = [0.05, 0.05, 0.08];

    fn camera() -> Camera {
        Camera::perspective(60.0, 1.0, 0.1, 100.0)
    }

    fn light() -> DirectionalLight {
        DirectionalLight {
            direction: Vec3::new(0.4, 0.7, 0.6),
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        }
    }

    fn view() -> Mat4 {
        look_at(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
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

    fn renderer_or_skip() -> Option<Renderer> {
        match Renderer::new() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                eprintln!("skipping GPU test, no GPU device available: {error}");
                None
            }
        }
    }

    fn grid_mesh(cells: u32) -> Mesh {
        let stride = cells + 1;
        let mut positions = Vec::with_capacity((stride * stride) as usize);
        let mut normals = Vec::with_capacity((stride * stride) as usize);
        let mut indices = Vec::with_capacity((cells * cells * 6) as usize);
        for row in 0..=cells {
            for column in 0..=cells {
                let u = column as f32 / cells as f32;
                let v = row as f32 / cells as f32;
                positions.push(Vec3::new(u * 2.0 - 1.0, 0.0, v * 2.0 - 1.0));
                normals.push(Vec3::new(0.0, 1.0, 0.0));
            }
        }
        for row in 0..cells {
            for column in 0..cells {
                let i0 = row * stride + column;
                let i1 = i0 + 1;
                let i2 = i0 + stride;
                let i3 = i2 + 1;
                indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }
        Mesh {
            positions,
            normals,
            indices,
        }
    }

    fn uv_quad(extent: f32) -> (Mesh, MeshAttributes) {
        let positions = [
            [-extent, -extent, 0.0],
            [-extent, extent, 0.0],
            [extent, extent, 0.0],
            [extent, -extent, 0.0],
        ];
        let normals = [[0.0, 0.0, -1.0]; 4];
        let indices = [0u32, 1, 2, 0, 2, 3];
        let mesh = Mesh::from_buffers(&positions, &normals, &indices);
        let uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        let attributes = mesh.attributes_with_uvs(uvs);
        (mesh, attributes)
    }

    #[test]
    fn renderer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Renderer>();
    }

    struct OverlayInk {
        changed: usize,
        min_x: u32,
        max_x: u32,
        min_y: u32,
        max_y: u32,
        columns: Vec<bool>,
    }

    fn overlay_ink(without: &Image, with: &Image, size: (u32, u32)) -> OverlayInk {
        let (width, height) = size;
        let mut ink = OverlayInk {
            changed: 0,
            min_x: width,
            max_x: 0,
            min_y: height,
            max_y: 0,
            columns: vec![false; width as usize],
        };
        for y in 0..height {
            for x in 0..width {
                if without.get_pixel(x, y) != with.get_pixel(x, y) {
                    ink.changed += 1;
                    ink.min_x = ink.min_x.min(x);
                    ink.max_x = ink.max_x.max(x);
                    ink.min_y = ink.min_y.min(y);
                    ink.max_y = ink.max_y.max(y);
                    ink.columns[x as usize] = true;
                }
            }
        }
        ink
    }

    fn ink_coverage(ink: &OverlayInk) -> f32 {
        let area = ((ink.max_x - ink.min_x + 1) * (ink.max_y - ink.min_y + 1)) as f32;
        ink.changed as f32 / area
    }

    #[test]
    fn image_blit_draws_the_image_over_the_target() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let width = 4u32;
        let height = 4u32;
        let mut image = Image::new(width, height);
        for y in 0..height {
            for x in 0..width {
                image.set_pixel(x, y, [x as u8 * 60, y as u8 * 60, 11]);
            }
        }
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let rgba = image.to_rgba8();
        renderer.draw_rgba_to_view(
            &view,
            wgpu::TextureFormat::Rgba8Unorm,
            (width, height),
            &rgba,
        )?;
        let bytes_per_row = align_up(u64::from(width) * 4, COPY_ROW_ALIGNMENT);
        let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image-readback"),
            size: bytes_per_row * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("image-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
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
        renderer.queue.submit(Some(encoder.finish()));
        let data = renderer.read_buffer(&readback)?;
        for y in 0..height {
            for x in 0..width {
                let offset = y as usize * bytes_per_row as usize + x as usize * 4;
                let pixel = [data[offset], data[offset + 1], data[offset + 2]];
                assert_eq!(
                    pixel,
                    image.get_pixel(x, y),
                    "the blit changed the pixel at {x},{y}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn text_overlay_changes_pixels_inside_text_bounds_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        if !renderer.has_overlay_fonts() {
            return Err("the renderer loaded no fonts, so the overlay cannot be verified".into());
        }
        let overlay = vec![TextOverlayItem {
            text: "Ag".to_string(),
            x: 10.0,
            y: 10.0,
            size_px: 24.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }];
        let without = renderer.render_offscreen(
            &[],
            &camera(),
            view(),
            &light(),
            BACKGROUND,
            (WIDTH, HEIGHT),
        )?;
        let scene = RenderScene {
            camera: &camera(),
            view: view(),
            light: &light(),
            background: BACKGROUND,
        };
        let with = renderer.render_offscreen_with_overlay(&[], scene, (WIDTH, HEIGHT), &overlay)?;
        let margin = 24.0f32;
        let bound_x = 10.0f32 - margin;
        let bound_y = 10.0f32 - margin;
        let bound_w = 24.0 * 3.0 + margin;
        let bound_h = 24.0 * 2.0 + margin;
        let mut changed = 0usize;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if without.get_pixel(x, y) != with.get_pixel(x, y) {
                    changed += 1;
                    assert!(
                        (x as f32) >= bound_x
                            && (x as f32) < bound_x + bound_w
                            && (y as f32) >= bound_y
                            && (y as f32) < bound_y + bound_h,
                        "changed pixel ({x}, {y}) outside text bounds"
                    );
                }
            }
        }
        assert!(changed > 0, "overlay produced no changed pixels");
        let ink = overlay_ink(&without, &with, (WIDTH, HEIGHT));
        let coverage = ink_coverage(&ink);
        assert!(
            coverage < 0.7,
            "overlay fills {coverage} of its bounds, which means solid blocks instead of letterforms"
        );
        assert_eq!(
            without.get_pixel(WIDTH - 1, HEIGHT - 1),
            with.get_pixel(WIDTH - 1, HEIGHT - 1)
        );
        Ok(())
    }

    #[test]
    fn text_overlay_renders_letterforms_not_solid_blocks() -> Result<(), Box<dyn std::error::Error>>
    {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        if !renderer.has_overlay_fonts() {
            return Err(
                "the renderer loaded no fonts, so the HUD phrase cannot be verified".into(),
            );
        }
        let size = (256u32, 96u32);
        let overlay = vec![TextOverlayItem {
            text: "Score: 0/4".to_string(),
            x: 16.0,
            y: 16.0,
            size_px: 28.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }];
        let without =
            renderer.render_offscreen(&[], &camera(), view(), &light(), BACKGROUND, size)?;
        let scene = RenderScene {
            camera: &camera(),
            view: view(),
            light: &light(),
            background: BACKGROUND,
        };
        let with = renderer.render_offscreen_with_overlay(&[], scene, size, &overlay)?;
        let ink = overlay_ink(&without, &with, size);
        assert!(ink.changed > 0, "the HUD phrase produced no changed pixels");
        let coverage = ink_coverage(&ink);
        assert!(
            coverage < 0.7,
            "the HUD phrase fills {coverage} of its bounds, which means solid blocks instead of letterforms"
        );
        let columns = &ink.columns[ink.min_x as usize..=ink.max_x as usize];
        assert!(
            columns.iter().any(|column| *column),
            "no text column carries ink"
        );
        assert!(
            columns.iter().any(|column| !*column),
            "every text column carries ink, which means solid blocks instead of letterforms"
        );
        Ok(())
    }

    #[test]
    fn unlit_instance_renders_flat_colour() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let cube_mesh = cube(1.0);
        let instances = vec![GpuInstance {
            mesh: &cube_mesh,
            model: model(Vec3::ZERO, 1.0),
            color: [0.2, 0.4, 0.8],
            lit: false,
        }];
        let image = renderer.render_offscreen_with(
            &instances,
            &camera(),
            view(),
            &light(),
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        assert_eq!(image.get_pixel(16, 16), [51, 102, 204]);
        Ok(())
    }

    #[test]
    fn lit_instance_is_shaded_by_the_directional_light() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let cube_mesh = cube(1.0);
        let lit = vec![GpuInstance {
            mesh: &cube_mesh,
            model: model(Vec3::ZERO, 1.0),
            color: [1.0, 1.0, 1.0],
            lit: true,
        }];
        let unlit = vec![GpuInstance {
            mesh: &cube_mesh,
            model: model(Vec3::ZERO, 1.0),
            color: [1.0, 1.0, 1.0],
            lit: false,
        }];
        let lit_image = renderer.render_offscreen_with(
            &lit,
            &camera(),
            view(),
            &light(),
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        let unlit_image = renderer.render_offscreen_with(
            &unlit,
            &camera(),
            view(),
            &light(),
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        let lit_pixel = lit_image.get_pixel(16, 16);
        let unlit_pixel = unlit_image.get_pixel(16, 16);
        assert_eq!(unlit_pixel, [255, 255, 255]);
        assert!(
            lit_pixel[0] < unlit_pixel[0],
            "lit pixel {lit_pixel:?} is not darker than unlit pixel {unlit_pixel:?}"
        );
        Ok(())
    }

    #[test]
    fn mesh_above_the_16_bit_vertex_limit_renders() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let mesh = grid_mesh(300);
        assert!(mesh.positions.len() > 65535);
        let instances = vec![GpuInstance {
            mesh: &mesh,
            model: Mat4::identity(),
            color: [0.8, 0.8, 0.8],
            lit: true,
        }];
        let grid_view = look_at(
            Vec3::new(0.0, 2.0, -3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let image = renderer.render_offscreen_with(
            &instances,
            &camera(),
            grid_view,
            &light(),
            [0.0, 0.0, 0.0],
            (64, 64),
        )?;
        let covered = image
            .pixels
            .iter()
            .filter(|pixel| **pixel != [0, 0, 0])
            .count();
        assert!(
            covered > 0,
            "mesh with {} vertices produced no covered pixels",
            mesh.positions.len()
        );
        Ok(())
    }

    #[test]
    fn imported_mesh_buffers_round_trip_and_render_offscreen(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let positions: [[f32; 3]; 4] = [
            [-0.8, -0.8, 0.0],
            [-0.8, 0.8, 0.0],
            [0.8, 0.8, 0.0],
            [0.8, -0.8, 0.0],
        ];
        let normals: [[f32; 3]; 4] = [
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
        ];
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let imported = Mesh::from_buffers(&positions, &normals, &indices);
        assert_eq!(imported.positions.len(), 4);
        assert_eq!(imported.normals.len(), 4);
        assert_eq!(imported.indices, indices);
        let instances = vec![MeshInstance {
            mesh: &imported,
            model: Mat4::identity(),
            color: [0.3, 0.9, 0.4],
        }];
        let scene = RenderScene {
            camera: &camera(),
            view: view(),
            light: &light(),
            background: [0.0, 0.0, 0.0],
        };
        let gpu = renderer.render_offscreen_scene(&instances, scene, 64, 64)?;
        assert_ne!(gpu.get_pixel(32, 32), [0, 0, 0]);
        Ok(())
    }

    #[test]
    fn textured_quad_samples_expected_texels() -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let (mesh, attributes) = uv_quad(2.0);
        let mut texture = Image::new(2, 2);
        texture.pixels[0] = [255, 0, 0];
        texture.pixels[1] = [0, 255, 0];
        texture.pixels[2] = [0, 0, 255];
        texture.pixels[3] = [255, 255, 255];
        let material = Material {
            albedo: Some(texture),
            ..Material::plain()
        };
        let instances = [GpuMaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: Mat4::identity(),
            color: [1.0, 1.0, 1.0],
            material: &material,
        }];
        let unlit_ambient = DirectionalLight {
            direction: Vec3::new(0.4, 0.7, 0.6),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        };
        let image = renderer.render_offscreen_materials(
            &instances,
            &camera(),
            view(),
            &unlit_ambient,
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        let top_left = image.get_pixel(6, 6);
        assert!(top_left[0] > 200 && top_left[1] < 40 && top_left[2] < 40);
        let top_right = image.get_pixel(25, 6);
        assert!(top_right[1] > 200 && top_right[0] < 40 && top_right[2] < 40);
        let bottom_left = image.get_pixel(6, 25);
        assert!(bottom_left[2] > 200 && bottom_left[0] < 40 && bottom_left[1] < 40);
        let bottom_right = image.get_pixel(25, 25);
        assert!(bottom_right[0] > 200 && bottom_right[1] > 200 && bottom_right[2] > 200);
        Ok(())
    }

    #[test]
    fn emissive_is_visible_with_a_black_albedo_and_no_light(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let (mesh, attributes) = uv_quad(1.0);
        let mut emissive = Image::new(1, 1);
        emissive.pixels[0] = [255, 32, 0];
        let material = Material {
            albedo: None,
            emissive: Some(emissive),
            ..Material::plain()
        };
        let instances = [GpuMaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: Mat4::identity(),
            color: [0.0, 0.0, 0.0],
            material: &material,
        }];
        let unlit_ambient = DirectionalLight {
            direction: Vec3::new(0.4, 0.7, 0.6),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        };
        let image = renderer.render_offscreen_materials(
            &instances,
            &camera(),
            view(),
            &unlit_ambient,
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        let pixel = image.get_pixel(16, 16);
        assert!(
            (i32::from(pixel[0]) - 255).abs() <= 1 && (i32::from(pixel[1]) - 32).abs() <= 1,
            "emissive pixel {pixel:?} does not match the emissive texture"
        );
        Ok(())
    }

    #[test]
    fn render_graph_executes_passes_in_order_and_produces_expected_frame(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let offscreen_graph = RenderGraph::offscreen(true, false);
        assert_eq!(
            offscreen_graph.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::UiOverlay,
            ]
        );
        assert!(offscreen_graph
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::OffscreenColorDepth));

        let surface_graph = RenderGraph::surface(false, false);
        assert_eq!(
            surface_graph.pass_order(),
            vec![RenderPassKind::Opaque, RenderPassKind::Transparent]
        );
        assert!(surface_graph
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::SurfaceColorDepth));

        let capture_graph = RenderGraph::capture(false);
        assert_eq!(
            capture_graph.pass_order(),
            vec![
                RenderPassKind::Opaque,
                RenderPassKind::Transparent,
                RenderPassKind::CaptureIds,
            ]
        );
        assert!(capture_graph
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::CaptureColorIdDepth));

        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        let positions: [[f32; 3]; 4] = [
            [-1.5, -1.5, 0.0],
            [-1.5, 1.5, 0.0],
            [1.5, 1.5, 0.0],
            [1.5, -1.5, 0.0],
        ];
        let normals: [[f32; 3]; 4] = [[0.0, 0.0, -1.0]; 4];
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let quad = Mesh::from_buffers(&positions, &normals, &indices);
        let opaque_mat = Material::plain();
        let transparent_mat = Material {
            alpha: 0.5,
            blend: BlendMode::Blend,
            ..Material::plain()
        };
        let instances = [
            GpuMaterialInstance {
                mesh: &quad,
                attributes: None,
                model: model(Vec3::new(0.0, 0.0, -1.0), 1.0),
                color: [1.0, 0.0, 0.0],
                material: &transparent_mat,
            },
            GpuMaterialInstance {
                mesh: &quad,
                attributes: None,
                model: model(Vec3::ZERO, 1.0),
                color: [0.0, 0.0, 1.0],
                material: &opaque_mat,
            },
        ];
        let unlit_ambient = DirectionalLight {
            direction: Vec3::new(0.0, 0.0, -1.0),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        };
        let frame = renderer.render_offscreen_materials(
            &instances,
            &camera(),
            view(),
            &unlit_ambient,
            [0.0, 0.0, 0.0],
            (32, 32),
        )?;
        let center = frame.get_pixel(16, 16);
        assert!(
            (i32::from(center[0]) - 128).abs() <= 2
                && center[1] == 0
                && (i32::from(center[2]) - 128).abs() <= 2,
            "expected blended center near [128, 0, 128], got {center:?}"
        );
        Ok(())
    }

    #[test]
    fn directional_shadow_darkens_occluded_surface_and_tracks_map_size(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return Ok(()),
        };
        assert_eq!(renderer.shadow_map_size(), 0);
        let mesh = cube(1.0);
        let instances = [
            MeshInstance {
                mesh: &mesh,
                model: model(Vec3::new(0.0, 0.0, 0.0), 4.0),
                color: [1.0, 1.0, 1.0],
            },
            MeshInstance {
                mesh: &mesh,
                model: model(Vec3::new(0.0, 3.0, 0.0), 1.5),
                color: [1.0, 1.0, 1.0],
            },
        ];
        let shadow_view = look_at(
            Vec3::new(0.0, 3.0, -6.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let sun = DirectionalLight {
            direction: Vec3::new(0.0, 1.0, 0.0),
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        };
        let without_shadows = renderer.render_offscreen(
            &instances,
            &camera(),
            shadow_view,
            &sun,
            [0.0, 0.0, 0.0],
            (64, 64),
        )?;
        assert_eq!(renderer.shadow_map_size(), 0);

        renderer.set_shadow_settings(ShadowSettings {
            enabled: true,
            soft: true,
            resolution_index: 2,
            distance: 20.0,
        });
        let with_shadows = renderer.render_offscreen(
            &instances,
            &camera(),
            shadow_view,
            &sun,
            [0.0, 0.0, 0.0],
            (64, 64),
        )?;
        assert_eq!(renderer.shadow_map_size(), shadow_map_size(64, 2, 2048));

        renderer.set_shadow_settings(ShadowSettings::disabled());
        assert_eq!(renderer.shadow_map_size(), 0);

        let unshadowed_ground = without_shadows.get_pixel(32, 32);
        let shadowed_ground = with_shadows.get_pixel(32, 32);
        assert!(
            shadowed_ground[0] < unshadowed_ground[0],
            "expected shadowed pixel {shadowed_ground:?} darker than unshadowed {unshadowed_ground:?}"
        );
        assert!(
            shadowed_ground[0] > 0,
            "expected ambient illumination on shadowed pixel, got {shadowed_ground:?}"
        );
        Ok(())
    }

    #[test]
    fn present_mode_for_v_sync_count_selects_vsync_or_no_vsync_fallback() {
        let all_modes = [
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::AutoNoVsync,
            wgpu::PresentMode::AutoVsync,
        ];
        assert_eq!(
            present_mode_for_v_sync_count(0, &all_modes),
            wgpu::PresentMode::AutoNoVsync
        );
        assert_eq!(
            present_mode_for_v_sync_count(
                0,
                &[
                    wgpu::PresentMode::Fifo,
                    wgpu::PresentMode::Immediate,
                    wgpu::PresentMode::Mailbox
                ]
            ),
            wgpu::PresentMode::Immediate
        );
        assert_eq!(
            present_mode_for_v_sync_count(
                0,
                &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox]
            ),
            wgpu::PresentMode::Mailbox
        );
        assert_eq!(
            present_mode_for_v_sync_count(0, &[wgpu::PresentMode::Fifo]),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            present_mode_for_v_sync_count(1, &all_modes),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            present_mode_for_v_sync_count(
                2,
                &[wgpu::PresentMode::AutoVsync, wgpu::PresentMode::Immediate]
            ),
            wgpu::PresentMode::AutoVsync
        );
    }

    #[test]
    fn renderer_tracks_shadow_settings_and_v_sync_count() {
        let renderer = match renderer_or_skip() {
            Some(renderer) => renderer,
            None => return,
        };
        assert_eq!(renderer.shadow_settings(), ShadowSettings::disabled());
        assert_eq!(renderer.v_sync_count(), 1);
        let updated = ShadowSettings {
            enabled: true,
            soft: false,
            resolution_index: 1,
            distance: 70.0,
        };
        renderer.set_shadow_settings(updated);
        assert_eq!(renderer.shadow_settings(), updated);
        renderer.set_v_sync_count(0);
        assert_eq!(renderer.v_sync_count(), 0);
        renderer.set_v_sync_count(2);
        assert_eq!(renderer.v_sync_count(), 2);
    }
}
