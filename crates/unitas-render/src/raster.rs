use crate::camera::{look_at, Camera};
use crate::image::Image;
use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;

pub fn render(
    meshes: &[Mesh],
    camera: &Camera,
    material_color: [f32; 3],
    light_dir: Vec3,
    width: u32,
    height: u32,
) -> Image {
    let default_view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    render_with_view(
        meshes,
        camera,
        default_view,
        material_color,
        light_dir,
        width,
        height,
    )
}

pub fn render_with_view(
    meshes: &[Mesh],
    camera: &Camera,
    view: Mat4,
    material_color: [f32; 3],
    light_dir: Vec3,
    width: u32,
    height: u32,
) -> Image {
    let mut image = Image::new(width, height);
    if width == 0 || height == 0 {
        return image;
    }

    let mut depth_buffer = vec![f32::INFINITY; (width as usize) * (height as usize)];
    let vp = camera.projection_matrix() * view;
    let light = light_dir.normalize();
    let width_f = width as f32;
    let height_f = height as f32;

    for mesh in meshes {
        for tri in mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            if i0 >= mesh.positions.len()
                || i1 >= mesh.positions.len()
                || i2 >= mesh.positions.len()
            {
                continue;
            }

            let p0 = mesh.positions[i0];
            let p1 = mesh.positions[i1];
            let p2 = mesh.positions[i2];

            let c0 = vp.transform_point(p0);
            let c1 = vp.transform_point(p1);
            let c2 = vp.transform_point(p2);

            if c0.w <= 1e-6 || c1.w <= 1e-6 || c2.w <= 1e-6 {
                continue;
            }

            let ndc0 = Vec3::new(c0.x / c0.w, c0.y / c0.w, c0.z / c0.w);
            let ndc1 = Vec3::new(c1.x / c1.w, c1.y / c1.w, c1.z / c1.w);
            let ndc2 = Vec3::new(c2.x / c2.w, c2.y / c2.w, c2.z / c2.w);

            let s0_x = (ndc0.x + 1.0) * 0.5 * width_f;
            let s0_y = (1.0 - ndc0.y) * 0.5 * height_f;
            let s1_x = (ndc1.x + 1.0) * 0.5 * width_f;
            let s1_y = (1.0 - ndc1.y) * 0.5 * height_f;
            let s2_x = (ndc2.x + 1.0) * 0.5 * width_f;
            let s2_y = (1.0 - ndc2.y) * 0.5 * height_f;

            let area = edge_function(s0_x, s0_y, s1_x, s1_y, s2_x, s2_y);
            if area.abs() < 1e-6 {
                continue;
            }

            let n0 = mesh.normals.get(i0).copied().unwrap_or(Vec3::ZERO);
            let n1 = mesh.normals.get(i1).copied().unwrap_or(Vec3::ZERO);
            let n2 = mesh.normals.get(i2).copied().unwrap_or(Vec3::ZERO);
            let face_normal = {
                let sum = n0 + n1 + n2;
                if sum.length_squared() > 1e-8 {
                    sum.normalize()
                } else {
                    (p1 - p0).cross(p2 - p0).normalize()
                }
            };

            let intensity = face_normal.dot(light).clamp(0.0, 1.0);
            let shaded_rgb = [
                to_u8(material_color[0] * intensity),
                to_u8(material_color[1] * intensity),
                to_u8(material_color[2] * intensity),
            ];

            let min_x = s0_x.min(s1_x).min(s2_x).floor().max(0.0) as u32;
            let max_x = s0_x
                .max(s1_x)
                .max(s2_x)
                .ceil()
                .min((width - 1) as f32)
                .max(0.0) as u32;
            let min_y = s0_y.min(s1_y).min(s2_y).floor().max(0.0) as u32;
            let max_y = s0_y
                .max(s1_y)
                .max(s2_y)
                .ceil()
                .min((height - 1) as f32)
                .max(0.0) as u32;

            let inv_area = 1.0 / area;

            for py in min_y..=max_y {
                let sample_y = (py as f32) + 0.5;
                for px in min_x..=max_x {
                    let sample_x = (px as f32) + 0.5;

                    let w0 = edge_function(s1_x, s1_y, s2_x, s2_y, sample_x, sample_y) * inv_area;
                    let w1 = edge_function(s2_x, s2_y, s0_x, s0_y, sample_x, sample_y) * inv_area;
                    let w2 = edge_function(s0_x, s0_y, s1_x, s1_y, sample_x, sample_y) * inv_area;

                    if w0 >= -1e-5 && w1 >= -1e-5 && w2 >= -1e-5 {
                        let depth = w0 * ndc0.z + w1 * ndc1.z + w2 * ndc2.z;
                        if (0.0..=1.0).contains(&depth) {
                            let idx = (py as usize) * (width as usize) + (px as usize);
                            if depth < depth_buffer[idx] {
                                depth_buffer[idx] = depth;
                                image.pixels[idx] = shaded_rgb;
                            }
                        }
                    }
                }
            }
        }
    }

    image
}

fn edge_function(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

fn to_u8(val: f32) -> u8 {
    (val.clamp(0.0, 1.0) * 255.0).round() as u8
}
