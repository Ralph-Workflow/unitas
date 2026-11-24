use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

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
        if let Some(entity) = graph.entity(id) {
            if entity.name == name {
                return Ok(entity.id.as_u64());
            }
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
        !overlaps.iter().any(|record| record.entity == blocker_handle),
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
    assert!(trigger_events
        .iter()
        .any(|(_, _, phase)| *phase == PHASE_STAY));
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
        "unexpected trigger events: {trigger_events:?}"
    );
    assert_eq!(runtime.scripts().len(), 1);
    Ok(())
}
