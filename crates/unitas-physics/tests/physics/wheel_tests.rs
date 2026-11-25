use unitas_core::EntityId;
use unitas_physics::wheel::{
    SuspensionSpring, Vehicle, WheelCollider, WheelDesc, WheelError, WheelFrictionCurve,
};
use unitas_physics::{BodyDesc, BodyRole, Shape, World};

const ALL_LAYERS: u32 = 1;
const GROUND: EntityId = EntityId::new(1);
const STEP: f64 = 1.0 / 120.0;
const WHEEL_RADIUS: f64 = 0.35;
const WHEEL_MASS: f64 = 30.0;
const SUSPENSION_DISTANCE: f64 = 0.3;
const SETTLE_STEPS: usize = 240;
const MOTOR_TORQUE: f64 = 100.0;
const SPIN_STEPS: usize = 120;

fn ground_body(position: [f64; 3], half_extents: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        shape: Shape::Box { half_extents },
        layer: ALL_LAYERS,
        role: BodyRole::Static,
        friction: 0.6,
        restitution: 0.0,
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

fn flat_ground_world() -> World {
    let mut world = World::new();
    assert!(world
        .add_body(GROUND, ground_body([0.0, -0.5, 0.0], [50.0, 0.5, 50.0]))
        .is_ok());
    world
}

fn test_wheel(local_position: [f64; 3]) -> Result<WheelCollider, WheelError> {
    let desc = WheelDesc::new(WHEEL_RADIUS, SUSPENSION_DISTANCE)?
        .with_mass(30.0)
        .with_suspension_spring(SuspensionSpring::new(40_000.0, 4_000.0, 0.5));
    WheelCollider::new(desc, local_position)
}

fn four_wheel_vehicle() -> Result<Vehicle, WheelError> {
    let wheels = vec![
        test_wheel([-0.8, 0.0, 1.2])?,
        test_wheel([0.8, 0.0, 1.2])?,
        test_wheel([-0.8, 0.0, -1.2])?,
        test_wheel([0.8, 0.0, -1.2])?,
    ];
    Ok(Vehicle::new(1000.0, wheels)?.with_position([0.0, 0.45, 0.0]))
}

fn settle(vehicle: &mut Vehicle, world: &World) -> Result<(), WheelError> {
    for _ in 0..SETTLE_STEPS {
        vehicle.step(world, STEP)?;
    }
    Ok(())
}

fn set_motor_torque(vehicle: &mut Vehicle, torque: f64) {
    for wheel in vehicle.wheels_mut() {
        let desc = wheel.desc_mut();
        desc.has_motor = true;
        desc.motor_torque = torque;
    }
}

fn set_brake_torque(vehicle: &mut Vehicle, torque: f64) {
    for wheel in vehicle.wheels_mut() {
        wheel.desc_mut().brake_torque = torque;
    }
}

fn set_steer_angle(vehicle: &mut Vehicle, front: bool, degrees: f64) {
    for (index, wheel) in vehicle.wheels_mut().iter_mut().enumerate() {
        if (index < 2) == front {
            let desc = wheel.desc_mut();
            desc.has_steer = true;
            desc.steer_angle_deg = degrees;
        }
    }
}

#[test]
fn wheel_desc_rejects_invalid_radius_and_suspension() {
    assert!(matches!(
        WheelDesc::new(0.0, SUSPENSION_DISTANCE),
        Err(WheelError::InvalidWheelDesc)
    ));
    assert!(matches!(
        WheelDesc::new(-1.0, SUSPENSION_DISTANCE),
        Err(WheelError::InvalidWheelDesc)
    ));
    assert!(matches!(
        WheelDesc::new(f64::NAN, SUSPENSION_DISTANCE),
        Err(WheelError::InvalidWheelDesc)
    ));
    assert!(matches!(
        WheelDesc::new(WHEEL_RADIUS, 0.0),
        Err(WheelError::InvalidWheelDesc)
    ));
    assert!(matches!(
        WheelDesc::new(WHEEL_RADIUS, -0.3),
        Err(WheelError::InvalidWheelDesc)
    ));
    assert!(WheelDesc::new(WHEEL_RADIUS, SUSPENSION_DISTANCE).is_ok());
}

#[test]
fn wheel_reports_ground_hit_with_normal_point_and_compression() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;

    let wheel = &vehicle.wheels()[0];
    assert!(wheel.is_grounded());
    assert!(wheel.ground_hit().is_some());
    if let Some(hit) = wheel.ground_hit() {
        assert_eq!(hit.entity, GROUND);
        assert!((hit.normal[0]).abs() < 1e-3);
        assert!((hit.normal[1] - 1.0).abs() < 1e-3);
        assert!((hit.normal[2]).abs() < 1e-3);
        assert!(hit.point[1].abs() < 1e-4);
        assert!((hit.point[0] + 0.8).abs() < 1e-2);
        assert!((hit.point[2] - 1.2).abs() < 1e-2);
    }
    assert!(wheel.compression() > 0.0);
    assert!(wheel.compression() <= SUSPENSION_DISTANCE);
    assert_eq!(vehicle.grounded_wheel_count(), 4);
    Ok(())
}

#[test]
fn wheel_without_ground_reports_not_grounded_and_applies_no_force() -> Result<(), WheelError> {
    let world = World::new();
    let mut vehicle = four_wheel_vehicle()?;
    let start = vehicle.position();

    for _ in 0..10 {
        vehicle.step(&world, STEP)?;
    }

    for wheel in vehicle.wheels() {
        assert!(!wheel.is_grounded());
        assert!(wheel.ground_hit().is_none());
        assert_eq!(wheel.compression(), 0.0);
    }
    assert_eq!(vehicle.grounded_wheel_count(), 0);
    assert!(vehicle.position()[1] < start[1]);
    Ok(())
}

#[test]
fn suspension_compression_stays_within_the_configured_travel() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?.with_position([0.0, 3.0, 0.0]);

    for _ in 0..600 {
        vehicle.step(&world, STEP)?;
        for wheel in vehicle.wheels() {
            assert!(wheel.compression() >= 0.0);
            assert!(wheel.compression() <= SUSPENSION_DISTANCE);
        }
    }

    assert!(vehicle.position()[1] > 0.2);
    for wheel in vehicle.wheels() {
        assert!(wheel.is_grounded());
        assert!(wheel.compression() > 0.0);
    }
    Ok(())
}

#[test]
fn motor_torque_accelerates_the_vehicle() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;
    set_motor_torque(&mut vehicle, 100.0);
    let start = vehicle.velocity()[2];

    for _ in 0..240 {
        vehicle.step(&world, STEP)?;
    }

    let speed = vehicle.velocity()[2];
    assert!(
        speed > start + 0.5,
        "speed {speed} did not grow from {start}"
    );
    assert!(vehicle.position()[2] > 1.0);
    Ok(())
}

#[test]
fn wheel_without_motor_produces_no_drive() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;
    let start = vehicle.velocity()[2];

    for _ in 0..240 {
        vehicle.step(&world, STEP)?;
    }

    assert!(start.abs() < 0.05);
    assert!(vehicle.velocity()[2].abs() < 0.05);
    assert!(vehicle.position()[2].abs() < 0.05);
    for wheel in vehicle.wheels() {
        assert!(!wheel.desc().has_motor);
        assert!(wheel.rpm().abs() < 1.0);
    }
    Ok(())
}

#[test]
fn brake_torque_decelerates_the_vehicle() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?.with_velocity([0.0, 0.0, 5.0]);
    settle(&mut vehicle, &world)?;
    let cruising = vehicle.velocity()[2];
    set_brake_torque(&mut vehicle, 200.0);

    for _ in 0..240 {
        vehicle.step(&world, STEP)?;
    }

    let stopped = vehicle.velocity()[2];
    assert!(
        stopped < cruising - 0.5,
        "speed {stopped} did not fall from {cruising}"
    );
    Ok(())
}

#[test]
fn steer_angle_rotates_the_wheel_forward_direction() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    set_steer_angle(&mut vehicle, true, 20.0);
    vehicle.step(&world, STEP)?;

    let front = &vehicle.wheels()[0];
    let rear = &vehicle.wheels()[3];
    assert!(front.ground_hit().is_some());
    assert!(rear.ground_hit().is_some());
    if let Some(hit) = front.ground_hit() {
        let radians = 20.0_f64.to_radians();
        assert!((hit.forward_dir[0] - radians.sin()).abs() < 1e-4);
        assert!((hit.forward_dir[1]).abs() < 1e-4);
        assert!((hit.forward_dir[2] - radians.cos()).abs() < 1e-4);
    }
    if let Some(hit) = rear.ground_hit() {
        assert!((hit.forward_dir[0]).abs() < 1e-4);
        assert!((hit.forward_dir[2] - 1.0).abs() < 1e-4);
    }
    Ok(())
}

#[test]
fn steer_angle_is_ignored_without_the_steer_flag() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    for wheel in vehicle.wheels_mut() {
        wheel.desc_mut().steer_angle_deg = 20.0;
    }
    vehicle.step(&world, STEP)?;

    let front = &vehicle.wheels()[0];
    assert!(front.ground_hit().is_some());
    if let Some(hit) = front.ground_hit() {
        assert!((hit.forward_dir[0]).abs() < 1e-9);
        assert!((hit.forward_dir[2] - 1.0).abs() < 1e-9);
    }
    Ok(())
}

#[test]
fn steering_turns_the_vehicle() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;
    set_motor_torque(&mut vehicle, 150.0);
    set_steer_angle(&mut vehicle, true, 25.0);

    for _ in 0..240 {
        vehicle.step(&world, STEP)?;
    }

    let forward = vehicle.forward_direction();
    assert!(forward[0] > 0.05, "heading {forward:?} did not turn");
    assert!(vehicle.velocity()[0] > 0.05);
    Ok(())
}

#[test]
fn every_wheel_reports_its_own_ground_hit() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;

    let expected = [[-0.8, 1.2], [0.8, 1.2], [-0.8, -1.2], [0.8, -1.2]];
    for (wheel, position) in vehicle.wheels().iter().zip(expected.iter()) {
        assert!(wheel.is_grounded());
        assert!(wheel.ground_hit().is_some());
        if let Some(hit) = wheel.ground_hit() {
            assert_eq!(hit.entity, GROUND);
            assert!((hit.point[0] - position[0]).abs() < 1e-2);
            assert!((hit.point[2] - position[1]).abs() < 1e-2);
        }
    }
    Ok(())
}

#[test]
fn vehicle_grounds_only_the_wheels_that_reach_a_surface() -> Result<(), WheelError> {
    let mut world = World::new();
    assert!(world
        .add_body(GROUND, ground_body([0.0, -0.5, -1.2], [1.0, 0.5, 1.0]))
        .is_ok());
    let mut vehicle = four_wheel_vehicle()?;

    vehicle.step(&world, STEP)?;

    assert_eq!(vehicle.grounded_wheel_count(), 2);
    assert!(!vehicle.wheels()[0].is_grounded());
    assert!(!vehicle.wheels()[1].is_grounded());
    assert!(vehicle.wheels()[2].is_grounded());
    assert!(vehicle.wheels()[3].is_grounded());
    Ok(())
}

#[test]
fn vehicle_step_rejects_a_non_positive_delta_time() -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;

    assert!(matches!(
        vehicle.step(&world, 0.0),
        Err(WheelError::InvalidDeltaTime(_))
    ));
    assert!(matches!(
        vehicle.step(&world, -0.1),
        Err(WheelError::InvalidDeltaTime(_))
    ));
    assert!(matches!(
        vehicle.step(&world, f64::NAN),
        Err(WheelError::InvalidDeltaTime(_))
    ));
    assert!(vehicle.step(&world, STEP).is_ok());
    Ok(())
}

fn drive_scenario() -> Result<([f64; 3], [f64; 3], f64), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    settle(&mut vehicle, &world)?;
    set_motor_torque(&mut vehicle, 120.0);
    set_steer_angle(&mut vehicle, true, 10.0);
    for _ in 0..120 {
        vehicle.step(&world, STEP)?;
    }
    Ok((
        vehicle.position(),
        vehicle.velocity(),
        vehicle.wheels()[0].rpm(),
    ))
}

#[test]
fn vehicle_step_is_deterministic() -> Result<(), WheelError> {
    let first = drive_scenario()?;
    let second = drive_scenario()?;

    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
    Ok(())
}

fn free_wheel_vehicle(motor_torque: f64) -> Result<Vehicle, WheelError> {
    let desc = WheelDesc::new(WHEEL_RADIUS, SUSPENSION_DISTANCE)?
        .with_mass(WHEEL_MASS)
        .with_motor_torque(motor_torque);
    let wheel = WheelCollider::new(desc, [0.0, 0.0, 0.0])?;
    Vehicle::new(1000.0, vec![wheel])
}

fn free_wheel_angular_velocity(motor_torque: f64) -> Result<f64, WheelError> {
    let world = World::new();
    let mut vehicle = free_wheel_vehicle(motor_torque)?;
    for _ in 0..SPIN_STEPS {
        vehicle.step(&world, STEP)?;
    }
    let wheel = vehicle.wheel(0).ok_or(WheelError::InvalidVehicle)?;
    Ok(wheel.angular_velocity())
}

#[test]
fn wheel_reports_rpm_and_angular_velocity_under_torque() -> Result<(), WheelError> {
    let world = World::new();
    let mut vehicle = free_wheel_vehicle(MOTOR_TORQUE)?;
    for _ in 0..SPIN_STEPS {
        vehicle.step(&world, STEP)?;
    }

    let wheel = vehicle.wheel(0).ok_or(WheelError::InvalidVehicle)?;
    assert!(!wheel.is_grounded(), "the wheel has no ground under it");

    let inertia = WHEEL_MASS * WHEEL_RADIUS * WHEEL_RADIUS;
    let expected_angular_velocity = MOTOR_TORQUE / inertia * STEP * SPIN_STEPS as f64;
    assert!(
        (wheel.angular_velocity() - expected_angular_velocity).abs() < 1e-9,
        "angular velocity {} did not match the torque driven {expected_angular_velocity}",
        wheel.angular_velocity()
    );

    let expected_rpm = expected_angular_velocity * 60.0 / std::f64::consts::TAU;
    assert!(
        (wheel.rpm() - expected_rpm).abs() < 1e-6,
        "rpm {} did not match the angular velocity {expected_angular_velocity}",
        wheel.rpm()
    );

    let doubled = free_wheel_angular_velocity(MOTOR_TORQUE * 2.0)?;
    assert!(
        (doubled - 2.0 * wheel.angular_velocity()).abs() < 1e-9,
        "the doubled torque produced {doubled} instead of twice the angular velocity"
    );
    Ok(())
}

#[test]
fn friction_curve_follows_the_authored_shape() {
    let curve = WheelFrictionCurve::new(0.1, 2.0, 0.5, 1.0, 1.5);

    assert_eq!(curve.evaluate(0.0), 2.0);
    assert_eq!(curve.evaluate(0.1), 2.0);
    assert!((curve.evaluate(0.3) - 1.5).abs() < 1e-12);
    assert_eq!(curve.evaluate(0.5), 1.0);
    assert_eq!(curve.evaluate(4.0), 1.0);
    assert_eq!(curve.evaluate(-0.3), curve.evaluate(0.3));
}

#[test]
fn phy_004_story_6_four_wheel_vehicle_accelerates_brakes_steers_and_reports_ground_hits_per_wheel(
) -> Result<(), WheelError> {
    let world = flat_ground_world();
    let mut vehicle = four_wheel_vehicle()?;
    vehicle.configure_vehicle_substeps(5.0, 4, 2)?;
    assert_eq!(vehicle.vehicle_substeps(), (5.0, 4, 2));

    settle(&mut vehicle, &world)?;
    assert_eq!(
        vehicle.grounded_wheel_count(),
        4,
        "PHY-004 story 6: all four wheels must be grounded after settling"
    );
    for (idx, wheel) in vehicle.wheels().iter().enumerate() {
        assert!(
            wheel.is_grounded() && wheel.ground_hit().is_some(),
            "PHY-004 story 6: wheel {idx} must report a ground hit"
        );
        assert!(
            wheel.sprung_mass() > 0.0,
            "PHY-004 story 6: wheel {idx} must report positive sprung mass"
        );
        let (pose_pos, _pose_rot) = vehicle
            .wheel_world_pose(idx)
            .ok_or(WheelError::InvalidVehicle)?;
        assert!(pose_pos[1].is_finite());
    }

    set_motor_torque(&mut vehicle, 140.0);
    for _ in 0..180 {
        vehicle.step(&world, STEP)?;
    }
    let accelerated_speed = vehicle.velocity()[2];
    assert!(
        accelerated_speed > 0.8,
        "PHY-004 story 6: four-wheel vehicle must accelerate under motor torque, got speed {accelerated_speed}"
    );

    set_steer_angle(&mut vehicle, true, 22.0);
    for _ in 0..180 {
        vehicle.step(&world, STEP)?;
    }
    let steered_heading = vehicle.forward_direction();
    assert!(
        steered_heading[0] > 0.05 && vehicle.velocity()[0] > 0.05,
        "PHY-004 story 6: four-wheel vehicle must turn under steer angle, got heading {steered_heading:?}"
    );

    let pre_brake_speed = (vehicle.velocity()[0].powi(2) + vehicle.velocity()[2].powi(2)).sqrt();
    set_motor_torque(&mut vehicle, 0.0);
    set_brake_torque(&mut vehicle, 250.0);
    for _ in 0..180 {
        vehicle.step(&world, STEP)?;
    }
    let post_brake_speed = (vehicle.velocity()[0].powi(2) + vehicle.velocity()[2].powi(2)).sqrt();
    assert!(
        post_brake_speed < pre_brake_speed - 0.5,
        "PHY-004 story 6: four-wheel vehicle must decelerate under brake torque ({pre_brake_speed} -> {post_brake_speed})"
    );
    for (idx, wheel) in vehicle.wheels().iter().enumerate() {
        let hit = wheel.ground_hit().ok_or(WheelError::InvalidVehicle)?;
        assert_eq!(
            hit.entity, GROUND,
            "PHY-004 story 6: wheel {idx} ground hit must identify the ground collider entity"
        );
    }
    Ok(())
}
