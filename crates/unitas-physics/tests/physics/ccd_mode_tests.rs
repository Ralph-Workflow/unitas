use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, PhysicsError, Shape, World,
};

const BALL_SPEED: f64 = 200.0;
const STEP: f64 = 0.02;
const WALL_SPEED: f64 = 0.5;

fn wall_desc(role: BodyRole, position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Box {
            half_extents: [0.05, 2.0, 2.0],
        }),
        layer: 0,
        role,
        friction: 0.0,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn ball_desc(mode: CollisionDetectionMode3D) -> BodyDesc {
    BodyDesc {
        position: [-3.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [BALL_SPEED, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.25 }),
        layer: 0,
        role: BodyRole::Dynamic,
        friction: 0.0,
        restitution: 0.0,
        collision_detection_mode: mode,
    }
}

fn run(mode: CollisionDetectionMode3D, moving_wall: bool) -> Result<f64, PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    let wall = EntityId::new(1);
    let ball = EntityId::new(2);
    if moving_wall {
        world.add_body(wall, wall_desc(BodyRole::Kinematic, [0.0, 0.0, 0.0]))?;
    } else {
        world.add_body(wall, wall_desc(BodyRole::Static, [0.0, 0.0, 0.0]))?;
    }
    world.add_body(ball, ball_desc(mode))?;
    for _ in 0..5 {
        if moving_wall {
            let position = world.position(wall).unwrap_or([0.0; 3]);
            world.move_position(wall, [position[0] - WALL_SPEED * STEP, 0.0, 0.0])?;
        }
        world.step(STEP)?;
    }
    Ok(world.position(ball).unwrap_or([0.0; 3])[0])
}

#[test]
fn discrete_collision_detection_tunnels_through_a_thin_wall() -> Result<(), PhysicsError> {
    let static_wall = run(CollisionDetectionMode3D::Discrete, false)?;
    let moving_wall = run(CollisionDetectionMode3D::Discrete, true)?;
    assert!(
        static_wall > 1.0,
        "expected the discrete ball to tunnel through the static wall, got {static_wall}"
    );
    assert!(
        moving_wall > 1.0,
        "expected the discrete ball to tunnel through the moving wall, got {moving_wall}"
    );
    Ok(())
}

#[test]
fn continuous_collision_detection_sweeps_static_geometry_only() -> Result<(), PhysicsError> {
    let static_wall = run(CollisionDetectionMode3D::Continuous, false)?;
    let moving_wall = run(CollisionDetectionMode3D::Continuous, true)?;
    assert!(
        static_wall < 0.0,
        "expected the continuous ball to stop at the static wall, got {static_wall}"
    );
    assert!(
        moving_wall > 1.0,
        "expected the continuous ball to miss the moving wall, got {moving_wall}"
    );
    Ok(())
}

#[test]
fn continuous_dynamic_collision_detection_sweeps_moving_geometry() -> Result<(), PhysicsError> {
    let static_wall = run(CollisionDetectionMode3D::ContinuousDynamic, false)?;
    let moving_wall = run(CollisionDetectionMode3D::ContinuousDynamic, true)?;
    assert!(
        static_wall < 0.0,
        "expected the dynamic-continuous ball to stop at the static wall, got {static_wall}"
    );
    assert!(
        moving_wall < 0.0,
        "expected the dynamic-continuous ball to stop at the moving wall, got {moving_wall}"
    );
    Ok(())
}

#[test]
fn continuous_speculative_collision_detection_uses_speculative_contacts() -> Result<(), PhysicsError>
{
    let static_wall = run(CollisionDetectionMode3D::ContinuousSpeculative, false)?;
    let moving_wall = run(CollisionDetectionMode3D::ContinuousSpeculative, true)?;
    assert!(
        (static_wall + 0.3).abs() < 0.001,
        "expected the speculative ball to stop on the wall surface, got {static_wall}"
    );
    assert!(
        moving_wall < 0.0,
        "expected the speculative ball to stop at the moving wall, got {moving_wall}"
    );
    Ok(())
}
