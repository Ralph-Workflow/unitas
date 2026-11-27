use super::gpu_shader::{SHADER_SOURCE, VERTEX_LAYOUT};
use super::solid_texture;
use super::{Pipelines, DEPTH_FORMAT, GLOBAL_FLOATS, INSTANCE_FLOATS};

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

pub(super) fn create_pipelines(
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

pub(super) fn create_capture_pipelines(
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

pub(super) fn build_shadow_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let depth_stencil = wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState {
            constant: 2,
            slope_scale: 2.0,
            clamp: 0.0,
        },
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_shadow"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(VERTEX_LAYOUT)],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth_stencil),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

pub(super) fn create_shadow_resources(
    device: &wgpu::Device,
    shadow_layout: &wgpu::BindGroupLayout,
    shadow_sampler: &wgpu::Sampler,
    size: u32,
) -> (wgpu::TextureView, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow-depth"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow"),
        layout: shadow_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
        ],
    });
    (view, bind_group)
}

pub(super) fn create_scene_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scene"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    })
}

pub(super) fn create_global_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub(super) fn create_instance_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub(super) fn create_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub(super) fn create_shadow_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    })
}

pub(super) fn create_scene_pipeline_layout(
    device: &wgpu::Device,
    global: &wgpu::BindGroupLayout,
    instance: &wgpu::BindGroupLayout,
    material: &wgpu::BindGroupLayout,
    shadow: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene"),
        bind_group_layouts: &[Some(global), Some(instance), Some(material), Some(shadow)],
        immediate_size: 0,
    })
}

pub(super) fn create_shadow_pipeline_layout(
    device: &wgpu::Device,
    global: &wgpu::BindGroupLayout,
    instance: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow"),
        bind_group_layouts: &[Some(global), Some(instance)],
        immediate_size: 0,
    })
}

pub(super) fn create_shadow_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shadow"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}

pub(super) fn create_material_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("material"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    })
}

pub(super) fn create_default_material_bind(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    material_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let albedo = solid_texture(device, queue, [255, 255, 255, 255]);
    let normal = solid_texture(device, queue, [128, 128, 255, 255]);
    let emissive = solid_texture(device, queue, [0, 0, 0, 255]);
    let params = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("material-default"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(
        &params,
        0,
        bytemuck::cast_slice::<f32, u8>(&[0.0f32, 0.0, 1.0, 0.0]),
    );
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material-default"),
        layout: material_layout,
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
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: params.as_entire_binding(),
            },
        ],
    })
}
