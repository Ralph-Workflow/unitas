use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_core::{EntityId, SceneGraph};
use unitas_physics::BodyRole;
use unitas_runtime::Runtime;

type TestResult = Result<(), Box<dyn Error>>;

const FRAME_DELTA: f64 = 1.0 / 60.0;
const FRAMES: usize = 150;
const ROTATE_DEGREES_PER_SECOND: f64 = 60.0;
const BOB_HEIGHT: f64 = 0.2;
const BOB_SPEED: f64 = 2.0;
const SETTLE_FRAMES: usize = 30;
const SWEEP_SPEED: f64 = 2.0;
const SWEEP_FRAMES: usize = 90;
const CARRY_DISTANCE: f64 = 0.5;
const PUSH_DISTANCE: f64 = 1.0;
const REST_TOLERANCE: f64 = 0.05;
const REST_SPEED: f64 = 0.05;

fn scene_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples")
        .join("marble-rush")
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

fn body_position(runtime: &Runtime, entity: EntityId) -> Result<[f64; 3], Box<dyn Error>> {
    match runtime.body_position(entity) {
        Some(position) => Ok(position),
        None => Err(String::from("the entity has no physics body").into()),
    }
}

fn horizontal_distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    let dx = first[0] - second[0];
    let dz = first[2] - second[2];
    (dx * dx + dz * dz).sqrt()
}

fn speed(runtime: &Runtime, entity: EntityId) -> Result<f64, Box<dyn Error>> {
    let velocity = match runtime.body_velocity(entity) {
        Some(velocity) => velocity,
        None => return Err(String::from("the entity has no physics body").into()),
    };
    Ok((velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]).sqrt())
}

fn step_rotating_platform(runtime: &mut Runtime, platform: EntityId, frame: usize) -> TestResult {
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
    Ok(())
}

#[test]
fn rotating_platform_carries_the_resting_marble() -> TestResult {
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
    assert_eq!(runtime.body_role(platform), Some(BodyRole::Kinematic));
    assert_eq!(runtime.body_role(marble), Some(BodyRole::Dynamic));

    for _ in 0..SETTLE_FRAMES {
        runtime.tick(FRAME_DELTA)?;
    }
    let start = body_position(&runtime, marble)?;
    let platform_start = body_position(&runtime, platform)?;

    for frame in 1..=FRAMES {
        step_rotating_platform(&mut runtime, platform, frame)?;
    }

    let end = body_position(&runtime, marble)?;
    let platform_end = body_position(&runtime, platform)?;
    let carried = horizontal_distance(start, end);
    assert!(
        carried >= CARRY_DISTANCE,
        "the rotating platform must carry the resting marble, it moved {carried} m"
    );
    assert!(
        end[2] < start[2],
        "the platform turns about +Y, so the marble must be carried toward -z, it moved from \
         {start:?} to {end:?}"
    );
    let ride_start = start[1] - platform_start[1];
    let ride_end = end[1] - platform_end[1];
    assert!(
        (ride_end - ride_start).abs() <= REST_TOLERANCE,
        "the marble must keep riding the platform at the same height, it rode {ride_start} m and \
         now rides {ride_end} m"
    );
    Ok(())
}

#[test]
fn kinematic_sweeper_pushes_the_resting_marble_through_contact() -> TestResult {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path())?;
    let (sweeper, marble) = {
        let graph = runtime
            .scene_graph()
            .ok_or("runtime exposes no scene graph after load_scene")?;
        (
            entity_named(graph, "Pusher")?,
            entity_named(graph, "PushedMarble")?,
        )
    };
    assert_eq!(runtime.body_role(sweeper), Some(BodyRole::Kinematic));
    assert_eq!(runtime.body_role(marble), Some(BodyRole::Dynamic));

    for _ in 0..SETTLE_FRAMES {
        runtime.tick(FRAME_DELTA)?;
    }
    let start = body_position(&runtime, marble)?;
    let rest_speed = speed(&runtime, marble)?;
    assert!(
        rest_speed <= REST_SPEED,
        "the marble must rest before the sweep, it moves at {rest_speed} m/s"
    );

    let mut target = body_position(&runtime, sweeper)?;
    for _ in 0..SWEEP_FRAMES {
        target[2] += SWEEP_SPEED * FRAME_DELTA;
        runtime.move_body_position(sweeper, target)?;
        runtime.tick(FRAME_DELTA)?;
    }

    let end = body_position(&runtime, marble)?;
    let pushed = end[2] - start[2];
    assert!(
        pushed >= PUSH_DISTANCE,
        "the kinematic sweeper must push the resting marble through contact, it moved {pushed} m"
    );
    assert!(
        (end[1] - start[1]).abs() <= REST_TOLERANCE,
        "the pushed marble must stay resting on the floor, it rose from y {} to y {}",
        start[1],
        end[1]
    );
    Ok(())
}
