use std::error::Error;

use unitas_core::EntityId;
use unitas_physics::navigation::NavMeshTriangle;
use unitas_runtime::Runtime;

const TICK_DELTA: f64 = 0.1;
const SIM_FRAMES: usize = 12;
const AGENT_SPEED: f64 = 3.5;
const PATH_LENGTH: f64 = 3.0;
const START_X: f64 = 0.5;
const DESTINATION: [f64; 3] = [3.5, 0.0, 0.5];
const TOLERANCE: f64 = 1e-3;

const WALKER_SCENE: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Walker\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0.5, y: 0, z: 0.5}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n";

fn cell(origin: [f64; 3], size: f64, area: u32) -> Vec<NavMeshTriangle> {
    let (x, y, z) = (origin[0], origin[1], origin[2]);
    vec![
        NavMeshTriangle::new([[x, y, z], [x, y, z + size], [x + size, y, z]], area),
        NavMeshTriangle::new(
            [[x + size, y, z], [x, y, z + size], [x + size, y, z + size]],
            area,
        ),
    ]
}

fn row(origin: [f64; 3], cells: usize, size: f64, area: u32) -> Vec<NavMeshTriangle> {
    let mut triangles = Vec::new();
    for index in 0..cells {
        let x = origin[0] + index as f64 * size;
        triangles.extend(cell([x, origin[1], origin[2]], size, area));
    }
    triangles
}

fn load_temp_scene(name: &str, yaml: &str) -> Result<Runtime, Box<dyn Error>> {
    let temp_dir =
        std::env::temp_dir().join(format!("unitas_runtime_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;
    let temp_scene = temp_dir.join(format!("{name}.unity"));
    std::fs::write(&temp_scene, yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&temp_scene);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;
    Ok(runtime)
}

fn entity_handle(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = runtime
        .scene_graph()
        .ok_or_else(|| String::from("no scene graph loaded"))?;
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(entity.id.as_u64());
        }
    }
    Err(String::from("entity not found in scene graph").into())
}

fn walker_x(runtime: &Runtime, agent: EntityId) -> Result<f64, Box<dyn Error>> {
    let graph = runtime
        .scene_graph()
        .ok_or_else(|| String::from("no scene graph loaded"))?;
    let transform = graph
        .world_transform(agent)
        .ok_or_else(|| String::from("walker has no world transform"))?;
    Ok(f64::from(transform.translation[0]))
}

#[test]
fn nav_001_tick_writes_agent_movement_back_to_the_scene_transform(
) -> Result<(), Box<dyn Error>> {
    let mut runtime = load_temp_scene("nav_writeback", WALKER_SCENE)?;
    let agent = EntityId::new(entity_handle(&runtime, "Walker")?);

    runtime
        .navigation_mut()
        .append_surface_triangles(&row([0.0, 0.0, 0.0], 4, 1.0, 0))?;
    runtime
        .navigation_mut()
        .add_agent(agent, "Walker", [START_X, 0.0, 0.5])?;
    runtime.navigation_mut().set_destination(agent, DESTINATION)?;

    runtime.tick(TICK_DELTA)?;
    for frame in 1..=SIM_FRAMES {
        runtime.tick(TICK_DELTA)?;
        let traveled = (AGENT_SPEED * TICK_DELTA * frame as f64).min(PATH_LENGTH);
        let expected_x = START_X + traveled;
        let actual_x = walker_x(&runtime, agent)?;
        assert!(
            (actual_x - expected_x).abs() <= TOLERANCE,
            "frame {frame}: expected world transform x {expected_x}, got {actual_x}"
        );
    }
    let final_x = walker_x(&runtime, agent)?;
    assert!(
        (final_x - DESTINATION[0]).abs() <= TOLERANCE,
        "agent transform should end at the destination, got x {final_x}"
    );
    Ok(())
}
