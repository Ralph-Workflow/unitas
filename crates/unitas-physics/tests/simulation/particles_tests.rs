use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use unitas_core::EntityId;
use unitas_physics::particles::{
    Curve, EmitParams, Particle, ParticleShape, ParticleSystem, ParticleSystemState, StopAction,
};
use unitas_physics::{BodyDesc, BodyRole, PhysicsError, Shape, World};

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
    system.set_color_over_lifetime(Curve::<4>::new(
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ));
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
        shape: Shape::Sphere { radius },
        layer: 1,
        role: BodyRole::Static,
        friction: 0.5,
        restitution: 0.0,
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
    box_system.set_shape(ParticleShape::Box { size: [1.0, 2.0, 3.0] });
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
