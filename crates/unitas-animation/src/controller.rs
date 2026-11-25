use std::collections::BTreeSet;

use crate::clip::{AnimationClip, AnimationEvent, ClipSample, SampledBinding};
use crate::error::AnimationError;
use crate::wire::{decode_controller_binary, encode_controller_binary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterType {
    Float,
    Int,
    Bool,
    Trigger,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParameterValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Trigger(bool),
}

impl ParameterValue {
    pub const fn parameter_type(&self) -> ParameterType {
        match self {
            Self::Float(_) => ParameterType::Float,
            Self::Int(_) => ParameterType::Int,
            Self::Bool(_) => ParameterType::Bool,
            Self::Trigger(_) => ParameterType::Trigger,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterDef {
    pub name: String,
    pub default_value: ParameterValue,
}

impl ParameterDef {
    pub fn new(name: impl Into<String>, default_value: ParameterValue) -> Self {
        Self {
            name: name.into(),
            default_value,
        }
    }

    pub fn float(name: impl Into<String>, default: f32) -> Self {
        Self::new(name, ParameterValue::Float(default))
    }

    pub fn int(name: impl Into<String>, default: i32) -> Self {
        Self::new(name, ParameterValue::Int(default))
    }

    pub fn bool(name: impl Into<String>, default: bool) -> Self {
        Self::new(name, ParameterValue::Bool(default))
    }

    pub fn trigger(name: impl Into<String>) -> Self {
        Self::new(name, ParameterValue::Trigger(false))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConditionOp {
    If,
    IfNot,
    Greater(f32),
    Less(f32),
    Equals(i32),
    NotEqual(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionCondition {
    pub parameter: String,
    pub op: ConditionOp,
}

impl TransitionCondition {
    pub fn new(parameter: impl Into<String>, op: ConditionOp) -> Self {
        Self {
            parameter: parameter.into(),
            op,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendTree2DMode {
    #[default]
    SimpleDirectional,
    FreeformDirectional,
    FreeformCartesian,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterruptionSource {
    #[default]
    None,
    Source,
    Destination,
    SourceThenDestination,
    DestinationThenSource,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum StateMotion {
    #[default]
    None,
    Clip(AnimationClip),
    BlendTree1D {
        parameter: String,
        thresholds: Vec<(f32, AnimationClip)>,
    },
    BlendTree2D {
        mode: BlendTree2DMode,
        parameter_x: String,
        parameter_y: String,
        children: Vec<([f32; 2], AnimationClip)>,
    },
    BlendTreeDirect {
        children: Vec<(String, AnimationClip)>,
    },
}

impl StateMotion {
    pub fn duration(&self, parameter_value: f32) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Clip(clip) => clip.effective_duration(),
            Self::BlendTree1D { thresholds, .. } => {
                if thresholds.is_empty() {
                    return 0.0;
                }
                let mut sorted: Vec<&(f32, AnimationClip)> = thresholds.iter().collect();
                sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
                if sorted.len() == 1 || parameter_value <= sorted[0].0 {
                    return sorted[0].1.effective_duration();
                }
                let last = sorted[sorted.len() - 1];
                if parameter_value >= last.0 {
                    return last.1.effective_duration();
                }
                for pair in sorted.windows(2) {
                    let (t0, c0) = pair[0];
                    let (t1, c1) = pair[1];
                    if parameter_value >= *t0 && parameter_value <= *t1 {
                        let span = t1 - t0;
                        let alpha = if span.abs() <= 1e-6 {
                            0.0
                        } else {
                            ((parameter_value - t0) / span).clamp(0.0, 1.0)
                        };
                        let d0 = c0.effective_duration();
                        let d1 = c1.effective_duration();
                        return d0 + (d1 - d0) * alpha;
                    }
                }
                sorted[0].1.effective_duration()
            }
            Self::BlendTree2D { children, .. } => children
                .first()
                .map_or(0.0, |(_, clip)| clip.effective_duration()),
            Self::BlendTreeDirect { children } => children
                .first()
                .map_or(0.0, |(_, clip)| clip.effective_duration()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatorState {
    pub name: String,
    pub motion: StateMotion,
    pub speed: f32,
    pub is_exit: bool,
    pub write_defaults: bool,
}

impl AnimatorState {
    pub fn new(name: impl Into<String>, motion: StateMotion) -> Self {
        Self {
            name: name.into(),
            motion,
            speed: 1.0,
            is_exit: false,
            write_defaults: true,
        }
    }

    pub fn with_speed(name: impl Into<String>, motion: StateMotion, speed: f32) -> Self {
        Self {
            name: name.into(),
            motion,
            speed,
            is_exit: false,
            write_defaults: true,
        }
    }

    pub fn with_write_defaults(
        name: impl Into<String>,
        motion: StateMotion,
        write_defaults: bool,
    ) -> Self {
        Self {
            name: name.into(),
            motion,
            speed: 1.0,
            is_exit: false,
            write_defaults,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionTarget {
    State(usize),
    Exit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateTransition {
    pub source_state: usize,
    pub target: TransitionTarget,
    pub conditions: Vec<TransitionCondition>,
    pub has_exit_time: bool,
    pub exit_time: f32,
    pub duration: f32,
    pub interruption_source: InterruptionSource,
}

impl StateTransition {
    pub fn new(
        source_state: usize,
        target: TransitionTarget,
        conditions: Vec<TransitionCondition>,
        has_exit_time: bool,
        exit_time: f32,
        duration: f32,
    ) -> Self {
        Self {
            source_state,
            target,
            conditions,
            has_exit_time,
            exit_time,
            duration: if duration.is_finite() {
                duration.max(0.0)
            } else {
                0.0
            },
            interruption_source: InterruptionSource::None,
        }
    }

    pub fn instant(
        source_state: usize,
        target: TransitionTarget,
        conditions: Vec<TransitionCondition>,
    ) -> Self {
        Self::new(source_state, target, conditions, false, 0.0, 0.0)
    }

    pub fn with_exit_time(
        source_state: usize,
        target: TransitionTarget,
        conditions: Vec<TransitionCondition>,
        exit_time: f32,
        duration: f32,
    ) -> Self {
        Self::new(source_state, target, conditions, true, exit_time, duration)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationDiagnostic {
    MissingMotion {
        state_name: String,
    },
    InvalidParameterReference {
        parameter: String,
        context: String,
    },
    UnreachableState {
        state_name: String,
    },
    InvalidBinding {
        object_path: String,
        component: String,
        property: String,
        reason: String,
    },
    ConflictingTransition {
        source_state: String,
        reason: String,
    },
}

impl std::fmt::Display for AnimationDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMotion { state_name } => {
                write!(formatter, "state '{state_name}' has no motion assigned")
            }
            Self::InvalidParameterReference { parameter, context } => {
                write!(
                    formatter,
                    "invalid parameter reference '{parameter}' in {context}"
                )
            }
            Self::UnreachableState { state_name } => {
                write!(
                    formatter,
                    "state '{state_name}' is unreachable from entry state"
                )
            }
            Self::InvalidBinding {
                object_path,
                component,
                property,
                reason,
            } => write!(
                formatter,
                "invalid binding '{object_path}:{component}.{property}': {reason}"
            ),
            Self::ConflictingTransition {
                source_state,
                reason,
            } => write!(
                formatter,
                "conflicting transition from state '{source_state}': {reason}"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatorController {
    pub name: String,
    pub parameters: Vec<ParameterDef>,
    pub states: Vec<AnimatorState>,
    pub entry_state: usize,
    pub transitions: Vec<StateTransition>,
}

impl AnimatorController {
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<ParameterDef>,
        states: Vec<AnimatorState>,
        entry_state: usize,
        transitions: Vec<StateTransition>,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            states,
            entry_state,
            transitions,
        }
    }

    pub fn encode_binary(&self) -> Vec<u8> {
        encode_controller_binary(self)
    }

    pub fn decode_binary(bytes: &[u8]) -> Result<Self, AnimationError> {
        decode_controller_binary(bytes)
    }

    pub fn validate(&self) -> Vec<AnimationDiagnostic> {
        let mut diagnostics = Vec::new();
        for state in &self.states {
            match &state.motion {
                StateMotion::None => {
                    diagnostics.push(AnimationDiagnostic::MissingMotion {
                        state_name: state.name.clone(),
                    });
                }
                StateMotion::Clip(clip) => {
                    validate_clip_bindings(clip, &mut diagnostics);
                }
                StateMotion::BlendTree1D {
                    parameter,
                    thresholds,
                } => {
                    match self.parameters.iter().find(|p| p.name == *parameter) {
                        None => diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                            parameter: parameter.clone(),
                            context: format!("blend tree in state '{}'", state.name),
                        }),
                        Some(param)
                            if param.default_value.parameter_type() != ParameterType::Float =>
                        {
                            diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                                parameter: parameter.clone(),
                                context: format!(
                                    "blend tree in state '{}' requires Float parameter",
                                    state.name
                                ),
                            });
                        }
                        Some(_) => {}
                    }
                    if thresholds.is_empty() {
                        diagnostics.push(AnimationDiagnostic::MissingMotion {
                            state_name: state.name.clone(),
                        });
                    }
                    for (_, clip) in thresholds {
                        validate_clip_bindings(clip, &mut diagnostics);
                    }
                }
                StateMotion::BlendTree2D {
                    parameter_x,
                    parameter_y,
                    children,
                    ..
                } => {
                    for parameter in [parameter_x, parameter_y] {
                        match self.parameters.iter().find(|p| p.name == *parameter) {
                            None => diagnostics.push(
                                AnimationDiagnostic::InvalidParameterReference {
                                    parameter: parameter.clone(),
                                    context: format!("2D blend tree in state '{}'", state.name),
                                },
                            ),
                            Some(param)
                                if param.default_value.parameter_type() != ParameterType::Float =>
                            {
                                diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                                    parameter: parameter.clone(),
                                    context: format!(
                                        "2D blend tree in state '{}' requires Float parameter",
                                        state.name
                                    ),
                                });
                            }
                            Some(_) => {}
                        }
                    }
                    if children.is_empty() {
                        diagnostics.push(AnimationDiagnostic::MissingMotion {
                            state_name: state.name.clone(),
                        });
                    }
                    for (_, clip) in children {
                        validate_clip_bindings(clip, &mut diagnostics);
                    }
                }
                StateMotion::BlendTreeDirect { children } => {
                    if children.is_empty() {
                        diagnostics.push(AnimationDiagnostic::MissingMotion {
                            state_name: state.name.clone(),
                        });
                    }
                    for (parameter, clip) in children {
                        match self.parameters.iter().find(|p| p.name == *parameter) {
                            None => diagnostics.push(
                                AnimationDiagnostic::InvalidParameterReference {
                                    parameter: parameter.clone(),
                                    context: format!("Direct blend tree in state '{}'", state.name),
                                },
                            ),
                            Some(param)
                                if param.default_value.parameter_type() != ParameterType::Float =>
                            {
                                diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                                    parameter: parameter.clone(),
                                    context: format!(
                                        "Direct blend tree in state '{}' requires Float parameter",
                                        state.name
                                    ),
                                });
                            }
                            Some(_) => {}
                        }
                        validate_clip_bindings(clip, &mut diagnostics);
                    }
                }
            }
        }

        for transition in &self.transitions {
            let source_name = self.states.get(transition.source_state).map_or_else(
                || format!("#{}", transition.source_state),
                |s| s.name.clone(),
            );
            if transition.source_state >= self.states.len() {
                diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                    source_state: source_name.clone(),
                    reason: "source state index out of bounds".to_string(),
                });
            }
            if let TransitionTarget::State(dst) = transition.target {
                if dst >= self.states.len() {
                    diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                        source_state: source_name.clone(),
                        reason: format!("target state index {dst} out of bounds"),
                    });
                }
            }
            if !transition.has_exit_time && transition.conditions.is_empty() {
                let duplicate = self
                    .transitions
                    .iter()
                    .filter(|other| {
                        other.source_state == transition.source_state
                            && !other.has_exit_time
                            && other.conditions.is_empty()
                    })
                    .count();
                if duplicate > 1 {
                    let conflict = AnimationDiagnostic::ConflictingTransition {
                        source_state: source_name.clone(),
                        reason: "multiple unconditional transitions without exit time".to_string(),
                    };
                    if !diagnostics.contains(&conflict) {
                        diagnostics.push(conflict);
                    }
                }
            }
            for cond in &transition.conditions {
                match self.parameters.iter().find(|p| p.name == cond.parameter) {
                    None => diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                        parameter: cond.parameter.clone(),
                        context: format!("transition from state '{source_name}'"),
                    }),
                    Some(param) => {
                        let ptype = param.default_value.parameter_type();
                        let valid = match cond.op {
                            ConditionOp::If | ConditionOp::IfNot => {
                                matches!(ptype, ParameterType::Bool | ParameterType::Trigger)
                            }
                            ConditionOp::Greater(_) | ConditionOp::Less(_) => {
                                matches!(ptype, ParameterType::Float | ParameterType::Int)
                            }
                            ConditionOp::Equals(_) | ConditionOp::NotEqual(_) => {
                                matches!(ptype, ParameterType::Int)
                            }
                        };
                        if !valid {
                            diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                                parameter: cond.parameter.clone(),
                                context: format!(
                                    "incompatible condition operator in transition from '{source_name}'"
                                ),
                            });
                        }
                    }
                }
            }
        }

        if !self.states.is_empty() {
            let mut reachable = BTreeSet::new();
            let mut queue = Vec::new();
            if self.entry_state < self.states.len() {
                reachable.insert(self.entry_state);
                queue.push(self.entry_state);
            }
            while let Some(current) = queue.pop() {
                for tr in &self.transitions {
                    if tr.source_state == current {
                        let next_idx = match tr.target {
                            TransitionTarget::State(idx) => idx,
                            TransitionTarget::Exit => self.entry_state,
                        };
                        if next_idx < self.states.len() && reachable.insert(next_idx) {
                            queue.push(next_idx);
                        }
                    }
                }
            }
            for (idx, state) in self.states.iter().enumerate() {
                if !reachable.contains(&idx) {
                    diagnostics.push(AnimationDiagnostic::UnreachableState {
                        state_name: state.name.clone(),
                    });
                }
            }
        }

        diagnostics
    }
}

fn sanitize_delta(delta: f32) -> f32 {
    if delta.is_finite() && delta > 0.0 {
        delta
    } else {
        0.0
    }
}

fn state_clips(state: &AnimatorState) -> Vec<&AnimationClip> {
    match &state.motion {
        StateMotion::None => Vec::new(),
        StateMotion::Clip(clip) => vec![clip],
        StateMotion::BlendTree1D { thresholds, .. } => {
            thresholds.iter().map(|(_, clip)| clip).collect()
        }
        StateMotion::BlendTree2D { children, .. } => {
            children.iter().map(|(_, clip)| clip).collect()
        }
        StateMotion::BlendTreeDirect { children } => {
            children.iter().map(|(_, clip)| clip).collect()
        }
    }
}

fn capture_defaults(controller: &AnimatorController) -> Vec<SampledBinding> {
    let mut defaults: Vec<SampledBinding> = Vec::new();
    for state in &controller.states {
        for clip in state_clips(state) {
            for binding in &clip.bindings {
                if defaults
                    .iter()
                    .any(|existing| existing.target == binding.target)
                {
                    continue;
                }
                defaults.push(SampledBinding {
                    target: binding.target.clone(),
                    value: binding.curve.evaluate(0.0),
                });
            }
        }
    }
    defaults
}

fn validate_clip_bindings(clip: &AnimationClip, diagnostics: &mut Vec<AnimationDiagnostic>) {
    for binding in &clip.bindings {
        if !binding.target.is_valid() {
            diagnostics.push(AnimationDiagnostic::InvalidBinding {
                object_path: binding.target.object_path.clone(),
                component: binding.target.component.clone(),
                property: binding.target.property.clone(),
                reason: "empty component or property name".to_string(),
            });
        } else if binding.target.component == "Transform" && !binding.target.is_transform() {
            diagnostics.push(AnimationDiagnostic::InvalidBinding {
                object_path: binding.target.object_path.clone(),
                component: binding.target.component.clone(),
                property: binding.target.property.clone(),
                reason: "unsupported Transform property".to_string(),
            });
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatorStateInfo {
    pub state_index: usize,
    pub state_name: String,
    pub normalized_time: f32,
    pub length: f32,
    pub speed: f32,
    pub weight: f32,
    pub is_in_transition: bool,
    pub next_state_index: Option<usize>,
    pub next_state_weight: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatorStepResult {
    pub state_info: AnimatorStateInfo,
    pub bindings: Vec<SampledBinding>,
    pub fired_events: Vec<AnimationEvent>,
    pub diagnostics: Vec<AnimationDiagnostic>,
    pub callbacks: Vec<StateMachineCallback>,
    pub writes_transforms: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatorUpdateMode {
    Normal,
    AnimatePhysics,
    UnscaledTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatorCullingMode {
    AlwaysAnimate,
    CullUpdateTransforms,
    CullCompletely,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatorFrame {
    pub scaled_delta: f32,
    pub unscaled_delta: f32,
    pub fixed_delta: f32,
}

impl AnimatorFrame {
    pub fn new(scaled_delta: f32, unscaled_delta: f32, fixed_delta: f32) -> Self {
        Self {
            scaled_delta,
            unscaled_delta,
            fixed_delta,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateMachineCallbackKind {
    OnStateEnter,
    OnStateUpdate,
    OnStateExit,
    OnStateMove,
    OnStateIK,
    OnStateMachineEnter,
    OnStateMachineExit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateMachineCallback {
    pub kind: StateMachineCallbackKind,
    pub state_index: usize,
    pub state_name: String,
    pub machine: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveTransition {
    target_state: usize,
    elapsed: f32,
    duration: f32,
    target_time: f32,
    interruption_source: InterruptionSource,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AnimatorRequest {
    Play(usize),
    CrossFade(usize, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animator {
    controller: AnimatorController,
    parameter_values: Vec<(String, ParameterValue)>,
    current_state: usize,
    state_time: f32,
    active_transition: Option<ActiveTransition>,
    speed: f32,
    diagnostics: Vec<AnimationDiagnostic>,
    update_mode: AnimatorUpdateMode,
    culling_mode: AnimatorCullingMode,
    visible: bool,
    state_frames: u32,
    machine_entered: bool,
    pending_request: Option<AnimatorRequest>,
    captured_defaults: Vec<SampledBinding>,
}

impl Animator {
    pub fn new(controller: AnimatorController) -> Self {
        let diagnostics = controller.validate();
        let parameter_values = controller
            .parameters
            .iter()
            .map(|p| (p.name.clone(), p.default_value))
            .collect();
        let entry = if controller.entry_state < controller.states.len() {
            controller.entry_state
        } else {
            0
        };
        let captured_defaults = capture_defaults(&controller);
        Self {
            controller,
            parameter_values,
            current_state: entry,
            state_time: 0.0,
            active_transition: None,
            speed: 1.0,
            diagnostics,
            update_mode: AnimatorUpdateMode::Normal,
            culling_mode: AnimatorCullingMode::AlwaysAnimate,
            visible: true,
            state_frames: 0,
            machine_entered: false,
            pending_request: None,
            captured_defaults,
        }
    }

    pub fn controller(&self) -> &AnimatorController {
        &self.controller
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        if speed.is_finite() {
            self.speed = speed.max(0.0);
        }
    }

    pub fn diagnostics(&self) -> &[AnimationDiagnostic] {
        &self.diagnostics
    }

    pub fn set_float(&mut self, name: &str, value: f32) -> Result<(), AnimationDiagnostic> {
        let Some((_, slot)) = self.parameter_values.iter_mut().find(|(n, _)| n == name) else {
            return Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_float: unknown parameter".to_string(),
            });
        };
        if let ParameterValue::Float(v) = slot {
            *v = if value.is_finite() { value } else { 0.0 };
            Ok(())
        } else {
            Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_float: parameter is not Float".to_string(),
            })
        }
    }

    pub fn get_float(&self, name: &str) -> Option<f32> {
        self.parameter_values
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| match v {
                ParameterValue::Float(f) => Some(*f),
                _ => None,
            })
    }

    pub fn set_int(&mut self, name: &str, value: i32) -> Result<(), AnimationDiagnostic> {
        let Some((_, slot)) = self.parameter_values.iter_mut().find(|(n, _)| n == name) else {
            return Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_int: unknown parameter".to_string(),
            });
        };
        if let ParameterValue::Int(v) = slot {
            *v = value;
            Ok(())
        } else {
            Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_int: parameter is not Int".to_string(),
            })
        }
    }

    pub fn get_int(&self, name: &str) -> Option<i32> {
        self.parameter_values
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| match v {
                ParameterValue::Int(i) => Some(*i),
                _ => None,
            })
    }

    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<(), AnimationDiagnostic> {
        let Some((_, slot)) = self.parameter_values.iter_mut().find(|(n, _)| n == name) else {
            return Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_bool: unknown parameter".to_string(),
            });
        };
        if let ParameterValue::Bool(v) = slot {
            *v = value;
            Ok(())
        } else {
            Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_bool: parameter is not Bool".to_string(),
            })
        }
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.parameter_values
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| match v {
                ParameterValue::Bool(b) => Some(*b),
                _ => None,
            })
    }

    pub fn set_trigger(&mut self, name: &str) -> Result<(), AnimationDiagnostic> {
        let Some((_, slot)) = self.parameter_values.iter_mut().find(|(n, _)| n == name) else {
            return Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_trigger: unknown parameter".to_string(),
            });
        };
        if let ParameterValue::Trigger(v) = slot {
            *v = true;
            Ok(())
        } else {
            Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "set_trigger: parameter is not Trigger".to_string(),
            })
        }
    }

    pub fn reset_trigger(&mut self, name: &str) -> Result<(), AnimationDiagnostic> {
        let Some((_, slot)) = self.parameter_values.iter_mut().find(|(n, _)| n == name) else {
            return Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "reset_trigger: unknown parameter".to_string(),
            });
        };
        if let ParameterValue::Trigger(v) = slot {
            *v = false;
            Ok(())
        } else {
            Err(AnimationDiagnostic::InvalidParameterReference {
                parameter: name.to_string(),
                context: "reset_trigger: parameter is not Trigger".to_string(),
            })
        }
    }

    pub fn is_trigger_set(&self, name: &str) -> Option<bool> {
        self.parameter_values
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| match v {
                ParameterValue::Trigger(b) => Some(*b),
                _ => None,
            })
    }

    pub fn current_state_info(&self) -> AnimatorStateInfo {
        let Some(state) = self.controller.states.get(self.current_state) else {
            return AnimatorStateInfo {
                state_index: 0,
                state_name: String::new(),
                normalized_time: 0.0,
                length: 0.0,
                speed: 1.0,
                weight: 1.0,
                is_in_transition: false,
                next_state_index: None,
                next_state_weight: 0.0,
            };
        };
        let blend_param = match &state.motion {
            StateMotion::BlendTree1D { parameter, .. } => self.get_float(parameter).unwrap_or(0.0),
            _ => 0.0,
        };
        let length = state.motion.duration(blend_param);
        let normalized_time = if length > 1e-6 {
            self.state_time / length
        } else if self.state_time > 0.0 {
            1.0
        } else {
            0.0
        };
        let (is_in_transition, weight, next_state_index, next_state_weight) =
            if let Some(tr) = self.active_transition {
                let alpha = if tr.duration > 1e-6 {
                    (tr.elapsed / tr.duration).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                (true, 1.0 - alpha, Some(tr.target_state), alpha)
            } else {
                (false, 1.0, None, 0.0)
            };
        AnimatorStateInfo {
            state_index: self.current_state,
            state_name: state.name.clone(),
            normalized_time,
            length,
            speed: state.speed * self.speed,
            weight,
            is_in_transition,
            next_state_index,
            next_state_weight,
        }
    }

    pub fn step(&mut self, dt_seconds: f32) -> AnimatorStepResult {
        let dt = sanitize_delta(dt_seconds);
        self.step_frame(&AnimatorFrame::new(dt, dt, dt))
    }

    pub fn step_frame(&mut self, frame: &AnimatorFrame) -> AnimatorStepResult {
        if self.culling_mode == AnimatorCullingMode::CullCompletely && !self.visible {
            return AnimatorStepResult {
                state_info: self.current_state_info(),
                bindings: Vec::new(),
                fired_events: Vec::new(),
                diagnostics: self.diagnostics.clone(),
                callbacks: Vec::new(),
                writes_transforms: false,
            };
        }
        let dt = self.update_delta(frame);
        let mut result = self.advance(dt);
        if self.culling_mode == AnimatorCullingMode::CullUpdateTransforms && !self.visible {
            result.writes_transforms = false;
        }
        result
    }

    fn update_delta(&self, frame: &AnimatorFrame) -> f32 {
        match self.update_mode {
            AnimatorUpdateMode::Normal => sanitize_delta(frame.scaled_delta),
            AnimatorUpdateMode::UnscaledTime => sanitize_delta(frame.unscaled_delta),
            AnimatorUpdateMode::AnimatePhysics => {
                let fixed = sanitize_delta(frame.fixed_delta);
                if fixed <= 1e-6 {
                    return sanitize_delta(frame.scaled_delta);
                }
                let steps = (sanitize_delta(frame.scaled_delta) / fixed).floor();
                steps * fixed
            }
        }
    }

    fn advance(&mut self, dt: f32) -> AnimatorStepResult {
        if self.controller.states.is_empty() {
            return AnimatorStepResult {
                state_info: self.current_state_info(),
                bindings: Vec::new(),
                fired_events: Vec::new(),
                diagnostics: self.diagnostics.clone(),
                callbacks: Vec::new(),
                writes_transforms: true,
            };
        }

        let mut callbacks: Vec<StateMachineCallback> = Vec::new();
        if !self.machine_entered {
            self.machine_entered = true;
            callbacks.push(self.machine_callback(StateMachineCallbackKind::OnStateMachineEnter));
        }
        let mut entered: Option<usize> = None;
        let mut exited: Option<usize> = None;

        if let Some(request) = self.pending_request.take() {
            match request {
                AnimatorRequest::Play(target) => {
                    if target < self.controller.states.len() {
                        exited = Some(self.current_state);
                        self.current_state = target;
                        self.state_time = 0.0;
                        self.active_transition = None;
                        self.state_frames = 0;
                        entered = Some(target);
                        let sample = self.sample_state(self.current_state, 0.0, 0.0);
                        let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
                        return self.finish(callbacks, entered, exited, bindings, sample.fired_events);
                    }
                }
                AnimatorRequest::CrossFade(target, duration) => {
                    if target < self.controller.states.len() {
                        if duration <= 1e-6 {
                            exited = Some(self.current_state);
                            self.current_state = target;
                            self.state_time = 0.0;
                            self.active_transition = None;
                            self.state_frames = 0;
                            entered = Some(target);
                            let sample = self.sample_state(self.current_state, 0.0, 0.0);
                            let bindings =
                                self.apply_write_defaults(self.current_state, sample.bindings);
                            return self.finish(
                                callbacks,
                                entered,
                                exited,
                                bindings,
                                sample.fired_events,
                            );
                        }
                        self.active_transition = Some(ActiveTransition {
                            target_state: target,
                            elapsed: 0.0,
                            duration: sanitize_delta(duration),
                            target_time: 0.0,
                            interruption_source: InterruptionSource::None,
                        });
                        entered = Some(target);
                    }
                }
            }
        }

        let source_speed = self
            .controller
            .states
            .get(self.current_state)
            .map_or(1.0, |s| s.speed);
        let prev_time = self.state_time;
        let scaled_source_dt = dt * self.speed * source_speed.max(0.0);
        let new_time = prev_time + scaled_source_dt;
        self.state_time = new_time;

        let source_length = self.state_duration(self.current_state);
        let current_norm = if source_length > 1e-6 {
            self.state_time / source_length
        } else if self.state_time > 0.0 {
            1.0
        } else {
            0.0
        };

        if let Some(active) = self.active_transition {
            let dst_len = self.state_duration(active.target_state);
            let dst_norm = if dst_len > 1e-6 {
                active.target_time / dst_len
            } else {
                0.0
            };
            let interrupt_tr_idx = match active.interruption_source {
                InterruptionSource::None => None,
                InterruptionSource::Source => {
                    self.find_interrupting_transition(self.current_state, current_norm, active.target_state)
                }
                InterruptionSource::Destination => {
                    self.find_interrupting_transition(active.target_state, dst_norm, active.target_state)
                }
                InterruptionSource::SourceThenDestination => self
                    .find_interrupting_transition(self.current_state, current_norm, active.target_state)
                    .or_else(|| {
                        self.find_interrupting_transition(active.target_state, dst_norm, active.target_state)
                    }),
                InterruptionSource::DestinationThenSource => self
                    .find_interrupting_transition(active.target_state, dst_norm, active.target_state)
                    .or_else(|| {
                        self.find_interrupting_transition(self.current_state, current_norm, active.target_state)
                    }),
            };
            if let Some(tr_idx) = interrupt_tr_idx {
                let transition = self.controller.transitions[tr_idx].clone();
                self.consume_transition_triggers(&transition);
                let target_idx = match transition.target {
                    TransitionTarget::State(idx) if idx < self.controller.states.len() => idx,
                    TransitionTarget::State(_) | TransitionTarget::Exit => self
                        .controller
                        .entry_state
                        .min(self.controller.states.len() - 1),
                };
                if transition.duration <= 1e-6 {
                    exited = Some(self.current_state);
                    self.current_state = target_idx;
                    self.state_time = 0.0;
                    self.active_transition = None;
                    self.state_frames = 0;
                    if self
                        .controller
                        .states
                        .get(self.current_state)
                        .is_some_and(|s| s.is_exit)
                    {
                        self.current_state = self
                            .controller
                            .entry_state
                            .min(self.controller.states.len() - 1);
                    }
                    entered = Some(self.current_state);
                    let sample = self.sample_state(self.current_state, 0.0, 0.0);
                    let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
                    return self.finish(callbacks, entered, exited, bindings, sample.fired_events);
                }
                self.active_transition = Some(ActiveTransition {
                    target_state: target_idx,
                    elapsed: 0.0,
                    duration: transition.duration,
                    target_time: 0.0,
                    interruption_source: transition.interruption_source,
                });
                entered = Some(target_idx);
            }
        } else if let Some(tr_idx) = self.find_matching_transition(self.current_state, current_norm)
        {
            let transition = self.controller.transitions[tr_idx].clone();
            self.consume_transition_triggers(&transition);
            let target_idx = match transition.target {
                TransitionTarget::State(idx) if idx < self.controller.states.len() => idx,
                TransitionTarget::State(_) | TransitionTarget::Exit => {
                    self.controller.entry_state.min(self.controller.states.len() - 1)
                }
            };
            if transition.duration <= 1e-6 {
                exited = Some(self.current_state);
                self.current_state = target_idx;
                self.state_time = 0.0;
                self.state_frames = 0;
                if self
                    .controller
                    .states
                    .get(self.current_state)
                    .is_some_and(|s| s.is_exit)
                {
                    self.current_state =
                        self.controller.entry_state.min(self.controller.states.len() - 1);
                }
                entered = Some(self.current_state);
                let sample = self.sample_state(self.current_state, 0.0, 0.0);
                let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
                return self.finish(callbacks, entered, exited, bindings, sample.fired_events);
            }
            self.active_transition = Some(ActiveTransition {
                target_state: target_idx,
                elapsed: 0.0,
                duration: transition.duration,
                target_time: 0.0,
                interruption_source: transition.interruption_source,
            });
            entered = Some(target_idx);
        }

        if let Some(mut tr) = self.active_transition {
            let target_speed = self
                .controller
                .states
                .get(tr.target_state)
                .map_or(1.0, |s| s.speed);
            let prev_target_time = tr.target_time;
            let next_target_time = prev_target_time + dt * self.speed * target_speed.max(0.0);
            tr.target_time = next_target_time;
            tr.elapsed += dt * self.speed;

            if tr.elapsed >= tr.duration {
                exited = Some(self.current_state);
                self.current_state = tr.target_state;
                self.state_time = tr.target_time;
                self.active_transition = None;
                self.state_frames = 0;
                if self
                    .controller
                    .states
                    .get(self.current_state)
                    .is_some_and(|s| s.is_exit)
                {
                    self.current_state = self
                        .controller
                        .entry_state
                        .min(self.controller.states.len() - 1);
                    self.state_time = 0.0;
                }
                let sample = self.sample_state(self.current_state, prev_target_time, self.state_time);
                let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
                return self.finish(callbacks, entered, exited, bindings, sample.fired_events);
            }

            self.active_transition = Some(tr);
            let alpha = (tr.elapsed / tr.duration).clamp(0.0, 1.0);
            let src_sample = self.sample_state(self.current_state, prev_time, new_time);
            let dst_sample = self.sample_state(tr.target_state, prev_target_time, next_target_time);
            let bindings =
                blend_sampled_bindings(&src_sample.bindings, &dst_sample.bindings, alpha);
            let mut fired_events = src_sample.fired_events;
            fired_events.extend(dst_sample.fired_events);
            let bindings = self.apply_write_defaults(self.current_state, bindings);
            return self.finish(callbacks, entered, exited, bindings, fired_events);
        }

        let sample = self.sample_state(self.current_state, prev_time, new_time);
        let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
        self.finish(callbacks, entered, exited, bindings, sample.fired_events)
    }

    fn finish(
        &mut self,
        mut callbacks: Vec<StateMachineCallback>,
        entered: Option<usize>,
        exited: Option<usize>,
        bindings: Vec<SampledBinding>,
        fired_events: Vec<AnimationEvent>,
    ) -> AnimatorStepResult {
        self.state_frames += 1;
        if let Some(index) = entered {
            callbacks.push(self.state_callback(StateMachineCallbackKind::OnStateEnter, index));
        }
        if self.state_frames >= 2 && exited != Some(self.current_state) {
            callbacks.push(self.state_callback(
                StateMachineCallbackKind::OnStateUpdate,
                self.current_state,
            ));
        }
        if let Some(index) = exited {
            callbacks.push(self.state_callback(StateMachineCallbackKind::OnStateExit, index));
        }
        AnimatorStepResult {
            state_info: self.current_state_info(),
            bindings,
            fired_events,
            diagnostics: self.diagnostics.clone(),
            callbacks,
            writes_transforms: true,
        }
    }

    fn apply_write_defaults(
        &self,
        state_index: usize,
        bindings: Vec<SampledBinding>,
    ) -> Vec<SampledBinding> {
        let Some(state) = self.controller.states.get(state_index) else {
            return bindings;
        };
        if !state.write_defaults {
            return bindings;
        }
        let mut result = bindings;
        for default in &self.captured_defaults {
            if result.iter().any(|binding| binding.target == default.target) {
                continue;
            }
            result.push(default.clone());
        }
        result
    }

    fn state_callback(
        &self,
        kind: StateMachineCallbackKind,
        state_index: usize,
    ) -> StateMachineCallback {
        let state_name = self
            .controller
            .states
            .get(state_index)
            .map(|state| state.name.clone())
            .unwrap_or_default();
        StateMachineCallback {
            kind,
            state_index,
            state_name,
            machine: None,
        }
    }

    fn machine_callback(&self, kind: StateMachineCallbackKind) -> StateMachineCallback {
        StateMachineCallback {
            kind,
            state_index: self.current_state,
            state_name: String::new(),
            machine: Some(self.controller.name.clone()),
        }
    }

    pub fn update_mode(&self) -> AnimatorUpdateMode {
        self.update_mode
    }

    pub fn set_update_mode(&mut self, update_mode: AnimatorUpdateMode) {
        self.update_mode = update_mode;
    }

    pub fn culling_mode(&self) -> AnimatorCullingMode {
        self.culling_mode
    }

    pub fn set_culling_mode(&mut self, culling_mode: AnimatorCullingMode) {
        self.culling_mode = culling_mode;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn captured_defaults(&self) -> &[SampledBinding] {
        &self.captured_defaults
    }

    pub fn state_index_by_name(&self, name: &str) -> Option<usize> {
        self.controller
            .states
            .iter()
            .position(|state| state.name == name)
    }

    pub fn play(&mut self, state_index: usize) {
        self.pending_request = Some(AnimatorRequest::Play(state_index));
    }

    pub fn cross_fade(&mut self, state_index: usize, duration: f32) {
        self.pending_request = Some(AnimatorRequest::CrossFade(state_index, duration));
    }

    pub fn reset(&mut self) {
        let entry = if self.controller.entry_state < self.controller.states.len() {
            self.controller.entry_state
        } else {
            0
        };
        self.current_state = entry;
        self.state_time = 0.0;
        self.active_transition = None;
        self.pending_request = None;
        self.state_frames = 0;
        self.machine_entered = false;
        self.parameter_values = self
            .controller
            .parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.default_value))
            .collect();
    }
    fn state_duration(&self, state_index: usize) -> f32 {
        let Some(state) = self.controller.states.get(state_index) else {
            return 0.0;
        };
        let blend_param = match &state.motion {
            StateMotion::BlendTree1D { parameter, .. } => self.get_float(parameter).unwrap_or(0.0),
            _ => 0.0,
        };
        state.motion.duration(blend_param)
    }

    fn find_matching_transition(&self, source_state: usize, normalized_time: f32) -> Option<usize> {
        for (idx, tr) in self.controller.transitions.iter().enumerate() {
            if tr.source_state != source_state {
                continue;
            }
            if tr.has_exit_time && normalized_time + 1e-6 < tr.exit_time {
                continue;
            }
            if !tr.has_exit_time && tr.conditions.is_empty() {
                return Some(idx);
            }
            if tr.conditions.iter().all(|c| self.evaluate_condition(c)) {
                return Some(idx);
            }
        }
        None
    }

    fn find_interrupting_transition(
        &self,
        source_state: usize,
        normalized_time: f32,
        exclude_target: usize,
    ) -> Option<usize> {
        for (idx, tr) in self.controller.transitions.iter().enumerate() {
            if tr.source_state != source_state {
                continue;
            }
            if tr.target == TransitionTarget::State(exclude_target) {
                continue;
            }
            if tr.has_exit_time && normalized_time + 1e-6 < tr.exit_time {
                continue;
            }
            if !tr.has_exit_time && tr.conditions.is_empty() {
                return Some(idx);
            }
            if tr.conditions.iter().all(|c| self.evaluate_condition(c)) {
                return Some(idx);
            }
        }
        None
    }

    fn evaluate_condition(&self, cond: &TransitionCondition) -> bool {
        let Some((_, value)) = self
            .parameter_values
            .iter()
            .find(|(name, _)| *name == cond.parameter)
        else {
            return false;
        };
        match (*value, cond.op) {
            (ParameterValue::Bool(b) | ParameterValue::Trigger(b), ConditionOp::If) => b,
            (ParameterValue::Bool(b) | ParameterValue::Trigger(b), ConditionOp::IfNot) => !b,
            (ParameterValue::Float(f), ConditionOp::Greater(threshold)) => f > threshold,
            (ParameterValue::Float(f), ConditionOp::Less(threshold)) => f < threshold,
            (ParameterValue::Int(i), ConditionOp::Greater(threshold)) => (i as f32) > threshold,
            (ParameterValue::Int(i), ConditionOp::Less(threshold)) => (i as f32) < threshold,
            (ParameterValue::Int(i), ConditionOp::Equals(target)) => i == target,
            (ParameterValue::Int(i), ConditionOp::NotEqual(target)) => i != target,
            _ => false,
        }
    }

    fn consume_transition_triggers(&mut self, transition: &StateTransition) {
        for cond in &transition.conditions {
            if matches!(cond.op, ConditionOp::If) {
                for (name, val) in &mut self.parameter_values {
                    if *name == cond.parameter {
                        if let ParameterValue::Trigger(flag) = val {
                            *flag = false;
                        }
                    }
                }
            }
        }
    }

    fn sample_state(&self, state_index: usize, prev_time: f32, current_time: f32) -> ClipSample {
        let Some(state) = self.controller.states.get(state_index) else {
            return ClipSample {
                bindings: Vec::new(),
                fired_events: Vec::new(),
                normalized_time: 0.0,
                wrapped_time: 0.0,
            };
        };
        match &state.motion {
            StateMotion::None => ClipSample {
                bindings: Vec::new(),
                fired_events: Vec::new(),
                normalized_time: 0.0,
                wrapped_time: 0.0,
            },
            StateMotion::Clip(clip) => clip.sample(prev_time, current_time),
            StateMotion::BlendTree1D {
                parameter,
                thresholds,
            } => {
                if thresholds.is_empty() {
                    return ClipSample {
                        bindings: Vec::new(),
                        fired_events: Vec::new(),
                        normalized_time: 0.0,
                        wrapped_time: 0.0,
                    };
                }
                let param_val = self.get_float(parameter).unwrap_or(0.0);
                let mut sorted: Vec<&(f32, AnimationClip)> = thresholds.iter().collect();
                sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
                if sorted.len() == 1 || param_val <= sorted[0].0 {
                    return sorted[0].1.sample(prev_time, current_time);
                }
                let last = sorted[sorted.len() - 1];
                if param_val >= last.0 {
                    return last.1.sample(prev_time, current_time);
                }
                for pair in sorted.windows(2) {
                    let (t0, c0) = pair[0];
                    let (t1, c1) = pair[1];
                    if param_val >= *t0 && param_val <= *t1 {
                        let span = t1 - t0;
                        let alpha = if span.abs() <= 1e-6 {
                            0.0
                        } else {
                            ((param_val - t0) / span).clamp(0.0, 1.0)
                        };
                        let s0 = c0.sample(prev_time, current_time);
                        let s1 = c1.sample(prev_time, current_time);
                        let bindings = blend_sampled_bindings(&s0.bindings, &s1.bindings, alpha);
                        let mut fired_events = s0.fired_events;
                        fired_events.extend(s1.fired_events);
                        return ClipSample {
                            bindings,
                            fired_events,
                            normalized_time: s0.normalized_time
                                + (s1.normalized_time - s0.normalized_time) * alpha,
                            wrapped_time: s0.wrapped_time
                                + (s1.wrapped_time - s0.wrapped_time) * alpha,
                        };
                    }
                }
                sorted[0].1.sample(prev_time, current_time)
            }
            StateMotion::BlendTree2D {
                parameter_x,
                parameter_y,
                children,
                ..
            } => {
                if children.is_empty() {
                    return ClipSample {
                        bindings: Vec::new(),
                        fired_events: Vec::new(),
                        normalized_time: 0.0,
                        wrapped_time: 0.0,
                    };
                }
                let px = self.get_float(parameter_x).unwrap_or(0.0);
                let py = self.get_float(parameter_y).unwrap_or(0.0);
                let mut raw_weights = Vec::with_capacity(children.len());
                for (pos, _) in children {
                    let dx = px - pos[0];
                    let dy = py - pos[1];
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= 1e-8 {
                        raw_weights.clear();
                        for (idx, _) in children.iter().enumerate() {
                            raw_weights.push(if children[idx].0 == *pos { 1.0 } else { 0.0 });
                        }
                        break;
                    }
                    raw_weights.push(1.0 / dist_sq);
                }
                let sum: f32 = raw_weights.iter().sum();
                let mut bindings = Vec::new();
                let mut fired_events = Vec::new();
                let mut acc_weight = 0.0_f32;
                let mut norm_time = 0.0_f32;
                let mut wrap_time = 0.0_f32;
                for ((_, clip), w_raw) in children.iter().zip(raw_weights) {
                    let w = if sum > 1e-8 { w_raw / sum } else { 0.0 };
                    if w <= 1e-6 {
                        continue;
                    }
                    let sample = clip.sample(prev_time, current_time);
                    let frac = w / (acc_weight + w);
                    bindings = if bindings.is_empty() {
                        sample.bindings
                    } else {
                        blend_sampled_bindings(&bindings, &sample.bindings, frac)
                    };
                    fired_events.extend(sample.fired_events);
                    norm_time += sample.normalized_time * w;
                    wrap_time += sample.wrapped_time * w;
                    acc_weight += w;
                }
                ClipSample {
                    bindings,
                    fired_events,
                    normalized_time: norm_time,
                    wrapped_time: wrap_time,
                }
            }
            StateMotion::BlendTreeDirect { children } => {
                if children.is_empty() {
                    return ClipSample {
                        bindings: Vec::new(),
                        fired_events: Vec::new(),
                        normalized_time: 0.0,
                        wrapped_time: 0.0,
                    };
                }
                let weights: Vec<f32> = children
                    .iter()
                    .map(|(param, _)| self.get_float(param).unwrap_or(0.0).max(0.0))
                    .collect();
                let sum: f32 = weights.iter().sum();
                let mut bindings = Vec::new();
                let mut fired_events = Vec::new();
                let mut acc_weight = 0.0_f32;
                let mut norm_time = 0.0_f32;
                let mut wrap_time = 0.0_f32;
                for ((_, clip), w_raw) in children.iter().zip(weights) {
                    let w = if sum > 1e-8 { w_raw / sum } else { 0.0 };
                    if w <= 1e-6 {
                        continue;
                    }
                    let sample = clip.sample(prev_time, current_time);
                    let frac = w / (acc_weight + w);
                    bindings = if bindings.is_empty() {
                        sample.bindings
                    } else {
                        blend_sampled_bindings(&bindings, &sample.bindings, frac)
                    };
                    fired_events.extend(sample.fired_events);
                    norm_time += sample.normalized_time * w;
                    wrap_time += sample.wrapped_time * w;
                    acc_weight += w;
                }
                ClipSample {
                    bindings,
                    fired_events,
                    normalized_time: norm_time,
                    wrapped_time: wrap_time,
                }
            }
        }
    }
}

pub fn blend_sampled_bindings(
    a: &[SampledBinding],
    b: &[SampledBinding],
    alpha: f32,
) -> Vec<SampledBinding> {
    let mut result = Vec::with_capacity(a.len().max(b.len()));
    for item_a in a {
        if let Some(item_b) = b.iter().find(|x| x.target == item_a.target) {
            result.push(SampledBinding {
                target: item_a.target.clone(),
                value: item_a.value.blend(&item_b.value, alpha),
            });
        } else {
            result.push(item_a.clone());
        }
    }
    for item_b in b {
        if !result.iter().any(|r| r.target == item_b.target) {
            result.push(item_b.clone());
        }
    }
    result
}
