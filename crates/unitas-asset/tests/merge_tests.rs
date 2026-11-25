use std::error::Error;
use std::path::PathBuf;

use unitas_asset::merge::{
    diff_streams, mcp_merge_asset, mcp_semantic_diff_asset, merge_bytes, merge_files,
    merge_files_to_path, resolve_conflicts_bytes, resolve_conflicts_to_path, semantic_diff,
    semantic_diff_bytes, semantic_diff_files, ConflictKind, ConflictResolution,
    ConflictResolutionChoice, DiffChangeKind, DiffOutcome, FileAccountabilityState,
    FileChangeOrigin, FileResolutionAction, MergeConflict, MergeError, MergeOutcome,
    OverrideSection, StructuralKind, WorkspaceGuardError, WorkspaceMergeGuard,
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

const BASE_SINGLE: &str =
    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Alpha\n";

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
    let document = stream
        .documents
        .iter()
        .find(|document| document.file_id == file_id)?;
    let mut node = &document.root;
    for segment in path.split('.') {
        node = node.get(segment)?;
    }
    node.as_scalar().map(|value| value.to_string())
}

fn file_ids(text: &str) -> Vec<i64> {
    match unity_yaml::parse(text) {
        Ok(stream) => {
            let mut ids: Vec<i64> = stream
                .documents
                .iter()
                .map(|document| document.file_id)
                .collect();
            ids.sort_unstable();
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
    members.sort_unstable();
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

const DIFF_HIERARCHY_OLD: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Root\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 110}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: Child\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n--- !u!1 &300\nGameObject:\n  m_Name: Legacy\n  m_Layer: 2\n  m_Component:\n  - component: {fileID: 310}\n--- !u!4 &310\nTransform:\n  m_GameObject: {fileID: 300}\n  m_Father: {fileID: 110}\n";

const DIFF_HIERARCHY_NEW: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Root\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 110}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: Child\n  m_Layer: 4\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n--- !u!1 &400\nGameObject:\n  m_Name: Spawned\n  m_Layer: 1\n  m_Component:\n  - component: {fileID: 410}\n--- !u!4 &410\nTransform:\n  m_GameObject: {fileID: 400}\n  m_Father: {fileID: 210}\n";

const DIFF_COMPONENT_OLD: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Hero\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n  - component: {fileID: 130}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 0, y: 1, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_Target: {fileID: 500, guid: bbbbbbbb22222222bbbbbbbb22222222, type: 3}\n  speed: 5\n--- !u!65 &130\nBoxCollider:\n  m_GameObject: {fileID: 100}\n  m_IsTrigger: 0\n";

const DIFF_COMPONENT_NEW: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Hero\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n  - component: {fileID: 140}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 2, y: 1, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_Target: {fileID: 600, guid: cccccccc33333333cccccccc33333333, type: 3}\n  speed: 10\n--- !u!54 &140\nRigidbody:\n  m_GameObject: {fileID: 100}\n  m_Mass: 2.5\n";

const DIFF_PREFAB_OLD: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1001 &1000\nPrefabInstance:\n  m_SourcePrefab: {fileID: 100100000, guid: 99999999888888887777777766666666, type: 3}\n  m_Modification:\n    m_TransformParent: {fileID: 0}\n    m_Modifications:\n    - target: {fileID: 5000, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_Name\n      value: Turret\n      objectReference: {fileID: 0}\n    - target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_LocalPosition.x\n      value: 1\n      objectReference: {fileID: 0}\n    - target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_LocalPosition.y\n      value: 2\n      objectReference: {fileID: 0}\n    m_RemovedComponents:\n    - {fileID: 5020, guid: 99999999888888887777777766666666, type: 3}\n    m_AddedComponents: []\n    m_AddedGameObjects: []\n";

const DIFF_PREFAB_NEW: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1001 &1000\nPrefabInstance:\n  m_SourcePrefab: {fileID: 100100000, guid: 99999999888888887777777766666666, type: 3}\n  m_Modification:\n    m_TransformParent: {fileID: 0}\n    m_Modifications:\n    - target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_LocalPosition.x\n      value: 4\n      objectReference: {fileID: 0}\n    - target: {fileID: 5000, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_Name\n      value: Turret\n      objectReference: {fileID: 0}\n    - target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_LocalPosition.z\n      value: 9\n      objectReference: {fileID: 0}\n    m_RemovedComponents:\n    - {fileID: 5020, guid: 99999999888888887777777766666666, type: 3}\n    - {fileID: 5030, guid: 99999999888888887777777766666666, type: 3}\n    m_AddedComponents:\n    - targetCorrespondingSourceObject: {fileID: 5000, guid: 99999999888888887777777766666666, type: 3}\n      insertIndex: -1\n      addedObject: {fileID: 7001}\n    m_AddedGameObjects:\n    - targetCorrespondingSourceObject: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      insertIndex: -1\n      addedObject: {fileID: 8001}\n";

#[test]
fn semantic_diff_reports_added_removed_and_changed_game_objects_with_hierarchy_paths_and_file_ids(
) -> TestResult {
    let outcome = semantic_diff(DIFF_HIERARCHY_OLD, DIFF_HIERARCHY_NEW)?;
    let diff = match outcome.as_diff() {
        Some(diff) => diff,
        None => return Err(String::from("expected diffable outcome").into()),
    };
    let added: Vec<_> = diff
        .objects
        .iter()
        .filter(|obj| obj.change == DiffChangeKind::Added)
        .collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].file_id, 400);
    assert_eq!(added[0].class_id, Some(1));
    assert_eq!(added[0].object_path, "Root/Child/Spawned");

    let removed: Vec<_> = diff
        .objects
        .iter()
        .filter(|obj| obj.change == DiffChangeKind::Removed)
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].file_id, 300);
    assert_eq!(removed[0].class_id, Some(1));
    assert_eq!(removed[0].object_path, "Root/Legacy");

    let changed: Vec<_> = diff
        .objects
        .iter()
        .filter(|obj| obj.change == DiffChangeKind::Changed)
        .collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].file_id, 200);
    assert_eq!(changed[0].class_id, Some(1));
    assert_eq!(changed[0].object_path, "Root/Child");
    assert_eq!(changed[0].properties.len(), 1);
    assert_eq!(changed[0].properties[0].property_path, "m_Layer");
    assert_eq!(changed[0].properties[0].old_value.as_deref(), Some("0"));
    assert_eq!(changed[0].properties[0].new_value.as_deref(), Some("4"));
    Ok(())
}

#[test]
fn semantic_diff_reports_component_and_property_changes_preserving_guids_and_pptrs() -> TestResult {
    let old_stream = unity_yaml::parse(DIFF_COMPONENT_OLD)?;
    let new_stream = unity_yaml::parse(DIFF_COMPONENT_NEW)?;
    let diff = diff_streams(&old_stream, &new_stream)?;

    let added_comp: Vec<_> = diff
        .components
        .iter()
        .filter(|comp| comp.change == DiffChangeKind::Added)
        .collect();
    assert_eq!(added_comp.len(), 1);
    assert_eq!(added_comp[0].file_id, 140);
    assert_eq!(added_comp[0].class_id, Some(54));
    assert_eq!(added_comp[0].type_name.as_deref(), Some("Rigidbody"));
    assert_eq!(added_comp[0].owner_file_id, Some(100));
    assert_eq!(added_comp[0].object_path, "Hero/Rigidbody");

    let removed_comp: Vec<_> = diff
        .components
        .iter()
        .filter(|comp| comp.change == DiffChangeKind::Removed)
        .collect();
    assert_eq!(removed_comp.len(), 1);
    assert_eq!(removed_comp[0].file_id, 130);
    assert_eq!(removed_comp[0].class_id, Some(65));
    assert_eq!(removed_comp[0].type_name.as_deref(), Some("BoxCollider"));
    assert_eq!(removed_comp[0].owner_file_id, Some(100));
    assert_eq!(removed_comp[0].object_path, "Hero/BoxCollider");

    let mono_comp = diff
        .components
        .iter()
        .find(|comp| comp.file_id == 120)
        .ok_or("expected MonoBehaviour component diff")?;
    assert_eq!(mono_comp.change, DiffChangeKind::Changed);
    assert_eq!(mono_comp.object_path, "Hero/MonoBehaviour");

    let target_prop = mono_comp
        .properties
        .iter()
        .find(|prop| prop.property_path == "m_Target")
        .ok_or("expected m_Target PPtr property diff")?;
    assert_eq!(
        target_prop.old_value.as_deref(),
        Some("{fileID: 500, guid: bbbbbbbb22222222bbbbbbbb22222222, type: 3}")
    );
    assert_eq!(
        target_prop.new_value.as_deref(),
        Some("{fileID: 600, guid: cccccccc33333333cccccccc33333333, type: 3}")
    );

    let speed_prop = mono_comp
        .properties
        .iter()
        .find(|prop| prop.property_path == "speed")
        .ok_or("expected speed property diff")?;
    assert_eq!(speed_prop.old_value.as_deref(), Some("5"));
    assert_eq!(speed_prop.new_value.as_deref(), Some("10"));

    let transform_comp = diff
        .components
        .iter()
        .find(|comp| comp.file_id == 110)
        .ok_or("expected Transform component diff")?;
    assert_eq!(transform_comp.change, DiffChangeKind::Changed);
    assert_eq!(transform_comp.properties.len(), 1);
    assert_eq!(
        transform_comp.properties[0].property_path,
        "m_LocalPosition.x"
    );
    assert_eq!(transform_comp.properties[0].old_value.as_deref(), Some("0"));
    assert_eq!(transform_comp.properties[0].new_value.as_deref(), Some("2"));
    Ok(())
}

#[test]
fn semantic_diff_reports_prefab_override_changes() -> TestResult {
    let outcome = semantic_diff_bytes(DIFF_PREFAB_OLD.as_bytes(), DIFF_PREFAB_NEW.as_bytes())?;
    let diff = match outcome.as_diff() {
        Some(diff) => diff,
        None => return Err(String::from("expected diffable outcome").into()),
    };

    let mod_changed = diff
        .prefab_overrides
        .iter()
        .find(|ov| {
            ov.section == OverrideSection::Modifications && ov.change == DiffChangeKind::Changed
        })
        .ok_or("expected modified m_Modifications override")?;
    assert_eq!(mod_changed.prefab_instance_file_id, 1000);
    assert_eq!(mod_changed.target_file_id, Some(5010));
    assert_eq!(
        mod_changed.target_guid.as_deref(),
        Some("99999999888888887777777766666666")
    );
    assert_eq!(
        mod_changed.property_path.as_deref(),
        Some("m_LocalPosition.x")
    );
    assert_eq!(mod_changed.old_value.as_deref(), Some("1"));
    assert_eq!(mod_changed.new_value.as_deref(), Some("4"));

    let mod_added = diff
        .prefab_overrides
        .iter()
        .find(|ov| {
            ov.section == OverrideSection::Modifications && ov.change == DiffChangeKind::Added
        })
        .ok_or("expected added m_Modifications override")?;
    assert_eq!(mod_added.target_file_id, Some(5010));
    assert_eq!(
        mod_added.property_path.as_deref(),
        Some("m_LocalPosition.z")
    );
    assert_eq!(mod_added.old_value, None);
    assert_eq!(mod_added.new_value.as_deref(), Some("9"));

    let mod_removed = diff
        .prefab_overrides
        .iter()
        .find(|ov| {
            ov.section == OverrideSection::Modifications && ov.change == DiffChangeKind::Removed
        })
        .ok_or("expected removed m_Modifications override")?;
    assert_eq!(mod_removed.target_file_id, Some(5010));
    assert_eq!(
        mod_removed.property_path.as_deref(),
        Some("m_LocalPosition.y")
    );
    assert_eq!(mod_removed.old_value.as_deref(), Some("2"));
    assert_eq!(mod_removed.new_value, None);

    let removed_comp = diff
        .prefab_overrides
        .iter()
        .find(|ov| ov.section == OverrideSection::RemovedComponents)
        .ok_or("expected m_RemovedComponents override change")?;
    assert_eq!(removed_comp.change, DiffChangeKind::Added);
    assert_eq!(removed_comp.target_file_id, Some(5030));
    assert_eq!(
        removed_comp.target_guid.as_deref(),
        Some("99999999888888887777777766666666")
    );

    let added_comp = diff
        .prefab_overrides
        .iter()
        .find(|ov| ov.section == OverrideSection::AddedComponents)
        .ok_or("expected m_AddedComponents override change")?;
    assert_eq!(added_comp.change, DiffChangeKind::Added);
    assert_eq!(added_comp.target_file_id, Some(5000));
    assert_eq!(added_comp.added_object_file_id, Some(7001));

    let added_go = diff
        .prefab_overrides
        .iter()
        .find(|ov| ov.section == OverrideSection::AddedGameObjects)
        .ok_or("expected m_AddedGameObjects override change")?;
    assert_eq!(added_go.change, DiffChangeKind::Added);
    assert_eq!(added_go.target_file_id, Some(5010));
    assert_eq!(added_go.added_object_file_id, Some(8001));
    Ok(())
}

#[test]
fn semantic_diff_is_empty_when_documents_and_properties_are_reordered() -> TestResult {
    let original = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Root\n  m_Layer: 0\n  m_TagString: Untagged\n  m_Component:\n  - component: {fileID: 110}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 1, y: 2, z: 3}\n--- !u!1001 &1000\nPrefabInstance:\n  m_SourcePrefab: {fileID: 100100000, guid: 99999999888888887777777766666666, type: 3}\n  m_Modification:\n    m_TransformParent: {fileID: 0}\n    m_Modifications:\n    - target: {fileID: 5000, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_Name\n      value: Turret\n    - target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_LocalPosition.x\n      value: 1\n";
    let reordered = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1001 &1000\nPrefabInstance:\n  m_Modification:\n    m_Modifications:\n    - propertyPath: m_LocalPosition.x\n      target: {fileID: 5010, guid: 99999999888888887777777766666666, type: 3}\n      value: 1\n    - value: Turret\n      target: {fileID: 5000, guid: 99999999888888887777777766666666, type: 3}\n      propertyPath: m_Name\n    m_TransformParent: {fileID: 0}\n  m_SourcePrefab: {fileID: 100100000, guid: 99999999888888887777777766666666, type: 3}\n--- !u!4 &110\nTransform:\n  m_LocalPosition: {z: 3, x: 1, y: 2}\n  m_Father: {fileID: 0}\n  m_GameObject: {fileID: 100}\n--- !u!1 &100\nGameObject:\n  m_TagString: Untagged\n  m_Layer: 0\n  m_Name: Root\n  m_Component:\n  - component: {fileID: 110}\n";
    let outcome = semantic_diff(original, reordered)?;
    let diff = match outcome.as_diff() {
        Some(diff) => diff,
        None => return Err(String::from("expected diffable outcome").into()),
    };
    assert!(diff.is_empty());
    assert!(diff.objects.is_empty());
    assert!(diff.components.is_empty());
    assert!(diff.properties.is_empty());
    assert!(diff.prefab_overrides.is_empty());
    Ok(())
}

#[test]
fn semantic_diff_reports_binary_asset_as_not_semantically_diffable() -> TestResult {
    let binary: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0xFE, 0xFF, 0x10, 0x20];
    let outcome = semantic_diff_bytes(&binary, DIFF_HIERARCHY_NEW.as_bytes())?;
    assert!(!outcome.is_diffable());
    match outcome {
        DiffOutcome::NotSemanticallyDiffable { reason } => {
            assert!(reason.contains("binary"));
        }
        DiffOutcome::Diff(diff) => {
            return Err(format!("expected NotSemanticallyDiffable, got {diff:?}").into())
        }
    }

    let temp = temp_dir("diff_files");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let left_path = temp.join("left.asset");
    let right_path = temp.join("right.asset");
    std::fs::write(&left_path, &binary)?;
    std::fs::write(&right_path, DIFF_HIERARCHY_NEW.as_bytes())?;
    let file_outcome = semantic_diff_files(&left_path, &right_path)?;
    let _ = std::fs::remove_dir_all(&temp);
    assert!(matches!(
        file_outcome,
        DiffOutcome::NotSemanticallyDiffable { .. }
    ));
    Ok(())
}

#[test]
fn edt_007_outcome_7_workspace_guard_accounts_for_unsaved_human_agent_and_conflicting_files_before_close_or_reload(
) -> TestResult {
    let mut guard = WorkspaceMergeGuard::new();
    let human_scene = PathBuf::from("Assets/Scenes/Main.unity");
    let agent_prefab = PathBuf::from("Assets/Prefabs/Turret.prefab");
    let conflicted_scene = PathBuf::from("Assets/Scenes/BossArena.unity");

    guard.record_saved(&human_scene, BASE_SCENE.as_bytes());
    guard.record_saved(&agent_prefab, DIFF_PREFAB_OLD.as_bytes());
    guard.record_saved(&conflicted_scene, BASE_SCENE.as_bytes());
    assert!(
        guard.can_close_workspace().is_ok(),
        "EDT-007 outcome 7: clean workspace must allow close"
    );

    guard.record_unsaved_edit(
        &human_scene,
        FileChangeOrigin::Person,
        OURS_LAYER.as_bytes(),
    );
    guard.record_unsaved_edit(
        &agent_prefab,
        FileChangeOrigin::AgentSession {
            session_id: String::from("agent-session-alpha"),
        },
        DIFF_PREFAB_NEW.as_bytes(),
    );
    let conflict_outcome = merge_bytes(
        BASE_SCENE.as_bytes(),
        OURS_NAME_CONFLICT.as_bytes(),
        THEIRS_NAME_CONFLICT.as_bytes(),
    )?;
    guard.record_external_change_or_conflict(
        &conflicted_scene,
        THEIRS_NAME_CONFLICT.as_bytes(),
        conflict_outcome.conflicts.clone(),
    );

    let unaccounted = guard.unaccounted_files();
    assert_eq!(
        unaccounted.len(),
        3,
        "EDT-007 outcome 7: every unsaved human, agent, and conflicting file must be listed as unaccounted"
    );
    assert!(
        unaccounted.iter().any(|entry| {
            entry.path == human_scene
                && entry.state == FileAccountabilityState::Unsaved
                && entry.origins.contains(&FileChangeOrigin::Person)
        }),
        "EDT-007 outcome 7: human unsaved scene must be tracked"
    );
    assert!(
        unaccounted.iter().any(|entry| {
            entry.path == agent_prefab
                && entry.state == FileAccountabilityState::Unsaved
                && entry.origins.contains(&FileChangeOrigin::AgentSession {
                    session_id: String::from("agent-session-alpha"),
                })
        }),
        "EDT-007 outcome 7: agent session unsaved prefab edit must be tracked with session attribution"
    );
    assert!(
        unaccounted.iter().any(|entry| {
            entry.path == conflicted_scene
                && entry.state == FileAccountabilityState::Conflicted
                && entry.unresolved_conflicts.len() == 1
        }),
        "EDT-007 outcome 7: conflicted file must report unresolved conflicts"
    );

    let close_err = match guard.can_close_workspace() {
        Ok(()) => {
            return Err(String::from(
                "EDT-007 outcome 7: expected close to be blocked while files are unaccounted",
            )
            .into())
        }
        Err(err) => err,
    };
    match close_err {
        WorkspaceGuardError::UnaccountedFiles { files } => {
            assert_eq!(
                files.len(),
                3,
                "EDT-007 outcome 7: close error must name all three unaccounted files"
            );
        }
        other => {
            return Err(
                format!("EDT-007 outcome 7: expected UnaccountedFiles, got {other:?}").into(),
            )
        }
    }

    assert!(
        guard.can_reload_file(&agent_prefab).is_err(),
        "EDT-007 outcome 7: reload of unsaved agent file must be blocked until accounted for"
    );
    assert!(
        guard.can_reload_file(&conflicted_scene).is_err(),
        "EDT-007 outcome 7: reload of conflicted file must be blocked until accounted for"
    );

    guard.account_for_file(&human_scene, FileResolutionAction::Save)?;
    assert!(
        guard.can_close_workspace().is_err(),
        "EDT-007 outcome 7: workspace close must remain blocked while agent and conflicted files are unaccounted"
    );

    guard.account_for_file(&agent_prefab, FileResolutionAction::KeepUnsavedWork)?;
    assert!(
        guard.can_reload_file(&agent_prefab).is_err(),
        "EDT-007 outcome 7: KeepUnsavedWork accounts for close/keep decision but still blocks silent reload over unsaved work"
    );
    guard.account_for_file(&agent_prefab, FileResolutionAction::DiscardAndReload)?;
    assert!(
        guard.can_reload_file(&agent_prefab).is_ok(),
        "EDT-007 outcome 7: after DiscardAndReload, file reload is permitted"
    );

    let resolved_scene = resolve_conflicts_bytes(
        BASE_SCENE.as_bytes(),
        OURS_NAME_CONFLICT.as_bytes(),
        THEIRS_NAME_CONFLICT.as_bytes(),
        &[ConflictResolution {
            file_id: 100,
            property: Some(String::from("m_Name")),
            choice: ConflictResolutionChoice::Theirs,
        }],
    )?;
    let resolved_bytes = resolved_scene
        .merged
        .as_deref()
        .ok_or("EDT-007 outcome 7: expected resolved merge text")?
        .as_bytes()
        .to_vec();
    guard.account_for_file(
        &conflicted_scene,
        FileResolutionAction::ResolveConflict { resolved_bytes },
    )?;
    assert!(
        guard.can_reload_file(&conflicted_scene).is_ok(),
        "EDT-007 outcome 7: resolved conflict permits reload"
    );
    assert!(
        guard.unaccounted_files().is_empty(),
        "EDT-007 outcome 7: all files accounted for"
    );
    assert!(
        guard.can_close_workspace().is_ok(),
        "EDT-007 outcome 7: workspace close succeeds once every unsaved and conflicting file is accounted for"
    );
    Ok(())
}

#[test]
fn edt_007_outcome_13_three_way_merge_scene_and_prefab_resolves_references_and_matches_mcp_entry_point(
) -> TestResult {
    let base_scene = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Player\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_FollowTarget: {fileID: 210}\n  speed: 5\n--- !u!1 &200\nGameObject:\n  m_Name: Companion\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n  m_LocalPosition: {x: 1, y: 0, z: 0}\n";
    let ours_scene = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Player\n  m_Layer: 3\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 10, y: 0, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_FollowTarget: {fileID: 210}\n  speed: 8\n--- !u!1 &200\nGameObject:\n  m_Name: Companion\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n  m_LocalPosition: {x: 1, y: 0, z: 0}\n";
    let theirs_scene = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Player\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 0, y: 5, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_FollowTarget: {fileID: 210}\n  speed: 5\n--- !u!1 &200\nGameObject:\n  m_Name: CompanionBot\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n  m_LocalPosition: {x: 1, y: 2, z: 0}\n";

    let direct_outcome = merge_bytes(
        base_scene.as_bytes(),
        ours_scene.as_bytes(),
        theirs_scene.as_bytes(),
    )?;
    assert!(
        direct_outcome.conflicts.is_empty(),
        "EDT-007 outcome 13: disjoint property changes across two branches must merge cleanly"
    );
    assert!(
        direct_outcome.references.is_empty(),
        "EDT-007 outcome 13: all internal references in merged scene must resolve"
    );
    let merged_scene_text = merged_text(&direct_outcome);
    assert_eq!(
        file_ids(merged_scene_text),
        vec![100, 110, 120, 200, 210],
        "EDT-007 outcome 13: merged scene must preserve all file IDs"
    );
    assert_eq!(
        property(merged_scene_text, 100, "GameObject.m_Layer"),
        Some(String::from("3"))
    );
    assert_eq!(
        property(merged_scene_text, 110, "Transform.m_LocalPosition.x"),
        Some(String::from("10"))
    );
    assert_eq!(
        property(merged_scene_text, 110, "Transform.m_LocalPosition.y"),
        Some(String::from("5"))
    );
    assert_eq!(
        property(merged_scene_text, 120, "MonoBehaviour.speed"),
        Some(String::from("8"))
    );
    assert_eq!(
        property(merged_scene_text, 200, "GameObject.m_Name"),
        Some(String::from("CompanionBot"))
    );

    let merged_stream = unity_yaml::parse(merged_scene_text)?;
    let assembled = unitas_asset::assemble_scene(&merged_stream)?;
    assert_eq!(
        assembled.len(),
        2,
        "EDT-007 outcome 13: merged scene must assemble with its GameObjects intact"
    );

    let mcp_outcome = mcp_merge_asset(base_scene, ours_scene, theirs_scene)?;
    assert_eq!(
        mcp_outcome, direct_outcome,
        "EDT-007 outcome 13: agent calling MCP merge entry point must obtain the exact same MergeOutcome"
    );

    let base_prefab = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: CratePrefab\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 1010}\n--- !u!4 &1010\nTransform:\n  m_GameObject: {fileID: 1000}\n  m_Father: {fileID: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n";
    let ours_prefab = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: CratePrefab\n  m_Layer: 6\n  m_Component:\n  - component: {fileID: 1010}\n--- !u!4 &1010\nTransform:\n  m_GameObject: {fileID: 1000}\n  m_Father: {fileID: 0}\n  m_LocalScale: {x: 2, y: 1, z: 1}\n";
    let theirs_prefab = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1000\nGameObject:\n  m_Name: HeavyCratePrefab\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 1010}\n--- !u!4 &1010\nTransform:\n  m_GameObject: {fileID: 1000}\n  m_Father: {fileID: 0}\n  m_LocalScale: {x: 1, y: 1, z: 3}\n";

    let prefab_direct = merge_bytes(
        base_prefab.as_bytes(),
        ours_prefab.as_bytes(),
        theirs_prefab.as_bytes(),
    )?;
    let prefab_mcp = mcp_merge_asset(base_prefab, ours_prefab, theirs_prefab)?;
    assert_eq!(
        prefab_mcp, prefab_direct,
        "EDT-007 outcome 13: prefab merge via MCP must match direct three-way merge"
    );
    assert!(prefab_direct.references.is_empty());
    let prefab_stream = unity_yaml::parse(merged_text(&prefab_direct))?;
    let assembled_prefab = unitas_asset::assemble_scene(&prefab_stream)?;
    assert_eq!(assembled_prefab.len(), 1);

    let theirs_conflict_scene = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Player\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 110}\n  - component: {fileID: 120}\n--- !u!4 &110\nTransform:\n  m_GameObject: {fileID: 100}\n  m_Father: {fileID: 0}\n  m_LocalPosition: {x: 0, y: 5, z: 0}\n--- !u!114 &120\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Script: {fileID: 11500000, guid: aaaaaaaa11111111aaaaaaaa11111111, type: 3}\n  m_FollowTarget: {fileID: 210}\n  speed: 12\n--- !u!1 &200\nGameObject:\n  m_Name: CompanionBot\n  m_Layer: 0\n  m_Component:\n  - component: {fileID: 210}\n--- !u!4 &210\nTransform:\n  m_GameObject: {fileID: 200}\n  m_Father: {fileID: 110}\n  m_LocalPosition: {x: 1, y: 2, z: 0}\n";
    let conflict_direct = merge_bytes(
        base_scene.as_bytes(),
        ours_scene.as_bytes(),
        theirs_conflict_scene.as_bytes(),
    )?;
    let conflict_mcp = mcp_merge_asset(base_scene, ours_scene, theirs_conflict_scene)?;
    assert_eq!(
        conflict_mcp, conflict_direct,
        "EDT-007 outcome 13: same-property conflict reported through MCP must equal direct merge conflict"
    );
    let speed_conflict = only_conflict(&conflict_mcp)?;
    assert_eq!(speed_conflict.kind, ConflictKind::Property);
    assert_eq!(speed_conflict.object_path, "Player/MonoBehaviour");
    assert_eq!(speed_conflict.property.as_deref(), Some("speed"));
    assert_eq!(speed_conflict.base.as_deref(), Some("5"));
    assert_eq!(speed_conflict.ours.as_deref(), Some("8"));
    assert_eq!(speed_conflict.theirs.as_deref(), Some("12"));

    let resolved_custom = resolve_conflicts_bytes(
        base_scene.as_bytes(),
        ours_scene.as_bytes(),
        theirs_conflict_scene.as_bytes(),
        &[ConflictResolution {
            file_id: 120,
            property: Some(String::from("speed")),
            choice: ConflictResolutionChoice::Custom(String::from("15")),
        }],
    )?;
    assert!(resolved_custom.conflicts.is_empty());
    assert!(resolved_custom.references.is_empty());
    assert_eq!(
        property(merged_text(&resolved_custom), 120, "MonoBehaviour.speed"),
        Some(String::from("15"))
    );

    let temp = temp_dir("resolve_to_path");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;
    let base_path = temp.join("base.unity");
    let ours_path = temp.join("ours.unity");
    let theirs_path = temp.join("theirs.unity");
    let out_path = temp.join("out.unity");
    std::fs::write(&base_path, base_scene)?;
    std::fs::write(&ours_path, ours_scene)?;
    std::fs::write(&theirs_path, theirs_conflict_scene)?;
    let written_outcome = resolve_conflicts_to_path(
        &base_path,
        &ours_path,
        &theirs_path,
        &[ConflictResolution {
            file_id: 120,
            property: Some(String::from("speed")),
            choice: ConflictResolutionChoice::Ours,
        }],
        &out_path,
    )?;
    let _ = std::fs::remove_dir_all(&temp);
    assert!(written_outcome.conflicts.is_empty());
    assert_eq!(
        property(merged_text(&written_outcome), 120, "MonoBehaviour.speed"),
        Some(String::from("8"))
    );

    let diff_direct = semantic_diff(base_scene, ours_scene)?;
    let diff_mcp = mcp_semantic_diff_asset(base_scene, ours_scene)?;
    assert_eq!(
        diff_mcp, diff_direct,
        "EDT-007 outcome 13: MCP semantic diff entry point must match direct semantic diff"
    );
    Ok(())
}
