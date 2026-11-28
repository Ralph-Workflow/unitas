use std::error::Error;

use unitas_animation::avatar_mask::AvatarMask;
use unitas_animation::{
    AnimationClip, Animator, CurveBinding, CurveBindingTarget, FloatCurve,
    AnimatorController, AnimatorControllerLayer, AnimatorState, ConditionOp, Keyframe,
    LayerBlendingMode, ParameterDef, StateMotion, StateTransition, TransitionCondition,
    TransitionTarget, WrapMode,
};

fn pose_clip(name: &str, duration: f32, path: &str, value: f32) -> AnimationClip {
    AnimationClip::new(
        name,
        duration,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new(path, "Transform", "localPosition.x"),
            FloatCurve::new(vec![
                Keyframe::linear(0.0, value),
                Keyframe::linear(duration, value),
            ]),
        )],
        Vec::new(),
    )
}

fn sample_x(result: &unitas_animation::AnimatorStepResult, path: &str) -> Option<f32> {
    result
        .bindings
        .iter()
        .find(|binding| binding.target.object_path == path)
        .and_then(|binding| match &binding.value {
            unitas_animation::AnimatedValue::Float(v) => Some(*v),
            _ => None,
        })
}

#[test]
fn upper_layer_overrides_base_with_weighted_masked_blend() -> Result<(), Box<dyn Error>> {
    let mut controller = AnimatorController::new(
        "Locomotion",
        vec![ParameterDef::float("wave", 0.0)],
        vec![AnimatorState::new(
            "Base",
            StateMotion::Clip(pose_clip("Base", 1.0, "Root", 0.0)),
        )],
        0,
        Vec::new(),
    );
    controller.layers = vec![AnimatorControllerLayer {
        name: "Upper Body".to_string(),
        weight: 0.5,
        mask: Some(AvatarMask::new("Upper").with_path("Root/Arm")),
        blending_mode: LayerBlendingMode::Override,
        states: vec![
            AnimatorState::new(
                "Still",
                StateMotion::Clip(pose_clip("Still", 1.0, "Root/Arm", 0.0)),
            ),
            AnimatorState::new(
                "Wave",
                StateMotion::Clip(pose_clip("Wave", 1.0, "Root/Arm", 10.0)),
            ),
        ],
        entry_state: 0,
        transitions: vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("wave", ConditionOp::Greater(0.5))],
        )],
    }];

    let mut animator = Animator::new(controller);
    let step = animator.step(0.1);
    // Masked path blends 50/50 with the base layer; masked-out leg is absent.
    assert_eq!(sample_x(&step, "Root"), Some(0.0));
    assert_eq!(sample_x(&step, "Root/Arm"), Some(0.0));
    assert_eq!(sample_x(&step, "Root/Leg"), None);

    animator.set_float("wave", 1.0).map_err(|d| format!("{d:?}"))?;
    let step = animator.step(0.1);
    assert_eq!(sample_x(&step, "Root"), Some(0.0));
    // Root/Arm is unique to the upper layer: at weight 0.5 it lerps from the
    // default value, so 10.0 lands halfway at 5.0.
    let arm = sample_x(&step, "Root/Arm").ok_or("missing Root/Arm binding")?;
    assert!((arm - 5.0).abs() < 1e-4);
    Ok(())
}

#[test]
fn zero_weight_layer_advances_but_does_not_blend() -> Result<(), Box<dyn Error>> {
    let mut controller = AnimatorController::new(
        "Base",
        Vec::new(),
        vec![AnimatorState::new(
            "Base",
            StateMotion::Clip(pose_clip("Base", 1.0, "Root", 2.0)),
        )],
        0,
        Vec::new(),
    );
    controller.layers = vec![AnimatorControllerLayer {
        name: "Silent".to_string(),
        weight: 0.0,
        mask: None,
        blending_mode: LayerBlendingMode::Override,
        states: vec![AnimatorState::new(
            "Shout",
            StateMotion::Clip(pose_clip("Shout", 1.0, "Root", 99.0)),
        )],
        entry_state: 0,
        transitions: Vec::new(),
    }];

    let mut animator = Animator::new(controller);
    let step = animator.step(0.1);
    let root = sample_x(&step, "Root").ok_or("missing Root binding")?;
    assert!((root - 2.0).abs() < 1e-4);
    Ok(())
}

#[test]
fn layer_controller_survives_binary_wire_roundtrip() -> Result<(), Box<dyn Error>> {
    let mut controller = AnimatorController::new(
        "Roundtrip",
        vec![ParameterDef::float("w", 0.0)],
        vec![AnimatorState::new(
            "Base",
            StateMotion::Clip(pose_clip("Base", 1.0, "Root", 0.0)),
        )],
        0,
        Vec::new(),
    );
    controller.layers = vec![
        AnimatorControllerLayer {
            name: "Masked".to_string(),
            weight: 0.25,
            mask: Some(AvatarMask::new("M").with_path("Root/Arm")),
            blending_mode: LayerBlendingMode::Override,
            states: vec![AnimatorState::new(
                "A",
                StateMotion::BlendTreeDirect {
                    children: vec![("w".to_string(), pose_clip("A1", 1.0, "Root/Arm", 8.0))],
                },
            )],
            entry_state: 0,
            transitions: Vec::new(),
        },
        AnimatorControllerLayer {
            name: "Additive".to_string(),
            weight: 1.0,
            mask: None,
            blending_mode: LayerBlendingMode::Additive,
            states: vec![AnimatorState::new(
                "B",
                StateMotion::Clip(pose_clip("B", 1.0, "Root/Leg", 4.0)),
            )],
            entry_state: 0,
            transitions: Vec::new(),
        },
    ];

    let bytes = controller.encode_binary();
    let decoded = AnimatorController::decode_binary(&bytes)?;
    assert_eq!(decoded, controller);

    let mut animator = Animator::new(decoded);
    animator.set_float("w", 1.0).map_err(|d| format!("{d:?}"))?;
    let step = animator.step(0.1);
    // Layer 0 contributes Root/Arm at weight 0.25 (8.0 * 0.25 = 2.0, lerped
    // from the default); layer 1 (additive, approximated as override) has
    // weight 1.0 and injects Root/Leg = 4.0.
    let arm = sample_x(&step, "Root/Arm").ok_or("missing Root/Arm binding")?;
    assert!((arm - 2.0).abs() < 1e-3);
    let leg = sample_x(&step, "Root/Leg").ok_or("missing Root/Leg binding")?;
    assert!((leg - 4.0).abs() < 1e-3);
    Ok(())
}
