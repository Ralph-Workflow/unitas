use unitas_asset::binary_ser::{
    is_text_serialized, read_serialized, read_serialized_file, BinarySerError, Endianness,
    SerializedAsset, SerializedFile, SerializedFileHeader, SerializedFileMetadata,
    SerializedObject, TypeTree, TypeTreeNode, TypedObject, TypedValue, SUPPORTED_VERSION,
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

struct NodeSpec<'a> {
    level: u8,
    type_flags: u8,
    type_name: &'a str,
    name: &'a str,
    byte_size: i32,
    meta_flag: i32,
}

fn encode_type_tree_blob(endian: Endianness, specs: &[NodeSpec<'_>]) -> Vec<u8> {
    let mut string_buf = Vec::new();
    let mut offsets = Vec::with_capacity(specs.len());
    for spec in specs {
        let type_offset = string_buf.len() as u32;
        string_buf.extend_from_slice(spec.type_name.as_bytes());
        string_buf.push(0);
        let name_offset = string_buf.len() as u32;
        string_buf.extend_from_slice(spec.name.as_bytes());
        string_buf.push(0);
        offsets.push((type_offset, name_offset));
    }

    let mut blob = Vec::new();
    put_i32(&mut blob, endian, specs.len() as i32);
    put_i32(&mut blob, endian, string_buf.len() as i32);
    for (index, (spec, (type_offset, name_offset))) in specs.iter().zip(offsets.iter()).enumerate()
    {
        put_i16(&mut blob, endian, 1);
        blob.push(spec.level);
        blob.push(spec.type_flags);
        put_u32(&mut blob, endian, *type_offset);
        put_u32(&mut blob, endian, *name_offset);
        put_i32(&mut blob, endian, spec.byte_size);
        put_i32(&mut blob, endian, index as i32);
        put_i32(&mut blob, endian, spec.meta_flag);
        put_u64(&mut blob, endian, 0);
    }
    blob.extend_from_slice(&string_buf);
    blob
}

fn game_object_type_tree(endian: Endianness) -> Vec<u8> {
    const ALIGN: i32 = 0x4000;
    encode_type_tree_blob(
        endian,
        &[
            NodeSpec {
                level: 0,
                type_flags: 0,
                type_name: "GameObject",
                name: "Base",
                byte_size: -1,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "vector",
                name: "m_Component",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 2,
                type_flags: 1,
                type_name: "Array",
                name: "Array",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "int",
                name: "size",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "ComponentPair",
                name: "data",
                byte_size: 12,
                meta_flag: 0,
            },
            NodeSpec {
                level: 4,
                type_flags: 0,
                type_name: "PPtr<Component>",
                name: "component",
                byte_size: 12,
                meta_flag: 0,
            },
            NodeSpec {
                level: 5,
                type_flags: 0,
                type_name: "int",
                name: "m_FileID",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 5,
                type_flags: 0,
                type_name: "SInt64",
                name: "m_PathID",
                byte_size: 8,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "unsigned int",
                name: "m_Layer",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "string",
                name: "m_Name",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 2,
                type_flags: 1,
                type_name: "Array",
                name: "Array",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "int",
                name: "size",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "char",
                name: "data",
                byte_size: 1,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "UInt16",
                name: "m_Tag",
                byte_size: 2,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "bool",
                name: "m_IsActive",
                byte_size: 1,
                meta_flag: ALIGN,
            },
        ],
    )
}

fn transform_pose_node_specs() -> [NodeSpec<'static>; 14] {
    [
        NodeSpec {
            level: 0,
            type_flags: 0,
            type_name: "Transform",
            name: "Base",
            byte_size: -1,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "PPtr<GameObject>",
            name: "m_GameObject",
            byte_size: 12,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "int",
            name: "m_FileID",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "SInt64",
            name: "m_PathID",
            byte_size: 8,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "Quaternionf",
            name: "m_LocalRotation",
            byte_size: 16,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "x",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "y",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "z",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "w",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "Vector3f",
            name: "m_LocalPosition",
            byte_size: 12,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "x",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "y",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "z",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "Vector3f",
            name: "m_LocalScale",
            byte_size: 12,
            meta_flag: 0,
        },
    ]
}

fn transform_hierarchy_node_specs() -> [NodeSpec<'static>; 12] {
    const ALIGN: i32 = 0x4000;
    [
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "x",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "y",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "float",
            name: "z",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "vector",
            name: "m_Children",
            byte_size: -1,
            meta_flag: ALIGN,
        },
        NodeSpec {
            level: 2,
            type_flags: 1,
            type_name: "Array",
            name: "Array",
            byte_size: -1,
            meta_flag: ALIGN,
        },
        NodeSpec {
            level: 3,
            type_flags: 0,
            type_name: "int",
            name: "size",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 3,
            type_flags: 0,
            type_name: "PPtr<Transform>",
            name: "data",
            byte_size: 12,
            meta_flag: 0,
        },
        NodeSpec {
            level: 4,
            type_flags: 0,
            type_name: "int",
            name: "m_FileID",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 4,
            type_flags: 0,
            type_name: "SInt64",
            name: "m_PathID",
            byte_size: 8,
            meta_flag: 0,
        },
        NodeSpec {
            level: 1,
            type_flags: 0,
            type_name: "PPtr<Transform>",
            name: "m_Father",
            byte_size: 12,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "int",
            name: "m_FileID",
            byte_size: 4,
            meta_flag: 0,
        },
        NodeSpec {
            level: 2,
            type_flags: 0,
            type_name: "SInt64",
            name: "m_PathID",
            byte_size: 8,
            meta_flag: 0,
        },
    ]
}

fn transform_type_tree(endian: Endianness) -> Vec<u8> {
    let mut specs = Vec::with_capacity(26);
    specs.extend(transform_pose_node_specs());
    specs.extend(transform_hierarchy_node_specs());
    encode_type_tree_blob(endian, &specs)
}

fn monobehaviour_type_tree(endian: Endianness) -> Vec<u8> {
    const ALIGN: i32 = 0x4000;
    encode_type_tree_blob(
        endian,
        &[
            NodeSpec {
                level: 0,
                type_flags: 0,
                type_name: "MonoBehaviour",
                name: "Base",
                byte_size: -1,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "PPtr<GameObject>",
                name: "m_GameObject",
                byte_size: 12,
                meta_flag: 0,
            },
            NodeSpec {
                level: 2,
                type_flags: 0,
                type_name: "int",
                name: "m_FileID",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 2,
                type_flags: 0,
                type_name: "SInt64",
                name: "m_PathID",
                byte_size: 8,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "UInt8",
                name: "m_Enabled",
                byte_size: 1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "PPtr<MonoScript>",
                name: "m_Script",
                byte_size: 12,
                meta_flag: 0,
            },
            NodeSpec {
                level: 2,
                type_flags: 0,
                type_name: "int",
                name: "m_FileID",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 2,
                type_flags: 0,
                type_name: "SInt64",
                name: "m_PathID",
                byte_size: 8,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "string",
                name: "m_Name",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 2,
                type_flags: 1,
                type_name: "Array",
                name: "Array",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "int",
                name: "size",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "char",
                name: "data",
                byte_size: 1,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "float",
                name: "speed",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "int",
                name: "ammo",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 1,
                type_flags: 0,
                type_name: "string",
                name: "callsign",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 2,
                type_flags: 1,
                type_name: "Array",
                name: "Array",
                byte_size: -1,
                meta_flag: ALIGN,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "int",
                name: "size",
                byte_size: 4,
                meta_flag: 0,
            },
            NodeSpec {
                level: 3,
                type_flags: 0,
                type_name: "char",
                name: "data",
                byte_size: 1,
                meta_flag: 0,
            },
        ],
    )
}

fn put_f32(out: &mut Vec<u8>, endian: Endianness, value: f32) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn put_u16(out: &mut Vec<u8>, endian: Endianness, value: u16) {
    out.extend_from_slice(&match endian {
        Endianness::Big => value.to_be_bytes(),
        Endianness::Little => value.to_le_bytes(),
    });
}

fn put_aligned_string(out: &mut Vec<u8>, endian: Endianness, text: &str) {
    put_i32(out, endian, text.len() as i32);
    out.extend_from_slice(text.as_bytes());
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn encode_game_object_payload(
    endian: Endianness,
    components: &[(i32, i64)],
    layer: u32,
    name: &str,
    tag: u16,
    is_active: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    put_i32(&mut out, endian, components.len() as i32);
    for (file_id, path_id) in components {
        put_i32(&mut out, endian, *file_id);
        put_i64(&mut out, endian, *path_id);
    }
    put_u32(&mut out, endian, layer);
    put_aligned_string(&mut out, endian, name);
    put_u16(&mut out, endian, tag);
    out.push(u8::from(is_active));
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
    out
}

fn encode_transform_payload(
    endian: Endianness,
    game_object: (i32, i64),
    rotation: [f32; 4],
    position: [f32; 3],
    scale: [f32; 3],
    children: &[(i32, i64)],
    father: (i32, i64),
) -> Vec<u8> {
    let mut out = Vec::new();
    put_i32(&mut out, endian, game_object.0);
    put_i64(&mut out, endian, game_object.1);
    for c in rotation {
        put_f32(&mut out, endian, c);
    }
    for c in position {
        put_f32(&mut out, endian, c);
    }
    for c in scale {
        put_f32(&mut out, endian, c);
    }
    put_i32(&mut out, endian, children.len() as i32);
    for (file_id, path_id) in children {
        put_i32(&mut out, endian, *file_id);
        put_i64(&mut out, endian, *path_id);
    }
    put_i32(&mut out, endian, father.0);
    put_i64(&mut out, endian, father.1);
    out
}

struct MonoBehaviourSpec<'a> {
    game_object: (i32, i64),
    enabled: u8,
    script: (i32, i64),
    name: &'a str,
    speed: f32,
    ammo: i32,
    callsign: &'a str,
}

fn encode_monobehaviour_payload(endian: Endianness, spec: &MonoBehaviourSpec<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    put_i32(&mut out, endian, spec.game_object.0);
    put_i64(&mut out, endian, spec.game_object.1);
    out.push(spec.enabled);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
    put_i32(&mut out, endian, spec.script.0);
    put_i64(&mut out, endian, spec.script.1);
    put_aligned_string(&mut out, endian, spec.name);
    put_f32(&mut out, endian, spec.speed);
    put_i32(&mut out, endian, spec.ammo);
    put_aligned_string(&mut out, endian, spec.callsign);
    out
}

struct TypedFixturePayloads<'a> {
    go: &'a [u8],
    tr: &'a [u8],
    mb: &'a [u8],
    unknown: &'a [u8],
    gap: &'a [u8],
    trailing: &'a [u8],
}

fn build_typed_v22_fixture(
    endian: Endianness,
    enable_type_tree: bool,
    payloads: &TypedFixturePayloads<'_>,
) -> Vec<u8> {
    let go_payload = payloads.go;
    let tr_payload = payloads.tr;
    let mb_payload = payloads.mb;
    let unknown_payload = payloads.unknown;
    let gap_between_objects = payloads.gap;
    let trailing = payloads.trailing;
    let build_meta = |data_offset: u64| -> Vec<u8> {
        let mut meta = Vec::new();
        put_u32(&mut meta, endian, UNITY_VERSION.len() as u32);
        meta.extend_from_slice(UNITY_VERSION.as_bytes());
        put_i32(&mut meta, endian, TARGET_PLATFORM);
        meta.push(u8::from(enable_type_tree));

        let types: &[(i32, u8, i16, Option<Vec<u8>>)] = &[
            (
                1,
                0,
                -1,
                if enable_type_tree {
                    Some(game_object_type_tree(endian))
                } else {
                    None
                },
            ),
            (
                4,
                0,
                -1,
                if enable_type_tree {
                    Some(transform_type_tree(endian))
                } else {
                    None
                },
            ),
            (
                114,
                0,
                0,
                if enable_type_tree {
                    Some(monobehaviour_type_tree(endian))
                } else {
                    None
                },
            ),
            (
                999,
                0,
                -1,
                if enable_type_tree {
                    Some(encode_type_tree_blob(
                        endian,
                        &[
                            NodeSpec {
                                level: 0,
                                type_flags: 0,
                                type_name: "MysteryObject",
                                name: "Base",
                                byte_size: -1,
                                meta_flag: 0,
                            },
                            NodeSpec {
                                level: 1,
                                type_flags: 0,
                                type_name: "UnsupportedBlobType",
                                name: "m_Blob",
                                byte_size: -1,
                                meta_flag: 0,
                            },
                        ],
                    ))
                } else {
                    None
                },
            ),
        ];
        put_u32(&mut meta, endian, types.len() as u32);
        for (class_id, is_stripped, script_index, tree_blob) in types {
            put_i32(&mut meta, endian, *class_id);
            meta.push(*is_stripped);
            put_i16(&mut meta, endian, *script_index);
            if *class_id == 114 || *class_id < 0 {
                meta.extend_from_slice(&[0xAA; 16]);
            }
            meta.extend_from_slice(&[0xBB; 16]);
            if let Some(blob) = tree_blob {
                meta.extend_from_slice(blob);
            }
        }

        let off0 = data_offset;
        let off1 = off0 + go_payload.len() as u64 + gap_between_objects.len() as u64;
        let off2 = off1 + tr_payload.len() as u64 + gap_between_objects.len() as u64;
        let off3 = off2 + mb_payload.len() as u64 + gap_between_objects.len() as u64;

        put_u32(&mut meta, endian, 4);

        put_i64(&mut meta, endian, 1001);
        put_i32(&mut meta, endian, 1);
        put_u64(&mut meta, endian, off0);
        put_u32(&mut meta, endian, go_payload.len() as u32);
        meta.push(0);

        put_i64(&mut meta, endian, 1002);
        put_i32(&mut meta, endian, 4);
        put_u64(&mut meta, endian, off1);
        put_u32(&mut meta, endian, tr_payload.len() as u32);
        meta.push(0);

        put_i64(&mut meta, endian, 1003);
        put_i32(&mut meta, endian, 114);
        put_u64(&mut meta, endian, off2);
        put_u32(&mut meta, endian, mb_payload.len() as u32);
        meta.push(0);

        put_i64(&mut meta, endian, 1004);
        put_i32(&mut meta, endian, 999);
        put_u64(&mut meta, endian, off3);
        put_u32(&mut meta, endian, unknown_payload.len() as u32);
        meta.push(0);

        meta
    };

    let probe = build_meta(0);
    let data_offset = HEADER_SIZE + probe.len();
    let metadata = build_meta(data_offset as u64);

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
    bytes.extend_from_slice(go_payload);
    bytes.extend_from_slice(gap_between_objects);
    bytes.extend_from_slice(tr_payload);
    bytes.extend_from_slice(gap_between_objects);
    bytes.extend_from_slice(mb_payload);
    bytes.extend_from_slice(gap_between_objects);
    bytes.extend_from_slice(unknown_payload);
    let file_size = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&file_size.to_be_bytes());
    bytes.extend_from_slice(trailing);
    bytes
}

#[test]
fn typed_decode_edit_write_back_and_reload_preserves_identity_and_unedited_bytes(
) -> Result<(), BinarySerError> {
    let endian = Endianness::Little;
    let go_payload =
        encode_game_object_payload(endian, &[(0, 1002), (0, 1003)], 3, "Hero", 1, true);
    let tr_payload = encode_transform_payload(
        endian,
        (0, 1001),
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 2.0, 3.0],
        [1.0, 1.0, 1.0],
        &[],
        (0, 0),
    );
    let mb_payload = encode_monobehaviour_payload(
        endian,
        &MonoBehaviourSpec {
            game_object: (0, 1001),
            enabled: 1,
            script: (1, 5555),
            name: "",
            speed: 4.5,
            ammo: 12,
            callsign: "Alpha",
        },
    );
    let unknown_payload = b"\xDE\xAD\xBE\xEF\x01\x02\x03\x04";
    let gap = b"\xAB\xCD\xEF\x12";
    let trailing = b"trailing-binary-region-preserved";

    let raw_bytes = build_typed_v22_fixture(
        endian,
        true,
        &TypedFixturePayloads {
            go: &go_payload,
            tr: &tr_payload,
            mb: &mb_payload,
            unknown: unknown_payload,
            gap,
            trailing,
        },
    );

    let mut asset = SerializedAsset::read(&raw_bytes)?;
    assert!(asset.is_binary());
    let file = asset.as_binary_mut().ok_or(BinarySerError::InvalidHeader(
        "expected mutable binary file",
    ))?;

    let mut decoded_go: TypedObject = file.decode_object(1001)?;
    assert_eq!(decoded_go.type_name, "GameObject");
    assert_eq!(
        decoded_go.get("m_Name").and_then(TypedValue::as_str),
        Some("Hero")
    );
    assert_eq!(
        decoded_go.get("m_Layer").and_then(TypedValue::as_u32),
        Some(3)
    );
    assert_eq!(
        decoded_go.get("m_IsActive").and_then(TypedValue::as_bool),
        Some(true)
    );

    decoded_go.set(
        "m_Name",
        TypedValue::String("HeroPrimeUpdatedLongerName".to_string()),
    )?;
    file.update_object(1001, &decoded_go)?;

    let mut decoded_mb: TypedObject = file.decode_object(1003)?;
    assert_eq!(decoded_mb.type_name, "MonoBehaviour");
    assert_eq!(
        decoded_mb.get("speed").and_then(TypedValue::as_f32),
        Some(4.5)
    );
    assert_eq!(
        decoded_mb.get("ammo").and_then(TypedValue::as_i32),
        Some(12)
    );
    assert_eq!(
        decoded_mb.get("callsign").and_then(TypedValue::as_str),
        Some("Alpha")
    );

    decoded_mb.set("speed", TypedValue::F32(9.75))?;
    decoded_mb.set("ammo", TypedValue::I32(42))?;
    decoded_mb.set("callsign", TypedValue::String("OmegaSquadron".to_string()))?;
    file.update_object(1003, &decoded_mb)?;

    let saved_bytes = asset.write_back();
    assert!(!is_text_serialized(&saved_bytes));
    assert!(saved_bytes.ends_with(trailing));

    let reloaded_asset = SerializedAsset::read(&saved_bytes)?;
    assert!(reloaded_asset.is_binary());
    let reloaded = reloaded_asset
        .as_binary()
        .ok_or(BinarySerError::InvalidHeader(
            "expected binary file on reload",
        ))?;

    assert_eq!(reloaded.objects.len(), 4);
    assert_eq!(reloaded.objects[0].path_id, 1001);
    assert_eq!(reloaded.objects[0].class_id, 1);
    assert_eq!(reloaded.objects[1].path_id, 1002);
    assert_eq!(reloaded.objects[1].class_id, 4);
    assert_eq!(reloaded.objects[2].path_id, 1003);
    assert_eq!(reloaded.objects[2].class_id, 114);
    assert_eq!(reloaded.objects[3].path_id, 1004);
    assert_eq!(reloaded.objects[3].class_id, 999);

    assert_eq!(reloaded.object_bytes(1)?, tr_payload.as_slice());
    assert_eq!(reloaded.object_bytes(3)?, unknown_payload.as_slice());

    let tr_start = reloaded.objects[1].byte_offset as usize;
    assert_eq!(&saved_bytes[tr_start - gap.len()..tr_start], gap);
    let mb_start = reloaded.objects[2].byte_offset as usize;
    assert_eq!(&saved_bytes[mb_start - gap.len()..mb_start], gap);
    let unk_start = reloaded.objects[3].byte_offset as usize;
    assert_eq!(&saved_bytes[unk_start - gap.len()..unk_start], gap);

    let reloaded_go = reloaded.decode_object(1001)?;
    assert_eq!(
        reloaded_go.get("m_Name").and_then(TypedValue::as_str),
        Some("HeroPrimeUpdatedLongerName")
    );
    assert_eq!(
        reloaded_go.get("m_Layer").and_then(TypedValue::as_u32),
        Some(3)
    );
    assert_eq!(
        reloaded_go.get("m_IsActive").and_then(TypedValue::as_bool),
        Some(true)
    );

    let reloaded_tr = reloaded.decode_object(1002)?;
    let pos = reloaded_tr
        .get("m_LocalPosition")
        .ok_or(BinarySerError::InvalidHeader("missing m_LocalPosition"))?;
    assert_eq!(pos.get("x").and_then(TypedValue::as_f32), Some(1.0));
    assert_eq!(pos.get("y").and_then(TypedValue::as_f32), Some(2.0));
    assert_eq!(pos.get("z").and_then(TypedValue::as_f32), Some(3.0));

    let reloaded_mb = reloaded.decode_object(1003)?;
    assert_eq!(
        reloaded_mb.get("speed").and_then(TypedValue::as_f32),
        Some(9.75)
    );
    assert_eq!(
        reloaded_mb.get("ammo").and_then(TypedValue::as_i32),
        Some(42)
    );
    assert_eq!(
        reloaded_mb.get("callsign").and_then(TypedValue::as_str),
        Some("OmegaSquadron")
    );

    let mut editable_tr = reloaded.decode_object(1002)?;
    let pos_mut = editable_tr
        .get_mut("m_LocalPosition")
        .ok_or(BinarySerError::InvalidHeader("missing m_LocalPosition"))?;
    if let Some(x_slot) = pos_mut.get_mut("x") {
        *x_slot = TypedValue::F32(7.25);
    }
    let children_val = editable_tr
        .get("m_Children")
        .and_then(TypedValue::as_array)
        .ok_or(BinarySerError::InvalidHeader("missing m_Children array"))?;
    assert!(children_val.is_empty());

    let mut file_for_tr = reloaded.clone();
    file_for_tr.update_object(1002, &editable_tr)?;
    let tr_reloaded = read_serialized_file(&file_for_tr.write_back())?.decode_object(1002)?;
    assert_eq!(
        tr_reloaded
            .get("m_LocalPosition")
            .and_then(|v| v.get("x"))
            .and_then(TypedValue::as_f32),
        Some(7.25)
    );

    let _ = TypeTree {
        nodes: Vec::<TypeTreeNode>::new(),
    };
    Ok(())
}

#[test]
fn typed_decode_and_edit_works_for_big_endian_and_shorter_payloads() -> Result<(), BinarySerError> {
    let endian = Endianness::Big;
    let go_payload = encode_game_object_payload(
        endian,
        &[(0, 1002)],
        1,
        "VeryLongOriginalGameObjectName",
        0,
        true,
    );
    let tr_payload = encode_transform_payload(
        endian,
        (0, 1001),
        [0.0, 0.0, 0.0, 1.0],
        [4.0, 5.0, 6.0],
        [1.0, 1.0, 1.0],
        &[],
        (0, 0),
    );
    let mb_payload = encode_monobehaviour_payload(
        endian,
        &MonoBehaviourSpec {
            game_object: (0, 1001),
            enabled: 1,
            script: (0, 0),
            name: "Controller",
            speed: 2.0,
            ammo: 5,
            callsign: "InitialCallsign",
        },
    );
    let unknown_payload = b"\xCA\xFE\xBA\xBE";
    let trailing = b"big-endian-tail";

    let raw_bytes = build_typed_v22_fixture(
        endian,
        true,
        &TypedFixturePayloads {
            go: &go_payload,
            tr: &tr_payload,
            mb: &mb_payload,
            unknown: unknown_payload,
            gap: &[],
            trailing,
        },
    );
    let mut file = read_serialized_file(&raw_bytes)?;
    let mut go = file.decode_object(1001)?;
    assert_eq!(
        go.get("m_Name").and_then(TypedValue::as_str),
        Some("VeryLongOriginalGameObjectName")
    );
    go.set("m_Name", TypedValue::String("A".to_string()))?;
    file.update_object(1001, &go)?;

    let saved = file.write_back();
    assert!(saved.len() < raw_bytes.len());
    assert!(saved.ends_with(trailing));

    let reloaded = read_serialized_file(&saved)?;
    assert_eq!(
        reloaded
            .decode_object(1001)?
            .get("m_Name")
            .and_then(TypedValue::as_str),
        Some("A")
    );
    assert_eq!(reloaded.object_bytes(1)?, tr_payload.as_slice());
    assert_eq!(reloaded.object_bytes(2)?, mb_payload.as_slice());
    assert_eq!(reloaded.object_bytes(3)?, unknown_payload.as_slice());
    Ok(())
}

#[test]
fn typed_decode_reports_missing_type_tree_and_unsupported_type() -> Result<(), BinarySerError> {
    let endian = Endianness::Little;
    let go_payload = encode_game_object_payload(endian, &[], 0, "Root", 0, true);
    let tr_payload = encode_transform_payload(
        endian,
        (0, 1001),
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        &[],
        (0, 0),
    );
    let mb_payload = encode_monobehaviour_payload(
        endian,
        &MonoBehaviourSpec {
            game_object: (0, 1001),
            enabled: 1,
            script: (0, 0),
            name: "",
            speed: 1.0,
            ammo: 1,
            callsign: "A",
        },
    );
    let unknown_payload = b"\x01\x02\x03\x04";

    let payloads = TypedFixturePayloads {
        go: &go_payload,
        tr: &tr_payload,
        mb: &mb_payload,
        unknown: unknown_payload,
        gap: &[],
        trailing: &[],
    };

    let no_tree_bytes = build_typed_v22_fixture(endian, false, &payloads);
    let no_tree_file = read_serialized_file(&no_tree_bytes)?;
    assert_eq!(
        no_tree_file.decode_object(1001),
        Err(BinarySerError::MissingTypeTree {
            path_id: 1001,
            class_id: 1
        })
    );

    let with_tree_bytes = build_typed_v22_fixture(endian, true, &payloads);
    let with_tree_file = read_serialized_file(&with_tree_bytes)?;
    assert_eq!(
        with_tree_file.decode_object(1004),
        Err(BinarySerError::UnsupportedTypeTree {
            path_id: 1004,
            class_id: 999,
            type_name: "UnsupportedBlobType".to_string(),
        })
    );
    assert_eq!(
        with_tree_file.decode_object(9999),
        Err(BinarySerError::ObjectNotFound { path_id: 9999 })
    );

    let mut go = with_tree_file.decode_object(1001)?;
    assert_eq!(
        go.set("nonexistent_field", TypedValue::I32(1)),
        Err(BinarySerError::FieldNotFound {
            type_name: "GameObject".to_string(),
            field: "nonexistent_field".to_string(),
        })
    );
    go.set("m_Name", TypedValue::I32(123))?;
    assert_eq!(
        with_tree_file.encode_object(&go),
        Err(BinarySerError::TypeMismatch {
            field: "m_Name".to_string(),
            expected: "string".to_string(),
        })
    );
    Ok(())
}
