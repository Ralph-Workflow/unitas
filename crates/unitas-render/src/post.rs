use std::collections::HashMap;

use crate::render_graph::PostPassKind;

use super::gpu_pipelines::{
    create_post_layout, create_post_pipelines, create_post_sampler, create_post_shader,
    PostPipelines,
};
use super::{lock, Pipelines, RenderError, Renderer, SurfaceFrame};

pub const POST_SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const POST_PARAMS_SIZE: u64 = 32;
pub const BLOOM_THRESHOLD: f32 = 0.8;
pub const BLOOM_INTENSITY: f32 = 0.35;
pub const EXPOSURE: f32 = 1.0;
pub const BLUR_WEIGHTS: [f32; 5] = [
    0.227_027_03,
    0.194_594_6,
    0.121_621_62,
    0.054_054_05,
    0.016_216_22,
];

const BLOOM_DIVISOR: u32 = 2;
const PARAM_SLOT_SIZE: u64 = 256;
const PARAM_SLOTS: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostSettings {
    pub enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub exposure: f32,
}

impl Default for PostSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            bloom_threshold: BLOOM_THRESHOLD,
            bloom_intensity: BLOOM_INTENSITY,
            exposure: EXPOSURE,
        }
    }
}

pub fn luminance(color: [f32; 3]) -> f32 {
    0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}

pub fn bright_pass(color: [f32; 3], threshold: f32) -> [f32; 3] {
    if luminance(color) <= threshold {
        [0.0, 0.0, 0.0]
    } else {
        color
    }
}

const fn aces_fit_raw(value: f32) -> f32 {
    (value * (value + 0.024_578_6) - 0.000_090_537)
        / (value * (0.983_729 * value + 0.432_951) + 0.238_081)
}

const ACES_FIT_WHITE: f32 = aces_fit_raw(1.0);

pub fn aces_fit(value: f32) -> f32 {
    (aces_fit_raw(value) / ACES_FIT_WHITE).max(0.0)
}

pub fn tonemap(color: [f32; 3], exposure: f32) -> [f32; 3] {
    [
        aces_fit(color[0] * exposure),
        aces_fit(color[1] * exposure),
        aces_fit(color[2] * exposure),
    ]
}

#[derive(Debug, Clone, Copy)]
struct PostParams {
    threshold: f32,
    intensity: f32,
    exposure: f32,
    texel_x: f32,
    texel_y: f32,
    direction_x: f32,
    direction_y: f32,
    reserved: f32,
}

impl PostParams {
    fn bytes(self) -> [u8; POST_PARAMS_SIZE as usize] {
        bytemuck::cast([
            self.threshold,
            self.intensity,
            self.exposure,
            self.texel_x,
            self.texel_y,
            self.direction_x,
            self.direction_y,
            self.reserved,
        ])
    }
}

#[derive(Clone)]
struct PostTargets {
    size: (u32, u32),
    bloom_size: (u32, u32),
    scene_view: wgpu::TextureView,
    bloom_views: [wgpu::TextureView; 2],
    bright_bind: wgpu::BindGroup,
    blur_binds: [wgpu::BindGroup; 2],
    tonemap_bind: wgpu::BindGroup,
}

pub(super) struct PostState {
    settings: PostSettings,
    sampler: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
    shader: wgpu::ShaderModule,
    pipelines: HashMap<wgpu::TextureFormat, PostPipelines>,
    scene_pipelines: Option<Pipelines>,
    targets: Option<PostTargets>,
}

impl PostState {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            settings: PostSettings::default(),
            sampler: create_post_sampler(device),
            layout: create_post_layout(device),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("post-params"),
                size: PARAM_SLOT_SIZE * PARAM_SLOTS,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            shader: create_post_shader(device),
            pipelines: HashMap::new(),
            scene_pipelines: None,
            targets: None,
        }
    }

    fn pipelines_for(
        &mut self,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
    ) -> PostPipelines {
        let entry = self.pipelines.entry(output_format).or_insert_with(|| {
            create_post_pipelines(
                device,
                &self.layout,
                &self.shader,
                output_format,
                POST_SCENE_FORMAT,
            )
        });
        entry.clone()
    }

    fn ensure_targets(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> Result<PostTargets, RenderError> {
        if size.0 == 0 || size.1 == 0 {
            return Err(RenderError::InvalidSize(size.0, size.1));
        }
        if let Some(targets) = &self.targets {
            if targets.size == size {
                return Ok(targets.clone());
            }
        }
        let bloom_size = (
            (size.0 / BLOOM_DIVISOR).max(1),
            (size.1 / BLOOM_DIVISOR).max(1),
        );
        let scene = post_texture(device, size, POST_SCENE_FORMAT, "post-scene");
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom = [
            post_texture(device, bloom_size, POST_SCENE_FORMAT, "post-bloom"),
            post_texture(device, bloom_size, POST_SCENE_FORMAT, "post-bloom"),
        ];
        let bloom_views = [
            bloom[0].create_view(&wgpu::TextureViewDescriptor::default()),
            bloom[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let bright_bind = self.bind_group(device, &scene_view, &scene_view, 0);
        // The blur passes read the source texture only; the bloom texture is
        // the pass's own color target, so it must not also be bound for reading.
        let blur_binds = [
            self.bind_group(device, &bloom_views[0], &bloom_views[0], 1),
            self.bind_group(device, &bloom_views[1], &bloom_views[1], 2),
        ];
        let tonemap_bind = self.bind_group(device, &scene_view, &bloom_views[0], 3);
        let targets = PostTargets {
            size,
            bloom_size,
            scene_view,
            bloom_views,
            bright_bind,
            blur_binds,
            tonemap_bind,
        };
        self.targets = Some(targets.clone());
        Ok(targets)
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        bloom: &wgpu::TextureView,
        slot: u64,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.params,
                        offset: slot * PARAM_SLOT_SIZE,
                        size: wgpu::BufferSize::new(POST_PARAMS_SIZE),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(bloom),
                },
            ],
        })
    }

    fn write_params(&self, queue: &wgpu::Queue, settings: PostSettings, bloom_size: (u32, u32)) {
        let texel_x = 1.0 / bloom_size.0.max(1) as f32;
        let texel_y = 1.0 / bloom_size.1.max(1) as f32;
        let slots = [
            PostParams {
                threshold: settings.bloom_threshold,
                intensity: settings.bloom_intensity,
                exposure: settings.exposure,
                texel_x,
                texel_y,
                direction_x: 0.0,
                direction_y: 0.0,
                reserved: 0.0,
            },
            PostParams {
                threshold: 0.0,
                intensity: 0.0,
                exposure: 0.0,
                texel_x,
                texel_y,
                direction_x: 1.0,
                direction_y: 0.0,
                reserved: 0.0,
            },
            PostParams {
                threshold: 0.0,
                intensity: 0.0,
                exposure: 0.0,
                texel_x,
                texel_y,
                direction_x: 0.0,
                direction_y: 1.0,
                reserved: 0.0,
            },
            PostParams {
                threshold: 0.0,
                intensity: settings.bloom_intensity,
                exposure: settings.exposure,
                texel_x,
                texel_y,
                direction_x: 0.0,
                direction_y: 0.0,
                reserved: 0.0,
            },
        ];
        for (index, params) in slots.iter().enumerate() {
            queue.write_buffer(
                &self.params,
                index as u64 * PARAM_SLOT_SIZE,
                &params.bytes(),
            );
        }
    }
}

fn post_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    label: &'static str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

impl Renderer {
    pub fn set_post_settings(&self, settings: PostSettings) {
        lock(&self.post).settings = settings;
    }

    pub fn post_settings(&self) -> PostSettings {
        lock(&self.post).settings
    }

    pub(super) fn post_scene_pipelines(&self) -> Pipelines {
        let mut state = lock(&self.post);
        let pipelines = state.scene_pipelines.get_or_insert_with(|| {
            super::gpu_pipelines::create_pipelines(
                &self.device,
                &self.pipeline_layout,
                &self.shader,
                POST_SCENE_FORMAT,
            )
        });
        pipelines.clone()
    }

    pub(super) fn post_scene_view(
        &self,
        size: (u32, u32),
    ) -> Result<wgpu::TextureView, RenderError> {
        let mut state = lock(&self.post);
        Ok(state.ensure_targets(&self.device, size)?.scene_view)
    }

    pub(super) fn post_scene_target(
        &self,
        size: (u32, u32),
        fallback: &wgpu::TextureView,
    ) -> Result<(wgpu::TextureView, Pipelines), RenderError> {
        if !self.post_settings().enabled {
            return Ok((fallback.clone(), self.offscreen_pipelines.clone()));
        }
        Ok((self.post_scene_view(size)?, self.post_scene_pipelines()))
    }

    pub(super) fn encode_post_chain(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        passes: &[PostPassKind],
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Result<(), RenderError> {
        let mut state = lock(&self.post);
        if !state.settings.enabled || passes.is_empty() {
            return Ok(());
        }
        let settings = state.settings;
        let targets = state.ensure_targets(&self.device, size)?;
        state.write_params(&self.queue, settings, targets.bloom_size);
        let pipelines = state.pipelines_for(&self.device, output_format);
        for pass in passes {
            let (pipeline, view, bind) = match pass {
                PostPassKind::Bright => (
                    &pipelines.bright,
                    &targets.bloom_views[0],
                    &targets.bright_bind,
                ),
                PostPassKind::BlurHorizontal => (
                    &pipelines.blur,
                    &targets.bloom_views[1],
                    &targets.blur_binds[0],
                ),
                PostPassKind::BlurVertical => (
                    &pipelines.blur,
                    &targets.bloom_views[0],
                    &targets.blur_binds[1],
                ),
                PostPassKind::Tonemap => (&pipelines.tonemap, output_view, &targets.tonemap_bind),
            };
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind, &[]);
            render_pass.draw(0..3, 0..1);
        }
        Ok(())
    }

    pub fn render_surface_frame_post(
        &self,
        surface: &wgpu::Surface<'_>,
        frame: &SurfaceFrame<'_>,
    ) -> Result<(), RenderError> {
        if !self.post_settings().enabled {
            return self.render_surface_frame(surface, frame);
        }
        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => return Err(RenderError::SurfaceUnavailable(format!("{other:?}"))),
        };
        let size = (
            surface_texture.texture.width(),
            surface_texture.texture.height(),
        );
        let format = {
            let state = lock(&self.surface);
            state
                .as_ref()
                .map_or(wgpu::TextureFormat::Bgra8Unorm, |surface| surface.format)
        };
        let scene_view = self.post_scene_view(frame.size)?;
        let scene_frame = SurfaceFrame {
            instances: frame.instances,
            scene: frame.scene,
            size: frame.size,
            overlay: &[],
            canvas_rects: &[],
            canvas_texts: &[],
        };
        self.render_frame_to_view(&scene_view, POST_SCENE_FORMAT, &scene_frame)?;
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface-post"),
            });
        let post = crate::render_graph::PostChain::bloom_tonemap();
        self.encode_post_chain(
            &mut encoder,
            post.passes(),
            &output_view,
            format,
            frame.size,
        )?;
        self.draw_ui_rects(&mut encoder, &output_view, format, size, frame.canvas_rects);
        self.draw_text_overlay(
            &mut encoder,
            &output_view,
            format,
            size,
            1.0,
            frame.canvas_texts,
        );
        self.draw_text_overlay(
            &mut encoder,
            &output_view,
            format,
            size,
            self.overlay_scale(size),
            frame.overlay,
        );
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(surface_texture);
        Ok(())
    }

    fn overlay_scale(&self, size: (u32, u32)) -> f32 {
        let state = lock(&self.surface);
        let Some(surface) = state.as_ref() else {
            return 1.0;
        };
        if surface.config_size.0 == 0 {
            return 1.0;
        }
        size.0 as f32 / surface.config_size.0 as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bright_pass_keeps_pixels_above_the_threshold_and_drops_the_rest() {
        assert_eq!(bright_pass([0.2, 0.3, 0.4], 0.5), [0.0, 0.0, 0.0]);
        assert_eq!(bright_pass([0.9, 0.9, 0.9], 0.5), [0.9, 0.9, 0.9]);
        assert_eq!(bright_pass([1.0, 0.0, 0.0], 0.5), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn aces_fit_maps_zero_to_zero_and_one_to_one() {
        assert_eq!(aces_fit(0.0), 0.0);
        assert!((aces_fit(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn aces_fit_keeps_the_midtone_close_to_unity() {
        assert!((aces_fit(0.18) - 0.1706).abs() < 0.002);
    }

    #[test]
    fn aces_fit_stays_monotonic() {
        let mut previous = aces_fit(0.0);
        let mut value = 0.05;
        while value <= 4.0 {
            let current = aces_fit(value);
            assert!(
                current >= previous,
                "the tonemap curve fell from {previous} to {current} at {value}"
            );
            previous = current;
            value += 0.05;
        }
    }

    #[test]
    fn tonemap_applies_the_exposure_before_the_curve() {
        assert_eq!(tonemap([0.5, 0.5, 0.5], 1.0), [aces_fit(0.5); 3]);
        assert_eq!(tonemap([0.5, 0.5, 0.5], 2.0), [aces_fit(1.0); 3]);
    }

    #[test]
    fn the_blur_weights_sum_to_one() {
        let tail: f32 = BLUR_WEIGHTS[1..].iter().sum();
        assert!((BLUR_WEIGHTS[0] + 2.0 * tail - 1.0).abs() < 1e-5);
    }
}
