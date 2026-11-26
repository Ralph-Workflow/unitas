use unitas_asset::model::{import_gltf, import_obj};
use unitas_asset::primitive::{primitive_mesh, BuiltinPrimitive, MeshData};
use unitas_asset::texture::artifact::texture_artifact;
use unitas_asset::texture::import::{TextureImportSettings, TextureImporterType};

type TestResult = Result<(), String>;

const QUAD_OBJ: &str = "\
v -1.0 -1.0 0.0
v -1.0 1.0 0.0
v 1.0 1.0 0.0
v 1.0 -1.0 0.0
vt 0.0 0.0
vt 0.0 1.0
vt 1.0 1.0
vt 1.0 0.0
vn 0.0 0.0 1.0
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
";

const QUAD_POSITIONS: [f32; 12] = [
    -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0,
];

const QUAD_UVS: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];

const QUAD_NORMALS: [f32; 12] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];

const QUAD_TANGENTS: [f32; 16] = [
    1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
];

fn assert_unit_tangents_perpendicular_to_normals(mesh: &MeshData) {
    assert_eq!(
        mesh.tangents.len(),
        mesh.positions.len(),
        "every vertex carries a tangent"
    );
    for (index, tangent) in mesh.tangents.iter().enumerate() {
        let normal = mesh.normals[index];
        let length =
            (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
        assert!(
            (length - 1.0).abs() < 1e-4,
            "tangent {index} is unit length, got {length}"
        );
        let alignment = tangent[0] * normal[0] + tangent[1] * normal[1] + tangent[2] * normal[2];
        assert!(
            alignment.abs() < 1e-4,
            "tangent {index} is perpendicular to its normal, got {alignment}"
        );
        assert!(
            tangent[3].abs() == 1.0,
            "tangent {index} carries a handedness sign, got {}",
            tangent[3]
        );
    }
}

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

fn push_f32(buffer: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}

fn quad_bin() -> Vec<u8> {
    let mut bin = Vec::new();
    push_f32(&mut bin, &QUAD_POSITIONS);
    push_f32(&mut bin, &QUAD_UVS);
    push_f32(&mut bin, &QUAD_NORMALS);
    for index in [0u16, 1, 2, 0, 2, 3] {
        bin.extend_from_slice(&index.to_le_bytes());
    }
    bin
}

fn quad_gltf(with_tangents: bool) -> String {
    if with_tangents {
        return String::from(
            "{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":204}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":48},
{\"buffer\":0,\"byteOffset\":48,\"byteLength\":32},
{\"buffer\":0,\"byteOffset\":80,\"byteLength\":48},
{\"buffer\":0,\"byteOffset\":128,\"byteLength\":12},
{\"buffer\":0,\"byteOffset\":140,\"byteLength\":64}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":4,\"type\":\"VEC3\"},
{\"bufferView\":1,\"componentType\":5126,\"count\":4,\"type\":\"VEC2\"},
{\"bufferView\":2,\"componentType\":5126,\"count\":4,\"type\":\"VEC3\"},
{\"bufferView\":3,\"componentType\":5123,\"count\":6,\"type\":\"SCALAR\"},
{\"bufferView\":4,\"componentType\":5126,\"count\":4,\"type\":\"VEC4\"}],
\"meshes\":[{\"name\":\"Quad\",\"primitives\":[{\"attributes\":{\"POSITION\":0,\"TEXCOORD_0\":1,\"NORMAL\":2,\"TANGENT\":4},\"indices\":3}]}],
\"nodes\":[{\"name\":\"QuadNode\",\"mesh\":0}]}",
        );
    }
    String::from(
        "{\"asset\":{\"version\":\"2.0\"},
\"buffers\":[{\"byteLength\":140}],
\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":48},
{\"buffer\":0,\"byteOffset\":48,\"byteLength\":32},
{\"buffer\":0,\"byteOffset\":80,\"byteLength\":48},
{\"buffer\":0,\"byteOffset\":128,\"byteLength\":12}],
\"accessors\":[{\"bufferView\":0,\"componentType\":5126,\"count\":4,\"type\":\"VEC3\"},
{\"bufferView\":1,\"componentType\":5126,\"count\":4,\"type\":\"VEC2\"},
{\"bufferView\":2,\"componentType\":5126,\"count\":4,\"type\":\"VEC3\"},
{\"bufferView\":3,\"componentType\":5123,\"count\":6,\"type\":\"SCALAR\"}],
\"meshes\":[{\"name\":\"Quad\",\"primitives\":[{\"attributes\":{\"POSITION\":0,\"TEXCOORD_0\":1,\"NORMAL\":2},\"indices\":3}]}],
\"nodes\":[{\"name\":\"QuadNode\",\"mesh\":0}]}",
    )
}

fn tga_bytes(width: u16, height: u16, pixels: &[[u8; 3]]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0);
    out.push(0);
    out.push(2);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.push(0);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.push(24);
    out.push(0);
    for pixel in pixels {
        out.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    out
}

#[test]
fn builtin_quad_carries_unit_tangents_perpendicular_to_its_normals() {
    let mesh = primitive_mesh(BuiltinPrimitive::Quad);
    assert_unit_tangents_perpendicular_to_normals(&mesh);
}

#[test]
fn imported_obj_quad_carries_unit_tangents_perpendicular_to_its_normals() -> TestResult {
    let mesh = match import_obj(QUAD_OBJ) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_unit_tangents_perpendicular_to_normals(&mesh);
    Ok(())
}

#[test]
fn gltf_without_tangents_computes_them_from_positions_and_uvs() -> TestResult {
    let bin = quad_bin();
    let mesh = match import_gltf(&make_glb(&quad_gltf(false), &bin)) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_unit_tangents_perpendicular_to_normals(&mesh);
    for tangent in &mesh.tangents {
        assert!(
            (tangent[0] - 1.0).abs() < 1e-4,
            "the computed tangent follows the u axis, got {tangent:?}"
        );
    }
    Ok(())
}

#[test]
fn gltf_uses_authored_tangents_when_the_source_provides_them() -> TestResult {
    let mut bin = quad_bin();
    push_f32(&mut bin, &QUAD_TANGENTS);
    let mesh = match import_gltf(&make_glb(&quad_gltf(true), &bin)) {
        Ok(mesh) => mesh,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_unit_tangents_perpendicular_to_normals(&mesh);
    for tangent in &mesh.tangents {
        assert!(
            (tangent[0] - 1.0).abs() < 1e-4,
            "the authored tangent direction is kept, got {tangent:?}"
        );
    }
    Ok(())
}

#[test]
fn texture_artifact_exposes_pixels_and_import_flags() -> TestResult {
    let bytes = tga_bytes(2, 1, &[[255, 0, 0], [0, 255, 0]]);
    let settings = TextureImportSettings::default();
    let artifact = match texture_artifact(&bytes, "checker.tga", &settings) {
        Ok(artifact) => artifact,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert_eq!(artifact.width, 2);
    assert_eq!(artifact.height, 1);
    assert_eq!(artifact.pixels.len(), 6);
    assert_eq!(&artifact.pixels[0..3], &[255, 0, 0]);
    assert!(artifact.srgb, "the default import samples in sRGB");
    assert!(
        !artifact.normal_map,
        "the default import is not a normal map"
    );
    assert!(
        artifact.generate_mips,
        "the default import builds a mip chain"
    );
    assert_eq!(artifact.settings.max_size, settings.max_size);
    Ok(())
}

#[test]
fn texture_artifact_reports_normal_map_and_linear_flags() -> TestResult {
    let bytes = tga_bytes(2, 1, &[[255, 0, 0], [0, 255, 0]]);
    let settings = TextureImportSettings {
        texture_type: TextureImporterType::NormalMap,
        srgb: false,
        mipmaps: false,
        ..TextureImportSettings::default()
    };
    let artifact = match texture_artifact(&bytes, "checker.tga", &settings) {
        Ok(artifact) => artifact,
        Err(error) => return Err(format!("import failed: {error:?}")),
    };
    assert!(!artifact.srgb, "a linear import reports no sRGB flag");
    assert!(artifact.normal_map, "a normal map import reports the flag");
    assert!(!artifact.generate_mips, "mip generation is off");
    assert_eq!(
        artifact.settings.texture_type,
        TextureImporterType::NormalMap
    );
    Ok(())
}
