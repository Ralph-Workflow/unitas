use unitas_core::EntityId;
use unitas_physics::terrain::{
    DetailInstance, Terrain, TerrainBody, TerrainData, TerrainError, TreeInstance, TreePrototype,
};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

const STEP_SECONDS: f64 = 1.0 / 60.0;
const SETTLE_STEPS: usize = 180;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn flat_data(resolution: usize, size: [f64; 3], height: f64) -> Result<TerrainData, TerrainError> {
    let data = TerrainData::new(resolution, size)?;
    let mut updates = Vec::new();
    for row in 0..resolution {
        for column in 0..resolution {
            updates.push(([column, row], height));
        }
    }
    data.set_heights(&updates);
    Ok(data)
}

fn cell_centre(resolution: usize, size: f64, column: usize, row: usize) -> [f64; 2] {
    let cell = size / resolution as f64;
    [
        -size * 0.5 + (column as f64 + 0.5) * cell,
        -size * 0.5 + (row as f64 + 0.5) * cell,
    ]
}

fn sphere_desc(position: [f64; 3], radius: f64) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: true,
        is_trigger: false,
        shape: Shape::Sphere { radius },
        layer: 0,
        role: BodyRole::Dynamic,
        friction: 0.6,
        restitution: 0.0,
    }
}

fn body_position(world: &World, entity: EntityId) -> Result<[f64; 3], PhysicsError> {
    match world.position(entity) {
        Some(position) => Ok(position),
        None => Err(PhysicsError::InvalidBody(entity)),
    }
}

fn body_velocity(world: &World, entity: EntityId) -> Result<[f64; 3], PhysicsError> {
    match world.velocity(entity) {
        Some(velocity) => Ok(velocity),
        None => Err(PhysicsError::InvalidBody(entity)),
    }
}

#[test]
fn terrain_data_reports_resolution_size_and_per_position_height() -> TestResult {
    let data = TerrainData::new(9, [20.0, 10.0, 20.0])?;
    assert_eq!(data.heightmap_resolution(), 9);
    assert_eq!(data.size(), [20.0, 10.0, 20.0]);
    assert_eq!(data.height_at(0, 0), 0.0);
    assert_eq!(data.height_at(8, 8), 0.0);
    data.set_height(4, 7, 12.5);
    assert_eq!(data.height_at(4, 7), 12.5);
    assert_eq!(data.height_at(0, 0), 0.0);
    Ok(())
}

#[test]
fn terrain_data_handles_share_the_same_heights() -> TestResult {
    let data = TerrainData::new(4, [8.0, 4.0, 8.0])?;
    let second_handle = data.clone();
    data.set_height(2, 1, 3.25);
    assert_eq!(second_handle.height_at(2, 1), 3.25);
    second_handle.set_height(0, 0, -1.5);
    assert_eq!(data.height_at(0, 0), -1.5);
    assert_eq!(data.heights().len(), 16);
    Ok(())
}

#[test]
fn set_heights_changes_only_the_given_positions() -> TestResult {
    let resolution = 5usize;
    let data = flat_data(resolution, [10.0, 5.0, 10.0], 1.0)?;
    let before = data.heights();
    data.set_heights(&[([1, 2], 7.0), ([3, 4], -2.0)]);
    let after = data.heights();
    assert_eq!(after.len(), before.len());
    for row in 0..resolution {
        for column in 0..resolution {
            let index = row * resolution + column;
            match (column, row) {
                (1, 2) => assert_eq!(after[index], 7.0),
                (3, 4) => assert_eq!(after[index], -2.0),
                _ => assert_eq!(after[index], before[index]),
            }
        }
    }
    assert_eq!(data.height_at(1, 2), 7.0);
    assert_eq!(data.height_at(3, 4), -2.0);
    Ok(())
}

#[test]
fn hole_cells_are_neither_walkable_nor_collidable() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 2.0)?;
    let terrain = Terrain::new(data.clone());
    let solid = cell_centre(resolution, size, 2, 2);
    let hole = cell_centre(resolution, size, 1, 1);
    assert!(terrain.is_walkable([solid[0], 0.0, solid[1]]));
    assert!(terrain.is_collidable([solid[0], 0.0, solid[1]]));
    assert!(!terrain.is_hole([solid[0], 0.0, solid[1]]));
    data.set_hole(1, 1, true);
    assert!(terrain.is_hole([hole[0], 0.0, hole[1]]));
    assert!(!terrain.is_walkable([hole[0], 0.0, hole[1]]));
    assert!(!terrain.is_collidable([hole[0], 0.0, hole[1]]));
    assert!(terrain.is_walkable([solid[0], 0.0, solid[1]]));
    assert!(terrain.is_collidable([solid[0], 0.0, solid[1]]));
    assert!(!terrain.is_hole([solid[0], 0.0, solid[1]]));
    Ok(())
}

#[test]
fn terrain_height_follows_its_world_placement() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 3.0)?;
    let terrain = Terrain::new(data).with_position([10.0, 5.0, -20.0]);
    assert_eq!(terrain.position(), [10.0, 5.0, -20.0]);
    assert_eq!(terrain.size(), [size, 4.0, size]);
    assert_eq!(terrain.height_at([10.0, 0.0, -20.0]), Some(8.0));
    let cell = cell_centre(resolution, size, 0, 0);
    assert_eq!(
        terrain.height_at([10.0 + cell[0], 0.0, -20.0 + cell[1]]),
        Some(8.0)
    );
    assert_eq!(terrain.height_at([1000.0, 0.0, -20.0]), None);
    assert_eq!(terrain.height_at([10.0, 0.0, 1000.0]), None);
    Ok(())
}

#[test]
fn height_change_is_visible_to_every_terrain_using_the_data() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 1.0)?;
    let first = Terrain::new(data.clone()).with_position([0.0, 0.0, 0.0]);
    let second = Terrain::new(data.clone()).with_position([100.0, 0.0, 0.0]);
    data.set_heights(&[([0, 0], 6.0)]);
    let cell = cell_centre(resolution, size, 0, 0);
    assert_eq!(first.height_at([cell[0], 0.0, cell[1]]), Some(6.0));
    assert_eq!(second.height_at([100.0 + cell[0], 0.0, cell[1]]), Some(6.0));
    Ok(())
}

#[test]
fn body_resting_on_terrain_is_supported_at_the_authored_height() -> TestResult {
    let data = flat_data(9, [20.0, 10.0, 20.0], 5.0)?;
    let terrain = Terrain::new(data);
    let mut world = World::new();
    let entity = EntityId::new(7);
    let radius = 0.5;
    world.add_body(entity, sphere_desc([0.0, 12.0, 0.0], radius))?;
    let body = TerrainBody::new(entity, Shape::Sphere { radius });
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[body]);
    }
    let position = body_position(&world, entity)?;
    assert!(
        (position[1] - 5.5).abs() < 1e-9,
        "resting height {position:?} differs from the authored height"
    );
    let velocity = body_velocity(&world, entity)?;
    assert!(
        velocity[1].abs() < 1e-9,
        "resting body still moves: {velocity:?}"
    );
    Ok(())
}

#[test]
fn body_on_a_slope_is_supported_at_the_authored_height() -> TestResult {
    let resolution = 9usize;
    let size = [20.0, 10.0, 20.0];
    let data = TerrainData::new(resolution, size)?;
    let mut updates = Vec::new();
    for row in 0..resolution {
        for column in 0..resolution {
            updates.push(([column, row], 2.0 + 0.5 * column as f64));
        }
    }
    data.set_heights(&updates);
    let terrain = Terrain::new(data);
    let mut world = World::new();
    let entity = EntityId::new(11);
    let radius = 0.5;
    let start_x = -3.0;
    world.add_body(entity, sphere_desc([start_x, 12.0, 0.0], radius))?;
    let body = TerrainBody::new(entity, Shape::Sphere { radius });
    let cell_width = size[0] / resolution as f64;
    let slope = 0.5 / cell_width;
    let rise = (1.0 + slope * slope).sqrt();
    let authored_height =
        |local_x: f64| 2.0 + slope * (local_x + size[0] * 0.5) - 0.5 * slope * cell_width;
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[body]);
        let position = body_position(&world, entity)?;
        let perpendicular = (position[1] - authored_height(position[0])) / rise;
        assert!(
            perpendicular >= radius - 1e-9,
            "body sank into the slope: {perpendicular}"
        );
    }
    let position = body_position(&world, entity)?;
    let expected = authored_height(position[0]) + radius * rise;
    assert!(
        (position[1] - expected).abs() < 1e-9,
        "slope resting height {position:?} differs from the authored height {expected}"
    );
    assert!(
        (position[0] - start_x).abs() < 1e-9,
        "body drifted down the slope: {position:?}"
    );
    Ok(())
}

#[test]
fn box_body_resting_on_terrain_is_supported_at_the_authored_height() -> TestResult {
    let data = flat_data(9, [20.0, 10.0, 20.0], 5.0)?;
    let terrain = Terrain::new(data);
    let mut world = World::new();
    let entity = EntityId::new(19);
    let shape = Shape::Box {
        half_extents: [0.5, 0.5, 0.5],
    };
    world.add_body(
        entity,
        BodyDesc {
            shape,
            ..sphere_desc([0.0, 12.0, 0.0], 0.0)
        },
    )?;
    let body = TerrainBody::new(entity, shape);
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[body]);
    }
    let position = body_position(&world, entity)?;
    assert!(
        (position[1] - 5.5).abs() < 1e-9,
        "box resting height {position:?} differs from the authored height"
    );
    let resolution = 9usize;
    let size = [20.0, 10.0, 20.0];
    let slope_data = TerrainData::new(resolution, size)?;
    let mut updates = Vec::new();
    for row in 0..resolution {
        for column in 0..resolution {
            updates.push(([column, row], 2.0 + 0.5 * column as f64));
        }
    }
    slope_data.set_heights(&updates);
    let slope_terrain = Terrain::new(slope_data);
    let mut slope_world = World::new();
    let slope_entity = EntityId::new(23);
    slope_world.add_body(
        slope_entity,
        BodyDesc {
            shape,
            ..sphere_desc([-3.0, 12.0, 0.0], 0.0)
        },
    )?;
    let slope_body = TerrainBody::new(slope_entity, shape);
    for _ in 0..SETTLE_STEPS {
        slope_world.step(STEP_SECONDS)?;
        slope_terrain.support_bodies(&mut slope_world, &[slope_body]);
    }
    let slope_position = body_position(&slope_world, slope_entity)?;
    let cell_width = size[0] / resolution as f64;
    let slope = 0.5 / cell_width;
    let corner_x = slope_position[0] + shape_half_extent(shape);
    let authored = 2.0 + slope * (corner_x + size[0] * 0.5) - 0.5 * slope * cell_width;
    assert!(
        (slope_position[1] - shape_half_extent(shape) - authored).abs() < 1e-9,
        "box corner {slope_position:?} is not supported at the authored height {authored}"
    );
    Ok(())
}

fn shape_half_extent(shape: Shape) -> f64 {
    match shape {
        Shape::Sphere { radius } => radius,
        Shape::Box { half_extents } => half_extents[0],
        Shape::Capsule { .. } => 0.0,
    }
}

#[test]
fn body_falls_through_a_hole_cell() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 5.0)?;
    let terrain = Terrain::new(data.clone());
    let hole = cell_centre(resolution, size, 1, 1);
    let solid = cell_centre(resolution, size, 2, 2);
    data.set_hole(1, 1, true);
    let mut world = World::new();
    let over_hole = EntityId::new(13);
    let over_solid = EntityId::new(14);
    let radius = 0.5;
    world.add_body(over_hole, sphere_desc([hole[0], 12.0, hole[1]], radius))?;
    world.add_body(over_solid, sphere_desc([solid[0], 12.0, solid[1]], radius))?;
    let hole_body = TerrainBody::new(over_hole, Shape::Sphere { radius });
    let solid_body = TerrainBody::new(over_solid, Shape::Sphere { radius });
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[hole_body, solid_body]);
    }
    let hole_position = body_position(&world, over_hole)?;
    let solid_position = body_position(&world, over_solid)?;
    assert!(
        hole_position[1] < 5.0,
        "body did not fall through the hole: {hole_position:?}"
    );
    assert!(
        (solid_position[1] - 5.5).abs() < 1e-9,
        "body over solid ground is not supported: {solid_position:?}"
    );
    Ok(())
}

#[test]
fn collision_follows_a_runtime_height_change() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 5.0)?;
    let terrain = Terrain::new(data.clone());
    let mut world = World::new();
    let entity = EntityId::new(17);
    let radius = 0.5;
    world.add_body(entity, sphere_desc([0.0, 12.0, 0.0], radius))?;
    let body = TerrainBody::new(entity, Shape::Sphere { radius });
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[body]);
    }
    let resting = body_position(&world, entity)?;
    assert!(
        (resting[1] - 5.5).abs() < 1e-9,
        "resting height {resting:?} differs from the authored height"
    );
    let mut updates = Vec::new();
    for row in 0..resolution {
        for column in 0..resolution {
            updates.push(([column, row], 9.0));
        }
    }
    data.set_heights(&updates);
    for _ in 0..SETTLE_STEPS {
        world.step(STEP_SECONDS)?;
        terrain.support_bodies(&mut world, &[body]);
    }
    let raised = body_position(&world, entity)?;
    assert!(
        (raised[1] - 9.5).abs() < 1e-9,
        "body did not follow the runtime height change: {raised:?}"
    );
    Ok(())
}

#[test]
fn large_world_terrain_is_accepted_and_reports_affected_systems() -> TestResult {
    let data = flat_data(9, [100_000.0, 500.0, 100_000.0], 10.0)?;
    let terrain = Terrain::new(data).with_position([100_000.0, 0.0, 0.0]);
    let impact = terrain.large_world_impact();
    assert!(impact.exceeds_threshold);
    assert_eq!(impact.threshold, 100_000.0);
    assert_eq!(impact.position, [100_000.0, 0.0, 0.0]);
    assert!(!impact.affected_systems.is_empty());
    assert!(impact
        .affected_systems
        .iter()
        .any(|system| system.contains("float-precision")));
    assert!(impact.summary().contains("float-precision"));
    assert_eq!(terrain.height_at([100_000.0, 0.0, 0.0]), Some(10.0));
    Ok(())
}

#[test]
fn vegetation_follows_the_terrain_height() -> TestResult {
    let resolution = 4usize;
    let size = 8.0;
    let data = flat_data(resolution, [size, 4.0, size], 2.0)?;
    let prototype = data.add_tree_prototype(TreePrototype {
        name: "SpeedTree8_Oak".to_string(),
        wind_excluded: true,
        lod_count: 4,
    });
    data.add_tree_prototype(TreePrototype {
        name: "Simple_Tree".to_string(),
        wind_excluded: false,
        lod_count: 2,
    });
    data.set_tree_instances(&[TreeInstance {
        prototype,
        normalized_position: [0.125, 0.375],
        height_offset: 0.0,
        rotation_y: 0.5,
        scale: 1.25,
    }]);
    data.set_detail_instances(&[DetailInstance {
        prototype: 0,
        normalized_position: [0.125, 0.375],
        height_offset: 0.1,
        scale: 1.0,
    }]);
    let terrain = Terrain::new(data.clone()).with_position([10.0, 4.0, -20.0]);
    let cell = cell_centre(resolution, size, 0, 1);
    let trees = terrain.tree_world_positions();
    let details = terrain.detail_world_positions();
    assert_eq!(trees.len(), 1);
    assert_eq!(details.len(), 1);
    assert_eq!(trees[0][0], 10.0 + cell[0]);
    assert_eq!(trees[0][2], -20.0 + cell[1]);
    assert_eq!(trees[0][1], 4.0 + 2.0);
    assert_eq!(details[0][1], 4.0 + 2.0 + 0.1);
    data.set_heights(&[([0, 1], 6.0)]);
    let trees = terrain.tree_world_positions();
    let details = terrain.detail_world_positions();
    assert_eq!(trees[0][1], 4.0 + 6.0);
    assert_eq!(details[0][1], 4.0 + 6.0 + 0.1);
    assert_eq!(data.wind_excluded_prototypes(), vec![prototype]);
    let prototypes = data.tree_prototypes();
    assert_eq!(prototypes.len(), 2);
    assert_eq!(prototypes[prototype].lod_count, 4);
    assert!(prototypes[prototype].wind_excluded);
    assert!(!prototypes[1].wind_excluded);
    Ok(())
}

#[test]
fn invalid_terrain_data_is_rejected() {
    assert!(TerrainData::new(0, [1.0, 1.0, 1.0]).is_err());
    assert!(TerrainData::new(4, [0.0, 1.0, 1.0]).is_err());
    assert!(TerrainData::new(4, [-1.0, 1.0, 1.0]).is_err());
    assert!(TerrainData::new(4, [f64::NAN, 1.0, 1.0]).is_err());
    assert!(TerrainData::new(4, [f64::INFINITY, 1.0, 1.0]).is_err());
}

#[test]
fn terrain_bounds_report_the_placed_extent() -> TestResult {
    let data = flat_data(4, [20.0, 10.0, 20.0], 5.0)?;
    let terrain = Terrain::new(data).with_position([10.0, 1.0, -4.0]);
    let bounds = terrain.bounds();
    assert_eq!(bounds.min[0], 0.0);
    assert_eq!(bounds.max[0], 20.0);
    assert_eq!(bounds.min[2], -14.0);
    assert_eq!(bounds.max[2], 6.0);
    assert_eq!(bounds.min[1], 6.0);
    assert_eq!(bounds.max[1], 6.0);
    Ok(())
}
