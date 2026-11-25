use unitas_core::EntityId;
use unitas_physics::navigation::{NavMesh, NavMeshTriangle, NavigationError};
use unitas_runtime::navigation::{NavigationRuntimeError, NavigationRuntimeWorld};

const TOLERANCE: f64 = 1e-6;

/// Two triangles covering one square cell of the floor.
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

/// A row of cells along the x axis.
fn row(origin: [f64; 3], cells: usize, size: f64, area: u32) -> Vec<NavMeshTriangle> {
    let mut triangles = Vec::new();
    for index in 0..cells {
        let x = origin[0] + index as f64 * size;
        triangles.extend(cell([x, origin[1], origin[2]], size, area));
    }
    triangles
}

fn floor() -> Result<NavMesh, NavigationError> {
    NavMesh::build(&row([0.0, 0.0, 0.0], 4, 1.0, 0), &[])
}

#[test]
fn navigation_runtime_binds_the_navmesh_to_entities() -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);

    world.add_agent(EntityId::new(1), "Walker", [0.5, 0.0, 0.5])?;

    assert_eq!(world.agent_count(), 1);
    assert_eq!(world.navmesh().polygon_count(), 8);
    assert!(world.is_on_navmesh(EntityId::new(1)));
    assert_eq!(world.position(EntityId::new(1)), Some([0.5, 0.0, 0.5]));
    Ok(())
}

#[test]
fn navigation_runtime_steps_the_agents_towards_their_destination(
) -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);
    world.add_agent(EntityId::new(1), "Walker", [0.5, 0.0, 0.5])?;

    world.set_destination(EntityId::new(1), [3.5, 0.0, 0.5])?;
    for _ in 0..20 {
        world.step(0.1)?;
    }

    let position = world
        .position(EntityId::new(1))
        .ok_or(NavigationRuntimeError::UnknownAgent(EntityId::new(1)))?;
    assert!((position[0] - 3.5).abs() < TOLERANCE);
    Ok(())
}

#[test]
fn navigation_runtime_keeps_the_transform_when_update_position_is_off(
) -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);
    world.add_agent(EntityId::new(1), "Walker", [0.5, 0.0, 0.5])?;
    let agent = world
        .agent_mut(EntityId::new(1))
        .ok_or(NavigationRuntimeError::UnknownAgent(EntityId::new(1)))?;
    agent.update_position = false;

    world.set_destination(EntityId::new(1), [3.5, 0.0, 0.5])?;
    world.step(0.5)?;

    assert_eq!(world.position(EntityId::new(1)), Some([0.5, 0.0, 0.5]));
    let next = world
        .next_position(EntityId::new(1))
        .ok_or(NavigationRuntimeError::UnknownAgent(EntityId::new(1)))?;
    assert!(next[0] > 0.5);
    Ok(())
}

#[test]
fn navigation_runtime_moves_an_agent_with_a_warp() -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);
    world.add_agent(EntityId::new(1), "Walker", [0.5, 0.0, 0.5])?;

    world.warp(EntityId::new(1), [2.5, 0.0, 0.5])?;

    assert_eq!(world.position(EntityId::new(1)), Some([2.5, 0.0, 0.5]));
    assert!(world.is_on_navmesh(EntityId::new(1)));
    Ok(())
}

#[test]
fn navigation_runtime_names_the_agent_of_an_off_mesh_destination(
) -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);
    world.add_agent(EntityId::new(2), "Scout", [0.5, 0.0, 0.5])?;
    world.warp(EntityId::new(2), [0.5, 4.0, 0.5])?;

    let result = world.set_destination(EntityId::new(2), [1.5, 0.0, 0.5]);

    assert!(matches!(
        result,
        Err(NavigationRuntimeError::Navigation(NavigationError::AgentOffNavMesh {
            ref name,
            ..
        })) if name == "Scout"
    ));
    Ok(())
}

#[test]
fn navigation_runtime_reports_an_unknown_agent() -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);

    assert!(matches!(
        world.set_destination(EntityId::new(9), [0.5, 0.0, 0.5]),
        Err(NavigationRuntimeError::UnknownAgent(entity)) if entity == EntityId::new(9)
    ));
    Ok(())
}

#[test]
fn navigation_runtime_refuses_a_second_agent_for_one_entity() -> Result<(), NavigationRuntimeError>
{
    let mut world = NavigationRuntimeWorld::new(floor()?);
    world.add_agent(EntityId::new(1), "Walker", [0.5, 0.0, 0.5])?;

    assert!(matches!(
        world.add_agent(EntityId::new(1), "Other", [1.5, 0.0, 0.5]),
        Err(NavigationRuntimeError::DuplicateAgent(entity)) if entity == EntityId::new(1)
    ));
    assert_eq!(world.agent_count(), 1);
    Ok(())
}

#[test]
fn navigation_runtime_rejects_an_invalid_delta_time() -> Result<(), NavigationRuntimeError> {
    let mut world = NavigationRuntimeWorld::new(floor()?);

    assert!(matches!(
        world.step(f64::NAN),
        Err(NavigationRuntimeError::InvalidDeltaTime(_))
    ));
    Ok(())
}
