use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, ForceMode, PhysicsError, Shape,
    World,
};

const TIMESTEP: f64 = 0.02;
const STEPS: usize = 150;
const MOVER_SPEED: f64 = 2.0;
const MOVER_SPIN: f64 = 1.5;

const GROUND: EntityId = EntityId::new(1);
const CRATE: EntityId = EntityId::new(2);
const STACKED_CRATE: EntityId = EntityId::new(3);
const BALL: EntityId = EntityId::new(4);
const PUSHED_CRATE: EntityId = EntityId::new(5);
const MOVER: EntityId = EntityId::new(6);

const TRACKED: [EntityId; 6] = [GROUND, CRATE, STACKED_CRATE, BALL, PUSHED_CRATE, MOVER];

const GROUND_POSITION: [f64; 3] = [0.0, -0.5, 0.0];
const CRATE_POSITION: [f64; 3] = [0.0, 0.5, 0.0];
const STACKED_CRATE_POSITION: [f64; 3] = [0.0, 1.55, 0.0];
const BALL_POSITION: [f64; 3] = [2.5, 0.4, 0.0];
const PUSHED_CRATE_POSITION: [f64; 3] = [-4.0, 0.5, 0.0];
const MOVER_POSITION: [f64; 3] = [-3.0, 2.3, 0.0];
const IDENTITY_ROTATION: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

#[derive(Clone, Copy, Debug)]
struct BodyState {
    entity: EntityId,
    position: [f64; 3],
    rotation: [f64; 4],
    velocity: [f64; 3],
}

fn body(
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
    role: BodyRole,
    shape: Shape,
    use_gravity: bool,
) -> BodyDesc {
    BodyDesc {
        position,
        rotation: IDENTITY_ROTATION,
        mass,
        velocity,
        use_gravity,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(shape),
        layer: 0,
        role,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn crate_shape() -> Shape {
    Shape::Box {
        half_extents: [0.5, 0.5, 0.5],
    }
}

fn build_world() -> Result<World, PhysicsError> {
    let mut world = World::new();
    world.add_body(
        GROUND,
        body(
            GROUND_POSITION,
            [0.0; 3],
            0.0,
            BodyRole::Static,
            Shape::Box {
                half_extents: [8.0, 0.5, 8.0],
            },
            false,
        ),
    )?;
    world.add_body(
        CRATE,
        body(
            CRATE_POSITION,
            [1.5, 0.0, 0.0],
            1.0,
            BodyRole::Dynamic,
            crate_shape(),
            true,
        ),
    )?;
    world.add_body(
        STACKED_CRATE,
        body(
            STACKED_CRATE_POSITION,
            [0.0, 0.0, -0.4],
            2.0,
            BodyRole::Dynamic,
            crate_shape(),
            true,
        ),
    )?;
    world.add_body(
        BALL,
        body(
            BALL_POSITION,
            [-2.0, 0.0, 0.0],
            0.5,
            BodyRole::Dynamic,
            Shape::Sphere { radius: 0.4 },
            true,
        ),
    )?;
    world.add_body(
        PUSHED_CRATE,
        body(
            PUSHED_CRATE_POSITION,
            [0.0; 3],
            1.0,
            BodyRole::Dynamic,
            crate_shape(),
            true,
        ),
    )?;
    world.add_body(
        MOVER,
        body(
            MOVER_POSITION,
            [0.0; 3],
            1.0,
            BodyRole::Kinematic,
            Shape::Box {
                half_extents: [0.5, 0.25, 0.5],
            },
            false,
        ),
    )?;
    Ok(world)
}

fn scripted_input(world: &mut World, step: usize) -> Result<(), PhysicsError> {
    if step.is_multiple_of(5) {
        world.add_impulse(PUSHED_CRATE, [0.4, 0.0, 0.0])?;
    }
    if step.is_multiple_of(7) {
        world.add_force(STACKED_CRATE, [0.0, 0.0, 0.2], ForceMode::Force)?;
    }
    let target = [
        MOVER_POSITION[0] + MOVER_SPEED * step as f64 * TIMESTEP,
        MOVER_POSITION[1],
        MOVER_POSITION[2],
    ];
    world.move_position(MOVER, target)?;
    let half_angle = MOVER_SPIN * step as f64 * TIMESTEP * 0.5;
    world.move_rotation(MOVER, [0.0, half_angle.sin(), 0.0, half_angle.cos()])?;
    Ok(())
}

fn snapshot(world: &World) -> Result<Vec<BodyState>, PhysicsError> {
    let mut states = Vec::with_capacity(TRACKED.len());
    for entity in TRACKED {
        let position = world
            .position(entity)
            .ok_or(PhysicsError::InvalidBody(entity))?;
        let rotation = world
            .rotation(entity)
            .ok_or(PhysicsError::InvalidBody(entity))?;
        let velocity = world
            .velocity(entity)
            .ok_or(PhysicsError::InvalidBody(entity))?;
        states.push(BodyState {
            entity,
            position,
            rotation,
            velocity,
        });
    }
    Ok(states)
}

fn replay() -> Result<Vec<Vec<BodyState>>, PhysicsError> {
    let mut world = build_world()?;
    let mut frames = Vec::with_capacity(STEPS);
    for step in 0..STEPS {
        scripted_input(&mut world, step)?;
        world.step(TIMESTEP)?;
        frames.push(snapshot(&world)?);
    }
    Ok(frames)
}

fn state_of(frame: &[BodyState], entity: EntityId) -> Result<BodyState, PhysicsError> {
    frame
        .iter()
        .copied()
        .find(|state| state.entity == entity)
        .ok_or(PhysicsError::InvalidBody(entity))
}

fn last_frame(frames: &[Vec<BodyState>]) -> Result<&[BodyState], PhysicsError> {
    frames
        .last()
        .map(Vec::as_slice)
        .ok_or(PhysicsError::InvalidBody(CRATE))
}

#[test]
fn identical_replay_inputs_produce_bit_identical_body_states() -> Result<(), PhysicsError> {
    let first = replay()?;
    let second = replay()?;
    assert_eq!(first.len(), second.len());
    for (frame_index, (left, right)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(left.len(), right.len());
        for (left_state, right_state) in left.iter().zip(right.iter()) {
            assert_eq!(left_state.entity, right_state.entity);
            assert_eq!(
                left_state.position.map(f64::to_bits),
                right_state.position.map(f64::to_bits),
                "position differs between replay runs at frame {frame_index} for {}",
                left_state.entity
            );
            assert_eq!(
                left_state.rotation.map(f64::to_bits),
                right_state.rotation.map(f64::to_bits),
                "rotation differs between replay runs at frame {frame_index} for {}",
                left_state.entity
            );
            assert_eq!(
                left_state.velocity.map(f64::to_bits),
                right_state.velocity.map(f64::to_bits),
                "velocity differs between replay runs at frame {frame_index} for {}",
                left_state.entity
            );
        }
    }
    Ok(())
}

#[test]
fn replay_state_reflects_simulated_motion() -> Result<(), PhysicsError> {
    let frames = replay()?;
    let final_frame = last_frame(&frames)?;

    let crate_state = state_of(final_frame, CRATE)?;
    assert_ne!(
        crate_state.position.map(f64::to_bits),
        CRATE_POSITION.map(f64::to_bits),
        "crate did not move during the replay"
    );

    let stacked_state = state_of(final_frame, STACKED_CRATE)?;
    assert_ne!(
        stacked_state.position.map(f64::to_bits),
        STACKED_CRATE_POSITION.map(f64::to_bits),
        "stacked crate did not move during the replay"
    );

    let ball_state = state_of(final_frame, BALL)?;
    assert_ne!(
        ball_state.position.map(f64::to_bits),
        BALL_POSITION.map(f64::to_bits),
        "ball did not move during the replay"
    );

    let mover_state = state_of(final_frame, MOVER)?;
    assert_ne!(
        mover_state.position.map(f64::to_bits),
        MOVER_POSITION.map(f64::to_bits),
        "kinematic mover did not follow the scripted path"
    );
    assert_ne!(
        mover_state.rotation.map(f64::to_bits),
        IDENTITY_ROTATION.map(f64::to_bits),
        "kinematic mover did not follow the scripted spin"
    );

    let mut peak_speed = 0.0_f64;
    for frame in &frames {
        let pushed_state = state_of(frame, PUSHED_CRATE)?;
        for axis in pushed_state.velocity {
            peak_speed = peak_speed.max(axis.abs());
        }
    }
    assert!(
        peak_speed > 0.0,
        "scripted impulses never reached the pushed crate"
    );

    Ok(())
}
