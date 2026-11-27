use std::error::Error;
use std::time::Instant;

use unitas_core::EntityId;
use unitas_physics::two_d::{BodyDesc2D, ColliderDesc2D, RigidbodySleepMode2D, World2D};

type TestResult = Result<(), Box<dyn Error>>;

const ACTIVE_BODIES: u32 = 500;
const GRID_COLUMNS: u32 = 25;
const BODY_SPACING: f32 = 1.0;
const BODY_RADIUS: f32 = 0.5;
const START_HEIGHT: f32 = 5.0;
const MEASURED_STEPS: u32 = 20;
const STEP_SECONDS: f32 = 1.0 / 60.0;
const STEP_CEILING_MS: f64 = 12.0;

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_step2d_steps_five_hundred_active_bodies_within_budget() -> TestResult {
    let mut world = World2D::new();
    for index in 0..ACTIVE_BODIES {
        let game_object = EntityId::new(u64::from(index) + 1);
        let body = world.add_body(
            BodyDesc2D::dynamic(
                game_object,
                [
                    (index % GRID_COLUMNS) as f32 * BODY_SPACING,
                    START_HEIGHT + (index / GRID_COLUMNS) as f32 * BODY_SPACING,
                ],
            )
            .with_sleep_mode(RigidbodySleepMode2D::NeverSleep),
        );
        world.add_collider(
            ColliderDesc2D::circle_collider(game_object, BODY_RADIUS).with_attached_body(body),
        );
    }
    let started = Instant::now();
    for _ in 0..MEASURED_STEPS {
        world.step(STEP_SECONDS);
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(MEASURED_STEPS);
    println!("measurement=physics2d_step value_ms={value_ms:.3} ceiling_ms={STEP_CEILING_MS:.3}");
    if value_ms > STEP_CEILING_MS {
        return Err(format!(
            "physics2d_step measured {value_ms:.3}ms against the {STEP_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
