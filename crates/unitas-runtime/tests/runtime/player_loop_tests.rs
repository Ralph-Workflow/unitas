//! The player loop phase tree: the shape of the default tree, the custom
//! systems it runs, and the frame order the engine kept when the tree took
//! over the frame loop.

use std::cell::RefCell;
use std::error::Error;
use std::path::Path;

use unitas_runtime::player_loop;
use unitas_runtime::{
    Callbacks, CustomSystemPosition, PlayerLoopSystem, Runtime, test_read_player_loop_buffer,
    test_write_path_buffer, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_player_loop_buffer, unitas_runtime_player_loop_default_tree,
    unitas_runtime_player_loop_set_tree, unitas_runtime_player_loop_tree,
};

const SCENE_WITH_SCRIPT: &str = "tests/fixtures/scene_with_script.unity";
const BEFORE_UPDATE: &str = "UnityEngine.PlayerLoop.Update.BeforeUpdate";
const AFTER_UPDATE: &str = "UnityEngine.PlayerLoop.Update.AfterUpdate";
const INITIALIZATION_SYSTEM: &str = "UnityEngine.PlayerLoop.Initialization.CustomSystem";
const POST_LATE_SYSTEM: &str = "UnityEngine.PlayerLoop.PostLateUpdate.CustomSystem";

type TestResult = Result<(), Box<dyn Error>>;
type TraceResult = Result<Vec<&'static str>, Box<dyn Error>>;

// The trace is per test thread: the runtime runs the callbacks on the
// calling thread, so a thread-local recorder keeps parallel tests from
// reading each other's frames.
thread_local! {
    static TRACE: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn record(entry: &'static str) {
    TRACE.with(|trace| trace.borrow_mut().push(entry));
}

extern "C" fn noop_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn noop_handle(_handle: u64) {}

extern "C" fn record_fixed(_handle: u64, _delta: f64) {
    record("fixed");
}

extern "C" fn record_update(_handle: u64, _delta: f64) {
    record("update");
}

extern "C" fn record_late(_handle: u64, _delta: f64) {
    record("late");
}

extern "C" fn record_before(_user_data: u64, _delta: f64) {
    record("before");
}

extern "C" fn record_after(_user_data: u64, _delta: f64) {
    record("after");
}

extern "C" fn record_initialization(_user_data: u64, _delta: f64) {
    record("initialization");
}

extern "C" fn record_post_late(_user_data: u64, _delta: f64) {
    record("post_late");
}

extern "C" fn noop_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn noop_trigger(_a: u64, _b: u64, _phase: u32) {}

fn callbacks() -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: record_fixed,
        update: record_update,
        late_update: record_late,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: noop_collision,
        trigger: noop_trigger,
    }
}

fn trace() -> TraceResult {
    Ok(TRACE.with(|trace| trace.borrow().clone()))
}

fn clear_trace() -> TestResult {
    TRACE.with(|trace| trace.borrow_mut().clear());
    Ok(())
}

#[test]
fn the_default_player_loop_lists_the_seven_unity_phases_in_order() -> TestResult {
    let tree = player_loop::default_tree();
    let phases: Vec<&str> = tree
        .children
        .iter()
        .map(|phase| phase.type_name.as_str())
        .collect();
    assert_eq!(
        phases,
        vec![
            player_loop::INITIALIZATION,
            player_loop::EARLY_UPDATE,
            player_loop::FIXED_UPDATE,
            player_loop::PRE_UPDATE,
            player_loop::UPDATE,
            player_loop::PRE_LATE_UPDATE,
            player_loop::POST_LATE_UPDATE,
        ]
    );
    let runtime = Runtime::new()?;
    assert_eq!(runtime.player_loop_tree(), &tree);
    Ok(())
}

#[test]
fn a_custom_system_runs_at_its_registered_position_every_frame() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(callbacks());
    runtime.register_player_loop_custom_system(
        "BeforeUpdate",
        BEFORE_UPDATE,
        player_loop::SCRIPT_RUN_BEHAVIOUR_UPDATE,
        CustomSystemPosition::Before,
        record_before,
        0,
    )?;
    runtime.register_player_loop_custom_system(
        "AfterUpdate",
        AFTER_UPDATE,
        player_loop::SCRIPT_RUN_BEHAVIOUR_UPDATE,
        CustomSystemPosition::After,
        record_after,
        0,
    )?;
    // The first tick only initializes the scripts.
    runtime.tick(0.0)?;
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["before", "update", "after", "late"]);
    // The custom systems run at their position in every frame.
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["fixed", "before", "update", "after", "late"]);
    Ok(())
}

#[test]
fn removing_a_built_in_system_removes_its_work() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(callbacks());
    let mut tree = runtime.player_loop_tree().clone();
    assert!(tree.remove(player_loop::SCRIPT_RUN_BEHAVIOUR_UPDATE));
    runtime.set_player_loop_tree(tree)?;
    runtime.tick(0.0)?;
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["late"]);
    Ok(())
}

#[test]
fn a_player_loop_tree_round_trips_through_the_abi() -> TestResult {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let length = unitas_runtime_player_loop_default_tree(handle);
    assert_ne!(length, 0);
    let address = unitas_runtime_player_loop_buffer(handle, length);
    assert_ne!(address, 0);
    let mut default_bytes = vec![0u8; usize::try_from(length)?];
    test_read_player_loop_buffer(handle, &mut default_bytes)?;

    // The same tree handed back to the runtime reads as the same tree.
    let path_address = unitas_runtime_path_buffer(handle, length);
    assert_ne!(path_address, 0);
    test_write_path_buffer(handle, &default_bytes)?;
    assert_eq!(
        unitas_runtime_player_loop_set_tree(handle, path_address, length),
        0
    );
    let current_length = unitas_runtime_player_loop_tree(handle);
    assert_eq!(current_length, length);
    let current_address = unitas_runtime_player_loop_buffer(handle, current_length);
    assert_ne!(current_address, 0);
    let mut current_bytes = vec![0u8; usize::try_from(current_length)?];
    test_read_player_loop_buffer(handle, &mut current_bytes)?;
    assert_eq!(current_bytes, default_bytes);
    Ok(())
}

#[test]
fn the_frame_order_the_engine_ran_before_the_player_loop_is_unchanged() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(callbacks());
    runtime.register_player_loop_custom_system(
        "InitializationSystem",
        INITIALIZATION_SYSTEM,
        player_loop::PLAYER_UPDATE_TIME,
        CustomSystemPosition::After,
        record_initialization,
        0,
    )?;
    runtime.register_player_loop_custom_system(
        "PostLateSystem",
        POST_LATE_SYSTEM,
        player_loop::SCRIPT_RUN_DELAYED_TASKS,
        CustomSystemPosition::After,
        record_post_late,
        0,
    )?;
    runtime.tick(0.0)?;
    clear_trace()?;
    // The frame runs the initialization work, the fixed step, the update
    // callbacks, the late update callbacks, and finally the post late work.
    runtime.tick(0.03)?;
    assert_eq!(
        trace()?,
        vec!["initialization", "fixed", "update", "late", "post_late"]
    );
    Ok(())
}

#[test]
fn a_custom_system_can_be_unregistered() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(callbacks());
    runtime.register_player_loop_custom_system(
        "AfterUpdate",
        AFTER_UPDATE,
        player_loop::SCRIPT_RUN_BEHAVIOUR_UPDATE,
        CustomSystemPosition::After,
        record_after,
        0,
    )?;
    runtime.tick(0.0)?;
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["update", "after", "late"]);
    assert!(runtime.unregister_player_loop_custom_system(AFTER_UPDATE));
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["fixed", "update", "late"]);
    Ok(())
}

#[test]
fn a_tree_that_drops_a_phase_drops_the_work_it_carried() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(callbacks());
    let mut tree: PlayerLoopSystem = runtime.player_loop_tree().clone();
    let Some(post_late) = tree
        .children
        .iter()
        .position(|phase| phase.type_name == player_loop::POST_LATE_UPDATE)
    else {
        return Err("the default tree has a post late update phase".into());
    };
    tree.children.remove(post_late);
    runtime.set_player_loop_tree(tree)?;
    runtime.tick(0.0)?;
    clear_trace()?;
    runtime.tick(0.01)?;
    assert_eq!(trace()?, vec!["update"]);
    Ok(())
}
