use unitas_core::EntityId;
use unitas_physics::navigation::{
    NavMesh, NavMeshArea, NavMeshLink, NavMeshObstacle, NavMeshQueryFilter, NavMeshSurface,
    NavMeshTriangle, NavigationError, PathQueryStatus,
};
use unitas_runtime::navigation::{NavigationRuntimeError, NavigationRuntimeWorld};

const TOLERANCE: f64 = 1e-6;

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

#[test]
fn nav_001_story_9_10_15_runtime_surface_rebuild_carving_link_and_reciprocal_avoidance(
) -> Result<(), NavigationRuntimeError> {
    let mut surface = NavMeshSurface::new();
    surface.set_geometry(row([0.0, 0.0, 0.0], 2, 1.0, 0));
    surface.set_areas(vec![NavMeshArea::new(0, 1.0), NavMeshArea::new(1, 5.0)]);
    let initial = surface.build_navmesh()?.clone();

    let mut world = NavigationRuntimeWorld::new(initial);
    let agent_a = EntityId::new(10);
    let agent_b = EntityId::new(11);
    world.add_agent(agent_a, "Alpha", [0.5, 0.0, 0.5])?;

    let mut updated_geom = row([0.0, 0.0, 0.0], 2, 1.0, 0);
    updated_geom.extend(row([4.0, 0.0, 0.0], 3, 1.0, 0));
    updated_geom.extend(row([4.0, 0.0, 1.0], 3, 1.0, 0));
    surface.add_link(NavMeshLink::new([1.5, 0.0, 0.5], [4.5, 0.0, 0.5], true));
    world.rebuild_surface(&mut surface, updated_geom)?;

    let rebuilt_path = world.set_destination(agent_a, [6.5, 0.0, 0.5])?;
    assert_eq!(
        rebuilt_path.status,
        PathQueryStatus::Complete,
        "NAV-001 Story 10: runtime rebuild of NavMeshSurface enables path across NavMeshLink"
    );

    for _ in 0..16 {
        world.step(0.15)?;
    }
    let pos_a = world
        .position(agent_a)
        .ok_or(NavigationRuntimeError::UnknownAgent(agent_a))?;
    assert!(
        (pos_a[0] - 6.5).abs() < TOLERANCE,
        "NAV-001 Story 10: agent crosses NavMeshLink after runtime surface rebuild"
    );

    world.add_agent(agent_b, "Bravo", [4.5, 0.0, 0.5])?;
    if let Some(a) = world.agent_mut(agent_a) {
        a.avoidance_priority = 10;
        a.radius = 0.4;
    }
    if let Some(b) = world.agent_mut(agent_b) {
        b.avoidance_priority = 90;
        b.radius = 0.4;
    }
    world.set_destination(agent_a, [4.5, 0.0, 0.5])?;
    world.set_destination(agent_b, [6.5, 0.0, 0.5])?;
    for _ in 0..6 {
        world.step(0.1)?;
    }
    let pos_a_after = world
        .position(agent_a)
        .ok_or(NavigationRuntimeError::UnknownAgent(agent_a))?;
    let pos_b_after = world
        .position(agent_b)
        .ok_or(NavigationRuntimeError::UnknownAgent(agent_b))?;
    let dist_xz = ((pos_a_after[0] - pos_b_after[0]).powi(2)
        + (pos_a_after[2] - pos_b_after[2]).powi(2))
    .sqrt();
    assert!(
        dist_xz > 0.2,
        "NAV-001 Story 15: runtime step resolves reciprocal avoidance between agents"
    );

    world.add_obstacle(NavMeshObstacle::carving_box(
        [5.5, 0.0, 0.5],
        [0.4, 1.0, 0.4],
    ))?;
    let mut filter = NavMeshQueryFilter::new(NavMesh::ALL_AREAS);
    filter.set_area_cost(0, 1.0)?;
    let detoured = world.calculate_path([4.5, 0.0, 0.5], [6.5, 0.0, 0.5], &filter);
    assert_eq!(detoured.status, PathQueryStatus::Complete);
    assert!(
        detoured.corners.iter().any(|c| c[2] > 0.9),
        "NAV-001 Story 9: runtime calculate_path detours around carving obstacle"
    );
    Ok(())
}
