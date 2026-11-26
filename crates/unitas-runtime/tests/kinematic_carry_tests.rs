use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_core::{EntityId, SceneGraph};
use unitas_runtime::Runtime;

type TestResult = Result<(), Box<dyn Error>>;

const FRAME_DELTA: f64 = 1.0 / 60.0;
const FRAMES: usize = 150;
const ROTATE_DEGREES_PER_SECOND: f64 = 60.0;
const BOB_HEIGHT: f64 = 0.2;
const BOB_SPEED: f64 = 2.0;

fn scene_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("roll-a-ball")
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

#[test]
fn diagnostic_kinematic_carry() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path())?;
    let (platform, marble) = {
        let graph = runtime
            .scene_graph()
            .ok_or("runtime exposes no scene graph after load_scene")?;
        (
            entity_named(graph, "RotatingPlatform")?,
            entity_named(graph, "PlatformMarble")?,
        )
    };
    println!(
        "platform pos {:?} rot {:?}",
        runtime.body_position(platform),
        runtime.physics_mut().world_3d().rotation(platform)
    );
    println!("marble pos {:?}", runtime.body_position(marble),);
    for frame in 1..=FRAMES {
        let time = FRAME_DELTA * frame as f64;
        let angle = ROTATE_DEGREES_PER_SECOND * FRAME_DELTA * frame as f64;
        let half = (angle.to_radians() * 0.5).sin();
        let cos = (angle.to_radians() * 0.5).cos();
        {
            let graph = runtime
                .scene_graph_mut()
                .ok_or("runtime exposes no mutable scene graph")?;
            let transform = graph
                .transform_mut(platform)
                .ok_or("platform has no transform")?;
            transform.rotation = [0.0, half as f32, 0.0, cos as f32];
            transform.translation[1] = 0.25 + (BOB_HEIGHT * (time * BOB_SPEED).sin()) as f32;
        }
        runtime.tick(FRAME_DELTA)?;
        if frame % 10 == 0 || frame < 4 {
            println!(
                "frame {frame} angle {angle:.2} marble pos {:?} vel {:?} sleeping {}",
                runtime.body_position(marble),
                runtime.physics_mut().world_3d().velocity(marble),
                runtime.physics_mut().world_3d().is_sleeping(marble),
            );
        }
    }
    println!(
        "FINAL platform {:?} marble {:?}",
        runtime.body_position(platform),
        runtime.body_position(marble)
    );
    Ok(())
}
