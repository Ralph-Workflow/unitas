pub mod avatar_mask;
pub mod clip;
pub mod controller;
pub mod curve;
pub mod error;
pub mod playable;
pub mod wire;

pub use clip::{
    AnimationClip, AnimationEvent, ClipSample, CurveBinding, CurveBindingTarget, SampledBinding,
    TransformChannel, TransformProperty, WrapMode,
};
pub use controller::{
    AnimationDiagnostic, Animator, AnimatorController, AnimatorState, AnimatorStateInfo,
    AnimatorStepResult, ConditionOp, ParameterDef, ParameterType, ParameterValue, StateMotion,
    StateTransition, TransitionCondition, TransitionTarget,
};
pub use curve::{
    AnimatedValue, AnimationCurve, FloatCurve, InterpolationMode, Keyframe, PropertyCurve,
};
pub use error::AnimationError;
pub use wire::{decode_controller_binary, encode_controller_binary, WIRE_MAGIC, WIRE_VERSION};
