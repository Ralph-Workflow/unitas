use std::error::Error;
use std::path::Path;

use unitas_core::EntityId;
use unitas_runtime::Runtime;

const ALPHA_FIXTURE_PATH: &str = "tests/fixtures/scene_alpha.unity";
const FRAME_SECONDS: f64 = 0.016;
const FRAME_COUNT: usize = 120;

#[test]
fn a_marked_entity_leaves_the_scene_graph() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    let entity = match runtime
        .scene_graph()
        .and_then(|graph| graph.roots().first().copied())
    {
        Some(entity) => entity,
        None => return Err(String::from("the scene has no root entity").into()),
    };
    assert!(
        runtime
            .scene_graph()
            .is_some_and(|graph| graph.contains(entity))
    );

    runtime.mark_destroyed(entity)?;

    assert!(
        runtime
            .scene_graph()
            .is_some_and(|graph| !graph.contains(entity))
    );
    Ok(())
}

#[test]
fn marking_an_entity_the_scene_does_not_own_is_reported() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;

    assert!(runtime.mark_destroyed(EntityId::new(4_242)).is_err());
    Ok(())
}

#[test]
fn a_tick_empties_the_scene_change_log() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;

    runtime.tick(FRAME_SECONDS)?;

    let changes = match runtime.scene_graph_mut() {
        Some(graph) => graph.take_changes(),
        None => return Err(String::from("no scene graph found").into()),
    };
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn the_change_log_stays_empty_across_many_ticks() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;

    for _ in 0..FRAME_COUNT {
        runtime.tick(FRAME_SECONDS)?;
    }

    let changes = match runtime.scene_graph_mut() {
        Some(graph) => graph.take_changes(),
        None => return Err(String::from("no scene graph found").into()),
    };
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn a_tick_sweeps_the_tombstones_of_destroyed_entities() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(Path::new(ALPHA_FIXTURE_PATH))?;
    let entity = match runtime
        .scene_graph()
        .and_then(|graph| graph.roots().first().copied())
    {
        Some(entity) => entity,
        None => return Err(String::from("the scene has no root entity").into()),
    };

    runtime.mark_destroyed(entity)?;
    runtime.tick(FRAME_SECONDS)?;

    let roots = match runtime.scene_graph() {
        Some(graph) => graph.roots(),
        None => return Err(String::from("no scene graph found").into()),
    };
    assert!(roots.is_empty());
    Ok(())
}
