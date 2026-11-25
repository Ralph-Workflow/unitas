use crate::camera::{look_at, Camera};
use crate::image::Image;
use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::{Mesh, MeshAttributes};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshInstance<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
}

/// A mesh plus per-material shading inputs. The plain [`MeshInstance`] keeps
/// the historical Lambert-only path; this carries textures and blend state so
/// the CPU raster shades exactly what the GPU renders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialInstance<'a> {
    pub mesh: &'a Mesh,
    pub attributes: Option<&'a MeshAttributes>,
    pub model: Mat4,
    pub color: [f32; 3],
    pub material: &'a Material,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Opaque,
    /// Alpha-blended translucent surface, drawn back-to-front after opaques.
    Blend,
}

/// Lit-material parameters shared by the CPU and GPU paths. Textures are
/// plain RGB; a missing texture means "white" (albedo/normal) or "black"
/// (emissive), matching the GPU's 1x1 placeholder textures.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub albedo: Option<Image>,
    pub normal: Option<Image>,
    pub emissive: Option<Image>,
    pub metallic: f32,
    pub smoothness: f32,
    /// Per-pixel opacity for [`BlendMode::Blend`] surfaces.
    pub alpha: f32,
    pub blend: BlendMode,
}

/// The shading used for plain [`MeshInstance`]s: pure ambient + Lambert.
pub static DEFAULT_MATERIAL: Material = Material::plain();

impl Material {
    pub const fn plain() -> Self {
        Self {
            albedo: None,
            normal: None,
            emissive: None,
            metallic: 0.0,
            smoothness: 0.0,
            alpha: 1.0,
            blend: BlendMode::Opaque,
        }
    }

    pub fn is_textured(&self) -> bool {
        self.albedo.is_some() || self.normal.is_some() || self.emissive.is_some()
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::plain()
    }
}

struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
}

struct ShadeContext<'a> {
    light_dir: Vec3,
    light_color: [f32; 3],
    intensity: f32,
    ambient: f32,
    camera_pos: Vec3,
    material: &'a Material,
    color: [f32; 3],
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
                true,
                None,
                &move |_, _, _| shaded_rgb,
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
    let material_instances: Vec<MaterialInstance<'_>> = instances
        .iter()
        .map(|instance| MaterialInstance {
            mesh: instance.mesh,
            attributes: None,
            model: instance.model,
            color: instance.color,
            material: &DEFAULT_MATERIAL,
        })
        .collect();
    render_scene_materials(
        &material_instances,
        camera,
        view,
        light,
        background,
        width,
        height,
    )
}

pub fn render_scene_materials(
    instances: &[MaterialInstance<'_>],
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
    let camera_pos = view
        .inverse()
        .map(|inverse| inverse.transform_point(Vec3::ZERO).truncate())
        .unwrap_or(Vec3::ZERO);

    for instance in instances
        .iter()
        .filter(|instance| instance.material.blend == BlendMode::Opaque)
    {
        draw_instance(
            instance,
            &vp,
            light_dir,
            light.color,
            t,
            1.0 - t,
            camera_pos,
            &mut depth_buffer,
            &mut image.pixels,
            width,
            height,
        );
    }

    // Translucent pass: draw back-to-front (larger view-space depth first).
    let mut translucent: Vec<&MaterialInstance<'_>> = instances
        .iter()
        .filter(|instance| instance.material.blend == BlendMode::Blend)
        .collect();
    translucent.sort_by(|a, b| {
        let depth_a = view.mul_mat4(&a.model).transform_point(Vec3::ZERO).z;
        let depth_b = view.mul_mat4(&b.model).transform_point(Vec3::ZERO).z;
        depth_b.partial_cmp(&depth_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    for instance in translucent {
        draw_instance(
            instance,
            &vp,
            light_dir,
            light.color,
            t,
            1.0 - t,
            camera_pos,
            &mut depth_buffer,
            &mut image.pixels,
            width,
            height,
        );
    }

    image
}

#[allow(clippy::too_many_arguments)]
fn draw_instance(
    instance: &MaterialInstance<'_>,
    vp: &Mat4,
    light_dir: Vec3,
    light_color: [f32; 3],
    t: f32,
    ambient: f32,
    camera_pos: Vec3,
    depth_buffer: &mut [f32],
    pixels: &mut [[u8; 3]],
    width: u32,
    height: u32,
) {
    let mesh = instance.mesh;
    let mvp = *vp * instance.model;
    let material = instance.material;
    let textured = material.is_textured();
    let attributes = instance.attributes;
    let write_depth = material.blend == BlendMode::Opaque;
    let alpha = if material.blend == BlendMode::Blend {
        Some(material.alpha.clamp(0.0, 1.0))
    } else {
        None
    };
    let context = ShadeContext {
        light_dir,
        light_color,
        intensity: t,
        ambient,
        camera_pos,
        material,
        color: instance.color,
    };

    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= mesh.positions.len() || i1 >= mesh.positions.len() || i2 >= mesh.positions.len() {
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

        let screen = [
            ScreenVertex {
                x: (ndc0.x + 1.0) * 0.5 * width as f32,
                y: (1.0 - ndc0.y) * 0.5 * height as f32,
                z: ndc0.z,
            },
            ScreenVertex {
                x: (ndc1.x + 1.0) * 0.5 * width as f32,
                y: (1.0 - ndc1.y) * 0.5 * height as f32,
                z: ndc1.z,
            },
            ScreenVertex {
                x: (ndc2.x + 1.0) * 0.5 * width as f32,
                y: (1.0 - ndc2.y) * 0.5 * height as f32,
                z: ndc2.z,
            },
        ];

        let n0 = transform_direction(instance.model, mesh.normals.get(i0).copied().unwrap_or(Vec3::ZERO));
        let n1 = transform_direction(instance.model, mesh.normals.get(i1).copied().unwrap_or(Vec3::ZERO));
        let n2 = transform_direction(instance.model, mesh.normals.get(i2).copied().unwrap_or(Vec3::ZERO));
        let sum = n0 + n1 + n2;
        let face_normal = if sum.length_squared() > 1e-8 {
            sum.normalize()
        } else {
            let m0 = instance.model.transform_point(p0).truncate();
            let m1 = instance.model.transform_point(p1).truncate();
            let m2 = instance.model.transform_point(p2).truncate();
            (m1 - m0).cross(m2 - m0).normalize()
        };

        if !textured {
            let lambert = face_normal.dot(light_dir).max(0.0);
            let shaded_rgb = [
                to_u8(instance.color[0] * (ambient + light_color[0] * lambert * t)),
                to_u8(instance.color[1] * (ambient + light_color[1] * lambert * t)),
                to_u8(instance.color[2] * (ambient + light_color[2] * lambert * t)),
            ];
            rasterize_triangle(
                depth_buffer,
                pixels,
                width,
                height,
                screen,
                write_depth,
                alpha,
                &move |_, _, _| shaded_rgb,
            );
            continue;
        }

        let uv0 = attributes
            .and_then(|attrs| attrs.uvs.get(i0).copied())
            .unwrap_or([0.0, 0.0]);
        let uv1 = attributes
            .and_then(|attrs| attrs.uvs.get(i1).copied())
            .unwrap_or([0.0, 0.0]);
        let uv2 = attributes
            .and_then(|attrs| attrs.uvs.get(i2).copied())
            .unwrap_or([0.0, 0.0]);
        let tangent0 = attributes
            .and_then(|attrs| attrs.tangents.get(i0).copied())
            .map(|tangent| transform_direction(instance.model, tangent))
            .unwrap_or(Vec3::ZERO);
        let tangent1 = attributes
            .and_then(|attrs| attrs.tangents.get(i1).copied())
            .map(|tangent| transform_direction(instance.model, tangent))
            .unwrap_or(Vec3::ZERO);
        let tangent2 = attributes
            .and_then(|attrs| attrs.tangents.get(i2).copied())
            .map(|tangent| transform_direction(instance.model, tangent))
            .unwrap_or(Vec3::ZERO);
        let centroid = (instance.model.transform_point(p0).truncate()
            + instance.model.transform_point(p1).truncate()
            + instance.model.transform_point(p2).truncate())
            * (1.0 / 3.0);
        let view_dir = (camera_pos - centroid).normalize();

        rasterize_triangle(
            depth_buffer,
            pixels,
            width,
            height,
            screen,
            write_depth,
            alpha,
            &|w0, w1, w2| {
                shade_textured_pixel(
                    &context,
                    face_normal,
                    w0,
                    w1,
                    w2,
                    n0,
                    n1,
                    n2,
                    tangent0,
                    tangent1,
                    tangent2,
                    uv0,
                    uv1,
                    uv2,
                    view_dir,
                )
            },
        );
    }
}

/// Per-pixel lit shading for textured materials; mirrors the GPU fragment
/// shader exactly (albedo multiply, tangent-space normal map, Blinn-Phong
/// specular gated by metallic*smoothness, emissive add).
#[allow(clippy::too_many_arguments)]
fn shade_textured_pixel(
    context: &ShadeContext<'_>,
    face_normal: Vec3,
    w0: f32,
    w1: f32,
    w2: f32,
    n0: Vec3,
    n1: Vec3,
    n2: Vec3,
    tangent0: Vec3,
    tangent1: Vec3,
    tangent2: Vec3,
    uv0: [f32; 2],
    uv1: [f32; 2],
    uv2: [f32; 2],
    view_dir: Vec3,
) -> [u8; 3] {
    let u = w0 * uv0[0] + w1 * uv1[0] + w2 * uv2[0];
    let v = w0 * uv0[1] + w1 * uv1[1] + w2 * uv2[1];
    let white = [1.0f32; 3];
    let black = [0.0f32; 3];
    let albedo_tex = context
        .material
        .albedo
        .as_ref()
        .map(|tex| tex.sample_bilinear(u, v))
        .unwrap_or(white);
    let emissive_tex = context
        .material
        .emissive
        .as_ref()
        .map(|tex| tex.sample_bilinear(u, v))
        .unwrap_or(black);

    let mut normal = (w0 * n0 + w1 * n1 + w2 * n2).normalize();
    if normal.length_squared() < 1e-12 {
        normal = face_normal;
    }
    if context.material.normal.is_some() {
        let tangent = (w0 * tangent0 + w1 * tangent1 + w2 * tangent2).normalize();
        if tangent.length_squared() > 1e-12 {
            let tangent = (tangent - normal * tangent.dot(normal)).normalize();
            let bitangent = normal.cross(tangent);
            let mapped = context.material.normal.as_ref().unwrap().sample_bilinear(u, v);
            let mapped = Vec3::new(
                mapped[0] * 2.0 - 1.0,
                mapped[1] * 2.0 - 1.0,
                mapped[2] * 2.0 - 1.0,
            );
            normal = (tangent * mapped.x + bitangent * mapped.y + normal * mapped.z).normalize();
        }
    }

    let albedo = [
        albedo_tex[0] * context.color[0],
        albedo_tex[1] * context.color[1],
        albedo_tex[2] * context.color[2],
    ];
    let metallic = context.material.metallic;
    let smoothness = context.material.smoothness;
    let lambert = normal.dot(context.light_dir).max(0.0);
    let half_dir = (context.light_dir + view_dir).normalize();
    let shininess = 8.0 + smoothness * 248.0;
    let specular = half_dir.dot(normal).max(0.0).powf(shininess) * metallic * smoothness;
    let mut rgb = [0.0f32; 3];
    for channel in 0..3 {
        let diffuse = albedo[channel] * (1.0 - metallic);
        rgb[channel] = context.ambient * albedo[channel]
            + context.light_color[channel] * (lambert * diffuse + specular) * context.intensity
            + emissive_tex[channel];
    }
    [to_u8(rgb[0]), to_u8(rgb[1]), to_u8(rgb[2])]
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
    write_depth: bool,
    alpha: Option<f32>,
    shade: &dyn Fn(f32, f32, f32) -> [u8; 3],
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
                        let src = shade(w0, w1, w2);
                        pixels[idx] = match alpha {
                            Some(a) => blend_pixel(pixels[idx], src, a),
                            None => src,
                        };
                        if write_depth {
                            depth_buffer[idx] = depth;
                        }
                    }
                }
            }
        }
    }
}

fn blend_pixel(dst: [u8; 3], src: [u8; 3], alpha: f32) -> [u8; 3] {
    let mut out = [0u8; 3];
    for channel in 0..3 {
        let mixed = f32::from(src[channel]) * alpha + f32::from(dst[channel]) * (1.0 - alpha);
        out[channel] = mixed.round() as u8;
    }
    out
}

fn edge_function(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

fn to_u8(val: f32) -> u8 {
    (val.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::MeshAttributes;

    fn camera() -> Camera {
        Camera::perspective(60.0, 1.0, 0.1, 100.0)
    }

    fn view() -> Mat4 {
        look_at(
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    fn unlit_light() -> DirectionalLight {
        DirectionalLight {
            direction: Vec3::new(0.4, 0.7, 0.6),
            color: [1.0, 1.0, 1.0],
            intensity: 0.0,
        }
    }

    /// Front-facing quad in the z=0 plane, spanning `-extent..extent`, with
    /// UVs mapped so v=1 is at the top of the screen.
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
        let uvs = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        let attributes = mesh.attributes_with_uvs(uvs);
        (mesh, attributes)
    }

    fn translated(x: f32, y: f32, z: f32) -> Mat4 {
        let mut model = Mat4::identity();
        model.set(3, 0, x);
        model.set(3, 1, y);
        model.set(3, 2, z);
        model
    }

    #[test]
    fn mat4_inverse_round_trips_affine_transforms() {
        let mut model = Mat4::identity();
        model.set(0, 0, 2.0);
        model.set(1, 1, 3.0);
        model.set(2, 2, 4.0);
        model.set(3, 0, 1.0);
        model.set(3, 1, -2.0);
        model.set(3, 2, 3.5);
        let view = look_at(
            Vec3::new(2.0, 1.0, -4.0),
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        for matrix in [model, view] {
            let inverse = matrix.inverse().expect("matrix is invertible");
            let product = matrix.mul_mat4(&inverse);
            for index in 0..16 {
                let expected = if index % 5 == 0 { 1.0 } else { 0.0 };
                assert!(
                    (product.data[index] - expected).abs() < 1e-5,
                    "index {index}: {}",
                    product.data[index]
                );
            }
        }
    }

    #[test]
    fn flat_material_instances_match_the_plain_render() {
        let mesh = crate::mesh::cube(1.0);
        let plain = [MeshInstance {
            mesh: &mesh,
            model: translated(0.0, 0.0, 0.0),
            color: [0.8, 0.4, 0.2],
        }];
        let material = Material::plain();
        let with_material = [MaterialInstance {
            mesh: &mesh,
            attributes: None,
            model: plain[0].model,
            color: plain[0].color,
            material: &material,
        }];
        let light = DirectionalLight {
            direction: Vec3::new(0.4, 0.7, 0.6),
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        };
        let a = render_scene(&plain, &camera(), view(), &light, [0.1, 0.2, 0.3], 64, 64);
        let b = render_scene_materials(
            &with_material,
            &camera(),
            view(),
            &light,
            [0.1, 0.2, 0.3],
            64,
            64,
        );
        assert_eq!(a.pixels, b.pixels);
    }

    #[test]
    fn textured_quad_samples_expected_texels() {
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
        let instances = [MaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: Mat4::identity(),
            color: [1.0, 1.0, 1.0],
            material: &material,
        }];
        let image = render_scene_materials(
            &instances,
            &camera(),
            view(),
            &unlit_light(),
            [0.0, 0.0, 0.0],
            32,
            32,
        );
        // With ambient 1.0 and no light, pixels equal the bilinear texel mix.
        let top_left = image.get_pixel(6, 6);
        assert!(top_left[0] > 200 && top_left[1] < 40 && top_left[2] < 40);
        let top_right = image.get_pixel(25, 6);
        assert!(top_right[1] > 200 && top_right[0] < 40 && top_right[2] < 40);
        let bottom_left = image.get_pixel(6, 25);
        assert!(bottom_left[2] > 200 && bottom_left[0] < 40 && bottom_left[1] < 40);
        let bottom_right = image.get_pixel(25, 25);
        assert!(bottom_right[0] > 200 && bottom_right[1] > 200 && bottom_right[2] > 200);
    }

    #[test]
    fn emissive_is_visible_with_a_black_albedo_and_no_light() {
        let (mesh, attributes) = uv_quad(1.0);
        let mut emissive = Image::new(1, 1);
        emissive.pixels[0] = [255, 32, 0];
        let material = Material {
            albedo: None,
            emissive: Some(emissive),
            ..Material::plain()
        };
        let instances = [MaterialInstance {
            mesh: &mesh,
            attributes: Some(&attributes),
            model: Mat4::identity(),
            color: [0.0, 0.0, 0.0],
            material: &material,
        }];
        let image = render_scene_materials(
            &instances,
            &camera(),
            view(),
            &unlit_light(),
            [0.0, 0.0, 0.0],
            32,
            32,
        );
        let pixel = image.get_pixel(16, 16);
        assert!(
            (pixel[0] as i32 - 255).abs() <= 1 && (pixel[1] as i32 - 32).abs() <= 1,
            "emissive pixel {pixel:?} does not match the emissive texture"
        );
    }

    #[test]
    fn translucent_quads_blend_back_to_front_regardless_of_input_order() {
        let (mesh, attributes) = uv_quad(2.0);
        let red = Material {
            alpha: 0.5,
            blend: BlendMode::Blend,
            ..Material::plain()
        };
        let blue = Material {
            alpha: 0.5,
            blend: BlendMode::Blend,
            ..Material::plain()
        };
        // Near (z=-0.5) red listed first; the sorter must still draw the far
        // blue quad first.
        let instances = [
            MaterialInstance {
                mesh: &mesh,
                attributes: Some(&attributes),
                model: translated(0.0, 0.0, -0.5),
                color: [1.0, 0.0, 0.0],
                material: &red,
            },
            MaterialInstance {
                mesh: &mesh,
                attributes: Some(&attributes),
                model: Mat4::identity(),
                color: [0.0, 0.0, 1.0],
                material: &blue,
            },
        ];
        let image = render_scene_materials(
            &instances,
            &camera(),
            view(),
            &unlit_light(),
            [0.0, 0.0, 0.0],
            32,
            32,
        );
        let pixel = image.get_pixel(16, 16);
        // out = red*0.5 + (blue*0.5)*0.5 = (127, 0, 64) on a black background.
        assert!(
            (pixel[0] as i32 - 127).abs() <= 2 && (pixel[2] as i32 - 64).abs() <= 2,
            "blended pixel {pixel:?} does not match back-to-front compositing"
        );
        assert!(pixel[0] > pixel[2], "nearer red quad must dominate the blend");
    }
}
