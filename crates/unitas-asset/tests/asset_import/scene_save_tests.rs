use std::error::Error;
use std::fs;
use std::path::PathBuf;

use unitas_asset::binary_ser::{
    Endianness, SerializedAsset, SerializedFile, SerializedFileHeader, SerializedFileMetadata,
    SUPPORTED_VERSION,
};
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
        assert_eq!(scene_save::emit_stream(&stream), original, "emitted text differs from {fixture}");
    }
    Ok(())
}

#[test]
fn emit_stream_round_trips_the_parsed_stream() -> TestResult {
    let original = fs::read_to_string(SCENE_FIXTURE)?;
    let stream = unity_yaml::parse(&original)?;
    assert_eq!(unity_yaml::parse(&scene_save::emit_stream(&stream))?, stream);
    Ok(())
}

#[test]
fn editing_one_field_changes_only_that_field() -> TestResult {
    let original = fs::read_to_string(SCENE_FIXTURE)?;
    let mut stream = unity_yaml::parse(&original)?;
    assert!(set_field(&mut stream.documents[0].root, &["GameObject", "m_Name"], "Root Renamed"));
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
    assert_eq!(unity_yaml::parse(&scene_save::emit_stream(&stream))?, stream);
    Ok(())
}

#[test]
fn saving_one_scene_of_an_additive_set_writes_only_that_scene() -> TestResult {
    let root = temp_root("additive_set");
    fs::create_dir_all(&root)?;
    let alpha_path = root.join("Alpha.unity");
    let beta_path = root.join("Beta.unity");
    let alpha_text = fs::read_to_string(SCENE_FIXTURE)?;
    // the quoted name marks the file as never rewritten: a rewritten scene
    // would emit the plain name
    let beta_text = fs::read_to_string(SAMPLE_SCENE_FIXTURE)?
        .replacen("m_Name: Main Camera\n", "m_Name: \"Main Camera\"\n", 1);
    fs::write(&alpha_path, &alpha_text)?;
    fs::write(&beta_path, &beta_text)?;

    let mut alpha_stream = unity_yaml::parse(&alpha_text)?;
    assert!(set_field(
        &mut alpha_stream.documents[0].root,
        &["GameObject", "m_Name"],
        "Alpha"
    ));
    let scenes = vec![
        SceneFile { path: alpha_path.clone(), stream: alpha_stream },
        SceneFile { path: beta_path.clone(), stream: unity_yaml::parse(&beta_text)? },
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
    // the save writes a sibling temp file before it replaces the target, so a
    // directory in that place fails the save before the target is touched
    fs::create_dir(root.join("Scene.unity.tmp"))?;

    let mut stream = unity_yaml::parse(&original)?;
    assert!(set_field(&mut stream.documents[0].root, &["GameObject", "m_Name"], "Changed"));
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
            assert!(set_field(&mut stream.documents[0].root, &["GameObject", "m_Name"], "Edited"));
        }
        SerializedAsset::Binary(_) => return Err(String::from("the scene fixture is not text").into()),
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
    let scenes = vec![SceneFile { path: path.clone(), stream: unity_yaml::parse(&text)? }];
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
    std::env::temp_dir().join(format!("unitas_asset_scene_save_{name}_{}", std::process::id()))
}
