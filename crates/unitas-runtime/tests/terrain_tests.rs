use unitas_core::EntityId;
use unitas_physics::terrain::{TerrainBody, TerrainData, TerrainError};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};
use unitas_runtime::terrain::{far_position_impact, TerrainRuntimeError, TerrainWorld};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TERRAIN_SIZE: [f64; 3] = [30.0, 10.0, 30.0];
const RESOLUTION: usize = 3;
const FLAT_HEIGHT: f64 = 5.0;
const HOLE_CELL_CENTRE: [f64; 3] = [-10.0, FLAT_HEIGHT, -10.0];

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

fn sphere_desc(position: [f64; 3], radius: f64) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: false,
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

#[test]
fn set_heights_is_visible_to_every_terrain_using_the_shared_data() -> TestResult {
    let data = flat_data(RESOLUTION, TERRAIN_SIZE, FLAT_HEIGHT)?;
    let mut terrains = TerrainWorld::new();
    let first = EntityId::new(1);
    let second = EntityId::new(2);
    terrains.register_terrain(first, &data, [0.0, 0.0, 0.0])?;
    terrains.register_terrain(second, &data, [100.0, 0.0, 0.0])?;

    terrains.set_heights(first, &[([0, 0], 7.5)])?;

    assert_eq!(terrains.height_at(first, HOLE_CELL_CENTRE), Some(7.5));
    assert_eq!(
        terrains.height_at(
            second,
            [100.0 + HOLE_CELL_CENTRE[0], 0.0, HOLE_CELL_CENTRE[2]]
        ),
        Some(7.5)
    );
    assert_eq!(
        terrains.height_at(
            second,
            [100.0 + HOLE_CELL_CENTRE[0], 0.0, HOLE_CELL_CENTRE[2]]
        ),
        terrains.height_at(first, HOLE_CELL_CENTRE)
    );
    let shared_height = terrains
        .terrain(first)
        .map(|terrain| terrain.data().height_at(0, 0));
    assert_eq!(shared_height, Some(7.5));
    Ok(())
}

#[test]
fn terrain_height_and_normal_agree_with_the_registered_heightfield_collider() -> TestResult {
    let data = flat_data(RESOLUTION, TERRAIN_SIZE, FLAT_HEIGHT)?;
    let mut terrains = TerrainWorld::new();
    let terrain_entity = EntityId::new(3);
    terrains.register_terrain(terrain_entity, &data, [0.0, 0.0, 0.0])?;

    let mut world = World::new();
    let body_entity = EntityId::new(9);
    let radius = 0.5;
    world.add_body(
        body_entity,
        sphere_desc([0.0, FLAT_HEIGHT + 0.25, 0.0], radius),
    )?;
    terrains.register_heightfield(
        terrain_entity,
        TerrainBody::new(body_entity, Shape::Sphere { radius }),
    )?;

    let contacts = terrains.apply_heightfields(&mut world);

    assert_eq!(
        contacts.len(),
        1,
        "expected one heightfield contact, got {contacts:?}"
    );
    assert_eq!(contacts[0].point, [0.0, FLAT_HEIGHT, 0.0]);
    assert_eq!(contacts[0].normal, [0.0, 1.0, 0.0]);
    assert_eq!(body_position(&world, body_entity)?[1], FLAT_HEIGHT + radius);
    assert_eq!(
        terrains.height_at(terrain_entity, [0.0, 0.0, 0.0]),
        Some(FLAT_HEIGHT)
    );
    assert_eq!(
        terrains.normal_at(terrain_entity, [0.0, 0.0, 0.0]),
        Some(contacts[0].normal)
    );
    Ok(())
}

#[test]
fn terrain_hole_cell_is_reported_and_blocks_the_heightfield_collider() -> TestResult {
    let data = flat_data(RESOLUTION, TERRAIN_SIZE, FLAT_HEIGHT)?;
    data.set_hole(0, 0, true);
    let mut terrains = TerrainWorld::new();
    let terrain_entity = EntityId::new(4);
    terrains.register_terrain(terrain_entity, &data, [0.0, 0.0, 0.0])?;

    let mut world = World::new();
    let body_entity = EntityId::new(10);
    let solid_entity = EntityId::new(12);
    let radius = 0.5;
    let resting = [HOLE_CELL_CENTRE[0], FLAT_HEIGHT + 0.25, HOLE_CELL_CENTRE[2]];
    let solid = [0.0, FLAT_HEIGHT + 0.25, 0.0];
    world.add_body(body_entity, sphere_desc(resting, radius))?;
    world.add_body(solid_entity, sphere_desc(solid, radius))?;
    terrains.register_heightfield(
        terrain_entity,
        TerrainBody::new(body_entity, Shape::Sphere { radius }),
    )?;
    terrains.register_heightfield(
        terrain_entity,
        TerrainBody::new(solid_entity, Shape::Sphere { radius }),
    )?;

    let contacts = terrains.apply_heightfields(&mut world);

    assert!(terrains.is_hole(terrain_entity, HOLE_CELL_CENTRE));
    assert_eq!(
        contacts.len(),
        1,
        "only the solid cell is supported: {contacts:?}"
    );
    assert_eq!(contacts[0].point, [0.0, FLAT_HEIGHT, 0.0]);
    assert_eq!(body_position(&world, body_entity)?, resting);
    assert_eq!(
        body_position(&world, solid_entity)?[1],
        FLAT_HEIGHT + radius
    );
    Ok(())
}

#[test]
fn terrain_far_position_helper_names_the_systems_that_must_update_a_distant_object() -> TestResult {
    let distant = far_position_impact([100_000.0, 0.0, 0.0]);

    assert!(distant.exceeds_threshold);
    assert_eq!(distant.threshold, 100_000.0);
    assert!(distant
        .affected_systems
        .iter()
        .any(|system| system.contains("physics")));
    assert!(distant
        .affected_systems
        .iter()
        .any(|system| system.contains("rendering")));
    assert!(distant
        .affected_systems
        .iter()
        .any(|system| system.contains("navigation")));

    let near = far_position_impact([10.0, 0.0, 0.0]);

    assert!(!near.exceeds_threshold);
    assert!(near.affected_systems.is_empty());
    Ok(())
}

#[test]
fn terrain_registration_rejects_duplicate_and_unknown_entities() -> TestResult {
    let data = flat_data(RESOLUTION, TERRAIN_SIZE, FLAT_HEIGHT)?;
    let mut terrains = TerrainWorld::new();
    let entity = EntityId::new(5);
    let unknown = EntityId::new(6);
    terrains.register_terrain(entity, &data, [0.0, 0.0, 0.0])?;

    assert_eq!(
        terrains.register_terrain(entity, &data, [0.0, 0.0, 0.0]),
        Err(TerrainRuntimeError::DuplicateTerrain(entity))
    );
    assert_eq!(
        terrains.set_heights(unknown, &[([0, 0], 1.0)]),
        Err(TerrainRuntimeError::UnknownTerrain(unknown))
    );
    assert_eq!(
        terrains.register_heightfield(
            unknown,
            TerrainBody::new(EntityId::new(11), Shape::Sphere { radius: 0.5 })
        ),
        Err(TerrainRuntimeError::UnknownTerrain(unknown))
    );

    assert!(terrains.remove_terrain(entity).is_some());
    assert_eq!(terrains.height_at(entity, [0.0, 0.0, 0.0]), None);
    assert_eq!(
        terrains.set_heights(entity, &[([0, 0], 1.0)]),
        Err(TerrainRuntimeError::UnknownTerrain(entity))
    );
    Ok(())
}
