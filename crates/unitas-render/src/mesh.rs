use std::f32::consts::PI;

use crate::math::Vec3;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn from_buffers(positions: &[[f32; 3]], normals: &[[f32; 3]], indices: &[u32]) -> Self {
        Self {
            positions: positions
                .iter()
                .map(|&[x, y, z]| Vec3::new(x, y, z))
                .collect(),
            normals: normals
                .iter()
                .map(|&[x, y, z]| Vec3::new(x, y, z))
                .collect(),
            indices: indices.to_vec(),
        }
    }

    pub fn attributes_with_uvs(&self, uvs: Vec<[f32; 2]>) -> MeshAttributes {
        MeshAttributes::compute(self, uvs)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshAttributes {
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<Vec3>,
}

impl MeshAttributes {
    pub fn compute(mesh: &Mesh, uvs: Vec<[f32; 2]>) -> Self {
        let mut tangents = vec![Vec3::ZERO; mesh.positions.len()];
        for tri in mesh.indices.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let (Some(p0), Some(p1), Some(p2)) = (
                mesh.positions.get(i0),
                mesh.positions.get(i1),
                mesh.positions.get(i2),
            ) else {
                continue;
            };
            let uv0 = uvs.get(i0).copied().unwrap_or([0.0, 0.0]);
            let uv1 = uvs.get(i1).copied().unwrap_or([0.0, 0.0]);
            let uv2 = uvs.get(i2).copied().unwrap_or([0.0, 0.0]);
            let edge1 = *p1 - *p0;
            let edge2 = *p2 - *p0;
            let du1 = uv1[0] - uv0[0];
            let dv1 = uv1[1] - uv0[1];
            let du2 = uv2[0] - uv0[0];
            let dv2 = uv2[1] - uv0[1];
            let det = du1 * dv2 - du2 * dv1;
            if det.abs() < 1e-12 {
                continue;
            }
            let tangent = (edge1 * dv2 - edge2 * dv1) * (1.0 / det);
            for index in [i0, i1, i2] {
                if let Some(slot) = tangents.get_mut(index) {
                    *slot = *slot + tangent;
                }
            }
        }
        for tangent in &mut tangents {
            if tangent.length_squared() > 1e-12 {
                *tangent = tangent.normalize();
            } else {
                *tangent = Vec3::ZERO;
            }
        }
        MeshAttributes { uvs, tangents }
    }
}

fn cube_z_faces(h: f32) -> [(Vec3, [Vec3; 4]); 2] {
    [
        (
            Vec3::new(0.0, 0.0, -1.0),
            [
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, h, -h),
                Vec3::new(h, h, -h),
                Vec3::new(h, -h, -h),
            ],
        ),
        (
            Vec3::new(0.0, 0.0, 1.0),
            [
                Vec3::new(h, -h, h),
                Vec3::new(h, h, h),
                Vec3::new(-h, h, h),
                Vec3::new(-h, -h, h),
            ],
        ),
    ]
}

fn cube_x_faces(h: f32) -> [(Vec3, [Vec3; 4]); 2] {
    [
        (
            Vec3::new(-1.0, 0.0, 0.0),
            [
                Vec3::new(-h, -h, h),
                Vec3::new(-h, h, h),
                Vec3::new(-h, h, -h),
                Vec3::new(-h, -h, -h),
            ],
        ),
        (
            Vec3::new(1.0, 0.0, 0.0),
            [
                Vec3::new(h, -h, -h),
                Vec3::new(h, h, -h),
                Vec3::new(h, h, h),
                Vec3::new(h, -h, h),
            ],
        ),
    ]
}

fn cube_y_faces(h: f32) -> [(Vec3, [Vec3; 4]); 2] {
    [
        (
            Vec3::new(0.0, -1.0, 0.0),
            [
                Vec3::new(-h, -h, h),
                Vec3::new(-h, -h, -h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, -h, h),
            ],
        ),
        (
            Vec3::new(0.0, 1.0, 0.0),
            [
                Vec3::new(-h, h, -h),
                Vec3::new(-h, h, h),
                Vec3::new(h, h, h),
                Vec3::new(h, h, -h),
            ],
        ),
    ]
}

fn cube_face_definitions(h: f32) -> [(Vec3, [Vec3; 4]); 6] {
    let z_faces = cube_z_faces(h);
    let x_faces = cube_x_faces(h);
    let y_faces = cube_y_faces(h);
    [
        z_faces[0], z_faces[1], x_faces[0], x_faces[1], y_faces[0], y_faces[1],
    ]
}

pub fn cube(size: f32) -> Mesh {
    let h = size * 0.5;
    let face_definitions = cube_face_definitions(h);

    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (face_idx, (normal, verts)) in face_definitions.iter().enumerate() {
        let base = (face_idx * 4) as u32;
        positions.extend_from_slice(verts);
        normals.extend_from_slice(&[*normal, *normal, *normal, *normal]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Mesh {
        positions,
        normals,
        indices,
    }
}

pub fn uv_sphere(radius: f32, sectors: u32, rings: u32) -> Mesh {
    let vertex_count = ((sectors + 1) * (rings + 1)) as usize;
    let index_count = (sectors * rings * 6) as usize;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(index_count);

    let rings_f = rings.max(1) as f32;
    let sectors_f = sectors.max(1) as f32;

    for r in 0..=rings {
        let phi = PI * (r as f32) / rings_f;
        let y = phi.cos();
        let sin_phi = phi.sin();

        for s in 0..=sectors {
            let theta = 2.0 * PI * (s as f32) / sectors_f;
            let x = sin_phi * theta.cos();
            let z = sin_phi * theta.sin();

            let normal = Vec3::new(x, y, z).normalize();
            positions.push(normal * radius);
            normals.push(normal);
        }
    }

    let stride = sectors + 1;
    for r in 0..rings {
        for s in 0..sectors {
            let i0 = r * stride + s;
            let i1 = i0 + 1;
            let i2 = (r + 1) * stride + s;
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
