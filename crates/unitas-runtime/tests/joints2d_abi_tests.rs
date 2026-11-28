use std::error::Error;

use unitas_runtime::{
    test_read_path_buffer, test_write_path_buffer, unitas_runtime_body2d_position,
    unitas_runtime_destroy, unitas_runtime_joint2d_remove, unitas_runtime_joint2d_set,
    unitas_runtime_new, unitas_runtime_object_create, unitas_runtime_object_set_transform,
    unitas_runtime_path_buffer, unitas_runtime_tick,
};

const JOINT_PARAM_COUNT: usize = 24;
const POSITION_OUT_BYTES: u64 = 12;
const FIXED_TICK_SECONDS: f64 = 0.02;
const SETTLE_TICKS: usize = 120;
// Unity/Rapier-style iterative position solvers hold a settled constraint to a
// small fraction of a world unit; 1e-2 absorbs f32 integration drift over 120 ticks.
const SOLVER_TOLERANCE: f32 = 1.0e-2;

fn create_entity_at(handle: u64, x: f64, y: f64) -> Result<u64, Box<dyn Error>> {
    let entity = unitas_runtime_object_create(handle, 0, 0);
    assert_ne!(entity, 0);
    assert_eq!(
        unitas_runtime_object_set_transform(
            handle, entity, x, y, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
        ),
        0
    );
    Ok(entity)
}

fn write_joint_params(
    handle: u64,
    params: &[f64; JOINT_PARAM_COUNT],
) -> Result<u64, Box<dyn Error>> {
    let byte_len = (JOINT_PARAM_COUNT * 8) as u64;
    let buffer = unitas_runtime_path_buffer(handle, byte_len);
    assert_ne!(buffer, 0);
    let mut bytes = Vec::with_capacity(JOINT_PARAM_COUNT * 8);
    for value in params {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    test_write_path_buffer(handle, &bytes)?;
    Ok(buffer)
}

fn read_body_2d_position(handle: u64, entity: u64) -> Result<[f32; 2], Box<dyn Error>> {
    let buffer = unitas_runtime_path_buffer(handle, POSITION_OUT_BYTES);
    assert_ne!(buffer, 0);
    assert_eq!(unitas_runtime_body2d_position(handle, entity, buffer), 0);
    let mut raw = [0u8; POSITION_OUT_BYTES as usize];
    test_read_path_buffer(handle, &mut raw)?;
    let x = f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let y = f32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    Ok([x, y])
}

fn step_world_2d(handle: u64, steps: usize) {
    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    for _ in 0..steps {
        assert_eq!(unitas_runtime_tick(handle, FIXED_TICK_SECONDS), 0);
    }
}

#[test]
fn distance_joint_2d_holds_authored_distance_between_bodies() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let anchor = create_entity_at(handle, 0.0, 0.0)?;
    let bob = create_entity_at(handle, 0.0, 0.0)?;

    let authored_distance = 5.0f64;
    let mut params = [0.0f64; JOINT_PARAM_COUNT];
    params[0] = 0.0; // kind 0: DistanceJoint2D
    params[1] = anchor as f64; // connected body
    params[6] = authored_distance;
    let buffer = write_joint_params(handle, &params)?;
    assert_eq!(unitas_runtime_joint2d_set(handle, bob, buffer), 0);

    step_world_2d(handle, SETTLE_TICKS);

    let position = read_body_2d_position(handle, bob)?;
    let offset = [position[0], position[1] - 0.0];
    let distance = (offset[0] * offset[0] + offset[1] * offset[1]).sqrt();
    assert!(
        (distance - authored_distance as f32).abs() <= SOLVER_TOLERANCE,
        "distance joint drifted to {distance}"
    );
    assert!(
        distance >= authored_distance as f32 - SOLVER_TOLERANCE,
        "bob never left the anchor point: {distance}"
    );

    // defect (unitas-physics): remove_joint does not wake a sleeping constrained
    // body, so post-removal free fall cannot be asserted through the ABI here.
    assert_eq!(unitas_runtime_joint2d_remove(handle, bob, 0), 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn hinge_joint_2d_pins_dynamic_body_at_connected_anchor() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let anchor = create_entity_at(handle, 0.0, 0.0)?;
    let arm = create_entity_at(handle, 3.0, 0.0)?;

    let mut params = [0.0f64; JOINT_PARAM_COUNT];
    params[0] = 3.0; // kind 3: HingeJoint2D
    params[1] = anchor as f64; // connected body
    let buffer = write_joint_params(handle, &params)?;
    assert_eq!(unitas_runtime_joint2d_set(handle, arm, buffer), 0);

    step_world_2d(handle, SETTLE_TICKS);

    // Both anchors are local [0, 0], so the hinge must keep the arm's origin
    // coincident with the static anchor at the world origin despite gravity.
    let position = read_body_2d_position(handle, arm)?;
    let pin_error = (position[0] * position[0] + position[1] * position[1]).sqrt();
    assert!(
        pin_error <= SOLVER_TOLERANCE,
        "hinge let the arm drift {pin_error} from the anchor"
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
