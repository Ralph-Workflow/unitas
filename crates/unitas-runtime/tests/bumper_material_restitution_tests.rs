use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_core::{EntityId, SceneGraph};
use unitas_runtime::Runtime;

type TestResult = Result<(), Box<dyn Error>>;

const BUMPER_RESTITUTION: f64 = 1.5;
const BUMPER_FRICTION: f64 = 0.2;
const LAUNCH_SPEED: f64 = 12.0;
const STEP_SECONDS: f64 = 1.0 / 240.0;
const MAX_STEPS: u32 = 960;

fn sample_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("marble-rush")
}

fn scene_path() -> PathBuf {
    sample_root()
        .join("Assets")
        .join("Scenes")
        .join("PhysicsLab.unity")
}

fn entity_named(graph: &SceneGraph, name: &str) -> Result<EntityId, Box<dyn Error>> {
    graph
        .iter_breadth_first()
        .find(|id| graph.entity(*id).is_some_and(|entity| entity.name == name))
        .ok_or_else(|| format!("scene has no entity named {name}").into())
}

fn load_lab() -> Result<Runtime, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path())?;
    Ok(runtime)
}

#[test]
fn the_bumper_material_reaches_the_physics_body() -> TestResult {
    let mut runtime = load_lab()?;
    let bumper = {
        let graph = runtime
            .scene_graph()
            .ok_or("runtime exposes no scene graph after load_scene")?;
        entity_named(graph, "Bumper")?
    };
    let material = runtime
        .physics_mut()
        .world_3d()
        .body_material(bumper)
        .ok_or("the bumper body carries no physics material")?;
    assert!(
        (material.restitution - BUMPER_RESTITUTION).abs() < 1e-9,
        "expected bumper restitution {BUMPER_RESTITUTION}, got {}",
        material.restitution
    );
    assert!(
        (material.friction - BUMPER_FRICTION).abs() < 1e-9,
        "expected bumper friction {BUMPER_FRICTION}, got {}",
        material.friction
    );
    Ok(())
}

#[test]
fn a_marble_launched_at_the_bumper_rebounds() -> TestResult {
    let mut runtime = load_lab()?;
    let marble = {
        let graph = runtime
            .scene_graph()
            .ok_or("runtime exposes no scene graph after load_scene")?;
        entity_named(graph, "BumperMarble")?
    };
    let world = runtime.physics_mut().world_3d_mut();
    world.set_velocity(marble, [0.0, 0.0, LAUNCH_SPEED])?;
    let mut outgoing = None;
    for _ in 0..MAX_STEPS {
        world.step(STEP_SECONDS)?;
        let velocity = world.velocity(marble).ok_or("the marble has no velocity")?;
        if velocity[2] < 0.0 {
            outgoing = Some(velocity);
            break;
        }
    }
    let outgoing = outgoing.ok_or("the marble never rebounded off the bumper")?;
    assert!(
        -outgoing[2] > LAUNCH_SPEED,
        "expected a rebound faster than the {LAUNCH_SPEED} m/s launch, got {outgoing:?}"
    );
    Ok(())
}
