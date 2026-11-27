use std::sync::mpsc;

use unitas_render::camera::{look_at, Camera};
use unitas_render::gpu::{CanvasRect, GpuMaterialInstance, RenderScene, Renderer, SurfaceFrame};
use unitas_render::math::{Mat4, Vec3};
use unitas_render::mesh::{cube, Mesh};
use unitas_render::model::{DirectionalLight, DEFAULT_MATERIAL};

use crate::render_tests::shared_renderer;

const SIZE: (u32, u32) = (96, 96);
const TOLERANCE: u8 = 1;

fn target(renderer: &Renderer, format: wgpu::TextureFormat) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = renderer.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("parity-target"),
        size: wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn read_back(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let row_bytes = u64::from(SIZE.0) * 4;
    let aligned = row_bytes.div_ceil(256) * 256;
    let buffer = renderer.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("parity-readback"),
        size: aligned * u64::from(SIZE.1),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("parity-copy"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(aligned as u32),
                rows_per_image: Some(SIZE.1),
            },
        },
        wgpu::Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue().submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    if renderer
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .is_err()
    {
        return Vec::new();
    }
    let Ok(Ok(())) = receiver.recv() else {
        return Vec::new();
    };
    let Ok(view) = slice.get_mapped_range() else {
        return Vec::new();
    };
    let mut pixels = Vec::with_capacity((SIZE.0 * SIZE.1 * 4) as usize);
    for row in 0..u64::from(SIZE.1) {
        let start = (row * aligned) as usize;
        pixels.extend_from_slice(&view[start..start + row_bytes as usize]);
    }
    drop(view);
    buffer.unmap();
    pixels
}

fn largest_difference(left: &[u8], right: &[u8]) -> u8 {
    left.iter()
        .zip(right.iter())
        .map(|(first, second)| first.abs_diff(*second))
        .max()
        .unwrap_or(0)
}

fn scene<'a>(
    mesh: &'a Mesh,
    camera: &'a Camera,
    light: &'a DirectionalLight,
) -> (Vec<GpuMaterialInstance<'a>>, RenderScene<'a>) {
    let mut values = [0.0f32; 16];
    values[0] = 1.0;
    values[5] = 1.0;
    values[10] = 1.0;
    values[14] = -3.0;
    values[15] = 1.0;
    let instances = vec![
        GpuMaterialInstance {
            mesh,
            attributes: None,
            model: Mat4::from_cols_array(values),
            color: [0.8, 0.4, 0.2],
            material: &DEFAULT_MATERIAL,
        },
        GpuMaterialInstance {
            mesh,
            attributes: None,
            model: Mat4::from_cols_array({
                let mut shifted = values;
                shifted[12] = 1.5;
                shifted
            }),
            color: [0.2, 0.6, 0.9],
            material: &DEFAULT_MATERIAL,
        },
    ];
    let scene = RenderScene {
        camera,
        view: look_at(
            Vec3::new(0.0, 1.0, -6.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
        light,
        background: [0.1, 0.2, 0.3],
    };
    (instances, scene)
}

#[test]
fn the_surface_path_shows_the_same_image_as_the_offscreen_path() {
    let Ok(renderer) = shared_renderer() else {
        return;
    };
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let light = DirectionalLight {
        direction: Vec3::new(0.4, 0.8, 0.5),
        color: [1.0, 1.0, 1.0],
        intensity: 0.85,
    };
    let (instances, scene) = scene(&mesh, &camera, &light);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let Ok(offscreen) =
        renderer.render_offscreen_materials_rgba_with_overlay(&instances, scene, SIZE, &[])
    else {
        return;
    };
    let (texture, view) = target(&renderer, format);
    let frame = SurfaceFrame {
        instances: &instances,
        scene,
        size: SIZE,
        overlay: &[],
        canvas_rects: &[],
        canvas_texts: &[],
    };
    if renderer
        .render_frame_to_view(&view, format, &frame)
        .is_err()
    {
        return;
    }
    let surface = read_back(&renderer, &texture);
    assert_eq!(surface.len(), offscreen.pixels.len());
    let difference = largest_difference(&offscreen.pixels, &surface);
    assert!(
        difference <= TOLERANCE,
        "the surface path differs from the offscreen path by {difference} of 255"
    );
}

#[test]
fn the_surface_path_shows_the_same_image_as_the_blit_path_in_the_surface_format() {
    let Ok(renderer) = shared_renderer() else {
        return;
    };
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let light = DirectionalLight {
        direction: Vec3::new(0.4, 0.8, 0.5),
        color: [1.0, 1.0, 1.0],
        intensity: 0.85,
    };
    let (instances, scene) = scene(&mesh, &camera, &light);
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let Ok(offscreen) =
        renderer.render_offscreen_materials_rgba_with_overlay(&instances, scene, SIZE, &[])
    else {
        return;
    };
    let (blit_texture, blit_view) = target(&renderer, format);
    if renderer
        .draw_rgba_to_view(&blit_view, format, SIZE, &offscreen)
        .is_err()
    {
        return;
    }
    let blit = read_back(&renderer, &blit_texture);
    let (surface_texture, surface_view) = target(&renderer, format);
    let frame = SurfaceFrame {
        instances: &instances,
        scene,
        size: SIZE,
        overlay: &[],
        canvas_rects: &[],
        canvas_texts: &[],
    };
    if renderer
        .render_frame_to_view(&surface_view, format, &frame)
        .is_err()
    {
        return;
    }
    let surface = read_back(&renderer, &surface_texture);
    assert_eq!(blit.len(), surface.len());
    let difference = largest_difference(&blit, &surface);
    assert!(
        difference <= TOLERANCE,
        "the surface path differs from the blit path by {difference} of 255"
    );
}

#[test]
fn the_surface_path_draws_the_canvas_over_the_frame() {
    let Ok(renderer) = shared_renderer() else {
        return;
    };
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let light = DirectionalLight {
        direction: Vec3::new(0.4, 0.8, 0.5),
        color: [1.0, 1.0, 1.0],
        intensity: 0.85,
    };
    let (instances, scene) = scene(&mesh, &camera, &light);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (plain_texture, plain_view) = target(&renderer, format);
    let plain_frame = SurfaceFrame {
        instances: &instances,
        scene,
        size: SIZE,
        overlay: &[],
        canvas_rects: &[],
        canvas_texts: &[],
    };
    if renderer
        .render_frame_to_view(&plain_view, format, &plain_frame)
        .is_err()
    {
        return;
    }
    let plain = read_back(&renderer, &plain_texture);
    let rects = [CanvasRect {
        rect: [10.0, 10.0, 40.0, 40.0],
        color: [1.0, 0.0, 0.0, 1.0],
        ..CanvasRect::default()
    }];
    let (canvas_texture, canvas_view) = target(&renderer, format);
    let canvas_frame = SurfaceFrame {
        instances: &instances,
        scene,
        size: SIZE,
        overlay: &[],
        canvas_rects: &rects,
        canvas_texts: &[],
    };
    if renderer
        .render_frame_to_view(&canvas_view, format, &canvas_frame)
        .is_err()
    {
        return;
    }
    let canvas = read_back(&renderer, &canvas_texture);
    let inside = ((20 * SIZE.0 + 20) * 4) as usize;
    assert_eq!(
        &canvas[inside..inside + 4],
        &[255, 0, 0, 255],
        "the canvas element was not drawn over the frame"
    );
    let outside = ((80 * SIZE.0 + 80) * 4) as usize;
    assert_eq!(
        &canvas[outside..outside + 4],
        &plain[outside..outside + 4],
        "the canvas element changed the frame outside its bounds"
    );
}

struct ParityUiItem {
    rect: [f32; 4],
    color: [f32; 4],
    sprite: Option<unitas_render::ui_pass::SpriteRef>,
    clip_rect: Option<[f32; 4]>,
    text: Option<unitas_render::ui_pass::UiText<'static>>,
}

impl unitas_render::ui_pass::UiElement for ParityUiItem {
    fn rect(&self) -> [f32; 4] {
        self.rect
    }

    fn color(&self) -> [f32; 4] {
        self.color
    }

    fn sprite(&self) -> Option<&unitas_render::ui_pass::SpriteRef> {
        self.sprite.as_ref()
    }

    fn clip_rect(&self) -> Option<[f32; 4]> {
        self.clip_rect
    }

    fn text(&self) -> Option<unitas_render::ui_pass::UiText<'_>> {
        self.text
    }
}

fn mean_channel_error(left: &[u8], right: &[u8]) -> f32 {
    if left.is_empty() || left.len() != right.len() {
        return f32::MAX;
    }
    let total: u64 = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    total as f32 / left.len() as f32
}

#[test]
fn surface_ui_sprites_aligned_wrapped_clipped_text_and_clip_rects_match_cpu_composite_ui() {
    use unitas_render::glyphs::{HorizontalAlign, VerticalAlign};
    use unitas_render::gpu::{TextOverlayItem, GPU_CPU_MEAN_ERROR_TOLERANCE};
    use unitas_render::ui_pass::{
        composite_ui, register_ui_sprite_image, GlyphTextPainter, SpriteRef, UiTarget, UiText,
    };
    use unitas_render::Rgba8;

    let Ok(renderer) = shared_renderer() else {
        return;
    };
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let light = DirectionalLight {
        direction: Vec3::new(0.4, 0.8, 0.5),
        color: [1.0, 1.0, 1.0],
        intensity: 0.85,
    };
    let (instances, scene) = scene(&mesh, &camera, &light);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let Ok(mut cpu_rgba) =
        renderer.render_offscreen_materials_rgba_with_overlay(&instances, scene, SIZE, &[])
    else {
        return;
    };

    let mut sprite_img = Rgba8::new(2, 2);
    sprite_img.pixels.copy_from_slice(&[
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ]);
    register_ui_sprite_image("parity-sprite-1", sprite_img.clone());
    let sprite_rect = [6.0, 6.0, 24.0, 24.0];
    let sprite_tint = [1.0, 0.5, 0.5, 0.8];
    let sprite_clip = [6.0, 6.0, 12.0, 24.0];
    let solid_rect = [36.0, 6.0, 24.0, 24.0];
    let solid_color = [0.2, 0.8, 0.4, 0.75];
    let solid_clip = [36.0, 6.0, 24.0, 12.0];
    let text_rect = [8.0, 36.0, 80.0, 26.0];
    let text_clip = [8.0, 36.0, 80.0, 20.0];
    let text_color = [1.0, 0.95, 0.2, 1.0];

    let cpu_items = [
        ParityUiItem {
            rect: sprite_rect,
            color: sprite_tint,
            sprite: Some(SpriteRef {
                texture_guid: "parity-sprite-1".to_string(),
                uv_rect: [0.0, 0.0, 1.0, 1.0],
            }),
            clip_rect: Some(sprite_clip),
            text: None,
        },
        ParityUiItem {
            rect: solid_rect,
            color: solid_color,
            sprite: None,
            clip_rect: Some(solid_clip),
            text: None,
        },
        ParityUiItem {
            rect: text_rect,
            color: text_color,
            sprite: None,
            clip_rect: None,
            text: Some(UiText {
                text: "HELLO WORLD\nCLIPPED LINE THREE",
                x: text_rect[0],
                y: text_rect[1],
                size_px: 14.0,
                color: text_color,
                horizontal_align: HorizontalAlign::Center,
                vertical_align: VerticalAlign::Top,
                wrap: true,
                max_width: Some(text_rect[2]),
                line_spacing: 1.1,
                clip_rect: Some(text_clip),
            }),
        },
    ];
    let mut painter = GlyphTextPainter::new();
    let mut cpu_target = UiTarget {
        image: &mut cpu_rgba,
        ids: None,
    };
    composite_ui(&mut cpu_target, &cpu_items, 1, Some(&mut painter));

    let gpu_rects = [
        CanvasRect {
            rect: sprite_rect,
            color: sprite_tint,
            sprite: Some(sprite_img),
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect: Some(sprite_clip),
        },
        CanvasRect {
            rect: solid_rect,
            color: solid_color,
            sprite: None,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            clip_rect: Some(solid_clip),
        },
    ];
    let gpu_texts = [TextOverlayItem {
        text: "HELLO WORLD\nCLIPPED LINE THREE".to_string(),
        x: text_rect[0],
        y: text_rect[1],
        size_px: 14.0,
        color: text_color,
        rect: text_rect,
        horizontal_align: HorizontalAlign::Center,
        vertical_align: VerticalAlign::Top,
        wrap: true,
        max_width: Some(text_rect[2]),
        line_spacing: 1.1,
        clip_rect: Some(text_clip),
    }];
    let (surface_texture, surface_view) = target(&renderer, format);
    let frame = SurfaceFrame {
        instances: &instances,
        scene,
        size: SIZE,
        overlay: &[],
        canvas_rects: &gpu_rects,
        canvas_texts: &gpu_texts,
    };
    if renderer
        .render_frame_to_view(&surface_view, format, &frame)
        .is_err()
    {
        return;
    }
    let gpu_pixels = read_back(&renderer, &surface_texture);
    assert_eq!(gpu_pixels.len(), cpu_rgba.pixels.len());

    let unclipped_sprite_px = ((10 * SIZE.0 + 10) * 4) as usize;
    let clipped_sprite_px = ((10 * SIZE.0 + 24) * 4) as usize;
    assert_eq!(
        &gpu_pixels[unclipped_sprite_px..unclipped_sprite_px + 3],
        &cpu_rgba.pixels[unclipped_sprite_px..unclipped_sprite_px + 3]
    );
    assert_eq!(
        &gpu_pixels[clipped_sprite_px..clipped_sprite_px + 3],
        &cpu_rgba.pixels[clipped_sprite_px..clipped_sprite_px + 3]
    );
    assert_ne!(
        &gpu_pixels[unclipped_sprite_px..unclipped_sprite_px + 3],
        &gpu_pixels[clipped_sprite_px..clipped_sprite_px + 3]
    );

    let unclipped_solid_px = ((10 * SIZE.0 + 44) * 4) as usize;
    let clipped_solid_px = ((24 * SIZE.0 + 44) * 4) as usize;
    assert_eq!(
        &gpu_pixels[unclipped_solid_px..unclipped_solid_px + 3],
        &cpu_rgba.pixels[unclipped_solid_px..unclipped_solid_px + 3]
    );
    assert_eq!(
        &gpu_pixels[clipped_solid_px..clipped_solid_px + 3],
        &cpu_rgba.pixels[clipped_solid_px..clipped_solid_px + 3]
    );

    for y in 57..SIZE.1 {
        for x in 8..88 {
            let idx = ((y * SIZE.0 + x) * 4) as usize;
            assert_eq!(
                &gpu_pixels[idx..idx + 3],
                &cpu_rgba.pixels[idx..idx + 3],
                "overflowing text ink escaped clip_rect at ({x}, {y})"
            );
        }
    }

    let mean_error = mean_channel_error(&cpu_rgba.pixels, &gpu_pixels);
    assert!(
        mean_error <= GPU_CPU_MEAN_ERROR_TOLERANCE,
        "GPU surface UI mean error {mean_error} exceeds tolerance {GPU_CPU_MEAN_ERROR_TOLERANCE}"
    );
}
