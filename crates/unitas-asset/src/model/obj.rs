//! OBJ import: static geometry with stable group/material sub-asset identities.

use std::collections::HashMap;

use crate::primitive::{MeshData, SubMesh};

use super::ModelImportError;

/// Vertex identity used for de-duplication: (position, uv, normal), `-1` marks an absent attribute.
type VertexKey = (i64, i64, i64);

/// Imports an OBJ source held as text.
pub fn import_obj(text: &str) -> Result<MeshData, ModelImportError> {
    ObjParser::default().parse(text)
}

/// Imports an OBJ source held as bytes.
pub fn import_obj_bytes(bytes: &[u8]) -> Result<MeshData, ModelImportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ModelImportError::MalformedObj {
        line: 0,
        reason: "source is not valid UTF-8".to_string(),
    })?;
    import_obj(text)
}

struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

struct OpenGroup {
    name: String,
    sub_asset_id: String,
    index_offset: u32,
}

#[derive(Default)]
struct ObjParser {
    source_positions: Vec<[f32; 3]>,
    source_normals: Vec<[f32; 3]>,
    source_uvs: Vec<[f32; 2]>,
    vertices: Vec<Vertex>,
    vertex_ids: HashMap<VertexKey, u32>,
    indices: Vec<u32>,
    sub_meshes: Vec<SubMesh>,
    group: Option<String>,
    material: Option<String>,
    open: Option<OpenGroup>,
}

impl ObjParser {
    fn parse(mut self, text: &str) -> Result<MeshData, ModelImportError> {
        for (number, raw) in text.lines().enumerate() {
            let line = number + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut tokens = trimmed.split_whitespace();
            let Some(directive) = tokens.next() else {
                continue;
            };
            let values: Vec<&str> = tokens.collect();
            match directive {
                "v" => self.push_position(&values, line)?,
                "vn" => self.push_normal(&values, line)?,
                "vt" => self.push_uv(&values, line)?,
                "f" => self.push_face(&values, line)?,
                "g" | "o" => self.group = single_name(&values),
                "usemtl" => self.material = single_name(&values),
                // Material library names have no slot in MeshData, so the directive is accepted and skipped.
                "mtllib" => {}
                _ => {}
            }
        }
        self.close_group();
        let positions = self.vertices.iter().map(|vertex| vertex.position).collect();
        let normals = self.vertices.iter().map(|vertex| vertex.normal).collect();
        let uvs = self.vertices.iter().map(|vertex| vertex.uv).collect();
        Ok(MeshData {
            name: "obj".to_string(),
            positions,
            normals,
            uvs,
            indices: self.indices,
            sub_meshes: self.sub_meshes,
        })
    }

    fn push_position(&mut self, values: &[&str], line: usize) -> Result<(), ModelImportError> {
        if values.len() < 3 {
            return Err(malformed(line, "vertex needs three coordinates"));
        }
        self.source_positions.push([
            parse_float(values[0], line)?,
            parse_float(values[1], line)?,
            parse_float(values[2], line)?,
        ]);
        Ok(())
    }

    fn push_normal(&mut self, values: &[&str], line: usize) -> Result<(), ModelImportError> {
        if values.len() < 3 {
            return Err(malformed(line, "normal needs three coordinates"));
        }
        self.source_normals.push([
            parse_float(values[0], line)?,
            parse_float(values[1], line)?,
            parse_float(values[2], line)?,
        ]);
        Ok(())
    }

    fn push_uv(&mut self, values: &[&str], line: usize) -> Result<(), ModelImportError> {
        if values.is_empty() {
            return Err(malformed(line, "texture coordinate needs a value"));
        }
        let u = parse_float(values[0], line)?;
        let v = match values.get(1) {
            Some(value) => parse_float(value, line)?,
            None => 0.0,
        };
        self.source_uvs.push([u, v]);
        Ok(())
    }

    fn push_face(&mut self, values: &[&str], line: usize) -> Result<(), ModelImportError> {
        if values.len() < 3 {
            return Err(malformed(line, "face needs at least three vertices"));
        }
        let mut corners = Vec::with_capacity(values.len());
        for token in values {
            corners.push(self.parse_corner(token, line)?);
        }
        self.open_group()?;
        for index in 1..corners.len() - 1 {
            let first = self.vertex_id(corners[0])?;
            let second = self.vertex_id(corners[index])?;
            let third = self.vertex_id(corners[index + 1])?;
            self.indices.push(first);
            self.indices.push(second);
            self.indices.push(third);
        }
        Ok(())
    }

    fn parse_corner(&self, token: &str, line: usize) -> Result<VertexKey, ModelImportError> {
        let parts: Vec<&str> = token.split('/').collect();
        if parts.is_empty() || parts.len() > 3 {
            return Err(malformed(line, format!("bad face token `{token}`")));
        }
        let position = parse_index(parts[0], line, self.source_positions.len(), "vertex")?;
        let uv = match parts.get(1) {
            None | Some(&"") => -1,
            Some(value) => parse_index(value, line, self.source_uvs.len(), "texture")?,
        };
        let normal = match parts.get(2) {
            None | Some(&"") => -1,
            Some(value) => parse_index(value, line, self.source_normals.len(), "normal")?,
        };
        Ok((position, uv, normal))
    }

    fn vertex_id(&mut self, key: VertexKey) -> Result<u32, ModelImportError> {
        if let Some(existing) = self.vertex_ids.get(&key) {
            return Ok(*existing);
        }
        let (position, uv, normal) = key;
        let position = self
            .source_positions
            .get(position as usize)
            .copied()
            .map_or([0.0; 3], |value| value);
        let normal = self
            .source_normals
            .get(normal as usize)
            .copied()
            .map_or([0.0; 3], |value| value);
        let uv = self
            .source_uvs
            .get(uv as usize)
            .copied()
            .map_or([0.0; 2], |value| value);
        let id = self.vertices.len() as u32;
        self.vertices.push(Vertex {
            position,
            normal,
            uv,
        });
        self.vertex_ids.insert(key, id);
        Ok(id)
    }

    fn open_group(&mut self) -> Result<(), ModelImportError> {
        let name = self
            .group
            .clone()
            .or_else(|| self.material.clone())
            .unwrap_or_else(|| "default".to_string());
        let sub_asset_id = match (&self.group, &self.material) {
            (Some(group), Some(material)) => format!("{group}/{material}"),
            (Some(group), None) => group.clone(),
            (None, Some(material)) => material.clone(),
            (None, None) => "default".to_string(),
        };
        if let Some(open) = &self.open {
            if open.sub_asset_id == sub_asset_id {
                return Ok(());
            }
        }
        self.close_group();
        self.open = Some(OpenGroup {
            name,
            sub_asset_id,
            index_offset: self.indices.len() as u32,
        });
        Ok(())
    }

    fn close_group(&mut self) {
        if let Some(open) = self.open.take() {
            let index_count = self.indices.len() as u32 - open.index_offset;
            self.sub_meshes.push(SubMesh {
                name: open.name,
                index_offset: open.index_offset,
                index_count,
                sub_asset_id: open.sub_asset_id,
            });
        }
    }
}

fn single_name(values: &[&str]) -> Option<String> {
    let name = values.join(" ");
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_float(value: &str, line: usize) -> Result<f32, ModelImportError> {
    value
        .parse::<f32>()
        .map_err(|_| malformed(line, format!("`{value}` is not a number")))
}

fn parse_index(value: &str, line: usize, count: usize, kind: &str) -> Result<i64, ModelImportError> {
    let parsed = value
        .parse::<i64>()
        .map_err(|_| malformed(line, format!("`{value}` is not a valid {kind} index")))?;
    if parsed == 0 {
        return Err(malformed(line, format!("{kind} index 0 is not valid")));
    }
    let resolved = if parsed > 0 {
        parsed - 1
    } else {
        count as i64 + parsed
    };
    if resolved < 0 || resolved >= count as i64 {
        return Err(malformed(
            line,
            format!("{kind} index {parsed} is out of range ({count} available)"),
        ));
    }
    Ok(resolved)
}

fn malformed(line: usize, reason: impl Into<String>) -> ModelImportError {
    ModelImportError::MalformedObj {
        line,
        reason: reason.into(),
    }
}
