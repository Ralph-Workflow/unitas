use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, CollisionEvent, Phase, PhysicsError, Shape, TriggerEvent, World,
};

const ROTATED_45_DEGREES_ABOUT_Z: [f64; 4] = [0.0, 0.0, 0.38268343, 0.92387953];

fn oriented_box(position: [f64; 3], rotation: [f64; 4], half_extents: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation,
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box { half_extents },
        layer: 0,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn sphere(position: [f64; 3], velocity: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity,
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Sphere { radius: 0.5 },
        layer: 0,
        role: BodyRole::Dynamic,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn box_body(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box {
            half_extents: [1.0, 1.0, 1.0],
        },
        layer: 0,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn spheres_report_enter_then_stay_then_exit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(left, sphere([-1.0, 0.0, 0.0], [0.6, 0.0, 0.0]))?;
    world.add_body(right, sphere([1.0, 0.0, 0.0], [-0.6, 0.0, 0.0]))?;

    world.step(1.0)?;
    let entered = world.drain_events();
    assert_eq!(
        entered,
        vec![CollisionEvent {
            a: left,
            b: right,
            phase: Phase::Enter
        }]
    );

    world.step(1.0)?;
    let stayed = world.drain_events();
    assert_eq!(
        stayed,
        vec![CollisionEvent {
            a: left,
            b: right,
            phase: Phase::Stay
        }]
    );

    world.set_velocity(left, [-0.6, 0.0, 0.0])?;
    world.set_velocity(right, [0.6, 0.0, 0.0])?;
    world.step(1.0)?;
    let exited = world.drain_events();
    assert_eq!(
        exited,
        vec![CollisionEvent {
            a: left,
            b: right,
            phase: Phase::Exit
        }]
    );
    Ok(())
}

#[test]
fn touching_spheres_do_not_overlap() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(1), sphere([-0.525, 0.0, 0.0], [0.0; 3]))?;
    world.add_body(EntityId::new(2), sphere([0.525, 0.0, 0.0], [0.0; 3]))?;

    world.step(0.1)?;

    assert!(world.drain_events().is_empty());
    Ok(())
}

#[test]
fn sphere_falling_into_static_box_reports_enter_then_stay() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let floor = EntityId::new(1);
    let ball = EntityId::new(2);
    world.add_body(
        floor,
        BodyDesc {
            shape: Shape::Box {
                half_extents: [2.0, 1.0, 2.0],
            },
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        ball,
        BodyDesc {
            position: [0.0, 1.6, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0; 3],
            use_gravity: true,
            is_trigger: false,
            shape: Shape::Sphere { radius: 0.5 },
            layer: 0,
            role: BodyRole::Dynamic,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    world.step(0.1)?;
    assert!(world.drain_events().is_empty());

    world.step(0.1)?;
    let entered = world.drain_events();
    assert_eq!(
        entered,
        vec![CollisionEvent {
            a: floor,
            b: ball,
            phase: Phase::Enter
        }]
    );

    world.step(0.1)?;
    let stayed = world.drain_events();
    assert_eq!(
        stayed,
        vec![CollisionEvent {
            a: floor,
            b: ball,
            phase: Phase::Stay
        }]
    );

    world.step(0.1)?;
    let stayed_again = world.drain_events();
    assert_eq!(
        stayed_again,
        vec![CollisionEvent {
            a: floor,
            b: ball,
            phase: Phase::Stay
        }]
    );
    Ok(())
}

#[test]
fn boxes_overlap_only_when_every_axis_overlaps() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let origin = EntityId::new(1);
    let along_x = EntityId::new(2);
    let along_z = EntityId::new(3);
    world.add_body(origin, box_body([0.0, 0.0, 0.0]))?;
    world.add_body(
        along_x,
        BodyDesc {
            role: BodyRole::Dynamic,
            mass: 1.0,
            ..box_body([1.5, 0.0, 0.0])
        },
    )?;
    world.add_body(along_z, box_body([0.0, 0.0, 3.0]))?;

    world.step(0.1)?;

    let events = world.drain_events();
    assert_eq!(
        events,
        vec![CollisionEvent {
            a: origin,
            b: along_x,
            phase: Phase::Enter
        }],
        "the overlap probe body must carry a Rigidbody because two static colliders exchange no messages"
    );
    Ok(())
}

#[test]
fn draining_events_empties_the_queue() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(left, sphere([-0.4, 0.0, 0.0], [0.0; 3]))?;
    world.add_body(right, sphere([0.4, 0.0, 0.0], [0.0; 3]))?;

    world.step(0.1)?;
    let first_drain = world.drain_events();
    assert_eq!(
        first_drain,
        vec![CollisionEvent {
            a: left,
            b: right,
            phase: Phase::Enter
        }]
    );

    let second_drain = world.drain_events();
    assert!(second_drain.is_empty());
    assert!(world.drain_trigger_events().is_empty());
    Ok(())
}

#[test]
fn trigger_and_non_trigger_pair_emits_trigger_enter_stay_exit_and_zero_collision_events(
) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let trigger_zone = EntityId::new(1);
    let mover = EntityId::new(2);
    world.add_body(
        trigger_zone,
        BodyDesc {
            is_trigger: true,
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(mover, sphere([-2.0, 0.0, 0.0], [1.2, 0.0, 0.0]))?;

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: trigger_zone,
            b: mover,
            phase: Phase::Enter,
        }]
    );

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: trigger_zone,
            b: mover,
            phase: Phase::Stay,
        }]
    );

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: trigger_zone,
            b: mover,
            phase: Phase::Exit,
        }]
    );
    Ok(())
}

#[test]
fn both_triggers_pair_emits_trigger_events_and_zero_collision_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(
        left,
        BodyDesc {
            is_trigger: true,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([-1.0, 0.0, 0.0], [0.6, 0.0, 0.0])
        },
    )?;
    world.add_body(
        right,
        BodyDesc {
            is_trigger: true,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([1.0, 0.0, 0.0], [-0.6, 0.0, 0.0])
        },
    )?;

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: left,
            b: right,
            phase: Phase::Enter,
        }]
    );

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: left,
            b: right,
            phase: Phase::Stay,
        }]
    );

    world.step(1.0)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: left,
            b: right,
            phase: Phase::Exit,
        }]
    );
    Ok(())
}

#[test]
fn draining_trigger_events_empties_the_trigger_queue() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(
        left,
        BodyDesc {
            is_trigger: true,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([-0.4, 0.0, 0.0], [0.0; 3])
        },
    )?;
    world.add_body(right, sphere([0.4, 0.0, 0.0], [0.0; 3]))?;

    world.step(0.1)?;
    let first_drain = world.drain_trigger_events();
    assert_eq!(
        first_drain,
        vec![TriggerEvent {
            a: left,
            b: right,
            phase: Phase::Enter,
        }]
    );
    assert!(world.drain_trigger_events().is_empty());
    assert!(world.drain_events().is_empty());
    Ok(())
}

#[test]
fn rotated_boxes_do_not_overlap_where_the_axis_aligned_test_would() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let rotated = EntityId::new(1);
    let corner = EntityId::new(2);
    world.add_body(
        rotated,
        oriented_box([0.0, 0.0, 0.0], ROTATED_45_DEGREES_ABOUT_Z, [1.0, 1.0, 1.0]),
    )?;
    world.add_body(
        corner,
        oriented_box([1.05, 1.05, 0.0], [0.0, 0.0, 0.0, 1.0], [0.3, 0.3, 0.3]),
    )?;

    world.step(0.1)?;

    assert!(world.drain_events().is_empty());
    Ok(())
}

#[test]
fn rotated_boxes_overlap_when_they_really_do() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let rotated = EntityId::new(1);
    let corner = EntityId::new(2);
    world.add_body(
        rotated,
        oriented_box([0.0, 0.0, 0.0], ROTATED_45_DEGREES_ABOUT_Z, [1.0, 1.0, 1.0]),
    )?;
    world.add_body(
        corner,
        BodyDesc {
            role: BodyRole::Dynamic,
            mass: 1.0,
            ..oriented_box([1.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.3, 0.3, 0.3])
        },
    )?;

    world.step(0.1)?;

    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: rotated,
            b: corner,
            phase: Phase::Enter
        }],
        "the overlap probe body must carry a Rigidbody because two static colliders exchange no messages"
    );
    Ok(())
}

#[test]
fn resting_sphere_on_static_box_floor_settles_with_bounded_penetration() -> Result<(), PhysicsError>
{
    let mut world = World::new();
    let floor = EntityId::new(1);
    let ball = EntityId::new(2);
    world.add_body(
        floor,
        BodyDesc {
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, -0.5, 0.0])
        },
    )?;
    world.add_body(
        ball,
        BodyDesc {
            use_gravity: true,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([0.0, 2.0, 0.0], [0.0; 3])
        },
    )?;

    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }

    let pos = world.position(ball).unwrap_or([0.0; 3]);
    let vel = world.velocity(ball).unwrap_or([0.0; 3]);
    assert!((pos[1] - 0.5).abs() < 1e-3);
    assert!(vel[1].abs() < 1e-3);
    assert_eq!(world.position(floor), Some([0.0, -0.5, 0.0]));
    Ok(())
}

#[test]
fn dynamic_box_landing_on_static_box_comes_to_rest() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let floor = EntityId::new(1);
    let crate_body = EntityId::new(2);
    world.add_body(
        floor,
        BodyDesc {
            shape: Shape::Box {
                half_extents: [4.0, 0.5, 4.0],
            },
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, -0.5, 0.0])
        },
    )?;
    world.add_body(
        crate_body,
        BodyDesc {
            position: [0.0, 1.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 2.0,
            velocity: [0.0; 3],
            use_gravity: true,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            layer: 0,
            role: BodyRole::Dynamic,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }

    let pos = world.position(crate_body).unwrap_or([0.0; 3]);
    let vel = world.velocity(crate_body).unwrap_or([0.0; 3]);
    assert!((pos[1] - 0.5).abs() < 1e-3);
    assert!(vel[1].abs() < 1e-3);
    assert_eq!(world.position(floor), Some([0.0, -0.5, 0.0]));
    Ok(())
}

#[test]
fn two_dynamic_bodies_collide_and_separate_in_proportion_to_inverse_mass(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(
        left,
        BodyDesc {
            mass: 1.0,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([-0.4, 0.0, 0.0], [2.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        right,
        BodyDesc {
            mass: 3.0,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([0.4, 0.0, 0.0], [-2.0, 0.0, 0.0])
        },
    )?;

    world.step(0.1)?;

    let left_pos = world.position(left).unwrap_or([0.0; 3]);
    let right_pos = world.position(right).unwrap_or([0.0; 3]);
    let left_vel = world.velocity(left).unwrap_or([0.0; 3]);
    let right_vel = world.velocity(right).unwrap_or([0.0; 3]);
    assert!((right_pos[0] - left_pos[0] - 1.0).abs() < 0.15);
    assert!((left_vel[0] - (-1.0)).abs() < 1e-4);
    assert!((right_vel[0] - (-1.0)).abs() < 1e-4);
    Ok(())
}

#[test]
fn dynamic_spheres_pushed_together_then_apart_report_enter_stay_exit() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([2.0, 0.0, 0.0]);
    let ball = EntityId::new(1);
    let wall = EntityId::new(2);
    world.add_body(
        ball,
        BodyDesc {
            use_gravity: true,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([-0.6, 0.0, 0.0], [1.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        wall,
        BodyDesc {
            role: BodyRole::Static,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([0.5, 0.0, 0.0], [0.0; 3])
        },
    )?;

    world.step(0.1)?;
    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: ball,
            b: wall,
            phase: Phase::Enter
        }]
    );

    world.step(0.1)?;
    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: ball,
            b: wall,
            phase: Phase::Stay
        }]
    );

    world.set_gravity([-20.0, 0.0, 0.0]);
    world.step(0.1)?;
    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: ball,
            b: wall,
            phase: Phase::Exit
        }]
    );
    Ok(())
}

#[test]
fn trigger_pair_is_untouched_by_the_solver() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let trigger_zone = EntityId::new(1);
    let mover = EntityId::new(2);
    world.add_body(
        trigger_zone,
        BodyDesc {
            is_trigger: true,
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(mover, sphere([-0.5, 0.0, 0.0], [1.0, 0.0, 0.0]))?;

    world.step(0.5)?;

    assert_eq!(world.position(mover), Some([0.0, 0.0, 0.0]));
    assert_eq!(world.velocity(mover), Some([1.0, 0.0, 0.0]));
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: trigger_zone,
            b: mover,
            phase: Phase::Enter,
        }]
    );
    Ok(())
}

#[test]
fn sphere_with_restitution_rebounds_off_a_static_floor() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let floor = EntityId::new(1);
    let ball = EntityId::new(2);
    world.add_body(
        floor,
        BodyDesc {
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            layer: 0,
            role: BodyRole::Static,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;
    world.add_body(
        ball,
        BodyDesc {
            position: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0, -5.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            shape: Shape::Sphere { radius: 0.5 },
            layer: 0,
            role: BodyRole::Dynamic,
            friction: 0.6,
            restitution: 0.5,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    world.step(0.1)?;
    world.step(0.1)?;

    let velocity = world.velocity(ball).unwrap_or([0.0; 3]);
    let position = world.position(ball).unwrap_or([0.0; 3]);
    assert!(velocity[1] > 0.0);
    assert!((velocity[1] - 1.25).abs() < 1e-9);
    assert!((position[1] - 0.5).abs() < 1e-3);
    Ok(())
}

#[test]
fn sliding_box_decelerates_with_friction_and_keeps_its_speed_without_it() -> Result<(), PhysicsError>
{
    fn sliding_world(friction: f64) -> Result<(World, EntityId), PhysicsError> {
        let mut world = World::new();
        let floor = EntityId::new(1);
        let slider = EntityId::new(2);
        world.add_body(
            floor,
            BodyDesc {
                position: [0.0, -0.5, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 0.0,
                velocity: [0.0; 3],
                use_gravity: false,
                is_trigger: false,
                shape: Shape::Box {
                    half_extents: [5.0, 0.5, 5.0],
                },
                layer: 0,
                role: BodyRole::Static,
                friction,
                restitution: 0.0,
                collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
            },
        )?;
        world.add_body(
            slider,
            BodyDesc {
                position: [0.0, 0.5, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 1.0,
                velocity: [2.0, 0.0, 0.0],
                use_gravity: true,
                is_trigger: false,
                shape: Shape::Box {
                    half_extents: [0.5, 0.5, 0.5],
                },
                layer: 0,
                role: BodyRole::Dynamic,
                friction,
                restitution: 0.0,
                collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
            },
        )?;
        Ok((world, slider))
    }

    let (mut with_friction, slider) = sliding_world(0.6)?;
    let (mut without_friction, frictionless_slider) = sliding_world(0.0)?;

    for _ in 0..5 {
        with_friction.step(1.0 / 60.0)?;
        without_friction.step(1.0 / 60.0)?;
    }

    let friction_velocity = with_friction.velocity(slider).unwrap_or([0.0; 3]);
    let frictionless_velocity = without_friction
        .velocity(frictionless_slider)
        .unwrap_or([0.0; 3]);
    assert!(friction_velocity[0] < 2.0);
    assert!(friction_velocity[0] > 1.0);
    assert!(friction_velocity[1].abs() < 1e-9);
    assert!((frictionless_velocity[0] - 2.0).abs() < 1e-12);
    assert!(frictionless_velocity[1].abs() < 1e-9);
    Ok(())
}

#[test]
fn pushed_kinematic_platform_carries_a_resting_dynamic_body() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let platform = EntityId::new(1);
    let rider = EntityId::new(2);
    world.add_body(
        platform,
        BodyDesc {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [2.0, 0.5, 2.0],
            },
            layer: 0,
            role: BodyRole::Kinematic,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;
    world.add_body(
        rider,
        BodyDesc {
            position: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0; 3],
            use_gravity: true,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            layer: 0,
            role: BodyRole::Dynamic,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
        },
    )?;

    world.step(1.0 / 60.0)?;
    assert!(world.velocity(rider).unwrap_or([0.0; 3])[0].abs() < 1e-12);

    world.set_position(platform, [0.02, 0.0, 0.0])?;
    world.step(1.0 / 60.0)?;

    let rider_velocity = world.velocity(rider).unwrap_or([0.0; 3]);
    assert!(rider_velocity[0] > 0.0);
    let platform_pos = world.position(platform).unwrap_or([0.0; 3]);
    assert!((platform_pos[0] - 0.02).abs() < 1e-6);
    assert!(platform_pos[1].abs() < 1e-6 && platform_pos[2].abs() < 1e-6);

    world.step(1.0 / 60.0)?;

    let rider_position = world.position(rider).unwrap_or([0.0; 3]);
    assert!(rider_position[0] > 0.0);
    Ok(())
}

fn capsule_body(
    position: [f64; 3],
    radius: f64,
    height: f64,
    direction: u8,
    role: BodyRole,
    velocity: [f64; 3],
) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: if role == BodyRole::Dynamic { 1.0 } else { 0.0 },
        velocity,
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Capsule {
            radius,
            height,
            direction,
        },
        layer: 0,
        role,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn dynamic_capsule_landing_on_static_box_floor_comes_to_rest() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let floor = EntityId::new(1);
    let capsule = EntityId::new(2);
    world.add_body(
        floor,
        BodyDesc {
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, -0.5, 0.0])
        },
    )?;
    world.add_body(
        capsule,
        BodyDesc {
            use_gravity: true,
            ..capsule_body([0.0, 2.0, 0.0], 0.5, 2.0, 1, BodyRole::Dynamic, [0.0; 3])
        },
    )?;

    for _ in 0..120 {
        world.step(1.0 / 60.0)?;
    }

    let pos = world.position(capsule).unwrap_or([0.0; 3]);
    let vel = world.velocity(capsule).unwrap_or([0.0; 3]);
    assert!((pos[1] - 1.0).abs() < 1e-3);
    assert!(vel[1].abs() < 1e-3);
    Ok(())
}

#[test]
fn dynamic_sphere_colliding_with_static_capsule_along_y_extent_is_stopped(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let post = EntityId::new(1);
    let ball = EntityId::new(2);
    world.add_body(
        post,
        capsule_body([0.0, 0.0, 0.0], 0.5, 3.0, 1, BodyRole::Static, [0.0; 3]),
    )?;
    world.add_body(ball, sphere([-1.2, 0.8, 0.0], [3.0, 0.0, 0.0]))?;

    world.step(0.1)?;

    let events = world.drain_events();
    assert_eq!(
        events,
        vec![CollisionEvent {
            a: post,
            b: ball,
            phase: Phase::Enter,
        }]
    );
    for _ in 0..9 {
        world.step(0.1)?;
    }
    world.drain_events();
    let ball_pos = world.position(ball).unwrap_or([0.0; 3]);
    assert!((ball_pos[0] - (-1.0)).abs() < 1e-3);
    Ok(())
}

#[test]
fn capsules_with_x_and_z_directions_collide_along_their_declared_axes() -> Result<(), PhysicsError>
{
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let x_capsule = EntityId::new(1);
    let z_capsule = EntityId::new(2);
    world.add_body(
        x_capsule,
        capsule_body([0.0, 0.0, 0.0], 0.5, 3.0, 0, BodyRole::Static, [0.0; 3]),
    )?;
    world.add_body(
        z_capsule,
        capsule_body(
            [1.0, 0.8, 0.0],
            0.5,
            3.0,
            2,
            BodyRole::Dynamic,
            [0.0, -2.0, 0.0],
        ),
    )?;

    world.step(0.1)?;

    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: x_capsule,
            b: z_capsule,
            phase: Phase::Enter,
        }]
    );
    for _ in 0..9 {
        world.step(0.1)?;
    }
    world.drain_events();
    let z_pos = world.position(z_capsule).unwrap_or([0.0; 3]);
    assert!((z_pos[1] - 1.0).abs() < 1e-3);
    Ok(())
}

#[test]
fn phy_001_two_static_overlapping_colliders_exchange_no_collision_or_trigger_messages(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let floor = EntityId::new(1);
    let crate_body = EntityId::new(2);
    world.add_body(floor, box_body([0.0, 0.0, 0.0]))?;
    world.add_body(crate_body, box_body([0.5, 0.0, 0.0]))?;

    world.step(0.1)?;
    assert!(
        world.drain_events().is_empty(),
        "PHY-001: two static colliders must exchange no collision messages"
    );
    assert!(
        world.drain_trigger_events().is_empty(),
        "PHY-001: two static colliders must exchange no trigger messages"
    );

    world.step(0.1)?;
    assert!(
        world.drain_events().is_empty(),
        "PHY-001: two static colliders must keep exchanging no collision messages"
    );
    Ok(())
}

#[test]
fn phy_001_story_13_two_rigidbody_less_triggers_exchange_no_trigger_messages(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(
        left,
        BodyDesc {
            is_trigger: true,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        right,
        BodyDesc {
            is_trigger: true,
            ..box_body([0.5, 0.0, 0.0])
        },
    )?;

    world.step(0.1)?;
    assert!(
        world.drain_trigger_events().is_empty(),
        "PHY-001 story 13: a trigger between two colliders without a Rigidbody must produce no trigger messages"
    );
    assert!(
        world.drain_events().is_empty(),
        "PHY-001 story 13: a trigger between two colliders without a Rigidbody must produce no collision messages"
    );
    Ok(())
}

#[test]
fn phy_001_kinematic_pairs_without_a_dynamic_counterpart_exchange_no_collision_messages(
) -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let platform = EntityId::new(1);
    let other_platform = EntityId::new(2);
    let wall_platform = EntityId::new(3);
    let wall = EntityId::new(4);
    let moving_platform = EntityId::new(5);
    let crate_body = EntityId::new(6);
    world.add_body(
        platform,
        BodyDesc {
            role: BodyRole::Kinematic,
            mass: 0.0,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        other_platform,
        BodyDesc {
            role: BodyRole::Kinematic,
            mass: 0.0,
            ..box_body([0.5, 0.0, 0.0])
        },
    )?;
    world.add_body(
        wall_platform,
        BodyDesc {
            role: BodyRole::Kinematic,
            mass: 0.0,
            ..box_body([10.0, 0.0, 0.0])
        },
    )?;
    world.add_body(wall, box_body([10.5, 0.0, 0.0]))?;
    world.add_body(
        moving_platform,
        BodyDesc {
            role: BodyRole::Kinematic,
            mass: 0.0,
            ..box_body([20.0, 0.0, 0.0])
        },
    )?;
    world.add_body(crate_body, sphere([20.5, 0.0, 0.0], [0.0; 3]))?;

    world.step(0.1)?;
    let events = world.drain_events();
    assert!(
        !events
            .iter()
            .any(|event| event.a == platform && event.b == other_platform),
        "PHY-001: two kinematic bodies must exchange no collision messages"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.a == wall_platform && event.b == wall),
        "PHY-001: a kinematic body must exchange no collision messages with a static collider"
    );
    assert!(
        events
            .iter()
            .any(|event| event.a == moving_platform && event.b == crate_body),
        "PHY-001: a kinematic body must exchange collision messages with a dynamic body"
    );
    Ok(())
}

#[test]
fn kinematic_capsule_pushes_dynamic_capsule() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let pusher = EntityId::new(1);
    let target = EntityId::new(2);
    world.add_body(
        pusher,
        capsule_body([-1.0, 0.0, 0.0], 0.5, 2.0, 1, BodyRole::Kinematic, [0.0; 3]),
    )?;
    world.add_body(
        target,
        capsule_body([0.0, 0.0, 0.0], 0.5, 2.0, 1, BodyRole::Dynamic, [0.0; 3]),
    )?;

    world.set_position(pusher, [-0.8, 0.0, 0.0])?;
    world.step(0.1)?;

    let target_pos = world.position(target).unwrap_or([0.0; 3]);
    assert!(target_pos[0] > 0.19);
    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: pusher,
            b: target,
            phase: Phase::Enter,
        }]
    );
    Ok(())
}

#[test]
fn kinematic_trigger_and_static_trigger_pair_emits_trigger_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let sensor = EntityId::new(1);
    let pickup = EntityId::new(2);
    world.add_body(
        sensor,
        BodyDesc {
            is_trigger: true,
            role: BodyRole::Kinematic,
            mass: 0.0,
            friction: 0.6,
            restitution: 0.0,
            ..sphere([0.0, 0.0, 0.0], [0.0; 3])
        },
    )?;
    world.add_body(
        pickup,
        BodyDesc {
            is_trigger: true,
            mass: 0.0,
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.5, 0.0, 0.0])
        },
    )?;

    world.step(0.02)?;
    assert!(world.drain_events().is_empty());
    assert_eq!(
        world.drain_trigger_events(),
        vec![TriggerEvent {
            a: sensor,
            b: pickup,
            phase: Phase::Enter,
        }]
    );
    Ok(())
}

#[test]
fn two_static_triggers_exchange_no_trigger_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    world.add_body(
        left,
        BodyDesc {
            is_trigger: true,
            mass: 0.0,
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.0, 0.0, 0.0])
        },
    )?;
    world.add_body(
        right,
        BodyDesc {
            is_trigger: true,
            mass: 0.0,
            friction: 0.6,
            restitution: 0.0,
            ..box_body([0.5, 0.0, 0.0])
        },
    )?;

    world.step(0.02)?;
    assert!(world.drain_events().is_empty());
    assert!(world.drain_trigger_events().is_empty());
    Ok(())
}
