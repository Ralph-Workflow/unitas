use crate::unity_yaml::{self, Stream, YamlError};
use std::fmt;

pub const SUPPORTED_VERSION: u32 = 22;

const HEADER_SIZE: usize = 20;
const OBJECT_ENTRY_SIZE: usize = 25;
const MONOBEHAVIOUR_CLASS_ID: i32 = 114;
const TYPE_HASH_SIZE: usize = 16;
const TYPE_TREE_NODE_SIZE: usize = 32;
const ALIGN_BYTES_FLAG: i32 = 0x4000;
const COMMON_STRING_OFFSET_FLAG: u32 = 0x8000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Big,
    Little,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFileHeader {
    pub metadata_size: u32,
    pub file_size: u32,
    pub version: u32,
    pub data_offset: u32,
    pub endianness: Endianness,
    pub header_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFileMetadata {
    pub unity_version: String,
    pub target_platform: i32,
    pub enable_type_tree: bool,
    pub type_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedObject {
    pub path_id: i64,
    pub class_id: i32,
    pub byte_offset: u64,
    pub byte_size: u32,
    pub is_stripped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTreeNode {
    pub version: i16,
    pub level: u8,
    pub type_flags: u8,
    pub type_name: String,
    pub name: String,
    pub byte_size: i32,
    pub index: i32,
    pub meta_flag: i32,
}

impl TypeTreeNode {
    pub fn aligns_after(&self) -> bool {
        (self.meta_flag & ALIGN_BYTES_FLAG) != 0
    }

    pub fn is_array(&self) -> bool {
        self.type_flags != 0 || self.type_name == "Array"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeTree {
    pub nodes: Vec<TypeTreeNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedValue {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Array(Vec<TypedValue>),
    Struct {
        type_name: String,
        fields: Vec<(String, TypedValue)>,
    },
}

impl TypedValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::I32(value) => Some(*value),
            Self::I16(value) => Some(i32::from(*value)),
            Self::I8(value) => Some(i32::from(*value)),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(*value),
            Self::U16(value) => Some(u32::from(*value)),
            Self::U8(value) => Some(u32::from(*value)),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(*value),
            Self::I32(value) => Some(i64::from(*value)),
            Self::I16(value) => Some(i64::from(*value)),
            Self::I8(value) => Some(i64::from(*value)),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(*value),
            Self::U32(value) => Some(u64::from(*value)),
            Self::U16(value) => Some(u64::from(*value)),
            Self::U8(value) => Some(u64::from(*value)),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(value) => Some(*value),
            Self::F32(value) => Some(f64::from(*value)),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[TypedValue]> {
        match self {
            Self::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn get(&self, field: &str) -> Option<&TypedValue> {
        match self {
            Self::Struct { fields, .. } => fields
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, field: &str) -> Option<&mut TypedValue> {
        match self {
            Self::Struct { fields, .. } => fields
                .iter_mut()
                .find(|(name, _)| name == field)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedObject {
    pub path_id: i64,
    pub class_id: i32,
    pub type_name: String,
    pub fields: Vec<(String, TypedValue)>,
}

impl TypedObject {
    pub fn get(&self, field: &str) -> Option<&TypedValue> {
        self.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, field: &str) -> Option<&mut TypedValue> {
        self.fields
            .iter_mut()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value)
    }

    pub fn set(&mut self, field: &str, value: TypedValue) -> Result<(), BinarySerError> {
        let slot = self
            .fields
            .iter_mut()
            .find(|(name, _)| name == field)
            .ok_or_else(|| BinarySerError::FieldNotFound {
                type_name: self.type_name.clone(),
                field: field.to_string(),
            })?;
        slot.1 = value;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFile {
    pub header: SerializedFileHeader,
    pub metadata: SerializedFileMetadata,
    pub objects: Vec<SerializedObject>,
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializedAsset {
    Text {
        stream: Stream,
        raw: Vec<u8>,
    },
    Binary(SerializedFile),
}

impl SerializedAsset {
    pub fn read(bytes: &[u8]) -> Result<Self, BinarySerError> {
        read_serialized(bytes)
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    pub fn as_text(&self) -> Option<&Stream> {
        match self {
            Self::Text { stream, .. } => Some(stream),
            Self::Binary(_) => None,
        }
    }

    pub fn as_binary(&self) -> Option<&SerializedFile> {
        match self {
            Self::Text { .. } => None,
            Self::Binary(file) => Some(file),
        }
    }

    pub fn as_binary_mut(&mut self) -> Option<&mut SerializedFile> {
        match self {
            Self::Text { .. } => None,
            Self::Binary(file) => Some(file),
        }
    }

    pub fn write_back(&self) -> Vec<u8> {
        match self {
            Self::Text { raw, .. } => raw.clone(),
            Self::Binary(file) => file.write_back(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinarySerError {
    Truncated {
        at: usize,
    },
    InvalidHeader(&'static str),
    UnsupportedVersion {
        found: u32,
        supported: u32,
    },
    UnsupportedEndianness,
    InvalidObjectTable(&'static str),
    InvalidText(YamlError),
    ObjectNotFound {
        path_id: i64,
    },
    MissingTypeTree {
        path_id: i64,
        class_id: i32,
    },
    UnsupportedTypeTree {
        path_id: i64,
        class_id: i32,
        type_name: String,
    },
    FieldNotFound {
        type_name: String,
        field: String,
    },
    TypeMismatch {
        field: String,
        expected: String,
    },
    InvalidPayload {
        path_id: i64,
        reason: &'static str,
    },
}

impl fmt::Display for BinarySerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { at } => write!(f, "truncated serialized file: {at} bytes required"),
            Self::InvalidHeader(message) => {
                write!(f, "invalid serialized file header: {message}")
            }
            Self::UnsupportedVersion { found, supported } => {
                write!(
                    f,
                    "unsupported serialized file version {found}, supported version is {supported}"
                )
            }
            Self::UnsupportedEndianness => {
                write!(f, "unsupported serialized file endianness flag")
            }
            Self::InvalidObjectTable(message) => {
                write!(f, "invalid serialized file object table: {message}")
            }
            Self::InvalidText(error) => {
                write!(f, "invalid text serialized asset: {error}")
            }
            Self::ObjectNotFound { path_id } => {
                write!(f, "serialized object with path_id {path_id} not found")
            }
            Self::MissingTypeTree { path_id, class_id } => {
                write!(
                    f,
                    "serialized object {path_id} (class_id {class_id}) has no embedded type tree"
                )
            }
            Self::UnsupportedTypeTree {
                path_id,
                class_id,
                type_name,
            } => {
                write!(
                    f,
                    "serialized object {path_id} (class_id {class_id}) uses unsupported type tree node '{type_name}'"
                )
            }
            Self::FieldNotFound { type_name, field } => {
                write!(f, "field '{field}' not found on type '{type_name}'")
            }
            Self::TypeMismatch { field, expected } => {
                write!(
                    f,
                    "field '{field}' value does not match expected type '{expected}'"
                )
            }
            Self::InvalidPayload { path_id, reason } => {
                write!(f, "invalid payload for object {path_id}: {reason}")
            }
        }
    }
}

impl std::error::Error for BinarySerError {}

pub fn is_text_serialized(bytes: &[u8]) -> bool {
    let trimmed = strip_utf8_bom_and_ascii_whitespace(bytes);
    trimmed.starts_with(b"%YAML") || trimmed.starts_with(b"%TAG") || trimmed.starts_with(b"---")
}

fn strip_utf8_bom_and_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let without_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let first_non_ws = without_bom
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(without_bom.len());
    &without_bom[first_non_ws..]
}

pub fn read_serialized(bytes: &[u8]) -> Result<SerializedAsset, BinarySerError> {
    if is_text_serialized(bytes) {
        let without_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        let text = std::str::from_utf8(without_bom)
            .map_err(|_| BinarySerError::InvalidHeader("text asset is not valid UTF-8"))?;
        let stream = unity_yaml::parse(text).map_err(BinarySerError::InvalidText)?;
        Ok(SerializedAsset::Text {
            stream,
            raw: bytes.to_vec(),
        })
    } else {
        let file = read_serialized_file(bytes)?;
        Ok(SerializedAsset::Binary(file))
    }
}

pub fn read_serialized_file(bytes: &[u8]) -> Result<SerializedFile, BinarySerError> {
    let header = read_header(bytes)?;
    if header.file_size > 0 && bytes.len() < header.file_size as usize {
        return Err(BinarySerError::Truncated {
            at: header.file_size as usize,
        });
    }
    let metadata_end = HEADER_SIZE.saturating_add(header.metadata_size as usize);
    if metadata_end > bytes.len() {
        return Err(BinarySerError::Truncated { at: metadata_end });
    }

    let parsed = parse_metadata_block(&bytes[HEADER_SIZE..metadata_end], header.endianness)?;

    Ok(SerializedFile {
        header,
        metadata: SerializedFileMetadata {
            unity_version: parsed.unity_version,
            target_platform: parsed.target_platform,
            enable_type_tree: parsed.enable_type_tree,
            type_count: parsed.type_count,
        },
        objects: parsed.objects,
        raw: bytes.to_vec(),
    })
}

impl SerializedFile {
    pub fn object_bytes(&self, index: usize) -> Result<&[u8], BinarySerError> {
        let object = self
            .objects
            .get(index)
            .ok_or(BinarySerError::InvalidObjectTable(
                "object index is out of range",
            ))?;
        let start = usize::try_from(object.byte_offset)
            .map_err(|_| BinarySerError::InvalidObjectTable("object offset does not fit a file"))?;
        let end = start.saturating_add(object.byte_size as usize);
        self.raw
            .get(start..end)
            .ok_or(BinarySerError::Truncated { at: end })
    }

    pub fn object_by_path_id(&self, path_id: i64) -> Option<&SerializedObject> {
        self.objects.iter().find(|object| object.path_id == path_id)
    }

    pub fn type_tree_for_class(&self, class_id: i32) -> Result<Option<TypeTree>, BinarySerError> {
        if !self.metadata.enable_type_tree {
            return Ok(None);
        }
        let metadata_end = HEADER_SIZE.saturating_add(self.header.metadata_size as usize);
        let metadata_slice = self
            .raw
            .get(HEADER_SIZE..metadata_end)
            .ok_or(BinarySerError::Truncated { at: metadata_end })?;
        let parsed = parse_metadata_block(metadata_slice, self.header.endianness)?;
        Ok(parsed
            .types
            .into_iter()
            .find(|entry| entry.class_id == class_id)
            .and_then(|entry| entry.tree))
    }

    pub fn decode_object(&self, path_id: i64) -> Result<TypedObject, BinarySerError> {
        let (index, object) = self
            .objects
            .iter()
            .enumerate()
            .find(|(_, obj)| obj.path_id == path_id)
            .ok_or(BinarySerError::ObjectNotFound { path_id })?;
        if !self.metadata.enable_type_tree {
            return Err(BinarySerError::MissingTypeTree {
                path_id,
                class_id: object.class_id,
            });
        }
        let tree = self.type_tree_for_class(object.class_id)?.ok_or(
            BinarySerError::MissingTypeTree {
                path_id,
                class_id: object.class_id,
            },
        )?;
        if tree.nodes.is_empty() {
            return Err(BinarySerError::MissingTypeTree {
                path_id,
                class_id: object.class_id,
            });
        }
        validate_type_tree(&tree, path_id, object.class_id)?;
        let payload = self.object_bytes(index)?;
        let mut cursor = PayloadCursor::new(payload, self.header.endianness, path_id);
        let root_value = decode_node(&tree.nodes, 0, &mut cursor)?;
        let (type_name, fields) = match root_value {
            TypedValue::Struct { type_name, fields } => (type_name, fields),
            other => (
                tree.nodes[0].type_name.clone(),
                vec![(tree.nodes[0].name.clone(), other)],
            ),
        };
        Ok(TypedObject {
            path_id,
            class_id: object.class_id,
            type_name,
            fields,
        })
    }

    pub fn encode_object(&self, typed: &TypedObject) -> Result<Vec<u8>, BinarySerError> {
        if !self.metadata.enable_type_tree {
            return Err(BinarySerError::MissingTypeTree {
                path_id: typed.path_id,
                class_id: typed.class_id,
            });
        }
        let tree = self.type_tree_for_class(typed.class_id)?.ok_or(
            BinarySerError::MissingTypeTree {
                path_id: typed.path_id,
                class_id: typed.class_id,
            },
        )?;
        if tree.nodes.is_empty() {
            return Err(BinarySerError::MissingTypeTree {
                path_id: typed.path_id,
                class_id: typed.class_id,
            });
        }
        validate_type_tree(&tree, typed.path_id, typed.class_id)?;
        let root_struct = TypedValue::Struct {
            type_name: typed.type_name.clone(),
            fields: typed.fields.clone(),
        };
        let mut out = Vec::new();
        encode_node(&tree.nodes, 0, &root_struct, self.header.endianness, &mut out)?;
        Ok(out)
    }

    pub fn update_object(
        &mut self,
        path_id: i64,
        typed: &TypedObject,
    ) -> Result<(), BinarySerError> {
        let new_payload = self.encode_object(typed)?;
        self.replace_object_bytes(path_id, &new_payload)
    }

    pub fn replace_object_bytes(
        &mut self,
        path_id: i64,
        new_payload: &[u8],
    ) -> Result<(), BinarySerError> {
        let target_index = self
            .objects
            .iter()
            .position(|obj| obj.path_id == path_id)
            .ok_or(BinarySerError::ObjectNotFound { path_id })?;

        let metadata_end = HEADER_SIZE.saturating_add(self.header.metadata_size as usize);
        let metadata_slice = self
            .raw
            .get(HEADER_SIZE..metadata_end)
            .ok_or(BinarySerError::Truncated { at: metadata_end })?;
        let parsed_meta = parse_metadata_block(metadata_slice, self.header.endianness)?;
        let table_start_in_raw = HEADER_SIZE.saturating_add(parsed_meta.object_table_offset);

        let target_obj = &self.objects[target_index];
        let old_start = usize::try_from(target_obj.byte_offset)
            .map_err(|_| BinarySerError::InvalidObjectTable("object offset does not fit a file"))?;
        let old_size = target_obj.byte_size as usize;
        let old_end = old_start.saturating_add(old_size);
        if old_end > self.raw.len() {
            return Err(BinarySerError::Truncated { at: old_end });
        }

        let new_size_u32 = u32::try_from(new_payload.len())
            .map_err(|_| BinarySerError::InvalidObjectTable("new payload exceeds u32 size"))?;
        let old_start_u64 = target_obj.byte_offset;

        for obj in &mut self.objects {
            if obj.path_id == path_id {
                obj.byte_size = new_size_u32;
            } else if obj.byte_offset > old_start_u64 {
                if new_payload.len() >= old_size {
                    let delta = (new_payload.len() - old_size) as u64;
                    obj.byte_offset = obj.byte_offset.saturating_add(delta);
                } else {
                    let delta = (old_size - new_payload.len()) as u64;
                    obj.byte_offset = obj.byte_offset.saturating_sub(delta);
                }
            }
        }

        self.raw
            .splice(old_start..old_end, new_payload.iter().copied());

        if self.header.file_size > 0 {
            if new_payload.len() >= old_size {
                let delta = (new_payload.len() - old_size) as u32;
                self.header.file_size = self.header.file_size.saturating_add(delta);
            } else {
                let delta = (old_size - new_payload.len()) as u32;
                self.header.file_size = self.header.file_size.saturating_sub(delta);
            }
            self.raw[4..8].copy_from_slice(&self.header.file_size.to_be_bytes());
        }

        let endian = self.header.endianness;
        for (idx, obj) in self.objects.iter().enumerate() {
            let entry_offset = table_start_in_raw.saturating_add(idx * OBJECT_ENTRY_SIZE);
            let offset_pos = entry_offset + 12;
            let size_pos = entry_offset + 20;
            write_u64(&mut self.raw[offset_pos..offset_pos + 8], endian, obj.byte_offset);
            write_u32(&mut self.raw[size_pos..size_pos + 4], endian, obj.byte_size);
        }

        Ok(())
    }

    pub fn write_back(&self) -> Vec<u8> {
        self.raw.clone()
    }
}

struct ParsedTypeEntry {
    class_id: i32,
    tree: Option<TypeTree>,
}

struct ParsedMetadataBlock {
    unity_version: String,
    target_platform: i32,
    enable_type_tree: bool,
    type_count: u32,
    types: Vec<ParsedTypeEntry>,
    objects: Vec<SerializedObject>,
    object_table_offset: usize,
}

fn parse_metadata_block(
    metadata_bytes: &[u8],
    endianness: Endianness,
) -> Result<ParsedMetadataBlock, BinarySerError> {
    let mut cursor = Cursor::new(metadata_bytes, endianness);
    let unity_version = read_unity_version(&mut cursor)?;
    let target_platform = cursor.i32()?;
    let enable_type_tree = match cursor.u8()? {
        0 => false,
        1 => true,
        _ => {
            return Err(BinarySerError::InvalidHeader(
                "enable_type_tree must be 0 or 1",
            ))
        }
    };
    let type_count = cursor.u32()?;
    let (types, objects, object_table_offset) =
        read_types_and_object_table(&mut cursor, type_count, enable_type_tree)?;
    Ok(ParsedMetadataBlock {
        unity_version,
        target_platform,
        enable_type_tree,
        type_count,
        types,
        objects,
        object_table_offset,
    })
}

fn read_header(bytes: &[u8]) -> Result<SerializedFileHeader, BinarySerError> {
    if bytes.len() < HEADER_SIZE {
        return Err(BinarySerError::Truncated { at: HEADER_SIZE });
    }
    let metadata_size = read_be_u32(bytes, 0);
    let file_size = read_be_u32(bytes, 4);
    let version = read_be_u32(bytes, 8);
    let data_offset = read_be_u32(bytes, 12);
    if version != SUPPORTED_VERSION {
        return Err(BinarySerError::UnsupportedVersion {
            found: version,
            supported: SUPPORTED_VERSION,
        });
    }
    let endianness = match bytes[16] {
        0 => Endianness::Little,
        1 => Endianness::Big,
        _ => return Err(BinarySerError::UnsupportedEndianness),
    };
    Ok(SerializedFileHeader {
        metadata_size,
        file_size,
        version,
        data_offset,
        endianness,
        header_size: HEADER_SIZE as u32,
    })
}

fn read_be_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn write_u32(target: &mut [u8], endian: Endianness, value: u32) {
    let bytes = match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    };
    target.copy_from_slice(&bytes);
}

fn write_u64(target: &mut [u8], endian: Endianness, value: u64) {
    let bytes = match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    };
    target.copy_from_slice(&bytes);
}

fn read_unity_version(cursor: &mut Cursor<'_>) -> Result<String, BinarySerError> {
    let saved = cursor.pos;
    if let Ok(length) = cursor.u32() {
        let len = length as usize;
        if len > 0 && len <= 64 && len <= cursor.remaining() {
            let raw = cursor.take(len)?;
            if !raw.contains(&0) {
                if let Ok(version) = String::from_utf8(raw.to_vec()) {
                    if version
                        .bytes()
                        .all(|byte| (0x20..=0x7E).contains(&byte))
                    {
                        return Ok(version);
                    }
                }
            }
        }
    }
    cursor.pos = saved;
    let remaining = &cursor.bytes[cursor.pos..];
    if let Some(nul_offset) = remaining.iter().position(|byte| *byte == 0) {
        if nul_offset > 0 && nul_offset <= 64 {
            let raw = cursor.take(nul_offset)?;
            let _ = cursor.u8()?;
            return String::from_utf8(raw.to_vec())
                .map_err(|_| BinarySerError::InvalidHeader("unity_version is not valid UTF-8"));
        }
    }
    cursor.pos = saved;
    let length = cursor.u32()? as usize;
    if length > cursor.remaining() {
        return Err(BinarySerError::InvalidHeader(
            "unity_version length exceeds the metadata block",
        ));
    }
    let raw = cursor.take(length)?;
    String::from_utf8(raw.to_vec())
        .map_err(|_| BinarySerError::InvalidHeader("unity_version is not valid UTF-8"))
}

fn read_types_and_object_table(
    cursor: &mut Cursor<'_>,
    type_count: u32,
    enable_type_tree: bool,
) -> Result<(Vec<ParsedTypeEntry>, Vec<SerializedObject>, usize), BinarySerError> {
    let saved = cursor.pos;
    for with_deps in [false, true] {
        cursor.pos = saved;
        cursor.object_table = false;
        if let Ok(types) =
            read_unity_v22_type_table(cursor, type_count, enable_type_tree, with_deps)
        {
            let after_types = cursor.pos;
            if let Ok((objects, table_offset)) = read_unity_v22_objects_from_type_end(cursor) {
                if cursor.remaining() == 0 {
                    return Ok((types, objects, table_offset));
                }
            }
            cursor.pos = after_types;
            if let Ok(object_count) = cursor.u32() {
                let table_offset = cursor.pos;
                if let Ok(objects) = read_object_table(cursor, object_count) {
                    if cursor.remaining() == 0 {
                        return Ok((types, objects, table_offset));
                    }
                }
            }
        }
    }

    cursor.pos = saved;
    cursor.object_table = false;
    skip_type_region(cursor)?;
    let object_count = cursor.u32()?;
    let table_offset = cursor.pos;
    let objects = read_object_table(cursor, object_count)?;
    Ok((Vec::new(), objects, table_offset))
}

fn read_unity_v22_type_table(
    cursor: &mut Cursor<'_>,
    type_count: u32,
    enable_type_tree: bool,
    with_type_dependencies: bool,
) -> Result<Vec<ParsedTypeEntry>, BinarySerError> {
    let mut entries = Vec::with_capacity(type_count as usize);
    for _ in 0..type_count {
        let class_id = cursor.i32()?;
        let is_stripped_type = cursor.u8()?;
        if is_stripped_type > 1 {
            return Err(BinarySerError::InvalidHeader(
                "is_stripped_type must be 0 or 1",
            ));
        }
        let _script_type_index = cursor.i16()?;
        if class_id == MONOBEHAVIOUR_CLASS_ID || class_id < 0 {
            let _script_id = cursor.take(TYPE_HASH_SIZE)?;
        }
        let _old_type_hash = cursor.take(TYPE_HASH_SIZE)?;
        let tree = if enable_type_tree {
            Some(read_v22_type_tree_blob(cursor, with_type_dependencies)?)
        } else {
            None
        };
        entries.push(ParsedTypeEntry { class_id, tree });
    }
    Ok(entries)
}

fn read_v22_type_tree_blob(
    cursor: &mut Cursor<'_>,
    with_type_dependencies: bool,
) -> Result<TypeTree, BinarySerError> {
    let node_count = cursor.i32()?;
    let string_buffer_size = cursor.i32()?;
    if node_count < 0 || string_buffer_size < 0 {
        return Err(BinarySerError::InvalidHeader(
            "negative type tree node or string buffer size",
        ));
    }
    let nodes_bytes = (node_count as usize).saturating_mul(TYPE_TREE_NODE_SIZE);
    let total_blob = nodes_bytes.saturating_add(string_buffer_size as usize);
    if total_blob > cursor.remaining() {
        return Err(BinarySerError::InvalidHeader(
            "type tree blob exceeds metadata block",
        ));
    }
    let raw_nodes = cursor.take(nodes_bytes)?;
    let string_buf = cursor.take(string_buffer_size as usize)?;
    if with_type_dependencies {
        let dep_count = cursor.i32()?;
        if dep_count < 0 {
            return Err(BinarySerError::InvalidHeader(
                "negative type dependency count",
            ));
        }
        let dep_bytes = (dep_count as usize).saturating_mul(4);
        if dep_bytes > cursor.remaining() {
            return Err(BinarySerError::InvalidHeader(
                "type dependencies exceed metadata block",
            ));
        }
        let _ = cursor.take(dep_bytes)?;
    }

    let mut node_cursor = Cursor::new(raw_nodes, cursor.endian);
    let mut nodes = Vec::with_capacity(node_count as usize);
    for _ in 0..node_count {
        let version = node_cursor.i16()?;
        let level = node_cursor.u8()?;
        let type_flags = node_cursor.u8()?;
        let type_str_offset = node_cursor.u32()?;
        let name_str_offset = node_cursor.u32()?;
        let byte_size = node_cursor.i32()?;
        let index = node_cursor.i32()?;
        let meta_flag = node_cursor.i32()?;
        let _ref_type_hash = node_cursor.u64()?;
        let type_name = resolve_type_tree_string(string_buf, type_str_offset);
        let name = resolve_type_tree_string(string_buf, name_str_offset);
        nodes.push(TypeTreeNode {
            version,
            level,
            type_flags,
            type_name,
            name,
            byte_size,
            index,
            meta_flag,
        });
    }

    Ok(TypeTree { nodes })
}

fn resolve_type_tree_string(string_buf: &[u8], offset: u32) -> String {
    if (offset & COMMON_STRING_OFFSET_FLAG) != 0 {
        let common_offset = (offset & !COMMON_STRING_OFFSET_FLAG) as usize;
        if let Some(common) = lookup_common_string(common_offset) {
            return common.to_string();
        }
    }
    let start = offset as usize;
    if start >= string_buf.len() {
        return String::new();
    }
    let slice = &string_buf[start..];
    let end = slice
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).into_owned()
}

fn lookup_common_string(offset: usize) -> Option<&'static str> {
    match offset {
        0 => Some("AABB"),
        5 => Some("AnimationClip"),
        19 => Some("AnimationCurve"),
        34 => Some("AnimationState"),
        49 => Some("Array"),
        55 => Some("Base"),
        60 => Some("BitField"),
        69 => Some("bitset"),
        76 => Some("bool"),
        81 => Some("char"),
        86 => Some("ColorRGBA"),
        96 => Some("Component"),
        106 => Some("data"),
        111 => Some("deque"),
        117 => Some("double"),
        124 => Some("dynamic_array"),
        138 => Some("FastPropertyName"),
        155 => Some("first"),
        161 => Some("float"),
        167 => Some("Font"),
        172 => Some("GameObject"),
        183 => Some("Generic Mono"),
        196 => Some("GradientNEW"),
        208 => Some("GUID"),
        213 => Some("GUIStyle"),
        222 => Some("int"),
        226 => Some("list"),
        231 => Some("long long"),
        241 => Some("map"),
        245 => Some("Matrix4x4f"),
        256 => Some("MdFour"),
        263 => Some("MonoBehaviour"),
        277 => Some("MonoScript"),
        288 => Some("m_ByteSize"),
        299 => Some("m_Curve"),
        307 => Some("m_EditorClassIdentifier"),
        331 => Some("m_EditorHideFlags"),
        349 => Some("m_Enabled"),
        359 => Some("m_ExtensionPtr"),
        374 => Some("m_GameObject"),
        387 => Some("m_Index"),
        395 => Some("m_IsArray"),
        405 => Some("m_IsStatic"),
        416 => Some("m_MetaFlag"),
        427 => Some("m_Name"),
        434 => Some("m_ObjectHideFlags"),
        452 => Some("m_PrefabInternal"),
        469 => Some("m_PrefabParentObject"),
        490 => Some("m_Script"),
        499 => Some("m_StaticEditorFlags"),
        519 => Some("m_Type"),
        526 => Some("m_Version"),
        536 => Some("Object"),
        543 => Some("pair"),
        548 => Some("PPtr<Component>"),
        564 => Some("PPtr<GameObject>"),
        581 => Some("PPtr<Material>"),
        596 => Some("PPtr<MonoBehaviour>"),
        616 => Some("PPtr<MonoScript>"),
        633 => Some("PPtr<Object>"),
        646 => Some("PPtr<Prefab>"),
        659 => Some("PPtr<Sprite>"),
        672 => Some("PPtr<TextAsset>"),
        688 => Some("PPtr<Texture>"),
        702 => Some("PPtr<Texture2D>"),
        718 => Some("PPtr<Transform>"),
        734 => Some("Prefab"),
        741 => Some("Quaternionf"),
        753 => Some("Rectf"),
        759 => Some("RectInt"),
        767 => Some("RectOffset"),
        778 => Some("second"),
        785 => Some("set"),
        789 => Some("short"),
        795 => Some("size"),
        800 => Some("SInt16"),
        807 => Some("SInt32"),
        814 => Some("SInt64"),
        821 => Some("SInt8"),
        827 => Some("staticvector"),
        840 => Some("string"),
        847 => Some("TextAsset"),
        857 => Some("TextMesh"),
        866 => Some("Texture"),
        874 => Some("Texture2D"),
        884 => Some("Transform"),
        894 => Some("TypelessData"),
        907 => Some("UInt16"),
        914 => Some("UInt32"),
        921 => Some("UInt64"),
        928 => Some("UInt8"),
        934 => Some("unsigned int"),
        947 => Some("unsigned long long"),
        966 => Some("unsigned short"),
        981 => Some("vector"),
        988 => Some("Vector2f"),
        997 => Some("Vector3f"),
        1006 => Some("Vector4f"),
        1015 => Some("m_ScriptingClassIdentifier"),
        1042 => Some("Gradient"),
        1051 => Some("Type*"),
        1057 => Some("int2_storage"),
        1070 => Some("int3_storage"),
        1083 => Some("BoundsInt"),
        1093 => Some("m_CorrespondingSourceObject"),
        1121 => Some("m_PrefabInstance"),
        1138 => Some("m_PrefabAsset"),
        1152 => Some("FileSize"),
        1161 => Some("Hash128"),
        _ => None,
    }
}

fn read_unity_v22_objects_from_type_end(
    cursor: &mut Cursor<'_>,
) -> Result<(Vec<SerializedObject>, usize), BinarySerError> {
    let object_count = cursor.u32()?;
    let after_count = cursor.pos;
    if let Ok(objects) = read_object_table(cursor, object_count) {
        if cursor.remaining() == 0 {
            return Ok((objects, after_count));
        }
    }
    cursor.pos = after_count;
    cursor.object_table = false;
    cursor.align4()?;
    let aligned_pos = cursor.pos;
    if let Ok(objects) = read_object_table(cursor, object_count) {
        if cursor.remaining() == 0 {
            return Ok((objects, aligned_pos));
        }
    }
    cursor.pos = aligned_pos;
    let objects = read_object_table(cursor, object_count)?;
    Ok((objects, aligned_pos))
}

fn skip_type_region(cursor: &mut Cursor<'_>) -> Result<(), BinarySerError> {
    let length = cursor.u32()? as usize;
    if length > cursor.remaining() {
        return Err(BinarySerError::InvalidHeader(
            "type table length exceeds the metadata block",
        ));
    }
    cursor.take(length).map(|_| ())
}

fn read_object_table(
    cursor: &mut Cursor<'_>,
    object_count: u32,
) -> Result<Vec<SerializedObject>, BinarySerError> {
    let needed = (object_count as usize).saturating_mul(OBJECT_ENTRY_SIZE);
    if needed > cursor.remaining() {
        return Err(BinarySerError::InvalidObjectTable(
            "object table is shorter than its declared entry count",
        ));
    }
    cursor.object_table = true;
    let mut objects = Vec::with_capacity(object_count as usize);
    for _ in 0..object_count {
        let path_id = cursor.i64()?;
        let class_id = cursor.i32()?;
        let byte_offset = cursor.u64()?;
        let byte_size = cursor.u32()?;
        let is_stripped = match cursor.u8()? {
            0 => false,
            1 => true,
            _ => {
                return Err(BinarySerError::InvalidObjectTable(
                    "is_stripped must be 0 or 1",
                ))
            }
        };
        objects.push(SerializedObject {
            path_id,
            class_id,
            byte_offset,
            byte_size,
            is_stripped,
        });
    }
    Ok(objects)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimitiveKind {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

fn classify_primitive(type_name: &str) -> Option<PrimitiveKind> {
    match type_name {
        "bool" => Some(PrimitiveKind::Bool),
        "SInt8" | "sbyte" => Some(PrimitiveKind::I8),
        "UInt8" | "char" | "byte" => Some(PrimitiveKind::U8),
        "SInt16" | "short" => Some(PrimitiveKind::I16),
        "UInt16" | "unsigned short" => Some(PrimitiveKind::U16),
        "int" | "SInt32" | "Type*" => Some(PrimitiveKind::I32),
        "unsigned int" | "UInt32" => Some(PrimitiveKind::U32),
        "SInt64" | "long long" => Some(PrimitiveKind::I64),
        "UInt64" | "unsigned long long" | "FileSize" => Some(PrimitiveKind::U64),
        "float" => Some(PrimitiveKind::F32),
        "double" => Some(PrimitiveKind::F64),
        _ => None,
    }
}

fn direct_children(nodes: &[TypeTreeNode], parent_idx: usize) -> Vec<usize> {
    let parent_level = nodes[parent_idx].level;
    let end = subtree_end(nodes, parent_idx);
    let mut children = Vec::new();
    let mut idx = parent_idx + 1;
    while idx < end {
        if nodes[idx].level == parent_level + 1 {
            children.push(idx);
        }
        idx += 1;
    }
    children
}

fn subtree_end(nodes: &[TypeTreeNode], parent_idx: usize) -> usize {
    let parent_level = nodes[parent_idx].level;
    let mut idx = parent_idx + 1;
    while idx < nodes.len() && nodes[idx].level > parent_level {
        idx += 1;
    }
    idx
}

fn is_string_node(nodes: &[TypeTreeNode], idx: usize) -> bool {
    let node = &nodes[idx];
    if node.type_name != "string" {
        return false;
    }
    let children = direct_children(nodes, idx);
    if children.len() != 1 {
        return false;
    }
    let array_idx = children[0];
    if !nodes[array_idx].is_array() {
        return false;
    }
    let array_children = direct_children(nodes, array_idx);
    array_children.len() == 2
        && matches!(nodes[array_children[1]].type_name.as_str(), "char" | "UInt8" | "byte")
}

fn is_array_container_node(nodes: &[TypeTreeNode], idx: usize) -> Option<usize> {
    let children = direct_children(nodes, idx);
    if children.len() == 1 && nodes[children[0]].is_array() {
        let array_idx = children[0];
        let array_children = direct_children(nodes, array_idx);
        if array_children.len() == 2 {
            return Some(array_idx);
        }
    }
    None
}

fn validate_type_tree(
    tree: &TypeTree,
    path_id: i64,
    class_id: i32,
) -> Result<(), BinarySerError> {
    if tree.nodes.is_empty() {
        return Err(BinarySerError::MissingTypeTree { path_id, class_id });
    }
    validate_node(&tree.nodes, 0, path_id, class_id)
}

fn validate_node(
    nodes: &[TypeTreeNode],
    idx: usize,
    path_id: i64,
    class_id: i32,
) -> Result<(), BinarySerError> {
    let node = &nodes[idx];
    if node.type_name.is_empty() {
        return Err(BinarySerError::UnsupportedTypeTree {
            path_id,
            class_id,
            type_name: "<empty>".to_string(),
        });
    }
    if is_string_node(nodes, idx) {
        return Ok(());
    }
    if let Some(array_idx) = is_array_container_node(nodes, idx) {
        let array_children = direct_children(nodes, array_idx);
        let size_idx = array_children[0];
        let elem_idx = array_children[1];
        if classify_primitive(&nodes[size_idx].type_name) != Some(PrimitiveKind::I32)
            && classify_primitive(&nodes[size_idx].type_name) != Some(PrimitiveKind::U32)
        {
            return Err(BinarySerError::UnsupportedTypeTree {
                path_id,
                class_id,
                type_name: nodes[size_idx].type_name.clone(),
            });
        }
        return validate_node(nodes, elem_idx, path_id, class_id);
    }
    if node.is_array() {
        let array_children = direct_children(nodes, idx);
        if array_children.len() != 2 {
            return Err(BinarySerError::UnsupportedTypeTree {
                path_id,
                class_id,
                type_name: node.type_name.clone(),
            });
        }
        return validate_node(nodes, array_children[1], path_id, class_id);
    }
    let children = direct_children(nodes, idx);
    if children.is_empty() {
        if classify_primitive(&node.type_name).is_none() {
            return Err(BinarySerError::UnsupportedTypeTree {
                path_id,
                class_id,
                type_name: node.type_name.clone(),
            });
        }
        return Ok(());
    }
    for child_idx in children {
        validate_node(nodes, child_idx, path_id, class_id)?;
    }
    Ok(())
}

fn decode_node(
    nodes: &[TypeTreeNode],
    idx: usize,
    cursor: &mut PayloadCursor<'_>,
) -> Result<TypedValue, BinarySerError> {
    let node = &nodes[idx];
    let value = if is_string_node(nodes, idx) {
        let array_idx = direct_children(nodes, idx)[0];
        let len_i32 = cursor.i32()?;
        if len_i32 < 0 {
            return Err(BinarySerError::InvalidPayload {
                path_id: cursor.path_id,
                reason: "negative string length",
            });
        }
        let raw = cursor.take(len_i32 as usize)?;
        let text = String::from_utf8(raw.to_vec()).map_err(|_| BinarySerError::InvalidPayload {
            path_id: cursor.path_id,
            reason: "string payload is not valid UTF-8",
        })?;
        if nodes[array_idx].aligns_after() {
            cursor.align4()?;
        }
        TypedValue::String(text)
    } else if let Some(array_idx) = is_array_container_node(nodes, idx) {
        let array_children = direct_children(nodes, array_idx);
        let elem_idx = array_children[1];
        let count_i32 = cursor.i32()?;
        if count_i32 < 0 {
            return Err(BinarySerError::InvalidPayload {
                path_id: cursor.path_id,
                reason: "negative array element count",
            });
        }
        let count = count_i32 as usize;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(decode_node(nodes, elem_idx, cursor)?);
        }
        if nodes[array_idx].aligns_after() {
            cursor.align4()?;
        }
        TypedValue::Array(items)
    } else if node.is_array() {
        let array_children = direct_children(nodes, idx);
        let elem_idx = array_children[1];
        let count_i32 = cursor.i32()?;
        if count_i32 < 0 {
            return Err(BinarySerError::InvalidPayload {
                path_id: cursor.path_id,
                reason: "negative array element count",
            });
        }
        let count = count_i32 as usize;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(decode_node(nodes, elem_idx, cursor)?);
        }
        TypedValue::Array(items)
    } else {
        let children = direct_children(nodes, idx);
        if children.is_empty() {
            let kind = classify_primitive(&node.type_name).ok_or_else(|| {
                BinarySerError::UnsupportedTypeTree {
                    path_id: cursor.path_id,
                    class_id: 0,
                    type_name: node.type_name.clone(),
                }
            })?;
            match kind {
                PrimitiveKind::Bool => TypedValue::Bool(cursor.u8()? != 0),
                PrimitiveKind::I8 => TypedValue::I8(cursor.u8()? as i8),
                PrimitiveKind::U8 => TypedValue::U8(cursor.u8()?),
                PrimitiveKind::I16 => TypedValue::I16(cursor.i16()?),
                PrimitiveKind::U16 => TypedValue::U16(cursor.u16()?),
                PrimitiveKind::I32 => TypedValue::I32(cursor.i32()?),
                PrimitiveKind::U32 => TypedValue::U32(cursor.u32()?),
                PrimitiveKind::I64 => TypedValue::I64(cursor.i64()?),
                PrimitiveKind::U64 => TypedValue::U64(cursor.u64()?),
                PrimitiveKind::F32 => TypedValue::F32(cursor.f32()?),
                PrimitiveKind::F64 => TypedValue::F64(cursor.f64()?),
            }
        } else {
            let mut fields = Vec::with_capacity(children.len());
            for child_idx in children {
                let field_name = nodes[child_idx].name.clone();
                let field_val = decode_node(nodes, child_idx, cursor)?;
                fields.push((field_name, field_val));
            }
            TypedValue::Struct {
                type_name: node.type_name.clone(),
                fields,
            }
        }
    };

    if node.aligns_after() {
        cursor.align4()?;
    }
    Ok(value)
}

fn encode_node(
    nodes: &[TypeTreeNode],
    idx: usize,
    value: &TypedValue,
    endian: Endianness,
    out: &mut Vec<u8>,
) -> Result<(), BinarySerError> {
    let node = &nodes[idx];
    if is_string_node(nodes, idx) {
        let array_idx = direct_children(nodes, idx)[0];
        let text = match value {
            TypedValue::String(s) => s.as_str(),
            _ => {
                return Err(BinarySerError::TypeMismatch {
                    field: node.name.clone(),
                    expected: "string".to_string(),
                })
            }
        };
        let len = i32::try_from(text.len()).map_err(|_| BinarySerError::TypeMismatch {
            field: node.name.clone(),
            expected: "string fits in i32".to_string(),
        })?;
        push_i32(out, endian, len);
        out.extend_from_slice(text.as_bytes());
        if nodes[array_idx].aligns_after() {
            align4_vec(out);
        }
    } else if let Some(array_idx) = is_array_container_node(nodes, idx) {
        let array_children = direct_children(nodes, array_idx);
        let elem_idx = array_children[1];
        let items = match value {
            TypedValue::Array(items) => items,
            _ => {
                return Err(BinarySerError::TypeMismatch {
                    field: node.name.clone(),
                    expected: node.type_name.clone(),
                })
            }
        };
        let count = i32::try_from(items.len()).map_err(|_| BinarySerError::TypeMismatch {
            field: node.name.clone(),
            expected: "array length fits in i32".to_string(),
        })?;
        push_i32(out, endian, count);
        for item in items {
            encode_node(nodes, elem_idx, item, endian, out)?;
        }
        if nodes[array_idx].aligns_after() {
            align4_vec(out);
        }
    } else if node.is_array() {
        let array_children = direct_children(nodes, idx);
        let elem_idx = array_children[1];
        let items = match value {
            TypedValue::Array(items) => items,
            _ => {
                return Err(BinarySerError::TypeMismatch {
                    field: node.name.clone(),
                    expected: "Array".to_string(),
                })
            }
        };
        let count = i32::try_from(items.len()).map_err(|_| BinarySerError::TypeMismatch {
            field: node.name.clone(),
            expected: "array length fits in i32".to_string(),
        })?;
        push_i32(out, endian, count);
        for item in items {
            encode_node(nodes, elem_idx, item, endian, out)?;
        }
    } else {
        let children = direct_children(nodes, idx);
        if children.is_empty() {
            let kind = classify_primitive(&node.type_name).ok_or_else(|| {
                BinarySerError::UnsupportedTypeTree {
                    path_id: 0,
                    class_id: 0,
                    type_name: node.type_name.clone(),
                }
            })?;
            encode_primitive(kind, value, &node.name, &node.type_name, endian, out)?;
        } else {
            let fields = match value {
                TypedValue::Struct { fields, .. } => fields,
                _ => {
                    return Err(BinarySerError::TypeMismatch {
                        field: node.name.clone(),
                        expected: node.type_name.clone(),
                    })
                }
            };
            for child_idx in children {
                let child_name = &nodes[child_idx].name;
                let (_, child_val) = fields
                    .iter()
                    .find(|(name, _)| name == child_name)
                    .ok_or_else(|| BinarySerError::FieldNotFound {
                        type_name: node.type_name.clone(),
                        field: child_name.clone(),
                    })?;
                encode_node(nodes, child_idx, child_val, endian, out)?;
            }
        }
    }

    if node.aligns_after() {
        align4_vec(out);
    }
    Ok(())
}

fn encode_primitive(
    kind: PrimitiveKind,
    value: &TypedValue,
    field_name: &str,
    type_name: &str,
    endian: Endianness,
    out: &mut Vec<u8>,
) -> Result<(), BinarySerError> {
    let mismatch = || BinarySerError::TypeMismatch {
        field: field_name.to_string(),
        expected: type_name.to_string(),
    };
    match kind {
        PrimitiveKind::Bool => {
            let b = value.as_bool().ok_or_else(mismatch)?;
            out.push(u8::from(b));
        }
        PrimitiveKind::I8 => {
            let v = match value {
                TypedValue::I8(v) => *v,
                _ => return Err(mismatch()),
            };
            out.push(v as u8);
        }
        PrimitiveKind::U8 => {
            let v = match value {
                TypedValue::U8(v) => *v,
                _ => return Err(mismatch()),
            };
            out.push(v);
        }
        PrimitiveKind::I16 => {
            let v = match value {
                TypedValue::I16(v) => *v,
                _ => return Err(mismatch()),
            };
            push_i16(out, endian, v);
        }
        PrimitiveKind::U16 => {
            let v = match value {
                TypedValue::U16(v) => *v,
                _ => return Err(mismatch()),
            };
            push_u16(out, endian, v);
        }
        PrimitiveKind::I32 => {
            let v = match value {
                TypedValue::I32(v) => *v,
                _ => return Err(mismatch()),
            };
            push_i32(out, endian, v);
        }
        PrimitiveKind::U32 => {
            let v = match value {
                TypedValue::U32(v) => *v,
                _ => return Err(mismatch()),
            };
            push_u32(out, endian, v);
        }
        PrimitiveKind::I64 => {
            let v = match value {
                TypedValue::I64(v) => *v,
                _ => return Err(mismatch()),
            };
            push_i64(out, endian, v);
        }
        PrimitiveKind::U64 => {
            let v = match value {
                TypedValue::U64(v) => *v,
                _ => return Err(mismatch()),
            };
            push_u64(out, endian, v);
        }
        PrimitiveKind::F32 => {
            let v = match value {
                TypedValue::F32(v) => *v,
                _ => return Err(mismatch()),
            };
            push_f32(out, endian, v);
        }
        PrimitiveKind::F64 => {
            let v = match value {
                TypedValue::F64(v) => *v,
                _ => return Err(mismatch()),
            };
            push_f64(out, endian, v);
        }
    }
    Ok(())
}

fn align4_vec(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn push_i16(out: &mut Vec<u8>, endian: Endianness, value: i16) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_u16(out: &mut Vec<u8>, endian: Endianness, value: u16) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_i32(out: &mut Vec<u8>, endian: Endianness, value: i32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_u32(out: &mut Vec<u8>, endian: Endianness, value: u32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_i64(out: &mut Vec<u8>, endian: Endianness, value: i64) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_u64(out: &mut Vec<u8>, endian: Endianness, value: u64) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_f32(out: &mut Vec<u8>, endian: Endianness, value: f32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn push_f64(out: &mut Vec<u8>, endian: Endianness, value: f64) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

struct PayloadCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    endian: Endianness,
    path_id: i64,
}

impl<'a> PayloadCursor<'a> {
    fn new(bytes: &'a [u8], endian: Endianness, path_id: i64) -> Self {
        Self {
            bytes,
            pos: 0,
            endian,
            path_id,
        }
    }

    fn align4(&mut self) -> Result<(), BinarySerError> {
        let aligned = (self.pos.saturating_add(3)) & !3;
        if aligned > self.bytes.len() {
            return Err(BinarySerError::InvalidPayload {
                path_id: self.path_id,
                reason: "payload truncated during 4-byte alignment",
            });
        }
        self.pos = aligned;
        Ok(())
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], BinarySerError> {
        let end = self.pos.saturating_add(length);
        if end > self.bytes.len() {
            return Err(BinarySerError::InvalidPayload {
                path_id: self.path_id,
                reason: "payload ends before declared field contents",
            });
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, BinarySerError> {
        Ok(self.take(1)?[0])
    }

    fn i16(&mut self) -> Result<i16, BinarySerError> {
        let bytes = self.take(2)?;
        let mut value = [0u8; 2];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i16::from_be_bytes(value),
            Endianness::Little => i16::from_le_bytes(value),
        })
    }

    fn u16(&mut self) -> Result<u16, BinarySerError> {
        let bytes = self.take(2)?;
        let mut value = [0u8; 2];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => u16::from_be_bytes(value),
            Endianness::Little => u16::from_le_bytes(value),
        })
    }

    fn i32(&mut self) -> Result<i32, BinarySerError> {
        let bytes = self.take(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i32::from_be_bytes(value),
            Endianness::Little => i32::from_le_bytes(value),
        })
    }

    fn u32(&mut self) -> Result<u32, BinarySerError> {
        let bytes = self.take(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => u32::from_be_bytes(value),
            Endianness::Little => u32::from_le_bytes(value),
        })
    }

    fn i64(&mut self) -> Result<i64, BinarySerError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i64::from_be_bytes(value),
            Endianness::Little => i64::from_le_bytes(value),
        })
    }

    fn u64(&mut self) -> Result<u64, BinarySerError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => u64::from_be_bytes(value),
            Endianness::Little => u64::from_le_bytes(value),
        })
    }

    fn f32(&mut self) -> Result<f32, BinarySerError> {
        let bytes = self.take(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => f32::from_be_bytes(value),
            Endianness::Little => f32::from_le_bytes(value),
        })
    }

    fn f64(&mut self) -> Result<f64, BinarySerError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => f64::from_be_bytes(value),
            Endianness::Little => f64::from_le_bytes(value),
        })
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    endian: Endianness,
    object_table: bool,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], endian: Endianness) -> Self {
        Cursor {
            bytes,
            pos: 0,
            endian,
            object_table: false,
        }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn align4(&mut self) -> Result<(), BinarySerError> {
        let aligned = (self.pos.saturating_add(3)) & !3;
        if aligned > self.bytes.len() {
            return Err(self.overrun());
        }
        self.pos = aligned;
        Ok(())
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], BinarySerError> {
        let end = self.pos.saturating_add(length);
        if end > self.bytes.len() {
            return Err(self.overrun());
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn overrun(&self) -> BinarySerError {
        if self.object_table {
            BinarySerError::InvalidObjectTable("object table ends before its declared contents")
        } else {
            BinarySerError::InvalidHeader("metadata block ends before its declared contents")
        }
    }

    fn u8(&mut self) -> Result<u8, BinarySerError> {
        Ok(self.take(1)?[0])
    }

    fn i16(&mut self) -> Result<i16, BinarySerError> {
        let bytes = self.take(2)?;
        let mut value = [0u8; 2];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i16::from_be_bytes(value),
            Endianness::Little => i16::from_le_bytes(value),
        })
    }

    fn u32(&mut self) -> Result<u32, BinarySerError> {
        let bytes = self.take(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => u32::from_be_bytes(value),
            Endianness::Little => u32::from_le_bytes(value),
        })
    }

    fn i32(&mut self) -> Result<i32, BinarySerError> {
        let bytes = self.take(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i32::from_be_bytes(value),
            Endianness::Little => i32::from_le_bytes(value),
        })
    }

    fn u64(&mut self) -> Result<u64, BinarySerError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => u64::from_be_bytes(value),
            Endianness::Little => u64::from_le_bytes(value),
        })
    }

    fn i64(&mut self) -> Result<i64, BinarySerError> {
        let bytes = self.take(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(match self.endian {
            Endianness::Big => i64::from_be_bytes(value),
            Endianness::Little => i64::from_le_bytes(value),
        })
    }
}
