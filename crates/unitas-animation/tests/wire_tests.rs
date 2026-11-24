use unitas_animation::{
    AnimationClip, AnimationError, Animator, AnimatorController, AnimatorState, ConditionOp,
    CurveBinding, CurveBindingTarget, FloatCurve, Keyframe, ParameterDef, ParameterValue,
    PropertyCurve, StateMotion, StateTransition, TransitionCondition, TransitionTarget, WrapMode,
};

fn build_rich_controller() -> AnimatorController {
    let float_curve = FloatCurve::new(vec![
        Keyframe::new(0.0, 0.0, 1.0, 1.0),
        Keyframe::linear(0.5, 2.0),
        Keyframe::stepped(1.0, 4.0),
    ]);
    let vec3_curve = PropertyCurve::Vector3([
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0), Keyframe::linear(1.0, 2.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 3.0), Keyframe::linear(1.0, 4.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 5.0), Keyframe::linear(1.0, 6.0)]),
    ]);
    let quat_curve = PropertyCurve::Quaternion([
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0)]),
    ]);
    let color_curve = PropertyCurve::Color([
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.1)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.2)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.3)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0)]),
    ]);

    let clip_a = AnimationClip::new(
        "ClipA",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                float_curve.clone(),
            ),
            CurveBinding::new(
                CurveBindingTarget::new("Child", "Transform", "localScale"),
                vec3_curve,
            ),
            CurveBinding::new(
                CurveBindingTarget::new("", "Transform", "localRotation"),
                quat_curve,
            ),
            CurveBinding::new(
                CurveBindingTarget::new("Mesh", "MeshRenderer", "material.color"),
                color_curve,
            ),
        ],
        vec![unitas_animation::AnimationEvent {
            time: 0.5,
            function_name: "OnMid".to_string(),
            float_parameter: 1.25,
            int_parameter: 42,
            string_parameter: "payload".to_string(),
        }],
    );

    let clip_b = AnimationClip::new(
        "ClipB",
        2.0,
        30.0,
        WrapMode::PingPong,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            float_curve,
        )],
        Vec::new(),
    );

    AnimatorController::new(
        "HeroController",
        vec![
            ParameterDef::float("speed", 0.5),
            ParameterDef::int("mode", 1),
            ParameterDef::bool("grounded", true),
            ParameterDef::trigger("attack"),
        ],
        vec![
            AnimatorState::with_speed("Locomotion", StateMotion::Clip(clip_a.clone()), 1.25),
            AnimatorState::new(
                "BlendState",
                StateMotion::BlendTree1D {
                    parameter: "speed".to_string(),
                    thresholds: vec![(0.0, clip_a), (1.0, clip_b)],
                },
            ),
            AnimatorState {
                name: "EmptyExit".to_string(),
                motion: StateMotion::None,
                speed: 1.0,
                is_exit: true,
            },
        ],
        0,
        vec![
            StateTransition::new(
                0,
                TransitionTarget::State(1),
                vec![
                    TransitionCondition::new("speed", ConditionOp::Greater(1.0)),
                    TransitionCondition::new("mode", ConditionOp::Equals(1)),
                    TransitionCondition::new("grounded", ConditionOp::If),
                    TransitionCondition::new("attack", ConditionOp::IfNot),
                ],
                true,
                0.75,
                0.2,
            ),
            StateTransition::new(
                1,
                TransitionTarget::Exit,
                vec![
                    TransitionCondition::new("speed", ConditionOp::Less(0.1)),
                    TransitionCondition::new("mode", ConditionOp::NotEqual(9)),
                ],
                false,
                0.0,
                0.0,
            ),
        ],
    )
}

#[test]
fn binary_wire_roundtrip_preserves_entire_controller_structure() -> Result<(), AnimationError> {
    let original = build_rich_controller();
    let encoded = original.encode_binary();
    let decoded = AnimatorController::decode_binary(&encoded)?;
    assert_eq!(decoded, original);
    Ok(())
}

#[test]
fn binary_wire_roundtrip_produces_identical_runtime_evaluation() -> Result<(), AnimationError> {
    let original = build_rich_controller();
    let bytes = original.encode_binary();
    let decoded = AnimatorController::decode_binary(&bytes)?;

    let mut anim_a = Animator::new(original);
    let mut anim_b = Animator::new(decoded);

    let _ = anim_a.set_float("speed", 2.0);
    let _ = anim_b.set_float("speed", 2.0);

    for _ in 0..5 {
        let res_a = anim_a.step(0.15);
        let res_b = anim_b.step(0.15);
        assert_eq!(res_a, res_b);
    }
    Ok(())
}

#[test]
fn decode_binary_rejects_bad_magic_and_version_and_truncated_input() {
    assert_eq!(
        AnimatorController::decode_binary(&[0, 1, 2]),
        Err(AnimationError::UnexpectedEof)
    );
    assert_eq!(
        AnimatorController::decode_binary(b"XXXX\x01\x00\x00\x00"),
        Err(AnimationError::InvalidMagic)
    );
    assert_eq!(
        AnimatorController::decode_binary(b"UANC\x09\x00\x00\x00"),
        Err(AnimationError::UnsupportedVersion(9))
    );

    let controller = build_rich_controller();
    let bytes = controller.encode_binary();
    for cut in [8usize, 12, bytes.len() / 2, bytes.len() - 1] {
        assert_eq!(
            AnimatorController::decode_binary(&bytes[..cut]),
            Err(AnimationError::UnexpectedEof)
        );
    }
}

#[test]
fn decode_binary_rejects_invalid_parameter_tag() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"UANC");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(b"Test");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(b"p");
    bytes.push(99);

    assert_eq!(
        AnimatorController::decode_binary(&bytes),
        Err(AnimationError::InvalidTag {
            section: "ParameterValue",
            tag: 99,
        })
    );
}

#[test]
fn wire_roundtrip_preserves_parameter_values() -> Result<(), AnimationError> {
    let controller = AnimatorController::new(
        "ParamCtrl",
        vec![
            ParameterDef::float("speed", 2.75),
            ParameterDef::int("mode", -3),
            ParameterDef::bool("grounded", true),
            ParameterDef::new("attack", ParameterValue::Trigger(true)),
        ],
        vec![AnimatorState::new("Idle", StateMotion::None)],
        0,
        Vec::new(),
    );

    let decoded = AnimatorController::decode_binary(&controller.encode_binary())?;
    let params = &decoded.parameters;

    assert_eq!(params.len(), 4);
    assert_eq!(params[0].name, "speed");
    assert_eq!(params[0].default_value, ParameterValue::Float(2.75));
    assert_eq!(params[1].name, "mode");
    assert_eq!(params[1].default_value, ParameterValue::Int(-3));
    assert_eq!(params[2].name, "grounded");
    assert_eq!(params[2].default_value, ParameterValue::Bool(true));
    assert_eq!(params[3].name, "attack");
    assert_eq!(params[3].default_value, ParameterValue::Trigger(true));
    Ok(())
}

#[test]
fn decode_binary_rejects_invalid_utf8_and_invalid_section_tags() {
    let mut bad_utf8 = Vec::new();
    bad_utf8.extend_from_slice(b"UANC");
    bad_utf8.extend_from_slice(&1u32.to_le_bytes());
    bad_utf8.extend_from_slice(&2u32.to_le_bytes());
    bad_utf8.extend_from_slice(&[0xFF, 0xFE]);
    assert_eq!(
        AnimatorController::decode_binary(&bad_utf8),
        Err(AnimationError::InvalidUtf8)
    );

    let mut bad_motion = Vec::new();
    bad_motion.extend_from_slice(b"UANC");
    bad_motion.extend_from_slice(&1u32.to_le_bytes());
    bad_motion.extend_from_slice(&1u32.to_le_bytes());
    bad_motion.extend_from_slice(b"C");
    bad_motion.extend_from_slice(&0u32.to_le_bytes());
    bad_motion.extend_from_slice(&1u32.to_le_bytes());
    bad_motion.extend_from_slice(&1u32.to_le_bytes());
    bad_motion.extend_from_slice(b"S");
    bad_motion.extend_from_slice(&1.0f32.to_le_bytes());
    bad_motion.push(0);
    bad_motion.push(77);
    assert_eq!(
        AnimatorController::decode_binary(&bad_motion),
        Err(AnimationError::InvalidTag {
            section: "StateMotion",
            tag: 77,
        })
    );
}

#[test]
fn animation_error_display_and_diagnostic_conversion_format_cleanly() {
    let diag = unitas_animation::AnimationDiagnostic::MissingMotion {
        state_name: "Idle".to_string(),
    };
    let err: AnimationError = diag.into();
    assert!(err.to_string().contains("Idle"));
    assert!(!AnimationError::UnexpectedEof.to_string().is_empty());
    assert!(!AnimationError::InvalidMagic.to_string().is_empty());
    assert!(AnimationError::UnsupportedVersion(2)
        .to_string()
        .contains('2'));
    assert!(!AnimationError::InvalidUtf8.to_string().is_empty());
    assert!(AnimationError::InvalidTag {
        section: "WrapMode",
        tag: 9
    }
    .to_string()
    .contains("WrapMode"));
}

#[test]
fn decode_binary_rejects_invalid_wrap_mode_property_curve_and_interpolation_mode_tags() {
    let ctrl = AnimatorController::new(
        "TagTest",
        Vec::new(),
        vec![AnimatorState::new(
            "S",
            StateMotion::Clip(AnimationClip::new(
                "C",
                1.0,
                60.0,
                WrapMode::Once,
                vec![CurveBinding::float(
                    CurveBindingTarget::transform("", "localPosition.x"),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 1.0)]),
                )],
                Vec::new(),
            )),
        )],
        0,
        Vec::new(),
    );
    let valid = ctrl.encode_binary();
    let mode_pos = valid.len() - 13;
    let mut bad_mode = valid.clone();
    bad_mode[mode_pos] = 9;
    assert_eq!(
        AnimatorController::decode_binary(&bad_mode),
        Err(AnimationError::InvalidTag {
            section: "InterpolationMode",
            tag: 9,
        })
    );
}

#[test]
fn decode_binary_rejects_invalid_transition_target_and_condition_op_tags() {
    let ctrl = AnimatorController::new(
        "TrTagTest",
        vec![ParameterDef::bool("b", false)],
        vec![AnimatorState::new("S", StateMotion::None)],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::Exit,
            vec![TransitionCondition::new("b", ConditionOp::If)],
        )],
    );
    let valid = ctrl.encode_binary();
    let op_pos = valid.len() - 1;
    let mut bad_op = valid.clone();
    bad_op[op_pos] = 44;
    assert_eq!(
        AnimatorController::decode_binary(&bad_op),
        Err(AnimationError::InvalidTag {
            section: "ConditionOp",
            tag: 44,
        })
    );
}

#[test]
fn roundtrip_clamp_forever_wrap_mode_and_empty_controller() -> Result<(), AnimationError> {
    let ctrl = AnimatorController::new(
        "ClampCtrl",
        Vec::new(),
        vec![AnimatorState::new(
            "Hold",
            StateMotion::Clip(AnimationClip::new(
                "HoldClip",
                1.0,
                24.0,
                WrapMode::ClampForever,
                Vec::new(),
                Vec::new(),
            )),
        )],
        0,
        Vec::new(),
    );
    let decoded = AnimatorController::decode_binary(&ctrl.encode_binary())?;
    assert_eq!(decoded, ctrl);
    Ok(())
}

#[test]
fn roundtrip_preserves_multi_threshold_blend_tree_and_multiple_events() -> Result<(), AnimationError> {
    let clip1 = AnimationClip::new(
        "C1",
        1.0,
        60.0,
        WrapMode::Loop,
        Vec::new(),
        vec![
            unitas_animation::AnimationEvent::with_float(0.1, "E1", 2.5),
            unitas_animation::AnimationEvent::with_int(0.2, "E2", -7),
        ],
    );
    let clip2 = AnimationClip::new("C2", 2.0, 60.0, WrapMode::Once, Vec::new(), Vec::new());
    let ctrl = AnimatorController::new(
        "MultiTree",
        vec![ParameterDef::float("p", 0.0)],
        vec![AnimatorState::new(
            "Tree",
            StateMotion::BlendTree1D {
                parameter: "p".to_string(),
                thresholds: vec![(-1.0, clip1), (1.0, clip2)],
            },
        )],
        0,
        Vec::new(),
    );
    let bytes = ctrl.encode_binary();
    let decoded = AnimatorController::decode_binary(&bytes)?;
    assert_eq!(decoded, ctrl);
    Ok(())
}


