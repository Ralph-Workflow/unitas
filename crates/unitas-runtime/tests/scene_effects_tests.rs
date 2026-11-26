use std::error::Error;
use std::path::Path;

use unitas_core::EntityId;
use unitas_runtime::Runtime;

const EFFECTS_FIXTURE_PATH: &str = "tests/fixtures/scene_effects.unity";

fn load_effects() -> Result<(Runtime, EntityId), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(EFFECTS_FIXTURE_PATH))?;
    let graph = runtime.scene_graph().ok_or("the scene holds no graph")?;
    let entity = *graph
        .roots()
        .first()
        .ok_or("the scene holds no root entity")?;
    Ok((runtime, entity))
}

#[test]
fn loading_the_fixture_registers_the_scene_particle_system() -> Result<(), Box<dyn Error>> {
    let (mut runtime, entity) = load_effects()?;

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    let snapshot = runtime
        .particles_mut()
        .snapshot(entity)
        .ok_or("the scene particle system is missing")?;
    assert!(
        snapshot.live_particle_count > 0,
        "the registered particle system must hold live particles"
    );
    assert!(
        snapshot.emitting,
        "the registered particle system must emit like play on awake"
    );
    Ok(())
}

#[test]
fn loading_the_fixture_registers_the_scene_trail_renderer() -> Result<(), Box<dyn Error>> {
    let (mut runtime, entity) = load_effects()?;

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;
    runtime.tick(0.1)?;

    let points = runtime
        .trails()
        .points(entity)
        .ok_or("the scene trail renderer is missing")?;
    assert!(
        !points.is_empty(),
        "the registered trail must hold ribbon points"
    );
    let draw_list = runtime.trail_draw_list();
    assert!(
        !draw_list.is_empty(),
        "the trail draw list must expose the ribbon points"
    );
    assert!(
        draw_list.iter().all(|item| item.entity == entity),
        "every trail draw item must belong to the scene trail entity"
    );
    Ok(())
}

#[test]
fn the_particle_draw_list_fills_after_the_ticks() -> Result<(), Box<dyn Error>> {
    let (mut runtime, _) = load_effects()?;

    assert!(
        runtime.particle_draw_list().is_empty(),
        "the particle draw list must start empty"
    );

    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    assert!(
        !runtime.particle_draw_list().is_empty(),
        "the particle draw list must expose the live particles"
    );
    Ok(())
}

#[test]
fn loading_the_same_scene_twice_produces_the_same_effects() -> Result<(), Box<dyn Error>> {
    let (mut first, first_entity) = load_effects()?;
    let (mut second, second_entity) = load_effects()?;

    for _ in 0..3 {
        first.tick(0.05)?;
        second.tick(0.05)?;
    }

    assert_eq!(
        first.particle_draw_list(),
        second.particle_draw_list(),
        "the same scene must produce the same particle draw list"
    );
    assert_eq!(
        first.trails().points(first_entity),
        second.trails().points(second_entity),
        "the same scene must produce the same trail points"
    );
    assert_eq!(
        first.trail_draw_list(),
        second.trail_draw_list(),
        "the same scene must produce the same trail draw list"
    );
    Ok(())
}

#[test]
fn reloading_the_scene_reproduces_the_same_effects() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(EFFECTS_FIXTURE_PATH))?;
    runtime.tick(0.02)?;
    runtime.tick(0.1)?;
    let particles = particle_effects(&runtime);
    let trails = trail_effects(&runtime);

    runtime.load_scene(Path::new(EFFECTS_FIXTURE_PATH))?;
    runtime.tick(0.02)?;
    runtime.tick(0.1)?;

    assert_eq!(
        particles,
        particle_effects(&runtime),
        "the reloaded scene must produce the same particle draw list"
    );
    assert_eq!(
        trails,
        trail_effects(&runtime),
        "the reloaded scene must produce the same trail draw list"
    );
    Ok(())
}

fn particle_effects(runtime: &Runtime) -> Vec<([f32; 3], f32, [f32; 4])> {
    runtime
        .particle_draw_list()
        .iter()
        .map(|item| (item.position, item.size, item.color))
        .collect()
}

fn trail_effects(runtime: &Runtime) -> Vec<([f32; 3], f32, [f32; 4])> {
    runtime
        .trail_draw_list()
        .iter()
        .map(|item| (item.position, item.width, item.color))
        .collect()
}
