pub mod avatar_mask;
pub mod clip;
pub mod controller;
pub mod curve;
pub mod error;
pub mod humanoid;
pub mod playable;
pub mod wire;

pub use clip::{
    AnimationClip, AnimationEvent, ClipSample, CurveBinding, CurveBindingTarget, SampledBinding,
    TransformChannel, TransformProperty, WrapMode,
};
pub use controller::{
    blend_sampled_bindings, AnimationDiagnostic, Animator, AnimatorController,
    AnimatorControllerLayer, AnimatorCullingMode, AnimatorFrame, AnimatorState, AnimatorStateInfo,
    AnimatorStepResult, AnimatorUpdateMode, BlendTree2DMode, ConditionOp, InterruptionSource,
    LayerBlendingMode, ParameterDef, ParameterType, ParameterValue, StateMachineCallback,
    StateMachineCallbackKind, StateMotion, StateTransition, TransitionCondition, TransitionTarget,
};
pub use curve::{
    AnimatedValue, AnimationCurve, FloatCurve, InterpolationMode, Keyframe, PropertyCurve,
};
pub use error::AnimationError;
pub use humanoid::{
    apply_humanoid_layer_with_mask, apply_humanoid_mask, apply_layered_humanoid_pose,
    blend_humanoid_layer, muscles_to_pose, pose_to_muscles, retarget, sample_humanoid_clip, Avatar,
    AvatarValidationIssue, BoneTransform, HumanBone, HumanBoneMapping, HumanPose, HumanPoseHandler,
    HumanoidAvatar, HumanoidBodyPart, HumanoidBone, HumanoidBoneMapping, HumanoidPose,
    MuscleChannel, MuscleId, MuscleLimit,
};
pub use wire::{decode_controller_binary, encode_controller_binary, WIRE_MAGIC, WIRE_VERSION};
