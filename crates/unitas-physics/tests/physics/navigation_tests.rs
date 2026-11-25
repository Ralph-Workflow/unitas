use unitas_core::EntityId;
use unitas_physics::navigation::{
    NavMesh, NavMeshAgent, NavMeshArea, NavMeshLink, NavMeshTriangle, NavigationError,
    PathQueryStatus, UnreachableReason,
};

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

/// Two strips of floor joined at both ends: the near strip is area 1, the far
/// strip and the two connectors are area 0.
fn two_strip_layout() -> Vec<NavMeshTriangle> {
    let mut triangles = row([0.0, 0.0, 0.0], 4, 1.0, 1);
    triangles.extend(row([0.0, 0.0, 2.0], 4, 1.0, 0));
    triangles.extend(cell([0.0, 0.0, 1.0], 1.0, 0));
    triangles.extend(cell([3.0, 0.0, 1.0], 1.0, 0));
    triangles
}

#[test]
fn navigation_navmesh_builds_one_polygon_per_walkable_triangle() -> Result<(), NavigationError> {
    let triangles = row([0.0, 0.0, 0.0], 3, 1.0, 0);
    let navmesh = NavMesh::build(&triangles, &[])?;

    assert_eq!(navmesh.polygon_count(), 6);
    assert!(navmesh
        .polygons()
        .iter()
        .all(|polygon| !polygon.neighbors.is_empty()));
    assert!(navmesh.polygon_at([0.5, 0.0, 0.5]).is_some());
    assert!(navmesh.polygon_at([0.5, 4.0, 0.5]).is_none());
    Ok(())
}

#[test]
fn navigation_navmesh_build_rejects_a_degenerate_triangle() {
    let triangles = vec![NavMeshTriangle::new(
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        0,
    )];

    assert!(matches!(
        NavMesh::build(&triangles, &[]),
        Err(NavigationError::DegenerateTriangle { index: 0 })
    ));
}

#[test]
fn navigation_path_query_reports_a_complete_path_between_connected_areas(
) -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 4, 1.0, 0), &[])?;

    let result = navmesh.find_path([0.5, 0.0, 0.5], [3.5, 0.0, 0.5]);

    assert_eq!(result.status, PathQueryStatus::Complete);
    assert_eq!(result.unreachable_reason, None);
    assert!(result.corners.len() >= 2);
    assert!((result.corners[0][0] - 0.5).abs() < TOLERANCE);
    let last = result.corners[result.corners.len() - 1];
    assert!((last[0] - 3.5).abs() < TOLERANCE);
    Ok(())
}

#[test]
fn navigation_path_query_reports_invalid_when_the_goal_is_off_the_navmesh(
) -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 2, 1.0, 0), &[])?;

    let result = navmesh.find_path([0.5, 0.0, 0.5], [0.5, 4.0, 0.5]);

    assert_eq!(result.status, PathQueryStatus::Invalid);
    assert_eq!(
        result.unreachable_reason,
        Some(UnreachableReason::GoalNotOnNavMesh)
    );
    assert!(result.corners.is_empty());
    Ok(())
}

#[test]
fn navigation_path_query_reports_invalid_when_the_start_is_off_the_navmesh(
) -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 2, 1.0, 0), &[])?;

    let result = navmesh.find_path([0.5, 4.0, 0.5], [0.5, 0.0, 0.5]);

    assert_eq!(result.status, PathQueryStatus::Invalid);
    assert_eq!(
        result.unreachable_reason,
        Some(UnreachableReason::StartNotOnNavMesh)
    );
    assert!(result.corners.is_empty());
    Ok(())
}

#[test]
fn navigation_path_query_reports_partial_when_the_goal_cannot_be_reached(
) -> Result<(), NavigationError> {
    let mut triangles = row([0.0, 0.0, 0.0], 2, 1.0, 0);
    triangles.extend(row([10.0, 0.0, 0.0], 2, 1.0, 0));
    let navmesh = NavMesh::build(&triangles, &[])?;

    let result = navmesh.find_path([0.5, 0.0, 0.5], [10.5, 0.0, 0.5]);

    assert_eq!(result.status, PathQueryStatus::Partial);
    assert_eq!(
        result.unreachable_reason,
        Some(UnreachableReason::NoRouteToGoal)
    );
    assert!(!result.corners.is_empty());
    assert!(result.corners.iter().all(|corner| corner[0] < 5.0));
    Ok(())
}

#[test]
fn navigation_path_query_prefers_the_areas_with_the_lower_cost() -> Result<(), NavigationError> {
    let expensive_near_strip = NavMesh::build(
        &two_strip_layout(),
        &[NavMeshArea::new(0, 1.0), NavMeshArea::new(1, 5.0)],
    )?;
    let expensive_far_strip = NavMesh::build(
        &two_strip_layout(),
        &[NavMeshArea::new(0, 5.0), NavMeshArea::new(1, 1.0)],
    )?;

    let detour = expensive_near_strip.find_path([0.5, 0.0, 0.5], [3.5, 0.0, 0.5]);
    let direct = expensive_far_strip.find_path([0.5, 0.0, 0.5], [3.5, 0.0, 0.5]);

    assert_eq!(detour.status, PathQueryStatus::Complete);
    assert_eq!(direct.status, PathQueryStatus::Complete);
    assert!(detour
        .corners
        .iter()
        .any(|corner| corner[2] >= 2.0 - TOLERANCE));
    assert!(direct
        .corners
        .iter()
        .all(|corner| corner[2] <= 1.0 + TOLERANCE));
    Ok(())
}

#[test]
fn navigation_agent_walks_towards_the_destination_at_its_speed() -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 4, 1.0, 0), &[])?;
    let mut agent = NavMeshAgent::new(EntityId::new(1), "Walker", [0.5, 0.0, 0.5]);
    agent.speed = 2.0;

    agent.set_destination(&navmesh, [3.5, 0.0, 0.5])?;
    agent.update(0.5);

    assert!((agent.position()[0] - 1.5).abs() < TOLERANCE);
    assert!((agent.velocity()[0] - 2.0).abs() < TOLERANCE);
    assert!(agent.has_path());
    Ok(())
}

#[test]
fn navigation_agent_keeps_its_transform_when_update_position_is_off() -> Result<(), NavigationError>
{
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 4, 1.0, 0), &[])?;
    let mut agent = NavMeshAgent::new(EntityId::new(1), "Walker", [0.5, 0.0, 0.5]);
    agent.speed = 2.0;
    agent.update_position = false;

    agent.set_destination(&navmesh, [3.5, 0.0, 0.5])?;
    agent.update(0.5);

    assert!((agent.next_position()[0] - 1.5).abs() < TOLERANCE);
    assert_eq!(agent.position(), [0.5, 0.0, 0.5]);
    Ok(())
}

#[test]
fn navigation_warp_relocates_the_agent_without_a_path() -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 4, 1.0, 0), &[])?;
    let mut agent = NavMeshAgent::new(EntityId::new(1), "Walker", [0.5, 0.0, 0.5]);
    assert!(!agent.has_path());

    agent.warp([2.5, 0.0, 0.5])?;

    assert_eq!(agent.position(), [2.5, 0.0, 0.5]);
    assert_eq!(agent.next_position(), [2.5, 0.0, 0.5]);
    assert!(agent.is_on_navmesh(&navmesh));
    assert!(!agent.has_path());
    Ok(())
}

#[test]
fn navigation_agent_off_the_navmesh_is_not_on_it() -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 2, 1.0, 0), &[])?;
    let mut agent = NavMeshAgent::new(EntityId::new(1), "Walker", [0.5, 0.0, 0.5]);
    assert!(agent.is_on_navmesh(&navmesh));

    agent.warp([0.5, 4.0, 0.5])?;

    assert!(!agent.is_on_navmesh(&navmesh));
    Ok(())
}

#[test]
fn navigation_set_destination_on_an_off_mesh_agent_names_the_agent() -> Result<(), NavigationError>
{
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 2, 1.0, 0), &[])?;
    let mut agent = NavMeshAgent::new(EntityId::new(7), "Scout", [0.5, 0.0, 0.5]);
    agent.warp([0.5, 4.0, 0.5])?;

    let result = agent.set_destination(&navmesh, [1.5, 0.0, 0.5]);

    assert!(matches!(
        result,
        Err(NavigationError::AgentOffNavMesh { ref name, .. }) if name == "Scout"
    ));
    let message = match result {
        Ok(_) => String::new(),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("Scout"));
    Ok(())
}

#[test]
fn navigation_agent_follows_an_explicit_link_between_two_areas() -> Result<(), NavigationError> {
    let mut triangles = row([0.0, 0.0, 0.0], 2, 1.0, 0);
    triangles.extend(row([6.0, 0.0, 0.0], 2, 1.0, 0));
    let mut navmesh = NavMesh::build(&triangles, &[])?;
    navmesh.add_link(NavMeshLink::new([2.0, 0.0, 0.5], [6.0, 0.0, 0.5], true))?;

    let result = navmesh.find_path([1.5, 0.0, 0.5], [6.5, 0.0, 0.5]);

    assert_eq!(result.status, PathQueryStatus::Complete);
    assert!(result.corners.iter().any(|corner| {
        (corner[0] - 2.0).abs() < TOLERANCE && (corner[2] - 0.5).abs() < TOLERANCE
    }));
    assert!(result.corners.iter().any(|corner| {
        (corner[0] - 6.0).abs() < TOLERANCE && (corner[2] - 0.5).abs() < TOLERANCE
    }));

    let mut agent = NavMeshAgent::new(EntityId::new(1), "Walker", [1.5, 0.0, 0.5]);
    agent.speed = 4.0;
    agent.set_destination(&navmesh, [6.5, 0.0, 0.5])?;
    agent.update(1.0);

    assert!(agent.position()[0] > 4.0);
    assert!((agent.position()[2] - 0.5).abs() < TOLERANCE);
    Ok(())
}

#[test]
fn navigation_a_link_endpoint_off_the_navmesh_is_rejected() -> Result<(), NavigationError> {
    let mut navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 2, 1.0, 0), &[])?;

    assert!(matches!(
        navmesh.add_link(NavMeshLink::new([0.5, 9.0, 0.5], [1.5, 0.0, 0.5], true)),
        Err(NavigationError::LinkEndpointOffNavMesh { .. })
    ));
    Ok(())
}
