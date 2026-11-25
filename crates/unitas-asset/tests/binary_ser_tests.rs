//! Tests for the binary serialized file reader.
//!
//! The crate sets `autotests = false`, so this file is compiled into the
//! crate as a module by `src/binary_ser.rs` instead of as its own test target.

use unitas_asset::binary_ser::{
    read_serialized_file, BinarySerError, Endianness, SerializedFile, SerializedFileHeader,
    SerializedFileMetadata, SerializedObject, SUPPORTED_VERSION,
};

const UNITY_VERSION: &str = "2022.3.16f1";
const TARGET_PLATFORM: i32 = 19;
const PAYLOAD_A: &[u8] = b"payload-alpha";
const PAYLOAD_B: &[u8] = b"payload-beta";
const TYPE_REGION: &[u8] = &[0x7f, 0x01, 0x02, 0x03];
const TYPE_COUNT: u32 = 3;
const HEADER_SIZE: usize = 20;

/// Offset of the object count inside the metadata block.
const OBJECT_COUNT_OFFSET: usize =
    HEADER_SIZE + 4 + UNITY_VERSION.len() + 4 + 1 + 4 + 4 + TYPE_REGION.len();

struct Fixture {
    bytes: Vec<u8>,
    data_offset: usize,
    metadata_size: usize,
}

fn put_u32(out: &mut Vec<u8>, endian: Endianness, value: u32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn put_i32(out: &mut Vec<u8>, endian: Endianness, value: i32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn put_u64(out: &mut Vec<u8>, endian: Endianness, value: u64) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn put_i64(out: &mut Vec<u8>, endian: Endianness, value: i64) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn build_metadata(endian: Endianness, data_offset: u64) -> Vec<u8> {
    let mut meta = Vec::new();
    put_u32(&mut meta, endian, UNITY_VERSION.len() as u32);
    meta.extend_from_slice(UNITY_VERSION.as_bytes());
    put_i32(&mut meta, endian, TARGET_PLATFORM);
    meta.push(1);
    put_u32(&mut meta, endian, TYPE_COUNT);
    put_u32(&mut meta, endian, TYPE_REGION.len() as u32);
    meta.extend_from_slice(TYPE_REGION);
    put_u32(&mut meta, endian, 2);
    put_i64(&mut meta, endian, 1);
    put_i32(&mut meta, endian, 21);
    put_u64(&mut meta, endian, data_offset);
    put_u32(&mut meta, endian, PAYLOAD_A.len() as u32);
    meta.push(0);
    put_i64(&mut meta, endian, 2);
    put_i32(&mut meta, endian, 28);
    put_u64(&mut meta, endian, data_offset + PAYLOAD_A.len() as u64);
    put_u32(&mut meta, endian, PAYLOAD_B.len() as u32);
    meta.push(1);
    meta
}

/// Builds a file with the layout documented in `src/binary_ser.rs`.
fn build_file(endian: Endianness, version: u32, trailing: &[u8]) -> Fixture {
    let probe = build_metadata(endian, 0);
    let data_offset = HEADER_SIZE + probe.len();
    let metadata = build_metadata(endian, data_offset as u64);

    let mut bytes = Vec::new();
    put_u32(&mut bytes, Endianness::Big, metadata.len() as u32);
    put_u32(&mut bytes, Endianness::Big, 0);
    put_u32(&mut bytes, Endianness::Big, version);
    put_u32(&mut bytes, Endianness::Big, data_offset as u32);
    bytes.push(match endian {
        Endianness::Little => 0,
        Endianness::Big => 1,
    });
    bytes.extend_from_slice(&[0, 0, 0]);
    bytes.extend_from_slice(&metadata);
    bytes.extend_from_slice(PAYLOAD_A);
    bytes.extend_from_slice(PAYLOAD_B);
    let file_size = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&file_size.to_be_bytes());
    bytes.extend_from_slice(trailing);

    Fixture {
        bytes,
        data_offset,
        metadata_size: metadata.len(),
    }
}

#[test]
fn reads_header_metadata_and_object_table() -> Result<(), BinarySerError> {
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    let file = read_serialized_file(&fixture.bytes)?;

    assert_eq!(file.header.metadata_size, fixture.metadata_size as u32);
    assert_eq!(file.header.file_size, fixture.bytes.len() as u32);
    assert_eq!(file.header.version, SUPPORTED_VERSION);
    assert_eq!(file.header.data_offset, fixture.data_offset as u32);
    assert_eq!(file.header.endianness, Endianness::Big);
    assert_eq!(file.header.header_size, HEADER_SIZE as u32);

    assert_eq!(file.metadata.unity_version, UNITY_VERSION);
    assert_eq!(file.metadata.target_platform, TARGET_PLATFORM);
    assert!(file.metadata.enable_type_tree);
    assert_eq!(file.metadata.type_count, TYPE_COUNT);

    assert_eq!(file.objects.len(), 2);
    let first = &file.objects[0];
    assert_eq!(first.path_id, 1);
    assert_eq!(first.class_id, 21);
    assert_eq!(first.byte_offset, fixture.data_offset as u64);
    assert_eq!(first.byte_size, PAYLOAD_A.len() as u32);
    assert!(!first.is_stripped);
    let second = &file.objects[1];
    assert_eq!(second.path_id, 2);
    assert_eq!(second.class_id, 28);
    assert_eq!(
        second.byte_offset,
        (fixture.data_offset + PAYLOAD_A.len()) as u64
    );
    assert_eq!(second.byte_size, PAYLOAD_B.len() as u32);
    assert!(second.is_stripped);
    Ok(())
}

#[test]
fn object_bytes_returns_the_payload() -> Result<(), BinarySerError> {
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    let file = read_serialized_file(&fixture.bytes)?;
    assert_eq!(file.object_bytes(0)?, PAYLOAD_A);
    assert_eq!(file.object_bytes(1)?, PAYLOAD_B);
    Ok(())
}

#[test]
fn object_by_path_id_finds_the_object() -> Result<(), BinarySerError> {
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    let file = read_serialized_file(&fixture.bytes)?;
    let found = file.object_by_path_id(2);
    assert_eq!(found.map(|object| object.class_id), Some(28));
    assert!(file.object_by_path_id(99).is_none());
    Ok(())
}

#[test]
fn write_back_is_byte_identical() -> Result<(), BinarySerError> {
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    let file = read_serialized_file(&fixture.bytes)?;
    assert_eq!(file.write_back(), fixture.bytes);
    assert_eq!(file.raw, fixture.bytes);
    Ok(())
}

#[test]
fn unknown_trailing_bytes_are_preserved() -> Result<(), BinarySerError> {
    let trailing = b"unknown-tail";
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, trailing);
    let file = read_serialized_file(&fixture.bytes)?;
    assert_eq!(file.write_back(), fixture.bytes);
    assert!(file.write_back().ends_with(trailing));
    Ok(())
}

#[test]
fn truncated_input_is_reported() {
    let fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    for cut in 0..fixture.bytes.len() {
        let result = read_serialized_file(&fixture.bytes[..cut]);
        assert!(
            matches!(&result, Err(BinarySerError::Truncated { .. })),
            "cut {cut} produced {result:?}"
        );
    }
}

#[test]
fn unsupported_version_is_reported() {
    for version in [SUPPORTED_VERSION + 1, 21, 30] {
        let fixture = build_file(Endianness::Big, version, &[]);
        let result = read_serialized_file(&fixture.bytes);
        assert!(
            matches!(
                &result,
                Err(BinarySerError::UnsupportedVersion { found, supported })
                    if *found == version && *supported == SUPPORTED_VERSION
            ),
            "version {version} produced {result:?}"
        );
    }
}

#[test]
fn unsupported_endianness_is_reported() {
    let mut fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    fixture.bytes[16] = 7;
    let result = read_serialized_file(&fixture.bytes);
    assert!(
        matches!(&result, Err(BinarySerError::UnsupportedEndianness)),
        "produced {result:?}"
    );
}

#[test]
fn corrupted_unity_version_length_is_reported() {
    let mut fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    fixture.bytes[HEADER_SIZE..HEADER_SIZE + 4].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
    let result = read_serialized_file(&fixture.bytes);
    assert!(
        matches!(&result, Err(BinarySerError::InvalidHeader(_))),
        "produced {result:?}"
    );
}

#[test]
fn corrupted_object_count_is_reported() {
    let mut fixture = build_file(Endianness::Big, SUPPORTED_VERSION, &[]);
    fixture.bytes[OBJECT_COUNT_OFFSET..OBJECT_COUNT_OFFSET + 4]
        .copy_from_slice(&0xffff_ffffu32.to_be_bytes());
    let result = read_serialized_file(&fixture.bytes);
    assert!(
        matches!(&result, Err(BinarySerError::InvalidObjectTable(_))),
        "produced {result:?}"
    );
}

#[test]
fn little_endian_host_metadata_is_read() -> Result<(), BinarySerError> {
    let fixture = build_file(Endianness::Little, SUPPORTED_VERSION, &[]);
    let file = read_serialized_file(&fixture.bytes)?;
    assert_eq!(file.header.endianness, Endianness::Little);
    assert_eq!(file.metadata.unity_version, UNITY_VERSION);
    assert_eq!(file.metadata.target_platform, TARGET_PLATFORM);
    assert_eq!(file.objects.len(), 2);
    assert_eq!(file.object_bytes(0)?, PAYLOAD_A);
    Ok(())
}

#[test]
fn object_bytes_reports_a_missing_payload() {
    let file = SerializedFile {
        header: SerializedFileHeader {
            metadata_size: 0,
            file_size: 50,
            version: SUPPORTED_VERSION,
            data_offset: 0,
            endianness: Endianness::Big,
            header_size: HEADER_SIZE as u32,
        },
        metadata: SerializedFileMetadata {
            unity_version: UNITY_VERSION.to_string(),
            target_platform: TARGET_PLATFORM,
            enable_type_tree: true,
            type_count: TYPE_COUNT,
        },
        objects: vec![SerializedObject {
            path_id: 1,
            class_id: 21,
            byte_offset: 40,
            byte_size: 32,
            is_stripped: false,
        }],
        raw: vec![0u8; 50],
    };
    assert!(matches!(
        file.object_bytes(0),
        Err(BinarySerError::Truncated { .. })
    ));
    assert!(matches!(
        file.object_bytes(1),
        Err(BinarySerError::InvalidObjectTable(_))
    ));
}
