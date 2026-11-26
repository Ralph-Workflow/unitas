use std::fmt;

use crate::model::import_model_sub_assets;
use crate::primitive::{builtin_primitive_by_identity, primitive_mesh, BuiltinPrimitive, MeshData};

pub const BUILTIN_MESH_GUID: &str = crate::primitive::BUILTIN_EXTRA_GUID;
const SUB_ASSET_FILE_ID_BASE: i64 = 4300000;
const SUB_ASSET_FILE_ID_STRIDE: i64 = 2;
const BUILTIN_SPHERE_SCALE: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct CollisionMesh {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollisionCookError {
    Import { file_name: String, reason: String },
    NoMesh { file_name: String },
}

impl fmt::Display for CollisionCookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollisionCookError::Import { file_name, reason } => {
                write!(
                    formatter,
                    "cannot cook collision mesh from {file_name}: {reason}"
                )
            }
            CollisionCookError::NoMesh { file_name } => {
                write!(formatter, "{file_name} carries no mesh to cook")
            }
        }
    }
}

impl std::error::Error for CollisionCookError {}

pub fn cook_builtin_collision_mesh(file_id: i64, guid: &str) -> Option<CollisionMesh> {
    let primitive = builtin_primitive_by_identity(file_id, guid)?;
    let mut mesh = primitive_mesh(primitive);
    if primitive == BuiltinPrimitive::Sphere {
        for vertex in &mut mesh.positions {
            *vertex = [
                vertex[0] * BUILTIN_SPHERE_SCALE,
                vertex[1] * BUILTIN_SPHERE_SCALE,
                vertex[2] * BUILTIN_SPHERE_SCALE,
            ];
        }
    }
    Some(CollisionMesh {
        vertices: mesh.positions,
        indices: mesh.indices,
    })
}

pub fn cook_collision_mesh(
    bytes: &[u8],
    file_name: &str,
    file_id: Option<i64>,
) -> Result<CollisionMesh, CollisionCookError> {
    let meshes =
        import_model_sub_assets(bytes, file_name).map_err(|error| CollisionCookError::Import {
            file_name: file_name.to_string(),
            reason: error.to_string(),
        })?;
    let mesh = select_mesh(&meshes, file_id).ok_or_else(|| CollisionCookError::NoMesh {
        file_name: file_name.to_string(),
    })?;
    Ok(CollisionMesh {
        vertices: mesh.positions,
        indices: mesh.indices,
    })
}

fn select_mesh(meshes: &[MeshData], file_id: Option<i64>) -> Option<MeshData> {
    let index = file_id.and_then(sub_asset_index);
    let mesh = index
        .and_then(|index| meshes.get(index))
        .or_else(|| meshes.first())?;
    if mesh.positions.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    Some(mesh.clone())
}

fn sub_asset_index(file_id: i64) -> Option<usize> {
    let offset = file_id.checked_sub(SUB_ASSET_FILE_ID_BASE)?;
    if offset < 0 || offset % SUB_ASSET_FILE_ID_STRIDE != 0 {
        return None;
    }
    usize::try_from(offset / SUB_ASSET_FILE_ID_STRIDE).ok()
}
