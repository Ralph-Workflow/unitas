use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_animation::{
    AnimationClip, AnimatorController, AnimatorState, StateMotion, WrapMode,
    encode_controller_binary,
};
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
// The counter tests share FIXED_STEPS and UPDATE_DELTAS, so they run one at a time.
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
    if unitas_runtime_tick(handle, 0.0) != 0 {
        return Err(String::from("initialization tick failed").into());
    }
    if unitas_runtime_tick(handle, 0.5) != 0 {
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
