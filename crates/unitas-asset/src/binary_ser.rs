use crate::unity_yaml::{self, Stream, YamlError};
use std::fmt;

pub const SUPPORTED_VERSION: u32 = 22;

const HEADER_SIZE: usize = 20;
const OBJECT_ENTRY_SIZE: usize = 25;
const MONOBEHAVIOUR_CLASS_ID: i32 = 114;
const TYPE_HASH_SIZE: usize = 16;
const TYPE_TREE_NODE_SIZE: usize = 32;

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

    let mut cursor = Cursor::new(&bytes[HEADER_SIZE..metadata_end], header.endianness);
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
    let objects = read_types_and_object_table(&mut cursor, type_count, enable_type_tree)?;

    Ok(SerializedFile {
        header,
        metadata: SerializedFileMetadata {
            unity_version,
            target_platform,
            enable_type_tree,
            type_count,
        },
        objects,
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

    pub fn write_back(&self) -> Vec<u8> {
        self.raw.clone()
    }
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
) -> Result<Vec<SerializedObject>, BinarySerError> {
    let saved = cursor.pos;
    if skip_unity_v22_type_table(cursor, type_count, enable_type_tree, false).is_ok() {
        let after_types = cursor.pos;
        if let Ok(objects) = read_unity_v22_objects_from_type_end(cursor) {
            if cursor.remaining() == 0 {
                return Ok(objects);
            }
        }
        cursor.pos = after_types;
        if let Ok(object_count) = cursor.u32() {
            if let Ok(objects) = read_object_table(cursor, object_count) {
                if cursor.remaining() == 0 {
                    return Ok(objects);
                }
            }
        }
    }

    cursor.pos = saved;
    cursor.object_table = false;
    if skip_unity_v22_type_table(cursor, type_count, enable_type_tree, true).is_ok() {
        let after_types = cursor.pos;
        if let Ok(objects) = read_unity_v22_objects_from_type_end(cursor) {
            if cursor.remaining() == 0 {
                return Ok(objects);
            }
        }
        cursor.pos = after_types;
        if let Ok(object_count) = cursor.u32() {
            if let Ok(objects) = read_object_table(cursor, object_count) {
                if cursor.remaining() == 0 {
                    return Ok(objects);
                }
            }
        }
    }

    cursor.pos = saved;
    cursor.object_table = false;
    skip_type_region(cursor)?;
    let object_count = cursor.u32()?;
    read_object_table(cursor, object_count)
}

fn skip_unity_v22_type_table(
    cursor: &mut Cursor<'_>,
    type_count: u32,
    enable_type_tree: bool,
    with_type_dependencies: bool,
) -> Result<(), BinarySerError> {
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
        if enable_type_tree {
            skip_v22_type_tree_blob(cursor, with_type_dependencies)?;
        }
    }
    Ok(())
}

fn skip_v22_type_tree_blob(
    cursor: &mut Cursor<'_>,
    with_type_dependencies: bool,
) -> Result<(), BinarySerError> {
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
    let _ = cursor.take(total_blob)?;
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
    Ok(())
}

fn read_unity_v22_objects_from_type_end(
    cursor: &mut Cursor<'_>,
) -> Result<Vec<SerializedObject>, BinarySerError> {
    let object_count = cursor.u32()?;
    let after_count = cursor.pos;
    if let Ok(objects) = read_object_table(cursor, object_count) {
        if cursor.remaining() == 0 {
            return Ok(objects);
        }
    }
    cursor.pos = after_count;
    cursor.object_table = false;
    cursor.align4()?;
    let aligned_pos = cursor.pos;
    if let Ok(objects) = read_object_table(cursor, object_count) {
        if cursor.remaining() == 0 {
            return Ok(objects);
        }
    }
    cursor.pos = aligned_pos;
    read_object_table(cursor, object_count)
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
