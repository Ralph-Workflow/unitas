use std::error::Error;
use std::path::Path;
use std::time::Instant;
use unitas_core::{EntityId, Transform};
use unitas_runtime::Runtime;

type TestResult = Result<(), Box<dyn Error>>;

const SCENE_FIXTURE: &str = "tests/fixtures/scene_with_script.unity";
const GAME_OBJECT_COUNT: usize = 100_000;
const WARMUP_FRAMES: u32 = 3;
const MEASURED_FRAMES: u32 = 10;
const FRAME_DELTA_SECONDS: f64 = 1.0 / 60.0;
const FRAME_CEILING_MS: f64 = 176.0;

fn build_hundred_thousand_game_objects(runtime: &mut Runtime) -> TestResult {
    let entities: Vec<EntityId> = {
        let graph = runtime
            .scene_graph_mut()
            .ok_or_else(|| String::from("the runtime has no active scene graph"))?;
        let mut entities = Vec::with_capacity(GAME_OBJECT_COUNT);
        for index in 0..GAME_OBJECT_COUNT {
            entities.push(graph.add_entity(format!("GameObject_{index}")));
        }
        entities
    };
    let transforms: Vec<Transform> = {
        let graph = runtime
            .scene_graph()
            .ok_or_else(|| String::from("the runtime has no active scene graph"))?;
        entities
            .iter()
            .map(|entity| graph.transform(*entity).cloned().unwrap_or_default())
            .collect()
    };
    let animation = runtime.animation_mut();
    for (entity, transform) in entities.iter().zip(transforms) {
        animation.ensure_animator(*entity, transform);
    }
    Ok(())
}

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_frame_loop_ticks_a_hundred_thousand_game_object_scene_within_frame_budget() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(SCENE_FIXTURE))?;
    build_hundred_thousand_game_objects(&mut runtime)?;
    let scene_entities = runtime.scene_graph().map_or(0, |graph| graph.len());
    let animated_entities = runtime.animation().entities().len();
    if scene_entities < GAME_OBJECT_COUNT || animated_entities < GAME_OBJECT_COUNT {
        return Err(format!(
            "frame_loop_100k scene holds {scene_entities} entities and {animated_entities} \
             animators, short of the {GAME_OBJECT_COUNT} GameObjects under measurement"
        )
        .into());
    }
    println!(
        "frame_loop_100k scene: {scene_entities} GameObjects, each with an Animator component"
    );
    for _ in 0..WARMUP_FRAMES {
        runtime.tick(FRAME_DELTA_SECONDS)?;
    }
    let started = Instant::now();
    for _ in 0..MEASURED_FRAMES {
        runtime.tick(FRAME_DELTA_SECONDS)?;
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(MEASURED_FRAMES);
    println!("measurement=frame_loop_100k value_ms={value_ms:.3} ceiling_ms={FRAME_CEILING_MS:.3}");
    if value_ms > FRAME_CEILING_MS {
        return Err(format!(
            "frame_loop_100k measured {value_ms:.3}ms against the {FRAME_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
