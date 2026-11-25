use unitas_animation::{
    AnimatedValue, AnimationClip, AnimationEvent, CurveBinding, CurveBindingTarget, FloatCurve,
    Keyframe, PropertyCurve, WrapMode,
};

fn make_linear_float_curve(t0: f32, v0: f32, t1: f32, v1: f32) -> FloatCurve {
    FloatCurve::new(vec![Keyframe::linear(t0, v0), Keyframe::linear(t1, v1)])
}

#[test]
fn clip_duration_inferred_from_max_keyframe_time_when_zero() {
    let clip = AnimationClip::new(
        "Walk",
        0.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                make_linear_float_curve(0.0, 0.0, 2.5, 10.0),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.y"),
                make_linear_float_curve(0.0, 0.0, 1.5, 5.0),
            ),
        ],
        Vec::new(),
    );
    assert!((clip.effective_duration() - 2.5).abs() < 1e-6);
}

#[test]
fn zero_duration_clip_evaluates_at_zero_and_fires_event_once_on_entry() {
    let clip = AnimationClip::new(
        "Pose",
        0.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            FloatCurve::new(vec![Keyframe::linear(0.0, 7.0)]),
        )],
        vec![AnimationEvent::new(0.0, "OnPose")],
    );
    assert_eq!(clip.effective_duration(), 0.0);

    let first = clip.sample_at(0.0, 0.016);
    assert_eq!(first.wrapped_time, 0.0);
    assert!(first.normalized_time.is_finite());
    assert_eq!(first.bindings.len(), 1);
    assert_eq!(first.bindings[0].value, AnimatedValue::Float(7.0));
    assert_eq!(first.fired_events.len(), 1);
    assert_eq!(first.fired_events[0].function_name, "OnPose");

    let second = clip.sample(0.016, 0.032);
    assert!(second.fired_events.is_empty());
}

#[test]
fn wrap_mode_once_and_clamp_forever_clamp_at_duration() {
    let once_clip = AnimationClip::new(
        "Jump",
        2.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.y"),
            make_linear_float_curve(0.0, 0.0, 2.0, 10.0),
        )],
        Vec::new(),
    );
    let clamp_clip = AnimationClip {
        wrap_mode: WrapMode::ClampForever,
        ..once_clip.clone()
    };

    let s_once = once_clip.sample(1.8, 2.5);
    assert!((s_once.wrapped_time - 2.0).abs() < 1e-6);
    assert!((s_once.normalized_time - 1.25).abs() < 1e-6);
    assert_eq!(s_once.bindings[0].value, AnimatedValue::Float(10.0));

    let s_clamp = clamp_clip.sample(1.8, 3.0);
    assert!((s_clamp.wrapped_time - 2.0).abs() < 1e-6);
    assert_eq!(s_clamp.bindings[0].value, AnimatedValue::Float(10.0));
}

#[test]
fn wrap_mode_loop_wraps_time_modulo_duration() {
    let clip = AnimationClip::new(
        "Run",
        2.0,
        60.0,
        WrapMode::Loop,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            make_linear_float_curve(0.0, 0.0, 2.0, 20.0),
        )],
        Vec::new(),
    );
    let sample = clip.sample(1.9, 2.5);
    assert!((sample.wrapped_time - 0.5).abs() < 1e-5);
    assert!((sample.normalized_time - 1.25).abs() < 1e-5);
    let AnimatedValue::Float(val) = sample.bindings[0].value else {
        assert_eq!(sample.bindings[0].value, AnimatedValue::Float(0.0));
        return;
    };
    assert!((val - 5.0).abs() < 1e-4);
}

#[test]
fn wrap_mode_ping_pong_alternates_forward_and_backward_cycles() {
    let clip = AnimationClip::new(
        "Pendulum",
        2.0,
        60.0,
        WrapMode::PingPong,
        vec![CurveBinding::float(
            CurveBindingTarget::new("", "Transform", "localPosition.x"),
            make_linear_float_curve(0.0, 0.0, 2.0, 10.0),
        )],
        Vec::new(),
    );
    let forward = clip.sample(0.0, 0.5);
    assert!((forward.wrapped_time - 0.5).abs() < 1e-5);
    let backward = clip.sample(2.0, 2.5);
    assert!((backward.wrapped_time - 1.5).abs() < 1e-5);
    let AnimatedValue::Float(val) = backward.bindings[0].value else {
        assert_eq!(backward.bindings[0].value, AnimatedValue::Float(0.0));
        return;
    };
    assert!((val - 7.5).abs() < 1e-4);

    let second_forward = clip.sample(4.0, 4.5);
    assert!((second_forward.wrapped_time - 0.5).abs() < 1e-5);
}

#[test]
fn same_frame_events_preserve_declared_order_even_at_identical_timestamps() {
    let clip = AnimationClip::new(
        "Combo",
        1.0,
        60.0,
        WrapMode::Once,
        Vec::new(),
        vec![
            AnimationEvent::with_int(0.5, "StepB", 2),
            AnimationEvent::with_int(0.2, "Early", 0),
            AnimationEvent::with_int(0.5, "StepA", 1),
            AnimationEvent::with_int(0.5, "StepC", 3),
        ],
    );

    let sample = clip.sample(0.0, 0.6);
    let names: Vec<&str> = sample
        .fired_events
        .iter()
        .map(|event| event.function_name.as_str())
        .collect();
    assert_eq!(names, vec!["Early", "StepB", "StepA", "StepC"]);
}

#[test]
fn loop_boundary_fires_end_and_start_events_in_chronological_order_without_double_firing() {
    let clip = AnimationClip::new(
        "LoopingStep",
        1.0,
        60.0,
        WrapMode::Loop,
        Vec::new(),
        vec![
            AnimationEvent::new(0.0, "LoopStart"),
            AnimationEvent::new(0.4, "Mid"),
            AnimationEvent::new(0.9, "NearEnd"),
            AnimationEvent::new(1.0, "LoopEnd"),
        ],
    );

    let first = clip.sample(0.0, 0.5);
    let first_names: Vec<&str> = first
        .fired_events
        .iter()
        .map(|e| e.function_name.as_str())
        .collect();
    assert_eq!(first_names, vec!["LoopStart", "Mid"]);

    let across_loop = clip.sample(0.5, 1.45);
    let across_names: Vec<&str> = across_loop
        .fired_events
        .iter()
        .map(|e| e.function_name.as_str())
        .collect();
    assert_eq!(across_names, vec!["NearEnd", "LoopEnd", "LoopStart", "Mid"]);
}

#[test]
fn multi_loop_step_fires_events_for_every_crossed_cycle() {
    let clip = AnimationClip::new(
        "FastSpin",
        1.0,
        60.0,
        WrapMode::Loop,
        Vec::new(),
        vec![AnimationEvent::new(0.5, "Halfway")],
    );

    let sample = clip.sample(0.1, 2.6);
    assert_eq!(sample.fired_events.len(), 3);
}

#[test]
fn loop_event_firing_is_capped_at_64_cycles_per_sample() {
    let clip = AnimationClip::new(
        "Capped",
        1.0,
        60.0,
        WrapMode::Loop,
        Vec::new(),
        vec![AnimationEvent::new(0.5, "Halfway")],
    );

    let sample = clip.sample(0.0, 200.0);
    assert_eq!(sample.fired_events.len(), 65);
}

#[test]
fn ping_pong_event_firing_is_capped_at_64_cycles_per_sample() {
    let clip = AnimationClip::new(
        "PingPongCapped",
        1.0,
        60.0,
        WrapMode::PingPong,
        Vec::new(),
        vec![AnimationEvent::new(0.5, "Halfway")],
    );

    let sample = clip.sample(0.0, 200.0);
    assert_eq!(sample.fired_events.len(), 65);
    assert!(sample
        .fired_events
        .iter()
        .all(|event| event.function_name == "Halfway"));
}

#[test]
fn whole_property_and_per_channel_bindings_coexist_and_carry_parsed_targets() {
    let clip = AnimationClip::new(
        "MixedBindings",
        1.0,
        60.0,
        WrapMode::Once,
        vec![
            CurveBinding::new(
                CurveBindingTarget::new("", "Transform", "localPosition"),
                PropertyCurve::Vector3([
                    make_linear_float_curve(0.0, 1.0, 1.0, 3.0),
                    make_linear_float_curve(0.0, 2.0, 1.0, 4.0),
                    make_linear_float_curve(0.0, 3.0, 1.0, 5.0),
                ]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("Arm/Hand", "Transform", "localScale.x"),
                make_linear_float_curve(0.0, 1.0, 1.0, 2.0),
            ),
            CurveBinding::new(
                CurveBindingTarget::new("Body", "MeshRenderer", "material.color"),
                PropertyCurve::Color([
                    make_linear_float_curve(0.0, 1.0, 1.0, 0.0),
                    make_linear_float_curve(0.0, 0.5, 1.0, 0.5),
                    make_linear_float_curve(0.0, 0.0, 1.0, 1.0),
                    make_linear_float_curve(0.0, 1.0, 1.0, 1.0),
                ]),
            ),
        ],
        Vec::new(),
    );

    let sample = clip.sample(0.0, 0.5);
    assert_eq!(sample.bindings.len(), 3);
    assert!(sample.bindings[0].target.is_transform());
    assert_eq!(
        sample.bindings[0].value,
        AnimatedValue::Vector3([2.0, 3.0, 4.0])
    );
    assert!(sample.bindings[1].target.is_transform());
    assert_eq!(sample.bindings[1].target.object_path, "Arm/Hand");
    assert_eq!(sample.bindings[1].value, AnimatedValue::Float(1.5));
    assert!(!sample.bindings[2].target.is_transform());
    assert_eq!(
        sample.bindings[2].value,
        AnimatedValue::Color([0.5, 0.5, 0.5, 1.0])
    );
}

#[test]
fn transform_property_and_channel_parsing_covers_all_transform_paths() {
    use unitas_animation::{TransformChannel, TransformProperty};
    let cases = [
        (
            "localPosition",
            Some((TransformProperty::LocalPosition, TransformChannel::All)),
        ),
        (
            "m_LocalPosition.x",
            Some((TransformProperty::LocalPosition, TransformChannel::X)),
        ),
        (
            "localPosition.y",
            Some((TransformProperty::LocalPosition, TransformChannel::Y)),
        ),
        (
            "position.z",
            Some((TransformProperty::LocalPosition, TransformChannel::Z)),
        ),
        (
            "localRotation",
            Some((TransformProperty::LocalRotation, TransformChannel::All)),
        ),
        (
            "localRotation.x",
            Some((TransformProperty::LocalRotation, TransformChannel::X)),
        ),
        (
            "localRotation.y",
            Some((TransformProperty::LocalRotation, TransformChannel::Y)),
        ),
        (
            "localRotation.z",
            Some((TransformProperty::LocalRotation, TransformChannel::Z)),
        ),
        (
            "m_LocalRotation.w",
            Some((TransformProperty::LocalRotation, TransformChannel::W)),
        ),
        (
            "localScale",
            Some((TransformProperty::LocalScale, TransformChannel::All)),
        ),
        (
            "localScale.x",
            Some((TransformProperty::LocalScale, TransformChannel::X)),
        ),
        (
            "localScale.y",
            Some((TransformProperty::LocalScale, TransformChannel::Y)),
        ),
        (
            "scale.z",
            Some((TransformProperty::LocalScale, TransformChannel::Z)),
        ),
        ("unknownProp", None),
    ];
    for (prop, expected) in cases {
        let target = CurveBindingTarget::transform("", prop);
        assert_eq!(target.parse_transform_property(), expected);
    }
    let non_transform = CurveBindingTarget::new("", "Camera", "fieldOfView");
    assert_eq!(non_transform.parse_transform_property(), None);
}

#[test]
fn once_clip_does_not_refire_events_after_reaching_duration() {
    let clip = AnimationClip::new(
        "OneShot",
        1.0,
        60.0,
        WrapMode::Once,
        Vec::new(),
        vec![
            AnimationEvent::with_float(0.0, "Start", 1.0),
            AnimationEvent::with_float(1.0, "End", 2.0),
        ],
    );
    let s1 = clip.sample(0.0, 1.0);
    assert_eq!(s1.fired_events.len(), 2);
    assert_eq!(s1.fired_events[0].float_parameter, 1.0);
    assert_eq!(s1.fired_events[1].float_parameter, 2.0);

    let s2 = clip.sample(1.0, 2.0);
    assert!(s2.fired_events.is_empty());
}

#[test]
fn ping_pong_clip_fires_events_on_forward_and_backward_sweeps() {
    let clip = AnimationClip::new(
        "Bounce",
        1.0,
        60.0,
        WrapMode::PingPong,
        Vec::new(),
        vec![
            AnimationEvent::new(0.3, "Pass03"),
            AnimationEvent::new(0.8, "Pass08"),
        ],
    );
    let forward = clip.sample(0.0, 1.0);
    let f_names: Vec<&str> = forward
        .fired_events
        .iter()
        .map(|e| e.function_name.as_str())
        .collect();
    assert_eq!(f_names, vec!["Pass03", "Pass08"]);

    let backward = clip.sample(1.0, 2.0);
    assert_eq!(backward.fired_events.len(), 2);
}

#[test]
fn clip_duration_inferred_from_event_time_when_no_curves_exist() {
    let clip = AnimationClip::new(
        "EventOnly",
        0.0,
        60.0,
        WrapMode::Once,
        Vec::new(),
        vec![AnimationEvent::new(1.75, "Marker")],
    );
    assert!((clip.effective_duration() - 1.75).abs() < 1e-6);
}

#[test]
fn invalid_sample_rate_and_negative_duration_sanitize_to_safe_defaults() {
    let clip = AnimationClip::new(
        "Sanitized",
        -5.0,
        -10.0,
        WrapMode::Once,
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(clip.duration, 0.0);
    assert_eq!(clip.sample_rate, 60.0);
}
