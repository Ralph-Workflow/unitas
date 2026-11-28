use std::collections::BTreeSet;

use crate::avatar_mask::AvatarMask;
use crate::clip::{AnimationClip, AnimationEvent, ClipSample, SampledBinding};
use crate::curve::AnimatedValue;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionSource {
    State(usize),
    AnyState,
}

const ANY_STATE_FLAG: usize = 1usize << 31;
const NO_SELF_FLAG: usize = 1usize << 30;
const SOURCE_INDEX_MASK: usize = !(ANY_STATE_FLAG | NO_SELF_FLAG);

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

    pub fn any_state(target: TransitionTarget, conditions: Vec<TransitionCondition>) -> Self {
        Self::instant(ANY_STATE_FLAG, target, conditions)
    }

    pub fn any_state_with_exit_time(
        target: TransitionTarget,
        conditions: Vec<TransitionCondition>,
        exit_time: f32,
        duration: f32,
    ) -> Self {
        Self::with_exit_time(ANY_STATE_FLAG, target, conditions, exit_time, duration)
    }

    pub fn source(&self) -> TransitionSource {
        if self.source_state & ANY_STATE_FLAG != 0 {
            TransitionSource::AnyState
        } else {
            TransitionSource::State(self.source_state & SOURCE_INDEX_MASK)
        }
    }

    pub fn can_transition_to_self(&self) -> bool {
        self.source_state & NO_SELF_FLAG == 0
    }

    pub fn with_can_transition_to_self(mut self, allow: bool) -> Self {
        if allow {
            self.source_state &= !NO_SELF_FLAG;
        } else {
            self.source_state |= NO_SELF_FLAG;
        }
        self
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

/// How an additional controller layer combines with the layers below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerBlendingMode {
    #[default]
    Override,
    // shortcut: additive layers currently blend like override layers (no
    // reference-pose delta); upgrade when additive reference pose import lands
    Additive,
}

/// An AnimatorController layer above the base layer. Unity layer 0 is the base
/// layer and stays in `AnimatorController`'s top-level `states`/`entry_state`/
/// `transitions` fields; layers 1..n live here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimatorControllerLayer {
    pub name: String,
    pub weight: f32,
    pub mask: Option<AvatarMask>,
    pub blending_mode: LayerBlendingMode,
    pub states: Vec<AnimatorState>,
    pub entry_state: usize,
    pub transitions: Vec<StateTransition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatorController {
    pub name: String,
    pub parameters: Vec<ParameterDef>,
    pub states: Vec<AnimatorState>,
    pub entry_state: usize,
    pub transitions: Vec<StateTransition>,
    pub layers: Vec<AnimatorControllerLayer>,
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
            layers: Vec::new(),
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
            self.validate_state(state, &mut diagnostics);
        }
        self.validate_transitions(&mut diagnostics);
        self.validate_reachability(&mut diagnostics);
        for (layer_index, layer) in self.layers.iter().enumerate() {
            for state in &layer.states {
                self.validate_state(state, &mut diagnostics);
            }
            self.validate_layer_transitions(layer, layer_index, &mut diagnostics);
        }
        diagnostics
    }

    fn validate_layer_transitions(
        &self,
        layer: &AnimatorControllerLayer,
        layer_index: usize,
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        for transition in &layer.transitions {
            let source_name = match transition.source() {
                TransitionSource::AnyState => "Any State".to_string(),
                TransitionSource::State(index) => layer
                    .states
                    .get(index)
                    .map_or_else(|| format!("#{index}"), |state| state.name.clone()),
            };
            if let TransitionSource::State(index) = transition.source() {
                if index >= layer.states.len() {
                    diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                        source_state: format!("layer {layer_index} ({source_name})"),
                        reason: "source state index out of bounds".to_string(),
                    });
                }
            }
            if let TransitionTarget::State(dst) = transition.target {
                if dst >= layer.states.len() {
                    diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                        source_state: format!("layer {layer_index} ({source_name})"),
                        reason: "target state index out of bounds".to_string(),
                    });
                }
            }
        }
    }

    fn validate_state(&self, state: &AnimatorState, diagnostics: &mut Vec<AnimationDiagnostic>) {
        match &state.motion {
            StateMotion::None => {
                diagnostics.push(AnimationDiagnostic::MissingMotion {
                    state_name: state.name.clone(),
                });
            }
            StateMotion::Clip(clip) => {
                validate_clip_bindings(clip, diagnostics);
            }
            StateMotion::BlendTree1D {
                parameter,
                thresholds,
            } => self.validate_blend_tree_1d(state, parameter, thresholds, diagnostics),
            StateMotion::BlendTree2D {
                parameter_x,
                parameter_y,
                children,
                ..
            } => {
                self.validate_blend_tree_2d(state, parameter_x, parameter_y, children, diagnostics);
            }
            StateMotion::BlendTreeDirect { children } => {
                self.validate_blend_tree_direct(state, children, diagnostics);
            }
        }
    }

    fn validate_blend_tree_1d(
        &self,
        state: &AnimatorState,
        parameter: &str,
        thresholds: &[(f32, AnimationClip)],
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        self.validate_float_parameter(parameter, "blend tree", &state.name, diagnostics);
        if thresholds.is_empty() {
            diagnostics.push(AnimationDiagnostic::MissingMotion {
                state_name: state.name.clone(),
            });
        }
        for (_, clip) in thresholds {
            validate_clip_bindings(clip, diagnostics);
        }
    }

    fn validate_blend_tree_2d(
        &self,
        state: &AnimatorState,
        parameter_x: &str,
        parameter_y: &str,
        children: &[([f32; 2], AnimationClip)],
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        for parameter in [parameter_x, parameter_y] {
            self.validate_float_parameter(parameter, "2D blend tree", &state.name, diagnostics);
        }
        if children.is_empty() {
            diagnostics.push(AnimationDiagnostic::MissingMotion {
                state_name: state.name.clone(),
            });
        }
        for (_, clip) in children {
            validate_clip_bindings(clip, diagnostics);
        }
    }

    fn validate_blend_tree_direct(
        &self,
        state: &AnimatorState,
        children: &[(String, AnimationClip)],
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        if children.is_empty() {
            diagnostics.push(AnimationDiagnostic::MissingMotion {
                state_name: state.name.clone(),
            });
        }
        for (parameter, clip) in children {
            self.validate_float_parameter(parameter, "Direct blend tree", &state.name, diagnostics);
            validate_clip_bindings(clip, diagnostics);
        }
    }

    fn validate_float_parameter(
        &self,
        parameter: &str,
        prefix: &str,
        state_name: &str,
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        match self.parameters.iter().find(|p| p.name == parameter) {
            None => diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                parameter: parameter.to_string(),
                context: format!("{prefix} in state '{state_name}'"),
            }),
            Some(param) if param.default_value.parameter_type() != ParameterType::Float => {
                diagnostics.push(AnimationDiagnostic::InvalidParameterReference {
                    parameter: parameter.to_string(),
                    context: format!("{prefix} in state '{state_name}' requires Float parameter"),
                });
            }
            Some(_) => {}
        }
    }

    fn validate_transitions(&self, diagnostics: &mut Vec<AnimationDiagnostic>) {
        for transition in &self.transitions {
            let source_name = match transition.source() {
                TransitionSource::AnyState => "Any State".to_string(),
                TransitionSource::State(index) => self
                    .states
                    .get(index)
                    .map_or_else(|| format!("#{index}"), |state| state.name.clone()),
            };
            self.validate_transition_bounds(transition, &source_name, diagnostics);
            self.validate_duplicate_transitions(transition, &source_name, diagnostics);
            self.validate_transition_conditions(transition, &source_name, diagnostics);
        }
    }

    fn validate_transition_bounds(
        &self,
        transition: &StateTransition,
        source_name: &str,
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        if let TransitionSource::State(index) = transition.source() {
            if index >= self.states.len() {
                diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                    source_state: source_name.to_string(),
                    reason: "source state index out of bounds".to_string(),
                });
            }
        }
        if let TransitionTarget::State(dst) = transition.target {
            if dst >= self.states.len() {
                diagnostics.push(AnimationDiagnostic::ConflictingTransition {
                    source_state: source_name.to_string(),
                    reason: format!("target state index {dst} out of bounds"),
                });
            }
        }
    }

    fn validate_duplicate_transitions(
        &self,
        transition: &StateTransition,
        source_name: &str,
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
        if transition.has_exit_time || !transition.conditions.is_empty() {
            return;
        }
        let duplicate = self
            .transitions
            .iter()
            .filter(|other| {
                other.source() == transition.source()
                    && !other.has_exit_time
                    && other.conditions.is_empty()
            })
            .count();
        if duplicate > 1 {
            let conflict = AnimationDiagnostic::ConflictingTransition {
                source_state: source_name.to_string(),
                reason: "multiple unconditional transitions without exit time".to_string(),
            };
            if !diagnostics.contains(&conflict) {
                diagnostics.push(conflict);
            }
        }
    }

    fn validate_transition_conditions(
        &self,
        transition: &StateTransition,
        source_name: &str,
        diagnostics: &mut Vec<AnimationDiagnostic>,
    ) {
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

    fn validate_reachability(&self, diagnostics: &mut Vec<AnimationDiagnostic>) {
        if self.states.is_empty() {
            return;
        }
        let mut reachable = BTreeSet::new();
        let mut queue = Vec::new();
        if self.entry_state < self.states.len() {
            reachable.insert(self.entry_state);
            queue.push(self.entry_state);
        }
        for transition in &self.transitions {
            if transition.source() != TransitionSource::AnyState {
                continue;
            }
            let next_idx = self.transition_target_index(transition);
            if next_idx < self.states.len() && reachable.insert(next_idx) {
                queue.push(next_idx);
            }
        }
        while let Some(current) = queue.pop() {
            for tr in &self.transitions {
                if tr.source() != TransitionSource::State(current) {
                    continue;
                }
                let next_idx = self.transition_target_index(tr);
                if next_idx < self.states.len() && reachable.insert(next_idx) {
                    queue.push(next_idx);
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

    fn transition_target_index(&self, transition: &StateTransition) -> usize {
        match transition.target {
            TransitionTarget::State(index) => index,
            TransitionTarget::Exit => self.entry_state,
        }
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

fn capture_defaults(states: &[AnimatorState]) -> Vec<SampledBinding> {
    let mut defaults: Vec<SampledBinding> = Vec::new();
    for state in states {
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
    target_frames: u32,
}

/// Per-layer playback state for additional `AnimatorControllerLayer`s.
/// The base layer keeps using the top-level `Animator` fields.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LayerPlayer {
    current_state: usize,
    state_time: f32,
    active: Option<ActiveTransition>,
    frames: u32,
    machine_entered: bool,
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
    keep_state_on_disable: bool,
    state_frames: u32,
    machine_entered: bool,
    pending_request: Option<AnimatorRequest>,
    captured_defaults: Vec<SampledBinding>,
    layer_players: Vec<LayerPlayer>,
    layer_defaults: Vec<Vec<SampledBinding>>,
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
        let captured_defaults = capture_defaults(&controller.states);
        let layer_defaults: Vec<Vec<SampledBinding>> = controller
            .layers
            .iter()
            .map(|layer| capture_defaults(&layer.states))
            .collect();
        let layer_players: Vec<LayerPlayer> = controller
            .layers
            .iter()
            .map(|layer| LayerPlayer {
                current_state: if layer.entry_state < layer.states.len() {
                    layer.entry_state
                } else {
                    0
                },
                state_time: 0.0,
                active: None,
                frames: 0,
                machine_entered: false,
            })
            .collect();
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
            keep_state_on_disable: false,
            state_frames: 0,
            machine_entered: false,
            pending_request: None,
            captured_defaults,
            layer_players,
            layer_defaults,
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
            self.speed = speed;
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
        self.advance_layers(dt, &mut result);
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

        if let Some(request) = self.pending_request.take() {
            if let Some(result) = self.handle_request(request, &mut callbacks, &mut entered) {
                return result;
            }
        }

        let (prev_time, new_time) = self.advance_source_time(dt);
        let current_norm = self.source_normalized_time();

        if let Some(result) = self.handle_transitions(current_norm, &mut callbacks, &mut entered) {
            return result;
        }

        self.sample_and_finish(dt, prev_time, new_time, callbacks, entered)
    }

    fn handle_request(
        &mut self,
        request: AnimatorRequest,
        callbacks: &mut Vec<StateMachineCallback>,
        entered: &mut Option<usize>,
    ) -> Option<AnimatorStepResult> {
        match request {
            AnimatorRequest::Play(target) => {
                if target < self.controller.states.len() {
                    let exited = self.current_state;
                    let (bindings, fired_events) = self.jump_to_state(target);
                    return Some(self.finish(
                        std::mem::take(callbacks),
                        Some(target),
                        Some(exited),
                        bindings,
                        fired_events,
                        None,
                    ));
                }
            }
            AnimatorRequest::CrossFade(target, duration) => {
                if target < self.controller.states.len() {
                    if duration <= 1e-6 {
                        let exited = self.current_state;
                        let (bindings, fired_events) = self.jump_to_state(target);
                        return Some(self.finish(
                            std::mem::take(callbacks),
                            Some(target),
                            Some(exited),
                            bindings,
                            fired_events,
                            None,
                        ));
                    }
                    self.active_transition = Some(ActiveTransition {
                        target_state: target,
                        elapsed: 0.0,
                        duration: sanitize_delta(duration),
                        target_time: 0.0,
                        interruption_source: InterruptionSource::None,
                        target_frames: 0,
                    });
                    *entered = Some(target);
                }
            }
        }
        None
    }

    fn jump_to_state(&mut self, target: usize) -> (Vec<SampledBinding>, Vec<AnimationEvent>) {
        self.current_state = target;
        self.state_time = 0.0;
        self.active_transition = None;
        self.state_frames = 0;
        let sample = self.sample_state(self.current_state, 0.0, 0.0);
        let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
        (bindings, sample.fired_events)
    }

    fn advance_source_time(&mut self, dt: f32) -> (f32, f32) {
        let source_speed = self
            .controller
            .states
            .get(self.current_state)
            .map_or(1.0, |s| s.speed);
        let prev_time = self.state_time;
        let scaled_source_dt = dt * self.speed * source_speed.max(0.0);
        let new_time = (prev_time + scaled_source_dt).max(0.0);
        self.state_time = new_time;
        (prev_time, new_time)
    }

    fn source_normalized_time(&self) -> f32 {
        let source_length = self.state_duration(self.current_state);
        if source_length > 1e-6 {
            self.state_time / source_length
        } else if self.state_time > 0.0 {
            1.0
        } else {
            0.0
        }
    }

    fn handle_transitions(
        &mut self,
        current_norm: f32,
        callbacks: &mut Vec<StateMachineCallback>,
        entered: &mut Option<usize>,
    ) -> Option<AnimatorStepResult> {
        if let Some(active) = self.active_transition {
            let dst_len = self.state_duration(active.target_state);
            let dst_norm = if dst_len > 1e-6 {
                active.target_time / dst_len
            } else {
                0.0
            };
            let interrupt_tr_idx = self.find_interrupt_transition(&active, current_norm, dst_norm);
            if let Some(tr_idx) = interrupt_tr_idx {
                let transition = self.controller.transitions[tr_idx].clone();
                self.consume_transition_triggers(&transition);
                let target_idx = self.transition_target_or_entry(&transition);
                if transition.duration <= 1e-6 {
                    return Some(self.complete_transition_now(target_idx, callbacks));
                }
                self.begin_transition(&transition, target_idx, entered);
            }
            return None;
        }
        if let Some(tr_idx) = self.find_matching_transition(self.current_state, current_norm) {
            let transition = self.controller.transitions[tr_idx].clone();
            self.consume_transition_triggers(&transition);
            let target_idx = self.transition_target_or_entry(&transition);
            if transition.duration <= 1e-6 {
                return Some(self.complete_transition_now(target_idx, callbacks));
            }
            self.begin_transition(&transition, target_idx, entered);
        }
        None
    }

    fn find_interrupt_transition(
        &self,
        active: &ActiveTransition,
        current_norm: f32,
        dst_norm: f32,
    ) -> Option<usize> {
        let source = self.current_state;
        let target = active.target_state;
        match active.interruption_source {
            InterruptionSource::None => None,
            InterruptionSource::Source => {
                self.find_interrupting_transition(source, current_norm, target)
            }
            InterruptionSource::Destination => {
                self.find_interrupting_transition(target, dst_norm, target)
            }
            InterruptionSource::SourceThenDestination => self
                .find_interrupting_transition(source, current_norm, target)
                .or_else(|| self.find_interrupting_transition(target, dst_norm, target)),
            InterruptionSource::DestinationThenSource => self
                .find_interrupting_transition(target, dst_norm, target)
                .or_else(|| self.find_interrupting_transition(source, current_norm, target)),
        }
    }

    fn transition_target_or_entry(&self, transition: &StateTransition) -> usize {
        match transition.target {
            TransitionTarget::State(index) if index < self.controller.states.len() => index,
            TransitionTarget::State(_) | TransitionTarget::Exit => self
                .controller
                .entry_state
                .min(self.controller.states.len() - 1),
        }
    }

    fn begin_transition(
        &mut self,
        transition: &StateTransition,
        target_idx: usize,
        entered: &mut Option<usize>,
    ) {
        self.active_transition = Some(ActiveTransition {
            target_state: target_idx,
            elapsed: 0.0,
            duration: transition.duration,
            target_time: 0.0,
            interruption_source: transition.interruption_source,
            target_frames: 0,
        });
        *entered = Some(target_idx);
    }

    fn complete_transition_now(
        &mut self,
        target_idx: usize,
        callbacks: &mut Vec<StateMachineCallback>,
    ) -> AnimatorStepResult {
        let exited = self.current_state;
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
        let entered = self.current_state;
        let sample = self.sample_state(self.current_state, 0.0, 0.0);
        let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
        self.finish(
            std::mem::take(callbacks),
            Some(entered),
            Some(exited),
            bindings,
            sample.fired_events,
            None,
        )
    }

    fn sample_and_finish(
        &mut self,
        dt: f32,
        prev_time: f32,
        new_time: f32,
        callbacks: Vec<StateMachineCallback>,
        entered: Option<usize>,
    ) -> AnimatorStepResult {
        if let Some(mut tr) = self.active_transition {
            return self
                .sample_active_transition(&mut tr, dt, prev_time, new_time, callbacks, entered);
        }

        let sample = self.sample_state(self.current_state, prev_time, new_time);
        let bindings = self.apply_write_defaults(self.current_state, sample.bindings);
        self.finish(
            callbacks,
            entered,
            None,
            bindings,
            sample.fired_events,
            None,
        )
    }

    fn sample_active_transition(
        &mut self,
        tr: &mut ActiveTransition,
        dt: f32,
        prev_time: f32,
        new_time: f32,
        callbacks: Vec<StateMachineCallback>,
        entered: Option<usize>,
    ) -> AnimatorStepResult {
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
            return self.complete_transition(tr, prev_target_time, callbacks, entered);
        }

        tr.target_frames += 1;
        let destination_update = if tr.target_frames >= 2 {
            Some(tr.target_state)
        } else {
            None
        };
        self.active_transition = Some(*tr);
        let alpha = (tr.elapsed / tr.duration).clamp(0.0, 1.0);
        let src_sample = self.sample_state(self.current_state, prev_time, new_time);
        let dst_sample = self.sample_state(tr.target_state, prev_target_time, next_target_time);
        let bindings = blend_sampled_bindings(&src_sample.bindings, &dst_sample.bindings, alpha);
        let mut fired_events = src_sample.fired_events;
        fired_events.extend(dst_sample.fired_events);
        let bindings = self.apply_write_defaults(self.current_state, bindings);
        self.finish(
            callbacks,
            entered,
            None,
            bindings,
            fired_events,
            destination_update,
        )
    }

    fn complete_transition(
        &mut self,
        tr: &ActiveTransition,
        prev_target_time: f32,
        callbacks: Vec<StateMachineCallback>,
        entered: Option<usize>,
    ) -> AnimatorStepResult {
        let exited = self.current_state;
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
        self.finish(
            callbacks,
            entered,
            Some(exited),
            bindings,
            sample.fired_events,
            None,
        )
    }

    fn finish(
        &mut self,
        mut callbacks: Vec<StateMachineCallback>,
        entered: Option<usize>,
        exited: Option<usize>,
        bindings: Vec<SampledBinding>,
        fired_events: Vec<AnimationEvent>,
        destination_update: Option<usize>,
    ) -> AnimatorStepResult {
        self.state_frames += 1;
        if let Some(index) = entered {
            callbacks.push(self.state_callback(StateMachineCallbackKind::OnStateEnter, index));
        }
        if self.state_frames >= 2 && exited != Some(self.current_state) {
            callbacks.push(
                self.state_callback(StateMachineCallbackKind::OnStateUpdate, self.current_state),
            );
        }
        if let Some(index) = destination_update {
            callbacks.push(self.state_callback(StateMachineCallbackKind::OnStateUpdate, index));
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
        self.apply_write_defaults_in(
            self.controller.states.get(state_index),
            &self.captured_defaults,
            bindings,
        )
    }

    fn apply_write_defaults_in(
        &self,
        state: Option<&AnimatorState>,
        defaults: &[SampledBinding],
        bindings: Vec<SampledBinding>,
    ) -> Vec<SampledBinding> {
        let Some(state) = state else {
            return bindings;
        };
        if !state.write_defaults {
            return bindings;
        }
        let mut result = bindings;
        for default in defaults {
            if result
                .iter()
                .any(|binding| binding.target == default.target)
            {
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

    pub fn keep_state_on_disable(&self) -> bool {
        self.keep_state_on_disable
    }

    pub fn set_keep_state_on_disable(&mut self, keep: bool) {
        self.keep_state_on_disable = keep;
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

    /// Jumps to a state. Applies to the base layer only; additional layers
    /// follow their own state machines.
    pub fn play(&mut self, state_index: usize) {
        self.pending_request = Some(AnimatorRequest::Play(state_index));
    }

    pub fn cross_fade(&mut self, state_index: usize, duration: f32) {
        self.pending_request = Some(AnimatorRequest::CrossFade(state_index, duration));
    }

    pub fn reset(&mut self) {
        if self.keep_state_on_disable {
            return;
        }
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
        for (index, layer) in self.controller.layers.iter().enumerate() {
            let player = &mut self.layer_players[index];
            player.current_state = if layer.entry_state < layer.states.len() {
                layer.entry_state
            } else {
                0
            };
            player.state_time = 0.0;
            player.active = None;
            player.frames = 0;
            player.machine_entered = false;
        }
    }
    fn state_duration(&self, state_index: usize) -> f32 {
        self.state_duration_in(&self.controller.states, state_index)
    }

    fn state_duration_in(&self, states: &[AnimatorState], state_index: usize) -> f32 {
        let Some(state) = states.get(state_index) else {
            return 0.0;
        };
        let blend_param = match &state.motion {
            StateMotion::BlendTree1D { parameter, .. } => self.get_float(parameter).unwrap_or(0.0),
            _ => 0.0,
        };
        state.motion.duration(blend_param)
    }

    fn find_matching_transition(&self, source_state: usize, normalized_time: f32) -> Option<usize> {
        self.find_transition(TransitionSource::AnyState, normalized_time)
            .or_else(|| {
                self.find_transition(TransitionSource::State(source_state), normalized_time)
            })
    }

    fn find_transition(&self, wanted: TransitionSource, normalized_time: f32) -> Option<usize> {
        for (idx, tr) in self.controller.transitions.iter().enumerate() {
            if tr.source() != wanted {
                continue;
            }
            if !self.transition_ready(tr, normalized_time) {
                continue;
            }
            return Some(idx);
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
            let source_matches = match tr.source() {
                TransitionSource::AnyState => true,
                TransitionSource::State(index) => index == source_state,
            };
            if !source_matches {
                continue;
            }
            if tr.target == TransitionTarget::State(exclude_target) {
                continue;
            }
            if !self.transition_ready(tr, normalized_time) {
                continue;
            }
            return Some(idx);
        }
        None
    }

    fn transition_ready(&self, transition: &StateTransition, normalized_time: f32) -> bool {
        if !transition.can_transition_to_self()
            && transition.target == TransitionTarget::State(self.current_state)
        {
            return false;
        }
        if transition.has_exit_time && normalized_time + 1e-6 < transition.exit_time {
            return false;
        }
        if !transition.has_exit_time && transition.conditions.is_empty() {
            return true;
        }
        transition
            .conditions
            .iter()
            .all(|c| self.evaluate_condition(c))
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
        self.sample_state_in(
            &self.controller.states,
            state_index,
            prev_time,
            current_time,
        )
    }

    fn sample_state_in(
        &self,
        states: &[AnimatorState],
        state_index: usize,
        prev_time: f32,
        current_time: f32,
    ) -> ClipSample {
        let Some(state) = states.get(state_index) else {
            return empty_clip_sample();
        };
        match &state.motion {
            StateMotion::None => empty_clip_sample(),
            StateMotion::Clip(clip) => clip.sample(prev_time, current_time),
            StateMotion::BlendTree1D {
                parameter,
                thresholds,
            } => self.sample_blend_tree_1d(parameter, thresholds, prev_time, current_time),
            StateMotion::BlendTree2D {
                parameter_x,
                parameter_y,
                children,
                ..
            } => self.sample_blend_tree_2d(
                parameter_x,
                parameter_y,
                children,
                prev_time,
                current_time,
            ),
            StateMotion::BlendTreeDirect { children } => {
                self.sample_blend_tree_direct(children, prev_time, current_time)
            }
        }
    }

    fn sample_blend_tree_1d(
        &self,
        parameter: &str,
        thresholds: &[(f32, AnimationClip)],
        prev_time: f32,
        current_time: f32,
    ) -> ClipSample {
        if thresholds.is_empty() {
            return empty_clip_sample();
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
                    wrapped_time: s0.wrapped_time + (s1.wrapped_time - s0.wrapped_time) * alpha,
                };
            }
        }
        sorted[0].1.sample(prev_time, current_time)
    }

    fn sample_blend_tree_2d(
        &self,
        parameter_x: &str,
        parameter_y: &str,
        children: &[([f32; 2], AnimationClip)],
        prev_time: f32,
        current_time: f32,
    ) -> ClipSample {
        if children.is_empty() {
            return empty_clip_sample();
        }
        let px = self.get_float(parameter_x).unwrap_or(0.0);
        let py = self.get_float(parameter_y).unwrap_or(0.0);
        let raw_weights = blend_tree_2d_weights(px, py, children);
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

    fn sample_blend_tree_direct(
        &self,
        children: &[(String, AnimationClip)],
        prev_time: f32,
        current_time: f32,
    ) -> ClipSample {
        if children.is_empty() {
            return empty_clip_sample();
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
    // Advances every additional controller layer and blends its sampled pose
    // into `result` by layer weight, restricted to the layer's avatar mask.
    // Bindings no lower layer animates lerp from their default value, so a
    // partial-weight layer only partially applies its unique targets.
    // Additive layers use the same weighted blend as override layers (see
    // LayerBlendingMode).
    fn advance_layers(&mut self, dt: f32, result: &mut AnimatorStepResult) {
        if self.controller.layers.is_empty() {
            return;
        }
        for layer_index in 0..self.controller.layers.len() {
            let weight = match self.controller.layers[layer_index].weight {
                w if w.is_finite() => w.clamp(0.0, 1.0),
                _ => 0.0,
            };
            let mut callbacks = Vec::new();
            let mut fired_events = Vec::new();
            let bindings = self.advance_layer(layer_index, dt, &mut callbacks, &mut fired_events);
            result.callbacks.extend(callbacks);
            result.fired_events.extend(fired_events);
            if weight <= 1e-6 || bindings.is_empty() {
                continue;
            }
            let layer_bindings = match &self.controller.layers[layer_index].mask {
                Some(mask) => mask.filter_bindings(&bindings),
                None => bindings,
            };
            if layer_bindings.is_empty() {
                continue;
            }
            result.bindings = blend_layer_bindings(&result.bindings, &layer_bindings, weight);
        }
    }

    // Runs one layer's state machine for a step and returns its sampled
    // bindings. Supports entry/default states, any-state and state
    // transitions with conditions or exit time, and duration cross-fades.
    // shortcut: no transition interruption in additional layers; upgrade if
    // layered interruption is ever observed in real controllers
    fn advance_layer(
        &mut self,
        layer_index: usize,
        dt: f32,
        callbacks: &mut Vec<StateMachineCallback>,
        fired_events: &mut Vec<AnimationEvent>,
    ) -> Vec<SampledBinding> {
        let entry = {
            let layer = &self.controller.layers[layer_index];
            if layer.states.is_empty() {
                return Vec::new();
            }
            layer.entry_state.min(layer.states.len() - 1)
        };
        let mut player = self.layer_players[layer_index];
        if !player.machine_entered {
            player.machine_entered = true;
            callbacks.push(StateMachineCallback {
                kind: StateMachineCallbackKind::OnStateMachineEnter,
                state_index: player.current_state,
                state_name: String::new(),
                machine: Some(self.controller.layers[layer_index].name.clone()),
            });
        }
        player.current_state = player
            .current_state
            .min(self.controller.layers[layer_index].states.len() - 1);

        let state_speed = self.controller.layers[layer_index].states[player.current_state].speed;
        let prev_time = player.state_time;
        let new_time = (prev_time + dt * self.speed * state_speed.max(0.0)).max(0.0);
        player.state_time = new_time;
        let current_norm = {
            let length = self.state_duration_in(
                &self.controller.layers[layer_index].states,
                player.current_state,
            );
            if length > 1e-6 {
                new_time / length
            } else if new_time > 0.0 {
                1.0
            } else {
                0.0
            }
        };

        if player.active.is_none() {
            let found = {
                let layer = &self.controller.layers[layer_index];
                layer
                    .transitions
                    .iter()
                    .position(|tr| {
                        tr.source() == TransitionSource::AnyState
                            && self.layer_transition_ready(tr, player.current_state, current_norm)
                    })
                    .or_else(|| {
                        layer.transitions.iter().position(|tr| {
                            tr.source() == TransitionSource::State(player.current_state)
                                && self.layer_transition_ready(
                                    tr,
                                    player.current_state,
                                    current_norm,
                                )
                        })
                    })
            };
            if let Some(tr_idx) = found {
                let transition = self.controller.layers[layer_index].transitions[tr_idx].clone();
                self.consume_transition_triggers(&transition);
                let target = match transition.target {
                    TransitionTarget::State(index)
                        if index < self.controller.layers[layer_index].states.len() =>
                    {
                        index
                    }
                    TransitionTarget::State(_) | TransitionTarget::Exit => entry,
                };
                if transition.duration <= 1e-6 {
                    player.current_state = target;
                    player.state_time = 0.0;
                    player.frames = 0;
                    self.push_layer_state_callback(
                        callbacks,
                        StateMachineCallbackKind::OnStateEnter,
                        layer_index,
                        target,
                    );
                } else {
                    player.active = Some(ActiveTransition {
                        target_state: target,
                        elapsed: 0.0,
                        duration: transition.duration,
                        target_time: 0.0,
                        interruption_source: InterruptionSource::None,
                        target_frames: 0,
                    });
                    self.push_layer_state_callback(
                        callbacks,
                        StateMachineCallbackKind::OnStateEnter,
                        layer_index,
                        target,
                    );
                }
            }
        }

        let sample = if let Some(mut active) = player.active.take() {
            let target_speed = self.controller.layers[layer_index]
                .states
                .get(active.target_state)
                .map_or(1.0, |s| s.speed);
            let prev_target_time = active.target_time;
            active.target_time += dt * self.speed * target_speed.max(0.0);
            active.elapsed += dt * self.speed;
            if active.elapsed >= active.duration {
                player.current_state = active.target_state;
                player.state_time = active.target_time;
                player.frames = 0;
                if self.controller.layers[layer_index].states[player.current_state].is_exit {
                    player.current_state = entry;
                    player.state_time = 0.0;
                }
                self.sample_state_in(
                    &self.controller.layers[layer_index].states,
                    player.current_state,
                    prev_target_time,
                    player.state_time,
                )
            } else {
                player.active = Some(active);
                player.frames += 1;
                let alpha = (active.elapsed / active.duration).clamp(0.0, 1.0);
                let src = self.sample_state_in(
                    &self.controller.layers[layer_index].states,
                    player.current_state,
                    prev_time,
                    new_time,
                );
                let dst = self.sample_state_in(
                    &self.controller.layers[layer_index].states,
                    active.target_state,
                    prev_target_time,
                    active.target_time,
                );
                let bindings = blend_sampled_bindings(&src.bindings, &dst.bindings, alpha);
                fired_events.extend(src.fired_events);
                fired_events.extend(dst.fired_events);
                ClipSample {
                    bindings,
                    fired_events: Vec::new(),
                    normalized_time: src.normalized_time
                        + (dst.normalized_time - src.normalized_time) * alpha,
                    wrapped_time: src.wrapped_time + (dst.wrapped_time - src.wrapped_time) * alpha,
                }
            }
        } else {
            if self.controller.layers[layer_index].states[player.current_state].is_exit {
                player.current_state = entry;
                player.state_time = 0.0;
            }
            self.sample_state_in(
                &self.controller.layers[layer_index].states,
                player.current_state,
                prev_time,
                new_time,
            )
        };

        player.frames += 1;
        self.layer_players[layer_index] = player;
        let bindings = self.apply_write_defaults_in(
            self.controller.layers[layer_index]
                .states
                .get(player.current_state),
            &self.layer_defaults[layer_index],
            sample.bindings,
        );
        fired_events.extend(sample.fired_events);
        bindings
    }

    fn layer_transition_ready(
        &self,
        transition: &StateTransition,
        current_state: usize,
        normalized_time: f32,
    ) -> bool {
        if !transition.can_transition_to_self()
            && transition.target == TransitionTarget::State(current_state)
        {
            return false;
        }
        if transition.has_exit_time && normalized_time + 1e-6 < transition.exit_time {
            return false;
        }
        if !transition.has_exit_time && transition.conditions.is_empty() {
            return true;
        }
        transition
            .conditions
            .iter()
            .all(|c| self.evaluate_condition(c))
    }

    fn push_layer_state_callback(
        &self,
        callbacks: &mut Vec<StateMachineCallback>,
        kind: StateMachineCallbackKind,
        layer_index: usize,
        state_index: usize,
    ) {
        let state_name = self.controller.layers[layer_index]
            .states
            .get(state_index)
            .map(|state| state.name.clone())
            .unwrap_or_default();
        callbacks.push(StateMachineCallback {
            kind,
            state_index,
            state_name,
            machine: Some(self.controller.layers[layer_index].name.clone()),
        });
    }
}

fn empty_clip_sample() -> ClipSample {
    ClipSample {
        bindings: Vec::new(),
        fired_events: Vec::new(),
        normalized_time: 0.0,
        wrapped_time: 0.0,
    }
}

fn blend_tree_2d_weights(px: f32, py: f32, children: &[([f32; 2], AnimationClip)]) -> Vec<f32> {
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
    raw_weights
}

pub fn blend_layer_bindings(
    lower: &[SampledBinding],
    layer: &[SampledBinding],
    weight: f32,
) -> Vec<SampledBinding> {
    let w = if weight.is_finite() {
        weight.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut result = Vec::with_capacity(lower.len().max(layer.len()));
    for item in lower {
        if let Some(top) = layer.iter().find(|x| x.target == item.target) {
            result.push(SampledBinding {
                target: item.target.clone(),
                value: item.value.blend(&top.value, w),
            });
        } else {
            result.push(item.clone());
        }
    }
    for item in layer {
        if result.iter().any(|r| r.target == item.target) {
            continue;
        }
        let value = match &item.value {
            AnimatedValue::Float(_) => AnimatedValue::Float(0.0).blend(&item.value, w),
            AnimatedValue::Vector3(_) => AnimatedValue::Vector3([0.0; 3]).blend(&item.value, w),
            AnimatedValue::Quaternion(_) => {
                AnimatedValue::Quaternion([0.0, 0.0, 0.0, 1.0]).blend(&item.value, w)
            }
            AnimatedValue::Color(_) => AnimatedValue::Color([0.0; 4]).blend(&item.value, w),
            AnimatedValue::ObjectReference(_) => {
                if w < 0.5 {
                    continue;
                }
                item.value
            }
        };
        result.push(SampledBinding {
            target: item.target.clone(),
            value,
        });
    }
    result
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
