use crate::curve::{AnimatedValue, FloatCurve, PropertyCurve};

const EVENT_EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WrapMode {
    #[default]
    Once,
    Loop,
    PingPong,
    ClampForever,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformProperty {
    LocalPosition,
    LocalRotation,
    LocalScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformChannel {
    All,
    X,
    Y,
    Z,
    W,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurveBindingTarget {
    pub object_path: String,
    pub component: String,
    pub property: String,
}

impl CurveBindingTarget {
    pub fn new(
        object_path: impl Into<String>,
        component: impl Into<String>,
        property: impl Into<String>,
    ) -> Self {
        Self {
            object_path: object_path.into(),
            component: component.into(),
            property: property.into(),
        }
    }

    pub fn transform(object_path: impl Into<String>, property: impl Into<String>) -> Self {
        Self::new(object_path, "Transform", property)
    }

    pub fn is_valid(&self) -> bool {
        !self.component.trim().is_empty() && !self.property.trim().is_empty()
    }

    pub fn is_transform(&self) -> bool {
        self.parse_transform_property().is_some()
    }

    pub fn parse_transform_property(&self) -> Option<(TransformProperty, TransformChannel)> {
        if self.component != "Transform" {
            return None;
        }
        match self.property.as_str() {
            "localPosition" | "m_LocalPosition" | "position" => {
                Some((TransformProperty::LocalPosition, TransformChannel::All))
            }
            "localPosition.x" | "m_LocalPosition.x" | "position.x" => {
                Some((TransformProperty::LocalPosition, TransformChannel::X))
            }
            "localPosition.y" | "m_LocalPosition.y" | "position.y" => {
                Some((TransformProperty::LocalPosition, TransformChannel::Y))
            }
            "localPosition.z" | "m_LocalPosition.z" | "position.z" => {
                Some((TransformProperty::LocalPosition, TransformChannel::Z))
            }
            "localRotation" | "m_LocalRotation" | "rotation" => {
                Some((TransformProperty::LocalRotation, TransformChannel::All))
            }
            "localRotation.x" | "m_LocalRotation.x" | "rotation.x" => {
                Some((TransformProperty::LocalRotation, TransformChannel::X))
            }
            "localRotation.y" | "m_LocalRotation.y" | "rotation.y" => {
                Some((TransformProperty::LocalRotation, TransformChannel::Y))
            }
            "localRotation.z" | "m_LocalRotation.z" | "rotation.z" => {
                Some((TransformProperty::LocalRotation, TransformChannel::Z))
            }
            "localRotation.w" | "m_LocalRotation.w" | "rotation.w" => {
                Some((TransformProperty::LocalRotation, TransformChannel::W))
            }
            "localScale" | "m_LocalScale" | "scale" => {
                Some((TransformProperty::LocalScale, TransformChannel::All))
            }
            "localScale.x" | "m_LocalScale.x" | "scale.x" => {
                Some((TransformProperty::LocalScale, TransformChannel::X))
            }
            "localScale.y" | "m_LocalScale.y" | "scale.y" => {
                Some((TransformProperty::LocalScale, TransformChannel::Y))
            }
            "localScale.z" | "m_LocalScale.z" | "scale.z" => {
                Some((TransformProperty::LocalScale, TransformChannel::Z))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurveBinding {
    pub target: CurveBindingTarget,
    pub curve: PropertyCurve,
}

impl CurveBinding {
    pub fn new(target: CurveBindingTarget, curve: PropertyCurve) -> Self {
        Self { target, curve }
    }

    pub fn float(target: CurveBindingTarget, curve: FloatCurve) -> Self {
        Self {
            target,
            curve: PropertyCurve::Float(curve),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEvent {
    pub time: f32,
    pub function_name: String,
    pub float_parameter: f32,
    pub int_parameter: i32,
    pub string_parameter: String,
}

impl AnimationEvent {
    pub fn new(time: f32, function_name: impl Into<String>) -> Self {
        Self {
            time,
            function_name: function_name.into(),
            float_parameter: 0.0,
            int_parameter: 0,
            string_parameter: String::new(),
        }
    }

    pub fn with_float(time: f32, function_name: impl Into<String>, float_parameter: f32) -> Self {
        Self {
            float_parameter,
            ..Self::new(time, function_name)
        }
    }

    pub fn with_int(time: f32, function_name: impl Into<String>, int_parameter: i32) -> Self {
        Self {
            int_parameter,
            ..Self::new(time, function_name)
        }
    }

    pub fn with_string(
        time: f32,
        function_name: impl Into<String>,
        string_parameter: impl Into<String>,
    ) -> Self {
        Self {
            string_parameter: string_parameter.into(),
            ..Self::new(time, function_name)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampledBinding {
    pub target: CurveBindingTarget,
    pub value: AnimatedValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipSample {
    pub bindings: Vec<SampledBinding>,
    pub fired_events: Vec<AnimationEvent>,
    pub normalized_time: f32,
    pub wrapped_time: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub sample_rate: f32,
    pub wrap_mode: WrapMode,
    pub bindings: Vec<CurveBinding>,
    pub events: Vec<AnimationEvent>,
}

impl AnimationClip {
    pub fn new(
        name: impl Into<String>,
        duration: f32,
        sample_rate: f32,
        wrap_mode: WrapMode,
        bindings: Vec<CurveBinding>,
        events: Vec<AnimationEvent>,
    ) -> Self {
        Self {
            name: name.into(),
            duration: if duration.is_finite() {
                duration.max(0.0)
            } else {
                0.0
            },
            sample_rate: if sample_rate.is_finite() && sample_rate > 0.0 {
                sample_rate
            } else {
                60.0
            },
            wrap_mode,
            bindings,
            events,
        }
    }

    pub fn effective_duration(&self) -> f32 {
        if self.duration.is_finite() && self.duration > 0.0 {
            return self.duration;
        }
        let max_curve = self
            .bindings
            .iter()
            .map(|b| b.curve.max_time())
            .fold(0.0_f32, f32::max);
        let max_event = self
            .events
            .iter()
            .filter(|e| e.time.is_finite())
            .map(|e| e.time.max(0.0))
            .fold(0.0_f32, f32::max);
        max_curve.max(max_event)
    }

    pub fn wrap_time(&self, current_time: f32) -> (f32, f32) {
        let d = self.effective_duration();
        let t = if current_time.is_finite() {
            current_time.max(0.0)
        } else {
            0.0
        };
        if d <= EVENT_EPSILON {
            let norm = if t > 0.0 { 1.0 } else { 0.0 };
            return (0.0, norm);
        }
        let normalized_time = t / d;
        let wrapped_time = match self.wrap_mode {
            WrapMode::Once | WrapMode::ClampForever => t.clamp(0.0, d),
            WrapMode::Loop => t.rem_euclid(d),
            WrapMode::PingPong => {
                let cycle = (t / d).floor() as i64;
                let rem = t.rem_euclid(d);
                if cycle.rem_euclid(2) == 0 {
                    rem
                } else {
                    d - rem
                }
            }
        };
        (wrapped_time, normalized_time)
    }

    pub fn sample_at(&self, elapsed: f32, dt: f32) -> ClipSample {
        let prev = if elapsed.is_finite() {
            elapsed.max(0.0)
        } else {
            0.0
        };
        let delta = if dt.is_finite() { dt } else { 0.0 };
        self.sample(prev, prev + delta)
    }

    pub fn sample(&self, prev_time: f32, current_time: f32) -> ClipSample {
        let prev = if prev_time.is_finite() {
            prev_time
        } else {
            0.0
        };
        let curr = if current_time.is_finite() {
            current_time
        } else {
            prev
        };
        let (wrapped_time, normalized_time) = self.wrap_time(curr);
        let bindings = self
            .bindings
            .iter()
            .map(|b| SampledBinding {
                target: b.target.clone(),
                value: b.curve.evaluate(wrapped_time),
            })
            .collect();
        let fired_events = self.collect_events(prev, curr);
        ClipSample {
            bindings,
            fired_events,
            normalized_time,
            wrapped_time,
        }
    }

    fn collect_events(&self, prev_time: f32, current_time: f32) -> Vec<AnimationEvent> {
        if self.events.is_empty() || (current_time - prev_time).abs() <= EVENT_EPSILON {
            return Vec::new();
        }
        let d = self.effective_duration();
        if d <= EVENT_EPSILON {
            if prev_time <= 0.0 && current_time > 0.0 {
                return self
                    .events
                    .iter()
                    .filter(|e| e.time.is_finite() && e.time.abs() <= EVENT_EPSILON)
                    .cloned()
                    .collect();
            }
            return Vec::new();
        }

        if current_time < prev_time {
            return self.collect_events_reverse(prev_time, current_time, d);
        }

        match self.wrap_mode {
            WrapMode::Once | WrapMode::ClampForever => {
                if prev_time >= d {
                    return Vec::new();
                }
                let lo = prev_time.max(0.0);
                let hi = current_time.min(d);
                let include_lo = prev_time <= 0.0;
                self.events_in_window(lo, hi, include_lo, true)
            }
            WrapMode::Loop => {
                let start_cycle = (prev_time.max(0.0) / d).floor() as i64;
                let end_cycle = (current_time.max(0.0) / d).floor() as i64;
                let max_cycles = (end_cycle - start_cycle).min(64);
                let mut fired = Vec::new();
                for offset in 0..=max_cycles {
                    let cycle = start_cycle + offset;
                    let cycle_start = cycle as f32 * d;
                    let cycle_end = (cycle + 1) as f32 * d;
                    let seg_lo = (prev_time.max(cycle_start) - cycle_start).clamp(0.0, d);
                    let seg_hi = (current_time.min(cycle_end) - cycle_start).clamp(0.0, d);
                    if seg_hi > seg_lo + EVENT_EPSILON
                        || (prev_time <= 0.0 && cycle == 0 && seg_hi >= seg_lo)
                    {
                        let include_lo = (prev_time <= 0.0 && cycle == 0) || offset > 0;
                        fired.extend(self.events_in_window(seg_lo, seg_hi, include_lo, true));
                    }
                }
                fired
            }
            WrapMode::PingPong => {
                let start_cycle = (prev_time.max(0.0) / d).floor() as i64;
                let end_cycle = (current_time.max(0.0) / d).floor() as i64;
                let max_cycles = (end_cycle - start_cycle).min(64);
                let mut fired = Vec::new();
                for offset in 0..=max_cycles {
                    let cycle = start_cycle + offset;
                    let cycle_start = cycle as f32 * d;
                    let cycle_end = (cycle + 1) as f32 * d;
                    let seg_lo = (prev_time.max(cycle_start) - cycle_start).clamp(0.0, d);
                    let seg_hi = (current_time.min(cycle_end) - cycle_start).clamp(0.0, d);
                    if seg_hi > seg_lo + EVENT_EPSILON {
                        let include_lo = (prev_time <= 0.0 && cycle == 0) || offset > 0;
                        if cycle.rem_euclid(2) == 0 {
                            fired.extend(self.events_in_window(seg_lo, seg_hi, include_lo, true));
                        } else {
                            let rev_lo = d - seg_hi;
                            let rev_hi = d - seg_lo;
                            fired.extend(
                                self.events_in_window_descending(rev_lo, rev_hi, true, include_lo),
                            );
                        }
                    }
                }
                fired
            }
        }
    }

    fn collect_events_reverse(
        &self,
        prev_time: f32,
        current_time: f32,
        d: f32,
    ) -> Vec<AnimationEvent> {
        match self.wrap_mode {
            WrapMode::Once | WrapMode::ClampForever => {
                if prev_time <= 0.0 {
                    return Vec::new();
                }
                let hi = prev_time.min(d);
                let lo = current_time.max(0.0);
                let include_hi = prev_time >= d;
                self.events_in_window_descending(lo, hi, true, include_hi)
            }
            WrapMode::Loop => {
                let start_cycle = (prev_time.max(0.0) / d).floor() as i64;
                let end_cycle = (current_time.max(0.0) / d).floor() as i64;
                let max_cycles = (start_cycle - end_cycle).clamp(0, 64);
                let mut fired = Vec::new();
                for offset in 0..=max_cycles {
                    let cycle = start_cycle - offset;
                    let cycle_start = cycle as f32 * d;
                    let cycle_end = (cycle + 1) as f32 * d;
                    let seg_lo = (current_time.max(cycle_start) - cycle_start).clamp(0.0, d);
                    let seg_hi = (prev_time.min(cycle_end) - cycle_start).clamp(0.0, d);
                    if seg_hi > seg_lo + EVENT_EPSILON {
                        let include_hi = offset > 0;
                        fired.extend(
                            self.events_in_window_descending(seg_lo, seg_hi, true, include_hi),
                        );
                    }
                }
                fired
            }
            WrapMode::PingPong => {
                let start_cycle = (prev_time.max(0.0) / d).floor() as i64;
                let end_cycle = (current_time.max(0.0) / d).floor() as i64;
                let max_cycles = (start_cycle - end_cycle).clamp(0, 64);
                let mut fired = Vec::new();
                for offset in 0..=max_cycles {
                    let cycle = start_cycle - offset;
                    let cycle_start = cycle as f32 * d;
                    let cycle_end = (cycle + 1) as f32 * d;
                    let seg_lo = (current_time.max(cycle_start) - cycle_start).clamp(0.0, d);
                    let seg_hi = (prev_time.min(cycle_end) - cycle_start).clamp(0.0, d);
                    if seg_hi > seg_lo + EVENT_EPSILON {
                        let include_hi = offset > 0;
                        if cycle.rem_euclid(2) == 0 {
                            fired.extend(
                                self.events_in_window_descending(seg_lo, seg_hi, true, include_hi),
                            );
                        } else {
                            let fwd_lo = d - seg_hi;
                            let fwd_hi = d - seg_lo;
                            fired.extend(self.events_in_window(fwd_lo, fwd_hi, include_hi, true));
                        }
                    }
                }
                fired
            }
        }
    }

    fn events_in_window_descending(
        &self,
        lo: f32,
        hi: f32,
        include_lo: bool,
        include_hi: bool,
    ) -> Vec<AnimationEvent> {
        let mut matched: Vec<(usize, &AnimationEvent)> = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if !e.time.is_finite() {
                    return false;
                }
                let ge_lo = if include_lo {
                    e.time >= lo - EVENT_EPSILON
                } else {
                    e.time > lo + EVENT_EPSILON
                };
                let le_hi = if include_hi {
                    e.time <= hi + EVENT_EPSILON
                } else {
                    e.time < hi - EVENT_EPSILON
                };
                ge_lo && le_hi
            })
            .collect();
        matched.sort_by(|(idx_a, ev_a), (idx_b, ev_b)| {
            ev_b.time
                .total_cmp(&ev_a.time)
                .then_with(|| idx_a.cmp(idx_b))
        });
        matched.into_iter().map(|(_, ev)| ev.clone()).collect()
    }

    fn events_in_window(
        &self,
        lo: f32,
        hi: f32,
        include_lo: bool,
        include_hi: bool,
    ) -> Vec<AnimationEvent> {
        let mut matched: Vec<(usize, &AnimationEvent)> = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if !e.time.is_finite() {
                    return false;
                }
                let ge_lo = if include_lo {
                    e.time >= lo - EVENT_EPSILON
                } else {
                    e.time > lo + EVENT_EPSILON
                };
                let le_hi = if include_hi {
                    e.time <= hi + EVENT_EPSILON
                } else {
                    e.time < hi - EVENT_EPSILON
                };
                ge_lo && le_hi
            })
            .collect();
        matched.sort_by(|(idx_a, ev_a), (idx_b, ev_b)| {
            ev_a.time
                .total_cmp(&ev_b.time)
                .then_with(|| idx_a.cmp(idx_b))
        });
        matched.into_iter().map(|(_, ev)| ev.clone()).collect()
    }
}
