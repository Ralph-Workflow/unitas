//! Reader for Unity's binary `SerializedFile` container.
//!
//! Only format version 22 (written by Unity 2022.3 LTS) is accepted; any other
//! version is reported as [`BinarySerError::UnsupportedVersion`] instead of
//! being parsed with a guessed layout.
//!
//! # Byte layout
//!
//! The 20-byte header is always big-endian. The metadata block and the object
//! table that follow it use the host endianness recorded in the header, the
//! way Unity writes them.
//!
//! ```text
//! header (20 bytes, big-endian)
//!   0   u32   metadata_size   byte length of the metadata block
//!   4   u32   file_size       total file length the producer wrote
//!   8   u32   version         format version (22)
//!  12   u32   data_offset     absolute offset of the first object payload
//!  16   u8    endianness      0 = little-endian host, 1 = big-endian host
//!  17   3 bytes               reserved, ignored
//!
//! metadata block (metadata_size bytes, host endianness)
//!   u32   unity_version length, then that many UTF-8 bytes
//!   i32   target_platform
//!   u8    enable_type_tree (0 or 1)
//!   u32   type_count
//!   u32   type_table_bytes  length of the opaque type table + type tree blob
//!   ...   type_table_bytes opaque bytes, not interpreted
//!   u32   object_count
//!   object_count entries, each
//!     i64 path_id
//!     i32 class_id
//!     u64 byte_offset  absolute offset of the payload in the file
//!     u32 byte_size    payload length in bytes
//!     u8  is_stripped  (0 or 1)
//! ```
//!
//! Object payloads follow the metadata block at `data_offset`. Every byte of
//! the input is kept in [`SerializedFile::raw`], so an unedited file is written
//! back byte for byte and content this reader does not interpret survives a
//! round trip.
//!
//! The type table is length-prefixed here because the type trees inside it
//! carry their own, version-specific layout. Unity writes that region without
//! a length, so a file produced by Unity itself needs a type-tree walk that
//! this reader does not implement yet.

use std::fmt;

/// The only Unity binary format version this reader accepts.
pub const SUPPORTED_VERSION: u32 = 22;

const HEADER_SIZE: usize = 20;
const OBJECT_ENTRY_SIZE: usize = 25;

/// Byte order of the metadata block and the object table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Big,
    Little,
}

/// The fixed-size header of a binary serialized file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFileHeader {
    pub metadata_size: u32,
    pub file_size: u32,
    pub version: u32,
    pub data_offset: u32,
    pub endianness: Endianness,
    /// Bytes consumed by the header block itself.
    pub header_size: u32,
}

/// The descriptive fields of the metadata block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFileMetadata {
    pub unity_version: String,
    pub target_platform: i32,
    pub enable_type_tree: bool,
    pub type_count: u32,
}

/// One entry of the object table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedObject {
    pub path_id: i64,
    pub class_id: i32,
    /// Absolute offset of the payload in the file.
    pub byte_offset: u64,
    pub byte_size: u32,
    pub is_stripped: bool,
}

/// A parsed binary serialized file with its original bytes kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFile {
    pub header: SerializedFileHeader,
    pub metadata: SerializedFileMetadata,
    pub objects: Vec<SerializedObject>,
    /// The whole original file, preserved byte for byte.
    pub raw: Vec<u8>,
}

/// Every failure this reader reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinarySerError {
    /// The file ends before the data at `at` bytes could be read.
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
        }
    }
}

impl std::error::Error for BinarySerError {}

/// Reads a binary serialized file, keeping every input byte in
/// [`SerializedFile::raw`].
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
    skip_type_region(&mut cursor)?;
    let object_count = cursor.u32()?;
    let objects = read_object_table(&mut cursor, object_count)?;

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
    /// The payload bytes of the object at `index`.
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

    /// The object with the given local file identifier, if the file has one.
    pub fn object_by_path_id(&self, path_id: i64) -> Option<&SerializedObject> {
        self.objects.iter().find(|object| object.path_id == path_id)
    }

    /// The original file bytes, byte-identical for an unedited file.
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

/// Skips the type table and its type trees, which this reader does not
/// interpret but keeps verbatim in the raw bytes.
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
