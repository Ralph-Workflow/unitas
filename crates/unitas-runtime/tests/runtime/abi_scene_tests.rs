use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use unitas_runtime::{
    AbiCharacterMoveInput, AbiCharacterMoveOutput, CharacterMoveRequest, Runtime,
    test_is_persistent_entity, test_read_path_buffer, test_write_path_buffer,
    unitas_runtime_active_scene_index, unitas_runtime_add_body_force, unitas_runtime_body_position,
    unitas_runtime_body_velocity, unitas_runtime_character_controller_config,
    unitas_runtime_character_move, unitas_runtime_character_teleport, unitas_runtime_company_name,
    unitas_runtime_destroy, unitas_runtime_dont_destroy_on_load, unitas_runtime_error_message,
    unitas_runtime_load_scene_mode, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_product_name, unitas_runtime_project_version, unitas_runtime_scene_count,
    unitas_runtime_scene_name, unitas_runtime_set_active_scene, unitas_runtime_set_body_position,
    unitas_runtime_set_body_velocity, unitas_runtime_set_callbacks, unitas_runtime_set_time_scale,
    unitas_runtime_tick, unitas_runtime_time_scale, unitas_runtime_unload_scene,
};

const SCENE_A: &str = "tests/fixtures/scene_with_script.unity";
const SCENE_B: &str = "tests/fixtures/scene_physics_collision_no_script.unity";
const SCENE_A_NAME: &str = "scene_with_script";
const SCENE_B_NAME: &str = "scene_physics_collision_no_script";
const MISSING_SCENE: &str = "tests/fixtures/nonexistent_scene.unity";

fn new_handle() -> Result<u64, Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    Ok(handle)
}

fn write_path(handle: u64, text: &str) -> Result<(u64, u64), Box<dyn Error>> {
    let bytes = text.as_bytes();
    let address = unitas_runtime_path_buffer(handle, 256);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, bytes)?;
    Ok((address, u64::try_from(bytes.len())?))
}

fn load_scene_mode(handle: u64, path: &str, mode: u32) -> Result<i32, Box<dyn Error>> {
    let (address, length) = write_path(handle, path)?;
    Ok(unitas_runtime_load_scene_mode(
        handle, address, length, mode,
    ))
}

fn entity_id_for(path: &str, name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(path))?;
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
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

#[test]
fn abi_load_scene_mode_single_replaces_the_scene_set() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert_eq!(unitas_runtime_scene_count(handle), 1);
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    assert_eq!(unitas_runtime_scene_count(handle), 1);
    assert_eq!(
        unitas_runtime_scene_name(handle, 0),
        SCENE_B_NAME.len() as i32
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_load_scene_mode_additive_keeps_the_previous_scene() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert_eq!(load_scene_mode(handle, SCENE_B, 1)?, 0);
    assert_eq!(unitas_runtime_scene_count(handle), 2);
    assert_eq!(
        unitas_runtime_scene_name(handle, 0),
        SCENE_A_NAME.len() as i32
    );
    assert_eq!(
        unitas_runtime_scene_name(handle, 1),
        SCENE_B_NAME.len() as i32
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_load_scene_mode_rejects_an_unknown_mode() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 7)?, -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_scene_count(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_load_scene_mode_rejects_a_foreign_buffer_address() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let length = u64::try_from(SCENE_A.len())?;
    assert_eq!(unitas_runtime_load_scene_mode(handle, 0, length, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_unload_scene_rejects_an_unknown_name_and_keeps_the_scene_set() -> Result<(), Box<dyn Error>>
{
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    let (address, length) = write_path(handle, "no_such_scene")?;
    assert!(unitas_runtime_unload_scene(handle, address, length) < 0);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_scene_count(handle), 1);
    assert_eq!(
        unitas_runtime_scene_name(handle, 0),
        SCENE_A_NAME.len() as i32
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_unload_scene_removes_the_loaded_scene() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert_eq!(load_scene_mode(handle, SCENE_B, 1)?, 0);
    assert_eq!(unitas_runtime_scene_count(handle), 2);
    let (address, length) = write_path(handle, SCENE_B_NAME)?;
    assert_eq!(unitas_runtime_unload_scene(handle, address, length), 0);
    assert_eq!(unitas_runtime_scene_count(handle), 1);
    assert_eq!(
        unitas_runtime_scene_name(handle, 0),
        SCENE_A_NAME.len() as i32
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_unload_scene_accepts_the_scene_path() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    let (address, length) = write_path(handle, SCENE_A)?;
    assert_eq!(unitas_runtime_unload_scene(handle, address, length), 0);
    assert_eq!(unitas_runtime_scene_count(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_scene_name_writes_a_nul_terminated_name_into_the_path_buffer() -> Result<(), Box<dyn Error>>
{
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    let length = unitas_runtime_scene_name(handle, 0);
    assert_eq!(length, SCENE_A_NAME.len() as i32);
    let mut bytes = vec![0u8; usize::try_from(length)? + 1];
    test_read_path_buffer(handle, &mut bytes)?;
    assert_eq!(&bytes[..SCENE_A_NAME.len()], SCENE_A_NAME.as_bytes());
    assert_eq!(bytes[SCENE_A_NAME.len()], 0);
    assert_ne!(unitas_runtime_path_buffer(handle, 256), 0);
    test_read_path_buffer(handle, &mut bytes)?;
    assert_eq!(&bytes[..SCENE_A_NAME.len()], SCENE_A_NAME.as_bytes());
    assert_eq!(bytes[SCENE_A_NAME.len()], 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_scene_name_rejects_an_out_of_range_index() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert!(unitas_runtime_scene_name(handle, 7) < 0);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_set_active_scene_selects_a_scene_and_rejects_a_bad_index() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert_eq!(load_scene_mode(handle, SCENE_B, 1)?, 0);
    assert_eq!(unitas_runtime_set_active_scene(handle, 1), 0);
    assert_eq!(unitas_runtime_active_scene_index(handle), 1);
    assert!(unitas_runtime_set_active_scene(handle, 9) < 0);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_active_scene_index(handle), 1);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_active_scene_index_is_minus_one_without_a_loaded_scene() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(unitas_runtime_active_scene_index(handle), -1);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_dont_destroy_on_load_keeps_the_entity_persistent_across_a_single_load()
-> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    let entity = entity_id_for(SCENE_A, "ScriptedObject")?;
    assert!(!test_is_persistent_entity(handle, entity)?);
    assert_eq!(unitas_runtime_dont_destroy_on_load(handle, entity), 0);
    assert!(test_is_persistent_entity(handle, entity)?);
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    assert!(test_is_persistent_entity(handle, entity)?);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_failed_load_reports_an_error_and_keeps_the_scene_set() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert!(load_scene_mode(handle, MISSING_SCENE, 0)? < 0);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_scene_count(handle), 1);
    assert_eq!(
        unitas_runtime_scene_name(handle, 0),
        SCENE_A_NAME.len() as i32
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

static CALLBACK_HANDLE: AtomicU64 = AtomicU64::new(0);
static NESTED_CALLS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn nested_create(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn nested_noop(_handle: u64) {}

extern "C" fn nested_noop_f64(_handle: u64, _delta: f64) {}

extern "C" fn nested_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn nested_on_disable(_handle: u64) {
    let handle = CALLBACK_HANDLE.load(Ordering::SeqCst);
    if handle == 0 {
        return;
    }
    unitas_runtime_set_time_scale(handle, 0.5);
    NESTED_CALLS.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn abi_scene_teardown_callbacks_can_call_back_into_the_runtime() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    CALLBACK_HANDLE.store(handle, Ordering::SeqCst);
    NESTED_CALLS.store(0, Ordering::SeqCst);
    unitas_runtime_set_callbacks(
        handle,
        nested_create,
        nested_noop,
        nested_noop,
        nested_noop,
        nested_noop_f64,
        nested_noop_f64,
        nested_noop_f64,
        nested_on_disable,
        nested_noop,
        nested_collision,
        nested_collision,
    );

    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    assert_eq!(NESTED_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(unitas_runtime_time_scale(handle), 0.5);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

const SETTINGS_SCENE: &str =
    "tests/fixtures/project_with_settings/Assets/Scenes/SettingsScene.unity";

fn read_nul_terminated_from_path_buffer(
    handle: u64,
    length: i32,
) -> Result<String, Box<dyn Error>> {
    let byte_len = usize::try_from(length)?;
    let mut bytes = vec![0u8; byte_len + 1];
    test_read_path_buffer(handle, &mut bytes)?;
    if bytes[byte_len] != 0 {
        return Err(String::from("missing NUL terminator in path buffer").into());
    }
    Ok(std::str::from_utf8(&bytes[..byte_len])?.to_string())
}

#[test]
fn abi_project_identity_externs_write_loaded_identity_into_path_buffer()
-> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert!(unitas_runtime_company_name(handle) < 0);
    assert!(unitas_runtime_product_name(handle) < 0);
    assert!(unitas_runtime_project_version(handle) < 0);

    assert_eq!(load_scene_mode(handle, SETTINGS_SCENE, 0)?, 0);

    let company_len = unitas_runtime_company_name(handle);
    assert_eq!(company_len, 6);
    assert_eq!(
        read_nul_terminated_from_path_buffer(handle, company_len)?,
        "Unitas"
    );

    let product_len = unitas_runtime_product_name(handle);
    assert_eq!(product_len, 15);
    assert_eq!(
        read_nul_terminated_from_path_buffer(handle, product_len)?,
        "SettingsFixture"
    );

    let version_len = unitas_runtime_project_version(handle);
    assert_eq!(version_len, 5);
    assert_eq!(
        read_nul_terminated_from_path_buffer(handle, version_len)?,
        "1.2.3"
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_project_identity_externs_return_negative_when_settings_missing_or_handle_invalid()
-> Result<(), Box<dyn Error>> {
    assert!(unitas_runtime_company_name(u64::MAX) < 0);
    assert!(unitas_runtime_product_name(u64::MAX) < 0);
    assert!(unitas_runtime_project_version(u64::MAX) < 0);

    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SETTINGS_SCENE, 0)?, 0);
    assert_eq!(unitas_runtime_company_name(handle), 6);

    assert_eq!(load_scene_mode(handle, SCENE_A, 0)?, 0);
    assert!(unitas_runtime_company_name(handle) < 0);
    assert!(unitas_runtime_product_name(handle) < 0);
    assert!(unitas_runtime_project_version(handle) < 0);

    let mut runtime = Runtime::new()?;
    assert_eq!(runtime.company_name(), None);
    assert_eq!(runtime.product_name(), None);
    assert_eq!(runtime.project_version(), None);
    runtime.load_scene(Path::new(SETTINGS_SCENE))?;
    assert_eq!(runtime.company_name(), Some("Unitas"));
    assert_eq!(runtime.product_name(), Some("SettingsFixture"));
    assert_eq!(runtime.project_version(), Some("1.2.3"));
    assert_eq!(
        runtime.project_identity().company_name.as_deref(),
        Some("Unitas")
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

fn read_vec3_from_path_buffer(handle: u64) -> Result<[f64; 3], Box<dyn Error>> {
    let mut bytes = [0u8; 24];
    test_read_path_buffer(handle, &mut bytes)?;
    let x = f64::from_le_bytes(bytes[0..8].try_into()?);
    let y = f64::from_le_bytes(bytes[8..16].try_into()?);
    let z = f64::from_le_bytes(bytes[16..24].try_into()?);
    Ok([x, y, z])
}

fn write_vec3_to_path_buffer(handle: u64, value: [f64; 3]) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, 24);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    let mut bytes = [0u8; 24];
    bytes[0..8].copy_from_slice(&value[0].to_le_bytes());
    bytes[8..16].copy_from_slice(&value[1].to_le_bytes());
    bytes[16..24].copy_from_slice(&value[2].to_le_bytes());
    test_write_path_buffer(handle, &bytes)?;
    Ok(address)
}

fn write_force_to_path_buffer(
    handle: u64,
    force: [f64; 3],
    mode: f64,
) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, 32);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&force[0].to_le_bytes());
    bytes[8..16].copy_from_slice(&force[1].to_le_bytes());
    bytes[16..24].copy_from_slice(&force[2].to_le_bytes());
    bytes[24..32].copy_from_slice(&mode.to_le_bytes());
    test_write_path_buffer(handle, &bytes)?;
    Ok(address)
}

#[test]
fn abi_body_position_and_velocity_report_live_state_and_reject_invalid_inputs()
-> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    let ball = entity_id_for(SCENE_B, "Ball")?;
    let crate_entity = entity_id_for(SCENE_B, "Crate")?;
    let out_buffer = unitas_runtime_path_buffer(handle, 24);
    assert_ne!(out_buffer, 0);

    assert_eq!(unitas_runtime_body_position(handle, ball, out_buffer), 0);
    let initial_pos = read_vec3_from_path_buffer(handle)?;
    assert!((initial_pos[0] - 0.0).abs() < 1e-6);
    assert!((initial_pos[1] - 0.2).abs() < 1e-6);
    assert!((initial_pos[2] - 0.0).abs() < 1e-6);

    assert_eq!(unitas_runtime_body_velocity(handle, ball, out_buffer), 0);
    let initial_vel = read_vec3_from_path_buffer(handle)?;
    assert_eq!(initial_vel, [0.0, 0.0, 0.0]);

    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    for _ in 0..100 {
        assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    }

    assert_eq!(unitas_runtime_body_position(handle, ball, out_buffer), 0);
    let resting_pos = read_vec3_from_path_buffer(handle)?;
    assert!(
        (resting_pos[1] - 1.0).abs() < 5e-3,
        "resting_pos = {resting_pos:?}"
    );

    assert_eq!(unitas_runtime_body_velocity(handle, ball, out_buffer), 0);
    let resting_vel = read_vec3_from_path_buffer(handle)?;
    assert!(resting_vel[1].abs() < 1e-3, "resting_vel = {resting_vel:?}");

    assert_eq!(
        unitas_runtime_body_position(handle, crate_entity, out_buffer),
        0
    );
    let crate_pos = read_vec3_from_path_buffer(handle)?;
    assert_eq!(crate_pos, [0.0, 0.0, 0.0]);

    assert_eq!(unitas_runtime_body_position(u64::MAX, ball, out_buffer), -1);
    assert_eq!(unitas_runtime_body_velocity(u64::MAX, ball, out_buffer), -1);

    assert_eq!(unitas_runtime_body_position(handle, ball, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_body_velocity(handle, ball, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(
        unitas_runtime_body_position(handle, 999_999, out_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_body_velocity(handle, 999_999, out_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_set_body_velocity_and_position_update_simulation_and_reject_invalid_writes()
-> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    let ball = entity_id_for(SCENE_B, "Ball")?;
    let crate_entity = entity_id_for(SCENE_B, "Crate")?;

    let vel_buffer = write_vec3_to_path_buffer(handle, [4.0, 8.0, -2.0])?;
    assert_eq!(
        unitas_runtime_set_body_velocity(handle, ball, vel_buffer),
        0
    );
    assert_eq!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_body_velocity(handle, ball, vel_buffer), 0);
    assert_eq!(read_vec3_from_path_buffer(handle)?, [4.0, 8.0, -2.0]);

    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.02), 0);

    assert_eq!(unitas_runtime_body_position(handle, ball, vel_buffer), 0);
    let moved_ball_pos = read_vec3_from_path_buffer(handle)?;
    assert!((moved_ball_pos[0] - 0.08).abs() < 1e-6);
    assert!(moved_ball_pos[1] > 0.3);
    assert!((moved_ball_pos[2] - (-0.04)).abs() < 1e-6);

    let pos_buffer = write_vec3_to_path_buffer(handle, [10.0, 20.0, 30.0])?;
    assert_eq!(
        unitas_runtime_set_body_position(handle, ball, pos_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(
        unitas_runtime_set_body_position(handle, crate_entity, pos_buffer),
        0
    );
    assert_eq!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_body_position(handle, crate_entity, pos_buffer),
        0
    );
    assert_eq!(read_vec3_from_path_buffer(handle)?, [10.0, 20.0, 30.0]);

    assert_eq!(
        unitas_runtime_set_body_position(u64::MAX, crate_entity, pos_buffer),
        -1
    );
    assert_eq!(
        unitas_runtime_set_body_velocity(u64::MAX, ball, pos_buffer),
        -1
    );
    assert_eq!(
        unitas_runtime_set_body_position(handle, crate_entity, 0),
        -2
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_set_body_velocity(handle, ball, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_set_body_position(handle, 999_999, pos_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_set_body_velocity(handle, 999_999, pos_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_add_body_force_applies_unity_force_mode_semantics() -> Result<(), Box<dyn Error>> {
    // The ball mass is 1 and the fixed step is 0.02, so Force and Acceleration
    // gain 2.0 * 0.02 of velocity per step while Impulse and VelocityChange
    // gain 2.0 immediately; Acceleration matches Force numerically only
    // because the mass is 1.
    let cases: [(f64, f64, f64); 4] = [
        (0.0, 0.0, 0.04),
        (1.0, 2.0, 2.0),
        (2.0, 0.0, 0.04),
        (3.0, 2.0, 2.0),
    ];
    for (mode, immediate_dvx, stepped_dvx) in cases {
        let handle = new_handle()?;
        assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
        let ball = entity_id_for(SCENE_B, "Ball")?;

        let lift_buffer = write_vec3_to_path_buffer(handle, [0.0, 8.0, 0.0])?;
        assert_eq!(
            unitas_runtime_set_body_velocity(handle, ball, lift_buffer),
            0
        );

        let force_buffer = write_force_to_path_buffer(handle, [2.0, 0.0, 0.0], mode)?;
        assert_eq!(unitas_runtime_add_body_force(handle, ball, force_buffer), 0);
        assert_eq!(unitas_runtime_error_message(handle), 0);

        assert_eq!(unitas_runtime_body_velocity(handle, ball, force_buffer), 0);
        let velocity = read_vec3_from_path_buffer(handle)?;
        assert!(
            (velocity[0] - immediate_dvx).abs() < 1e-9,
            "mode {mode}: the immediate velocity gain must be {immediate_dvx}, got {velocity:?}"
        );
        assert!((velocity[1] - 8.0).abs() < 1e-9);

        // The first tick only initializes scripts; the second runs one step.
        assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
        assert_eq!(unitas_runtime_tick(handle, 0.02), 0);

        assert_eq!(unitas_runtime_body_velocity(handle, ball, force_buffer), 0);
        let velocity = read_vec3_from_path_buffer(handle)?;
        assert!(
            (velocity[0] - stepped_dvx).abs() < 1e-9,
            "mode {mode}: the stepped velocity gain must be {stepped_dvx}, got {velocity:?}"
        );
        assert!(
            velocity[1] < 8.0,
            "mode {mode}: gravity must decelerate the lifted ball, got {velocity:?}"
        );

        assert_eq!(unitas_runtime_destroy(handle), 0);
    }
    Ok(())
}

#[test]
fn abi_add_body_force_rejects_invalid_buffers_handles_and_modes() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    assert_eq!(load_scene_mode(handle, SCENE_B, 0)?, 0);
    let ball = entity_id_for(SCENE_B, "Ball")?;
    let force_buffer = write_force_to_path_buffer(handle, [1.0, 0.0, 0.0], 0.0)?;

    assert_eq!(
        unitas_runtime_add_body_force(u64::MAX, ball, force_buffer),
        -1
    );
    assert_eq!(unitas_runtime_add_body_force(handle, ball, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_add_body_force(handle, 999_999, force_buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    let bad_mode_buffer = write_force_to_path_buffer(handle, [1.0, 0.0, 0.0], 9.0)?;
    assert_eq!(
        unitas_runtime_add_body_force(handle, ball, bad_mode_buffer),
        -2
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

fn default_abi_character_input(
    entity_id: u64,
    position: [f64; 3],
    motion: [f64; 3],
) -> AbiCharacterMoveInput {
    AbiCharacterMoveInput {
        entity_id,
        position,
        motion,
        radius: 0.5,
        height: 2.0,
        center: [0.0, 0.0, 0.0],
        slope_limit_deg: 45.0,
        step_offset: 0.3,
        skin_width: 0.08,
        min_move_distance: 0.001,
        delta_seconds: 0.02,
        layer_mask: u32::MAX,
        enabled: 1,
        reserved: [0; 3],
    }
}

fn write_character_scene(temp_dir: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    std::fs::create_dir_all(temp_dir)?;
    let path = temp_dir.join("character_abi_scene.unity");
    let yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Floor\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: -0.5, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!65 &102\nBoxCollider:\n  m_GameObject: {fileID: 100}\n  m_IsTrigger: 0\n  m_Size: {x: 20, y: 1, z: 20}\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!1 &200\nGameObject:\n  m_Name: Wall\n  m_Layer: 0\n--- !u!4 &201\nTransform:\n  m_GameObject: {fileID: 200}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 2, y: 1, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!65 &202\nBoxCollider:\n  m_GameObject: {fileID: 200}\n  m_IsTrigger: 0\n  m_Size: {x: 1, y: 4, z: 4}\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!1 &300\nGameObject:\n  m_Name: Player\n  m_Layer: 2\n--- !u!4 &301\nTransform:\n  m_GameObject: {fileID: 300}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 1.5, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!65 &302\nBoxCollider:\n  m_GameObject: {fileID: 300}\n  m_IsTrigger: 0\n  m_Size: {x: 1, y: 2, z: 1}\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!143 &303\nCharacterController:\n  m_GameObject: {fileID: 300}\n  m_Enabled: 1\n  m_Radius: 0.4\n  m_Height: 1.8\n  m_Center: {x: 0, y: 0.1, z: 0}\n  m_SlopeLimit: 50\n  m_StepOffset: 0.25\n  m_SkinWidth: 0.05\n  m_MinMoveDistance: 0.002\n";
    std::fs::write(&path, yaml)?;
    Ok(path)
}

#[test]
fn abi_character_move_resolves_ground_and_wall_collisions_and_excludes_self_collider()
-> Result<(), Box<dyn Error>> {
    let temp_dir =
        std::env::temp_dir().join(format!("unitas_abi_character_move_{}", std::process::id()));
    let scene_path = write_character_scene(&temp_dir)?;
    let scene_str = match scene_path.to_str() {
        Some(s) => s.to_string(),
        None => return Err(String::from("non-utf8 temp path").into()),
    };

    let handle = new_handle()?;
    let load_code = load_scene_mode(handle, &scene_str, 0)?;
    let floor_id = entity_id_for(&scene_str, "Floor")?;
    let player_id = entity_id_for(&scene_str, "Player")?;
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert_eq!(load_code, 0);

    let mut cfg_bytes = [0u8; 64];
    let cfg_buf = unitas_runtime_path_buffer(handle, 64);
    assert_ne!(cfg_buf, 0);
    assert_eq!(
        unitas_runtime_character_controller_config(handle, player_id, cfg_buf),
        0
    );
    test_read_path_buffer(handle, &mut cfg_bytes)?;
    let cfg_radius = f64::from_le_bytes(cfg_bytes[0..8].try_into()?);
    let cfg_height = f64::from_le_bytes(cfg_bytes[8..16].try_into()?);
    let cfg_cy = f64::from_le_bytes(cfg_bytes[24..32].try_into()?);
    let cfg_slope = f64::from_le_bytes(cfg_bytes[40..48].try_into()?);
    let cfg_step = f64::from_le_bytes(cfg_bytes[48..56].try_into()?);
    let cfg_skin = f64::from_le_bytes(cfg_bytes[56..64].try_into()?);
    assert!((cfg_radius - 0.4).abs() < 1e-6);
    assert!((cfg_height - 1.8).abs() < 1e-6);
    assert!((cfg_cy - 0.1).abs() < 1e-6);
    assert!((cfg_slope - 50.0).abs() < 1e-6);
    assert!((cfg_step - 0.25).abs() < 1e-6);
    assert!((cfg_skin - 0.05).abs() < 1e-6);

    let down_input = default_abi_character_input(player_id, [0.0, 1.5, 0.0], [0.0, -1.0, 0.0]);
    let mut down_output = AbiCharacterMoveOutput::default();
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&down_input), Some(&mut down_output)),
        0
    );
    assert_eq!(down_output.is_grounded, 1);
    assert_ne!(down_output.collision_flags & 4, 0);
    assert_eq!(down_output.ground_entity_id, floor_id);
    assert!((down_output.position[1] - 1.08).abs() < 1e-4);

    let wall_input = default_abi_character_input(player_id, down_output.position, [3.0, -0.1, 0.0]);
    let mut wall_output = AbiCharacterMoveOutput::default();
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&wall_input), Some(&mut wall_output)),
        0
    );
    assert_eq!(wall_output.is_grounded, 1);
    assert_ne!(wall_output.collision_flags & 1, 0);
    assert_ne!(wall_output.collision_flags & 4, 0);
    assert!((wall_output.position[0] - 0.92).abs() < 1e-4);

    let teleport_pos = write_vec3_to_path_buffer(handle, [0.0, 5.0, 0.0])?;
    assert_eq!(
        unitas_runtime_character_teleport(handle, player_id, teleport_pos),
        0
    );
    let hover_input = default_abi_character_input(player_id, [0.0, 5.0, 0.0], [0.5, 0.0, 0.0]);
    let mut hover_output = AbiCharacterMoveOutput::default();
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&hover_input), Some(&mut hover_output)),
        0
    );
    assert_eq!(hover_output.is_grounded, 0);
    assert_eq!(hover_output.collision_flags, 0);
    assert_eq!(hover_output.ground_entity_id, 0);
    assert!((hover_output.position[0] - 0.5).abs() < 1e-6);
    assert!((hover_output.position[1] - 5.0).abs() < 1e-6);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_character_move_validates_handles_pointers_and_parameters() -> Result<(), Box<dyn Error>> {
    let handle = new_handle()?;
    let valid_input = default_abi_character_input(0, [0.0, 2.0, 0.0], [1.0, 0.0, 0.0]);
    let mut output = AbiCharacterMoveOutput::default();

    assert_eq!(
        unitas_runtime_character_move(u64::MAX, Some(&valid_input), Some(&mut output)),
        -1
    );
    assert_eq!(
        unitas_runtime_character_move(handle, None, Some(&mut output)),
        -2
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&valid_input), None),
        -2
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    let mut invalid_input = valid_input;
    invalid_input.radius = -1.0;
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&invalid_input), Some(&mut output)),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    let mut disabled_input = valid_input;
    disabled_input.enabled = 0;
    assert_eq!(
        unitas_runtime_character_move(handle, Some(&disabled_input), Some(&mut output)),
        0
    );
    assert_eq!(output.position, [0.0, 2.0, 0.0]);
    assert_eq!(output.is_grounded, 0);
    assert_eq!(output.collision_flags, 0);

    let mut runtime = Runtime::new()?;
    let move_res = runtime.character_move(CharacterMoveRequest {
        entity: None,
        position: [0.0, 2.0, 0.0],
        motion: [1.0, 0.0, 0.0],
        radius: 0.5,
        height: 2.0,
        center: [0.0, 0.0, 0.0],
        slope_limit_deg: 45.0,
        step_offset: 0.3,
        skin_width: 0.08,
        min_move_distance: 0.001,
        layer_mask: u32::MAX,
        enabled: true,
        delta_seconds: 0.0,
    })?;
    assert_eq!(move_res.position, [1.0, 2.0, 0.0]);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
