//! Binary FBX geometry import tests (PRJ-004).

use std::error::Error;

use unitas_asset::model::fbx::{import_fbx, import_fbx_bytes, FbxImportError};

const FBX_VERSION_64: u32 = 7500;
const FBX_VERSION_32: u32 = 7400;

/// A node record to emit: name, encoded properties, nested records.
struct NodeSpec<'a> {
    name: &'a str,
    properties: Vec<Vec<u8>>,
    children: Vec<NodeSpec<'a>>,
}

fn node<'a>(name: &'a str, properties: Vec<Vec<u8>>, children: Vec<NodeSpec<'a>>) -> NodeSpec<'a> {
    NodeSpec {
        name,
        properties,
        children,
    }
}

fn prop_string(value: &str) -> Vec<u8> {
    let mut out = vec![b'S'];
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out
}

fn prop_int(value: i32) -> Vec<u8> {
    let mut out = vec![b'I'];
    out.extend_from_slice(&value.to_le_bytes());
    out
}

fn prop_double_array(values: &[f64]) -> Vec<u8> {
    let mut out = vec![b'd'];
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((values.len() * 8) as u32).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn prop_int_array(values: &[i32]) -> Vec<u8> {
    let mut out = vec![b'i'];
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((values.len() * 4) as u32).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// An array property whose payload is a zlib stream, as FBX writes for deflate arrays.
fn prop_compressed_double_array(values: &[f64]) -> Vec<u8> {
    let mut raw = Vec::new();
    for value in values {
        raw.extend_from_slice(&value.to_le_bytes());
    }
    let compressed = zlib_stored(&raw);
    let mut out = vec![b'd'];
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    out.extend_from_slice(&compressed);
    out
}

/// A deflate array whose payload is not a zlib stream.
fn prop_broken_compressed_array() -> Vec<u8> {
    let mut out = vec![b'd'];
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    out
}

/// zlib container holding one stored (uncompressed) deflate block.
fn zlib_stored(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01, 0x01];
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(&(!(payload.len() as u16)).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&adler32(payload).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut low: u32 = 1;
    let mut high: u32 = 0;
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

/// Encodes one node record; `base` is the absolute offset the record is written at.
fn build_node(base: u64, spec: &NodeSpec, is_64: bool) -> Vec<u8> {
    let property_bytes: Vec<u8> = spec.properties.concat();
    let name_len = spec.name.len();
    let header_len = if is_64 {
        8 + 4 + 8 + 1 + name_len
    } else {
        4 + 4 + 4 + 1 + name_len
    };
    let children_base = base + header_len as u64 + property_bytes.len() as u64;
    let mut child_bytes: Vec<Vec<u8>> = Vec::new();
    let mut offset = children_base;
    for child in &spec.children {
        let bytes = build_node(offset, child, is_64);
        offset += bytes.len() as u64;
        child_bytes.push(bytes);
    }
    let content_len: usize = property_bytes.len()
        + child_bytes
            .iter()
            .map(|bytes| bytes.len())
            .sum::<usize>();
    let end_offset = base + header_len as u64 + content_len as u64;
    let mut out = Vec::new();
    if is_64 {
        out.extend_from_slice(&end_offset.to_le_bytes());
        out.extend_from_slice(&(spec.properties.len() as u32).to_le_bytes());
        out.extend_from_slice(&(property_bytes.len() as u64).to_le_bytes());
    } else {
        out.extend_from_slice(&(end_offset as u32).to_le_bytes());
        out.extend_from_slice(&(spec.properties.len() as u32).to_le_bytes());
        out.extend_from_slice(&(property_bytes.len() as u32).to_le_bytes());
    }
    out.push(name_len as u8);
    out.extend_from_slice(spec.name.as_bytes());
    out.extend_from_slice(&property_bytes);
    for bytes in child_bytes {
        out.extend_from_slice(&bytes);
    }
    out
}

/// Writes a binary FBX file whose root record is `root`.
fn fbx_file(root: &NodeSpec, version: u32) -> Vec<u8> {
    let is_64 = version >= FBX_VERSION_64;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"Kaydara FBX Binary  \x00");
    bytes.extend_from_slice(&version.to_le_bytes());
    if is_64 {
        bytes.extend_from_slice(&[0u8; 16]);
    } else {
        bytes.extend_from_slice(&[0u8; 2]);
    }
    let root_bytes = build_node(bytes.len() as u64, root, is_64);
    bytes.extend_from_slice(&root_bytes);
    bytes
}

fn geometry_node<'a>(
    name: &'a str,
    vertices: Vec<Vec<u8>>,
    polygons: Vec<Vec<u8>>,
    layers: Vec<NodeSpec<'a>>,
) -> NodeSpec<'a> {
    let mut children = vec![
        node("Vertices", vertices, vec![]),
        node("PolygonVertexIndex", polygons, vec![]),
    ];
    children.extend(layers);
    node(
        "Geometry",
        vec![prop_int(124), prop_string(name)],
        children,
    )
}

/// One triangle and one quad over seven control points.
fn tri_quad_geometry() -> NodeSpec<'static> {
    let vertices = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
        0.0, 1.0, 1.0,
    ];
    let polygons = [0, 1, -2, 3, 4, 5, -6];
    geometry_node(
        "TriQuad",
        vec![prop_double_array(&vertices)],
        vec![prop_int_array(&polygons)],
        vec![],
    )
}

fn objects_root<'a>(children: Vec<NodeSpec<'a>>) -> NodeSpec<'a> {
    node("Objects", vec![], children)
}

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn imports_triangle_and_quad_with_fan_triangulation() -> TestResult {
    let bytes = fbx_file(&objects_root(vec![tri_quad_geometry()]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 1);
    let mesh = &meshes[0];
    assert_eq!(mesh.name, "TriQuad");
    assert_eq!(mesh.positions.len(), 7);
    assert_eq!(mesh.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[6], [0.0, 1.0, 1.0]);
    assert_eq!(mesh.indices.len(), 9);
    assert_eq!(
        mesh.indices,
        vec![0, 1, 2, 3, 4, 5, 3, 5, 6],
        "the quad fans from its first vertex"
    );
    assert_eq!(mesh.sub_meshes.len(), 1);
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "TriQuad");
    assert_eq!(mesh.sub_meshes[0].index_offset, 0);
    assert_eq!(mesh.sub_meshes[0].index_count, 9);
    Ok(())
}

#[test]
fn imports_direct_mapped_normals() -> TestResult {
    let vertices = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let normals = [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0];
    let layer = node(
        "LayerElementNormal",
        vec![
            prop_string("Direct"),
            prop_string("Direct"),
            prop_double_array(&normals),
        ],
        vec![],
    );
    let geometry = geometry_node(
        "Tri",
        vec![prop_double_array(&vertices)],
        vec![prop_int_array(&[0, 1, -2])],
        vec![layer],
    );
    let bytes = fbx_file(&objects_root(vec![geometry]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 1);
    assert_eq!(
        meshes[0].normals,
        vec![[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]
    );
    Ok(())
}

#[test]
fn imports_index_to_direct_uvs() -> TestResult {
    let vertices = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
    ];
    let uvs = [0.0, 0.0, 1.0, 1.0];
    let layer = node(
        "LayerElementUV",
        vec![
            prop_string("IndexToDirect"),
            prop_string("IndexToDirect"),
            prop_double_array(&uvs),
            prop_int_array(&[0, 1, 1, 0]),
        ],
        vec![],
    );
    let geometry = geometry_node(
        "Quad",
        vec![prop_double_array(&vertices)],
        vec![prop_int_array(&[0, 1, 2, -3])],
        vec![layer],
    );
    let bytes = fbx_file(&objects_root(vec![geometry]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 1);
    assert_eq!(
        meshes[0].uvs,
        vec![[0.0, 0.0], [1.0, 1.0], [1.0, 1.0], [0.0, 0.0]]
    );
    Ok(())
}

#[test]
fn imports_compressed_array_payload() -> TestResult {
    let vertices = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let geometry = geometry_node(
        "Compressed",
        vec![prop_compressed_double_array(&vertices)],
        vec![prop_int_array(&[0, 1, -2])],
        vec![],
    );
    let bytes = fbx_file(&objects_root(vec![geometry]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].positions.len(), 3);
    assert_eq!(meshes[0].positions[1], [1.0, 0.0, 0.0]);
    assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    Ok(())
}

#[test]
fn imports_compressed_node_payload() -> TestResult {
    let vertices = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let vertices_node = build_node(
        0,
        &node(
            "Vertices",
            vec![prop_double_array(&vertices)],
            vec![],
        ),
        true,
    );
    let polygons_node = build_node(
        vertices_node.len() as u64,
        &node(
            "PolygonVertexIndex",
            vec![prop_int_array(&[0, 1, -2])],
            vec![],
        ),
        true,
    );
    let mut payload = vertices_node;
    payload.extend_from_slice(&polygons_node);
    let compressed = zlib_stored(&payload);
    let mut property = vec![b'd'];
    property.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    property.extend_from_slice(&1u32.to_le_bytes());
    property.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    property.extend_from_slice(&compressed);
    let geometry = node("Compressed", vec![property], vec![]);
    let bytes = fbx_file(&objects_root(vec![geometry]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].positions.len(), 3);
    assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    Ok(())
}

#[test]
fn reports_unsupported_rig_content() -> TestResult {
    let bytes = fbx_file(
        &objects_root(vec![node("Deformer", vec![prop_string("Skin")], vec![])]),
        FBX_VERSION_64,
    );
    match import_fbx(&bytes) {
        Err(FbxImportError::UnsupportedContent { feature }) => {
            assert_eq!(feature, "fbx_rig");
        }
        other => return Err(format!("expected unsupported rig content, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn reports_unsupported_animation_content() -> TestResult {
    let bytes = fbx_file(
        &objects_root(vec![node("AnimationCurve", vec![], vec![])]),
        FBX_VERSION_64,
    );
    match import_fbx(&bytes) {
        Err(FbxImportError::UnsupportedContent { feature }) => {
            assert_eq!(feature, "fbx_animation");
        }
        other => {
            return Err(format!("expected unsupported animation content, got {other:?}").into());
        }
    }
    Ok(())
}

#[test]
fn reports_truncated_file() -> TestResult {
    let bytes = fbx_file(&objects_root(vec![tri_quad_geometry()]), FBX_VERSION_64);
    let truncated = &bytes[..bytes.len() / 2];
    match import_fbx(truncated) {
        Err(FbxImportError::Truncated) => {}
        other => return Err(format!("expected truncated error, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn reports_broken_compressed_payload() -> TestResult {
    let geometry = geometry_node(
        "Broken",
        vec![prop_broken_compressed_array()],
        vec![prop_int_array(&[0, 1, -2])],
        vec![],
    );
    let bytes = fbx_file(&objects_root(vec![geometry]), FBX_VERSION_64);
    match import_fbx(&bytes) {
        Err(FbxImportError::Inflate(_)) => {}
        other => return Err(format!("expected inflate error, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn imports_32_bit_node_records() -> TestResult {
    let bytes = fbx_file(&objects_root(vec![tri_quad_geometry()]), FBX_VERSION_32);
    let meshes = import_fbx_bytes(&bytes)?;
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].positions.len(), 7);
    assert_eq!(meshes[0].indices, vec![0, 1, 2, 3, 4, 5, 3, 5, 6]);
    Ok(())
}

#[test]
fn reports_missing_geometry() -> TestResult {
    let bytes = fbx_file(&objects_root(vec![]), FBX_VERSION_64);
    match import_fbx(&bytes) {
        Err(FbxImportError::MissingGeometry) => {}
        other => return Err(format!("expected missing geometry error, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn reports_invalid_signature() -> TestResult {
    let mut bytes = fbx_file(&objects_root(vec![tri_quad_geometry()]), FBX_VERSION_64);
    bytes[0] = b'X';
    match import_fbx(&bytes) {
        Err(FbxImportError::InvalidSignature) => {}
        other => return Err(format!("expected invalid signature error, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn geometry_nodes_keep_their_own_mesh_identity() -> TestResult {
    let hull = geometry_node(
        "Hull",
        vec![prop_double_array(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])],
        vec![prop_int_array(&[0, 1, -2])],
        vec![],
    );
    let turret = geometry_node(
        "Turret",
        vec![prop_double_array(&[
            0.0, 0.0, 2.0, 1.0, 0.0, 2.0, 1.0, 1.0, 2.0, 0.0, 1.0, 2.0,
        ])],
        vec![prop_int_array(&[0, 1, 2, -3])],
        vec![],
    );
    let bytes = fbx_file(&objects_root(vec![hull, turret]), FBX_VERSION_64);
    let meshes = import_fbx(&bytes)?;
    assert_eq!(meshes.len(), 2, "each geometry node stays its own mesh");
    assert_eq!(meshes[0].name, "Hull");
    assert_eq!(meshes[0].positions.len(), 3);
    assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    assert_eq!(meshes[0].sub_meshes[0].sub_asset_id, "Hull");
    assert_eq!(meshes[1].name, "Turret");
    assert_eq!(meshes[1].positions.len(), 4);
    assert_eq!(meshes[1].indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(meshes[1].sub_meshes[0].sub_asset_id, "Turret");
    Ok(())
}
