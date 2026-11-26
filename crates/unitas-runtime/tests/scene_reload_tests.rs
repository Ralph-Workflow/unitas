use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

use unitas_runtime::{Callbacks, Runtime};

const FIXTURE_PATH: &str = "tests/fixtures/scene_with_script.unity";

static CREATED: Mutex<Vec<(u64, u64, u64)>> = Mutex::new(Vec::new());

extern "C" fn rec_create(entity: u64, hi: u64, lo: u64) -> u64 {
    if let Ok(mut created) = CREATED.lock() {
        created.push((entity, hi, lo));
    }
    entity.max(1)
}

extern "C" fn rec_noop_u64(_handle: u64) {}
extern "C" fn rec_noop_u64_f64(_handle: u64, _dt: f64) {}
extern "C" fn rec_collision(_a: u64, _b: u64, _phase: u32) {}
extern "C" fn rec_trigger(_a: u64, _b: u64, _phase: u32) {}

fn callbacks() -> Callbacks {
    Callbacks {
        create_script: rec_create,
        awake: rec_noop_u64,
        on_enable: rec_noop_u64,
        start: rec_noop_u64,
        fixed_update: rec_noop_u64_f64,
        update: rec_noop_u64_f64,
        late_update: rec_noop_u64_f64,
        on_disable: rec_noop_u64,
        destroy: rec_noop_u64,
        collision: rec_collision,
        trigger: rec_trigger,
    }
}

#[test]
fn probe_reload_creates_scripts_again() -> Result<(), Box<dyn Error>> {
    if let Ok(mut created) = CREATED.lock() {
        created.clear();
    }
    let mut runtime = Runtime::new()?;
    runtime.set_callbacks(callbacks());
    runtime.load_scene(Path::new(FIXTURE_PATH))?;
    runtime.tick(0.02)?;
    let after_first = CREATED.lock().map(|c| c.len()).unwrap_or(0);
    println!("after first load+tick: {after_first}");

    runtime.load_scene(Path::new(FIXTURE_PATH))?;
    let after_reload = CREATED.lock().map(|c| c.len()).unwrap_or(0);
    println!("after reload (no tick): {after_reload}");
    runtime.tick(0.02)?;
    let after_reload_tick = CREATED.lock().map(|c| c.len()).unwrap_or(0);
    println!("after reload+tick: {after_reload_tick}");
    runtime.tick(0.02)?;
    let after_two_ticks = CREATED.lock().map(|c| c.len()).unwrap_or(0);
    println!("after two ticks: {after_two_ticks}");

    let created = CREATED.lock().map(|c| c.clone()).unwrap_or_default();
    println!("created: {created:?}");
    println!("scripts: {:?}", runtime.scripts());
    Ok(())
}
