use std::error::Error;

use unitas_asset::primitive::{
    builtin_file_id, builtin_guid, builtin_primitive_by_file_id, builtin_primitive_by_name,
    primitive_mesh, BuiltinPrimitive, MeshData, BUILTIN_EXTRA_GUID,
};

type TestResult = Result<(), Box<dyn Error>>;

const ALL_PRIMITIVES: [BuiltinPrimitive; 6] = [
    BuiltinPrimitive::Cube,
    BuiltinPrimitive::Sphere,
    BuiltinPrimitive::Capsule,
    BuiltinPrimitive::Cylinder,
    BuiltinPrimitive::Plane,
    BuiltinPrimitive::Quad,
];

/// Unity's built-in resource identity, one row per primitive.
const EXPECTED_IDENTITY: [(BuiltinPrimitive, i64); 6] = [
    (BuiltinPrimitive::Cube, 10202),
    (BuiltinPrimitive::Cylinder, 10206),
    (BuiltinPrimitive::Sphere, 10207),
    (BuiltinPrimitive::Capsule, 10208),
    (BuiltinPrimitive::Plane, 10209),
    (BuiltinPrimitive::Quad, 10210),
];

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn face_normal(mesh: &MeshData, triangle: usize) -> [f32; 3] {
    let a = mesh.indices[triangle * 3] as usize;
    let b = mesh.indices[triangle * 3 + 1] as usize;
    let c = mesh.indices[triangle * 3 + 2] as usize;
    let first = [
        mesh.positions[b][0] - mesh.positions[a][0],
        mesh.positions[b][1] - mesh.positions[a][1],
        mesh.positions[b][2] - mesh.positions[a][2],
    ];
    let second = [
        mesh.positions[c][0] - mesh.positions[a][0],
        mesh.positions[c][1] - mesh.positions[a][1],
        mesh.positions[c][2] - mesh.positions[a][2],
    ];
    cross(&first, &second)
}

fn bounds(mesh: &MeshData) -> ([f32; 3], [f32; 3]) {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    for position in &mesh.positions {
        for axis in 0..3 {
            if position[axis] < minimum[axis] {
                minimum[axis] = position[axis];
            }
            if position[axis] > maximum[axis] {
                maximum[axis] = position[axis];
            }
        }
    }
    (minimum, maximum)
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "expected {expected}, got {actual}"
    );
}

fn assert_bounds(mesh: &MeshData, expected_minimum: [f32; 3], expected_maximum: [f32; 3]) {
    let (minimum, maximum) = bounds(mesh);
    for axis in 0..3 {
        assert_close(minimum[axis], expected_minimum[axis]);
        assert_close(maximum[axis], expected_maximum[axis]);
    }
}

#[test]
fn cube_has_twenty_four_vertices_and_thirty_six_indices() -> TestResult {
    let cube = primitive_mesh(BuiltinPrimitive::Cube);
    assert_eq!(cube.positions.len(), 24);
    assert_eq!(cube.normals.len(), 24);
    assert_eq!(cube.uvs.len(), 24);
    assert_eq!(cube.indices.len(), 36);
    assert_eq!(cube.name, "Cube");
    Ok(())
}

#[test]
fn quad_and_plane_are_single_quads() -> TestResult {
    let quad = primitive_mesh(BuiltinPrimitive::Quad);
    assert_eq!(quad.positions.len(), 4);
    assert_eq!(quad.indices.len(), 6);
    let plane = primitive_mesh(BuiltinPrimitive::Plane);
    assert_eq!(plane.positions.len(), 4);
    assert_eq!(plane.indices.len(), 6);
    Ok(())
}

#[test]
fn cylinder_has_a_side_band_and_two_axis_aligned_caps() -> TestResult {
    let cylinder = primitive_mesh(BuiltinPrimitive::Cylinder);
    assert_eq!(cylinder.positions.len(), 102);
    assert_eq!(cylinder.indices.len(), 288);
    assert_bounds(&cylinder, [-0.5, -1.0, -0.5], [0.5, 1.0, 0.5]);
    let mut cap_normals = 0;
    for normal in &cylinder.normals {
        let radial = (normal[0] * normal[0] + normal[2] * normal[2]).sqrt();
        if radial < 1e-6 {
            assert!(normal[1].abs() > 0.999);
            cap_normals += 1;
        } else {
            assert_close(normal[1], 0.0);
        }
    }
    assert_eq!(cap_normals, 52);
    Ok(())
}

#[test]
fn capsule_is_a_closed_capsule_around_a_unit_high_cylinder() -> TestResult {
    let capsule = primitive_mesh(BuiltinPrimitive::Capsule);
    assert_eq!(capsule.positions.len(), 302);
    assert_eq!(capsule.indices.len(), 1728);
    assert_bounds(&capsule, [-0.5, -1.0, -0.5], [0.5, 1.0, 0.5]);
    for position in &capsule.positions {
        let closest = position[1].clamp(-0.5, 0.5);
        let offset = [
            position[0],
            position[1] - closest,
            position[2],
        ];
        let distance =
            (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert_close(distance, 0.5);
    }
    Ok(())
}

#[test]
fn sphere_is_a_unit_uv_sphere_with_poles_on_the_y_axis() -> TestResult {
    let sphere = primitive_mesh(BuiltinPrimitive::Sphere);
    assert_eq!(sphere.positions.len(), 277);
    assert_eq!(sphere.indices.len(), 1584);
    assert_bounds(&sphere, [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
    let poles: Vec<[f32; 3]> = sphere
        .positions
        .iter()
        .copied()
        .filter(|position| position[0].abs() < 1e-6 && position[2].abs() < 1e-6)
        .collect();
    assert_eq!(poles.len(), 2);
    assert_close(poles[0][1], -1.0);
    assert_close(poles[1][1], 1.0);
    Ok(())
}

#[test]
fn meshes_span_their_axis_aligned_bounds() -> TestResult {
    assert_bounds(
        &primitive_mesh(BuiltinPrimitive::Cube),
        [-0.5, -0.5, -0.5],
        [0.5, 0.5, 0.5],
    );
    assert_bounds(
        &primitive_mesh(BuiltinPrimitive::Quad),
        [-0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
    );
    assert_bounds(
        &primitive_mesh(BuiltinPrimitive::Plane),
        [-5.0, 0.0, -5.0],
        [5.0, 0.0, 5.0],
    );
    Ok(())
}

#[test]
fn vertex_normals_point_outward() -> TestResult {
    let cube = primitive_mesh(BuiltinPrimitive::Cube);
    for normal in &cube.normals {
        let axes = normal[0].abs() + normal[1].abs() + normal[2].abs();
        assert_close(axes, 1.0);
    }
    let sphere = primitive_mesh(BuiltinPrimitive::Sphere);
    for (position, normal) in sphere.positions.iter().zip(&sphere.normals) {
        assert_close(dot(position, normal), 1.0);
    }
    let plane = primitive_mesh(BuiltinPrimitive::Plane);
    for normal in &plane.normals {
        assert!((normal[0]).abs() < 1e-6);
        assert_close(normal[1], 1.0);
        assert!((normal[2]).abs() < 1e-6);
    }
    let quad = primitive_mesh(BuiltinPrimitive::Quad);
    for normal in &quad.normals {
        assert!((normal[0]).abs() < 1e-6);
        assert!((normal[1]).abs() < 1e-6);
        assert_close(normal[2], -1.0);
    }
    Ok(())
}

#[test]
fn triangles_are_wound_consistently_outward() -> TestResult {
    for primitive in ALL_PRIMITIVES {
        let mesh = primitive_mesh(primitive);
        assert_eq!(mesh.indices.len() % 3, 0);
        for triangle in 0..mesh.indices.len() / 3 {
            let normal = face_normal(&mesh, triangle);
            let mut average = [0.0; 3];
            for corner in 0..3 {
                let index = mesh.indices[triangle * 3 + corner] as usize;
                assert!(index < mesh.positions.len());
                for (axis, value) in average.iter_mut().enumerate() {
                    *value += mesh.normals[index][axis];
                }
            }
            assert!(
                dot(&normal, &average) > 0.0,
                "{:?} triangle {triangle} is wound inwards",
                primitive
            );
        }
    }
    Ok(())
}

#[test]
fn cube_top_face_triangle_normal_points_up() -> TestResult {
    let cube = primitive_mesh(BuiltinPrimitive::Cube);
    let mut top_faces = 0;
    for triangle in 0..cube.indices.len() / 3 {
        let a = cube.indices[triangle * 3] as usize;
        let b = cube.indices[triangle * 3 + 1] as usize;
        let c = cube.indices[triangle * 3 + 2] as usize;
        if cube.positions[a][1] == 0.5
            && cube.positions[b][1] == 0.5
            && cube.positions[c][1] == 0.5
        {
            let normal = face_normal(&cube, triangle);
            assert_close(normal[1], 1.0);
            top_faces += 1;
        }
    }
    assert_eq!(top_faces, 2);
    Ok(())
}

#[test]
fn identity_table_matches_unitys_builtin_resources() -> TestResult {
    assert_eq!(BUILTIN_EXTRA_GUID, "0000000000000000e000000000000000");
    for (primitive, file_id) in EXPECTED_IDENTITY {
        assert_eq!(builtin_file_id(primitive), file_id);
        assert_eq!(builtin_guid(primitive), BUILTIN_EXTRA_GUID);
    }
    Ok(())
}

#[test]
fn lookups_resolve_known_names_and_file_ids() -> TestResult {
    for (primitive, file_id) in EXPECTED_IDENTITY {
        let name = primitive_mesh(primitive).name;
        assert_eq!(builtin_primitive_by_name(&name), Some(primitive));
        assert_eq!(builtin_primitive_by_file_id(file_id), Some(primitive));
    }
    assert_eq!(builtin_primitive_by_name("Teapot"), None);
    assert_eq!(builtin_primitive_by_name(""), None);
    assert_eq!(builtin_primitive_by_file_id(10203), None);
    assert_eq!(builtin_primitive_by_file_id(0), None);
    Ok(())
}

#[test]
fn generators_are_deterministic() -> TestResult {
    for primitive in ALL_PRIMITIVES {
        assert_eq!(primitive_mesh(primitive), primitive_mesh(primitive));
    }
    Ok(())
}

#[test]
fn sub_meshes_cover_the_mesh_with_stable_sub_asset_ids() -> TestResult {
    for primitive in ALL_PRIMITIVES {
        let mesh = primitive_mesh(primitive);
        assert_eq!(mesh.sub_meshes.len(), 1);
        let sub_mesh = &mesh.sub_meshes[0];
        assert_eq!(sub_mesh.index_offset, 0);
        assert_eq!(sub_mesh.index_count as usize, mesh.indices.len());
        assert_eq!(sub_mesh.sub_asset_id, mesh.name);
        assert_eq!(sub_mesh.name, mesh.name);
    }
    Ok(())
}
