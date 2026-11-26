use std::cell::RefCell;
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

#[path = "roll_a_ball_tests.rs"]
mod roll_a_ball_tests;

use unitas_runtime::{
    Callbacks, LoadSceneMode, Runtime, RuntimeError, test_read_query_buffer,
    test_write_path_buffer, unitas_runtime_box_cast, unitas_runtime_destroy,
    unitas_runtime_error_message, unitas_runtime_fixed_timestep, unitas_runtime_linecast,
    unitas_runtime_load_scene, unitas_runtime_new, unitas_runtime_overlap_sphere,
    unitas_runtime_path_buffer, unitas_runtime_query_buffer, unitas_runtime_raycast,
    unitas_runtime_set_time_scale, unitas_runtime_sphere_cast, unitas_runtime_time_scale,
};

const FIXTURE_PATH: &str = "tests/fixtures/scene_with_script.unity";
const QUERY_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_query_abi.unity";
const TRIGGER_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_trigger.unity";
const ALPHA_FIXTURE_PATH: &str = "tests/fixtures/scene_alpha.unity";
const BETA_FIXTURE_PATH: &str = "tests/fixtures/scene_beta.unity";
const COLLISION_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_collision.unity";
const NESTED_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_rotated_nested.unity";
const MISSING_FIXTURE_PATH: &str = "tests/fixtures/scene_missing.unity";
const QUERY_RECORD_SIZE: usize = 64;

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
    normal: [f64; 3],
}

fn entity_id_for(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph found").into()),
    };
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(id.as_u64());
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
    let normal = [
        f64::from_le_bytes(bytes[offset + 40..offset + 48].try_into()?),
        f64::from_le_bytes(bytes[offset + 48..offset + 56].try_into()?),
        f64::from_le_bytes(bytes[offset + 56..offset + 64].try_into()?),
    ];
    Ok(QueryRecord {
        entity,
        distance,
        point,
        normal,
    })
}

fn query_bytes(handle: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = vec![0u8; 4 * QUERY_RECORD_SIZE];
    test_read_query_buffer(handle, &mut bytes)?;
    Ok(bytes)
}

fn query_runtime_handle() -> Result<u64, Box<dyn Error>> {
    runtime_handle_for(QUERY_FIXTURE_PATH)
}

fn runtime_handle_for(fixture: &str) -> Result<u64, Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let address = unitas_runtime_path_buffer(handle, 256);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    let fixture_bytes = fixture.as_bytes();
    test_write_path_buffer(handle, fixture_bytes)?;
    let rc = unitas_runtime_load_scene(handle, address, fixture_bytes.len() as u64);
    if rc != 0 {
        return Err(String::from("query fixture scene load failed").into());
    }
    Ok(handle)
}

fn unit_length(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

#[test]
fn runtime_abi_query_exports_decode_hits() -> Result<(), Box<dyn Error>> {
    let mut probe = Runtime::new()?;
    probe.load_scene(Path::new(QUERY_FIXTURE_PATH))?;
    let target = entity_id_for(&probe, "QueryTarget")?;
    let other = entity_id_for(&probe, "QueryOther")?;

    let handle = query_runtime_handle()?;

    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let count = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 32, 0);
    assert_eq!(count, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert_eq!(record.entity, target);
    assert!((record.distance - 4.5).abs() < 1e-9);
    assert!((record.point[0] + 0.5).abs() < 1e-9);
    assert!(record.point[1].abs() < 1e-9);
    assert!(record.point[2].abs() < 1e-9);
    assert!((record.normal[0] + 1.0).abs() < 1e-9);
    assert!(record.normal[1].abs() < 1e-9);
    assert!(record.normal[2].abs() < 1e-9);
    let normal_length = unit_length(record.normal);
    assert!((normal_length - 1.0).abs() < 1e-9);

    let all = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, u32::MAX, 0);
    assert_eq!(all, 2);
    let bytes = query_bytes(handle)?;
    let first = decode_record(&bytes, 0)?;
    let second = decode_record(&bytes, QUERY_RECORD_SIZE)?;
    assert_eq!(first.entity, target);
    assert!((first.distance - 4.5).abs() < 1e-9);
    assert_eq!(second.entity, other);
    assert!((second.distance - 6.5).abs() < 1e-9);
    assert!((second.point[0] - 1.5).abs() < 1e-9);
    assert!((second.normal[0] + 1.0).abs() < 1e-9);
    assert!(second.normal[1].abs() < 1e-9);
    assert!(second.normal[2].abs() < 1e-9);

    let masked = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 2, 0);
    assert_eq!(masked, 0);

    let overlap = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, u32::MAX, 0);
    assert_eq!(overlap, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert_eq!(record.entity, target);
    assert_eq!(record.distance, 0.0);
    assert_eq!(record.point, [0.0, 0.0, 0.0]);
    assert_eq!(record.normal, [0.0, 0.0, 0.0]);

    let other_overlap = unitas_runtime_overlap_sphere(handle, 2.0, 0.0, 0.0, 0.5, u32::MAX, 0);
    assert_eq!(other_overlap, 1);
    let bytes = query_bytes(handle)?;
    assert_eq!(decode_record(&bytes, 0)?.entity, other);

    let masked_overlap = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, 2, 0);
    assert_eq!(masked_overlap, 0);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_layer_mask_selects_only_the_requested_layer() -> Result<(), Box<dyn Error>> {
    let handle = query_runtime_handle()?;
    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let layer_two = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 4, 0);
    assert_eq!(layer_two, 0);

    let layer_five = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 32, 0);
    assert_eq!(layer_five, 1);
    let bytes = query_bytes(handle)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(QUERY_FIXTURE_PATH))?;
    let target = entity_id_for(&runtime, "QueryTarget")?;
    assert_eq!(decode_record(&bytes, 0)?.entity, target);
    assert!(
        runtime
            .raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 4, false)
            .is_empty()
    );
    assert_eq!(
        runtime
            .raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 32, false)
            .len(),
        1
    );

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_query_flag_controls_trigger_inclusion() -> Result<(), Box<dyn Error>> {
    let mut probe = Runtime::new()?;
    probe.load_scene(Path::new(TRIGGER_FIXTURE_PATH))?;
    let ball = entity_id_for(&probe, "Ball")?;
    let crate_entity = entity_id_for(&probe, "Crate")?;

    let handle = runtime_handle_for(TRIGGER_FIXTURE_PATH)?;
    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let without_triggers =
        unitas_runtime_raycast(handle, 0.0, 5.0, 0.0, 0.0, -1.0, 0.0, 20.0, 1, 0);
    assert_eq!(without_triggers, 1);
    let bytes = query_bytes(handle)?;
    assert_eq!(decode_record(&bytes, 0)?.entity, ball);

    let with_triggers = unitas_runtime_raycast(handle, 0.0, 5.0, 0.0, 0.0, -1.0, 0.0, 20.0, 1, 1);
    assert_eq!(with_triggers, 2);
    let bytes = query_bytes(handle)?;
    assert_eq!(decode_record(&bytes, 0)?.entity, ball);
    assert_eq!(
        decode_record(&bytes, QUERY_RECORD_SIZE)?.entity,
        crate_entity
    );

    let overlap_without = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, 1, 0);
    assert_eq!(overlap_without, 1);
    let bytes = query_bytes(handle)?;
    assert_eq!(decode_record(&bytes, 0)?.entity, ball);

    let overlap_with = unitas_runtime_overlap_sphere(handle, 0.0, 0.0, 0.0, 0.5, 1, 1);
    assert_eq!(overlap_with, 2);
    let bytes = query_bytes(handle)?;
    let mut overlapped = [
        decode_record(&bytes, 0)?.entity,
        decode_record(&bytes, QUERY_RECORD_SIZE)?.entity,
    ];
    overlapped.sort_unstable();
    let mut expected = [ball, crate_entity];
    expected.sort_unstable();
    assert_eq!(overlapped, expected);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_query_exports_reject_bad_handle() {
    let bad = unitas_runtime_raycast(u64::MAX, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 10.0, u32::MAX, 0);
    assert!(bad < 0);
    let bad_overlap = unitas_runtime_overlap_sphere(u64::MAX, 0.0, 0.0, 0.0, 1.0, u32::MAX, 0);
    assert!(bad_overlap < 0);
    assert_eq!(unitas_runtime_query_buffer(u64::MAX, 64), 0);
}

#[test]
fn runtime_abi_new_cast_exports_reject_bad_handle() {
    let bad_line =
        unitas_runtime_linecast(u64::MAX, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 10.0, u32::MAX, 0);
    assert!(bad_line < 0);
    let bad_sphere = unitas_runtime_sphere_cast(
        u64::MAX,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        1.0,
        10.0,
        u32::MAX,
        0,
    );
    assert!(bad_sphere < 0);
    let bad_box = unitas_runtime_box_cast(
        u64::MAX,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        1.0,
        1.0,
        1.0,
        10.0,
        u32::MAX,
        0,
    );
    assert!(bad_box < 0);
}

#[test]
fn runtime_abi_linecast_matches_raycast() -> Result<(), Box<dyn Error>> {
    let handle = query_runtime_handle()?;
    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let ray_count = unitas_runtime_raycast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 32, 0);
    assert_eq!(ray_count, 1);
    let bytes = query_bytes(handle)?;
    let ray = decode_record(&bytes, 0)?;

    let line_count = unitas_runtime_linecast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 20.0, 32, 0);
    assert_eq!(line_count, 1);
    let bytes = query_bytes(handle)?;
    let line = decode_record(&bytes, 0)?;
    assert_eq!(line.entity, ray.entity);
    assert!((line.distance - ray.distance).abs() < 1e-9);
    assert!((line.point[0] - ray.point[0]).abs() < 1e-9);
    assert!((line.point[1] - ray.point[1]).abs() < 1e-9);
    assert!((line.point[2] - ray.point[2]).abs() < 1e-9);
    assert!((line.normal[0] + 1.0).abs() < 1e-9);
    assert!(line.normal[1].abs() < 1e-9);
    assert!(line.normal[2].abs() < 1e-9);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_sphere_cast_reports_target_with_normal() -> Result<(), Box<dyn Error>> {
    let handle = query_runtime_handle()?;
    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let count =
        unitas_runtime_sphere_cast(handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.25, 20.0, 32, 0);
    assert_eq!(count, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert!(record.distance > 4.0 && record.distance <= 4.6);
    assert!((record.point[0] + 0.5).abs() < 1e-9);
    assert!(record.point[1].abs() < 1e-9);
    assert!(record.point[2].abs() < 1e-9);
    assert!((record.normal[0] + 1.0).abs() < 1e-9);
    assert!(record.normal[1].abs() < 1e-9);
    assert!(record.normal[2].abs() < 1e-9);
    assert!((unit_length(record.normal) - 1.0).abs() < 1e-9);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_abi_box_cast_reports_target_with_normal() -> Result<(), Box<dyn Error>> {
    let handle = query_runtime_handle()?;
    let buffer = unitas_runtime_query_buffer(handle, 4 * QUERY_RECORD_SIZE as u32);
    assert_ne!(buffer, 0);

    let count = unitas_runtime_box_cast(
        handle, -5.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 0.5, 0.5, 20.0, 32, 0,
    );
    assert_eq!(count, 1);
    let bytes = query_bytes(handle)?;
    let record = decode_record(&bytes, 0)?;
    assert!((record.distance - 4.0).abs() < 1e-2);
    assert!((record.point[0] + 0.5).abs() < 1e-2);
    assert!(record.point[1].abs() < 1e-2);
    assert!(record.point[2].abs() < 1e-2);
    assert!((record.normal[0] + 1.0).abs() < 1e-9);
    assert!(record.normal[1].abs() < 1e-9);
    assert!(record.normal[2].abs() < 1e-9);
    assert!((unit_length(record.normal) - 1.0).abs() < 1e-9);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);
    Ok(())
}

#[test]
fn runtime_query_api_exposes_linecast_sphere_cast_and_box_cast() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(QUERY_FIXTURE_PATH))?;
    let target = entity_id_for(&runtime, "QueryTarget")?;

    let ray = runtime.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 32, false);
    assert_eq!(ray.len(), 1);
    assert_eq!(ray[0].entity, target);
    assert!((ray[0].distance - 4.5).abs() < 1e-9);
    assert!((ray[0].normal[0] + 1.0).abs() < 1e-9);

    let line = runtime.linecast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 32, false);
    assert_eq!(line.len(), 1);
    assert_eq!(line[0].entity, target);
    assert!((line[0].distance - ray[0].distance).abs() < 1e-9);

    let sphere = runtime.sphere_cast([-5.0, 0.0, 0.0], 0.25, [1.0, 0.0, 0.0], 20.0, 32, false);
    assert_eq!(sphere.len(), 1);
    assert_eq!(sphere[0].entity, target);
    assert!(sphere[0].distance > 4.0 && sphere[0].distance <= 4.6);
    assert!((sphere[0].normal[0] + 1.0).abs() < 1e-9);

    let boxed = runtime.box_cast(
        [-5.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        20.0,
        32,
        false,
    );
    assert_eq!(boxed.len(), 1);
    assert_eq!(boxed[0].entity, target);
    assert!((boxed[0].distance - 4.0).abs() < 1e-2);
    assert!((boxed[0].normal[0] + 1.0).abs() < 1e-9);

    let overlap = runtime.overlap_sphere([0.0, 0.0, 0.0], 0.5, 32, false);
    assert_eq!(overlap.len(), 1);
    assert_eq!(overlap[0].entity, target);
    assert_eq!(overlap[0].distance, 0.0);
    assert_eq!(overlap[0].point, [0.0, 0.0, 0.0]);
    assert_eq!(overlap[0].normal, [0.0, 0.0, 0.0]);
    Ok(())
}

const SETTINGS_FIXTURE_PATH: &str =
    "tests/fixtures/project_with_settings/Assets/Scenes/SettingsScene.unity";

static SETTINGS_FIXED_STEPS: Mutex<Vec<f64>> = Mutex::new(Vec::new());

extern "C" fn settings_fixed_update(_handle: u64, step: f64) {
    if let Ok(mut steps) = SETTINGS_FIXED_STEPS.lock() {
        steps.push(step);
    }
}

#[test]
fn runtime_load_scene_applies_project_settings_timestep_and_gravity() -> Result<(), Box<dyn Error>>
{
    if let Ok(mut steps) = SETTINGS_FIXED_STEPS.lock() {
        steps.clear();
    }
    let mut runtime = Runtime::new()?;
    assert_eq!(runtime.fixed_timestep(), 0.02);

    runtime.load_scene(Path::new(SETTINGS_FIXTURE_PATH))?;
    let expected_dt = 0.03333333333333333;
    assert_eq!(runtime.fixed_timestep(), expected_dt);

    runtime.set_callbacks(Callbacks {
        create_script: accum_create,
        awake: accum_noop_u64,
        on_enable: accum_noop_u64,
        start: accum_noop_u64,
        fixed_update: settings_fixed_update,
        update: accum_noop_u64_f64,
        late_update: accum_noop_u64_f64,
        on_disable: accum_noop_u64,
        destroy: accum_noop_u64,
        collision: accum_collision,
        trigger: accum_trigger,
    });

    runtime.tick(0.0)?;
    runtime.tick(0.04)?;

    let steps = match SETTINGS_FIXED_STEPS.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return Err(String::from("poisoned").into()),
    };
    assert_eq!(steps, vec![expected_dt]);

    let ball_id = entity_id_for(&runtime, "Ball")?;
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph").into()),
    };
    let ball_entity = match graph
        .breadth_first_order()
        .into_iter()
        .find(|id| id.as_u64() == ball_id)
    {
        Some(id) => id,
        None => return Err(String::from("ball entity not found").into()),
    };

    let velocity = match runtime.body_velocity(ball_entity) {
        Some(v) => v,
        None => return Err(String::from("ball body velocity not found").into()),
    };
    assert_eq!(velocity[0], 0.0);
    assert!((velocity[1] - (-2.0 * expected_dt)).abs() < 1e-6);
    assert_eq!(velocity[2], 0.0);

    let overlap = runtime.overlap_sphere([0.0, 10.0, 0.0], 2.0, u32::MAX, false);
    assert_eq!(overlap.len(), 1);
    assert!((overlap[0].point[1] - (10.0 + (-2.0 * expected_dt) * expected_dt)).abs() < 1e-3);
    Ok(())
}

#[test]
fn runtime_load_scene_without_or_with_malformed_project_settings_uses_defaults()
-> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE_PATH))?;
    assert_eq!(runtime.fixed_timestep(), 0.02);
    runtime.tick(0.0)?;
    runtime.tick(0.02)?;

    let ball_id = entity_id_for(&runtime, "Ball")?;
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph").into()),
    };
    let ball_entity = match graph
        .breadth_first_order()
        .into_iter()
        .find(|id| id.as_u64() == ball_id)
    {
        Some(id) => id,
        None => return Err(String::from("ball entity not found").into()),
    };
    let default_velocity = match runtime.body_velocity(ball_entity) {
        Some(v) => v,
        None => return Err(String::from("ball body velocity not found").into()),
    };
    assert!((default_velocity[1] - (-9.81 * 0.02)).abs() < 1e-6);

    let temp_root = std::env::temp_dir().join(format!(
        "unitas_runtime_malformed_settings_{}",
        std::process::id()
    ));
    let settings_dir = temp_root.join("ProjectSettings");
    let scenes_dir = temp_root.join("Assets").join("Scenes");
    std::fs::create_dir_all(&settings_dir)?;
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::write(
        settings_dir.join("TimeManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!5 &1\nTimeManager:\n  m_FixedTimestep: {unclosed\n",
    )?;
    std::fs::write(
        settings_dir.join("DynamicsManager.asset"),
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\nPhysicsManager:\n  m_Gravity: {unclosed\n",
    )?;
    let temp_scene = scenes_dir.join("Scene.unity");
    std::fs::copy(Path::new(SETTINGS_FIXTURE_PATH), &temp_scene)?;

    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_root);
    load_res?;

    assert_eq!(runtime.fixed_timestep(), 0.02);
    runtime.tick(0.0)?;
    runtime.tick(0.02)?;
    let malformed_velocity = match runtime.body_velocity(ball_entity) {
        Some(v) => v,
        None => return Err(String::from("ball body velocity not found").into()),
    };
    assert!((malformed_velocity[1] - (-9.81 * 0.02)).abs() < 1e-6);
    Ok(())
}

#[test]
fn runtime_abi_fixed_timestep_returns_configured_value_and_sets_error_on_invalid_handle()
-> Result<(), Box<dyn Error>> {
    let handle = runtime_handle_for(SETTINGS_FIXTURE_PATH)?;
    let step = unitas_runtime_fixed_timestep(handle);
    assert_eq!(step, 0.03333333333333333);
    assert_eq!(unitas_runtime_error_message(handle), 0);

    let destroy_rc = unitas_runtime_destroy(handle);
    assert_eq!(destroy_rc, 0);

    let invalid_step = unitas_runtime_fixed_timestep(u64::MAX);
    assert_eq!(invalid_step, 0.0);
    let err_ptr = unitas_runtime_error_message(u64::MAX);
    assert_ne!(err_ptr, 0);
    Ok(())
}

static TIMESCALE_FIXED_STEPS: Mutex<usize> = Mutex::new(0);
static TIMESCALE_UPDATE_DELTAS: Mutex<Vec<f64>> = Mutex::new(Vec::new());
static TIMESCALE_RUN_LOCK: Mutex<()> = Mutex::new(());

extern "C" fn timescale_create(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn timescale_noop_u64(_handle: u64) {}

extern "C" fn timescale_noop_f64(_handle: u64, _delta: f64) {}

extern "C" fn timescale_fixed(_handle: u64, _step: f64) {
    if let Ok(mut steps) = TIMESCALE_FIXED_STEPS.lock() {
        *steps += 1;
    }
}

extern "C" fn timescale_update(_handle: u64, delta: f64) {
    if let Ok(mut deltas) = TIMESCALE_UPDATE_DELTAS.lock() {
        deltas.push(delta);
    }
}

extern "C" fn timescale_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn timescale_trigger(_a: u64, _b: u64, _phase: u32) {}

struct TimeScaleRun {
    fixed_steps: usize,
    fall_distance: f64,
    update_deltas: Vec<f64>,
}

fn run_time_scale_scene(time_scale: f64, ticks: usize) -> Result<TimeScaleRun, Box<dyn Error>> {
    let _run_guard = match TIMESCALE_RUN_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    if let Ok(mut steps) = TIMESCALE_FIXED_STEPS.lock() {
        *steps = 0;
    }
    if let Ok(mut deltas) = TIMESCALE_UPDATE_DELTAS.lock() {
        deltas.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SETTINGS_FIXTURE_PATH))?;
    runtime.set_time_scale(time_scale);
    runtime.set_callbacks(Callbacks {
        create_script: timescale_create,
        awake: timescale_noop_u64,
        on_enable: timescale_noop_u64,
        start: timescale_noop_u64,
        fixed_update: timescale_fixed,
        update: timescale_update,
        late_update: timescale_noop_f64,
        on_disable: timescale_noop_u64,
        destroy: timescale_noop_u64,
        collision: timescale_collision,
        trigger: timescale_trigger,
    });
    let ball_id = entity_id_for(&runtime, "Ball")?;
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph found").into()),
    };
    let ball_entity = match graph
        .breadth_first_order()
        .into_iter()
        .find(|id| id.as_u64() == ball_id)
    {
        Some(id) => id,
        None => return Err(String::from("ball entity not found").into()),
    };
    let start_y = match runtime.body_position(ball_entity) {
        Some(position) => position[1],
        None => return Err(String::from("ball body position not found").into()),
    };
    for _ in 0..ticks {
        runtime.tick(0.02)?;
    }
    let end_y = match runtime.body_position(ball_entity) {
        Some(position) => position[1],
        None => return Err(String::from("ball body position not found").into()),
    };
    let fixed_steps = match TIMESCALE_FIXED_STEPS.lock() {
        Ok(steps) => *steps,
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    let update_deltas = match TIMESCALE_UPDATE_DELTAS.lock() {
        Ok(deltas) => deltas.clone(),
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    Ok(TimeScaleRun {
        fixed_steps,
        fall_distance: start_y - end_y,
        update_deltas,
    })
}

#[test]
fn runtime_time_scale_defaults_to_one_and_is_settable() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    assert_eq!(runtime.time_scale(), 1.0);
    runtime.set_time_scale(0.5);
    assert_eq!(runtime.time_scale(), 0.5);
    runtime.set_time_scale(0.0);
    assert_eq!(runtime.time_scale(), 0.0);
    runtime.set_time_scale(-1.0);
    assert_eq!(runtime.time_scale(), -1.0);
    Ok(())
}

#[test]
fn runtime_freezing_time_scale_pauses_gameplay_audio() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    assert!(!runtime.audio().is_gameplay_paused());
    runtime.set_time_scale(0.0);
    assert!(runtime.audio().is_gameplay_paused());
    runtime.set_time_scale(1.0);
    assert!(!runtime.audio().is_gameplay_paused());
    Ok(())
}

#[test]
fn runtime_time_scale_halves_the_simulated_time_of_a_falling_body() -> Result<(), Box<dyn Error>> {
    let full = run_time_scale_scene(1.0, 51)?;
    let half = run_time_scale_scene(0.5, 51)?;
    let full_half_ticks = run_time_scale_scene(1.0, 26)?;

    assert_eq!(full.fixed_steps, 30);
    assert_eq!(half.fixed_steps, 15);
    assert_eq!(full_half_ticks.fixed_steps, 15);
    assert_eq!(full.update_deltas.len(), 50);
    assert_eq!(half.update_deltas.len(), 50);
    assert_eq!(full.update_deltas[0], 0.02);
    assert_eq!(half.update_deltas[0], 0.01);
    assert!((half.fall_distance - full_half_ticks.fall_distance).abs() < 1e-12);
    assert!(half.fall_distance < full.fall_distance);
    assert!(full.fall_distance > 1.0);
    Ok(())
}

#[test]
fn runtime_zero_time_scale_freezes_the_world_while_update_still_runs() -> Result<(), Box<dyn Error>>
{
    let frozen = run_time_scale_scene(0.0, 51)?;
    assert_eq!(frozen.fixed_steps, 0);
    assert_eq!(frozen.fall_distance, 0.0);
    assert_eq!(frozen.update_deltas.len(), 50);
    assert_eq!(frozen.update_deltas[0], 0.0);
    Ok(())
}

#[test]
fn runtime_abi_time_scale_round_trips_and_rejects_invalid_handles() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    assert_eq!(unitas_runtime_time_scale(handle), 1.0);
    assert_eq!(unitas_runtime_set_time_scale(handle, 0.5), 0);
    assert_eq!(unitas_runtime_time_scale(handle), 0.5);
    assert_eq!(unitas_runtime_set_time_scale(u64::MAX, 0.5), -1);
    assert_eq!(unitas_runtime_time_scale(u64::MAX), 0.0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

thread_local! {
    static SCENE_EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SCENE_UPDATE_HANDLES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

fn record_scene_event(event: &str) {
    SCENE_EVENTS.with(|events| events.borrow_mut().push(event.to_string()));
}
extern "C" fn scene_create_script(entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    entity
}

extern "C" fn scene_awake(_handle: u64) {
    record_scene_event("Awake");
}

extern "C" fn scene_on_enable(_handle: u64) {
    record_scene_event("OnEnable");
}

extern "C" fn scene_start(_handle: u64) {
    record_scene_event("Start");
}

extern "C" fn scene_fixed_update(_handle: u64, _step: f64) {}

extern "C" fn scene_update(handle: u64, _delta: f64) {
    SCENE_UPDATE_HANDLES.with(|handles| handles.borrow_mut().push(handle));
}

extern "C" fn scene_late_update(_handle: u64, _delta: f64) {}

extern "C" fn scene_on_disable(_handle: u64) {
    record_scene_event("OnDisable");
}

extern "C" fn scene_destroy(_handle: u64) {
    record_scene_event("OnDestroy");
}

extern "C" fn scene_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn scene_trigger(_a: u64, _b: u64, _phase: u32) {}

fn scene_callbacks() -> Callbacks {
    Callbacks {
        create_script: scene_create_script,
        awake: scene_awake,
        on_enable: scene_on_enable,
        start: scene_start,
        fixed_update: scene_fixed_update,
        update: scene_update,
        late_update: scene_late_update,
        on_disable: scene_on_disable,
        destroy: scene_destroy,
        collision: scene_collision,
        trigger: scene_trigger,
    }
}

fn clear_scene_state() {
    SCENE_EVENTS.with(|events| events.borrow_mut().clear());
    SCENE_UPDATE_HANDLES.with(|handles| handles.borrow_mut().clear());
}

fn scene_events() -> Vec<String> {
    SCENE_EVENTS.with(|events| events.borrow().clone())
}

fn scene_update_handles() -> Vec<u64> {
    SCENE_UPDATE_HANDLES.with(|handles| handles.borrow().clone())
}

#[test]
fn runtime_single_load_replaces_the_loaded_scene_set() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_alpha"));
    assert_eq!(runtime.scripts().len(), 1);
    assert_eq!(
        runtime.scripts()[0].guid,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph found").into()),
    };
    let alpha_name = graph
        .entity(runtime.scripts()[0].entity)
        .map(|entity| entity.name.clone());
    assert_eq!(alpha_name, Some(String::from("AlphaObject")));

    runtime.load_scene(Path::new(BETA_FIXTURE_PATH))?;
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_beta"));
    assert_eq!(runtime.scene_name(1), None);
    assert_eq!(runtime.scripts().len(), 1);
    assert_eq!(
        runtime.scripts()[0].guid,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    );
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph found").into()),
    };
    let beta_name = graph
        .entity(runtime.scripts()[0].entity)
        .map(|entity| entity.name.clone());
    assert_eq!(beta_name, Some(String::from("BetaObject")));
    Ok(())
}

#[test]
fn runtime_additive_load_keeps_both_scenes_alive() -> Result<(), Box<dyn Error>> {
    clear_scene_state();
    let mut runtime = Runtime::new()?;
    runtime.set_callbacks(scene_callbacks());
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    let count =
        runtime.load_scene_with_mode(Path::new(BETA_FIXTURE_PATH), LoadSceneMode::Additive)?;
    assert_eq!(count, 2);
    assert_eq!(runtime.scene_count(), 2);
    assert_eq!(runtime.scene_name(0), Some("scene_alpha"));
    assert_eq!(runtime.scene_name(1), Some("scene_beta"));
    assert_eq!(runtime.active_scene_index(), Some(0));
    let alpha_scene = match runtime.scene(0) {
        Some(scene) => scene,
        None => return Err(String::from("alpha scene missing").into()),
    };
    let beta_scene = match runtime.scene(1) {
        Some(scene) => scene,
        None => return Err(String::from("beta scene missing").into()),
    };
    let alpha_entity = match alpha_scene.scripts().first() {
        Some(script) => script.entity,
        None => return Err(String::from("alpha script missing").into()),
    };
    let beta_entity = match beta_scene.scripts().first() {
        Some(script) => script.entity,
        None => return Err(String::from("beta script missing").into()),
    };
    assert_ne!(alpha_entity, beta_entity);
    runtime.tick(0.02)?;
    runtime.tick(0.02)?;
    let handles = scene_update_handles();
    assert!(handles.contains(&alpha_entity.as_u64()));
    assert!(handles.contains(&beta_entity.as_u64()));
    Ok(())
}

#[test]
fn runtime_active_scene_selection_rejects_out_of_range_indexes() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    assert_eq!(runtime.active_scene_index(), None);
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    assert_eq!(runtime.active_scene_index(), Some(0));
    runtime.load_scene_with_mode(Path::new(BETA_FIXTURE_PATH), LoadSceneMode::Additive)?;
    runtime.set_active_scene(1)?;
    assert_eq!(runtime.active_scene_index(), Some(1));
    match runtime.set_active_scene(5) {
        Err(RuntimeError::InvalidSceneIndex(index)) => assert_eq!(index, 5),
        Err(_) => return Err(String::from("unexpected error variant").into()),
        Ok(()) => return Err(String::from("out of range scene index accepted").into()),
    }
    assert_eq!(runtime.active_scene_index(), Some(1));
    assert_eq!(runtime.scene_count(), 2);
    assert_eq!(runtime.scene_name(1), Some("scene_beta"));
    Ok(())
}

#[test]
fn runtime_unload_of_an_unknown_scene_is_an_error_and_changes_nothing() -> Result<(), Box<dyn Error>>
{
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    match runtime.unload_scene("scene_missing") {
        Err(RuntimeError::SceneNotFound(name)) => assert_eq!(name, "scene_missing"),
        Err(_) => return Err(String::from("unexpected error variant").into()),
        Ok(()) => return Err(String::from("unload of an unknown scene succeeded").into()),
    }
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_alpha"));
    assert_eq!(runtime.active_scene_index(), Some(0));
    assert_eq!(runtime.scripts().len(), 1);
    Ok(())
}

#[test]
fn runtime_unload_scene_disables_then_destroys_its_scripts() -> Result<(), Box<dyn Error>> {
    clear_scene_state();
    let mut runtime = Runtime::new()?;
    runtime.set_callbacks(scene_callbacks());
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    runtime.tick(0.02)?;
    runtime.load_scene_with_mode(Path::new(BETA_FIXTURE_PATH), LoadSceneMode::Additive)?;
    runtime.tick(0.02)?;
    clear_scene_state();
    runtime.unload_scene("scene_beta")?;
    let events = scene_events();
    assert_eq!(
        events,
        vec![String::from("OnDisable"), String::from("OnDestroy")]
    );
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_alpha"));
    assert_eq!(runtime.active_scene_index(), Some(0));
    Ok(())
}

#[test]
fn runtime_unload_scene_removes_only_its_physics_bodies() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(COLLISION_FIXTURE_PATH))?;
    runtime.load_scene_with_mode(Path::new(NESTED_FIXTURE_PATH), LoadSceneMode::Additive)?;
    let collision_scene = match runtime.scene(0) {
        Some(scene) => scene,
        None => return Err(String::from("collision scene missing").into()),
    };
    let nested_scene = match runtime.scene(1) {
        Some(scene) => scene,
        None => return Err(String::from("nested scene missing").into()),
    };
    let collision_bodies: Vec<_> = collision_scene.bodies().to_vec();
    let nested_bodies: Vec<_> = nested_scene.bodies().to_vec();
    assert_eq!(collision_bodies.len(), 2);
    assert_eq!(nested_bodies.len(), 2);
    for body in &collision_bodies {
        assert!(runtime.body_position(*body).is_some());
    }
    for body in &nested_bodies {
        assert!(runtime.body_position(*body).is_some());
    }
    runtime.unload_scene("scene_physics_rotated_nested")?;
    for body in &nested_bodies {
        assert!(runtime.body_position(*body).is_none());
    }
    for body in &collision_bodies {
        assert!(runtime.body_position(*body).is_some());
    }
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_physics_collision"));
    Ok(())
}

#[test]
fn runtime_dont_destroy_on_load_survives_a_single_load_replacement() -> Result<(), Box<dyn Error>> {
    clear_scene_state();
    let mut runtime = Runtime::new()?;
    runtime.set_callbacks(scene_callbacks());
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    runtime.tick(0.02)?;
    let alpha_entity = match runtime.scripts().first() {
        Some(script) => script.entity,
        None => return Err(String::from("alpha script missing").into()),
    };
    runtime.dont_destroy_on_load(alpha_entity)?;
    assert!(runtime.is_persistent(alpha_entity));
    runtime.load_scene(Path::new(BETA_FIXTURE_PATH))?;
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_beta"));
    assert!(runtime.is_persistent(alpha_entity));
    runtime.tick(0.02)?;
    let events = scene_events();
    let awake = events.iter().filter(|event| *event == "Awake").count();
    let disabled = events.iter().filter(|event| *event == "OnDisable").count();
    let destroyed = events.iter().filter(|event| *event == "OnDestroy").count();
    assert_eq!(awake, 2);
    assert_eq!(disabled, 0);
    assert_eq!(destroyed, 0);
    let scripts = runtime.scripts();
    assert_eq!(scripts.len(), 2);
    let persistent = match scripts.iter().find(|script| script.entity == alpha_entity) {
        Some(script) => script,
        None => return Err(String::from("persistent script missing").into()),
    };
    assert_eq!(persistent.script_handle, alpha_entity.as_u64());
    assert!(persistent.awake_called);
    Ok(())
}

#[test]
fn runtime_failed_load_leaves_the_scene_set_and_world_unchanged() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(COLLISION_FIXTURE_PATH))?;
    let bodies: Vec<_> = match runtime.scene(0) {
        Some(scene) => scene.bodies().to_vec(),
        None => return Err(String::from("collision scene missing").into()),
    };
    let script_count = runtime.scripts().len();
    match runtime.load_scene(Path::new(MISSING_FIXTURE_PATH)) {
        Err(_) => {}
        Ok(_) => return Err(String::from("load of a missing scene succeeded").into()),
    }
    assert_eq!(runtime.scene_count(), 1);
    assert_eq!(runtime.scene_name(0), Some("scene_physics_collision"));
    assert_eq!(runtime.active_scene_index(), Some(0));
    assert_eq!(runtime.scripts().len(), script_count);
    for body in &bodies {
        assert!(runtime.body_position(*body).is_some());
    }
    Ok(())
}

#[test]
fn runtime_loads_capsule_collider_and_supports_raycast_and_overlap_queries()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_capsule_query_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("capsule_query.unity");
    let yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Pillar\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!136 &102\nCapsuleCollider:\n  m_GameObject: {fileID: 100}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Height: 2.0\n  m_Direction: 1\n  m_Center: {x: 0, y: 0, z: 0}\n";
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let pillar = entity_id_for(&runtime, "Pillar")?;
    let top_hits = runtime.raycast([0.0, 5.0, 0.0], [0.0, -1.0, 0.0], 20.0, 1, false);
    assert_eq!(top_hits.len(), 1);
    assert_eq!(top_hits[0].entity, pillar);
    assert!((top_hits[0].distance - 4.0).abs() < 1e-6);
    assert!((top_hits[0].point[1] - 1.0).abs() < 1e-6);
    assert!((top_hits[0].normal[1] - 1.0).abs() < 1e-6);

    let side_hits = runtime.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1, false);
    assert_eq!(side_hits.len(), 1);
    assert_eq!(side_hits[0].entity, pillar);
    assert!((side_hits[0].distance - 4.5).abs() < 1e-6);
    assert!((side_hits[0].point[0] + 0.5).abs() < 1e-6);
    assert!((side_hits[0].normal[0] + 1.0).abs() < 1e-6);

    let overlaps = runtime.overlap_sphere([0.0, 0.9, 0.0], 0.2, 1, false);
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0].entity, pillar);
    Ok(())
}

#[test]
fn runtime_maps_capsule_collider_direction_rotation_center_and_simulates_dynamic_fall()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_capsule_direction_sim_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join("capsule_direction_sim.unity");
    let yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: HorizontalBar\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!136 &102\nCapsuleCollider:\n  m_GameObject: {fileID: 100}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Height: 4.0\n  m_Direction: 0\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: RotatedZBar\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {fileID: 200}\n  m_LocalRotation: {x: 0, y: 0.70710678, z: 0, w: 0.70710678}\n  m_LocalPosition: {x: 0, y: 5, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!136 &202\nCapsuleCollider:\n  m_GameObject: {fileID: 200}\n  m_IsTrigger: 1\n  m_Radius: 0.5\n  m_Height: 4.0\n  m_Direction: 2\n  m_Center: {x: 0, y: 0, z: 1}\n--- !u!1 &300\nGameObject:\n  m_Name: FallingCapsule\n  m_Layer: 0\n--- !u!4 &301\nTransform:\n  m_GameObject: {fileID: 300}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 2.0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!54 &302\nRigidbody:\n  m_GameObject: {fileID: 300}\n  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 0\n--- !u!136 &303\nCapsuleCollider:\n  m_GameObject: {fileID: 300}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Height: 2.0\n  m_Direction: 1\n  m_Center: {x: 0, y: 0, z: 0}\n";
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let bar = entity_id_for(&runtime, "HorizontalBar")?;
    let rotated = entity_id_for(&runtime, "RotatedZBar")?;
    let falling_id = entity_id_for(&runtime, "FallingCapsule")?;

    let x_cap_hit = runtime.raycast([-5.0, 0.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1, false);
    assert_eq!(x_cap_hit.len(), 1);
    assert_eq!(x_cap_hit[0].entity, bar);
    assert!((x_cap_hit[0].distance - 3.0).abs() < 1e-6);
    assert!((x_cap_hit[0].point[0] + 2.0).abs() < 1e-6);

    let trigger_excluded = runtime.raycast([-5.0, 5.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1, false);
    assert!(trigger_excluded.is_empty());

    let trigger_included = runtime.raycast([-5.0, 5.0, 0.0], [1.0, 0.0, 0.0], 20.0, 1, true);
    assert_eq!(trigger_included.len(), 1);
    assert_eq!(trigger_included[0].entity, rotated);
    assert!((trigger_included[0].distance - 4.0).abs() < 1e-5);
    assert!((trigger_included[0].point[0] + 1.0).abs() < 1e-5);

    for _ in 0..40 {
        runtime.tick(0.02)?;
    }
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph").into()),
    };
    let falling_entity = match graph
        .breadth_first_order()
        .into_iter()
        .find(|id| id.as_u64() == falling_id)
    {
        Some(id) => id,
        None => return Err(String::from("falling entity not found").into()),
    };
    let pos = match runtime.body_position(falling_entity) {
        Some(p) => p,
        None => return Err(String::from("missing falling capsule position").into()),
    };
    assert!((pos[1] - 1.5).abs() < 1e-4);
    Ok(())
}

#[test]
fn runtime_forwards_capsule_material_and_geometry_diagnostics() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_capsule_diagnostics_{}",
        std::process::id()
    ));
    let assets_dir = temp_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;
    let missing_guid = "deadbeefdeadbeefdeadbeefdeadbeef";
    let scene_path = assets_dir.join("CapsuleDiagnostics.unity");
    let yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: ShortCapsule\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!136 &102\nCapsuleCollider:\n  m_GameObject: {{fileID: 100}}\n  m_Material: {{fileID: 13400000, guid: {missing_guid}, type: 2}}\n  m_IsTrigger: 0\n  m_Radius: 0.5\n  m_Height: 0.4\n  m_Direction: 1\n  m_Center: {{x: 0, y: 0, z: 0}}\n"
    );
    std::fs::write(&scene_path, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let capsule_id = entity_id_for(&runtime, "ShortCapsule")?;
    let graph = match runtime.scene_graph() {
        Some(g) => g,
        None => return Err(String::from("no scene graph").into()),
    };
    let capsule_entity = match graph
        .breadth_first_order()
        .into_iter()
        .find(|id| id.as_u64() == capsule_id)
    {
        Some(id) => id,
        None => return Err(String::from("capsule entity not found").into()),
    };
    let all_diags = runtime.diagnostics();
    assert_eq!(all_diags.len(), 2);
    let entity_diags = runtime.entity_diagnostics(capsule_entity);
    assert_eq!(entity_diags.len(), 2);
    assert!(entity_diags.iter().any(|msg| msg.contains(missing_guid)));
    assert!(entity_diags.iter().any(|msg| msg.contains("clamped")));

    let hits = runtime.raycast([0.0, 5.0, 0.0], [0.0, -1.0, 0.0], 20.0, 1, false);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entity, capsule_id);
    assert!((hits[0].distance - 4.8).abs() < 1e-6);
    Ok(())
}

fn build_test_wav(
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    pcm_samples: &[i16],
) -> Vec<u8> {
    let byte_rate = sample_rate * u32::from(channels) * (u32::from(bits_per_sample) / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (pcm_samples.len() * 2) as u32;
    let riff_size = 4 + (8 + 16) + (8 + data_size);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&audio_format.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in pcm_samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

#[test]
fn runtime_audio_scene_play_on_awake_ticks_to_completed_and_teardown_stops_source()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_audio_play_on_awake_{}",
        std::process::id()
    ));
    let assets_dir = temp_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;
    let wav_guid = "11111111111111111111111111111111";
    let wav_path = assets_dir.join("beep.wav");
    let wav_bytes = build_test_wav(1, 1, 10, 16, &[16384, 16384, 16384, 16384, 16384]);
    std::fs::write(&wav_path, &wav_bytes)?;
    std::fs::write(
        assets_dir.join("beep.wav.meta"),
        format!("fileFormatVersion: 2\nguid: {wav_guid}\n"),
    )?;
    let scene_path = assets_dir.join("AudioScene.unity");
    let yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Speaker\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 3, y: 4, z: 5}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!82 &102\nAudioSource:\n  m_GameObject: {{fileID: 100}}\n  m_Enabled: 1\n  m_audioClip: {{fileID: 8300000, guid: {wav_guid}, type: 3}}\n  m_PlayOnAwake: 1\n  m_Volume: 0.75\n  m_Pitch: 1.0\n  m_Loop: 0\n  m_Mute: 0\n  m_SpatialBlend: 1.0\n  m_MinDistance: 2.0\n  m_MaxDistance: 50.0\n  m_Priority: 64\n"
    );
    std::fs::write(&scene_path, yaml)?;

    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let speaker_id = entity_id_for(&runtime, "Speaker")?;
    let speaker = unitas_core::EntityId::new(speaker_id);
    let scene = match runtime.scene(0) {
        Some(s) => s,
        None => return Err(String::from("missing scene").into()),
    };
    assert_eq!(scene.audio_sources().len(), 1);
    assert_eq!(scene.audio_sources()[0].0, speaker);
    assert!(runtime.audio_source_config(speaker).is_some());
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Stopped
    );

    runtime.tick(0.02)?;
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Playing
    );
    assert_eq!(runtime.audio().source_position(speaker), [3.0, 4.0, 5.0]);

    runtime.tick(0.2)?;
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Playing
    );
    assert!((runtime.audio().source_time(speaker) - 0.2).abs() < 1e-6);

    runtime.tick(0.35)?;
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Completed
    );
    assert!((runtime.audio().source_time(speaker) - 0.5).abs() < 1e-6);

    runtime.audio_mut().play(speaker, false);
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Playing
    );
    runtime.unload_scene("AudioScene")?;
    assert_eq!(
        runtime.audio().source_state(speaker),
        unitas_audio::AudioSourceState::Stopped
    );
    assert!(runtime.audio().source(speaker).is_none());
    Ok(())
}

#[test]
fn runtime_audio_time_scale_and_application_pause_control_audio_advancement()
-> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let path_addr = unitas_runtime_path_buffer(handle, 256);
    assert_ne!(path_addr, 0);
    let fixture_bytes = FIXTURE_PATH.as_bytes();
    test_write_path_buffer(handle, fixture_bytes)?;
    assert_eq!(
        unitas_runtime_load_scene(handle, path_addr, fixture_bytes.len() as u64),
        0
    );

    let entity = unitas_runtime::unitas_runtime_audio_create(handle, 0);
    assert_ne!(entity, 0);
    let clip_id =
        unitas_runtime::unitas_runtime_audio_register_clip_samples(handle, 1, 100, 200, 0);
    assert_ne!(clip_id, 0);
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_clip(handle, entity, clip_id),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_play(handle, entity, 0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, entity),
        2
    );

    assert_eq!(unitas_runtime::unitas_runtime_tick(handle, 0.02), 0);

    assert_eq!(unitas_runtime_set_time_scale(handle, 0.0), 0);
    assert_eq!(unitas_runtime::unitas_runtime_tick(handle, 0.5), 0);
    assert!((unitas_runtime::unitas_runtime_audio_time(handle, entity) - 0.0).abs() < 1e-9);

    assert_eq!(unitas_runtime_set_time_scale(handle, 1.0), 0);
    assert_eq!(unitas_runtime::unitas_runtime_tick(handle, 0.5), 0);
    assert!((unitas_runtime::unitas_runtime_audio_time(handle, entity) - 0.5).abs() < 1e-9);

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_paused(handle, 1),
        0
    );
    assert_eq!(unitas_runtime::unitas_runtime_tick(handle, 0.5), 0);
    assert!((unitas_runtime::unitas_runtime_audio_time(handle, entity) - 0.5).abs() < 1e-9);

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_paused(handle, 0),
        0
    );
    assert_eq!(unitas_runtime::unitas_runtime_tick(handle, 0.25), 0);
    assert!((unitas_runtime::unitas_runtime_audio_time(handle, entity) - 0.75).abs() < 1e-9);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn runtime_audio_unsupported_or_invalid_clip_emits_diagnostic_and_fails_source()
-> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_audio_diagnostics_{}",
        std::process::id()
    ));
    let assets_dir = temp_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;

    let ogg_guid = "22222222222222222222222222222222";
    let ogg_path = assets_dir.join("music.ogg");
    std::fs::write(&ogg_path, b"OggS\x00\x02\x00\x00")?;
    std::fs::write(
        assets_dir.join("music.ogg.meta"),
        format!("fileFormatVersion: 2\nguid: {ogg_guid}\n"),
    )?;

    let bad_wav_guid = "33333333333333333333333333333333";
    let bad_wav_path = assets_dir.join("float.wav");
    let bad_wav_bytes = build_test_wav(3, 1, 44100, 16, &[0, 0]);
    std::fs::write(&bad_wav_path, &bad_wav_bytes)?;
    std::fs::write(
        assets_dir.join("float.wav.meta"),
        format!("fileFormatVersion: 2\nguid: {bad_wav_guid}\n"),
    )?;

    let scene_path = assets_dir.join("BadAudioScene.unity");
    let yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: OggSpeaker\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!82 &102\nAudioSource:\n  m_GameObject: {{fileID: 100}}\n  m_Enabled: 1\n  m_audioClip: {{fileID: 8300000, guid: {ogg_guid}, type: 3}}\n  m_PlayOnAwake: 1\n--- !u!1 &200\nGameObject:\n  m_Name: BadWavSpeaker\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {{fileID: 200}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 0, y: 0, z: 0}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!82 &202\nAudioSource:\n  m_GameObject: {{fileID: 200}}\n  m_Enabled: 1\n  m_audioClip: {{fileID: 8300000, guid: {bad_wav_guid}, type: 3}}\n  m_PlayOnAwake: 1\n"
    );
    std::fs::write(&scene_path, yaml)?;

    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let ogg_id = unitas_core::EntityId::new(entity_id_for(&runtime, "OggSpeaker")?);
    let bad_wav_id = unitas_core::EntityId::new(entity_id_for(&runtime, "BadWavSpeaker")?);

    assert_eq!(runtime.diagnostics().len(), 2);
    assert_eq!(runtime.entity_diagnostics(ogg_id).len(), 1);
    assert_eq!(runtime.entity_diagnostics(bad_wav_id).len(), 1);

    assert_eq!(
        runtime.audio().source_state(ogg_id),
        unitas_audio::AudioSourceState::Failed
    );
    assert_eq!(
        runtime.audio().inaudible_reason(ogg_id),
        Some(unitas_audio::SilenceReason::UnsupportedClip)
    );
    assert_eq!(
        runtime.audio().source_state(bad_wav_id),
        unitas_audio::AudioSourceState::Failed
    );
    assert_eq!(
        runtime.audio().inaudible_reason(bad_wav_id),
        Some(unitas_audio::SilenceReason::UnsupportedClip)
    );

    runtime.tick(0.02)?;
    assert_eq!(
        runtime.audio().source_state(ogg_id),
        unitas_audio::AudioSourceState::Failed
    );
    assert_eq!(
        runtime.audio().source_state(bad_wav_id),
        unitas_audio::AudioSourceState::Failed
    );
    Ok(())
}

#[test]
fn runtime_abi_audio_exports_round_trip() -> Result<(), Box<dyn Error>> {
    let temp_dir =
        std::env::temp_dir().join(format!("unitas_runtime_abi_audio_{}", std::process::id()));
    let assets_dir = temp_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;
    let wav_guid = "44444444444444444444444444444444";
    let wav_path = assets_dir.join("tone.wav");
    let wav_bytes = build_test_wav(1, 1, 10, 16, &[16384, 16384, 16384, 16384]);
    std::fs::write(&wav_path, &wav_bytes)?;
    std::fs::write(
        assets_dir.join("tone.wav.meta"),
        format!("fileFormatVersion: 2\nguid: {wav_guid}\n"),
    )?;
    let scene_path = assets_dir.join("AbiAudioScene.unity");
    let yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: AbiSpeaker\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalRotation: {{x: 0, y: 0, z: 0, w: 1}}\n  m_LocalPosition: {{x: 1, y: 2, z: 3}}\n  m_LocalScale: {{x: 1, y: 1, z: 1}}\n  m_Children: []\n  m_Father: {{fileID: 0}}\n--- !u!82 &102\nAudioSource:\n  m_GameObject: {{fileID: 100}}\n  m_Enabled: 1\n  m_audioClip: {{fileID: 8300000, guid: {wav_guid}, type: 3}}\n  m_PlayOnAwake: 0\n  m_Volume: 0.5\n  m_Pitch: 1.25\n  m_Loop: 1\n  m_Mute: 0\n  m_SpatialBlend: 0.5\n  m_MinDistance: 2.0\n  m_MaxDistance: 20.0\n  m_Priority: 100\n"
    );
    std::fs::write(&scene_path, yaml)?;

    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let buf_addr = unitas_runtime_path_buffer(handle, 512);
    assert_ne!(buf_addr, 0);
    let scene_path_str = scene_path.to_string_lossy();
    let scene_bytes = scene_path_str.as_bytes();
    test_write_path_buffer(handle, scene_bytes)?;
    assert_eq!(
        unitas_runtime_load_scene(handle, buf_addr, scene_bytes.len() as u64),
        0
    );

    let rc_cfg = unitas_runtime::unitas_runtime_audio_source_config(handle, 1, buf_addr);
    assert_eq!(rc_cfg, 0);
    let mut cfg_bytes = [0u8; 44];
    unitas_runtime::test_read_path_buffer(handle, &mut cfg_bytes)?;
    let cfg_vol = f32::from_le_bytes(cfg_bytes[0..4].try_into()?);
    let cfg_pitch = f32::from_le_bytes(cfg_bytes[4..8].try_into()?);
    let cfg_blend = f32::from_le_bytes(cfg_bytes[8..12].try_into()?);
    let cfg_min = f32::from_le_bytes(cfg_bytes[12..16].try_into()?);
    let cfg_max = f32::from_le_bytes(cfg_bytes[16..20].try_into()?);
    let cfg_clip_id = u64::from_le_bytes(cfg_bytes[20..28].try_into()?);
    let cfg_clip_dur = f64::from_le_bytes(cfg_bytes[28..36].try_into()?);
    assert!((cfg_vol - 0.5).abs() < 1e-6);
    assert!((cfg_pitch - 1.25).abs() < 1e-6);
    assert!((cfg_blend - 0.5).abs() < 1e-6);
    assert!((cfg_min - 2.0).abs() < 1e-6);
    assert!((cfg_max - 20.0).abs() < 1e-6);
    assert_ne!(cfg_clip_id, 0);
    assert!((cfg_clip_dur - 0.4).abs() < 1e-6);
    assert_eq!(cfg_bytes[36], 100);
    assert_eq!(cfg_bytes[37], 1);
    assert_eq!(cfg_bytes[38], 0);
    assert_eq!(cfg_bytes[39], 0);
    assert_eq!(cfg_bytes[40], 1);

    let wav_path_str = wav_path.to_string_lossy();
    let wav_path_bytes = wav_path_str.as_bytes();
    test_write_path_buffer(handle, wav_path_bytes)?;
    let loaded_clip = unitas_runtime::unitas_runtime_audio_load_clip(
        handle,
        buf_addr,
        wav_path_bytes.len() as u64,
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert_ne!(loaded_clip, 0);

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_clip_info(handle, loaded_clip, buf_addr),
        0
    );
    let mut info_bytes = [0u8; 16];
    unitas_runtime::test_read_path_buffer(handle, &mut info_bytes)?;
    let info_dur = f64::from_le_bytes(info_bytes[0..8].try_into()?);
    let info_rate = u32::from_le_bytes(info_bytes[8..12].try_into()?);
    let info_ch = u32::from_le_bytes(info_bytes[12..16].try_into()?);
    assert!((info_dur - 0.4).abs() < 1e-6);
    assert_eq!(info_rate, 10);
    assert_eq!(info_ch, 1);

    let dyn_entity = unitas_runtime::unitas_runtime_audio_create(handle, 0);
    assert!(dyn_entity > 1);
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_clip(handle, dyn_entity, loaded_clip),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_volume(handle, dyn_entity, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_pitch(handle, dyn_entity, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_loop(handle, dyn_entity, 0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_priority(handle, dyn_entity, 64),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_spatial_blend(handle, dyn_entity, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_min_max_distance(handle, dyn_entity, 1.0, 10.0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_position(handle, dyn_entity, 0.0, 0.0, 2.0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_listener(handle, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        0
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, dyn_entity),
        5
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_silence_reason(handle, dyn_entity),
        7
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_play(handle, dyn_entity, 0),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, dyn_entity),
        2
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_silence_reason(handle, dyn_entity),
        -1
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_mute(handle, dyn_entity, 1),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_silence_reason(handle, dyn_entity),
        1
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_mute(handle, dyn_entity, 0),
        0
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_pause(handle, dyn_entity),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, dyn_entity),
        3
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_silence_reason(handle, dyn_entity),
        2
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_unpause(handle, dyn_entity),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, dyn_entity),
        2
    );

    let mixed_frames = unitas_runtime::unitas_runtime_audio_mix(handle, 2, buf_addr);
    assert_eq!(mixed_frames, 2);
    let mut mix_bytes = [0u8; 16];
    unitas_runtime::test_read_path_buffer(handle, &mut mix_bytes)?;
    let sample_l0 = f32::from_le_bytes(mix_bytes[0..4].try_into()?);
    let sample_r0 = f32::from_le_bytes(mix_bytes[4..8].try_into()?);
    assert!(sample_l0 > 0.0);
    assert!((sample_l0 - sample_r0).abs() < 1e-6);

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_time(handle, dyn_entity, 0.1),
        0
    );
    assert!((unitas_runtime::unitas_runtime_audio_time(handle, dyn_entity) - 0.1).abs() < 1e-6);

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_clear_listener(handle),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_silence_reason(handle, dyn_entity),
        5
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_stop(handle, dyn_entity),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_state(handle, dyn_entity),
        5
    );

    assert_eq!(
        unitas_runtime::unitas_runtime_audio_set_custom_duration(handle, dyn_entity, 1.5),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_audio_destroy(handle, dyn_entity),
        0
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
