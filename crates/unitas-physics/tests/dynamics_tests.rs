use unitas_core::EntityId;
use unitas_physics::simulate::{Simulation, SimulationError};
use unitas_physics::{BodyDesc, BodyRole, ForceMode, PhysicsError, Shape, World};

const NO_GRAVITY: [f64; 3] = [0.0, 0.0, 0.0];
const TOLERANCE: f64 = 1e-6;

fn dynamic_box(half_extents: [f64; 3], mass: f64) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        shape: Shape::Box { half_extents },
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn moving_box(velocity: [f64; 3]) -> BodyDesc {
    BodyDesc {
        velocity,
        ..dynamic_box([0.5, 0.5, 0.5], 1.0)
    }
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= TOLERANCE
}

fn rotation_of(world: &World, entity: EntityId) -> Result<[f64; 4], PhysicsError> {
    world
        .rotation(entity)
        .ok_or(PhysicsError::InvalidBody(entity))
}

fn angular_velocity_of(world: &World, entity: EntityId) -> Result<[f64; 3], PhysicsError> {
    world
        .angular_velocity(entity)
        .ok_or(PhysicsError::InvalidBody(entity))
}

#[test]
fn a_torque_spins_a_dynamic_body_and_changes_its_orientation() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, dynamic_box([0.5, 0.5, 0.5], 2.0))?;

    let inertia = world
        .angular_inertia(entity)
        .ok_or(PhysicsError::InvalidBody(entity))?;
    let expected_inertia = 2.0 * (0.25 + 0.25) / 3.0;
    assert!(
        close(inertia[1], expected_inertia),
        "the inertia of the box must come from its shape: expected {expected_inertia}, got {inertia:?}"
    );

    world.add_torque(entity, [0.0, 6.0, 0.0], ForceMode::Force)?;
    world.step(0.02)?;

    let angular_velocity = angular_velocity_of(&world, entity)?;
    let expected_spin = 6.0 * 0.02 / expected_inertia;
    assert!(
        close(angular_velocity[1], expected_spin),
        "the torque must spin the body about the y axis: expected {expected_spin}, got {angular_velocity:?}"
    );
    assert!(
        close(angular_velocity[0], 0.0) && close(angular_velocity[2], 0.0),
        "the torque must not spin the body about the other axes: got {angular_velocity:?}"
    );

    for _ in 0..10 {
        world.step(0.02)?;
    }
    let rotation = rotation_of(&world, entity)?;
    assert!(
        rotation[1].abs() > TOLERANCE,
        "the body must have turned about the y axis: got {rotation:?}"
    );
    Ok(())
}

#[test]
fn an_angular_impulse_changes_the_angular_velocity_and_the_orientation() -> Result<(), PhysicsError>
{
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, dynamic_box([0.5, 0.5, 0.5], 2.0))?;

    world.add_torque(entity, [0.0, 0.0, 2.0], ForceMode::Impulse)?;
    let angular_velocity = angular_velocity_of(&world, entity)?;
    assert!(
        angular_velocity[2] > 0.0,
        "the angular impulse must produce a non-zero angular velocity: got {angular_velocity:?}"
    );

    for _ in 0..10 {
        world.step(0.02)?;
    }
    let rotation = rotation_of(&world, entity)?;
    assert!(
        rotation[2].abs() > TOLERANCE,
        "the body must have turned about the z axis: got {rotation:?}"
    );
    Ok(())
}

#[test]
fn an_angular_velocity_set_on_a_dynamic_body_drives_its_orientation() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, dynamic_box([0.5, 0.5, 0.5], 2.0))?;

    world.set_angular_velocity(entity, [0.0, 2.0, 0.0])?;
    assert_eq!(angular_velocity_of(&world, entity)?, [0.0, 2.0, 0.0]);

    for _ in 0..10 {
        world.step(0.02)?;
    }
    let rotation = rotation_of(&world, entity)?;
    assert!(
        rotation[1].abs() > TOLERANCE,
        "the body must have turned about the y axis: got {rotation:?}"
    );
    Ok(())
}

#[test]
fn the_interpolation_alpha_is_zero_at_a_step_boundary_and_rises_toward_one(
) -> Result<(), SimulationError> {
    let mut simulation = Simulation::new(World::with_gravity(NO_GRAVITY));
    simulation.set_fixed_timestep(0.02)?;
    let entity = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(entity, moving_box([2.0, 0.0, 0.0]))?;

    assert_eq!(simulation.advance_fixed(0.02)?, 1);
    assert!(
        close(simulation.interpolation_alpha(), 0.0),
        "the alpha must be zero at a step boundary: got {}",
        simulation.interpolation_alpha()
    );

    assert_eq!(simulation.advance_fixed(0.005)?, 0);
    assert!(
        close(simulation.interpolation_alpha(), 0.25),
        "the alpha must rise toward one as the next step advances: got {}",
        simulation.interpolation_alpha()
    );

    assert_eq!(simulation.advance_fixed(0.015)?, 1);
    assert!(
        close(simulation.interpolation_alpha(), 0.0),
        "the alpha must return to zero at the next step boundary: got {}",
        simulation.interpolation_alpha()
    );
    Ok(())
}

#[test]
fn the_interpolated_pose_lies_between_the_two_step_snapshots() -> Result<(), SimulationError> {
    let mut simulation = Simulation::new(World::with_gravity(NO_GRAVITY));
    simulation.set_fixed_timestep(0.02)?;
    let entity = EntityId::new(1);
    simulation
        .world_mut()
        .add_body(entity, moving_box([2.0, 0.0, 0.0]))?;

    simulation.advance_fixed(0.02)?;
    let current = simulation
        .world()
        .position(entity)
        .ok_or(PhysicsError::InvalidBody(entity))?;
    assert!(
        close(current[0], 0.04),
        "the body must have moved one fixed step: got {current:?}"
    );

    let (start, _) = simulation
        .interpolated_pose(entity)
        .ok_or(PhysicsError::InvalidBody(entity))?;
    assert!(
        close(start[0], 0.0),
        "the interpolated pose at the step boundary must be the earlier snapshot: got {start:?}"
    );

    simulation.advance_fixed(0.01)?;
    let (middle, _) = simulation
        .interpolated_pose(entity)
        .ok_or(PhysicsError::InvalidBody(entity))?;
    assert!(
        middle[0] > start[0] && middle[0] < current[0],
        "the interpolated pose must lie between the two snapshots: got {middle:?} between {start:?} and {current:?}"
    );

    let (end, _) = simulation
        .world()
        .interpolated_pose(entity, 1.0)
        .ok_or(PhysicsError::InvalidBody(entity))?;
    assert!(
        close(end[0], current[0]),
        "the interpolated pose at alpha one must be the later snapshot: got {end:?} and {current:?}"
    );
    Ok(())
}
