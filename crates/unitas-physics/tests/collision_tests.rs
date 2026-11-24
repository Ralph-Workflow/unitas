use unitas_core::EntityId;
use unitas_physics::{BodyDesc, CollisionEvent, Phase, PhysicsError, Shape, TriggerEvent, World};

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
    world.add_body(EntityId::new(1), sphere([-0.5, 0.0, 0.0], [0.0; 3]))?;
    world.add_body(EntityId::new(2), sphere([0.5, 0.0, 0.0], [0.0; 3]))?;

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
    world.add_body(along_x, box_body([1.5, 0.0, 0.0]))?;
    world.add_body(along_z, box_body([0.0, 0.0, 3.0]))?;

    world.step(0.1)?;

    let events = world.drain_events();
    assert_eq!(
        events,
        vec![CollisionEvent {
            a: origin,
            b: along_x,
            phase: Phase::Enter
        }]
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
            ..sphere([-1.0, 0.0, 0.0], [0.6, 0.0, 0.0])
        },
    )?;
    world.add_body(
        right,
        BodyDesc {
            is_trigger: true,
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
        oriented_box([1.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.3, 0.3, 0.3]),
    )?;

    world.step(0.1)?;

    assert_eq!(
        world.drain_events(),
        vec![CollisionEvent {
            a: rotated,
            b: corner,
            phase: Phase::Enter
        }]
    );
    Ok(())
}
