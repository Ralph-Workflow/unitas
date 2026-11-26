use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, Shape, World,
};

const ROTATE: bool = true;
const CCD: bool = true;
const BUMPER: bool = true;
const PLATFORM: bool = true;
const FAST: bool = true;

fn desc(
    position: [f64; 3],
    shape: Shape,
    role: BodyRole,
    velocity: [f64; 3],
    restitution: f64,
    ccd: CollisionDetectionMode3D,
) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: if role == BodyRole::Dynamic { 1.0 } else { 0.0 },
        velocity,
        use_gravity: role == BodyRole::Dynamic,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(shape),
        layer: 0,
        role,
        friction: 0.5,
        restitution,
        collision_detection_mode: ccd,
    }
    .with_collision_detection_mode(ccd)
}

#[test]
fn physics_lab_mix_does_not_panic() -> Result<(), unitas_physics::PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        desc(
            [0.0, -0.25, 0.0],
            Shape::Box {
                half_extents: [2.0, 0.25, 12.0],
            },
            BodyRole::Static,
            [0.0; 3],
            0.0,
            CollisionDetectionMode3D::Discrete,
        ),
    )?;
    world.add_body(
        EntityId::new(2),
        desc(
            [0.0, 1.0, 20.0],
            Shape::Box {
                half_extents: [2.0, 1.0, 0.25],
            },
            BodyRole::Static,
            [0.0; 3],
            0.0,
            CollisionDetectionMode3D::Discrete,
        ),
    )?;
    if FAST {
        world.add_body(
            EntityId::new(3),
            desc(
                [0.0, 0.5, 0.0],
                Shape::Sphere { radius: 0.5 },
                BodyRole::Dynamic,
                [0.0, 0.0, 30.0],
                0.0,
                if CCD {
                    CollisionDetectionMode3D::Continuous
                } else {
                    CollisionDetectionMode3D::Discrete
                },
            ),
        )?;
    }
    if BUMPER {
        world.add_body(
            EntityId::new(4),
            desc(
                [10.0, 0.5, 0.0],
                Shape::Sphere { radius: 0.75 },
                BodyRole::Static,
                [0.0; 3],
                1.5,
                CollisionDetectionMode3D::Discrete,
            ),
        )?;
        world.add_body(
            EntityId::new(5),
            desc(
                [10.0, 0.5, 5.0],
                Shape::Sphere { radius: 0.5 },
                BodyRole::Dynamic,
                [0.0, 0.0, -12.0],
                0.0,
                CollisionDetectionMode3D::Discrete,
            ),
        )?;
    }
    if PLATFORM {
        world.add_body(
            EntityId::new(6),
            desc(
                [-10.0, 0.0, 0.0],
                Shape::Box {
                    half_extents: [2.0, 0.25, 2.0],
                },
                BodyRole::Kinematic,
                [0.0; 3],
                0.0,
                CollisionDetectionMode3D::Discrete,
            ),
        )?;
        world.add_body(
            EntityId::new(7),
            desc(
                [-8.5, 0.75, 0.0],
                Shape::Sphere { radius: 0.5 },
                BodyRole::Dynamic,
                [0.0; 3],
                0.0,
                CollisionDetectionMode3D::Discrete,
            ),
        )?;
    }
    let mut angle = 0.0_f64;
    for _ in 0..300 {
        if ROTATE {
            angle += 90.0_f64.to_radians() / 60.0;
            let (s, c) = (angle / 2.0).sin_cos();
            world.set_rotation(EntityId::new(6), [0.0, s, 0.0, c])?;
        }
        world.step(1.0 / 60.0)?;
    }
    Ok(())
}

#[test]
fn rider_stays_on_rotating_platform() -> Result<(), unitas_physics::PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(6),
        desc(
            [0.0, 0.0, 0.0],
            Shape::Box {
                half_extents: [2.0, 0.25, 2.0],
            },
            BodyRole::Kinematic,
            [0.0; 3],
            0.0,
            CollisionDetectionMode3D::Discrete,
        ),
    )?;
    world.add_body(
        EntityId::new(7),
        desc(
            [1.5, 0.75, 0.0],
            Shape::Sphere { radius: 0.5 },
            BodyRole::Dynamic,
            [0.0; 3],
            0.0,
            CollisionDetectionMode3D::Discrete,
        ),
    )?;
    let mut angle = 0.0_f64;
    for _ in 0..300 {
        angle += 90.0_f64.to_radians() / 60.0;
        let (s, c) = (angle / 2.0).sin_cos();
        world.set_rotation(EntityId::new(6), [0.0, s, 0.0, c])?;
        world.step(1.0 / 60.0)?;
    }
    let rider = world
        .position(EntityId::new(7))
        .ok_or(unitas_physics::PhysicsError::InvalidBody(EntityId::new(7)))?;
    assert!(rider[1] > 0.5, "rider fell through the platform: {rider:?}");
    let moved = (rider[0] * rider[0] + rider[2] * rider[2]).sqrt();
    assert!(moved > 1.0, "rider was not carried: {rider:?}");
    Ok(())
}
