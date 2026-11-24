use std::error::Error;

use unitas_animation::{
    decode_controller_binary, encode_controller_binary, AnimationClip, AnimatorController,
    AnimatorState, ConditionOp, CurveBinding, CurveBindingTarget, FloatCurve, Keyframe,
    ParameterDef, StateMotion, StateTransition, TransitionCondition, TransitionTarget, WrapMode,
};
use unitas_runtime::{
    test_read_error_message, test_read_path_buffer, test_write_path_buffer,
    unitas_runtime_animator_attach, unitas_runtime_animator_controller_create,
    unitas_runtime_animator_controller_destroy, unitas_runtime_animator_get_bool,
    unitas_runtime_animator_get_float, unitas_runtime_animator_get_int,
    unitas_runtime_animator_get_speed, unitas_runtime_animator_reset_trigger,
    unitas_runtime_animator_set_bool, unitas_runtime_animator_set_controller,
    unitas_runtime_animator_set_float, unitas_runtime_animator_set_int,
    unitas_runtime_animator_set_speed, unitas_runtime_animator_set_trigger,
    unitas_runtime_animator_state_info, unitas_runtime_animator_state_name, unitas_runtime_destroy,
    unitas_runtime_error_message, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_set_time_scale, unitas_runtime_tick,
};

const STATE_INFO_SIZE: usize = 80;
const INVALID_HANDLE: u64 = u64::MAX;

fn buffer_address(handle: u64, capacity: usize) -> Result<u64, Box<dyn Error>> {
    let address = unitas_runtime_path_buffer(handle, capacity as u64);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    Ok(address)
}

fn write_bytes(handle: u64, data: &[u8]) -> Result<u64, Box<dyn Error>> {
    let address = buffer_address(handle, data.len())?;
    test_write_path_buffer(handle, data)?;
    Ok(address)
}

fn read_bytes(handle: u64, length: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = vec![0u8; length];
    if length > 0 {
        let _ = buffer_address(handle, length)?;
    }
    test_read_path_buffer(handle, &mut bytes)?;
    Ok(bytes)
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(raw)
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

#[derive(Clone, Copy, Debug)]
struct StateInfo {
    state_index: u32,
    normalized_time: f32,
    length: f32,
    speed: f32,
    weight: f32,
    is_in_transition: bool,
    next_state_index: Option<u32>,
    next_state_weight: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    fired_event_count: u32,
    diagnostic_count: u32,
}

fn parse_state_info(bytes: &[u8]) -> StateInfo {
    let next = read_u32(bytes, 24);
    StateInfo {
        state_index: read_u32(bytes, 0),
        normalized_time: read_f32(bytes, 4),
        length: read_f32(bytes, 8),
        speed: read_f32(bytes, 12),
        weight: read_f32(bytes, 16),
        is_in_transition: read_u32(bytes, 20) != 0,
        next_state_index: if next == u32::MAX { None } else { Some(next) },
        next_state_weight: read_f32(bytes, 28),
        position: [
            read_f32(bytes, 32),
            read_f32(bytes, 36),
            read_f32(bytes, 40),
        ],
        rotation: [
            read_f32(bytes, 44),
            read_f32(bytes, 48),
            read_f32(bytes, 52),
            read_f32(bytes, 56),
        ],
        scale: [
            read_f32(bytes, 60),
            read_f32(bytes, 64),
            read_f32(bytes, 68),
        ],
        fired_event_count: read_u32(bytes, 72),
        diagnostic_count: read_u32(bytes, 76),
    }
}

fn state_info(handle: u64, entity: u64) -> Result<StateInfo, Box<dyn Error>> {
    let address = buffer_address(handle, STATE_INFO_SIZE)?;
    let code = unitas_runtime_animator_state_info(handle, entity, address);
    if code != 0 {
        return Err(format!("animator state info failed with code {code}").into());
    }
    let bytes = read_bytes(handle, STATE_INFO_SIZE)?;
    Ok(parse_state_info(&bytes))
}

fn state_name(handle: u64, entity: u64) -> Result<String, Box<dyn Error>> {
    let length = unitas_runtime_animator_state_name(handle, entity);
    if length < 0 {
        return Err(format!("animator state name failed with code {length}").into());
    }
    let bytes = read_bytes(handle, length as usize)?;
    Ok(String::from_utf8(bytes)?)
}

fn idle_clip() -> AnimationClip {
    AnimationClip::new("Idle", 2.0, 60.0, WrapMode::Loop, Vec::new(), Vec::new())
}

fn single_state_controller(name: &str) -> AnimatorController {
    AnimatorController::new(
        name,
        Vec::new(),
        vec![AnimatorState::new("Idle", StateMotion::Clip(idle_clip()))],
        0,
        Vec::new(),
    )
}

fn animator_runtime(controller: &AnimatorController) -> Result<(u64, u64), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let bytes = encode_controller_binary(controller);
    let address = write_bytes(handle, &bytes)?;
    let controller_handle =
        unitas_runtime_animator_controller_create(handle, address, bytes.len() as u64);
    if controller_handle == 0 {
        return Err(String::from("animator controller creation failed").into());
    }
    let entity = unitas_runtime_animator_attach(handle, 0);
    if entity == 0 {
        return Err(String::from("animator attach failed").into());
    }
    if unitas_runtime_animator_set_controller(handle, entity, controller_handle) != 0 {
        return Err(String::from("animator controller assignment failed").into());
    }
    Ok((handle, entity))
}

fn tick(handle: u64, delta_seconds: f64) -> Result<(), Box<dyn Error>> {
    let code = unitas_runtime_tick(handle, delta_seconds);
    if code != 0 {
        return Err(format!("runtime tick failed with code {code}").into());
    }
    Ok(())
}

#[test]
fn animator_abi_controller_attach_and_state_progress() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let controller = single_state_controller("Locomotion");
    let bytes = encode_controller_binary(&controller);
    let decoded = decode_controller_binary(&bytes)?;
    assert_eq!(decoded, controller);

    let address = write_bytes(handle, &bytes)?;
    let controller_handle =
        unitas_runtime_animator_controller_create(handle, address, bytes.len() as u64);
    assert_ne!(controller_handle, 0);

    let entity = unitas_runtime_animator_attach(handle, 0);
    assert_ne!(entity, 0);
    assert_eq!(
        unitas_runtime_animator_set_controller(handle, entity, controller_handle),
        0
    );

    let info = state_info(handle, entity)?;
    assert_eq!(info.state_index, 0);
    assert_eq!(state_name(handle, entity)?, "Idle");
    assert!((info.normalized_time - 0.0).abs() < 1e-6);
    assert!((info.length - 2.0).abs() < 1e-6);
    assert!(!info.is_in_transition);
    assert_eq!(info.next_state_index, None);
    assert!((info.weight - 1.0).abs() < 1e-6);
    assert!((info.next_state_weight - 0.0).abs() < 1e-6);
    assert_eq!(info.fired_event_count, 0);
    assert_eq!(info.diagnostic_count, 0);

    // The runtime's first tick only initializes the frame, so prime it before measuring.
    tick(handle, 0.0)?;
    tick(handle, 1.0)?;
    let info = state_info(handle, entity)?;
    assert!((info.normalized_time - 0.5).abs() < 1e-5);
    assert_eq!(info.state_index, 0);
    assert_eq!(state_name(handle, entity)?, "Idle");

    tick(handle, 1.0)?;
    let info = state_info(handle, entity)?;
    assert!((info.normalized_time - 1.0).abs() < 1e-5);

    assert_eq!(
        unitas_runtime_animator_controller_destroy(handle, controller_handle),
        0
    );
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn animator_abi_parameters_drive_state_transitions() -> Result<(), Box<dyn Error>> {
    let controller = AnimatorController::new(
        "Parameters",
        vec![
            ParameterDef::float("speed", 0.0),
            ParameterDef::int("count", 0),
            ParameterDef::bool("crouch", false),
        ],
        vec![
            AnimatorState::new("Idle", StateMotion::Clip(idle_clip())),
            AnimatorState::new("Run", StateMotion::Clip(idle_clip())),
            AnimatorState::new("Walk", StateMotion::Clip(idle_clip())),
            AnimatorState::new("Crouch", StateMotion::Clip(idle_clip())),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new("speed", ConditionOp::Greater(0.5))],
            ),
            StateTransition::instant(
                1,
                TransitionTarget::State(2),
                vec![TransitionCondition::new("count", ConditionOp::Equals(3))],
            ),
            StateTransition::instant(
                2,
                TransitionTarget::State(3),
                vec![TransitionCondition::new("crouch", ConditionOp::If)],
            ),
        ],
    );
    let (handle, entity) = animator_runtime(&controller)?;
    tick(handle, 0.0)?;
    assert_eq!(state_info(handle, entity)?.state_index, 0);

    let name = write_bytes(handle, b"speed")?;
    assert_eq!(
        unitas_runtime_animator_set_float(handle, entity, name, 5, 1.0),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 1);
    assert_eq!(state_name(handle, entity)?, "Run");

    // The path buffer is one scratch buffer: the name is written again right
    // before the call that consumes it.
    let name = write_bytes(handle, b"speed")?;
    let out = buffer_address(handle, 4)?;
    assert_eq!(
        unitas_runtime_animator_get_float(handle, entity, name, 5, out),
        0
    );
    let value = read_f32(&read_bytes(handle, 4)?, 0);
    assert!((value - 1.0).abs() < 1e-6);

    let name = write_bytes(handle, b"count")?;
    assert_eq!(
        unitas_runtime_animator_set_int(handle, entity, name, 5, 3),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 2);
    assert_eq!(state_name(handle, entity)?, "Walk");

    let name = write_bytes(handle, b"count")?;
    let out = buffer_address(handle, 4)?;
    assert_eq!(
        unitas_runtime_animator_get_int(handle, entity, name, 5, out),
        0
    );
    let value = read_u32(&read_bytes(handle, 4)?, 0);
    assert_eq!(value as i32, 3);

    let name = write_bytes(handle, b"crouch")?;
    assert_eq!(
        unitas_runtime_animator_set_bool(handle, entity, name, 6, 1),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 3);
    assert_eq!(state_name(handle, entity)?, "Crouch");

    let name = write_bytes(handle, b"crouch")?;
    let out = buffer_address(handle, 4)?;
    assert_eq!(
        unitas_runtime_animator_get_bool(handle, entity, name, 6, out),
        0
    );
    let value = read_u32(&read_bytes(handle, 4)?, 0);
    assert_eq!(value, 1);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn animator_abi_trigger_fires_once_until_reset() -> Result<(), Box<dyn Error>> {
    let controller = AnimatorController::new(
        "Trigger",
        vec![ParameterDef::trigger("go")],
        vec![
            AnimatorState::new("Idle", StateMotion::Clip(idle_clip())),
            AnimatorState::new("Jump", StateMotion::Clip(idle_clip())),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new("go", ConditionOp::If)],
            ),
            StateTransition::instant(
                1,
                TransitionTarget::State(0),
                vec![TransitionCondition::new("go", ConditionOp::If)],
            ),
        ],
    );
    let (handle, entity) = animator_runtime(&controller)?;
    tick(handle, 0.0)?;
    assert_eq!(state_info(handle, entity)?.state_index, 0);

    let name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, name, 2),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 1);
    assert_eq!(state_name(handle, entity)?, "Jump");

    // The trigger was consumed by the transition, so no second transition happens.
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 1);

    // A second trigger is required for the next transition.
    let name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, name, 2),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 0);
    assert_eq!(state_name(handle, entity)?, "Idle");

    // A reset trigger clears the pending trigger, so no transition happens.
    let name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, name, 2),
        0
    );
    let name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_reset_trigger(handle, entity, name, 2),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 0);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn animator_abi_time_scale_scales_animator_advance() -> Result<(), Box<dyn Error>> {
    let controller = single_state_controller("Scaled");
    let (full_handle, full_entity) = animator_runtime(&controller)?;
    assert_eq!(unitas_runtime_set_time_scale(full_handle, 1.0), 0);
    tick(full_handle, 0.0)?;
    for _ in 0..4 {
        tick(full_handle, 0.25)?;
    }
    let full = state_info(full_handle, full_entity)?.normalized_time;

    let (half_handle, half_entity) = animator_runtime(&controller)?;
    assert_eq!(unitas_runtime_set_time_scale(half_handle, 0.5), 0);
    tick(half_handle, 0.0)?;
    for _ in 0..4 {
        tick(half_handle, 0.25)?;
    }
    let half = state_info(half_handle, half_entity)?.normalized_time;

    let (frozen_handle, frozen_entity) = animator_runtime(&controller)?;
    assert_eq!(unitas_runtime_set_time_scale(frozen_handle, 0.0), 0);
    tick(frozen_handle, 0.0)?;
    for _ in 0..4 {
        tick(frozen_handle, 0.25)?;
    }
    let frozen = state_info(frozen_handle, frozen_entity)?.normalized_time;

    assert!(
        (full - 0.5).abs() < 1e-5,
        "full speed normalized time {full}"
    );
    assert!(
        (half - full * 0.5).abs() < 1e-5,
        "half speed normalized time {half} vs {full}"
    );
    assert!(frozen.abs() < 1e-6, "frozen normalized time {frozen}");

    assert_eq!(unitas_runtime_destroy(full_handle), 0);
    assert_eq!(unitas_runtime_destroy(half_handle), 0);
    assert_eq!(unitas_runtime_destroy(frozen_handle), 0);
    Ok(())
}

#[test]
fn animator_abi_clip_moves_entity_transform() -> Result<(), Box<dyn Error>> {
    let clip = AnimationClip::new(
        "Move",
        2.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(2.0, 4.0)]),
        )],
        Vec::new(),
    );
    let controller = AnimatorController::new(
        "Move",
        Vec::new(),
        vec![AnimatorState::new("Move", StateMotion::Clip(clip))],
        0,
        Vec::new(),
    );
    let (handle, entity) = animator_runtime(&controller)?;
    tick(handle, 0.0)?;

    let info = state_info(handle, entity)?;
    assert!((info.position[0] - 0.0).abs() < 1e-6);
    assert!((info.position[1] - 0.0).abs() < 1e-6);
    assert!((info.position[2] - 0.0).abs() < 1e-6);
    assert!((info.scale[0] - 1.0).abs() < 1e-6);
    assert!((info.scale[1] - 1.0).abs() < 1e-6);
    assert!((info.scale[2] - 1.0).abs() < 1e-6);
    assert!((info.rotation[0] - 0.0).abs() < 1e-6);
    assert!((info.rotation[1] - 0.0).abs() < 1e-6);
    assert!((info.rotation[2] - 0.0).abs() < 1e-6);
    assert!((info.rotation[3] - 1.0).abs() < 1e-6);

    tick(handle, 1.0)?;
    let info = state_info(handle, entity)?;
    assert!(
        (info.position[0] - 2.0).abs() < 1e-5,
        "animated position {}",
        info.position[0]
    );
    assert!((info.position[1] - 0.0).abs() < 1e-6);
    assert!((info.position[2] - 0.0).abs() < 1e-6);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn animator_abi_speed_getter_and_setter_round_trip() -> Result<(), Box<dyn Error>> {
    let controller = single_state_controller("Speed");
    let (handle, entity) = animator_runtime(&controller)?;
    let out = buffer_address(handle, 4)?;
    assert_eq!(unitas_runtime_animator_get_speed(handle, entity, out), 0);
    let value = read_f32(&read_bytes(handle, 4)?, 0);
    assert!((value - 1.0).abs() < 1e-6);

    assert_eq!(unitas_runtime_animator_set_speed(handle, entity, 2.0), 0);
    assert_eq!(unitas_runtime_animator_get_speed(handle, entity, out), 0);
    let value = read_f32(&read_bytes(handle, 4)?, 0);
    assert!((value - 2.0).abs() < 1e-6);

    tick(handle, 0.0)?;
    tick(handle, 0.5)?;
    let info = state_info(handle, entity)?;
    assert!(
        (info.normalized_time - 0.5).abs() < 1e-5,
        "normalized time {}",
        info.normalized_time
    );
    assert!((info.speed - 2.0).abs() < 1e-6);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn animator_abi_error_paths_report_failures() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    assert_eq!(unitas_runtime_animator_attach(INVALID_HANDLE, 0), 0);
    assert_eq!(
        unitas_runtime_animator_controller_create(INVALID_HANDLE, 0, 0),
        0
    );
    assert_eq!(
        unitas_runtime_animator_controller_destroy(INVALID_HANDLE, 1),
        -1
    );
    assert_eq!(
        unitas_runtime_animator_set_controller(INVALID_HANDLE, 1, 1),
        -1
    );
    assert_eq!(
        unitas_runtime_animator_set_float(INVALID_HANDLE, 1, 0, 0, 0.0),
        -1
    );
    assert_eq!(
        unitas_runtime_animator_get_float(INVALID_HANDLE, 1, 0, 0, 0),
        -1
    );
    assert_eq!(
        unitas_runtime_animator_set_speed(INVALID_HANDLE, 1, 1.0),
        -1
    );
    assert_eq!(unitas_runtime_animator_state_info(INVALID_HANDLE, 1, 0), -1);
    assert_eq!(unitas_runtime_animator_state_name(INVALID_HANDLE, 1), -1);

    let garbage = b"not a controller";
    let address = write_bytes(handle, garbage)?;
    assert_eq!(
        unitas_runtime_animator_controller_create(handle, address, garbage.len() as u64),
        0
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);

    let controller = AnimatorController::new(
        "Errors",
        vec![ParameterDef::float("speed", 0.0)],
        vec![AnimatorState::new("Idle", StateMotion::Clip(idle_clip()))],
        0,
        Vec::new(),
    );
    let bytes = encode_controller_binary(&controller);
    let address = write_bytes(handle, &bytes)?;
    let controller_handle =
        unitas_runtime_animator_controller_create(handle, address, bytes.len() as u64);
    assert_ne!(controller_handle, 0);
    let entity = unitas_runtime_animator_attach(handle, 0);
    assert_ne!(entity, 0);
    assert_eq!(
        unitas_runtime_animator_set_controller(handle, entity, controller_handle),
        0
    );

    let unknown = write_bytes(handle, b"missing")?;
    assert_eq!(
        unitas_runtime_animator_set_float(handle, entity, unknown, 7, 1.0),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    let message = test_read_error_message(handle)?;
    assert!(
        message.contains("missing"),
        "unexpected error message {message}"
    );

    assert_eq!(
        unitas_runtime_animator_set_int(handle, entity, unknown, 7, 1),
        -4
    );
    assert_eq!(
        unitas_runtime_animator_set_bool(handle, entity, unknown, 7, 1),
        -4
    );
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, unknown, 7),
        -4
    );
    assert_eq!(
        unitas_runtime_animator_reset_trigger(handle, entity, unknown, 7),
        -4
    );

    let out = buffer_address(handle, 4)?;
    assert_eq!(
        unitas_runtime_animator_get_float(handle, entity, unknown, 7, out),
        -4
    );

    let unknown_entity = 9999;
    let speed = write_bytes(handle, b"speed")?;
    assert_eq!(
        unitas_runtime_animator_set_float(handle, unknown_entity, speed, 5, 1.0),
        -4
    );
    assert_ne!(unitas_runtime_error_message(handle), 0);
    let message = test_read_error_message(handle)?;
    assert!(
        message.contains("9999"),
        "unexpected error message {message}"
    );
    let out = buffer_address(handle, STATE_INFO_SIZE)?;
    assert_eq!(
        unitas_runtime_animator_state_info(handle, unknown_entity, out),
        -4
    );
    assert_eq!(
        unitas_runtime_animator_state_name(handle, unknown_entity),
        -4
    );
    assert_eq!(
        unitas_runtime_animator_set_controller(handle, unknown_entity, controller_handle),
        -4
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

fn scenario_controller() -> AnimatorController {
    let idle = AnimationClip::new("Idle", 2.0, 60.0, WrapMode::Loop, Vec::new(), Vec::new());
    let run = AnimationClip::new(
        "Run",
        2.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(2.0, 4.0)]),
        )],
        Vec::new(),
    );
    AnimatorController::new(
        "Locomotion",
        vec![ParameterDef::trigger("go")],
        vec![
            AnimatorState::new("Idle", StateMotion::Clip(idle)),
            AnimatorState::new("Run", StateMotion::Clip(run)),
        ],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("go", ConditionOp::If)],
        )],
    )
}

type Observation = (u32, String, f32, f32);

fn run_scenario() -> Result<Vec<Observation>, Box<dyn Error>> {
    let controller = scenario_controller();
    let (handle, entity) = animator_runtime(&controller)?;
    tick(handle, 0.0)?;
    tick(handle, 0.5)?;
    let mut observations = Vec::new();
    let info = state_info(handle, entity)?;
    observations.push((
        info.state_index,
        state_name(handle, entity)?,
        info.normalized_time,
        info.position[0],
    ));
    let trigger_name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, trigger_name, 2),
        0
    );
    for _ in 0..2 {
        tick(handle, 0.25)?;
        let info = state_info(handle, entity)?;
        observations.push((
            info.state_index,
            state_name(handle, entity)?,
            info.normalized_time,
            info.position[0],
        ));
    }
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(observations)
}

#[test]
fn animator_abi_scenario_is_repeatable() -> Result<(), Box<dyn Error>> {
    let first = run_scenario()?;
    let second = run_scenario()?;
    assert_eq!(first, second, "identical inputs must produce identical outputs");
    assert!(
        first.iter().any(|observation| observation.0 == 1),
        "the trigger must move the animator into the Run state: {first:?}"
    );
    Ok(())
}
