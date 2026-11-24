use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use unitas_runtime::{
    test_is_persistent_entity, test_read_path_buffer, test_write_path_buffer,
    unitas_runtime_active_scene_index, unitas_runtime_company_name, unitas_runtime_destroy,
    unitas_runtime_dont_destroy_on_load, unitas_runtime_error_message,
    unitas_runtime_load_scene_mode, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_product_name, unitas_runtime_project_version, unitas_runtime_scene_count,
    unitas_runtime_scene_name, unitas_runtime_set_active_scene, unitas_runtime_set_callbacks,
    unitas_runtime_set_time_scale, unitas_runtime_tick, unitas_runtime_time_scale,
    unitas_runtime_unload_scene, Runtime,
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

/// Pins the path buffer, writes `text` into it and returns (address, length).
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
        if let Some(entity) = graph.entity(id) {
            if entity.name == name {
                return Ok(id.as_u64());
            }
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
    // Re-pinning the buffer must not wipe the name the scene export wrote.
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
fn abi_dont_destroy_on_load_keeps_the_entity_persistent_across_a_single_load(
) -> Result<(), Box<dyn Error>> {
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
    // Calls back into the runtime while the runtime is tearing a scene down.
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

fn read_nul_terminated_from_path_buffer(handle: u64, length: i32) -> Result<String, Box<dyn Error>> {
    let byte_len = usize::try_from(length)?;
    let mut bytes = vec![0u8; byte_len + 1];
    test_read_path_buffer(handle, &mut bytes)?;
    if bytes[byte_len] != 0 {
        return Err(String::from("missing NUL terminator in path buffer").into());
    }
    Ok(std::str::from_utf8(&bytes[..byte_len])?.to_string())
}

#[test]
fn abi_project_identity_externs_write_loaded_identity_into_path_buffer(
) -> Result<(), Box<dyn Error>> {
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
fn abi_project_identity_externs_return_negative_when_settings_missing_or_handle_invalid(
) -> Result<(), Box<dyn Error>> {
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
