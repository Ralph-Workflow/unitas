use unitas_core::EntityId;
use unitas_physics::two_d::{
    BodyDesc2D, ColliderDesc2D, ContactFilter2D, PhysicsMaterial2D, PlatformEffector2D, Shape2D,
    World2D,
};

#[test]
fn capsule_collider_0_75_by_1_375_rests_on_a_platform_and_reports_its_shape() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let floor_go = EntityId::new(1);
    let rider_go = EntityId::new(2);

    let floor_body = world.add_body(BodyDesc2D::static_body(floor_go, [0.0, 0.0]));
    let floor_col = world.add_collider(
        ColliderDesc2D::box_collider(floor_go, [5.0, 0.5]).with_attached_body(floor_body),
    );

    let rider_body = world.add_body(BodyDesc2D::dynamic(rider_go, [0.0, 2.0]));
    let rider_col = world.add_collider(
        ColliderDesc2D::capsule_collider(rider_go, 0.375, 1.375, 1).with_attached_body(rider_body),
    );

    assert!(matches!(
        world.collider(rider_col).map(|collider| &collider.shape),
        Some(Shape2D::Capsule { radius, height, .. })
            if (*radius - 0.375).abs() < 1e-6 && (*height - 1.375).abs() < 1e-6
    ));

    for _ in 0..120 {
        world.step(0.02);
    }

    let resting_y = world.body(rider_body).map_or(2.0, |body| body.position[1]);
    assert!(
        (resting_y - 1.1875).abs() < 0.05,
        "a vertical 0.75x1.375 capsule must rest with its centre 0.6875 above the platform top, got {resting_y}"
    );
    assert!(
        world.is_touching(rider_col, floor_col),
        "the capsule must report a contact with the platform it rests on"
    );

    let contacts = world.get_contacts(rider_col);
    assert!(
        contacts.iter().any(|contact| contact.normal[1] > 0.9),
        "the capsule contact normal must point up, got {contacts:?}"
    );
}

#[test]
fn frozen_rotation_holds_the_body_rotation_under_torque() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let frozen_go = EntityId::new(10);
    let free_go = EntityId::new(11);

    let frozen_body =
        world.add_body(BodyDesc2D::dynamic(frozen_go, [0.0, 0.0]).with_freeze_rotation(true));
    let free_body = world.add_body(BodyDesc2D::dynamic(free_go, [2.0, 0.0]));

    for _ in 0..10 {
        if let Some(body) = world.body_mut(frozen_body) {
            body.add_torque(10.0);
        }
        if let Some(body) = world.body_mut(free_body) {
            body.add_torque(10.0);
        }
        world.step(0.02);
    }

    let frozen = world.body(frozen_body);
    assert!(
        frozen.is_some_and(|body| body.rotation.abs() < 1e-6),
        "a frozen body must not rotate under torque"
    );
    assert!(
        frozen.is_some_and(|body| body.angular_velocity.abs() < 1e-6),
        "a frozen body must not keep an angular velocity"
    );

    let free = world.body(free_body);
    assert!(
        free.is_some_and(|body| body.rotation > 0.01),
        "an unfrozen body must still rotate under torque"
    );
}

#[test]
fn box_cast_reports_hits_filtered_by_contact_filter_and_normal_angle() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let wall_go = EntityId::new(1);
    let marker_go = EntityId::new(2);

    let wall_body = world.add_body(BodyDesc2D::static_body(wall_go, [5.0, 0.0]));
    let wall_col = world.add_collider(
        ColliderDesc2D::box_collider(wall_go, [0.25, 5.0])
            .with_attached_body(wall_body)
            .with_layer(2),
    );
    let marker_col = world.add_collider(
        ColliderDesc2D::circle_collider(marker_go, 0.25)
            .with_offset([0.0, 3.0])
            .with_layer(1)
            .with_trigger(true),
    );

    let hits = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::no_filter(),
    );
    assert_eq!(hits.len(), 1, "the cast must report the wall it reaches");
    assert_eq!(hits[0].collider, wall_col);
    assert!(
        (hits[0].distance - 4.5).abs() < 1e-3,
        "the cast must stop when the box reaches the wall face, got {}",
        hits[0].distance
    );
    assert!(
        hits[0].normal[0] < -0.99 && hits[0].normal[1].abs() < 1e-3,
        "the wall hit normal must point back at the caster, got {:?}",
        hits[0].normal
    );

    assert_box_cast_angle_and_layer_filters(&world, marker_col);
}

fn assert_box_cast_angle_and_layer_filters(
    world: &World2D,
    marker_col: unitas_physics::two_d::ColliderId2D,
) {
    let upward_only = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::default().with_normal_angle(0.0, 90.0),
    );
    assert!(
        upward_only.is_empty(),
        "a normal-angle filter that excludes the wall normal must drop the hit"
    );

    let side_only = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::default().with_normal_angle(90.0, 270.0),
    );
    assert_eq!(
        side_only.len(),
        1,
        "the wall normal must pass the side filter"
    );

    let wrong_layer = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::default().with_layer_mask(1 << 1),
    );
    assert!(
        wrong_layer.is_empty(),
        "a layer mask that excludes the wall layer must drop the hit"
    );

    let right_layer = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::default().with_layer_mask(1 << 2),
    );
    assert_eq!(right_layer.len(), 1);

    let solid_only = world.box_cast(
        [0.0, 0.0],
        [0.25, 0.25],
        0.0,
        [1.0, 0.0],
        20.0,
        ContactFilter2D::default().with_triggers(false),
    );
    assert_eq!(solid_only.len(), 1);
    assert!(
        solid_only.iter().all(|hit| hit.collider != marker_col),
        "a trigger collider must not be reported when triggers are filtered out"
    );
}

fn side_contact_velocity(use_side_friction: bool, use_side_bounce: bool) -> ([f32; 2], [f32; 2]) {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let platform_go = EntityId::new(1);
    let body_go = EntityId::new(2);

    let platform_body = world.add_body(BodyDesc2D::static_body(platform_go, [0.0, 0.0]));
    let platform_col = world.add_collider(
        ColliderDesc2D::box_collider(platform_go, [1.0, 0.25])
            .with_attached_body(platform_body)
            .with_used_by_effector(true)
            .with_material(PhysicsMaterial2D::new(0.4, 0.5)),
    );
    world.attach_effector(
        platform_col,
        PlatformEffector2D::default()
            .with_side_arc(60.0)
            .with_side_friction(use_side_friction)
            .with_side_bounce(use_side_bounce),
    );

    let body_id =
        world.add_body(BodyDesc2D::dynamic(body_go, [1.2, 0.0]).with_velocity([-5.0, -3.0]));
    world.add_collider(
        ColliderDesc2D::box_collider(body_go, [0.25, 0.25])
            .with_attached_body(body_id)
            .with_material(PhysicsMaterial2D::new(0.4, 0.5)),
    );

    world.step(0.02);
    let body = world.body(body_id);
    (
        body.map_or([0.0, 0.0], |body| body.linear_velocity),
        body.map_or([0.0, 0.0], |body| body.position),
    )
}

#[test]
fn platform_effector_side_friction_and_side_bounce_are_honored() {
    let (friction_velocity, _) = side_contact_velocity(true, false);
    let (plain_velocity, _) = side_contact_velocity(false, false);
    assert!(
        friction_velocity[1] > plain_velocity[1] + 0.5,
        "side friction must slow the tangential slide along the platform side, got {friction_velocity:?} against {plain_velocity:?}"
    );
    assert!(
        plain_velocity[1] < -2.5,
        "without side friction the tangential slide must survive, got {plain_velocity:?}"
    );

    let (bounce_velocity, _) = side_contact_velocity(false, true);
    assert!(
        bounce_velocity[0] > 1.0,
        "side bounce must push the body back off the platform side, got {bounce_velocity:?}"
    );
    assert!(
        plain_velocity[0].abs() < 1.0,
        "without side bounce the body must not rebound off the platform side, got {plain_velocity:?}"
    );
}

fn build_rider_stick_world(
    stick: bool,
) -> (
    World2D,
    unitas_physics::two_d::BodyId2D,
    unitas_physics::two_d::BodyId2D,
) {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let carrier_go = EntityId::new(1);
    let rider_go = EntityId::new(2);

    let carrier_body =
        world.add_body(BodyDesc2D::kinematic(carrier_go, [0.0, 0.0]).with_velocity([2.0, 0.0]));
    world.add_collider(
        ColliderDesc2D::box_collider(carrier_go, [2.0, 0.25]).with_attached_body(carrier_body),
    );
    let rider_body = world.add_body(BodyDesc2D::dynamic(rider_go, [0.0, 0.5]));
    world.add_collider(
        ColliderDesc2D::box_collider(rider_go, [0.25, 0.25]).with_attached_body(rider_body),
    );
    if stick {
        assert!(world.stick_rider(rider_body, carrier_body));
        assert_eq!(world.rider_carrier(rider_body), Some(carrier_body));
    }
    (world, carrier_body, rider_body)
}

#[test]
fn rider_stick_carries_the_rider_over_a_waypoint_leg_without_drift() {
    let (mut world, carrier_body, rider_body) = build_rider_stick_world(true);
    for _ in 0..25 {
        world.step(0.02);
    }

    let rider = world
        .body(rider_body)
        .map_or([0.0, 0.0], |body| body.position);
    let carrier = world
        .body(carrier_body)
        .map_or([0.0, 0.0], |body| body.position);
    let drift = ((rider[0] - carrier[0]).powi(2) + (rider[1] - carrier[1] - 0.5).powi(2)).sqrt();
    assert!(
        drift < 0.0625,
        "a stuck rider must keep its offset over the waypoint leg, drifted {drift} ({rider:?} on {carrier:?})"
    );

    let (mut loose, loose_carrier, loose_rider) = build_rider_stick_world(false);
    for _ in 0..25 {
        loose.step(0.02);
    }
    let loose_rider_position = loose
        .body(loose_rider)
        .map_or([0.0, 0.0], |body| body.position);
    let loose_carrier_position = loose
        .body(loose_carrier)
        .map_or([0.0, 0.0], |body| body.position);
    let loose_drift = ((loose_rider_position[0] - loose_carrier_position[0]).powi(2)
        + (loose_rider_position[1] - loose_carrier_position[1] - 0.5).powi(2))
    .sqrt();
    assert!(
        loose_drift > 0.0625,
        "an unsticked rider must drift off the carrier, drifted {loose_drift}"
    );
}
