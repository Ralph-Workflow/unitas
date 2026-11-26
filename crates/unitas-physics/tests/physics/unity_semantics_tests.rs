use unitas_core::EntityId;
use unitas_physics::{
    combine_materials, BodyDesc, BodyInterpolation, BodyRole, BounceCombine,
    CollisionDetectionMode3D, FrictionCombine, PhysicsError, PhysicsMaterial3D, Shape,
    SolverSettings, World,
};

const INCOMING_SPEED: f64 = 10.0;

fn material(friction: f64, restitution: f64, combine: BounceCombine) -> PhysicsMaterial3D {
    PhysicsMaterial3D {
        friction,
        restitution,
        friction_combine: FrictionCombine::Average,
        restitution_combine: combine,
    }
}

fn bumper_desc() -> BodyDesc {
    BodyDesc {
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box {
            half_extents: [0.05, 2.0, 2.0],
        },
        layer: 0,
        role: BodyRole::Static,
        friction: 0.0,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn ball_desc() -> BodyDesc {
    BodyDesc {
        position: [-3.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [INCOMING_SPEED, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Sphere { radius: 0.25 },
        layer: 0,
        role: BodyRole::Dynamic,
        friction: 0.0,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn bounce_speed(
    bumper_restitution: f64,
    ball_restitution: f64,
    combine: BounceCombine,
) -> Result<f64, PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let bumper = EntityId::new(1);
    let ball = EntityId::new(2);
    world.add_body_with_material(
        bumper,
        bumper_desc(),
        Some(material(0.0, bumper_restitution, combine)),
    )?;
    world.add_body_with_material(
        ball,
        ball_desc(),
        Some(material(0.0, ball_restitution, BounceCombine::Average)),
    )?;
    for _ in 0..240 {
        world.step(1.0 / 60.0)?;
        if let Some(velocity) = world.velocity(ball) {
            if velocity[0] < -0.001 {
                return Ok(-velocity[0]);
            }
        }
    }
    Ok(0.0)
}

#[test]
fn max_bounce_combine_keeps_the_authored_restitution_above_one() -> Result<(), PhysicsError> {
    let outgoing = bounce_speed(1.5, 0.0, BounceCombine::Maximum)?;
    assert!(
        (outgoing - 15.0).abs() < 0.05,
        "expected 1.5 times the incoming speed, got {outgoing}"
    );
    Ok(())
}

#[test]
fn average_bounce_combine_averages_the_restitutions() -> Result<(), PhysicsError> {
    let outgoing = bounce_speed(1.5, 0.0, BounceCombine::Average)?;
    assert!(
        (outgoing - 7.5).abs() < 0.05,
        "expected the average restitution 0.75 to give 7.5, got {outgoing}"
    );
    Ok(())
}

#[test]
fn minimum_bounce_combine_takes_the_lower_restitution() -> Result<(), PhysicsError> {
    let outgoing = bounce_speed(1.5, 0.0, BounceCombine::Minimum)?;
    assert!(
        outgoing < 0.05,
        "expected the minimum restitution 0 to stop the bounce, got {outgoing}"
    );
    Ok(())
}

#[test]
fn maximum_bounce_combine_takes_the_higher_restitution() -> Result<(), PhysicsError> {
    let outgoing = bounce_speed(0.5, 1.0, BounceCombine::Maximum)?;
    assert!(
        (outgoing - 10.0).abs() < 0.05,
        "expected the maximum restitution 1 to keep the speed, got {outgoing}"
    );
    Ok(())
}

#[test]
fn multiply_bounce_combine_multiplies_the_restitutions() -> Result<(), PhysicsError> {
    let outgoing = bounce_speed(1.5, 0.5, BounceCombine::Multiply)?;
    assert!(
        (outgoing - 7.5).abs() < 0.05,
        "expected the product restitution 0.75 to give 7.5, got {outgoing}"
    );
    Ok(())
}

#[test]
fn average_friction_combine_averages_the_frictions() {
    let first = PhysicsMaterial3D {
        friction: 0.2,
        restitution: 0.0,
        friction_combine: FrictionCombine::Average,
        restitution_combine: BounceCombine::Average,
    };
    let second = PhysicsMaterial3D {
        friction: 0.8,
        restitution: 0.0,
        friction_combine: FrictionCombine::Average,
        restitution_combine: BounceCombine::Average,
    };
    let combined = combine_materials(&first, &second);
    assert!((combined.friction - 0.5).abs() < 1e-9);
    assert_eq!(combined.friction_combine, FrictionCombine::Average);
}

#[test]
fn minimum_friction_combine_takes_the_lower_friction() {
    let first = PhysicsMaterial3D {
        friction: 0.2,
        restitution: 0.0,
        friction_combine: FrictionCombine::Minimum,
        restitution_combine: BounceCombine::Average,
    };
    let second = PhysicsMaterial3D {
        friction: 0.8,
        restitution: 0.0,
        friction_combine: FrictionCombine::Minimum,
        restitution_combine: BounceCombine::Average,
    };
    let combined = combine_materials(&first, &second);
    assert!((combined.friction - 0.2).abs() < 1e-9);
    assert_eq!(combined.friction_combine, FrictionCombine::Minimum);
}

#[test]
fn multiply_friction_combine_multiplies_the_frictions() {
    let first = PhysicsMaterial3D {
        friction: 0.2,
        restitution: 0.0,
        friction_combine: FrictionCombine::Multiply,
        restitution_combine: BounceCombine::Average,
    };
    let second = PhysicsMaterial3D {
        friction: 0.8,
        restitution: 0.0,
        friction_combine: FrictionCombine::Multiply,
        restitution_combine: BounceCombine::Average,
    };
    let combined = combine_materials(&first, &second);
    assert!((combined.friction - 0.16).abs() < 1e-9);
    assert_eq!(combined.friction_combine, FrictionCombine::Multiply);
}

#[test]
fn maximum_friction_combine_takes_the_higher_friction() {
    let first = PhysicsMaterial3D {
        friction: 0.2,
        restitution: 0.0,
        friction_combine: FrictionCombine::Maximum,
        restitution_combine: BounceCombine::Average,
    };
    let second = PhysicsMaterial3D {
        friction: 0.8,
        restitution: 0.0,
        friction_combine: FrictionCombine::Maximum,
        restitution_combine: BounceCombine::Average,
    };
    let combined = combine_materials(&first, &second);
    assert!((combined.friction - 0.8).abs() < 1e-9);
    assert_eq!(combined.friction_combine, FrictionCombine::Maximum);
}

#[test]
fn the_more_permissive_combine_rule_wins() {
    let bumper = PhysicsMaterial3D {
        friction: 0.0,
        restitution: 1.5,
        friction_combine: FrictionCombine::Maximum,
        restitution_combine: BounceCombine::Maximum,
    };
    let ball = PhysicsMaterial3D {
        friction: 0.6,
        restitution: 0.0,
        friction_combine: FrictionCombine::Average,
        restitution_combine: BounceCombine::Average,
    };
    let combined = combine_materials(&bumper, &ball);
    assert_eq!(combined.restitution_combine, BounceCombine::Maximum);
    assert_eq!(combined.friction_combine, FrictionCombine::Maximum);
    assert!((combined.restitution - 1.5).abs() < 1e-9);
    assert!((combined.friction - 0.6).abs() < 1e-9);
}

#[test]
fn the_permissive_combine_rule_wins_over_the_stricter_one() {
    let bumper = PhysicsMaterial3D {
        friction: 0.0,
        restitution: 1.5,
        friction_combine: FrictionCombine::Average,
        restitution_combine: BounceCombine::Average,
    };
    let ball = PhysicsMaterial3D {
        friction: 0.6,
        restitution: 0.0,
        friction_combine: FrictionCombine::Multiply,
        restitution_combine: BounceCombine::Multiply,
    };
    let combined = combine_materials(&bumper, &ball);
    assert_eq!(combined.restitution_combine, BounceCombine::Multiply);
    assert_eq!(combined.friction_combine, FrictionCombine::Multiply);
    assert!((combined.restitution - 0.0).abs() < 1e-9);
    assert!((combined.friction - 0.0).abs() < 1e-9);
}

fn slide_distance(friction: f64, combine: FrictionCombine) -> Result<f64, PhysicsError> {
    let mut world = World::with_gravity([0.0, -9.81, 0.0]);
    let floor = EntityId::new(1);
    let ball = EntityId::new(2);
    let mut floor_desc = bumper_desc();
    floor_desc.shape = Shape::Box {
        half_extents: [20.0, 0.5, 20.0],
    };
    floor_desc.position = [0.0, -0.5, 0.0];
    world.add_body_with_material(
        floor,
        floor_desc,
        Some(PhysicsMaterial3D {
            friction,
            restitution: 0.0,
            friction_combine: combine,
            restitution_combine: BounceCombine::Average,
        }),
    )?;
    let mut ball_desc = ball_desc();
    ball_desc.position = [0.0, 0.25, 0.0];
    ball_desc.velocity = [6.0, 0.0, 0.0];
    ball_desc.use_gravity = true;
    ball_desc.friction = friction;
    world.add_body_with_material(
        ball,
        ball_desc,
        Some(PhysicsMaterial3D {
            friction,
            restitution: 0.0,
            friction_combine: combine,
            restitution_combine: BounceCombine::Average,
        }),
    )?;
    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }
    Ok(world.position(ball).map_or(0.0, |position| position[0]))
}

#[test]
fn the_combined_friction_reaches_the_simulation() -> Result<(), PhysicsError> {
    let slippery = slide_distance(0.16, FrictionCombine::Multiply)?;
    let grippy = slide_distance(0.8, FrictionCombine::Maximum)?;
    assert!(
        slippery > grippy,
        "expected the slippery pair to slide further than the grippy one: {slippery} vs {grippy}"
    );
    Ok(())
}

fn floor_world() -> Result<(World, EntityId, EntityId), PhysicsError> {
    let mut world = World::with_gravity([0.0, -9.81, 0.0]);
    let floor = EntityId::new(1);
    let ball = EntityId::new(2);
    let mut floor_desc = bumper_desc();
    floor_desc.shape = Shape::Box {
        half_extents: [10.0, 0.5, 10.0],
    };
    floor_desc.position = [0.0, -0.5, 0.0];
    world.add_body(floor, floor_desc)?;
    let mut ball_desc = ball_desc();
    ball_desc.position = [0.0, 1.0, 0.0];
    ball_desc.velocity = [0.0; 3];
    ball_desc.use_gravity = true;
    world.add_body(ball, ball_desc)?;
    Ok((world, floor, ball))
}

#[test]
fn the_collision_payload_reports_contacts_and_impulse() -> Result<(), PhysicsError> {
    let (mut world, floor, ball) = floor_world()?;
    let mut payload = None;
    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
        if let Some(found) = world.collision_payload(floor, ball) {
            if found.total_impulse > 0.0 {
                payload = Some(found.clone());
                break;
            }
        }
    }
    let payload = payload.ok_or(PhysicsError::InvalidBody(ball))?;
    assert!(
        !payload.contacts.is_empty(),
        "expected at least one contact point"
    );
    assert!(
        payload.total_impulse > 0.0,
        "expected a non-zero total impulse, got {}",
        payload.total_impulse
    );
    let contact = payload.contacts[0];
    assert!(
        contact.impulse > 0.0,
        "expected a non-zero contact impulse, got {}",
        contact.impulse
    );
    assert!(
        contact.normal[1] > 0.9,
        "expected the normal to point from the floor to the ball, got {:?}",
        contact.normal
    );
    assert!(
        contact.point[1] < 0.1,
        "expected the contact point on the floor surface, got {:?}",
        contact.point
    );
    assert!(
        payload.relative_velocity[1] > 0.0,
        "expected the relative velocity to point away from the falling ball, got {:?}",
        payload.relative_velocity
    );
    Ok(())
}

#[test]
fn the_collision_payload_reports_a_resting_contact() -> Result<(), PhysicsError> {
    let (mut world, floor, ball) = floor_world()?;
    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }
    let payload = world
        .collision_payload(floor, ball)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    assert!(
        payload.total_impulse > 0.0,
        "expected the resting contact to carry the ball's weight, got {}",
        payload.total_impulse
    );
    Ok(())
}

#[test]
fn the_collision_payload_is_reachable_in_either_order() -> Result<(), PhysicsError> {
    let (mut world, floor, ball) = floor_world()?;
    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }
    let forward = world
        .collision_payload(floor, ball)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    let backward = world
        .collision_payload(ball, floor)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    assert_eq!(forward, backward);
    Ok(())
}

fn deterministic_run() -> Result<Vec<[f64; 3]>, PhysicsError> {
    let mut world = World::with_gravity([0.0, -9.81, 0.0]);
    let ground = EntityId::new(1);
    let mut ground_desc = bumper_desc();
    ground_desc.shape = Shape::Box {
        half_extents: [20.0, 0.5, 20.0],
    };
    ground_desc.position = [0.0, -0.5, 0.0];
    world.add_body(ground, ground_desc)?;
    for index in 0..6u64 {
        let entity = EntityId::new(index + 2);
        let mut desc = ball_desc();
        desc.position = [index as f64 * 0.6, 1.0 + index as f64 * 0.4, 0.0];
        desc.velocity = [1.0, 0.0, 0.5];
        desc.shape = Shape::Box {
            half_extents: [0.25, 0.25, 0.25],
        };
        desc.mass = 1.0 + index as f64;
        world.add_body(entity, desc)?;
    }
    for _ in 0..180 {
        world.step(1.0 / 60.0)?;
    }
    Ok((2..8u64)
        .map(|index| world.position(EntityId::new(index)).unwrap_or([0.0; 3]))
        .collect())
}

#[test]
fn two_identical_runs_produce_identical_positions() -> Result<(), PhysicsError> {
    let first = deterministic_run()?;
    let second = deterministic_run()?;
    assert_eq!(first.len(), second.len());
    for (index, (first, second)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(
            first, second,
            "run {index} diverged: {first:?} vs {second:?}"
        );
    }
    Ok(())
}

#[test]
fn solver_settings_reach_the_world() {
    let mut world = World::new();
    let settings = SolverSettings {
        num_solver_iterations: 8,
        num_additional_friction_iterations: 2,
        num_internal_pgs_iterations: 3,
        contact_offset: 0.02,
        allowed_linear_error: 0.01,
        max_depenetration_velocity: 5.0,
        deterministic_accumulation: false,
    };
    world.set_solver_settings(settings);
    assert_eq!(world.solver_settings(), settings);
}

#[test]
fn drag_slows_a_moving_body() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let ball = EntityId::new(1);
    let mut desc = ball_desc();
    desc.velocity = [10.0, 0.0, 0.0];
    world.add_body(ball, desc)?;
    world.set_body_drag(ball, 2.0, 0.0);
    for _ in 0..60 {
        world.step(1.0 / 60.0)?;
    }
    let velocity = world
        .velocity(ball)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    assert!(
        velocity[0] < 9.0,
        "expected the drag to slow the body, got {velocity:?}"
    );
    Ok(())
}

#[test]
fn interpolation_modes_differ_in_the_reported_pose() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let ball = EntityId::new(1);
    let mut desc = ball_desc();
    desc.velocity = [10.0, 0.0, 0.0];
    world.add_body(ball, desc)?;
    world.step(1.0 / 60.0)?;
    let exact = world
        .position(ball)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    world.set_body_interpolation(ball, BodyInterpolation::None);
    let none = world
        .interpolated_pose(ball, 0.5)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    world.set_body_interpolation(ball, BodyInterpolation::Interpolate);
    let interpolate = world
        .interpolated_pose(ball, 0.5)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    assert_eq!(none.0, exact);
    assert!(interpolate.0[0] < exact[0]);
    Ok(())
}

#[test]
fn the_max_angular_velocity_caps_the_spin() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let ball = EntityId::new(1);
    let mut desc = ball_desc();
    desc.velocity = [0.0; 3];
    world.add_body(ball, desc)?;
    world.set_body_max_angular_velocity(ball, 1.0);
    world.set_angular_velocity(ball, [0.0, 0.0, 100.0])?;
    let angular = world
        .angular_velocity(ball)
        .ok_or(PhysicsError::InvalidBody(ball))?;
    assert!(
        angular[2] <= 1.0,
        "expected the spin to be capped at 1, got {angular:?}"
    );
    Ok(())
}

#[test]
fn a_layer_override_excludes_a_layer_pair() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    let mut left_desc = bumper_desc();
    left_desc.layer = 1;
    world.add_body(left, left_desc)?;
    let mut right_desc = bumper_desc();
    right_desc.position = [0.05, 0.0, 0.0];
    right_desc.layer = 2;
    right_desc.role = BodyRole::Dynamic;
    right_desc.mass = 1.0;
    world.add_body(right, right_desc)?;
    world.set_body_layer_overrides(left, None, Some(1u32 << 2), 10);
    for _ in 0..10 {
        world.step(1.0 / 60.0)?;
    }
    let events = world.drain_events();
    assert!(
        events.is_empty(),
        "expected the excluded layer pair to report no collision, got {events:?}"
    );
    Ok(())
}

#[test]
fn a_layer_override_includes_a_layer_pair() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    let mut left_desc = bumper_desc();
    left_desc.layer = 1;
    world.add_body(left, left_desc)?;
    let mut right_desc = bumper_desc();
    right_desc.position = [0.05, 0.0, 0.0];
    right_desc.layer = 2;
    right_desc.role = BodyRole::Dynamic;
    right_desc.mass = 1.0;
    world.add_body(right, right_desc)?;
    world.set_body_layer_overrides(left, Some(1u32 << 2), None, 10);
    for _ in 0..10 {
        world.step(1.0 / 60.0)?;
    }
    let events = world.drain_events();
    assert!(
        !events.is_empty(),
        "expected the included layer pair to report a collision"
    );
    Ok(())
}
