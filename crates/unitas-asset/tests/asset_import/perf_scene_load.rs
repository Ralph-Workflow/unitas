use std::error::Error;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use unitas_asset::scene::assemble_scene_file_bundle;

type TestResult = Result<(), Box<dyn Error>>;

const LOAD_PASSES: u32 = 20;
const SCENE_LOAD_CEILING_MS: f64 = 24.0;

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_scene_load_assembles_the_roll_a_ball_scene_within_budget() -> TestResult {
    let scene_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/roll_a_ball_project/Assets/Scenes/Main.unity");
    let started = Instant::now();
    for _ in 0..LOAD_PASSES {
        black_box(assemble_scene_file_bundle(&scene_path)?);
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(LOAD_PASSES);
    println!("measurement=scene_load value_ms={value_ms:.3} ceiling_ms={SCENE_LOAD_CEILING_MS:.3}");
    if value_ms > SCENE_LOAD_CEILING_MS {
        return Err(format!(
            "scene_load measured {value_ms:.3}ms against the {SCENE_LOAD_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
