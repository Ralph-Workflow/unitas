use std::error::Error;
use std::time::Instant;

use unitas_core::EntityId;
use unitas_physics::{BodyDesc, BodyRole, Shape, World};

type TestResult = Result<(), Box<dyn Error>>;

const ACTIVE_BODIES: u32 = 2_000;
const GRID_COLUMNS: u32 = 50;
const BODY_SPACING: f64 = 1.2;
const MEASURED_STEPS: u32 = 20;
const STEP_SECONDS: f64 = 1.0 / 60.0;
const STEP_CEILING_MS: f64 = 2_088.0;

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_physics_step_moves_two_thousand_active_bodies_within_budget() -> TestResult {
    let mut world = World::new();
    for index in 0..ACTIVE_BODIES {
        world.add_body(
            EntityId::new(u64::from(index) + 1),
            BodyDesc {
                position: [
                    f64::from(index % GRID_COLUMNS) * BODY_SPACING,
                    5.0,
                    f64::from(index / GRID_COLUMNS) * BODY_SPACING,
                ],
                rotation: [0.0, 0.0, 0.0, 1.0],
                mass: 1.0,
                velocity: [0.0; 3],
                use_gravity: true,
                is_trigger: false,
                shape: Shape::Sphere { radius: 0.5 },
                layer: 0,
                role: BodyRole::Dynamic,
                friction: 0.6,
                restitution: 0.0,
            },
        )?;
    }
    let started = Instant::now();
    for _ in 0..MEASURED_STEPS {
        world.step(STEP_SECONDS)?;
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(MEASURED_STEPS);
    println!("measurement=physics_step value_ms={value_ms:.3} ceiling_ms={STEP_CEILING_MS:.3}");
    if value_ms > STEP_CEILING_MS {
        return Err(format!(
            "physics_step measured {value_ms:.3}ms against the {STEP_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
