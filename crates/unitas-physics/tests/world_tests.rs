use unitas_core::EntityId;
use unitas_physics::{BodyDesc, PhysicsError, Shape, World};

const GRAVITY: f64 = 9.81;

fn sphere(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Sphere { radius: 0.5 },
        layer: 0,
    }
}

#[test]
fn dynamic_body_falls_under_gravity() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        BodyDesc {
            use_gravity: true,
            ..sphere([0.0, 10.0, 0.0])
        },
    )?;

    let delta_seconds = 0.01;
    let steps = 10usize;
    for _ in 0..steps {
        world.step(delta_seconds)?;
    }

    let expected_y =
        10.0 - GRAVITY * delta_seconds * delta_seconds * (steps * (steps + 1) / 2) as f64;
    let position = world.position(entity);
    assert!(position.is_some());
    let y = position.map_or(0.0, |value| value[1]);
    assert!((y - expected_y).abs() < 1e-12);
    Ok(())
}

#[test]
fn static_body_never_moves_and_never_accelerates() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        BodyDesc {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [4.0, 5.0, 6.0],
            use_gravity: true,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            layer: 0,
        },
    )?;

    for _ in 0..10 {
        world.step(0.1)?;
    }

    assert_eq!(world.position(entity), Some([1.0, 2.0, 3.0]));
    assert_eq!(world.velocity(entity), Some([4.0, 5.0, 6.0]));
    Ok(())
}

#[test]
fn body_without_partner_emits_no_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0]))?;

    world.step(0.1)?;
    world.step(0.1)?;

    assert!(world.drain_events().is_empty());
    Ok(())
}

#[test]
fn duplicate_entity_is_rejected() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);
    world.add_body(entity, sphere([0.0, 0.0, 0.0]))?;

    let outcome = world.add_body(entity, sphere([1.0, 0.0, 0.0]));

    assert_eq!(outcome, Err(PhysicsError::DuplicateEntity(entity)));
    assert_eq!(world.body_count(), 1);
    Ok(())
}

#[test]
fn invalid_body_descriptions_are_rejected() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let entity = EntityId::new(1);

    let zero_radius = BodyDesc {
        shape: Shape::Sphere { radius: 0.0 },
        ..sphere([0.0, 0.0, 0.0])
    };
    let negative_extent = BodyDesc {
        shape: Shape::Box {
            half_extents: [1.0, -1.0, 1.0],
        },
        ..sphere([0.0, 0.0, 0.0])
    };
    let infinite_mass = BodyDesc {
        mass: f64::INFINITY,
        ..sphere([0.0, 0.0, 0.0])
    };

    assert_eq!(
        world.add_body(entity, zero_radius),
        Err(PhysicsError::InvalidBody(entity))
    );
    assert_eq!(
        world.add_body(entity, negative_extent),
        Err(PhysicsError::InvalidBody(entity))
    );
    assert_eq!(
        world.add_body(entity, infinite_mass),
        Err(PhysicsError::InvalidBody(entity))
    );
    assert_eq!(world.body_count(), 0);
    Ok(())
}

#[test]
fn invalid_delta_times_are_rejected() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([0.0, 0.0, 0.0]))?;

    assert_eq!(world.step(-0.1), Err(PhysicsError::InvalidDeltaTime(-0.1)));
    assert!(matches!(
        world.step(f64::NAN),
        Err(PhysicsError::InvalidDeltaTime(_))
    ));
    assert!(matches!(
        world.step(f64::INFINITY),
        Err(PhysicsError::InvalidDeltaTime(_))
    ));
    Ok(())
}

#[test]
fn custom_gravity_accelerates_dynamic_body_along_configured_vector() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, -1.62, 0.0]);
    assert_eq!(world.gravity(), [0.0, -1.62, 0.0]);
    let entity = EntityId::new(1);
    world.add_body(
        entity,
        BodyDesc {
            use_gravity: true,
            ..sphere([0.0, 0.0, 0.0])
        },
    )?;

    world.step(1.0)?;

    assert_eq!(world.velocity(entity), Some([0.0, -1.62, 0.0]));
    assert_eq!(world.position(entity), Some([0.0, -1.62, 0.0]));
    Ok(())
}

#[test]
fn three_dimensional_and_zero_gravity_respect_use_gravity_and_static_bodies() -> Result<(), PhysicsError> {
    let mut custom_world = World::with_gravity([1.5, -2.0, 0.5]);
    let gravity_body = EntityId::new(1);
    let floating_body = EntityId::new(2);
    let static_body = EntityId::new(3);
    custom_world.add_body(
        gravity_body,
        BodyDesc {
            use_gravity: true,
            ..sphere([0.0, 0.0, 0.0])
        },
    )?;
    custom_world.add_body(
        floating_body,
        BodyDesc {
            use_gravity: false,
            velocity: [1.0, 1.0, 1.0],
            ..sphere([0.0, 0.0, 0.0])
        },
    )?;
    custom_world.add_body(
        static_body,
        BodyDesc {
            mass: 0.0,
            use_gravity: true,
            ..sphere([5.0, 5.0, 5.0])
        },
    )?;

    custom_world.step(2.0)?;

    assert_eq!(custom_world.velocity(gravity_body), Some([3.0, -4.0, 1.0]));
    assert_eq!(custom_world.position(gravity_body), Some([6.0, -8.0, 2.0]));
    assert_eq!(custom_world.velocity(floating_body), Some([1.0, 1.0, 1.0]));
    assert_eq!(custom_world.position(floating_body), Some([2.0, 2.0, 2.0]));
    assert_eq!(custom_world.velocity(static_body), Some([0.0, 0.0, 0.0]));
    assert_eq!(custom_world.position(static_body), Some([5.0, 5.0, 5.0]));

    let mut zero_world = World::with_gravity([0.0, 0.0, 0.0]);
    let zero_entity = EntityId::new(4);
    zero_world.add_body(
        zero_entity,
        BodyDesc {
            use_gravity: true,
            ..sphere([1.0, 2.0, 3.0])
        },
    )?;

    zero_world.step(1.0)?;

    assert_eq!(zero_world.velocity(zero_entity), Some([0.0, 0.0, 0.0]));
    assert_eq!(zero_world.position(zero_entity), Some([1.0, 2.0, 3.0]));
    assert_eq!(World::default().gravity(), [0.0, -9.81, 0.0]);
    Ok(())
}
