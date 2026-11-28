use unitas_core::EntityId;
use unitas_physics::two_d::{
    AreaEffector2D, BodyDesc2D, ColliderDesc2D, MessageKind2D, PlatformEffector2D, PointEffector2D,
    SurfaceEffector2D, World2D,
};

#[test]
fn phy_002_story_5_platform_effector_allows_upward_pass_through_without_events_and_lands_from_above(
) {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let platform_go = EntityId::new(1);
    let player_go = EntityId::new(2);

    let platform_body = world.add_body(BodyDesc2D::static_body(platform_go, [0.0, 0.0]));
    let platform_col = world.add_collider(
        ColliderDesc2D::box_collider(platform_go, [3.0, 0.25])
            .with_attached_body(platform_body)
            .with_used_by_effector(true),
    );
    world.attach_effector(platform_col, PlatformEffector2D::default());

    let player_body =
        world.add_body(BodyDesc2D::dynamic(player_go, [0.0, -1.5]).with_velocity([0.0, 12.0]));
    let player_col = world.add_collider(
        ColliderDesc2D::box_collider(player_go, [0.4, 0.4]).with_attached_body(player_body),
    );

    assert_platform_upward_pass(&mut world, player_body, player_col, platform_col);
    assert_platform_landing_from_above(
        &mut world,
        player_go,
        player_body,
        player_col,
        platform_col,
    );
}

fn assert_platform_upward_pass(
    world: &mut World2D,
    player_body: unitas_physics::two_d::BodyId2D,
    player_col: unitas_physics::two_d::ColliderId2D,
    platform_col: unitas_physics::two_d::ColliderId2D,
) {
    let mut max_y = -1.5f32;
    let mut passed_above = false;
    for _ in 0..35 {
        world.step(0.02);
        let y = world.body(player_body).map_or(-1.5, |b| b.position[1]);
        if y > max_y {
            max_y = y;
        }
        let ascent_messages = world.drain_messages();
        assert!(
            ascent_messages.is_empty(),
            "PHY-002 story 5: the upward pass through a one-way platform must fire zero collision or trigger messages, got {ascent_messages:?}"
        );
        assert!(
            !world.is_touching(player_col, platform_col),
            "PHY-002 story 5: the body must not touch the one-way platform while passing upward"
        );
        if y > 0.8 {
            passed_above = true;
            break;
        }
    }
    assert!(
        passed_above,
        "player must pass above platform (max_y={max_y})"
    );
}

fn assert_platform_landing_from_above(
    world: &mut World2D,
    player_go: EntityId,
    player_body: unitas_physics::two_d::BodyId2D,
    player_col: unitas_physics::two_d::ColliderId2D,
    platform_col: unitas_physics::two_d::ColliderId2D,
) {
    let mut landed_enter = false;
    let mut landed_stay = false;
    for _ in 0..160 {
        world.step(0.02);
        for msg in world.drain_messages() {
            if msg.recipient == player_go {
                if msg.kind == MessageKind2D::CollisionEnter2D {
                    landed_enter = true;
                }
                if msg.kind == MessageKind2D::CollisionStay2D {
                    landed_stay = true;
                }
            }
        }
    }
    assert!(
        landed_enter,
        "player landing from above must receive OnCollisionEnter2D"
    );
    assert!(
        landed_stay,
        "player resting on platform must receive OnCollisionStay2D"
    );
    assert!(world.is_touching(player_col, platform_col));
    let final_y = world.body(player_body).map_or(0.0, |b| b.position[1]);
    assert!(
        (final_y - 0.65).abs() < 0.1,
        "player should rest on top of platform near y=0.65, got {final_y}"
    );
}

#[test]
fn platform_effector_respects_collider_mask_and_surface_arc() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let platform_go = EntityId::new(1);
    let ignored_go = EntityId::new(2);

    let platform_col = world.add_collider(
        ColliderDesc2D::box_collider(platform_go, [3.0, 0.25]).with_used_by_effector(true),
    );
    world.attach_effector(
        platform_col,
        PlatformEffector2D::new(120.0).with_collider_mask(1 << 0),
    );

    let ignored_body = world.add_body(BodyDesc2D::dynamic(ignored_go, [0.0, 1.0]).with_layer(2));
    world.add_collider(
        ColliderDesc2D::circle_collider(ignored_go, 0.3)
            .with_attached_body(ignored_body)
            .with_layer(2),
    );

    for _ in 0..30 {
        world.step(0.02);
    }
    let y = world.body(ignored_body).map_or(1.0, |b| b.position[1]);
    assert!(
        y < -0.5,
        "body on layer excluded by PlatformEffector2D collider_mask must fall through, got y={y}"
    );
    assert!(world.drain_messages().is_empty());
}

#[test]
fn surface_effector_accelerates_contacting_body_along_tangent() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let belt_go = EntityId::new(10);
    let crate_go = EntityId::new(11);

    let belt_body = world.add_body(BodyDesc2D::static_body(belt_go, [0.0, 0.0]));
    let belt_col = world.add_collider(
        ColliderDesc2D::box_collider(belt_go, [5.0, 0.5])
            .with_attached_body(belt_body)
            .with_used_by_effector(true),
    );
    world.attach_effector(belt_col, SurfaceEffector2D::new(4.0, 0.5));

    let crate_body = world.add_body(BodyDesc2D::dynamic(crate_go, [0.0, 0.95]));
    world.add_collider(
        ColliderDesc2D::box_collider(crate_go, [0.5, 0.5]).with_attached_body(crate_body),
    );

    for _ in 0..10 {
        world.step(0.02);
    }

    let vx = world.body(crate_body).map_or(0.0, |b| b.linear_velocity[0]);
    let x = world.body(crate_body).map_or(0.0, |b| b.position[0]);
    assert!(
        vx > 3.5,
        "surface effector should drive tangential speed toward 4.0, got {vx}"
    );
    assert!(
        x > 0.2,
        "crate should move rightward along conveyor, got {x}"
    );
}

#[test]
fn area_effector_applies_directional_force_and_drag_inside_trigger() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let wind_go = EntityId::new(20);
    let ball_go = EntityId::new(21);

    let wind_col = world.add_collider(
        ColliderDesc2D::box_collider(wind_go, [4.0, 4.0])
            .with_trigger(true)
            .with_used_by_effector(true),
    );
    world.attach_effector(
        wind_col,
        AreaEffector2D::new(90.0, 20.0, 0.5).with_collider_mask(1 << 1),
    );

    let ball_body = world.add_body(
        BodyDesc2D::dynamic(ball_go, [0.0, 0.0])
            .with_layer(1)
            .with_velocity([4.0, 0.0]),
    );
    world.add_collider(
        ColliderDesc2D::circle_collider(ball_go, 0.4)
            .with_attached_body(ball_body)
            .with_layer(1),
    );

    for _ in 0..10 {
        world.step(0.05);
    }

    let vel = world
        .body(ball_body)
        .map_or([0.0, 0.0], |b| b.linear_velocity);
    assert!(
        vel[1] > 5.0,
        "area effector at 90 deg must push upward, got vy={}",
        vel[1]
    );
    assert!(
        vel[0] < 4.0,
        "area effector drag must damp initial horizontal velocity, got vx={}",
        vel[0]
    );
}

#[test]
fn point_effector_attracts_and_repels_bodies_with_layer_mask() {
    let mut world = World2D::with_gravity([0.0, 0.0]);
    let (magnet_col, metal_body, wood_body) = setup_point_effector_world(&mut world);

    for _ in 0..10 {
        world.step(0.05);
    }

    let metal_x = world.body(metal_body).map_or(3.0, |b| b.position[0]);
    let wood_x = world.body(wood_body).map_or(3.0, |b| b.position[0]);
    assert!(
        metal_x < 2.5,
        "negative PointEffector2D force must attract metal toward origin, got x={metal_x}"
    );
    assert!(
        (wood_x - 3.0).abs() < 1e-5,
        "excluded layer must not be moved by PointEffector2D"
    );

    world.clear_effectors(magnet_col);
    world.attach_effector(
        magnet_col,
        PointEffector2D::new(50.0, 1.0).with_collider_mask(1 << 1),
    );
    if let Some(b) = world.body_mut(metal_body) {
        b.linear_velocity = [0.0, 0.0];
        b.position = [2.0, 0.0];
    }
    for _ in 0..10 {
        world.step(0.05);
    }
    let repelled_x = world.body(metal_body).map_or(2.0, |b| b.position[0]);
    assert!(
        repelled_x > 2.5,
        "positive PointEffector2D force must repel metal away from origin, got x={repelled_x}"
    );
}

fn setup_point_effector_world(
    world: &mut World2D,
) -> (
    unitas_physics::two_d::ColliderId2D,
    unitas_physics::two_d::BodyId2D,
    unitas_physics::two_d::BodyId2D,
) {
    let magnet_go = EntityId::new(30);
    let metal_go = EntityId::new(31);
    let wood_go = EntityId::new(32);

    let magnet_col = world.add_collider(
        ColliderDesc2D::circle_collider(magnet_go, 5.0)
            .with_trigger(true)
            .with_used_by_effector(true),
    );
    world.attach_effector(
        magnet_col,
        PointEffector2D::new(-30.0, 1.0).with_collider_mask(1 << 1),
    );

    let metal_body = world.add_body(BodyDesc2D::dynamic(metal_go, [3.0, 0.0]).with_layer(1));
    world.add_collider(
        ColliderDesc2D::circle_collider(metal_go, 0.3)
            .with_attached_body(metal_body)
            .with_layer(1),
    );

    let wood_body = world.add_body(BodyDesc2D::dynamic(wood_go, [3.0, 1.0]).with_layer(2));
    world.add_collider(
        ColliderDesc2D::circle_collider(wood_go, 0.3)
            .with_attached_body(wood_body)
            .with_layer(2),
    );
    (magnet_col, metal_body, wood_body)
}

#[test]
fn buoyancy_effector_floats_and_sinks_bodies_by_density_and_applies_submerged_drag_and_flow() {
    let mut world = World2D::with_gravity([0.0, -10.0]);
    let (cork_body, stone_body, air_body) = setup_buoyancy_effector_world(&mut world);

    for _ in 0..40 {
        world.step(0.02);
    }

    let cork = world.body(cork_body);
    let cork_y = cork.map_or(0.0, |b| b.position[1]);
    let cork_x = cork.map_or(-2.0, |b| b.position[0]);
    let cork_vx = cork.map_or(0.0, |b| b.linear_velocity[0]);
    let cork_ang_vel = cork.map_or(4.0, |b| b.angular_velocity);
    assert!(
        cork_y > 0.3,
        "low-mass cork submerged below surface_level=2.0 in density=2.0 water must float upward, got y={cork_y}"
    );
    assert!(
        cork_x > -1.8 && cork_vx > 0.5,
        "submerged cork must experience rightward flow force, got x={cork_x}, vx={cork_vx}"
    );
    assert!(
        cork_ang_vel < 1.5,
        "submerged angular drag must damp cork angular velocity from 4.0, got {cork_ang_vel}"
    );

    let stone_y = world.body(stone_body).map_or(0.0, |b| b.position[1]);
    assert!(
        stone_y < -1.0,
        "high-mass stone whose weight exceeds displaced fluid buoyancy must sink, got y={stone_y}"
    );

    let air_x = world.body(air_body).map_or(0.0, |b| b.position[0]);
    let air_y = world.body(air_body).map_or(3.2, |b| b.position[1]);
    assert!(
        air_x.abs() < 1e-4 && (air_y - 3.2).abs() < 1e-4,
        "body inside effector collider but above surface_level=2.0 must receive zero buoyancy or flow, got ({air_x}, {air_y})"
    );
}

fn setup_buoyancy_effector_world(
    world: &mut World2D,
) -> (
    unitas_physics::two_d::BodyId2D,
    unitas_physics::two_d::BodyId2D,
    unitas_physics::two_d::BodyId2D,
) {
    use unitas_physics::two_d::effector::BuoyancyEffector2D;

    let water_go = EntityId::new(40);
    let cork_go = EntityId::new(41);
    let stone_go = EntityId::new(42);
    let air_go = EntityId::new(43);

    let water_col = world.add_collider(
        ColliderDesc2D::box_collider(water_go, [10.0, 4.0])
            .with_trigger(true)
            .with_used_by_effector(true),
    );
    world.attach_effector(
        water_col,
        BuoyancyEffector2D::new(2.0, 2.0)
            .with_drag(2.0, 3.0)
            .with_flow(0.0, 8.0, 0.0)
            .with_collider_mask(1 << 0),
    );

    let cork_body = world.add_body(
        BodyDesc2D::dynamic(cork_go, [-2.0, 0.0])
            .with_mass(1.0)
            .with_velocity([0.0, -2.0]),
    );
    if let Some(b) = world.body_mut(cork_body) {
        b.angular_velocity = 4.0;
    }
    world.add_collider(
        ColliderDesc2D::box_collider(cork_go, [0.5, 0.5]).with_attached_body(cork_body),
    );

    let stone_body = world.add_body(BodyDesc2D::dynamic(stone_go, [2.0, 0.0]).with_mass(8.0));
    world.add_collider(
        ColliderDesc2D::box_collider(stone_go, [0.5, 0.5]).with_attached_body(stone_body),
    );

    let air_body = world.add_body(
        BodyDesc2D::dynamic(air_go, [0.0, 3.2])
            .with_mass(1.0)
            .with_gravity_scale(0.0),
    );
    world.add_collider(
        ColliderDesc2D::box_collider(air_go, [0.5, 0.5]).with_attached_body(air_body),
    );
    (cork_body, stone_body, air_body)
}
