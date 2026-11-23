use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_runtime::{
    test_write_path_buffer, unitas_runtime_destroy, unitas_runtime_error_message,
    unitas_runtime_load_scene, unitas_runtime_new, unitas_runtime_path_buffer, Callbacks, Runtime,
};

const FIXTURE_PATH: &str = "tests/fixtures/scene_with_script.unity";

#[test]
fn runtime_scene_binding_extracts_script_and_guid_halves() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    let path = Path::new(FIXTURE_PATH);
    let count = runtime.load_scene(path)?;
    assert_eq!(count, 1);
    let scripts = runtime.scripts();
    assert_eq!(scripts.len(), 1);
    let script = match scripts.first() {
        Some(s) => s,
        None => return Err(String::from("no script found").into()),
    };
    assert_eq!(script.guid, "00000000000000000000000000000001");
    assert_eq!(script.guid_hi, 0);
    assert_eq!(script.guid_lo, 1);
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph found").into()),
    };
    let entity = match graph.entity(script.entity) {
        Some(e) => e,
        None => return Err(String::from("script entity not in scene graph").into()),
    };
    assert_eq!(entity.name, "ScriptedObject");
    Ok(())
}

static LIFECYCLE_EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

extern "C" fn lc_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    100
}

extern "C" fn lc_awake(_handle: u64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("Awake"));
    }
}

extern "C" fn lc_on_enable(_handle: u64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("OnEnable"));
    }
}

extern "C" fn lc_start(_handle: u64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("Start"));
    }
}

extern "C" fn lc_fixed_update(_handle: u64, _step: f64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("FixedUpdate"));
    }
}

extern "C" fn lc_update(_handle: u64, _dt: f64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("Update"));
    }
}

extern "C" fn lc_late_update(_handle: u64, _dt: f64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("LateUpdate"));
    }
}

extern "C" fn lc_on_disable(_handle: u64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("OnDisable"));
    }
}

extern "C" fn lc_destroy(_handle: u64) {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.push(String::from("OnDestroy"));
    }
}

#[test]
fn runtime_lifecycle_order_over_two_ticks_and_shutdown() -> Result<(), Box<dyn Error>> {
    if let Ok(mut events) = LIFECYCLE_EVENTS.lock() {
        events.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(FIXTURE_PATH))?;
    runtime.set_callbacks(Callbacks {
        create_script: lc_create_script,
        awake: lc_awake,
        on_enable: lc_on_enable,
        start: lc_start,
        fixed_update: lc_fixed_update,
        update: lc_update,
        late_update: lc_late_update,
        on_disable: lc_on_disable,
        destroy: lc_destroy,
    });
    runtime.tick(0.02)?;
    runtime.tick(0.02)?;
    runtime.shutdown()?;
    let events = match LIFECYCLE_EVENTS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let expected = vec![
        String::from("Awake"),
        String::from("OnEnable"),
        String::from("Start"),
        String::from("FixedUpdate"),
        String::from("Update"),
        String::from("LateUpdate"),
        String::from("OnDisable"),
        String::from("OnDestroy"),
    ];
    assert_eq!(events, expected);
    assert_eq!(
        events.join(", "),
        "Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate, OnDisable, OnDestroy"
    );
    Ok(())
}

static ACCUM_FIXED_COUNTS: Mutex<usize> = Mutex::new(0);

extern "C" fn accum_create(_entity: u64, _hi: u64, _lo: u64) -> u64 {
    1
}
extern "C" fn accum_noop_u64(_handle: u64) {}
extern "C" fn accum_noop_u64_f64(_handle: u64, _dt: f64) {}
extern "C" fn accum_fixed(_handle: u64, _step: f64) {
    if let Ok(mut count) = ACCUM_FIXED_COUNTS.lock() {
        *count += 1;
    }
}

#[test]
fn runtime_fixed_step_accumulator_accumulates_and_dispatches() -> Result<(), Box<dyn Error>> {
    if let Ok(mut count) = ACCUM_FIXED_COUNTS.lock() {
        *count = 0;
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(FIXTURE_PATH))?;
    runtime.set_callbacks(Callbacks {
        create_script: accum_create,
        awake: accum_noop_u64,
        on_enable: accum_noop_u64,
        start: accum_noop_u64,
        fixed_update: accum_fixed,
        update: accum_noop_u64_f64,
        late_update: accum_noop_u64_f64,
        on_disable: accum_noop_u64,
        destroy: accum_noop_u64,
    });
    runtime.tick(0.02)?;
    let count0 = match ACCUM_FIXED_COUNTS.lock() {
        Ok(c) => *c,
        Err(_) => return Err(String::from("poisoned").into()),
    };
    assert_eq!(count0, 0);

    runtime.tick(0.01)?;
    let count1 = match ACCUM_FIXED_COUNTS.lock() {
        Ok(c) => *c,
        Err(_) => return Err(String::from("poisoned").into()),
    };
    assert_eq!(count1, 0);

    runtime.tick(0.01)?;
    let count2 = match ACCUM_FIXED_COUNTS.lock() {
        Ok(c) => *c,
        Err(_) => return Err(String::from("poisoned").into()),
    };
    assert_eq!(count2, 1);

    runtime.tick(0.04)?;
    let count3 = match ACCUM_FIXED_COUNTS.lock() {
        Ok(c) => *c,
        Err(_) => return Err(String::from("poisoned").into()),
    };
    assert_eq!(count3, 3);
    Ok(())
}

#[test]
fn runtime_abi_path_buffer_round_trip() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let address = unitas_runtime_path_buffer(handle, 256);
    assert_ne!(address, 0);
    let fixture_bytes = FIXTURE_PATH.as_bytes();
    test_write_path_buffer(handle, fixture_bytes)?;
    let rc = unitas_runtime_load_scene(handle, address, fixture_bytes.len() as u64);
    assert_eq!(rc, 0);
    let err_ptr = unitas_runtime_error_message(handle);
    assert_eq!(err_ptr, 0);
    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_error_path_for_missing_scene_file() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    let missing_path = Path::new("tests/fixtures/nonexistent_scene.unity");
    let result = runtime.load_scene(missing_path);
    assert!(result.is_err());

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let address = unitas_runtime_path_buffer(handle, 256);
    assert_ne!(address, 0);
    let bad_bytes = b"tests/fixtures/nonexistent_scene.unity";
    test_write_path_buffer(handle, bad_bytes)?;
    let rc = unitas_runtime_load_scene(handle, address, bad_bytes.len() as u64);
    assert!(rc < 0);
    let err_ptr = unitas_runtime_error_message(handle);
    assert_ne!(err_ptr, 0);
    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}
