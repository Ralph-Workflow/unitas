use crate::camera::Camera;
use crate::glyphs::{
    layout_text_in_box, shape_glyph_quads, Rect, TextFrame, TextLayoutBox, TextPipeline, TextStyle,
    TexturedQuad,
};
use crate::image::Rgba8;
use crate::math::Mat4;
use crate::model::{DirectionalLight, MeshInstance};
use crate::render_graph::{RenderGraph, RenderPassKind};

use super::gpu_pipelines::create_pipelines;
use super::{
    background_color, lock, material_draw_items, opaque_order, plain_draw_items, transparent_order,
    CanvasRect, DrawItem, GpuInstance, GpuMaterialInstance, ImageTarget, Pipelines, RenderError,
    RenderScene, Renderer, SurfaceState, TextOverlayItem, TextState, TextTarget,
};

#[derive(Debug, Clone, Copy)]
pub struct SurfaceFrame<'a> {
    pub instances: &'a [GpuMaterialInstance<'a>],
    pub scene: RenderScene<'a>,
    pub size: (u32, u32),
    pub overlay: &'a [TextOverlayItem],
    pub canvas_rects: &'a [CanvasRect],
    pub canvas_texts: &'a [TextOverlayItem],
}

struct SurfaceTarget<'a> {
    color_view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
    pipelines: &'a Pipelines,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    scale: f32,
}

struct SurfaceOverlays<'a> {
    overlay: &'a [TextOverlayItem],
    canvas_rects: &'a [CanvasRect],
    canvas_texts: &'a [TextOverlayItem],
}

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
        self.render_surface_items(surface, items, scene, overlay, &[], &[])
    }

    pub fn render_surface_frame(
        &self,
        surface: &wgpu::Surface<'_>,
        frame: &SurfaceFrame<'_>,
    ) -> Result<(), RenderError> {
        let items = material_draw_items(frame.instances);
        self.render_surface_items(
            surface,
            items,
            frame.scene,
            frame.overlay,
            frame.canvas_rects,
            frame.canvas_texts,
        )
    }

    fn render_surface_items(
        &self,
        surface: &wgpu::Surface<'_>,
        items: Vec<DrawItem<'_>>,
        scene: RenderScene<'_>,
        overlay: &[TextOverlayItem],
        canvas_rects: &[CanvasRect],
        canvas_texts: &[TextOverlayItem],
    ) -> Result<(), RenderError> {
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
        let color_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = surface_state
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scale = size.0 as f32 / surface_state.config_size.0 as f32;
        let format = surface_state.format;
        let pipelines = surface_state.pipelines.clone();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface"),
            });
        self.encode_frame(
            &mut encoder,
            SurfaceTarget {
                color_view: &color_view,
                depth_view: &depth_view,
                pipelines: &pipelines,
                format,
                size,
                scale,
            },
            &items,
            scene,
            SurfaceOverlays {
                overlay,
                canvas_rects,
                canvas_texts,
            },
        )?;
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(surface_texture);
        Ok(())
    }

    pub fn render_frame_to_view(
        &self,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        frame: &SurfaceFrame<'_>,
    ) -> Result<(), RenderError> {
        if frame.size.0 == 0 || frame.size.1 == 0 {
            return Ok(());
        }
        let items = material_draw_items(frame.instances);
        let pipelines = create_pipelines(&self.device, &self.pipeline_layout, &self.shader, format);
        let depth = self.create_depth_texture(frame.size.0, frame.size.1);
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        self.encode_frame(
            &mut encoder,
            SurfaceTarget {
                color_view: view,
                depth_view: &depth_view,
                pipelines: &pipelines,
                format,
                size: frame.size,
                scale: 1.0,
            },
            &items,
            frame.scene,
            SurfaceOverlays {
                overlay: frame.overlay,
                canvas_rects: frame.canvas_rects,
                canvas_texts: frame.canvas_texts,
            },
        )?;
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    fn encode_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: SurfaceTarget<'_>,
        items: &[DrawItem<'_>],
        scene: RenderScene<'_>,
        overlays: SurfaceOverlays<'_>,
    ) -> Result<(), RenderError> {
        let SurfaceOverlays {
            overlay,
            canvas_rects,
            canvas_texts,
        } = overlays;
        let SurfaceTarget {
            color_view,
            depth_view,
            pipelines,
            format,
            size,
            scale,
        } = target;
        let RenderScene {
            camera,
            view,
            light,
            background,
        } = scene;
        let mut camera = *camera;
        camera.aspect = size.0 as f32 / size.1 as f32;
        let shadows_enabled = self.prepare_shadow_map(size.0.max(size.1));
        let sprites = self.sprites();
        let (prepared, prepared_sprites) =
            self.prepare_draw_items(items, &sprites, &camera, view, light)?;
        let graph = RenderGraph::surface(!overlay.is_empty(), shadows_enabled)
            .with_sprites(!sprites.is_empty());
        for graph_pass in graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(encoder, items, &prepared);
                }
                RenderPassKind::Opaque => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("surface-opaque"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: color_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(background_color(background)),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: depth_view,
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
                        pipelines,
                        items,
                        &prepared,
                        &opaque_order(items),
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(items, view);
                    if !indices.is_empty() {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("surface-transparent"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: color_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: depth_view,
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
                        self.draw_pass_indices(&mut pass, pipelines, items, &prepared, &indices);
                    }
                }
                RenderPassKind::Sprites => {
                    self.encode_sprite_pass(
                        encoder,
                        &[color_view],
                        depth_view,
                        &prepared_sprites,
                        &pipelines.sprite,
                        items.len(),
                    );
                }
                RenderPassKind::UiOverlay => {
                    self.draw_text_overlay(encoder, color_view, format, size, scale, overlay);
                }
                RenderPassKind::CaptureIds => {}
            }
        }
        self.draw_ui_rects(encoder, color_view, format, size, canvas_rects);
        self.draw_text_overlay(encoder, color_view, format, size, 1.0, canvas_texts);
        Ok(())
    }

    pub(super) fn draw_text_overlay(
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
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: size.0 as f32,
            height: size.1 as f32,
        };
        let mut batches: Vec<(Rect, Vec<TexturedQuad>)> = Vec::new();
        for item in overlay {
            if item.text.is_empty() || item.size_px <= 0.0 {
                continue;
            }
            let scaled = item.size_px * scale;
            let style = TextStyle::new(scaled, scaled, item.color);
            let layout_box = TextLayoutBox {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: item.rect[2] * scale,
                    height: item.rect[3] * scale,
                },
                horizontal_align: item.horizontal_align,
                vertical_align: item.vertical_align,
                wrap: item.wrap,
                max_width: item.max_width.map(|w| w * scale),
                line_spacing: item.line_spacing,
            };
            let layout = layout_text_in_box(fonts, &item.text, &layout_box, &style);
            let mut item_quads = shape_glyph_quads(fonts, cache, atlas, &layout);
            if item_quads.is_empty() {
                continue;
            }
            let dx = item.x * scale;
            let dy = item.y * scale;
            for quad in &mut item_quads {
                quad.x += dx;
                quad.y += dy;
            }
            let clip = resolve_ui_clip_rect(
                size.0,
                size.1,
                item.clip_rect
                    .map(|c| [c[0] * scale, c[1] * scale, c[2] * scale, c[3] * scale]),
            );
            if let Some((last_clip, last_quads)) = batches.last_mut() {
                if *last_clip == clip {
                    last_quads.extend(item_quads);
                    continue;
                }
            }
            batches.push((clip, item_quads));
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
        for (clip, quads) in &batches {
            let mut target_frame = TextFrame {
                encoder,
                view,
                frame,
                clip: *clip,
            };
            target.pipeline.draw(
                &self.device,
                &self.queue,
                &target.resources,
                &mut target_frame,
                quads,
            );
        }
    }

    pub(super) fn draw_ui_rects(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        rects: &[CanvasRect],
    ) {
        if rects.is_empty() || size.0 == 0 || size.1 == 0 {
            return;
        }
        let mut text = lock(&self.text);
        let target = text.solid_targets.entry(format).or_insert_with(|| {
            let pipeline = TextPipeline::new(&self.device, format);
            let white = Rgba8 {
                width: 1,
                height: 1,
                pixels: vec![255, 255, 255, 255],
            };
            let resources = pipeline.upload_image(&self.device, &self.queue, &white);
            ImageTarget {
                pipeline,
                resources: Some(resources),
                size: (1, 1),
            }
        });
        let Some(solid_resources) = &target.resources else {
            return;
        };
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: size.0 as f32,
            height: size.1 as f32,
        };
        for rect in rects {
            let Some(bounds) = ui_rect_bounds(rect.rect, size.0, size.1) else {
                continue;
            };
            let uv = if rect.sprite.is_some() {
                rect.uv_rect
            } else {
                [0.0, 0.0, 1.0, 1.0]
            };
            let quad = TexturedQuad {
                x: bounds.0 as f32,
                y: bounds.1 as f32,
                width: (bounds.2 - bounds.0) as f32,
                height: (bounds.3 - bounds.1) as f32,
                uv,
                color: rect.color,
            };
            let clip = resolve_ui_clip_rect(size.0, size.1, rect.clip_rect);
            let mut target_frame = TextFrame {
                encoder,
                view,
                frame,
                clip,
            };
            if let Some(sprite) = rect
                .sprite
                .as_ref()
                .filter(|img| img.width > 0 && img.height > 0)
            {
                let sprite_resources =
                    target
                        .pipeline
                        .upload_image(&self.device, &self.queue, sprite);
                target.pipeline.draw(
                    &self.device,
                    &self.queue,
                    &sprite_resources,
                    &mut target_frame,
                    std::slice::from_ref(&quad),
                );
            } else {
                target.pipeline.draw(
                    &self.device,
                    &self.queue,
                    solid_resources,
                    &mut target_frame,
                    std::slice::from_ref(&quad),
                );
            }
        }
    }
}

fn ui_rect_bounds(rect: [f32; 4], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect[0].floor().max(0.0);
    let y0 = rect[1].floor().max(0.0);
    let x1 = (rect[0] + rect[2]).ceil().max(0.0).min(width as f32);
    let y1 = (rect[1] + rect[3]).ceil().max(0.0).min(height as f32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
}

fn resolve_ui_clip_rect(width: u32, height: u32, clip_rect: Option<[f32; 4]>) -> Rect {
    let screen = Rect {
        x: 0.0,
        y: 0.0,
        width: width as f32,
        height: height as f32,
    };
    let Some(clip) = clip_rect else {
        return screen;
    };
    let x0 = clip[0].floor().max(0.0);
    let y0 = clip[1].floor().max(0.0);
    let x1 = (clip[0] + clip[2]).ceil().min(screen.width).max(x0);
    let y1 = (clip[1] + clip[3]).ceil().min(screen.height).max(y0);
    Rect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}
