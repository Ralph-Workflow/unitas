use unitas_core::EntityId;
use unitas_physics::two_d::{BodyDesc2D, ColliderDesc2D, World2D};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, PhysicsError, Shape, World};

fn static_floor() -> BodyDesc {
    BodyDesc {
        role: BodyRole::Static,
        collider_shape: ColliderShape::Primitive(Shape::Box {
            half_extents: [5.0, 0.5, 5.0],
        }),
        position: [0.0, -0.5, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn dynamic_box(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        role: BodyRole::Dynamic,
        collider_shape: ColliderShape::Primitive(Shape::Box {
            half_extents: [0.5, 0.5, 0.5],
        }),
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: true,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
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

    let resting = world.position(box_body);
    world.step(1.0 / 120.0)?;
    assert_eq!(world.position(box_body), resting);

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
        if world.body(box_body).is_some_and(|body| body.is_sleeping()) {
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

#[test]
fn configurable_sleep_threshold_controls_when_a_slow_body_falls_asleep() -> Result<(), PhysicsError>
{
    let mut default_world = World::with_gravity([0.0, 0.0, 0.0]);
    let body = EntityId::new(1);
    default_world.add_body(
        body,
        BodyDesc {
            velocity: [0.2, 0.0, 0.0],
            use_gravity: false,
            ..dynamic_box([0.0, 0.0, 0.0])
        },
    )?;
    for _ in 0..10 {
        default_world.step(0.1)?;
    }
    assert!(!default_world.is_sleeping(body));

    let mut configured_world = World::with_gravity([0.0, 0.0, 0.0]);
    configured_world.set_sleep_threshold(0.25, 0.3);
    configured_world.add_body(
        body,
        BodyDesc {
            velocity: [0.2, 0.0, 0.0],
            use_gravity: false,
            ..dynamic_box([0.0, 0.0, 0.0])
        },
    )?;

    configured_world.step(0.1)?;
    assert!(!configured_world.is_sleeping(body));
    configured_world.step(0.1)?;
    assert!(!configured_world.is_sleeping(body));
    configured_world.step(0.1)?;
    assert!(configured_world.is_sleeping(body));
    assert_eq!(configured_world.velocity(body), Some([0.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn phy_001_story_1_dynamic_box_on_static_floor_receives_enter_once_stay_while_resting_sleeps_wakes_on_force_and_exits_when_lifted(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let floor = EntityId::new(1);
    let box_body = EntityId::new(2);
    world.add_body(floor, static_floor())?;
    world.add_body(box_body, dynamic_box([0.0, 1.5, 0.0]))?;

    let dt = 1.0 / 60.0;
    let mut enter_count = 0usize;
    let mut stay_count = 0usize;
    let mut unexpected_exit_count = 0usize;
    let mut asleep_step: Option<usize> = None;

    for step_idx in 0..240 {
        world.step(dt)?;
        for event in world.drain_events() {
            assert_eq!(
                (event.a, event.b),
                (floor, box_body),
                "PHY-001 story 1: unexpected collision pair in step {step_idx}"
            );
            match event.phase {
                unitas_physics::Phase::Enter => enter_count += 1,
                unitas_physics::Phase::Stay => stay_count += 1,
                unitas_physics::Phase::Exit => unexpected_exit_count += 1,
            }
        }
        if world.is_sleeping(box_body) && asleep_step.is_none() {
            asleep_step = Some(step_idx);
            break;
        }
    }

    assert_eq!(
        unexpected_exit_count, 0,
        "PHY-001 story 1: unexpected OnCollisionExit while resting on floor"
    );

    assert_eq!(
        enter_count, 1,
        "PHY-001 story 1: dynamic box dropped onto static floor must receive OnCollisionEnter once"
    );
    assert!(
        stay_count >= 1,
        "PHY-001 story 1: dynamic box resting on static floor must receive OnCollisionStay"
    );
    assert!(
        asleep_step.is_some(),
        "PHY-001 story 1: dynamic box must fall asleep after velocity stays below sleep threshold"
    );
    assert!(world.is_sleeping(box_body));
    let resting_pos = world.position(box_body).unwrap_or([0.0; 3]);
    // Rolling physics lets the box micro-slide and settle during impact before
    // sleeping, so lateral rest position varies at the contact-recovery scale
    // (rapier prediction distance 2e-3) rather than at 1e-3. 1e-2 stays well
    // inside the box half-extent and still excludes any real slide-off.
    assert!(
        (resting_pos[0] - 0.0).abs() <= 1e-2
            && (resting_pos[1] - 0.5).abs() <= 1e-3
            && (resting_pos[2] - 0.0).abs() <= 1e-2,
        "PHY-001 story 1: resting box position {resting_pos:?} must agree with floor top within allowed floating-point variation"
    );

    world.add_force(
        box_body,
        [0.0, 480.0, 0.0],
        unitas_physics::ForceMode::Force,
    )?;
    assert!(
        !world.is_sleeping(box_body),
        "PHY-001 story 1: applying a force must wake the sleeping dynamic box"
    );

    let mut exit_count = 0usize;
    for _ in 0..30 {
        world.step(dt)?;
        for event in world.drain_events() {
            if event.phase == unitas_physics::Phase::Exit {
                exit_count += 1;
            }
        }
    }
    assert_eq!(
        exit_count, 1,
        "PHY-001 story 1: dynamic box lifted off the floor must receive OnCollisionExit once"
    );
    let lifted_pos = world.position(box_body).unwrap_or([0.0; 3]);
    assert!(
        lifted_pos[1] > resting_pos[1] + 0.2,
        "PHY-001 story 1: lifted box position {lifted_pos:?} must rise above resting position {resting_pos:?}"
    );
    Ok(())
}
