use super::*;
use crate::unity_yaml;

use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const GUID_KEY: &str = "b2222222222222222222222222222222";

#[test]
fn field_rename_map_builds_programmatically_and_merges_former_names() {
    let mut map = FieldRenameMap::new();
    assert!(map.is_empty());
    map.add_rename(GUID_KEY, "shieldCapacity", &["m_LegacyShieldPoints"]);
    map.add_rename(GUID_KEY, "shieldCapacity", &["oldShield"]);
    map.add_rename(GUID_KEY, "unrenamedField", &[]);

    let renames = map.renames_for(GUID_KEY);
    assert_eq!(renames.len(), 1);
    assert_eq!(renames[0].current, "shieldCapacity");
    assert_eq!(renames[0].former, vec!["m_LegacyShieldPoints", "oldShield"]);
    assert!(map.renames_for("Unitas.Gameplay.Unknown").is_empty());
    assert!(!map.is_empty());

    let mut same = FieldRenameMap::new();
    same.add_rename(
        GUID_KEY,
        "shieldCapacity",
        &["m_LegacyShieldPoints", "oldShield"],
    );
    assert_eq!(map, same);
}

const MANAGED_REFERENCE_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &101
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_Script: {fileID: 11500000, guid: b2222222222222222222222222222222, type: 3}
  activeBehavior:
    rid: 2001
  references:
    version: 2
    RefIds:
    - rid: 2001
      type:
        class: RemovedStealthTactic
        ns: Unitas.Gameplay
        asm: Unitas.Gameplay
      data:
        cloakDuration: 8.5
        energyCost: 40
    - rid: 2002
      type:
        class: PatrolTactic
        ns: Unitas.Gameplay
        asm: Unitas.Gameplay
      data:
        routeId: 7
";

#[test]
fn managed_reference_api_resolves_by_rid_and_reports_missing_types() -> TestResult {
    let stream = unity_yaml::parse(MANAGED_REFERENCE_SCENE)?;
    let body = stream.documents[0]
        .root
        .get("MonoBehaviour")
        .ok_or("missing MonoBehaviour")?;

    let ids = ref_ids(body).ok_or("missing RefIds")?;
    assert_eq!(ids.len(), 2);

    let entry = resolve_managed_reference(body, 2001).ok_or("missing rid 2001")?;
    assert_eq!(entry.get("rid").and_then(Node::as_i64), Some(2001));
    assert_eq!(
        entry
            .get("data")
            .and_then(|data| data.get("cloakDuration"))
            .and_then(Node::as_f64),
        Some(8.5)
    );
    assert!(resolve_managed_reference(body, 9999).is_none());

    let missing = missing_managed_types(body, &["PatrolTactic"]);
    let missing: Vec<(&str, &str, &str)> = missing
        .iter()
        .map(|entry| (entry.asm, entry.ns, entry.class))
        .collect();
    assert_eq!(
        missing,
        vec![("Unitas.Gameplay", "Unitas.Gameplay", "RemovedStealthTactic")]
    );

    let none_missing = missing_managed_types(body, &["PatrolTactic", "RemovedStealthTactic"]);
    assert!(none_missing.is_empty());
    Ok(())
}

#[test]
fn migrate_stream_renames_former_keys_and_current_key_wins() -> TestResult {
    let scene = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &101
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_Script: {fileID: 11500000, guid: b2222222222222222222222222222222, type: 3}
  shieldCapacity: 64
  m_LegacyShieldPoints: 32
--- !u!114 &102
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_Script: {fileID: 11500000, guid: b2222222222222222222222222222222, type: 3}
  m_LegacyShieldPoints: 320
";
    let mut stream = unity_yaml::parse(scene)?;
    let mut map = FieldRenameMap::new();
    map.add_rename(
        "b2222222222222222222222222222222",
        "shieldCapacity",
        &["m_LegacyShieldPoints"],
    );
    migrate_stream(&mut stream, &map);

    let first = stream.documents[0]
        .root
        .get("MonoBehaviour")
        .ok_or("missing first MonoBehaviour")?;
    assert_eq!(first.get("shieldCapacity").and_then(Node::as_i64), Some(64));
    assert_eq!(
        first.get("m_LegacyShieldPoints").and_then(Node::as_i64),
        Some(32),
        "the current key wins and the former key is left untouched"
    );

    let second = stream.documents[1]
        .root
        .get("MonoBehaviour")
        .ok_or("missing second MonoBehaviour")?;
    assert_eq!(
        second.get("shieldCapacity").and_then(Node::as_i64),
        Some(320)
    );
    assert!(second.get("m_LegacyShieldPoints").is_none());

    let mut untouched = unity_yaml::parse(scene)?;
    migrate_stream(&mut untouched, &FieldRenameMap::new());
    let pristine = unity_yaml::parse(scene)?;
    assert_eq!(untouched, pristine);
    Ok(())
}
