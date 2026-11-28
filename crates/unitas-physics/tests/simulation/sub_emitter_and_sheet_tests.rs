use unitas_core::EntityId;
use unitas_physics::particles::{
    Curve, EmitParams, ParticleSystem, SubEmitterConfig, SubEmitterTrigger, TextureSheetAnimation,
    TextureSheetAnimationType,
};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, Shape, World};

fn static_collider(position: [f64; 3], radius: f64, is_trigger: bool) -> BodyDesc {
    BodyDesc {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        mass: 0.0,
        velocity: [0.0; 3],
        use_gravity: false,
        is_trigger,
        collider_shape: ColliderShape::Primitive(Shape::Sphere { radius }),
        layer: 1,
        role: BodyRole::Static,
        friction: 0.5,
        restitution: 0.0,
        collision_detection_mode: CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn sub_emitter_birth_collision_trigger_and_death_events_fire_at_parent_particle_positions(
) -> Result<(), unitas_physics::PhysicsError> {
    let mut world = World::new();
    let solid = EntityId::new(1);
    let trigger_zone = EntityId::new(2);
    world.add_body(solid, static_collider([1.0, 0.0, 0.0], 0.5, false))?;
    world.add_body(trigger_zone, static_collider([3.0, 0.0, 0.0], 0.5, true))?;

    let mut system = configured_sub_emitter_system();
    system.set_emit_position([1.0, 0.0, 0.0]);
    system.emit(1);
    system.set_emit_position([3.0, 0.0, 0.0]);
    system.emit(1);

    let birth_events = system.drain_sub_emitter_events();
    assert_eq!(birth_events.len(), 2);
    assert_eq!(birth_events[0].trigger, SubEmitterTrigger::Birth);
    assert_eq!(birth_events[0].sub_emitter, EntityId::new(100));
    assert_eq!(birth_events[0].position, [1.0, 0.0, 0.0]);
    assert_eq!(birth_events[0].emit_count, 2);
    assert_eq!(birth_events[1].position, [3.0, 0.0, 0.0]);

    system.update_collisions(&world);
    let contact_events = system.drain_sub_emitter_events();
    assert_eq!(contact_events.len(), 2);
    assert!(contact_events.iter().any(|event| {
        event.trigger == SubEmitterTrigger::Collision
            && event.sub_emitter == EntityId::new(101)
            && event.position == [1.0, 0.0, 0.0]
            && event.emit_count == 3
    }));
    assert!(contact_events.iter().any(|event| {
        event.trigger == SubEmitterTrigger::Trigger
            && event.sub_emitter == EntityId::new(102)
            && event.position == [3.0, 0.0, 0.0]
            && event.emit_count == 4
    }));

    system.simulate(1.0);
    let death_events = system.drain_sub_emitter_events();
    assert_eq!(death_events.len(), 2);
    assert!(death_events.iter().all(|event| {
        event.trigger == SubEmitterTrigger::Death
            && event.sub_emitter == EntityId::new(103)
            && event.emit_count == 5
    }));
    Ok(())
}

fn configured_sub_emitter_system() -> ParticleSystem {
    let mut system = ParticleSystem::new(42);
    system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 0.5,
        start_lifetime: 1.0,
        start_color: [1.0; 4],
    });
    system.add_sub_emitter(SubEmitterConfig::new(
        SubEmitterTrigger::Birth,
        EntityId::new(100),
        2,
    ));
    system.add_sub_emitter(SubEmitterConfig::new(
        SubEmitterTrigger::Collision,
        EntityId::new(101),
        3,
    ));
    system.add_sub_emitter(SubEmitterConfig::new(
        SubEmitterTrigger::Trigger,
        EntityId::new(102),
        4,
    ));
    system.add_sub_emitter(SubEmitterConfig::new(
        SubEmitterTrigger::Death,
        EntityId::new(103),
        5,
    ));
    system
}

#[test]
fn texture_sheet_animation_evaluates_whole_sheet_and_single_row_uv_rects_over_lifetime() {
    let sheet = TextureSheetAnimation::new(4, 2)
        .with_cycles(1.0)
        .with_frame_over_time(Curve::new([0.0], [1.0]));

    let uv_start = sheet.evaluate_uv(0.0);
    assert_eq!(uv_start.frame_index, 0);
    assert!((uv_start.uv_rect[0] - 0.0).abs() < 1e-6);
    assert!((uv_start.uv_rect[1] - 0.0).abs() < 1e-6);
    assert!((uv_start.uv_rect[2] - 0.25).abs() < 1e-6);
    assert!((uv_start.uv_rect[3] - 0.5).abs() < 1e-6);

    let uv_mid = sheet.evaluate_uv(0.5);
    assert_eq!(uv_mid.frame_index, 4);
    assert!((uv_mid.uv_rect[0] - 0.0).abs() < 1e-6);
    assert!((uv_mid.uv_rect[1] - 0.5).abs() < 1e-6);
    assert!((uv_mid.uv_rect[2] - 0.25).abs() < 1e-6);
    assert!((uv_mid.uv_rect[3] - 0.5).abs() < 1e-6);

    let uv_end = sheet.evaluate_uv(1.0);
    assert_eq!(uv_end.frame_index, 7);
    assert!((uv_end.uv_rect[0] - 0.75).abs() < 1e-6);
    assert!((uv_end.uv_rect[1] - 0.5).abs() < 1e-6);

    let single_row = TextureSheetAnimation::new(4, 2)
        .with_animation_type(TextureSheetAnimationType::SingleRow)
        .with_row_index(1);
    let row_mid = single_row.evaluate_uv(0.5);
    assert_eq!(row_mid.frame_index, 6);
    assert!((row_mid.uv_rect[0] - 0.5).abs() < 1e-6);
    assert!((row_mid.uv_rect[1] - 0.5).abs() < 1e-6);
}

#[test]
fn particle_system_updates_particle_uvs_as_particles_age_with_texture_sheet_animation() {
    let mut system = ParticleSystem::new(9);
    system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 1.0,
        start_lifetime: 2.0,
        start_color: [1.0; 4],
    });
    system.set_texture_sheet_animation(Some(TextureSheetAnimation::new(2, 2)));
    system.emit(1);
    assert_eq!(system.particle_uvs().len(), 1);
    assert_eq!(system.particle_uvs()[0].frame_index, 0);
    assert_eq!(system.particle_uvs()[0].uv_rect, [0.0, 0.0, 0.5, 0.5]);

    system.simulate(1.0);
    assert_eq!(system.particle_uvs()[0].frame_index, 2);
    assert_eq!(system.particle_uvs()[0].uv_rect, [0.0, 0.5, 0.5, 0.5]);

    system.simulate(0.6);
    assert_eq!(system.particle_uvs()[0].frame_index, 3);
    assert_eq!(system.particle_uvs()[0].uv_rect, [0.5, 0.5, 0.5, 0.5]);
}
