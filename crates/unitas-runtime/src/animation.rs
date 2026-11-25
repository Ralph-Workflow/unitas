use std::collections::BTreeMap;

use unitas_animation::{
    AnimatedValue, AnimationDiagnostic, AnimationError, AnimationEvent, Animator,
    AnimatorController, AnimatorStateInfo, CurveBindingTarget, SampledBinding, TransformChannel,
    TransformProperty,
};
use unitas_core::{compose_world, normalize_rotation, EntityId, Transform};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationRuntimeDiagnostic {
    Controller(AnimationDiagnostic),
    UnsupportedBinding {
        entity: EntityId,
        object_path: String,
        component: String,
        property: String,
    },
    UnresolvedChildPath {
        entity: EntityId,
        object_path: String,
    },
}

impl std::fmt::Display for AnimationRuntimeDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Controller(diag) => write!(formatter, "{diag}"),
            Self::UnsupportedBinding {
                object_path,
                component,
                property,
                ..
            } => write!(
                formatter,
                "binding '{object_path}:{component}.{property}' is not yet applied in runtime"
            ),
            Self::UnresolvedChildPath { object_path, .. } => write!(
                formatter,
                "child object path '{object_path}' could not be resolved in scene graph"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityAnimatorEntry {
    pub controller_handle: u64,
    pub animator: Option<Animator>,
    pub transform: Transform,
    pub speed: f32,
    pub fired_events: Vec<AnimationEvent>,
    pub typed_diagnostics: Vec<AnimationRuntimeDiagnostic>,
    pub diagnostics: Vec<String>,
}

impl EntityAnimatorEntry {
    pub fn new(initial_transform: Transform) -> Self {
        Self {
            controller_handle: 0,
            animator: None,
            transform: initial_transform,
            speed: 1.0,
            fired_events: Vec::new(),
            typed_diagnostics: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn push_diagnostic(&mut self, diagnostic: AnimationRuntimeDiagnostic) {
        if !self.typed_diagnostics.contains(&diagnostic) {
            let text = diagnostic.to_string();
            self.typed_diagnostics.push(diagnostic);
            self.diagnostics.push(text);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnimationWorld {
    controllers: BTreeMap<u64, AnimatorController>,
    next_controller_handle: u64,
    entries: BTreeMap<EntityId, EntityAnimatorEntry>,
}

impl AnimationWorld {
    pub fn new() -> Self {
        Self {
            controllers: BTreeMap::new(),
            next_controller_handle: 1,
            entries: BTreeMap::new(),
        }
    }

    pub fn register_controller(&mut self, controller: AnimatorController) -> u64 {
        let handle = if self.next_controller_handle == 0 {
            1
        } else {
            self.next_controller_handle
        };
        self.next_controller_handle = handle.saturating_add(1);
        self.controllers.insert(handle, controller);
        handle
    }

    pub fn decode_and_register_controller(&mut self, bytes: &[u8]) -> Result<u64, AnimationError> {
        let controller = AnimatorController::decode_binary(bytes)?;
        Ok(self.register_controller(controller))
    }

    pub fn remove_controller(&mut self, controller_handle: u64) -> bool {
        self.controllers.remove(&controller_handle).is_some()
    }

    pub fn controller(&self, controller_handle: u64) -> Option<&AnimatorController> {
        self.controllers.get(&controller_handle)
    }

    pub fn ensure_animator(
        &mut self,
        entity: EntityId,
        initial_transform: Transform,
    ) -> &mut EntityAnimatorEntry {
        self.entries
            .entry(entity)
            .or_insert_with(|| EntityAnimatorEntry::new(initial_transform))
    }

    pub fn remove_animator(&mut self, entity: EntityId) -> bool {
        self.entries.remove(&entity).is_some()
    }

    pub fn entry(&self, entity: EntityId) -> Option<&EntityAnimatorEntry> {
        self.entries.get(&entity)
    }

    pub fn entry_mut(&mut self, entity: EntityId) -> Option<&mut EntityAnimatorEntry> {
        self.entries.get_mut(&entity)
    }

    pub fn entities(&self) -> Vec<EntityId> {
        self.entries.keys().copied().collect()
    }

    pub fn set_controller(
        &mut self,
        entity: EntityId,
        controller_handle: u64,
    ) -> Result<Vec<SampledBinding>, String> {
        if controller_handle == 0 {
            let Some(entry) = self.entries.get_mut(&entity) else {
                return Err(format!("animator not found for entity {}", entity.as_u64()));
            };
            entry.controller_handle = 0;
            entry.animator = None;
            entry.fired_events.clear();
            entry.typed_diagnostics.clear();
            entry.diagnostics.clear();
            return Ok(Vec::new());
        }
        let Some(controller) = self.controllers.get(&controller_handle).cloned() else {
            return Err(format!("animator controller {controller_handle} not found"));
        };
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let mut animator = Animator::new(controller);
        animator.set_speed(entry.speed);
        let initial_diags: Vec<AnimationDiagnostic> = animator.diagnostics().to_vec();
        entry.controller_handle = controller_handle;
        entry.fired_events.clear();
        entry.typed_diagnostics.clear();
        entry.diagnostics.clear();
        for diag in initial_diags {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
        }
        let step_result = animator.step(0.0);
        for diag in step_result.diagnostics {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
        }
        entry.fired_events.extend(step_result.fired_events);
        entry.animator = Some(animator);
        let child_bindings = apply_self_bindings_to_entry(entity, entry, &step_result.bindings);
        Ok(child_bindings)
    }

    pub fn set_speed(&mut self, entity: EntityId, speed: f32) -> Result<(), String> {
        if !speed.is_finite() {
            return Err("animator speed must be finite".to_string());
        }
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let clamped = speed.max(0.0);
        entry.speed = clamped;
        if let Some(animator) = &mut entry.animator {
            animator.set_speed(clamped);
        }
        Ok(())
    }

    pub fn speed(&self, entity: EntityId) -> Option<f32> {
        let entry = self.entries.get(&entity)?;
        Some(entry.animator.as_ref().map_or(entry.speed, Animator::speed))
    }

    pub fn set_float(&mut self, entity: EntityId, name: &str, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("float parameter value must be finite".to_string());
        }
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &mut entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        if let Err(diag) = animator.set_float(name, value) {
            let msg = diag.to_string();
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            return Err(msg);
        }
        Ok(())
    }

    pub fn get_float(&self, entity: EntityId, name: &str) -> Result<f32, String> {
        let Some(entry) = self.entries.get(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        animator
            .get_float(name)
            .ok_or_else(|| format!("float parameter '{name}' not found"))
    }

    pub fn set_int(&mut self, entity: EntityId, name: &str, value: i32) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &mut entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        if let Err(diag) = animator.set_int(name, value) {
            let msg = diag.to_string();
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            return Err(msg);
        }
        Ok(())
    }

    pub fn get_int(&self, entity: EntityId, name: &str) -> Result<i32, String> {
        let Some(entry) = self.entries.get(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        animator
            .get_int(name)
            .ok_or_else(|| format!("int parameter '{name}' not found"))
    }

    pub fn set_bool(&mut self, entity: EntityId, name: &str, value: bool) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &mut entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        if let Err(diag) = animator.set_bool(name, value) {
            let msg = diag.to_string();
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            return Err(msg);
        }
        Ok(())
    }

    pub fn get_bool(&self, entity: EntityId, name: &str) -> Result<bool, String> {
        let Some(entry) = self.entries.get(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        animator
            .get_bool(name)
            .ok_or_else(|| format!("bool parameter '{name}' not found"))
    }

    pub fn set_trigger(&mut self, entity: EntityId, name: &str) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &mut entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        if let Err(diag) = animator.set_trigger(name) {
            let msg = diag.to_string();
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            return Err(msg);
        }
        Ok(())
    }

    pub fn reset_trigger(&mut self, entity: EntityId, name: &str) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &mut entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        if let Err(diag) = animator.reset_trigger(name) {
            let msg = diag.to_string();
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            return Err(msg);
        }
        Ok(())
    }

    pub fn state_info(&self, entity: EntityId) -> Result<AnimatorStateInfo, String> {
        let Some(entry) = self.entries.get(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        let Some(animator) = &entry.animator else {
            return Err(format!(
                "animator on entity {} has no controller attached",
                entity.as_u64()
            ));
        };
        Ok(animator.current_state_info())
    }

    pub fn transform(&self, entity: EntityId) -> Option<&Transform> {
        self.entries.get(&entity).map(|entry| &entry.transform)
    }

    pub fn set_transform(&mut self, entity: EntityId, transform: Transform) {
        if let Some(entry) = self.entries.get_mut(&entity) {
            entry.transform = transform;
        }
    }

    pub fn fired_events(&self, entity: EntityId) -> Option<&[AnimationEvent]> {
        self.entries
            .get(&entity)
            .map(|entry| entry.fired_events.as_slice())
    }

    pub fn fired_event_count(&self, entity: EntityId) -> Option<usize> {
        self.entries
            .get(&entity)
            .map(|entry| entry.fired_events.len())
    }

    pub fn diagnostics(&self, entity: EntityId) -> Option<&[String]> {
        self.entries
            .get(&entity)
            .map(|entry| entry.diagnostics.as_slice())
    }

    pub fn typed_diagnostics(&self, entity: EntityId) -> Option<&[AnimationRuntimeDiagnostic]> {
        self.entries
            .get(&entity)
            .map(|entry| entry.typed_diagnostics.as_slice())
    }

    pub fn diagnostic_count(&self, entity: EntityId) -> Option<usize> {
        self.entries
            .get(&entity)
            .map(|entry| entry.diagnostics.len())
    }

    pub fn record_diagnostic(
        &mut self,
        entity: EntityId,
        diagnostic: AnimationRuntimeDiagnostic,
    ) {
        if let Some(entry) = self.entries.get_mut(&entity) {
            entry.push_diagnostic(diagnostic);
        }
    }

    pub fn tick(&mut self, scaled_delta_seconds: f64) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let dt = if scaled_delta_seconds.is_finite() && scaled_delta_seconds > 0.0 {
            scaled_delta_seconds as f32
        } else {
            0.0
        };
        let mut all_child_bindings = Vec::new();
        for (entity, entry) in &mut self.entries {
            let Some(animator) = &mut entry.animator else {
                continue;
            };
            let step_result = animator.step(dt);
            for diag in step_result.diagnostics {
                entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
            }
            entry.fired_events.extend(step_result.fired_events);
            let child_bindings =
                apply_self_bindings_to_entry(*entity, entry, &step_result.bindings);
            if !child_bindings.is_empty() {
                all_child_bindings.push((*entity, child_bindings));
            }
        }
        all_child_bindings
    }
}

fn apply_self_bindings_to_entry(
    entity: EntityId,
    entry: &mut EntityAnimatorEntry,
    bindings: &[SampledBinding],
) -> Vec<SampledBinding> {
    let mut child_bindings = Vec::new();
    for binding in bindings {
        let Some((prop, ch)) = binding.target.parse_transform_property() else {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::UnsupportedBinding {
                entity,
                object_path: binding.target.object_path.clone(),
                component: binding.target.component.clone(),
                property: binding.target.property.clone(),
            });
            continue;
        };
        if binding.target.object_path.is_empty() {
            apply_transform_property(&mut entry.transform, prop, ch, binding.value);
        } else {
            child_bindings.push(binding.clone());
        }
    }
    child_bindings
}

pub fn apply_transform_property(
    transform: &mut Transform,
    property: TransformProperty,
    channel: TransformChannel,
    value: AnimatedValue,
) {
    match (property, channel, value) {
        (TransformProperty::LocalPosition, TransformChannel::All, AnimatedValue::Vector3(v)) => {
            transform.translation = v;
        }
        (TransformProperty::LocalPosition, TransformChannel::X, AnimatedValue::Float(v)) => {
            transform.translation[0] = v;
        }
        (TransformProperty::LocalPosition, TransformChannel::Y, AnimatedValue::Float(v)) => {
            transform.translation[1] = v;
        }
        (TransformProperty::LocalPosition, TransformChannel::Z, AnimatedValue::Float(v)) => {
            transform.translation[2] = v;
        }
        (TransformProperty::LocalRotation, TransformChannel::All, AnimatedValue::Quaternion(q)) => {
            transform.rotation = normalize_rotation(q);
        }
        (TransformProperty::LocalRotation, TransformChannel::X, AnimatedValue::Float(v)) => {
            let mut q = transform.rotation;
            q[0] = v;
            transform.rotation = normalize_rotation(q);
        }
        (TransformProperty::LocalRotation, TransformChannel::Y, AnimatedValue::Float(v)) => {
            let mut q = transform.rotation;
            q[1] = v;
            transform.rotation = normalize_rotation(q);
        }
        (TransformProperty::LocalRotation, TransformChannel::Z, AnimatedValue::Float(v)) => {
            let mut q = transform.rotation;
            q[2] = v;
            transform.rotation = normalize_rotation(q);
        }
        (TransformProperty::LocalRotation, TransformChannel::W, AnimatedValue::Float(v)) => {
            let mut q = transform.rotation;
            q[3] = v;
            transform.rotation = normalize_rotation(q);
        }
        (TransformProperty::LocalScale, TransformChannel::All, AnimatedValue::Vector3(v)) => {
            transform.scale = v;
        }
        (TransformProperty::LocalScale, TransformChannel::X, AnimatedValue::Float(v)) => {
            transform.scale[0] = v;
        }
        (TransformProperty::LocalScale, TransformChannel::Y, AnimatedValue::Float(v)) => {
            transform.scale[1] = v;
        }
        (TransformProperty::LocalScale, TransformChannel::Z, AnimatedValue::Float(v)) => {
            transform.scale[2] = v;
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimatorRootMotion {
    pub delta_position: [f32; 3],
    pub delta_rotation: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AvatarIkGoal {
    LeftFoot,
    RightFoot,
    LeftHand,
    RightHand,
}

impl AvatarIkGoal {
    pub const fn default_hint(self) -> AvatarIkHint {
        match self {
            Self::LeftFoot => AvatarIkHint::LeftKnee,
            Self::RightFoot => AvatarIkHint::RightKnee,
            Self::LeftHand => AvatarIkHint::LeftElbow,
            Self::RightHand => AvatarIkHint::RightElbow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AvatarIkHint {
    LeftKnee,
    RightKnee,
    LeftElbow,
    RightElbow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBoneIkChain {
    pub root_position: [f32; 3],
    pub mid_position: [f32; 3],
    pub tip_position: [f32; 3],
    pub tip_rotation: [f32; 4],
}

impl Default for TwoBoneIkChain {
    fn default() -> Self {
        Self {
            root_position: [0.0; 3],
            mid_position: [0.0, 1.0, 0.0],
            tip_position: [0.0, 2.0, 0.0],
            tip_rotation: unitas_core::IDENTITY_ROTATION,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBoneIkResult {
    pub root_position: [f32; 3],
    pub mid_position: [f32; 3],
    pub tip_position: [f32; 3],
    pub root_rotation_delta: [f32; 4],
    pub mid_rotation_delta: [f32; 4],
    pub tip_rotation: [f32; 4],
}

impl TwoBoneIkResult {
    pub fn solved_bone_rotations(
        &self,
        root_rotation: [f32; 4],
        mid_rotation: [f32; 4],
    ) -> ([f32; 4], [f32; 4], [f32; 4]) {
        let solved_root = unitas_core::multiply_rotation(
            self.root_rotation_delta,
            normalize_rotation(root_rotation),
        );
        let solved_mid = unitas_core::multiply_rotation(
            self.mid_rotation_delta,
            unitas_core::multiply_rotation(
                self.root_rotation_delta,
                normalize_rotation(mid_rotation),
            ),
        );
        (solved_root, solved_mid, self.tip_rotation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IkGoalPose {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub position_weight: f32,
    pub rotation_weight: f32,
}

impl Default for IkGoalPose {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: unitas_core::IDENTITY_ROTATION,
            position_weight: 0.0,
            rotation_weight: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IkHintPose {
    pub position: [f32; 3],
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookAtIkConfig {
    pub origin: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub target: [f32; 3],
    pub weight: f32,
    pub body_weight: f32,
    pub head_weight: f32,
    pub eyes_weight: f32,
    pub clamp_weight: f32,
}

impl Default for LookAtIkConfig {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            target: [0.0, 0.0, 1.0],
            weight: 0.0,
            body_weight: 0.0,
            head_weight: 1.0,
            eyes_weight: 0.0,
            clamp_weight: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookAtIkResult {
    pub body_rotation: [f32; 4],
    pub head_rotation: [f32; 4],
    pub eyes_rotation: [f32; 4],
}

impl Default for LookAtIkResult {
    fn default() -> Self {
        Self {
            body_rotation: unitas_core::IDENTITY_ROTATION,
            head_rotation: unitas_core::IDENTITY_ROTATION,
            eyes_rotation: unitas_core::IDENTITY_ROTATION,
        }
    }
}

pub fn solve_two_bone_ik(
    chain: &TwoBoneIkChain,
    target_position: [f32; 3],
    target_rotation: [f32; 4],
    bend_hint: Option<[f32; 3]>,
    position_weight: f32,
    rotation_weight: f32,
    hint_weight: f32,
) -> TwoBoneIkResult {
    let pos_w = sanitize_weight(position_weight);
    let rot_w = sanitize_weight(rotation_weight);
    let hint_w = sanitize_weight(hint_weight);
    let root = chain.root_position;
    let orig_mid = chain.mid_position;
    let orig_tip = chain.tip_position;
    let orig_tip_rot = normalize_rotation(chain.tip_rotation);
    let goal_tip_rot = normalize_rotation(target_rotation);
    let tip_rotation = slerp_rotation(orig_tip_rot, goal_tip_rot, rot_w);

    if pos_w <= 1e-6 {
        return TwoBoneIkResult {
            root_position: root,
            mid_position: orig_mid,
            tip_position: orig_tip,
            root_rotation_delta: unitas_core::IDENTITY_ROTATION,
            mid_rotation_delta: unitas_core::IDENTITY_ROTATION,
            tip_rotation,
        };
    }

    let upper_vec = subtract_vectors(orig_mid, root);
    let lower_vec = subtract_vectors(orig_tip, orig_mid);
    let l1 = vector_length(upper_vec);
    let l2 = vector_length(lower_vec);
    if l1 <= 1e-6 || l2 <= 1e-6 {
        return TwoBoneIkResult {
            root_position: root,
            mid_position: orig_mid,
            tip_position: orig_tip,
            root_rotation_delta: unitas_core::IDENTITY_ROTATION,
            mid_rotation_delta: unitas_core::IDENTITY_ROTATION,
            tip_rotation,
        };
    }

    let effective_target = lerp_vector(orig_tip, target_position, pos_w);
    let root_to_target = subtract_vectors(effective_target, root);
    let raw_dist = vector_length(root_to_target);
    let orig_root_to_tip = subtract_vectors(orig_tip, root);
    let axis = if raw_dist > 1e-6 {
        scale_vector(root_to_target, 1.0 / raw_dist)
    } else if vector_length(orig_root_to_tip) > 1e-6 {
        normalize_vector(orig_root_to_tip, [0.0, 1.0, 0.0])
    } else {
        normalize_vector(upper_vec, [0.0, 1.0, 0.0])
    };

    let max_reach = l1 + l2;
    let min_reach = (l1 - l2).abs().max(1e-5);
    let orig_bend_dir = perpendicular_component(upper_vec, axis);
    let hinted_bend_dir = match bend_hint {
        Some(hint_pos) if hint_w > 1e-6 => {
            let to_hint = subtract_vectors(hint_pos, root);
            let hint_perp = perpendicular_component(to_hint, axis);
            let orig_norm = fallback_perpendicular(orig_bend_dir, axis);
            let hint_norm = fallback_perpendicular(hint_perp, axis);
            fallback_perpendicular(lerp_vector(orig_norm, hint_norm, hint_w), axis)
        }
        _ => fallback_perpendicular(orig_bend_dir, axis),
    };

    let (mid_position, tip_position) = if raw_dist >= max_reach {
        let mid = add_vectors(root, scale_vector(axis, l1));
        let tip = add_vectors(root, scale_vector(axis, max_reach));
        (mid, tip)
    } else {
        let clamped_d = raw_dist.clamp(min_reach, max_reach);
        let cos_alpha = ((l1 * l1 + clamped_d * clamped_d - l2 * l2) / (2.0 * l1 * clamped_d))
            .clamp(-1.0, 1.0);
        let sin_alpha = (1.0 - cos_alpha * cos_alpha).max(0.0).sqrt();
        let along = scale_vector(axis, l1 * cos_alpha);
        let perp = scale_vector(hinted_bend_dir, l1 * sin_alpha);
        let mid = add_vectors(root, add_vectors(along, perp));
        let tip = add_vectors(root, scale_vector(axis, clamped_d));
        (mid, tip)
    };

    let solved_upper = subtract_vectors(mid_position, root);
    let solved_lower = subtract_vectors(tip_position, mid_position);
    let root_rotation_delta = rotation_from_to(upper_vec, solved_upper);
    let rotated_orig_lower = unitas_core::rotate_vector(root_rotation_delta, lower_vec);
    let mid_rotation_delta = rotation_from_to(rotated_orig_lower, solved_lower);

    TwoBoneIkResult {
        root_position: root,
        mid_position,
        tip_position,
        root_rotation_delta,
        mid_rotation_delta,
        tip_rotation,
    }
}

pub fn solve_look_at_ik(config: &LookAtIkConfig) -> LookAtIkResult {
    let overall_weight = sanitize_weight(config.weight);
    if overall_weight <= 1e-6 {
        return LookAtIkResult::default();
    }
    let clamp_weight = sanitize_weight(config.clamp_weight);
    if clamp_weight >= 1.0 - 1e-6 {
        return LookAtIkResult::default();
    }
    let forward = normalize_vector(config.forward, [0.0, 0.0, 1.0]);
    let to_target = subtract_vectors(config.target, config.origin);
    let target_len = vector_length(to_target);
    if target_len <= 1e-6 {
        return LookAtIkResult::default();
    }
    let desired = scale_vector(to_target, 1.0 / target_len);
    let raw_angle = dot_product(forward, desired).clamp(-1.0, 1.0).acos();
    let max_angle = std::f32::consts::PI * (1.0 - clamp_weight);
    let clamped_angle = raw_angle.min(max_angle);
    let clamp_ratio = if raw_angle > 1e-6 {
        (clamped_angle / raw_angle).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let full_rotation = rotation_from_to_with_up(forward, desired, config.up);
    let body_w = sanitize_weight(config.body_weight) * overall_weight * clamp_ratio;
    let head_w = sanitize_weight(config.head_weight) * overall_weight * clamp_ratio;
    let eyes_w = sanitize_weight(config.eyes_weight) * overall_weight * clamp_ratio;

    LookAtIkResult {
        body_rotation: slerp_rotation(unitas_core::IDENTITY_ROTATION, full_rotation, body_w),
        head_rotation: slerp_rotation(unitas_core::IDENTITY_ROTATION, full_rotation, head_w),
        eyes_rotation: slerp_rotation(unitas_core::IDENTITY_ROTATION, full_rotation, eyes_w),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoBoneIkSkeletonChain {
    pub root_path: String,
    pub mid_path: String,
    pub tip_path: String,
}

impl TwoBoneIkSkeletonChain {
    pub fn new(
        root_path: impl Into<String>,
        mid_path: impl Into<String>,
        tip_path: impl Into<String>,
    ) -> Self {
        Self {
            root_path: root_path.into(),
            mid_path: mid_path.into(),
            tip_path: tip_path.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimatorIkPose {
    pub body_position: Option<[f32; 3]>,
    pub body_rotation: Option<[f32; 4]>,
    chains: BTreeMap<AvatarIkGoal, TwoBoneIkChain>,
    goals: BTreeMap<AvatarIkGoal, IkGoalPose>,
    hints: BTreeMap<AvatarIkHint, IkHintPose>,
    look_at: Option<LookAtIkConfig>,
    solved_goals: BTreeMap<AvatarIkGoal, TwoBoneIkResult>,
    solved_look_at: Option<LookAtIkResult>,
}

impl AnimatorIkPose {
    pub fn set_ik_chain(&mut self, goal: AvatarIkGoal, chain: TwoBoneIkChain) {
        self.chains.insert(goal, chain);
    }

    pub fn ik_chain(&self, goal: AvatarIkGoal) -> Option<&TwoBoneIkChain> {
        self.chains.get(&goal)
    }

    pub fn set_ik_goal(&mut self, goal: AvatarIkGoal, pose: IkGoalPose) {
        self.goals.insert(goal, pose);
    }

    pub fn ik_goal(&self, goal: AvatarIkGoal) -> Option<&IkGoalPose> {
        self.goals.get(&goal)
    }

    pub fn set_ik_goal_position(&mut self, goal: AvatarIkGoal, position: [f32; 3]) {
        self.goals.entry(goal).or_default().position = position;
    }

    pub fn ik_goal_position(&self, goal: AvatarIkGoal) -> [f32; 3] {
        self.goals
            .get(&goal)
            .map_or([0.0; 3], |pose| pose.position)
    }

    pub fn set_ik_goal_rotation(&mut self, goal: AvatarIkGoal, rotation: [f32; 4]) {
        self.goals.entry(goal).or_default().rotation = normalize_rotation(rotation);
    }

    pub fn ik_goal_rotation(&self, goal: AvatarIkGoal) -> [f32; 4] {
        self.goals
            .get(&goal)
            .map_or(unitas_core::IDENTITY_ROTATION, |pose| pose.rotation)
    }

    pub fn set_ik_goal_position_weight(&mut self, goal: AvatarIkGoal, weight: f32) {
        self.goals.entry(goal).or_default().position_weight = sanitize_weight(weight);
    }

    pub fn ik_goal_position_weight(&self, goal: AvatarIkGoal) -> f32 {
        self.goals
            .get(&goal)
            .map_or(0.0, |pose| pose.position_weight)
    }

    pub fn set_ik_goal_rotation_weight(&mut self, goal: AvatarIkGoal, weight: f32) {
        self.goals.entry(goal).or_default().rotation_weight = sanitize_weight(weight);
    }

    pub fn ik_goal_rotation_weight(&self, goal: AvatarIkGoal) -> f32 {
        self.goals
            .get(&goal)
            .map_or(0.0, |pose| pose.rotation_weight)
    }

    pub fn set_ik_hint(&mut self, hint: AvatarIkHint, pose: IkHintPose) {
        self.hints.insert(hint, pose);
    }

    pub fn ik_hint(&self, hint: AvatarIkHint) -> Option<&IkHintPose> {
        self.hints.get(&hint)
    }

    pub fn set_ik_hint_position(&mut self, hint: AvatarIkHint, position: [f32; 3]) {
        self.hints.entry(hint).or_default().position = position;
    }

    pub fn ik_hint_position(&self, hint: AvatarIkHint) -> [f32; 3] {
        self.hints
            .get(&hint)
            .map_or([0.0; 3], |pose| pose.position)
    }

    pub fn set_ik_hint_position_weight(&mut self, hint: AvatarIkHint, weight: f32) {
        self.hints.entry(hint).or_default().weight = sanitize_weight(weight);
    }

    pub fn ik_hint_position_weight(&self, hint: AvatarIkHint) -> f32 {
        self.hints.get(&hint).map_or(0.0, |pose| pose.weight)
    }

    pub fn set_look_at(&mut self, config: LookAtIkConfig) {
        self.look_at = Some(config);
    }

    pub fn look_at(&self) -> Option<&LookAtIkConfig> {
        self.look_at.as_ref()
    }

    pub fn set_look_at_position(&mut self, target: [f32; 3]) {
        self.look_at
            .get_or_insert_with(LookAtIkConfig::default)
            .target = target;
    }

    pub fn set_look_at_weight(
        &mut self,
        weight: f32,
        body_weight: f32,
        head_weight: f32,
        eyes_weight: f32,
        clamp_weight: f32,
    ) {
        let config = self.look_at.get_or_insert_with(LookAtIkConfig::default);
        config.weight = sanitize_weight(weight);
        config.body_weight = sanitize_weight(body_weight);
        config.head_weight = sanitize_weight(head_weight);
        config.eyes_weight = sanitize_weight(eyes_weight);
        config.clamp_weight = sanitize_weight(clamp_weight);
    }

    pub fn solved_goal(&self, goal: AvatarIkGoal) -> Option<&TwoBoneIkResult> {
        self.solved_goals.get(&goal)
    }

    pub fn solved_look_at(&self) -> Option<&LookAtIkResult> {
        self.solved_look_at.as_ref()
    }

    pub fn solve(&mut self) {
        self.solved_goals.clear();
        for (goal, goal_pose) in &self.goals {
            let chain = self.chains.get(goal).copied().unwrap_or_default();
            let default_hint = goal.default_hint();
            let (hint_pos, hint_weight) = match self.hints.get(&default_hint) {
                Some(hint) => (Some(hint.position), hint.weight),
                None => (None, 0.0),
            };
            let result = solve_two_bone_ik(
                &chain,
                goal_pose.position,
                goal_pose.rotation,
                hint_pos,
                goal_pose.position_weight,
                goal_pose.rotation_weight,
                hint_weight,
            );
            self.solved_goals.insert(*goal, result);
        }
        self.solved_look_at = self.look_at.as_ref().map(solve_look_at_ik);
    }
}

pub trait AnimatorScriptHooks {
    fn on_animator_ik(&mut self, _entity: EntityId, _layer_index: u32, _pose: &mut AnimatorIkPose) {}
    fn on_animator_move(&mut self, _entity: EntityId, _motion: &mut AnimatorRootMotion) -> bool {
        false
    }
}

struct AnimatorHookEntry {
    ik_layers: Vec<u32>,
    skeleton_chains: BTreeMap<AvatarIkGoal, TwoBoneIkSkeletonChain>,
    look_at_bone_path: Option<String>,
    script: Option<Box<dyn AnimatorScriptHooks>>,
    last_ik_pose: Option<AnimatorIkPose>,
}

#[derive(Default)]
pub struct AnimatorHookWorld {
    entries: BTreeMap<EntityId, AnimatorHookEntry>,
}

impl AnimatorHookWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach(&mut self, entity: EntityId, script: Box<dyn AnimatorScriptHooks>) {
        let entry = self.entry_mut(entity);
        entry.script = Some(script);
    }

    pub fn detach(&mut self, entity: EntityId) -> bool {
        self.entries.remove(&entity).is_some()
    }

    pub fn is_attached(&self, entity: EntityId) -> bool {
        self.entries.contains_key(&entity)
    }

    pub fn set_ik_layers(&mut self, entity: EntityId, layers: Vec<u32>) {
        let entry = self.entry_mut(entity);
        entry.ik_layers = layers;
    }

    pub fn ik_layers(&self, entity: EntityId) -> &[u32] {
        self.entries
            .get(&entity)
            .map(|entry| entry.ik_layers.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_skeleton_ik_chain(
        &mut self,
        entity: EntityId,
        goal: AvatarIkGoal,
        chain: TwoBoneIkSkeletonChain,
    ) {
        self.entry_mut(entity).skeleton_chains.insert(goal, chain);
    }

    pub fn skeleton_ik_chain(
        &self,
        entity: EntityId,
        goal: AvatarIkGoal,
    ) -> Option<&TwoBoneIkSkeletonChain> {
        self.entries
            .get(&entity)
            .and_then(|entry| entry.skeleton_chains.get(&goal))
    }

    pub fn set_look_at_bone_path(&mut self, entity: EntityId, bone_path: impl Into<String>) {
        self.entry_mut(entity).look_at_bone_path = Some(bone_path.into());
    }

    pub fn look_at_bone_path(&self, entity: EntityId) -> Option<&str> {
        self.entries
            .get(&entity)
            .and_then(|entry| entry.look_at_bone_path.as_deref())
    }

    pub fn last_ik_pose(&self, entity: EntityId) -> Option<&AnimatorIkPose> {
        self.entries
            .get(&entity)
            .and_then(|entry| entry.last_ik_pose.as_ref())
    }

    fn entry_mut(&mut self, entity: EntityId) -> &mut AnimatorHookEntry {
        self.entries.entry(entity).or_insert_with(|| AnimatorHookEntry {
            ik_layers: Vec::new(),
            skeleton_chains: BTreeMap::new(),
            look_at_bone_path: None,
            script: None,
            last_ik_pose: None,
        })
    }
}

impl AnimationWorld {
    pub fn tick_with_hooks(
        &mut self,
        scaled_delta_seconds: f64,
        hooks: &mut AnimatorHookWorld,
    ) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let previous: Vec<(EntityId, Transform)> = self
            .entries
            .iter()
            .map(|(entity, entry)| (*entity, entry.transform.clone()))
            .collect();
        let mut child_bindings = self.tick(scaled_delta_seconds);
        for (entity, previous_transform) in previous {
            let Some(entry) = self.entries.get_mut(&entity) else {
                continue;
            };
            let Some(hook_entry) = hooks.entries.get_mut(&entity) else {
                continue;
            };
            let Some(script) = hook_entry.script.as_mut() else {
                continue;
            };
            let sampled_translation = entry.transform.translation;
            let sampled_rotation = entry.transform.rotation;
            let mut motion = AnimatorRootMotion {
                delta_position: subtract_vectors(
                    sampled_translation,
                    previous_transform.translation,
                ),
                delta_rotation: rotation_delta(
                    previous_transform.rotation,
                    sampled_rotation,
                ),
            };
            let existing_entity_bindings: Vec<SampledBinding> = child_bindings
                .iter()
                .find(|(e, _)| *e == entity)
                .map(|(_, b)| b.clone())
                .unwrap_or_default();
            let mut skeleton_locals = build_skeleton_locals(&existing_entity_bindings);
            for chain_paths in hook_entry.skeleton_chains.values() {
                ensure_skeleton_path(&mut skeleton_locals, &chain_paths.root_path);
                ensure_skeleton_path(&mut skeleton_locals, &chain_paths.mid_path);
                ensure_skeleton_path(&mut skeleton_locals, &chain_paths.tip_path);
            }
            if let Some(ref look_path) = hook_entry.look_at_bone_path {
                ensure_skeleton_path(&mut skeleton_locals, look_path);
            }

            let ik_layers = hook_entry.ik_layers.clone();
            let mut pose = AnimatorIkPose {
                body_position: Some(sampled_translation),
                body_rotation: Some(sampled_rotation),
                ..AnimatorIkPose::default()
            };
            populate_pose_from_skeleton(
                &mut pose,
                &entry.transform,
                &skeleton_locals,
                &hook_entry.skeleton_chains,
                hook_entry.look_at_bone_path.as_deref(),
            );
            let mut ik_ran = false;
            for layer_index in ik_layers {
                ik_ran = true;
                populate_pose_from_skeleton(
                    &mut pose,
                    &entry.transform,
                    &skeleton_locals,
                    &hook_entry.skeleton_chains,
                    hook_entry.look_at_bone_path.as_deref(),
                );
                script.on_animator_ik(entity, layer_index, &mut pose);
                pose.solve();
                if let Some(body_position) = pose.body_position {
                    entry.transform.translation = body_position;
                }
                if let Some(body_rotation) = pose.body_rotation {
                    entry.transform.rotation = normalize_rotation(body_rotation);
                }
                apply_solved_ik_to_skeleton(
                    &pose,
                    &entry.transform,
                    &mut skeleton_locals,
                    &hook_entry.skeleton_chains,
                    hook_entry.look_at_bone_path.as_deref(),
                );
            }
            if ik_ran {
                let ik_bindings = build_ik_child_bindings(
                    &skeleton_locals,
                    &hook_entry.skeleton_chains,
                    hook_entry.look_at_bone_path.as_deref(),
                );
                if !ik_bindings.is_empty() {
                    merge_child_bindings(&mut child_bindings, entity, ik_bindings);
                }
                hook_entry.last_ik_pose = Some(pose);
            }
            let ik_override_translation =
                if entry.transform.translation != sampled_translation {
                    Some(entry.transform.translation)
                } else {
                    None
                };
            let ik_override_rotation = if entry.transform.rotation != sampled_rotation {
                Some(entry.transform.rotation)
            } else {
                None
            };
            if script.on_animator_move(entity, &mut motion) {
                let mut moved_translation =
                    add_vectors(previous_transform.translation, motion.delta_position);
                if let Some(ik_pos) = ik_override_translation {
                    for axis in 0..3 {
                        if (ik_pos[axis] - sampled_translation[axis]).abs() > 1e-6 {
                            moved_translation[axis] = ik_pos[axis];
                        }
                    }
                }
                entry.transform.translation = moved_translation;
                entry.transform.rotation = ik_override_rotation.unwrap_or_else(|| {
                    unitas_core::multiply_rotation(
                        motion.delta_rotation,
                        previous_transform.rotation,
                    )
                });
            }
        }
        child_bindings
    }
}

fn build_skeleton_locals(bindings: &[SampledBinding]) -> BTreeMap<String, Transform> {
    let mut locals: BTreeMap<String, Transform> = BTreeMap::new();
    for binding in bindings {
        if binding.target.object_path.is_empty() {
            continue;
        }
        let Some((prop, ch)) = binding.target.parse_transform_property() else {
            continue;
        };
        ensure_skeleton_path(&mut locals, &binding.target.object_path);
        if let Some(transform) = locals.get_mut(&binding.target.object_path) {
            apply_transform_property(transform, prop, ch, binding.value);
        }
    }
    locals
}

fn ensure_skeleton_path(locals: &mut BTreeMap<String, Transform>, path: &str) {
    if path.is_empty() {
        return;
    }
    let mut prefix = String::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        locals.entry(prefix.clone()).or_default();
    }
}

fn parent_skeleton_path(path: &str) -> &str {
    match path.rfind('/') {
        Some(idx) => &path[..idx],
        None => "",
    }
}

fn compute_skeleton_world_transform(
    root_transform: &Transform,
    locals: &BTreeMap<String, Transform>,
    path: &str,
) -> Transform {
    if path.is_empty() {
        return root_transform.clone();
    }
    let mut world = root_transform.clone();
    let mut prefix = String::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        let local = locals.get(&prefix).cloned().unwrap_or_default();
        world = compose_world(&world, &local);
    }
    world
}

fn populate_pose_from_skeleton(
    pose: &mut AnimatorIkPose,
    root_transform: &Transform,
    locals: &BTreeMap<String, Transform>,
    skeleton_chains: &BTreeMap<AvatarIkGoal, TwoBoneIkSkeletonChain>,
    look_at_bone_path: Option<&str>,
) {
    for (goal, chain_paths) in skeleton_chains {
        let root_world =
            compute_skeleton_world_transform(root_transform, locals, &chain_paths.root_path);
        let mid_world =
            compute_skeleton_world_transform(root_transform, locals, &chain_paths.mid_path);
        let tip_world =
            compute_skeleton_world_transform(root_transform, locals, &chain_paths.tip_path);
        pose.set_ik_chain(
            *goal,
            TwoBoneIkChain {
                root_position: root_world.translation,
                mid_position: mid_world.translation,
                tip_position: tip_world.translation,
                tip_rotation: tip_world.rotation,
            },
        );
    }
    if let Some(bone_path) = look_at_bone_path {
        let head_world = compute_skeleton_world_transform(root_transform, locals, bone_path);
        let forward = unitas_core::rotate_vector(head_world.rotation, [0.0, 0.0, 1.0]);
        let up = unitas_core::rotate_vector(head_world.rotation, [0.0, 1.0, 0.0]);
        let config = pose.look_at.get_or_insert_with(LookAtIkConfig::default);
        config.origin = head_world.translation;
        config.forward = forward;
        config.up = up;
    }
}

fn apply_solved_ik_to_skeleton(
    pose: &AnimatorIkPose,
    root_transform: &Transform,
    locals: &mut BTreeMap<String, Transform>,
    skeleton_chains: &BTreeMap<AvatarIkGoal, TwoBoneIkSkeletonChain>,
    look_at_bone_path: Option<&str>,
) {
    for (goal, chain_paths) in skeleton_chains {
        let Some(solved) = pose.solved_goal(*goal) else {
            continue;
        };
        let root_world =
            compute_skeleton_world_transform(root_transform, locals, &chain_paths.root_path);
        let mid_world =
            compute_skeleton_world_transform(root_transform, locals, &chain_paths.mid_path);
        let (solved_root_world, solved_mid_world, solved_tip_world) =
            solved.solved_bone_rotations(root_world.rotation, mid_world.rotation);

        let root_parent_world = compute_skeleton_world_transform(
            root_transform,
            locals,
            parent_skeleton_path(&chain_paths.root_path),
        );
        let new_root_local = normalize_rotation(unitas_core::multiply_rotation(
            conjugate_rotation(root_parent_world.rotation),
            solved_root_world,
        ));
        if let Some(root_local) = locals.get_mut(&chain_paths.root_path) {
            root_local.rotation = new_root_local;
        }

        let mid_parent_world = compute_skeleton_world_transform(
            root_transform,
            locals,
            parent_skeleton_path(&chain_paths.mid_path),
        );
        let new_mid_local = normalize_rotation(unitas_core::multiply_rotation(
            conjugate_rotation(mid_parent_world.rotation),
            solved_mid_world,
        ));
        if let Some(mid_local) = locals.get_mut(&chain_paths.mid_path) {
            mid_local.rotation = new_mid_local;
        }

        let tip_parent_world = compute_skeleton_world_transform(
            root_transform,
            locals,
            parent_skeleton_path(&chain_paths.tip_path),
        );
        let new_tip_local = normalize_rotation(unitas_core::multiply_rotation(
            conjugate_rotation(tip_parent_world.rotation),
            solved_tip_world,
        ));
        if let Some(tip_local) = locals.get_mut(&chain_paths.tip_path) {
            tip_local.rotation = new_tip_local;
        }
    }

    if let (Some(bone_path), Some(solved_look)) = (look_at_bone_path, pose.solved_look_at()) {
        let head_world = compute_skeleton_world_transform(root_transform, locals, bone_path);
        let new_head_world = normalize_rotation(unitas_core::multiply_rotation(
            solved_look.head_rotation,
            head_world.rotation,
        ));
        let head_parent_world = compute_skeleton_world_transform(
            root_transform,
            locals,
            parent_skeleton_path(bone_path),
        );
        let new_head_local = normalize_rotation(unitas_core::multiply_rotation(
            conjugate_rotation(head_parent_world.rotation),
            new_head_world,
        ));
        if let Some(head_local) = locals.get_mut(bone_path) {
            head_local.rotation = new_head_local;
        }
    }
}

fn build_ik_child_bindings(
    locals: &BTreeMap<String, Transform>,
    skeleton_chains: &BTreeMap<AvatarIkGoal, TwoBoneIkSkeletonChain>,
    look_at_bone_path: Option<&str>,
) -> Vec<SampledBinding> {
    let mut paths = Vec::new();
    for chain in skeleton_chains.values() {
        for p in [&chain.root_path, &chain.mid_path, &chain.tip_path] {
            if !p.is_empty() && !paths.contains(p) {
                paths.push(p.clone());
            }
        }
    }
    if let Some(look_path) = look_at_bone_path {
        let look_string = look_path.to_string();
        if !look_string.is_empty() && !paths.contains(&look_string) {
            paths.push(look_string);
        }
    }
    let mut bindings = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(transform) = locals.get(&path) {
            bindings.push(SampledBinding {
                target: CurveBindingTarget::transform(path, "localRotation"),
                value: AnimatedValue::Quaternion(transform.rotation),
            });
        }
    }
    bindings
}

fn merge_child_bindings(
    all_child_bindings: &mut Vec<(EntityId, Vec<SampledBinding>)>,
    entity: EntityId,
    ik_bindings: Vec<SampledBinding>,
) {
    if let Some((_, existing)) = all_child_bindings.iter_mut().find(|(e, _)| *e == entity) {
        for ik_binding in ik_bindings {
            if let Some(slot) = existing.iter_mut().find(|b| b.target == ik_binding.target) {
                slot.value = ik_binding.value;
            } else {
                existing.push(ik_binding);
            }
        }
    } else {
        all_child_bindings.push((entity, ik_bindings));
    }
}

fn sanitize_weight(weight: f32) -> f32 {
    if weight.is_finite() {
        weight.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn subtract_vectors(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add_vectors(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale_vector(v: [f32; 3], factor: f32) -> [f32; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

fn lerp_vector(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn dot_product(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn vector_length(v: [f32; 3]) -> f32 {
    dot_product(v, v).sqrt()
}

fn normalize_vector(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let len = vector_length(v);
    if len > 1e-6 && len.is_finite() {
        scale_vector(v, 1.0 / len)
    } else {
        fallback
    }
}

fn perpendicular_component(v: [f32; 3], unit_axis: [f32; 3]) -> [f32; 3] {
    let proj = dot_product(v, unit_axis);
    subtract_vectors(v, scale_vector(unit_axis, proj))
}

fn fallback_perpendicular(candidate: [f32; 3], unit_axis: [f32; 3]) -> [f32; 3] {
    let len = vector_length(candidate);
    if len > 1e-6 && len.is_finite() {
        return scale_vector(candidate, 1.0 / len);
    }
    let reference = if unit_axis[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    normalize_vector(cross_product(unit_axis, reference), [0.0, 0.0, 1.0])
}

fn rotation_from_to(from: [f32; 3], to: [f32; 3]) -> [f32; 4] {
    rotation_from_to_with_up(from, to, [0.0, 1.0, 0.0])
}

fn rotation_from_to_with_up(from: [f32; 3], to: [f32; 3], up_hint: [f32; 3]) -> [f32; 4] {
    let u = normalize_vector(from, [0.0, 0.0, 1.0]);
    let v = normalize_vector(to, u);
    let d = dot_product(u, v).clamp(-1.0, 1.0);
    if d >= 1.0 - 1e-6 {
        return unitas_core::IDENTITY_ROTATION;
    }
    if d <= -1.0 + 1e-6 {
        let ortho = fallback_perpendicular(perpendicular_component(up_hint, u), u);
        return normalize_rotation([ortho[0], ortho[1], ortho[2], 0.0]);
    }
    let axis = cross_product(u, v);
    normalize_rotation([axis[0], axis[1], axis[2], 1.0 + d])
}

fn slerp_rotation(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = sanitize_weight(t);
    if t <= 1e-6 {
        return normalize_rotation(a);
    }
    if t >= 1.0 - 1e-6 {
        return normalize_rotation(b);
    }
    let qa = normalize_rotation(a);
    let mut qb = normalize_rotation(b);
    let mut cos_half = qa[0] * qb[0] + qa[1] * qb[1] + qa[2] * qb[2] + qa[3] * qb[3];
    if cos_half < 0.0 {
        qb = [-qb[0], -qb[1], -qb[2], -qb[3]];
        cos_half = -cos_half;
    }
    if cos_half > 0.9995 {
        return normalize_rotation([
            qa[0] + (qb[0] - qa[0]) * t,
            qa[1] + (qb[1] - qa[1]) * t,
            qa[2] + (qb[2] - qa[2]) * t,
            qa[3] + (qb[3] - qa[3]) * t,
        ]);
    }
    let half_angle = cos_half.clamp(-1.0, 1.0).acos();
    let sin_half = half_angle.sin();
    if sin_half.abs() <= 1e-6 {
        return qa;
    }
    let ratio_a = ((1.0 - t) * half_angle).sin() / sin_half;
    let ratio_b = (t * half_angle).sin() / sin_half;
    normalize_rotation([
        qa[0] * ratio_a + qb[0] * ratio_b,
        qa[1] * ratio_a + qb[1] * ratio_b,
        qa[2] * ratio_a + qb[2] * ratio_b,
        qa[3] * ratio_a + qb[3] * ratio_b,
    ])
}

fn conjugate_rotation(rotation: [f32; 4]) -> [f32; 4] {
    [-rotation[0], -rotation[1], -rotation[2], rotation[3]]
}

fn rotation_delta(previous: [f32; 4], current: [f32; 4]) -> [f32; 4] {
    unitas_core::multiply_rotation(current, conjugate_rotation(previous))
}
