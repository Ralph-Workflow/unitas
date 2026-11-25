use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_core::EntityId;
use unitas_physics::BodyRole;
use unitas_runtime::{
    Runtime, test_write_path_buffer, unitas_runtime_destroy, unitas_runtime_error_message,
    unitas_runtime_load_scene_mode, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_set_body_rotation,
};

const COLLISION_FIXTURE: &str = "tests/fixtures/scene_physics_collision.unity";
const BALL_NAME: &str = "Ball";
const CRATE_NAME: &str = "Crate";
const QUARTER_TURN_Y: [f64; 4] = [
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
];
const IDENTITY_ROTATION: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const TOLERANCE: f64 = 1e-6;
const TICK_DELTA: f64 = 0.02;
const TICK_COUNT: usize = 2;

fn kinematic_scene_path(label: &str) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_kinematic_rotation_{label}_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let scene_path = temp_dir.join("kinematic_rotation.unity");
    let yaml = include_str!("fixtures/scene_physics_collision.unity").replace(
        "  m_Mass: 1\n  m_UseGravity: 1",
        "  m_Mass: 1\n  m_UseGravity: 1\n  m_IsKinematic: 1",
    );
    std::fs::write(&scene_path, yaml)?;
    Ok((temp_dir, scene_path))
}

fn entity_id(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
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

fn entity_id_for_scene(scene_path: &Path, name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(scene_path)?;
    entity_id(&runtime, name)
}

fn body_rotation(runtime: &mut Runtime, entity: EntityId) -> Result<[f64; 4], Box<dyn Error>> {
    match runtime.physics_mut().world_3d().rotation(entity) {
        Some(rotation) => Ok(rotation),
        None => Err(String::from("entity has no physics body").into()),
    }
}

fn assert_rotation_close(actual: [f64; 4], expected: [f64; 4], label: &str) {
    for axis in 0..4 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= TOLERANCE,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }
}

fn write_quaternion_to_path_buffer(handle: u64, value: [f64; 4]) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, 32);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&value[0].to_le_bytes());
    bytes[8..16].copy_from_slice(&value[1].to_le_bytes());
    bytes[16..24].copy_from_slice(&value[2].to_le_bytes());
    bytes[24..32].copy_from_slice(&value[3].to_le_bytes());
    test_write_path_buffer(handle, &bytes)?;
    Ok(address)
}

#[test]
fn runtime_set_body_rotation_turns_the_kinematic_physics_body() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = kinematic_scene_path("runtime")?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let ball = EntityId::new(entity_id(&runtime, BALL_NAME)?);
    assert_eq!(runtime.body_role(ball), Some(BodyRole::Kinematic));
    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }

    assert_rotation_close(
        body_rotation(&mut runtime, ball)?,
        IDENTITY_ROTATION,
        "kinematic rotation before the runtime write",
    );
    runtime.set_body_rotation(ball, QUARTER_TURN_Y)?;
    assert_rotation_close(
        body_rotation(&mut runtime, ball)?,
        QUARTER_TURN_Y,
        "kinematic rotation after the runtime write",
    );
    runtime.tick(TICK_DELTA)?;
    assert_rotation_close(
        body_rotation(&mut runtime, ball)?,
        QUARTER_TURN_Y,
        "kinematic rotation after a fixed step",
    );
    Ok(())
}

#[test]
fn runtime_set_body_rotation_rejects_a_dynamic_body() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(COLLISION_FIXTURE))?;
    let ball = EntityId::new(entity_id(&runtime, BALL_NAME)?);
    assert_eq!(runtime.body_role(ball), Some(BodyRole::Dynamic));
    assert!(
        runtime.set_body_rotation(ball, QUARTER_TURN_Y).is_err(),
        "a dynamic body owns its rotation and must reject a script rotation write"
    );
    Ok(())
}

#[test]
fn abi_set_body_rotation_turns_the_kinematic_body_and_rejects_invalid_writes()
-> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = kinematic_scene_path("abi")?;
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let path_bytes = scene_path.to_string_lossy().as_bytes().to_vec();
    let address = unitas_runtime_path_buffer(handle, path_bytes.len() as u64);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, &path_bytes)?;
    let ball = entity_id_for_scene(&scene_path, BALL_NAME)?;
    let crate_entity = entity_id_for_scene(&scene_path, CRATE_NAME)?;
    let load_code = unitas_runtime_load_scene_mode(handle, address, path_bytes.len() as u64, 0);
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert_eq!(load_code, 0);

    let buffer = write_quaternion_to_path_buffer(handle, QUARTER_TURN_Y)?;
    assert_eq!(unitas_runtime_set_body_rotation(handle, ball, buffer), 0);
    assert_eq!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_set_body_rotation(handle, crate_entity, buffer),
        0
    );
    assert_eq!(unitas_runtime_error_message(handle), 0);
    assert_eq!(
        unitas_runtime_set_body_rotation(handle, 999_999, buffer),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_set_body_rotation(handle, ball, 0), -2);
    assert_ne!(unitas_runtime_error_message(handle), 0);
    assert_eq!(unitas_runtime_set_body_rotation(u64::MAX, ball, buffer), -1);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
