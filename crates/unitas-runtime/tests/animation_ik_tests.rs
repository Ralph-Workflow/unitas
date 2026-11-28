use std::cell::RefCell;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;

use unitas_animation::curve::{AssetReference, ObjectReferenceCurve, ObjectReferenceKey};
use unitas_animation::{
    AnimationClip, AnimatorController, AnimatorState, CurveBinding, CurveBindingTarget, FloatCurve,
    Keyframe, PropertyCurve, StateMotion, WrapMode, encode_controller_binary,
};
use unitas_core::{EntityId, IDENTITY_ROTATION, Transform, rotate_vector};
use unitas_runtime::animation::{
    AnimatorHookWorld, AnimatorIkPose, AnimatorRootMotion, AnimatorScriptHooks, AvatarIkGoal,
    AvatarIkHint, IkGoalPose, IkHintPose, LookAtIkConfig, TwoBoneIkChain, TwoBoneIkSkeletonChain,
    solve_look_at_ik, solve_two_bone_ik, string_to_hash,
};
use unitas_runtime::{
    AnimationWorld, test_read_path_buffer, test_write_path_buffer, unitas_runtime_animator_attach,
    unitas_runtime_animator_controller_create, unitas_runtime_animator_cross_fade,
    unitas_runtime_animator_cross_fade_hash, unitas_runtime_animator_get_apply_root_motion,
    unitas_runtime_animator_get_ik_hint_position,
    unitas_runtime_animator_get_ik_hint_position_weight, unitas_runtime_animator_get_ik_position,
    unitas_runtime_animator_get_ik_position_weight, unitas_runtime_animator_get_ik_rotation,
    unitas_runtime_animator_get_ik_rotation_weight, unitas_runtime_animator_play,
    unitas_runtime_animator_play_hash, unitas_runtime_animator_play_queued,
    unitas_runtime_animator_root_motion, unitas_runtime_animator_set_apply_root_motion,
    unitas_runtime_animator_set_controller, unitas_runtime_animator_set_ik_hint_position,
    unitas_runtime_animator_set_ik_hint_position_weight, unitas_runtime_animator_set_ik_position,
    unitas_runtime_animator_set_ik_position_weight, unitas_runtime_animator_set_ik_rotation,
    unitas_runtime_animator_set_ik_rotation_weight, unitas_runtime_animator_set_look_at_position,
    unitas_runtime_animator_set_look_at_weight, unitas_runtime_animator_state_info,
    unitas_runtime_animator_state_name, unitas_runtime_animator_string_to_hash,
    unitas_runtime_destroy, unitas_runtime_new, unitas_runtime_path_buffer, unitas_runtime_tick,
};

fn vec_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn dot_vec(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize_vec(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn angle_between_vectors(a: [f32; 3], b: [f32; 3]) -> f32 {
    let na = normalize_vec(a);
    let nb = normalize_vec(b);
    dot_vec(na, nb).clamp(-1.0, 1.0).acos()
}

fn quat_angle_difference(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3])
        .abs()
        .clamp(0.0, 1.0);
    2.0 * dot.acos()
}

fn forward_kinematics_from_result(
    chain: &TwoBoneIkChain,
    root_rotation: [f32; 4],
    mid_rotation: [f32; 4],
    solved_root_rot: [f32; 4],
    solved_mid_rot: [f32; 4],
) -> ([f32; 3], [f32; 3]) {
    let inv_root = [
        -root_rotation[0],
        -root_rotation[1],
        -root_rotation[2],
        root_rotation[3],
    ];
    let inv_mid = [
        -mid_rotation[0],
        -mid_rotation[1],
        -mid_rotation[2],
        mid_rotation[3],
    ];
    let upper_world = [
        chain.mid_position[0] - chain.root_position[0],
        chain.mid_position[1] - chain.root_position[1],
        chain.mid_position[2] - chain.root_position[2],
    ];
    let lower_world = [
        chain.tip_position[0] - chain.mid_position[0],
        chain.tip_position[1] - chain.mid_position[1],
        chain.tip_position[2] - chain.mid_position[2],
    ];
    let local_upper = rotate_vector(inv_root, upper_world);
    let local_lower = rotate_vector(inv_mid, lower_world);
    let fk_upper = rotate_vector(solved_root_rot, local_upper);
    let fk_mid = [
        chain.root_position[0] + fk_upper[0],
        chain.root_position[1] + fk_upper[1],
        chain.root_position[2] + fk_upper[2],
    ];
    let fk_lower = rotate_vector(solved_mid_rot, local_lower);
    let fk_tip = [
        fk_mid[0] + fk_lower[0],
        fk_mid[1] + fk_lower[1],
        fk_mid[2] + fk_lower[2],
    ];
    (fk_mid, fk_tip)
}

#[test]
fn two_bone_ik_reaches_reachable_target_within_tolerance_and_preserves_bone_lengths() {
    let chain = TwoBoneIkChain {
        root_position: [0.0, 1.5, 0.0],
        mid_position: [0.6, 1.5, 0.0],
        tip_position: [1.1, 1.5, 0.0],
        tip_rotation: IDENTITY_ROTATION,
    };
    let l1 = vec_distance(chain.root_position, chain.mid_position);
    let l2 = vec_distance(chain.mid_position, chain.tip_position);
    let target_pos = [0.55, 1.15, 0.35];
    let target_rot = [0.0, 0.38268343, 0.0, 0.9238795];
    let hint_pos = [0.4, 1.8, -0.5];

    let result = solve_two_bone_ik(
        &chain,
        target_pos,
        target_rot,
        Some(hint_pos),
        1.0,
        1.0,
        1.0,
    );

    assert!(
        vec_distance(result.tip_position, target_pos) < 1e-3,
        "tip position {:?} did not reach target {:?}",
        result.tip_position,
        target_pos
    );
    let solved_l1 = vec_distance(result.root_position, result.mid_position);
    let solved_l2 = vec_distance(result.mid_position, result.tip_position);
    assert!(
        (solved_l1 - l1).abs() < 1e-4,
        "upper bone length changed: {solved_l1} vs {l1}"
    );
    assert!(
        (solved_l2 - l2).abs() < 1e-4,
        "lower bone length changed: {solved_l2} vs {l2}"
    );
    assert!(
        quat_angle_difference(result.tip_rotation, target_rot) < 1e-3,
        "tip rotation {:?} did not match target rotation {:?}",
        result.tip_rotation,
        target_rot
    );

    let (solved_root_rot, solved_mid_rot, solved_tip_rot) =
        result.solved_bone_rotations(IDENTITY_ROTATION, IDENTITY_ROTATION);
    let (fk_mid, fk_tip) = forward_kinematics_from_result(
        &chain,
        IDENTITY_ROTATION,
        IDENTITY_ROTATION,
        solved_root_rot,
        solved_mid_rot,
    );
    assert!(vec_distance(fk_mid, result.mid_position) < 1e-3);
    assert!(vec_distance(fk_tip, target_pos) < 1e-3);
    assert!(quat_angle_difference(solved_tip_rot, target_rot) < 1e-3);
}

#[test]
fn two_bone_ik_stretches_straight_toward_unreachable_target_preserving_bone_lengths() {
    let chain = TwoBoneIkChain {
        root_position: [1.0, 2.0, -0.5],
        mid_position: [1.0, 1.2, -0.5],
        tip_position: [1.0, 0.5, -0.5],
        tip_rotation: IDENTITY_ROTATION,
    };
    let l1 = vec_distance(chain.root_position, chain.mid_position);
    let l2 = vec_distance(chain.mid_position, chain.tip_position);
    let total_length = l1 + l2;
    let target_pos = [4.0, 6.0, -0.5];

    let result = solve_two_bone_ik(&chain, target_pos, IDENTITY_ROTATION, None, 1.0, 1.0, 0.0);

    let solved_l1 = vec_distance(result.root_position, result.mid_position);
    let solved_l2 = vec_distance(result.mid_position, result.tip_position);
    let root_to_tip = vec_distance(result.root_position, result.tip_position);

    assert!((solved_l1 - l1).abs() < 1e-4);
    assert!((solved_l2 - l2).abs() < 1e-4);
    assert!(
        (root_to_tip - total_length).abs() < 1e-3,
        "stretched limb distance {root_to_tip} != {total_length}"
    );

    let dir_to_target = normalize_vec([
        target_pos[0] - chain.root_position[0],
        target_pos[1] - chain.root_position[1],
        target_pos[2] - chain.root_position[2],
    ]);
    let dir_to_mid = normalize_vec([
        result.mid_position[0] - chain.root_position[0],
        result.mid_position[1] - chain.root_position[1],
        result.mid_position[2] - chain.root_position[2],
    ]);
    let dir_to_tip = normalize_vec([
        result.tip_position[0] - chain.root_position[0],
        result.tip_position[1] - chain.root_position[1],
        result.tip_position[2] - chain.root_position[2],
    ]);
    assert!(angle_between_vectors(dir_to_target, dir_to_mid) < 1e-3);
    assert!(angle_between_vectors(dir_to_target, dir_to_tip) < 1e-3);
}

#[test]
fn two_bone_ik_respects_pole_hint_and_partial_weights() {
    let chain = TwoBoneIkChain {
        root_position: [0.0, 0.0, 0.0],
        mid_position: [0.0, 1.0, 0.0],
        tip_position: [0.0, 2.0, 0.0],
        tip_rotation: IDENTITY_ROTATION,
    };
    let target = [0.0, 1.2, 0.0];

    let plus_z = solve_two_bone_ik(
        &chain,
        target,
        IDENTITY_ROTATION,
        Some([0.0, 0.6, 2.0]),
        1.0,
        0.0,
        1.0,
    );
    let minus_z = solve_two_bone_ik(
        &chain,
        target,
        IDENTITY_ROTATION,
        Some([0.0, 0.6, -2.0]),
        1.0,
        0.0,
        1.0,
    );
    assert!(plus_z.mid_position[2] > 0.5);
    assert!(minus_z.mid_position[2] < -0.5);
    assert!(vec_distance(plus_z.tip_position, target) < 1e-3);
    assert!(vec_distance(minus_z.tip_position, target) < 1e-3);

    let zero_weight = solve_two_bone_ik(
        &chain,
        [1.0, 0.5, 0.5],
        [0.0, 0.70710677, 0.0, 0.70710677],
        Some([0.0, 1.0, 1.0]),
        0.0,
        0.0,
        1.0,
    );
    assert!(vec_distance(zero_weight.mid_position, chain.mid_position) < 1e-6);
    assert!(vec_distance(zero_weight.tip_position, chain.tip_position) < 1e-6);
    assert!(quat_angle_difference(zero_weight.tip_rotation, chain.tip_rotation) < 1e-6);

    let half_target = [1.0, 1.0, 0.0];
    let half_weight = solve_two_bone_ik(
        &chain,
        half_target,
        [0.0, 0.70710677, 0.0, 0.70710677],
        None,
        0.5,
        0.5,
        0.0,
    );
    let blended_goal = [0.5, 1.5, 0.0];
    assert!(vec_distance(half_weight.tip_position, blended_goal) < 1e-3);
    let expected_half_rot = [0.0, 0.38268343, 0.0, 0.9238795];
    assert!(quat_angle_difference(half_weight.tip_rotation, expected_half_rot) < 1e-3);
}

#[test]
fn look_at_ik_rotates_forward_vector_toward_target_according_to_weights_and_clamp() {
    let base_config = LookAtIkConfig {
        origin: [0.0, 1.6, 0.0],
        forward: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        target: [2.0, 1.6, 2.0],
        weight: 1.0,
        body_weight: 0.25,
        head_weight: 1.0,
        eyes_weight: 0.5,
        clamp_weight: 0.0,
    };
    let solved = solve_look_at_ik(&base_config);
    let desired_dir = normalize_vec([
        base_config.target[0] - base_config.origin[0],
        base_config.target[1] - base_config.origin[1],
        base_config.target[2] - base_config.origin[2],
    ]);
    let head_forward = rotate_vector(solved.head_rotation, base_config.forward);
    assert!(
        angle_between_vectors(head_forward, desired_dir) < 1e-3,
        "head forward {head_forward:?} did not align with desired {desired_dir:?}"
    );

    let full_angle = angle_between_vectors(base_config.forward, desired_dir);
    let body_forward = rotate_vector(solved.body_rotation, base_config.forward);
    let eyes_forward = rotate_vector(solved.eyes_rotation, base_config.forward);
    let body_angle = angle_between_vectors(base_config.forward, body_forward);
    let eyes_angle = angle_between_vectors(base_config.forward, eyes_forward);
    assert!((body_angle - full_angle * 0.25).abs() < 1e-3);
    assert!((eyes_angle - full_angle * 0.5).abs() < 1e-3);

    let clamped_config = LookAtIkConfig {
        target: [1.0, 1.6, -1.0],
        clamp_weight: 0.75,
        ..base_config
    };
    let clamped = solve_look_at_ik(&clamped_config);
    let clamped_head_forward = rotate_vector(clamped.head_rotation, base_config.forward);
    let clamped_head_angle = angle_between_vectors(base_config.forward, clamped_head_forward);
    let expected_max = std::f32::consts::PI * 0.25;
    assert!(
        (clamped_head_angle - expected_max).abs() < 1e-3,
        "clamped head angle {clamped_head_angle} != expected {expected_max}"
    );

    let fully_clamped = solve_look_at_ik(&LookAtIkConfig {
        clamp_weight: 1.0,
        ..base_config
    });
    assert_eq!(fully_clamped.head_rotation, IDENTITY_ROTATION);
}

#[derive(Default)]
struct EvaluationOrderLog {
    events: Vec<String>,
    observed_sampled_body_x_in_ik: Vec<f32>,
    observed_delta_x_in_move: Vec<f32>,
}

struct IkAndMoveTestHook {
    log: Rc<RefCell<EvaluationOrderLog>>,
    hand_target: [f32; 3],
    look_target: [f32; 3],
    override_move: bool,
    move_scale: f32,
}

impl AnimatorScriptHooks for IkAndMoveTestHook {
    fn on_animator_ik(&mut self, _entity: EntityId, layer_index: u32, pose: &mut AnimatorIkPose) {
        let mut log = self.log.borrow_mut();
        log.events.push(format!("on_animator_ik:{layer_index}"));
        if let Some(pos) = pose.body_position {
            log.observed_sampled_body_x_in_ik.push(pos[0]);
        }
        pose.set_ik_goal_position(AvatarIkGoal::LeftHand, self.hand_target);
        pose.set_ik_goal_position_weight(AvatarIkGoal::LeftHand, 1.0);
        pose.set_ik_hint_position(AvatarIkHint::LeftElbow, [0.3, 1.1, -0.5]);
        pose.set_ik_hint_position_weight(AvatarIkHint::LeftElbow, 1.0);
        pose.set_look_at_position(self.look_target);
        pose.set_look_at_weight(1.0, 0.2, 1.0, 0.5, 0.2);
    }

    fn on_animator_move(&mut self, _entity: EntityId, motion: &mut AnimatorRootMotion) -> bool {
        let mut log = self.log.borrow_mut();
        log.events.push("on_animator_move".to_string());
        log.observed_delta_x_in_move.push(motion.delta_position[0]);
        if self.override_move {
            motion.delta_position[0] *= self.move_scale;
            motion.delta_position[2] = 0.25;
        }
        self.override_move
    }
}

fn skeleton_animation_clip() -> AnimationClip {
    AnimationClip::new(
        "SkeletonStep",
        1.0,
        60.0,
        WrapMode::Loop,
        vec![
            CurveBinding::float(
                CurveBindingTarget::new("", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.0), Keyframe::linear(1.0, 2.0)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("ArmL_Upper", "Transform", "localPosition.y"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 1.4), Keyframe::linear(1.0, 1.4)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("ArmL_Upper/ArmL_Lower", "Transform", "localPosition.x"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 0.4), Keyframe::linear(1.0, 0.4)]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new(
                    "ArmL_Upper/ArmL_Lower/ArmL_Hand",
                    "Transform",
                    "localPosition.x",
                ),
                FloatCurve::new(vec![
                    Keyframe::linear(0.0, 0.35),
                    Keyframe::linear(1.0, 0.35),
                ]),
            ),
            CurveBinding::float(
                CurveBindingTarget::new("Head", "Transform", "localPosition.y"),
                FloatCurve::new(vec![Keyframe::linear(0.0, 1.6), Keyframe::linear(1.0, 1.6)]),
            ),
        ],
        Vec::new(),
    )
}

#[test]
fn animation_world_evaluates_clip_then_on_animator_ik_then_solvers_then_on_animator_move_on_skeleton()
-> Result<(), Box<dyn Error>> {
    let mut world = AnimationWorld::new();
    let entity = EntityId::new(42);
    world.ensure_animator(entity, Transform::default());
    let controller = AnimatorController::new(
        "IkSkeletonController",
        Vec::new(),
        vec![AnimatorState::new(
            "Step",
            StateMotion::Clip(skeleton_animation_clip()),
        )],
        0,
        Vec::new(),
    );
    let controller_handle = world.register_controller(controller);
    world.set_controller(entity, controller_handle)?;

    let mut hooks = AnimatorHookWorld::new();
    hooks.set_ik_layers(entity, vec![0]);
    hooks.set_skeleton_ik_chain(
        entity,
        AvatarIkGoal::LeftHand,
        TwoBoneIkSkeletonChain::new(
            "ArmL_Upper",
            "ArmL_Upper/ArmL_Lower",
            "ArmL_Upper/ArmL_Lower/ArmL_Hand",
        ),
    );
    hooks.set_look_at_bone_path(entity, "Head");

    let log = Rc::new(RefCell::new(EvaluationOrderLog::default()));
    let hand_target = [1.45, 1.15, 0.25];
    let look_target = [2.0, 1.6, 3.0];
    hooks.attach(
        entity,
        Box::new(IkAndMoveTestHook {
            log: log.clone(),
            hand_target,
            look_target,
            override_move: true,
            move_scale: 0.5,
        }),
    );

    let child_updates = world.tick_with_hooks(0.5, &mut hooks);

    let log_borrow = log.borrow();
    assert_eq!(
        log_borrow.events,
        vec![
            "on_animator_ik:0".to_string(),
            "on_animator_move".to_string()
        ]
    );
    assert_eq!(log_borrow.observed_sampled_body_x_in_ik.len(), 1);
    assert!(
        (log_borrow.observed_sampled_body_x_in_ik[0] - 1.0).abs() < 1e-5,
        "OnAnimatorIK must observe clip-evaluated root pose (1.0), got {}",
        log_borrow.observed_sampled_body_x_in_ik[0]
    );
    assert_eq!(log_borrow.observed_delta_x_in_move.len(), 1);
    assert!(
        (log_borrow.observed_delta_x_in_move[0] - 1.0).abs() < 1e-5,
        "OnAnimatorMove must receive root motion delta (1.0), got {}",
        log_borrow.observed_delta_x_in_move[0]
    );

    let root_transform = world.transform(entity).ok_or("missing root transform")?;
    assert!(
        (root_transform.translation[0] - 0.5).abs() < 1e-5,
        "OnAnimatorMove scaled x delta to 0.5, got {}",
        root_transform.translation[0]
    );
    assert!(
        (root_transform.translation[2] - 0.25).abs() < 1e-5,
        "OnAnimatorMove set z delta to 0.25, got {}",
        root_transform.translation[2]
    );

    let last_pose = hooks
        .last_ik_pose(entity)
        .ok_or("missing solved IK pose on hook world")?;
    let solved_hand = last_pose
        .solved_goal(AvatarIkGoal::LeftHand)
        .ok_or("missing solved LeftHand goal")?;
    assert!(
        vec_distance(solved_hand.tip_position, hand_target) < 1e-3,
        "solved LeftHand tip {:?} did not reach target {:?}",
        solved_hand.tip_position,
        hand_target
    );

    let solved_look = last_pose
        .solved_look_at()
        .ok_or("missing solved look-at result")?;
    let head_dir = rotate_vector(solved_look.head_rotation, [0.0, 0.0, 1.0]);
    let expected_look_dir = normalize_vec([
        look_target[0] - 1.0,
        look_target[1] - 1.6,
        look_target[2] - 0.0,
    ]);
    assert!(
        angle_between_vectors(head_dir, expected_look_dir) < 1e-3,
        "head look direction {head_dir:?} did not match {expected_look_dir:?}"
    );

    let (_, bindings) = child_updates
        .iter()
        .find(|(e, _)| *e == entity)
        .ok_or("missing child bindings for entity")?;
    assert!(
        bindings
            .iter()
            .any(|b| b.target.object_path == "ArmL_Upper" && b.target.property == "localRotation")
    );
    assert!(bindings.iter().any(|b| {
        b.target.object_path == "ArmL_Upper/ArmL_Lower" && b.target.property == "localRotation"
    }));
    assert!(
        bindings
            .iter()
            .any(|b| b.target.object_path == "Head" && b.target.property == "localRotation")
    );
    Ok(())
}

#[test]
fn animator_ik_pose_solves_multiple_goals_and_hints_with_unity_style_accessors() {
    let mut pose = AnimatorIkPose::default();
    let left_foot_chain = TwoBoneIkChain {
        root_position: [-0.2, 0.9, 0.0],
        mid_position: [-0.2, 0.5, 0.05],
        tip_position: [-0.2, 0.1, 0.0],
        tip_rotation: IDENTITY_ROTATION,
    };
    pose.set_ik_chain(AvatarIkGoal::LeftFoot, left_foot_chain);
    pose.set_ik_goal(
        AvatarIkGoal::LeftFoot,
        IkGoalPose {
            position: [-0.2, 0.25, 0.2],
            rotation: IDENTITY_ROTATION,
            position_weight: 1.0,
            rotation_weight: 1.0,
        },
    );
    pose.set_ik_hint(
        AvatarIkHint::LeftKnee,
        IkHintPose {
            position: [-0.2, 0.55, 0.6],
            weight: 1.0,
        },
    );
    pose.set_look_at(LookAtIkConfig {
        origin: [0.0, 1.5, 0.0],
        forward: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        target: [0.0, 1.5, 3.0],
        weight: 1.0,
        body_weight: 0.1,
        head_weight: 0.9,
        eyes_weight: 1.0,
        clamp_weight: 0.5,
    });

    pose.solve();

    let solved_foot = pose.solved_goal(AvatarIkGoal::LeftFoot);
    assert!(solved_foot.is_some());
    if let Some(foot) = solved_foot {
        assert!(vec_distance(foot.tip_position, [-0.2, 0.25, 0.2]) < 1e-3);
        assert!(foot.mid_position[2] > 0.1);
    }
    assert!(pose.solved_look_at().is_some());
}

fn write_test_tga(path: &Path, width: u16, height: u16, rgb: [u8; 3]) -> std::io::Result<()> {
    let mut bytes = vec![0u8; 18 + (width as usize) * (height as usize) * 3];
    bytes[2] = 2;
    bytes[12..14].copy_from_slice(&width.to_le_bytes());
    bytes[14..16].copy_from_slice(&height.to_le_bytes());
    bytes[16] = 24;
    bytes[17] = 0x20;
    for chunk in bytes[18..].chunks_exact_mut(3) {
        chunk[0] = rgb[2];
        chunk[1] = rgb[1];
        chunk[2] = rgb[0];
    }
    std::fs::write(path, bytes)
}

fn build_three_step_sprite_swap_controller(
    guid_a: &str,
    guid_b: &str,
    missing_guid: &str,
    target: &CurveBindingTarget,
) -> Result<AnimatorController, Box<dyn Error>> {
    let ref_a = AssetReference::from_hex(guid_a, 21300000).ok_or("invalid guid_a")?;
    let ref_b = AssetReference::from_hex(guid_b, 21300000).ok_or("invalid guid_b")?;
    let ref_missing =
        AssetReference::from_hex(missing_guid, 21300099).ok_or("invalid missing_guid")?;
    let clip = AnimationClip::new(
        "Swap",
        3.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::new(
            target.clone(),
            PropertyCurve::ObjectReference(ObjectReferenceCurve::new(vec![
                ObjectReferenceKey::new(0.0, ref_a),
                ObjectReferenceKey::new(1.0, ref_b),
                ObjectReferenceKey::new(2.0, ref_missing),
            ])),
        )],
        Vec::new(),
    );
    Ok(AnimatorController::new(
        "SwapCtrl",
        Vec::new(),
        vec![AnimatorState::new("Swap", StateMotion::Clip(clip))],
        0,
        Vec::new(),
    ))
}

#[test]
fn ani_001_animation_world_resolves_sprite_swap_and_records_missing_sprite_diagnostic()
-> Result<(), Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_anim_world_sprite_swap_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;

    let guid_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let guid_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let missing_guid = "deadbeefdeadbeefdeadbeefdeadbeef";
    let tex_a = dir.join("a.tga");
    let tex_b = dir.join("b.tga");
    write_test_tga(&tex_a, 16, 16, [255, 0, 0])?;
    write_test_tga(&tex_b, 32, 32, [0, 255, 0])?;
    for (meta_name, guid) in [("a.tga.meta", guid_a), ("b.tga.meta", guid_b)] {
        std::fs::write(
            dir.join(meta_name),
            format!("fileFormatVersion: 2\nguid: {guid}\nTextureImporter:\n  sRGBTexture: 0\n"),
        )?;
    }

    let target = CurveBindingTarget::new("", "SpriteRenderer", "m_Sprite");
    let controller =
        build_three_step_sprite_swap_controller(guid_a, guid_b, missing_guid, &target)?;

    let mut world = AnimationWorld::new();
    world.register_sprite_asset(guid_a, &tex_a);
    world.register_sprite_asset(guid_b, &tex_b);
    let entity = EntityId::new(1);
    world.ensure_animator(entity, Transform::default());
    let handle = world.register_controller(controller);
    world.set_controller(entity, handle)?;

    let tex0 = world
        .resolved_sprite(entity, &target)
        .flatten()
        .ok_or("expected resolved sprite at t=0")?;
    assert_eq!((tex0.width, tex0.height), (16, 16));
    assert_eq!(world.diagnostic_count(entity), Some(0));

    world.tick(1.0);
    let tex1 = world
        .resolved_sprite(entity, &target)
        .flatten()
        .ok_or("expected resolved sprite at t=1")?;
    assert_eq!((tex1.width, tex1.height), (32, 32));
    assert_eq!(world.diagnostic_count(entity), Some(0));

    world.tick(1.0);
    let _ = std::fs::remove_dir_all(&dir);
    let diags = world
        .diagnostics(entity)
        .ok_or("expected diagnostics slice")?;
    assert!(
        diags.iter().any(|d| d.contains(missing_guid)
            && d.contains("SpriteRenderer")
            && d.contains("m_Sprite")),
        "expected diagnostic naming asset and member, got {diags:?}"
    );
    Ok(())
}

fn build_two_state_controller() -> AnimatorController {
    let idle_clip = AnimationClip::new(
        "IdleClip",
        1.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::new(
            CurveBindingTarget::transform("", "localPosition.x"),
            PropertyCurve::Float(FloatCurve::new(vec![
                Keyframe::linear(0.0, 0.0),
                Keyframe::linear(1.0, 10.0),
            ])),
        )],
        Vec::new(),
    );
    let run_clip = AnimationClip::new(
        "RunClip",
        1.0,
        60.0,
        WrapMode::Once,
        vec![CurveBinding::new(
            CurveBindingTarget::transform("", "localPosition.x"),
            PropertyCurve::Float(FloatCurve::new(vec![
                Keyframe::linear(0.0, 100.0),
                Keyframe::linear(1.0, 110.0),
            ])),
        )],
        Vec::new(),
    );
    AnimatorController::new(
        "Locomotion",
        Vec::new(),
        vec![
            AnimatorState::new("Idle", StateMotion::Clip(idle_clip)),
            AnimatorState::new("Run", StateMotion::Clip(run_clip)),
        ],
        0,
        Vec::new(),
    )
}

#[test]
fn ani_002_play_play_queued_cross_fade_root_motion_and_ik_pose() -> Result<(), Box<dyn Error>> {
    let mut world = AnimationWorld::new();
    let entity = EntityId::new(1);
    world.ensure_animator(entity, Transform::default());
    let handle = world.register_controller(build_two_state_controller());
    world.set_controller(entity, handle)?;

    assert_eq!(world.apply_root_motion(entity), Some(true));
    world.set_apply_root_motion(entity, false)?;
    assert_eq!(world.apply_root_motion(entity), Some(false));
    world.set_apply_root_motion(entity, true)?;

    world.play_state(entity, "Run")?;
    world.tick(0.25);
    let info = world.state_info(entity)?;
    assert_eq!(info.state_name, "Run");
    let rm = world.root_motion(entity).ok_or("expected root motion")?;
    assert!((rm.delta_position[0] - 2.5).abs() < 1e-4);

    world.play_state(entity, "Idle")?;
    world.play_queued_state(entity, "Run")?;
    world.tick(0.5);
    assert_eq!(world.state_info(entity)?.state_name, "Idle");
    world.tick(0.6);
    assert_eq!(world.state_info(entity)?.state_name, "Run");

    world.cross_fade_state(entity, "Idle", 0.5)?;
    world.tick(0.1);
    let fade_info = world.state_info(entity)?;
    assert!(fade_info.is_in_transition);

    let hash_idle = string_to_hash("Idle");
    let hash_run = string_to_hash("Run");
    assert_ne!(hash_idle, hash_run);
    assert_eq!(hash_idle, string_to_hash("Idle"));
    world.play_state_hash(entity, hash_idle)?;
    world.tick(0.0);
    assert_eq!(world.state_info(entity)?.state_name, "Idle");

    world.set_ik_position(entity, AvatarIkGoal::LeftHand, [1.0, 2.0, 3.0])?;
    world.set_ik_position_weight(entity, AvatarIkGoal::LeftHand, 0.75)?;
    world.set_ik_rotation(entity, AvatarIkGoal::LeftHand, [0.0, 0.0, 0.0, 1.0])?;
    world.set_ik_rotation_weight(entity, AvatarIkGoal::LeftHand, 0.5)?;
    world.set_ik_hint_position(entity, AvatarIkHint::LeftElbow, [0.5, 1.5, 2.5])?;
    world.set_ik_hint_position_weight(entity, AvatarIkHint::LeftElbow, 0.8)?;
    world.set_look_at_position(entity, [0.0, 1.6, 5.0])?;
    world.set_look_at_weight(entity, [0.9, 0.2, 0.8, 0.1, 0.5])?;

    assert_eq!(
        world.ik_position(entity, AvatarIkGoal::LeftHand),
        Some([1.0, 2.0, 3.0])
    );
    assert_eq!(
        world.ik_position_weight(entity, AvatarIkGoal::LeftHand),
        Some(0.75)
    );
    assert_eq!(
        world.ik_rotation(entity, AvatarIkGoal::LeftHand),
        Some([0.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(
        world.ik_rotation_weight(entity, AvatarIkGoal::LeftHand),
        Some(0.5)
    );
    assert_eq!(
        world.ik_hint_position(entity, AvatarIkHint::LeftElbow),
        Some([0.5, 1.5, 2.5])
    );
    assert_eq!(
        world.ik_hint_position_weight(entity, AvatarIkHint::LeftElbow),
        Some(0.8)
    );
    assert_eq!(world.look_at_position(entity), Some([0.0, 1.6, 5.0]));
    assert_eq!(world.look_at_weight(entity), Some(0.9));
    Ok(())
}

fn abi_write(handle: u64, bytes: &[u8]) -> Result<u64, Box<dyn Error>> {
    let addr = unitas_runtime_path_buffer(handle, bytes.len() as u64);
    if addr == 0 {
        return Err("buffer allocation failed".into());
    }
    test_write_path_buffer(handle, bytes)?;
    Ok(addr)
}

fn abi_read(handle: u64, len: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = vec![0u8; len];
    let _ = unitas_runtime_path_buffer(handle, len as u64);
    test_read_path_buffer(handle, &mut out)?;
    Ok(out)
}

fn abi_f32(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

fn abi_u32(bytes: &[u8], offset: usize) -> u32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(raw)
}

fn abi_state_name(handle: u64, entity: u64) -> Result<String, Box<dyn Error>> {
    let len = unitas_runtime_animator_state_name(handle, entity);
    if len < 0 {
        return Err("state_name failed".into());
    }
    Ok(String::from_utf8(abi_read(handle, len as usize)?)?)
}

#[test]
fn ani_002_abi_play_queued_cross_fade_root_motion_and_ik_exports() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let encoded = encode_controller_binary(&build_two_state_controller());
    let addr = abi_write(handle, &encoded)?;
    let ctrl = unitas_runtime_animator_controller_create(handle, addr, encoded.len() as u64);
    assert_ne!(ctrl, 0);
    let entity = unitas_runtime_animator_attach(handle, 0);
    assert_ne!(entity, 0);
    assert_eq!(
        unitas_runtime_animator_set_controller(handle, entity, ctrl),
        0
    );

    let idle_addr = abi_write(handle, b"Idle")?;
    let idle_hash = unitas_runtime_animator_string_to_hash(handle, idle_addr, 4);
    assert_eq!(idle_hash, string_to_hash("Idle"));

    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Idle");

    let run_addr = abi_write(handle, b"Run")?;
    assert_eq!(unitas_runtime_animator_play(handle, entity, run_addr, 3), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.1), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Run");

    assert_eq!(
        unitas_runtime_animator_play_hash(handle, entity, idle_hash),
        0
    );
    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Idle");

    let run_addr = abi_write(handle, b"Run")?;
    assert_eq!(
        unitas_runtime_animator_play_queued(handle, entity, run_addr, 3),
        0
    );
    assert_eq!(unitas_runtime_tick(handle, 0.5), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Idle");
    assert_eq!(unitas_runtime_tick(handle, 0.6), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Run");

    let idle_addr = abi_write(handle, b"Idle")?;
    assert_eq!(
        unitas_runtime_animator_cross_fade(handle, entity, idle_addr, 4, 0.5),
        0
    );
    assert_eq!(unitas_runtime_tick(handle, 0.1), 0);
    let info_buf = unitas_runtime_path_buffer(handle, 80);
    assert_eq!(
        unitas_runtime_animator_state_info(handle, entity, info_buf),
        0
    );
    let info_bytes = abi_read(handle, 80)?;
    assert_ne!(abi_u32(&info_bytes, 20), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.5), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Idle");

    let run_hash = string_to_hash("Run");
    assert_eq!(
        unitas_runtime_animator_cross_fade_hash(handle, entity, run_hash, 0.2),
        0
    );
    assert_eq!(unitas_runtime_tick(handle, 0.3), 0);
    assert_eq!(abi_state_name(handle, entity)?, "Run");

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn ani_002_abi_root_motion_and_ik_getters_setters() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let encoded = encode_controller_binary(&build_two_state_controller());
    let addr = abi_write(handle, &encoded)?;
    let ctrl = unitas_runtime_animator_controller_create(handle, addr, encoded.len() as u64);
    let entity = unitas_runtime_animator_attach(handle, 0);
    assert_eq!(
        unitas_runtime_animator_set_controller(handle, entity, ctrl),
        0
    );

    let out4 = unitas_runtime_path_buffer(handle, 4);
    assert_eq!(
        unitas_runtime_animator_get_apply_root_motion(handle, entity, out4),
        0
    );
    assert_eq!(abi_u32(&abi_read(handle, 4)?, 0), 1);
    assert_eq!(
        unitas_runtime_animator_set_apply_root_motion(handle, entity, 0),
        0
    );
    assert_eq!(
        unitas_runtime_animator_get_apply_root_motion(handle, entity, out4),
        0
    );
    assert_eq!(abi_u32(&abi_read(handle, 4)?, 0), 0);
    assert_eq!(
        unitas_runtime_animator_set_apply_root_motion(handle, entity, 1),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    assert_eq!(unitas_runtime_tick(handle, 0.2), 0);
    let out28 = unitas_runtime_path_buffer(handle, 28);
    assert_eq!(
        unitas_runtime_animator_root_motion(handle, entity, out28),
        0
    );
    let rm_bytes = abi_read(handle, 28)?;
    assert!((abi_f32(&rm_bytes, 0) - 2.0).abs() < 1e-4);
    assert!((abi_f32(&rm_bytes, 24) - 1.0).abs() < 1e-4);

    assert_eq!(
        unitas_runtime_animator_set_ik_position(handle, entity, 3, 1.5, 2.5, -0.5),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_ik_position_weight(handle, entity, 3, 0.85),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_ik_rotation(
            handle,
            entity,
            3,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_ik_rotation_weight(handle, entity, 3, 0.65),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_ik_hint_position(handle, entity, 3, 0.5, 1.25, -0.25),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_ik_hint_position_weight(handle, entity, 3, 0.45),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_look_at_position(handle, entity, 0.0, 1.7, 10.0),
        0
    );
    assert_eq!(
        unitas_runtime_animator_set_look_at_weight(handle, entity, 0.9, 0.25, 0.75, 0.1, 0.5),
        0
    );

    let out12 = unitas_runtime_path_buffer(handle, 12);
    assert_eq!(
        unitas_runtime_animator_get_ik_position(handle, entity, 3, out12),
        0
    );
    let pos_b = abi_read(handle, 12)?;
    assert_eq!(
        [abi_f32(&pos_b, 0), abi_f32(&pos_b, 4), abi_f32(&pos_b, 8)],
        [1.5, 2.5, -0.5]
    );
    let out4 = unitas_runtime_path_buffer(handle, 4);
    assert_eq!(
        unitas_runtime_animator_get_ik_position_weight(handle, entity, 3, out4),
        0
    );
    assert!((abi_f32(&abi_read(handle, 4)?, 0) - 0.85).abs() < 1e-6);

    let out16 = unitas_runtime_path_buffer(handle, 16);
    assert_eq!(
        unitas_runtime_animator_get_ik_rotation(handle, entity, 3, out16),
        0
    );
    let rot_b = abi_read(handle, 16)?;
    assert!((abi_f32(&rot_b, 4) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    assert!((abi_f32(&rot_b, 12) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    let out4 = unitas_runtime_path_buffer(handle, 4);
    assert_eq!(
        unitas_runtime_animator_get_ik_rotation_weight(handle, entity, 3, out4),
        0
    );
    assert!((abi_f32(&abi_read(handle, 4)?, 0) - 0.65).abs() < 1e-6);

    let out12 = unitas_runtime_path_buffer(handle, 12);
    assert_eq!(
        unitas_runtime_animator_get_ik_hint_position(handle, entity, 3, out12),
        0
    );
    let h_b = abi_read(handle, 12)?;
    assert_eq!(
        [abi_f32(&h_b, 0), abi_f32(&h_b, 4), abi_f32(&h_b, 8)],
        [0.5, 1.25, -0.25]
    );
    let out4 = unitas_runtime_path_buffer(handle, 4);
    assert_eq!(
        unitas_runtime_animator_get_ik_hint_position_weight(handle, entity, 3, out4),
        0
    );
    assert!((abi_f32(&abi_read(handle, 4)?, 0) - 0.45).abs() < 1e-6);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
