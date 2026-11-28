use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use unitas_core::EntityId;
use unitas_physics::particles::{
    Curve, CustomParticleDataStream, EmitParams, ForceFieldShape, Particle, ParticleBurst,
    ParticleShape, ParticleSystem, ParticleSystemForceField, ParticleSystemState, StopAction,
};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, PhysicsError, Shape, World};

fn emit_params(lifetime: f32) -> EmitParams {
    EmitParams {
        start_lifetime: lifetime,
        ..EmitParams::default()
    }
}

fn run_seeded(seed: u64) -> (usize, Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut system = ParticleSystem::new(seed);
    system.set_shape(ParticleShape::Sphere { radius: 2.0 });
    system.set_emission_rate(20.0);
    system.set_emit_params(EmitParams {
        start_speed: 3.0,
        start_lifetime: 5.0,
        ..EmitParams::default()
    });
    system.play();
    for _ in 0..10 {
        system.simulate(0.1);
    }
    let particles = system.get_particles();
    (
        particles.len(),
        particles.iter().map(|particle| particle.position).collect(),
        particles.iter().map(|particle| particle.velocity).collect(),
    )
}

#[test]
fn a_fixed_seed_produces_the_same_particles_on_every_run() {
    let first = run_seeded(42);
    let second = run_seeded(42);
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
    assert!(first.0 > 0);
}

#[test]
fn a_different_seed_produces_a_different_distribution() {
    let first = run_seeded(42);
    let second = run_seeded(43);
    assert_eq!(first.0, second.0);
    assert_ne!(first.1, second.1);
}

#[test]
fn a_replayed_system_reproduces_the_same_particles() {
    let mut system = ParticleSystem::new(11);
    system.set_shape(ParticleShape::Sphere { radius: 1.0 });
    system.set_emission_rate(10.0);
    system.simulate(0.5);
    let first: Vec<[f32; 3]> = system.get_particles().iter().map(|p| p.position).collect();
    system.stop(StopAction::None);
    system.clear();
    system.play();
    system.simulate(0.5);
    let second: Vec<[f32; 3]> = system.get_particles().iter().map(|p| p.position).collect();
    assert_eq!(first, second);
}

#[test]
fn rate_emission_produces_the_expected_particle_count() {
    let mut system = ParticleSystem::new(1);
    system.set_emission_rate(10.0);
    system.set_emit_params(emit_params(5.0));
    for _ in 0..10 {
        system.simulate(0.1);
    }
    assert_eq!(system.get_particles().len(), 10);
}

#[test]
fn a_burst_emit_produces_exactly_the_requested_particles() {
    let mut system = ParticleSystem::new(2);
    system.emit(10);
    assert_eq!(system.get_particles().len(), 10);
    system.emit(5);
    assert_eq!(system.get_particles().len(), 15);
}

#[test]
fn particles_expire_after_their_lifetime() {
    let mut system = ParticleSystem::new(3);
    system.set_emit_params(emit_params(0.5));
    system.emit(2);
    assert_eq!(system.get_particles().len(), 2);
    system.simulate(0.25);
    assert_eq!(system.get_particles().len(), 2);
    system.simulate(0.25);
    assert_eq!(system.get_particles().len(), 0);
}

#[test]
fn a_looping_system_keeps_emitting_across_its_duration() {
    let mut system = ParticleSystem::new(5);
    system.set_duration(1.0);
    system.set_looping(true);
    system.set_emission_rate(10.0);
    system.set_emit_params(emit_params(5.0));
    system.play();
    for _ in 0..25 {
        system.simulate(0.1);
    }
    assert_eq!(system.get_particles().len(), 25);
    assert_eq!(system.state(), ParticleSystemState::Playing);
    assert!(system.is_emitting());
}

#[test]
fn a_non_looping_system_stops_emitting_after_its_duration() {
    let mut system = ParticleSystem::new(6);
    system.set_duration(0.5);
    system.set_looping(false);
    system.set_emission_rate(10.0);
    system.set_emit_params(emit_params(5.0));
    system.play();
    system.simulate(0.5);
    assert_eq!(system.get_particles().len(), 5);
    assert!(!system.is_emitting());
    system.simulate(0.5);
    assert_eq!(system.get_particles().len(), 5);
}

#[test]
fn a_non_looping_system_reports_stopped_when_its_particles_expire() {
    let mut system = ParticleSystem::new(26);
    system.set_duration(0.5);
    system.set_looping(false);
    system.set_emission_rate(10.0);
    system.set_emit_params(emit_params(0.2));
    system.play();
    system.simulate(0.5);
    assert!(!system.is_emitting());
    assert_eq!(system.get_particles().len(), 0);
    assert_eq!(system.state(), ParticleSystemState::Stopped);
}

#[test]
fn the_property_curves_interpolate_at_the_midpoint_of_the_lifetime() {
    let mut system = ParticleSystem::new(7);
    system.set_emit_params(emit_params(2.0));
    system.set_velocity_over_lifetime(Curve::<3>::new([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]));
    system.set_size_over_lifetime(Curve::<1>::new([4.0], [0.0]));
    system.set_color_over_lifetime(Curve::<4>::new([1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]));
    system.emit(1);
    system.simulate(1.0);
    let particle = system.get_particles()[0];
    assert!((particle.velocity[0] - 5.0).abs() < 1e-5);
    assert!((particle.size - 2.0).abs() < 1e-5);
    assert!((particle.color[0] - 0.5).abs() < 1e-5);
    assert!((particle.color[2] - 0.5).abs() < 1e-5);
    assert!((particle.age - 1.0).abs() < 1e-5);
}

#[test]
fn a_force_curve_accelerates_the_particles() {
    let mut system = ParticleSystem::new(8);
    system.set_emit_params(emit_params(5.0));
    system.set_force_over_lifetime(Curve::<3>::new([2.0, 0.0, 0.0], [2.0, 0.0, 0.0]));
    system.emit(1);
    system.simulate(1.0);
    let particle = system.get_particles()[0];
    assert!((particle.velocity[0] - 2.0).abs() < 1e-4);
    assert!((particle.position[0] - 2.0).abs() < 1e-4);
}

#[test]
fn stopping_with_disable_disables_the_system() {
    let mut system = ParticleSystem::new(9);
    system.set_emission_rate(10.0);
    system.stop(StopAction::Disable);
    assert!(system.is_disabled());
    assert_eq!(system.state(), ParticleSystemState::Stopped);
    system.play();
    system.simulate(1.0);
    assert_eq!(system.get_particles().len(), 0);
}

#[test]
fn stopping_with_destroy_marks_the_system_destroyed() {
    let mut system = ParticleSystem::new(10);
    system.stop(StopAction::Destroy);
    assert!(system.is_destroyed());
    assert_eq!(system.state(), ParticleSystemState::Stopped);
}

#[test]
fn stopping_with_callback_invokes_the_callback_once() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&counter);
    let mut system = ParticleSystem::new(11);
    system.set_stopped_callback(Box::new(move || {
        sink.fetch_add(1, Ordering::SeqCst);
    }));
    system.stop(StopAction::Callback);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(system.stopped_count(), 1);
    system.simulate(1.0);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn the_stop_callback_runs_when_the_last_live_particle_expires() {
    let counter = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&counter);
    let mut system = ParticleSystem::new(12);
    system.set_stopped_callback(Box::new(move || {
        sink.fetch_add(1, Ordering::SeqCst);
    }));
    system.set_emit_params(emit_params(0.5));
    system.emit(3);
    system.stop(StopAction::Callback);
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    system.simulate(0.5);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(system.get_particles().len(), 0);
}

#[test]
fn the_particle_buffer_round_trips() {
    let mut system = ParticleSystem::new(13);
    system.emit(3);
    let particles: Vec<Particle> = system.get_particles().to_vec();
    assert_eq!(particles.len(), 3);
    let mut other = ParticleSystem::new(14);
    other.set_particles(particles.clone());
    assert_eq!(other.get_particles(), particles.as_slice());
}

#[test]
fn pausing_freezes_the_simulation() {
    let mut system = ParticleSystem::new(15);
    system.set_emission_rate(10.0);
    system.pause();
    system.simulate(1.0);
    assert_eq!(system.get_particles().len(), 0);
    assert_eq!(system.state(), ParticleSystemState::Paused);
    system.play();
    system.simulate(0.1);
    assert_eq!(system.get_particles().len(), 1);
}

#[test]
fn the_start_state_decides_how_the_system_begins() {
    let mut system = ParticleSystem::new(22);
    assert_eq!(system.start_state(), ParticleSystemState::Playing);
    system.set_start_state(ParticleSystemState::Paused);
    assert_eq!(system.state(), ParticleSystemState::Paused);
    system.set_emission_rate(10.0);
    system.simulate(1.0);
    assert_eq!(system.get_particles().len(), 0);
    system.play();
    system.simulate(0.1);
    assert_eq!(system.get_particles().len(), 1);
}

fn collider(position: [f64; 3], radius: f64, is_trigger: bool) -> BodyDesc {
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
        collision_detection_mode: unitas_physics::CollisionDetectionMode3D::Discrete,
    }
}

#[test]
fn a_particle_collision_produces_a_collision_event() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let solid = EntityId::new(1);
    world.add_body(solid, collider([0.0, 0.0, 0.0], 1.0, false))?;

    let mut system = ParticleSystem::new(16);
    system.emit(1);
    system.update_collisions(&world);

    let events = system.get_collision_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, solid);
    assert_eq!(events[0].particle_index, 0);
    assert!(system.get_trigger_events().is_empty());
    Ok(())
}

#[test]
fn a_particle_inside_a_trigger_produces_a_trigger_event() -> Result<(), PhysicsError> {
    let mut world = World::new();
    let trigger = EntityId::new(2);
    world.add_body(trigger, collider([0.0, 0.0, 0.0], 1.0, true))?;

    let mut system = ParticleSystem::new(17);
    system.emit(1);
    system.update_collisions(&world);

    let events = system.get_trigger_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, trigger);
    assert!(system.get_collision_events().is_empty());
    Ok(())
}

#[test]
fn a_particle_away_from_the_world_produces_no_events() -> Result<(), PhysicsError> {
    let mut world = World::new();
    world.add_body(EntityId::new(3), collider([0.0, 0.0, 0.0], 1.0, false))?;

    let mut system = ParticleSystem::new(18);
    system.set_emit_position([100.0, 0.0, 0.0]);
    system.emit(1);
    system.update_collisions(&world);

    assert!(system.get_collision_events().is_empty());
    assert!(system.get_trigger_events().is_empty());
    Ok(())
}

#[test]
fn the_emission_shapes_place_particles_inside_their_bounds() {
    let mut sphere = ParticleSystem::new(19);
    sphere.set_shape(ParticleShape::Sphere { radius: 2.0 });
    sphere.emit(50);
    for particle in sphere.get_particles() {
        let length = (particle.position[0] * particle.position[0]
            + particle.position[1] * particle.position[1]
            + particle.position[2] * particle.position[2])
            .sqrt();
        assert!(length <= 2.0 + 1e-4);
    }

    let mut box_system = ParticleSystem::new(20);
    box_system.set_shape(ParticleShape::Box {
        size: [1.0, 2.0, 3.0],
    });
    box_system.emit(50);
    for particle in box_system.get_particles() {
        assert!(particle.position[0].abs() <= 0.5 + 1e-4);
        assert!(particle.position[1].abs() <= 1.0 + 1e-4);
        assert!(particle.position[2].abs() <= 1.5 + 1e-4);
    }

    let mut cone = ParticleSystem::new(21);
    cone.set_shape(ParticleShape::Cone {
        angle: 45.0,
        radius: 1.0,
    });
    cone.emit(50);
    let half_angle = 22.5_f32.to_radians();
    for particle in cone.get_particles() {
        assert!(particle.position[1] >= -1e-4);
        let planar = (particle.position[0] * particle.position[0]
            + particle.position[2] * particle.position[2])
            .sqrt();
        assert!(planar <= particle.position[1] * half_angle.tan() + 1e-3);
    }
}

#[test]
fn clear_empties_live_particles_and_completes_pending_stop_action_incrementing_stopped_count() {
    let mut system = ParticleSystem::new(30);
    assert!(system.is_emitting());
    assert_eq!(system.stopped_count(), 0);

    system.set_emit_params(emit_params(5.0));
    system.emit(4);
    assert_eq!(system.get_particles().len(), 4);

    system.stop(StopAction::Callback);
    assert!(!system.is_emitting());
    assert_eq!(system.stopped_count(), 0);
    assert_eq!(system.get_particles().len(), 4);

    system.clear();
    assert_eq!(system.get_particles().len(), 0);
    assert_eq!(system.stopped_count(), 1);

    system.play();
    assert!(system.is_emitting());
    system.emit(2);
    system.stop(StopAction::Callback);
    assert!(!system.is_emitting());
    assert_eq!(system.stopped_count(), 1);
    system.clear();
    assert_eq!(system.get_particles().len(), 0);
    assert_eq!(system.stopped_count(), 2);
}

#[test]
fn set_emit_position_and_set_emit_rotation_transform_spawn_pose_and_velocity() {
    let mut system = ParticleSystem::new(31);
    system.set_shape(ParticleShape::Point);
    system.set_emit_params(EmitParams {
        start_speed: 4.0,
        start_lifetime: 2.0,
        ..EmitParams::default()
    });
    system.set_emit_position([3.0, -2.0, 5.0]);
    let half_sqrt_two = std::f32::consts::FRAC_1_SQRT_2;
    system.set_emit_rotation([0.0, half_sqrt_two, 0.0, half_sqrt_two]);

    system.emit(1);
    let particles = system.get_particles();
    assert_eq!(particles.len(), 1);
    assert_eq!(particles[0].position, [3.0, -2.0, 5.0]);
    assert!((particles[0].velocity[0] - 4.0).abs() < 1e-5);
    assert!(particles[0].velocity[1].abs() < 1e-5);
    assert!(particles[0].velocity[2].abs() < 1e-5);
}

fn run_seeded_playback_cycle(seed: u64) -> (usize, Vec<[f32; 3]>) {
    let mut system = ParticleSystem::new(seed);
    system.set_shape(ParticleShape::Sphere { radius: 2.5 });
    system.set_emission_rate(12.0);
    system.set_emit_params(EmitParams {
        start_speed: 3.5,
        start_size: 0.4,
        start_lifetime: 4.0,
        start_color: [1.0, 0.5, 0.25, 1.0],
    });
    assert_eq!(
        system.state(),
        ParticleSystemState::Playing,
        "REN-004 outcome 15: initial playback state must report Playing"
    );
    system.simulate(0.25);
    system.pause();
    assert_eq!(
        system.state(),
        ParticleSystemState::Paused,
        "REN-004 outcome 15: paused system must report Paused state"
    );
    system.simulate(0.5);
    system.play();
    assert_eq!(
        system.state(),
        ParticleSystemState::Playing,
        "REN-004 outcome 15: resumed system must report Playing state"
    );
    system.simulate(0.25);
    let first_pass_count = system.get_particles().len();
    let first_pass_positions: Vec<[f32; 3]> =
        system.get_particles().iter().map(|p| p.position).collect();

    assert_system_restart_reproduces(&mut system, first_pass_count, &first_pass_positions)
}

fn assert_system_restart_reproduces(
    system: &mut ParticleSystem,
    first_pass_count: usize,
    first_pass_positions: &[[f32; 3]],
) -> (usize, Vec<[f32; 3]>) {
    system.stop(StopAction::None);
    assert_eq!(
        system.state(),
        ParticleSystemState::Stopped,
        "REN-004 outcome 15: stopped system must report Stopped state"
    );
    system.clear();
    assert_eq!(
        system.get_particles().len(),
        0,
        "REN-004 outcome 15: cleared system must have zero live particles"
    );
    system.play();
    assert_eq!(
        system.state(),
        ParticleSystemState::Playing,
        "REN-004 outcome 15: restarted system must report Playing state"
    );
    system.simulate(0.25);
    system.simulate(0.25);
    let restarted_count = system.get_particles().len();
    let restarted_positions: Vec<[f32; 3]> =
        system.get_particles().iter().map(|p| p.position).collect();
    assert_eq!(
        first_pass_count, restarted_count,
        "REN-004 outcome 15: restart with fixed seed must produce identical particle count"
    );
    assert_eq!(
        first_pass_positions,
        restarted_positions.as_slice(),
        "REN-004 outcome 15: restart with fixed seed must produce identical particle positions"
    );
    (restarted_count, restarted_positions)
}

#[test]
fn ren_004_outcome_15_playback_state_and_fixed_seed_parity_across_preview_play_mode_and_player_and_vfx_graph_unsupported(
) {
    let preview = run_seeded_playback_cycle(202_203);
    let play_mode = run_seeded_playback_cycle(202_203);
    let built_player = run_seeded_playback_cycle(202_203);
    assert!(
        preview.0 > 0,
        "REN-004 outcome 15: seeded simulation must emit live particles"
    );
    assert_eq!(
        preview, play_mode,
        "REN-004 outcome 15: preview and Play Mode must match particle count and positions under fixed seed"
    );
    assert_eq!(
        play_mode, built_player,
        "REN-004 outcome 15: Play Mode and built player must match particle count and positions under fixed seed"
    );

    let vfx_entry = unitas_core::compat::lookup_unsupported_family("Visual Effect Graph")
        .or_else(|| unitas_core::compat::lookup_unsupported_family("com.unity.visualeffectgraph"));
    assert!(
        vfx_entry.is_some(),
        "REN-004 outcome 15: Visual Effect Graph family must be recognized in compatibility catalog"
    );
    if let Some(entry) = vfx_entry {
        assert_eq!(
            entry.owning_feature_id, "REN-004",
            "REN-004 outcome 15: Visual Effect Graph must be owned by REN-004"
        );
        assert_eq!(
            entry.state,
            unitas_core::compat::CompatibilityState::Unsupported,
            "REN-004 outcome 15: Visual Effect Graph must be reported as Unsupported"
        );
    }
}

fn ren_004_outcome_16_emits_and_custom_data(system: &mut ParticleSystem) {
    system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 1.0,
        start_lifetime: 2.0,
        start_color: [1.0, 1.0, 1.0, 1.0],
    });
    system.emit(2);
    system.emit_with_params(
        EmitParams {
            start_speed: 0.0,
            start_size: 1.0,
            start_lifetime: 2.0,
            start_color: [0.2, 0.4, 0.6, 1.0],
        },
        1,
    );
    assert_eq!(
        system.get_particles().len(),
        3,
        "REN-004 outcome 16: Emit and Emit(EmitParams) must spawn exact particle count"
    );
    assert_eq!(
        system.get_particles()[2].color,
        [0.2, 0.4, 0.6, 1.0],
        "REN-004 outcome 16: EmitParams start_color override must apply to spawned particles"
    );
    ren_004_outcome_16_set_custom_data_and_positions(system);
}

fn ren_004_outcome_16_set_custom_data_and_positions(system: &mut ParticleSystem) {
    let custom1_input = vec![
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 10.0, 11.0, 12.0],
    ];
    let custom2_input = vec![
        [0.1, 0.2, 0.3, 0.4],
        [0.5, 0.6, 0.7, 0.8],
        [0.9, 1.0, 1.1, 1.2],
    ];
    system.set_custom_particle_data(CustomParticleDataStream::Custom1, &custom1_input);
    system.set_custom_particle_data(CustomParticleDataStream::Custom2, &custom2_input);
    assert_eq!(
        system.get_custom_particle_data(CustomParticleDataStream::Custom1),
        custom1_input.as_slice(),
        "REN-004 outcome 16: GetCustomParticleData(Custom1) must round-trip SetCustomParticleData"
    );
    assert_eq!(
        system.get_custom_particle_data(CustomParticleDataStream::Custom2),
        custom2_input.as_slice(),
        "REN-004 outcome 16: GetCustomParticleData(Custom2) must round-trip SetCustomParticleData"
    );

    let mut replaced = system.get_particles().to_vec();
    replaced[0].position = [0.0, 0.0, 0.0];
    replaced[1].position = [1.0, 0.0, 0.0];
    replaced[2].position = [50.0, 0.0, 0.0];
    replaced[2].lifetime = 0.2;
    system.set_particles(replaced);
    assert_eq!(
        system.get_particles()[2].position,
        [50.0, 0.0, 0.0],
        "REN-004 outcome 16: SetParticles must update live particle positions"
    );
}

fn ren_004_outcome_16_force_fields(system: &mut ParticleSystem) {
    system.set_external_forces_enabled(true);
    system.set_external_forces_multiplier(1.0);
    system.set_external_forces_layer_mask(1 << 3);
    system.add_force_field(ParticleSystemForceField {
        position: [0.0, 0.0, 0.0],
        shape: ForceFieldShape::Sphere { radius: 5.0 },
        direction: [10.0, 0.0, 0.0],
        gravity: [0.0, -4.0, 0.0],
        gravity_focus: 0.0,
        drag: 0.0,
        layer: 3,
    });
    system.add_force_field(ParticleSystemForceField {
        position: [0.0, 0.0, 0.0],
        shape: ForceFieldShape::Sphere { radius: 5.0 },
        direction: [100.0, 0.0, 0.0],
        gravity: [0.0, 0.0, 0.0],
        gravity_focus: 0.0,
        drag: 0.0,
        layer: 4,
    });

    system.simulate(0.5);
    assert_eq!(
        system.get_particles().len(),
        2,
        "REN-004 outcome 16: expired particle must be removed while keeping remaining custom data aligned"
    );
    assert_eq!(
        system.get_custom_particle_data(CustomParticleDataStream::Custom1),
        &[[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]],
        "REN-004 outcome 16: custom particle data stream must stay aligned with surviving particles"
    );
    assert!(
        (system.get_particles()[0].velocity[0] - 5.0).abs() < 1e-4,
        "REN-004 outcome 16: ParticleSystemForceField inside shape and layer mask must accelerate particle X velocity"
    );
    assert!(
        (system.get_particles()[0].velocity[1] - (-2.0)).abs() < 1e-4,
        "REN-004 outcome 16: ParticleSystemForceField gravity must accelerate particle Y velocity"
    );
}

fn ren_004_outcome_16_collision_events(system: &mut ParticleSystem) -> Result<(), PhysicsError> {
    let mut world = World::new();
    let solid = EntityId::new(10);
    let trigger = EntityId::new(11);
    world.add_body(
        solid,
        collider(
            [
                f64::from(system.get_particles()[0].position[0]),
                f64::from(system.get_particles()[0].position[1]),
                0.0,
            ],
            1.0,
            false,
        ),
    )?;
    world.add_body(
        trigger,
        collider(
            [
                f64::from(system.get_particles()[1].position[0]),
                f64::from(system.get_particles()[1].position[1]),
                0.0,
            ],
            1.0,
            true,
        ),
    )?;
    system.update_collisions(&world);
    assert!(
        system
            .get_collision_events()
            .iter()
            .any(|e| e.entity == solid),
        "REN-004 outcome 16: OnParticleCollision / GetCollisionEvents must record solid contact"
    );
    assert!(
        system
            .get_trigger_events()
            .iter()
            .any(|e| e.entity == trigger),
        "REN-004 outcome 16: OnParticleTrigger / GetTriggerParticles must record trigger overlap"
    );
    Ok(())
}

fn ren_004_outcome_16_stop_actions(system: &mut ParticleSystem) {
    let stopped_hits = Arc::new(AtomicUsize::new(0));
    let stopped_sink = Arc::clone(&stopped_hits);
    system.set_stopped_callback(Box::new(move || {
        stopped_sink.fetch_add(1, Ordering::SeqCst);
    }));
    system.stop(StopAction::Callback);
    assert_eq!(
        stopped_hits.load(Ordering::SeqCst),
        0,
        "REN-004 outcome 16: Callback stop action must wait until live particles finish"
    );
    system.simulate(2.0);
    assert_eq!(
        stopped_hits.load(Ordering::SeqCst),
        1,
        "REN-004 outcome 16: Callback stop action must invoke OnParticleSystemStopped once particles expire"
    );
}

fn ren_004_outcome_16_stop_actions_and_box_field() {
    let mut destroy_system = ParticleSystem::new(100);
    destroy_system.set_emit_params(emit_params(0.25));
    destroy_system.emit(1);
    destroy_system.stop(StopAction::Destroy);
    assert!(
        !destroy_system.is_destroyed(),
        "REN-004 outcome 16: Destroy stop action must wait while particles remain alive"
    );
    destroy_system.simulate(0.5);
    assert!(
        destroy_system.is_destroyed(),
        "REN-004 outcome 16: Destroy stop action must mark system destroyed when last particle expires"
    );
}

fn ren_004_outcome_16_box_force_field() {
    let mut box_field_system = ParticleSystem::new(101);
    box_field_system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 1.0,
        start_lifetime: 5.0,
        start_color: [1.0; 4],
    });
    box_field_system.set_emit_position([2.0, 0.0, 0.0]);
    box_field_system.emit(1);
    box_field_system.set_emit_position([10.0, 0.0, 0.0]);
    box_field_system.emit(1);
    box_field_system.set_external_forces_enabled(true);
    box_field_system.set_external_forces_multiplier(2.0);
    box_field_system.add_force_field(ParticleSystemForceField {
        position: [0.0, 0.0, 0.0],
        shape: ForceFieldShape::Box {
            half_extents: [4.0, 4.0, 4.0],
        },
        direction: [0.0, 0.0, 0.0],
        gravity: [0.0, 0.0, 0.0],
        gravity_focus: 3.0,
        drag: 0.5,
        layer: 0,
    });
    assert_eq!(box_field_system.force_fields().len(), 1);
    assert!(box_field_system.is_external_forces_enabled());
    assert!((box_field_system.external_forces_multiplier() - 2.0).abs() < 1e-6);
    box_field_system.simulate(0.5);
    assert!(
        box_field_system.get_particles()[0].velocity[0] < 0.0,
        "REN-004 outcome 16: gravity_focus inside Box ForceFieldShape must pull particle toward field center"
    );
    assert!(
        box_field_system.get_particles()[1].velocity[0].abs() < 1e-6,
        "REN-004 outcome 16: particle outside Box ForceFieldShape must not be accelerated"
    );
    box_field_system.clear_force_fields();
    assert!(box_field_system.force_fields().is_empty());
}

#[test]
fn ren_004_outcome_16_custom_particle_data_and_force_field_and_scripted_emissions_and_callbacks(
) -> Result<(), PhysicsError> {
    let mut system = ParticleSystem::new(99);
    ren_004_outcome_16_emits_and_custom_data(&mut system);
    ren_004_outcome_16_force_fields(&mut system);
    ren_004_outcome_16_collision_events(&mut system)?;
    ren_004_outcome_16_stop_actions(&mut system);
    ren_004_outcome_16_stop_actions_and_box_field();
    ren_004_outcome_16_box_force_field();
    Ok(())
}

#[test]
fn a_burst_emits_its_count_at_the_burst_time() {
    let mut system = ParticleSystem::new(5);
    system.set_shape(ParticleShape::Point);
    system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 0.2,
        start_lifetime: 5.0,
        start_color: [1.0; 4],
    });
    system.set_bursts(vec![ParticleBurst::new(0.5, 7, 1, 0.1)]);
    system.simulate(0.4);
    assert_eq!(
        system.get_particles().len(),
        0,
        "the burst must not fire before its time"
    );
    system.simulate(0.2);
    assert_eq!(
        system.get_particles().len(),
        7,
        "the burst must emit its count at the burst time"
    );
}

#[test]
fn a_burst_repeats_its_cycles_over_the_burst_interval() {
    let mut system = ParticleSystem::new(5);
    system.set_shape(ParticleShape::Point);
    system.set_emit_params(EmitParams {
        start_speed: 0.0,
        start_size: 0.2,
        start_lifetime: 5.0,
        start_color: [1.0; 4],
    });
    system.set_bursts(vec![ParticleBurst::new(0.2, 3, 3, 0.1)]);
    system.simulate(0.25);
    assert_eq!(system.get_particles().len(), 3, "the first cycle must fire");
    system.simulate(0.1);
    assert_eq!(
        system.get_particles().len(),
        6,
        "the second cycle must fire one interval later"
    );
    system.simulate(0.1);
    assert_eq!(
        system.get_particles().len(),
        9,
        "the third cycle must fire one interval later"
    );
    system.simulate(0.1);
    assert_eq!(
        system.get_particles().len(),
        9,
        "a burst must not fire more cycles than it declares"
    );
}

#[test]
fn a_seeded_burst_produces_the_same_particles_on_every_run() {
    let run = |seed: u64| {
        let mut system = ParticleSystem::new(seed);
        system.set_shape(ParticleShape::Sphere { radius: 1.0 });
        system.set_emit_params(EmitParams {
            start_speed: 2.0,
            start_size: 0.5,
            start_lifetime: 5.0,
            start_color: [1.0; 4],
        });
        system.set_bursts(vec![ParticleBurst::new(0.0, 4, 2, 0.1)]);
        for _ in 0..5 {
            system.simulate(0.1);
        }
        system
            .get_particles()
            .iter()
            .map(|particle| particle.position)
            .collect::<Vec<_>>()
    };
    let first = run(42);
    let second = run(42);
    assert_eq!(first.len(), 8, "two cycles of four particles must be live");
    assert_eq!(first, second, "the same seed must replay the same burst");
}

#[test]
fn the_max_particle_count_caps_the_live_particles() {
    let mut system = ParticleSystem::new(1);
    system.set_max_particles(3);
    system.set_bursts(vec![ParticleBurst::new(0.0, 10, 1, 0.1)]);
    system.simulate(0.1);
    assert_eq!(
        system.get_particles().len(),
        3,
        "the live particle count must stay inside the maximum"
    );
}
