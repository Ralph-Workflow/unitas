use unitas_core::EntityId;
use unitas_physics::{
    BodyDesc, BodyRole, CharacterController, CollisionFlags, PhysicsError, Shape, World,
};

const ALL_LAYERS: u32 = 1;

fn static_box(position: [f64; 3], half_extents: [f64; 3]) -> BodyDesc {
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
    }
}

fn oriented_static_box(
    position: [f64; 3],
    rotation: [f64; 4],
    half_extents: [f64; 3],
) -> BodyDesc {
    BodyDesc {
        rotation,
        ..static_box(position, half_extents)
    }
}

fn rotation_about_z(angle_deg: f64) -> [f64; 4] {
    let half_rad = 0.5 * angle_deg.to_radians();
    [0.0, 0.0, half_rad.sin(), half_rad.cos()]
}

#[test]
fn character_lands_on_ground_and_reports_below_and_grounded() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let ground = EntityId::new(1);
    world.add_body(ground, static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]))?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let result = controller.move_character(&world, [0.0, 1.2, 0.0], [0.0, -0.5, 0.0])?;

    assert!(result.is_grounded);
    assert!(result.collision_flags.contains(CollisionFlags::BELOW));
    assert!(!result.collision_flags.contains(CollisionFlags::ABOVE));
    assert!(!result.collision_flags.contains(CollisionFlags::SIDES));
    assert_eq!(result.ground_entity, Some(ground));
    assert!(controller.is_grounded());
    assert_eq!(controller.collision_flags(), result.collision_flags);
    assert_eq!(controller.ground_entity(), Some(ground));
    assert!((result.position[1] - (1.0 + controller.skin_width)).abs() < 0.05);
    Ok(())
}

#[test]
fn character_hits_wall_and_slides_along_surface_with_sides_flag() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 2.0, 0.0], [0.5, 2.0, 5.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let result = controller.move_character(&world, [0.5, 1.08, 0.0], [2.0, -0.1, 1.5])?;

    assert!(result.collision_flags.contains(CollisionFlags::SIDES));
    assert!(result.collision_flags.contains(CollisionFlags::BELOW));
    assert!(result.is_grounded);
    assert!(result.position[0] <= 1.0 + controller.skin_width + 1e-3);
    assert!((result.position[2] - 1.5).abs() < 1e-3);
    Ok(())
}

#[test]
fn character_hits_ceiling_and_clamps_upward_motion_with_above_flag() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, 3.0, 0.0], [5.0, 0.5, 5.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let result = controller.move_character(&world, [0.0, 1.2, 0.0], [0.0, 1.0, 0.0])?;

    assert!(!result.is_grounded);
    assert!(result.collision_flags.contains(CollisionFlags::ABOVE));
    assert!(!result.collision_flags.contains(CollisionFlags::BELOW));
    assert!(result.position[1] <= 1.5 - controller.skin_width + 1e-3);
    Ok(())
}

#[test]
fn character_climbs_step_within_step_offset_and_stays_grounded() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 0.1, 0.0], [1.0, 0.1, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.step_offset = 0.3;

    let mut pos = [0.0, 1.08, 0.0];
    for _ in 0..10 {
        let res = controller.move_character(&world, pos, [0.25, -0.1, 0.0])?;
        pos = res.position;
    }

    assert!(pos[0] > 1.5);
    assert!(pos[1] >= 1.2);
    assert!(controller.is_grounded());
    assert!(controller.collision_flags().contains(CollisionFlags::BELOW));
    Ok(())
}

#[test]
fn step_taller_than_step_offset_blocks_forward_motion_and_sets_sides() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 0.25, 0.0], [1.0, 0.25, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.step_offset = 0.3;

    let result = controller.move_character(&world, [0.4, 1.08, 0.0], [1.0, -0.1, 0.0])?;

    assert!(result.position[0] <= 0.5 + controller.skin_width + 1e-2);
    assert!(result.position[1] < 1.2);
    assert!(result.collision_flags.contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn step_blocked_by_low_overhead_ceiling_cannot_be_climbed() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 0.125, 0.0], [1.0, 0.125, 2.0]),
    )?;
    world.add_body(
        EntityId::new(3),
        static_box([2.0, 2.65, 0.0], [1.0, 0.5, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.step_offset = 0.3;

    let result = controller.move_character(&world, [0.4, 1.08, 0.0], [1.0, -0.1, 0.0])?;

    assert!(result.position[0] <= 0.5 + controller.skin_width + 1e-2);
    assert!(result.collision_flags.contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn character_climbs_walkable_slope_within_slope_limit() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        oriented_static_box([3.0, 0.5, 0.0], rotation_about_z(25.0), [3.0, 0.5, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.slope_limit_deg = 45.0;

    let mut pos = [0.0, 1.08, 0.0];
    for _ in 0..15 {
        let res = controller.move_character(&world, pos, [0.25, -0.1, 0.0])?;
        pos = res.position;
    }

    assert!(pos[0] > 2.0);
    assert!(pos[1] > 1.3);
    assert!(controller.is_grounded());
    assert!(controller.collision_flags().contains(CollisionFlags::BELOW));
    Ok(())
}

#[test]
fn excessive_slope_steeper_than_slope_limit_acts_as_wall() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        oriented_static_box([2.5, 1.5, 0.0], rotation_about_z(60.0), [3.0, 0.5, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.slope_limit_deg = 45.0;
    controller.step_offset = 0.2;

    let mut pos = [0.0, 1.08, 0.0];
    for _ in 0..15 {
        let res = controller.move_character(&world, pos, [0.25, -0.1, 0.0])?;
        pos = res.position;
    }

    assert!(pos[1] < 1.35);
    assert!(controller.collision_flags().contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn narrow_step_too_thin_to_support_tread_is_rejected() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([1.0, 0.1, 0.0], [0.02, 0.1, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.step_offset = 0.3;

    let result = controller.move_character(&world, [0.35, 1.08, 0.0], [0.25, -0.1, 0.0])?;

    assert!(result.position[1] < 1.15);
    assert!(result.collision_flags.contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn kinematic_platform_moved_via_set_position_carries_grounded_character() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let platform = EntityId::new(1);
    world.add_body(
        platform,
        BodyDesc {
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            layer: ALL_LAYERS,
            role: BodyRole::Kinematic,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let landed = controller.move_character_with_dt(&world, [0.0, 1.2, 0.0], [0.0, -0.3, 0.0], 0.1)?;
    assert!(landed.is_grounded);
    assert_eq!(landed.ground_entity, Some(platform));

    world.set_position(platform, [0.5, -0.5, -0.25])?;
    world.step(0.1)?;

    let carried = controller.move_character_with_dt(&world, landed.position, [0.0, -0.05, 0.0], 0.1)?;
    assert!(carried.is_grounded);
    assert!((carried.position[0] - 0.5).abs() < 1e-2);
    assert!((carried.position[2] - (-0.25)).abs() < 1e-2);
    Ok(())
}

#[test]
fn moving_platform_carries_grounded_character_via_world_velocity() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let platform = EntityId::new(1);
    world.add_body(
        platform,
        BodyDesc {
            position: [0.0, -0.5, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            velocity: [4.0, 0.0, -2.0],
            use_gravity: false,
            is_trigger: false,
            shape: Shape::Box {
                half_extents: [5.0, 0.5, 5.0],
            },
            layer: ALL_LAYERS,
            role: BodyRole::Kinematic,
            friction: 0.6,
            restitution: 0.0,
        },
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let first =
        controller.move_character_with_dt(&world, [0.0, 1.2, 0.0], [0.0, -0.3, 0.0], 0.1)?;
    assert!(first.is_grounded);
    assert_eq!(first.ground_entity, Some(platform));

    let second =
        controller.move_character_with_dt(&world, first.position, [0.0, -0.05, 0.0], 0.25)?;
    assert!(second.is_grounded);
    assert!((second.position[0] - (first.position[0] + 1.0)).abs() < 1e-3);
    assert!((second.position[2] - (first.position[2] - 0.5)).abs() < 1e-3);
    Ok(())
}

#[test]
fn teleport_sets_position_unconditionally_and_clears_transient_state() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 2.0, 0.0], [0.5, 2.0, 5.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let landed = controller.move_character(&world, [0.0, 1.2, 0.0], [0.0, -0.3, 0.0])?;
    assert!(landed.is_grounded);
    assert!(!controller.collision_flags().is_empty());

    let destination = controller.teleport([5.0, 4.0, -3.0])?;
    assert_eq!(destination, [5.0, 4.0, -3.0]);
    assert!(!controller.is_grounded());
    assert_eq!(controller.collision_flags(), CollisionFlags::NONE);
    assert_eq!(controller.ground_entity(), None);
    Ok(())
}

#[test]
fn overlapping_start_depenetrates_character_out_of_collider() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, 1.0, 0.0], [1.0, 1.0, 1.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let result = controller.move_character(&world, [1.2, 1.0, 0.0], [0.0, 0.0, 0.0])?;

    assert!(result.position[0] >= 1.5 - 1e-3);
    assert!(result.collision_flags.contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn crouch_shape_change_allows_passing_under_low_ceiling() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 1.9, 0.0], [1.0, 0.5, 2.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let standing = controller.move_character(&world, [0.3, 1.08, 0.0], [1.5, -0.05, 0.0])?;
    assert!(standing.position[0] <= 0.5 + controller.skin_width + 1e-2);
    assert!(standing.collision_flags.contains(CollisionFlags::SIDES));

    controller.height = 1.0;
    controller.center = [0.0, -0.5, 0.0];
    let crouched = controller.move_character(&world, [0.3, 1.08, 0.0], [1.5, -0.05, 0.0])?;
    assert!((crouched.position[0] - 1.8).abs() < 1e-2);
    assert!(!crouched.collision_flags.contains(CollisionFlags::SIDES));
    Ok(())
}

#[test]
fn multiple_motions_in_one_frame_compose_cleanly() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        static_box([2.0, 2.0, 0.0], [0.5, 2.0, 5.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    let first = controller.move_character(&world, [0.0, 1.2, 0.0], [0.5, -0.3, 0.0])?;
    assert!(first.is_grounded);
    assert!((first.position[0] - 0.5).abs() < 1e-3);

    let second = controller.move_character(&world, first.position, [1.0, 0.0, 0.75])?;
    assert!(second.collision_flags.contains(CollisionFlags::SIDES));
    assert!(second.position[0] <= 1.0 + controller.skin_width + 1e-3);
    assert!((second.position[2] - 0.75).abs() < 1e-3);
    Ok(())
}

#[test]
fn disabled_controller_and_sub_threshold_motion_do_not_sweep() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.min_move_distance = 0.01;
    let tiny = controller.move_character(&world, [0.0, 2.0, 0.0], [0.001, 0.0, 0.0])?;
    assert_eq!(tiny.position, [0.0, 2.0, 0.0]);
    assert_eq!(tiny.collision_flags, CollisionFlags::NONE);

    controller.enabled = false;
    let disabled = controller.move_character(&world, [0.0, 2.0, 0.0], [5.0, -5.0, 0.0])?;
    assert_eq!(disabled.position, [0.0, 2.0, 0.0]);
    assert!(!disabled.is_grounded);
    assert_eq!(disabled.collision_flags, CollisionFlags::NONE);
    Ok(())
}

#[test]
fn invalid_controller_configurations_return_physics_error() {
    let world = World::new();

    assert_eq!(
        CharacterController::new(0.0, 2.0),
        Err(PhysicsError::InvalidCharacterController)
    );
    assert_eq!(
        CharacterController::new(-0.5, 2.0),
        Err(PhysicsError::InvalidCharacterController)
    );
    assert_eq!(
        CharacterController::new(0.5, 0.0),
        Err(PhysicsError::InvalidCharacterController)
    );
    assert_eq!(
        CharacterController::new(f64::NAN, 2.0),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut bad_step = CharacterController::default();
    bad_step.step_offset = 2.5;
    assert_eq!(
        bad_step.move_character(&world, [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut negative_step = CharacterController::default();
    negative_step.step_offset = -0.1;
    assert_eq!(
        negative_step.validate(),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut bad_skin = CharacterController::default();
    bad_skin.skin_width = 0.0;
    assert_eq!(
        bad_skin.validate(),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut bad_slope = CharacterController::default();
    bad_slope.slope_limit_deg = 95.0;
    assert_eq!(
        bad_slope.validate(),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut bad_min_move = CharacterController::default();
    bad_min_move.min_move_distance = -0.01;
    assert_eq!(
        bad_min_move.validate(),
        Err(PhysicsError::InvalidCharacterController)
    );

    let mut valid = CharacterController::default();
    assert_eq!(
        valid.teleport([f64::NAN, 0.0, 0.0]),
        Err(PhysicsError::InvalidCharacterController)
    );
}

#[test]
fn triggers_excluded_entity_and_layer_mask_are_ignored_by_character_move() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(
        EntityId::new(1),
        static_box([0.0, -0.5, 0.0], [10.0, 0.5, 10.0]),
    )?;
    world.add_body(
        EntityId::new(2),
        BodyDesc {
            is_trigger: true,
            ..static_box([1.0, 1.0, 0.0], [0.5, 1.0, 1.0])
        },
    )?;
    world.add_body(
        EntityId::new(3),
        BodyDesc {
            layer: 2,
            ..static_box([1.5, 1.0, 0.0], [0.5, 1.0, 1.0])
        },
    )?;
    let self_entity = EntityId::new(4);
    world.add_body(
        self_entity,
        static_box([0.0, 1.08, 0.0], [0.5, 1.0, 0.5]),
    )?;

    let mut controller = CharacterController::new(0.5, 2.0)?;
    controller.layer_mask = ALL_LAYERS;
    controller.excluded_entity = Some(self_entity);

    let result = controller.move_character(&world, [0.0, 1.08, 0.0], [3.0, -0.05, 0.0])?;
    assert!((result.position[0] - 3.0).abs() < 1e-3);
    assert!(!result.collision_flags.contains(CollisionFlags::SIDES));
    assert!(result.is_grounded);
    Ok(())
}

#[test]
fn collision_flags_bit_operations_compose_and_mask() {
    let mut flags = CollisionFlags::empty();
    assert!(flags.is_empty());
    assert_eq!(flags.bits(), 0);

    flags |= CollisionFlags::SIDES | CollisionFlags::BELOW;
    assert!(!flags.is_empty());
    assert!(flags.contains(CollisionFlags::SIDES));
    assert!(flags.contains(CollisionFlags::BELOW));
    assert!(!flags.contains(CollisionFlags::ABOVE));
    assert!(flags.intersects(CollisionFlags::SIDES));
    assert_eq!(flags.bits(), 5);

    flags &= CollisionFlags::BELOW;
    assert_eq!(flags, CollisionFlags::BELOW);
    assert_eq!(
        CollisionFlags::from_bits(0xFF),
        CollisionFlags::SIDES | CollisionFlags::ABOVE | CollisionFlags::BELOW
    );
}

