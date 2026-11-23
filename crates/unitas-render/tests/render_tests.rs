use std::ops::{Add, Sub};
use std::path::PathBuf;
use unitas_render::camera::{look_at, Camera};
use unitas_render::image::Image;
use unitas_render::math::{Mat4, Vec3, Vec4};
use unitas_render::mesh::{cube, uv_sphere, Mesh};
use unitas_render::raster::{render, render_with_view};

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
fn raster_render_centered_cube_fills_center_pixel_and_is_deterministic() {
    let mesh = cube(1.0);
    let cam = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let color = [1.0, 0.5, 0.25];
    let light_dir = Vec3::new(0.0, 0.0, -1.0);

    let img1 = render(std::slice::from_ref(&mesh), &cam, color, light_dir, 64, 64);
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let img2 = render_with_view(&[mesh], &cam, view, color, light_dir, 64, 64);

    let non_bg_count = img1.pixels.iter().filter(|&&px| px != [0, 0, 0]).count();
    assert!(non_bg_count > 0);
    assert_eq!(img1.get_pixel(32, 32), [255, 128, 64]);
    assert_eq!(img1.to_bmp_bytes(), img2.to_bmp_bytes());
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

    let tmp_path: PathBuf = std::env::temp_dir().join(format!(
        "unitas_render_test_{}.bmp",
        std::process::id()
    ));
    assert!(img.write_bmp(&tmp_path).is_ok());
    let disk_bytes = std::fs::read(&tmp_path).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp_path);
    assert_eq!(disk_bytes, bytes);
}
