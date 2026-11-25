use unitas_core::EntityId;
use unitas_physics::{BodyDesc, BodyRole, CollisionDetectionMode3D, PhysicsError, Shape, World};

const ROTATION_SPEED_DEGREES: f64 = 60.0;
const TICK: f64 = 1.0 / 60.0;
const TICKS: usize = 150;
const PLATFORM_HALF_EXTENT_X: f64 = 2.0;
const PLATFORM_HALF_EXTENT_Y: f64 = 0.25;
const PLATFORM_HALF_EXTENT_Z: f64 = 2.0;
const PLATFORM_CENTER_Y: f64 = 0.25;
const MARBLE_RADIUS: f64 = 0.5;
const MARBLE_OFFSET_X: f64 = 1.5;
const FRICTION: f64 = 0.6;

fn rotation_around_y(angle_degrees: f64) -> [f64; 4] {
    let half = angle_degrees.to_radians() * 0.5;
    [0.0, half.sin(), 0.0, half.cos()]
}

fn platform() -> BodyDesc {
    BodyDesc {
        position: [0.0, PLATFORM_CENTER_Y, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box {
            half_extents: [
                PLATFORM_HALF_EXTENT_X,
                PLATFORM_HALF_EXTENT_Y,
                PLATFORM_HALF_EXTENT_Z,
            ],
        },
        layer: 0,
        role: BodyRole::Kinematic,
        friction: FRICTION,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn marble() -> BodyDesc {
    BodyDesc {
        position: [
            MARBLE_OFFSET_X,
            PLATFORM_CENTER_Y + PLATFORM_HALF_EXTENT_Y + MARBLE_RADIUS,
            0.0,
        ],
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0, 0.0, 0.0],
        use_gravity: true,
        is_trigger: false,
        shape: Shape::Sphere {
            radius: MARBLE_RADIUS,
        },
        layer: 0,
        role: BodyRole::Dynamic,
        friction: FRICTION,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn angle_around_platform(position: [f64; 3]) -> f64 {
    position[2].atan2(position[0]).to_degrees()
}

#[test]
fn externally_driven_kinematic_rotation_carries_a_resting_marble() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let platform_entity = EntityId::new(1);
    let marble_entity = EntityId::new(2);
    world.add_body(platform_entity, platform())?;
    world.add_body(marble_entity, marble())?;
    let Some(start) = world.position(marble_entity) else {
        return Err(PhysicsError::InvalidBody(marble_entity));
    };

    for tick in 1..=TICKS {
        let angle = ROTATION_SPEED_DEGREES * TICK * tick as f64;
        world.set_rotation(platform_entity, rotation_around_y(angle))?;
        world.step(TICK)?;
    }

    let Some(carried) = world.position(marble_entity) else {
        return Err(PhysicsError::InvalidBody(marble_entity));
    };
    let travelled = ((carried[0] - start[0]).powi(2) + (carried[2] - start[2]).powi(2)).sqrt();
    assert!(
        travelled > 1.0,
        "the rotating platform must carry the marble, it moved {travelled} m"
    );

    let advanced =
        (angle_around_platform(start) - angle_around_platform(carried)).rem_euclid(360.0);
    assert!(
        advanced > 90.0,
        "the marble must ride around the platform axis, it advanced {advanced} degrees"
    );

    let radius = (carried[0].powi(2) + carried[2].powi(2)).sqrt();
    assert!(
        radius < PLATFORM_HALF_EXTENT_X,
        "the carried marble must stay on the platform, its radius is {radius} m"
    );

    let resting_height = PLATFORM_CENTER_Y + PLATFORM_HALF_EXTENT_Y + MARBLE_RADIUS;
    assert!(
        (carried[1] - resting_height).abs() < 0.05,
        "the carried marble must keep resting on the platform, its height is {}",
        carried[1]
    );
    Ok(())
}
