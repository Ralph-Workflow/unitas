use std::collections::BTreeMap;

use unitas_animation::{
    AnimatedValue, AnimationDiagnostic, AnimationError, AnimationEvent, Animator,
    AnimatorController, AnimatorStateInfo, SampledBinding, TransformChannel, TransformProperty,
};
use unitas_core::{normalize_rotation, EntityId, Transform};

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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimatorIkPose {
    pub body_position: Option<[f32; 3]>,
}

pub trait AnimatorScriptHooks {
    fn on_animator_ik(&mut self, _entity: EntityId, _layer_index: u32, _pose: &mut AnimatorIkPose) {}
    fn on_animator_move(&mut self, _entity: EntityId, _motion: &mut AnimatorRootMotion) -> bool {
        false
    }
}

struct AnimatorHookEntry {
    ik_layers: Vec<u32>,
    script: Option<Box<dyn AnimatorScriptHooks>>,
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

    fn entry_mut(&mut self, entity: EntityId) -> &mut AnimatorHookEntry {
        self.entries.entry(entity).or_insert_with(|| AnimatorHookEntry {
            ik_layers: Vec::new(),
            script: None,
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
        let child_bindings = self.tick(scaled_delta_seconds);
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
            let ik_layers = hook_entry.ik_layers.clone();
            let mut motion = AnimatorRootMotion {
                delta_position: subtract_vectors(
                    entry.transform.translation,
                    previous_transform.translation,
                ),
                delta_rotation: rotation_delta(
                    previous_transform.rotation,
                    entry.transform.rotation,
                ),
            };
            if script.on_animator_move(entity, &mut motion) {
                entry.transform.translation =
                    add_vectors(previous_transform.translation, motion.delta_position);
                entry.transform.rotation = unitas_core::multiply_rotation(
                    motion.delta_rotation,
                    previous_transform.rotation,
                );
            }
            for layer_index in ik_layers {
                let mut pose = AnimatorIkPose::default();
                script.on_animator_ik(entity, layer_index, &mut pose);
                if let Some(body_position) = pose.body_position {
                    entry.transform.translation = body_position;
                }
            }
        }
        child_bindings
    }
}

fn subtract_vectors(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add_vectors(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn conjugate_rotation(rotation: [f32; 4]) -> [f32; 4] {
    [-rotation[0], -rotation[1], -rotation[2], rotation[3]]
}

fn rotation_delta(previous: [f32; 4], current: [f32; 4]) -> [f32; 4] {
    unitas_core::multiply_rotation(current, conjugate_rotation(previous))
}
