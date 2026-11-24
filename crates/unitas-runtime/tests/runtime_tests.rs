use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_runtime::{
    test_read_query_buffer, test_write_path_buffer, unitas_runtime_destroy,
    unitas_runtime_error_message, unitas_runtime_load_scene, unitas_runtime_new,
    unitas_runtime_overlap_sphere, unitas_runtime_path_buffer, unitas_runtime_query_buffer,
    unitas_runtime_raycast, Callbacks, Runtime,
};

const FIXTURE_PATH: &str = "tests/fixtures/scene_with_script.unity";
const QUERY_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_query_abi.unity";
const QUERY_RECORD_SIZE: usize = 40;

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

extern "C" fn lc_collision(_a: u64, _b: u64, _phase: u32) {}
extern "C" fn lc_trigger(_a: u64, _b: u64, _phase: u32) {}

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
        collision: lc_collision,
        trigger: lc_trigger,
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

extern "C" fn accum_collision(_a: u64, _b: u64, _phase: u32) {}
extern "C" fn accum_trigger(_a: u64, _b: u64, _phase: u32) {}

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
        collision: accum_collision,
        trigger: accum_trigger,
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

struct QueryRecord {
    entity: u64,
    distance: f64,
    point: [f64; 3],
}

fn entity_id_for(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph found").into()),
    };
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id) {
            if entity.name == name {
                return Ok(id.as_u64());
            }
        }
    }
    Err(String::from("entity not found").into())
}

fn decode_record(bytes: &[u8], offset: usize) -> Result<QueryRecord, Box<dyn Error>> {
    let entity = u64::from_le_bytes(bytes[offset..offset + 8].try_into()?);
    let distance = f64::from_le_bytes(bytes[offset + 8..offset + 16].try_into()?);
    let point = [
        f64::from_le_bytes(bytes[offset + 16..offset + 24].try_into()?),
        f64::from_le_bytes(bytes[offset + 24..offset + 32].try_into()?),
        f64::from_le_bytes(bytes[offset + 32..offset + 40].try_into()?),
    ];
    Ok(QueryRecord {
        entity,
        distance,
        point,
    })
}

fn query_bytes(handle: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = vec![0u8; 4 * QUERY_RECORD_SIZE];
    test_read_query_buffer(handle, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn runtime_abi_query_exports_decode_hits() -> Result<(), Box<dyn Error>> {
    let mut probe = Runtime::new()?;
    probe.load_scene(Path::new(QUERY_FIXTURE_PATH))?;
    let target = entity_id_for(&probe, "QueryTarget")?;
    let other = entity_id_for(&probe, "QueryOther")?;

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let address = unitas_runtime_path_buffer(handle, 256);
    assert_ne!(address, 0);
    let fixture_bytes = QUERY_FIXTURE_PATH.as_bytes();
    test_write_path_buffer(handle, fixture_bytes)?;
    let rc = unitas_runtime_load_scene(handle, address, fixture_bytes.len() as u64);
    assert_eq!(rc, 0);

    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let count = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 4);
    assert_eq!(count, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert_eq!(record.entity, target);
    assert!((record.distance - 4.5).abs() < 1e-9);
    assert!((record.point[0] + 0.5).abs() < 1e-9);
    assert!(record.point[1].abs() < 1e-9);
    assert!(record.point[2].abs() < 1e-9);

    let all = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, u32::MAX);
    assert_eq!(all, 2);
    let bytes = query_bytes(handle)?;
    let first = decode_record(&bytes, 0)?;
    let second = decode_record(&bytes, QUERY_RECORD_SIZE)?;
    assert_eq!(first.entity, target);
    assert!((first.distance - 4.5).abs() < 1e-9);
    assert_eq!(second.entity, other);
    assert!((second.distance - 6.5).abs() < 1e-9);
    assert!((second.point[0] - 1.5).abs() < 1e-9);

    let masked = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 2);
    assert_eq!(masked, 0);

    let overlap = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, u32::MAX);
    assert_eq!(overlap, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert_eq!(record.entity, target);
    assert_eq!(record.distance, 0.0);
    assert_eq!(record.point, [0.0, 0.0, 0.0]);

    let other_overlap = unitas_runtime_overlap_sphere(handle, 2.0, 0.0, 0.0, 0.5, u32::MAX);
    assert_eq!(other_overlap, 1);
    let bytes = query_bytes(handle)?;
    assert_eq!(decode_record(&bytes, 0)?.entity, other);

    let masked_overlap = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, 2);
    assert_eq!(masked_overlap, 0);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_query_exports_reject_bad_handle() {
    let bad = unitas_runtime_raycast(u64::MAX, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 10.0, u32::MAX);
    assert!(bad < 0);
    let bad_overlap = unitas_runtime_overlap_sphere(u64::MAX, 0.0, 0.0, 0.0, 1.0, u32::MAX);
    assert!(bad_overlap < 0);
    assert_eq!(unitas_runtime_query_buffer(u64::MAX, 64), 0);
}
