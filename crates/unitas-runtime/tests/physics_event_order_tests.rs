use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_core::EntityId;
use unitas_runtime::{Callbacks, Runtime};

type TestResult = Result<(), Box<dyn Error>>;

const TRIGGER_FIXTURE_PATH: &str = "tests/fixtures/scene_physics_trigger.unity";
const BALL_NAME: &str = "Ball";
// The ball starts above the trigger, so the first trigger message arrives after
// the first fixed step: the 30 fps tick then batches two fixed steps into one
// rendered frame, the 60 fps tick never does.
const BALL_START_HEIGHT: f64 = 1.2;
const SIMULATED_SECONDS: f64 = 1.0;

static TRACE: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static RUN_LOCK: Mutex<()> = Mutex::new(());

extern "C" fn noop_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn noop_handle(_handle: u64) {}

extern "C" fn noop_handle_delta(_handle: u64, _delta: f64) {}

extern "C" fn record_fixed_update(_handle: u64, _delta: f64) {
    record("F");
}

extern "C" fn record_collision(_a: u64, _b: u64, _phase: u32) {
    record("C");
}

extern "C" fn record_trigger(_a: u64, _b: u64, _phase: u32) {
    record("T");
}

fn record(entry: &'static str) {
    if let Ok(mut trace) = TRACE.lock() {
        trace.push(entry);
    }
}

fn callbacks() -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: record_fixed_update,
        update: noop_handle_delta,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision: record_collision,
        trigger: record_trigger,
    }
}

fn entity_id(runtime: &Runtime, name: &str) -> Result<EntityId, Box<dyn Error>> {
    let graph = runtime
        .scene_graph()
        .ok_or("runtime exposes no scene graph after load_scene")?;
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(entity.id);
        }
    }

    Err(format!("scene has no entity named {name}").into())
}

// The scripted path is the same at every rate: the ball falls into the trigger
// and the trace records the fixed steps and the physics messages they produce.
fn run_trace(rate: f64) -> Result<String, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(TRIGGER_FIXTURE_PATH))?;
    let ball = entity_id(&runtime, BALL_NAME)?;
    runtime.teleport_body_position(ball, [0.0, BALL_START_HEIGHT, 0.0])?;
    runtime.set_callbacks(callbacks());
    TRACE.lock()?.clear();
    // The first tick only initializes the scripts, so every rate needs one extra
    // tick to simulate the same amount of time.
    let ticks = (SIMULATED_SECONDS * rate).round() as usize + 1;
    for _ in 0..ticks {
        runtime.tick(1.0 / rate)?;
    }

    let trace = TRACE.lock()?.clone();
    Ok(trace.join(""))
}

#[test]
fn the_physics_message_order_is_the_same_at_every_frame_rate() -> TestResult {
    let _guard = RUN_LOCK.lock()?;
    let at_30 = run_trace(30.0)?;
    let at_60 = run_trace(60.0)?;
    assert!(
        at_30.contains('T'),
        "the 30 fps run produced no trigger message: {at_30}"
    );
    assert!(
        at_60.contains('T'),
        "the 60 fps run produced no trigger message: {at_60}"
    );
    assert_eq!(
        at_30, at_60,
        "the physics message order differs between 30 fps and 60 fps"
    );
    Ok(())
}
