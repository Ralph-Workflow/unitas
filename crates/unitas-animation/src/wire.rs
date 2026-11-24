use crate::clip::{AnimationClip, AnimationEvent, CurveBinding, CurveBindingTarget, WrapMode};
use crate::controller::{
    AnimatorController, AnimatorState, ConditionOp, ParameterDef, ParameterValue, StateMotion,
    StateTransition, TransitionCondition, TransitionTarget,
};
use crate::curve::{FloatCurve, InterpolationMode, Keyframe, PropertyCurve};
use crate::error::AnimationError;

pub const WIRE_MAGIC: [u8; 4] = *b"UANC";
pub const WIRE_VERSION: u32 = 1;

pub fn encode_controller_binary(controller: &AnimatorController) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&WIRE_MAGIC);
    write_u32(&mut out, WIRE_VERSION);
    write_str(&mut out, &controller.name);

    write_u32(&mut out, controller.parameters.len() as u32);
    for param in &controller.parameters {
        write_str(&mut out, &param.name);
        match param.default_value {
            ParameterValue::Float(v) => {
                write_u8(&mut out, 0);
                write_f32(&mut out, v);
            }
            ParameterValue::Int(v) => {
                write_u8(&mut out, 1);
                write_i32(&mut out, v);
            }
            ParameterValue::Bool(v) => {
                write_u8(&mut out, 2);
                write_bool(&mut out, v);
            }
            ParameterValue::Trigger(v) => {
                write_u8(&mut out, 3);
                write_bool(&mut out, v);
            }
        }
    }

    write_u32(&mut out, controller.states.len() as u32);
    for state in &controller.states {
        write_str(&mut out, &state.name);
        write_f32(&mut out, state.speed);
        write_bool(&mut out, state.is_exit);
        match &state.motion {
            StateMotion::None => write_u8(&mut out, 0),
            StateMotion::Clip(clip) => {
                write_u8(&mut out, 1);
                write_clip(&mut out, clip);
            }
            StateMotion::BlendTree1D {
                parameter,
                thresholds,
            } => {
                write_u8(&mut out, 2);
                write_str(&mut out, parameter);
                write_u32(&mut out, thresholds.len() as u32);
                for (threshold, clip) in thresholds {
                    write_f32(&mut out, *threshold);
                    write_clip(&mut out, clip);
                }
            }
        }
    }

    write_u32(&mut out, controller.entry_state as u32);

    write_u32(&mut out, controller.transitions.len() as u32);
    for tr in &controller.transitions {
        write_u32(&mut out, tr.source_state as u32);
        match tr.target {
            TransitionTarget::State(idx) => {
                write_u8(&mut out, 0);
                write_u32(&mut out, idx as u32);
            }
            TransitionTarget::Exit => {
                write_u8(&mut out, 1);
                write_u32(&mut out, 0);
            }
        }
        write_bool(&mut out, tr.has_exit_time);
        write_f32(&mut out, tr.exit_time);
        write_f32(&mut out, tr.duration);
        write_u32(&mut out, tr.conditions.len() as u32);
        for cond in &tr.conditions {
            write_str(&mut out, &cond.parameter);
            match cond.op {
                ConditionOp::If => write_u8(&mut out, 0),
                ConditionOp::IfNot => write_u8(&mut out, 1),
                ConditionOp::Greater(v) => {
                    write_u8(&mut out, 2);
                    write_f32(&mut out, v);
                }
                ConditionOp::Less(v) => {
                    write_u8(&mut out, 3);
                    write_f32(&mut out, v);
                }
                ConditionOp::Equals(v) => {
                    write_u8(&mut out, 4);
                    write_i32(&mut out, v);
                }
                ConditionOp::NotEqual(v) => {
                    write_u8(&mut out, 5);
                    write_i32(&mut out, v);
                }
            }
        }
    }

    out
}

pub fn decode_controller_binary(bytes: &[u8]) -> Result<AnimatorController, AnimationError> {
    let mut cursor = Cursor::new(bytes);
    let magic = cursor.read_exact(4)?;
    if magic != WIRE_MAGIC {
        return Err(AnimationError::InvalidMagic);
    }
    let version = cursor.read_u32()?;
    if version != WIRE_VERSION {
        return Err(AnimationError::UnsupportedVersion(version));
    }
    let name = cursor.read_str()?;

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

    let state_count = cursor.read_u32()? as usize;
    let mut states = Vec::with_capacity(state_count.min(1024));
    for _ in 0..state_count {
        let state_name = cursor.read_str()?;
        let speed = cursor.read_f32()?;
        let is_exit = cursor.read_bool("AnimatorState.is_exit")?;
        let motion_tag = cursor.read_u8()?;
        let motion = match motion_tag {
            0 => StateMotion::None,
            1 => StateMotion::Clip(read_clip(&mut cursor)?),
            2 => {
                let parameter = cursor.read_str()?;
                let threshold_count = cursor.read_u32()? as usize;
                let mut thresholds = Vec::with_capacity(threshold_count.min(1024));
                for _ in 0..threshold_count {
                    let threshold = cursor.read_f32()?;
                    let clip = read_clip(&mut cursor)?;
                    thresholds.push((threshold, clip));
                }
                StateMotion::BlendTree1D {
                    parameter,
                    thresholds,
                }
            }
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "StateMotion",
                    tag: other,
                })
            }
        };
        states.push(AnimatorState {
            name: state_name,
            motion,
            speed,
            is_exit,
        });
    }

    let entry_state = cursor.read_u32()? as usize;

    let tr_count = cursor.read_u32()? as usize;
    let mut transitions = Vec::with_capacity(tr_count.min(1024));
    for _ in 0..tr_count {
        let source_state = cursor.read_u32()? as usize;
        let target_tag = cursor.read_u8()?;
        let target_idx = cursor.read_u32()? as usize;
        let target = match target_tag {
            0 => TransitionTarget::State(target_idx),
            1 => TransitionTarget::Exit,
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "TransitionTarget",
                    tag: other,
                })
            }
        };
        let has_exit_time = cursor.read_bool("StateTransition.has_exit_time")?;
        let exit_time = cursor.read_f32()?;
        let duration = cursor.read_f32()?;
        let cond_count = cursor.read_u32()? as usize;
        let mut conditions = Vec::with_capacity(cond_count.min(1024));
        for _ in 0..cond_count {
            let parameter = cursor.read_str()?;
            let op_tag = cursor.read_u8()?;
            let op = match op_tag {
                0 => ConditionOp::If,
                1 => ConditionOp::IfNot,
                2 => ConditionOp::Greater(cursor.read_f32()?),
                3 => ConditionOp::Less(cursor.read_f32()?),
                4 => ConditionOp::Equals(cursor.read_i32()?),
                5 => ConditionOp::NotEqual(cursor.read_i32()?),
                other => {
                    return Err(AnimationError::InvalidTag {
                        section: "ConditionOp",
                        tag: other,
                    })
                }
            };
            conditions.push(TransitionCondition::new(parameter, op));
        }
        transitions.push(StateTransition {
            source_state,
            target,
            conditions,
            has_exit_time,
            exit_time,
            duration,
        });
    }

    Ok(AnimatorController {
        name,
        parameters,
        states,
        entry_state,
        transitions,
    })
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
        }
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

fn read_clip(cursor: &mut Cursor<'_>) -> Result<AnimationClip, AnimationError> {
    let name = cursor.read_str()?;
    let duration = cursor.read_f32()?;
    let sample_rate = cursor.read_f32()?;
    let wrap_tag = cursor.read_u8()?;
    let wrap_mode = match wrap_tag {
        0 => WrapMode::Once,
        1 => WrapMode::Loop,
        2 => WrapMode::PingPong,
        3 => WrapMode::ClampForever,
        other => {
            return Err(AnimationError::InvalidTag {
                section: "WrapMode",
                tag: other,
            })
        }
    };

    let binding_count = cursor.read_u32()? as usize;
    let mut bindings = Vec::with_capacity(binding_count.min(1024));
    for _ in 0..binding_count {
        let object_path = cursor.read_str()?;
        let component = cursor.read_str()?;
        let property = cursor.read_str()?;
        let curve_tag = cursor.read_u8()?;
        let curve = match curve_tag {
            0 => PropertyCurve::Float(read_float_curve(cursor)?),
            1 => PropertyCurve::Vector3([
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
            ]),
            2 => PropertyCurve::Quaternion([
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
            ]),
            3 => PropertyCurve::Color([
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
                read_float_curve(cursor)?,
            ]),
            other => {
                return Err(AnimationError::InvalidTag {
                    section: "PropertyCurve",
                    tag: other,
                })
            }
        };
        bindings.push(CurveBinding::new(
            CurveBindingTarget::new(object_path, component, property),
            curve,
        ));
    }

    let event_count = cursor.read_u32()? as usize;
    let mut events = Vec::with_capacity(event_count.min(1024));
    for _ in 0..event_count {
        let time = cursor.read_f32()?;
        let function_name = cursor.read_str()?;
        let float_parameter = cursor.read_f32()?;
        let int_parameter = cursor.read_i32()?;
        let string_parameter = cursor.read_str()?;
        events.push(AnimationEvent {
            time,
            function_name,
            float_parameter,
            int_parameter,
            string_parameter,
        });
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

    fn read_i32(&mut self) -> Result<i32, AnimationError> {
        let s = self.read_exact(4)?;
        Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
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
