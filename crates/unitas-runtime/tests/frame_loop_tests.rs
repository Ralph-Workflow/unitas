use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_animation::{
    AnimationClip, AnimatorController, AnimatorState, CurveBinding, CurveBindingTarget, FloatCurve,
    Keyframe, StateMotion, WrapMode, encode_controller_binary,
};
use unitas_core::{EntityId, Transform};
use unitas_jobs::JobSystem;
use unitas_runtime::{
    Callbacks, DEFAULT_MAXIMUM_DELTA_TIME, Runtime, test_read_path_buffer, test_write_path_buffer,
    unitas_runtime_animator_attach, unitas_runtime_animator_controller_create,
    unitas_runtime_animator_set_controller, unitas_runtime_animator_state_info,
    unitas_runtime_load_scene, unitas_runtime_maximum_delta_time, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_set_callbacks,
    unitas_runtime_set_maximum_delta_time, unitas_runtime_tick,
};

const SCENE_WITH_SCRIPT: &str = "tests/fixtures/scene_with_script.unity";
const STATE_INFO_SIZE: usize = 80;
const NORMALIZED_TIME_OFFSET: usize = 4;
const TEN_SECOND_FRAME: f64 = 10.0;
const FIXED_STEPS_IN_TEN_SECONDS: usize = 500;

static FIXED_STEPS: Mutex<usize> = Mutex::new(0);
static UPDATE_DELTAS: Mutex<Vec<f64>> = Mutex::new(Vec::new());
static FRAME_LOOP_RUN_LOCK: Mutex<()> = Mutex::new(());

struct AnimatorProbe {
    handle: u64,
    entity: u64,
    normalized_times: Vec<f32>,
}

static ANIMATOR_PROBE: Mutex<Option<AnimatorProbe>> = Mutex::new(None);

extern "C" fn frame_create(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn frame_noop_u64(_handle: u64) {}

extern "C" fn frame_noop_f64(_handle: u64, _delta: f64) {}

extern "C" fn frame_fixed(_handle: u64, _step: f64) {
    if let Ok(mut steps) = FIXED_STEPS.lock() {
        *steps += 1;
    }
}

extern "C" fn frame_update(_handle: u64, delta: f64) {
    if let Ok(mut deltas) = UPDATE_DELTAS.lock() {
        deltas.push(delta);
    }
}

extern "C" fn frame_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn frame_trigger(_a: u64, _b: u64, _phase: u32) {}

fn frame_callbacks() -> Callbacks {
    Callbacks {
        create_script: frame_create,
        awake: frame_noop_u64,
        on_enable: frame_noop_u64,
        start: frame_noop_u64,
        fixed_update: frame_fixed,
        update: frame_update,
        late_update: frame_noop_f64,
        on_disable: frame_noop_u64,
        destroy: frame_noop_u64,
        collision: frame_collision,
        trigger: frame_trigger,
        ..Callbacks::default()
    }
}

fn fixed_step_count() -> Result<usize, Box<dyn Error>> {
    match FIXED_STEPS.lock() {
        Ok(guard) => Ok(*guard),
        Err(_) => Err(String::from("fixed step counter poisoned").into()),
    }
}

fn update_deltas() -> Result<Vec<f64>, Box<dyn Error>> {
    match UPDATE_DELTAS.lock() {
        Ok(guard) => Ok(guard.clone()),
        Err(_) => Err(String::from("update delta recorder poisoned").into()),
    }
}

#[test]
fn frame_loop_fixed_step_catch_up_is_clamped_by_maximum_delta_time() -> Result<(), Box<dyn Error>> {
    let _run_guard = match FRAME_LOOP_RUN_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    if let Ok(mut steps) = FIXED_STEPS.lock() {
        *steps = 0;
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_callbacks(frame_callbacks());
    runtime.tick(0.0)?;
    runtime.tick(TEN_SECOND_FRAME)?;
    let steps = fixed_step_count()?;
    let fixed_step = runtime.fixed_timestep();
    let expected = (DEFAULT_MAXIMUM_DELTA_TIME / fixed_step).floor() as usize;
    assert_eq!(steps, expected);
    assert_eq!(steps, 16);
    assert_ne!(steps, FIXED_STEPS_IN_TEN_SECONDS);
    Ok(())
}

#[test]
fn frame_loop_zero_time_scale_runs_no_fixed_steps_and_reports_a_zero_scaled_delta()
-> Result<(), Box<dyn Error>> {
    let _run_guard = match FRAME_LOOP_RUN_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(String::from("mutex poisoned").into()),
    };
    if let Ok(mut steps) = FIXED_STEPS.lock() {
        *steps = 0;
    }
    if let Ok(mut deltas) = UPDATE_DELTAS.lock() {
        deltas.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    runtime.set_time_scale(0.0);
    runtime.set_callbacks(frame_callbacks());
    runtime.tick(0.0)?;
    runtime.tick(0.5)?;
    assert_eq!(fixed_step_count()?, 0);
    let deltas = update_deltas()?;
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0], 0.0);
    Ok(())
}

#[test]
fn frame_loop_runtime_maximum_delta_time_defaults_to_the_unity_value_and_rejects_invalid_input()
-> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    assert_eq!(runtime.maximum_delta_time(), DEFAULT_MAXIMUM_DELTA_TIME);
    runtime.set_maximum_delta_time(0.25);
    assert_eq!(runtime.maximum_delta_time(), 0.25);
    runtime.set_maximum_delta_time(0.0);
    assert_eq!(runtime.maximum_delta_time(), DEFAULT_MAXIMUM_DELTA_TIME);
    runtime.set_maximum_delta_time(-1.0);
    assert_eq!(runtime.maximum_delta_time(), DEFAULT_MAXIMUM_DELTA_TIME);
    runtime.set_maximum_delta_time(f64::NAN);
    assert_eq!(runtime.maximum_delta_time(), DEFAULT_MAXIMUM_DELTA_TIME);
    Ok(())
}

extern "C" fn probe_create(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn probe_collision(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn probe_trigger(_a: u64, _b: u64, _phase: u32) {}

extern "C" fn probe_late_update(_handle: u64, _delta: f64) {
    let Ok(mut guard) = ANIMATOR_PROBE.lock() else {
        return;
    };
    let Some(probe) = guard.as_mut() else {
        return;
    };
    let address = unitas_runtime_path_buffer(probe.handle, STATE_INFO_SIZE as u64);
    if address == 0 {
        return;
    }
    if unitas_runtime_animator_state_info(probe.handle, probe.entity, address) != 0 {
        return;
    }
    let mut bytes = [0u8; STATE_INFO_SIZE];
    if test_read_path_buffer(probe.handle, &mut bytes).is_err() {
        return;
    }
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[NORMALIZED_TIME_OFFSET..NORMALIZED_TIME_OFFSET + 4]);
    probe.normalized_times.push(f32::from_le_bytes(raw));
}

fn setup_animator_probe_entity(handle: u64) -> Result<u64, Box<dyn Error>> {
    let controller = AnimatorController::new(
        "Probe",
        Vec::new(),
        vec![AnimatorState::new(
            "Idle",
            StateMotion::Clip(AnimationClip::new(
                "Idle",
                2.0,
                60.0,
                WrapMode::Loop,
                Vec::new(),
                Vec::new(),
            )),
        )],
        0,
        Vec::new(),
    );
    let controller_bytes = encode_controller_binary(&controller);
    let controller_length = u64::try_from(controller_bytes.len())?;
    let controller_address = unitas_runtime_path_buffer(handle, controller_length);
    if controller_address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, &controller_bytes)?;
    let controller_handle =
        unitas_runtime_animator_controller_create(handle, controller_address, controller_length);
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
    Ok(entity)
}

#[test]
fn frame_loop_animator_evaluation_precedes_the_late_update_dispatch() -> Result<(), Box<dyn Error>>
{
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let path_address = unitas_runtime_path_buffer(handle, 256);
    if path_address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    let path_bytes = SCENE_WITH_SCRIPT.as_bytes();
    test_write_path_buffer(handle, path_bytes)?;
    let path_length = u64::try_from(path_bytes.len())?;
    if unitas_runtime_load_scene(handle, path_address, path_length) != 0 {
        return Err(String::from("scene load failed").into());
    }
    let entity = setup_animator_probe_entity(handle)?;
    {
        let Ok(mut guard) = ANIMATOR_PROBE.lock() else {
            return Err(String::from("animator probe poisoned").into());
        };
        *guard = Some(AnimatorProbe {
            handle,
            entity,
            normalized_times: Vec::new(),
        });
    }
    let callback_code = unitas_runtime_set_callbacks(
        handle,
        probe_create,
        frame_noop_u64,
        frame_noop_u64,
        frame_noop_u64,
        frame_noop_f64,
        frame_noop_f64,
        probe_late_update,
        frame_noop_u64,
        frame_noop_u64,
        probe_collision,
        probe_trigger,
    );
    if callback_code != 0 {
        return Err(format!("callback registration failed with code {callback_code}").into());
    }
    if unitas_runtime_tick(handle, 0.0) != 0 || unitas_runtime_tick(handle, 0.5) != 0 {
        return Err(String::from("tick failed").into());
    }
    let normalized_times = match ANIMATOR_PROBE.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(probe) => probe.normalized_times.clone(),
            None => return Err(String::from("animator probe missing").into()),
        },
        Err(_) => return Err(String::from("animator probe poisoned").into()),
    };
    assert_eq!(normalized_times.len(), 1);
    assert!(
        normalized_times[0] > 0.0,
        "the animator must be evaluated before the LateUpdate dispatch"
    );
    Ok(())
}

#[test]
fn frame_loop_maximum_delta_time_abi_round_trips_the_configured_value() -> Result<(), Box<dyn Error>>
{
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    assert_eq!(
        unitas_runtime_maximum_delta_time(handle),
        DEFAULT_MAXIMUM_DELTA_TIME
    );
    assert_eq!(unitas_runtime_set_maximum_delta_time(handle, 0.1), 0);
    assert_eq!(unitas_runtime_maximum_delta_time(handle), 0.1);
    assert_eq!(unitas_runtime_set_maximum_delta_time(handle, 0.0), 0);
    assert_eq!(
        unitas_runtime_maximum_delta_time(handle),
        DEFAULT_MAXIMUM_DELTA_TIME
    );
    assert_eq!(unitas_runtime_set_maximum_delta_time(handle, -1.0), 0);
    assert_eq!(
        unitas_runtime_maximum_delta_time(handle),
        DEFAULT_MAXIMUM_DELTA_TIME
    );
    assert_eq!(unitas_runtime_set_maximum_delta_time(handle, f64::NAN), 0);
    assert_eq!(
        unitas_runtime_maximum_delta_time(handle),
        DEFAULT_MAXIMUM_DELTA_TIME
    );
    Ok(())
}

fn build_parallel_animation_controller(seed: usize) -> AnimatorController {
    let offset = (seed as f32) * 0.25 + 1.0;
    let speed_factor = 1.0 + ((seed % 7) as f32) * 0.15;
    let clip = AnimationClip::new(
        format!("Move_{seed}"),
        2.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, offset),
                    Keyframe::linear(1.0, offset + 3.5 * speed_factor),
                    Keyframe::linear(2.0, offset - 1.25 * speed_factor),
                ]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.y"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, -offset),
                    Keyframe::linear(2.0, offset * 2.0),
                ]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localScale.z"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 1.0),
                    Keyframe::linear(2.0, 1.0 + speed_factor * 0.5),
                ]),
            ),
        ],
        Vec::new(),
    );
    AnimatorController::new(
        format!("Controller_{seed}"),
        Vec::new(),
        vec![AnimatorState::with_speed(
            "Run",
            StateMotion::Clip(clip),
            speed_factor,
        )],
        0,
        Vec::new(),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameAnimatorSnapshot {
    entity: u64,
    translation_bits: [u32; 3],
    rotation_bits: [u32; 4],
    scale_bits: [u32; 3],
    normalized_time_bits: u32,
}

fn spawn_animated_entities(
    runtime: &mut Runtime,
    entity_count: usize,
) -> Result<Vec<EntityId>, Box<dyn Error>> {
    let mut entities = Vec::with_capacity(entity_count);
    for index in 0..entity_count {
        let entity = {
            let graph = runtime
                .scene_graph_mut()
                .ok_or_else(|| String::from("active scene graph missing"))?;
            let id = graph.add_entity(format!("Animated_{index}"));
            if let Some(transform) = graph.transform_mut(id) {
                transform.translation = [index as f32 * 0.1, index as f32 * -0.05, 0.5];
            }
            id
        };
        let ensured = runtime.ensure_animator(entity.as_u64());
        let controller = build_parallel_animation_controller(index);
        let handle = runtime.animation_mut().register_controller(controller);
        runtime.set_animator_controller(ensured, handle)?;
        entities.push(ensured);
    }
    Ok(entities)
}

fn run_animated_frame_loop_with_workers(
    workers: usize,
    entity_count: usize,
    ticks: usize,
) -> Result<(Runtime, Vec<FrameAnimatorSnapshot>), Box<dyn Error>> {
    let mut runtime = Runtime::with_job_system(std::sync::Arc::new(JobSystem::new(workers)))?;
    runtime.load_scene(Path::new(SCENE_WITH_SCRIPT))?;
    let entities = spawn_animated_entities(&mut runtime, entity_count)?;
    runtime.tick(0.0)?;
    let mut snapshots = Vec::with_capacity(ticks * entity_count);
    for step in 0..ticks {
        let delta = 1.0 / 60.0 + (step % 3) as f64 * 0.0025;
        runtime.tick(delta)?;
        for entity in &entities {
            let transform = runtime
                .animation()
                .transform(*entity)
                .cloned()
                .unwrap_or_else(Transform::default);
            let graph_transform = runtime
                .scene_graph()
                .and_then(|graph| graph.transform(*entity).cloned())
                .unwrap_or_else(Transform::default);
            assert_eq!(
                transform.translation.map(f32::to_bits),
                graph_transform.translation.map(f32::to_bits)
            );
            let state = runtime.animation().state_info(*entity)?;
            snapshots.push(FrameAnimatorSnapshot {
                entity: entity.as_u64(),
                translation_bits: transform.translation.map(f32::to_bits),
                rotation_bits: transform.rotation.map(f32::to_bits),
                scale_bits: transform.scale.map(f32::to_bits),
                normalized_time_bits: state.normalized_time.to_bits(),
            });
        }
    }
    Ok((runtime, snapshots))
}

fn run_serial_animation_world(
    entity_count: usize,
    ticks: usize,
) -> Result<unitas_runtime::AnimationWorld, Box<dyn Error>> {
    let mut world = unitas_runtime::AnimationWorld::new();
    for index in 0..entity_count {
        let entity = EntityId::new((index + 1) as u64);
        let initial = Transform {
            translation: [index as f32 * 0.1, index as f32 * -0.05, 0.5],
            ..Transform::default()
        };
        world.ensure_animator(entity, initial);
        let handle = world.register_controller(build_parallel_animation_controller(index));
        world.set_controller(entity, handle)?;
    }
    for step in 0..ticks {
        let delta = 1.0 / 60.0 + (step % 3) as f64 * 0.0025;
        world.tick(delta);
    }
    Ok(world)
}

#[test]
fn frame_loop_parallel_animation_slice_dispatches_across_multiple_workers_with_bit_identical_outputs()
-> Result<(), Box<dyn Error>> {
    let entity_count = 64;
    let ticks = 12;
    let (single_runtime, single_worker_outputs) =
        run_animated_frame_loop_with_workers(1, entity_count, ticks)?;
    let (multi_runtime, multi_worker_outputs) =
        run_animated_frame_loop_with_workers(4, entity_count, ticks)?;

    assert_eq!(single_runtime.last_animation_job_workers(), 1);
    assert!(
        multi_runtime.last_animation_job_workers() > 1,
        "expected parallel animation slice to execute across >1 worker threads, observed {}",
        multi_runtime.last_animation_job_workers()
    );
    assert_eq!(single_worker_outputs.len(), ticks * entity_count);
    assert_eq!(single_worker_outputs, multi_worker_outputs);
    let serial_world = run_serial_animation_world(entity_count, ticks)?;
    for index in 0..entity_count {
        let serial_entity = EntityId::new((index + 1) as u64);
        let serial_transform = serial_world
            .transform(serial_entity)
            .ok_or("serial transform missing")?;
        let snapshot = multi_worker_outputs[(ticks - 1) * entity_count + index];
        assert_eq!(
            serial_transform.translation.map(f32::to_bits),
            snapshot.translation_bits
        );
        assert_eq!(
            serial_transform.rotation.map(f32::to_bits),
            snapshot.rotation_bits
        );
        assert_eq!(
            serial_transform.scale.map(f32::to_bits),
            snapshot.scale_bits
        );
        assert_eq!(
            serial_world
                .state_info(serial_entity)?
                .normalized_time
                .to_bits(),
            snapshot.normalized_time_bits
        );
    }
    Ok(())
}
