//! Plan U-2: a scene-authored NavMeshSurface and Terrain reach the live
//! runtime worlds through `Runtime::load_scene`.
//!
//! Fixture: `tests/fixtures/project_nav_terrain/Assets/Scenes/NavTerrain.unity`
//! - Floor: builtin Unity Plane mesh (fileID 10209) + NavMeshSurface
//!   (script guid 7a5ac11cc976e418e8d13136b07e1f52) at the origin.
//! - Runner: NavMeshAgent at (1, 0, 1) with speed 4.25, radius 0.4,
//!   avoidancePriority 35, stoppingDistance 0.2, height 1.8.
//! - GroundTerrain: Terrain referencing MeadowData.asset
//!   (guid 90000000000000000000000000000099), heightmap resolution 3,
//!   size (30, 40, 30), authored normalized heights below.

use std::error::Error;
use std::path::PathBuf;

use unitas_core::EntityId;
use unitas_runtime::Runtime;

const SCENE_RELATIVE: &str = "tests/fixtures/project_nav_terrain/Assets/Scenes/NavTerrain.unity";
const RUNNER_NAME: &str = "Runner";
const TERRAIN_NAME: &str = "GroundTerrain";

const AGENT_POSITION: [f64; 3] = [1.0, 0.0, 1.0];
const AGENT_SPEED: f64 = 4.25;
const AGENT_RADIUS: f64 = 0.4;
const AGENT_AVOIDANCE_PRIORITY: u32 = 35;
const AGENT_STOPPING_DISTANCE: f64 = 0.2;
const AGENT_HEIGHT: f64 = 1.8;
const AGENT_ACCELERATION: f64 = 6.0;

// Authored MeadowData.asset m_Heights (row-major, resolution 3).
const TERRAIN_RESOLUTION: usize = 3;
const TERRAIN_SIZE: [f64; 3] = [30.0, 40.0, 30.0];
const TERRAIN_CENTER: [f64; 3] = [15.0, 0.0, 15.0];
const TERRAIN_HEIGHTS: [f32; 9] = [0.0, 0.25, 0.5, 0.25, 0.5, 0.75, 0.5, 0.75, 1.0];

fn fixture_scene_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SCENE_RELATIVE)
}

fn entity_by_name(runtime: &Runtime, name: &str) -> Result<EntityId, Box<dyn Error>> {
    let graph = runtime
        .scene_graph()
        .ok_or_else(|| String::from("no scene graph loaded"))?;
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(id);
        }
    }
    Err(format!("entity {name} not found in scene graph").into())
}

#[test]
fn navmesh_surface_triangles_reach_the_navigation_world() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&fixture_scene_path())?;

    // The Unity Plane primitive is a 10x10 XZ quad at the origin: two
    // walkable triangles with corners at (±5, 0, ±5).
    let triangles = runtime.navigation_mut().surface_triangles();
    assert_eq!(
        triangles.len(),
        2,
        "expected the two authored plane triangles"
    );
    let mut corners: Vec<[f64; 3]> = triangles.iter().flat_map(|t| t.vertices).collect();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    corners.dedup();
    assert_eq!(corners.len(), 4);
    for corner in corners {
        assert!(
            corner[1].abs() <= 1e-9,
            "plane triangle corner must sit on y = 0, got {corner:?}"
        );
        assert!(
            corner[0].abs() <= 5.0 + 1e-9 && corner[2].abs() <= 5.0 + 1e-9,
            "plane triangle corner must be within the authored plane extents, got {corner:?}"
        );
    }
    assert!(
        runtime
            .navigation_mut()
            .navmesh()
            .polygon_at([0.0, 0.0, 0.0])
            .is_some(),
        "the surface triangles must rebuild the runtime navmesh"
    );
    Ok(())
}

#[test]
fn navmesh_agent_reaches_the_navigation_world_with_authored_settings() -> Result<(), Box<dyn Error>>
{
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&fixture_scene_path())?;

    let agent = entity_by_name(&runtime, RUNNER_NAME)?;
    let navigation = runtime.navigation_mut();
    assert!(
        navigation.has_agent(agent),
        "scene agent missing from the navigation world"
    );
    let agent = navigation.agent(agent).ok_or("agent lookup failed")?;
    assert_eq!(agent.speed, AGENT_SPEED);
    assert_eq!(agent.radius, AGENT_RADIUS);
    assert_eq!(agent.avoidance_priority, AGENT_AVOIDANCE_PRIORITY);
    assert_eq!(agent.position(), AGENT_POSITION);
    let props = navigation
        .agent_props(agent.entity())
        .ok_or("agent extra props missing")?;
    assert_eq!(props.stopping_distance, AGENT_STOPPING_DISTANCE);
    assert_eq!(props.height, AGENT_HEIGHT);
    assert_eq!(props.acceleration, AGENT_ACCELERATION);
    Ok(())
}

#[test]
fn terrain_reaches_the_terrain_world_with_authored_heights() -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&fixture_scene_path())?;

    let terrain = entity_by_name(&runtime, TERRAIN_NAME)?;
    let world = runtime.terrain();
    assert!(
        world.has_terrain(terrain),
        "scene terrain missing from the terrain world"
    );
    assert_eq!(
        world.resolution(terrain),
        Some([TERRAIN_RESOLUTION, TERRAIN_RESOLUTION])
    );
    assert_eq!(world.size(terrain), Some(TERRAIN_SIZE));
    // The runtime registers the terrain at the world-space center of its footprint.
    assert_eq!(world.position(terrain), Some(TERRAIN_CENTER));

    let heights = world
        .get_heights(terrain, 0, 0, TERRAIN_RESOLUTION, TERRAIN_RESOLUTION)
        .ok_or("terrain heights missing")?;
    assert_eq!(heights.len(), TERRAIN_HEIGHTS.len());
    for (actual, expected) in heights.iter().zip(TERRAIN_HEIGHTS.iter()) {
        assert!(
            (f64::from(*actual) - f64::from(*expected)).abs() < 1e-6,
            "authored height {expected} was not preserved, got {actual}"
        );
    }
    Ok(())
}
