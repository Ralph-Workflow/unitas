use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_animation::curve::{AssetReference, ObjectReferenceCurve, ObjectReferenceKey};
use unitas_animation::{
    AnimationClip, AnimationEvent, AnimatorController, AnimatorState, ConditionOp, CurveBinding,
    CurveBindingTarget, FloatCurve, InterruptionSource, Keyframe, ParameterDef, PropertyCurve,
    StateMotion, StateTransition, TransitionCondition, TransitionTarget, WrapMode,
};

use super::{
    import_animation_clip, node_bool, node_f64, node_i32, node_sequence,
    normalize_multiline_flow_braces, AnimImportError, ImportedAnimationClip,
};
use crate::meta::GuidIndex;
use crate::unity_yaml::{parse, Document, YamlNode};

const ANIMATOR_CONTROLLER_CLASS_ID: u32 = 91;
const BLEND_TREE_CLASS_ID: u32 = 206;
const ANIMATOR_STATE_TRANSITION_CLASS_ID: u32 = 1101;
const ANIMATOR_STATE_CLASS_ID: u32 = 1102;
const ANIMATOR_STATE_MACHINE_CLASS_ID: u32 = 1107;
const ANIMATOR_TRANSITION_CLASS_ID: u32 = 1109;

pub fn import_animator_controller(
    path: &Path,
    guids: &GuidIndex,
) -> Result<AnimatorController, AnimImportError> {
    let asset_label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("AnimatorController"))
        .to_string();
    let bytes = fs::read(path).map_err(|error| AnimImportError::Read {
        asset: asset_label.clone(),
        message: error.to_string(),
    })?;
    let without_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let raw_text = std::str::from_utf8(without_bom).map_err(|_| AnimImportError::InvalidUtf8)?;
    let normalized = normalize_multiline_flow_braces(raw_text);
    let stream = parse(&normalized)?;

    let mut docs_by_id: BTreeMap<i64, &Document> = BTreeMap::new();
    let mut controller_doc: Option<&Document> = None;
    let mut first_state_machine_id: Option<i64> = None;

    for doc in &stream.documents {
        let file_id = doc.file_id;
        if file_id != 0 {
            docs_by_id.insert(file_id, doc);
        }
        if doc.class_id == ANIMATOR_CONTROLLER_CLASS_ID
            || doc.root.get("AnimatorController").is_some()
        {
            controller_doc = Some(doc);
        } else if (doc.class_id == ANIMATOR_STATE_MACHINE_CLASS_ID
            || doc.root.get("AnimatorStateMachine").is_some())
            && first_state_machine_id.is_none()
            && file_id != 0
        {
            first_state_machine_id = Some(file_id);
        }
    }

    let controller_doc =
        controller_doc.ok_or_else(|| AnimImportError::MissingAnimatorControllerDocument {
            asset: asset_label.clone(),
        })?;
    let controller_node = controller_doc
        .root
        .get("AnimatorController")
        .unwrap_or(&controller_doc.root);
    if controller_node.as_mapping().is_none() {
        return Err(AnimImportError::MissingAnimatorControllerDocument { asset: asset_label });
    }

    let fallback_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&asset_label)
        .to_string();
    let name = controller_node
        .get("m_Name")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|s: &&str| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(fallback_name);

    let parameters = parse_parameters(controller_node, &asset_label)?;
    let state_machine_id =
        resolve_state_machine_id(controller_node, first_state_machine_id, &asset_label)?;
    let state_machine_doc = docs_by_id.get(&state_machine_id).copied().ok_or_else(|| {
        AnimImportError::UnresolvedReference {
            asset: asset_label.clone(),
            member: "m_AnimatorStateMachines".to_string(),
            reference: format!("fileID {state_machine_id}"),
        }
    })?;
    let state_machine_node = state_machine_doc
        .root
        .get("AnimatorStateMachine")
        .unwrap_or(&state_machine_doc.root);

    check_sub_state_machines(state_machine_node, &asset_label)?;

    let state_ids = parse_child_state_ids(state_machine_node, &asset_label)?;
    if state_ids.is_empty() {
        return Err(AnimImportError::MalformedController {
            asset: asset_label,
            member: "m_ChildStates".to_string(),
            reason: "AnimatorStateMachine defines no child states".to_string(),
        });
    }

    let mut state_index_by_id: BTreeMap<i64, usize> = BTreeMap::new();
    for (idx, state_id) in state_ids.iter().copied().enumerate() {
        state_index_by_id.insert(state_id, idx);
    }

    let mut states = Vec::with_capacity(state_ids.len());
    let mut transitions = Vec::new();

    if let Some(any_items) = node_sequence(state_machine_node.get("m_AnyStateTransitions"))? {
        for item in any_items {
            let trans_id = extract_file_id(item);
            if trans_id == 0 {
                continue;
            }
            let transition = parse_transition(
                trans_id,
                None,
                &docs_by_id,
                &state_index_by_id,
                &asset_label,
            )?;
            transitions.push(transition);
        }
    }

    for (source_idx, state_id) in state_ids.iter().copied().enumerate() {
        let state_doc = docs_by_id.get(&state_id).copied().ok_or_else(|| {
            AnimImportError::UnresolvedReference {
                asset: asset_label.clone(),
                member: "m_ChildStates".to_string(),
                reference: format!("fileID {state_id}"),
            }
        })?;
        if state_doc.class_id != 0
            && state_doc.class_id != ANIMATOR_STATE_CLASS_ID
            && state_doc.root.get("AnimatorState").is_none()
        {
            return Err(AnimImportError::MalformedController {
                asset: asset_label,
                member: format!("state {state_id}"),
                reason: format!(
                    "expected class 1102 AnimatorState, found class {}",
                    state_doc.class_id
                ),
            });
        }
        let state_node = state_doc
            .root
            .get("AnimatorState")
            .unwrap_or(&state_doc.root);
        let (state, state_transitions) = parse_state(
            state_id,
            source_idx,
            state_node,
            &docs_by_id,
            &state_index_by_id,
            guids,
            &asset_label,
        )?;
        states.push(state);
        transitions.extend(state_transitions);
    }

    let entry_state = resolve_entry_state(
        state_machine_node,
        &docs_by_id,
        &state_index_by_id,
        &asset_label,
    )?;

    Ok(AnimatorController::new(
        name,
        parameters,
        states,
        entry_state,
        transitions,
    ))
}

fn parse_parameters(
    controller_node: &YamlNode,
    asset_label: &str,
) -> Result<Vec<ParameterDef>, AnimImportError> {
    let Some(items) = node_sequence(controller_node.get("m_AnimatorParameters"))? else {
        return Ok(Vec::new());
    };
    let mut parameters = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if item.as_mapping().is_none() {
            return Err(AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member: format!("m_AnimatorParameters[{index}]"),
                reason: "parameter entry is not a mapping".to_string(),
            });
        }
        let param_name = item
            .get("m_Name")
            .and_then(YamlNode::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member: format!("m_AnimatorParameters[{index}].m_Name"),
                reason: "parameter name is empty".to_string(),
            })?
            .to_string();
        let param_type = node_i32(item, "m_Type", 0);
        let def = match param_type {
            1 => {
                let default = node_f64(item, "m_DefaultFloat", 0.0) as f32;
                ParameterDef::float(param_name, default)
            }
            3 => {
                let default = node_i32(item, "m_DefaultInt", 0);
                ParameterDef::int(param_name, default)
            }
            4 => {
                let default = node_bool(item, "m_DefaultBool", false);
                ParameterDef::bool(param_name, default)
            }
            9 => ParameterDef::trigger(param_name),
            other => {
                return Err(AnimImportError::UnsupportedConstruct {
                    asset: asset_label.to_string(),
                    member: format!("m_AnimatorParameters[{index}] ({param_name})"),
                    reason: format!("unsupported AnimatorControllerParameterType {other}"),
                });
            }
        };
        parameters.push(def);
    }
    Ok(parameters)
}

fn resolve_state_machine_id(
    controller_node: &YamlNode,
    first_state_machine_id: Option<i64>,
    asset_label: &str,
) -> Result<i64, AnimImportError> {
    let mut layer_machine_id = 0i64;
    if let Some(layers) = node_sequence(controller_node.get("m_AnimatorLayers"))? {
        if layers.len() > 1 {
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: "m_AnimatorLayers".to_string(),
                reason: format!(
                    "multiple animator layers ({}) are not supported in single-layer AnimatorController",
                    layers.len()
                ),
            });
        }
        if let Some(layer) = layers.first() {
            if let Some(nested) = node_sequence(layer.get("m_AnimatorLayers"))? {
                if !nested.is_empty() {
                    return Err(AnimImportError::UnsupportedConstruct {
                        asset: asset_label.to_string(),
                        member: "m_AnimatorLayers[0].m_AnimatorLayers".to_string(),
                        reason: "nested animator layers are not supported".to_string(),
                    });
                }
            }
            if let Some(sm_node) = layer.get("m_StateMachine") {
                layer_machine_id = extract_file_id(sm_node);
            }
        }
    }
    if layer_machine_id != 0 {
        return Ok(layer_machine_id);
    }
    if let Some(machines) = node_sequence(controller_node.get("m_AnimatorStateMachines"))? {
        if machines.len() > 1 {
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: "m_AnimatorStateMachines".to_string(),
                reason: format!(
                    "multiple state machines ({}) are not supported",
                    machines.len()
                ),
            });
        }
        if let Some(first) = machines.first() {
            let id = extract_file_id(first);
            if id != 0 {
                return Ok(id);
            }
        }
    }
    first_state_machine_id.ok_or_else(|| AnimImportError::MalformedController {
        asset: asset_label.to_string(),
        member: "m_AnimatorLayers".to_string(),
        reason: "no AnimatorStateMachine (class 1107) referenced or present".to_string(),
    })
}

fn check_sub_state_machines(
    state_machine_node: &YamlNode,
    asset_label: &str,
) -> Result<(), AnimImportError> {
    if let Some(sub_machines) = node_sequence(state_machine_node.get("m_ChildStateMachines"))? {
        if !sub_machines.is_empty() {
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: "AnimatorStateMachine.m_ChildStateMachines".to_string(),
                reason: "sub-state machines are not supported".to_string(),
            });
        }
    }
    Ok(())
}

fn parse_child_state_ids(
    state_machine_node: &YamlNode,
    asset_label: &str,
) -> Result<Vec<i64>, AnimImportError> {
    let Some(items) = node_sequence(state_machine_node.get("m_ChildStates"))? else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let state_ref = item.get("m_State").unwrap_or(item);
        let id = extract_file_id(state_ref);
        if id == 0 {
            return Err(AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member: format!("m_ChildStates[{index}]"),
                reason: "child state reference has missing or zero fileID".to_string(),
            });
        }
        ids.push(id);
    }
    Ok(ids)
}

fn resolve_entry_state(
    state_machine_node: &YamlNode,
    docs_by_id: &BTreeMap<i64, &Document>,
    state_index_by_id: &BTreeMap<i64, usize>,
    asset_label: &str,
) -> Result<usize, AnimImportError> {
    if let Some(default_node) = state_machine_node.get("m_DefaultState") {
        let default_id = extract_file_id(default_node);
        if default_id != 0 {
            return state_index_by_id.get(&default_id).copied().ok_or_else(|| {
                AnimImportError::UnresolvedReference {
                    asset: asset_label.to_string(),
                    member: "AnimatorStateMachine.m_DefaultState".to_string(),
                    reference: format!("fileID {default_id}"),
                }
            });
        }
    }
    if let Some(entry_transitions) = node_sequence(state_machine_node.get("m_EntryTransitions"))? {
        for entry_ref in entry_transitions {
            let trans_id = extract_file_id(entry_ref);
            if trans_id == 0 {
                continue;
            }
            let trans_doc = docs_by_id.get(&trans_id).copied().ok_or_else(|| {
                AnimImportError::UnresolvedReference {
                    asset: asset_label.to_string(),
                    member: "AnimatorStateMachine.m_EntryTransitions".to_string(),
                    reference: format!("fileID {trans_id}"),
                }
            })?;
            let trans_node = trans_doc
                .root
                .get("AnimatorTransition")
                .or_else(|| trans_doc.root.get("AnimatorStateTransition"))
                .unwrap_or(&trans_doc.root);
            let _ = ANIMATOR_TRANSITION_CLASS_ID;
            if let Some(dst_node) = trans_node
                .get("m_DstState")
                .or_else(|| trans_node.get("m_DestinationState"))
            {
                let dst_id = extract_file_id(dst_node);
                if dst_id != 0 {
                    return state_index_by_id.get(&dst_id).copied().ok_or_else(|| {
                        AnimImportError::UnresolvedReference {
                            asset: asset_label.to_string(),
                            member: "AnimatorStateMachine.m_EntryTransitions.m_DestinationState"
                                .to_string(),
                            reference: format!("fileID {dst_id}"),
                        }
                    });
                }
            }
        }
    }
    Ok(0)
}

fn parse_state(
    state_id: i64,
    source_idx: usize,
    state_node: &YamlNode,
    docs_by_id: &BTreeMap<i64, &Document>,
    state_index_by_id: &BTreeMap<i64, usize>,
    guids: &GuidIndex,
    asset_label: &str,
) -> Result<(AnimatorState, Vec<StateTransition>), AnimImportError> {
    if let Some(child_states) = node_sequence(state_node.get("m_ChildStates"))? {
        if !child_states.is_empty() {
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: format!("AnimatorState({state_id}).m_ChildStates"),
                reason: "sub-state machines on AnimatorState are not supported".to_string(),
            });
        }
    }
    let state_name = state_node
        .get("m_Name")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("State_{state_id}"));
    let speed = node_f64(state_node, "m_Speed", 1.0) as f32;
    let write_defaults = node_bool(state_node, "m_WriteDefaultValues", true);
    let state_loop_override = state_node
        .get("m_Loop")
        .or_else(|| state_node.get("m_LoopTime"))
        .map(|_| {
            node_bool(state_node, "m_Loop", false) || node_bool(state_node, "m_LoopTime", false)
        });

    let motion = parse_state_motion(
        &state_name,
        state_node.get("m_Motion"),
        state_loop_override,
        docs_by_id,
        guids,
        asset_label,
    )?;

    let mut state = AnimatorState::with_speed(state_name.clone(), motion, speed);
    state.write_defaults = write_defaults;

    let mut state_transitions = Vec::new();
    if let Some(trans_items) = node_sequence(state_node.get("m_Transitions"))? {
        for item in trans_items {
            let trans_id = extract_file_id(item);
            if trans_id == 0 {
                continue;
            }
            let transition = parse_transition(
                trans_id,
                Some(source_idx),
                docs_by_id,
                state_index_by_id,
                asset_label,
            )?;
            state_transitions.push(transition);
        }
    }

    Ok((state, state_transitions))
}

fn parse_state_motion(
    state_name: &str,
    motion_node: Option<&YamlNode>,
    state_loop_override: Option<bool>,
    docs_by_id: &BTreeMap<i64, &Document>,
    guids: &GuidIndex,
    asset_label: &str,
) -> Result<StateMotion, AnimImportError> {
    let Some(motion_node) = motion_node else {
        return Ok(StateMotion::None);
    };
    let file_id = extract_file_id(motion_node);
    let guid = extract_guid(motion_node);

    if let Some(guid_str) = guid {
        let clip = load_clip_by_guid(
            &guid_str,
            state_loop_override,
            guids,
            asset_label,
            &format!("AnimatorState({state_name}).m_Motion"),
        )?;
        return Ok(StateMotion::Clip(clip));
    }

    if file_id == 0 {
        return Ok(StateMotion::None);
    }

    let target_doc =
        docs_by_id
            .get(&file_id)
            .copied()
            .ok_or_else(|| AnimImportError::UnresolvedReference {
                asset: asset_label.to_string(),
                member: format!("AnimatorState({state_name}).m_Motion"),
                reference: format!("fileID {file_id}"),
            })?;

    if target_doc.class_id == BLEND_TREE_CLASS_ID || target_doc.root.get("BlendTree").is_some() {
        let tree_node = target_doc.root.get("BlendTree").unwrap_or(&target_doc.root);
        return parse_blend_tree(
            state_name,
            file_id,
            tree_node,
            state_loop_override,
            guids,
            asset_label,
        );
    }

    Err(AnimImportError::UnsupportedConstruct {
        asset: asset_label.to_string(),
        member: format!("AnimatorState({state_name}).m_Motion"),
        reason: format!(
            "unsupported in-file motion class {} for fileID {file_id}",
            target_doc.class_id
        ),
    })
}

fn parse_blend_tree(
    state_name: &str,
    tree_id: i64,
    tree_node: &YamlNode,
    state_loop_override: Option<bool>,
    guids: &GuidIndex,
    asset_label: &str,
) -> Result<StateMotion, AnimImportError> {
    let tree_name = tree_node
        .get("m_Name")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(state_name);
    let blend_type = node_i32(tree_node, "m_BlendType", 0);
    if blend_type != 0 {
        let kind = match blend_type {
            1 => "2D Simple Directional",
            2 => "2D Freeform Directional",
            3 => "2D Freeform Cartesian",
            4 => "Direct",
            _ => "non-1D",
        };
        return Err(AnimImportError::UnsupportedConstruct {
            asset: asset_label.to_string(),
            member: format!("BlendTree({tree_name}, fileID {tree_id}).m_BlendType"),
            reason: format!("{kind} blend tree (m_BlendType: {blend_type}) is not supported"),
        });
    }

    let parameter = tree_node
        .get("m_BlendParameter")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AnimImportError::MalformedController {
            asset: asset_label.to_string(),
            member: format!("BlendTree({tree_name}).m_BlendParameter"),
            reason: "1D BlendTree has empty m_BlendParameter".to_string(),
        })?
        .to_string();

    let Some(children) = node_sequence(tree_node.get("m_Childs"))? else {
        return Err(AnimImportError::MalformedController {
            asset: asset_label.to_string(),
            member: format!("BlendTree({tree_name}).m_Childs"),
            reason: "1D BlendTree has no children".to_string(),
        });
    };
    if children.is_empty() {
        return Err(AnimImportError::MalformedController {
            asset: asset_label.to_string(),
            member: format!("BlendTree({tree_name}).m_Childs"),
            reason: "1D BlendTree has empty m_Childs".to_string(),
        });
    }

    let use_auto = node_bool(tree_node, "m_UseAutomaticThresholds", false);
    let min_threshold = node_f64(tree_node, "m_MinThreshold", 0.0) as f32;
    let max_threshold = node_f64(tree_node, "m_MaxThreshold", 1.0) as f32;
    let count = children.len();
    let mut thresholds = Vec::with_capacity(count);

    for (idx, child) in children.iter().enumerate() {
        let member = format!("BlendTree({tree_name}).m_Childs[{idx}]");
        let Some(motion_node) = child.get("m_Motion") else {
            return Err(AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member,
                reason: "ChildMotion is missing m_Motion".to_string(),
            });
        };
        let Some(guid_str) = extract_guid(motion_node) else {
            let child_file_id = extract_file_id(motion_node);
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: format!("{member}.m_Motion"),
                reason: format!(
                    "BlendTree child motion must reference an AnimationClip GUID (found fileID {child_file_id})"
                ),
            });
        };
        let clip = load_clip_by_guid(
            &guid_str,
            state_loop_override,
            guids,
            asset_label,
            &format!("{member}.m_Motion"),
        )?;
        let threshold = if child.get("m_Threshold").is_some() {
            node_f64(child, "m_Threshold", 0.0) as f32
        } else if use_auto && count > 1 {
            let frac = (idx as f32) / ((count - 1) as f32);
            min_threshold + (max_threshold - min_threshold) * frac
        } else {
            min_threshold
        };
        thresholds.push((threshold, clip));
    }

    Ok(StateMotion::BlendTree1D {
        parameter,
        thresholds,
    })
}

fn parse_transition(
    trans_id: i64,
    source_state: Option<usize>,
    docs_by_id: &BTreeMap<i64, &Document>,
    state_index_by_id: &BTreeMap<i64, usize>,
    asset_label: &str,
) -> Result<StateTransition, AnimImportError> {
    let trans_doc =
        docs_by_id
            .get(&trans_id)
            .copied()
            .ok_or_else(|| AnimImportError::UnresolvedReference {
                asset: asset_label.to_string(),
                member: "m_Transitions".to_string(),
                reference: format!("fileID {trans_id}"),
            })?;
    if trans_doc.class_id != 0
        && trans_doc.class_id != ANIMATOR_STATE_TRANSITION_CLASS_ID
        && trans_doc.root.get("AnimatorStateTransition").is_none()
    {
        return Err(AnimImportError::MalformedController {
            asset: asset_label.to_string(),
            member: format!("transition {trans_id}"),
            reason: format!(
                "expected class 1101 AnimatorStateTransition, found class {}",
                trans_doc.class_id
            ),
        });
    }
    let trans_node = trans_doc
        .root
        .get("AnimatorStateTransition")
        .unwrap_or(&trans_doc.root);
    let member_prefix = format!("AnimatorStateTransition({trans_id})");

    let is_exit = node_bool(trans_node, "m_IsExit", false);
    let target = if is_exit {
        TransitionTarget::Exit
    } else {
        let dst_node = trans_node
            .get("m_DstState")
            .or_else(|| trans_node.get("m_DestinationState"))
            .ok_or_else(|| AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member: format!("{member_prefix}.m_DestinationState"),
                reason: "transition has neither m_IsExit nor m_DestinationState/m_DstState"
                    .to_string(),
            })?;
        let dst_id = extract_file_id(dst_node);
        if dst_id == 0 {
            return Err(AnimImportError::MalformedController {
                asset: asset_label.to_string(),
                member: format!("{member_prefix}.m_DestinationState"),
                reason: "destination state fileID is 0".to_string(),
            });
        }
        let dst_index = state_index_by_id.get(&dst_id).copied().ok_or_else(|| {
            AnimImportError::UnresolvedReference {
                asset: asset_label.to_string(),
                member: format!("{member_prefix}.m_DestinationState"),
                reference: format!("fileID {dst_id}"),
            }
        })?;
        TransitionTarget::State(dst_index)
    };

    let has_exit_time = node_bool(trans_node, "m_HasExitTime", false);
    let exit_time = node_f64(trans_node, "m_ExitTime", 0.0) as f32;
    let duration = if trans_node.get("m_TransitionDuration").is_some() {
        node_f64(trans_node, "m_TransitionDuration", 0.0) as f32
    } else {
        node_f64(trans_node, "m_Duration", 0.0) as f32
    };
    let can_transition_to_self = node_bool(trans_node, "m_CanTransitionToSelf", true);
    let interruption_source = match node_i32(trans_node, "m_InterruptionSource", 0) {
        0 => InterruptionSource::None,
        1 => InterruptionSource::Source,
        2 => InterruptionSource::Destination,
        3 => InterruptionSource::SourceThenDestination,
        4 => InterruptionSource::DestinationThenSource,
        other => {
            return Err(AnimImportError::UnsupportedConstruct {
                asset: asset_label.to_string(),
                member: format!("{member_prefix}.m_InterruptionSource"),
                reason: format!("unsupported TransitionInterruptionSource {other}"),
            });
        }
    };

    let mut conditions = Vec::new();
    if let Some(cond_items) = node_sequence(trans_node.get("m_Conditions"))? {
        for (idx, cond_node) in cond_items.iter().enumerate() {
            let cond_member = format!("{member_prefix}.m_Conditions[{idx}]");
            let parameter = cond_node
                .get("m_ConditionEvent")
                .and_then(YamlNode::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AnimImportError::MalformedController {
                    asset: asset_label.to_string(),
                    member: format!("{cond_member}.m_ConditionEvent"),
                    reason: "condition parameter name is empty".to_string(),
                })?
                .to_string();
            let mode = node_i32(cond_node, "m_ConditionMode", 0);
            let threshold_f = node_f64(cond_node, "m_ConditionThreshold", 0.0);
            let op = match mode {
                1 => ConditionOp::If,
                2 => ConditionOp::IfNot,
                3 => ConditionOp::Greater(threshold_f as f32),
                4 => ConditionOp::Less(threshold_f as f32),
                6 => ConditionOp::Equals(threshold_f.round() as i32),
                7 => ConditionOp::NotEqual(threshold_f.round() as i32),
                other => {
                    return Err(AnimImportError::UnsupportedConstruct {
                        asset: asset_label.to_string(),
                        member: format!("{cond_member}.m_ConditionMode"),
                        reason: format!("unsupported AnimatorConditionMode {other}"),
                    });
                }
            };
            conditions.push(TransitionCondition::new(parameter, op));
        }
    }

    let mut transition = match source_state {
        Some(src_idx) => StateTransition::new(
            src_idx,
            target,
            conditions,
            has_exit_time,
            exit_time,
            duration,
        ),
        None => {
            if has_exit_time {
                StateTransition::any_state_with_exit_time(target, conditions, exit_time, duration)
            } else {
                let mut t = StateTransition::any_state(target, conditions);
                t.duration = if duration.is_finite() {
                    duration.max(0.0)
                } else {
                    0.0
                };
                t
            }
        }
    };
    transition.interruption_source = interruption_source;
    transition = transition.with_can_transition_to_self(can_transition_to_self);
    Ok(transition)
}

fn load_clip_by_guid(
    guid: &str,
    state_loop_override: Option<bool>,
    guids: &GuidIndex,
    asset_label: &str,
    member: &str,
) -> Result<AnimationClip, AnimImportError> {
    let trimmed = guid.trim();
    let clip_path: PathBuf = guids
        .get(trimmed)
        .or_else(|| guids.get(&trimmed.to_ascii_lowercase()))
        .map(Path::to_path_buf)
        .ok_or_else(|| AnimImportError::UnresolvedReference {
            asset: asset_label.to_string(),
            member: member.to_string(),
            reference: format!("guid {trimmed}"),
        })?;
    let clip_label = clip_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .to_string();
    let clip_bytes = fs::read(&clip_path).map_err(|error| AnimImportError::Read {
        asset: clip_label.clone(),
        message: error.to_string(),
    })?;
    let imported = import_animation_clip(&clip_bytes, &clip_label)?;
    convert_imported_clip(imported, state_loop_override, &clip_label)
}

fn convert_imported_clip(
    imported: ImportedAnimationClip,
    state_loop_override: Option<bool>,
    clip_label: &str,
) -> Result<AnimationClip, AnimImportError> {
    let settings_span = (imported.settings.stop_time - imported.settings.start_time) as f32;
    let duration = if settings_span.is_finite() && settings_span > 0.0 {
        settings_span
    } else {
        0.0
    };
    let sample_rate = imported.sample_rate as f32;
    let is_loop = match state_loop_override {
        Some(flag) => flag && imported.settings.loop_time,
        None => imported.settings.loop_time,
    };
    let wrap_mode = if is_loop || imported.wrap_mode == 2 {
        WrapMode::Loop
    } else if imported.wrap_mode == 4 {
        WrapMode::PingPong
    } else if imported.wrap_mode == 8 {
        WrapMode::ClampForever
    } else {
        WrapMode::Once
    };

    let mut bindings = Vec::new();
    for pos in &imported.position_curves {
        bindings.push(CurveBinding::new(
            CurveBindingTarget::transform(pos.path.clone(), "localPosition"),
            PropertyCurve::Vector3(vec3_keyframes_to_curves(&pos.keyframes)),
        ));
    }
    for rot in &imported.rotation_curves {
        bindings.push(CurveBinding::new(
            CurveBindingTarget::transform(rot.path.clone(), "localRotation"),
            PropertyCurve::Quaternion(quat_keyframes_to_curves(&rot.keyframes)),
        ));
    }
    for scale in &imported.scale_curves {
        bindings.push(CurveBinding::new(
            CurveBindingTarget::transform(scale.path.clone(), "localScale"),
            PropertyCurve::Vector3(vec3_keyframes_to_curves(&scale.keyframes)),
        ));
    }
    for fc in &imported.float_curves {
        let component = class_id_to_component_name(fc.class_id, clip_label, &fc.attribute)?;
        let keys = fc
            .keyframes
            .iter()
            .map(|kf| {
                Keyframe::new(
                    kf.time as f32,
                    kf.value as f32,
                    kf.in_slope as f32,
                    kf.out_slope as f32,
                )
            })
            .collect();
        bindings.push(CurveBinding::float(
            CurveBindingTarget::new(fc.path.clone(), component, fc.attribute.clone()),
            FloatCurve::new(keys),
        ));
    }
    for pptr in &imported.pptr_curves {
        let component = class_id_to_component_name(pptr.class_id, clip_label, &pptr.attribute)?;
        let mut keys = Vec::with_capacity(pptr.keyframes.len());
        for (idx, kf) in pptr.keyframes.iter().enumerate() {
            let reference = match kf.guid.as_deref() {
                Some(guid_str) => {
                    AssetReference::from_hex(guid_str, kf.file_id).ok_or_else(|| {
                        AnimImportError::MalformedController {
                            asset: clip_label.to_string(),
                            member: format!("m_PPtrCurves[{}].curve[{idx}]", pptr.attribute),
                            reason: format!("invalid asset GUID '{guid_str}'"),
                        }
                    })?
                }
                None if kf.file_id == 0 => AssetReference::NULL,
                None => {
                    return Err(AnimImportError::MalformedController {
                        asset: clip_label.to_string(),
                        member: format!("m_PPtrCurves[{}].curve[{idx}]", pptr.attribute),
                        reason: format!(
                            "PPtr keyframe with non-zero fileID {} is missing a GUID",
                            kf.file_id
                        ),
                    });
                }
            };
            keys.push(ObjectReferenceKey::new(kf.time as f32, reference));
        }
        bindings.push(CurveBinding::new(
            CurveBindingTarget::new(pptr.path.clone(), component, pptr.attribute.clone()),
            PropertyCurve::ObjectReference(ObjectReferenceCurve::new(keys)),
        ));
    }

    let events = imported
        .events
        .into_iter()
        .map(|ev| AnimationEvent {
            time: ev.time as f32,
            function_name: ev.function_name,
            float_parameter: ev.float_parameter as f32,
            int_parameter: ev.int_parameter,
            string_parameter: ev.data,
        })
        .collect();

    Ok(AnimationClip::new(
        imported.name,
        duration,
        sample_rate,
        wrap_mode,
        bindings,
        events,
    ))
}

fn class_id_to_component_name(
    class_id: i32,
    clip_label: &str,
    attribute: &str,
) -> Result<&'static str, AnimImportError> {
    match class_id {
        1 => Ok("GameObject"),
        4 | 224 => Ok("Transform"),
        23 => Ok("MeshRenderer"),
        25 => Ok("Renderer"),
        108 => Ok("Light"),
        114 => Ok("MonoBehaviour"),
        137 => Ok("SkinnedMeshRenderer"),
        212 => Ok("SpriteRenderer"),
        225 => Ok("CanvasGroup"),
        other => Err(AnimImportError::UnsupportedConstruct {
            asset: clip_label.to_string(),
            member: format!("curve({attribute}).classID"),
            reason: format!("unsupported curve classID {other}"),
        }),
    }
}

fn vec3_keyframes_to_curves(keyframes: &[super::AnimKeyframeVec3]) -> [FloatCurve; 3] {
    let mut x = Vec::with_capacity(keyframes.len());
    let mut y = Vec::with_capacity(keyframes.len());
    let mut z = Vec::with_capacity(keyframes.len());
    for kf in keyframes {
        let t = kf.time as f32;
        x.push(Keyframe::new(
            t,
            kf.value[0] as f32,
            kf.in_slope[0] as f32,
            kf.out_slope[0] as f32,
        ));
        y.push(Keyframe::new(
            t,
            kf.value[1] as f32,
            kf.in_slope[1] as f32,
            kf.out_slope[1] as f32,
        ));
        z.push(Keyframe::new(
            t,
            kf.value[2] as f32,
            kf.in_slope[2] as f32,
            kf.out_slope[2] as f32,
        ));
    }
    [FloatCurve::new(x), FloatCurve::new(y), FloatCurve::new(z)]
}

fn quat_keyframes_to_curves(keyframes: &[super::AnimKeyframeQuat]) -> [FloatCurve; 4] {
    let mut x = Vec::with_capacity(keyframes.len());
    let mut y = Vec::with_capacity(keyframes.len());
    let mut z = Vec::with_capacity(keyframes.len());
    let mut w = Vec::with_capacity(keyframes.len());
    for kf in keyframes {
        let t = kf.time as f32;
        x.push(Keyframe::new(
            t,
            kf.value[0] as f32,
            kf.in_slope[0] as f32,
            kf.out_slope[0] as f32,
        ));
        y.push(Keyframe::new(
            t,
            kf.value[1] as f32,
            kf.in_slope[1] as f32,
            kf.out_slope[1] as f32,
        ));
        z.push(Keyframe::new(
            t,
            kf.value[2] as f32,
            kf.in_slope[2] as f32,
            kf.out_slope[2] as f32,
        ));
        w.push(Keyframe::new(
            t,
            kf.value[3] as f32,
            kf.in_slope[3] as f32,
            kf.out_slope[3] as f32,
        ));
    }
    [
        FloatCurve::new(x),
        FloatCurve::new(y),
        FloatCurve::new(z),
        FloatCurve::new(w),
    ]
}

fn extract_file_id(node: &YamlNode) -> i64 {
    node.get("fileID").and_then(YamlNode::as_i64).unwrap_or(0)
}

fn extract_guid(node: &YamlNode) -> Option<String> {
    node.get("guid")
        .and_then(YamlNode::as_str)
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_animation::controller::TransitionSource;
    use unitas_animation::{Animator, ParameterValue};

    fn unique_temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "unitas_controller_import_{label}_{}_{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn imports_wick_controller_from_2d_platformer_sample() -> Result<(), Box<dyn std::error::Error>>
    {
        let sample_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/2d-platformer/Assets");
        let guids = GuidIndex::build(&sample_root)?;
        let controller_path = sample_root.join("Animation/Wick.controller");

        let controller = import_animator_controller(&controller_path, &guids)?;
        assert_eq!(controller.name, "Wick");
        assert_eq!(
            controller.parameters,
            vec![
                ParameterDef::float("Speed", 0.0),
                ParameterDef::bool("Grounded", true),
                ParameterDef::trigger("Hurt"),
                ParameterDef::trigger("Cheer"),
            ]
        );
        assert_eq!(controller.entry_state, 0);
        let state_names: Vec<&str> = controller.states.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            state_names,
            vec!["Idle", "Run", "Jump", "Fall", "Land", "Hurt", "Cheer"]
        );

        match &controller.states[1].motion {
            StateMotion::BlendTree1D {
                parameter,
                thresholds,
            } => {
                assert_eq!(parameter, "Speed");
                assert_eq!(thresholds.len(), 2);
                assert_eq!(thresholds[0].0, 0.0);
                assert_eq!(thresholds[0].1.name, "Idle");
                assert_eq!(thresholds[1].0, 1.0);
                assert_eq!(thresholds[1].1.name, "Run");
                assert_eq!(thresholds[1].1.events.len(), 2);
                assert_eq!(thresholds[1].1.events[0].function_name, "OnFootstep");
                assert!((thresholds[1].1.events[0].time - 0.125).abs() < 1e-6);
                assert!((thresholds[1].1.events[1].time - 0.375).abs() < 1e-6);
            }
            other => return Err(format!("expected 1D blend tree on Run, found {other:?}").into()),
        }

        assert!(controller.validate().is_empty());

        let mut animator = Animator::new(controller);
        let _ = animator.set_float("Speed", 1.0);
        let mut fired = Vec::new();
        for _ in 0..30 {
            let step = animator.step(0.02);
            fired.extend(step.fired_events.into_iter().map(|e| e.function_name));
        }
        assert!(fired.iter().any(|name| name == "OnFootstep"));
        Ok(())
    }

    #[test]
    fn diagnoses_2d_blend_tree_and_sub_state_machine_and_unresolved_guid(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = unique_temp_dir("diagnostics")?;
        let guids = GuidIndex::default();

        let blend_2d_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!91 &9100000\nAnimatorController:\n  m_Name: TwoD\n  m_AnimatorParameters:\n  - m_Name: MoveX\n    m_Type: 1\n    m_DefaultFloat: 0\n  m_AnimatorLayers:\n  - m_Name: Base Layer\n    m_StateMachine: {fileID: 110700000}\n--- !u!1107 &110700000\nAnimatorStateMachine:\n  m_Name: Base Layer\n  m_ChildStates:\n  - m_State: {fileID: 110200000}\n  m_DefaultState: {fileID: 110200000}\n  m_ChildStateMachines: []\n--- !u!1102 &110200000\nAnimatorState:\n  m_Name: Move\n  m_Speed: 1\n  m_Motion: {fileID: 206000000}\n--- !u!206 &206000000\nBlendTree:\n  m_Name: Move2D\n  m_BlendType: 1\n  m_BlendParameter: MoveX\n  m_Childs: []\n";
        let blend_2d_path = dir.join("TwoD.controller");
        std::fs::write(&blend_2d_path, blend_2d_yaml)?;
        let err = import_animator_controller(&blend_2d_path, &guids);
        match err {
            Err(AnimImportError::UnsupportedConstruct {
                asset,
                member,
                reason,
            }) => {
                assert_eq!(asset, "TwoD.controller");
                assert!(member.contains("Move2D"));
                assert!(reason.contains("2D"));
            }
            other => {
                return Err(format!(
                    "expected UnsupportedConstruct for 2D blend tree, got {other:?}"
                )
                .into())
            }
        }

        let sub_sm_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!91 &9100000\nAnimatorController:\n  m_Name: SubSm\n  m_AnimatorParameters: []\n  m_AnimatorLayers:\n  - m_Name: Base Layer\n    m_StateMachine: {fileID: 110700000}\n--- !u!1107 &110700000\nAnimatorStateMachine:\n  m_Name: Base Layer\n  m_ChildStates: []\n  m_ChildStateMachines:\n  - m_StateMachine: {fileID: 110700001}\n";
        let sub_sm_path = dir.join("SubSm.controller");
        std::fs::write(&sub_sm_path, sub_sm_yaml)?;
        match import_animator_controller(&sub_sm_path, &guids) {
            Err(AnimImportError::UnsupportedConstruct { asset, member, .. }) => {
                assert_eq!(asset, "SubSm.controller");
                assert!(member.contains("m_ChildStateMachines"));
            }
            other => {
                return Err(format!(
                    "expected UnsupportedConstruct for sub-state machine, got {other:?}"
                )
                .into())
            }
        }

        let missing_guid_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!91 &9100000\nAnimatorController:\n  m_Name: MissingClip\n  m_AnimatorParameters: []\n  m_AnimatorLayers:\n  - m_Name: Base Layer\n    m_StateMachine: {fileID: 110700000}\n--- !u!1107 &110700000\nAnimatorStateMachine:\n  m_Name: Base Layer\n  m_ChildStates:\n  - {fileID: 110200000}\n  m_DefaultState: {fileID: 110200000}\n--- !u!1102 &110200000\nAnimatorState:\n  m_Name: Idle\n  m_Motion: {fileID: 7400000, guid: 70000000000000000000000000000099, type: 2}\n";
        let missing_path = dir.join("MissingClip.controller");
        std::fs::write(&missing_path, missing_guid_yaml)?;
        match import_animator_controller(&missing_path, &guids) {
            Err(AnimImportError::UnresolvedReference {
                asset,
                member,
                reference,
            }) => {
                assert_eq!(asset, "MissingClip.controller");
                assert!(member.contains("Idle"));
                assert!(reference.contains("70000000000000000000000000000099"));
            }
            other => return Err(format!("expected UnresolvedReference, got {other:?}").into()),
        }

        let _ = std::fs::remove_dir_all(&dir);
        let _ = ParameterValue::Float(0.0);
        let _ = TransitionSource::AnyState;
        Ok(())
    }
}
