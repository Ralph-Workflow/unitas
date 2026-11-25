//! Built-in primitive meshes and the identity table of Unity's built-in resources.

use std::f32::consts::PI;

/// Unity's `unity_builtin_extra` resource GUID, shared by every built-in primitive.
pub const BUILTIN_EXTRA_GUID: &str = "0000000000000000e000000000000000";

/// The six meshes Unity ships as built-in resources.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuiltinPrimitive {
    Cube,
    Sphere,
    Capsule,
    Cylinder,
    Plane,
    Quad,
}

/// Vertex data of one mesh, in Unity's component order.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshData {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub sub_meshes: Vec<SubMesh>,
}

/// One index range of a mesh, addressable by a sub-asset id that survives reimport.
#[derive(Clone, Debug, PartialEq)]
pub struct SubMesh {
    pub name: String,
    pub index_offset: u32,
    pub index_count: u32,
    pub sub_asset_id: String,
}

/// Built-in identity data: primitive, resource GUID and local file id.
const IDENTITY_TABLE: [(BuiltinPrimitive, &str, i64); 6] = [
    (BuiltinPrimitive::Cube, BUILTIN_EXTRA_GUID, 10202),
    (BuiltinPrimitive::Cylinder, BUILTIN_EXTRA_GUID, 10206),
    (BuiltinPrimitive::Sphere, BUILTIN_EXTRA_GUID, 10207),
    (BuiltinPrimitive::Capsule, BUILTIN_EXTRA_GUID, 10208),
    (BuiltinPrimitive::Plane, BUILTIN_EXTRA_GUID, 10209),
    (BuiltinPrimitive::Quad, BUILTIN_EXTRA_GUID, 10210),
];

const CUBE_HALF_EXTENT: f32 = 0.5;
const QUAD_HALF_EXTENT: f32 = 0.5;
/// Unity's built-in Plane spans ten units, unlike the unit-sized cube and quad.
const PLANE_HALF_EXTENT: f32 = 5.0;
const SPHERE_RADIUS: f32 = 1.0;
const SPHERE_SIDES: usize = 24;
const SPHERE_RINGS: usize = 12;
const CYLINDER_RADIUS: f32 = 0.5;
const CYLINDER_HALF_HEIGHT: f32 = 1.0;
const CYLINDER_SIDES: usize = 24;
const CAPSULE_RADIUS: f32 = 0.5;
const CAPSULE_HALF_BODY: f32 = 0.5;
const CAPSULE_SIDES: usize = 24;
const CAPSULE_CAP_RINGS: usize = 6;

/// The local file id Unity assigns to a built-in primitive.
pub fn builtin_file_id(p: BuiltinPrimitive) -> i64 {
    IDENTITY_TABLE
        .iter()
        .find(|(entry, _, _)| *entry == p)
        .map_or(0, |(_, _, file_id)| *file_id)
}

/// The resource GUID a built-in primitive is addressed by.
pub fn builtin_guid(p: BuiltinPrimitive) -> &'static str {
    IDENTITY_TABLE
        .iter()
        .find(|(entry, _, _)| *entry == p)
        .map_or(BUILTIN_EXTRA_GUID, |(_, guid, _)| *guid)
}

/// The primitive Unity names `name`, if it is one of the built-ins.
pub fn builtin_primitive_by_name(name: &str) -> Option<BuiltinPrimitive> {
    match name {
        "Cube" => Some(BuiltinPrimitive::Cube),
        "Sphere" => Some(BuiltinPrimitive::Sphere),
        "Capsule" => Some(BuiltinPrimitive::Capsule),
        "Cylinder" => Some(BuiltinPrimitive::Cylinder),
        "Plane" => Some(BuiltinPrimitive::Plane),
        "Quad" => Some(BuiltinPrimitive::Quad),
        _ => None,
    }
}

/// The primitive Unity addresses with local file id `file_id`, if any.
pub fn builtin_primitive_by_file_id(file_id: i64) -> Option<BuiltinPrimitive> {
    IDENTITY_TABLE
        .iter()
        .find(|(_, _, entry_id)| *entry_id == file_id)
        .map(|(entry, _, _)| *entry)
}

/// The primitive whose identity pair matches both `file_id` and `guid`.
///
/// A file id alone identifies every built-in in the table, so the GUID is
/// checked as well: a caller holding a resolved identity only accepts the
/// primitive when both halves agree.
pub fn builtin_primitive_by_identity(file_id: i64, guid: &str) -> Option<BuiltinPrimitive> {
    let primitive = builtin_primitive_by_file_id(file_id)?;
    if builtin_guid(primitive).eq_ignore_ascii_case(guid) {
        Some(primitive)
    } else {
        None
    }
}

/// The mesh of a built-in primitive, identical on every call.
pub fn primitive_mesh(p: BuiltinPrimitive) -> MeshData {
    let mut mesh = match p {
        BuiltinPrimitive::Cube => cube(),
        BuiltinPrimitive::Sphere => sphere(),
        BuiltinPrimitive::Capsule => capsule(),
        BuiltinPrimitive::Cylinder => cylinder(),
        BuiltinPrimitive::Plane => plane(),
        BuiltinPrimitive::Quad => quad(),
    };
    mesh.sub_meshes = vec![SubMesh {
        name: mesh.name.clone(),
        index_offset: 0,
        index_count: mesh.indices.len() as u32,
        sub_asset_id: mesh.name.clone(),
    }];
    mesh
}

#[derive(Default)]
struct Builder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Builder {
    fn vertex(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);
        index
    }

    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    fn finish(self, name: &str) -> MeshData {
        MeshData {
            name: name.to_string(),
            positions: self.positions,
            normals: self.normals,
            uvs: self.uvs,
            indices: self.indices,
            sub_meshes: Vec::new(),
        }
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// A closed ring of `sides + 1` vertices, the seam duplicated so `u` reaches one.
fn build_ring(
    builder: &mut Builder,
    sides: usize,
    vertex: impl Fn(f32) -> ([f32; 3], [f32; 3], [f32; 2]),
) -> Vec<u32> {
    (0..=sides)
        .map(|step| {
            let u = step as f32 / sides as f32;
            let (position, normal, uv) = vertex(u);
            builder.vertex(position, normal, uv)
        })
        .collect()
}

/// Connects a lower ring to the ring above it, wound so faces point outward.
fn stitch(builder: &mut Builder, lower: &[u32], upper: &[u32]) {
    for step in 0..lower.len() - 1 {
        builder.triangle(lower[step], upper[step], upper[step + 1]);
        builder.triangle(lower[step], upper[step + 1], lower[step + 1]);
    }
}

/// One cube face: outward normal, corner and the two edges leaving that corner.
struct Face {
    normal: [f32; 3],
    corner: [f32; 3],
    first: [f32; 3],
    second: [f32; 3],
}

fn cube() -> MeshData {
    let h = CUBE_HALF_EXTENT;
    // cross(first, second) is the outward normal of the face.
    let faces = [
        Face {
            normal: [1.0, 0.0, 0.0],
            corner: [h, -h, -h],
            first: [0.0, 1.0, 0.0],
            second: [0.0, 0.0, 1.0],
        },
        Face {
            normal: [-1.0, 0.0, 0.0],
            corner: [-h, -h, h],
            first: [0.0, 1.0, 0.0],
            second: [0.0, 0.0, -1.0],
        },
        Face {
            normal: [0.0, 1.0, 0.0],
            corner: [-h, h, -h],
            first: [0.0, 0.0, 1.0],
            second: [1.0, 0.0, 0.0],
        },
        Face {
            normal: [0.0, -1.0, 0.0],
            corner: [-h, -h, h],
            first: [0.0, 0.0, -1.0],
            second: [1.0, 0.0, 0.0],
        },
        Face {
            normal: [0.0, 0.0, 1.0],
            corner: [-h, -h, h],
            first: [1.0, 0.0, 0.0],
            second: [0.0, 1.0, 0.0],
        },
        Face {
            normal: [0.0, 0.0, -1.0],
            corner: [h, -h, -h],
            first: [-1.0, 0.0, 0.0],
            second: [0.0, 1.0, 0.0],
        },
    ];
    let mut builder = Builder::default();
    for face in faces {
        let base = builder.positions.len() as u32;
        let corner = face.corner;
        let first = face.first;
        let second = face.second;
        let normal = face.normal;
        let corners = [
            corner,
            add(corner, first),
            add(add(corner, first), second),
            add(corner, second),
        ];
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for (step, position) in corners.iter().enumerate() {
            builder.vertex(*position, normal, uvs[step]);
        }
        builder.triangle(base, base + 1, base + 2);
        builder.triangle(base, base + 2, base + 3);
    }
    builder.finish("Cube")
}

fn quad() -> MeshData {
    let h = QUAD_HALF_EXTENT;
    let normal = [0.0, 0.0, -1.0];
    let corners = [
        [-h, -h, 0.0],
        [-h, h, 0.0],
        [h, h, 0.0],
        [h, -h, 0.0],
    ];
    let uvs = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    let mut builder = Builder::default();
    for (step, position) in corners.iter().enumerate() {
        builder.vertex(*position, normal, uvs[step]);
    }
    builder.triangle(0, 1, 2);
    builder.triangle(0, 2, 3);
    builder.finish("Quad")
}

fn plane() -> MeshData {
    let e = PLANE_HALF_EXTENT;
    let normal = [0.0, 1.0, 0.0];
    let corners = [
        [-e, 0.0, -e],
        [-e, 0.0, e],
        [e, 0.0, e],
        [e, 0.0, -e],
    ];
    let uvs = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    let mut builder = Builder::default();
    for (step, position) in corners.iter().enumerate() {
        builder.vertex(*position, normal, uvs[step]);
    }
    builder.triangle(0, 1, 2);
    builder.triangle(0, 2, 3);
    builder.finish("Plane")
}

fn cylinder() -> MeshData {
    let mut builder = Builder::default();
    let bottom = build_ring(&mut builder, CYLINDER_SIDES, |u| {
        let theta = u * 2.0 * PI;
        let (sin_theta, cos_theta) = theta.sin_cos();
        (
            [
                CYLINDER_RADIUS * cos_theta,
                -CYLINDER_HALF_HEIGHT,
                CYLINDER_RADIUS * sin_theta,
            ],
            [cos_theta, 0.0, sin_theta],
            [u, 0.0],
        )
    });
    let top = build_ring(&mut builder, CYLINDER_SIDES, |u| {
        let theta = u * 2.0 * PI;
        let (sin_theta, cos_theta) = theta.sin_cos();
        (
            [
                CYLINDER_RADIUS * cos_theta,
                CYLINDER_HALF_HEIGHT,
                CYLINDER_RADIUS * sin_theta,
            ],
            [cos_theta, 0.0, sin_theta],
            [u, 1.0],
        )
    });
    stitch(&mut builder, &bottom, &top);
    let caps = [
        (CYLINDER_HALF_HEIGHT, [0.0, 1.0, 0.0]),
        (-CYLINDER_HALF_HEIGHT, [0.0, -1.0, 0.0]),
    ];
    for (y, normal) in caps {
        let center = builder.vertex([0.0, y, 0.0], normal, [0.5, 0.5]);
        let ring = build_ring(&mut builder, CYLINDER_SIDES, |u| {
            let theta = u * 2.0 * PI;
            let (sin_theta, cos_theta) = theta.sin_cos();
            (
                [
                    CYLINDER_RADIUS * cos_theta,
                    y,
                    CYLINDER_RADIUS * sin_theta,
                ],
                normal,
                [0.5 + 0.5 * cos_theta, 0.5 + 0.5 * sin_theta],
            )
        });
        for step in 0..CYLINDER_SIDES {
            if normal[1] > 0.0 {
                builder.triangle(center, ring[step + 1], ring[step]);
            } else {
                builder.triangle(center, ring[step], ring[step + 1]);
            }
        }
    }
    builder.finish("Cylinder")
}

fn sphere() -> MeshData {
    let mut builder = Builder::default();
    let bottom_pole = builder.vertex([0.0, -SPHERE_RADIUS, 0.0], [0.0, -1.0, 0.0], [0.5, 0.0]);
    let mut previous: Vec<u32> = Vec::new();
    for ring_index in 1..SPHERE_RINGS {
        let phi = PI * ring_index as f32 / SPHERE_RINGS as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let mut ring = Vec::new();
        for step in 0..=SPHERE_SIDES {
            let u = step as f32 / SPHERE_SIDES as f32;
            let theta = u * 2.0 * PI;
            let (sin_theta, cos_theta) = theta.sin_cos();
            ring.push(builder.vertex(
                [
                    SPHERE_RADIUS * sin_phi * cos_theta,
                    -SPHERE_RADIUS * cos_phi,
                    SPHERE_RADIUS * sin_phi * sin_theta,
                ],
                [sin_phi * cos_theta, -cos_phi, sin_phi * sin_theta],
                [u, ring_index as f32 / SPHERE_RINGS as f32],
            ));
        }
        if ring_index == 1 {
            for step in 0..SPHERE_SIDES {
                builder.triangle(bottom_pole, ring[step], ring[step + 1]);
            }
        } else {
            stitch(&mut builder, &previous, &ring);
        }
        previous = ring;
    }
    let top_pole = builder.vertex([0.0, SPHERE_RADIUS, 0.0], [0.0, 1.0, 0.0], [0.5, 1.0]);
    for step in 0..SPHERE_SIDES {
        builder.triangle(previous[step], top_pole, previous[step + 1]);
    }
    builder.finish("Sphere")
}

fn capsule() -> MeshData {
    let mut builder = Builder::default();
    let bottom = -CAPSULE_HALF_BODY - CAPSULE_RADIUS;
    let top = CAPSULE_HALF_BODY + CAPSULE_RADIUS;
    let mut previous: Vec<u32> = Vec::new();
    let bottom_pole = builder.vertex([0.0, bottom, 0.0], [0.0, -1.0, 0.0], [0.5, 0.0]);
    for ring_index in 1..CAPSULE_CAP_RINGS {
        let phi = (PI / 2.0) * ring_index as f32 / CAPSULE_CAP_RINGS as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let y = bottom + CAPSULE_RADIUS * (1.0 - cos_phi);
        let mut ring = Vec::new();
        for step in 0..=CAPSULE_SIDES {
            let u = step as f32 / CAPSULE_SIDES as f32;
            let theta = u * 2.0 * PI;
            let (sin_theta, cos_theta) = theta.sin_cos();
            ring.push(builder.vertex(
                [
                    CAPSULE_RADIUS * sin_phi * cos_theta,
                    y,
                    CAPSULE_RADIUS * sin_phi * sin_theta,
                ],
                [sin_phi * cos_theta, -cos_phi, sin_phi * sin_theta],
                [u, (y - bottom) / (top - bottom)],
            ));
        }
        if ring_index == 1 {
            for step in 0..CAPSULE_SIDES {
                builder.triangle(bottom_pole, ring[step], ring[step + 1]);
            }
        } else {
            stitch(&mut builder, &previous, &ring);
        }
        previous = ring;
    }
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    for step in 0..=CAPSULE_SIDES {
        let u = step as f32 / CAPSULE_SIDES as f32;
        let theta = u * 2.0 * PI;
        let (sin_theta, cos_theta) = theta.sin_cos();
        let normal = [cos_theta, 0.0, sin_theta];
        lower.push(builder.vertex(
            [CAPSULE_RADIUS * cos_theta, -CAPSULE_HALF_BODY, CAPSULE_RADIUS * sin_theta],
            normal,
            [u, (0.0 - bottom) / (top - bottom)],
        ));
        upper.push(builder.vertex(
            [CAPSULE_RADIUS * cos_theta, CAPSULE_HALF_BODY, CAPSULE_RADIUS * sin_theta],
            normal,
            [u, (CAPSULE_HALF_BODY - bottom) / (top - bottom)],
        ));
    }
    stitch(&mut builder, &previous, &lower);
    stitch(&mut builder, &lower, &upper);
    for ring_index in 1..CAPSULE_CAP_RINGS {
        let phi = (PI / 2.0) * ring_index as f32 / CAPSULE_CAP_RINGS as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let y = CAPSULE_HALF_BODY + CAPSULE_RADIUS * sin_phi;
        let mut ring = Vec::new();
        for step in 0..=CAPSULE_SIDES {
            let u = step as f32 / CAPSULE_SIDES as f32;
            let theta = u * 2.0 * PI;
            let (sin_theta, cos_theta) = theta.sin_cos();
            ring.push(builder.vertex(
                [
                    CAPSULE_RADIUS * cos_phi * cos_theta,
                    y,
                    CAPSULE_RADIUS * cos_phi * sin_theta,
                ],
                [cos_phi * cos_theta, sin_phi, cos_phi * sin_theta],
                [u, (y - bottom) / (top - bottom)],
            ));
        }
        stitch(&mut builder, &previous, &ring);
        previous = ring;
    }
    let top_pole = builder.vertex([0.0, top, 0.0], [0.0, 1.0, 0.0], [0.5, 1.0]);
    for step in 0..CAPSULE_SIDES {
        builder.triangle(previous[step], top_pole, previous[step + 1]);
    }
    builder.finish("Capsule")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face_normal(a: &[f32; 3], b: &[f32; 3], c: &[f32; 3]) -> [f32; 3] {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    const ALL: [BuiltinPrimitive; 6] = [
        BuiltinPrimitive::Cube,
        BuiltinPrimitive::Sphere,
        BuiltinPrimitive::Capsule,
        BuiltinPrimitive::Cylinder,
        BuiltinPrimitive::Plane,
        BuiltinPrimitive::Quad,
    ];

    #[test]
    fn identity_table_holds_unity_file_ids() {
        assert_eq!(builtin_file_id(BuiltinPrimitive::Cube), 10202);
        assert_eq!(builtin_file_id(BuiltinPrimitive::Cylinder), 10206);
        assert_eq!(builtin_file_id(BuiltinPrimitive::Sphere), 10207);
        assert_eq!(builtin_file_id(BuiltinPrimitive::Capsule), 10208);
        assert_eq!(builtin_file_id(BuiltinPrimitive::Plane), 10209);
        assert_eq!(builtin_file_id(BuiltinPrimitive::Quad), 10210);
        for primitive in ALL {
            assert_eq!(builtin_guid(primitive), BUILTIN_EXTRA_GUID);
        }
    }

    #[test]
    fn lookups_resolve_known_names_and_file_ids() {
        for primitive in ALL {
            let name = primitive_mesh(primitive).name;
            assert_eq!(builtin_primitive_by_name(&name), Some(primitive));
            assert_eq!(
                builtin_primitive_by_file_id(builtin_file_id(primitive)),
                Some(primitive)
            );
        }
        assert_eq!(builtin_primitive_by_name("Teapot"), None);
        assert_eq!(builtin_primitive_by_file_id(10203), None);
    }

    #[test]
    fn generators_are_deterministic() {
        for primitive in ALL {
            assert_eq!(primitive_mesh(primitive), primitive_mesh(primitive));
        }
    }

    #[test]
    fn counts_match_the_generators() {
        let cube = primitive_mesh(BuiltinPrimitive::Cube);
        assert_eq!(cube.positions.len(), 24);
        assert_eq!(cube.indices.len(), 36);
        let quad = primitive_mesh(BuiltinPrimitive::Quad);
        assert_eq!(quad.positions.len(), 4);
        assert_eq!(quad.indices.len(), 6);
        let plane = primitive_mesh(BuiltinPrimitive::Plane);
        assert_eq!(plane.positions.len(), 4);
        assert_eq!(plane.indices.len(), 6);
    }

    #[test]
    fn cube_spans_half_a_unit_and_winds_outward() {
        let cube = primitive_mesh(BuiltinPrimitive::Cube);
        for position in &cube.positions {
            assert!(position[0].abs() <= 0.5);
            assert!(position[1].abs() <= 0.5);
            assert!(position[2].abs() <= 0.5);
        }
        let mut top_normals: Vec<[f32; 3]> = Vec::new();
        for step in 0..cube.indices.len() / 3 {
            let a = cube.indices[step * 3] as usize;
            let b = cube.indices[step * 3 + 1] as usize;
            let c = cube.indices[step * 3 + 2] as usize;
            if cube.positions[a][1] == 0.5
                && cube.positions[b][1] == 0.5
                && cube.positions[c][1] == 0.5
            {
                top_normals.push(face_normal(
                    &cube.positions[a],
                    &cube.positions[b],
                    &cube.positions[c],
                ));
            }
        }
        assert_eq!(top_normals.len(), 2);
        for normal in &top_normals {
            assert!((normal[1] - 1.0).abs() < 1e-6);
        }
    }
}
