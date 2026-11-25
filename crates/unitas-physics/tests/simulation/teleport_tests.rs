use unitas_core::EntityId;
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

fn dynamic_ball(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        shape: Shape::Sphere { radius: 0.5 },
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.5,
        restitution: 0.0,
    }
}

#[test]
fn teleport_moves_a_dynamic_body_and_keeps_its_velocity() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let ball = EntityId::new(1);
    world.add_body(ball, dynamic_ball([0.0, 1.0, 0.0]))?;
    world.set_velocity(ball, [3.0, 0.0, 0.0])?;

    assert_eq!(world.teleport_position(ball, [5.0, 2.0, -4.0]), Ok(()));
    assert_eq!(world.position(ball), Some([5.0, 2.0, -4.0]));
    assert_eq!(world.velocity(ball), Some([3.0, 0.0, 0.0]));

    // The teleport is not swept motion, so the next step only applies velocity.
    world.step(0.5)?;
    assert_eq!(world.position(ball), Some([6.5, 2.0, -4.0]));
    Ok(())
}

#[test]
fn teleport_moves_a_kinematic_body_without_reporting_swept_motion() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let platform = EntityId::new(1);
    world.add_body(
        platform,
        BodyDesc {
            role: BodyRole::Kinematic,
            shape: Shape::Box {
                half_extents: [1.0, 0.25, 1.0],
            },
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            layer: 0,
            friction: 0.5,
            restitution: 0.0,
        },
    )?;

    world.teleport_position(platform, [10.0, 0.0, 0.0])?;
    assert_eq!(world.position(platform), Some([10.0, 0.0, 0.0]));

    world.step(0.5)?;
    assert_eq!(
        world.effective_velocity(platform),
        Some([0.0, 0.0, 0.0]),
        "a teleport must not be reported as swept motion"
    );
    Ok(())
}

#[test]
fn teleport_rejects_unknown_bodies_and_non_finite_positions() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let ball = EntityId::new(1);
    world.add_body(ball, dynamic_ball([0.0, 0.0, 0.0]))?;

    let absent = EntityId::new(2);
    assert_eq!(
        world.teleport_position(absent, [1.0, 0.0, 0.0]),
        Err(PhysicsError::InvalidBody(absent))
    );
    assert_eq!(
        world.teleport_position(ball, [f64::NAN, 0.0, 0.0]),
        Err(PhysicsError::InvalidBody(ball))
    );
    assert_eq!(world.position(ball), Some([0.0, 0.0, 0.0]));
    Ok(())
}
