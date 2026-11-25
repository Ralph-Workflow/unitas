use std::error::Error;
use std::path::Path;

use unitas_animation::{
    decode_controller_binary, encode_controller_binary, AnimationClip, AnimatorController,
    AnimatorState, ConditionOp, CurveBinding, CurveBindingTarget, FloatCurve, Keyframe,
    ParameterDef, StateMotion, StateTransition, TransitionCondition, TransitionTarget, WrapMode,
};
use unitas_core::EntityId;
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
    unitas_runtime_set_time_scale, unitas_runtime_tick, AnimationRuntimeDiagnostic, Runtime,
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

    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 1);

    let name = write_bytes(handle, b"go")?;
    assert_eq!(
        unitas_runtime_animator_set_trigger(handle, entity, name, 2),
        0
    );
    tick(handle, 0.1)?;
    assert_eq!(state_info(handle, entity)?.state_index, 0);
    assert_eq!(state_name(handle, entity)?, "Idle");

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

const CHILD_HIERARCHY_SCENE_PATH: &str = "tests/fixtures/scene_animation_children.unity";

fn child_hierarchy_runtime() -> Result<(Runtime, EntityId, EntityId), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(CHILD_HIERARCHY_SCENE_PATH))?;
    let root = entity_by_name(&runtime, "Root")?;
    let child = entity_by_name(&runtime, "Child")?;
    Ok((runtime, root, child))
}

fn entity_by_name(runtime: &Runtime, name: &str) -> Result<EntityId, Box<dyn Error>> {
    let graph = runtime.scene_graph().ok_or("no scene graph found")?;
    graph
        .breadth_first_order()
        .into_iter()
        .find(|id| graph.entity(*id).is_some_and(|entity| entity.name == name))
        .ok_or_else(|| -> Box<dyn Error> { format!("entity '{name}' not found").into() })
}

fn local_position(runtime: &Runtime, entity: EntityId) -> Result<[f32; 3], Box<dyn Error>> {
    let graph = runtime.scene_graph().ok_or("no scene graph found")?;
    let transform = graph
        .transform(entity)
        .ok_or("entity has no transform in the scene graph")?;
    Ok(transform.translation)
}

fn register_controller(
    runtime: &mut Runtime,
    controller: &AnimatorController,
) -> Result<u64, Box<dyn Error>> {
    let bytes = encode_controller_binary(controller);
    let decoded = decode_controller_binary(&bytes)?;
    if decoded != *controller {
        return Err(String::from("controller round trip mismatch").into());
    }
    Ok(runtime
        .animation_mut()
        .decode_and_register_controller(&bytes)?)
}

fn position_clip(path: &str) -> AnimationClip {
    AnimationClip::new(
        "Move",
        2.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new(path, "Transform", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(2.0, 4.0)]),
        )],
        Vec::new(),
    )
}

#[test]
fn animator_runtime_non_transform_binding_reports_diagnostic_and_keeps_advancing(
) -> Result<(), Box<dyn Error>> {
    let clip = AnimationClip::new(
        "Tint",
        2.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "SpriteRenderer", "color"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(2.0, 1.0)]),
        )],
        Vec::new(),
    );
    let controller = AnimatorController::new(
        "Tint",
        vec![ParameterDef::trigger("go")],
        vec![
            AnimatorState::new("Tint", StateMotion::Clip(clip.clone())),
            AnimatorState::new("Fade", StateMotion::Clip(clip)),
        ],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("go", ConditionOp::If)],
        )],
    );
    let mut runtime = Runtime::new()?;
    let entity = runtime.ensure_animator(0);
    assert_eq!(runtime.animation().diagnostic_count(entity), Some(0));

    let handle = register_controller(&mut runtime, &controller)?;
    runtime.set_animator_controller(entity, handle)?;
    assert_eq!(runtime.animation().diagnostic_count(entity), Some(1));
    let diagnostics = runtime
        .animation()
        .typed_diagnostics(entity)
        .ok_or("animator has no typed diagnostics")?;
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
        AnimationRuntimeDiagnostic::UnsupportedBinding {
            object_path,
            component,
            property,
            ..
        } => {
            assert_eq!(object_path, "");
            assert_eq!(component, "SpriteRenderer");
            assert_eq!(property, "color");
        }
        other => return Err(format!("unexpected diagnostic {other:?}").into()),
    }
    assert!(
        diagnostics[0]
            .to_string()
            .contains("is not yet applied in runtime"),
        "diagnostic text {}",
        diagnostics[0]
    );

    runtime.tick(0.0)?;
    runtime.tick(1.0)?;
    let info = runtime.animation().state_info(entity)?;
    assert_eq!(info.state_index, 0);
    assert_eq!(info.state_name, "Tint");
    assert!(
        (info.normalized_time - 0.5).abs() < 1e-5,
        "normalized time {}",
        info.normalized_time
    );

    runtime.animation_mut().set_trigger(entity, "go")?;
    runtime.tick(0.1)?;
    let info = runtime.animation().state_info(entity)?;
    assert_eq!(info.state_index, 1);
    assert_eq!(info.state_name, "Fade");
    assert_eq!(runtime.animation().diagnostic_count(entity), Some(1));
    Ok(())
}

#[test]
fn animator_runtime_child_object_path_moves_child_transform() -> Result<(), Box<dyn Error>> {
    let (mut runtime, root, child) = child_hierarchy_runtime()?;
    let controller = AnimatorController::new(
        "ChildMove",
        Vec::new(),
        vec![AnimatorState::new(
            "ChildMove",
            StateMotion::Clip(position_clip("Parent/Child")),
        )],
        0,
        Vec::new(),
    );
    let entity = runtime.ensure_animator(root.as_u64());
    let handle = register_controller(&mut runtime, &controller)?;
    runtime.set_animator_controller(entity, handle)?;
    runtime.tick(0.0)?;
    let before = local_position(&runtime, child)?;
    assert!(
        (before[0] - 0.0).abs() < 1e-6,
        "child local position before the clip {}",
        before[0]
    );

    runtime.tick(1.0)?;
    let after = local_position(&runtime, child)?;
    assert!(
        (after[0] - 2.0).abs() < 1e-5,
        "child local position {}",
        after[0]
    );
    assert!((after[1] - 0.0).abs() < 1e-6);
    assert!((after[2] - 0.0).abs() < 1e-6);
    assert_eq!(runtime.animation().diagnostic_count(entity), Some(0));
    Ok(())
}

#[test]
fn animator_runtime_unresolved_child_path_records_diagnostic() -> Result<(), Box<dyn Error>> {
    let (mut runtime, root, child) = child_hierarchy_runtime()?;
    let controller = AnimatorController::new(
        "MissingChild",
        Vec::new(),
        vec![AnimatorState::new(
            "MissingChild",
            StateMotion::Clip(position_clip("Parent/Missing")),
        )],
        0,
        Vec::new(),
    );
    let entity = runtime.ensure_animator(root.as_u64());
    let handle = register_controller(&mut runtime, &controller)?;
    runtime.set_animator_controller(entity, handle)?;
    runtime.tick(0.0)?;
    runtime.tick(1.0)?;

    let diagnostics = runtime
        .animation()
        .typed_diagnostics(entity)
        .ok_or("animator has no typed diagnostics")?;
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
        AnimationRuntimeDiagnostic::UnresolvedChildPath { object_path, .. } => {
            assert_eq!(object_path, "Parent/Missing");
        }
        other => return Err(format!("unexpected diagnostic {other:?}").into()),
    }
    assert!(
        diagnostics[0]
            .to_string()
            .contains("could not be resolved in scene graph"),
        "diagnostic text {}",
        diagnostics[0]
    );
    assert_eq!(runtime.animation().diagnostic_count(entity), Some(1));

    let after = local_position(&runtime, child)?;
    assert!(
        (after[0] - 0.0).abs() < 1e-6,
        "an unresolved path must not move the child {}",
        after[0]
    );
    let info = runtime.animation().state_info(entity)?;
    assert!(
        (info.normalized_time - 0.5).abs() < 1e-5,
        "normalized time {}",
        info.normalized_time
    );
    Ok(())
}

#[derive(Default)]
struct HookState {
    moves: Vec<unitas_runtime::animation::AnimatorRootMotion>,
    ik_layers: Vec<u32>,
}

struct HookRecorder {
    state: std::rc::Rc<std::cell::RefCell<HookState>>,
    ik_body_position: Option<[f32; 3]>,
    handles_move: bool,
}

impl unitas_runtime::animation::AnimatorScriptHooks for HookRecorder {
    fn on_animator_ik(
        &mut self,
        _entity: EntityId,
        layer_index: u32,
        pose: &mut unitas_runtime::animation::AnimatorIkPose,
    ) {
        self.state.borrow_mut().ik_layers.push(layer_index);
        pose.body_position = self.ik_body_position;
    }

    fn on_animator_move(
        &mut self,
        _entity: EntityId,
        motion: &mut unitas_runtime::animation::AnimatorRootMotion,
    ) -> bool {
        self.state.borrow_mut().moves.push(*motion);
        if self.handles_move {
            motion.delta_position = [0.0; 3];
        }
        self.handles_move
    }
}

fn hook_position_clip() -> AnimationClip {
    AnimationClip::new(
        "HookMove",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![
                Keyframe::linear(0.0, 0.0),
                Keyframe::linear(1.0, 1.0),
            ]),
        )],
        Vec::new(),
    )
}

fn hook_controller() -> AnimatorController {
    AnimatorController::new(
        "Hook",
        Vec::new(),
        vec![AnimatorState::new("Move", StateMotion::Clip(hook_position_clip()))],
        0,
        Vec::new(),
    )
}

fn hook_world() -> Result<(unitas_runtime::AnimationWorld, EntityId), Box<dyn Error>> {
    let mut world = unitas_runtime::AnimationWorld::new();
    let entity = EntityId::new(11);
    world.ensure_animator(entity, unitas_core::Transform::default());
    let controller_handle = world.register_controller(hook_controller());
    world
        .set_controller(entity, controller_handle)
        .map_err(|message| -> Box<dyn Error> { message.into() })?;
    Ok((world, entity))
}

#[test]
fn animation_ik_hook_alters_the_pose_after_the_layer_pose_is_evaluated(
) -> Result<(), Box<dyn Error>> {
    let (mut world, entity) = hook_world()?;
    let state = std::rc::Rc::new(std::cell::RefCell::new(HookState::default()));
    let mut hooks = unitas_runtime::animation::AnimatorHookWorld::new();
    hooks.attach(
        entity,
        Box::new(HookRecorder {
            state: state.clone(),
            ik_body_position: Some([1.0, 2.0, 3.0]),
            handles_move: false,
        }),
    );
    hooks.set_ik_layers(entity, vec![0]);
    assert!(hooks.is_attached(entity));
    assert_eq!(hooks.ik_layers(entity), &[0]);

    world.tick_with_hooks(0.5, &mut hooks);

    assert_eq!(state.borrow().ik_layers, vec![0]);
    let transform = world.transform(entity).ok_or("missing animator transform")?;
    assert_eq!(transform.translation, [1.0, 2.0, 3.0]);
    assert!(hooks.detach(entity));
    assert!(!hooks.is_attached(entity));
    Ok(())
}

#[test]
fn animation_move_hook_receives_root_motion_and_suppresses_the_automatic_application(
) -> Result<(), Box<dyn Error>> {
    let (mut world, entity) = hook_world()?;
    let state = std::rc::Rc::new(std::cell::RefCell::new(HookState::default()));
    let mut hooks = unitas_runtime::animation::AnimatorHookWorld::new();
    hooks.attach(
        entity,
        Box::new(HookRecorder {
            state: state.clone(),
            ik_body_position: None,
            handles_move: true,
        }),
    );

    world.tick_with_hooks(0.5, &mut hooks);

    let moves = state.borrow().moves.clone();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].delta_position, [0.5, 0.0, 0.0]);
    assert_eq!(moves[0].delta_rotation, [0.0, 0.0, 0.0, 1.0]);
    let transform = world.transform(entity).ok_or("missing animator transform")?;
    assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn animation_without_a_move_hook_applies_root_motion_automatically() -> Result<(), Box<dyn Error>> {
    let (mut world, entity) = hook_world()?;
    let mut hooks = unitas_runtime::animation::AnimatorHookWorld::new();

    world.tick_with_hooks(0.5, &mut hooks);

    let transform = world.transform(entity).ok_or("missing animator transform")?;
    assert_eq!(transform.translation, [0.5, 0.0, 0.0]);
    Ok(())
}
