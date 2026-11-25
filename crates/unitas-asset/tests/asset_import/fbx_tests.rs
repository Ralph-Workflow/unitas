use std::error::Error;

use unitas_asset::model::fbx::{
    import_fbx, import_fbx_bytes, import_fbx_with_settings, FbxAnimationCompression,
    FbxAnimationType, FbxClipDefinition, FbxImportError, FbxImportSettings, FbxMaterialLocation,
};

const FBX_VERSION_64: u32 = 7500;
const FBX_VERSION_32: u32 = 7400;

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

fn prop_broken_compressed_array() -> Vec<u8> {
    let mut out = vec![b'd'];
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    out
}

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
    let content_len: usize =
        property_bytes.len() + child_bytes.iter().map(|bytes| bytes.len()).sum::<usize>();
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
    node("Geometry", vec![prop_int(124), prop_string(name)], children)
}

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

fn objects_root(children: Vec<NodeSpec<'_>>) -> NodeSpec<'_> {
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
    let vertices = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
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
        &node("Vertices", vec![prop_double_array(&vertices)], vec![]),
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
fn prj004_outcome15_humanoid_rig_content_is_reported_unsupported() -> TestResult {
    let rig_bytes = fbx_file(
        &objects_root(vec![node("Deformer", vec![prop_string("Skin")], vec![])]),
        FBX_VERSION_64,
    );
    match import_fbx(&rig_bytes) {
        Err(FbxImportError::UnsupportedContent { feature }) => {
            assert_eq!(feature, "fbx_rig");
        }
        other => return Err(format!("expected unsupported rig content, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn prj004_outcome15_humanoid_animation_content_is_reported_unsupported() -> TestResult {
    let animation_bytes = fbx_file(
        &objects_root(vec![node("AnimationCurve", vec![], vec![])]),
        FBX_VERSION_64,
    );
    match import_fbx(&animation_bytes) {
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
        vec![prop_double_array(&[
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ])],
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

#[test]
fn prj004_outcome15_humanoid_fbx_imports_avatar_split_clips_with_loop_pose_baked_root_rotation_optimal_compression_and_extracted_materials(
) -> TestResult {
    let body_geom = tri_quad_geometry();
    let hips_bone = node(
        "Model",
        vec![prop_int(200), prop_string("Mixamo_Hips")],
        vec![],
    );
    let spine_bone = node(
        "Model",
        vec![prop_int(201), prop_string("Mixamo_Spine")],
        vec![],
    );
    let head_bone = node(
        "Model",
        vec![prop_int(202), prop_string("Mixamo_Head")],
        vec![],
    );
    let armor_mat = node(
        "Material",
        vec![prop_int(300), prop_string("KnightArmor")],
        vec![],
    );
    let cloth_mat = node(
        "Material",
        vec![prop_int(301), prop_string("KnightCape")],
        vec![],
    );
    let curve_node = node(
        "AnimationCurve",
        vec![prop_double_array(&[
            0.0, 10.0, 20.0, 30.0, 40.0, 95.0, 50.0, 60.0, 70.0, 80.0, 90.0,
        ])],
        vec![],
    );

    let fbx_bytes = fbx_file(
        &objects_root(vec![
            body_geom, hips_bone, spine_bone, head_bone, armor_mat, cloth_mat, curve_node,
        ]),
        FBX_VERSION_64,
    );

    let settings = FbxImportSettings {
        animation_type: FbxAnimationType::Humanoid,
        animation_compression: FbxAnimationCompression::Optimal,
        rotation_error: 0.5,
        position_error: 0.5,
        scale_error: 0.5,
        material_location: FbxMaterialLocation::External,
        extract_materials: true,
        clip_definitions: vec![
            FbxClipDefinition {
                name: "Idle".to_string(),
                first_frame: 0.0,
                last_frame: 4.0,
                loop_time: true,
                loop_pose: true,
                bake_root_rotation: true,
                bake_root_position_y: true,
                bake_root_position_xz: false,
            },
            FbxClipDefinition {
                name: "Run".to_string(),
                first_frame: 4.0,
                last_frame: 10.0,
                loop_time: true,
                loop_pose: false,
                bake_root_rotation: true,
                bake_root_position_y: true,
                bake_root_position_xz: true,
            },
        ],
    };

    let bundle = import_fbx_with_settings(&fbx_bytes, "Assets/Models/Knight.fbx", &settings)?;
    assert_eq!(bundle.meshes.len(), 1);
    assert_eq!(bundle.meshes[0].name, "TriQuad");

    let avatar = bundle
        .avatar
        .as_ref()
        .ok_or("PRJ-004 Outcome 15 expected Humanoid avatar")?;
    assert!(
        avatar.is_humanoid,
        "PRJ-004 Outcome 15 avatar must be Humanoid"
    );
    assert_eq!(avatar.name, "KnightAvatar");
    assert_eq!(avatar.sub_asset_id, "KnightAvatar");
    assert_eq!(avatar.root_bone, "Mixamo_Hips");
    assert_eq!(avatar.human_bones.len(), 3);
    assert_eq!(avatar.human_bones[0].human_name, "Hips");
    assert_eq!(avatar.human_bones[1].human_name, "Spine");
    assert_eq!(avatar.human_bones[2].human_name, "Head");

    assert_eq!(
        bundle.clips.len(),
        2,
        "PRJ-004 Outcome 15 split clips Idle and Run produced"
    );
    let idle = &bundle.clips[0];
    assert_eq!(idle.name, "Idle");
    assert_eq!(idle.sub_asset_id, "Idle");
    assert_eq!((idle.first_frame, idle.last_frame), (0.0, 4.0));
    assert!(idle.loop_time);
    assert!(idle.loop_pose, "PRJ-004 Outcome 15 loop pose honored");
    assert!(
        idle.baked_root_rotation,
        "PRJ-004 Outcome 15 baked root rotation honored"
    );
    assert_eq!(idle.compression, FbxAnimationCompression::Optimal);
    let idle_first = idle
        .root_rotation_keyframes
        .first()
        .ok_or("missing first keyframe")?
        .value;
    let idle_last = idle
        .root_rotation_keyframes
        .last()
        .ok_or("missing last keyframe")?
        .value;
    assert!(
        (idle_first - idle_last).abs() < 1e-5,
        "PRJ-004 Outcome 15 loop_pose aligns end pose with start pose"
    );

    let run = &bundle.clips[1];
    assert_eq!(run.name, "Run");
    assert_eq!(run.sub_asset_id, "Run");
    assert_eq!((run.first_frame, run.last_frame), (4.0, 10.0));
    assert!(run.baked_root_rotation);
    assert_eq!(run.compression, FbxAnimationCompression::Optimal);
    assert!(
        run.root_rotation_keyframes.len() < 7,
        "PRJ-004 Outcome 15 Optimal compression reduces linear keyframes while keeping inflection points"
    );
    assert!(
        run.root_rotation_keyframes
            .iter()
            .any(|kf| (kf.value - 95.0).abs() < 1e-4),
        "PRJ-004 Outcome 15 Optimal compression retains non-linear peak keyframe"
    );

    assert_eq!(
        bundle.materials.len(),
        2,
        "PRJ-004 Outcome 15 extracted materials produced"
    );
    assert_eq!(bundle.materials[0].name, "KnightArmor");
    assert_eq!(
        bundle.materials[0].extracted_path.as_deref(),
        Some("Assets/Models/KnightArmor.mat")
    );
    assert_eq!(bundle.materials[1].name, "KnightCape");
    assert_eq!(
        bundle.materials[1].extracted_path.as_deref(),
        Some("Assets/Models/KnightCape.mat")
    );

    let reimported = import_fbx_with_settings(&fbx_bytes, "Assets/Models/Knight.fbx", &settings)?;
    assert_eq!(reimported.avatar, bundle.avatar);
    assert_eq!(reimported.clips, bundle.clips);
    assert_eq!(reimported.materials, bundle.materials);
    Ok(())
}
