//! PHY-001 deterministic rerun for the combined fixed-timestep + CCD path.
//!
//! Spec basis (docs/product-specification/physics-navigation-terrain.md, PHY-001):
//! - "The project's enhanced-determinism setting ... selects a configuration
//!   whose results depend only on inputs and target, never on job scheduling."
//! - Acceptance outcome 18: "the same simulation run twice on one target yields
//!   identical results. (PHY-001)"
//!
//! The scene is the kart/marble geometry: a 36 m/s marble with continuous CCD
//! against a 0.2 m static fence, driven through the fixed-step driver
//! (`Simulation::advance_fixed`) at Unity's default 0.02 s fixed timestep.
//! Three runs start from the same constructed snapshot: a reference run fed
//! 0.02 s frames, a rerun fed 0.01 s frame fragments that the accumulator
//! coalesces into the same fixed steps, and a full replay. One state hash is
//! recorded per fixed step; the three runs must produce bit-identical hash
//! sequences. The expected outcome is derived from the spec wording above,
//! not from a first run of the implementation.

use unitas_core::EntityId;
use unitas_physics::simulate::{Simulation, SimulationError};
use unitas_physics::{
    BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, PhysicsError, Shape, World,
};

const FIXED_TIMESTEP: f64 = 0.02;
const FIXED_STEPS: usize = 20;
const MARBLE_SPEED: f64 = 36.0;

const FENCE: EntityId = EntityId::new(1);
const MARBLE: EntityId = EntityId::new(2);
const TRACKED: [EntityId; 2] = [FENCE, MARBLE];

fn build_world() -> Result<World, PhysicsError> {
    let mut world = World::with_gravity([0.0; 3]);
    world.add_body(
        FENCE,
        BodyDesc {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            collider_shape: ColliderShape::Primitive(Shape::Box {
                half_extents: [0.1, 1.0, 1.0],
            }),
            layer: 0,
            role: BodyRole::Static,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: CollisionDetectionMode3D::Discrete,
        },
    )?;
    world.add_body(
        MARBLE,
        BodyDesc {
            position: [-3.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [MARBLE_SPEED, 0.0, 0.0],
            use_gravity: false,
            is_trigger: false,
            collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.1 }),
            layer: 0,
            role: BodyRole::Dynamic,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: CollisionDetectionMode3D::Continuous,
        },
    )?;
    Ok(world)
}

fn mix(hash: &mut u64, bits: u64) {
    *hash ^= bits;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
}

fn state_hash(world: &World) -> Result<u64, PhysicsError> {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
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
        for value in position
            .iter()
            .chain(rotation.iter())
            .chain(velocity.iter())
        {
            mix(&mut hash, value.to_bits());
        }
    }
    Ok(hash)
}

/// Feeds each entry of `frame_deltas` to the fixed-step driver as one rendered
/// frame and records one state hash per fixed step taken, plus the marble's
/// final position as evidence the CCD path engaged.
fn run(frame_deltas: &[f64]) -> Result<(Vec<u64>, f64), SimulationError> {
    let mut simulation = Simulation::new(build_world()?);
    simulation.set_fixed_timestep(FIXED_TIMESTEP)?;
    let mut hashes = Vec::new();
    for &frame_delta in frame_deltas {
        let steps = simulation.advance_fixed(frame_delta)?;
        for _ in 0..steps {
            hashes.push(state_hash(simulation.world())?);
        }
    }
    let marble_x = simulation
        .world()
        .position(MARBLE)
        .ok_or(PhysicsError::InvalidBody(MARBLE))?[0];
    Ok((hashes, marble_x))
}

#[test]
fn phy001_fixed_step_ccd_rerun_is_bit_identical_across_frame_splits_and_full_replay(
) -> Result<(), SimulationError> {
    let reference = run(&[FIXED_TIMESTEP; FIXED_STEPS])?;

    // Same snapshot, frame time arriving in 0.01 s fragments that the
    // accumulator coalesces into the same 0.02 s fixed steps.
    let split_frames = [0.01, 0.01].repeat(FIXED_STEPS);
    let split = run(&split_frames)?;

    let replay = run(&[FIXED_TIMESTEP; FIXED_STEPS])?;

    // The marble travels 0.72 m per fixed step, three times the 0.2 m fence;
    // without the CCD sweep it tunnels to roughly +11 m. Stopping short of
    // the fence proves this run exercised the continuous-collision path.
    for (label, (_, marble_x)) in [
        ("reference", &reference),
        ("split", &split),
        ("replay", &replay),
    ] {
        assert!(
            *marble_x < 0.0,
            "the continuous marble tunneled through the 0.2 m fence in the {label} run, got x = {marble_x}"
        );
    }

    assert_eq!(
        reference.0.len(),
        FIXED_STEPS,
        "the reference run must take one fixed step per 0.02 s frame"
    );
    assert_eq!(
        split.0.len(),
        FIXED_STEPS,
        "0.01 s frame fragments must coalesce into the same fixed steps"
    );
    // PHY-001: results depend only on inputs and target; the same fixed steps
    // must hash identically no matter how the frame time was split.
    assert_eq!(
        reference.0, split.0,
        "PHY-001: the same simulation run with frame time split into 0.01 s fragments diverged"
    );
    // PHY-001 outcome 18: the same simulation run twice on one target yields
    // identical results.
    assert_eq!(
        reference.0, replay.0,
        "PHY-001: a full replay of the fixed-step CCD run did not yield identical results"
    );
    Ok(())
}
