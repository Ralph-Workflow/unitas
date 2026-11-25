use unitas_core::{multiply_rotation, normalize_rotation, Transform, IDENTITY_ROTATION};

use crate::avatar_mask::AvatarMask;
use crate::clip::{AnimationClip, CurveBindingTarget, SampledBinding};
use crate::curve::AnimatedValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HumanoidBodyPart {
    Root,
    Body,
    Head,
    LeftLeg,
    RightLeg,
    LeftArm,
    RightArm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HumanBone {
    Hips,
    Spine,
    Chest,
    Neck,
    Head,
    LeftShoulder,
    LeftUpperArm,
    LeftLowerArm,
    LeftHand,
    RightShoulder,
    RightUpperArm,
    RightLowerArm,
    RightHand,
    LeftUpperLeg,
    LeftLowerLeg,
    LeftFoot,
    RightUpperLeg,
    RightLowerLeg,
    RightFoot,
}

pub type HumanoidBone = HumanBone;

impl HumanBone {
    pub const REQUIRED: [Self; 7] = [
        Self::Hips,
        Self::Spine,
        Self::Head,
        Self::LeftUpperArm,
        Self::RightUpperArm,
        Self::LeftUpperLeg,
        Self::RightUpperLeg,
    ];

    pub const fn body_part(self) -> HumanoidBodyPart {
        match self {
            Self::Hips => HumanoidBodyPart::Root,
            Self::Spine | Self::Chest => HumanoidBodyPart::Body,
            Self::Neck | Self::Head => HumanoidBodyPart::Head,
            Self::LeftShoulder | Self::LeftUpperArm | Self::LeftLowerArm | Self::LeftHand => {
                HumanoidBodyPart::LeftArm
            }
            Self::RightShoulder | Self::RightUpperArm | Self::RightLowerArm | Self::RightHand => {
                HumanoidBodyPart::RightArm
            }
            Self::LeftUpperLeg | Self::LeftLowerLeg | Self::LeftFoot => HumanoidBodyPart::LeftLeg,
            Self::RightUpperLeg | Self::RightLowerLeg | Self::RightFoot => {
                HumanoidBodyPart::RightLeg
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MuscleChannel {
    SpineFrontBack,
    SpineLeftRight,
    SpineTwistLeftRight,
    NeckNodDownUp,
    NeckTurnLeftRight,
    LeftArmDownUp,
    LeftArmFrontBack,
    LeftForearmStretch,
    RightArmDownUp,
    RightArmFrontBack,
    RightForearmStretch,
    LeftLegFrontBack,
    LeftLegInOut,
    LeftKneeStretch,
    RightLegFrontBack,
    RightLegInOut,
    RightKneeStretch,
}

pub type MuscleId = MuscleChannel;

impl MuscleChannel {
    pub const COUNT: usize = 17;

    pub const ALL: [Self; Self::COUNT] = [
        Self::SpineFrontBack,
        Self::SpineLeftRight,
        Self::SpineTwistLeftRight,
        Self::NeckNodDownUp,
        Self::NeckTurnLeftRight,
        Self::LeftArmDownUp,
        Self::LeftArmFrontBack,
        Self::LeftForearmStretch,
        Self::RightArmDownUp,
        Self::RightArmFrontBack,
        Self::RightForearmStretch,
        Self::LeftLegFrontBack,
        Self::LeftLegInOut,
        Self::LeftKneeStretch,
        Self::RightLegFrontBack,
        Self::RightLegInOut,
        Self::RightKneeStretch,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::SpineFrontBack => 0,
            Self::SpineLeftRight => 1,
            Self::SpineTwistLeftRight => 2,
            Self::NeckNodDownUp => 3,
            Self::NeckTurnLeftRight => 4,
            Self::LeftArmDownUp => 5,
            Self::LeftArmFrontBack => 6,
            Self::LeftForearmStretch => 7,
            Self::RightArmDownUp => 8,
            Self::RightArmFrontBack => 9,
            Self::RightForearmStretch => 10,
            Self::LeftLegFrontBack => 11,
            Self::LeftLegInOut => 12,
            Self::LeftKneeStretch => 13,
            Self::RightLegFrontBack => 14,
            Self::RightLegInOut => 15,
            Self::RightKneeStretch => 16,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::SpineFrontBack => "SpineFrontBack",
            Self::SpineLeftRight => "SpineLeftRight",
            Self::SpineTwistLeftRight => "SpineTwistLeftRight",
            Self::NeckNodDownUp => "NeckNodDownUp",
            Self::NeckTurnLeftRight => "NeckTurnLeftRight",
            Self::LeftArmDownUp => "LeftArmDownUp",
            Self::LeftArmFrontBack => "LeftArmFrontBack",
            Self::LeftForearmStretch => "LeftForearmStretch",
            Self::RightArmDownUp => "RightArmDownUp",
            Self::RightArmFrontBack => "RightArmFrontBack",
            Self::RightForearmStretch => "RightForearmStretch",
            Self::LeftLegFrontBack => "LeftLegFrontBack",
            Self::LeftLegInOut => "LeftLegInOut",
            Self::LeftKneeStretch => "LeftKneeStretch",
            Self::RightLegFrontBack => "RightLegFrontBack",
            Self::RightLegInOut => "RightLegInOut",
            Self::RightKneeStretch => "RightKneeStretch",
        }
    }

    pub fn from_property_name(property: &str) -> Option<Self> {
        let stripped = property.strip_prefix("muscle.").unwrap_or(property);
        Self::ALL.into_iter().find(|ch| ch.name() == stripped)
    }

    pub const fn target_bone(self) -> HumanBone {
        match self {
            Self::SpineFrontBack | Self::SpineLeftRight | Self::SpineTwistLeftRight => {
                HumanBone::Spine
            }
            Self::NeckNodDownUp | Self::NeckTurnLeftRight => HumanBone::Head,
            Self::LeftArmDownUp | Self::LeftArmFrontBack => HumanBone::LeftUpperArm,
            Self::LeftForearmStretch => HumanBone::LeftLowerArm,
            Self::RightArmDownUp | Self::RightArmFrontBack => HumanBone::RightUpperArm,
            Self::RightForearmStretch => HumanBone::RightLowerArm,
            Self::LeftLegFrontBack | Self::LeftLegInOut => HumanBone::LeftUpperLeg,
            Self::LeftKneeStretch => SelfBoneMapping::LEFT_KNEE_BONE,
            Self::RightLegFrontBack | Self::RightLegInOut => HumanBone::RightUpperLeg,
            Self::RightKneeStretch => SelfBoneMapping::RIGHT_KNEE_BONE,
        }
    }

    pub const fn body_part(self) -> HumanoidBodyPart {
        self.target_bone().body_part()
    }

    pub const fn default_limit(self) -> MuscleLimit {
        match self {
            Self::SpineFrontBack | Self::SpineLeftRight | Self::SpineTwistLeftRight => {
                MuscleLimit {
                    min_degrees: -40.0,
                    max_degrees: 40.0,
                    min: -1.0,
                    max: 1.0,
                }
            }
            Self::NeckNodDownUp | Self::NeckTurnLeftRight => MuscleLimit {
                min_degrees: -45.0,
                max_degrees: 45.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftArmDownUp | Self::RightArmDownUp => MuscleLimit {
                min_degrees: -60.0,
                max_degrees: 100.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftArmFrontBack | Self::RightArmFrontBack => MuscleLimit {
                min_degrees: -70.0,
                max_degrees: 70.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftForearmStretch | Self::RightForearmStretch => MuscleLimit {
                min_degrees: -80.0,
                max_degrees: 80.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftLegFrontBack | Self::RightLegFrontBack => MuscleLimit {
                min_degrees: -90.0,
                max_degrees: 50.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftLegInOut | Self::RightLegInOut => MuscleLimit {
                min_degrees: -60.0,
                max_degrees: 60.0,
                min: -1.0,
                max: 1.0,
            },
            Self::LeftKneeStretch | Self::RightKneeStretch => MuscleLimit {
                min_degrees: -80.0,
                max_degrees: 80.0,
                min: -1.0,
                max: 1.0,
            },
        }
    }

    pub const fn axis(self) -> [f32; 3] {
        match self {
            Self::SpineFrontBack
            | Self::NeckNodDownUp
            | Self::LeftArmFrontBack
            | Self::RightArmFrontBack
            | Self::LeftLegFrontBack
            | Self::RightLegFrontBack
            | Self::LeftKneeStretch
            | Self::RightKneeStretch => [1.0, 0.0, 0.0],
            Self::SpineTwistLeftRight | Self::NeckTurnLeftRight => [0.0, 1.0, 0.0],
            Self::SpineLeftRight
            | Self::LeftArmDownUp
            | Self::RightArmDownUp
            | Self::LeftForearmStretch
            | Self::RightForearmStretch
            | Self::LeftLegInOut
            | Self::RightLegInOut => [0.0, 0.0, 1.0],
        }
    }
}

struct SelfBoneMapping;

impl SelfBoneMapping {
    const LEFT_KNEE_BONE: HumanBone = HumanBone::LeftLowerLeg;
    const RIGHT_KNEE_BONE: HumanBone = HumanBone::RightLowerLeg;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MuscleLimit {
    pub min_degrees: f32,
    pub max_degrees: f32,
    pub min: f32,
    pub max: f32,
}

impl MuscleLimit {
    pub fn new(min_degrees: f32, max_degrees: f32) -> Self {
        Self::with_normalized_bounds(min_degrees, max_degrees, -1.0, 1.0)
    }

    pub fn with_normalized_bounds(
        min_degrees: f32,
        max_degrees: f32,
        min_normalized: f32,
        max_normalized: f32,
    ) -> Self {
        let min_clean = if min_degrees.is_finite() {
            min_degrees
        } else {
            -45.0
        };
        let max_clean = if max_degrees.is_finite() {
            max_degrees
        } else {
            45.0
        };
        let (deg_lo, deg_hi) = if min_clean <= max_clean {
            (min_clean, max_clean)
        } else {
            (max_clean, min_clean)
        };
        let norm_a = Self::clamp_normalized(min_normalized);
        let norm_b = Self::clamp_normalized(max_normalized);
        let (norm_lo, norm_hi) = if norm_a <= norm_b {
            (norm_a, norm_b)
        } else {
            (norm_b, norm_a)
        };
        Self {
            min_degrees: deg_lo,
            max_degrees: deg_hi,
            min: norm_lo,
            max: norm_hi,
        }
    }

    pub const fn min(self) -> f32 {
        self.min
    }

    pub const fn max(self) -> f32 {
        self.max
    }

    pub fn clamp(self, value: f32) -> f32 {
        Self::clamp_normalized(value).clamp(self.min, self.max)
    }

    pub fn clamp_normalized(value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn evaluate_degrees(self, normalized: f32) -> f32 {
        let clamped = self.clamp(normalized);
        if clamped >= 0.0 {
            clamped * self.max_degrees
        } else {
            (-clamped) * self.min_degrees
        }
    }

    pub fn normalize_degrees(self, degrees: f32) -> f32 {
        if !degrees.is_finite() {
            return 0.0;
        }
        if degrees >= 0.0 {
            if self.max_degrees.abs() > 1e-6 {
                self.clamp(degrees / self.max_degrees)
            } else {
                0.0
            }
        } else if self.min_degrees.abs() > 1e-6 {
            self.clamp(-(degrees / self.min_degrees))
        } else {
            0.0
        }
    }
}

pub type HumanoidBoneMapping = HumanBoneMapping;

#[derive(Clone, Debug, PartialEq)]
pub struct HumanBoneMapping {
    pub bone: HumanBone,
    pub transform_path: String,
    pub bind_translation: [f32; 3],
    pub bind_rotation: [f32; 4],
}

impl HumanBoneMapping {
    pub fn new(bone: HumanBone, transform_path: impl Into<String>) -> Self {
        Self {
            bone,
            transform_path: transform_path.into(),
            bind_translation: [0.0, 0.0, 0.0],
            bind_rotation: IDENTITY_ROTATION,
        }
    }

    pub fn with_bind_translation(mut self, bind_translation: [f32; 3]) -> Self {
        self.bind_translation = sanitize_vec3(bind_translation, [0.0, 0.0, 0.0]);
        self
    }

    pub fn with_bind_rotation(mut self, bind_rotation: [f32; 4]) -> Self {
        self.bind_rotation = normalize_rotation(bind_rotation);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarValidationIssue {
    MissingRequiredBone(HumanBone),
    EmptyTransformPath(HumanBone),
    DuplicateTransformPath {
        path: String,
        first: HumanBone,
        second: HumanBone,
    },
}

pub type Avatar = HumanoidAvatar;

#[derive(Clone, Debug, PartialEq)]
pub struct HumanoidAvatar {
    name: String,
    bones: Vec<HumanBoneMapping>,
    muscle_limits: [MuscleLimit; MuscleChannel::COUNT],
}

impl HumanoidAvatar {
    pub fn new(name: impl Into<String>) -> Self {
        let muscle_limits = MuscleChannel::ALL.map(MuscleChannel::default_limit);
        Self {
            name: name.into(),
            bones: Vec::new(),
            muscle_limits,
        }
    }

    pub fn with_bone(mut self, mapping: HumanBoneMapping) -> Self {
        if let Some(existing) = self.bones.iter_mut().find(|b| b.bone == mapping.bone) {
            *existing = mapping;
        } else {
            self.bones.push(mapping);
        }
        self
    }

    pub fn with_muscle_limit(mut self, channel: MuscleChannel, limit: MuscleLimit) -> Self {
        self.muscle_limits[channel.index()] = limit;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bones(&self) -> &[HumanBoneMapping] {
        &self.bones
    }

    pub fn bone_mapping(&self, bone: HumanBone) -> Option<&HumanBoneMapping> {
        self.bones.iter().find(|b| b.bone == bone)
    }

    pub fn muscle_limit(&self, channel: MuscleChannel) -> MuscleLimit {
        self.muscle_limits[channel.index()]
    }

    pub fn hips_scale(&self) -> f32 {
        self.bone_mapping(HumanBone::Hips)
            .map(|hips| {
                let y = hips.bind_translation[1].abs();
                if y > 1e-5 {
                    y
                } else {
                    1.0
                }
            })
            .unwrap_or(1.0)
    }

    pub fn validate(&self) -> Vec<AvatarValidationIssue> {
        let mut issues = Vec::new();
        for required in HumanBone::REQUIRED {
            if self.bone_mapping(required).is_none() {
                issues.push(AvatarValidationIssue::MissingRequiredBone(required));
            }
        }
        for (idx, mapping) in self.bones.iter().enumerate() {
            if mapping.transform_path.trim().is_empty() {
                issues.push(AvatarValidationIssue::EmptyTransformPath(mapping.bone));
                continue;
            }
            for earlier in &self.bones[..idx] {
                if earlier.transform_path == mapping.transform_path {
                    issues.push(AvatarValidationIssue::DuplicateTransformPath {
                        path: mapping.transform_path.clone(),
                        first: earlier.bone,
                        second: mapping.bone,
                    });
                    break;
                }
            }
        }
        issues
    }
}

pub type HumanoidPose = HumanPose;

#[derive(Clone, Debug, PartialEq)]
pub struct HumanPose {
    pub body_position: [f32; 3],
    pub body_rotation: [f32; 4],
    muscles: [f32; MuscleChannel::COUNT],
}

impl Default for HumanPose {
    fn default() -> Self {
        Self {
            body_position: [0.0, 1.0, 0.0],
            body_rotation: IDENTITY_ROTATION,
            muscles: [0.0; MuscleChannel::COUNT],
        }
    }
}

impl HumanPose {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn muscle(&self, channel: MuscleChannel) -> f32 {
        self.muscles[channel.index()]
    }

    pub fn set_muscle(&mut self, channel: MuscleChannel, normalized_value: f32) {
        self.muscles[channel.index()] = MuscleLimit::clamp_normalized(normalized_value);
    }

    pub fn muscles(&self) -> &[f32; MuscleChannel::COUNT] {
        &self.muscles
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoneTransform {
    pub bone: HumanBone,
    pub transform_path: String,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HumanPoseHandler {
    avatar: HumanoidAvatar,
    pose: HumanPose,
    bone_transforms: Vec<BoneTransform>,
}

impl HumanPoseHandler {
    pub fn new(avatar: HumanoidAvatar) -> Self {
        let pose = HumanPose::default();
        let mut handler = Self {
            avatar,
            pose,
            bone_transforms: Vec::new(),
        };
        handler.recompute_bone_transforms();
        handler
    }

    pub fn avatar(&self) -> &HumanoidAvatar {
        &self.avatar
    }

    pub fn human_pose(&self) -> &HumanPose {
        &self.pose
    }

    pub fn set_human_pose(&mut self, pose: &HumanPose) {
        let mut sanitized = HumanPose {
            body_position: sanitize_vec3(pose.body_position, [0.0, 1.0, 0.0]),
            body_rotation: normalize_rotation(pose.body_rotation),
            muscles: [0.0; MuscleChannel::COUNT],
        };
        for channel in MuscleChannel::ALL {
            let clamped = self
                .avatar
                .muscle_limit(channel)
                .clamp(pose.muscle(channel));
            sanitized.set_muscle(channel, clamped);
        }
        self.pose = sanitized;
        self.recompute_bone_transforms();
    }

    pub fn sample_clip(&mut self, clip: &AnimationClip, time: f32) {
        let sample = clip.sample(0.0, time);
        self.apply_sampled_bindings(&sample.bindings);
    }

    pub fn apply_sampled_bindings(&mut self, bindings: &[SampledBinding]) {
        let mut next_pose = self.pose.clone();
        for binding in bindings {
            if let Some(channel) = MuscleChannel::from_property_name(&binding.target.property) {
                if let AnimatedValue::Float(v) = binding.value {
                    next_pose.set_muscle(channel, v);
                }
                continue;
            }
            match (binding.target.property.as_str(), binding.value) {
                ("RootT" | "bodyPosition", AnimatedValue::Vector3(v)) => {
                    next_pose.body_position = sanitize_vec3(v, next_pose.body_position);
                }
                ("RootT.x", AnimatedValue::Float(x)) if x.is_finite() => {
                    next_pose.body_position[0] = x;
                }
                ("RootT.y", AnimatedValue::Float(y)) if y.is_finite() => {
                    next_pose.body_position[1] = y;
                }
                ("RootT.z", AnimatedValue::Float(z)) if z.is_finite() => {
                    next_pose.body_position[2] = z;
                }
                ("RootQ" | "bodyRotation", AnimatedValue::Quaternion(q)) => {
                    next_pose.body_rotation = normalize_rotation(q);
                }
                _ => {}
            }
        }
        self.set_human_pose(&next_pose);
    }

    pub fn evaluated_muscle_degrees(&self, channel: MuscleChannel) -> f32 {
        let limit = self.avatar.muscle_limit(channel);
        limit.evaluate_degrees(self.pose.muscle(channel))
    }

    pub fn bone_transforms(&self) -> &[BoneTransform] {
        &self.bone_transforms
    }

    pub fn to_sampled_bindings(&self) -> Vec<SampledBinding> {
        let mut out = Vec::with_capacity(self.bone_transforms.len() * 2);
        for bone in &self.bone_transforms {
            out.push(SampledBinding {
                target: CurveBindingTarget::transform(bone.transform_path.clone(), "localPosition"),
                value: AnimatedValue::Vector3(bone.translation),
            });
            out.push(SampledBinding {
                target: CurveBindingTarget::transform(bone.transform_path.clone(), "localRotation"),
                value: AnimatedValue::Quaternion(bone.rotation),
            });
        }
        out
    }

    fn recompute_bone_transforms(&mut self) {
        let hips_scale = self.avatar.hips_scale();
        let mut transforms = Vec::with_capacity(self.avatar.bones.len());
        for mapping in &self.avatar.bones {
            let translation = if mapping.bone == HumanBone::Hips {
                [
                    self.pose.body_position[0] * hips_scale,
                    self.pose.body_position[1] * hips_scale,
                    self.pose.body_position[2] * hips_scale,
                ]
            } else {
                mapping.bind_translation
            };
            let mut rotation = if mapping.bone == HumanBone::Hips {
                multiply_rotation(mapping.bind_rotation, self.pose.body_rotation)
            } else {
                mapping.bind_rotation
            };
            for channel in MuscleChannel::ALL {
                if channel.target_bone() == mapping.bone {
                    let degrees = self.evaluated_muscle_degrees(channel);
                    let delta = axis_angle_degrees(channel.axis(), degrees);
                    rotation = multiply_rotation(rotation, delta);
                }
            }
            transforms.push(BoneTransform {
                bone: mapping.bone,
                transform_path: mapping.transform_path.clone(),
                translation,
                rotation: normalize_rotation(rotation),
            });
        }
        self.bone_transforms = transforms;
    }
}

pub fn apply_humanoid_layer_with_mask(
    avatar: &HumanoidAvatar,
    base: &HumanPose,
    overlay: &HumanPose,
    weight: f32,
    mask: &AvatarMask,
    allowed_body_parts: &[HumanoidBodyPart],
) -> HumanPose {
    let w = if weight.is_finite() {
        weight.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut result = base.clone();
    let root_allowed = body_part_and_path_allowed(
        avatar,
        HumanBone::Hips,
        HumanoidBodyPart::Root,
        mask,
        allowed_body_parts,
    );
    if root_allowed {
        result.body_position = [
            lerp(base.body_position[0], overlay.body_position[0], w),
            lerp(base.body_position[1], overlay.body_position[1], w),
            lerp(base.body_position[2], overlay.body_position[2], w),
        ];
        if let AnimatedValue::Quaternion(q) = AnimatedValue::Quaternion(base.body_rotation)
            .blend(&AnimatedValue::Quaternion(overlay.body_rotation), w)
        {
            result.body_rotation = q;
        }
    }
    for channel in MuscleChannel::ALL {
        let bone = channel.target_bone();
        let part = channel.body_part();
        if body_part_and_path_allowed(avatar, bone, part, mask, allowed_body_parts) {
            let blended = lerp(base.muscle(channel), overlay.muscle(channel), w);
            result.set_muscle(channel, blended);
        }
    }
    result
}

pub fn apply_layered_humanoid_pose(
    avatar: &HumanoidAvatar,
    base: &HumanPose,
    overlay: &HumanPose,
    weight: f32,
    mask: &AvatarMask,
) -> HumanPose {
    apply_humanoid_layer_with_mask(avatar, base, overlay, weight, mask, &[])
}

pub fn blend_humanoid_layer(
    avatar: &HumanoidAvatar,
    base: &HumanPose,
    overlay: &HumanPose,
    weight: f32,
    mask: &AvatarMask,
) -> HumanPose {
    apply_humanoid_layer_with_mask(avatar, base, overlay, weight, mask, &[])
}

pub fn apply_humanoid_mask(
    avatar: &HumanoidAvatar,
    base: &HumanPose,
    overlay: &HumanPose,
    weight: f32,
    mask: &AvatarMask,
) -> HumanPose {
    apply_humanoid_layer_with_mask(avatar, base, overlay, weight, mask, &[])
}

pub fn sample_humanoid_clip(clip: &AnimationClip, time: f32) -> HumanPose {
    let mut handler = HumanPoseHandler::new(HumanoidAvatar::new("ClipSampler"));
    handler.sample_clip(clip, time);
    handler.human_pose().clone()
}

pub fn muscles_to_pose(avatar: &HumanoidAvatar, pose: &HumanPose) -> Vec<(String, Transform)> {
    let mut handler = HumanPoseHandler::new(avatar.clone());
    handler.set_human_pose(pose);
    handler
        .bone_transforms()
        .iter()
        .map(|bt| {
            let mut tr = Transform::new();
            tr.translation = bt.translation;
            tr.rotation = bt.rotation;
            (bt.transform_path.clone(), tr)
        })
        .collect()
}

pub fn pose_to_muscles(
    avatar: &HumanoidAvatar,
    local_transforms: &[(String, Transform)],
) -> HumanPose {
    let mut pose = HumanPose::default();
    let hips_scale = avatar.hips_scale();
    for mapping in &avatar.bones {
        let Some((_, tr)) = local_transforms
            .iter()
            .find(|(path, _)| path == &mapping.transform_path)
        else {
            continue;
        };
        if mapping.bone == HumanBone::Hips {
            pose.body_position = [
                tr.translation[0] / hips_scale,
                tr.translation[1] / hips_scale,
                tr.translation[2] / hips_scale,
            ];
            let inv_bind = conjugate_rotation(mapping.bind_rotation);
            pose.body_rotation = normalize_rotation(multiply_rotation(inv_bind, tr.rotation));
            continue;
        }
        let inv_bind = conjugate_rotation(mapping.bind_rotation);
        let rel_rot = normalize_rotation(multiply_rotation(inv_bind, tr.rotation));
        for channel in MuscleChannel::ALL {
            if channel.target_bone() == mapping.bone {
                let degrees = extract_twist_degrees(rel_rot, channel.axis());
                let limit = avatar.muscle_limit(channel);
                pose.set_muscle(channel, limit.normalize_degrees(degrees));
            }
        }
    }
    pose
}

pub fn retarget(
    clip_pose: &HumanPose,
    _source_avatar: &HumanoidAvatar,
    target_avatar: &HumanoidAvatar,
) -> HumanPose {
    let mut handler = HumanPoseHandler::new(target_avatar.clone());
    handler.set_human_pose(clip_pose);
    handler.human_pose().clone()
}

fn conjugate_rotation(q: [f32; 4]) -> [f32; 4] {
    let n = normalize_rotation(q);
    [-n[0], -n[1], -n[2], n[3]]
}

fn extract_twist_degrees(q: [f32; 4], axis: [f32; 3]) -> f32 {
    let n = normalize_rotation(q);
    let dot = n[0] * axis[0] + n[1] * axis[1] + n[2] * axis[2];
    let proj = [axis[0] * dot, axis[1] * dot, axis[2] * dot];
    let proj_len = (proj[0] * proj[0] + proj[1] * proj[1] + proj[2] * proj[2]).sqrt();
    let signed_sin = if dot >= 0.0 { proj_len } else { -proj_len };
    let half_angle = signed_sin.atan2(n[3]);
    (half_angle * 2.0).to_degrees()
}

fn body_part_and_path_allowed(
    avatar: &HumanoidAvatar,
    bone: HumanBone,
    part: HumanoidBodyPart,
    mask: &AvatarMask,
    allowed_body_parts: &[HumanoidBodyPart],
) -> bool {
    let part_ok = allowed_body_parts.is_empty() || allowed_body_parts.contains(&part);
    if !part_ok {
        return false;
    }
    if mask.is_empty() {
        return true;
    }
    avatar
        .bone_mapping(bone)
        .is_some_and(|mapping| mask.includes_path(&mapping.transform_path))
}

fn axis_angle_degrees(axis: [f32; 3], degrees: f32) -> [f32; 4] {
    if !degrees.is_finite() {
        return IDENTITY_ROTATION;
    }
    let half_rad = degrees.to_radians() * 0.5;
    let (s, c) = half_rad.sin_cos();
    normalize_rotation([axis[0] * s, axis[1] * s, axis[2] * s, c])
}

fn sanitize_vec3(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    if v.iter().all(|c| c.is_finite()) {
        v
    } else {
        fallback
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
