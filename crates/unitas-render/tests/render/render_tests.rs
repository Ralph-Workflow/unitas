use std::ops::{Add, Sub};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use unitas_render::camera::{look_at, Camera};
use unitas_render::gpu::{GpuMaterialInstance, RenderScene, Renderer};
use unitas_render::image::Image;
use unitas_render::math::{Mat4, Vec3, Vec4};
use unitas_render::mesh::{cube, uv_sphere, Mesh, MeshAttributes};
use unitas_render::model::{DirectionalLight, Material, MeshInstance};
use unitas_render::sprite::{sort_sprites, Sprite};

pub(crate) fn shared_renderer() -> Result<MutexGuard<'static, Renderer>, String> {
    static RENDERER: OnceLock<Result<Mutex<Renderer>, String>> = OnceLock::new();
    let slot = RENDERER.get_or_init(|| {
        Renderer::new()
            .map(Mutex::new)
            .map_err(|error| error.to_string())
    });
    let mutex = match slot {
        Ok(mutex) => mutex,
        Err(message) => return Err(String::clone(message)),
    };
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => Ok(poisoned.into_inner()),
    }
}

fn shaded_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn scaled_translated_model(scale: f32, translation: Vec3) -> Mat4 {
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
fn vec3_operations_and_operators() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, -1.0, 2.0);

    assert_eq!(a.add(b), Vec3::new(5.0, 1.0, 5.0));
    assert_eq!(a + b, Vec3::new(5.0, 1.0, 5.0));
    assert_eq!(a.sub(b), Vec3::new(-3.0, 3.0, 1.0));
    assert_eq!(a - b, Vec3::new(-3.0, 3.0, 1.0));
    assert_eq!(a.scale(2.0), Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
    assert!((a.dot(b) - 8.0).abs() < 1e-6);
    assert_eq!(
        Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0)),
        Vec3::new(0.0, 0.0, 1.0)
    );

    let v = Vec3::new(3.0, 4.0, 0.0);
    assert!((v.length() - 5.0).abs() < 1e-6);
    let n = v.normalize();
    assert!((n.x - 0.6).abs() < 1e-6);
    assert!((n.y - 0.8).abs() < 1e-6);
    assert!(n.z.abs() < 1e-6);
}

#[test]
fn mat4_multiplication_and_transform() {
    let id = Mat4::identity();
    let v4 = Vec4::new(1.0, 2.0, 3.0, 1.0);
    assert_eq!(id * v4, v4);

    let mut translate_cols = *id.as_array();
    translate_cols[12] = 10.0;
    translate_cols[13] = 20.0;
    translate_cols[14] = 30.0;
    let translate = Mat4::from_cols_array(translate_cols);

    let transformed = translate.transform_point(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(transformed, Vec4::new(11.0, 22.0, 33.0, 1.0));

    let combined = translate * id;
    assert_eq!(combined, translate);
}

#[test]
fn perspective_projection_matrix_matches_unity_formulas() {
    let fov_deg: f32 = 60.0;
    let aspect: f32 = 16.0 / 9.0;
    let near: f32 = 0.1;
    let far: f32 = 100.0;

    let cam = Camera::perspective(fov_deg, aspect, near, far);
    let proj = cam.projection_matrix();
    let m = proj.as_array();

    let half_fov_tan = (fov_deg.to_radians() * 0.5).tan();
    let expected_m00 = 1.0 / (aspect * half_fov_tan);
    let expected_m11 = 1.0 / half_fov_tan;
    let expected_m22 = far / (near - far);
    let expected_m23 = (far * near) / (near - far);
    let expected_m32 = -1.0;

    assert!((m[0] - expected_m00).abs() < 1e-6);
    assert!((m[5] - expected_m11).abs() < 1e-6);
    assert!((m[10] - expected_m22).abs() < 1e-6);
    assert!((m[11] - expected_m32).abs() < 1e-6);
    assert!((m[14] - expected_m23).abs() < 1e-6);
    assert!(m[15].abs() < 1e-6);
}

#[test]
fn orthographic_projection_matrix_matches_formulas() {
    let size: f32 = 5.0;
    let aspect: f32 = 2.0;
    let near: f32 = 0.3;
    let far: f32 = 50.0;

    let cam = Camera::orthographic(size, aspect, near, far);
    let proj = cam.projection_matrix();
    let m = proj.as_array();

    let expected_m00 = 1.0 / (size * aspect);
    let expected_m11 = 1.0 / size;
    let expected_m22 = 1.0 / (near - far);
    let expected_m23 = near / (near - far);

    assert!((m[0] - expected_m00).abs() < 1e-6);
    assert!((m[5] - expected_m11).abs() < 1e-6);
    assert!((m[10] - expected_m22).abs() < 1e-6);
    assert!(m[11].abs() < 1e-6);
    assert!((m[14] - expected_m23).abs() < 1e-6);
    assert!((m[15] - 1.0).abs() < 1e-6);
}

#[test]
fn look_at_transforms_origin_to_negative_z_in_view_space() {
    let eye = Vec3::new(0.0, 0.0, -5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = look_at(eye, target, up);

    let origin_view = view.transform_point(target);
    assert!(origin_view.x.abs() < 1e-6);
    assert!(origin_view.y.abs() < 1e-6);
    assert!((origin_view.z - (-5.0)).abs() < 1e-6);
    assert!((origin_view.w - 1.0).abs() < 1e-6);

    let cam = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let clip = (cam.projection_matrix() * view).transform_point(target);
    assert!(clip.x.abs() < 1e-6);
    assert!(clip.y.abs() < 1e-6);
    assert!((clip.w - 5.0).abs() < 1e-6);
    let ndc_z = clip.z / clip.w;
    assert!((0.0..=1.0).contains(&ndc_z));
}

#[test]
fn cube_mesh_has_expected_counts_bounds_and_unit_normals() {
    let m: Mesh = cube(1.0);
    assert_eq!(m.positions.len(), 24);
    assert_eq!(m.normals.len(), 24);
    assert_eq!(m.indices.len(), 36);

    for &idx in &m.indices {
        assert!((idx as usize) < m.positions.len());
    }
    for p in &m.positions {
        assert!((p.x.abs() - 0.5).abs() < 1e-6);
        assert!((p.y.abs() - 0.5).abs() < 1e-6);
        assert!((p.z.abs() - 0.5).abs() < 1e-6);
    }
    for n in &m.normals {
        assert!((n.length() - 1.0).abs() < 1e-6);
    }
}

#[test]
fn uv_sphere_mesh_has_exact_vertex_and_index_counts_and_unit_normals() {
    let sectors = 12;
    let rings = 8;
    let radius = 2.0;
    let m = uv_sphere(radius, sectors, rings);

    let expected_vertices = ((sectors + 1) * (rings + 1)) as usize;
    let expected_indices = (sectors * rings * 6) as usize;
    assert_eq!(m.positions.len(), expected_vertices);
    assert_eq!(m.normals.len(), expected_vertices);
    assert_eq!(m.indices.len(), expected_indices);

    for &idx in &m.indices {
        assert!((idx as usize) < m.positions.len());
    }
    for (p, n) in m.positions.iter().zip(m.normals.iter()) {
        assert!((p.length() - radius).abs() < 1e-5);
        assert!((n.length() - 1.0).abs() < 1e-5);
    }
}

#[test]
fn render_offscreen_centered_cube_fills_center_pixel_and_is_deterministic(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let cam = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let color = [1.0, 0.5, 0.25];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 1.0,
    };
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let instances = [MeshInstance {
        mesh: &mesh,
        model: Mat4::identity(),
        color,
    }];
    let img1 =
        renderer.render_offscreen(&instances, &cam, view, &light, [0.0, 0.0, 0.0], (64, 64))?;
    let scene = RenderScene {
        camera: &cam,
        view,
        light: &light,
        background: [0.0, 0.0, 0.0],
    };
    let img2 = renderer.render_offscreen_scene(&instances, scene, 64, 64)?;

    let non_bg_count = img1.pixels.iter().filter(|&&px| px != [0, 0, 0]).count();
    assert!(non_bg_count > 0);
    assert_eq!(img1.get_pixel(32, 32), [255, 128, 64]);
    assert_eq!(img1.to_bmp_bytes(), img2.to_bmp_bytes());
    Ok(())
}

#[test]
fn bmp_writer_emits_valid_uncompressed_24bit_bmp_with_padded_row_stride() {
    let mut img = Image::new(3, 2);
    img.set_pixel(0, 0, [255, 0, 0]);
    img.set_pixel(1, 0, [0, 255, 0]);
    img.set_pixel(2, 0, [0, 0, 255]);
    img.set_pixel(0, 1, [10, 20, 30]);
    img.set_pixel(1, 1, [40, 50, 60]);
    img.set_pixel(2, 1, [70, 80, 90]);

    let bytes = img.to_bmp_bytes();
    assert_eq!(&bytes[0..2], b"BM");

    let row_stride = 12usize;
    let pixel_data_size = row_stride * 2;
    let total_size = 54 + pixel_data_size;
    assert_eq!(bytes.len(), total_size);

    let file_size = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize;
    let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
    let dib_size = u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]);
    let width = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
    let height = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
    let planes = u16::from_le_bytes([bytes[26], bytes[27]]);
    let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
    let compression = u32::from_le_bytes([bytes[30], bytes[31], bytes[32], bytes[33]]);

    assert_eq!(file_size, total_size);
    assert_eq!(data_offset, 54);
    assert_eq!(dib_size, 40);
    assert_eq!(width, 3);
    assert_eq!(height, 2);
    assert_eq!(planes, 1);
    assert_eq!(bpp, 24);
    assert_eq!(compression, 0);

    let bottom_row = &bytes[54..66];
    assert_eq!(bottom_row, &[30, 20, 10, 60, 50, 40, 90, 80, 70, 0, 0, 0]);
    let top_row = &bytes[66..78];
    assert_eq!(top_row, &[0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 0, 0]);

    let tmp_path: PathBuf =
        std::env::temp_dir().join(format!("unitas_render_test_{}.bmp", std::process::id()));
    assert!(img.write_bmp(&tmp_path).is_ok());
    let disk_bytes = std::fs::read(&tmp_path).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp_path);
    assert_eq!(disk_bytes, bytes);
}

#[test]
fn perspective_camera_renders_cube_center_and_background() -> Result<(), Box<dyn std::error::Error>>
{
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let model = scaled_translated_model(2.0, Vec3::new(0.0, 0.0, -1.0));
    let instances = [MeshInstance {
        mesh: &mesh,
        model,
        color: [0.8, 0.4, 0.2],
    }];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [0.9, 0.8, 0.7],
        intensity: 0.5,
    };

    let image =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.2, 0.4, 0.6], (64, 64))?;

    let front_face_normal = model
        .mul_vec4(Vec4::from_direction(Vec3::new(0.0, 0.0, -1.0)))
        .truncate()
        .normalize();
    let lambert = front_face_normal.dot(light.direction.normalize()).max(0.0);
    assert!((lambert - 1.0).abs() < 1e-6);

    let t = 0.5_f32;
    let expected_center = [
        shaded_byte(0.8 * ((1.0 - t) + 0.9 * lambert * t)),
        shaded_byte(0.4 * ((1.0 - t) + 0.8 * lambert * t)),
        shaded_byte(0.2 * ((1.0 - t) + 0.7 * lambert * t)),
    ];
    assert_eq!(expected_center, [194, 92, 43]);
    assert_eq!(image.get_pixel(32, 32), expected_center);
    assert_eq!(image.get_pixel(0, 0), [51, 102, 153]);
    Ok(())
}

#[test]
fn orthographic_camera_renders_cube_center_and_background() -> Result<(), Box<dyn std::error::Error>>
{
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let camera = Camera::orthographic(4.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let model = scaled_translated_model(2.0, Vec3::new(0.0, 0.0, -1.0));
    let instances = [MeshInstance {
        mesh: &mesh,
        model,
        color: [0.5, 0.5, 0.5],
    }];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 1.0,
    };

    let image = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.25, 0.5],
        (64, 64),
    )?;

    let lambert = 1.0_f32;
    let t = 1.0_f32;
    let expected_center = [
        shaded_byte(0.5 * ((1.0 - t) + 1.0 * lambert * t)),
        shaded_byte(0.5 * ((1.0 - t) + 1.0 * lambert * t)),
        shaded_byte(0.5 * ((1.0 - t) + 1.0 * lambert * t)),
    ];
    assert_eq!(expected_center, [128, 128, 128]);
    assert_eq!(image.get_pixel(32, 32), expected_center);
    assert_eq!(image.get_pixel(0, 0), [0, 64, 128]);
    Ok(())
}

#[test]
fn zero_intensity_light_renders_material_color() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let model = scaled_translated_model(2.0, Vec3::new(0.0, 0.0, -1.0));
    let instances = [MeshInstance {
        mesh: &mesh,
        model,
        color: [1.0, 0.5, 0.25],
    }];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [0.9, 0.8, 0.7],
        intensity: 0.0,
    };

    let image =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.0, 0.0, 0.0], (64, 64))?;

    let t = 0.0_f32;
    let expected_center = [
        shaded_byte(1.0 * ((1.0 - t) + 0.9 * 1.0 * t)),
        shaded_byte(0.5 * ((1.0 - t) + 0.8 * 1.0 * t)),
        shaded_byte(0.25 * ((1.0 - t) + 0.7 * 1.0 * t)),
    ];
    assert_eq!(expected_center, [255, 128, 64]);
    assert_eq!(image.get_pixel(32, 32), expected_center);
    Ok(())
}

#[test]
fn zero_sized_image_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let instances = [MeshInstance {
        mesh: &mesh,
        model: Mat4::identity(),
        color: [1.0, 1.0, 1.0],
    }];
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 1.0,
    };

    let zero_width =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.0, 0.0, 0.0], (0, 8))?;
    assert_eq!(zero_width.width, 0);
    assert_eq!(zero_width.height, 8);
    assert!(zero_width.pixels.is_empty());

    let zero_height =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.0, 0.0, 0.0], (8, 0))?;
    assert_eq!(zero_height.width, 8);
    assert_eq!(zero_height.height, 0);
    assert!(zero_height.pixels.is_empty());

    let zero_both =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.0, 0.0, 0.0], (0, 0))?;
    assert_eq!(zero_both.width, 0);
    assert_eq!(zero_both.height, 0);
    assert!(zero_both.pixels.is_empty());
    Ok(())
}

#[test]
fn nearer_instance_occludes_farther_instance() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let far_model = scaled_translated_model(2.0, Vec3::new(0.0, 0.0, 0.0));
    let near_model = scaled_translated_model(2.0, Vec3::new(0.0, 0.0, -1.0));
    let far_instance = MeshInstance {
        mesh: &mesh,
        model: far_model,
        color: [0.0, 0.0, 1.0],
    };
    let near_instance = MeshInstance {
        mesh: &mesh,
        model: near_model,
        color: [1.0, 0.0, 0.0],
    };
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
    };

    let far_only = renderer.render_offscreen(
        std::slice::from_ref(&far_instance),
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (64, 64),
    )?;
    assert_eq!(far_only.get_pixel(32, 32), [0, 0, 255]);

    let instances = [far_instance, near_instance];
    let image =
        renderer.render_offscreen(&instances, &camera, view, &light, [0.0, 0.0, 0.0], (64, 64))?;
    assert_eq!(image.get_pixel(32, 32), [255, 0, 0]);
    Ok(())
}

#[test]
fn material_instances_upload_one_mesh_and_one_bind_group_per_material(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let (mesh, attributes) = uv_quad(0.5);
    let mut albedo = Image::new(2, 2);
    for pixel in &mut albedo.pixels {
        *pixel = [255, 0, 0];
    }
    let flat = Material {
        metallic: 0.25,
        smoothness: 0.5,
        ..Material::plain()
    };
    let textured = Material {
        albedo: Some(albedo),
        metallic: 0.25,
        smoothness: 0.5,
        ..Material::plain()
    };
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
    };
    let instances = [
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: scaled_translated_model(1.0, Vec3::new(-2.0, 0.0, 0.0)),
            color: [1.0, 1.0, 1.0],
            material: &flat,
        },
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: scaled_translated_model(1.0, Vec3::new(0.0, 0.0, 0.0)),
            color: [1.0, 1.0, 1.0],
            material: &flat,
        },
        GpuMaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: scaled_translated_model(1.0, Vec3::new(2.0, 0.0, 0.0)),
            color: [1.0, 1.0, 1.0],
            material: &textured,
        },
    ];

    renderer.invalidate_resource_cache();
    let meshes_before = renderer.mesh_upload_count();
    let materials_before = renderer.material_bind_group_count();
    let image = renderer.render_offscreen_materials(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (96, 96),
    )?;
    assert_eq!(renderer.mesh_upload_count(), meshes_before + 1);
    assert_eq!(renderer.material_bind_group_count(), materials_before + 2);

    let row = 48;
    let textured_pixel = (0..96)
        .map(|x| (x, image.get_pixel(x, row)))
        .find(|(_, pixel)| pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60);
    let flat_pixel = (0..96)
        .map(|x| (x, image.get_pixel(x, row)))
        .find(|(_, pixel)| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200);
    let (textured_x, textured_pixel) = match textured_pixel {
        Some(found) => found,
        None => return Err("the textured instance drew no red pixel".into()),
    };
    let (flat_x, flat_pixel) = match flat_pixel {
        Some(found) => found,
        None => return Err("the flat instances drew no white pixel".into()),
    };
    assert_ne!(textured_x, flat_x);
    assert_ne!(textured_pixel, flat_pixel);

    let meshes_before = renderer.mesh_upload_count();
    let materials_before = renderer.material_bind_group_count();
    renderer.render_offscreen_materials(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (96, 96),
    )?;
    assert_eq!(renderer.mesh_upload_count(), meshes_before);
    assert_eq!(renderer.material_bind_group_count(), materials_before);

    renderer.invalidate_resource_cache();
    let meshes_before = renderer.mesh_upload_count();
    let materials_before = renderer.material_bind_group_count();
    renderer.render_offscreen_materials(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (96, 96),
    )?;
    assert_eq!(renderer.mesh_upload_count(), meshes_before + 1);
    assert_eq!(renderer.material_bind_group_count(), materials_before + 2);
    Ok(())
}

#[test]
fn overlapping_sprites_render_sorted_and_textured_offscreen(
) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = shared_renderer()?;
    let camera = Camera::orthographic(2.0, 1.0, 0.1, 100.0);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
    };

    let mut back_tex = Image::new(2, 2);
    for pixel in &mut back_tex.pixels {
        *pixel = [0, 0, 255];
    }
    let mut front_tex = Image::new(2, 1);
    front_tex.set_pixel(0, 0, [255, 0, 0]);
    front_tex.set_pixel(1, 0, [0, 255, 0]);

    let mut sprites = vec![
        Sprite {
            texture: Arc::new(front_tex),
            model: scaled_translated_model(2.0, Vec3::new(0.0, 0.0, 0.0)),
            color: [1.0, 1.0, 1.0, 1.0],
            sorting_layer: 1,
            sorting_order: 5,
        },
        Sprite {
            texture: Arc::new(back_tex),
            model: scaled_translated_model(3.5, Vec3::new(0.0, 0.0, 0.0)),
            color: [1.0, 1.0, 1.0, 1.0],
            sorting_layer: 0,
            sorting_order: 10,
        },
    ];
    sort_sprites(&mut sprites);
    renderer.set_sprites(sprites);
    let image = renderer.render_offscreen(&[], &camera, view, &light, [0.0, 0.0, 0.0], (64, 64))?;
    renderer.set_sprites(Vec::new());

    let left_front = image.get_pixel(20, 32);
    let right_front = image.get_pixel(44, 32);
    let outer_back = image.get_pixel(10, 32);
    let corner_bg = image.get_pixel(0, 0);

    assert!(
        left_front[0] > 200 && left_front[1] < 40 && left_front[2] < 40,
        "expected red on left half of front sprite, got {left_front:?}"
    );
    assert!(
        right_front[1] > 200 && right_front[0] < 40 && right_front[2] < 40,
        "expected green on right half of front sprite, got {right_front:?}"
    );
    assert_eq!(outer_back, [0, 0, 255]);
    assert_eq!(corner_bg, [0, 0, 0]);
    Ok(())
}
