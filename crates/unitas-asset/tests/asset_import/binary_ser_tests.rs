use unitas_asset::binary_ser::{
    is_text_serialized, read_serialized, read_serialized_file, BinarySerError, Endianness,
    SerializedAsset, SerializedFile, SerializedFileHeader, SerializedFileMetadata,
    SerializedObject, SUPPORTED_VERSION,
};

const UNITY_VERSION: &str = "2022.3.16f1";
const TARGET_PLATFORM: i32 = 19;
const PAYLOAD_A: &[u8] = b"payload-alpha";
const PAYLOAD_B: &[u8] = b"payload-beta";
const TYPE_REGION: &[u8] = &[0x7f, 0x01, 0x02, 0x03];
const TYPE_COUNT: u32 = 3;
const HEADER_SIZE: usize = 20;

const OBJECT_COUNT_OFFSET: usize =
    HEADER_SIZE + 4 + UNITY_VERSION.len() + 4 + 1 + 4 + 4 + TYPE_REGION.len();

struct Fixture {
    bytes: Vec<u8>,
    data_offset: usize,
    metadata_size: usize,
}

fn put_i16(out: &mut Vec<u8>, endian: Endianness, value: i16) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
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

fn build_real_v22_metadata(
    endian: Endianness,
    enable_type_tree: bool,
    with_type_dependencies: bool,
    data_offset: u64,
) -> Vec<u8> {
    let mut meta = Vec::new();
    put_u32(&mut meta, endian, UNITY_VERSION.len() as u32);
    meta.extend_from_slice(UNITY_VERSION.as_bytes());
    put_i32(&mut meta, endian, TARGET_PLATFORM);
    meta.push(u8::from(enable_type_tree));

    let type_classes: &[(i32, u8, i16)] = &[(21, 0, -1), (114, 0, 0), (-7, 1, 1)];
    put_u32(&mut meta, endian, type_classes.len() as u32);
    for (class_id, is_stripped, script_index) in type_classes {
        put_i32(&mut meta, endian, *class_id);
        meta.push(*is_stripped);
        put_i16(&mut meta, endian, *script_index);
        if *class_id == 114 || *class_id < 0 {
            meta.extend_from_slice(&[0xAA; 16]);
        }
        meta.extend_from_slice(&[0xBB; 16]);
        if enable_type_tree {
            put_i32(&mut meta, endian, 2);
            put_i32(&mut meta, endian, 12);
            meta.extend_from_slice(&[0xCC; 32 * 2]);
            meta.extend_from_slice(b"m_Name\0Base\0");
            if with_type_dependencies {
                put_i32(&mut meta, endian, 1);
                put_i32(&mut meta, endian, 0);
            }
        }
    }

    put_u32(&mut meta, endian, 2);
    put_i64(&mut meta, endian, 101);
    put_i32(&mut meta, endian, 21);
    put_u64(&mut meta, endian, data_offset);
    put_u32(&mut meta, endian, PAYLOAD_A.len() as u32);
    meta.push(0);
    put_i64(&mut meta, endian, 202);
    put_i32(&mut meta, endian, 114);
    put_u64(&mut meta, endian, data_offset + PAYLOAD_A.len() as u64);
    put_u32(&mut meta, endian, PAYLOAD_B.len() as u32);
    meta.push(1);
    meta
}

fn build_real_v22_file(
    endian: Endianness,
    enable_type_tree: bool,
    with_type_dependencies: bool,
    trailing: &[u8],
) -> Fixture {
    let probe = build_real_v22_metadata(endian, enable_type_tree, with_type_dependencies, 0);
    let data_offset = HEADER_SIZE + probe.len();
    let metadata = build_real_v22_metadata(
        endian,
        enable_type_tree,
        with_type_dependencies,
        data_offset as u64,
    );

    let mut bytes = Vec::new();
    put_u32(&mut bytes, Endianness::Big, metadata.len() as u32);
    put_u32(&mut bytes, Endianness::Big, 0);
    put_u32(&mut bytes, Endianness::Big, SUPPORTED_VERSION);
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

#[test]
fn reads_real_unity_v22_unprefixed_type_table_without_type_tree() -> Result<(), BinarySerError> {
    let trailing = b"v22-trailing-bytes";
    let fixture = build_real_v22_file(Endianness::Little, false, false, trailing);
    let file = read_serialized_file(&fixture.bytes)?;

    assert_eq!(file.header.version, SUPPORTED_VERSION);
    assert_eq!(file.header.endianness, Endianness::Little);
    assert_eq!(file.metadata.unity_version, UNITY_VERSION);
    assert_eq!(file.metadata.target_platform, TARGET_PLATFORM);
    assert!(!file.metadata.enable_type_tree);
    assert_eq!(file.metadata.type_count, 3);
    assert_eq!(file.objects.len(), 2);
    assert_eq!(file.objects[0].path_id, 101);
    assert_eq!(file.objects[0].class_id, 21);
    assert_eq!(file.object_bytes(0)?, PAYLOAD_A);
    assert_eq!(file.objects[1].path_id, 202);
    assert_eq!(file.objects[1].class_id, 114);
    assert!(file.objects[1].is_stripped);
    assert_eq!(file.object_bytes(1)?, PAYLOAD_B);
    assert_eq!(file.write_back(), fixture.bytes);
    assert!(file.write_back().ends_with(trailing));
    Ok(())
}

#[test]
fn reads_real_unity_v22_unprefixed_type_table_with_type_tree() -> Result<(), BinarySerError> {
    for endian in [Endianness::Little, Endianness::Big] {
        for with_deps in [false, true] {
            let trailing = b"type-tree-tail";
            let fixture = build_real_v22_file(endian, true, with_deps, trailing);
            let file = read_serialized_file(&fixture.bytes)?;

            assert_eq!(file.header.endianness, endian);
            assert!(file.metadata.enable_type_tree);
            assert_eq!(file.metadata.type_count, 3);
            assert_eq!(file.objects.len(), 2);
            assert_eq!(file.object_bytes(0)?, PAYLOAD_A);
            assert_eq!(file.object_bytes(1)?, PAYLOAD_B);
            assert_eq!(file.write_back(), fixture.bytes);
        }
    }
    Ok(())
}

#[test]
fn mixed_sniffing_reads_text_yaml_and_binary_v22() -> Result<(), BinarySerError> {
    let yaml_bytes = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: TestMat\n";
    assert!(is_text_serialized(yaml_bytes));

    let text_asset = read_serialized(yaml_bytes)?;
    assert!(text_asset.is_text());
    assert!(!text_asset.is_binary());
    assert_eq!(text_asset.write_back(), yaml_bytes);
    let stream = text_asset
        .as_text()
        .ok_or(BinarySerError::InvalidHeader("expected text stream"))?;
    assert_eq!(stream.documents.len(), 1);
    assert_eq!(stream.documents[0].class_id, 21);
    assert_eq!(stream.documents[0].file_id, 2100000);

    let binary_fixture = build_real_v22_file(Endianness::Little, true, false, b"extra");
    assert!(!is_text_serialized(&binary_fixture.bytes));

    let binary_asset = SerializedAsset::read(&binary_fixture.bytes)?;
    assert!(binary_asset.is_binary());
    assert!(!binary_asset.is_text());
    assert_eq!(binary_asset.write_back(), binary_fixture.bytes);
    let file = binary_asset
        .as_binary()
        .ok_or(BinarySerError::InvalidHeader("expected binary file"))?;
    assert_eq!(file.objects.len(), 2);
    assert_eq!(file.object_bytes(0)?, PAYLOAD_A);

    match text_asset {
        SerializedAsset::Text { stream, raw } => {
            assert_eq!(stream.documents.len(), 1);
            assert_eq!(raw, yaml_bytes);
        }
        SerializedAsset::Binary(_) => {
            return Err(BinarySerError::InvalidHeader("unexpected binary variant"));
        }
    }
    Ok(())
}
