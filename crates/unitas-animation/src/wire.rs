use crate::clip::{AnimationClip, AnimationEvent, CurveBinding, CurveBindingTarget, WrapMode};
use crate::controller::{
    AnimatorController, AnimatorControllerLayer, AnimatorState, BlendTree2DMode, ConditionOp,
    InterruptionSource, LayerBlendingMode, ParameterDef, ParameterValue, StateMotion,
    StateTransition, TransitionCondition, TransitionTarget,
};
use crate::curve::{
    AssetReference, FloatCurve, InterpolationMode, Keyframe, ObjectReferenceCurve,
    ObjectReferenceKey, PropertyCurve,
};
use crate::error::AnimationError;

pub const WIRE_MAGIC: [u8; 4] = *b"UANC";
pub const WIRE_VERSION: u32 = 2;
const EXTENSION_MAGIC: u32 = 0x5744_0001;
const EXTENSION_LAYERS_MAGIC: u32 = 0x5744_0002;

pub fn encode_controller_binary(controller: &AnimatorController) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&WIRE_MAGIC);
    write_u32(&mut out, WIRE_VERSION);
    write_str(&mut out, &controller.name);

    write_parameters(&mut out, controller);
    write_states(&mut out, controller);
    write_u32(&mut out, controller.entry_state as u32);
    write_transitions(&mut out, controller);
    write_extensions(&mut out, controller);

    out
}

fn write_parameters(out: &mut Vec<u8>, controller: &AnimatorController) {
    write_u32(out, controller.parameters.len() as u32);
    for param in &controller.parameters {
        write_str(out, &param.name);
        match param.default_value {
            ParameterValue::Float(v) => {
                write_u8(out, 0);
                write_f32(out, v);
            }
            ParameterValue::Int(v) => {
                write_u8(out, 1);
                write_i32(out, v);
            }
            ParameterValue::Bool(v) => {
                write_u8(out, 2);
                write_bool(out, v);
            }
            ParameterValue::Trigger(v) => {
                write_u8(out, 3);
                write_bool(out, v);
            }
        }
    }
}

fn write_states(out: &mut Vec<u8>, controller: &AnimatorController) {
    write_states_slice(out, &controller.states);
}

fn write_states_slice(out: &mut Vec<u8>, states: &[AnimatorState]) {
    write_u32(out, states.len() as u32);
    for state in states {
        write_str(out, &state.name);
        write_f32(out, state.speed);
        write_bool(out, state.is_exit);
        write_motion(out, &state.motion);
    }
}

fn write_motion(out: &mut Vec<u8>, motion: &StateMotion) {
    match motion {
        StateMotion::None => write_u8(out, 0),
        StateMotion::Clip(clip) => {
            write_u8(out, 1);
            write_clip(out, clip);
        }
        StateMotion::BlendTree1D {
            parameter,
            thresholds,
        } => {
            write_u8(out, 2);
            write_str(out, parameter);
            write_u32(out, thresholds.len() as u32);
            for (threshold, clip) in thresholds {
                write_f32(out, *threshold);
                write_clip(out, clip);
            }
        }
        StateMotion::BlendTree2D {
            mode,
            parameter_x,
            parameter_y,
            children,
        } => {
            write_u8(out, 3);
            let mode_tag = match mode {
                BlendTree2DMode::SimpleDirectional => 0,
                BlendTree2DMode::FreeformDirectional => 1,
                BlendTree2DMode::FreeformCartesian => 2,
            };
            write_u8(out, mode_tag);
            write_str(out, parameter_x);
            write_str(out, parameter_y);
            write_u32(out, children.len() as u32);
            for (pos, clip) in children {
                write_f32(out, pos[0]);
                write_f32(out, pos[1]);
                write_clip(out, clip);
            }
        }
        StateMotion::BlendTreeDirect { children } => {
            write_u8(out, 4);
            write_u32(out, children.len() as u32);
            for (param, clip) in children {
                write_str(out, param);
                write_clip(out, clip);
            }
        }
    }
}

fn write_transitions(out: &mut Vec<u8>, controller: &AnimatorController) {
    write_transitions_slice(out, &controller.transitions);
}

fn write_transitions_slice(out: &mut Vec<u8>, transitions: &[StateTransition]) {
    write_u32(out, transitions.len() as u32);
    for tr in transitions {
        write_u32(out, tr.source_state as u32);
        write_transition_target(out, &tr.target);
        write_bool(out, tr.has_exit_time);
        write_f32(out, tr.exit_time);
        write_f32(out, tr.duration);
        write_u32(out, tr.conditions.len() as u32);
        for cond in &tr.conditions {
            write_str(out, &cond.parameter);
            write_condition_op(out, &cond.op);
        }
    }
}

fn write_transition_target(out: &mut Vec<u8>, target: &TransitionTarget) {
    match target {
        TransitionTarget::State(idx) => {
            write_u8(out, 0);
            write_u32(out, *idx as u32);
        }
        TransitionTarget::Exit => {
            write_u8(out, 1);
            write_u32(out, 0);
        }
    }
}

fn write_condition_op(out: &mut Vec<u8>, op: &ConditionOp) {
    match op {
        ConditionOp::If => write_u8(out, 0),
        ConditionOp::IfNot => write_u8(out, 1),
        ConditionOp::Greater(v) => {
            write_u8(out, 2);
            write_f32(out, *v);
        }
        ConditionOp::Less(v) => {
            write_u8(out, 3);
            write_f32(out, *v);
        }
        ConditionOp::Equals(v) => {
            write_u8(out, 4);
            write_i32(out, *v);
        }
        ConditionOp::NotEqual(v) => {
            write_u8(out, 5);
            write_i32(out, *v);
        }
    }
}

fn write_extensions(out: &mut Vec<u8>, controller: &AnimatorController) {
    write_u32(out, EXTENSION_MAGIC);
    write_u32(out, controller.states.len() as u32);
    for state in &controller.states {
        write_bool(out, state.write_defaults);
    }
    if !controller.layers.is_empty() {
        write_u32(out, EXTENSION_LAYERS_MAGIC);
        write_u32(out, controller.layers.len() as u32);
        for layer in &controller.layers {
            write_str(out, &layer.name);
            write_f32(out, layer.weight);
            write_u8(
                out,
                match layer.blending_mode {
                    LayerBlendingMode::Override => 0,
                    LayerBlendingMode::Additive => 1,
                },
            );
            match &layer.mask {
                Some(mask) => {
                    write_u8(out, 1);
                    write_str(out, mask.name());
                    write_u32(out, mask.paths().len() as u32);
                    for path in mask.paths() {
                        write_str(out, path);
                    }
                }
                None => write_u8(out, 0),
            }
            write_states_slice(out, &layer.states);
            write_u32(out, layer.entry_state as u32);
            write_transitions_slice(out, &layer.transitions);
        }
    }
}

pub fn decode_controller_binary(bytes: &[u8]) -> Result<AnimatorController, AnimationError> {
    let mut cursor = Cursor::new(bytes);
    let magic = cursor.read_exact(4)?;
    if magic != WIRE_MAGIC {
        return Err(AnimationError::InvalidMagic);
    }
    let version = cursor.read_u32()?;
    if version > WIRE_VERSION {
        return Err(AnimationError::UnsupportedVersion(version));
    }
    let name = cursor.read_str()?;

    let parameters = read_parameters(&mut cursor)?;
    let mut states = read_states(&mut cursor)?;
    let entry_state = cursor.read_u32()? as usize;
    let transitions = read_transitions(&mut cursor)?;
    read_extensions(&mut cursor, &mut states)?;
    let layers = read_layers_extension(&mut cursor)?;

    Ok(AnimatorController {
        name,
        parameters,
        states,
        entry_state,
        transitions,
        layers,
    })
}

fn read_parameters(cursor: &mut Cursor<'_>) -> Result<Vec<ParameterDef>, AnimationError> {
    let param_count = cursor.read_u32()? as usize;
    let mut parameters = Vec::with_capacity(param_count.min(1024));
    for _ in 0..param_count {
        let param_name = cursor.read_str()?;
        let tag = cursor.read_u8()?;
        let default_value = match tag {
            0 => ParameterValue::Float(cursor.read_f32()?),
            1 => ParameterValue::Int(cursor.read_i32()?),
            2 => ParameterValue::Bool(cursor.read_bool("ParameterValue.Bool")?),
            3 => ParameterValue::Trigger(cursor.read_bool("ParameterValue.Trigger")?),
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "ParameterValue",
                    tag: other,
                })
            }
        };
        parameters.push(ParameterDef::new(param_name, default_value));
    }
    Ok(parameters)
}

fn read_states(cursor: &mut Cursor<'_>) -> Result<Vec<AnimatorState>, AnimationError> {
    let state_count = cursor.read_u32()? as usize;
    let mut states = Vec::with_capacity(state_count.min(1024));
    for _ in 0..state_count {
        let state_name = cursor.read_str()?;
        let speed = cursor.read_f32()?;
        let is_exit = cursor.read_bool("AnimatorState.is_exit")?;
        let motion_tag = cursor.read_u8()?;
        let motion = read_motion(cursor, motion_tag)?;
        states.push(AnimatorState {
            name: state_name,
            motion,
            speed,
            is_exit,
            write_defaults: true,
        });
    }
    Ok(states)
}

fn read_motion(cursor: &mut Cursor<'_>, motion_tag: u8) -> Result<StateMotion, AnimationError> {
    match motion_tag {
        0 => Ok(StateMotion::None),
        1 => Ok(StateMotion::Clip(read_clip(cursor)?)),
        2 => read_blend_tree_1d_motion(cursor),
        3 => read_blend_tree_2d_motion(cursor),
        4 => read_blend_tree_direct_motion(cursor),
        other => Err(AnimationError::InvalidTag {
            section: "StateMotion",
            tag: other,
        }),
    }
}

fn read_blend_tree_1d_motion(cursor: &mut Cursor<'_>) -> Result<StateMotion, AnimationError> {
    let parameter = cursor.read_str()?;
    let threshold_count = cursor.read_u32()? as usize;
    let mut thresholds = Vec::with_capacity(threshold_count.min(1024));
    for _ in 0..threshold_count {
        let threshold = cursor.read_f32()?;
        let clip = read_clip(cursor)?;
        thresholds.push((threshold, clip));
    }
    Ok(StateMotion::BlendTree1D {
        parameter,
        thresholds,
    })
}

fn read_blend_tree_2d_motion(cursor: &mut Cursor<'_>) -> Result<StateMotion, AnimationError> {
    let mode = read_blend_tree_2d_mode(cursor)?;
    let parameter_x = cursor.read_str()?;
    let parameter_y = cursor.read_str()?;
    let count = cursor.read_u32()? as usize;
    let mut children = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let x = cursor.read_f32()?;
        let y = cursor.read_f32()?;
        let clip = read_clip(cursor)?;
        children.push(([x, y], clip));
    }
    Ok(StateMotion::BlendTree2D {
        mode,
        parameter_x,
        parameter_y,
        children,
    })
}

fn read_blend_tree_direct_motion(cursor: &mut Cursor<'_>) -> Result<StateMotion, AnimationError> {
    let count = cursor.read_u32()? as usize;
    let mut children = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let param = cursor.read_str()?;
        let clip = read_clip(cursor)?;
        children.push((param, clip));
    }
    Ok(StateMotion::BlendTreeDirect { children })
}

fn read_blend_tree_2d_mode(cursor: &mut Cursor<'_>) -> Result<BlendTree2DMode, AnimationError> {
    let mode_tag = cursor.read_u8()?;
    match mode_tag {
        0 => Ok(BlendTree2DMode::SimpleDirectional),
        1 => Ok(BlendTree2DMode::FreeformDirectional),
        2 => Ok(BlendTree2DMode::FreeformCartesian),
        other => Err(AnimationError::InvalidTag {
            section: "BlendTree2DMode",
            tag: other,
        }),
    }
}

fn read_transitions(cursor: &mut Cursor<'_>) -> Result<Vec<StateTransition>, AnimationError> {
    let tr_count = cursor.read_u32()? as usize;
    let mut transitions = Vec::with_capacity(tr_count.min(1024));
    for _ in 0..tr_count {
        let source_state = cursor.read_u32()? as usize;
        let target = read_transition_target(cursor)?;
        let has_exit_time = cursor.read_bool("StateTransition.has_exit_time")?;
        let exit_time = cursor.read_f32()?;
        let duration = cursor.read_f32()?;
        let cond_count = cursor.read_u32()? as usize;
        let mut conditions = Vec::with_capacity(cond_count.min(1024));
        for _ in 0..cond_count {
            let parameter = cursor.read_str()?;
            let op = read_condition_op(cursor)?;
            conditions.push(TransitionCondition::new(parameter, op));
        }
        transitions.push(StateTransition {
            source_state,
            target,
            conditions,
            has_exit_time,
            exit_time,
            duration,
            interruption_source: InterruptionSource::None,
        });
    }
    Ok(transitions)
}

fn read_transition_target(cursor: &mut Cursor<'_>) -> Result<TransitionTarget, AnimationError> {
    let target_tag = cursor.read_u8()?;
    let target_idx = cursor.read_u32()? as usize;
    match target_tag {
        0 => Ok(TransitionTarget::State(target_idx)),
        1 => Ok(TransitionTarget::Exit),
        other => Err(AnimationError::InvalidTag {
            section: "TransitionTarget",
            tag: other,
        }),
    }
}

fn read_condition_op(cursor: &mut Cursor<'_>) -> Result<ConditionOp, AnimationError> {
    let op_tag = cursor.read_u8()?;
    match op_tag {
        0 => Ok(ConditionOp::If),
        1 => Ok(ConditionOp::IfNot),
        2 => Ok(ConditionOp::Greater(cursor.read_f32()?)),
        3 => Ok(ConditionOp::Less(cursor.read_f32()?)),
        4 => Ok(ConditionOp::Equals(cursor.read_i32()?)),
        5 => Ok(ConditionOp::NotEqual(cursor.read_i32()?)),
        other => Err(AnimationError::InvalidTag {
            section: "ConditionOp",
            tag: other,
        }),
    }
}

fn read_extensions(
    cursor: &mut Cursor<'_>,
    states: &mut [AnimatorState],
) -> Result<(), AnimationError> {
    if cursor.remaining() < 4 || cursor.peek_u32()? != EXTENSION_MAGIC {
        return Ok(());
    }
    cursor.skip(4);
    let flag_count = cursor.read_u32()? as usize;
    if flag_count != states.len() {
        return Err(AnimationError::InvalidTag {
            section: "StateExtension",
            tag: flag_count.min(255) as u8,
        });
    }
    for state in states.iter_mut() {
        state.write_defaults = cursor.read_bool("AnimatorState.write_defaults")?;
    }
    Ok(())
}

fn read_layers_extension(
    cursor: &mut Cursor<'_>,
) -> Result<Vec<AnimatorControllerLayer>, AnimationError> {
    if cursor.remaining() < 4 || cursor.peek_u32()? != EXTENSION_LAYERS_MAGIC {
        return Ok(Vec::new());
    }
    cursor.skip(4);
    let layer_count = cursor.read_u32()? as usize;
    let mut layers = Vec::with_capacity(layer_count.min(64));
    for _ in 0..layer_count {
        let name = cursor.read_str()?;
        let weight = cursor.read_f32()?;
        let blending_mode = match cursor.read_u8()? {
            0 => LayerBlendingMode::Override,
            1 => LayerBlendingMode::Additive,
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "LayerBlendingMode",
                    tag: other,
                })
            }
        };
        let mask = match cursor.read_u8()? {
            0 => None,
            1 => {
                let mask_name = cursor.read_str()?;
                let path_count = cursor.read_u32()? as usize;
                let mut mask = crate::avatar_mask::AvatarMask::new(mask_name);
                for _ in 0..path_count.min(4096) {
                    mask = mask.with_path(cursor.read_str()?);
                }
                Some(mask)
            }
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "LayerMask",
                    tag: other,
                })
            }
        };
        let states = read_states(cursor)?;
        let entry_state = cursor.read_u32()? as usize;
        let transitions = read_transitions(cursor)?;
        layers.push(AnimatorControllerLayer {
            name,
            weight,
            mask,
            blending_mode,
            states,
            entry_state,
            transitions,
        });
    }
    Ok(layers)
}

fn write_clip(out: &mut Vec<u8>, clip: &AnimationClip) {
    write_str(out, &clip.name);
    write_f32(out, clip.duration);
    write_f32(out, clip.sample_rate);
    let wrap_tag = match clip.wrap_mode {
        WrapMode::Once => 0,
        WrapMode::Loop => 1,
        WrapMode::PingPong => 2,
        WrapMode::ClampForever => 3,
    };
    write_u8(out, wrap_tag);

    write_u32(out, clip.bindings.len() as u32);
    for binding in &clip.bindings {
        write_binding(out, binding);
    }

    write_u32(out, clip.events.len() as u32);
    for ev in &clip.events {
        write_f32(out, ev.time);
        write_str(out, &ev.function_name);
        write_f32(out, ev.float_parameter);
        write_i32(out, ev.int_parameter);
        write_str(out, &ev.string_parameter);
    }
}

fn write_binding(out: &mut Vec<u8>, binding: &CurveBinding) {
    write_str(out, &binding.target.object_path);
    write_str(out, &binding.target.component);
    write_str(out, &binding.target.property);
    match &binding.curve {
        PropertyCurve::Float(c) => {
            write_u8(out, 0);
            write_float_curve(out, c);
        }
        PropertyCurve::Vector3(cs) => {
            write_u8(out, 1);
            for c in cs {
                write_float_curve(out, c);
            }
        }
        PropertyCurve::Quaternion(cs) => {
            write_u8(out, 2);
            for c in cs {
                write_float_curve(out, c);
            }
        }
        PropertyCurve::Color(cs) => {
            write_u8(out, 3);
            for c in cs {
                write_float_curve(out, c);
            }
        }
        PropertyCurve::ObjectReference(c) => {
            write_u8(out, 4);
            write_u32(out, c.keys.len() as u32);
            for key in &c.keys {
                write_f32(out, key.time);
                write_u128(out, key.reference.guid);
                write_i64(out, key.reference.file_id);
            }
        }
    }
}

fn read_clip(cursor: &mut Cursor<'_>) -> Result<AnimationClip, AnimationError> {
    let name = cursor.read_str()?;
    let duration = cursor.read_f32()?;
    let sample_rate = cursor.read_f32()?;
    let wrap_mode = read_wrap_mode(cursor)?;

    let binding_count = cursor.read_u32()? as usize;
    let mut bindings = Vec::with_capacity(binding_count.min(1024));
    for _ in 0..binding_count {
        let object_path = cursor.read_str()?;
        let component = cursor.read_str()?;
        let property = cursor.read_str()?;
        let curve = read_property_curve(cursor)?;
        bindings.push(CurveBinding::new(
            CurveBindingTarget::new(object_path, component, property),
            curve,
        ));
    }

    let event_count = cursor.read_u32()? as usize;
    let mut events = Vec::with_capacity(event_count.min(1024));
    for _ in 0..event_count {
        events.push(read_event(cursor)?);
    }

    Ok(AnimationClip {
        name,
        duration,
        sample_rate,
        wrap_mode,
        bindings,
        events,
    })
}

fn read_wrap_mode(cursor: &mut Cursor<'_>) -> Result<WrapMode, AnimationError> {
    let wrap_tag = cursor.read_u8()?;
    match wrap_tag {
        0 => Ok(WrapMode::Once),
        1 => Ok(WrapMode::Loop),
        2 => Ok(WrapMode::PingPong),
        3 => Ok(WrapMode::ClampForever),
        other => Err(AnimationError::InvalidTag {
            section: "WrapMode",
            tag: other,
        }),
    }
}

fn read_property_curve(cursor: &mut Cursor<'_>) -> Result<PropertyCurve, AnimationError> {
    let curve_tag = cursor.read_u8()?;
    match curve_tag {
        0 => Ok(PropertyCurve::Float(read_float_curve(cursor)?)),
        1 => Ok(PropertyCurve::Vector3([
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
        ])),
        2 => Ok(PropertyCurve::Quaternion([
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
        ])),
        3 => Ok(PropertyCurve::Color([
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
            read_float_curve(cursor)?,
        ])),
        4 => {
            let key_count = cursor.read_u32()? as usize;
            let mut keys = Vec::with_capacity(key_count.min(4096));
            for _ in 0..key_count {
                let time = cursor.read_f32()?;
                let guid = cursor.read_u128()?;
                let file_id = cursor.read_i64()?;
                keys.push(ObjectReferenceKey::new(
                    time,
                    AssetReference::new(guid, file_id),
                ));
            }
            Ok(PropertyCurve::ObjectReference(ObjectReferenceCurve::new(
                keys,
            )))
        }
        other => Err(AnimationError::InvalidTag {
            section: "PropertyCurve",
            tag: other,
        }),
    }
}

fn read_event(cursor: &mut Cursor<'_>) -> Result<AnimationEvent, AnimationError> {
    let time = cursor.read_f32()?;
    let function_name = cursor.read_str()?;
    let float_parameter = cursor.read_f32()?;
    let int_parameter = cursor.read_i32()?;
    let string_parameter = cursor.read_str()?;
    Ok(AnimationEvent {
        time,
        function_name,
        float_parameter,
        int_parameter,
        string_parameter,
    })
}

fn write_float_curve(out: &mut Vec<u8>, curve: &FloatCurve) {
    write_u32(out, curve.keys.len() as u32);
    for key in &curve.keys {
        write_f32(out, key.time);
        write_f32(out, key.value);
        write_f32(out, key.in_tangent);
        write_f32(out, key.out_tangent);
        let mode_tag = match key.mode {
            InterpolationMode::Hermite => 0,
            InterpolationMode::Linear => 1,
            InterpolationMode::Stepped => 2,
        };
        write_u8(out, mode_tag);
    }
}

fn read_float_curve(cursor: &mut Cursor<'_>) -> Result<FloatCurve, AnimationError> {
    let key_count = cursor.read_u32()? as usize;
    let mut keys = Vec::with_capacity(key_count.min(4096));
    for _ in 0..key_count {
        let time = cursor.read_f32()?;
        let value = cursor.read_f32()?;
        let in_tangent = cursor.read_f32()?;
        let out_tangent = cursor.read_f32()?;
        let mode_tag = cursor.read_u8()?;
        let mode = match mode_tag {
            0 => InterpolationMode::Hermite,
            1 => InterpolationMode::Linear,
            2 => InterpolationMode::Stepped,
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "InterpolationMode",
                    tag: other,
                })
            }
        };
        keys.push(Keyframe::with_mode(
            time,
            value,
            in_tangent,
            out_tangent,
            mode,
        ));
    }
    Ok(FloatCurve::new(keys))
}

fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

fn write_bool(out: &mut Vec<u8>, v: bool) {
    out.push(u8::from(v));
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_i64(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn write_u128(out: &mut Vec<u8>, v: u128) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], AnimationError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(AnimationError::UnexpectedEof)?;
        if end > self.bytes.len() {
            return Err(AnimationError::UnexpectedEof);
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, AnimationError> {
        let slice = self.read_exact(1)?;
        Ok(slice[0])
    }

    fn read_bool(&mut self, section: &'static str) -> Result<bool, AnimationError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(AnimationError::InvalidTag {
                section,
                tag: other,
            }),
        }
    }

    fn read_u32(&mut self) -> Result<u32, AnimationError> {
        let s = self.read_exact(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn peek_u32(&self) -> Result<u32, AnimationError> {
        let end = self
            .pos
            .checked_add(4)
            .ok_or(AnimationError::UnexpectedEof)?;
        if end > self.bytes.len() {
            return Err(AnimationError::UnexpectedEof);
        }
        let slice = &self.bytes[self.pos..end];
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn skip(&mut self, len: usize) {
        self.pos = self.pos.saturating_add(len).min(self.bytes.len());
    }

    fn read_i32(&mut self) -> Result<i32, AnimationError> {
        let s = self.read_exact(4)?;
        Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn read_i64(&mut self) -> Result<i64, AnimationError> {
        let s = self.read_exact(8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(s);
        Ok(i64::from_le_bytes(bytes))
    }
    fn read_u128(&mut self) -> Result<u128, AnimationError> {
        let s = self.read_exact(16)?;
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(s);
        Ok(u128::from_le_bytes(bytes))
    }

    fn read_f32(&mut self) -> Result<f32, AnimationError> {
        let s = self.read_exact(4)?;
        Ok(f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn read_str(&mut self) -> Result<String, AnimationError> {
        let len = self.read_u32()? as usize;
        let s = self.read_exact(len)?;
        std::str::from_utf8(s)
            .map(ToOwned::to_owned)
            .map_err(|_| AnimationError::InvalidUtf8)
    }
}
