use std::collections::HashMap;

use cosmic_text::fontdb;
pub use cosmic_text::SwashCache;
use cosmic_text::{Attrs, Buffer, CacheKey, FontSystem, Metrics, Shaping};

use crate::image::Rgba8;

const ATLAS_PADDING: u32 = 1;

#[derive(Debug)]
pub struct FontLibrary {
    system: FontSystem,
}

impl FontLibrary {
    pub fn new() -> Self {
        Self {
            system: FontSystem::new_with_locale_and_db(
                "en-US".to_string(),
                fontdb::Database::new(),
            ),
        }
    }

    pub fn add_font(&mut self, bytes: Vec<u8>) -> bool {
        let had_fonts = !self.system.db().is_empty();
        self.system.db_mut().load_font_data(bytes);
        had_fonts || !self.system.db().is_empty()
    }

    pub fn load_system_fonts(&mut self) -> bool {
        let had_fonts = !self.system.db().is_empty();
        self.system.db_mut().load_system_fonts();
        had_fonts || !self.system.db().is_empty()
    }

    pub fn has_fonts(&self) -> bool {
        !self.system.db().is_empty()
    }
}

impl Default for FontLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub color: [f32; 4],
}

impl TextStyle {
    pub const fn new(font_size: f32, line_height: f32, color: [f32; 4]) -> Self {
        Self {
            font_size,
            line_height,
            color,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AtlasKey {
    font_id: fontdb::ID,
    glyph_id: u16,
    font_size_bits: u32,
}

impl AtlasKey {
    fn from_cache_key(key: CacheKey) -> Self {
        Self {
            font_id: key.font_id,
            glyph_id: key.glyph_id,
            font_size_bits: key.font_size_bits,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    pub cache_key: CacheKey,
    pub x: i32,
    pub y: i32,
    pub baseline_y: f32,
    pub width: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextLayout {
    pub width: f32,
    pub height: f32,
    pub glyphs: Vec<ShapedGlyph>,
}

pub fn layout_text(
    fonts: &mut FontLibrary,
    text: &str,
    max_width: Option<f32>,
    style: &TextStyle,
) -> TextLayout {
    let font_size = if style.font_size > 0.0 {
        style.font_size
    } else {
        1.0
    };
    let mut buffer = Buffer::new(
        &mut fonts.system,
        Metrics::new(font_size, style.line_height),
    );
    buffer.set_size(&mut fonts.system, max_width, None);
    buffer.set_text(&mut fonts.system, text, Attrs::new(), Shaping::Advanced);

    let mut layout = TextLayout::default();
    for run in buffer.layout_runs() {
        layout.height = layout.height.max(run.line_top + run.line_height);
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let color = glyph
                .color_opt
                .map(|c| {
                    let [r, g, b, a] = c.as_rgba();
                    [
                        f32::from(r) / 255.0,
                        f32::from(g) / 255.0,
                        f32::from(b) / 255.0,
                        f32::from(a) / 255.0,
                    ]
                })
                .unwrap_or(style.color);
            layout.width = layout.width.max(glyph.x + glyph.w);
            layout.glyphs.push(ShapedGlyph {
                cache_key: physical.cache_key,
                x: physical.x,
                y: physical.y,
                baseline_y: run.line_y,
                width: glyph.w,
                color,
            });
        }
    }
    layout
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAtlas {
    pub image: Rgba8,
    regions: HashMap<AtlasKey, AtlasRegion>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            image: Rgba8::new(width, height),
            regions: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    pub fn pack(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        if width == 0 || height == 0 || width > self.image.width || height > self.image.height {
            return None;
        }
        if self.cursor_x + width > self.image.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_height + ATLAS_PADDING;
            self.row_height = 0;
        }
        if self.cursor_y + height > self.image.height {
            return None;
        }
        let region = AtlasRegion {
            x: self.cursor_x,
            y: self.cursor_y,
            width,
            height,
        };
        self.cursor_x += width + ATLAS_PADDING;
        self.row_height = self.row_height.max(height);
        Some(region)
    }

    pub fn blit_mask(&mut self, region: AtlasRegion, mask: &[u8]) {
        for row in 0..region.height {
            for col in 0..region.width {
                let Some(&alpha) = mask.get((row * region.width + col) as usize) else {
                    break;
                };
                let offset = ((region.y + row) * self.image.width + region.x + col) as usize * 4;
                self.image.pixels[offset] = 255;
                self.image.pixels[offset + 1] = 255;
                self.image.pixels[offset + 2] = 255;
                self.image.pixels[offset + 3] = alpha;
            }
        }
    }

    pub fn uv(&self, region: AtlasRegion) -> [f32; 4] {
        let width = self.image.width as f32;
        let height = self.image.height as f32;
        [
            region.x as f32 / width,
            region.y as f32 / height,
            (region.x + region.width) as f32 / width,
            (region.y + region.height) as f32 / height,
        ]
    }

    pub fn region(&self, key: &CacheKey) -> Option<AtlasRegion> {
        self.regions.get(&AtlasKey::from_cache_key(*key)).copied()
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TexturedQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: [f32; 4],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn rasterize(
    fonts: &mut FontLibrary,
    cache: &mut SwashCache,
    atlas: &mut GlyphAtlas,
    layout: &TextLayout,
) -> Vec<TexturedQuad> {
    let mut quads = Vec::new();
    for glyph in &layout.glyphs {
        let key = AtlasKey::from_cache_key(glyph.cache_key);
        let Some(image) = cache.get_image(&mut fonts.system, glyph.cache_key).clone() else {
            continue;
        };
        if image.placement.width == 0 || image.placement.height == 0 {
            continue;
        }
        let region = match atlas.regions.get(&key) {
            Some(region) => *region,
            None => {
                let Some(region) = atlas.pack(image.placement.width, image.placement.height) else {
                    continue;
                };
                atlas.blit_mask(region, &image.data);
                atlas.regions.insert(key, region);
                region
            }
        };
        quads.push(TexturedQuad {
            x: (glyph.x + image.placement.left) as f32,
            y: glyph.baseline_y + glyph.y as f32 - image.placement.top as f32,
            width: image.placement.width as f32,
            height: image.placement.height as f32,
            uv: atlas.uv(region),
            color: glyph.color,
        });
    }
    quads
}

pub fn draw_text(frame: &mut Rgba8, atlas: &GlyphAtlas, quads: &[TexturedQuad], clip: Rect) {
    if frame.width == 0 || frame.height == 0 {
        return;
    }
    let clip_right = clip.x + clip.width;
    let clip_bottom = clip.y + clip.height;
    for quad in quads {
        if quad.width <= 0.0 || quad.height <= 0.0 {
            continue;
        }
        let x0 = quad.x.max(clip.x);
        let y0 = quad.y.max(clip.y);
        let x1 = (quad.x + quad.width).min(clip_right);
        let y1 = (quad.y + quad.height).min(clip_bottom);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let start_x = x0.max(0.0).ceil() as u32;
        let start_y = y0.max(0.0).ceil() as u32;
        let end_x = x1.min(frame.width as f32).floor() as u32;
        let end_y = y1.min(frame.height as f32).floor() as u32;
        for py in start_y..end_y {
            let v = (py as f32 + 0.5 - quad.y) / quad.height;
            let src_y =
                ((quad.uv[1] + v * (quad.uv[3] - quad.uv[1])) * atlas.image.height as f32) as u32;
            for px in start_x..end_x {
                let u = (px as f32 + 0.5 - quad.x) / quad.width;
                let src_x = ((quad.uv[0] + u * (quad.uv[2] - quad.uv[0]))
                    * atlas.image.width as f32) as u32;
                let src = atlas.image.get_pixel(src_x, src_y);
                let alpha = (f32::from(src[3]) / 255.0) * quad.color[3];
                if alpha <= 0.0 {
                    continue;
                }
                let dst_offset = ((py * frame.width + px) as usize) * 4;
                for (channel, src_byte) in src.iter().enumerate().take(3) {
                    let src_channel = (f32::from(*src_byte) / 255.0) * quad.color[channel] * alpha;
                    let dst = f32::from(frame.pixels[dst_offset + channel]) / 255.0;
                    let blended = src_channel + dst * (1.0 - alpha);
                    frame.pixels[dst_offset + channel] = (blended * 255.0).round() as u8;
                }
                frame.pixels[dst_offset + 3] = 255;
            }
        }
    }
}

#[cfg(feature = "device")]
const TEXT_SHADER_SOURCE: &str = r"
struct Globals {
    resolution: vec2<f32>,
};

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> globals: Globals;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexIn) -> VertexOut {
    var out: VertexOut;
    let clip_x = vertex.position.x / globals.resolution.x * 2.0 - 1.0;
    let clip_y = 1.0 - vertex.position.y / globals.resolution.y * 2.0;
    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.texcoord = vertex.texcoord;
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_main(fs_in: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, fs_in.texcoord);
    let alpha = texel.a * fs_in.color.a;
    return vec4<f32>(fs_in.color.rgb * texel.rgb * alpha, alpha);
}
";

#[cfg(feature = "device")]
const TEXT_VERTEX_FLOATS: usize = 8;

#[cfg(feature = "device")]
const TEXT_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 16,
        shader_location: 2,
    },
];

#[cfg(feature = "device")]
const TEXT_VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (TEXT_VERTEX_FLOATS * 4) as u64,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &TEXT_VERTEX_ATTRIBUTES,
};

#[cfg(feature = "device")]
pub struct GlyphResources {
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    pub uniform: wgpu::Buffer,
}

#[cfg(feature = "device")]
pub struct TextFrame<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub frame: Rect,
    pub clip: Rect,
}

#[cfg(feature = "device")]
pub struct TextPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[cfg(feature = "device")]
impl TextPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text"),
            source: wgpu::ShaderSource::Wgsl(TEXT_SHADER_SOURCE.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text"),
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(TEXT_VERTEX_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn upload_atlas(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: &GlyphAtlas,
    ) -> GlyphResources {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: atlas.image.width.max(1),
                height: atlas.image.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for row in 0..atlas.image.height {
            let start = (row * atlas.image.width) as usize * 4;
            let end = start + (atlas.image.width as usize) * 4;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: row, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &atlas.image.pixels[start..end],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: None,
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: atlas.image.width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph_atlas"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_globals"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_atlas"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        GlyphResources {
            texture,
            bind_group,
            uniform,
        }
    }

    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &GlyphResources,
        target: &mut TextFrame<'_>,
        quads: &[TexturedQuad],
    ) {
        let frame_w = target.frame.width;
        let frame_h = target.frame.height;
        if frame_w <= 0.0 || frame_h <= 0.0 {
            return;
        }
        let resolution = [frame_w, frame_h];
        queue.write_buffer(&resources.uniform, 0, bytemuck::cast_slice(&resolution));
        let (vertices, indices) = build_geometry(quads);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_vertices"),
            size: ((vertices.len().max(1)) * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !vertices.is_empty() {
            queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_indices"),
            size: ((indices.len().max(1)) * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !indices.is_empty() {
            queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
        }
        let scissor_x = target.clip.x.max(0.0).min(frame_w) as u32;
        let scissor_y = target.clip.y.max(0.0).min(frame_h) as u32;
        let scissor_w = (target.clip.x + target.clip.width)
            .max(0.0)
            .min(frame_w)
            .max(scissor_x as f32) as u32
            - scissor_x;
        let scissor_h = (target.clip.y + target.clip.height)
            .max(0.0)
            .min(frame_h)
            .max(scissor_y as f32) as u32
            - scissor_y;
        let mut pass = target
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &resources.bind_group, &[]);
        pass.set_scissor_rect(scissor_x, scissor_y, scissor_w.max(1), scissor_h.max(1));
        if !indices.is_empty() {
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
    }
}

#[cfg(feature = "device")]
pub fn build_geometry(quads: &[TexturedQuad]) -> (Vec<f32>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(quads.len() * 4 * TEXT_VERTEX_FLOATS);
    let mut indices = Vec::with_capacity(quads.len() * 6);
    for (quad_index, quad) in quads.iter().enumerate() {
        let base = (quad_index * 4) as u32;
        let corners = [
            (quad.x, quad.y, quad.uv[0], quad.uv[1]),
            (quad.x + quad.width, quad.y, quad.uv[2], quad.uv[1]),
            (
                quad.x + quad.width,
                quad.y + quad.height,
                quad.uv[2],
                quad.uv[3],
            ),
            (quad.x, quad.y + quad.height, quad.uv[0], quad.uv[3]),
        ];
        for (x, y, u, v) in corners {
            vertices.extend_from_slice(&[x, y, u, v]);
            vertices.extend_from_slice(&quad.color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_mask(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        vec![alpha; (width * height) as usize]
    }

    #[test]
    fn atlas_pack_fills_rows_then_wraps() {
        let mut atlas = GlyphAtlas::new(16, 16);
        let first = atlas.pack(8, 4);
        let second = atlas.pack(7, 6);
        let third = atlas.pack(8, 4);
        assert_eq!(
            first,
            Some(AtlasRegion {
                x: 0,
                y: 0,
                width: 8,
                height: 4
            })
        );
        assert_eq!(
            second,
            Some(AtlasRegion {
                x: 9,
                y: 0,
                width: 7,
                height: 6
            })
        );
        assert_eq!(
            third,
            Some(AtlasRegion {
                x: 0,
                y: 7,
                width: 8,
                height: 4
            })
        );
        assert_eq!(atlas.pack(17, 1), None);
        assert_eq!(atlas.pack(1, 17), None);
    }

    #[test]
    fn atlas_pack_reports_overflow() {
        let mut atlas = GlyphAtlas::new(8, 4);
        assert!(atlas.pack(8, 4).is_some());
        assert_eq!(atlas.pack(1, 1), None);
        assert_eq!(atlas.pack(0, 1), None);
    }

    #[test]
    fn atlas_blit_writes_alpha_mask() -> Result<(), Box<dyn std::error::Error>> {
        let mut atlas = GlyphAtlas::new(4, 4);
        let region = atlas.pack(2, 2).ok_or("pack failed")?;
        atlas.blit_mask(region, &solid_mask(2, 2, 128));
        let pixel = atlas.image.get_pixel(1, 1);
        assert_eq!(pixel, [255, 255, 255, 128]);
        let outside = atlas.image.get_pixel(3, 3);
        assert_eq!(outside, [0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn atlas_uv_maps_region() {
        let atlas = GlyphAtlas::new(8, 4);
        let uv = atlas.uv(AtlasRegion {
            x: 2,
            y: 1,
            width: 2,
            height: 2,
        });
        assert_eq!(uv, [0.25, 0.25, 0.5, 0.75]);
    }

    fn synthetic_atlas() -> Result<(GlyphAtlas, AtlasRegion), Box<dyn std::error::Error>> {
        let mut atlas = GlyphAtlas::new(4, 4);
        let region = atlas.pack(2, 2).ok_or("pack failed")?;
        atlas.blit_mask(region, &solid_mask(2, 2, 255));
        Ok((atlas, region))
    }

    fn solid_quad(x: f32, y: f32, size: f32, uv: [f32; 4], color: [f32; 4]) -> TexturedQuad {
        TexturedQuad {
            x,
            y,
            width: size,
            height: size,
            uv,
            color,
        }
    }

    #[test]
    fn draw_text_applies_quad_color() -> Result<(), Box<dyn std::error::Error>> {
        let (atlas, region) = synthetic_atlas()?;
        let uv = atlas.uv(region);
        let quad = solid_quad(0.0, 0.0, 2.0, uv, [1.0, 0.0, 0.0, 1.0]);
        let mut frame = Rgba8::new(2, 2);
        draw_text(
            &mut frame,
            &atlas,
            &[quad],
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 2.0,
            },
        );
        assert_eq!(frame.get_pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(frame.get_pixel(1, 1), [255, 0, 0, 255]);
        Ok(())
    }

    #[test]
    fn draw_text_honors_clip_rect() -> Result<(), Box<dyn std::error::Error>> {
        let (atlas, region) = synthetic_atlas()?;
        let uv = atlas.uv(region);
        let quad = solid_quad(0.0, 0.0, 4.0, uv, [0.0, 0.0, 1.0, 1.0]);
        let mut frame = Rgba8::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                let offset = ((y * 4 + x) * 4) as usize;
                frame.pixels[offset] = 10;
                frame.pixels[offset + 1] = 20;
                frame.pixels[offset + 2] = 30;
                frame.pixels[offset + 3] = 255;
            }
        }
        draw_text(
            &mut frame,
            &atlas,
            &[quad],
            Rect {
                x: 1.0,
                y: 1.0,
                width: 1.0,
                height: 1.0,
            },
        );
        assert_eq!(frame.get_pixel(1, 1), [0, 0, 255, 255]);
        assert_eq!(frame.get_pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(frame.get_pixel(3, 3), [10, 20, 30, 255]);
        assert_eq!(frame.get_pixel(2, 1), [10, 20, 30, 255]);
        Ok(())
    }

    #[test]
    fn draw_text_alpha_blends_over_background() -> Result<(), Box<dyn std::error::Error>> {
        let mut atlas = GlyphAtlas::new(4, 4);
        let region = atlas.pack(2, 2).ok_or("pack failed")?;
        atlas.blit_mask(region, &solid_mask(2, 2, 128));
        let uv = atlas.uv(region);
        let quad = solid_quad(0.0, 0.0, 2.0, uv, [1.0, 1.0, 1.0, 1.0]);
        let mut frame = Rgba8::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                let offset = ((y * 2 + x) * 4) as usize;
                frame.pixels[offset] = 100;
                frame.pixels[offset + 1] = 100;
                frame.pixels[offset + 2] = 100;
                frame.pixels[offset + 3] = 255;
            }
        }
        draw_text(
            &mut frame,
            &atlas,
            &[quad],
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 2.0,
            },
        );
        let pixel = frame.get_pixel(0, 0);
        assert_eq!(pixel[0], 178);
        Ok(())
    }

    #[test]
    fn layout_of_empty_text_has_no_glyphs() {
        let mut fonts = FontLibrary::new();
        let style = TextStyle::new(16.0, 20.0, [1.0, 1.0, 1.0, 1.0]);
        let layout = layout_text(&mut fonts, "", Some(100.0), &style);
        assert!(layout.glyphs.is_empty());
        assert_eq!(layout.width, 0.0);
        assert_eq!(layout.height, 20.0);
    }

    #[test]
    fn shaping_layout_and_atlas_are_deterministic() {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return;
        }
        let style = TextStyle::new(24.0, 30.0, [1.0, 1.0, 1.0, 1.0]);
        let first = layout_text(&mut fonts, "Hello, world!", None, &style);
        let second = layout_text(&mut fonts, "Hello, world!", None, &style);
        assert_eq!(first, second);
        assert!(!first.glyphs.is_empty());
        assert!(first.width > 0.0);
        assert!(first.height > 0.0);

        let wide = layout_text(&mut fonts, "hello world hello", Some(10000.0), &style);
        let narrow = layout_text(&mut fonts, "hello world hello", Some(60.0), &style);
        let wide_lines = wide
            .glyphs
            .iter()
            .map(|g| g.baseline_y.to_bits())
            .collect::<std::collections::BTreeSet<_>>();
        let narrow_lines = narrow
            .glyphs
            .iter()
            .map(|g| g.baseline_y.to_bits())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(wide_lines.len(), 1);
        assert!(narrow_lines.len() > 1);

        let mut cache = SwashCache::new();
        let mut atlas_a = GlyphAtlas::new(256, 256);
        let quads_a = rasterize(&mut fonts, &mut cache, &mut atlas_a, &first);
        let mut atlas_b = GlyphAtlas::new(256, 256);
        let quads_b = rasterize(&mut fonts, &mut cache, &mut atlas_b, &first);
        assert!(!quads_a.is_empty());
        assert_eq!(quads_a, quads_b);
        assert_eq!(atlas_a, atlas_b);
        assert!(atlas_a.image.pixels.chunks_exact(4).any(|px| px[3] > 0));
    }

    #[test]
    fn smoke_rasterizes_one_glyph_into_atlas() -> Result<(), Box<dyn std::error::Error>> {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return Ok(());
        }
        let style = TextStyle::new(32.0, 40.0, [1.0, 1.0, 1.0, 1.0]);
        let layout = layout_text(&mut fonts, "A", None, &style);
        assert_eq!(layout.glyphs.len(), 1);
        let mut cache = SwashCache::new();
        let mut atlas = GlyphAtlas::new(64, 64);
        let quads = rasterize(&mut fonts, &mut cache, &mut atlas, &layout);
        assert_eq!(quads.len(), 1);
        let region = atlas
            .region(&layout.glyphs[0].cache_key)
            .ok_or("the atlas holds no region for the rasterized glyph")?;
        assert!(region.width > 0 && region.height > 0);
        assert!(atlas.image.pixels.chunks_exact(4).any(|px| px[3] > 0));
        Ok(())
    }

    #[test]
    fn rasterized_phrase_coverage_is_letterform_like() -> Result<(), Box<dyn std::error::Error>> {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return Err("no system fonts are available to rasterize the HUD phrase".into());
        }
        let style = TextStyle::new(28.0, 28.0, [1.0, 1.0, 1.0, 1.0]);
        let layout = layout_text(&mut fonts, "Score: 0/4", None, &style);
        let mut cache = SwashCache::new();
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let quads = rasterize(&mut fonts, &mut cache, &mut atlas, &layout);
        assert!(!quads.is_empty(), "the phrase produced no glyph quads");
        let columns = layout.width.ceil() as usize;
        let mut inked_columns = vec![false; columns];
        let mut inked = 0usize;
        let mut blank = 0usize;
        let mut partial = 0usize;
        for quad in &quads {
            let region_x = (quad.uv[0] * atlas.image.width as f32).round() as u32;
            let region_y = (quad.uv[1] * atlas.image.height as f32).round() as u32;
            let region_width =
                ((quad.uv[2] - quad.uv[0]) * atlas.image.width as f32).round() as u32;
            let region_height =
                ((quad.uv[3] - quad.uv[1]) * atlas.image.height as f32).round() as u32;
            for row in 0..region_height {
                for col in 0..region_width {
                    let alpha = atlas.image.get_pixel(region_x + col, region_y + row)[3];
                    let column = (quad.x + col as f32).floor() as usize;
                    if column >= inked_columns.len() {
                        continue;
                    }
                    if alpha == 0 {
                        blank += 1;
                    } else {
                        inked_columns[column] = true;
                        if alpha == 255 {
                            inked += 1;
                        } else {
                            partial += 1;
                        }
                    }
                }
            }
        }
        assert!(inked + partial > 0, "the rasterized phrase carries no ink");
        assert!(
            blank > 0,
            "the rasterized phrase fills every glyph pixel, which means solid blocks"
        );
        assert!(
            partial > 0,
            "the rasterized mask is uniformly opaque, which means solid blocks"
        );
        assert!(
            inked_columns.iter().any(|column| *column),
            "no text column carries ink"
        );
        assert!(
            inked_columns.iter().any(|column| !*column),
            "every text column carries ink, which means solid blocks"
        );
        Ok(())
    }

    #[test]
    fn rasterize_reuses_atlas_regions() {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return;
        }
        let style = TextStyle::new(18.0, 24.0, [1.0, 1.0, 1.0, 1.0]);
        let layout = layout_text(&mut fonts, "aaaa", None, &style);
        let mut cache = SwashCache::new();
        let mut atlas = GlyphAtlas::new(256, 256);
        let quads_first = rasterize(&mut fonts, &mut cache, &mut atlas, &layout);
        let region_count = atlas.regions.len();
        let quads_second = rasterize(&mut fonts, &mut cache, &mut atlas, &layout);
        assert_eq!(quads_first, quads_second);
        assert_eq!(atlas.regions.len(), region_count);
        assert!(region_count < quads_first.len() || quads_first.len() <= 4);
    }
}
