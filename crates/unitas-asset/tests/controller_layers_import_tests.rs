use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_animation::{
    Animator, AnimatorController, LayerBlendingMode, StateMotion,
};
use unitas_asset::anim_import::{import_animator_controller, AnimImportError};
use unitas_asset::meta::GuidIndex;

const CLIP_A_GUID: &str = "aa000000000000000000000000000001";
const CLIP_B_GUID: &str = "bb000000000000000000000000000002";
const CLIP_C_GUID: &str = "cc000000000000000000000000000003";
const MASK_GUID: &str = "dd000000000000000000000000000004";

fn anim_yaml(name: &str, path: &str, value: f32) -> String {
    format!(
        "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!74 &7400000
AnimationClip:
  m_Name: {name}
  serializedVersion: 7
  m_Legacy: 0
  m_Compressed: 0
  m_UseHighQualityCurve: 1
  m_RotationCurves: []
  m_CompressedRotationCurves: []
  m_EulerCurves: []
  m_PositionCurves: []
  m_ScaleCurves: []
  m_FloatCurves:
  - curve:
      serializedVersion: 2
      m_Curve:
      - time: 0
        value: {value}
        inSlope: 0
        outSlope: 0
      - time: 1
        value: {value}
        inSlope: 0
        outSlope: 0
      m_PreInfinity: 2
      m_PostInfinity: 2
      m_RotationOrder: 4
    attribute: localPosition.x
    path: {path}
    classID: 4
    script: {{fileID: 0}}
  m_PPtrCurves: []
  m_SampleRate: 60
  m_WrapMode: 0
  m_AnimationClipSettings:
    serializedVersion: 2
    m_StartTime: 0
    m_StopTime: 1
    m_LoopTime: 1
  m_Events: []
"
    )
}

fn write_fixture_assets(dir: &Path) -> Result<GuidIndex, Box<dyn Error>> {
    let fixtures: [(&str, &str, String); 4] = [
        ("PoseA.anim", CLIP_A_GUID, anim_yaml("PoseA", "Root", 0.0)),
        ("PoseB.anim", CLIP_B_GUID, anim_yaml("PoseB", "Root/Arm", 10.0)),
        ("PoseC.anim", CLIP_C_GUID, anim_yaml("PoseC", "Root/Leg", 4.0)),
        (
            "UpperBody.mask",
            MASK_GUID,
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!319 &31900000
AvatarMask:
  m_Name: UpperBody
  m_Elements:
  - m_Path: Root/Arm
    m_Weight: 1
  - m_Path: Root/Leg
    m_Weight: 0
  m_Mask: 0
  m_HumanoidBodyMask: []
"
                .to_string(),
        ),
    ];
    for (file_name, guid, body) in fixtures {
        fs::write(dir.join(file_name), body)?;
        fs::write(dir.join(format!("{file_name}.meta")), format!("guid: {guid}\n"))?;
    }
    Ok(GuidIndex::build(dir)?)
}

fn unique_temp_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "unitas_controller_layers_{label}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

const MULTI_LAYER_CONTROLLER: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!91 &9100000
AnimatorController:
  m_Name: Layered
  m_AnimatorParameters:
  - m_Name: wave
    m_Type: 1
    m_DefaultFloat: 0
  m_AnimatorLayers:
  - m_Name: Base Layer
    m_DefaultWeight: 1
    m_BlendingMode: 0
    m_SyncedLayerIndex: -1
    m_Mask: {fileID: 0}
    m_StateMachine: {fileID: 110700000}
  - m_Name: Upper Body
    m_DefaultWeight: 0.5
    m_BlendingMode: 0
    m_SyncedLayerIndex: -1
    m_Mask: {fileID: 31900000, guid: dd000000000000000000000000000004, type: 3}
    m_StateMachine: {fileID: 110700001}
--- !u!1107 &110700000
AnimatorStateMachine:
  m_Name: Base Layer
  m_ChildStates:
  - m_State: {fileID: 110200000}
  m_ChildStateMachines: []
  m_AnyStateTransitions: []
  m_EntryTransitions: []
  m_StateMachineTransitions: []
  m_DefaultState: {fileID: 110200000}
--- !u!1102 &110200000
AnimatorState:
  m_Name: BaseState
  m_Speed: 1
  m_Motion: {fileID: 7400000, guid: aa000000000000000000000000000001, type: 2}
--- !u!1107 &110700001
AnimatorStateMachine:
  m_Name: Upper Body
  m_ChildStates:
  - m_State: {fileID: 110200001}
  m_ChildStateMachines: []
  m_AnyStateTransitions: []
  m_EntryTransitions: []
  m_StateMachineTransitions: []
  m_DefaultState: {fileID: 110200001}
--- !u!1102 &110200001
AnimatorState:
  m_Name: UpperState
  m_Speed: 1
  m_Motion: {fileID: 206000000}
--- !u!206 &206000000
BlendTree:
  m_Name: UpperBlend
  m_BlendType: 4
  m_BlendParameter:
  m_BlendParameterY:
  m_MinThreshold: 0
  m_MaxThreshold: 1
  m_UseAutomaticThresholds: 0
  m_Childs:
  - m_Motion: {fileID: 7400000, guid: bb000000000000000000000000000002, type: 2}
    m_Threshold: 0
    m_Position: {x: 0, y: 0}
    m_TimeScale: 1
    m_CycleOffset: 0
    m_DirectBlendParameter: wave
    m_Mirror: 0
";

#[test]
fn imports_multi_layer_controller_with_mask_weight_and_direct_blend_tree(
) -> Result<(), Box<dyn Error>> {
    let dir = unique_temp_dir("multi_layer")?;
    let guids = write_fixture_assets(&dir)?;
    let controller_path = dir.join("Layered.controller");
    fs::write(&controller_path, MULTI_LAYER_CONTROLLER)?;

    let controller = import_animator_controller(&controller_path, &guids)?;

    assert_eq!(controller.name, "Layered");
    // Unity layer 0 maps onto the base layer fields.
    assert_eq!(controller.states.len(), 1);
    assert_eq!(controller.states[0].name, "BaseState");
    assert_eq!(controller.entry_state, 0);
    // Unity layer 1 becomes runtime layer 0.
    assert_eq!(controller.layers.len(), 1);
    let layer = &controller.layers[0];
    assert_eq!(layer.name, "Upper Body");
    assert!((layer.weight - 0.5).abs() < 1e-6);
    assert_eq!(layer.blending_mode, LayerBlendingMode::Override);
    let mask = layer.mask.as_ref().ok_or("layer mask imported")?;
    assert!(mask.includes_path("Root/Arm"));
    assert!(!mask.includes_path("Root/Leg"));
    assert_eq!(layer.states.len(), 1);
    match &layer.states[0].motion {
        StateMotion::BlendTreeDirect { children } => {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].0, "wave");
            assert_eq!(children[0].1.name, "PoseB");
        }
        other => return Err(format!("expected Direct blend tree, got {other:?}").into()),
    }
    assert!(controller.validate().is_empty());

    // End to end: cooked artifact roundtrip, then runtime blending.
    let decoded = AnimatorController::decode_binary(&controller.encode_binary())?;
    assert_eq!(decoded, controller);
    let mut animator = Animator::new(decoded);
    animator.set_float("wave", 1.0).map_err(|d| format!("{d:?}"))?;
    let step = animator.step(0.1);
    let arm = step
        .bindings
        .iter()
        .find(|binding| binding.target.object_path == "Root/Arm")
        .and_then(|binding| match &binding.value {
            unitas_animation::AnimatedValue::Float(v) => Some(*v),
            _ => None,
        })
        .ok_or("masked arm binding present")?;
    // Layer weight 0.5 blends the 10.0 arm pose halfway onto the base.
    assert!((arm - 5.0).abs() < 1e-3, "arm value {arm}");

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

const NESTED_MACHINE_CONTROLLER: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!91 &9100000
AnimatorController:
  m_Name: Nested
  m_AnimatorParameters:
  - m_Name: done
    m_Type: 4
    m_DefaultBool: 0
  - m_Name: back
    m_Type: 4
    m_DefaultBool: 0
  m_AnimatorLayers:
  - m_Name: Base Layer
    m_DefaultWeight: 1
    m_BlendingMode: 0
    m_SyncedLayerIndex: -1
    m_Mask: {fileID: 0}
    m_StateMachine: {fileID: 110700000}
--- !u!1107 &110700000
AnimatorStateMachine:
  m_Name: Base Layer
  m_ChildStates:
  - m_State: {fileID: 110200000}
  m_ChildStateMachines:
  - m_StateMachine: {fileID: 110700001}
    m_Position: {x: 300, y: 0}
  m_AnyStateTransitions: []
  m_EntryTransitions: []
  m_StateMachineTransitions:
  - first: {fileID: 110700001}
    second:
    - {fileID: 110100001}
  m_DefaultState: {fileID: 110200000}
--- !u!1102 &110200000
AnimatorState:
  m_Name: Idle
  m_Speed: 1
  m_Motion: {fileID: 7400000, guid: aa000000000000000000000000000001, type: 2}
  m_Transitions:
  - {fileID: 110100000}
--- !u!1101 &110100000
AnimatorStateTransition:
  m_Name: IdleToMove
  m_DstState: {fileID: 110700001}
  m_Conditions:
  - m_ConditionMode: 1
    m_ConditionEvent: done
  m_HasExitTime: 0
  m_TransitionDuration: 0
--- !u!1107 &110700001
AnimatorStateMachine:
  m_Name: Move
  m_ChildStates:
  - m_State: {fileID: 110200001}
  - m_State: {fileID: 110200002}
  m_ChildStateMachines: []
  m_AnyStateTransitions: []
  m_EntryTransitions: []
  m_StateMachineTransitions: []
  m_DefaultState: {fileID: 110200002}
--- !u!1102 &110200001
AnimatorState:
  m_Name: Walk
  m_Speed: 1
  m_Motion: {fileID: 7400000, guid: bb000000000000000000000000000002, type: 2}
--- !u!1102 &110200002
AnimatorState:
  m_Name: Run
  m_Speed: 1
  m_Motion: {fileID: 7400000, guid: cc000000000000000000000000000003, type: 2}
--- !u!1101 &110100001
AnimatorStateTransition:
  m_Name: MoveToWalk
  m_DstState: {fileID: 110200001}
  m_Conditions:
  - m_ConditionMode: 1
    m_ConditionEvent: back
  m_HasExitTime: 0
  m_TransitionDuration: 0
";

#[test]
fn imports_nested_state_machines_with_machine_node_transitions(
) -> Result<(), Box<dyn Error>> {
    let dir = unique_temp_dir("nested")?;
    let guids = write_fixture_assets(&dir)?;
    let controller_path = dir.join("Nested.controller");
    fs::write(&controller_path, NESTED_MACHINE_CONTROLLER)?;

    let controller = import_animator_controller(&controller_path, &guids)?;

    let names: Vec<&str> = controller.states.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["Idle", "Walk", "Run"]);
    assert_eq!(controller.entry_state, 0);
    // Transition targeting the sub-machine node enters its default state (Run, index 2).
    let idle_to_move = controller
        .transitions
        .iter()
        .find(|t| t.source() == unitas_animation::controller::TransitionSource::State(0))
        .ok_or("Idle transition")?;
    assert_eq!(
        idle_to_move.target,
        unitas_animation::TransitionTarget::State(2)
    );
    // The machine-node transition (m_StateMachineTransitions) resolves to the
    // sub-machine's default state as its source.
    let machine_to_walk = controller
        .transitions
        .iter()
        .find(|t| t.source() == unitas_animation::controller::TransitionSource::State(2))
        .ok_or("machine-node transition")?;
    assert_eq!(
        machine_to_walk.target,
        unitas_animation::TransitionTarget::State(1)
    );
    assert!(controller.validate().is_empty());

    // Runtime: done=true moves Idle -> Run; back=true moves Run -> Walk.
    let mut animator = Animator::new(controller);
    animator.set_bool("done", true).map_err(|d| format!("{d:?}"))?;
    let step = animator.step(0.1);
    assert_eq!(step.state_info.state_name, "Run");
    animator.set_bool("back", true).map_err(|d| format!("{d:?}"))?;
    let step = animator.step(0.1);
    assert_eq!(step.state_info.state_name, "Walk");

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn malformed_direct_tree_names_faulting_member() -> Result<(), Box<dyn Error>> {
    let dir = unique_temp_dir("malformed_direct")?;
    let guids = write_fixture_assets(&dir)?;
    let yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!91 &9100000
AnimatorController:
  m_Name: BadDirect
  m_AnimatorParameters: []
  m_AnimatorLayers:
  - m_Name: Base Layer
    m_DefaultWeight: 1
    m_BlendingMode: 0
    m_SyncedLayerIndex: -1
    m_Mask: {fileID: 0}
    m_StateMachine: {fileID: 110700000}
--- !u!1107 &110700000
AnimatorStateMachine:
  m_Name: Base Layer
  m_ChildStates:
  - m_State: {fileID: 110200000}
  m_ChildStateMachines: []
  m_DefaultState: {fileID: 110200000}
--- !u!1102 &110200000
AnimatorState:
  m_Name: Broken
  m_Speed: 1
  m_Motion: {fileID: 206000000}
--- !u!206 &206000000
BlendTree:
  m_Name: BrokenTree
  m_BlendType: 4
  m_Childs:
  - m_Motion: {fileID: 7400000, guid: bb000000000000000000000000000002, type: 2}
    m_DirectBlendParameter:
";
    let controller_path = dir.join("BadDirect.controller");
    fs::write(&controller_path, yaml)?;
    match import_animator_controller(&controller_path, &guids) {
        Err(AnimImportError::MalformedController { member, reason, .. }) => {
            assert!(member.contains("m_DirectBlendParameter"), "member: {member}");
            assert!(reason.contains("empty"), "reason: {reason}");
        }
        other => return Err(format!("expected MalformedController, got {other:?}").into()),
    }

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn synced_layers_remain_unsupported_with_named_member() -> Result<(), Box<dyn Error>> {
    let dir = unique_temp_dir("synced")?;
    let guids = write_fixture_assets(&dir)?;
    let yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!91 &9100000
AnimatorController:
  m_Name: Synced
  m_AnimatorParameters: []
  m_AnimatorLayers:
  - m_Name: Base Layer
    m_DefaultWeight: 1
    m_BlendingMode: 0
    m_SyncedLayerIndex: -1
    m_Mask: {fileID: 0}
    m_StateMachine: {fileID: 110700000}
  - m_Name: Copy
    m_DefaultWeight: 1
    m_BlendingMode: 0
    m_SyncedLayerIndex: 0
    m_Mask: {fileID: 0}
    m_StateMachine: {fileID: 110700000}
--- !u!1107 &110700000
AnimatorStateMachine:
  m_Name: Base Layer
  m_ChildStates:
  - m_State: {fileID: 110200000}
  m_ChildStateMachines: []
  m_DefaultState: {fileID: 110200000}
--- !u!1102 &110200000
AnimatorState:
  m_Name: Idle
  m_Speed: 1
  m_Motion: {fileID: 7400000, guid: aa000000000000000000000000000001, type: 2}
";
    let controller_path = dir.join("Synced.controller");
    fs::write(&controller_path, yaml)?;
    match import_animator_controller(&controller_path, &guids) {
        Err(AnimImportError::UnsupportedConstruct { member, .. }) => {
            assert!(member.contains("m_SyncedLayerIndex"), "member: {member}");
        }
        other => return Err(format!("expected UnsupportedConstruct, got {other:?}").into()),
    }

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
