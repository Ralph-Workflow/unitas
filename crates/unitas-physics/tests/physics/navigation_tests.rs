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

#[test]
fn navigation_sample_position_snaps_within_max_distance_and_filters_by_area_mask(
) -> Result<(), NavigationError> {
    let mut triangles = cell([0.0, 0.0, 0.0], 2.0, 0);
    triangles.extend(cell([5.0, 0.0, 0.0], 2.0, 1));
    let navmesh = NavMesh::build(&triangles, &[])?;

    let hit = navmesh.sample_position([2.5, 0.0, 1.0], 1.0, NavMesh::ALL_AREAS);
    assert!(hit.is_some());
    if let Some(hit) = hit {
        assert!(hit.hit);
        assert!((hit.position[0] - 2.0).abs() < TOLERANCE);
        assert!((hit.position[1] - 0.0).abs() < TOLERANCE);
        assert!((hit.position[2] - 1.0).abs() < TOLERANCE);
        assert!((hit.distance - 0.5).abs() < TOLERANCE);
        assert_eq!(hit.mask, 1 << 0);
        assert!(hit.normal[1].abs() > 0.9);
        assert_eq!(hit.polygon_index, Some(1));
    }

    let miss = navmesh.sample_position([3.5, 0.0, 1.0], 1.0, NavMesh::ALL_AREAS);
    assert!(miss.is_none());

    let masked = navmesh.sample_position([2.5, 0.0, 1.0], 4.0, 1 << 1);
    assert!(masked.is_some());
    if let Some(hit) = masked {
        assert!((hit.position[0] - 5.0).abs() < TOLERANCE);
        assert!((hit.distance - 2.5).abs() < TOLERANCE);
        assert_eq!(hit.mask, 1 << 1);
    }
    Ok(())
}

#[test]
fn navigation_raycast_reports_unblocked_inside_and_blocked_at_boundary_edge(
) -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 3, 1.0, 0), &[])?;

    let unblocked = navmesh.raycast([0.5, 0.0, 0.5], [2.5, 0.0, 0.5], NavMesh::ALL_AREAS);
    assert!(!unblocked.hit);
    assert!((unblocked.position[0] - 2.5).abs() < TOLERANCE);
    assert!((unblocked.distance - 2.0).abs() < TOLERANCE);

    let blocked = navmesh.raycast([0.5, 0.0, 0.5], [4.5, 0.0, 0.5], NavMesh::ALL_AREAS);
    assert!(blocked.hit);
    assert!((blocked.position[0] - 3.0).abs() < TOLERANCE);
    assert!((blocked.position[2] - 0.5).abs() < TOLERANCE);
    assert!((blocked.distance - 2.5).abs() < TOLERANCE);
    assert!((blocked.normal[0] - 1.0).abs() < TOLERANCE);
    assert_eq!(blocked.mask, 1 << 0);
    Ok(())
}

#[test]
fn navigation_find_closest_edge_returns_nearest_boundary_point_normal_and_distance(
) -> Result<(), NavigationError> {
    let navmesh = NavMesh::build(&row([0.0, 0.0, 0.0], 3, 1.0, 0), &[])?;

    let hit = navmesh.find_closest_edge([1.5, 0.0, 0.2], NavMesh::ALL_AREAS);
    assert!(hit.is_some());
    if let Some(hit) = hit {
        assert!(hit.hit);
        assert!((hit.position[0] - 1.5).abs() < TOLERANCE);
        assert!((hit.position[1] - 0.0).abs() < TOLERANCE);
        assert!((hit.position[2] - 0.0).abs() < TOLERANCE);
        assert!((hit.distance - 0.2).abs() < TOLERANCE);
        assert!((hit.normal[2] - (-1.0)).abs() < TOLERANCE);
        assert_eq!(hit.mask, 1 << 0);
    }
    Ok(())
}

#[test]
fn navigation_off_mesh_link_manual_and_auto_traversal() -> Result<(), NavigationError> {
    let mut triangles = row([0.0, 0.0, 0.0], 2, 1.0, 0);
    triangles.extend(row([6.0, 0.0, 0.0], 2, 1.0, 0));
    let mut navmesh = NavMesh::build(&triangles, &[])?;
    navmesh.add_link(NavMeshLink::new([2.0, 0.0, 0.5], [6.0, 0.0, 0.5], true))?;

    let mut manual_agent = NavMeshAgent::new(EntityId::new(1), "Jumper", [1.5, 0.0, 0.5]);
    manual_agent.speed = 2.0;
    manual_agent.auto_traverse_off_mesh_link = false;
    manual_agent.set_destination(&navmesh, [7.5, 0.0, 0.5])?;

    manual_agent.update(0.5);
    assert!(manual_agent.is_on_off_mesh_link());
    assert!((manual_agent.position()[0] - 2.0).abs() < TOLERANCE);
    let current_link = manual_agent.current_off_mesh_link();
    assert!(current_link.is_some());
    if let Some(link) = current_link {
        assert!((link.start[0] - 2.0).abs() < TOLERANCE);
        assert!((link.end[0] - 6.0).abs() < TOLERANCE);
    }

    manual_agent.update(1.0);
    assert!(manual_agent.is_on_off_mesh_link());
    assert!((manual_agent.position()[0] - 2.0).abs() < TOLERANCE);

    assert!(manual_agent.complete_off_mesh_link());
    assert!(!manual_agent.is_on_off_mesh_link());
    assert!(manual_agent.current_off_mesh_link().is_none());
    assert!((manual_agent.position()[0] - 6.0).abs() < TOLERANCE);

    manual_agent.update(0.5);
    assert!((manual_agent.position()[0] - 7.0).abs() < TOLERANCE);

    let mut auto_agent = NavMeshAgent::new(EntityId::new(2), "AutoJumper", [1.5, 0.0, 0.5]);
    auto_agent.speed = 4.0;
    auto_agent.auto_traverse_off_mesh_link = true;
    auto_agent.set_destination(&navmesh, [7.5, 0.0, 0.5])?;

    auto_agent.update(0.5);
    assert!(auto_agent.is_on_off_mesh_link());
    assert!(auto_agent.position()[0] > 2.0 && auto_agent.position()[0] < 6.0);

    auto_agent.update(1.0);
    assert!(!auto_agent.is_on_off_mesh_link());
    assert!(auto_agent.position()[0] > 6.0);
    Ok(())
}

#[test]
fn navigation_agent_update_rotation_and_auto_repath() -> Result<(), NavigationError> {
    let mut triangles = row([0.0, 0.0, 0.0], 2, 1.0, 0);
    triangles.extend(row([6.0, 0.0, 0.0], 2, 1.0, 0));
    let mut navmesh = NavMesh::build(&triangles, &[])?;

    let mut agent = NavMeshAgent::new(EntityId::new(1), "Scout", [0.5, 0.0, 0.5]);
    agent.speed = 2.0;
    agent.update_rotation = false;
    let initial_rotation = agent.rotation();
    let partial = agent.set_destination(&navmesh, [7.5, 0.0, 0.5])?;
    assert_eq!(partial.status, PathQueryStatus::Partial);

    agent.update(0.2);
    assert_eq!(agent.rotation(), initial_rotation);

    agent.update_rotation = true;
    agent.update(0.2);
    assert_ne!(agent.rotation(), initial_rotation);
    let forward = agent.forward();
    assert!(forward[0] > 0.7);
    assert!((forward[0] * forward[0] + forward[2] * forward[2] - 1.0).abs() < TOLERANCE);

    navmesh.add_link(NavMeshLink::new([2.0, 0.0, 0.5], [6.0, 0.0, 0.5], true))?;
    agent.auto_repath = true;
    agent.step_with_navmesh(&navmesh, 2.0);
    assert!(agent.position()[0] > 2.0);
    Ok(())
}

#[test]
fn navigation_off_mesh_link_supports_one_way_cost_override_and_activation(
) -> Result<(), NavigationError> {
    let mut triangles = row([0.0, 0.0, 0.0], 4, 1.0, 0);
    triangles.extend(row([10.0, 0.0, 0.0], 2, 1.0, 0));
    let mut navmesh = NavMesh::build(
        &triangles,
        &[NavMeshArea::new(0, 5.0), NavMeshArea::new(2, 1.0)],
    )?;
    navmesh.add_link(
        NavMeshLink::new([0.5, 0.0, 0.5], [3.5, 0.0, 0.5], false)
            .with_area(2)
            .with_cost_override(0.1),
    )?;
    navmesh.add_link(NavMeshLink::new([3.5, 0.0, 0.5], [10.5, 0.0, 0.5], false))?;

    let forward_shortcut = navmesh.find_path([0.5, 0.0, 0.5], [3.5, 0.0, 0.5]);
    assert_eq!(forward_shortcut.status, PathQueryStatus::Complete);
    assert_eq!(forward_shortcut.corners.len(), 4);

    let reverse_walk = navmesh.find_path([3.5, 0.0, 0.5], [0.5, 0.0, 0.5]);
    assert_eq!(reverse_walk.status, PathQueryStatus::Complete);
    assert!(reverse_walk.corners.len() > 4);

    let across_gap = navmesh.find_path([0.5, 0.0, 0.5], [10.5, 0.0, 0.5]);
    assert_eq!(across_gap.status, PathQueryStatus::Complete);

    let reverse_gap = navmesh.find_path([10.5, 0.0, 0.5], [0.5, 0.0, 0.5]);
    assert_eq!(reverse_gap.status, PathQueryStatus::Partial);

    assert!(navmesh.set_link_activated(1, false));
    let disabled_gap = navmesh.find_path([0.5, 0.0, 0.5], [10.5, 0.0, 0.5]);
    assert_eq!(disabled_gap.status, PathQueryStatus::Partial);
    Ok(())
}
