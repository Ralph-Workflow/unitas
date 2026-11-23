use std::f32::consts::PI;

use crate::math::Vec3;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

pub fn cube(size: f32) -> Mesh {
    let h = size * 0.5;
    let face_definitions: [(Vec3, [Vec3; 4]); 6] = [
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
    ];

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
