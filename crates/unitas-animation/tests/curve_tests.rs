use unitas_animation::{
    AnimatedValue, AnimationCurve, FloatCurve, InterpolationMode, Keyframe, PropertyCurve,
};

#[test]
fn empty_float_curve_evaluates_to_zero() {
    let curve = FloatCurve::new(Vec::new());
    assert_eq!(curve.evaluate(0.0), 0.0);
    assert_eq!(curve.evaluate(5.0), 0.0);
}

#[test]
fn single_keyframe_curve_clamps_everywhere() {
    let curve = FloatCurve::new(vec![Keyframe::linear(1.0, 4.5)]);
    assert_eq!(curve.evaluate(-10.0), 4.5);
    assert_eq!(curve.evaluate(1.0), 4.5);
    assert_eq!(curve.evaluate(10.0), 4.5);
}

#[test]
fn linear_curve_interpolates_and_clamps_bounds() {
    let curve = FloatCurve::new(vec![Keyframe::linear(1.0, 2.0), Keyframe::linear(3.0, 6.0)]);
    assert_eq!(curve.evaluate(0.0), 2.0);
    assert_eq!(curve.evaluate(1.0), 2.0);
    assert!((curve.evaluate(2.0) - 4.0).abs() < 1e-6);
    assert!((curve.evaluate(2.5) - 5.0).abs() < 1e-6);
    assert_eq!(curve.evaluate(3.0), 6.0);
    assert_eq!(curve.evaluate(4.0), 6.0);
}

#[test]
fn stepped_curve_holds_left_keyframe_until_next_keyframe() {
    let curve = FloatCurve::new(vec![
        Keyframe::stepped(0.0, 10.0),
        Keyframe::stepped(1.0, 20.0),
        Keyframe::stepped(2.0, 30.0),
    ]);
    assert_eq!(curve.evaluate(-0.5), 10.0);
    assert_eq!(curve.evaluate(0.0), 10.0);
    assert_eq!(curve.evaluate(0.999), 10.0);
    assert_eq!(curve.evaluate(1.0), 20.0);
    assert_eq!(curve.evaluate(1.5), 20.0);
    assert_eq!(curve.evaluate(2.0), 30.0);
    assert_eq!(curve.evaluate(5.0), 30.0);
}

#[test]
fn cubic_hermite_with_zero_tangents_follows_smoothstep_curve() {
    let curve = AnimationCurve::new(vec![
        Keyframe::new(0.0, 0.0, 0.0, 0.0),
        Keyframe::new(2.0, 10.0, 0.0, 0.0),
    ]);
    assert!((curve.evaluate(0.0) - 0.0).abs() < 1e-6);
    assert!((curve.evaluate(0.5) - 1.5625).abs() < 1e-5);
    assert!((curve.evaluate(1.0) - 5.0).abs() < 1e-5);
    assert!((curve.evaluate(1.5) - 8.4375).abs() < 1e-5);
    assert!((curve.evaluate(2.0) - 10.0).abs() < 1e-6);
}

#[test]
fn cubic_hermite_with_nonzero_tangents_respects_dt_scaling() {
    let curve = AnimationCurve::new(vec![
        Keyframe::new(0.0, 0.0, 0.0, 4.0),
        Keyframe::new(2.0, 0.0, -4.0, 0.0),
    ]);
    assert!((curve.evaluate(1.0) - 2.0).abs() < 1e-5);
}

#[test]
fn clamped_auto_tangents_prevent_overshoot_at_extrema_and_smooth_monotone() {
    let curve = AnimationCurve::clamped_auto(vec![
        Keyframe::new(0.0, 0.0, 0.0, 0.0),
        Keyframe::new(1.0, 2.0, 0.0, 0.0),
        Keyframe::new(2.0, 4.0, 0.0, 0.0),
        Keyframe::new(3.0, 1.0, 0.0, 0.0),
    ]);
    let keys = curve.keyframes();
    assert_eq!(keys[2].in_tangent, 0.0);
    assert_eq!(keys[2].out_tangent, 0.0);
    assert!((keys[1].in_tangent - 2.0).abs() < 1e-5);
    assert!((keys[1].out_tangent - 2.0).abs() < 1e-5);
    assert!(curve.evaluate(1.5) <= 4.0 + 1e-5);
}

#[test]
fn unsorted_keyframes_are_sorted_by_time_on_construction() {
    let curve = FloatCurve::new(vec![
        Keyframe::linear(2.0, 20.0),
        Keyframe::linear(0.0, 0.0),
        Keyframe::linear(1.0, 10.0),
    ]);
    assert!((curve.evaluate(0.5) - 5.0).abs() < 1e-5);
    assert!((curve.evaluate(1.5) - 15.0).abs() < 1e-5);
}

#[test]
fn mixed_interpolation_modes_across_segments_use_segment_left_mode() {
    let curve = FloatCurve::new(vec![
        Keyframe::stepped(0.0, 1.0),
        Keyframe::linear(1.0, 5.0),
        Keyframe::new(2.0, 9.0, 0.0, 0.0),
    ]);
    assert_eq!(curve.keyframes()[0].mode, InterpolationMode::Stepped);
    assert_eq!(curve.evaluate(0.5), 1.0);
    assert!((curve.evaluate(1.5) - 7.0).abs() < 1e-5);
}

#[test]
fn keyframe_new_defaults_to_hermite_mode_while_with_mode_uses_the_requested_mode() {
    let default = Keyframe::new(0.25, 1.5, 2.0, 3.0);
    assert_eq!(default.mode, InterpolationMode::Hermite);
    assert_eq!(default.time, 0.25);
    assert_eq!(default.value, 1.5);
    assert_eq!(default.in_tangent, 2.0);
    assert_eq!(default.out_tangent, 3.0);

    let stepped = Keyframe::with_mode(0.25, 1.5, 2.0, 3.0, InterpolationMode::Stepped);
    assert_eq!(stepped.mode, InterpolationMode::Stepped);
    assert_ne!(stepped.mode, default.mode);

    let linear = Keyframe::with_mode(0.25, 1.5, 2.0, 3.0, InterpolationMode::Linear);
    assert_eq!(linear.mode, InterpolationMode::Linear);

    let hermite = Keyframe::with_mode(0.25, 1.5, 2.0, 3.0, InterpolationMode::Hermite);
    assert_eq!(hermite.mode, InterpolationMode::Hermite);
}

#[test]
fn property_curve_vector3_evaluates_each_channel_independently() {
    let prop = PropertyCurve::Vector3([
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0), Keyframe::linear(1.0, 3.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 2.0), Keyframe::linear(1.0, 6.0)]),
        FloatCurve::new(vec![Keyframe::stepped(0.0, 5.0), Keyframe::stepped(1.0, 9.0)]),
    ]);
    assert_eq!(
        prop.evaluate(0.5),
        AnimatedValue::Vector3([2.0, 4.0, 5.0])
    );
}

#[test]
fn property_curve_quaternion_normalizes_interpolated_components_and_handles_antipodal() {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let prop = PropertyCurve::Quaternion([
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, -s)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0), Keyframe::linear(1.0, -s)]),
    ]);
    let value = prop.evaluate(0.5);
    let AnimatedValue::Quaternion(q) = value else {
        assert_eq!(value, AnimatedValue::Float(0.0));
        return;
    };
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    assert!((len - 1.0).abs() < 1e-5);
    assert!(q[3] > 0.9);
}

#[test]
fn property_curve_color_and_animated_value_blending_preserve_unit_quaternion() {
    let color_curve = PropertyCurve::Color([
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.2), Keyframe::linear(1.0, 0.8)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.4), Keyframe::linear(1.0, 0.4)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 0.6), Keyframe::linear(1.0, 1.0)]),
        FloatCurve::new(vec![Keyframe::linear(0.0, 1.0), Keyframe::linear(1.0, 0.0)]),
    ]);
    let AnimatedValue::Color(c) = color_curve.evaluate(0.5) else {
        assert_eq!(color_curve.evaluate(0.5), AnimatedValue::Float(0.0));
        return;
    };
    assert!((c[0] - 0.5).abs() < 1e-6);
    assert!((c[1] - 0.4).abs() < 1e-6);
    assert!((c[2] - 0.8).abs() < 1e-6);
    assert!((c[3] - 0.5).abs() < 1e-6);

    let s = std::f32::consts::FRAC_1_SQRT_2;
    let q0 = AnimatedValue::Quaternion([0.0, 0.0, 0.0, 1.0]);
    let q1 = AnimatedValue::Quaternion([0.0, -s, 0.0, -s]);
    let blended = q0.blend(&q1, 0.5);
    let AnimatedValue::Quaternion(qb) = blended else {
        assert_eq!(blended, AnimatedValue::Float(0.0));
        return;
    };
    let len = (qb[0] * qb[0] + qb[1] * qb[1] + qb[2] * qb[2] + qb[3] * qb[3]).sqrt();
    assert!((len - 1.0).abs() < 1e-5);
    assert!(qb[3] > 0.9);
}

#[test]
fn clamped_auto_handles_empty_and_single_keyframe_curves() {
    let empty = AnimationCurve::clamped_auto(Vec::new());
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.max_time(), 0.0);

    let single = AnimationCurve::clamped_auto(vec![Keyframe::new(1.5, 9.0, 0.0, 0.0)]);
    assert_eq!(single.len(), 1);
    assert_eq!(single.max_time(), 1.5);
    assert_eq!(single.keyframes()[0].in_tangent, 0.0);
    assert_eq!(single.keyframes()[0].out_tangent, 0.0);
    assert_eq!(single.evaluate(0.0), 9.0);
}

#[test]
fn duplicate_keyframe_timestamps_evaluate_deterministically_without_nan() {
    let curve = FloatCurve::new(vec![
        Keyframe::linear(1.0, 2.0),
        Keyframe::linear(1.0, 8.0),
        Keyframe::linear(2.0, 10.0),
    ]);
    assert!(curve.evaluate(1.0).is_finite());
    assert!((curve.evaluate(1.5) - 9.0).abs() < 1e-5);
}

#[test]
fn non_finite_evaluation_time_or_tangents_fall_back_safely() {
    let curve = FloatCurve::new(vec![
        Keyframe::new(0.0, 2.0, f32::NAN, f32::INFINITY),
        Keyframe::new(1.0, 6.0, f32::NEG_INFINITY, f32::NAN),
    ]);
    assert_eq!(curve.evaluate(f32::NAN), 2.0);
    let mid = curve.evaluate(0.5);
    assert!(mid.is_finite());
    assert!((mid - 4.0).abs() < 1e-5);
}

#[test]
fn animated_value_blend_clamps_out_of_range_and_non_finite_weights() {
    let a = AnimatedValue::Float(10.0);
    let b = AnimatedValue::Float(20.0);
    assert_eq!(a.blend(&b, -5.0), AnimatedValue::Float(10.0));
    assert_eq!(a.blend(&b, 5.0), AnimatedValue::Float(20.0));
    assert_eq!(a.blend(&b, f32::NAN), AnimatedValue::Float(10.0));

    let v0 = AnimatedValue::Vector3([1.0, 2.0, 3.0]);
    let v1 = AnimatedValue::Vector3([5.0, 6.0, 7.0]);
    assert_eq!(v0.blend(&v1, 0.25), AnimatedValue::Vector3([2.0, 3.0, 4.0]));
}

#[test]
fn animated_value_blend_with_mismatched_variants_picks_by_weight_threshold() {
    let f = AnimatedValue::Float(3.0);
    let v = AnimatedValue::Vector3([1.0, 2.0, 3.0]);
    assert_eq!(f.blend(&v, 0.2), f);
    assert_eq!(f.blend(&v, 0.8), v);
}

#[test]
fn property_curve_float_and_max_time_across_all_variants() {
    let fc = PropertyCurve::Float(FloatCurve::new(vec![
        Keyframe::linear(0.0, 1.0),
        Keyframe::linear(3.5, 8.0),
    ]));
    assert_eq!(fc.max_time(), 3.5);
    assert_eq!(fc.evaluate(1.75), AnimatedValue::Float(4.5));

    let vc = PropertyCurve::Vector3([
        FloatCurve::new(vec![Keyframe::linear(1.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(4.0, 0.0)]),
        FloatCurve::new(vec![Keyframe::linear(2.0, 0.0)]),
    ]);
    assert_eq!(vc.max_time(), 4.0);
}

