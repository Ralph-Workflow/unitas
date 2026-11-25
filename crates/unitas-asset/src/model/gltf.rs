use crate::json::{self, JsonValue};
use crate::primitive::{MeshData, SubMesh};

use super::ModelImportError;

const GLB_MAGIC: &[u8] = b"glTF";
const GLB_JSON_CHUNK: u32 = 0x4E4F_534A;
const GLB_BIN_CHUNK: u32 = 0x004E_4942;

const COMPONENT_FLOAT: u32 = 5126;
const COMPONENT_UNSIGNED_BYTE: u32 = 5121;
const COMPONENT_UNSIGNED_SHORT: u32 = 5123;
const COMPONENT_UNSIGNED_INT: u32 = 5125;

pub fn import_gltf(bytes: &[u8]) -> Result<MeshData, ModelImportError> {
    if bytes.starts_with(GLB_MAGIC) {
        import_glb(bytes)
    } else {
        let text = std::str::from_utf8(bytes).map_err(|_| ModelImportError::MalformedGltf {
            reason: "source is not valid UTF-8".to_string(),
        })?;
        let document = json::parse(text).map_err(|error| ModelImportError::MalformedGltf {
            reason: format!("invalid glTF JSON: {error}"),
        })?;
        build_mesh(&document, None)
    }
}

fn import_glb(bytes: &[u8]) -> Result<MeshData, ModelImportError> {
    if bytes.len() < 12 {
        return Err(ModelImportError::MalformedGlb {
            reason: "the header is shorter than 12 bytes".to_string(),
        });
    }
    let version = read_u32_at(bytes, 4)?;
    if version != 2 {
        return Err(ModelImportError::MalformedGlb {
            reason: format!("version {version} is not supported"),
        });
    }
    let mut offset = 12usize;
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk: Option<&[u8]> = None;
    while offset + 8 <= bytes.len() {
        let length = read_u32_at(bytes, offset)? as usize;
        let kind = read_u32_at(bytes, offset + 4)?;
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .ok_or_else(|| ModelImportError::MalformedGlb {
                reason: "a chunk length overflows".to_string(),
            })?;
        if end > bytes.len() {
            return Err(ModelImportError::MalformedGlb {
                reason: "a chunk extends past the end of the file".to_string(),
            });
        }
        if kind == GLB_JSON_CHUNK {
            if json_chunk.is_none() {
                json_chunk = Some(&bytes[start..end]);
            }
        } else if kind == GLB_BIN_CHUNK && bin_chunk.is_none() {
            bin_chunk = Some(&bytes[start..end]);
        }
        offset = end;
    }
    let Some(json_chunk) = json_chunk else {
        return Err(ModelImportError::MalformedGlb {
            reason: "the file has no JSON chunk".to_string(),
        });
    };
    let document = json::parse_bytes(trim_padding(json_chunk)).map_err(|error| {
        ModelImportError::MalformedGltf {
            reason: format!("invalid glTF JSON: {error}"),
        }
    })?;
    build_mesh(&document, bin_chunk)
}

fn build_mesh(document: &JsonValue, glb_bin: Option<&[u8]>) -> Result<MeshData, ModelImportError> {
    let buffers = read_buffers(document, glb_bin)?;
    let buffer_views = read_buffer_views(document)?;
    let accessors = read_accessors(document)?;
    let node_names = read_node_names(document);

    let meshes = array(document, "meshes");
    if meshes.is_empty() {
        return Err(ModelImportError::MalformedGltf {
            reason: "the document declares no meshes".to_string(),
        });
    }

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut sub_meshes: Vec<SubMesh> = Vec::new();
    let mut mesh_name = String::new();

    for (mesh_index, mesh_json) in meshes.iter().enumerate() {
        let label = mesh_label(mesh_index, mesh_json, &node_names);
        if mesh_name.is_empty() {
            mesh_name = label.clone();
        }
        for (primitive_index, primitive_json) in array(mesh_json, "primitives").iter().enumerate() {
            let attributes = primitive_json.get("attributes");
            let position_accessor = match attributes
                .and_then(|attributes| attributes.get("POSITION"))
                .and_then(|value| value.as_i64())
            {
                Some(index) => index as usize,
                None => {
                    return Err(ModelImportError::MissingAttribute {
                        mesh: mesh_index,
                        primitive: primitive_index,
                        attribute: "POSITION".to_string(),
                    })
                }
            };
            let normal_accessor = attributes
                .and_then(|attributes| attributes.get("NORMAL"))
                .and_then(|value| value.as_i64())
                .map(|index| index as usize);
            let uv_accessor = attributes
                .and_then(|attributes| attributes.get("TEXCOORD_0"))
                .and_then(|value| value.as_i64())
                .map(|index| index as usize);

            let local_positions =
                read_vec3(&buffers, &buffer_views, &accessors, position_accessor)?;
            let local_normals = match normal_accessor {
                Some(index) => Some(read_vec3(&buffers, &buffer_views, &accessors, index)?),
                None => None,
            };
            let local_uvs = match uv_accessor {
                Some(index) => Some(read_vec2(&buffers, &buffer_views, &accessors, index)?),
                None => None,
            };
            if let Some(local_normals) = &local_normals {
                if local_normals.len() != local_positions.len() {
                    return Err(ModelImportError::MalformedGltf {
                        reason: format!(
                            "mesh {mesh_index} primitive {primitive_index} has {} normals for {} positions",
                            local_normals.len(),
                            local_positions.len()
                        ),
                    });
                }
            }
            if let Some(local_uvs) = &local_uvs {
                if local_uvs.len() != local_positions.len() {
                    return Err(ModelImportError::MalformedGltf {
                        reason: format!(
                            "mesh {mesh_index} primitive {primitive_index} has {} texture coordinates for {} positions",
                            local_uvs.len(),
                            local_positions.len()
                        ),
                    });
                }
            }

            let local_indices = match primitive_json
                .get("indices")
                .and_then(|value| value.as_i64())
            {
                Some(index) => read_indices(&buffers, &buffer_views, &accessors, index as usize)?,
                None => (0..local_positions.len() as u32).collect(),
            };
            for index in &local_indices {
                if *index as usize >= local_positions.len() {
                    return Err(ModelImportError::MalformedGltf {
                        reason: format!(
                            "mesh {mesh_index} primitive {primitive_index} indexes vertex {index} of {}",
                            local_positions.len()
                        ),
                    });
                }
            }

            let vertex_base = positions.len() as u32;
            let index_base = indices.len() as u32;
            let vertex_count = local_positions.len();
            positions.extend(local_positions);
            normals.extend(local_normals.unwrap_or_else(|| vec![[0.0; 3]; vertex_count]));
            uvs.extend(local_uvs.unwrap_or_else(|| vec![[0.0; 2]; vertex_count]));
            for index in local_indices {
                indices.push(index + vertex_base);
            }
            let sub_asset_id = format!("{label}#{primitive_index}");
            sub_meshes.push(SubMesh {
                name: sub_asset_id.clone(),
                index_offset: index_base,
                index_count: indices.len() as u32 - index_base,
                sub_asset_id,
            });
        }
    }

    Ok(MeshData {
        name: mesh_name,
        positions,
        normals,
        uvs,
        indices,
        sub_meshes,
    })
}

fn read_buffers(
    document: &JsonValue,
    glb_bin: Option<&[u8]>,
) -> Result<Vec<Vec<u8>>, ModelImportError> {
    let entries = array(document, "buffers");
    let mut buffers = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let byte_length = entry
            .get("byteLength")
            .and_then(|value| value.as_i64())
            .map_or(0usize, |value| value.max(0) as usize);
        match entry.get("uri").and_then(|value| value.as_str()) {
            Some(uri) => buffers.push(decode_buffer_uri(uri, byte_length)?),
            None => {
                let is_first = index == 0;
                let Some(bin) = glb_bin.filter(|_| is_first) else {
                    return Err(ModelImportError::MissingBuffer { buffer: index });
                };
                buffers.push(bin.to_vec());
            }
        }
    }
    Ok(buffers)
}

fn decode_buffer_uri(uri: &str, byte_length: usize) -> Result<Vec<u8>, ModelImportError> {
    let Some(rest) = uri.strip_prefix("data:") else {
        return Err(ModelImportError::UnsupportedBufferUri {
            uri: uri.to_string(),
        });
    };
    let Some(separator) = rest.find(',') else {
        return Err(ModelImportError::InvalidBase64 {
            reason: "the data URI has no payload".to_string(),
        });
    };
    let (meta, payload) = rest.split_at(separator);
    if !meta.ends_with(";base64") {
        return Err(ModelImportError::UnsupportedBufferUri {
            uri: uri.to_string(),
        });
    }
    let bytes = base64_decode(&payload[1..])?;
    if bytes.len() < byte_length {
        return Err(ModelImportError::MalformedGltf {
            reason: format!(
                "the data URI holds {} bytes but the buffer declares {byte_length}",
                bytes.len()
            ),
        });
    }
    Ok(bytes)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ModelImportError> {
    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;
    let mut padding = false;
    for character in input.chars() {
        if character == '=' {
            padding = true;
            continue;
        }
        if character.is_ascii_whitespace() {
            continue;
        }
        if padding {
            return Err(ModelImportError::InvalidBase64 {
                reason: "the payload continues after the padding".to_string(),
            });
        }
        let value = base64_value(character).ok_or_else(|| ModelImportError::InvalidBase64 {
            reason: format!("`{character}` is not a base64 character"),
        })?;
        accumulator = ((accumulator << 6) | value) & 0x0FFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xFF) as u8);
        }
    }
    Ok(output)
}

fn base64_value(character: char) -> Option<u32> {
    match character {
        'A'..='Z' => Some(character as u32 - 'A' as u32),
        'a'..='z' => Some(character as u32 - 'a' as u32 + 26),
        '0'..='9' => Some(character as u32 - '0' as u32 + 52),
        '+' => Some(62),
        '/' => Some(63),
        _ => None,
    }
}

fn read_buffer_views(document: &JsonValue) -> Result<Vec<BufferView>, ModelImportError> {
    let entries = array(document, "bufferViews");
    let mut views = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Some(buffer) = entry.get("buffer").and_then(|value| value.as_i64()) else {
            return Err(ModelImportError::MalformedGltf {
                reason: format!("buffer view {index} names no buffer"),
            });
        };
        let byte_offset = entry
            .get("byteOffset")
            .and_then(|value| value.as_i64())
            .map_or(0usize, |value| value.max(0) as usize);
        if entry
            .get("byteLength")
            .and_then(|value| value.as_i64())
            .is_none()
        {
            return Err(ModelImportError::MalformedGltf {
                reason: format!("buffer view {index} declares no byte length"),
            });
        }
        let byte_stride = entry
            .get("byteStride")
            .and_then(|value| value.as_i64())
            .map(|value| value.max(0) as usize);
        views.push(BufferView {
            buffer: buffer.max(0) as usize,
            byte_offset,
            byte_stride,
        });
    }
    Ok(views)
}

fn read_accessors(document: &JsonValue) -> Result<Vec<Accessor>, ModelImportError> {
    let entries = array(document, "accessors");
    let mut accessors = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let component_type = entry
            .get("componentType")
            .and_then(|value| value.as_i64())
            .map_or(0u32, |value| value as u32);
        let count = entry
            .get("count")
            .and_then(|value| value.as_i64())
            .map_or(0usize, |value| value.max(0) as usize);
        let kind = match entry.get("type").and_then(|value| value.as_str()) {
            Some("SCALAR") | None => AccessorKind::Scalar,
            Some("VEC2") => AccessorKind::Vec2,
            Some("VEC3") => AccessorKind::Vec3,
            Some("VEC4") => AccessorKind::Vec4,
            Some(other) => {
                return Err(ModelImportError::MalformedGltf {
                    reason: format!("accessor {index} has unknown type {other}"),
                })
            }
        };
        accessors.push(Accessor {
            buffer_view: entry
                .get("bufferView")
                .and_then(|value| value.as_i64())
                .map(|value| value.max(0) as usize),
            byte_offset: entry
                .get("byteOffset")
                .and_then(|value| value.as_i64())
                .map_or(0usize, |value| value.max(0) as usize),
            component_type,
            count,
            kind,
        });
    }
    Ok(accessors)
}

fn read_node_names(document: &JsonValue) -> Vec<NodeName> {
    let mut names: Vec<NodeName> = Vec::new();
    for node in array(document, "nodes") {
        let Some(mesh) = node.get("mesh").and_then(|value| value.as_i64()) else {
            continue;
        };
        let mesh = mesh.max(0) as usize;
        if names.len() <= mesh {
            names.resize(mesh + 1, NodeName::default());
        }
        if !names[mesh].seen {
            names[mesh].seen = true;
            names[mesh].name = node
                .get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
    }
    names
}

fn mesh_label(mesh_index: usize, mesh_json: &JsonValue, node_names: &[NodeName]) -> String {
    if let Some(node_name) = node_names.get(mesh_index) {
        if let Some(name) = &node_name.name {
            if !name.is_empty() {
                return name.clone();
            }
        }
    }
    if let Some(name) = mesh_json.get("name").and_then(|value| value.as_str()) {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    format!("mesh_{mesh_index}")
}

fn read_vec3(
    buffers: &[Vec<u8>],
    views: &[BufferView],
    accessors: &[Accessor],
    accessor_index: usize,
) -> Result<Vec<[f32; 3]>, ModelImportError> {
    let accessor = accessors
        .get(accessor_index)
        .ok_or(ModelImportError::MissingAccessor {
            accessor: accessor_index,
        })?;
    if accessor.component_type != COMPONENT_FLOAT || accessor.kind != AccessorKind::Vec3 {
        return Err(ModelImportError::UnsupportedAccessorType {
            accessor: accessor_index,
            component_type: accessor.component_type,
            expected: "FLOAT VEC3".to_string(),
        });
    }
    let (buffer, base, stride) = accessor_location(buffers, views, accessor, accessor_index)?;
    let mut values = Vec::with_capacity(accessor.count);
    for index in 0..accessor.count {
        let offset = base + index * stride;
        values.push([
            read_f32(buffer, offset)?,
            read_f32(buffer, offset + 4)?,
            read_f32(buffer, offset + 8)?,
        ]);
    }
    Ok(values)
}

fn read_vec2(
    buffers: &[Vec<u8>],
    views: &[BufferView],
    accessors: &[Accessor],
    accessor_index: usize,
) -> Result<Vec<[f32; 2]>, ModelImportError> {
    let accessor = accessors
        .get(accessor_index)
        .ok_or(ModelImportError::MissingAccessor {
            accessor: accessor_index,
        })?;
    if accessor.component_type != COMPONENT_FLOAT || accessor.kind != AccessorKind::Vec2 {
        return Err(ModelImportError::UnsupportedAccessorType {
            accessor: accessor_index,
            component_type: accessor.component_type,
            expected: "FLOAT VEC2".to_string(),
        });
    }
    let (buffer, base, stride) = accessor_location(buffers, views, accessor, accessor_index)?;
    let mut values = Vec::with_capacity(accessor.count);
    for index in 0..accessor.count {
        let offset = base + index * stride;
        values.push([read_f32(buffer, offset)?, read_f32(buffer, offset + 4)?]);
    }
    Ok(values)
}

fn read_indices(
    buffers: &[Vec<u8>],
    views: &[BufferView],
    accessors: &[Accessor],
    accessor_index: usize,
) -> Result<Vec<u32>, ModelImportError> {
    let accessor = accessors
        .get(accessor_index)
        .ok_or(ModelImportError::MissingAccessor {
            accessor: accessor_index,
        })?;
    if accessor.kind != AccessorKind::Scalar {
        return Err(ModelImportError::UnsupportedAccessorType {
            accessor: accessor_index,
            component_type: accessor.component_type,
            expected: "a SCALAR index".to_string(),
        });
    }
    match accessor.component_type {
        COMPONENT_UNSIGNED_BYTE | COMPONENT_UNSIGNED_SHORT | COMPONENT_UNSIGNED_INT => {}
        other => {
            return Err(ModelImportError::UnsupportedAccessorType {
                accessor: accessor_index,
                component_type: other,
                expected: "an UNSIGNED_BYTE, UNSIGNED_SHORT or UNSIGNED_INT index".to_string(),
            })
        }
    }
    let (buffer, base, stride) = accessor_location(buffers, views, accessor, accessor_index)?;
    let mut values = Vec::with_capacity(accessor.count);
    for index in 0..accessor.count {
        let offset = base + index * stride;
        let value = match accessor.component_type {
            COMPONENT_UNSIGNED_BYTE => read_u8(buffer, offset)? as u32,
            COMPONENT_UNSIGNED_SHORT => read_u16(buffer, offset)? as u32,
            _ => read_u32_buffer(buffer, offset)?,
        };
        values.push(value);
    }
    Ok(values)
}

fn accessor_location<'a>(
    buffers: &'a [Vec<u8>],
    views: &[BufferView],
    accessor: &Accessor,
    accessor_index: usize,
) -> Result<(&'a [u8], usize, usize), ModelImportError> {
    let Some(view_index) = accessor.buffer_view else {
        return Err(ModelImportError::UnsupportedAccessorType {
            accessor: accessor_index,
            component_type: accessor.component_type,
            expected: "a buffer view".to_string(),
        });
    };
    let view = views
        .get(view_index)
        .ok_or(ModelImportError::MissingBufferView {
            buffer_view: view_index,
        })?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or(ModelImportError::MissingBuffer {
            buffer: view.buffer,
        })?;
    let element = component_size(accessor.component_type) * accessor.kind.components();
    let stride = view.byte_stride.unwrap_or(element).max(element);
    let base = view.byte_offset + accessor.byte_offset;
    Ok((buffer.as_slice(), base, stride))
}

fn read_f32(buffer: &[u8], offset: usize) -> Result<f32, ModelImportError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: "a buffer offset overflows".to_string(),
        })?;
    let slice = buffer
        .get(offset..end)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: format!("a read of 4 bytes at {offset} is past the end of the buffer"),
        })?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(slice);
    Ok(f32::from_le_bytes(raw))
}

fn read_u8(buffer: &[u8], offset: usize) -> Result<u8, ModelImportError> {
    buffer
        .get(offset)
        .copied()
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: format!("a read of 1 byte at {offset} is past the end of the buffer"),
        })
}

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16, ModelImportError> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: "a buffer offset overflows".to_string(),
        })?;
    let slice = buffer
        .get(offset..end)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: format!("a read of 2 bytes at {offset} is past the end of the buffer"),
        })?;
    let mut raw = [0u8; 2];
    raw.copy_from_slice(slice);
    Ok(u16::from_le_bytes(raw))
}

fn read_u32_buffer(buffer: &[u8], offset: usize) -> Result<u32, ModelImportError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: "a buffer offset overflows".to_string(),
        })?;
    let slice = buffer
        .get(offset..end)
        .ok_or_else(|| ModelImportError::MalformedGltf {
            reason: format!("a read of 4 bytes at {offset} is past the end of the buffer"),
        })?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(slice);
    Ok(u32::from_le_bytes(raw))
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32, ModelImportError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| ModelImportError::MalformedGlb {
            reason: "an offset overflows".to_string(),
        })?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| ModelImportError::MalformedGlb {
            reason: format!("a read of 4 bytes at {offset} is past the end of the file"),
        })?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(slice);
    Ok(u32::from_le_bytes(raw))
}

fn trim_padding(chunk: &[u8]) -> &[u8] {
    match chunk.iter().rposition(|byte| *byte != b' ' && *byte != 0) {
        Some(index) => &chunk[..=index],
        None => &chunk[..0],
    }
}

fn array<'a>(value: &'a JsonValue, key: &str) -> &'a [JsonValue] {
    match value.get(key).and_then(|item| item.as_array()) {
        Some(items) => items,
        None => &[],
    }
}

fn component_size(component_type: u32) -> usize {
    match component_type {
        5120 | COMPONENT_UNSIGNED_BYTE => 1,
        5122 | COMPONENT_UNSIGNED_SHORT => 2,
        COMPONENT_UNSIGNED_INT | COMPONENT_FLOAT => 4,
        _ => 4,
    }
}

#[derive(Clone, Default)]
struct NodeName {
    seen: bool,
    name: Option<String>,
}

struct BufferView {
    buffer: usize,
    byte_offset: usize,
    byte_stride: Option<usize>,
}

#[derive(PartialEq)]
enum AccessorKind {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
}

impl AccessorKind {
    fn components(&self) -> usize {
        match self {
            AccessorKind::Scalar => 1,
            AccessorKind::Vec2 => 2,
            AccessorKind::Vec3 => 3,
            AccessorKind::Vec4 => 4,
        }
    }
}

struct Accessor {
    buffer_view: Option<usize>,
    byte_offset: usize,
    component_type: u32,
    count: usize,
    kind: AccessorKind,
}
