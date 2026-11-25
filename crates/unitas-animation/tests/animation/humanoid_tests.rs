use std::error::Error;

use unitas_animation::avatar_mask::AvatarMask;
use unitas_animation::humanoid::{
    apply_humanoid_layer_with_mask, apply_humanoid_mask, apply_layered_humanoid_pose,
    blend_humanoid_layer, muscles_to_pose, pose_to_muscles, retarget, sample_humanoid_clip, Avatar,
    AvatarValidationIssue, BoneTransform, HumanBone, HumanBoneMapping, HumanPose, HumanPoseHandler,
    HumanoidAvatar, HumanoidBodyPart, HumanoidBone, HumanoidBoneMapping, HumanoidPose,
    MuscleChannel, MuscleId, MuscleLimit,
};
use unitas_animation::{
    AnimatedValue, AnimationClip, CurveBinding, CurveBindingTarget, FloatCurve, Keyframe, WrapMode,
};
use unitas_core::Transform;

fn build_proportioned_avatar(
    name: &str,
    height_scale: f32,
    spine_max_degrees: f32,
    shoulder_max_degrees: f32,
) -> HumanoidAvatar {
    HumanoidAvatar::new(name)
        .with_bone(
            HumanBoneMapping::new(HumanBone::Hips, "Root/Hips").with_bind_translation([
                0.0,
                1.0 * height_scale,
                0.0,
            ]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::Spine, "Root/Hips/Spine").with_bind_translation([
                0.0,
                0.45 * height_scale,
                0.0,
            ]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::Head, "Root/Hips/Spine/Head").with_bind_translation([
                0.0,
                0.35 * height_scale,
                0.0,
            ]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::LeftUpperArm, "Root/Hips/Spine/LeftArm")
                .with_bind_translation([-0.25 * height_scale, 0.3 * height_scale, 0.0]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::RightUpperArm, "Root/Hips/Spine/RightArm")
                .with_bind_translation([0.25 * height_scale, 0.3 * height_scale, 0.0]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::LeftUpperLeg, "Root/Hips/LeftLeg")
                .with_bind_translation([-0.15 * height_scale, -0.5 * height_scale, 0.0]),
        )
        .with_bone(
            HumanBoneMapping::new(HumanBone::RightUpperLeg, "Root/Hips/RightLeg")
                .with_bind_translation([0.15 * height_scale, -0.5 * height_scale, 0.0]),
        )
        .with_muscle_limit(
            MuscleChannel::SpineFrontBack,
            MuscleLimit::new(-30.0, spine_max_degrees),
        )
        .with_muscle_limit(
            MuscleChannel::LeftArmDownUp,
            MuscleLimit::new(-60.0, shoulder_max_degrees),
        )
        .with_muscle_limit(
            MuscleChannel::LeftLegFrontBack,
            MuscleLimit::new(-90.0, 90.0),
        )
}

fn build_humanoid_clip() -> AnimationClip {
    AnimationClip::new(
        "HumanoidWaveAndStride",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Animator", "SpineFrontBack"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 0.5)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Animator", "LeftArmDownUp"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 0.8)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Animator", "LeftLegFrontBack"),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, -0.5),
                    Keyframe::linear(1.0, 0.5),
                ]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("", "Animator", "RootT.y"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 1.0), Keyframe::linear(1.0, 1.2)]),
            ),
        ],
        Vec::new(),
    )
}

#[test]
fn humanoid_clip_sampled_onto_two_differently_proportioned_avatars_yields_equal_muscles_and_proportioned_transforms(
) -> Result<(), Box<dyn Error>> {
    let short_avatar = build_proportioned_avatar("ShortAvatar", 1.0, 40.0, 100.0);
    let tall_avatar = build_proportioned_avatar("TallAvatar", 1.8, 40.0, 100.0);
    assert!(short_avatar.validate().is_empty());
    assert!(tall_avatar.validate().is_empty());

    let clip = build_humanoid_clip();
    let mut short_handler = HumanPoseHandler::new(short_avatar);
    let mut tall_handler = HumanPoseHandler::new(tall_avatar);

    short_handler.sample_clip(&clip, 0.5);
    tall_handler.sample_clip(&clip, 0.5);

    let short_pose = short_handler.human_pose();
    let tall_pose = tall_handler.human_pose();

    for channel in MuscleChannel::ALL {
        let short_muscle = short_pose.muscle(channel);
        let tall_muscle = tall_pose.muscle(channel);
        assert!(
            (short_muscle - tall_muscle).abs() < 1e-6,
            "muscle {channel:?} differed: {short_muscle} vs {tall_muscle}"
        );
    }

    assert!((short_pose.muscle(MuscleChannel::SpineFrontBack) - 0.25).abs() < 1e-5);
    assert!((short_pose.muscle(MuscleChannel::LeftArmDownUp) - 0.4).abs() < 1e-5);
    assert!(short_pose.muscle(MuscleChannel::LeftLegFrontBack).abs() < 1e-5);

    let short_bones = short_handler.bone_transforms();
    let tall_bones = tall_handler.bone_transforms();

    let short_spine: &BoneTransform = short_bones
        .iter()
        .find(|b| b.bone == HumanBone::Spine)
        .ok_or("missing short spine")?;
    let tall_spine: &BoneTransform = tall_bones
        .iter()
        .find(|b| b.bone == HumanBone::Spine)
        .ok_or("missing tall spine")?;

    assert_eq!(short_spine.rotation, tall_spine.rotation);
    assert!((short_spine.translation[1] - 0.45).abs() < 1e-5);
    assert!((tall_spine.translation[1] - 0.45 * 1.8).abs() < 1e-5);

    let short_hips = short_bones
        .iter()
        .find(|b| b.bone == HumanBone::Hips)
        .ok_or("missing short hips")?;
    let tall_hips = tall_bones
        .iter()
        .find(|b| b.bone == HumanBone::Hips)
        .ok_or("missing tall hips")?;
    assert!((short_hips.translation[1] - 1.1).abs() < 1e-5);
    assert!((tall_hips.translation[1] - 1.1 * 1.8).abs() < 1e-5);

    let mut roundtrip_handler =
        HumanPoseHandler::new(build_proportioned_avatar("Roundtrip", 1.4, 40.0, 100.0));
    roundtrip_handler.set_human_pose(short_pose);
    for channel in MuscleChannel::ALL {
        assert!(
            (roundtrip_handler.human_pose().muscle(channel) - short_pose.muscle(channel)).abs()
                < 1e-5
        );
    }
    Ok(())
}

#[test]
fn humanoid_retargeting_clamps_out_of_range_muscle_values_and_honors_per_avatar_degree_limits(
) -> Result<(), Box<dyn Error>> {
    let flexible_avatar = build_proportioned_avatar("Flexible", 1.0, 60.0, 120.0);
    let stiff_avatar = build_proportioned_avatar("Stiff", 1.0, 20.0, 45.0);

    let mut pose = HumanPose::default();
    pose.set_muscle(MuscleChannel::SpineFrontBack, 1.75);
    pose.set_muscle(MuscleChannel::LeftArmDownUp, -2.5);

    assert!((pose.muscle(MuscleChannel::SpineFrontBack) - 1.0).abs() < 1e-6);
    assert!((pose.muscle(MuscleChannel::LeftArmDownUp) - (-1.0)).abs() < 1e-6);

    let mut flex_handler = HumanPoseHandler::new(flexible_avatar);
    let mut stiff_handler = HumanPoseHandler::new(stiff_avatar);
    flex_handler.set_human_pose(&pose);
    stiff_handler.set_human_pose(&pose);

    assert!(
        (flex_handler
            .human_pose()
            .muscle(MuscleChannel::SpineFrontBack)
            - 1.0)
            .abs()
            < 1e-6
    );
    assert!(
        (stiff_handler
            .human_pose()
            .muscle(MuscleChannel::SpineFrontBack)
            - 1.0)
            .abs()
            < 1e-6
    );

    let flex_deg = flex_handler.evaluated_muscle_degrees(MuscleChannel::SpineFrontBack);
    let stiff_deg = stiff_handler.evaluated_muscle_degrees(MuscleChannel::SpineFrontBack);
    assert!((flex_deg - 60.0).abs() < 1e-4);
    assert!((stiff_deg - 20.0).abs() < 1e-4);

    let flex_spine = flex_handler
        .bone_transforms()
        .iter()
        .find(|b| b.bone == HumanBone::Spine)
        .ok_or("missing flex spine")?;
    let stiff_spine = stiff_handler
        .bone_transforms()
        .iter()
        .find(|b| b.bone == HumanBone::Spine)
        .ok_or("missing stiff spine")?;
    assert_ne!(flex_spine.rotation, stiff_spine.rotation);
    Ok(())
}

#[test]
fn avatar_mask_restricts_humanoid_layer_by_body_part_and_transform_path(
) -> Result<(), Box<dyn Error>> {
    let avatar = build_proportioned_avatar("MaskedAvatar", 1.0, 45.0, 90.0);

    let mut base_pose = HumanPose::default();
    base_pose.set_muscle(MuscleChannel::SpineFrontBack, 0.2);
    base_pose.set_muscle(MuscleChannel::LeftArmDownUp, 0.1);
    base_pose.set_muscle(MuscleChannel::LeftLegFrontBack, -0.6);

    let mut overlay_pose = HumanPose::default();
    overlay_pose.set_muscle(MuscleChannel::SpineFrontBack, 0.9);
    overlay_pose.set_muscle(MuscleChannel::LeftArmDownUp, 0.8);
    overlay_pose.set_muscle(MuscleChannel::LeftLegFrontBack, 0.7);

    let upper_body_mask = AvatarMask::new("UpperBody")
        .with_path("Root/Hips/Spine")
        .with_path("Root/Hips/Spine/LeftArm");

    let blended = apply_humanoid_layer_with_mask(
        &avatar,
        &base_pose,
        &overlay_pose,
        1.0,
        &upper_body_mask,
        &[HumanoidBodyPart::Body, HumanoidBodyPart::LeftArm],
    );

    assert!(
        (blended.muscle(MuscleChannel::SpineFrontBack) - 0.9).abs() < 1e-5,
        "masked-in spine muscle should take overlay value"
    );
    assert!(
        (blended.muscle(MuscleChannel::LeftArmDownUp) - 0.8).abs() < 1e-5,
        "masked-in left arm muscle should take overlay value"
    );
    assert!(
        (blended.muscle(MuscleChannel::LeftLegFrontBack) - (-0.6)).abs() < 1e-5,
        "masked-out left leg muscle must preserve base pose"
    );

    let mut handler = HumanPoseHandler::new(avatar.clone());
    handler.set_human_pose(&overlay_pose);
    let all_bindings = handler.to_sampled_bindings();
    let filtered = upper_body_mask.filter_bindings(&all_bindings);
    assert!(filtered
        .iter()
        .any(|b| b.target.object_path == "Root/Hips/Spine"));
    assert!(filtered
        .iter()
        .any(|b| b.target.object_path == "Root/Hips/Spine/LeftArm"));
    assert!(!filtered
        .iter()
        .any(|b| b.target.object_path == "Root/Hips/LeftLeg"));
    assert!(filtered.iter().all(|b| matches!(
        b.value,
        AnimatedValue::Quaternion(_) | AnimatedValue::Vector3(_)
    )));
    Ok(())
}

#[test]
fn invalid_avatar_definition_reports_missing_required_bones_and_duplicate_paths() {
    let incomplete = HumanoidAvatar::new("Incomplete")
        .with_bone(HumanBoneMapping::new(HumanBone::Hips, "Root/Hips"))
        .with_bone(HumanBoneMapping::new(HumanBone::Spine, "Root/Hips"));
    let issues = incomplete.validate();
    assert!(issues.iter().any(|issue| matches!(
        issue,
        AvatarValidationIssue::MissingRequiredBone(HumanBone::Head)
    )));
    assert!(issues.iter().any(|issue| matches!(
        issue,
        AvatarValidationIssue::DuplicateTransformPath { path, .. } if path == "Root/Hips"
    )));
}

#[test]
fn pose_to_muscles_retarget_and_muscles_to_pose_roundtrip_across_different_avatars(
) -> Result<(), Box<dyn Error>> {
    let source_avatar: Avatar = build_proportioned_avatar("SourceAvatar", 1.0, 40.0, 100.0)
        .with_bone(
            HumanoidBoneMapping::new(HumanoidBone::LeftShoulder, "Root/Hips/Spine/LeftShoulder")
                .with_bind_translation([-0.1, 0.3, 0.0]),
        )
        .with_bone(
            HumanoidBoneMapping::new(HumanoidBone::RightShoulder, "Root/Hips/Spine/RightShoulder")
                .with_bind_translation([0.1, 0.3, 0.0]),
        );
    let target_avatar: Avatar = build_proportioned_avatar("TargetAvatar", 2.0, 40.0, 100.0)
        .with_bone(
            HumanoidBoneMapping::new(HumanoidBone::LeftShoulder, "Root/Hips/Spine/LeftShoulder")
                .with_bind_translation([-0.2, 0.6, 0.0]),
        )
        .with_bone(
            HumanoidBoneMapping::new(HumanoidBone::RightShoulder, "Root/Hips/Spine/RightShoulder")
                .with_bind_translation([0.2, 0.6, 0.0]),
        );

    let clip = build_humanoid_clip();
    let clip_pose: HumanoidPose = sample_humanoid_clip(&clip, 0.5);
    let source_local_transforms: Vec<(String, Transform)> =
        muscles_to_pose(&source_avatar, &clip_pose);
    let extracted_source_pose = pose_to_muscles(&source_avatar, &source_local_transforms);

    assert!(
        (extracted_source_pose.muscle(MuscleChannel::SpineFrontBack)
            - clip_pose.muscle(MuscleChannel::SpineFrontBack))
        .abs()
            < 1e-4
    );
    assert!(
        (extracted_source_pose.muscle(MuscleChannel::LeftArmDownUp)
            - clip_pose.muscle(MuscleChannel::LeftArmDownUp))
        .abs()
            < 1e-4
    );

    let retargeted_pose = retarget(&extracted_source_pose, &source_avatar, &target_avatar);
    for channel in MuscleChannel::ALL {
        assert!(
            (retargeted_pose.muscle(channel) - extracted_source_pose.muscle(channel)).abs() < 1e-5
        );
    }

    let target_local_transforms = muscles_to_pose(&target_avatar, &retargeted_pose);
    let source_hips = source_local_transforms
        .iter()
        .find(|(path, _)| path == "Root/Hips")
        .map(|(_, tr)| tr)
        .ok_or("missing source hips transform")?;
    let target_hips = target_local_transforms
        .iter()
        .find(|(path, _)| path == "Root/Hips")
        .map(|(_, tr)| tr)
        .ok_or("missing target hips transform")?;

    assert!((source_hips.translation[1] - 1.1).abs() < 1e-4);
    assert!((target_hips.translation[1] - 2.2).abs() < 1e-4);

    let base_pose = HumanoidPose::default();
    let mask = AvatarMask::new("UpperOnly").with_path("Root/Hips/Spine");
    let layered =
        apply_layered_humanoid_pose(&target_avatar, &base_pose, &retargeted_pose, 1.0, &mask);
    let blended = blend_humanoid_layer(&target_avatar, &base_pose, &retargeted_pose, 1.0, &mask);
    assert_eq!(layered, blended);
    assert!(
        (layered.muscle(MuscleChannel::SpineFrontBack)
            - retargeted_pose.muscle(MuscleChannel::SpineFrontBack))
        .abs()
            < 1e-5
    );
    Ok(())
}

#[test]
fn muscle_id_alias_per_muscle_normalized_limits_and_apply_humanoid_mask_work_deterministically(
) -> Result<(), Box<dyn Error>> {
    let custom_limit = MuscleLimit::with_normalized_bounds(-45.0, 45.0, -0.5, 0.75);
    assert!((custom_limit.min() - (-0.5)).abs() < 1e-6);
    assert!((custom_limit.max() - 0.75).abs() < 1e-6);

    let restricted_avatar = build_proportioned_avatar("RestrictedAvatar", 1.0, 45.0, 90.0)
        .with_muscle_limit(MuscleId::SpineFrontBack, custom_limit);

    let mut source_pose = HumanoidPose::default();
    source_pose.set_muscle(MuscleId::SpineFrontBack, 0.9);
    source_pose.set_muscle(MuscleId::LeftArmDownUp, 0.3);

    let source_avatar = build_proportioned_avatar("UnrestrictedSource", 1.0, 45.0, 90.0);
    let retargeted = retarget(&source_pose, &source_avatar, &restricted_avatar);
    assert!(
        (retargeted.muscle(MuscleId::SpineFrontBack) - 0.75).abs() < 1e-5,
        "retargeting onto avatar with custom normalized max=0.75 must clamp 0.9 to 0.75"
    );
    assert!(
        (retargeted.muscle(MuscleId::LeftArmDownUp) - 0.3).abs() < 1e-5,
        "in-range muscle value must be preserved"
    );

    let mut base_pose = HumanoidPose::default();
    base_pose.set_muscle(MuscleId::SpineFrontBack, -0.2);
    base_pose.set_muscle(MuscleId::LeftLegFrontBack, -0.4);

    let mask = AvatarMask::new("SpineMask").with_path("Root/Hips/Spine");
    let masked = apply_humanoid_mask(&restricted_avatar, &base_pose, &retargeted, 1.0, &mask);
    assert!((masked.muscle(MuscleId::SpineFrontBack) - 0.75).abs() < 1e-5);
    assert!((masked.muscle(MuscleId::LeftLegFrontBack) - (-0.4)).abs() < 1e-5);
    Ok(())
}
