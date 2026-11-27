use crate::camera::Camera;
use crate::capture::{CaptureFrame, CaptureInstance, CaptureObject, ScreenBounds};
use crate::image::Rgba8;
use crate::math::Mat4;
use crate::model::DirectionalLight;
use crate::render_graph::{RenderGraph, RenderPassKind};

use super::{
    align_up, background_color, capture_draw_items, opaque_order, transparent_order, DrawItem,
    PreparedDraw, RenderError, Renderer, COPY_ROW_ALIGNMENT, ID_FORMAT, OFFSCREEN_FORMAT,
};

struct CaptureTargets<'a> {
    graph: RenderGraph,
    items: Vec<DrawItem<'a>>,
    prepared: Vec<PreparedDraw>,
    color_texture: wgpu::Texture,
    id_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    id_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    color_readback: wgpu::Buffer,
    id_readback: wgpu::Buffer,
    bytes_per_row: u64,
    width: u32,
    height: u32,
    background: [f32; 3],
    view: Mat4,
}

impl Renderer {
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
        let shadows_enabled = self.prepare_shadow_map(width.max(height));
        let graph = RenderGraph::capture(shadows_enabled);
        let color_texture =
            self.create_capture_texture("capture-color", width, height, OFFSCREEN_FORMAT);
        let id_texture = self.create_capture_texture("capture-ids", width, height, ID_FORMAT);
        let depth_texture = self.create_depth_texture(width, height);
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let id_view = id_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let items = capture_draw_items(instances);
        let prepared = self.prepare_draw_items(&items, camera, view, light)?;
        let bytes_per_row = align_up(u64::from(width) * 4, COPY_ROW_ALIGNMENT);
        let color_readback = create_readback_buffer(
            &self.device,
            "capture-color-readback",
            bytes_per_row,
            height,
        );
        let id_readback =
            create_readback_buffer(&self.device, "capture-id-readback", bytes_per_row, height);
        let targets = CaptureTargets {
            graph,
            items,
            prepared,
            color_texture,
            id_texture,
            color_view,
            id_view,
            depth_view,
            color_readback,
            id_readback,
            bytes_per_row,
            width,
            height,
            background,
            view,
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("capture"),
            });
        self.encode_capture_passes(&mut encoder, &targets);
        self.queue.submit(Some(encoder.finish()));
        let color_data = self.read_buffer(&targets.color_readback)?;
        let id_data = self.read_buffer(&targets.id_readback)?;
        Ok(capture_frame_from_readback(
            &color_data,
            &id_data,
            bytes_per_row,
            width,
            height,
            instances,
        ))
    }

    fn create_capture_texture(
        &self,
        label: &'static str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn encode_capture_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &CaptureTargets<'_>,
    ) {
        for graph_pass in targets.graph.passes() {
            match graph_pass.kind {
                RenderPassKind::Shadow => {
                    self.encode_shadow_pass(encoder, &targets.items, &targets.prepared);
                }
                RenderPassKind::Opaque => {
                    self.encode_capture_opaque(encoder, targets);
                }
                RenderPassKind::Transparent => {
                    self.encode_capture_transparent(encoder, targets);
                }
                RenderPassKind::CaptureIds => {
                    self.encode_capture_readback(encoder, targets);
                }
                RenderPassKind::UiOverlay => {}
            }
        }
    }

    fn encode_capture_opaque(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &CaptureTargets<'_>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("capture-opaque"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color(targets.background)),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.id_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth_view,
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
            &targets.items,
            &targets.prepared,
            &opaque_order(&targets.items),
        );
    }

    fn encode_capture_transparent(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &CaptureTargets<'_>,
    ) {
        let indices = transparent_order(&targets.items, targets.view);
        if indices.is_empty() {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("capture-transparent"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.id_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth_view,
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
        self.draw_pass_indices(
            &mut pass,
            &self.capture_pipelines,
            &targets.items,
            &targets.prepared,
            &indices,
        );
    }

    fn encode_capture_readback(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &CaptureTargets<'_>,
    ) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &targets.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &targets.color_readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(targets.bytes_per_row as u32),
                    rows_per_image: Some(targets.height),
                },
            },
            wgpu::Extent3d {
                width: targets.width,
                height: targets.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &targets.id_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &targets.id_readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(targets.bytes_per_row as u32),
                    rows_per_image: Some(targets.height),
                },
            },
            wgpu::Extent3d {
                width: targets.width,
                height: targets.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn create_readback_buffer(
    device: &wgpu::Device,
    label: &'static str,
    bytes_per_row: u64,
    height: u32,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes_per_row * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
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
