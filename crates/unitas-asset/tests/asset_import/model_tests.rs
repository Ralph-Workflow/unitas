use unitas_asset::model::fbx::FbxImportError;
use unitas_asset::model::{
    import_fbx, import_gltf, import_model, import_model_sub_assets, import_obj, import_obj_bytes,
    ModelImportError,
};

const CUBE_OBJ: &str = "\
# unit cube
mtllib cube.mtl
o Cube
usemtl Red
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
vn 0.0 0.0 1.0
g Front
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
g Back
f 2/2/1 1/1/1 4/4/1
f 2/2/1 4/4/1 3/3/1
";

const CUBE_OBJ_REORDERED: &str = "\
mtllib cube.mtl
o Cube
usemtl Red
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
vn 0.0 0.0 1.0
g Back
f 2/2/1 1/1/1 4/4/1
f 2/2/1 4/4/1 3/3/1
g Front
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
";

const TRIANGLE_GLTF: &str = "\
{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":42}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":36},
{\"buffer\":0,\"byteOffset\":36,\"byteLength\":6}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":3,\"type\":\"VEC3\"},
{\"bufferView\":1,\"componentType\":5123,\"count\":3,\"type\":\"SCALAR\"}],
\"meshes\":[{\"name\":\"Tri\",\"primitives\":[{\"attributes\":{\"POSITION\":0},\"indices\":1}]}],
\"nodes\":[{\"name\":\"TriNode\",\"mesh\":0}]}";

const TRIANGLE_BIN: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    128, 63, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0,
];

const TRIANGLE_GLTF_DATA_URI: &str = "\
{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":42,\"uri\":\"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA\"}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":36},
{\"buffer\":0,\"byteOffset\":36,\"byteLength\":6}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":3,\"type\":\"VEC3\"},
{\"bufferView\":1,\"componentType\":5123,\"count\":3,\"type\":\"SCALAR\"}],
\"meshes\":[{\"name\":\"Tri\",\"primitives\":[{\"attributes\":{\"POSITION\":0},\"indices\":1}]}],
\"nodes\":[{\"name\":\"TriNode\",\"mesh\":0}]}";

const INTERLEAVED_GLTF: &str = "\
{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":46,\"uri\":\"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAQABAA==\"}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":40,\"byteStride\":20},
{\"buffer\":0,\"byteOffset\":40,\"byteLength\":6}],
\"accessors\":[{\"bufferView\":0,\"byteOffset\":0,\"componentType\":5126,\"count\":2,\"type\":\"VEC3\"},
{\"bufferView\":0,\"byteOffset\":12,\"componentType\":5126,\"count\":2,\"type\":\"VEC2\"},
{\"bufferView\":1,\"componentType\":5123,\"count\":3,\"type\":\"SCALAR\"}],
\"meshes\":[{\"name\":\"Quad\",\"primitives\":[{\"attributes\":{\"POSITION\":0,\"TEXCOORD_0\":1},\"indices\":2}]}],
\"nodes\":[{\"name\":\"QuadNode\",\"mesh\":0}]}";

fn make_glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json_chunk = json.as_bytes().to_vec();
    while !json_chunk.len().is_multiple_of(4) {
        json_chunk.push(b' ');
    }
    let mut bin_chunk = bin.to_vec();
    while !bin_chunk.len().is_multiple_of(4) {
        bin_chunk.push(0);
    }
    let total = 12 + 8 + json_chunk.len() + 8 + bin_chunk.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
    out.extend_from_slice(&json_chunk);
    out.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes());
    out.extend_from_slice(&bin_chunk);
    out
}

fn sub_mesh_positions(mesh: &unitas_asset::primitive::MeshData, id: &str) -> Vec<[f32; 3]> {
    let sub = match mesh.sub_meshes.iter().find(|sub| sub.sub_asset_id == id) {
        Some(sub) => sub,
        None => return Vec::new(),
    };
    let start = sub.index_offset as usize;
    let end = start + sub.index_count as usize;
    mesh.indices[start..end]
        .iter()
        .map(|index| match mesh.positions.get(*index as usize) {
            Some(position) => *position,
            None => [0.0; 3],
        })
        .collect()
}

#[test]
fn obj_with_two_groups_produces_two_sub_meshes() -> Result<(), String> {
    let mesh = match import_obj(CUBE_OBJ) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.positions.len(), 4, "one vertex per (v,vt,vn) triple");
    assert_eq!(mesh.normals.len(), 4);
    assert_eq!(mesh.uvs.len(), 4);
    assert_eq!(
        mesh.indices.len(),
        12,
        "two quads triangulated into four triangles"
    );
    assert_eq!(mesh.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[1], [1.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[3], [0.0, 1.0, 0.0]);
    assert_eq!(mesh.normals[0], [0.0, 0.0, 1.0]);
    assert_eq!(mesh.uvs[0], [0.0, 0.0]);
    assert_eq!(mesh.uvs[2], [1.0, 1.0]);
    assert_eq!(mesh.sub_meshes.len(), 2, "one sub-mesh per group");
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "Front/Red");
    assert_eq!(mesh.sub_meshes[0].name, "Front");
    assert_eq!(mesh.sub_meshes[0].index_offset, 0);
    assert_eq!(mesh.sub_meshes[0].index_count, 6);
    assert_eq!(mesh.sub_meshes[1].sub_asset_id, "Back/Red");
    assert_eq!(mesh.sub_meshes[1].name, "Back");
    assert_eq!(mesh.sub_meshes[1].index_offset, 6);
    assert_eq!(mesh.sub_meshes[1].index_count, 6);
    Ok(())
}

#[test]
fn obj_negative_indices_resolve_relative_to_the_end() -> Result<(), String> {
    let source = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vn 0.0 0.0 1.0
f -3/-3/-1 -2/-2/-1 -1/-1/-1
";
    let mesh = match import_obj(source) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[1], [1.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[2], [0.0, 1.0, 0.0]);
    assert_eq!(mesh.uvs[1], [1.0, 0.0]);
    assert_eq!(mesh.indices, vec![0, 1, 2]);
    assert_eq!(mesh.sub_meshes.len(), 1);
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "default");
    Ok(())
}

#[test]
fn obj_bytes_and_text_agree() -> Result<(), String> {
    let from_text = match import_obj(CUBE_OBJ) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    let from_bytes = match import_obj_bytes(CUBE_OBJ.as_bytes()) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(from_text.positions, from_bytes.positions);
    assert_eq!(from_text.indices, from_bytes.indices);
    assert_eq!(
        from_text
            .sub_meshes
            .iter()
            .map(|sub| sub.sub_asset_id.clone())
            .collect::<Vec<_>>(),
        from_bytes
            .sub_meshes
            .iter()
            .map(|sub| sub.sub_asset_id.clone())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn glb_import_reads_positions_and_indices() -> Result<(), String> {
    let glb = make_glb(TRIANGLE_GLTF, TRIANGLE_BIN);
    let mesh = match import_gltf(&glb) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.name, "TriNode", "the mesh name comes from the node");
    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[1], [1.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[2], [0.0, 1.0, 0.0]);
    assert_eq!(mesh.indices, vec![0, 1, 2]);
    assert_eq!(mesh.sub_meshes.len(), 1, "one sub-mesh per primitive");
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "TriNode#0");
    assert_eq!(mesh.sub_meshes[0].index_offset, 0);
    assert_eq!(mesh.sub_meshes[0].index_count, 3);
    Ok(())
}

#[test]
fn gltf_data_uri_buffer_matches_the_glb_result() -> Result<(), String> {
    let from_gltf = match import_gltf(TRIANGLE_GLTF_DATA_URI.as_bytes()) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    let from_glb = match import_gltf(&make_glb(TRIANGLE_GLTF, TRIANGLE_BIN)) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(from_gltf.positions, from_glb.positions);
    assert_eq!(from_gltf.indices, from_glb.indices);
    assert_eq!(from_gltf.sub_meshes.len(), 1);
    assert_eq!(from_gltf.sub_meshes[0].sub_asset_id, "TriNode#0");
    Ok(())
}

#[test]
fn interleaved_accessor_reads_stride_padded_data() -> Result<(), String> {
    let mesh = match import_gltf(INTERLEAVED_GLTF.as_bytes()) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.positions.len(), 2);
    assert_eq!(mesh.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[1], [1.0, 0.0, 0.0]);
    assert_eq!(mesh.uvs.len(), 2);
    assert_eq!(mesh.uvs[0], [0.0, 0.0]);
    assert_eq!(mesh.uvs[1], [1.0, 0.0]);
    assert_eq!(mesh.indices, vec![0, 1, 1]);
    Ok(())
}

#[test]
fn sub_asset_ids_are_stable_across_reimport_and_reordering() -> Result<(), String> {
    let first = match import_obj(CUBE_OBJ) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    let second = match import_obj(CUBE_OBJ) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    let ids = |mesh: &unitas_asset::primitive::MeshData| -> Vec<String> {
        mesh.sub_meshes
            .iter()
            .map(|sub| sub.sub_asset_id.clone())
            .collect()
    };
    assert_eq!(
        ids(&first),
        ids(&second),
        "the same bytes yield the same ids"
    );

    let reordered = match import_obj(CUBE_OBJ_REORDERED) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    let mut original_ids = ids(&first);
    let mut reordered_ids = ids(&reordered);
    original_ids.sort();
    reordered_ids.sort();
    assert_eq!(
        original_ids, reordered_ids,
        "ids stay bound to the group names"
    );
    assert_eq!(
        sub_mesh_positions(&first, "Front/Red"),
        sub_mesh_positions(&reordered, "Front/Red"),
        "the id still names the same geometry after a reorder"
    );
    assert_eq!(
        sub_mesh_positions(&first, "Back/Red"),
        sub_mesh_positions(&reordered, "Back/Red")
    );
    Ok(())
}

#[test]
fn malformed_obj_reports_a_named_error() -> Result<(), String> {
    let error = match import_obj("v 0 0 0\nf 1 2\n") {
        Ok(_) => return Err("expected a malformed OBJ error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::MalformedObj { line, reason } => {
            assert_eq!(line, 2);
            assert!(
                reason.contains("face"),
                "reason names the problem: {reason}"
            );
        }
        other => return Err(format!("unexpected error: {other:?}")),
    }

    let error = match import_obj("v 0 0 0\nf 1/x 1 1\n") {
        Ok(_) => return Err("expected a malformed OBJ error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::MalformedObj { line, reason } => {
            assert_eq!(line, 2);
            assert!(
                reason.contains("index"),
                "reason names the problem: {reason}"
            );
        }
        other => return Err(format!("unexpected error: {other:?}")),
    }

    let error = match import_obj("v 0 0 0\nv 1 0 0\nf 1 2 9\n") {
        Ok(_) => return Err("expected a malformed OBJ error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::MalformedObj { line, reason } => {
            assert_eq!(line, 3);
            assert!(
                reason.contains("out of range"),
                "reason names the problem: {reason}"
            );
        }
        other => return Err(format!("unexpected error: {other:?}")),
    }
    Ok(())
}

#[test]
fn malformed_gltf_reports_a_named_error() -> Result<(), String> {
    let missing_accessor = "\
{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":42,\"uri\":\"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA\"}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":36}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":3,\"type\":\"VEC3\"}],
\"meshes\":[{\"name\":\"Tri\",\"primitives\":[{\"attributes\":{\"POSITION\":7}}]}],
\"nodes\":[{\"name\":\"TriNode\",\"mesh\":0}]}";
    let error = match import_gltf(missing_accessor.as_bytes()) {
        Ok(_) => return Err("expected a missing accessor error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::MissingAccessor { accessor } => assert_eq!(accessor, 7),
        other => return Err(format!("unexpected error: {other:?}")),
    }

    let bad_buffer = "\
{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":42,\"uri\":\"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA\"}],
\"bufferViews\":[{\"buffer\":3,\"byteOffset\":0,\"byteLength\":36}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":3,\"type\":\"VEC3\"}],
\"meshes\":[{\"name\":\"Tri\",\"primitives\":[{\"attributes\":{\"POSITION\":0}}]}],
\"nodes\":[{\"name\":\"TriNode\",\"mesh\":0}]}";
    let error = match import_gltf(bad_buffer.as_bytes()) {
        Ok(_) => return Err("expected a missing buffer error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::MissingBuffer { buffer } => assert_eq!(buffer, 3),
        other => return Err(format!("unexpected error: {other:?}")),
    }
    Ok(())
}

#[test]
fn import_model_dispatches_on_the_file_name() -> Result<(), String> {
    let mesh = match import_model(CUBE_OBJ.as_bytes(), "cube.obj") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.sub_meshes.len(), 2);
    assert_eq!(
        mesh.name, "cube",
        "the OBJ mesh name comes from the file name"
    );

    let mesh = match import_model(&make_glb(TRIANGLE_GLTF, TRIANGLE_BIN), "tri.glb") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.sub_meshes.len(), 1);

    let mesh = match import_model(TRIANGLE_GLTF_DATA_URI.as_bytes(), "tri.gltf") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.sub_meshes.len(), 1);

    let fbx_bytes = make_fbx(&[make_fbx_geometry(
        "TriQuad",
        &[
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0,
            1.0, 0.0, 1.0, 1.0,
        ],
        &[0, 1, -2, 3, 4, 5, -6],
    )]);
    let mesh = match import_model(&fbx_bytes, "scene.fbx") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(mesh.name, "TriQuad");
    assert_eq!(mesh.positions.len(), 7);
    assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5, 3, 5, 6]);
    assert_eq!(mesh.sub_meshes.len(), 1);
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "TriQuad");

    let error = match import_model(CUBE_OBJ.as_bytes(), "scene.blend") {
        Ok(_) => return Err("expected an unsupported file type error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::UnsupportedFormat { file_name, format } => {
            assert_eq!(file_name, "scene.blend");
            assert_eq!(format, "blend");
        }
        other => return Err(format!("unexpected error: {other:?}")),
    }
    Ok(())
}

#[test]
fn blend_files_report_unsupported_by_name_without_external_conversion() -> Result<(), String> {
    let blender_header = b"BLENDER-v300REND";
    let error = match import_model(blender_header, "Assets/Models/character.blend") {
        Ok(_) => return Err("expected .blend import to be rejected".to_string()),
        Err(error) => error,
    };
    assert_eq!(
        error,
        ModelImportError::UnsupportedFormat {
            file_name: "Assets/Models/character.blend".to_string(),
            format: "blend".to_string(),
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("Assets/Models/character.blend"),
        "expected file name in error message: {message}"
    );
    assert!(
        message.contains("blend"),
        "expected format name in error message: {message}"
    );
    assert!(
        message.to_ascii_lowercase().contains("unsupported"),
        "expected unsupported status in error message: {message}"
    );

    let sub_error = match import_model_sub_assets(blender_header, "character.BLEND") {
        Ok(_) => return Err("expected .BLEND sub-asset import to be rejected".to_string()),
        Err(error) => error,
    };
    assert_eq!(
        sub_error,
        ModelImportError::UnsupportedFormat {
            file_name: "character.BLEND".to_string(),
            format: "blend".to_string(),
        }
    );
    Ok(())
}

struct FbxTestNode<'a> {
    name: &'a str,
    properties: Vec<Vec<u8>>,
    children: Vec<FbxTestNode<'a>>,
}

fn fbx_prop_string(value: &str) -> Vec<u8> {
    let mut out = vec![b'S'];
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out
}

fn fbx_prop_int(value: i32) -> Vec<u8> {
    let mut out = vec![b'I'];
    out.extend_from_slice(&value.to_le_bytes());
    out
}

fn fbx_prop_double_array(values: &[f64]) -> Vec<u8> {
    let mut out = vec![b'd'];
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((values.len() * 8) as u32).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn fbx_prop_int_array(values: &[i32]) -> Vec<u8> {
    let mut out = vec![b'i'];
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((values.len() * 4) as u32).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn build_fbx_node(base: u64, spec: &FbxTestNode) -> Vec<u8> {
    let property_bytes: Vec<u8> = spec.properties.concat();
    let name_len = spec.name.len();
    let header_len = 8 + 4 + 8 + 1 + name_len;
    let children_base = base + header_len as u64 + property_bytes.len() as u64;
    let mut child_bytes: Vec<Vec<u8>> = Vec::new();
    let mut offset = children_base;
    for child in &spec.children {
        let bytes = build_fbx_node(offset, child);
        offset += bytes.len() as u64;
        child_bytes.push(bytes);
    }
    let content_len: usize =
        property_bytes.len() + child_bytes.iter().map(|bytes| bytes.len()).sum::<usize>();
    let end_offset = base + header_len as u64 + content_len as u64;
    let mut out = Vec::new();
    out.extend_from_slice(&end_offset.to_le_bytes());
    out.extend_from_slice(&(spec.properties.len() as u32).to_le_bytes());
    out.extend_from_slice(&(property_bytes.len() as u64).to_le_bytes());
    out.push(name_len as u8);
    out.extend_from_slice(spec.name.as_bytes());
    out.extend_from_slice(&property_bytes);
    for bytes in child_bytes {
        out.extend_from_slice(&bytes);
    }
    out
}

fn make_fbx(objects_children: &[FbxTestNode]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"Kaydara FBX Binary  \x00");
    bytes.extend_from_slice(&7500u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 16]);
    let mut children = Vec::with_capacity(objects_children.len());
    for child in objects_children {
        children.push(clone_fbx_node(child));
    }
    let root = FbxTestNode {
        name: "Objects",
        properties: Vec::new(),
        children,
    };
    let root_bytes = build_fbx_node(bytes.len() as u64, &root);
    bytes.extend_from_slice(&root_bytes);
    bytes
}

fn clone_fbx_node<'a>(node: &FbxTestNode<'a>) -> FbxTestNode<'a> {
    FbxTestNode {
        name: node.name,
        properties: node.properties.clone(),
        children: node.children.iter().map(clone_fbx_node).collect(),
    }
}

fn make_fbx_geometry<'a>(name: &'a str, vertices: &[f64], polygons: &[i32]) -> FbxTestNode<'a> {
    FbxTestNode {
        name: "Geometry",
        properties: vec![fbx_prop_int(124), fbx_prop_string(name)],
        children: vec![
            FbxTestNode {
                name: "Vertices",
                properties: vec![fbx_prop_double_array(vertices)],
                children: Vec::new(),
            },
            FbxTestNode {
                name: "PolygonVertexIndex",
                properties: vec![fbx_prop_int_array(polygons)],
                children: Vec::new(),
            },
        ],
    }
}

#[test]
fn import_model_combines_multiple_fbx_geometry_nodes_with_stable_sub_asset_ids(
) -> Result<(), String> {
    let first = make_fbx_geometry(
        "Hull",
        &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        &[0, 1, -2],
    );
    let second = make_fbx_geometry(
        "Turret",
        &[0.0, 0.0, 2.0, 1.0, 0.0, 2.0, 1.0, 1.0, 2.0, 0.0, 1.0, 2.0],
        &[0, 1, 2, -3],
    );
    let fbx_bytes = make_fbx(&[first, second]);
    let direct = match import_fbx(&fbx_bytes) {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("direct import_fbx failed: {error:?}")),
    };
    assert_eq!(direct.len(), 2);

    let mesh = match import_model(&fbx_bytes, "tank.FBX") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import_model failed: {error:?}")),
    };
    assert_eq!(mesh.name, "Hull");
    assert_eq!(mesh.positions.len(), 7);
    assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5, 3, 5, 6]);
    assert_eq!(mesh.sub_meshes.len(), 2);
    assert_eq!(mesh.sub_meshes[0].sub_asset_id, "Hull");
    assert_eq!(mesh.sub_meshes[0].index_offset, 0);
    assert_eq!(mesh.sub_meshes[0].index_count, 3);
    assert_eq!(mesh.sub_meshes[1].sub_asset_id, "Turret");
    assert_eq!(mesh.sub_meshes[1].index_offset, 3);
    assert_eq!(mesh.sub_meshes[1].index_count, 6);
    Ok(())
}

#[test]
fn import_model_maps_fbx_rig_and_animation_rejection_and_malformed_errors() -> Result<(), String> {
    let rig_bytes = make_fbx(&[FbxTestNode {
        name: "Deformer",
        properties: vec![fbx_prop_string("Skin")],
        children: Vec::new(),
    }]);
    let error = match import_model(&rig_bytes, "rigged.fbx") {
        Ok(_) => return Err("expected an unsupported FBX rig error".to_string()),
        Err(error) => error,
    };
    match &error {
        ModelImportError::Fbx(FbxImportError::UnsupportedContent { feature }) => {
            assert_eq!(feature, "fbx_rig");
        }
        other => return Err(format!("unexpected error for rig: {other:?}")),
    }
    assert!(
        error.to_string().contains("fbx_rig"),
        "display diagnostic names fbx_rig: {error}"
    );

    let anim_bytes = make_fbx(&[FbxTestNode {
        name: "AnimationStack",
        properties: Vec::new(),
        children: Vec::new(),
    }]);
    let error = match import_model(&anim_bytes, "animated.fbx") {
        Ok(_) => return Err("expected an unsupported FBX animation error".to_string()),
        Err(error) => error,
    };
    match &error {
        ModelImportError::Fbx(FbxImportError::UnsupportedContent { feature }) => {
            assert_eq!(feature, "fbx_animation");
        }
        other => return Err(format!("unexpected error for animation: {other:?}")),
    }
    assert!(
        error.to_string().contains("fbx_animation"),
        "display diagnostic names fbx_animation: {error}"
    );

    let error = match import_model(b"not an fbx stream", "corrupt.fbx") {
        Ok(_) => return Err("expected an invalid FBX signature error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::Fbx(FbxImportError::InvalidSignature) => {}
        other => return Err(format!("unexpected error for invalid signature: {other:?}")),
    }

    let empty_fbx = make_fbx(&[]);
    let error = match import_model(&empty_fbx, "empty.fbx") {
        Ok(_) => return Err("expected a missing FBX geometry error".to_string()),
        Err(error) => error,
    };
    match error {
        ModelImportError::Fbx(FbxImportError::MissingGeometry) => {}
        other => return Err(format!("unexpected error for missing geometry: {other:?}")),
    }
    Ok(())
}

fn sub_asset_ids(meshes: &[unitas_asset::primitive::MeshData]) -> Vec<String> {
    meshes
        .iter()
        .flat_map(|mesh| {
            mesh.sub_meshes
                .iter()
                .map(|sub| sub.sub_asset_id.clone())
                .collect::<Vec<String>>()
        })
        .collect()
}

fn two_mesh_fbx_bytes() -> Vec<u8> {
    let hull = make_fbx_geometry(
        "Hull",
        &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        &[0, 1, -2],
    );
    let turret = make_fbx_geometry(
        "Turret",
        &[0.0, 0.0, 2.0, 1.0, 0.0, 2.0, 1.0, 1.0, 2.0, 0.0, 1.0, 2.0],
        &[0, 1, 2, -3],
    );
    make_fbx(&[hull, turret])
}

#[test]
fn import_model_sub_assets_keeps_every_fbx_mesh_identity() -> Result<(), String> {
    let fbx_bytes = two_mesh_fbx_bytes();

    let meshes = match import_model_sub_assets(&fbx_bytes, "tank.fbx") {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(meshes.len(), 2, "each source mesh keeps its own identity");
    assert_eq!(meshes[0].name, "Hull");
    assert_eq!(meshes[0].positions.len(), 3);
    assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    assert_eq!(meshes[0].sub_meshes.len(), 1);
    assert_eq!(meshes[0].sub_meshes[0].sub_asset_id, "Hull");
    assert_eq!(meshes[1].name, "Turret");
    assert_eq!(meshes[1].positions.len(), 4);
    assert_eq!(meshes[1].indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(meshes[1].sub_meshes[0].sub_asset_id, "Turret");

    let combined = match import_model(&fbx_bytes, "tank.fbx") {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(
        combined.positions.len(),
        7,
        "the combined convenience path still merges the meshes"
    );
    assert_eq!(combined.sub_meshes.len(), 2);
    assert_eq!(combined.sub_meshes[1].sub_asset_id, "Turret");
    Ok(())
}

#[test]
fn import_model_sub_assets_identities_survive_reimport() -> Result<(), String> {
    let fbx_bytes = two_mesh_fbx_bytes();
    let first = match import_model_sub_assets(&fbx_bytes, "tank.fbx") {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("first import failed: {error:?}")),
    };
    let second = match import_model_sub_assets(&fbx_bytes, "tank.fbx") {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("second import failed: {error:?}")),
    };
    assert_eq!(
        sub_asset_ids(&first),
        vec!["Hull".to_string(), "Turret".to_string()]
    );
    assert_eq!(
        sub_asset_ids(&second),
        sub_asset_ids(&first),
        "reimporting the same bytes keeps the sub-asset identities"
    );
    Ok(())
}

#[test]
fn import_model_sub_assets_returns_one_mesh_for_obj_and_gltf() -> Result<(), String> {
    let meshes = match import_model_sub_assets(CUBE_OBJ.as_bytes(), "cube.obj") {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].name, "cube");
    assert_eq!(
        sub_asset_ids(&meshes),
        vec!["Front/Red".to_string(), "Back/Red".to_string()]
    );

    let meshes = match import_model_sub_assets(&make_glb(TRIANGLE_GLTF, TRIANGLE_BIN), "tri.glb") {
        Ok(meshes) => meshes,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(meshes.len(), 1);
    assert_eq!(sub_asset_ids(&meshes), vec!["TriNode#0".to_string()]);
    Ok(())
}
