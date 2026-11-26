use unitas_core::EntityId;
use unitas_physics::{BodyDesc, BodyRole, ForceMode, PhysicsError, Shape, World};

fn body(role: BodyRole, mass: f64) -> BodyDesc {
    BodyDesc {
        role,
        shape: Shape::Box { half_extents: [0.5, 0.5, 0.5] },
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        layer: 0,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn probe() -> Result<(), PhysicsError> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let light = EntityId::new(1);
    let heavy = EntityId::new(2);
    world.add_body(light, body(BodyRole::Dynamic, 2.0))?;
    world.add_body(heavy, body(BodyRole::Dynamic, 8.0))?;
    world.add_force(light, [3.0, 0.0, 0.0], ForceMode::Acceleration)?;
    world.add_force(heavy, [3.0, 0.0, 0.0], ForceMode::Acceleration)?;
    world.step(0.5)?;
    println!("light velocity {:?}", world.velocity(light));
    println!("heavy velocity {:?}", world.velocity(heavy));
    println!("light position {:?}", world.position(light));
    println!("heavy position {:?}", world.position(heavy));
    println!("light rotation {:?}", world.rotation(light));
    println!("heavy rotation {:?}", world.rotation(heavy));
    println!("events {:?}", world.drain_events());
    Ok(())
}
