use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project::{
    build_compatibility_report, GuidLookup, MigrationStatus, OpenReport, Project,
    BASELINE_UNITY_VERSION,
};
use unitas_asset::scene;
use unitas_asset::scene_save;
use unitas_asset::trust::TrustStore;
use unitas_asset::unity_yaml::{self, Node};

type TestResult = Result<(), Box<dyn Error>>;

const SCENE_GUID: &str = "5ca1ab1e000000000000000000000001";
const PICKUP_PREFAB_GUID: &str = "5ca1ab1e000000000000000000000002";
const PLAYER_MATERIAL_GUID: &str = "5ca1ab1e000000000000000000000003";
const FLOOR_MATERIAL_GUID: &str = "5ca1ab1e000000000000000000000004";
const PICKUP_MATERIAL_GUID: &str = "5ca1ab1e000000000000000000000005";
const PLAYER_CONTROLLER_GUID: &str = "5ca1ab1e000000000000000000000006";
const GAME_MANAGER_GUID: &str = "5ca1ab1e000000000000000000000007";
const PICKUP_COUNT: usize = 12;
const PICKUP_GUID_REFERENCES_PER_INSTANCE: usize = 5;
const BALL_RIGIDBODY_FILE_ID: i64 = 4002;
const SCORE_TEXT_FILE_ID: i64 = 6002;
const PICKUP_COLLIDER_FILE_ID: i64 = 13500000;

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/roll_a_ball_project")
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_roll_a_ball_project_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut entries: Vec<PathBuf> = fs::read_dir(source)?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for entry in entries {
        let target = destination.join(entry.file_name().unwrap_or_default());
        if entry.is_dir() {
            copy_tree(&entry, &target)?;
        } else {
            fs::copy(&entry, &target)?;
        }
    }
    Ok(())
}

fn copied_sample(name: &str) -> std::io::Result<PathBuf> {
    let root = temp_dir(name)?.join("RollABall");
    copy_tree(&sample_root(), &root)?;
    Ok(root)
}

fn open_sample(root: &Path, trust_dir: &Path) -> Result<(Project, OpenReport), Box<dyn Error>> {
    let store = TrustStore::new(trust_dir.join("trust.json"));
    let (project, report) = Project::open(root, &store)?;
    Ok((project, report))
}

fn document_identities(stream: &unity_yaml::Stream) -> Vec<(u32, i64)> {
    stream
        .documents
        .iter()
        .map(|document| (document.class_id, document.file_id))
        .collect()
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

fn field<'a>(stream: &'a unity_yaml::Stream, file_id: i64, keys: &[&str]) -> Option<&'a Node> {
    let document = stream
        .documents
        .iter()
        .find(|document| document.file_id == file_id)?;
    let mut node = &document.root;
    for key in keys {
        node = node.get(key)?;
    }
    Some(node)
}

fn assert_sample_inventory(report: &OpenReport) -> TestResult {
    let expected = [
        (SCENE_GUID, "Assets/Scenes/Main.unity"),
        (PICKUP_PREFAB_GUID, "Assets/Prefabs/Pickup.prefab"),
        (PLAYER_MATERIAL_GUID, "Assets/Materials/Player.mat"),
        (FLOOR_MATERIAL_GUID, "Assets/Materials/Floor.mat"),
        (PICKUP_MATERIAL_GUID, "Assets/Materials/Pickup.mat"),
        (PLAYER_CONTROLLER_GUID, "Assets/Scripts/PlayerController.cs"),
        (GAME_MANAGER_GUID, "Assets/Scripts/GameManager.cs"),
    ];
    for (guid, path) in expected {
        match report.inventory.lookup(guid) {
            GuidLookup::Asset(entry) => {
                assert_eq!(
                    entry.path,
                    PathBuf::from(path),
                    "the inventory entry for {guid} must point at {path}"
                );
            }
            other => {
                return Err(
                    format!("the inventory must contain {guid} for {path}: {other:?}").into(),
                )
            }
        }
    }
    Ok(())
}

#[test]
fn roll_a_ball_project_opens_current_with_no_unsupported_diagnostics() -> TestResult {
    let root = sample_root();
    if !root.is_dir() {
        return Err(format!("the sample project is missing at {}", root.display()).into());
    }
    let trust_dir = temp_dir("open")?;
    let (_, report) = open_sample(&root, &trust_dir)?;
    assert_eq!(
        report.declared_version.as_deref(),
        Some("2022.3.45f1"),
        "the sample must pin a Unity 2022.3 LTS version"
    );
    assert_eq!(report.effective_behavior_version, BASELINE_UNITY_VERSION);
    assert_eq!(
        report.migration,
        MigrationStatus::Current,
        "a 2022.3 sample must open without migration or unsupported-version diagnostics"
    );
    assert!(
        report.trust_gate.blocked_vectors.is_empty(),
        "the sample must trip no trust-gated vectors: {:?}",
        report.trust_gate.blocked_vectors
    );
    assert!(
        report.package_modifications.is_empty(),
        "the sample packages must open without modification findings: {:?}",
        report.package_modifications
    );
    assert!(
        report.inventory.duplicate_guids.is_empty(),
        "the sample must declare no duplicate guids: {:?}",
        report.inventory.duplicate_guids
    );
    assert_sample_inventory(&report)?;
    for setting in ["ProjectSettings.asset", "DynamicsManager.asset"] {
        assert!(
            report
                .recognized_settings
                .iter()
                .any(|recognized| recognized == setting),
            "the sample must declare a recognized {setting}: {:?}",
            report.recognized_settings
        );
    }
    assert!(
        report
            .recognized_packages
            .iter()
            .any(|package| package == "com.unity.ugui"),
        "the sample manifest must resolve its packages: {:?}",
        report.recognized_packages
    );
    let compatibility = build_compatibility_report(&root, &report);
    assert!(
        compatibility.accounts_for_all(&report),
        "the compatibility report must account for every asset, package, and setting"
    );
    let unsupported: Vec<String> = compatibility
        .asset_findings
        .iter()
        .chain(compatibility.package_findings.iter())
        .chain(compatibility.setting_findings.iter())
        .filter(|finding| finding.state != "Supported")
        .map(|finding| format!("{}: {}", finding.identity, finding.state))
        .collect();
    assert!(
        unsupported.is_empty(),
        "every feature the sample uses must be supported: {unsupported:?}"
    );
    Ok(())
}

#[test]
fn roll_a_ball_scene_round_trip_is_exact_and_keeps_guid_and_file_id_stability() -> TestResult {
    let root = copied_sample("scene_round_trip")?;
    let trust_dir = temp_dir("scene_round_trip_trust")?;
    let (_, report) = open_sample(&root, &trust_dir)?;
    assert_eq!(report.migration, MigrationStatus::Current);
    let scene_path = root.join("Assets/Scenes/Main.unity");
    let meta_path = root.join("Assets/Scenes/Main.unity.meta");
    let meta_before = fs::read_to_string(&meta_path)?;
    let original = fs::read_to_string(&scene_path)?;
    let mut stream = unity_yaml::parse_file(&scene_path)?;
    assert_eq!(
        scene_save::emit_stream(&stream),
        original,
        "an unchanged Main.unity must emit byte-identical text (Exact tier)"
    );
    let identities_before = document_identities(&stream);
    let pickup_guid_before = original.matches(PICKUP_PREFAB_GUID).count();
    assert_eq!(
        pickup_guid_before,
        PICKUP_COUNT * PICKUP_GUID_REFERENCES_PER_INSTANCE,
        "the scene must reference the Pickup prefab for all {PICKUP_COUNT} pickups"
    );
    scene_save::save_stream(&scene_path, &stream)?;
    assert_eq!(
        fs::read_to_string(&scene_path)?,
        original,
        "saving an unedited scene must leave the file bytes unchanged"
    );
    assert!(
        set_field(
            &mut stream
                .documents
                .iter_mut()
                .find(|document| document.file_id == BALL_RIGIDBODY_FILE_ID)
                .ok_or("the scene must declare the Ball rigidbody document")?
                .root,
            &["Rigidbody", "m_Mass"],
            "2.5"
        ),
        "the Ball rigidbody must expose m_Mass"
    );
    assert!(
        set_field(
            &mut stream
                .documents
                .iter_mut()
                .find(|document| document.file_id == SCORE_TEXT_FILE_ID)
                .ok_or("the scene must declare the score TextMesh document")?
                .root,
            &["TextMesh", "m_Text"],
            "Score 12"
        ),
        "the score text must expose m_Text"
    );
    scene_save::save_stream(&scene_path, &stream)?;
    let saved = fs::read_to_string(&scene_path)?;
    let reloaded = unity_yaml::parse_file(&scene_path)?;
    assert_eq!(
        reloaded
            .documents
            .iter()
            .find(|document| document.file_id == BALL_RIGIDBODY_FILE_ID)
            .and_then(|document| document.root.get("Rigidbody"))
            .and_then(|body| body.get("m_Mass"))
            .and_then(Node::as_str),
        Some("2.5"),
        "the edited Ball mass must survive save and reload"
    );
    assert_eq!(
        reloaded
            .documents
            .iter()
            .find(|document| document.file_id == SCORE_TEXT_FILE_ID)
            .and_then(|document| document.root.get("TextMesh"))
            .and_then(|body| body.get("m_Text"))
            .and_then(Node::as_str),
        Some("Score 12"),
        "the edited score text must survive save and reload"
    );
    assert_eq!(
        document_identities(&reloaded),
        identities_before,
        "every document class and fileID must be stable across the edit and save"
    );
    assert_eq!(
        saved.matches(PICKUP_PREFAB_GUID).count(),
        pickup_guid_before,
        "every Pickup prefab guid reference must be stable across the edit and save"
    );
    assert_eq!(
        fs::read_to_string(&meta_path)?,
        meta_before,
        "the scene .meta (and its guid) must be untouched by load, edit, and save"
    );
    assert!(
        meta_before.contains(SCENE_GUID),
        "the scene .meta must carry the stable scene guid"
    );
    let assembled = scene::assemble_scene_file_bundle(&scene_path)?;
    assert!(
        assembled.diagnostics.is_empty(),
        "the saved scene must import without diagnostics: {:?}",
        assembled.diagnostics
    );
    let triggers = assembled
        .physics
        .iter()
        .filter(|binding| {
            binding
                .sphere_collider
                .as_ref()
                .is_some_and(|collider| collider.is_trigger)
                || binding
                    .box_collider
                    .as_ref()
                    .is_some_and(|collider| collider.is_trigger)
        })
        .count();
    assert_eq!(
        triggers, PICKUP_COUNT,
        "the reloaded scene must expand all {PICKUP_COUNT} Pickup prefab triggers"
    );
    let ball = assembled
        .physics
        .iter()
        .find(|binding| {
            assembled
                .graph
                .entity(binding.entity)
                .is_some_and(|entity| entity.name == "Ball")
        })
        .ok_or("the reloaded scene must bind the Ball physics body")?;
    let rigidbody = ball
        .rigidbody
        .as_ref()
        .ok_or("the Ball must keep its rigidbody")?;
    assert_eq!(rigidbody.mass, 2.5, "the Ball mass edit must reach import");
    assert!(!rigidbody.is_kinematic, "the Ball must stay dynamic");
    let ball_collider = ball
        .sphere_collider
        .as_ref()
        .ok_or("the Ball must keep its sphere collider")?;
    assert_eq!(ball_collider.radius, 0.5, "the Ball radius must survive");
    assert!(
        !ball_collider.is_trigger,
        "the Ball collider must stay solid"
    );
    Ok(())
}

#[test]
fn roll_a_ball_prefab_round_trip_is_exact_and_keeps_guid_and_file_id_stability() -> TestResult {
    let root = copied_sample("prefab_round_trip")?;
    let trust_dir = temp_dir("prefab_round_trip_trust")?;
    let (_, report) = open_sample(&root, &trust_dir)?;
    assert_eq!(report.migration, MigrationStatus::Current);
    let prefab_path = root.join("Assets/Prefabs/Pickup.prefab");
    let meta_path = root.join("Assets/Prefabs/Pickup.prefab.meta");
    let meta_before = fs::read_to_string(&meta_path)?;
    let original = fs::read_to_string(&prefab_path)?;
    let mut stream = unity_yaml::parse_file(&prefab_path)?;
    assert_eq!(
        scene_save::emit_stream(&stream),
        original,
        "an unchanged Pickup.prefab must emit byte-identical text (Exact tier)"
    );
    let identities_before = document_identities(&stream);
    scene_save::save_stream(&prefab_path, &stream)?;
    assert_eq!(
        fs::read_to_string(&prefab_path)?,
        original,
        "saving an unedited prefab must leave the file bytes unchanged"
    );
    assert!(
        set_field(
            &mut stream
                .documents
                .iter_mut()
                .find(|document| document.file_id == PICKUP_COLLIDER_FILE_ID)
                .ok_or("the prefab must declare the pickup trigger collider")?
                .root,
            &["SphereCollider", "m_Radius"],
            "0.75"
        ),
        "the pickup collider must expose m_Radius"
    );
    scene_save::save_stream(&prefab_path, &stream)?;
    let reloaded = unity_yaml::parse_file(&prefab_path)?;
    assert_eq!(
        field(
            &reloaded,
            PICKUP_COLLIDER_FILE_ID,
            &["SphereCollider", "m_Radius"]
        )
        .and_then(Node::as_str),
        Some("0.75"),
        "the edited pickup radius must survive save and reload"
    );
    assert_eq!(
        field(
            &reloaded,
            PICKUP_COLLIDER_FILE_ID,
            &["SphereCollider", "m_IsTrigger"]
        )
        .and_then(Node::as_str),
        Some("1"),
        "the pickup collider must stay a trigger"
    );
    assert_eq!(
        document_identities(&reloaded),
        identities_before,
        "every prefab document class and fileID must be stable across the edit and save"
    );
    assert_eq!(
        fs::read_to_string(&meta_path)?,
        meta_before,
        "the prefab .meta (and its guid) must be untouched by load, edit, and save"
    );
    assert!(
        meta_before.contains(PICKUP_PREFAB_GUID),
        "the prefab .meta must carry the stable prefab guid"
    );
    let assembled = scene::assemble_scene_file_bundle(&prefab_path)?;
    assert!(
        assembled.diagnostics.is_empty(),
        "the saved prefab must import without diagnostics: {:?}",
        assembled.diagnostics
    );
    let collider = assembled
        .physics
        .iter()
        .find_map(|binding| binding.sphere_collider.as_ref())
        .ok_or("the reloaded prefab must bind its sphere collider")?;
    assert_eq!(collider.radius, 0.75, "the radius edit must reach import");
    assert!(
        collider.is_trigger,
        "the pickup collider must stay a trigger"
    );
    Ok(())
}
