use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_core::EntityId;
use unitas_physics::BodyRole;
use unitas_runtime::{Callbacks, Runtime};

const PHYSICS_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_collision.unity";
const TRIGGER_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_trigger.unity";
const SCRIPT_FIXTURE_PATH: &str = "tests/fixtures/scene_with_script.unity";
const ROTATED_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_rotated_nested.unity";
const BALL_NAME: &str = "Ball";
const CRATE_NAME: &str = "Crate";
const BLOCKER_NAME: &str = "Blocker";
const LAYER_ZERO_MASK: u32 = 1;
const QUERY_TOLERANCE: f64 = 1e-6;
const PHASE_ENTER: u32 = 0;
const PHASE_STAY: u32 = 1;
const PHASE_EXIT: u32 = 2;
const TICK_COUNT: usize = 40;
const TICK_DELTA: f64 = 0.02;
const EXPECTED_PHASE_SEQUENCE: &str = "0111111111111111111111112";

static PHYSICS_COLLISION_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static PHYSICS_TRIGGER_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static TRIGGER_COLLISION_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static TRIGGER_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static SCRIPT_COLLISION_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static SCRIPT_TRIGGER_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());

extern "C" fn record_physics_collision(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = PHYSICS_COLLISION_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn record_physics_trigger(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = PHYSICS_TRIGGER_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn record_trigger_collision(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = TRIGGER_COLLISION_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn record_trigger_event(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = TRIGGER_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn record_script_collision(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = SCRIPT_COLLISION_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn record_script_trigger(a: u64, b: u64, phase: u32) {
    if let Ok(mut events) = SCRIPT_TRIGGER_EVENTS.lock() {
        events.push((a, b, phase));
    }
}

extern "C" fn noop_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn noop_handle(_handle: u64) {}

extern "C" fn noop_handle_delta(_handle: u64, _delta: f64) {}

fn physics_callbacks() -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: noop_handle_delta,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: record_physics_collision,
        trigger: record_physics_trigger,
    }
}

fn trigger_callbacks() -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: noop_handle_delta,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: record_trigger_collision,
        trigger: record_trigger_event,
    }
}

fn script_callbacks() -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: noop_handle_delta,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: record_script_collision,
        trigger: record_script_trigger,
    }
}

fn entity_handle(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph loaded").into()),
    };
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(entity.id.as_u64());
        }
    }
    Err(String::from("entity not found in scene graph").into())
}

fn phase_sequence(events: &[(u64, u64, u32)]) -> String {
    events
        .iter()
        .map(|(_, _, phase)| char::from_digit(*phase, 10).unwrap_or('?'))
        .collect()
}

fn rotated_runtime() -> Result<Runtime, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ROTATED_FIXTURE_PATH))?;
    Ok(runtime)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= QUERY_TOLERANCE,
        "{label}: expected {expected}, got {actual}"
    );
}

fn assert_vector_close(actual: [f64; 3], expected: [f64; 3], label: &str) {
    for index in 0..3 {
        assert!(
            (actual[index] - expected[index]).abs() <= QUERY_TOLERANCE,
            "{label} component {index}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn runtime_raycast_hits_rotated_crate_from_above() -> Result<(), Box<dyn Error>> {
    let runtime = rotated_runtime()?;
    let crate_handle = entity_handle(&runtime, CRATE_NAME)?;
    let hits = runtime.raycast(
        [0.0, 10.0, 0.0],
        [0.0, -1.0, 0.0],
        20.0,
        LAYER_ZERO_MASK,
        false,
    );
    assert_eq!(hits.len(), 1, "expected a single hit, got {hits:?}");
    let hit = &hits[0];
    assert_eq!(hit.entity, crate_handle);
    assert_close(hit.distance, 7.0, "crate top distance");
    assert_vector_close(hit.point, [0.0, 3.0, 0.0], "crate top point");
    assert_vector_close(hit.normal, [0.0, 1.0, 0.0], "crate top normal");
    Ok(())
}

#[test]
fn runtime_raycast_hits_rotated_crate_from_the_side() -> Result<(), Box<dyn Error>> {
    let runtime = rotated_runtime()?;
    let crate_handle = entity_handle(&runtime, CRATE_NAME)?;
    let hits = runtime.raycast(
        [-5.0, 2.0, 0.0],
        [1.0, 0.0, 0.0],
        20.0,
        LAYER_ZERO_MASK,
        false,
    );
    assert_eq!(hits.len(), 1, "expected a single hit, got {hits:?}");
    let hit = &hits[0];
    assert_eq!(hit.entity, crate_handle);
    assert_close(hit.distance, 4.5, "crate side distance");
    assert_vector_close(hit.point, [-0.5, 2.0, 0.0], "crate side point");
    assert_vector_close(hit.normal, [-1.0, 0.0, 0.0], "crate side normal");
    Ok(())
}

#[test]
fn runtime_raycast_hits_blocker_below_the_rotated_crate() -> Result<(), Box<dyn Error>> {
    let runtime = rotated_runtime()?;
    let blocker_handle = entity_handle(&runtime, BLOCKER_NAME)?;
    let hits = runtime.raycast(
        [-5.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        20.0,
        LAYER_ZERO_MASK,
        false,
    );
    assert_eq!(hits.len(), 1, "expected a single hit, got {hits:?}");
    let hit = &hits[0];
    assert_eq!(hit.entity, blocker_handle);
    assert_close(hit.distance, 6.5, "blocker distance");
    assert_vector_close(hit.point, [1.5, 0.0, 0.0], "blocker point");
    assert_vector_close(hit.normal, [-1.0, 0.0, 0.0], "blocker normal");
    Ok(())
}

#[test]
fn runtime_overlap_sphere_finds_the_rotated_crate_only() -> Result<(), Box<dyn Error>> {
    let runtime = rotated_runtime()?;
    let crate_handle = entity_handle(&runtime, CRATE_NAME)?;
    let blocker_handle = entity_handle(&runtime, BLOCKER_NAME)?;
    let overlaps = runtime.overlap_sphere([0.0, 2.0, 0.0], 0.1, LAYER_ZERO_MASK, false);
    assert!(
        overlaps.iter().any(|record| record.entity == crate_handle),
        "the crate centre must overlap, got {overlaps:?}"
    );
    assert!(
        !overlaps
            .iter()
            .any(|record| record.entity == blocker_handle),
        "the blocker must not overlap, got {overlaps:?}"
    );
    Ok(())
}

#[test]
fn runtime_dispatches_ball_crate_collision_phases() -> Result<(), Box<dyn Error>> {
    if let Ok(mut events) = PHYSICS_COLLISION_EVENTS.lock() {
        events.clear();
    }
    if let Ok(mut events) = PHYSICS_TRIGGER_EVENTS.lock() {
        events.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(PHYSICS_FIXTURE_PATH))?;
    runtime.set_callbacks(physics_callbacks());
    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }
    let events = match PHYSICS_COLLISION_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let trigger_events = match PHYSICS_TRIGGER_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let ball = entity_handle(&runtime, BALL_NAME)?;
    let crate_handle = entity_handle(&runtime, CRATE_NAME)?;
    assert_eq!(runtime.scripts().len(), 1);
    assert!(
        trigger_events.is_empty(),
        "unexpected trigger events on non-trigger scene: {trigger_events:?}"
    );
    assert!(!events.is_empty(), "no collision events were dispatched");
    assert_eq!(events[0].2, PHASE_ENTER);
    assert!(events.iter().any(|(_, _, phase)| *phase == PHASE_STAY));
    assert_eq!(events[events.len() - 1].2, PHASE_EXIT);
    for (a, b, _) in &events {
        assert_eq!(*a, ball);
        assert_eq!(*b, crate_handle);
    }
    assert_eq!(phase_sequence(&events), EXPECTED_PHASE_SEQUENCE);
    Ok(())
}

#[test]
fn runtime_dispatches_ball_crate_trigger_phases() -> Result<(), Box<dyn Error>> {
    if let Ok(mut events) = TRIGGER_COLLISION_EVENTS.lock() {
        events.clear();
    }
    if let Ok(mut events) = TRIGGER_EVENTS.lock() {
        events.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE_PATH))?;
    runtime.set_callbacks(trigger_callbacks());
    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }
    let collision_events = match TRIGGER_COLLISION_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let trigger_events = match TRIGGER_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let ball = entity_handle(&runtime, BALL_NAME)?;
    let crate_handle = entity_handle(&runtime, CRATE_NAME)?;
    assert_eq!(runtime.scripts().len(), 1);
    assert!(
        collision_events.is_empty(),
        "unexpected collision events on trigger scene: {collision_events:?}"
    );
    assert!(
        !trigger_events.is_empty(),
        "no trigger events were dispatched"
    );
    assert_eq!(trigger_events[0].2, PHASE_ENTER);
    assert!(
        trigger_events
            .iter()
            .any(|(_, _, phase)| *phase == PHASE_STAY)
    );
    assert_eq!(trigger_events[trigger_events.len() - 1].2, PHASE_EXIT);
    for (a, b, _) in &trigger_events {
        assert_eq!(*a, ball);
        assert_eq!(*b, crate_handle);
    }
    assert_eq!(phase_sequence(&trigger_events), EXPECTED_PHASE_SEQUENCE);
    Ok(())
}

#[test]
fn runtime_scene_without_physics_dispatches_no_collisions() -> Result<(), Box<dyn Error>> {
    if let Ok(mut events) = SCRIPT_COLLISION_EVENTS.lock() {
        events.clear();
    }
    if let Ok(mut events) = SCRIPT_TRIGGER_EVENTS.lock() {
        events.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCRIPT_FIXTURE_PATH))?;
    runtime.set_callbacks(script_callbacks());
    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }
    let collision_events = match SCRIPT_COLLISION_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let trigger_events = match SCRIPT_TRIGGER_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    assert!(
        collision_events.is_empty(),
        "unexpected collision events: {collision_events:?}"
    );
    assert!(
        trigger_events.is_empty(),
        "unexpected trigger events on non-trigger scene: {trigger_events:?}"
    );
    assert_eq!(runtime.scripts().len(), 1);
    Ok(())
}

#[test]
fn runtime_maps_body_roles_and_writes_simulated_dynamic_positions_back_to_scene_graph()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_resting_sphere_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("resting_sphere.unity");
    let yaml = include_str!("../fixtures/scene_physics_collision.unity").replace(
        "  m_LocalPosition: {x: 0.9999999, y: 0.499, z: 0}",
        "  m_LocalPosition: {x: 0, y: 1.5, z: 0}",
    );
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;
    let ball = EntityId::new(entity_handle(&runtime, BALL_NAME)?);
    let crate_entity = EntityId::new(entity_handle(&runtime, CRATE_NAME)?);
    assert_eq!(runtime.body_role(ball), Some(BodyRole::Dynamic));
    assert_eq!(runtime.body_role(crate_entity), Some(BodyRole::Static));

    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }

    let ball_body_pos = match runtime.body_position(ball) {
        Some(pos) => pos,
        None => return Err(String::from("missing ball body position").into()),
    };
    let crate_body_pos = match runtime.body_position(crate_entity) {
        Some(pos) => pos,
        None => return Err(String::from("missing crate body position").into()),
    };
    assert_vector_close(ball_body_pos, [0.0, 1.25, 0.0], "ball body position");
    assert_vector_close(crate_body_pos, [0.0, 0.0, 0.0], "crate body position");

    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("missing scene graph").into()),
    };
    let ball_local = match graph.transform(ball) {
        Some(transform) => transform.translation,
        None => return Err(String::from("missing ball local transform").into()),
    };
    let ball_world = match graph.world_transform(ball) {
        Some(transform) => transform.translation,
        None => return Err(String::from("missing ball world transform").into()),
    };
    assert_close(
        f64::from(ball_local[1]),
        ball_body_pos[1],
        "ball local transform y",
    );
    assert_close(
        f64::from(ball_world[1]),
        ball_body_pos[1],
        "ball world transform y",
    );
    let crate_local = match graph.transform(crate_entity) {
        Some(transform) => transform.translation,
        None => return Err(String::from("missing crate local transform").into()),
    };
    assert_eq!(crate_local, [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn runtime_maps_kinematic_rigidbody_role_and_preserves_transform_under_gravity()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_kinematic_role_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("kinematic_role.unity");
    let yaml = include_str!("../fixtures/scene_physics_collision.unity").replace(
        "  m_Mass: 1\n  m_UseGravity: 1",
        "  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 1",
    );
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let ball = EntityId::new(entity_handle(&runtime, BALL_NAME)?);
    assert_eq!(runtime.body_role(ball), Some(BodyRole::Kinematic));
    for _ in 0..10 {
        runtime.tick(TICK_DELTA)?;
    }
    let pos = match runtime.body_position(ball) {
        Some(pos) => pos,
        None => return Err(String::from("missing ball body position").into()),
    };
    assert_vector_close(pos, [0.9999999, 0.499, 0.0], "kinematic ball position");
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("missing scene graph").into()),
    };
    let local = match graph.transform(ball) {
        Some(transform) => transform.translation,
        None => return Err(String::from("missing ball transform").into()),
    };
    assert_close(f64::from(local[1]), 0.499, "kinematic ball transform y");
    Ok(())
}

#[test]
fn runtime_writes_simulated_dynamic_position_back_for_parented_collider_with_offset()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_parented_dynamic_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("parented_dynamic.unity");
    let yaml = include_str!("../fixtures/scene_physics_rotated_nested.unity").replace(
        "  - component: {fileID: 1311639361}\n  - component: {fileID: 1311639362}",
        "  - component: {fileID: 1311639361}\n  - component: {fileID: 1311639362}\n  - component: {fileID: 1311639365}",
    )
    .replace(
        "  m_Center: {x: 0, y: 0, z: 0}\n--- !u!1 &519420062",
        "  m_Center: {x: 0.5, y: 0, z: 0}\n--- !u!54 &1311639365\nRigidbody:\n  m_ObjectHideFlags: 0\n  m_CorrespondingSourceObject: {fileID: 0}\n  m_PrefabInstance: {fileID: 0}\n  m_PrefabAsset: {fileID: 0}\n  m_GameObject: {fileID: 519420061}\n  m_Mass: 1\n  m_UseGravity: 1\n--- !u!1 &519420062",
    );
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let crate_entity = EntityId::new(entity_handle(&runtime, CRATE_NAME)?);
    assert_eq!(runtime.body_role(crate_entity), Some(BodyRole::Dynamic));
    runtime.tick(TICK_DELTA)?;
    runtime.tick(TICK_DELTA)?;

    let body_pos = match runtime.body_position(crate_entity) {
        Some(pos) => pos,
        None => return Err(String::from("missing crate body position").into()),
    };
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("missing scene graph").into()),
    };
    let world_transform = match graph.world_transform(crate_entity) {
        Some(transform) => transform,
        None => return Err(String::from("missing crate world transform").into()),
    };
    assert_close(
        f64::from(world_transform.translation[0]),
        body_pos[0],
        "parented crate world x",
    );
    assert_close(
        f64::from(world_transform.translation[1]) + 0.5,
        body_pos[1],
        "parented crate world y plus rotated offset",
    );
    assert_close(
        f64::from(world_transform.translation[2]),
        body_pos[2],
        "parented crate world z",
    );
    Ok(())
}

#[test]
fn runtime_syncs_kinematic_scene_graph_transform_edits_and_carries_resting_dynamic_body()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_kinematic_carry_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("kinematic_carry.unity");
    let yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Platform\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!54 &102\nRigidbody:\n  m_GameObject: {fileID: 100}\n  m_Mass: 1\n  m_UseGravity: 0\n  m_IsKinematic: 1\n--- !u!65 &103\nBoxCollider:\n  m_GameObject: {fileID: 100}\n  m_IsTrigger: 0\n  m_Size: {x: 4, y: 1, z: 4}\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: Cargo\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {fileID: 200}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 1.0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!54 &202\nRigidbody:\n  m_GameObject: {fileID: 200}\n  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 0\n--- !u!65 &203\nBoxCollider:\n  m_GameObject: {fileID: 200}\n  m_IsTrigger: 0\n  m_Size: {x: 1, y: 1, z: 1}\n  m_Center: {x: 0, y: 0, z: 0}\n";
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let platform = EntityId::new(entity_handle(&runtime, "Platform")?);
    let cargo = EntityId::new(entity_handle(&runtime, "Cargo")?);
    assert_eq!(runtime.body_role(platform), Some(BodyRole::Kinematic));
    assert_eq!(runtime.body_role(cargo), Some(BodyRole::Dynamic));

    runtime.tick(TICK_DELTA)?;
    for step in 1..=25 {
        let x = step as f32 * 0.04;
        let graph = match runtime.scene_graph_mut() {
            Some(graph) => graph,
            None => return Err(String::from("missing mutable scene graph").into()),
        };
        let transform = match graph.transform_mut(platform) {
            Some(transform) => transform,
            None => return Err(String::from("missing platform transform").into()),
        };
        transform.translation[0] = x;
        runtime.tick(TICK_DELTA)?;
    }

    let platform_pos = match runtime.body_position(platform) {
        Some(pos) => pos,
        None => return Err(String::from("missing platform body position").into()),
    };
    let cargo_pos = match runtime.body_position(cargo) {
        Some(pos) => pos,
        None => return Err(String::from("missing cargo body position").into()),
    };
    assert_close(platform_pos[0], 1.0, "platform x after kinematic sync");
    assert!(
        cargo_pos[0] > 0.3,
        "expected resting dynamic body to be carried horizontally, got x = {}",
        cargo_pos[0]
    );
    assert_close(cargo_pos[1], 1.0, "cargo resting y on platform");
    Ok(())
}

#[test]
fn runtime_maps_physic_material_restitution_and_records_unresolved_material_diagnostics()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_physic_material_{}",
        std::process::id()
    ));
    let assets_dir = temp_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;

    let mat_guid = "feedfacefeedfacefeedfacefeedface";
    let missing_guid = "deadbeefdeadbeefdeadbeefdeadbeef";
    let mat_path = assets_dir.join("Bouncy.physicMaterial");
    let mat_meta_path = assets_dir.join("Bouncy.physicMaterial.meta");
    std::fs::write(
        &mat_path,
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!134 &13400000\nPhysicMaterial:\n  m_Name: Bouncy\n  m_DynamicFriction: 0.2\n  m_StaticFriction: 0.2\n  m_Bounciness: 1.0\n",
    )?;
    std::fs::write(
        &mat_meta_path,
        format!("fileFormatVersion: 2\nguid: {mat_guid}\n"),
    )?;

    let bouncy_scene = assets_dir.join("BouncyScene.unity");
    std::fs::write(
        &bouncy_scene,
        format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Floor\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!65 &102\nBoxCollider:\n  m_GameObject: {{fileID: 100}}\n  m_Material: {{fileID: 13400000, guid: {mat_guid}, type: 2}}\n  m_IsTrigger: 0\n  m_Size: {{x: 4, y: 1, z: 4}}\n  m_Center: {{x: 0, y: 0, z: 0}}\n--- !u!1 &200\nGameObject:\n  m_Name: Ball\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {{fileID: 200}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 2.0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!54 &202\nRigidbody:\n  m_GameObject: {{fileID: 200}}\n  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 0\n--- !u!135 &203\nSphereCollider:\n  m_GameObject: {{fileID: 200}}\n  m_Material: {{fileID: 13400000, guid: {mat_guid}, type: 2}}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Center: {{x: 0, y: 0, z: 0}}\n"
        ),
    )?;

    let unresolved_scene = assets_dir.join("UnresolvedScene.unity");
    std::fs::write(
        &unresolved_scene,
        format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Floor\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!65 &102\nBoxCollider:\n  m_GameObject: {{fileID: 100}}\n  m_Material: {{fileID: 13400000, guid: {missing_guid}, type: 2}}\n  m_IsTrigger: 0\n  m_Size: {{x: 4, y: 1, z: 4}}\n  m_Center: {{x: 0, y: 0, z: 0}}\n--- !u!1 &200\nGameObject:\n  m_Name: Ball\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {{fileID: 200}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 2.0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!54 &202\nRigidbody:\n  m_GameObject: {{fileID: 200}}\n  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 0\n--- !u!135 &203\nSphereCollider:\n  m_GameObject: {{fileID: 200}}\n  m_Material: {{fileID: 13400000, guid: {missing_guid}, type: 2}}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Center: {{x: 0, y: 0, z: 0}}\n"
        ),
    )?;

    let mut bouncy_runtime = Runtime::new()?;
    let bouncy_load = bouncy_runtime.load_scene(&bouncy_scene);
    let mut unresolved_runtime = Runtime::new()?;
    let unresolved_load = unresolved_runtime.load_scene(&unresolved_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    bouncy_load?;
    unresolved_load?;

    assert!(bouncy_runtime.diagnostics().is_empty());
    let bouncy_ball = EntityId::new(entity_handle(&bouncy_runtime, "Ball")?);
    let mut max_rebound_velocity = 0.0_f64;
    for _ in 0..35 {
        bouncy_runtime.tick(TICK_DELTA)?;
        if let Some(vel) = bouncy_runtime.body_velocity(bouncy_ball)
            && vel[1] > max_rebound_velocity
        {
            max_rebound_velocity = vel[1];
        }
    }
    assert!(
        max_rebound_velocity > 2.0,
        "expected bouncy ball to rebound upward, max v_y = {max_rebound_velocity}"
    );

    let unresolved_ball = EntityId::new(entity_handle(&unresolved_runtime, "Ball")?);
    let unresolved_floor = EntityId::new(entity_handle(&unresolved_runtime, "Floor")?);
    assert_eq!(unresolved_runtime.diagnostics().len(), 2);
    let ball_diags = unresolved_runtime.entity_diagnostics(unresolved_ball);
    assert_eq!(ball_diags.len(), 1);
    assert!(ball_diags[0].contains(missing_guid));
    let floor_diags = unresolved_runtime.entity_diagnostics(unresolved_floor);
    assert_eq!(floor_diags.len(), 1);
    assert!(floor_diags[0].contains(missing_guid));

    let mut unresolved_max_rebound = 0.0_f64;
    for _ in 0..35 {
        unresolved_runtime.tick(TICK_DELTA)?;
        if let Some(vel) = unresolved_runtime.body_velocity(unresolved_ball)
            && vel[1] > unresolved_max_rebound
        {
            unresolved_max_rebound = vel[1];
        }
    }
    assert!(
        unresolved_max_rebound < 0.1,
        "expected unresolved material fallback to have zero restitution, max v_y = {unresolved_max_rebound}"
    );
    Ok(())
}

#[test]
fn phy_001_runtime_physics_world_reports_no_events_for_pairs_that_cannot_exchange_messages()
-> Result<(), Box<dyn Error>> {
    use unitas_physics::{BodyDesc, BodyRole, Shape};
    use unitas_runtime::physics::PhysicsRuntimeWorld;

    let mut runtime_world = PhysicsRuntimeWorld::new();
    let floor = EntityId::new(1);
    let trigger_zone = EntityId::new(2);
    let crate_body = EntityId::new(3);
    let dynamic_body = EntityId::new(4);

    runtime_world.add_body_3d(
        floor,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [2.0, 0.5, 2.0],
            },
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;
    runtime_world.add_body_3d(
        trigger_zone,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            position: [10.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: true,
            layer: 0,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;
    runtime_world.add_body_3d(
        crate_body,
        BodyDesc {
            role: BodyRole::Static,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            position: [10.5, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;
    runtime_world.add_body_3d(
        dynamic_body,
        BodyDesc {
            role: BodyRole::Dynamic,
            shape: Shape::Sphere { radius: 0.5 },
            position: [0.0, 0.4, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;

    let (collisions, triggers) = runtime_world.step_3d(0.02)?;
    assert!(
        collisions.len() == 1 && collisions[0].a == floor && collisions[0].b == dynamic_body,
        "PHY-001: the runtime physics world must report a collision event only for the pair that carries a Rigidbody, got {collisions:?}"
    );
    assert!(
        triggers.is_empty(),
        "PHY-001 story 13: the runtime physics world must report no trigger events for two colliders without a Rigidbody"
    );
    Ok(())
}

#[test]
fn phy_001_story_13_runtime_routes_3d_messages_to_collider_and_rigidbody_game_objects_and_honors_action_matrix()
 {
    use unitas_physics::matrix::{CollisionMessage, PhysicsObject};
    use unitas_runtime::physics::PhysicsRuntimeWorld;

    let mut runtime_world = PhysicsRuntimeWorld::new();

    let mut kinematic_platform = PhysicsObject::new(100, 0);
    kinematic_platform.has_rigidbody = true;
    kinematic_platform.is_kinematic = true;

    let mut compound_dynamic = PhysicsObject::new(200, 1);
    compound_dynamic.has_rigidbody = true;
    compound_dynamic.collider_object_id = Some(201);
    compound_dynamic.rigidbody_object_id = Some(200);

    let collision_deliveries = runtime_world.route_3d_message(
        CollisionMessage::OnCollisionEnter,
        &kinematic_platform,
        &compound_dynamic,
    );
    let mut collision_receivers: Vec<u64> = collision_deliveries
        .iter()
        .map(|d| d.receiver_object_id)
        .collect();
    collision_receivers.sort_unstable();
    assert_eq!(
        collision_receivers,
        vec![100, 201, 200]
            .into_iter()
            .fold(Vec::new(), |mut acc, x| {
                acc.push(x);
                acc.sort_unstable();
                acc
            }),
        "PHY-001 story 13: route_3d_message must deliver OnCollisionEnter to kinematic body and both child collider and parent Rigidbody GameObjects"
    );

    let mut trigger_a = PhysicsObject::new(300, 0);
    trigger_a.is_trigger = true;
    let mut trigger_b = PhysicsObject::new(301, 0);
    trigger_b.is_trigger = true;
    assert!(
        runtime_world
            .route_3d_message(CollisionMessage::OnTriggerEnter, &trigger_a, &trigger_b)
            .is_empty(),
        "PHY-001 story 13: trigger pair without a Rigidbody must route no trigger messages"
    );

    let mut disabled_trigger = PhysicsObject::new(400, 0);
    disabled_trigger.is_trigger = true;
    disabled_trigger.behaviour_enabled = false;
    let trigger_deliveries = runtime_world.route_3d_message(
        CollisionMessage::OnTriggerEnter,
        &disabled_trigger,
        &compound_dynamic,
    );
    assert!(
        trigger_deliveries
            .iter()
            .any(|d| d.receiver_object_id == 400 && d.message == CollisionMessage::OnTriggerEnter),
        "PHY-001 story 13: disabled MonoBehaviour on trigger collider GameObject must still receive OnTriggerEnter"
    );

    runtime_world.set_layer_collision_3d(0, 1, false);
    assert!(!runtime_world.layer_collision_enabled_3d(0, 1));
    assert!(
        runtime_world
            .route_3d_message(
                CollisionMessage::OnTriggerEnter,
                &disabled_trigger,
                &compound_dynamic,
            )
            .is_empty(),
        "PHY-001 story 2 & story 13: disabling layer pair in 3D matrix stops message routing"
    );
}
