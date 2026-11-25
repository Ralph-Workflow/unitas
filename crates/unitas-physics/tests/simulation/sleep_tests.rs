use unitas_core::EntityId;
use unitas_physics::two_d::{BodyDesc2D, ColliderDesc2D, World2D};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

fn static_floor() -> BodyDesc {
    BodyDesc {
        role: BodyRole::Static,
        shape: Shape::Box {
            half_extents: [5.0, 0.5, 5.0],
        },
        position: [0.0, -0.5, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
    }
}

fn dynamic_box(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        shape: Shape::Box {
            half_extents: [0.5, 0.5, 0.5],
        },
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: true,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
    }
}

#[test]
fn a_box_dropped_onto_a_floor_falls_asleep_and_wakes_when_motion_is_applied(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let floor = EntityId::new(1);
    let box_body = EntityId::new(2);
    world.add_body(floor, static_floor())?;
    world.add_body(box_body, dynamic_box([0.0, 3.0, 0.0]))?;

    let mut asleep = false;
    for _ in 0..600 {
        world.step(1.0 / 120.0)?;
        if world.is_sleeping(box_body) {
            asleep = true;
            break;
        }
    }
    assert!(asleep, "a resting box must fall asleep");
    assert_eq!(world.velocity(box_body), Some([0.0, 0.0, 0.0]));

    // The body stays where it rested while it sleeps.
    let resting = world.position(box_body);
    world.step(1.0 / 120.0)?;
    assert_eq!(world.position(box_body), resting);

    // Applying motion wakes it again.
    world.set_velocity(box_body, [1.0, 0.0, 0.0])?;
    assert!(!world.is_sleeping(box_body));
    world.step(1.0 / 120.0)?;
    assert!(world.position(box_body).unwrap_or([0.0; 3])[0] > resting.unwrap_or([0.0; 3])[0]);
    Ok(())
}

#[test]
fn a_sleeping_body_is_woken_by_a_body_that_reaches_it() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.set_sleep_threshold(0.05, 0.5);
    let floor = EntityId::new(1);
    let resting = EntityId::new(2);
    let pusher = EntityId::new(3);
    world.add_body(floor, static_floor())?;
    world.add_body(resting, dynamic_box([0.0, 1.0, 0.0]))?;
    world.add_body(
        pusher,
        BodyDesc {
            velocity: [4.0, 0.0, 0.0],
            use_gravity: false,
            ..dynamic_box([-3.0, 1.0, 0.0])
        },
    )?;

    world.sleep(resting)?;
    assert!(world.is_sleeping(resting));

    for _ in 0..600 {
        world.step(1.0 / 120.0)?;
        if !world.is_sleeping(resting) {
            break;
        }
    }
    assert!(
        !world.is_sleeping(resting),
        "a moving body must wake the body it reaches"
    );
    Ok(())
}

#[test]
fn sleep_and_wake_report_unknown_bodies_without_panicking() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let absent = EntityId::new(9);
    assert_eq!(world.sleep(absent), Err(PhysicsError::InvalidBody(absent)));
    assert!(!world.is_sleeping(absent));
    world.wake_up(absent);
    Ok(())
}

#[test]
fn a_2d_box_dropped_onto_a_floor_falls_asleep_and_wakes_when_a_force_is_applied() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let floor_go = EntityId::new(101);
    let box_go = EntityId::new(102);

    let floor_body = world.add_body(BodyDesc2D::static_body(floor_go, [0.0, 0.0]));
    world.add_collider(
        ColliderDesc2D::box_collider(floor_go, [5.0, 0.5]).with_attached_body(floor_body),
    );
    let box_body = world.add_body(BodyDesc2D::dynamic(box_go, [0.0, 3.0]));
    world.add_collider(ColliderDesc2D::circle_collider(box_go, 0.5).with_attached_body(box_body));

    let mut asleep = false;
    for _ in 0..600 {
        world.step(1.0 / 120.0);
        if world.body(box_body).map(|body| body.is_sleeping()) == Some(true) {
            asleep = true;
            break;
        }
    }
    assert!(asleep, "a resting 2D box must fall asleep");
    assert_eq!(
        world.body(box_body).map(|body| body.linear_velocity),
        Some([0.0, 0.0])
    );

    let resting = world.body(box_body).map(|body| body.position);
    world.step(1.0 / 120.0);
    assert_eq!(world.body(box_body).map(|body| body.position), resting);

    if let Some(body) = world.body_mut(box_body) {
        body.add_force([10.0, 0.0]);
    }
    assert_eq!(
        world.body(box_body).map(|body| body.is_sleeping()),
        Some(false)
    );

    world.step(1.0 / 120.0);
    assert!(
        world.body(box_body).map_or(0.0, |body| body.position[0])
            > resting.map_or(0.0, |position| position[0])
    );
}

#[test]
fn a_2d_body_put_to_sleep_stops_moving_until_it_is_woken() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let go = EntityId::new(103);
    let body = world.add_body(BodyDesc2D::dynamic(go, [0.0, 5.0]));
    world.add_collider(ColliderDesc2D::circle_collider(go, 0.5).with_attached_body(body));

    world.step(0.02);
    assert_eq!(world.body(body).map(|body| body.is_sleeping()), Some(false));

    if let Some(body) = world.body_mut(body) {
        body.sleep();
    }
    assert_eq!(world.body(body).map(|body| body.is_sleeping()), Some(true));
    assert_eq!(
        world.body(body).map(|body| body.linear_velocity),
        Some([0.0, 0.0])
    );

    let resting = world.body(body).map(|body| body.position);
    world.step(0.5);
    assert_eq!(
        world.body(body).map(|body| body.position),
        resting,
        "a sleeping 2D body must not move under gravity"
    );

    if let Some(body) = world.body_mut(body) {
        body.wake_up();
    }
    assert_eq!(world.body(body).map(|body| body.is_sleeping()), Some(false));

    world.step(0.02);
    assert!(
        world.body(body).map_or(0.0, |body| body.position[1])
            < resting.map_or(0.0, |position| position[1])
    );
}
