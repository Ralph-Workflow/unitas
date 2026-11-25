use std::error::Error;
use std::path::Path;

use unitas_core::EntityId;
use unitas_physics::CollisionDetectionMode3D;
use unitas_runtime::{
    Runtime, test_write_path_buffer, unitas_runtime_body_set_collision_detection_mode,
    unitas_runtime_destroy, unitas_runtime_error_message, unitas_runtime_load_scene,
    unitas_runtime_new, unitas_runtime_path_buffer,
};

const SCENE_PATH: &str = "tests/fixtures/scene_physics_collision_no_script.unity";
const BALL_NAME: &str = "Ball";
const MODE_DISCRETE: u32 = 0;
const MODE_CONTINUOUS: u32 = 1;
const MODE_UNKNOWN: u32 = 7;
const MISSING_ENTITY: u64 = 0xDEAD_BEEF;

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

fn load_scene(handle: u64, path: &str) -> Result<i32, Box<dyn Error>> {
    let bytes = path.as_bytes();
    let address = unitas_runtime_path_buffer(handle, 256);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, bytes)?;
    Ok(unitas_runtime_load_scene(
        handle,
        address,
        u64::try_from(bytes.len())?,
    ))
}

#[test]
fn runtime_sets_collision_detection_mode_on_a_loaded_body() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_PATH))?;
    let ball = EntityId::new(entity_id_for(SCENE_PATH, BALL_NAME)?);
    runtime
        .physics_mut()
        .set_collision_detection_mode_3d(ball, CollisionDetectionMode3D::Continuous)?;
    runtime
        .physics_mut()
        .set_collision_detection_mode_3d(ball, CollisionDetectionMode3D::Discrete)?;
    Ok(())
}

#[test]
fn runtime_rejects_collision_detection_mode_for_an_unknown_body() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_PATH))?;
    assert!(
        runtime
            .physics_mut()
            .set_collision_detection_mode_3d(
                EntityId::new(MISSING_ENTITY),
                CollisionDetectionMode3D::Continuous,
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn abi_sets_collision_detection_mode_on_a_loaded_body() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    assert_eq!(load_scene(handle, SCENE_PATH)?, 0);
    let ball = entity_id_for(SCENE_PATH, BALL_NAME)?;
    assert_eq!(
        unitas_runtime_body_set_collision_detection_mode(handle, ball, MODE_CONTINUOUS),
        0
    );
    assert_eq!(
        unitas_runtime_body_set_collision_detection_mode(handle, ball, MODE_DISCRETE),
        0
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_rejects_an_unknown_collision_detection_mode() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    assert_eq!(load_scene(handle, SCENE_PATH)?, 0);
    let ball = entity_id_for(SCENE_PATH, BALL_NAME)?;
    assert_eq!(
        unitas_runtime_body_set_collision_detection_mode(handle, ball, MODE_UNKNOWN),
        -2
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn abi_rejects_collision_detection_mode_for_an_unknown_body() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    assert_eq!(load_scene(handle, SCENE_PATH)?, 0);
    assert_eq!(
        unitas_runtime_body_set_collision_detection_mode(handle, MISSING_ENTITY, MODE_CONTINUOUS),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
