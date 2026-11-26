use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, BrokenJointEvent3D, CharacterJoint3D, ColliderShape,
    CollisionDetectionMode3D, ConfigurableJoint3D, ConfigurableJointMotion, FixedJoint3D,
    HingeJoint3D, Joint3D, JointDrive3D, Shape, SpringJoint3D, World,
};

fn dynamic_sphere(position: [f64; 3], mass: f64) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.25 }),
        layer: 0,
        role: BodyRole::Dynamic,
        friction: 0.5,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn static_box(position: [f64; 3]) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger: false,
        collider_shape: ColliderShape::Primitive(Shape::Box {
            half_extents: [0.25, 0.25, 0.25],
        }),
        layer: 0,
        role: BodyRole::Static,
        friction: 0.5,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

fn quat_z(angle: f64) -> [f64; 4] {
    let half = 0.5 * angle;
    [0.0, 0.0, half.sin(), half.cos()]
}

#[test]
fn fixed_joint_3d_locks_relative_position_and_orientation() -> Result<(), Box<dyn std::error::Error>>
{
    let mut world = World::with_gravity([0.0, -9.81, 0.0]);
    let base = EntityId::new(1);
    let child = EntityId::new(2);

    world.add_body(base, static_box([0.0, 5.0, 0.0]))?;
    let mut child_desc = dynamic_sphere([3.0, 1.0, -2.0], 1.0);
    child_desc.use_gravity = true;
    child_desc.rotation = quat_z(0.8);
    world.add_body(child, child_desc)?;

    let joint_id = world.add_joint_3d(Joint3D::Fixed(
        FixedJoint3D::new(base, Some(child)).with_anchors([1.0, 0.0, 0.0], [-0.5, 0.0, 0.0]),
    ))?;
    assert_eq!(world.joint_count_3d(), 1);
    assert!(world.joint_3d(joint_id).is_some());

    for _ in 0..10 {
        world.step(0.02)?;
    }

    let pos = world.position(child).ok_or("missing child position")?;
    let rot = world.rotation(child).ok_or("missing child rotation")?;
    assert!(
        (pos[0] - 1.5).abs() < 1e-4 && (pos[1] - 5.0).abs() < 1e-4 && pos[2].abs() < 1e-4,
        "FixedJoint3D must hold child at world anchor offset [1.5, 5.0, 0.0], got {pos:?}"
    );
    assert!(
        rot[0].abs() < 1e-4
            && rot[1].abs() < 1e-4
            && rot[2].abs() < 1e-4
            && (rot[3] - 1.0).abs() < 1e-4,
        "FixedJoint3D must lock child rotation to base orientation, got {rot:?}"
    );

    assert!(world.remove_joint_3d(joint_id));
    assert_eq!(world.joint_count_3d(), 0);
    Ok(())
}

#[test]
fn hinge_joint_3d_enforces_axis_motor_spring_and_angle_limits(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let base = EntityId::new(10);
    let door = EntityId::new(11);

    world.add_body(base, static_box([0.0, 0.0, 0.0]))?;
    world.add_body(door, dynamic_sphere([1.0, 0.0, 0.0], 1.0))?;

    let hinge = HingeJoint3D::new(base, Some(door), [0.0, 0.0, 1.0])
        .with_anchors([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0])
        .with_motor(6.0, 100.0, false)
        .with_limits(-0.4, 0.5);
    let joint_id = world.add_joint_3d(Joint3D::Hinge(hinge))?;

    for _ in 0..30 {
        world.step(0.02)?;
    }

    let angle = world
        .hinge_angle_3d(joint_id)
        .ok_or("missing hinge angle")?;
    assert!(
        (angle - 0.5).abs() < 1e-3,
        "HingeJoint3D motor and upper limit must clamp angle to 0.5 rad, got {angle}"
    );
    let door_pos = world.position(door).ok_or("missing door position")?;
    assert!(
        (door_pos[0] - 0.5_f64.cos()).abs() < 1e-2
            && (door_pos[1] - 0.5_f64.sin()).abs() < 1e-2
            && door_pos[2].abs() < 1e-4,
        "HingeJoint3D door position must swing along arc of radius 1.0 at angle 0.5, got {door_pos:?}"
    );

    world.remove_joint_3d(joint_id);
    let spring_hinge = HingeJoint3D::new(base, Some(door), [0.0, 0.0, 1.0])
        .with_anchors([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0])
        .with_spring(60.0, 12.0, -0.2)
        .with_limits(-0.4, 0.5);
    let spring_id = world.add_joint_3d(Joint3D::Hinge(spring_hinge))?;

    for _ in 0..80 {
        world.step(0.02)?;
    }

    let spring_angle = world
        .hinge_angle_3d(spring_id)
        .ok_or("missing spring hinge angle")?;
    assert!(
        (spring_angle - (-0.2)).abs() < 0.05,
        "HingeJoint3D spring must pull hinge toward target_position -0.2, got {spring_angle}"
    );
    Ok(())
}

#[test]
fn spring_joint_3d_pulls_bodies_toward_distance_band_with_damping(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let anchor = EntityId::new(20);
    let bob = EntityId::new(21);

    world.add_body(anchor, static_box([0.0, 0.0, 0.0]))?;
    world.add_body(bob, dynamic_sphere([5.0, 0.0, 0.0], 1.0))?;

    let sj = SpringJoint3D::new(anchor, Some(bob), 40.0, 8.0)
        .with_distance_band(1.0, 2.0)
        .with_tolerance(0.01);
    world.add_joint_3d(Joint3D::Spring(sj))?;

    for _ in 0..80 {
        world.step(0.02)?;
    }

    let pos = world.position(bob).ok_or("missing bob position")?;
    assert!(
        pos[0] >= 0.95 && pos[0] <= 2.1,
        "SpringJoint3D must pull stretched body from x=5.0 into the [1.0, 2.0] distance band, got x={}",
        pos[0]
    );
    Ok(())
}

#[test]
fn character_joint_3d_anchors_bodies_and_clamps_twist_and_swing_cone(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let pelvis = EntityId::new(30);
    let thigh = EntityId::new(31);

    world.add_body(pelvis, static_box([0.0, 2.0, 0.0]))?;
    let mut thigh_desc = dynamic_sphere([2.0, 0.0, 0.0], 1.0);
    thigh_desc.rotation = quat_z(1.2);
    world.add_body(thigh, thigh_desc)?;

    let cj = CharacterJoint3D::new(pelvis, Some(thigh))
        .with_anchors([0.0, -0.5, 0.0], [0.0, 0.5, 0.0])
        .with_axes([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
        .with_twist_limits(-0.25, 0.25)
        .with_swing_limits(0.3, 0.35);
    world.add_joint_3d(Joint3D::Character(cj))?;

    world.step(0.02)?;

    let thigh_pos = world.position(thigh).ok_or("missing thigh position")?;
    let thigh_rot = world.rotation(thigh).ok_or("missing thigh rotation")?;
    let half_angle = thigh_rot[3].clamp(-1.0, 1.0).acos();
    let total_angle = 2.0 * half_angle;
    assert!(
        total_angle <= 0.36,
        "CharacterJoint3D must clamp initial 1.2 rad Z-swing to swing2_limit 0.35 rad, got {total_angle}"
    );
    let socket_world = [0.0, 1.5, 0.0];
    let dist_from_socket = ((thigh_pos[0] - socket_world[0]).powi(2)
        + (thigh_pos[1] - socket_world[1]).powi(2)
        + (thigh_pos[2] - socket_world[2]).powi(2))
    .sqrt();
    assert!(
        (dist_from_socket - 0.5).abs() < 1e-3,
        "CharacterJoint3D must keep thigh anchor at distance 0.5 from socket, got {dist_from_socket}"
    );
    Ok(())
}

#[test]
fn configurable_joint_3d_enforces_locked_limited_free_axes_and_position_drives(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let frame = EntityId::new(40);
    let carriage = EntityId::new(41);

    world.add_body(frame, static_box([0.0, 0.0, 0.0]))?;
    let mut carriage_desc = dynamic_sphere([0.0, 3.0, 4.0], 1.0);
    carriage_desc.rotation = quat_z(0.9);
    world.add_body(carriage, carriage_desc)?;

    let cfg = ConfigurableJoint3D::new(frame, Some(carriage))
        .with_linear_motions(
            ConfigurableJointMotion::Free,
            ConfigurableJointMotion::Locked,
            ConfigurableJointMotion::Limited,
        )
        .with_angular_motions(
            ConfigurableJointMotion::Locked,
            ConfigurableJointMotion::Locked,
            ConfigurableJointMotion::Limited,
        )
        .with_linear_limit(1.25)
        .with_angular_limits(-0.1, 0.1, 0.1, 0.3)
        .with_x_drive(JointDrive3D::new(50.0, 10.0, 500.0))
        .with_target_position([2.0, 0.0, 0.0]);
    world.add_joint_3d(Joint3D::Configurable(cfg))?;

    for _ in 0..60 {
        world.step(0.02)?;
    }

    let pos = world
        .position(carriage)
        .ok_or("missing carriage position")?;
    let rot = world
        .rotation(carriage)
        .ok_or("missing carriage rotation")?;
    let z_angle = 2.0 * rot[2].atan2(rot[3]);
    assert!(
        (pos[0] - 2.0).abs() < 0.08,
        "ConfigurableJoint3D x_drive on Free X axis must drive carriage to target x=2.0, got {}",
        pos[0]
    );
    assert!(
        pos[1].abs() < 1e-4,
        "ConfigurableJoint3D Locked Y axis must clamp y to 0.0, got {}",
        pos[1]
    );
    assert!(
        (pos[2] - 1.25).abs() < 1e-4,
        "ConfigurableJoint3D Limited Z axis must clamp z=4.0 to linear_limit 1.25, got {}",
        pos[2]
    );
    assert!(
        (z_angle - 0.3).abs() < 1e-3,
        "ConfigurableJoint3D Limited Angular Z must clamp 0.9 rad to 0.3 rad, got {z_angle}"
    );
    Ok(())
}

#[test]
fn joint_3d_breaks_when_force_or_torque_exceeds_threshold_and_emits_event(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let anchor = EntityId::new(50);
    let heavy_pull = EntityId::new(51);
    let heavy_twist = EntityId::new(52);

    world.add_body(anchor, static_box([0.0, 0.0, 0.0]))?;
    world.add_body(heavy_pull, dynamic_sphere([5.0, 0.0, 0.0], 1.0))?;
    let mut twist_desc = dynamic_sphere([0.0, 0.0, 0.0], 1.0);
    twist_desc.rotation = quat_z(1.0);
    world.add_body(heavy_twist, twist_desc)?;

    let force_joint = world.add_joint_3d(Joint3D::Spring(
        SpringJoint3D::new(anchor, Some(heavy_pull), 100.0, 0.0)
            .with_distance_band(0.0, 1.0)
            .with_break_force(50.0),
    ))?;
    let torque_joint = world.add_joint_3d(Joint3D::Fixed(
        FixedJoint3D::new(anchor, Some(heavy_twist)).with_break_torque(10.0),
    ))?;
    assert_eq!(world.joint_count_3d(), 2);

    world.step(0.02)?;

    assert_eq!(world.joint_count_3d(), 0);
    assert!(world.joint_3d(force_joint).is_none());
    assert!(world.joint_3d(torque_joint).is_none());

    let broken: Vec<BrokenJointEvent3D> = world.drain_broken_joints_3d();
    assert_eq!(broken.len(), 2);
    assert_eq!(broken[0].joint_id, force_joint);
    assert_eq!(broken[0].entity, anchor);
    assert!(broken[0].break_force > 50.0);
    assert_eq!(broken[1].joint_id, torque_joint);
    assert_eq!(broken[1].entity, anchor);
    assert!(broken[1].break_force > 10.0);
    assert!(world.drain_broken_joints_3d().is_empty());
    Ok(())
}
