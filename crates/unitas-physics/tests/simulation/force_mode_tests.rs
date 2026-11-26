use unitas_core::EntityId;
use unitas_physics::{BodyDesc, BodyRole, ForceMode, PhysicsError, Shape, World};

const NO_GRAVITY: [f64; 3] = [0.0, 0.0, 0.0];

fn body(role: BodyRole, mass: f64) -> BodyDesc {
    BodyDesc {
        role,
        shape: Shape::Box {
            half_extents: [0.5, 0.5, 0.5],
        },
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

fn velocity_of(world: &World, entity: EntityId) -> Result<[f64; 3], PhysicsError> {
    world
        .velocity(entity)
        .ok_or(PhysicsError::InvalidBody(entity))
}

fn close(actual: [f64; 3], expected: [f64; 3]) -> bool {
    (0..3).all(|axis| (actual[axis] - expected[axis]).abs() <= 1e-12)
}

#[test]
fn force_mode_discriminants_match_unity() -> Result<(), PhysicsError> {
    assert_eq!(ForceMode::Force as i32, 0);
    assert_eq!(ForceMode::Impulse as i32, 1);
    assert_eq!(ForceMode::Acceleration as i32, 2);
    assert_eq!(ForceMode::VelocityChange as i32, 3);
    Ok(())
}

#[test]
fn force_mode_force_applies_f_over_m_times_dt_during_step() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, body(BodyRole::Dynamic, 2.0))?;
    world.add_force(entity, [4.0, 0.0, 0.0], ForceMode::Force)?;
    assert_eq!(velocity_of(&world, entity)?, [0.0, 0.0, 0.0]);
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [1.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn force_mode_acceleration_applies_a_times_dt_regardless_of_mass() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let light = EntityId::new(1);
    let heavy = EntityId::new(2);
    // Spawn the boxes apart: overlapped spawns make contact resolution
    // contaminate the force-mode measurement this test isolates.
    let mut light_body = body(BodyRole::Dynamic, 2.0);
    light_body.position = [-2.0, 0.0, 0.0];
    let mut heavy_body = body(BodyRole::Dynamic, 8.0);
    heavy_body.position = [2.0, 0.0, 0.0];
    world.add_body(light, light_body)?;
    world.add_body(heavy, heavy_body)?;
    world.add_force(light, [3.0, 0.0, 0.0], ForceMode::Acceleration)?;
    world.add_force(heavy, [3.0, 0.0, 0.0], ForceMode::Acceleration)?;
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, light)?, [1.5, 0.0, 0.0]);
    assert_eq!(velocity_of(&world, heavy)?, [1.5, 0.0, 0.0]);
    Ok(())
}

#[test]
fn force_mode_impulse_applies_f_over_m_at_call_time() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, body(BodyRole::Dynamic, 2.0))?;
    world.add_force(entity, [4.0, 0.0, 0.0], ForceMode::Impulse)?;
    assert_eq!(velocity_of(&world, entity)?, [2.0, 0.0, 0.0]);
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [2.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn force_mode_velocity_change_applies_f_at_call_time() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, body(BodyRole::Dynamic, 2.0))?;
    world.add_force(entity, [4.0, 0.0, 0.0], ForceMode::VelocityChange)?;
    assert_eq!(velocity_of(&world, entity)?, [4.0, 0.0, 0.0]);
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [4.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn forces_on_kinematic_bodies_are_ignored() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, body(BodyRole::Kinematic, 2.0))?;
    for mode in [
        ForceMode::Force,
        ForceMode::Impulse,
        ForceMode::Acceleration,
        ForceMode::VelocityChange,
    ] {
        world.add_force(entity, [4.0, 5.0, 6.0], mode)?;
    }
    assert_eq!(velocity_of(&world, entity)?, [0.0, 0.0, 0.0]);
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [0.0, 0.0, 0.0]);
    assert_eq!(world.position(entity), Some([0.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn force_accumulator_is_cleared_after_each_step() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity(NO_GRAVITY);
    let entity = EntityId::new(1);
    world.add_body(entity, body(BodyRole::Dynamic, 2.0))?;
    world.add_force(entity, [4.0, 0.0, 0.0], ForceMode::Force)?;
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [1.0, 0.0, 0.0]);
    world.step(0.5)?;
    assert_eq!(velocity_of(&world, entity)?, [1.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn force_mode_dt_scaling_two_half_steps_equal_one_full_step() -> Result<(), PhysicsError> {
    let mut full = World::with_gravity(NO_GRAVITY);
    let full_entity = EntityId::new(1);
    full.add_body(full_entity, body(BodyRole::Dynamic, 2.0))?;
    full.add_force(full_entity, [4.0, 1.0, -2.0], ForceMode::Force)?;
    full.step(1.0)?;

    let mut half = World::with_gravity(NO_GRAVITY);
    let half_entity = EntityId::new(1);
    half.add_body(half_entity, body(BodyRole::Dynamic, 2.0))?;
    half.add_force(half_entity, [4.0, 1.0, -2.0], ForceMode::Force)?;
    half.step(0.5)?;
    half.add_force(half_entity, [4.0, 1.0, -2.0], ForceMode::Force)?;
    half.step(0.5)?;

    let full_velocity = velocity_of(&full, full_entity)?;
    let half_velocity = velocity_of(&half, half_entity)?;
    assert_eq!(full_velocity, [2.0, 0.5, -1.0]);
    assert!(close(half_velocity, full_velocity));
    Ok(())
}
