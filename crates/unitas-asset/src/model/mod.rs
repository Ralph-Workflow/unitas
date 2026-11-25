use std::fmt;

use crate::primitive::{MeshData, SubMesh};

pub mod fbx;
pub mod gltf;
pub mod obj;

pub use fbx::{import_fbx, import_fbx_bytes, FbxImportError};
pub use gltf::import_gltf;
pub use obj::{import_obj, import_obj_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelImportError {
    MalformedObj {
        line: usize,
        reason: String,
    },
    MalformedGltf {
        reason: String,
    },
    MalformedGlb {
        reason: String,
    },
    Fbx(FbxImportError),
    MissingAccessor {
        accessor: usize,
    },
    UnsupportedAccessorType {
        accessor: usize,
        component_type: u32,
        expected: String,
    },
    MissingBuffer {
        buffer: usize,
    },
    MissingBufferView {
        buffer_view: usize,
    },
    MissingMesh {
        mesh: usize,
    },
    MissingNode {
        node: usize,
    },
    MissingAttribute {
        mesh: usize,
        primitive: usize,
        attribute: String,
    },
    UnsupportedBufferUri {
        uri: String,
    },
    InvalidBase64 {
        reason: String,
    },
    UnsupportedFileType {
        file_name: String,
    },
    UnsupportedFormat {
        file_name: String,
        format: String,
    },
}

impl From<FbxImportError> for ModelImportError {
    fn from(error: FbxImportError) -> Self {
        ModelImportError::Fbx(error)
    }
}

impl fmt::Display for ModelImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelImportError::MalformedObj { line, reason } => {
                write!(formatter, "malformed OBJ at line {line}: {reason}")
            }
            ModelImportError::MalformedGltf { reason } => {
                write!(formatter, "malformed glTF: {reason}")
            }
            ModelImportError::MalformedGlb { reason } => {
                write!(formatter, "malformed GLB: {reason}")
            }
            ModelImportError::Fbx(error) => write!(formatter, "{error}"),
            ModelImportError::MissingAccessor { accessor } => {
                write!(formatter, "glTF accessor {accessor} does not exist")
            }
            ModelImportError::UnsupportedAccessorType {
                accessor,
                component_type,
                expected,
            } => write!(
                formatter,
                "glTF accessor {accessor} has component type {component_type}, expected {expected}"
            ),
            ModelImportError::MissingBuffer { buffer } => {
                write!(formatter, "glTF buffer {buffer} does not exist")
            }
            ModelImportError::MissingBufferView { buffer_view } => {
                write!(formatter, "glTF buffer view {buffer_view} does not exist")
            }
            ModelImportError::MissingMesh { mesh } => {
                write!(formatter, "glTF mesh {mesh} does not exist")
            }
            ModelImportError::MissingNode { node } => {
                write!(formatter, "glTF node {node} does not exist")
            }
            ModelImportError::MissingAttribute {
                mesh,
                primitive,
                attribute,
            } => write!(
                formatter,
                "glTF mesh {mesh} primitive {primitive} has no {attribute} attribute"
            ),
            ModelImportError::UnsupportedBufferUri { uri } => {
                write!(formatter, "unsupported glTF buffer URI: {uri}")
            }
            ModelImportError::InvalidBase64 { reason } => {
                write!(formatter, "invalid base64 payload: {reason}")
            }
            ModelImportError::UnsupportedFileType { file_name } => {
                write!(formatter, "unsupported model file type: {file_name}")
            }
            ModelImportError::UnsupportedFormat { file_name, format } => {
                write!(
                    formatter,
                    "{file_name}: {format} format is Unsupported (content-creation application files are not converted)"
                )
            }
        }
    }
}

impl std::error::Error for ModelImportError {}

pub fn import_model(bytes: &[u8], file_name: &str) -> Result<MeshData, ModelImportError> {
    let meshes = import_model_sub_assets(bytes, file_name)?;
    Ok(combine_meshes(meshes, file_name))
}

pub fn import_model_sub_assets(
    bytes: &[u8],
    file_name: &str,
) -> Result<Vec<MeshData>, ModelImportError> {
    if is_blender_source(bytes, file_name) {
        return Err(ModelImportError::UnsupportedFormat {
            file_name: file_name.to_string(),
            format: "blend".to_string(),
        });
    }
    let extension = match file_name.rsplit('.').next() {
        Some(extension) => extension.to_ascii_lowercase(),
        None => String::new(),
    };
    match extension.as_str() {
        "obj" => {
            let mut mesh = obj::import_obj_bytes(bytes)?;
            mesh.name = file_stem(file_name);
            Ok(vec![mesh])
        }
        "gltf" | "glb" => Ok(vec![gltf::import_gltf(bytes)?]),
        "fbx" => Ok(fbx::import_fbx(bytes)?),
        "max" | "ma" | "mb" | "c4d" | "lxo" | "3ds" => Err(ModelImportError::UnsupportedFormat {
            file_name: file_name.to_string(),
            format: extension,
        }),
        _ => Err(ModelImportError::UnsupportedFileType {
            file_name: extension,
        }),
    }
}

fn is_blender_source(bytes: &[u8], file_name: &str) -> bool {
    let has_blend_ext = file_name
        .rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("blend"));
    has_blend_ext || bytes.starts_with(b"BLENDER")
}

fn combine_meshes(mut meshes: Vec<MeshData>, file_name: &str) -> MeshData {
    if meshes.len() == 1 {
        if let Some(mut single) = meshes.pop() {
            if single.name.is_empty() {
                single.name = file_stem(file_name);
            }
            return single;
        }
    }
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut sub_meshes: Vec<SubMesh> = Vec::new();
    let mut mesh_name = String::new();
    let any_normals = meshes.iter().any(|mesh| !mesh.normals.is_empty());
    let any_uvs = meshes.iter().any(|mesh| !mesh.uvs.is_empty());

    for mesh in meshes {
        if mesh_name.is_empty() && !mesh.name.is_empty() {
            mesh_name = mesh.name.clone();
        }
        let vertex_base = positions.len() as u32;
        let index_base = indices.len() as u32;
        let vertex_count = mesh.positions.len();
        positions.extend(mesh.positions);
        if any_normals {
            if mesh.normals.is_empty() {
                normals.extend(vec![[0.0; 3]; vertex_count]);
            } else {
                normals.extend(mesh.normals);
            }
        }
        if any_uvs {
            if mesh.uvs.is_empty() {
                uvs.extend(vec![[0.0; 2]; vertex_count]);
            } else {
                uvs.extend(mesh.uvs);
            }
        }
        for index in mesh.indices {
            indices.push(index + vertex_base);
        }
        if mesh.sub_meshes.is_empty() {
            let label = if mesh.name.is_empty() {
                file_stem(file_name)
            } else {
                mesh.name
            };
            sub_meshes.push(SubMesh {
                name: label.clone(),
                index_offset: index_base,
                index_count: indices.len() as u32 - index_base,
                sub_asset_id: label,
            });
        } else {
            for sub in mesh.sub_meshes {
                sub_meshes.push(SubMesh {
                    name: sub.name,
                    index_offset: sub.index_offset + index_base,
                    index_count: sub.index_count,
                    sub_asset_id: sub.sub_asset_id,
                });
            }
        }
    }

    if mesh_name.is_empty() {
        mesh_name = file_stem(file_name);
    }

    MeshData {
        name: mesh_name,
        positions,
        normals,
        uvs,
        indices,
        sub_meshes,
    }
}

fn file_stem(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(index) if index > 0 => file_name[..index].to_string(),
        _ => file_name.to_string(),
    }
}
