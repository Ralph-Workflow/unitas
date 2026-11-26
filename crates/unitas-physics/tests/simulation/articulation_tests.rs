use std::f64::consts::{FRAC_PI_2, PI};

use unitas_core::EntityId;
use unitas_physics::articulation::{ArticulationBody, ArticulationError, ArticulationJoint};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, Shape, World};

const TIMESTEP: f64 = 1.0 / 60.0;
const STEPS: usize = 300;
const HALF_PI: f64 = FRAC_PI_2;
const THIRD_PI: f64 = PI / 3.0;

const ARM_CENTER_OFFSET: f64 = 0.45;
const ARM_HALF_SEGMENT: f64 = 0.37;
const FOREARM_CENTER_OFFSET: f64 = 0.30;
const FOREARM_SLIDE: f64 = 0.35;
const FOREARM_LIMIT: f64 = 0.5;

fn link(position: [f64; 3], shape: Shape) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 1.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(shape),
        layer: 0,
        role: BodyRole::Kinematic,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn arm_world() -> Result<World, Box<dyn std::error::Error>> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        link(
            [0.0, 0.0, 0.0],
            Shape::Box {
                half_extents: [0.1, 0.1, 0.1],
            },
        ),
    )?;
    world.add_body(
        EntityId::new(2),
        link(
            [ARM_CENTER_OFFSET, 0.0, 0.0],
            Shape::Capsule {
                radius: 0.08,
                height: 0.9,
                direction: 0,
            },
        ),
    )?;
    world.add_body(
        EntityId::new(3),
        link(
            [
                ARM_CENTER_OFFSET + ARM_HALF_SEGMENT + FOREARM_CENTER_OFFSET,
                0.0,
                0.0,
            ],
            Shape::Capsule {
                radius: 0.06,
                height: 0.6,
                direction: 0,
            },
        ),
    )?;
    Ok(world)
}

fn shoulder(target: f64) -> ArticulationJoint {
    ArticulationJoint::revolute(EntityId::new(1), EntityId::new(2))
        .with_anchor([0.0, 0.0, 0.0])
        .with_child_anchor([-ARM_CENTER_OFFSET, 0.0, 0.0])
        .with_axis([0.0, 0.0, 1.0])
        .with_limits(-HALF_PI, HALF_PI)
        .with_target(target)
}

fn slide(target: f64) -> ArticulationJoint {
    ArticulationJoint::prismatic(EntityId::new(2), EntityId::new(3))
        .with_anchor([ARM_HALF_SEGMENT, 0.0, 0.0])
        .with_child_anchor([-FOREARM_CENTER_OFFSET, 0.0, 0.0])
        .with_axis([1.0, 0.0, 0.0])
        .with_limits(0.0, FOREARM_LIMIT)
        .with_target(target)
}

#[test]
fn phy_004_story_7_articulated_arm_reaches_drive_targets_inside_authored_limits(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = arm_world()?;
    let mut articulation = ArticulationBody::new(vec![shoulder(THIRD_PI), slide(FOREARM_SLIDE)])?;

    let mut shoulder_low = f64::INFINITY;
    let mut shoulder_high = f64::NEG_INFINITY;
    let mut slide_low = f64::INFINITY;
    let mut slide_high = f64::NEG_INFINITY;

    for _ in 0..STEPS {
        articulation.step(&mut world, TIMESTEP)?;
        world.step(TIMESTEP)?;

        let shoulder_state = articulation
            .joint_state(0)
            .ok_or("missing shoulder state")?;
        let slide_state = articulation.joint_state(1).ok_or("missing slide state")?;
        shoulder_low = shoulder_low.min(shoulder_state.value);
        shoulder_high = shoulder_high.max(shoulder_state.value);
        slide_low = slide_low.min(slide_state.value);
        slide_high = slide_high.max(slide_state.value);
    }

    let shoulder_state = articulation
        .joint_state(0)
        .ok_or("missing shoulder state")?;
    assert!(
        (shoulder_state.value - THIRD_PI).abs() < 1e-3,
        "revolute joint reached {} instead of the authored target",
        shoulder_state.value
    );
    assert!(
        shoulder_low >= -HALF_PI - 1e-9 && shoulder_high <= HALF_PI + 1e-9,
        "revolute joint left its authored limits: {shoulder_low} to {shoulder_high}"
    );

    let slide_state = articulation.joint_state(1).ok_or("missing slide state")?;
    assert!(
        (slide_state.value - FOREARM_SLIDE).abs() < 1e-3,
        "prismatic joint reached {} instead of the authored target",
        slide_state.value
    );
    assert!(
        slide_low >= -1e-9 && slide_high <= FOREARM_LIMIT + 1e-9,
        "prismatic joint left its authored limits: {slide_low} to {slide_high}"
    );

    let reach = ARM_CENTER_OFFSET + ARM_HALF_SEGMENT + FOREARM_SLIDE + FOREARM_CENTER_OFFSET;
    let expected = [reach * THIRD_PI.cos(), reach * THIRD_PI.sin(), 0.0];
    let forearm = world
        .position(EntityId::new(3))
        .ok_or("missing forearm body")?;
    assert!(
        (forearm[0] - expected[0]).abs() < 1e-2 && (forearm[1] - expected[1]).abs() < 1e-2,
        "forearm ended at {forearm:?} instead of following the arm to {expected:?}"
    );

    Ok(())
}

#[test]
fn joint_drive_never_leaves_the_authored_limits() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = arm_world()?;
    let mut articulation =
        ArticulationBody::new(vec![shoulder(HALF_PI + 1.0), slide(FOREARM_SLIDE)])?;

    for _ in 0..STEPS {
        articulation.step(&mut world, TIMESTEP)?;
        world.step(TIMESTEP)?;

        let shoulder_state = articulation
            .joint_state(0)
            .ok_or("missing shoulder state")?;
        assert!(
            shoulder_state.value <= HALF_PI + 1e-9,
            "revolute joint drove past its upper limit to {}",
            shoulder_state.value
        );
        assert!(
            shoulder_state.value >= -HALF_PI - 1e-9,
            "revolute joint drove past its lower limit to {}",
            shoulder_state.value
        );
    }

    let shoulder_state = articulation
        .joint_state(0)
        .ok_or("missing shoulder state")?;
    assert!(
        (shoulder_state.value - HALF_PI).abs() < 1e-6,
        "revolute joint rested at {} instead of its upper limit",
        shoulder_state.value
    );

    Ok(())
}

#[test]
fn articulation_rejects_a_joint_without_an_axis() {
    let joint = ArticulationJoint::revolute(EntityId::new(1), EntityId::new(2)).with_axis([0.0; 3]);
    assert!(matches!(
        ArticulationBody::new(vec![joint]),
        Err(ArticulationError::InvalidJoint)
    ));
}

#[test]
fn articulation_rejects_a_cycle_between_links() {
    let first = ArticulationJoint::revolute(EntityId::new(1), EntityId::new(2));
    let second = ArticulationJoint::revolute(EntityId::new(2), EntityId::new(1));
    assert!(matches!(
        ArticulationBody::new(vec![first, second]),
        Err(ArticulationError::InvalidJoint)
    ));
}

#[test]
fn joint_state_reports_stepped_value_and_velocity_and_none_out_of_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = arm_world()?;
    let mut articulation = ArticulationBody::new(vec![shoulder(THIRD_PI), slide(FOREARM_SLIDE)])?;

    assert_eq!(
        articulation.joint_state(0),
        Some(unitas_physics::articulation::JointState {
            value: 0.0,
            velocity: 0.0,
        })
    );
    assert_eq!(
        articulation.joint_state(1),
        Some(unitas_physics::articulation::JointState {
            value: 0.0,
            velocity: 0.0,
        })
    );
    assert_eq!(articulation.joint_state(2), None);

    articulation.step(&mut world, TIMESTEP)?;

    let expected_shoulder_vel = 100.0 * THIRD_PI * TIMESTEP;
    let expected_shoulder_val = expected_shoulder_vel * TIMESTEP;
    let shoulder_after_one = articulation
        .joint_state(0)
        .ok_or("missing shoulder state")?;
    assert!((shoulder_after_one.velocity - expected_shoulder_vel).abs() < 1e-12);
    assert!((shoulder_after_one.value - expected_shoulder_val).abs() < 1e-12);

    let expected_slide_vel = 100.0 * FOREARM_SLIDE * TIMESTEP;
    let expected_slide_val = expected_slide_vel * TIMESTEP;
    let slide_after_one = articulation.joint_state(1).ok_or("missing slide state")?;
    assert!((slide_after_one.velocity - expected_slide_vel).abs() < 1e-12);
    assert!((slide_after_one.value - expected_slide_val).abs() < 1e-12);

    Ok(())
}
