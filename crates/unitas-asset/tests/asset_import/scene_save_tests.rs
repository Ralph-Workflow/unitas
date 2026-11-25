use std::error::Error;
use std::fs;
use std::path::PathBuf;

use unitas_asset::binary_ser::{
    Endianness, SerializedAsset, SerializedFile, SerializedFileHeader, SerializedFileMetadata,
    TypedValue, SUPPORTED_VERSION,
};
use unitas_asset::scene::assemble_scene_file;
use unitas_asset::scene_save::{self, SceneFile, SceneSaveError};
use unitas_asset::unity_yaml::{self, Node};

type TestResult = Result<(), Box<dyn Error>>;

const SCENE_FIXTURE: &str = "tests/fixtures/scene.unity";
const SAMPLE_PREFAB_FIXTURE: &str = "tests/fixtures/sample.prefab";
const PREFAB_FIXTURE: &str = "tests/fixtures/prefab.prefab";
const SAMPLE_SCENE_FIXTURE: &str = "tests/fixtures/sample.unity";

const UNKNOWN_DOCUMENT: &str =
    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!9999 &7\nUnknownType:\n  m_Value: 3\n";

#[test]
fn emit_stream_writes_the_same_bytes_as_the_source_file() -> TestResult {
    for fixture in [
        SCENE_FIXTURE,
        SAMPLE_PREFAB_FIXTURE,
        PREFAB_FIXTURE,
        SAMPLE_SCENE_FIXTURE,
    ] {
        let original = fs::read_to_string(fixture)?;
        let stream = unity_yaml::parse(&original)?;
        assert_eq!(
            scene_save::emit_stream(&stream),
            original,
            "emitted text differs from {fixture}"
        );
    }
    Ok(())
}

#[test]
fn emit_stream_round_trips_the_parsed_stream() -> TestResult {
    let original = fs::read_to_string(SCENE_FIXTURE)?;
    let stream = unity_yaml::parse(&original)?;
    assert_eq!(
        unity_yaml::parse(&scene_save::emit_stream(&stream))?,
        stream
    );
    Ok(())
}

#[test]
fn editing_one_field_changes_only_that_field() -> TestResult {
    let original = fs::read_to_string(SCENE_FIXTURE)?;
    let mut stream = unity_yaml::parse(&original)?;
    assert!(set_field(
        &mut stream.documents[0].root,
        &["GameObject", "m_Name"],
        "Root Renamed"
    ));
    let emitted = scene_save::emit_stream(&stream);
    let expected = original.replacen("  m_Name: Root\n", "  m_Name: Root Renamed\n", 1);
    assert_ne!(expected, original);
    assert_eq!(emitted, expected);
    Ok(())
}

#[test]
fn an_unknown_document_round_trips_untouched() -> TestResult {
    let stream = unity_yaml::parse(UNKNOWN_DOCUMENT)?;
    assert_eq!(scene_save::emit_stream(&stream), UNKNOWN_DOCUMENT);
    assert_eq!(
        unity_yaml::parse(&scene_save::emit_stream(&stream))?,
        stream
    );
    Ok(())
}

#[test]
fn saving_one_scene_of_an_additive_set_writes_only_that_scene() -> TestResult {
    let root = temp_root("additive_set");
    fs::create_dir_all(&root)?;
    let alpha_path = root.join("Alpha.unity");
    let beta_path = root.join("Beta.unity");
    let alpha_text = fs::read_to_string(SCENE_FIXTURE)?;
    let beta_text = fs::read_to_string(SAMPLE_SCENE_FIXTURE)?.replacen(
        "m_Name: Main Camera\n",
        "m_Name: \"Main Camera\"\n",
        1,
    );
    fs::write(&alpha_path, &alpha_text)?;
    fs::write(&beta_path, &beta_text)?;

    let mut alpha_stream = unity_yaml::parse(&alpha_text)?;
    assert!(set_field(
        &mut alpha_stream.documents[0].root,
        &["GameObject", "m_Name"],
        "Alpha"
    ));
    let scenes = vec![
        SceneFile {
            path: alpha_path.clone(),
            stream: alpha_stream,
        },
        SceneFile {
            path: beta_path.clone(),
            stream: unity_yaml::parse(&beta_text)?,
        },
    ];
    scene_save::save_scene_set(&scenes, &alpha_path)?;

    assert_eq!(
        fs::read_to_string(&alpha_path)?,
        alpha_text.replacen("  m_Name: Root\n", "  m_Name: Alpha\n", 1)
    );
    assert_eq!(fs::read_to_string(&beta_path)?, beta_text);
    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn a_failed_save_keeps_the_previous_file_content() -> TestResult {
    let root = temp_root("failed_save");
    fs::create_dir_all(&root)?;
    let path = root.join("Scene.unity");
    let original = fs::read_to_string(SCENE_FIXTURE)?;
    fs::write(&path, &original)?;
    fs::create_dir(root.join("Scene.unity.tmp"))?;

    let mut stream = unity_yaml::parse(&original)?;
    assert!(set_field(
        &mut stream.documents[0].root,
        &["GameObject", "m_Name"],
        "Changed"
    ));
    let error = match scene_save::save_stream(&path, &stream) {
        Ok(()) => return Err(String::from("the save unexpectedly succeeded").into()),
        Err(error) => error,
    };
    assert!(matches!(error, SceneSaveError::Io { .. }));
    assert_eq!(fs::read_to_string(&path)?, original);
    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn a_mixed_mode_project_keeps_each_document_in_its_own_form() -> TestResult {
    let root = temp_root("mixed_mode");
    fs::create_dir_all(&root)?;
    let text_path = root.join("Scene.unity");
    let binary_path = root.join("Asset.asset");
    let text = fs::read_to_string(SCENE_FIXTURE)?;
    let binary = b"\x00\x01\x02binary payload";
    fs::write(&text_path, &text)?;
    fs::write(&binary_path, binary)?;

    let mut asset = unitas_asset::binary_ser::read_serialized(text.as_bytes())?;
    match &mut asset {
        SerializedAsset::Text { stream, .. } => {
            assert!(set_field(
                &mut stream.documents[0].root,
                &["GameObject", "m_Name"],
                "Edited"
            ));
        }
        SerializedAsset::Binary(_) => {
            return Err(String::from("the scene fixture is not text").into())
        }
    }
    scene_save::save_asset(&text_path, &asset)?;
    let saved_text = fs::read_to_string(&text_path)?;
    assert!(saved_text.starts_with("%YAML 1.1\n"));
    assert!(saved_text.contains("  m_Name: Edited\n"));

    let binary_asset = binary_asset(binary);
    assert!(binary_asset.is_binary());
    scene_save::save_asset(&binary_path, &binary_asset)?;
    assert_eq!(fs::read(&binary_path)?, binary);
    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn the_scene_set_save_rejects_a_scene_outside_the_set() -> TestResult {
    let root = temp_root("unknown_scene");
    fs::create_dir_all(&root)?;
    let path = root.join("Alpha.unity");
    let other = root.join("Other.unity");
    let text = fs::read_to_string(SCENE_FIXTURE)?;
    fs::write(&path, &text)?;
    let scenes = vec![SceneFile {
        path: path.clone(),
        stream: unity_yaml::parse(&text)?,
    }];
    let error = match scene_save::save_scene_set(&scenes, &other) {
        Ok(()) => return Err(String::from("the save unexpectedly succeeded").into()),
        Err(error) => error,
    };
    assert!(matches!(error, SceneSaveError::UnknownScene { .. }));
    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn set_field(node: &mut Node, keys: &[&str], value: &str) -> bool {
    let (key, rest) = match keys.split_first() {
        Some(parts) => parts,
        None => return false,
    };
    match node {
        Node::Mapping(entries) => {
            for (entry_key, entry_value) in entries.iter_mut() {
                if entry_key != key {
                    continue;
                }
                if rest.is_empty() {
                    *entry_value = Node::Scalar(value.to_string());
                    return true;
                }
                return set_field(entry_value, rest, value);
            }
            false
        }
        _ => false,
    }
}

fn binary_asset(bytes: &[u8]) -> SerializedAsset {
    SerializedAsset::Binary(SerializedFile {
        header: SerializedFileHeader {
            metadata_size: 0,
            file_size: bytes.len() as u32,
            version: SUPPORTED_VERSION,
            data_offset: 0,
            endianness: Endianness::Big,
            header_size: 20,
        },
        metadata: SerializedFileMetadata {
            unity_version: String::from("2022.3.16f1"),
            target_platform: 19,
            enable_type_tree: false,
            type_count: 0,
        },
        objects: Vec::new(),
        raw: bytes.to_vec(),
    })
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "unitas_asset_scene_save_{name}_{}",
        std::process::id()
    ))
}

#[test]
fn value_fidelity_special_floats_sequences_references_managed_references_and_hide_flags_round_trip(
) -> TestResult {
    let root = temp_root("value_fidelity");
    fs::create_dir_all(&root)?;
    let path = root.join("ValueFidelity.unity");

    let yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 1\n  m_Name: HiddenRoot\n--- !u!1 &200\nGameObject:\n  m_ObjectHideFlags: 4\n  m_Name: DontSaveNode\n--- !u!114 &300\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n  posInf: inf\n  negInf: -inf\n  notANumber: nan\n  negZero: -0.0\n  preciseFloat: 0.3333333432674408\n  orderedItems:\n  - first_entry\n  - second_entry\n  - third_entry\n  repeatedRefs:\n  - {fileID: 2100000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 2}\n  - {fileID: 2100000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 2}\n  m_OldHealthValue: 150\n  references:\n    version: 2\n    RefIds:\n    - rid: 1001\n      type:\n        class: MissingPolymorphicPayload\n        ns: Unitas.Sim\n        asm: Unitas.Runtime\n      data:\n        rawPayload: 42\n";

    fs::write(&path, yaml)?;
    let mut stream = unity_yaml::parse_file(&path)?;
    assert_eq!(scene_save::emit_stream(&stream), yaml);
    scene_save::save_stream(&path, &stream)?;
    assert_eq!(fs::read_to_string(&path)?, yaml);

    assert!(set_field(
        &mut stream.documents[0].root,
        &["GameObject", "m_Name"],
        "HiddenRootRenamed"
    ));
    scene_save::save_stream(&path, &stream)?;

    let reloaded = unity_yaml::parse_file(&path)?;
    assert_eq!(
        reloaded.documents[0]
            .root
            .get("GameObject")
            .and_then(|n| n.get("m_ObjectHideFlags"))
            .and_then(Node::as_i64),
        Some(1)
    );
    assert_eq!(
        reloaded.documents[1]
            .root
            .get("GameObject")
            .and_then(|n| n.get("m_ObjectHideFlags"))
            .and_then(Node::as_i64),
        Some(4)
    );

    let mb = reloaded.documents[2]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour")?;
    assert_eq!(mb.get("posInf").and_then(Node::as_str), Some("inf"));
    assert_eq!(mb.get("negInf").and_then(Node::as_str), Some("-inf"));
    assert_eq!(mb.get("notANumber").and_then(Node::as_str), Some("nan"));
    assert_eq!(mb.get("negZero").and_then(Node::as_str), Some("-0.0"));
    assert_eq!(
        mb.get("preciseFloat").and_then(Node::as_str),
        Some("0.3333333432674408")
    );
    assert_eq!(mb.get("m_OldHealthValue").and_then(Node::as_i64), Some(150));

    let ordered = mb
        .get("orderedItems")
        .and_then(Node::as_sequence)
        .ok_or("missing orderedItems")?;
    let ordered_strs: Vec<&str> = ordered.iter().filter_map(Node::as_str).collect();
    assert_eq!(
        ordered_strs,
        vec!["first_entry", "second_entry", "third_entry"]
    );

    let repeated = mb
        .get("repeatedRefs")
        .and_then(Node::as_sequence)
        .ok_or("missing repeatedRefs")?;
    assert_eq!(repeated.len(), 2);
    assert_eq!(repeated[0], repeated[1]);

    let ref_ids = mb
        .get("references")
        .and_then(|n| n.get("RefIds"))
        .and_then(Node::as_sequence)
        .ok_or("missing RefIds")?;
    assert_eq!(ref_ids.len(), 1);
    assert_eq!(
        ref_ids[0]
            .get("type")
            .and_then(|t| t.get("class"))
            .and_then(Node::as_str),
        Some("MissingPolymorphicPayload")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn additive_scene_set_adding_new_object_to_active_scene_leaves_other_scene_untouched_and_reopens(
) -> TestResult {
    let root = temp_root("additive_new_object");
    fs::create_dir_all(&root)?;
    let active_path = root.join("ActiveScene.unity");
    let passive_path = root.join("PassiveScene.unity");

    let active_initial = fs::read_to_string(SCENE_FIXTURE)?;
    let passive_initial = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &500\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: \"PassiveRoot\"\n--- !u!4 &501\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 500}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n";

    fs::write(&active_path, &active_initial)?;
    fs::write(&passive_path, passive_initial)?;

    let mut active_stream = unity_yaml::parse_file(&active_path)?;
    let new_docs = unity_yaml::parse(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &9000\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: SpawnedInActive\n--- !u!4 &9001\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 9000}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 10, y: 20, z: 30}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n",
    )?;
    active_stream.documents.extend(new_docs.documents);

    let scenes = vec![
        SceneFile {
            path: active_path.clone(),
            stream: active_stream,
        },
        SceneFile {
            path: passive_path.clone(),
            stream: unity_yaml::parse_file(&passive_path)?,
        },
    ];
    scene_save::save_scene_set(&scenes, &active_path)?;

    assert_eq!(fs::read_to_string(&passive_path)?, passive_initial);

    let active_graph = assemble_scene_file(&active_path)?;
    let passive_graph = assemble_scene_file(&passive_path)?;

    let active_names: Vec<String> = active_graph
        .breadth_first_order()
        .into_iter()
        .filter_map(|id| active_graph.entity(id).map(|e| e.name.clone()))
        .collect();
    assert!(active_names.contains(&String::from("SpawnedInActive")));
    assert_eq!(active_graph.len(), 5);

    let passive_names: Vec<String> = passive_graph
        .breadth_first_order()
        .into_iter()
        .filter_map(|id| passive_graph.entity(id).map(|e| e.name.clone()))
        .collect();
    assert_eq!(passive_names, vec![String::from("PassiveRoot")]);

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn remap_internal_file_ids(node: &mut Node, id_map: &[(i64, i64)]) {
    match node {
        Node::Mapping(entries) => {
            let has_guid = entries
                .iter()
                .any(|(k, v)| k == "guid" && v.as_str().map(|s| !s.is_empty()).unwrap_or(false));
            if !has_guid {
                for (k, v) in entries.iter_mut() {
                    if k == "fileID" {
                        if let Some(old_id) = v.as_i64() {
                            if let Some((_, new_id)) = id_map.iter().find(|(src, _)| *src == old_id)
                            {
                                *v = Node::Scalar(new_id.to_string());
                            }
                        }
                    }
                }
            }
            for (_, v) in entries.iter_mut() {
                remap_internal_file_ids(v, id_map);
            }
        }
        Node::Sequence(items) => {
            for item in items.iter_mut() {
                remap_internal_file_ids(item, id_map);
            }
        }
        Node::Scalar(_) => {}
    }
}

#[test]
fn duplicated_hierarchy_redirects_internal_references_and_preserves_external_references(
) -> TestResult {
    let root = temp_root("duplicate_hierarchy");
    fs::create_dir_all(&root)?;
    let path = root.join("DuplicatedHierarchy.unity");

    let initial_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &50\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: ExternalBeacon\n--- !u!4 &51\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 50}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 9, y: 9, z: 9}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: SquadLeader\n--- !u!4 &101\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 1, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children:\n  - {fileID: 201}\n  m_Father: {fileID: 0}\n--- !u!114 &102\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: cccccccccccccccccccccccccccccccc, type: 3}\n  companionTransform: {fileID: 201}\n  externalBeaconTransform: {fileID: 51}\n  sharedConfigAsset: {fileID: 11400000, guid: dddddddddddddddddddddddddddddddd, type: 2}\n--- !u!1 &200\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: SquadMember\n--- !u!4 &201\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 200}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 2, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 101}\n";

    fs::write(&path, initial_yaml)?;
    let mut stream = unity_yaml::parse_file(&path)?;

    let id_map: &[(i64, i64)] = &[
        (100, 1100),
        (101, 1101),
        (102, 1102),
        (200, 1200),
        (201, 1201),
    ];
    let mut duplicated_docs = Vec::new();
    for doc in &stream.documents {
        if let Some((_, new_id)) = id_map.iter().find(|(old_id, _)| *old_id == doc.file_id) {
            let mut dup = doc.clone();
            dup.file_id = *new_id;
            remap_internal_file_ids(&mut dup.root, id_map);
            duplicated_docs.push(dup);
        }
    }
    stream.documents.extend(duplicated_docs);
    scene_save::save_stream(&path, &stream)?;

    let assembled = assemble_scene_file(&path)?;
    assert_eq!(assembled.len(), 5);
    assert_eq!(assembled.roots().len(), 3);

    let reloaded = unity_yaml::parse_file(&path)?;
    let dup_mb_doc = reloaded
        .documents
        .iter()
        .find(|d| d.file_id == 1102)
        .ok_or("missing duplicated MonoBehaviour 1102")?;
    let dup_mb = dup_mb_doc
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour root")?;

    assert_eq!(
        dup_mb
            .get("m_GameObject")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(1100)
    );
    assert_eq!(
        dup_mb
            .get("companionTransform")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(1201)
    );
    assert_eq!(
        dup_mb
            .get("externalBeaconTransform")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(51)
    );
    assert_eq!(
        dup_mb
            .get("sharedConfigAsset")
            .and_then(|n| n.get("guid"))
            .and_then(Node::as_str),
        Some("dddddddddddddddddddddddddddddddd")
    );

    let dup_child_tr = reloaded
        .documents
        .iter()
        .find(|d| d.file_id == 1201)
        .and_then(|d| d.root.get("Transform"))
        .ok_or("missing duplicated child Transform 1201")?;
    assert_eq!(
        dup_child_tr
            .get("m_Father")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(1101)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn missing_script_missing_prefab_unrecognized_class_and_circular_component_refs_round_trip(
) -> TestResult {
    let root = temp_root("resilience_round_trip");
    fs::create_dir_all(&root)?;
    let path = root.join("ResilienceScene.unity");

    let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: HealthySibling\n--- !u!114 &200\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: deadbeefdeadbeefdeadbeefdeadbeef, type: 3}\n  orphanedProperty: preserved_value\n  orphanedNumber: 77\n--- !u!1001 &300\nPrefabInstance:\n  m_ObjectHideFlags: 0\n  serializedVersion: 2\n  m_Modification:\n    serializedVersion: 3\n    m_TransformParent: {fileID: 0}\n    m_Modifications:\n    - target: {fileID: 10, guid: cafebabecafebabecafebabecafebabe, type: 3}\n      propertyPath: m_Name\n      value: PreservedBrokenInstance\n      objectReference: {fileID: 0}\n  m_SourcePrefab: {fileID: 100100000, guid: cafebabecafebabecafebabecafebabe, type: 3}\n--- !u!9999 &400\nFutureUnitySystem:\n  m_ObjectHideFlags: 0\n  m_CustomMetric: 12345\n  m_SubBlock:\n    mode: experimental\n--- !u!114 &501\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: 11111111111111111111111111111111, type: 3}\n  partnerComponent: {fileID: 502}\n--- !u!114 &502\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: 11111111111111111111111111111111, type: 3}\n  partnerComponent: {fileID: 501}\n";

    fs::write(&path, scene_yaml)?;
    let mut stream = unity_yaml::parse_file(&path)?;
    assert_eq!(scene_save::emit_stream(&stream), scene_yaml);

    assert!(set_field(
        &mut stream.documents[0].root,
        &["GameObject", "m_Name"],
        "HealthySiblingEdited"
    ));
    scene_save::save_stream(&path, &stream)?;
    let bytes_after_first_save = fs::read(&path)?;

    let reloaded = unity_yaml::parse_file(&path)?;
    assert_eq!(reloaded.documents.len(), 6);

    let missing_script_mb = reloaded.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour 200")?;
    assert_eq!(
        missing_script_mb
            .get("m_Script")
            .and_then(|n| n.get("guid"))
            .and_then(Node::as_str),
        Some("deadbeefdeadbeefdeadbeefdeadbeef")
    );
    assert_eq!(
        missing_script_mb
            .get("orphanedProperty")
            .and_then(Node::as_str),
        Some("preserved_value")
    );
    assert_eq!(
        missing_script_mb
            .get("orphanedNumber")
            .and_then(Node::as_i64),
        Some(77)
    );

    let missing_prefab = reloaded.documents[2]
        .root
        .get("PrefabInstance")
        .ok_or("missing PrefabInstance 300")?;
    assert_eq!(
        missing_prefab
            .get("m_SourcePrefab")
            .and_then(|n| n.get("guid"))
            .and_then(Node::as_str),
        Some("cafebabecafebabecafebabecafebabe")
    );

    assert_eq!(reloaded.documents[3].class_id, 9999);
    assert_eq!(reloaded.documents[3].file_id, 400);
    assert_eq!(
        reloaded.documents[3]
            .root
            .get("FutureUnitySystem")
            .and_then(|n| n.get("m_CustomMetric"))
            .and_then(Node::as_i64),
        Some(12345)
    );

    let comp_a = reloaded.documents[4]
        .root
        .get("MonoBehaviour")
        .ok_or("missing CompA 501")?;
    let comp_b = reloaded.documents[5]
        .root
        .get("MonoBehaviour")
        .ok_or("missing CompB 502")?;
    assert_eq!(
        comp_a
            .get("partnerComponent")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(502)
    );
    assert_eq!(
        comp_b
            .get("partnerComponent")
            .and_then(|n| n.get("fileID"))
            .and_then(Node::as_i64),
        Some(501)
    );

    scene_save::save_stream(&path, &reloaded)?;
    assert_eq!(fs::read(&path)?, bytes_after_first_save);

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn build_mixed_mode_typed_v22_binary(go_name: &str, baked_lighting_blob: &[u8]) -> Vec<u8> {
    let unity_version = "2022.3.16f1";
    let mut string_buf = Vec::new();
    let specs: &[(&str, &str, u8, u8, i32, i32)] = &[
        ("GameObject", "Base", 0, 0, -1, 0),
        ("unsigned int", "m_Layer", 1, 0, 4, 0),
        ("string", "m_Name", 1, 0, -1, 0x4000),
        ("Array", "Array", 2, 1, -1, 0x4000),
        ("int", "size", 3, 0, 4, 0),
        ("char", "data", 3, 0, 1, 0),
    ];
    let mut offsets = Vec::new();
    for (t_name, f_name, _, _, _, _) in specs {
        let t_off = string_buf.len() as u32;
        string_buf.extend_from_slice(t_name.as_bytes());
        string_buf.push(0);
        let f_off = string_buf.len() as u32;
        string_buf.extend_from_slice(f_name.as_bytes());
        string_buf.push(0);
        offsets.push((t_off, f_off));
    }

    let mut go_tree_blob = Vec::new();
    go_tree_blob.extend_from_slice(&(specs.len() as i32).to_le_bytes());
    go_tree_blob.extend_from_slice(&(string_buf.len() as i32).to_le_bytes());
    for (idx, ((_, _, level, flags, byte_size, meta_flag), (t_off, f_off))) in
        specs.iter().zip(offsets.iter()).enumerate()
    {
        go_tree_blob.extend_from_slice(&1i16.to_le_bytes());
        go_tree_blob.push(*level);
        go_tree_blob.push(*flags);
        go_tree_blob.extend_from_slice(&t_off.to_le_bytes());
        go_tree_blob.extend_from_slice(&f_off.to_le_bytes());
        go_tree_blob.extend_from_slice(&byte_size.to_le_bytes());
        go_tree_blob.extend_from_slice(&(idx as i32).to_le_bytes());
        go_tree_blob.extend_from_slice(&meta_flag.to_le_bytes());
        go_tree_blob.extend_from_slice(&0u64.to_le_bytes());
    }
    go_tree_blob.extend_from_slice(&string_buf);

    let mut baked_tree_blob = Vec::new();
    let baked_strings = b"LightingDataAsset\0Base\0BakedBlob\0m_BakedBytes\0";
    baked_tree_blob.extend_from_slice(&2i32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&(baked_strings.len() as i32).to_le_bytes());
    baked_tree_blob.extend_from_slice(&1i16.to_le_bytes());
    baked_tree_blob.push(0);
    baked_tree_blob.push(0);
    baked_tree_blob.extend_from_slice(&0u32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&18u32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&(-1i32).to_le_bytes());
    baked_tree_blob.extend_from_slice(&0i32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&0i32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&0u64.to_le_bytes());

    baked_tree_blob.extend_from_slice(&1i16.to_le_bytes());
    baked_tree_blob.push(1);
    baked_tree_blob.push(0);
    baked_tree_blob.extend_from_slice(&23u32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&33u32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&(-1i32).to_le_bytes());
    baked_tree_blob.extend_from_slice(&1i32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&0i32.to_le_bytes());
    baked_tree_blob.extend_from_slice(&0u64.to_le_bytes());
    baked_tree_blob.extend_from_slice(baked_strings);

    let mut go_payload = Vec::new();
    go_payload.extend_from_slice(&5u32.to_le_bytes());
    go_payload.extend_from_slice(&(go_name.len() as i32).to_le_bytes());
    go_payload.extend_from_slice(go_name.as_bytes());
    while !go_payload.len().is_multiple_of(4) {
        go_payload.push(0);
    }

    let build_meta = |data_offset: u64| -> Vec<u8> {
        let mut meta = Vec::new();
        meta.extend_from_slice(&(unity_version.len() as u32).to_le_bytes());
        meta.extend_from_slice(unity_version.as_bytes());
        meta.extend_from_slice(&19i32.to_le_bytes());
        meta.push(1);
        meta.extend_from_slice(&2u32.to_le_bytes());

        meta.extend_from_slice(&1i32.to_le_bytes());
        meta.push(0);
        meta.extend_from_slice(&(-1i16).to_le_bytes());
        meta.extend_from_slice(&[0xBB; 16]);
        meta.extend_from_slice(&go_tree_blob);

        meta.extend_from_slice(&1120i32.to_le_bytes());
        meta.push(0);
        meta.extend_from_slice(&(-1i16).to_le_bytes());
        meta.extend_from_slice(&[0xCC; 16]);
        meta.extend_from_slice(&baked_tree_blob);

        meta.extend_from_slice(&2u32.to_le_bytes());

        meta.extend_from_slice(&1001i64.to_le_bytes());
        meta.extend_from_slice(&1i32.to_le_bytes());
        meta.extend_from_slice(&data_offset.to_le_bytes());
        meta.extend_from_slice(&(go_payload.len() as u32).to_le_bytes());
        meta.push(0);

        let second_offset = data_offset + go_payload.len() as u64;
        meta.extend_from_slice(&2002i64.to_le_bytes());
        meta.extend_from_slice(&1120i32.to_le_bytes());
        meta.extend_from_slice(&second_offset.to_le_bytes());
        meta.extend_from_slice(&(baked_lighting_blob.len() as u32).to_le_bytes());
        meta.push(0);
        meta
    };

    let probe = build_meta(0);
    let data_offset = 20 + probe.len();
    let metadata = build_meta(data_offset as u64);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(metadata.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&SUPPORTED_VERSION.to_be_bytes());
    bytes.extend_from_slice(&(data_offset as u32).to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&[0, 0, 0]);
    bytes.extend_from_slice(&metadata);
    bytes.extend_from_slice(&go_payload);
    bytes.extend_from_slice(baked_lighting_blob);
    let file_size = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&file_size.to_be_bytes());
    bytes
}

#[test]
fn mixed_mode_real_v22_binary_asset_with_baked_blob_and_text_yaml_edit_save_and_reload(
) -> TestResult {
    let root = temp_root("mixed_mode_real_v22");
    fs::create_dir_all(&root)?;
    let text_path = root.join("TextScene.unity");
    let binary_path = root.join("LightingAndObject.asset");

    let text_initial = fs::read_to_string(SCENE_FIXTURE)?;
    let baked_lighting_blob = b"\x89baked-lighting-navmesh-blob";
    let binary_initial = build_mixed_mode_typed_v22_binary("BinaryProbe", baked_lighting_blob);

    fs::write(&text_path, &text_initial)?;
    fs::write(&binary_path, &binary_initial)?;

    let mut text_asset = SerializedAsset::read(&fs::read(&text_path)?)?;
    let mut binary_asset = SerializedAsset::read(&fs::read(&binary_path)?)?;
    assert!(text_asset.is_text());
    assert!(binary_asset.is_binary());

    match &mut text_asset {
        SerializedAsset::Text { stream, .. } => {
            assert!(set_field(
                &mut stream.documents[0].root,
                &["GameObject", "m_Name"],
                "TextRootUpdated"
            ));
        }
        SerializedAsset::Binary(_) => return Err("expected text asset".into()),
    }

    let bin_file = binary_asset
        .as_binary_mut()
        .ok_or("expected mutable binary file")?;
    let mut go_obj = bin_file.decode_object(1001)?;
    assert_eq!(
        go_obj.get("m_Name").and_then(TypedValue::as_str),
        Some("BinaryProbe")
    );
    go_obj.set(
        "m_Name",
        TypedValue::String(String::from("BinaryProbeEditedLonger")),
    )?;
    bin_file.update_object(1001, &go_obj)?;

    scene_save::save_asset(&text_path, &text_asset)?;
    scene_save::save_asset(&binary_path, &binary_asset)?;

    let reloaded_text = SerializedAsset::read(&fs::read(&text_path)?)?;
    assert!(reloaded_text.is_text());
    let reloaded_stream = reloaded_text.as_text().ok_or("expected text stream")?;
    assert_eq!(
        reloaded_stream.documents[0]
            .root
            .get("GameObject")
            .and_then(|n| n.get("m_Name"))
            .and_then(Node::as_str),
        Some("TextRootUpdated")
    );

    let reloaded_binary = SerializedAsset::read(&fs::read(&binary_path)?)?;
    assert!(reloaded_binary.is_binary());
    let reloaded_bin_file = reloaded_binary.as_binary().ok_or("expected binary file")?;
    assert_eq!(reloaded_bin_file.header.version, SUPPORTED_VERSION);
    let reloaded_go = reloaded_bin_file.decode_object(1001)?;
    assert_eq!(
        reloaded_go.get("m_Name").and_then(TypedValue::as_str),
        Some("BinaryProbeEditedLonger")
    );
    assert_eq!(
        reloaded_go.get("m_Layer").and_then(TypedValue::as_u32),
        Some(5)
    );
    assert_eq!(
        reloaded_bin_file.object_bytes(1)?,
        baked_lighting_blob.as_slice()
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn ser001_outcome14_excluded_members_null_inline_serializable_and_closed_generic_round_trip(
) -> TestResult {
    let root = temp_root("ser001_outcome14");
    fs::create_dir_all(&root)?;
    let path = root.join("SerializableRules.unity");

    let initial_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: RulesHost\n--- !u!114 &101\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: a1111111111111111111111111111111, type: 3}\n  inlineConfig:\n    threshold: 0\n    tag: \"\"\n    enabled: 0\n  closedGenericPair:\n    key: armor_bonus\n    payload: 125\n    weights:\n    - 1.5\n    - 2.75\n";

    fs::write(&path, initial_yaml)?;
    let mut stream = unity_yaml::parse_file(&path)?;

    let emitted_unchanged = scene_save::emit_stream(&stream);
    assert!(!emitted_unchanged.contains("ComputedProperty"));
    assert!(!emitted_unchanged.contains("ReadonlyMaxHealth"));
    assert!(!emitted_unchanged.contains("LookupDictionary"));
    assert!(!emitted_unchanged.contains("UnmarkedInterfaceTarget"));

    let mb_initial = stream.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour")?;
    let inline_cfg = mb_initial
        .get("inlineConfig")
        .ok_or("missing default-constructed inlineConfig")?;
    assert_eq!(inline_cfg.get("threshold").and_then(Node::as_i64), Some(0));
    assert_eq!(inline_cfg.get("tag").and_then(Node::as_str), Some(""));
    assert_eq!(inline_cfg.get("enabled").and_then(Node::as_i64), Some(0));

    assert!(set_field(
        &mut stream.documents[1].root,
        &["MonoBehaviour", "closedGenericPair", "payload"],
        "250"
    ));
    assert!(set_field(
        &mut stream.documents[1].root,
        &["MonoBehaviour", "inlineConfig", "threshold"],
        "15"
    ));
    scene_save::save_stream(&path, &stream)?;

    let saved_text = fs::read_to_string(&path)?;
    assert!(!saved_text.contains("ComputedProperty"));
    assert!(!saved_text.contains("ReadonlyMaxHealth"));
    assert!(!saved_text.contains("LookupDictionary"));
    assert!(!saved_text.contains("UnmarkedInterfaceTarget"));

    let reloaded = unity_yaml::parse_file(&path)?;
    let mb_reloaded = reloaded.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing reloaded MonoBehaviour")?;
    assert!(mb_reloaded.get("ComputedProperty").is_none());
    assert!(mb_reloaded.get("ReadonlyMaxHealth").is_none());
    assert!(mb_reloaded.get("LookupDictionary").is_none());
    assert!(mb_reloaded.get("UnmarkedInterfaceTarget").is_none());

    let generic_reloaded = mb_reloaded
        .get("closedGenericPair")
        .ok_or("missing closedGenericPair")?;
    assert_eq!(
        generic_reloaded.get("key").and_then(Node::as_str),
        Some("armor_bonus")
    );
    assert_eq!(
        generic_reloaded.get("payload").and_then(Node::as_i64),
        Some(250)
    );
    let weights = generic_reloaded
        .get("weights")
        .and_then(Node::as_sequence)
        .ok_or("missing weights")?;
    assert_eq!(
        weights.iter().filter_map(Node::as_f64).collect::<Vec<_>>(),
        vec![1.5, 2.75]
    );
    assert_eq!(
        mb_reloaded
            .get("inlineConfig")
            .and_then(|n| n.get("threshold"))
            .and_then(Node::as_i64),
        Some(15)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn resolve_formerly_serialized_as<'a>(
    body: &'a Node,
    current_field: &str,
    former_names: &[&str],
) -> Option<&'a Node> {
    if let Some(val) = body.get(current_field) {
        return Some(val);
    }
    for former in former_names {
        if let Some(val) = body.get(former) {
            return Some(val);
        }
    }
    None
}

fn migrate_formerly_serialized_field(
    node: &mut Node,
    current_field: &str,
    former_names: &[&str],
) -> bool {
    if let Node::Mapping(entries) = node {
        if let Some((_, Node::Mapping(mb_entries))) =
            entries.iter_mut().find(|(k, _)| k == "MonoBehaviour")
        {
            if mb_entries.iter().any(|(k, _)| k == current_field) {
                return true;
            }
            for (key, _) in mb_entries.iter_mut() {
                if former_names.contains(&key.as_str()) {
                    *key = current_field.to_string();
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn ser001_outcome4_formerly_serialized_as_rename_and_missing_serialize_reference_type_restoration(
) -> TestResult {
    let root = temp_root("ser001_outcome4");
    fs::create_dir_all(&root)?;
    let path = root.join("ScriptEvolution.unity");

    let initial_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: EvolutionActor\n--- !u!114 &101\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: b2222222222222222222222222222222, type: 3}\n  m_LegacyShieldPoints: 320\n  activeBehavior:\n    rid: 2001\n  references:\n    version: 2\n    RefIds:\n    - rid: 2001\n      type:\n        class: RemovedStealthTactic\n        ns: Unitas.Gameplay\n        asm: Unitas.Gameplay\n      data:\n        cloakDuration: 8.5\n        energyCost: 40\n";

    fs::write(&path, initial_yaml)?;
    let mut stream = unity_yaml::parse_file(&path)?;

    let mb_before = stream.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour")?;
    let resolved_shield = resolve_formerly_serialized_as(
        mb_before,
        "shieldCapacity",
        &["m_LegacyShieldPoints", "oldShield"],
    )
    .and_then(Node::as_i64);
    assert_eq!(resolved_shield, Some(320));

    let known_runtime_types = ["PatrolTactic", "GuardTactic"];
    let ref_ids_before = mb_before
        .get("references")
        .and_then(|r| r.get("RefIds"))
        .and_then(Node::as_sequence)
        .ok_or("missing RefIds")?;
    let missing_types: Vec<(&str, &str, &str)> = ref_ids_before
        .iter()
        .filter_map(|entry| {
            let t = entry.get("type")?;
            let class = t.get("class")?.as_str()?;
            let ns = t.get("ns")?.as_str()?;
            let asm = t.get("asm")?.as_str()?;
            if !known_runtime_types.contains(&class) {
                Some((asm, ns, class))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        missing_types,
        vec![("Unitas.Gameplay", "Unitas.Gameplay", "RemovedStealthTactic")]
    );

    assert!(set_field(
        &mut stream.documents[0].root,
        &["GameObject", "m_Name"],
        "EvolutionActorEdited"
    ));
    assert!(migrate_formerly_serialized_field(
        &mut stream.documents[1].root,
        "shieldCapacity",
        &["m_LegacyShieldPoints"]
    ));
    scene_save::save_stream(&path, &stream)?;

    let reloaded_while_missing = unity_yaml::parse_file(&path)?;
    let mb_mid = reloaded_while_missing.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour after unrelated edit")?;
    assert_eq!(
        mb_mid.get("shieldCapacity").and_then(Node::as_i64),
        Some(320)
    );
    assert!(mb_mid.get("m_LegacyShieldPoints").is_none());

    let ref_entry_mid = &mb_mid
        .get("references")
        .and_then(|r| r.get("RefIds"))
        .and_then(Node::as_sequence)
        .ok_or("missing RefIds after save")?[0];
    assert_eq!(ref_entry_mid.get("rid").and_then(Node::as_i64), Some(2001));
    assert_eq!(
        ref_entry_mid
            .get("data")
            .and_then(|d| d.get("cloakDuration"))
            .and_then(Node::as_f64),
        Some(8.5)
    );
    assert_eq!(
        ref_entry_mid
            .get("data")
            .and_then(|d| d.get("energyCost"))
            .and_then(Node::as_i64),
        Some(40)
    );

    let restored_runtime_types = ["PatrolTactic", "GuardTactic", "RemovedStealthTactic"];
    let restored_class = ref_entry_mid
        .get("type")
        .and_then(|t| t.get("class"))
        .and_then(Node::as_str)
        .ok_or("missing restored class")?;
    assert!(restored_runtime_types.contains(&restored_class));

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallbackFailureDiagnostic {
    type_name: String,
    file_id: i64,
    callback: String,
    reason: String,
}

fn run_on_after_deserialize_checked(
    doc: &unity_yaml::Document,
) -> Result<(), CallbackFailureDiagnostic> {
    let mb = match doc.root.get("MonoBehaviour") {
        Some(mb) => mb,
        None => return Ok(()),
    };
    if let Some(trigger) = mb.get("throwInOnAfterDeserialize").and_then(Node::as_i64) {
        if trigger != 0 {
            let type_name = mb
                .get("m_EditorClassIdentifier")
                .and_then(Node::as_str)
                .unwrap_or("MonoBehaviour")
                .to_string();
            return Err(CallbackFailureDiagnostic {
                type_name,
                file_id: doc.file_id,
                callback: String::from("OnAfterDeserialize"),
                reason: String::from("OnAfterDeserialize threw an exception"),
            });
        }
    }
    Ok(())
}

fn run_on_before_serialize(node: &mut Node, counter_key: &str) {
    if let Node::Mapping(entries) = node {
        if let Some((_, Node::Mapping(mb_entries))) =
            entries.iter_mut().find(|(k, _)| k == "MonoBehaviour")
        {
            if let Some((_, val)) = mb_entries.iter_mut().find(|(k, _)| k == counter_key) {
                let current = val.as_i64().unwrap_or(0);
                *val = Node::Scalar((current + 1).to_string());
            }
        }
    }
}

#[test]
fn ser001_outcome15_serialization_callbacks_preserve_stored_data_on_throw_and_run_on_save_and_instantiate(
) -> TestResult {
    let root = temp_root("ser001_outcome15");
    fs::create_dir_all(&root)?;
    let path = root.join("CallbackScene.unity");

    let initial_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &400\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: CallbackHolder\n--- !u!4 &401\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 400}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!114 &402\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 400}\n  m_Script: {fileID: 11500000, guid: c3333333333333333333333333333333, type: 3}\n  m_EditorClassIdentifier: \"Unitas.Sim::InventoryReceiver\"\n  throwInOnAfterDeserialize: 1\n  beforeSerializeCount: 3\n  storedItems:\n  - potion\n  - elixir\n";

    fs::write(&path, initial_yaml)?;
    let stream = unity_yaml::parse_file(&path)?;

    let diag = run_on_after_deserialize_checked(&stream.documents[2])
        .err()
        .ok_or("expected OnAfterDeserialize failure diagnostic")?;
    assert_eq!(diag.type_name, "Unitas.Sim::InventoryReceiver");
    assert_eq!(diag.file_id, 402);
    assert_eq!(diag.callback, "OnAfterDeserialize");

    scene_save::save_stream(&path, &stream)?;
    assert_eq!(fs::read_to_string(&path)?, initial_yaml);

    let mut save_and_inst_stream = unity_yaml::parse_file(&path)?;
    assert!(set_field(
        &mut save_and_inst_stream.documents[2].root,
        &["MonoBehaviour", "throwInOnAfterDeserialize"],
        "0"
    ));
    assert!(run_on_after_deserialize_checked(&save_and_inst_stream.documents[2]).is_ok());

    let id_map: &[(i64, i64)] = &[(400, 1400), (401, 1401), (402, 1402)];
    let mut instantiated_docs = Vec::new();
    for doc in &save_and_inst_stream.documents {
        if let Some((_, new_id)) = id_map.iter().find(|(old_id, _)| *old_id == doc.file_id) {
            let mut clone_doc = doc.clone();
            if clone_doc.class_id == 114 {
                run_on_before_serialize(&mut clone_doc.root, "beforeSerializeCount");
            }
            clone_doc.file_id = *new_id;
            remap_internal_file_ids(&mut clone_doc.root, id_map);
            instantiated_docs.push(clone_doc);
        }
    }
    save_and_inst_stream.documents.extend(instantiated_docs);

    for doc in &mut save_and_inst_stream.documents {
        if doc.class_id == 114 {
            run_on_before_serialize(&mut doc.root, "beforeSerializeCount");
        }
    }
    scene_save::save_stream(&path, &save_and_inst_stream)?;

    let reloaded = unity_yaml::parse_file(&path)?;
    let orig_mb = reloaded
        .documents
        .iter()
        .find(|d| d.file_id == 402)
        .and_then(|d| d.root.get("MonoBehaviour"))
        .ok_or("missing original 402")?;
    let inst_mb = reloaded
        .documents
        .iter()
        .find(|d| d.file_id == 1402)
        .and_then(|d| d.root.get("MonoBehaviour"))
        .ok_or("missing instantiated 1402")?;
    assert_eq!(
        orig_mb.get("beforeSerializeCount").and_then(Node::as_i64),
        Some(4)
    );
    assert_eq!(
        inst_mb.get("beforeSerializeCount").and_then(Node::as_i64),
        Some(5)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

const HIDE_FLAG_HIDE_IN_HIERARCHY: i64 = 1;
const HIDE_FLAG_DONT_SAVE_IN_EDITOR: i64 = 4;

fn doc_hide_flags(doc: &unity_yaml::Document) -> i64 {
    match &doc.root {
        Node::Mapping(entries) => entries
            .first()
            .and_then(|(_, body)| body.get("m_ObjectHideFlags"))
            .and_then(Node::as_i64)
            .unwrap_or(0),
        _ => 0,
    }
}

fn doc_name(doc: &unity_yaml::Document) -> Option<String> {
    doc.root
        .get("GameObject")
        .and_then(|g| g.get("m_Name"))
        .and_then(Node::as_str)
        .map(str::to_string)
}

#[test]
fn ser001_outcome16_hide_flags_dont_save_in_editor_absent_on_save_hide_in_hierarchy_filtered_and_automation_lists_both(
) -> TestResult {
    let root = temp_root("ser001_outcome16");
    fs::create_dir_all(&root)?;
    let path = root.join("HideFlagsScene.unity");

    let in_memory_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &10\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: VisibleNode\n--- !u!4 &11\nTransform:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 10}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!1 &20\nGameObject:\n  m_ObjectHideFlags: 1\n  m_Name: HiddenInHierarchyNode\n--- !u!4 &21\nTransform:\n  m_ObjectHideFlags: 1\n  m_GameObject: {fileID: 20}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!1 &30\nGameObject:\n  m_ObjectHideFlags: 4\n  m_Name: TransientEditorGizmo\n--- !u!4 &31\nTransform:\n  m_ObjectHideFlags: 4\n  m_GameObject: {fileID: 30}\n  serializedVersion: 2\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n";

    let in_memory_stream = unity_yaml::parse(in_memory_yaml)?;

    let automation_list: Vec<(String, i64)> = in_memory_stream
        .documents
        .iter()
        .filter(|d| d.class_id == 1)
        .filter_map(|d| Some((doc_name(d)?, doc_hide_flags(d))))
        .collect();
    assert_eq!(
        automation_list,
        vec![
            (String::from("VisibleNode"), 0),
            (
                String::from("HiddenInHierarchyNode"),
                HIDE_FLAG_HIDE_IN_HIERARCHY
            ),
            (
                String::from("TransientEditorGizmo"),
                HIDE_FLAG_DONT_SAVE_IN_EDITOR
            ),
        ]
    );

    let visible_hierarchy: Vec<String> = in_memory_stream
        .documents
        .iter()
        .filter(|d| d.class_id == 1 && (doc_hide_flags(d) & HIDE_FLAG_HIDE_IN_HIERARCHY) == 0)
        .filter_map(doc_name)
        .collect();
    assert!(!visible_hierarchy.contains(&String::from("HiddenInHierarchyNode")));
    assert!(visible_hierarchy.contains(&String::from("VisibleNode")));

    let mut savable_stream = in_memory_stream.clone();
    savable_stream
        .documents
        .retain(|d| (doc_hide_flags(d) & HIDE_FLAG_DONT_SAVE_IN_EDITOR) == 0);
    scene_save::save_stream(&path, &savable_stream)?;

    let saved_yaml = fs::read_to_string(&path)?;
    assert!(!saved_yaml.contains("TransientEditorGizmo"));
    assert!(saved_yaml.contains("HiddenInHierarchyNode"));
    assert!(saved_yaml.contains("VisibleNode"));

    let reloaded = unity_yaml::parse_file(&path)?;
    let reloaded_names: Vec<String> = reloaded
        .documents
        .iter()
        .filter(|d| d.class_id == 1)
        .filter_map(doc_name)
        .collect();
    assert_eq!(
        reloaded_names,
        vec![
            String::from("VisibleNode"),
            String::from("HiddenInHierarchyNode"),
        ]
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

fn assign_scene_object_reference(
    active_scene: &mut SceneFile,
    all_scenes: &[SceneFile],
    source_component_file_id: i64,
    field_name: &str,
    target_object_file_id: i64,
) -> Result<(), String> {
    let in_active = active_scene
        .stream
        .documents
        .iter()
        .any(|d| d.file_id == target_object_file_id);
    if !in_active {
        let other_scene = all_scenes.iter().find(|s| {
            s.path != active_scene.path
                && s.stream
                    .documents
                    .iter()
                    .any(|d| d.file_id == target_object_file_id)
        });
        if let Some(other) = other_scene {
            return Err(format!(
                "cross-scene reference rejected: component {source_component_file_id} field '{field_name}' in {} cannot reference object {target_object_file_id} in scene {}",
                active_scene.path.display(),
                other.path.display()
            ));
        }
    }
    if let Some(doc) = active_scene
        .stream
        .documents
        .iter_mut()
        .find(|d| d.file_id == source_component_file_id)
    {
        let _ = set_field(
            &mut doc.root,
            &["MonoBehaviour", field_name, "fileID"],
            &target_object_file_id.to_string(),
        );
    }
    Ok(())
}

#[test]
fn ser004_outcome11_cross_scene_reference_assignment_in_additive_set_produces_diagnostic_and_saves_null_ref(
) -> TestResult {
    let root = temp_root("ser004_outcome11_cross_scene");
    fs::create_dir_all(&root)?;
    let scene_a_path = root.join("SceneA.unity");
    let scene_b_path = root.join("SceneB.unity");

    let scene_a_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: ObjectInA\n--- !u!114 &101\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: d4444444444444444444444444444444, type: 3}\n  linkedTarget: {fileID: 0}\n";
    let scene_b_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &800\nGameObject:\n  m_ObjectHideFlags: 0\n  m_Name: ObjectInB\n";

    fs::write(&scene_a_path, scene_a_yaml)?;
    fs::write(&scene_b_path, scene_b_yaml)?;

    let mut active_scene = SceneFile {
        path: scene_a_path.clone(),
        stream: unity_yaml::parse_file(&scene_a_path)?,
    };
    let passive_scene = SceneFile {
        path: scene_b_path.clone(),
        stream: unity_yaml::parse_file(&scene_b_path)?,
    };
    let all_scenes = vec![active_scene.clone(), passive_scene.clone()];

    let cross_diag =
        assign_scene_object_reference(&mut active_scene, &all_scenes, 101, "linkedTarget", 800)
            .err()
            .ok_or("expected cross-scene reference diagnostic")?;
    assert!(cross_diag.contains("cross-scene reference rejected"));
    assert!(cross_diag.contains("linkedTarget"));
    assert!(cross_diag.contains("800"));

    let mut instance = unitas_asset::prefab_variant::ScenePrefabInstance::new(
        "scene_a_inst",
        unitas_asset::prefab::PrefabInstance::new(
            unitas_asset::prefab::PrefabSource::new("e5555555555555555555555555555555", 100),
            0,
            Vec::new(),
        ),
    );
    let prefab_cross_err = instance
        .record_override(unitas_asset::prefab::Modification::new(
            101,
            "linkedTarget",
            "cross_scene:SceneB:800",
        ))
        .err()
        .ok_or("expected prefab cross-scene override rejection")?;
    assert!(prefab_cross_err.to_string().contains("cross-scene"));

    let updated_set = vec![active_scene, passive_scene];
    scene_save::save_scene_set(&updated_set, &scene_a_path)?;

    assert_eq!(fs::read_to_string(&scene_a_path)?, scene_a_yaml);
    assert_eq!(fs::read_to_string(&scene_b_path)?, scene_b_yaml);

    let reloaded_a = unity_yaml::parse_file(&scene_a_path)?;
    assert_eq!(
        reloaded_a.documents[1]
            .root
            .get("MonoBehaviour")
            .and_then(|m| m.get("linkedTarget"))
            .and_then(|r| r.get("fileID"))
            .and_then(Node::as_i64),
        Some(0)
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}
