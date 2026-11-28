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

fn read_texture_pixels(
    renderer: &Renderer,
    target: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
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
            texture: target,
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
    renderer.read_buffer(&readback)
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
    let data = read_texture_pixels(&renderer, &target, width, height)?;
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

fn count_changed_pixels_inside(
    without: &Image,
    with: &Image,
    bound_x: f32,
    bound_y: f32,
    bound_w: f32,
    bound_h: f32,
) -> usize {
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
    changed
}

#[test]
fn text_overlay_changes_pixels_inside_text_bounds_only() -> Result<(), Box<dyn std::error::Error>> {
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
        ..TextOverlayItem::default()
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
    let changed = count_changed_pixels_inside(&without, &with, bound_x, bound_y, bound_w, bound_h);
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
fn text_overlay_renders_letterforms_not_solid_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = match renderer_or_skip() {
        Some(renderer) => renderer,
        None => return Ok(()),
    };
    if !renderer.has_overlay_fonts() {
        return Err("the renderer loaded no fonts, so the HUD phrase cannot be verified".into());
    }
    let size = (256u32, 96u32);
    let overlay = vec![TextOverlayItem {
        text: "Score: 0/4".to_string(),
        x: 16.0,
        y: 16.0,
        size_px: 28.0,
        color: [1.0, 1.0, 1.0, 1.0],
        ..TextOverlayItem::default()
    }];
    let without = renderer.render_offscreen(&[], &camera(), view(), &light(), BACKGROUND, size)?;
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
fn imported_mesh_buffers_round_trip_and_render_offscreen() -> Result<(), Box<dyn std::error::Error>>
{
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
fn emissive_is_visible_with_a_black_albedo_and_no_light() -> Result<(), Box<dyn std::error::Error>>
{
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

fn assert_offscreen_graph_order() {
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
}

fn assert_surface_graph_order() {
    let surface_graph = RenderGraph::surface(false, false);
    assert_eq!(
        surface_graph.pass_order(),
        vec![RenderPassKind::Opaque, RenderPassKind::Transparent]
    );
    assert!(surface_graph
        .passes()
        .iter()
        .all(|pass| pass.target == RenderTargetKind::SurfaceColorDepth));
}

fn assert_capture_graph_order() {
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
}

fn blended_center_pixel(renderer: &Renderer) -> Result<[u8; 3], Box<dyn std::error::Error>> {
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
    Ok(frame.get_pixel(16, 16))
}

#[test]
fn render_graph_executes_passes_in_order_and_produces_expected_frame(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_offscreen_graph_order();
    assert_surface_graph_order();
    assert_capture_graph_order();
    let renderer = match renderer_or_skip() {
        Some(renderer) => renderer,
        None => return Ok(()),
    };
    let center = blended_center_pixel(&renderer)?;
    assert!(
        (i32::from(center[0]) - 128).abs() <= 2
            && center[1] == 0
            && (i32::from(center[2]) - 128).abs() <= 2,
        "expected blended center near [128, 0, 128], got {center:?}"
    );
    Ok(())
}

fn shadow_camera_view() -> Mat4 {
    look_at(
        Vec3::new(0.0, 3.0, -6.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    )
}

fn shadow_sun() -> DirectionalLight {
    DirectionalLight {
        direction: Vec3::new(0.0, 1.0, 0.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.8,
    }
}

fn render_with_shadows(
    renderer: &Renderer,
    instances: &[MeshInstance],
    shadow_view: Mat4,
    sun: &DirectionalLight,
) -> Result<Image, RenderError> {
    renderer.set_shadow_settings(ShadowSettings {
        enabled: true,
        soft: true,
        resolution_index: 2,
        distance: 20.0,
    });
    renderer.render_offscreen(
        instances,
        &camera(),
        shadow_view,
        sun,
        [0.0, 0.0, 0.0],
        (64, 64),
    )
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
    let shadow_view = shadow_camera_view();
    let sun = shadow_sun();
    let without_shadows = renderer.render_offscreen(
        &instances,
        &camera(),
        shadow_view,
        &sun,
        [0.0, 0.0, 0.0],
        (64, 64),
    )?;
    assert_eq!(renderer.shadow_map_size(), 0);

    let with_shadows = render_with_shadows(&renderer, &instances, shadow_view, &sun)?;
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
        present_mode_for_v_sync_count(0, &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox]),
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

#[test]
fn pack_globals_uniform_fits_slot_budget_and_caps_lights_at_max() {
    assert!((GLOBAL_FLOATS * 4) as u64 <= GLOBAL_SLOT);
    assert_eq!(GLOBAL_SLOT % UNIFORM_SLOT, 0);
    let dir_light = light();
    let many_lights = [GpuLight::point(Vec3::new(1.0, 2.0, 3.0), [0.5, 0.6, 0.7], 2.0, 10.0); 10];
    let packed = pack_globals_uniform(
        Mat4::identity(),
        Vec3::new(0.0, 1.0, -5.0),
        &dir_light,
        &many_lights,
        [0.1, 0.2, 0.3],
        ShadowSettings::disabled(),
        0,
    );
    assert!((packed[48] - 0.1).abs() < 1e-6);
    assert!((packed[49] - 0.2).abs() < 1e-6);
    assert!((packed[50] - 0.3).abs() < 1e-6);
    assert!((packed[51] - MAX_GPU_LIGHTS as f32).abs() < 1e-6);
    assert!((packed[52] - 1.0).abs() < 1e-6);
    assert!((packed[53] - 2.0).abs() < 1e-6);
    assert!((packed[54] - 3.0).abs() < 1e-6);
    assert!((packed[55] - 10.0).abs() < 1e-6);
    assert!((packed[59] - 1.0).abs() < 1e-6);
    assert!((packed[60] - 0.5).abs() < 1e-6);
    assert!((packed[61] - 0.6).abs() < 1e-6);
    assert!((packed[62] - 0.7).abs() < 1e-6);
    assert!((packed[63] - 2.0).abs() < 1e-6);
}

fn make_spot_light() -> GpuLight {
    GpuLight::spot(
        Vec3::new(0.0, 0.0, -3.0),
        Vec3::new(0.0, 0.0, -1.0),
        [0.0, 0.0, 1.0],
        0.8,
        10.0,
        20.0,
        45.0,
    )
}

fn assert_legacy_and_scene_renders_match(
    renderer: &Renderer,
    instances: &[MeshInstance],
    cam: &Camera,
    cam_view: Mat4,
    light: &DirectionalLight,
) -> Result<(), Box<dyn std::error::Error>> {
    let single_legacy =
        renderer.render_offscreen(instances, cam, cam_view, light, [0.0, 0.0, 0.0], (64, 64))?;
    let single_scene = renderer.render_offscreen_scene(
        instances,
        RenderScene {
            camera: cam,
            view: cam_view,
            light,
            background: [0.0, 0.0, 0.0],
        },
        64,
        64,
    )?;
    assert_eq!(single_legacy.pixels, single_scene.pixels);
    Ok(())
}

fn render_with_scene_lights(
    renderer: &Renderer,
    instances: &[MeshInstance],
    scene: RenderScene<'_>,
    lights: &[GpuLight],
    ambient: [f32; 3],
) -> Result<Image, RenderError> {
    renderer.render_offscreen_scene_lights(instances, scene, lights, ambient, 64, 64)
}

fn assert_scene_light_accumulation(
    renderer: &Renderer,
    instances: &[MeshInstance],
    scene: RenderScene<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dark = render_with_scene_lights(renderer, instances, scene, &[], [0.0, 0.0, 0.0])?;
    assert_eq!(dark.get_pixel(32, 32), [0, 0, 0]);
    let with_ambient = render_with_scene_lights(renderer, instances, scene, &[], [0.2, 0.0, 0.0])?;
    let ambient_px = with_ambient.get_pixel(32, 32);
    assert!(ambient_px[0] >= 48 && ambient_px[1] == 0 && ambient_px[2] == 0);
    let point_light = GpuLight::point(Vec3::new(0.0, 0.0, -3.0), [0.0, 1.0, 0.0], 0.8, 10.0);
    let with_point =
        render_with_scene_lights(renderer, instances, scene, &[point_light], [0.2, 0.0, 0.0])?;
    let point_px = with_point.get_pixel(32, 32);
    assert!(point_px[0] >= 48 && point_px[1] > 100 && point_px[2] == 0);
    let spot_light = make_spot_light();
    let all_lights = [point_light, spot_light];
    let with_all =
        render_with_scene_lights(renderer, instances, scene, &all_lights, [0.2, 0.0, 0.0])?;
    let all_px = with_all.get_pixel(32, 32);
    assert!(all_px[0] >= 48 && all_px[1] > 100 && all_px[2] > 100);
    Ok(())
}

#[test]
fn point_spot_and_ambient_lights_accumulate_and_empty_matches_single_directional(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = match renderer_or_skip() {
        Some(renderer) => renderer,
        None => return Ok(()),
    };
    let cube_mesh = cube(2.0);
    let instances = [MeshInstance {
        mesh: &cube_mesh,
        model: Mat4::identity(),
        color: [1.0, 1.0, 1.0],
    }];
    let cam = camera();
    let cam_view = view();
    let dir_light = DirectionalLight {
        direction: Vec3::new(0.0, 1.0, 0.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.4,
    };
    assert_legacy_and_scene_renders_match(&renderer, &instances, &cam, cam_view, &dir_light)?;
    let zero_dir = DirectionalLight {
        direction: Vec3::new(0.0, 1.0, 0.0),
        color: [0.0, 0.0, 0.0],
        intensity: 1.0,
    };
    let zero_scene = RenderScene {
        camera: &cam,
        view: cam_view,
        light: &zero_dir,
        background: [0.0, 0.0, 0.0],
    };
    assert_scene_light_accumulation(&renderer, &instances, zero_scene)?;
    renderer.set_scene_lighting(&[], [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn same_texture_sprites_batch_into_one_draw_and_texture_change_breaks_the_run(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    let renderer = match renderer_or_skip() {
        Some(renderer) => renderer,
        None => return Ok(()),
    };
    let texture = Arc::new(Image::new(2, 2));
    let other = Arc::new(Image::new(2, 2));
    let make = |texture: &Arc<Image>, order: i32| Sprite {
        texture: Arc::clone(texture),
        model: model(Vec3::new(f32::from(order as i16), 0.0, 0.0), 1.0),
        color: [1.0, 1.0, 1.0, 1.0],
        sorting_layer: 0,
        sorting_order: order,
    };
    let batched = vec![make(&texture, 0), make(&texture, 1), make(&texture, 2)];
    let runs = renderer.prepare_sprite_runs(&batched)?;
    assert_eq!(
        runs.len(),
        1,
        "expected one batched draw for the texture run"
    );
    assert_eq!(
        runs[0].mesh.index_count, 18,
        "expected three expanded quads"
    );

    let split = vec![make(&texture, 0), make(&other, 1), make(&texture, 2)];
    let runs = renderer.prepare_sprite_runs(&split)?;
    assert_eq!(runs.len(), 3, "expected a texture change to break the run");
    assert!(runs.iter().all(|run| run.mesh.index_count == 6));
    Ok(())
}

#[test]
fn batched_sprites_render_expected_pixels() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    let renderer = match renderer_or_skip() {
        Some(renderer) => renderer,
        None => return Ok(()),
    };
    let mut texture = Image::new(2, 2);
    for pixel in &mut texture.pixels {
        *pixel = [255, 255, 255];
    }
    let texture = Arc::new(texture);
    let make = |x: f32, z: f32, color: [f32; 4]| Sprite {
        texture: Arc::clone(&texture),
        model: model(Vec3::new(x, 0.0, z), 1.0),
        color,
        sorting_layer: 0,
        sorting_order: 0,
    };
    let sprites = vec![
        make(-0.75, -0.3, [1.0, 1.0, 1.0, 1.0]),
        make(0.0, 0.0, [1.0, 0.0, 0.0, 1.0]),
        make(0.75, -0.3, [1.0, 1.0, 1.0, 1.0]),
    ];
    renderer.set_sprites(sprites);
    let image = renderer.render_offscreen(
        &[],
        &Camera::orthographic(2.0, 1.0, 0.1, 100.0),
        view(),
        &DirectionalLight {
            direction: Vec3::new(0.0, 0.0, -1.0),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        },
        [0.0, 0.0, 0.0],
        (64, 64),
    )?;
    renderer.set_sprites(Vec::new());
    assert_eq!(image.get_pixel(16, 32), [255, 255, 255]);
    assert_eq!(image.get_pixel(26, 32), [255, 0, 0]);
    assert_eq!(image.get_pixel(32, 32), [255, 0, 0]);
    assert_eq!(image.get_pixel(38, 32), [255, 255, 255]);
    assert_eq!(image.get_pixel(48, 32), [255, 255, 255]);
    assert_eq!(image.get_pixel(0, 0), [0, 0, 0]);
    Ok(())
}
