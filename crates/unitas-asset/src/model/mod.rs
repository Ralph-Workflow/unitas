//! Model import for OBJ and glTF/GLB sources.
//!
//! Every importer produces `MeshData` whose sub-meshes carry a `sub_asset_id`
//! derived from the named group or primitive in the source, so reimporting the
//! same bytes keeps the same identities and malformed sources are reported as
//! named diagnostics instead of panics.

use std::fmt;

use crate::primitive::MeshData;

pub mod fbx;
pub mod gltf;
pub mod obj;

pub use gltf::import_gltf;
pub use obj::{import_obj, import_obj_bytes};

/// Named reasons why a model source could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelImportError {
    MalformedObj { line: usize, reason: String },
    MalformedGltf { reason: String },
    MalformedGlb { reason: String },
    MissingAccessor { accessor: usize },
    UnsupportedAccessorType {
        accessor: usize,
        component_type: u32,
        expected: String,
    },
    MissingBuffer { buffer: usize },
    MissingBufferView { buffer_view: usize },
    MissingMesh { mesh: usize },
    MissingNode { node: usize },
    MissingAttribute {
        mesh: usize,
        primitive: usize,
        attribute: String,
    },
    UnsupportedBufferUri { uri: String },
    InvalidBase64 { reason: String },
    UnsupportedFileType { file_name: String },
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
        }
    }
}

impl std::error::Error for ModelImportError {}

/// Imports a model source, choosing the reader from the file extension.
pub fn import_model(bytes: &[u8], file_name: &str) -> Result<MeshData, ModelImportError> {
    let extension = match file_name.rsplit('.').next() {
        Some(extension) => extension.to_ascii_lowercase(),
        None => String::new(),
    };
    match extension.as_str() {
        "obj" => {
            let mut mesh = obj::import_obj_bytes(bytes)?;
            mesh.name = file_stem(file_name);
            Ok(mesh)
        }
        "gltf" | "glb" => gltf::import_gltf(bytes),
        _ => Err(ModelImportError::UnsupportedFileType { file_name: extension }),
    }
}

fn file_stem(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(index) if index > 0 => file_name[..index].to_string(),
        _ => file_name.to_string(),
    }
}
