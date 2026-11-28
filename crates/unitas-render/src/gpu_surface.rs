use crate::camera::Camera;
use crate::glyphs::{
    layout_text_in_box, shape_glyph_quads, FontLibrary, GlyphAtlas, GlyphResources, Rect,
    SwashCache, TextFrame, TextLayoutBox, TextPipeline, TextStyle, TexturedQuad,
};
use crate::image::Rgba8;
use crate::math::Mat4;
use crate::model::{DirectionalLight, MeshInstance};
use crate::render_graph::{RenderGraph, RenderPassKind};

use super::gpu_pipelines::create_pipelines;
use super::{
    background_color, lock, material_draw_items, opaque_order, plain_draw_items, transparent_order,
    CanvasRect, DrawItem, GpuInstance, GpuMaterialInstance, ImageTarget, Pipelines, PreparedDraw,
    PreparedSprite, RenderError, RenderScene, Renderer, SurfaceState, TextOverlayItem, TextState,
    TextTarget,
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

struct SurfacePasses<'a> {
    graph: &'a RenderGraph,
    items: &'a [DrawItem<'a>],
    prepared: &'a [PreparedDraw],
    prepared_sprites: &'a [PreparedSprite],
    view: Mat4,
    background: [f32; 3],
    overlay: &'a [TextOverlayItem],
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

    fn surface_target<'a>(
        surface_state: &'a SurfaceState,
        color_view: &'a wgpu::TextureView,
        depth_view: &'a wgpu::TextureView,
        size: (u32, u32),
    ) -> SurfaceTarget<'a> {
        SurfaceTarget {
            color_view,
            depth_view,
            pipelines: &surface_state.pipelines,
            format: surface_state.format,
            size,
            scale: size.0 as f32 / surface_state.config_size.0 as f32,
        }
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
        let target = Self::surface_target(surface_state, &color_view, &depth_view, size);
        let overlays = SurfaceOverlays {
            overlay,
            canvas_rects,
            canvas_texts,
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("surface"),
            });
        self.encode_frame(&mut encoder, target, &items, scene, overlays)?;
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

    fn encode_surface_opaque_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &SurfaceTarget<'_>,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
        background: [f32; 3],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("surface-opaque"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(background_color(background)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target.depth_view,
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
        self.draw_pass_indices(&mut pass, target.pipelines, items, prepared, &indices);
    }

    fn encode_surface_transparent_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &SurfaceTarget<'_>,
        items: &[DrawItem<'_>],
        prepared: &[PreparedDraw],
        indices: &[usize],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("surface-transparent"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target.depth_view,
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
        self.draw_pass_indices(&mut pass, target.pipelines, items, prepared, indices);
    }

    fn encode_surface_sprite_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &SurfaceTarget<'_>,
        prepared_sprites: &[PreparedSprite],
        item_count: usize,
    ) {
        self.encode_sprite_pass(
            encoder,
            &[target.color_view],
            target.depth_view,
            prepared_sprites,
            &target.pipelines.sprite,
            item_count,
        );
    }

    fn encode_surface_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &SurfaceTarget<'_>,
        passes: &SurfacePasses<'_>,
    ) {
        for graph_pass in passes.graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(encoder, passes.items, passes.prepared);
                }
                RenderPassKind::Opaque => {
                    self.encode_surface_opaque_pass(
                        encoder,
                        target,
                        passes.items,
                        passes.prepared,
                        passes.background,
                    );
                }
                RenderPassKind::Transparent => {
                    let indices = transparent_order(passes.items, passes.view);
                    if !indices.is_empty() {
                        self.encode_surface_transparent_pass(
                            encoder,
                            target,
                            passes.items,
                            passes.prepared,
                            &indices,
                        );
                    }
                }
                RenderPassKind::Sprites => {
                    self.encode_surface_sprite_pass(
                        encoder,
                        target,
                        passes.prepared_sprites,
                        passes.items.len(),
                    );
                }
                RenderPassKind::UiOverlay => {
                    self.draw_text_overlay(
                        encoder,
                        target.color_view,
                        target.format,
                        target.size,
                        target.scale,
                        passes.overlay,
                    );
                }
                RenderPassKind::CaptureIds => {}
            }
        }
    }

    fn encode_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: SurfaceTarget<'_>,
        items: &[DrawItem<'_>],
        scene: RenderScene<'_>,
        overlays: SurfaceOverlays<'_>,
    ) -> Result<(), RenderError> {
        let mut camera = *scene.camera;
        camera.aspect = target.size.0 as f32 / target.size.1 as f32;
        let shadows_enabled = self.prepare_shadow_map(target.size.0.max(target.size.1));
        let sprites = self.sprites();
        let (prepared, prepared_sprites) =
            self.prepare_draw_items(items, &sprites, &camera, scene.view, scene.light)?;
        let graph = RenderGraph::surface(!overlays.overlay.is_empty(), shadows_enabled)
            .with_sprites(!sprites.is_empty());
        let passes = SurfacePasses {
            graph: &graph,
            items,
            prepared: &prepared,
            prepared_sprites: &prepared_sprites,
            view: scene.view,
            background: scene.background,
            overlay: overlays.overlay,
        };
        self.encode_surface_passes(encoder, &target, &passes);
        self.draw_ui_rects(
            encoder,
            target.color_view,
            target.format,
            target.size,
            overlays.canvas_rects,
        );
        self.draw_text_overlay(
            encoder,
            target.color_view,
            target.format,
            target.size,
            1.0,
            overlays.canvas_texts,
        );
        Ok(())
    }

    fn frame_rect(size: (u32, u32)) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: size.0 as f32,
            height: size.1 as f32,
        }
    }

    fn text_layout_box(item: &TextOverlayItem, scale: f32) -> TextLayoutBox {
        TextLayoutBox {
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
        }
    }

    fn build_text_batches(
        fonts: &mut FontLibrary,
        cache: &mut SwashCache,
        atlas: &mut GlyphAtlas,
        size: (u32, u32),
        scale: f32,
        overlay: &[TextOverlayItem],
    ) -> Vec<(Rect, Vec<TexturedQuad>)> {
        let mut batches: Vec<(Rect, Vec<TexturedQuad>)> = Vec::new();
        for item in overlay {
            if item.text.is_empty() || item.size_px <= 0.0 {
                continue;
            }
            let scaled = item.size_px * scale;
            let style = TextStyle::new(scaled, scaled, item.color);
            let layout_box = Self::text_layout_box(item, scale);
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
        batches
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
        let frame = Self::frame_rect(size);
        let batches = Self::build_text_batches(fonts, cache, atlas, size, scale, overlay);
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

    fn ui_rect_quad(rect: &CanvasRect, bounds: (u32, u32, u32, u32)) -> TexturedQuad {
        let uv = if rect.sprite.is_some() {
            rect.uv_rect
        } else {
            [0.0, 0.0, 1.0, 1.0]
        };
        TexturedQuad {
            x: bounds.0 as f32,
            y: bounds.1 as f32,
            width: (bounds.2 - bounds.0) as f32,
            height: (bounds.3 - bounds.1) as f32,
            uv,
            color: rect.color,
        }
    }

    fn solid_image_target<'a>(
        &self,
        text: &'a mut TextState,
        format: wgpu::TextureFormat,
    ) -> &'a ImageTarget {
        text.solid_targets.entry(format).or_insert_with(|| {
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
        })
    }

    fn draw_ui_rect(
        &self,
        target: &ImageTarget,
        solid_resources: &GlyphResources,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: (u32, u32),
        rect: &CanvasRect,
    ) {
        let Some(bounds) = ui_rect_bounds(rect.rect, size.0, size.1) else {
            return;
        };
        let quad = Self::ui_rect_quad(rect, bounds);
        let clip = resolve_ui_clip_rect(size.0, size.1, rect.clip_rect);
        let mut target_frame = TextFrame {
            encoder,
            view,
            frame: Self::frame_rect(size),
            clip,
        };
        if let Some(sprite) = rect
            .sprite
            .as_ref()
            .filter(|img| img.width > 0 && img.height > 0)
        {
            let sprite_resources = target
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
        let target = self.solid_image_target(&mut text, format);
        let Some(solid_resources) = &target.resources else {
            return;
        };
        for rect in rects {
            self.draw_ui_rect(target, solid_resources, encoder, view, size, rect);
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
