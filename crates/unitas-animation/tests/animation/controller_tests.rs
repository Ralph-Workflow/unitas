use unitas_animation::{
    AnimatedValue, AnimationClip, AnimationDiagnostic, AnimationEvent, Animator,
    AnimatorController, AnimatorState, ConditionOp, CurveBinding, CurveBindingTarget, FloatCurve,
    Keyframe, ParameterDef, PropertyCurve, StateMotion, StateTransition, TransitionCondition,
    TransitionTarget, WrapMode,
};

fn simple_clip(name: &str, duration: f32, x0: f32, x1: f32) -> AnimationClip {
    AnimationClip::new(
        name,
        duration,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![
                Keyframe::linear(0.0, x0),
                Keyframe::linear(duration, x1),
            ]),
        )],
        Vec::new(),
    )
}

#[test]
fn animator_starts_in_entry_state_and_advances_normalized_time_with_speed() {
    let controller = AnimatorController::new(
        "Locomotion",
        Vec::new(),
        vec![AnimatorState::with_speed(
            "Idle",
            StateMotion::Clip(simple_clip("IdleClip", 2.0, 0.0, 4.0)),
            1.5,
        )],
        0,
        Vec::new(),
    );

    let mut animator = Animator::new(controller);
    animator.set_speed(2.0);
    assert!((animator.speed() - 2.0).abs() < 1e-6);

    let step = animator.step(0.25);
    let info = step.state_info;
    assert_eq!(info.state_index, 0);
    assert_eq!(info.state_name, "Idle");
    assert!((info.normalized_time - 0.375).abs() < 1e-5);
    assert_eq!(step.bindings.len(), 1);
    let AnimatedValue::Float(x) = step.bindings[0].value else {
        assert_eq!(step.bindings[0].value, AnimatedValue::Float(0.0));
        return;
    };
    assert!((x - 1.5).abs() < 1e-5);
}

#[test]
fn float_parameter_greater_and_less_conditions_drive_transition() {
    let controller = AnimatorController::new(
        "SpeedCtrl",
        vec![ParameterDef::float("speed", 0.0)],
        vec![
            AnimatorState::new(
                "Idle",
                StateMotion::Clip(simple_clip("Idle", 1.0, 0.0, 0.0)),
            ),
            AnimatorState::new(
                "Run",
                StateMotion::Clip(simple_clip("Run", 1.0, 10.0, 20.0)),
            ),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new("speed", ConditionOp::Greater(0.5))],
            ),
            StateTransition::instant(
                1,
                TransitionTarget::State(0),
                vec![TransitionCondition::new("speed", ConditionOp::Less(0.2))],
            ),
        ],
    );

    let mut animator = Animator::new(controller);
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_name, "Idle");

    assert_eq!(animator.set_float("speed", 1.0), Ok(()));
    assert_eq!(animator.get_float("speed"), Some(1.0));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_name, "Run");

    assert_eq!(animator.set_float("speed", 0.1), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_name, "Idle");
}

#[test]
fn int_and_bool_parameters_and_and_or_transition_semantics() {
    let controller = AnimatorController::new(
        "Combat",
        vec![
            ParameterDef::bool("grounded", false),
            ParameterDef::int("stance", 0),
            ParameterDef::bool("override", false),
        ],
        vec![
            AnimatorState::new("Air", StateMotion::Clip(simple_clip("Air", 1.0, 0.0, 1.0))),
            AnimatorState::new(
                "Ready",
                StateMotion::Clip(simple_clip("Ready", 1.0, 5.0, 5.0)),
            ),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![
                    TransitionCondition::new("grounded", ConditionOp::If),
                    TransitionCondition::new("stance", ConditionOp::Equals(2)),
                ],
            ),
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new("override", ConditionOp::If)],
            ),
            StateTransition::instant(
                1,
                TransitionTarget::State(0),
                vec![
                    TransitionCondition::new("grounded", ConditionOp::IfNot),
                    TransitionCondition::new("stance", ConditionOp::NotEqual(2)),
                ],
            ),
        ],
    );

    let mut animator = Animator::new(controller);
    assert_eq!(animator.set_bool("grounded", true), Ok(()));
    assert_eq!(animator.get_bool("grounded"), Some(true));
    assert_eq!(animator.set_int("stance", 1), Ok(()));
    assert_eq!(animator.get_int("stance"), Some(1));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 0);

    assert_eq!(animator.set_int("stance", 2), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 1);

    assert_eq!(animator.set_bool("grounded", false), Ok(()));
    assert_eq!(animator.set_int("stance", 0), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 0);

    assert_eq!(animator.set_bool("override", true), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 1);
}

#[test]
fn trigger_parameter_is_consumed_when_transition_fires_and_can_be_reset() {
    let controller = AnimatorController::new(
        "JumpCtrl",
        vec![ParameterDef::trigger("jump")],
        vec![
            AnimatorState::new(
                "Idle",
                StateMotion::Clip(simple_clip("Idle", 1.0, 0.0, 0.0)),
            ),
            AnimatorState::new(
                "Jump",
                StateMotion::Clip(simple_clip("Jump", 1.0, 1.0, 2.0)),
            ),
        ],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("jump", ConditionOp::If)],
        )],
    );

    let mut animator = Animator::new(controller);
    assert_eq!(animator.is_trigger_set("jump"), Some(false));
    assert_eq!(animator.set_trigger("jump"), Ok(()));
    assert_eq!(animator.is_trigger_set("jump"), Some(true));
    assert_eq!(animator.reset_trigger("jump"), Ok(()));
    assert_eq!(animator.is_trigger_set("jump"), Some(false));

    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 0);

    assert_eq!(animator.set_trigger("jump"), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 1);
    assert_eq!(animator.is_trigger_set("jump"), Some(false));
}

#[test]
fn has_exit_time_waits_for_normalized_time_threshold_and_exit_target_returns_to_entry() {
    let controller = AnimatorController::new(
        "AttackChain",
        Vec::new(),
        vec![
            AnimatorState::new(
                "Idle",
                StateMotion::Clip(simple_clip("Idle", 1.0, 0.0, 0.0)),
            ),
            AnimatorState::new(
                "Attack",
                StateMotion::Clip(simple_clip("Attack", 2.0, 0.0, 10.0)),
            ),
        ],
        0,
        vec![
            StateTransition::with_exit_time(0, TransitionTarget::State(1), Vec::new(), 0.5, 0.0),
            StateTransition::with_exit_time(1, TransitionTarget::Exit, Vec::new(), 0.8, 0.0),
        ],
    );

    let mut animator = Animator::new(controller);
    animator.step(0.3);
    assert_eq!(animator.current_state_info().state_name, "Idle");

    animator.step(0.25);
    assert_eq!(animator.current_state_info().state_name, "Attack");

    animator.step(1.0);
    assert_eq!(animator.current_state_info().state_name, "Attack");

    animator.step(0.7);
    assert_eq!(animator.current_state_info().state_name, "Idle");
}

#[test]
fn crossfade_transition_duration_blends_source_and_destination_until_completion() {
    let controller = AnimatorController::new(
        "Crossfade",
        vec![ParameterDef::bool("go", false)],
        vec![
            AnimatorState::new("A", StateMotion::Clip(simple_clip("A", 10.0, 0.0, 0.0))),
            AnimatorState::new("B", StateMotion::Clip(simple_clip("B", 10.0, 10.0, 10.0))),
        ],
        0,
        vec![StateTransition::new(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("go", ConditionOp::If)],
            false,
            0.0,
            1.0,
        )],
    );

    let mut animator = Animator::new(controller);
    assert_eq!(animator.set_bool("go", true), Ok(()));

    let mid = animator.step(0.4);
    assert!(mid.state_info.is_in_transition);
    assert_eq!(mid.state_info.state_index, 0);
    assert_eq!(mid.state_info.next_state_index, Some(1));
    assert!((mid.state_info.weight - 0.6).abs() < 1e-5);
    assert!((mid.state_info.next_state_weight - 0.4).abs() < 1e-5);
    let AnimatedValue::Float(val) = mid.bindings[0].value else {
        assert_eq!(mid.bindings[0].value, AnimatedValue::Float(0.0));
        return;
    };
    assert!((val - 4.0).abs() < 1e-4);

    let done = animator.step(0.7);
    assert!(!done.state_info.is_in_transition);
    assert_eq!(done.state_info.state_index, 1);
    assert_eq!(done.state_info.next_state_index, None);
    assert_eq!(done.state_info.weight, 1.0);
    assert_eq!(done.bindings[0].value, AnimatedValue::Float(10.0));
}

#[test]
fn blend_tree_1d_interpolates_adjacent_clips_and_normalizes_quaternions() {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let clip_idle = AnimationClip::new(
        "Idle",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 0.0)]),
            ),
            CurveBinding::new(
                CurveBindingTarget::new("", "Transform", "localRotation"),
                PropertyCurve::Quaternion([
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 1.0)]),
                ]),
            ),
        ],
        Vec::new(),
    );
    let clip_walk = AnimationClip::new(
        "Walk",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 4.0), Keyframe::linear(1.0, 4.0)]),
            ),
            CurveBinding::new(
                CurveBindingTarget::new("", "Transform", "localRotation"),
                PropertyCurve::Quaternion([
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, s)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, s)]),
                ]),
            ),
        ],
        Vec::new(),
    );
    let clip_run = AnimationClip::new(
        "Run",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 10.0),
                    Keyframe::linear(1.0, 10.0),
                ]),
            ),
            CurveBinding::new(
                CurveBindingTarget::new("", "Transform", "localRotation"),
                PropertyCurve::Quaternion([
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, s)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
                    FloatCurve::new(vec![Keyframe::linear(0.0, s)]),
                ]),
            ),
        ],
        Vec::new(),
    );

    let controller = AnimatorController::new(
        "BlendCtrl",
        vec![ParameterDef::float("speed", 0.5)],
        vec![AnimatorState::new(
            "Move",
            StateMotion::BlendTree1D {
                parameter: "speed".to_string(),
                thresholds: vec![(0.0, clip_idle), (1.0, clip_walk), (2.0, clip_run)],
            },
        )],
        0,
        Vec::new(),
    );

    let mut animator = Animator::new(controller);
    let step = animator.step(0.1);
    assert_eq!(step.bindings.len(), 2);
    assert_eq!(step.bindings[0].value, AnimatedValue::Float(2.0));
    let AnimatedValue::Quaternion(q) = step.bindings[1].value else {
        assert_eq!(step.bindings[1].value, AnimatedValue::Float(0.0));
        return;
    };
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    assert!((len - 1.0).abs() < 1e-5);

    assert_eq!(animator.set_float("speed", 1.5), Ok(()));
    let step2 = animator.step(0.1);
    assert_eq!(step2.bindings[0].value, AnimatedValue::Float(7.0));
}

#[test]
fn controller_validation_reports_all_five_diagnostic_categories() {
    let invalid_clip = AnimationClip::new(
        "BadClip",
        1.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 1.0)]),
        )],
        Vec::new(),
    );

    let controller = AnimatorController::new(
        "Broken",
        vec![ParameterDef::float("speed", 0.0)],
        vec![
            AnimatorState::new("Entry", StateMotion::Clip(invalid_clip)),
            AnimatorState::new("EmptyMotion", StateMotion::None),
            AnimatorState::new(
                "BadBlend",
                StateMotion::BlendTree1D {
                    parameter: "missing_blend_param".to_string(),
                    thresholds: Vec::new(),
                },
            ),
            AnimatorState::new(
                "Orphan",
                StateMotion::Clip(simple_clip("OrphanClip", 1.0, 0.0, 1.0)),
            ),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new(
                    "missing_cond_param",
                    ConditionOp::If,
                )],
            ),
            StateTransition::instant(0, TransitionTarget::State(1), Vec::new()),
            StateTransition::instant(0, TransitionTarget::State(2), Vec::new()),
        ],
    );

    let diagnostics = controller.validate();
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::MissingMotion { state_name } if state_name == "EmptyMotion"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::InvalidParameterReference { parameter, .. }
            if parameter == "missing_cond_param"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::InvalidParameterReference { parameter, .. }
            if parameter == "missing_blend_param"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::UnreachableState { state_name } if state_name == "Orphan"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::InvalidBinding { property, .. } if property == "localPosition.x"
    )));
    assert!(diagnostics.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::ConflictingTransition { source_state, .. } if source_state == "Entry"
    )));
}

#[test]
fn invalid_parameter_mutation_returns_typed_diagnostic() {
    let controller = AnimatorController::new(
        "Simple",
        vec![ParameterDef::float("speed", 1.0)],
        vec![AnimatorState::new(
            "Idle",
            StateMotion::Clip(simple_clip("Idle", 1.0, 0.0, 1.0)),
        )],
        0,
        Vec::new(),
    );
    let mut animator = Animator::new(controller);
    let err = animator.set_float("nonexistent", 2.0);
    assert!(matches!(
        err,
        Err(AnimationDiagnostic::InvalidParameterReference { .. })
    ));
    let wrong_type = animator.set_bool("speed", true);
    assert!(matches!(
        wrong_type,
        Err(AnimationDiagnostic::InvalidParameterReference { .. })
    ));
}

#[test]
fn animator_step_collects_clip_events_from_active_state() {
    let clip = AnimationClip::new(
        "Action",
        1.0,
        60.0,
        WrapMode::Once,
        Vec::new(),
        vec![AnimationEvent::with_string(0.25, "OnHit", "sword")],
    );
    let controller = AnimatorController::new(
        "EvtCtrl",
        Vec::new(),
        vec![AnimatorState::new("Action", StateMotion::Clip(clip))],
        0,
        Vec::new(),
    );
    let mut animator = Animator::new(controller);
    let step1 = animator.step(0.2);
    assert!(step1.fired_events.is_empty());
    let step2 = animator.step(0.1);
    assert_eq!(step2.fired_events.len(), 1);
    assert_eq!(step2.fired_events[0].function_name, "OnHit");
    assert_eq!(step2.fired_events[0].string_parameter, "sword");
}

#[test]
fn empty_controller_steps_gracefully_with_default_state_info() {
    let controller = AnimatorController::new("Empty", Vec::new(), Vec::new(), 0, Vec::new());
    let mut animator = Animator::new(controller);
    let res = animator.step(0.5);
    assert_eq!(res.state_info.state_index, 0);
    assert_eq!(res.state_info.state_name, "");
    assert!(res.bindings.is_empty());
    assert!(res.fired_events.is_empty());
}

#[test]
fn state_marked_is_exit_immediately_cycles_back_to_entry_state_on_transition() {
    let controller = AnimatorController::new(
        "ExitStateTest",
        vec![ParameterDef::trigger("finish")],
        vec![
            AnimatorState::new(
                "Main",
                StateMotion::Clip(simple_clip("Main", 1.0, 2.0, 2.0)),
            ),
            AnimatorState {
                name: "ExitNode".to_string(),
                motion: StateMotion::None,
                speed: 1.0,
                is_exit: true,
            },
        ],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("finish", ConditionOp::If)],
        )],
    );
    let mut animator = Animator::new(controller);
    assert_eq!(animator.set_trigger("finish"), Ok(()));
    let step = animator.step(0.1);
    assert_eq!(step.state_info.state_index, 0);
    assert_eq!(step.state_info.state_name, "Main");
}

#[test]
fn blend_tree_1d_clamps_below_min_and_above_max_thresholds() {
    let controller = AnimatorController::new(
        "ClampBlend",
        vec![ParameterDef::float("blend", -10.0)],
        vec![AnimatorState::new(
            "Tree",
            StateMotion::BlendTree1D {
                parameter: "blend".to_string(),
                thresholds: vec![
                    (0.0, simple_clip("Low", 1.0, 1.0, 1.0)),
                    (1.0, simple_clip("High", 2.0, 5.0, 5.0)),
                ],
            },
        )],
        0,
        Vec::new(),
    );
    let mut animator = Animator::new(controller);
    let low_step = animator.step(0.1);
    assert_eq!(low_step.bindings[0].value, AnimatedValue::Float(1.0));
    assert_eq!(low_step.state_info.length, 1.0);

    assert_eq!(animator.set_float("blend", 10.0), Ok(()));
    let high_step = animator.step(0.1);
    assert_eq!(high_step.bindings[0].value, AnimatedValue::Float(5.0));
    assert_eq!(high_step.state_info.length, 2.0);
}

#[test]
fn crossfade_transition_merges_disjoint_and_overlapping_bindings() {
    let clip_a = AnimationClip::new(
        "A",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::transform("", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::transform("", "localPosition.y"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 3.0)]),
            ),
        ],
        Vec::new(),
    );
    let clip_b = AnimationClip::new(
        "B",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::transform("", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 10.0)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::transform("", "localPosition.z"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 7.0)]),
            ),
        ],
        Vec::new(),
    );
    let controller = AnimatorController::new(
        "DisjointFade",
        vec![ParameterDef::bool("fade", true)],
        vec![
            AnimatorState::new("A", StateMotion::Clip(clip_a)),
            AnimatorState::new("B", StateMotion::Clip(clip_b)),
        ],
        0,
        vec![StateTransition::new(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("fade", ConditionOp::If)],
            false,
            0.0,
            1.0,
        )],
    );
    let mut animator = Animator::new(controller);
    let step = animator.step(0.5);
    assert_eq!(step.bindings.len(), 3);
    assert_eq!(step.bindings[0].value, AnimatedValue::Float(5.0));
    assert_eq!(step.bindings[1].value, AnimatedValue::Float(3.0));
    assert_eq!(step.bindings[2].value, AnimatedValue::Float(7.0));
}

#[test]
fn validation_catches_incompatible_condition_operators_and_non_float_blend_parameter() {
    let controller = AnimatorController::new(
        "TypeMismatch",
        vec![
            ParameterDef::bool("flag", false),
            ParameterDef::float("flt", 0.0),
        ],
        vec![AnimatorState::new(
            "Tree",
            StateMotion::BlendTree1D {
                parameter: "flag".to_string(),
                thresholds: vec![(0.0, simple_clip("C", 1.0, 0.0, 1.0))],
            },
        )],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::Exit,
                vec![TransitionCondition::new("flag", ConditionOp::Greater(0.5))],
            ),
            StateTransition::instant(
                0,
                TransitionTarget::Exit,
                vec![TransitionCondition::new("flt", ConditionOp::Equals(1))],
            ),
        ],
    );
    let diags = controller.validate();
    assert_eq!(
        diags
            .iter()
            .filter(|d| matches!(d, AnimationDiagnostic::InvalidParameterReference { .. }))
            .count(),
        3
    );
    for d in &diags {
        assert!(!d.to_string().is_empty());
    }
}

#[test]
fn validation_catches_invalid_transform_property_and_out_of_bounds_transition_states() {
    let bad_transform_clip = AnimationClip::new(
        "BadTransform",
        1.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::float(
            CurveBindingTarget::transform("", "eulerAngles.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 0.0)]),
        )],
        Vec::new(),
    );
    let controller = AnimatorController::new(
        "BoundsCheck",
        Vec::new(),
        vec![AnimatorState::new(
            "Only",
            StateMotion::Clip(bad_transform_clip),
        )],
        0,
        vec![
            StateTransition::instant(0, TransitionTarget::State(5), Vec::new()),
            StateTransition::instant(9, TransitionTarget::State(0), Vec::new()),
        ],
    );
    let diags = controller.validate();
    assert!(diags.iter().any(|d| matches!(
        d,
        AnimationDiagnostic::InvalidBinding { property, .. } if property == "eulerAngles.x"
    )));
    assert_eq!(
        diags
            .iter()
            .filter(|d| matches!(d, AnimationDiagnostic::ConflictingTransition { .. }))
            .count(),
        2
    );
}

#[test]
fn int_parameter_greater_and_less_conditions_evaluate_against_float_thresholds() {
    let controller = AnimatorController::new(
        "IntCompare",
        vec![ParameterDef::int("combo", 0)],
        vec![
            AnimatorState::new("S0", StateMotion::Clip(simple_clip("C0", 1.0, 0.0, 0.0))),
            AnimatorState::new("S1", StateMotion::Clip(simple_clip("C1", 1.0, 1.0, 1.0))),
        ],
        0,
        vec![
            StateTransition::instant(
                0,
                TransitionTarget::State(1),
                vec![TransitionCondition::new("combo", ConditionOp::Greater(2.5))],
            ),
            StateTransition::instant(
                1,
                TransitionTarget::State(0),
                vec![TransitionCondition::new("combo", ConditionOp::Less(1.5))],
            ),
        ],
    );
    let mut animator = Animator::new(controller);
    assert_eq!(animator.set_int("combo", 3), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 1);

    assert_eq!(animator.set_int("combo", 1), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 0);
}

#[test]
fn trigger_if_not_condition_allows_transition_when_trigger_is_unset() {
    let controller = AnimatorController::new(
        "TriggerIfNot",
        vec![ParameterDef::trigger("block")],
        vec![
            AnimatorState::new("S0", StateMotion::Clip(simple_clip("C0", 1.0, 0.0, 0.0))),
            AnimatorState::new("S1", StateMotion::Clip(simple_clip("C1", 1.0, 1.0, 1.0))),
        ],
        0,
        vec![StateTransition::instant(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("block", ConditionOp::IfNot)],
        )],
    );
    let mut animator = Animator::new(controller);
    assert_eq!(animator.set_trigger("block"), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 0);

    assert_eq!(animator.reset_trigger("block"), Ok(()));
    animator.step(0.1);
    assert_eq!(animator.current_state_info().state_index, 1);
}

#[test]
fn negative_or_nan_step_delta_time_does_not_corrupt_state_time() {
    let controller = AnimatorController::new(
        "SafeStep",
        Vec::new(),
        vec![AnimatorState::new(
            "Idle",
            StateMotion::Clip(simple_clip("Idle", 2.0, 0.0, 4.0)),
        )],
        0,
        Vec::new(),
    );
    let mut animator = Animator::new(controller);
    animator.step(0.5);
    let t_before = animator.current_state_info().normalized_time;
    animator.step(-1.0);
    animator.step(f32::NAN);
    assert_eq!(animator.current_state_info().normalized_time, t_before);
}

#[test]
fn animator_speed_zero_pauses_state_and_crossfade_progression() {
    let controller = AnimatorController::new(
        "PauseCtrl",
        vec![ParameterDef::bool("go", true)],
        vec![
            AnimatorState::new("S0", StateMotion::Clip(simple_clip("C0", 1.0, 0.0, 10.0))),
            AnimatorState::new("S1", StateMotion::Clip(simple_clip("C1", 1.0, 10.0, 20.0))),
        ],
        0,
        vec![StateTransition::new(
            0,
            TransitionTarget::State(1),
            vec![TransitionCondition::new("go", ConditionOp::If)],
            false,
            0.0,
            1.0,
        )],
    );
    let mut animator = Animator::new(controller);
    animator.step(0.25);
    assert!(animator.current_state_info().is_in_transition);
    let w_before = animator.current_state_info().next_state_weight;

    animator.set_speed(0.0);
    animator.step(1.0);
    assert!(animator.current_state_info().is_in_transition);
    assert_eq!(animator.current_state_info().next_state_weight, w_before);
}

#[test]
fn parameter_getters_return_none_on_unknown_or_mismatched_type() {
    let controller = AnimatorController::new(
        "Getters",
        vec![
            ParameterDef::float("f", 1.0),
            ParameterDef::int("i", 2),
            ParameterDef::bool("b", true),
            ParameterDef::trigger("t"),
        ],
        Vec::new(),
        0,
        Vec::new(),
    );
    let mut animator = Animator::new(controller);
    assert_eq!(animator.get_float("i"), None);
    assert_eq!(animator.get_int("f"), None);
    assert_eq!(animator.get_bool("t"), None);
    assert_eq!(animator.is_trigger_set("b"), None);
    assert_eq!(animator.get_float("missing"), None);
    assert!(animator.set_int("f", 1).is_err());
    assert!(animator.set_trigger("b").is_err());
    assert!(animator.reset_trigger("b").is_err());
    assert!(animator.set_trigger("missing").is_err());
    assert!(animator.reset_trigger("missing").is_err());
    assert_eq!(animator.controller().name, "Getters");
    assert!(animator.diagnostics().is_empty());
}
