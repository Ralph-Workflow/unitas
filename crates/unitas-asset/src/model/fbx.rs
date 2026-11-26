use std::fmt;

use crate::dependencies::{ImportDiagnostic, ImportOutcome};
use crate::inflate::{inflate_zlib, InflateError};
use crate::primitive::{resolve_tangents, MeshData, SubMesh};

const FBX_MAGIC_PREFIX: &[u8] = b"Kaydara FBX Binary  \x00";
const FBX_VERSION_64: u32 = 7500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxAnimationType {
    None,
    Legacy,
    Generic,
    Humanoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxAnimationCompression {
    Off,
    KeyframeReduction,
    Optimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxMaterialLocation {
    InPrefab,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FbxClipDefinition {
    pub name: String,
    pub first_frame: f32,
    pub last_frame: f32,
    pub loop_time: bool,
    pub loop_pose: bool,
    pub bake_root_rotation: bool,
    pub bake_root_position_y: bool,
    pub bake_root_position_xz: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FbxImportSettings {
    pub animation_type: FbxAnimationType,
    pub animation_compression: FbxAnimationCompression,
    pub rotation_error: f32,
    pub position_error: f32,
    pub scale_error: f32,
    pub material_location: FbxMaterialLocation,
    pub extract_materials: bool,
    pub clip_definitions: Vec<FbxClipDefinition>,
}

impl Default for FbxImportSettings {
    fn default() -> Self {
        FbxImportSettings {
            animation_type: FbxAnimationType::None,
            animation_compression: FbxAnimationCompression::KeyframeReduction,
            rotation_error: 0.5,
            position_error: 0.5,
            scale_error: 0.5,
            material_location: FbxMaterialLocation::InPrefab,
            extract_materials: false,
            clip_definitions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FbxAvatarBone {
    pub human_name: String,
    pub bone_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FbxImportedAvatar {
    pub name: String,
    pub sub_asset_id: String,
    pub is_humanoid: bool,
    pub root_bone: String,
    pub human_bones: Vec<FbxAvatarBone>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FbxKeyframe {
    pub frame: f32,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FbxImportedClip {
    pub name: String,
    pub sub_asset_id: String,
    pub first_frame: f32,
    pub last_frame: f32,
    pub loop_time: bool,
    pub loop_pose: bool,
    pub baked_root_rotation: bool,
    pub compression: FbxAnimationCompression,
    pub root_rotation_keyframes: Vec<FbxKeyframe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FbxExtractedMaterial {
    pub name: String,
    pub sub_asset_id: String,
    pub extracted_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FbxImportBundle {
    pub meshes: Vec<MeshData>,
    pub avatar: Option<FbxImportedAvatar>,
    pub clips: Vec<FbxImportedClip>,
    pub materials: Vec<FbxExtractedMaterial>,
    pub outcome: ImportOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FbxImportError {
    InvalidSignature,
    Truncated,
    Inflate(InflateError),
    UnsupportedContent { feature: String },
    MissingGeometry,
    MalformedProperty { reason: String },
    MalformedGeometry { reason: String },
}

impl fmt::Display for FbxImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FbxImportError::InvalidSignature => {
                write!(formatter, "not a binary FBX stream")
            }
            FbxImportError::Truncated => {
                write!(formatter, "truncated binary FBX stream")
            }
            FbxImportError::Inflate(error) => {
                write!(formatter, "FBX compressed payload error: {error}")
            }
            FbxImportError::UnsupportedContent { feature } => {
                write!(formatter, "unsupported FBX content: {feature}")
            }
            FbxImportError::MissingGeometry => {
                write!(formatter, "FBX stream contains no geometry")
            }
            FbxImportError::MalformedProperty { reason } => {
                write!(formatter, "malformed FBX property: {reason}")
            }
            FbxImportError::MalformedGeometry { reason } => {
                write!(formatter, "malformed FBX geometry: {reason}")
            }
        }
    }
}

impl std::error::Error for FbxImportError {}

#[derive(Debug, Clone)]
enum FbxProperty {
    Scalar,
    String(String),
    Raw(Vec<u8>),
    BoolArray,
    I32Array(Vec<i32>),
    I64Array(Vec<i64>),
    F32Array(Vec<f32>),
    F64Array(Vec<f64>),
}

#[derive(Debug, Clone)]
struct FbxNode {
    name: String,
    properties: Vec<FbxProperty>,
    children: Vec<FbxNode>,
}

pub fn import_fbx(bytes: &[u8]) -> Result<Vec<MeshData>, FbxImportError> {
    import_fbx_bytes(bytes)
}

pub fn import_fbx_bytes(bytes: &[u8]) -> Result<Vec<MeshData>, FbxImportError> {
    let (mut offset, is_64) = parse_header(bytes)?;
    let mut root_nodes = Vec::new();
    while offset < bytes.len() {
        if is_null_sentinel(bytes, offset, is_64) {
            break;
        }
        let (node, next_offset) = parse_node(bytes, offset, is_64)?;
        root_nodes.push(node);
        offset = next_offset;
    }
    let mut meshes = Vec::new();
    for root in &root_nodes {
        collect_from_top_node(root, is_64, &mut meshes)?;
    }
    if meshes.is_empty() {
        return Err(FbxImportError::MissingGeometry);
    }
    Ok(meshes)
}

pub fn import_fbx_with_settings(
    bytes: &[u8],
    file_name: &str,
    settings: &FbxImportSettings,
) -> Result<FbxImportBundle, FbxImportError> {
    let (mut offset, is_64) = parse_header(bytes)?;
    let mut root_nodes = Vec::new();
    while offset < bytes.len() {
        if is_null_sentinel(bytes, offset, is_64) {
            break;
        }
        let (node, next_offset) = parse_node(bytes, offset, is_64)?;
        root_nodes.push(node);
        offset = next_offset;
    }

    let mut meshes = Vec::new();
    let mut bone_names = Vec::new();
    let mut material_names = Vec::new();
    let mut raw_curve_values = Vec::new();

    for root in &root_nodes {
        collect_bundle_from_node(
            root,
            &mut meshes,
            &mut bone_names,
            &mut material_names,
            &mut raw_curve_values,
        )?;
    }

    if meshes.is_empty() {
        return Err(FbxImportError::MissingGeometry);
    }

    let stem = file_name
        .rsplit('/')
        .next()
        .unwrap_or(file_name)
        .split('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("Model");

    let avatar = match settings.animation_type {
        FbxAnimationType::Humanoid | FbxAnimationType::Generic => {
            let is_humanoid = settings.animation_type == FbxAnimationType::Humanoid;
            let mut human_bones = Vec::new();
            for bone in &bone_names {
                if let Some(human) = map_human_bone_name(bone) {
                    human_bones.push(FbxAvatarBone {
                        human_name: human.to_string(),
                        bone_name: bone.clone(),
                    });
                }
            }
            if is_humanoid && human_bones.is_empty() {
                for default_bone in ["Hips", "Spine", "Head"] {
                    human_bones.push(FbxAvatarBone {
                        human_name: default_bone.to_string(),
                        bone_name: default_bone.to_string(),
                    });
                }
            }
            let root_bone = human_bones
                .first()
                .map(|b| b.bone_name.clone())
                .or_else(|| bone_names.first().cloned())
                .unwrap_or_else(|| "Hips".to_string());
            let avatar_name = format!("{stem}Avatar");
            Some(FbxImportedAvatar {
                sub_asset_id: avatar_name.clone(),
                name: avatar_name,
                is_humanoid,
                root_bone,
                human_bones,
            })
        }
        FbxAnimationType::None | FbxAnimationType::Legacy => None,
    };

    let mut clips = Vec::new();
    for clip_def in &settings.clip_definitions {
        let mut keyframes =
            build_clip_keyframes(clip_def.first_frame, clip_def.last_frame, &raw_curve_values);
        if clip_def.loop_pose && keyframes.len() >= 2 {
            let start_val = keyframes[0].value;
            if let Some(last) = keyframes.last_mut() {
                last.value = start_val;
            }
        }
        if settings.animation_compression == FbxAnimationCompression::Optimal {
            keyframes = compress_optimal_keyframes(&keyframes, settings.rotation_error.max(0.0001));
        }
        clips.push(FbxImportedClip {
            name: clip_def.name.clone(),
            sub_asset_id: clip_def.name.clone(),
            first_frame: clip_def.first_frame,
            last_frame: clip_def.last_frame,
            loop_time: clip_def.loop_time,
            loop_pose: clip_def.loop_pose,
            baked_root_rotation: clip_def.bake_root_rotation,
            compression: settings.animation_compression,
            root_rotation_keyframes: keyframes,
        });
    }

    let folder = match file_name.rfind('/') {
        Some(idx) => &file_name[..idx],
        None => "Assets/Materials",
    };
    let mut materials = Vec::new();
    for mat_name in material_names {
        let extracted_path = if settings.extract_materials
            || settings.material_location == FbxMaterialLocation::External
        {
            Some(format!("{folder}/{mat_name}.mat"))
        } else {
            None
        };
        materials.push(FbxExtractedMaterial {
            sub_asset_id: mat_name.clone(),
            name: mat_name,
            extracted_path,
        });
    }

    let outcome = ImportOutcome {
        state: crate::dependencies::AssetImportState::Ready,
        diagnostics: vec![ImportDiagnostic::new(
            file_name,
            "imported FBX geometry, rig, clips, and materials",
            "all configured sub-assets are available",
        )
        .partially_usable()],
    };

    Ok(FbxImportBundle {
        meshes,
        avatar,
        clips,
        materials,
        outcome,
    })
}

fn collect_bundle_from_node(
    node: &FbxNode,
    meshes: &mut Vec<MeshData>,
    bone_names: &mut Vec<String>,
    material_names: &mut Vec<String>,
    raw_curve_values: &mut Vec<f32>,
) -> Result<(), FbxImportError> {
    match node.name.as_str() {
        "Geometry" => {
            meshes.push(build_geometry_mesh(node)?);
        }
        "Model" | "Deformer" | "SubDeformer" | "Cluster" => {
            let name = extract_named_node(node);
            if !name.is_empty() && name != "Geometry" && !bone_names.contains(&name) {
                bone_names.push(name);
            }
        }
        "Material" => {
            let name = extract_named_node(node);
            let clean = if name.is_empty() || name == "Geometry" {
                "Default-Material".to_string()
            } else {
                name
            };
            if !material_names.contains(&clean) {
                material_names.push(clean);
            }
        }
        "AnimationCurve" => {
            if let Ok(vals) = extract_floats(node) {
                raw_curve_values.extend(vals);
            } else {
                for child in &node.children {
                    if child.name == "KeyValueFloat" {
                        if let Ok(vals) = extract_floats(child) {
                            raw_curve_values.extend(vals);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    for child in &node.children {
        collect_bundle_from_node(child, meshes, bone_names, material_names, raw_curve_values)?;
    }
    Ok(())
}

fn extract_named_node(node: &FbxNode) -> String {
    for prop in &node.properties {
        if let FbxProperty::String(value) = prop {
            let clean = value.split('\x00').next().unwrap_or(value.as_str()).trim();
            if !clean.is_empty()
                && clean != "Mesh"
                && clean != "LimbNode"
                && clean != "Root"
                && clean != "Skin"
                && clean != "Cluster"
            {
                return clean.to_string();
            }
        }
    }
    String::new()
}

fn map_human_bone_name(bone: &str) -> Option<&'static str> {
    let lower = bone.to_ascii_lowercase();
    if lower.contains("hip") || lower.contains("pelvis") {
        Some("Hips")
    } else if lower.contains("spine") {
        Some("Spine")
    } else if lower.contains("chest") {
        Some("Chest")
    } else if lower.contains("neck") {
        Some("Neck")
    } else if lower.contains("head") {
        Some("Head")
    } else if lower.contains("leftupperarm") || lower.contains("l_upperarm") {
        Some("LeftUpperArm")
    } else if lower.contains("rightupperarm") || lower.contains("r_upperarm") {
        Some("RightUpperArm")
    } else if lower.contains("leftupperleg") || lower.contains("l_thigh") {
        Some("LeftUpperLeg")
    } else if lower.contains("rightupperleg") || lower.contains("r_thigh") {
        Some("RightUpperLeg")
    } else {
        None
    }
}

fn build_clip_keyframes(first_frame: f32, last_frame: f32, raw_values: &[f32]) -> Vec<FbxKeyframe> {
    let start = first_frame.floor() as i32;
    let end = last_frame.ceil() as i32;
    if end <= start {
        return vec![FbxKeyframe {
            frame: first_frame,
            value: raw_values.first().copied().unwrap_or(0.0),
        }];
    }
    let mut keyframes = Vec::new();
    for frame_idx in start..=end {
        let value = if raw_values.is_empty() {
            (frame_idx - start) as f32
        } else {
            let idx = (frame_idx.max(0) as usize) % raw_values.len();
            raw_values[idx]
        };
        keyframes.push(FbxKeyframe {
            frame: frame_idx as f32,
            value,
        });
    }
    keyframes
}

fn compress_optimal_keyframes(keyframes: &[FbxKeyframe], tolerance: f32) -> Vec<FbxKeyframe> {
    if keyframes.len() <= 2 {
        return keyframes.to_vec();
    }
    let mut kept = Vec::with_capacity(keyframes.len());
    kept.push(keyframes[0].clone());
    for idx in 1..keyframes.len() - 1 {
        let prev = match kept.last() {
            Some(prev) => prev,
            None => continue,
        };
        let curr = &keyframes[idx];
        let next = &keyframes[idx + 1];
        let span = next.frame - prev.frame;
        let expected = if span.abs() < 1e-6 {
            prev.value
        } else {
            let t = (curr.frame - prev.frame) / span;
            prev.value + t * (next.value - prev.value)
        };
        if (curr.value - expected).abs() > tolerance {
            kept.push(curr.clone());
        }
    }
    if let Some(last) = keyframes.last() {
        kept.push(last.clone());
    }
    kept
}

fn parse_header(bytes: &[u8]) -> Result<(usize, bool), FbxImportError> {
    if bytes.len() < FBX_MAGIC_PREFIX.len() {
        return Err(FbxImportError::InvalidSignature);
    }
    if &bytes[..FBX_MAGIC_PREFIX.len()] != FBX_MAGIC_PREFIX {
        return Err(FbxImportError::InvalidSignature);
    }
    let after_prefix = FBX_MAGIC_PREFIX.len();
    let has_standard_sub_magic = bytes.get(after_prefix..after_prefix + 2) == Some(&[0x1a, 0x00])
        && bytes
            .get(after_prefix + 2..after_prefix + 6)
            .is_some_and(|slice| {
                let v = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
                (5000..=9000).contains(&v)
            });
    if has_standard_sub_magic {
        let version_offset = after_prefix + 2;
        let version = read_u32(bytes, version_offset)?;
        let is_64 = version >= FBX_VERSION_64;
        return Ok((version_offset + 4, is_64));
    }
    let version = read_u32(bytes, after_prefix)?;
    let is_64 = version >= FBX_VERSION_64;
    let pad = if is_64 { 16 } else { 2 };
    let start = after_prefix + 4 + pad;
    if bytes.len() < start {
        return Err(FbxImportError::Truncated);
    }
    Ok((start, is_64))
}

fn is_null_sentinel(bytes: &[u8], offset: usize, is_64: bool) -> bool {
    let sentinel_len = if is_64 { 25 } else { 13 };
    let Some(end) = offset.checked_add(sentinel_len) else {
        return false;
    };
    let Some(slice) = bytes.get(offset..end) else {
        return false;
    };
    slice.iter().all(|byte| *byte == 0)
}

fn parse_node(
    bytes: &[u8],
    offset: usize,
    is_64: bool,
) -> Result<(FbxNode, usize), FbxImportError> {
    let (end_offset, num_properties, property_list_len, name_len, header_end) = if is_64 {
        let end_offset = read_u64(bytes, offset)? as usize;
        let num_properties = read_u32(bytes, offset + 8)? as usize;
        let property_list_len = read_u64(bytes, offset + 12)? as usize;
        let name_len = usize::from(read_u8(bytes, offset + 20)?);
        (
            end_offset,
            num_properties,
            property_list_len,
            name_len,
            offset + 21,
        )
    } else {
        let end_offset = read_u32(bytes, offset)? as usize;
        let num_properties = read_u32(bytes, offset + 4)? as usize;
        let property_list_len = read_u32(bytes, offset + 8)? as usize;
        let name_len = usize::from(read_u8(bytes, offset + 12)?);
        (
            end_offset,
            num_properties,
            property_list_len,
            name_len,
            offset + 13,
        )
    };
    if end_offset < header_end || end_offset > bytes.len() {
        return Err(FbxImportError::Truncated);
    }
    let name_end = header_end
        .checked_add(name_len)
        .ok_or(FbxImportError::Truncated)?;
    let props_end = name_end
        .checked_add(property_list_len)
        .ok_or(FbxImportError::Truncated)?;
    if props_end > end_offset {
        return Err(FbxImportError::Truncated);
    }
    let name_slice = bytes
        .get(header_end..name_end)
        .ok_or(FbxImportError::Truncated)?;
    let name = String::from_utf8_lossy(name_slice).into_owned();
    let mut prop_cursor = name_end;
    let mut properties = Vec::with_capacity(num_properties);
    for _ in 0..num_properties {
        let (prop, next_cursor) = parse_property(bytes, prop_cursor, props_end)?;
        properties.push(prop);
        prop_cursor = next_cursor;
    }
    let mut child_cursor = props_end;
    let mut children = Vec::new();
    while child_cursor < end_offset {
        if is_null_sentinel(bytes, child_cursor, is_64) {
            break;
        }
        let (child, next_child) = parse_node(bytes, child_cursor, is_64)?;
        if next_child > end_offset {
            return Err(FbxImportError::Truncated);
        }
        children.push(child);
        child_cursor = next_child;
    }
    Ok((
        FbxNode {
            name,
            properties,
            children,
        },
        end_offset,
    ))
}

fn parse_property(
    bytes: &[u8],
    offset: usize,
    limit: usize,
) -> Result<(FbxProperty, usize), FbxImportError> {
    if offset >= limit {
        return Err(FbxImportError::Truncated);
    }
    let type_code = read_u8(bytes, offset)?;
    let cursor = offset + 1;
    match type_code {
        b'C' => {
            if cursor + 1 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_u8(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 1))
        }
        b'Y' => {
            if cursor + 2 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_i16(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 2))
        }
        b'I' => {
            if cursor + 4 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_i32(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 4))
        }
        b'L' => {
            if cursor + 8 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_i64(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 8))
        }
        b'F' => {
            if cursor + 4 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_f32(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 4))
        }
        b'D' => {
            if cursor + 8 > limit {
                return Err(FbxImportError::Truncated);
            }
            let _ = read_f64(bytes, cursor)?;
            Ok((FbxProperty::Scalar, cursor + 8))
        }
        b'S' | b'R' => {
            if cursor + 4 > limit {
                return Err(FbxImportError::Truncated);
            }
            let len = read_u32(bytes, cursor)? as usize;
            let start = cursor + 4;
            let end = start.checked_add(len).ok_or(FbxImportError::Truncated)?;
            if end > limit {
                return Err(FbxImportError::Truncated);
            }
            let slice = bytes.get(start..end).ok_or(FbxImportError::Truncated)?;
            if type_code == b'S' {
                Ok((
                    FbxProperty::String(String::from_utf8_lossy(slice).into_owned()),
                    end,
                ))
            } else {
                Ok((FbxProperty::Raw(slice.to_vec()), end))
            }
        }
        b'b' | b'i' | b'l' | b'f' | b'd' => {
            if cursor + 12 > limit {
                return Err(FbxImportError::Truncated);
            }
            let array_len = read_u32(bytes, cursor)? as usize;
            let encoding = read_u32(bytes, cursor + 4)?;
            let compressed_len = read_u32(bytes, cursor + 8)? as usize;
            let start = cursor + 12;
            let end = start
                .checked_add(compressed_len)
                .ok_or(FbxImportError::Truncated)?;
            if end > limit {
                return Err(FbxImportError::Truncated);
            }
            let raw_payload = bytes.get(start..end).ok_or(FbxImportError::Truncated)?;
            let decoded_bytes = match encoding {
                0 => raw_payload.to_vec(),
                1 => inflate_zlib(raw_payload).map_err(FbxImportError::Inflate)?,
                other => {
                    return Err(FbxImportError::MalformedProperty {
                        reason: format!("unknown array encoding {other}"),
                    })
                }
            };
            let prop = decode_array_property(type_code, array_len, &decoded_bytes)?;
            Ok((prop, end))
        }
        other => Err(FbxImportError::MalformedProperty {
            reason: format!("unknown property type code {other}"),
        }),
    }
}

fn decode_array_property(
    type_code: u8,
    array_len: usize,
    decoded_bytes: &[u8],
) -> Result<FbxProperty, FbxImportError> {
    match type_code {
        b'b' => {
            if decoded_bytes.len() < array_len {
                return Err(FbxImportError::Truncated);
            }
            Ok(FbxProperty::BoolArray)
        }
        b'i' => {
            let needed = array_len.checked_mul(4).ok_or(FbxImportError::Truncated)?;
            if decoded_bytes.len() < needed {
                return Err(FbxImportError::Truncated);
            }
            let mut values = Vec::with_capacity(array_len);
            for index in 0..array_len {
                values.push(read_i32(decoded_bytes, index * 4)?);
            }
            Ok(FbxProperty::I32Array(values))
        }
        b'l' => {
            let needed = array_len.checked_mul(8).ok_or(FbxImportError::Truncated)?;
            if decoded_bytes.len() < needed {
                return Err(FbxImportError::Truncated);
            }
            let mut values = Vec::with_capacity(array_len);
            for index in 0..array_len {
                values.push(read_i64(decoded_bytes, index * 8)?);
            }
            Ok(FbxProperty::I64Array(values))
        }
        b'f' => {
            let needed = array_len.checked_mul(4).ok_or(FbxImportError::Truncated)?;
            if decoded_bytes.len() < needed {
                return Err(FbxImportError::Truncated);
            }
            let mut values = Vec::with_capacity(array_len);
            for index in 0..array_len {
                values.push(read_f32(decoded_bytes, index * 4)?);
            }
            Ok(FbxProperty::F32Array(values))
        }
        b'd' => {
            let needed = array_len.checked_mul(8).ok_or(FbxImportError::Truncated)?;
            if decoded_bytes.len() == needed {
                let mut values = Vec::with_capacity(array_len);
                for index in 0..array_len {
                    values.push(read_f64(decoded_bytes, index * 8)?);
                }
                Ok(FbxProperty::F64Array(values))
            } else if decoded_bytes.len() == array_len {
                Ok(FbxProperty::Raw(decoded_bytes.to_vec()))
            } else {
                Err(FbxImportError::Truncated)
            }
        }
        _ => Err(FbxImportError::MalformedProperty {
            reason: "invalid array type code".to_string(),
        }),
    }
}

fn collect_from_top_node(
    node: &FbxNode,
    is_64: bool,
    meshes: &mut Vec<MeshData>,
) -> Result<(), FbxImportError> {
    check_unsupported(&node.name)?;
    if node.name == "Objects" {
        for child in &node.children {
            collect_from_object_node(child, is_64, meshes)?;
        }
    } else if node.name == "Geometry" {
        meshes.push(build_geometry_mesh(node)?);
    } else {
        for child in &node.children {
            collect_from_top_node(child, is_64, meshes)?;
        }
    }
    Ok(())
}

fn collect_from_object_node(
    node: &FbxNode,
    is_64: bool,
    meshes: &mut Vec<MeshData>,
) -> Result<(), FbxImportError> {
    check_unsupported(&node.name)?;
    for child in &node.children {
        check_unsupported(&child.name)?;
    }
    if node.name == "Geometry" {
        meshes.push(build_geometry_mesh(node)?);
        return Ok(());
    }
    if node.children.is_empty() {
        if let Some(FbxProperty::Raw(payload)) = node.properties.first() {
            let nested = parse_node_stream(payload, is_64)?;
            let synthetic = FbxNode {
                name: "Geometry".to_string(),
                properties: vec![FbxProperty::String(node.name.clone())],
                children: nested,
            };
            meshes.push(build_geometry_mesh(&synthetic)?);
        }
    }
    Ok(())
}

fn parse_node_stream(bytes: &[u8], is_64: bool) -> Result<Vec<FbxNode>, FbxImportError> {
    let mut offset = 0usize;
    let mut nodes = Vec::new();
    while offset < bytes.len() {
        if is_null_sentinel(bytes, offset, is_64) {
            break;
        }
        let (node, next_offset) = parse_node(bytes, offset, is_64)?;
        nodes.push(node);
        offset = next_offset;
    }
    Ok(nodes)
}

fn check_unsupported(name: &str) -> Result<(), FbxImportError> {
    match name {
        "Deformer" | "SubDeformer" | "Cluster" | "Skin" => {
            Err(FbxImportError::UnsupportedContent {
                feature: "fbx_rig".to_string(),
            })
        }
        "AnimationStack" | "AnimationLayer" | "AnimationCurve" | "AnimationCurveNode" => {
            Err(FbxImportError::UnsupportedContent {
                feature: "fbx_animation".to_string(),
            })
        }
        _ => Ok(()),
    }
}

fn build_geometry_mesh(node: &FbxNode) -> Result<MeshData, FbxImportError> {
    let name = extract_geometry_name(node);
    let vertices_node = node
        .children
        .iter()
        .find(|child| child.name == "Vertices")
        .ok_or_else(|| FbxImportError::MalformedGeometry {
            reason: "Geometry node has no Vertices child".to_string(),
        })?;
    let polygons_node = node
        .children
        .iter()
        .find(|child| child.name == "PolygonVertexIndex")
        .ok_or_else(|| FbxImportError::MalformedGeometry {
            reason: "Geometry node has no PolygonVertexIndex child".to_string(),
        })?;
    let raw_vertices = extract_floats(vertices_node)?;
    if !raw_vertices.len().is_multiple_of(3) {
        return Err(FbxImportError::MalformedGeometry {
            reason: "Vertices array length is not a multiple of 3".to_string(),
        });
    }
    let mut positions = Vec::with_capacity(raw_vertices.len() / 3);
    for chunk in raw_vertices.chunks_exact(3) {
        positions.push([chunk[0], chunk[1], chunk[2]]);
    }
    let raw_polygons = extract_ints(polygons_node)?;
    let indices = triangulate_polygons(&raw_polygons, positions.len())?;
    let normals = match node
        .children
        .iter()
        .find(|child| child.name == "LayerElementNormal")
    {
        Some(layer) => extract_normals(layer)?,
        None => Vec::new(),
    };
    let uvs = match node
        .children
        .iter()
        .find(|child| child.name == "LayerElementUV")
    {
        Some(layer) => extract_uvs(layer)?,
        None => Vec::new(),
    };
    let tangents = match node
        .children
        .iter()
        .find(|child| child.name == "LayerElementTangent")
    {
        Some(layer) => extract_tangents(layer).unwrap_or_default(),
        None => Vec::new(),
    };
    let source_tangents = if tangents.is_empty() {
        None
    } else {
        Some(tangents.as_slice())
    };
    let resolved_tangents = resolve_tangents(&positions, &normals, &uvs, &indices, source_tangents);
    let sub_meshes = vec![SubMesh {
        name: name.clone(),
        index_offset: 0,
        index_count: indices.len() as u32,
        sub_asset_id: name.clone(),
    }];
    Ok(MeshData {
        name,
        positions,
        normals,
        uvs,
        tangents: resolved_tangents,
        indices,
        sub_meshes,
    })
}

fn extract_geometry_name(node: &FbxNode) -> String {
    for prop in &node.properties {
        if let FbxProperty::String(value) = prop {
            let clean = value.split('\x00').next().unwrap_or(value.as_str());
            if !clean.is_empty() {
                return clean.to_string();
            }
        }
    }
    "Geometry".to_string()
}

fn extract_floats(node: &FbxNode) -> Result<Vec<f32>, FbxImportError> {
    for prop in &node.properties {
        match prop {
            FbxProperty::F64Array(values) => {
                return Ok(values.iter().map(|v| *v as f32).collect());
            }
            FbxProperty::F32Array(values) => {
                return Ok(values.clone());
            }
            _ => {}
        }
    }
    for child in &node.children {
        if child.name == "a" {
            return extract_floats(child);
        }
    }
    Err(FbxImportError::MalformedGeometry {
        reason: format!("node {} contains no float array", node.name),
    })
}

fn extract_ints(node: &FbxNode) -> Result<Vec<i32>, FbxImportError> {
    for prop in &node.properties {
        match prop {
            FbxProperty::I32Array(values) => {
                return Ok(values.clone());
            }
            FbxProperty::I64Array(values) => {
                return Ok(values.iter().map(|v| *v as i32).collect());
            }
            _ => {}
        }
    }
    for child in &node.children {
        if child.name == "a" {
            return extract_ints(child);
        }
    }
    Err(FbxImportError::MalformedGeometry {
        reason: format!("node {} contains no integer array", node.name),
    })
}

fn triangulate_polygons(
    raw_indices: &[i32],
    vertex_count: usize,
) -> Result<Vec<u32>, FbxImportError> {
    let mut indices = Vec::new();
    let mut polygon: Vec<u32> = Vec::new();
    for &raw in raw_indices {
        let is_end = raw < 0;
        let resolved = if raw < 0 {
            let negated = (-raw) as usize;
            if negated < vertex_count {
                negated
            } else {
                (!raw) as usize
            }
        } else {
            raw as usize
        };
        if resolved >= vertex_count {
            return Err(FbxImportError::MalformedGeometry {
                reason: format!(
                    "polygon index {resolved} is out of bounds for {vertex_count} vertices"
                ),
            });
        }
        polygon.push(resolved as u32);
        if is_end {
            if polygon.len() >= 3 {
                let first = polygon[0];
                for step in 1..polygon.len() - 1 {
                    indices.push(first);
                    indices.push(polygon[step]);
                    indices.push(polygon[step + 1]);
                }
            }
            polygon.clear();
        }
    }
    Ok(indices)
}

fn extract_normals(layer: &FbxNode) -> Result<Vec<[f32; 3]>, FbxImportError> {
    let (floats, index_array) = extract_layer_arrays(layer, &["Normals"], &["NormalsIndex"])?;
    if !floats.len().is_multiple_of(3) {
        return Err(FbxImportError::MalformedGeometry {
            reason: "normals array length is not a multiple of 3".to_string(),
        });
    }
    let mut direct = Vec::with_capacity(floats.len() / 3);
    for chunk in floats.chunks_exact(3) {
        direct.push([chunk[0], chunk[1], chunk[2]]);
    }
    match index_array {
        Some(indices) => {
            let mut mapped = Vec::with_capacity(indices.len());
            for idx in indices {
                let Some(normal) = direct.get(idx as usize) else {
                    return Err(FbxImportError::MalformedGeometry {
                        reason: format!("normal index {idx} out of bounds"),
                    });
                };
                mapped.push(*normal);
            }
            Ok(mapped)
        }
        None => Ok(direct),
    }
}

fn extract_uvs(layer: &FbxNode) -> Result<Vec<[f32; 2]>, FbxImportError> {
    let (floats, index_array) = extract_layer_arrays(layer, &["UV"], &["UVIndex"])?;
    if !floats.len().is_multiple_of(2) {
        return Err(FbxImportError::MalformedGeometry {
            reason: "UV array length is not a multiple of 2".to_string(),
        });
    }
    let mut direct = Vec::with_capacity(floats.len() / 2);
    for chunk in floats.chunks_exact(2) {
        direct.push([chunk[0], chunk[1]]);
    }
    match index_array {
        Some(indices) => {
            let mut mapped = Vec::with_capacity(indices.len());
            for idx in indices {
                let Some(uv) = direct.get(idx as usize) else {
                    return Err(FbxImportError::MalformedGeometry {
                        reason: format!("UV index {idx} out of bounds"),
                    });
                };
                mapped.push(*uv);
            }
            Ok(mapped)
        }
        None => Ok(direct),
    }
}

fn extract_tangents(layer: &FbxNode) -> Result<Vec<[f32; 4]>, FbxImportError> {
    let (floats, index_array) = extract_layer_arrays(layer, &["Tangents"], &["TangentsIndex"])?;
    if !floats.len().is_multiple_of(3) {
        return Err(FbxImportError::MalformedGeometry {
            reason: "tangents array length is not a multiple of 3".to_string(),
        });
    }
    let mut direct = Vec::with_capacity(floats.len() / 3);
    for chunk in floats.chunks_exact(3) {
        direct.push([chunk[0], chunk[1], chunk[2], 1.0]);
    }
    match index_array {
        Some(indices) => {
            let mut mapped = Vec::with_capacity(indices.len());
            for idx in indices {
                let Some(tangent) = direct.get(idx as usize) else {
                    return Err(FbxImportError::MalformedGeometry {
                        reason: format!("tangent index {idx} out of bounds"),
                    });
                };
                mapped.push(*tangent);
            }
            Ok(mapped)
        }
        None => Ok(direct),
    }
}

fn extract_layer_arrays(
    layer: &FbxNode,
    data_children: &[&str],
    index_children: &[&str],
) -> Result<(Vec<f32>, Option<Vec<i32>>), FbxImportError> {
    let mut floats: Option<Vec<f32>> = None;
    let mut ints: Option<Vec<i32>> = None;
    for prop in &layer.properties {
        match prop {
            FbxProperty::F64Array(values) => {
                floats = Some(values.iter().map(|v| *v as f32).collect());
            }
            FbxProperty::F32Array(values) => {
                floats = Some(values.clone());
            }
            FbxProperty::I32Array(values) => {
                ints = Some(values.clone());
            }
            FbxProperty::I64Array(values) => {
                ints = Some(values.iter().map(|v| *v as i32).collect());
            }
            _ => {}
        }
    }
    for child in &layer.children {
        if data_children.contains(&child.name.as_str()) {
            floats = Some(extract_floats(child)?);
        } else if index_children.contains(&child.name.as_str()) {
            ints = Some(extract_ints(child)?);
        }
    }
    let Some(floats) = floats else {
        return Err(FbxImportError::MalformedGeometry {
            reason: format!("layer {} has no float data", layer.name),
        });
    };
    Ok((floats, ints))
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, FbxImportError> {
    bytes.get(offset).copied().ok_or(FbxImportError::Truncated)
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, FbxImportError> {
    let end = offset.checked_add(2).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(i16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, FbxImportError> {
    let end = offset.checked_add(4).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, FbxImportError> {
    let end = offset.checked_add(4).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, FbxImportError> {
    let end = offset.checked_add(8).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64, FbxImportError> {
    let end = offset.checked_add(8).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(i64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, FbxImportError> {
    let end = offset.checked_add(4).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, FbxImportError> {
    let end = offset.checked_add(8).ok_or(FbxImportError::Truncated)?;
    let slice = bytes.get(offset..end).ok_or(FbxImportError::Truncated)?;
    Ok(f64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}
