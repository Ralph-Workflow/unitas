use std::cell::RefCell;
use std::error::Error;
use std::path::Path;

use unitas_core::EntityId;
use unitas_runtime::{
    Callbacks, Runtime, test_write_path_buffer, unitas_runtime_bind_destroy_channel,
    unitas_runtime_body_position, unitas_runtime_destroy, unitas_runtime_error_message,
    unitas_runtime_load_scene, unitas_runtime_mark_destroyed, unitas_runtime_new,
    unitas_runtime_object_attach_script, unitas_runtime_object_create,
    unitas_runtime_object_create_body, unitas_runtime_object_set_transform,
    unitas_runtime_path_buffer, unitas_runtime_set_callbacks, unitas_runtime_tick,
};

const TRIGGER_FIXTURE: &str = "tests/fixtures/scene_physics_collision_no_script.unity";
const SCRIPT_FIXTURE: &str = "tests/fixtures/scene_with_script.unity";

thread_local! {
    static OBJ_EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static OBJ_TRIGGERS: RefCell<Vec<(u64, u64, u32)>> = const { RefCell::new(Vec::new()) };
    static REENTRANT_HANDLE: RefCell<u64> = const { RefCell::new(0) };
    static REENTRANT_ENTITY: RefCell<u64> = const { RefCell::new(0) };
    static REENTRANT_TRANSFORM_RC: RefCell<i32> = const { RefCell::new(-999) };
    static NEXT_HANDLE_OFFSET: RefCell<u64> = const { RefCell::new(1000) };
}

fn clear_obj_events() {
    OBJ_EVENTS.with(|cell| cell.borrow_mut().clear());
    OBJ_TRIGGERS.with(|cell| cell.borrow_mut().clear());
    REENTRANT_HANDLE.with(|cell| *cell.borrow_mut() = 0);
    REENTRANT_ENTITY.with(|cell| *cell.borrow_mut() = 0);
    REENTRANT_TRANSFORM_RC.with(|cell| *cell.borrow_mut() = -999);
    NEXT_HANDLE_OFFSET.with(|cell| *cell.borrow_mut() = 1000);
}

fn push_obj_event(event: String) {
    OBJ_EVENTS.with(|cell| cell.borrow_mut().push(event));
}

fn take_obj_events() -> Vec<String> {
    OBJ_EVENTS.with(|cell| cell.borrow().clone())
}

fn take_obj_triggers() -> Vec<(u64, u64, u32)> {
    OBJ_TRIGGERS.with(|cell| cell.borrow().clone())
}

extern "C" fn obj_create_script(entity: u64, guid_hi: u64, guid_lo: u64) -> u64 {
    push_obj_event(format!("create_script:{entity}:{guid_hi:x}:{guid_lo:x}"));
    let runtime_handle = REENTRANT_HANDLE.with(|cell| *cell.borrow());
    let target_entity = REENTRANT_ENTITY.with(|cell| *cell.borrow());
    if runtime_handle != 0 && target_entity == entity {
        let rc = unitas_runtime_object_set_transform(
            runtime_handle,
            entity,
            4.0,
            5.0,
            6.0,
            0.0,
            0.0,
            0.0,
            1.0,
            1.0,
            1.0,
            1.0,
        );
        REENTRANT_TRANSFORM_RC.with(|cell| *cell.borrow_mut() = rc);
    }
    let offset = NEXT_HANDLE_OFFSET.with(|cell| {
        let mut current = cell.borrow_mut();
        let val = *current;
        *current += 1000;
        val
    });
    entity.saturating_add(offset)
}

extern "C" fn obj_awake(handle: u64) {
    push_obj_event(format!("awake:{handle}"));
}

extern "C" fn obj_on_enable(handle: u64) {
    push_obj_event(format!("on_enable:{handle}"));
}

extern "C" fn obj_start(handle: u64) {
    push_obj_event(format!("start:{handle}"));
}

extern "C" fn obj_fixed_update(handle: u64, _dt: f64) {
    push_obj_event(format!("fixed_update:{handle}"));
}

extern "C" fn obj_update(handle: u64, _dt: f64) {
    push_obj_event(format!("update:{handle}"));
}

extern "C" fn obj_late_update(handle: u64, _dt: f64) {
    push_obj_event(format!("late_update:{handle}"));
}

extern "C" fn obj_on_disable(handle: u64) {
    push_obj_event(format!("on_disable:{handle}"));
}

extern "C" fn obj_destroy(handle: u64) {
    push_obj_event(format!("destroy:{handle}"));
}

extern "C" fn obj_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn obj_trigger(a: u64, b: u64, phase: u32) {
    OBJ_TRIGGERS.with(|cell| cell.borrow_mut().push((a, b, phase)));
}

fn obj_callbacks() -> Callbacks {
    Callbacks {
        create_script: obj_create_script,
        awake: obj_awake,
        on_enable: obj_on_enable,
        start: obj_start,
        fixed_update: obj_fixed_update,
        update: obj_update,
        late_update: obj_late_update,
        on_disable: obj_on_disable,
        destroy: obj_destroy,
        collision: obj_collision,
        trigger: obj_trigger,
    }
}

fn write_text_to_path_buffer(handle: u64, text: &str) -> Result<(u64, u64), Box<dyn Error>> {
    let bytes = text.as_bytes();
    let capacity = bytes.len().max(64) as u64;
    let addr = unitas_runtime_path_buffer(handle, capacity);
    if addr == 0 {
        return Err(String::from("failed to allocate path buffer").into());
    }
    test_write_path_buffer(handle, bytes)?;
    Ok((addr, bytes.len() as u64))
}

#[test]
fn runtime_object_creates_entity_in_active_scene_graph() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCRIPT_FIXTURE))?;

    let entity = runtime.create_runtime_object("SpawnedPickup")?;
    assert_ne!(entity.as_u64(), 0);

    let graph = runtime.scene_graph().ok_or("missing active scene graph")?;
    assert!(graph.contains(entity));
    let node = graph.entity(entity).ok_or("entity missing from graph")?;
    assert_eq!(node.name, "SpawnedPickup");
    assert_eq!(runtime.find_entity_by_name("SpawnedPickup"), Some(entity));
    Ok(())
}

#[test]
fn runtime_object_attaches_script_and_runs_lifecycle_in_order() -> Result<(), Box<dyn Error>> {
    clear_obj_events();
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE))?;
    runtime.set_callbacks(obj_callbacks());

    runtime.tick(0.02)?;
    clear_obj_events();

    let entity = runtime.create_runtime_object("RuntimeScripted")?;
    let entity_raw = entity.as_u64();
    let expected_handle = entity_raw + 1000;
    runtime.attach_runtime_script(entity, 0x1234, 0x5678)?;

    assert_eq!(
        take_obj_events(),
        vec![format!("create_script:{entity_raw}:1234:5678")]
    );

    runtime.tick(0.02)?;
    assert_eq!(
        take_obj_events(),
        vec![
            format!("create_script:{entity_raw}:1234:5678"),
            format!("awake:{expected_handle}"),
            format!("on_enable:{expected_handle}"),
            format!("start:{expected_handle}"),
            format!("fixed_update:{expected_handle}"),
            format!("update:{expected_handle}"),
            format!("late_update:{expected_handle}"),
        ]
    );

    clear_obj_events();
    runtime.tick(0.02)?;
    assert_eq!(
        take_obj_events(),
        vec![
            format!("fixed_update:{expected_handle}"),
            format!("update:{expected_handle}"),
            format!("late_update:{expected_handle}"),
        ]
    );
    Ok(())
}

#[test]
fn runtime_object_creates_trigger_body_and_fires_trigger_event_with_new_entity_id()
-> Result<(), Box<dyn Error>> {
    clear_obj_events();
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE))?;
    runtime.set_callbacks(obj_callbacks());
    let ball = runtime.find_entity_by_name("Ball").ok_or("missing Ball")?;

    runtime.tick(0.02)?;
    clear_obj_events();

    let pickup = runtime.create_runtime_object("RuntimePickup")?;
    runtime.set_runtime_object_transform(
        pickup,
        [0.0, 0.2, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;
    runtime.create_runtime_body(pickup, 0, 0.45, [0.0, 0.0, 0.0], 0.0, true)?;

    runtime.tick(0.02)?;
    let triggers = take_obj_triggers();
    assert!(
        triggers.iter().any(|&(a, b, phase)| {
            phase == 0
                && ((a == ball.as_u64() && b == pickup.as_u64())
                    || (a == pickup.as_u64() && b == ball.as_u64()))
        }),
        "expected trigger enter between Ball ({}) and RuntimePickup ({}), got {triggers:?}",
        ball.as_u64(),
        pickup.as_u64()
    );
    Ok(())
}

#[test]
fn runtime_object_destroy_removes_entity_from_graph_scripts_and_physics_world()
-> Result<(), Box<dyn Error>> {
    clear_obj_events();
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE))?;
    runtime.set_callbacks(obj_callbacks());
    runtime.tick(0.02)?;

    let pickup = runtime.create_runtime_object("DisposablePickup")?;
    let expected_handle = pickup.as_u64() + 1000;
    runtime.set_runtime_object_transform(
        pickup,
        [3.0, 1.0, -2.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;
    runtime.attach_runtime_script(pickup, 0xabcd, 0xef01)?;
    runtime.create_runtime_body(pickup, 0, 0.5, [0.0, 0.0, 0.0], 0.0, true)?;
    runtime.tick(0.02)?;

    assert!(
        runtime
            .scene_graph()
            .is_some_and(|graph| graph.contains(pickup))
    );
    assert_eq!(runtime.body_position(pickup), Some([3.0, 1.0, -2.0]));
    assert!(runtime.scripts().iter().any(|s| s.entity == pickup));

    clear_obj_events();
    runtime.mark_destroyed(pickup)?;
    runtime.tick(0.02)?;

    assert!(
        runtime
            .scene_graph()
            .is_some_and(|graph| !graph.contains(pickup))
    );
    assert_eq!(runtime.body_position(pickup), None);
    assert!(!runtime.scripts().iter().any(|s| s.entity == pickup));
    let events = take_obj_events();
    assert!(
        !events
            .iter()
            .any(|e| e == &format!("update:{expected_handle}")),
        "destroyed script must not receive update: {events:?}"
    );
    Ok(())
}

#[test]
fn runtime_object_abi_exports_round_trip_and_validate_arguments() -> Result<(), Box<dyn Error>> {
    clear_obj_events();
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    assert_eq!(unitas_runtime_bind_destroy_channel(handle), 0);

    let (scene_addr, scene_len) = write_text_to_path_buffer(handle, TRIGGER_FIXTURE)?;
    assert_eq!(unitas_runtime_load_scene(handle, scene_addr, scene_len), 0);
    assert_eq!(
        unitas_runtime_set_callbacks(
            handle,
            obj_create_script,
            obj_awake,
            obj_on_enable,
            obj_start,
            obj_fixed_update,
            obj_update,
            obj_late_update,
            obj_on_disable,
            obj_destroy,
            obj_collision,
            obj_trigger,
        ),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    clear_obj_events();

    let (name_addr, name_len) = write_text_to_path_buffer(handle, "AbiSpawnedPickup")?;
    let entity = unitas_runtime_object_create(handle, name_addr, name_len);
    assert_ne!(entity, 0);

    REENTRANT_HANDLE.with(|cell| *cell.borrow_mut() = handle);
    REENTRANT_ENTITY.with(|cell| *cell.borrow_mut() = entity);
    assert_eq!(
        unitas_runtime_object_attach_script(handle, entity, 0x1111, 0x2222),
        0
    );
    assert_eq!(REENTRANT_TRANSFORM_RC.with(|cell| *cell.borrow()), 0);
    assert_eq!(
        take_obj_events(),
        vec![format!("create_script:{entity}:1111:2222")]
    );

    assert_eq!(
        unitas_runtime_object_create_body(handle, entity, 0, 0.45, 0.0, 0.0, 0.0, 0.0, 1),
        0
    );
    assert_eq!(
        unitas_runtime_object_set_transform(
            handle, entity, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
        ),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    let expected_handle = entity + 1000;
    assert_eq!(
        take_obj_events(),
        vec![
            format!("create_script:{entity}:1111:2222"),
            format!("awake:{expected_handle}"),
            format!("on_enable:{expected_handle}"),
            format!("start:{expected_handle}"),
            format!("fixed_update:{expected_handle}"),
            format!("update:{expected_handle}"),
            format!("late_update:{expected_handle}"),
        ]
    );
    let triggers = take_obj_triggers();
    assert!(
        triggers
            .iter()
            .any(|&(a, b, phase)| phase == 0 && (a == entity || b == entity)),
        "expected ABI trigger event for entity {entity}, got {triggers:?}"
    );

    assert_ne!(
        unitas_runtime_object_create_body(handle, entity, 99, 0.5, 0.5, 0.5, 0.5, 1.0, 0),
        0
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_mark_destroyed(handle, entity), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);

    let out_buf = unitas_runtime_path_buffer(handle, 64);
    assert_ne!(unitas_runtime_body_position(handle, entity, out_buf), 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn runtime_object_supports_box_and_capsule_shapes_and_transform_sync() -> Result<(), Box<dyn Error>>
{
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE))?;

    let box_entity = runtime.create_runtime_object("BoxObj")?;
    runtime.set_runtime_object_transform(
        box_entity,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;
    runtime.create_runtime_body(box_entity, 1, 0.0, [0.5, 0.5, 0.5], 0.0, false)?;
    assert_eq!(runtime.body_position(box_entity), Some([1.0, 2.0, 3.0]));

    runtime.set_runtime_object_transform(
        box_entity,
        [4.0, 5.0, 6.0],
        [0.0, 0.0, 0.0, 1.0],
        [2.0, 2.0, 2.0],
    )?;
    assert_eq!(runtime.body_position(box_entity), Some([4.0, 5.0, 6.0]));
    let world_tf = runtime
        .world_transform_of(box_entity)
        .ok_or("missing transform")?;
    assert_eq!(world_tf.translation, [4.0, 5.0, 6.0]);
    assert_eq!(world_tf.scale, [2.0, 2.0, 2.0]);

    let cap_entity = runtime.create_runtime_object("CapsuleObj")?;
    runtime.create_runtime_body(cap_entity, 2, 0.5, [0.0, 0.0, 0.0], 2.0, true)?;
    assert_eq!(runtime.body_position(cap_entity), Some([0.0, 0.0, 0.0]));

    assert!(
        runtime
            .create_runtime_body(cap_entity, 7, 0.5, [0.0, 0.0, 0.0], 2.0, false)
            .is_err()
    );
    assert!(
        runtime
            .attach_runtime_script(EntityId::new(999_999), 1, 2)
            .is_err()
    );

    clear_obj_events();
    runtime.set_callbacks(obj_callbacks());
    runtime.tick(0.02)?;
    clear_obj_events();
    let multi = runtime.create_runtime_object("MultiScriptObj")?;
    runtime.attach_runtime_script(multi, 0x10, 0x20)?;
    runtime.attach_runtime_script(multi, 0x30, 0x40)?;
    runtime.tick(0.02)?;
    let h1 = multi.as_u64() + 1000;
    let h2 = multi.as_u64() + 2000;
    let events = take_obj_events();
    assert!(events.contains(&format!("awake:{h1}")));
    assert!(events.contains(&format!("awake:{h2}")));
    assert!(events.contains(&format!("start:{h1}")));
    assert!(events.contains(&format!("start:{h2}")));
    assert!(events.contains(&format!("update:{h1}")));
    assert!(events.contains(&format!("update:{h2}")));
    Ok(())
}
