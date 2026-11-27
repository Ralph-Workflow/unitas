use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_asset::project::{MigrationStatus, Project};
use unitas_asset::trust::TrustStore;
use unitas_asset::{parse_dynamics_manager, parse_time_manager};
use unitas_core::{EntityId, SceneGraph};
use unitas_runtime::Runtime;
use unitas_runtime::scripting::ProjectScriptIndex;

type TestResult = Result<(), Box<dyn Error>>;

const PLAYER_CONTROLLER_GUID: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f809";
const ROTATOR_GUID: &str = "2b3c4d5e6f708192a3b4c5d6e7f8091a";
const SCORE_MANAGER_GUID: &str = "3c4d5e6f708192a3b4c5d6e7f8091a2b";
const PICKUP_GUID: &str = "b2c3d4e5f60718293a4b5c6d7e8f90a1";
const GOLD_PICKUP_MARKER_GUID: &str = "d5e6f70818293a4b5c6d7e8f90a1b2c3";
const HUD_CONTROLLER_GUID: &str = "e3f4a5b6c7d8e9f0a1b2c3d4e5f60718";
// Unity's own legacy uGUI script GUIDs. They are not project scripts, so the
// project script index never resolves them.
const UGUI_SCRIPT_GUIDS: [&str; 4] = [
    unitas_asset::scene::CANVAS_SCALER_SCRIPT_GUID,
    unitas_asset::scene::BUTTON_SCRIPT_GUID,
    unitas_asset::scene::TEXT_SCRIPT_GUID,
    unitas_asset::scene::IMAGE_SCRIPT_GUID,
];
const SCENE_GUID: &str = "9f1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e";
const SCRIPT_DIRECTORY: &str = "Assets/Scripts";

fn sample_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("roll-a-ball")
}

fn scene_path() -> PathBuf {
    sample_root()
        .join("Assets")
        .join("Scenes")
        .join("RollABall.unity")
}

fn entity_named(graph: &SceneGraph, name: &str) -> Result<EntityId, Box<dyn Error>> {
    graph
        .iter_breadth_first()
        .find(|id| graph.entity(*id).is_some_and(|entity| entity.name == name))
        .ok_or_else(|| format!("scene has no entity named {name}").into())
}

fn child_names(graph: &SceneGraph, parent: EntityId) -> Vec<&str> {
    graph
        .children(parent)
        .iter()
        .filter_map(|id| graph.entity(*id).map(|entity| entity.name.as_str()))
        .collect()
}

fn loaded_scene() -> Result<Runtime, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path())?;
    Ok(runtime)
}

#[test]
fn scene_loads_with_project_settings_applied() -> TestResult {
    let mut runtime = Runtime::new()?;
    let script_count = runtime.load_scene(&scene_path())?;
    // Every MonoBehaviour in the scene counts, including the legacy uGUI
    // components (Text, Image, Button, CanvasScaler) that carry no project
    // script of their own.
    assert_eq!(script_count, 29);
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("RollABall"));
    assert_eq!(runtime.fixed_timestep(), 0.02);
    assert_eq!(runtime.company_name(), Some("Unitas"));
    assert_eq!(runtime.product_name(), Some("RollABall"));
    assert_eq!(runtime.project_version(), Some("1.0.0"));

    let mut guids: Vec<&str> = runtime
        .scripts()
        .iter()
        .map(|script| script.guid.as_str())
        .collect();
    // The legacy uGUI components carry Unity's own script GUIDs; this test
    // pins the project's own scripts.
    guids.retain(|guid| !UGUI_SCRIPT_GUIDS.contains(guid));
    guids.sort_unstable();
    assert_eq!(
        guids,
        vec![
            PLAYER_CONTROLLER_GUID,
            ROTATOR_GUID,
            ROTATOR_GUID,
            ROTATOR_GUID,
            ROTATOR_GUID,
            SCORE_MANAGER_GUID,
            PICKUP_GUID,
            PICKUP_GUID,
            PICKUP_GUID,
            GOLD_PICKUP_MARKER_GUID,
            HUD_CONTROLLER_GUID,
        ]
    );
    Ok(())
}

#[test]
fn scene_project_settings_carry_the_loader_values() -> TestResult {
    let settings = sample_root().join("ProjectSettings");
    let time = parse_time_manager(&settings.join("TimeManager.asset"))?;
    assert_eq!(time.map(|value| value.fixed_timestep), Some(0.02));
    let dynamics = parse_dynamics_manager(&settings.join("DynamicsManager.asset"))?;
    assert_eq!(dynamics.map(|value| value.gravity), Some([0.0, -9.81, 0.0]));
    Ok(())
}

#[test]
fn scene_hierarchy_matches_the_expected_tree() -> TestResult {
    let runtime = loaded_scene()?;
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("loaded scene has no graph").into()),
    };

    let root_names: Vec<&str> = graph
        .roots()
        .iter()
        .filter_map(|id| graph.entity(*id).map(|entity| entity.name.as_str()))
        .collect();
    assert_eq!(
        root_names,
        vec![
            "Main Camera",
            "Directional Light",
            "Arena",
            "Pickups",
            "PlayerBall",
            "CollectBurst",
            "LandingDust",
            "ArenaProp",
            "CollectChime",
            // The uGUI hierarchy hangs off RectTransforms, not Transforms, so
            // the scene graph carries the canvas objects as roots of their
            // own while the canvas binding keeps their real hierarchy.
            "HudCanvas",
            "ScoreLabel",
            "ComboLabel",
            "TimerLabel",
            "TitlePanel",
            "TitleText",
            "PlayButton",
            "QuitButton",
            "ResultsPanel",
            "ResultsTitle",
            "ResultsTime",
            "ResultsScore",
            "RetryButton",
            "ArenaSelectButton"
        ]
    );

    let arena = entity_named(graph, "Arena")?;
    assert_eq!(
        child_names(graph, arena),
        vec![
            "ArenaFloor",
            "Wall_North",
            "Wall_South",
            "Wall_East",
            "Wall_West"
        ]
    );

    let pickups = entity_named(graph, "Pickups")?;
    assert_eq!(
        child_names(graph, pickups),
        vec![
            "Pickup_1",
            "Pickup_2",
            "Pickup_3",
            "Pickup_4",
            "PickupPrefab_1",
            "PickupPrefab_2",
            "PickupGold"
        ]
    );

    let player = entity_named(graph, "PlayerBall")?;
    assert_eq!(child_names(graph, player), vec!["PickupSensor"]);
    assert_eq!(graph.parent(player), None);

    let sensor = entity_named(graph, "PickupSensor")?;
    assert_eq!(graph.parent(sensor), Some(player));
    Ok(())
}

#[test]
fn sample_opens_as_a_unity_2022_project() -> TestResult {
    let trust = TrustStore::new(sample_root().join(".unity-trust.json"));
    let (_project, report) = Project::open(&sample_root(), &trust)?;
    assert_eq!(report.declared_version.as_deref(), Some("2022.3.62f1"));
    assert_eq!(report.migration, MigrationStatus::Current);

    let guids: Vec<&str> = report
        .inventory
        .entries
        .iter()
        .map(|entry| entry.guid.as_str())
        .collect();
    assert!(guids.contains(&PLAYER_CONTROLLER_GUID));
    assert!(guids.contains(&ROTATOR_GUID));
    assert!(guids.contains(&SCORE_MANAGER_GUID));
    assert!(guids.contains(&SCENE_GUID));
    Ok(())
}

#[test]
fn scene_scripts_resolve_to_sample_sources_through_the_script_index() -> TestResult {
    let runtime = loaded_scene()?;
    let index = ProjectScriptIndex::build(&sample_root())?;
    // Eleven sample scripts: the nine the sample shipped with, plus the ghost
    // replay component the S14 cross-attempt replay needs and the reduced
    // motion preference helper the trails feature reads.
    assert_eq!(index.script_count(), 11);
    let root = sample_root();

    for script in runtime.scripts() {
        if UGUI_SCRIPT_GUIDS.contains(&script.guid.as_str()) {
            continue;
        }
        let record = match index.script_by_guid(&script.guid) {
            Some(record) => record,
            None => {
                return Err(format!("script index has no entry for {}", script.guid).into());
            }
        };
        assert!(record.path.starts_with(&root));
        assert_eq!(
            record
                .path
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("cs")
        );
        assert_eq!(
            record.relative_path,
            format!("{SCRIPT_DIRECTORY}/{}.cs", record.script_name)
        );
    }
    Ok(())
}

fn physics_lab_scene_path() -> PathBuf {
    sample_root()
        .join("Assets")
        .join("Scenes")
        .join("PhysicsLab.unity")
}

/// The target project's own physics-probe scene carries a `MeshCollider` that
/// names a concave mesh, and the runtime binds it as the mesh it names rather
/// than as the mesh's bounding box.
#[test]
fn the_sample_mesh_collider_produces_a_mesh_shaped_collider() -> TestResult {
    use unitas_physics::ColliderShape;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&physics_lab_scene_path())?;
    let graph = runtime.scene_graph().ok_or("no scene graph found")?;
    let entity = entity_named(graph, "MeshFrame")?;
    let scene = runtime.scene(0).ok_or("no loaded scene")?;
    let colliders = scene.body_colliders(entity);
    assert!(
        colliders
            .iter()
            .any(|collider| matches!(&collider.shape, ColliderShape::Mesh { .. })),
        "the MeshFrame's collider is the mesh it names: {colliders:?}"
    );
    Ok(())
}
