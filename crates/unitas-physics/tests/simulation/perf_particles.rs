use std::error::Error;
use std::time::Instant;

use unitas_core::EntityId;
use unitas_physics::particles::{EmitParams, ParticleShape, ParticleSystem};
use unitas_physics::{BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, Shape, World};

type TestResult = Result<(), Box<dyn Error>>;

const SYSTEM_COUNT: usize = 8;
const SEEDED_PARTICLES: u32 = 500;
const EMISSION_RATE: f32 = 60.0;
const START_LIFETIME: f32 = 10.0;
const COLLIDER_COUNT: u32 = 20;
const MEASURED_FRAMES: u32 = 120;
const STEP_SECONDS: f32 = 1.0 / 60.0;
const STEP_CEILING_MS: f64 = 10.0;

fn build_colliders() -> Result<World, Box<dyn Error>> {
    let mut world = World::new();
    for index in 0..COLLIDER_COUNT {
        world.add_body(
            EntityId::new(u64::from(index) + 1),
            BodyDesc {
                position: [f64::from(index % 5) * 2.0, 0.0, f64::from(index / 5) * 2.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 0.0,
                velocity: [0.0; 3],
                use_gravity: false,
                is_trigger: false,
                collider_shape: ColliderShape::Primitive(Shape::Sphere { radius: 0.5 }),
                layer: 0,
                role: BodyRole::Static,
                friction: 0.6,
                restitution: 0.0,
                collision_detection_mode: CollisionDetectionMode3D::Discrete,
            },
        )?;
    }
    Ok(world)
}

fn build_system(seed: u64) -> ParticleSystem {
    let mut system = ParticleSystem::new(seed);
    system.set_shape(ParticleShape::Sphere { radius: 1.0 });
    system.set_emission_rate(EMISSION_RATE);
    system.set_emit_params(EmitParams {
        start_speed: 2.0,
        start_size: 0.25,
        start_lifetime: START_LIFETIME,
        ..EmitParams::default()
    });
    system.emit(SEEDED_PARTICLES);
    system
}

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_particle_eval_steps_thousands_of_live_particles_within_budget() -> TestResult {
    let world = build_colliders()?;
    let mut systems: Vec<ParticleSystem> = Vec::with_capacity(SYSTEM_COUNT);
    for index in 0..SYSTEM_COUNT {
        systems.push(build_system(u64::try_from(index)? + 1));
    }
    let live_particles: usize = systems
        .iter()
        .map(|system| system.get_particles().len())
        .sum();
    if live_particles < 4_000 {
        return Err(
            format!("perf_particle_eval seeded only {live_particles} live particles").into(),
        );
    }
    let started = Instant::now();
    for _ in 0..MEASURED_FRAMES {
        for system in &mut systems {
            system.simulate(STEP_SECONDS);
            system.update_collisions(&world);
        }
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(MEASURED_FRAMES);
    println!("measurement=particle_eval value_ms={value_ms:.3} ceiling_ms={STEP_CEILING_MS:.3}");
    if value_ms > STEP_CEILING_MS {
        return Err(format!(
            "particle_eval measured {value_ms:.3}ms against the {STEP_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
