use crate::camera::Camera;
use crate::image::Rgba8;
use crate::math::Mat4;
use crate::model::{DirectionalLight, MeshInstance};
use crate::render_graph::{RenderGraph, RenderPassKind};

use super::gpu_pipelines::create_pipelines;
use super::{
    background_color, lock, material_draw_items, opaque_order, plain_draw_items, transparent_order,
    DrawItem, GpuInstance, GpuMaterialInstance, RenderError, RenderScene, Renderer, SurfaceState,
    TextOverlayItem,
};

#[must_use]
pub fn present_mode_for_v_sync_count(
    v_sync_count: u32,
    supported: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    if v_sync_count == 0 {
        for candidate in [
            wgpu::PresentMode::AutoNoVsync,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Mailbox,
        ] {
            if supported.contains(&candidate) {
                return candidate;
            }
        }
    } else {
        for candidate in [wgpu::PresentMode::Fifo, wgpu::PresentMode::AutoVsync] {
            if supported.contains(&candidate) {
                return candidate;
            }
        }
    }
    supported
        .first()
        .copied()
        .unwrap_or(wgpu::PresentMode::Fifo)
}

impl Renderer {
    pub fn configure_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
    ) -> Result<wgpu::SurfaceConfiguration, RenderError> {
        self.configure_surface_with_v_sync(surface, width, height, self.v_sync_count())
    }

    pub fn configure_surface_with_v_sync(
        &self,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        v_sync_count: u32,
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
        self.set_v_sync_count(v_sync_count);
        let capabilities = surface.get_capabilities(&self.adapter);
        configuration.present_mode =
            present_mode_for_v_sync_count(v_sync_count, &capabilities.present_modes);
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
        self.render_to_surface_items(surface, plain_draw_items(instances), scene, overlay)
    }

    pub fn render_to_surface_materials(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[GpuMaterialInstance<'_>],
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
        self.render_to_surface_materials_with_overlay(surface, instances, scene, &[])
    }

    pub fn render_to_surface_materials_with_overlay(
        &self,
        surface: &wgpu::Surface<'_>,
        instances: &[GpuMaterialInstance<'_>],
        scene: RenderScene<'_>,
        overlay: &[TextOverlayItem],
    ) -> Result<(), RenderError> {
        self.render_to_surface_items(surface, material_draw_items(instances), scene, overlay)
    }

    fn render_to_surface_items(
        &self,
        surface: &wgpu::Surface<'_>,
        items: Vec<DrawItem<'_>>,
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
        let shadows_enabled = self.prepare_shadow_map(size.0.max(size.1));
        let sprites = self.sprites();
        let (prepared, prepared_sprites) =
            self.prepare_draw_items(&items, &sprites, &camera, view, light)?;
        let graph = RenderGraph::surface(!overlay.is_empty(), shadows_enabled)
            .with_sprites(!sprites.is_empty());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface"),
            });
        for graph_pass in graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(&mut encoder, &items, &prepared);
                }
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
                RenderPassKind::Sprites => {
                    self.encode_sprite_pass(
                        &mut encoder,
                        &[&color_view],
                        &depth_view,
                        &prepared_sprites,
                        &surface_state.pipelines.sprite,
                        items.len(),
                    );
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

    /// Presents a composed frame on a window surface.
    pub fn present_rgba_to_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        image: &Rgba8,
    ) -> Result<(), RenderError> {
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
            match state.as_ref() {
                Some(surface_state) => surface_state.format,
                None => return Err(RenderError::SurfaceNotConfigured),
            }
        };
        {
            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.draw_rgba_to_view(&view, format, size, image)?;
        }
        self.queue.present(surface_texture);
        Ok(())
    }
}
