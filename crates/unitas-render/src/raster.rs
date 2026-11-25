use crate::camera::{look_at, Camera};
use crate::image::Image;
use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::Mesh;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshInstance<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
}

struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
}

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

            rasterize_triangle(
                &mut depth_buffer,
                &mut image.pixels,
                width,
                height,
                [
                    ScreenVertex {
                        x: s0_x,
                        y: s0_y,
                        z: ndc0.z,
                    },
                    ScreenVertex {
                        x: s1_x,
                        y: s1_y,
                        z: ndc1.z,
                    },
                    ScreenVertex {
                        x: s2_x,
                        y: s2_y,
                        z: ndc2.z,
                    },
                ],
                shaded_rgb,
            );
        }
    }

    image
}

pub fn render_scene(
    instances: &[MeshInstance<'_>],
    camera: &Camera,
    view: Mat4,
    light: &DirectionalLight,
    background: [f32; 3],
    width: u32,
    height: u32,
) -> Image {
    let mut image = Image::new(width, height);
    if width == 0 || height == 0 {
        return image;
    }

    let background_rgb = [
        to_u8(background[0]),
        to_u8(background[1]),
        to_u8(background[2]),
    ];
    for pixel in &mut image.pixels {
        *pixel = background_rgb;
    }

    let mut depth_buffer = vec![f32::INFINITY; (width as usize) * (height as usize)];
    let vp = camera.projection_matrix() * view;
    let light_dir = light.direction.normalize();
    let t = light.intensity.clamp(0.0, 1.0);
    let ambient = 1.0 - t;
    let width_f = width as f32;
    let height_f = height as f32;

    for instance in instances {
        let mesh = instance.mesh;
        let mvp = vp * instance.model;
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

            let c0 = mvp.transform_point(p0);
            let c1 = mvp.transform_point(p1);
            let c2 = mvp.transform_point(p2);

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

            let n0 = mesh.normals.get(i0).copied().unwrap_or(Vec3::ZERO);
            let n1 = mesh.normals.get(i1).copied().unwrap_or(Vec3::ZERO);
            let n2 = mesh.normals.get(i2).copied().unwrap_or(Vec3::ZERO);
            let sum = transform_direction(instance.model, n0)
                + transform_direction(instance.model, n1)
                + transform_direction(instance.model, n2);
            let face_normal = if sum.length_squared() > 1e-8 {
                sum.normalize()
            } else {
                let m0 = instance.model.transform_point(p0).truncate();
                let m1 = instance.model.transform_point(p1).truncate();
                let m2 = instance.model.transform_point(p2).truncate();
                (m1 - m0).cross(m2 - m0).normalize()
            };

            let lambert = face_normal.dot(light_dir).max(0.0);
            let shaded_rgb = [
                to_u8(instance.color[0] * (ambient + light.color[0] * lambert * t)),
                to_u8(instance.color[1] * (ambient + light.color[1] * lambert * t)),
                to_u8(instance.color[2] * (ambient + light.color[2] * lambert * t)),
            ];

            rasterize_triangle(
                &mut depth_buffer,
                &mut image.pixels,
                width,
                height,
                [
                    ScreenVertex {
                        x: s0_x,
                        y: s0_y,
                        z: ndc0.z,
                    },
                    ScreenVertex {
                        x: s1_x,
                        y: s1_y,
                        z: ndc1.z,
                    },
                    ScreenVertex {
                        x: s2_x,
                        y: s2_y,
                        z: ndc2.z,
                    },
                ],
                shaded_rgb,
            );
        }
    }

    image
}

fn transform_direction(model: Mat4, direction: Vec3) -> Vec3 {
    model.mul_vec4(Vec4::from_direction(direction)).truncate()
}

fn rasterize_triangle(
    depth_buffer: &mut [f32],
    pixels: &mut [[u8; 3]],
    width: u32,
    height: u32,
    screen: [ScreenVertex; 3],
    rgb: [u8; 3],
) {
    let area = edge_function(
        screen[0].x,
        screen[0].y,
        screen[1].x,
        screen[1].y,
        screen[2].x,
        screen[2].y,
    );
    if area.abs() < 1e-6 {
        return;
    }

    let min_x = screen[0]
        .x
        .min(screen[1].x)
        .min(screen[2].x)
        .floor()
        .max(0.0) as u32;
    let max_x = screen[0]
        .x
        .max(screen[1].x)
        .max(screen[2].x)
        .ceil()
        .min((width - 1) as f32)
        .max(0.0) as u32;
    let min_y = screen[0]
        .y
        .min(screen[1].y)
        .min(screen[2].y)
        .floor()
        .max(0.0) as u32;
    let max_y = screen[0]
        .y
        .max(screen[1].y)
        .max(screen[2].y)
        .ceil()
        .min((height - 1) as f32)
        .max(0.0) as u32;

    let inv_area = 1.0 / area;

    for py in min_y..=max_y {
        let sample_y = (py as f32) + 0.5;
        for px in min_x..=max_x {
            let sample_x = (px as f32) + 0.5;

            let w0 = edge_function(
                screen[1].x,
                screen[1].y,
                screen[2].x,
                screen[2].y,
                sample_x,
                sample_y,
            ) * inv_area;
            let w1 = edge_function(
                screen[2].x,
                screen[2].y,
                screen[0].x,
                screen[0].y,
                sample_x,
                sample_y,
            ) * inv_area;
            let w2 = edge_function(
                screen[0].x,
                screen[0].y,
                screen[1].x,
                screen[1].y,
                sample_x,
                sample_y,
            ) * inv_area;

            if w0 >= -1e-5 && w1 >= -1e-5 && w2 >= -1e-5 {
                let depth = w0 * screen[0].z + w1 * screen[1].z + w2 * screen[2].z;
                if (0.0..=1.0).contains(&depth) {
                    let idx = (py as usize) * (width as usize) + (px as usize);
                    if depth < depth_buffer[idx] {
                        depth_buffer[idx] = depth;
                        pixels[idx] = rgb;
                    }
                }
            }
        }
    }
}

fn edge_function(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

fn to_u8(val: f32) -> u8 {
    (val.clamp(0.0, 1.0) * 255.0).round() as u8
}
