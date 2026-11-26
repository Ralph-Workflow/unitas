use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use crate::camera::Camera;
use crate::capture::{CaptureFrame, CaptureInstance, CaptureObject, ScreenBounds};
use crate::glyphs::{
    layout_text, shape_glyph_quads, FontLibrary, GlyphAtlas, GlyphResources, Rect, SwashCache,
    TextFrame, TextPipeline, TextStyle,
};
use crate::image::{Image, Rgba8};
use crate::math::{Mat4, Vec3};
use crate::mesh::{Mesh, MeshAttributes};
use crate::model::{effective_alpha, DirectionalLight, Material, MaterialInstance, MeshInstance};
use crate::render_graph::{RenderGraph, RenderPassKind};

const VERTEX_FLOATS: usize = 11;
const VERTEX_STRIDE: u64 = 44;
const UNIFORM_SLOT: u64 = 256;
const GLOBAL_FLOATS: usize = 28;
const INSTANCE_FLOATS: usize = 24;
const SLOT_FLOATS: usize = (UNIFORM_SLOT / 4) as usize;
const COPY_ROW_ALIGNMENT: u64 = 256;
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub const GPU_CPU_MEAN_ERROR_TOLERANCE: f32 = 4.0;

const SHADER_SOURCE: &str = r"
struct Globals {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    camera_pos: vec4<f32>,
};

struct InstanceData {
    model: mat4x4<f32>,
    color: vec4<f32>,
    flags: vec4<f32>,
};

struct MaterialParams {
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> inst: InstanceData;
@group(2) @binding(0) var albedo_tex: texture_2d<f32>;
@group(2) @binding(1) var normal_tex: texture_2d<f32>;
@group(2) @binding(2) var emissive_tex: texture_2d<f32>;
@group(2) @binding(3) var material_sampler: sampler;
@group(2) @binding(4) var<uniform> material: MaterialParams;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) world_pos: vec3<f32>,
    @location(5) alpha: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
) -> VertexOut {
    var out: VertexOut;
    let world = inst.model * vec4<f32>(position, 1.0);
    out.clip_position = globals.view_proj * world;
    let transformed = (inst.model * vec4<f32>(normal, 0.0)).xyz;
    let len = length(transformed);
    out.normal = select(vec3<f32>(0.0, 0.0, 0.0), transformed / len, len > 1e-8);
    out.tangent = (inst.model * vec4<f32>(tangent, 0.0)).xyz;
    out.color = inst.color.rgb;
    out.uv = uv;
    out.world_pos = world.xyz;
    out.alpha = inst.color.a;
    return out;
}

fn surface_color(vs_out: VertexOut) -> vec3<f32> {
    let n_len = length(vs_out.normal);
    var n = select(vec3<f32>(0.0, 0.0, 0.0), vs_out.normal / n_len, n_len > 1e-8);
    let t_len = length(vs_out.tangent);
    if (t_len > 1e-4 && n_len > 1e-8) {
        var t = vs_out.tangent / t_len;
        t = normalize(t - n * dot(n, t));
        let b = cross(n, t);
        let nm = textureSample(normal_tex, material_sampler, vs_out.uv).xyz * 2.0 - 1.0;
        n = normalize(t * nm.x + b * nm.y + n * nm.z);
    }
    let albedo = textureSample(albedo_tex, material_sampler, vs_out.uv).rgb * vs_out.color;
    let emissive = textureSample(emissive_tex, material_sampler, vs_out.uv).rgb;
    let metallic = material.params.x;
    let smoothness = material.params.y;
    let lambert = max(dot(n, globals.light_dir.xyz), 0.0);
    let view_dir = normalize(globals.camera_pos.xyz - vs_out.world_pos);
    let half_dir = normalize(globals.light_dir.xyz + view_dir);
    let shininess = 8.0 + smoothness * 248.0;
    let specular = pow(max(dot(n, half_dir), 0.0), shininess) * metallic * smoothness;
    let diffuse = albedo * (1.0 - metallic);
    let shade = globals.light_color.w * albedo
        + globals.light_color.rgb * (lambert * diffuse + specular) * globals.light_dir.w;
    return shade + emissive;
}

@fragment
fn fs_lit(vs_out: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(surface_color(vs_out), vs_out.alpha);
}

@fragment
fn fs_unlit(vs_out: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(vs_out.color, vs_out.alpha);
}

struct CaptureOut {
    @location(0) color: vec4<f32>,
    @location(1) id: u32,
};

@fragment
fn fs_capture_lit(vs_out: VertexOut) -> CaptureOut {
    var out: CaptureOut;
    out.color = vec4<f32>(surface_color(vs_out), 1.0);
    out.id = u32(round(inst.flags.x));
    return out;
}

@fragment
fn fs_capture_unlit(vs_out: VertexOut) -> CaptureOut {
    var out: CaptureOut;
    out.color = vec4<f32>(vs_out.color, 1.0);
    out.id = u32(round(inst.flags.x));
    return out;
}
";

const ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 12,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 24,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 32,
        shader_location: 3,
    },
];

const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: VERTEX_STRIDE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &ATTRIBUTES,
};

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
}

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    global_layout: wgpu::BindGroupLayout,
    instance_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    offscreen_pipelines: Pipelines,
    capture_pipelines: Pipelines,
    uniforms: Mutex<UniformState>,
    material_sampler: wgpu::Sampler,
    material_bind: wgpu::BindGroup,
    surface: Mutex<Option<SurfaceState>>,
    text: Mutex<TextState>,
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
        format: wgpu::TextureFormat::Rgba8Unorm,
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

fn color_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::REPLACE),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn blend_color_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn id_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn create_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> Pipelines {
    let targets = [Some(color_target(format))];
    let blend_targets = [Some(blend_color_target(format))];
    build_pipelines(
        device,
        layout,
        shader,
        "fs_lit",
        "fs_unlit",
        &targets,
        &blend_targets,
    )
}

fn create_capture_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    id_format: wgpu::TextureFormat,
) -> Pipelines {
    let targets = [Some(color_target(color_format)), Some(id_target(id_format))];
    let blend_targets = [
        Some(blend_color_target(color_format)),
        Some(id_target(id_format)),
    ];
    build_pipelines(
        device,
        layout,
        shader,
        "fs_capture_lit",
        "fs_capture_unlit",
        &targets,
        &blend_targets,
    )
}

fn build_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    lit_entry_point: &str,
    unlit_entry_point: &str,
    targets: &[Option<wgpu::ColorTargetState>],
    blend_targets: &[Option<wgpu::ColorTargetState>],
) -> Pipelines {
    let lit = build_pipeline(
        device,
        layout,
        shader,
        lit_entry_point,
        targets,
        "lit",
        true,
    );
    let unlit = build_pipeline(
        device,
        layout,
        shader,
        unlit_entry_point,
        targets,
        "unlit",
        true,
    );
    let lit_blend = build_pipeline(
        device,
        layout,
        shader,
        lit_entry_point,
        blend_targets,
        "lit-blend",
        false,
    );
    let unlit_blend = build_pipeline(
        device,
        layout,
        shader,
        unlit_entry_point,
        blend_targets,
        "unlit-blend",
        false,
    );
    Pipelines {
        lit,
        unlit,
        lit_blend,
        unlit_blend,
    }
}

fn build_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &str,
    targets: &[Option<wgpu::ColorTargetState>],
    label: &'static str,
    depth_write: bool,
) -> wgpu::RenderPipeline {
    let depth_stencil = wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(depth_write),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(VERTEX_LAYOUT)],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth_stencil),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets,
        }),
        multiview_mask: None,
        cache: None,
    })
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((GLOBAL_FLOATS * 4) as u64),
                },
                count: None,
            }],
        });
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("instance"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new((INSTANCE_FLOATS * 4) as u64),
                },
                count: None,
            }],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[
                Some(&global_layout),
                Some(&instance_layout),
                Some(&material_layout),
            ],
            immediate_size: 0,
        });
        let offscreen_pipelines =
            create_pipelines(&device, &pipeline_layout, &shader, OFFSCREEN_FORMAT);
        let capture_pipelines = create_capture_pipelines(
            &device,
            &pipeline_layout,
            &shader,
            OFFSCREEN_FORMAT,
            ID_FORMAT,
        );
        let material_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let default_albedo = solid_texture(&device, &queue, [255, 255, 255, 255]);
        let default_normal = solid_texture(&device, &queue, [128, 128, 255, 255]);
        let default_emissive = solid_texture(&device, &queue, [0, 0, 0, 255]);
        let default_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material-default"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &default_params,
            0,
            bytemuck::cast_slice::<f32, u8>(&[0.0f32, 0.0, 1.0, 0.0]),
        );
        let material_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material-default"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&default_albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&default_normal),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&default_emissive),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&material_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: default_params.as_entire_binding(),
                },
            ],
        });
        let uniforms = create_uniforms(&device, &global_layout, &instance_layout, 0);
        let mut fonts = FontLibrary::new();
        fonts.load_system_fonts();
        let text = TextState {
            fonts,
            cache: SwashCache::new(),
            atlas: GlyphAtlas::new(1024, 1024),
            targets: HashMap::new(),
        };
        Ok(Renderer {
            instance,
            adapter,
            device,
            queue,
            global_layout,
            instance_layout,
            material_layout,
            pipeline_layout,
            shader,
            offscreen_pipelines,
            capture_pipelines,
            uniforms: Mutex::new(uniforms),
            material_sampler,
            material_bind,
            surface: Mutex::new(None),
            text: Mutex::new(text),
        })
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
        self.render_offscreen_items(material_draw_items(instances), scene, size, &[])
    }

    fn render_offscreen_items(
        &self,
        items: Vec<DrawItem<'_>>,
        scene: RenderScene<'_>,
        size: (u32, u32),
        overlay: &[TextOverlayItem],
    ) -> Result<Image, RenderError> {
        let (width, height) = size;
        if width == 0 || height == 0 {
            return Ok(Image::new(width, height));
        }
        let RenderScene {
            camera,
            view,
            light,
            background,
        } = scene;
        let graph = RenderGraph::offscreen(!overlay.is_empty());
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

        let prepared = self.prepare_draw_items(&items, camera, view, light)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("offscreen"),
            });
        for graph_pass in graph.passes() {
            match graph_pass.kind {
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
        Ok(image_from_rgba(&data, width, height, bytes_per_row))
    }

    pub fn render_offscreen_with_ids(
        &self,
        instances: &[CaptureInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
        size: (u32, u32),
    ) -> Result<CaptureFrame, RenderError> {
        let (width, height) = size;
        if width == 0 || height == 0 {
            return Ok(CaptureFrame {
                image: Rgba8::new(width, height),
                id_map: Vec::new(),
                objects: Vec::new(),
            });
        }
        let graph = RenderGraph::capture();
        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture-color"),
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
        let id_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture-ids"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ID_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth_texture = self.create_depth_texture(width, height);
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id_view = id_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let items = capture_draw_items(instances);
        let prepared = self.prepare_draw_items(&items, camera, view, light)?;
        let bytes_per_row = align_up(u64::from(width) * 4, COPY_ROW_ALIGNMENT);
        let color_readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture-color-readback"),
            size: bytes_per_row * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let id_readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture-id-readback"),
            size: bytes_per_row * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("capture"),
            });
        for graph_pass in graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Opaque => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("capture-opaque"),
                        color_attachments: &[
                            Some(wgpu::RenderPassColorAttachment {
                                view: &color_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(background_color(background)),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                            Some(wgpu::RenderPassColorAttachment {
                                view: &id_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store,
                                },
                            }),
                        ],
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
                        &self.capture_pipelines,
                        &items,
                        &prepared,
                        &opaque_order(&items),
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(&items, view);
                    if !indices.is_empty() {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("capture-transparent"),
                            color_attachments: &[
                                Some(wgpu::RenderPassColorAttachment {
                                    view: &color_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                }),
                                Some(wgpu::RenderPassColorAttachment {
                                    view: &id_view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                }),
                            ],
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
                            &self.capture_pipelines,
                            &items,
                            &prepared,
                            &indices,
                        );
                    }
                }
                RenderPassKind::CaptureIds => {
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &color_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &color_readback,
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
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &id_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &id_readback,
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
                }
                RenderPassKind::UiOverlay => {}
            }
        }
        self.queue.submit(Some(encoder.finish()));
        let color_data = self.read_buffer(&color_readback)?;
        let id_data = self.read_buffer(&id_readback)?;
        Ok(capture_frame_from_readback(
            &color_data,
            &id_data,
            bytes_per_row,
            width,
            height,
            instances,
        ))
    }

    pub fn configure_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
    ) -> Result<wgpu::SurfaceConfiguration, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidSize(width, height));
        }
        let mut configuration = match surface.get_default_config(&self.adapter, width, height) {
            Some(configuration) => configuration,
            None => {
                return Err(RenderError::SurfaceUnavailable(
                    "no supported surface configuration".to_string(),
                ))
            }
        };
        configuration.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface.configure(&self.device, &configuration);
        let pipelines = create_pipelines(
            &self.device,
            &self.pipeline_layout,
            &self.shader,
            configuration.format,
        );
        let depth = self.create_depth_texture(width, height);
        let mut state = lock(&self.surface);
        *state = Some(SurfaceState {
            pipelines,
            format: configuration.format,
            config_size: (width, height),
            depth,
            depth_size: (width, height),
        });
        Ok(configuration)
    }

    pub fn render_to_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[MeshInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
    ) -> Result<(), RenderError> {
        let converted: Vec<GpuInstance<'_>> = instances.iter().map(GpuInstance::from).collect();
        self.render_to_surface_with(surface, &converted, camera, view, light, background)
    }

    pub fn render_to_surface_scene(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[MeshInstance<'_>],
        scene: RenderScene<'_>,
    ) -> Result<(), RenderError> {
        self.render_to_surface(
            surface,
            instances,
            scene.camera,
            scene.view,
            scene.light,
            scene.background,
        )
    }

    pub fn render_to_surface_with(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[GpuInstance<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
        background: [f32; 3],
    ) -> Result<(), RenderError> {
        let scene = RenderScene {
            camera,
            view,
            light,
            background,
        };
        self.render_to_surface_with_overlay(surface, instances, scene, &[])
    }

    pub fn render_to_surface_with_overlay(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[GpuInstance<'_>],
        scene: RenderScene<'_>,
        overlay: &[TextOverlayItem],
    ) -> Result<(), RenderError> {
        let RenderScene {
            camera,
            view,
            light,
            background,
        } = scene;
        let mut state = lock(&self.surface);
        let surface_state = match state.as_mut() {
            Some(surface_state) => surface_state,
            None => return Err(RenderError::SurfaceNotConfigured),
        };
        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => return Err(RenderError::SurfaceUnavailable(format!("{other:?}"))),
        };
        let size = (
            surface_texture.texture.width(),
            surface_texture.texture.height(),
        );
        if surface_state.depth_size != size {
            surface_state.depth = self.create_depth_texture(size.0, size.1);
            surface_state.depth_size = size;
        }
        let mut camera = *camera;
        camera.aspect = size.0 as f32 / size.1 as f32;
        let color_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = surface_state
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        let items = plain_draw_items(instances);
        let prepared = self.prepare_draw_items(&items, &camera, view, light)?;
        let graph = RenderGraph::surface(!overlay.is_empty());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface"),
            });
        for graph_pass in graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Opaque => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("surface-opaque"),
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
                        &surface_state.pipelines,
                        &items,
                        &prepared,
                        &opaque_order(&items),
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(&items, view);
                    if !indices.is_empty() {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("surface-transparent"),
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
                            &surface_state.pipelines,
                            &items,
                            &prepared,
                            &indices,
                        );
                    }
                }
                RenderPassKind::UiOverlay => {
                    let scale = size.0 as f32 / surface_state.config_size.0 as f32;
                    let format = surface_state.format;
                    self.draw_text_overlay(&mut encoder, &color_view, format, size, scale, overlay);
                }
                RenderPassKind::CaptureIds => {}
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(surface_texture);
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
        material: Option<&Material>,
    ) -> Result<Option<MeshBuffers>, RenderError> {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            return Ok(None);
        }
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
        let material_bind = material.map(|mat| self.create_material_bind_group(mat));
        Ok(Some(MeshBuffers {
            vertices: vertex_buffer,
            indices: index_buffer,
            index_count: mesh.indices.len() as u32,
            material_bind,
        }))
    }

    fn create_material_bind_group(&self, material: &Material) -> wgpu::BindGroup {
        let albedo = match &material.albedo {
            Some(image) => image_texture(&self.device, &self.queue, image, [255, 255, 255, 255]),
            None => solid_texture(&self.device, &self.queue, [255, 255, 255, 255]),
        };
        let normal = match &material.normal {
            Some(image) => image_texture(&self.device, &self.queue, image, [128, 128, 255, 255]),
            None => solid_texture(&self.device, &self.queue, [128, 128, 255, 255]),
        };
        let emissive = match &material.emissive {
            Some(image) => image_texture(&self.device, &self.queue, image, [0, 0, 0, 255]),
            None => solid_texture(&self.device, &self.queue, [0, 0, 0, 255]),
        };
        let params = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material-params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let metallic = material.metallic.clamp(0.0, 1.0);
        let smoothness = material.smoothness.clamp(0.0, 1.0);
        let alpha = effective_alpha(material);
        self.queue.write_buffer(
            &params,
            0,
            bytemuck::cast_slice::<f32, u8>(&[metallic, smoothness, alpha, 0.0]),
        );
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
                    resource: params.as_entire_binding(),
                },
            ],
        })
    }

    fn prepare_draw_items(
        &self,
        items: &[DrawItem<'_>],
        camera: &Camera,
        view: Mat4,
        light: &DirectionalLight,
    ) -> Result<Vec<Option<MeshBuffers>>, RenderError> {
        let view_proj = camera.projection_matrix() * view;
        let mut meshes: Vec<Option<MeshBuffers>> = Vec::with_capacity(items.len());
        for item in items {
            meshes.push(self.upload_mesh(item.instance.mesh, item.attributes, item.material)?);
        }

        let mut state = lock(&self.uniforms);
        if state.capacity < items.len() {
            let capacity = items.len().max(state.capacity * 2).max(1);
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
        globals[16] = direction.x;
        globals[17] = direction.y;
        globals[18] = direction.z;
        globals[19] = light.intensity.clamp(0.0, 1.0);
        globals[20] = light.color[0];
        globals[21] = light.color[1];
        globals[22] = light.color[2];
        globals[23] = 1.0 - globals[19];
        self.queue
            .write_buffer(&state.buffer, 0, bytemuck::cast_slice::<f32, u8>(&globals));

        let mut instance_data: Vec<f32> = vec![0.0; SLOT_FLOATS * items.len()];
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
        if !instance_data.is_empty() {
            self.queue.write_buffer(
                &state.buffer,
                UNIFORM_SLOT,
                bytemuck::cast_slice::<f32, u8>(&instance_data),
            );
        }
        Ok(meshes)
    }

    fn draw_pass_indices(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipelines: &Pipelines,
        items: &[DrawItem<'_>],
        meshes: &[Option<MeshBuffers>],
        indices: &[usize],
    ) {
        let state = lock(&self.uniforms);
        pass.set_bind_group(0, &state.global_bind_group, &[]);
        for &index in indices {
            let item = &items[index];
            let mesh = match &meshes[index] {
                Some(mesh) => mesh,
                None => continue,
            };
            pass.set_bind_group(
                1,
                &state.instance_bind_group,
                &[((UNIFORM_SLOT * (index as u64 + 1)) as u32)],
            );
            let material_bind = mesh.material_bind.as_ref().unwrap_or(&self.material_bind);
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

struct MeshBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    material_bind: Option<wgpu::BindGroup>,
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

fn image_from_rgba(data: &[u8], width: u32, height: u32, bytes_per_row: u64) -> Image {
    let mut image = Image::new(width, height);
    let width = width as usize;
    let height = height as usize;
    let bytes_per_row = bytes_per_row as usize;
    for y in 0..height {
        let row_start = y * bytes_per_row;
        for x in 0..width {
            let offset = row_start + x * 4;
            if offset + 3 >= data.len() {
                continue;
            }
            image.pixels[y * width + x] = [data[offset], data[offset + 1], data[offset + 2]];
        }
    }
    image
}

fn capture_frame_from_readback(
    color_data: &[u8],
    id_data: &[u8],
    bytes_per_row: u64,
    width: u32,
    height: u32,
    instances: &[CaptureInstance<'_>],
) -> CaptureFrame {
    let row_bytes = (width as usize) * 4;
    let bytes_per_row = bytes_per_row as usize;
    let mut image = Rgba8::new(width, height);
    let mut id_map = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height as usize {
        let row_start = y * bytes_per_row;
        let color_end = row_start + row_bytes;
        if color_end <= color_data.len() {
            image.pixels[y * row_bytes..(y + 1) * row_bytes]
                .copy_from_slice(&color_data[row_start..color_end]);
        }
        for x in 0..width as usize {
            let offset = row_start + x * 4;
            let value = if offset + 3 < id_data.len() {
                u32::from_le_bytes([
                    id_data[offset],
                    id_data[offset + 1],
                    id_data[offset + 2],
                    id_data[offset + 3],
                ])
            } else {
                0
            };
            id_map.push(value);
        }
    }
    let objects = capture_objects(&id_map, width, instances);
    CaptureFrame {
        image,
        id_map,
        objects,
    }
}

fn capture_objects(
    id_map: &[u32],
    width: u32,
    instances: &[CaptureInstance<'_>],
) -> Vec<CaptureObject> {
    let mut extents: Vec<Option<(u32, u32, u32, u32)>> = vec![None; instances.len() + 1];
    for (index, id) in id_map.iter().enumerate() {
        if *id == 0 || *id as usize > extents.len() - 1 {
            continue;
        }
        let x = (index % width as usize) as u32;
        let y = (index / width as usize) as u32;
        extents[*id as usize] = Some(match extents[*id as usize] {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    }
    let mut objects = Vec::new();
    for (id, extent) in extents.iter().enumerate() {
        let extent = match extent {
            Some(extent) => extent,
            None => continue,
        };
        let name = match id.checked_sub(1).and_then(|index| instances.get(index)) {
            Some(instance) => instance.name,
            None => "",
        };
        objects.push(CaptureObject {
            id: id as u32,
            name: name.to_string(),
            bounds: ScreenBounds {
                x: extent.0,
                y: extent.1,
                width: extent.2 - extent.0 + 1,
                height: extent.3 - extent.1 + 1,
            },
        });
    }
    objects
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
        let offscreen_graph = RenderGraph::offscreen(true);
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

        let surface_graph = RenderGraph::surface(false);
        assert_eq!(
            surface_graph.pass_order(),
            vec![RenderPassKind::Opaque, RenderPassKind::Transparent]
        );
        assert!(surface_graph
            .passes()
            .iter()
            .all(|pass| pass.target == RenderTargetKind::SurfaceColorDepth));

        let capture_graph = RenderGraph::capture();
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
}
