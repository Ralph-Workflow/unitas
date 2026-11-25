use std::error::Error;
use std::path::PathBuf;

use unitas_asset::merge::{
    merge_bytes, merge_files, merge_files_to_path, ConflictKind, MergeConflict, MergeError,
    MergeOutcome, StructuralKind,
};
use unitas_asset::unity_yaml;

type TestResult = Result<(), Box<dyn Error>>;

const BASE_SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";

const OURS_LAYER: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 5\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";

const THEIRS_NAME: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta Renamed\n  m_Layer: 0\n";

const BASE_REFERENCED: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n--- !u!114 &300\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: 0000000000000000f000000000000000, type: 3}\n  m_Target: {fileID: 200, guid: 11111111111111111111111111111111, type: 3}\n";

const OURS_REFERENCED: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 7\n--- !u!114 &300\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: 0000000000000000f000000000000000, type: 3}\n  m_Target: {fileID: 200, guid: 11111111111111111111111111111111, type: 3}\n";

const THEIRS_REFERENCED: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n--- !u!114 &300\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: 0000000000000000f000000000000000, type: 3}\n  m_Target: {fileID: 200, guid: 11111111111111111111111111111111, type: 3}\n";

const OURS_NAME_CONFLICT: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha Ours\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";

const THEIRS_NAME_CONFLICT: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha Theirs\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";

const OURS_DELETES_BETA: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n";

const THEIRS_CHANGES_BETA: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 9\n";

const BASE_HIERARCHY: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: A\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: B\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 0}\n";

const OURS_REPARENTS_A: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: A\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 210}\n--- !u!1 &200\nGameObject:\n  m_Name: B\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 0}\n";

const THEIRS_REPARENTS_B: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: A\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: B\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n";

const BASE_SINGLE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n";

const OURS_RETYPE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!4 &100\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n";

const THEIRS_RENAMES: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha Renamed\n";

const BASE_COMPONENTS: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Component:\n  - component: {fileID: 111}\n  - component: {fileID: 112}\n  - component: {fileID: 113}\n";

const OURS_COMPONENTS: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Component:\n  - component: {fileID: 112}\n  - component: {fileID: 111}\n  - component: {fileID: 113}\n";

const THEIRS_COMPONENTS: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Component:\n  - component: {fileID: 113}\n  - component: {fileID: 112}\n  - component: {fileID: 111}\n";

const BASE_META: &str = "fileFormatVersion: 2\nguid: 0000000000000000f000000000000000\nNativeFormatImporter:\n  externalObjects: {}\n  mainObjectFileID: 11400000\n  userData: \n  assetBundleName: \n";

const OURS_META: &str = "fileFormatVersion: 2\nguid: 0000000000000000f000000000000000\nNativeFormatImporter:\n  externalObjects: {}\n  mainObjectFileID: 2100000\n  userData: \n  assetBundleName: \n";

const THEIRS_META: &str = "fileFormatVersion: 2\nguid: 0000000000000000f000000000000000\nNativeFormatImporter:\n  externalObjects: {}\n  mainObjectFileID: 11400000\n  userData: \n  assetBundleName: bundle\n";

const OURS_DANGLING: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 0\n  m_Target: {fileID: 999}\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("unitas_merge_{name}_{}", std::process::id()))
}

fn property(text: &str, file_id: i64, path: &str) -> Option<String> {
    let stream = unity_yaml::parse(text).ok()?;
    let document = stream.documents.iter().find(|document| document.file_id == file_id)?;
    let mut node = &document.root;
    for segment in path.split('.') {
        node = node.get(segment)?;
    }
    node.as_scalar().map(|value| value.to_string())
}

fn file_ids(text: &str) -> Vec<i64> {
    match unity_yaml::parse(text) {
        Ok(stream) => {
            let mut ids: Vec<i64> = stream.documents.iter().map(|document| document.file_id).collect();
            ids.sort();
            ids
        }
        Err(_) => Vec::new(),
    }
}

fn merged_text(outcome: &MergeOutcome) -> &str {
    outcome.merged.as_deref().unwrap_or_default()
}

fn only_conflict(outcome: &MergeOutcome) -> Result<&MergeConflict, Box<dyn Error>> {
    match outcome.conflicts.first() {
        Some(conflict) if outcome.conflicts.len() == 1 => Ok(conflict),
        _ => Err(format!(
            "expected exactly one conflict, found {}",
            outcome.conflicts.len()
        )
        .into()),
    }
}

#[test]
fn merge_combines_changes_to_different_objects() -> TestResult {
    let outcome = merge_bytes(
        BASE_SCENE.as_bytes(),
        OURS_LAYER.as_bytes(),
        THEIRS_NAME.as_bytes(),
    )?;
    assert!(outcome.conflicts.is_empty());
    assert!(outcome.references.is_empty());
    assert_eq!(outcome.objects, 2);
    let merged = merged_text(&outcome);
    assert_eq!(
        property(merged, 100, "GameObject.m_Layer"),
        Some(String::from("5"))
    );
    assert_eq!(
        property(merged, 200, "GameObject.m_Name"),
        Some(String::from("Beta Renamed"))
    );
    Ok(())
}

#[test]
fn merge_combines_changes_to_different_properties_of_one_object() -> TestResult {
    let ours = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n  m_Layer: 5\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";
    let theirs = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha Renamed\n  m_Layer: 0\n--- !u!1 &200\nGameObject:\n  m_Name: Beta\n  m_Layer: 0\n";
    let outcome = merge_bytes(BASE_SCENE.as_bytes(), ours.as_bytes(), theirs.as_bytes())?;
    assert!(outcome.conflicts.is_empty());
    let merged = merged_text(&outcome);
    assert_eq!(
        property(merged, 100, "GameObject.m_Layer"),
        Some(String::from("5"))
    );
    assert_eq!(
        property(merged, 100, "GameObject.m_Name"),
        Some(String::from("Alpha Renamed"))
    );
    Ok(())
}

#[test]
fn merge_preserves_file_ids_and_guids_of_surviving_objects() -> TestResult {
    let outcome = merge_bytes(
        BASE_REFERENCED.as_bytes(),
        OURS_REFERENCED.as_bytes(),
        THEIRS_REFERENCED.as_bytes(),
    )?;
    assert!(outcome.conflicts.is_empty());
    let merged = merged_text(&outcome);
    assert_eq!(file_ids(merged), vec![100, 300]);
    assert!(merged.contains("0000000000000000f000000000000000"));
    assert!(merged.contains("11111111111111111111111111111111"));
    assert_eq!(
        property(merged, 300, "MonoBehaviour.m_Target.fileID"),
        Some(String::from("200"))
    );
    Ok(())
}

#[test]
fn merge_reports_property_conflict_with_base_and_both_values() -> TestResult {
    let outcome = merge_bytes(
        BASE_SCENE.as_bytes(),
        OURS_NAME_CONFLICT.as_bytes(),
        THEIRS_NAME_CONFLICT.as_bytes(),
    )?;
    let conflict = only_conflict(&outcome)?;
    assert_eq!(conflict.kind, ConflictKind::Property);
    assert_eq!(conflict.property.as_deref(), Some("m_Name"));
    assert_eq!(conflict.object_path, "Alpha");
    assert_eq!(conflict.file_id, 100);
    assert_eq!(conflict.base.as_deref(), Some("Alpha"));
    assert_eq!(conflict.ours.as_deref(), Some("Alpha Ours"));
    assert_eq!(conflict.theirs.as_deref(), Some("Alpha Theirs"));
    assert!(outcome.merged.is_none());
    Ok(())
}

#[test]
fn merge_reports_delete_versus_change_as_structural_conflict() -> TestResult {
    let outcome = merge_bytes(
        BASE_SCENE.as_bytes(),
        OURS_DELETES_BETA.as_bytes(),
        THEIRS_CHANGES_BETA.as_bytes(),
    )?;
    let conflict = only_conflict(&outcome)?;
    assert_eq!(
        conflict.kind,
        ConflictKind::Structural(StructuralKind::DeleteVersusChange {
            deleted_by: unitas_asset::merge::MergeSide::Ours
        })
    );
    assert_eq!(conflict.file_id, 200);
    assert_eq!(conflict.ours, None);
    assert!(conflict.theirs.is_some());
    assert!(conflict.base.is_some());
    assert!(outcome.merged.is_none());
    Ok(())
}

#[test]
fn merge_reports_reparent_cycle_as_structural_conflict() -> TestResult {
    let outcome = merge_bytes(
        BASE_HIERARCHY.as_bytes(),
        OURS_REPARENTS_A.as_bytes(),
        THEIRS_REPARENTS_B.as_bytes(),
    )?;
    let conflict = only_conflict(&outcome)?;
    let cycle = match &conflict.kind {
        ConflictKind::Structural(StructuralKind::ReparentCycle { cycle }) => cycle.clone(),
        other => return Err(format!("expected a reparent cycle conflict, found {other:?}").into()),
    };
    let mut members = cycle.clone();
    members.sort();
    assert_eq!(members, vec![110, 210]);
    assert_eq!(conflict.property.as_deref(), Some("m_Father"));
    assert!(outcome.merged.is_none());
    Ok(())
}

#[test]
fn merge_reports_file_id_reuse_as_structural_conflict() -> TestResult {
    let outcome = merge_bytes(
        BASE_SINGLE.as_bytes(),
        OURS_RETYPE.as_bytes(),
        THEIRS_RENAMES.as_bytes(),
    )?;
    let conflict = only_conflict(&outcome)?;
    assert_eq!(
        conflict.kind,
        ConflictKind::Structural(StructuralKind::FileIdReuse {
            base_class: Some(1),
            ours_class: Some(4),
            theirs_class: Some(1),
        })
    );
    assert_eq!(conflict.file_id, 100);
    assert!(outcome.merged.is_none());
    Ok(())
}

#[test]
fn merge_reports_conflicting_array_reordering_as_structural_conflict() -> TestResult {
    let outcome = merge_bytes(
        BASE_COMPONENTS.as_bytes(),
        OURS_COMPONENTS.as_bytes(),
        THEIRS_COMPONENTS.as_bytes(),
    )?;
    let conflict = only_conflict(&outcome)?;
    assert_eq!(
        conflict.kind,
        ConflictKind::Structural(StructuralKind::ArrayReorder)
    );
    assert_eq!(conflict.property.as_deref(), Some("m_Component"));
    assert!(outcome.merged.is_none());
    Ok(())
}

#[test]
fn merge_reports_binary_asset_as_not_mergeable() -> TestResult {
    let binary: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0xFE, 0xFF, 0x10, 0x20];
    let result = merge_bytes(&binary, &binary, &binary);
    let error = match result {
        Ok(outcome) => {
            return Err(format!("expected a not mergeable error, got {:?}", outcome.objects).into())
        }
        Err(error) => error,
    };
    match error {
        MergeError::NotMergeable { reason } => {
            assert!(reason.contains("binary"));
        }
        other => return Err(format!("expected a not mergeable error, found {other:?}").into()),
    }
    Ok(())
}

#[test]
fn merge_reports_unreadable_asset_type_as_not_mergeable() -> TestResult {
    let text = "this is not a unity asset\n";
    let result = merge_bytes(text.as_bytes(), text.as_bytes(), text.as_bytes());
    let error = match result {
        Ok(outcome) => {
            return Err(format!("expected a not mergeable error, got {:?}", outcome.objects).into())
        }
        Err(error) => error,
    };
    match error {
        MergeError::NotMergeable { reason } => {
            assert!(!reason.is_empty());
        }
        other => return Err(format!("expected a not mergeable error, found {other:?}").into()),
    }
    Ok(())
}

#[test]
fn merge_of_meta_asset_preserves_guid_and_combines_properties() -> TestResult {
    let outcome = merge_bytes(
        BASE_META.as_bytes(),
        OURS_META.as_bytes(),
        THEIRS_META.as_bytes(),
    )?;
    assert!(outcome.conflicts.is_empty());
    let merged = merged_text(&outcome);
    assert!(merged.contains("guid: 0000000000000000f000000000000000"));
    assert!(merged.contains("mainObjectFileID: 2100000"));
    assert!(merged.contains("assetBundleName: bundle"));
    Ok(())
}

#[test]
fn merge_file_entry_point_agrees_with_in_memory_merge() -> TestResult {
    let temp = temp_dir("agreement");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let base = temp.join("base.unity");
    let ours = temp.join("ours.unity");
    let theirs = temp.join("theirs.unity");
    std::fs::write(&base, BASE_SCENE)?;
    std::fs::write(&ours, OURS_LAYER)?;
    std::fs::write(&theirs, THEIRS_NAME)?;
    let from_files = merge_files(&base, &ours, &theirs)?;
    let from_memory = merge_bytes(
        BASE_SCENE.as_bytes(),
        OURS_LAYER.as_bytes(),
        THEIRS_NAME.as_bytes(),
    )?;
    let _ = std::fs::remove_dir_all(&temp);
    assert_eq!(from_files, from_memory);
    Ok(())
}

#[test]
fn merge_write_leaves_output_unchanged_when_conflicts_exist() -> TestResult {
    let temp = temp_dir("conflict");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let base = temp.join("base.unity");
    let ours = temp.join("ours.unity");
    let theirs = temp.join("theirs.unity");
    let output = temp.join("merged.unity");
    std::fs::write(&base, BASE_SCENE)?;
    std::fs::write(&ours, OURS_NAME_CONFLICT)?;
    std::fs::write(&theirs, THEIRS_NAME_CONFLICT)?;
    std::fs::write(&output, b"original bytes")?;
    let result = merge_files_to_path(&base, &ours, &theirs, &output);
    let error = match result {
        Ok(_) => return Err(String::from("expected the merge write to be refused").into()),
        Err(error) => error,
    };
    let conflicts = match &error {
        MergeError::UnresolvedConflicts { conflicts } => conflicts.clone(),
        other => return Err(format!("expected unresolved conflicts, found {other:?}").into()),
    };
    assert_eq!(conflicts.len(), 1);
    let bytes = std::fs::read(&output)?;
    let _ = std::fs::remove_dir_all(&temp);
    assert_eq!(bytes, b"original bytes".to_vec());
    Ok(())
}

#[test]
fn merge_write_reports_unresolved_reference_and_keeps_output() -> TestResult {
    let temp = temp_dir("reference");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let base = temp.join("base.unity");
    let ours = temp.join("ours.unity");
    let theirs = temp.join("theirs.unity");
    let output = temp.join("merged.unity");
    std::fs::write(&base, BASE_SCENE)?;
    std::fs::write(&ours, OURS_DANGLING)?;
    std::fs::write(&theirs, THEIRS_NAME)?;
    std::fs::write(&output, b"original bytes")?;
    let result = merge_files_to_path(&base, &ours, &theirs, &output);
    let error = match result {
        Ok(_) => return Err(String::from("expected the merge write to be refused").into()),
        Err(error) => error,
    };
    let references = match &error {
        MergeError::UnresolvedReferences { references } => references.clone(),
        other => return Err(format!("expected unresolved references, found {other:?}").into()),
    };
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].referenced_file_id, 999);
    assert_eq!(references[0].file_id, 100);
    let bytes = std::fs::read(&output)?;
    let _ = std::fs::remove_dir_all(&temp);
    assert_eq!(bytes, b"original bytes".to_vec());
    Ok(())
}

#[test]
fn merge_write_stores_merged_result_that_parses_back() -> TestResult {
    let temp = temp_dir("write");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let base = temp.join("base.unity");
    let ours = temp.join("ours.unity");
    let theirs = temp.join("theirs.unity");
    let output = temp.join("merged.unity");
    std::fs::write(&base, BASE_SCENE)?;
    std::fs::write(&ours, OURS_LAYER)?;
    std::fs::write(&theirs, THEIRS_NAME)?;
    let outcome = merge_files_to_path(&base, &ours, &theirs, &output)?;
    let written = std::fs::read_to_string(&output)?;
    let _ = std::fs::remove_dir_all(&temp);
    assert_eq!(written, merged_text(&outcome));
    assert_eq!(
        property(&written, 100, "GameObject.m_Layer"),
        Some(String::from("5"))
    );
    assert_eq!(
        property(&written, 200, "GameObject.m_Name"),
        Some(String::from("Beta Renamed"))
    );
    Ok(())
}

