use std::error::Error;
use std::hint::black_box;
use std::time::Instant;

use unitas_animation::{
    AnimationClip, Animator, AnimatorController, AnimatorState, CurveBinding, CurveBindingTarget,
    FloatCurve, Keyframe, StateMotion, WrapMode,
};

type TestResult = Result<(), Box<dyn Error>>;

const CHARACTERS: usize = 200;
const MEASURED_FRAMES: u32 = 120;
const FRAME_SECONDS: f32 = 1.0 / 60.0;
const EVAL_CEILING_MS: f64 = 3.6;

const TRANSFORM_CHANNELS: [&str; 10] = [
    "localPosition.x",
    "localPosition.y",
    "localPosition.z",
    "localRotation.x",
    "localRotation.y",
    "localRotation.z",
    "localRotation.w",
    "localScale.x",
    "localScale.y",
    "localScale.z",
];

fn walk_clip() -> AnimationClip {
    let bindings = TRANSFORM_CHANNELS
        .iter()
        .map(|channel| {
            CurveBinding::float(
                CurveBindingTarget::new("Armature/Hips", "Transform", *channel),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 0.0),
                    Keyframe::linear(0.5, 1.0),
                    Keyframe::linear(1.0, 0.0),
                ]),
            )
        })
        .collect();
    AnimationClip::new("Walk", 1.0, 60.0, WrapMode::Loop, bindings, Vec::new())
}

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_animation_eval_steps_two_hundred_characters_within_budget() -> TestResult {
    let controller = AnimatorController::new(
        "Locomotion",
        Vec::new(),
        vec![AnimatorState::new("Walk", StateMotion::Clip(walk_clip()))],
        0,
        Vec::new(),
    );
    let mut animators: Vec<Animator> = (0..CHARACTERS)
        .map(|_| Animator::new(controller.clone()))
        .collect();
    let started = Instant::now();
    for _ in 0..MEASURED_FRAMES {
        for animator in &mut animators {
            black_box(animator.step(FRAME_SECONDS).bindings.len());
        }
    }
    let value_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(MEASURED_FRAMES);
    println!("measurement=animation_eval value_ms={value_ms:.3} ceiling_ms={EVAL_CEILING_MS:.3}");
    if value_ms > EVAL_CEILING_MS {
        return Err(format!(
            "animation_eval measured {value_ms:.3}ms against the {EVAL_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
