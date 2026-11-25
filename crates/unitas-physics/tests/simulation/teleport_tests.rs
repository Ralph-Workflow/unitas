use unitas_core::EntityId;
use unitas_physics::two_d::{BodyDesc2D, ColliderDesc2D, MessageKind2D, World2D};
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
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
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
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
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

#[test]
fn a_2d_teleport_moves_a_dynamic_body_immediately_and_keeps_its_velocity() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let go = EntityId::new(201);
    let body = world.add_body(BodyDesc2D::dynamic(go, [0.0, 1.0]).with_velocity([3.0, 0.0]));
    world.add_collider(ColliderDesc2D::circle_collider(go, 0.5).with_attached_body(body));

    if let Some(body) = world.body_mut(body) {
        body.position = [5.0, 2.0];
    }
    assert_eq!(world.body(body).map(|body| body.position), Some([5.0, 2.0]));
    assert_eq!(
        world.body(body).map(|body| body.linear_velocity),
        Some([3.0, 0.0])
    );

    world.step(0.5);
    assert_eq!(
        world.body(body).map(|body| body.position),
        Some([6.5, 2.0]),
        "a 2D teleport is not swept motion"
    );
}

#[test]
fn a_2d_move_position_applies_at_the_next_step_and_not_before() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let go = EntityId::new(202);
    let body = world.add_body(BodyDesc2D::dynamic(go, [0.0, 1.0]));
    world.add_collider(ColliderDesc2D::circle_collider(go, 0.5).with_attached_body(body));

    if let Some(body) = world.body_mut(body) {
        body.move_position([5.0, 2.0]);
    }
    assert_eq!(world.body(body).map(|body| body.position), Some([0.0, 1.0]));

    world.step(0.02);
    assert_eq!(world.body(body).map(|body| body.position), Some([5.0, 2.0]));
}

#[test]
fn a_2d_teleport_out_of_a_trigger_reports_the_trigger_exit() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let trigger_go = EntityId::new(203);
    let mover_go = EntityId::new(204);

    world.add_collider(ColliderDesc2D::box_collider(trigger_go, [1.0, 1.0]).with_trigger(true));
    let mover_body = world.add_body(BodyDesc2D::dynamic(mover_go, [0.0, 0.0]));
    world.add_collider(
        ColliderDesc2D::circle_collider(mover_go, 0.5).with_attached_body(mover_body),
    );

    world.step(0.02);
    let enter_messages = world.drain_messages();
    assert!(!enter_messages.is_empty());
    assert!(enter_messages
        .iter()
        .all(|message| message.kind == MessageKind2D::TriggerEnter2D));

    if let Some(body) = world.body_mut(mover_body) {
        body.position = [5.0, 0.0];
    }
    world.step(0.02);
    let exit_messages = world.drain_messages();
    assert!(!exit_messages.is_empty());
    assert!(exit_messages
        .iter()
        .all(|message| message.kind == MessageKind2D::TriggerExit2D));
}
