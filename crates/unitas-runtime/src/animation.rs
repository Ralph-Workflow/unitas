use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use unitas_animation::curve::AssetReference;
use unitas_animation::{
    AnimatedValue, AnimationClip, AnimationDiagnostic, AnimationError, AnimationEvent, Animator,
    AnimatorController, AnimatorCullingMode, AnimatorFrame, AnimatorStateInfo, AnimatorStepResult,
    AnimatorUpdateMode, CurveBindingTarget, SampledBinding, StateMachineCallback,
    StateMachineCallbackKind, TransformChannel, TransformProperty, WrapMode,
    blend_sampled_bindings,
};
use unitas_core::{EntityId, Transform, compose_world, normalize_rotation};
use unitas_jobs::JobSystem;
use unitas_render::image::Image;

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
    UnresolvedObjectReference {
        entity: EntityId,
        object_path: String,
        component: String,
        property: String,
        guid: String,
        file_id: i64,
    },
    LegacyAnimation {
        entity: EntityId,
        message: String,
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
            Self::UnresolvedObjectReference {
                entity,
                object_path,
                component,
                property,
                guid,
                file_id,
            } => write!(
                formatter,
                "{component}.{property} on entity {} (path '{object_path}') references sprite asset guid {guid} (fileID {file_id}) that could not be resolved",
                entity.as_u64()
            ),
            Self::LegacyAnimation { message, .. } => {
                write!(formatter, "legacy animation: {message}")
            }
        }
    }
}

pub fn is_sprite_renderer_sprite_binding(target: &CurveBindingTarget) -> bool {
    target.component == "SpriteRenderer"
        && matches!(target.property.as_str(), "m_Sprite" | "sprite")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedObjectReference {
    pub target: CurveBindingTarget,
    pub reference: AssetReference,
    pub changes: u32,
}

struct ParallelAnimatorOutput {
    animator: Animator,
    result: AnimatorStepResult,
    worker: std::thread::ThreadId,
}

type ParallelAnimatorSlot = Arc<Mutex<Option<ParallelAnimatorOutput>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct EntityAnimatorEntry {
    pub controller_handle: u64,
    pub animator: Option<Animator>,
    pub transform: Transform,
    pub speed: f32,
    pub fired_events: Vec<AnimationEvent>,
    pub typed_diagnostics: Vec<AnimationRuntimeDiagnostic>,
    pub diagnostics: Vec<String>,
    pub update_mode: AnimatorUpdateMode,
    pub culling_mode: AnimatorCullingMode,
    pub visible: bool,
    pub enabled: bool,
    pub keep_state_on_disable: bool,
    pub callbacks: Vec<StateMachineCallback>,
    pub root_motion: AnimatorRootMotion,
    pub writes_transforms: bool,
    pub applied_object_references: Vec<AppliedObjectReference>,
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
            update_mode: AnimatorUpdateMode::Normal,
            culling_mode: AnimatorCullingMode::AlwaysAnimate,
            visible: true,
            enabled: true,
            keep_state_on_disable: false,
            callbacks: Vec::new(),
            root_motion: AnimatorRootMotion::default(),
            writes_transforms: true,
            applied_object_references: Vec::new(),
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
    legacy: BTreeMap<EntityId, LegacyAnimationComponent>,
    animated: BTreeSet<EntityId>,
    pending_transform_writes: BTreeSet<EntityId>,
    sprite_assets: BTreeMap<String, PathBuf>,
    sprite_textures: BTreeMap<PathBuf, Arc<Image>>,
    resolved_sprites: BTreeMap<(EntityId, CurveBindingTarget), Option<Arc<Image>>>,
    last_job_workers: usize,
}

impl AnimationWorld {
    pub fn new() -> Self {
        Self {
            controllers: BTreeMap::new(),
            next_controller_handle: 1,
            entries: BTreeMap::new(),
            legacy: BTreeMap::new(),
            animated: BTreeSet::new(),
            pending_transform_writes: BTreeSet::new(),
            sprite_assets: BTreeMap::new(),
            sprite_textures: BTreeMap::new(),
            resolved_sprites: BTreeMap::new(),
            last_job_workers: 0,
        }
    }

    pub fn register_sprite_asset(&mut self, guid: &str, path: &Path) {
        let lower = guid.trim().to_ascii_lowercase();
        self.sprite_assets.insert(lower, path.to_path_buf());
    }

    pub fn register_sprite_texture(&mut self, guid: &str, path: &Path, texture: Arc<Image>) {
        let lower = guid.trim().to_ascii_lowercase();
        self.sprite_assets.insert(lower, path.to_path_buf());
        self.sprite_textures.insert(path.to_path_buf(), texture);
    }

    pub fn has_sprite_assets(&self) -> bool {
        !self.sprite_assets.is_empty()
    }

    pub fn resolved_sprite(
        &self,
        entity: EntityId,
        target: &CurveBindingTarget,
    ) -> Option<Option<&Arc<Image>>> {
        self.resolved_sprites
            .get(&(entity, target.clone()))
            .map(Option::as_ref)
    }

    pub fn clear_sprite_assets(&mut self) {
        self.sprite_assets.clear();
        self.sprite_textures.clear();
        self.resolved_sprites.clear();
    }

    pub fn last_job_workers(&self) -> usize {
        self.last_job_workers
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
        self.pending_transform_writes.insert(entity);
        self.entries
            .entry(entity)
            .or_insert_with(|| EntityAnimatorEntry::new(initial_transform))
    }

    pub fn remove_animator(&mut self, entity: EntityId) -> bool {
        self.animated.remove(&entity);
        self.pending_transform_writes.remove(&entity);
        self.resolved_sprites
            .retain(|(owner, _), _| *owner != entity);
        self.entries.remove(&entity).is_some()
    }

    pub fn entry(&self, entity: EntityId) -> Option<&EntityAnimatorEntry> {
        self.entries.get(&entity)
    }

    pub fn entry_mut(&mut self, entity: EntityId) -> Option<&mut EntityAnimatorEntry> {
        self.entries.get_mut(&entity)
    }

    pub fn applied_object_reference(
        &self,
        entity: EntityId,
        target: &CurveBindingTarget,
    ) -> Option<&AppliedObjectReference> {
        self.entries
            .get(&entity)?
            .applied_object_references
            .iter()
            .find(|applied| applied.target == *target)
    }

    pub fn applied_object_references(&self, entity: EntityId) -> &[AppliedObjectReference] {
        self.entries
            .get(&entity)
            .map(|entry| entry.applied_object_references.as_slice())
            .unwrap_or_default()
    }

    pub fn entities(&self) -> Vec<EntityId> {
        self.entries.keys().copied().collect()
    }

    pub(crate) fn take_pending_transform_writes(&mut self) -> Vec<EntityId> {
        std::mem::take(&mut self.pending_transform_writes)
            .into_iter()
            .collect()
    }

    pub(crate) fn queue_transform_write(&mut self, entity: EntityId) {
        self.pending_transform_writes.insert(entity);
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
            self.animated.remove(&entity);
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
        animator.set_update_mode(entry.update_mode);
        animator.set_culling_mode(entry.culling_mode);
        animator.set_visible(entry.visible);
        let initial_diags: Vec<AnimationDiagnostic> = animator.diagnostics().to_vec();
        entry.controller_handle = controller_handle;
        entry.fired_events.clear();
        entry.typed_diagnostics.clear();
        entry.diagnostics.clear();
        for diag in initial_diags {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
        }
        let defaults = animator.captured_defaults().to_vec();
        let mut transform = entry.transform.clone();
        let mut default_children =
            apply_bindings_to_transform(entity, entry, &mut transform, &defaults);
        let step_result = animator.step(0.0);
        for diag in step_result.diagnostics {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
        }
        entry.fired_events.extend(step_result.fired_events);
        entry.callbacks = step_result.callbacks;
        entry.animator = Some(animator);
        self.animated.insert(entity);
        let child_bindings =
            apply_bindings_to_transform(entity, entry, &mut transform, &step_result.bindings);
        entry.transform = transform;
        self.pending_transform_writes.insert(entity);
        default_children.extend(child_bindings);
        self.resolve_entity_object_references(entity);
        Ok(default_children)
    }

    pub fn set_speed(&mut self, entity: EntityId, speed: f32) -> Result<(), String> {
        if !speed.is_finite() {
            return Err("animator speed must be finite".to_string());
        }
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.speed = speed;
        if let Some(animator) = &mut entry.animator {
            animator.set_speed(speed);
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
        self.pending_transform_writes.insert(entity);
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

    pub fn record_diagnostic(&mut self, entity: EntityId, diagnostic: AnimationRuntimeDiagnostic) {
        if let Some(entry) = self.entries.get_mut(&entity) {
            entry.push_diagnostic(diagnostic);
        }
    }

    pub fn tick(&mut self, scaled_delta_seconds: f64) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let delta = if scaled_delta_seconds.is_finite() && scaled_delta_seconds > 0.0 {
            scaled_delta_seconds as f32
        } else {
            0.0
        };
        self.tick_frame(&AnimatorFrame::new(delta, delta, 1.0 / 60.0))
    }

    pub fn tick_parallel(
        &mut self,
        scaled_delta_seconds: f64,
        jobs: &JobSystem,
    ) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let delta = if scaled_delta_seconds.is_finite() && scaled_delta_seconds > 0.0 {
            scaled_delta_seconds as f32
        } else {
            0.0
        };
        self.tick_frame_parallel(&AnimatorFrame::new(delta, delta, 1.0 / 60.0), jobs)
    }

    pub fn tick_frame(&mut self, frame: &AnimatorFrame) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let mut all_child_bindings = Vec::new();
        let animated: Vec<EntityId> = self.animated.iter().copied().collect();
        let mut written = Vec::new();
        for entity in animated {
            let Some(entry) = self.entries.get_mut(&entity) else {
                continue;
            };
            if !entry.enabled {
                continue;
            }
            let Some(animator) = &mut entry.animator else {
                continue;
            };
            let step_result = animator.step_frame(frame);
            if Self::commit_animator_step(entity, entry, step_result, &mut all_child_bindings) {
                written.push(entity);
            }
        }
        for entity in written {
            self.pending_transform_writes.insert(entity);
        }
        self.last_job_workers = usize::from(!self.animated.is_empty());
        self.tick_legacy(frame, &mut all_child_bindings);
        self.resolve_all_object_references();
        all_child_bindings
    }

    pub fn tick_frame_parallel(
        &mut self,
        frame: &AnimatorFrame,
        jobs: &JobSystem,
    ) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let mut all_child_bindings = Vec::new();
        let animated: Vec<EntityId> = self.animated.iter().copied().collect();
        let mut tasks = Vec::with_capacity(animated.len());
        let mut slots: Vec<ParallelAnimatorSlot> = Vec::with_capacity(animated.len());
        for entity in animated {
            let Some(entry) = self.entries.get_mut(&entity) else {
                continue;
            };
            if !entry.enabled {
                continue;
            }
            let Some(animator) = entry.animator.take() else {
                continue;
            };
            tasks.push((entity, Mutex::new(Some(animator))));
            slots.push(Arc::new(Mutex::new(None)));
        }
        let task_count = tasks.len();
        if task_count == 0 {
            self.last_job_workers = 0;
            self.tick_legacy(frame, &mut all_child_bindings);
            self.resolve_all_object_references();
            return all_child_bindings;
        }
        let shared_tasks = Arc::new(tasks);
        let shared_slots = Arc::new(slots);
        let frame_copy = *frame;
        let workers = jobs.active_workers().max(1);
        let batch_size = if workers <= 1 {
            i64::try_from(task_count).unwrap_or(1)
        } else {
            let target_batches = (workers * 4).min(task_count).max(workers);
            let raw = (task_count / target_batches).max(1);
            i64::try_from(raw).unwrap_or(1)
        };
        let length = i64::try_from(task_count).unwrap_or(0);
        let job_tasks = Arc::clone(&shared_tasks);
        let job_slots = Arc::clone(&shared_slots);
        let handle = jobs.schedule_parallel_for(
            length,
            batch_size,
            move |start, count| {
                let worker = std::thread::current().id();
                let start_idx = usize::try_from(start).unwrap_or(0);
                let count_idx = usize::try_from(count).unwrap_or(0);
                let end_idx = start_idx.saturating_add(count_idx).min(job_tasks.len());
                for idx in start_idx..end_idx {
                    let taken = job_tasks[idx]
                        .1
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .take();
                    if let Some(mut animator) = taken {
                        let result = animator.step_frame(&frame_copy);
                        let mut slot = job_slots[idx]
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        *slot = Some(ParallelAnimatorOutput {
                            animator,
                            result,
                            worker,
                        });
                    }
                }
            },
            &[],
        );
        jobs.complete(handle);
        let mut written = Vec::new();
        let mut observed_threads = Vec::new();
        for (idx, (entity, _)) in shared_tasks.iter().enumerate() {
            let outcome = shared_slots[idx]
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            let Some(output) = outcome else {
                continue;
            };
            if !observed_threads.contains(&output.worker) {
                observed_threads.push(output.worker);
            }
            let Some(entry) = self.entries.get_mut(entity) else {
                continue;
            };
            entry.animator = Some(output.animator);
            if Self::commit_animator_step(*entity, entry, output.result, &mut all_child_bindings) {
                written.push(*entity);
            }
        }
        for entity in written {
            self.pending_transform_writes.insert(entity);
        }
        self.last_job_workers = observed_threads.len();
        self.tick_legacy(frame, &mut all_child_bindings);
        self.resolve_all_object_references();
        all_child_bindings
    }

    fn commit_animator_step(
        entity: EntityId,
        entry: &mut EntityAnimatorEntry,
        step_result: AnimatorStepResult,
        all_child_bindings: &mut Vec<(EntityId, Vec<SampledBinding>)>,
    ) -> bool {
        for diag in step_result.diagnostics {
            entry.push_diagnostic(AnimationRuntimeDiagnostic::Controller(diag));
        }
        entry.fired_events.extend(step_result.fired_events);
        entry.callbacks = step_result.callbacks;
        entry.writes_transforms = step_result.writes_transforms;
        let previous = entry.transform.clone();
        let mut sampled = entry.transform.clone();
        let child_bindings =
            apply_bindings_to_transform(entity, entry, &mut sampled, &step_result.bindings);
        entry.root_motion = AnimatorRootMotion {
            delta_position: subtract_vectors(sampled.translation, previous.translation),
            delta_rotation: rotation_delta(previous.rotation, sampled.rotation),
        };
        let wrote = step_result.writes_transforms;
        if wrote {
            entry.transform = sampled;
        }
        if !child_bindings.is_empty() {
            all_child_bindings.push((entity, child_bindings));
        }
        wrote
    }

    fn tick_legacy(
        &mut self,
        frame: &AnimatorFrame,
        all_child_bindings: &mut Vec<(EntityId, Vec<SampledBinding>)>,
    ) {
        let entities: Vec<EntityId> = self.legacy.keys().copied().collect();
        for entity in entities {
            let Some(component) = self.legacy.get_mut(&entity) else {
                continue;
            };
            let sample = component.step(frame.scaled_delta);
            let Some(entry) = self.entries.get_mut(&entity) else {
                continue;
            };
            for message in &sample.errors {
                entry.push_diagnostic(AnimationRuntimeDiagnostic::LegacyAnimation {
                    entity,
                    message: message.clone(),
                });
            }
            if sample.bindings.is_empty() {
                continue;
            }
            entry.fired_events.extend(sample.fired_events);
            let previous = entry.transform.clone();
            let mut sampled = entry.transform.clone();
            let child_bindings =
                apply_bindings_to_transform(entity, entry, &mut sampled, &sample.bindings);
            let weight = component.weight();
            if weight < 1.0 {
                sampled.translation =
                    blend_vector(previous.translation, sampled.translation, weight);
                sampled.rotation = blend_quaternion(previous.rotation, sampled.rotation, weight);
                sampled.scale = blend_vector(previous.scale, sampled.scale, weight);
            }
            entry.transform = sampled;
            self.pending_transform_writes.insert(entity);
            if !child_bindings.is_empty() {
                all_child_bindings.push((entity, child_bindings));
            }
        }
    }

    pub fn set_update_mode(
        &mut self,
        entity: EntityId,
        mode: AnimatorUpdateMode,
    ) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.update_mode = mode;
        if let Some(animator) = &mut entry.animator {
            animator.set_update_mode(mode);
        }
        Ok(())
    }

    pub fn update_mode(&self, entity: EntityId) -> Option<AnimatorUpdateMode> {
        self.entries.get(&entity).map(|entry| entry.update_mode)
    }

    pub fn set_culling_mode(
        &mut self,
        entity: EntityId,
        mode: AnimatorCullingMode,
    ) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.culling_mode = mode;
        if let Some(animator) = &mut entry.animator {
            animator.set_culling_mode(mode);
        }
        Ok(())
    }

    pub fn culling_mode(&self, entity: EntityId) -> Option<AnimatorCullingMode> {
        self.entries.get(&entity).map(|entry| entry.culling_mode)
    }

    pub fn set_visible(&mut self, entity: EntityId, visible: bool) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.visible = visible;
        if let Some(animator) = &mut entry.animator {
            animator.set_visible(visible);
        }
        Ok(())
    }

    pub fn is_visible(&self, entity: EntityId) -> Option<bool> {
        self.entries.get(&entity).map(|entry| entry.visible)
    }

    pub fn set_keep_state_on_disable(
        &mut self,
        entity: EntityId,
        keep: bool,
    ) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.keep_state_on_disable = keep;
        Ok(())
    }

    pub fn keep_state_on_disable(&self, entity: EntityId) -> Option<bool> {
        self.entries
            .get(&entity)
            .map(|entry| entry.keep_state_on_disable)
    }

    pub fn disable_animator(&mut self, entity: EntityId) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        if !entry.enabled {
            return Ok(());
        }
        entry.enabled = false;
        if entry.keep_state_on_disable {
            return Ok(());
        }
        let (machine, defaults) = {
            let Some(animator) = &mut entry.animator else {
                return Ok(());
            };
            let machine = animator.controller().name.clone();
            animator.reset();
            (machine, animator.captured_defaults().to_vec())
        };
        entry.callbacks = vec![StateMachineCallback {
            kind: StateMachineCallbackKind::OnStateMachineExit,
            state_index: 0,
            state_name: String::new(),
            machine: Some(machine),
        }];
        let mut transform = entry.transform.clone();
        apply_bindings_to_transform(entity, entry, &mut transform, &defaults);
        entry.transform = transform;
        self.pending_transform_writes.insert(entity);
        self.resolve_entity_object_references(entity);
        Ok(())
    }

    pub fn enable_animator(&mut self, entity: EntityId) -> Result<(), String> {
        let Some(entry) = self.entries.get_mut(&entity) else {
            return Err(format!("animator not found for entity {}", entity.as_u64()));
        };
        entry.enabled = true;
        Ok(())
    }

    pub fn is_enabled(&self, entity: EntityId) -> Option<bool> {
        self.entries.get(&entity).map(|entry| entry.enabled)
    }

    pub fn callbacks(&self, entity: EntityId) -> Option<&[StateMachineCallback]> {
        self.entries
            .get(&entity)
            .map(|entry| entry.callbacks.as_slice())
    }

    pub fn root_motion(&self, entity: EntityId) -> Option<AnimatorRootMotion> {
        self.entries.get(&entity).map(|entry| entry.root_motion)
    }

    pub fn set_legacy_animation(&mut self, entity: EntityId, component: LegacyAnimationComponent) {
        self.legacy.insert(entity, component);
    }

    pub fn legacy_animation(&self, entity: EntityId) -> Option<&LegacyAnimationComponent> {
        self.legacy.get(&entity)
    }

    pub fn legacy_animation_mut(
        &mut self,
        entity: EntityId,
    ) -> Option<&mut LegacyAnimationComponent> {
        self.legacy.get_mut(&entity)
    }

    pub fn remove_legacy_animation(&mut self, entity: EntityId) -> bool {
        self.legacy.remove(&entity).is_some()
    }

    fn resolve_all_object_references(&mut self) {
        if self.sprite_assets.is_empty() {
            return;
        }
        let entities: Vec<EntityId> = self.entries.keys().copied().collect();
        for entity in entities {
            self.resolve_entity_object_references(entity);
        }
    }

    fn resolve_entity_object_references(&mut self, entity: EntityId) {
        if self.sprite_assets.is_empty() {
            return;
        }
        let Some(applied_list) = self
            .entries
            .get(&entity)
            .map(|entry| entry.applied_object_references.clone())
        else {
            return;
        };
        for applied in applied_list {
            if !is_sprite_renderer_sprite_binding(&applied.target) {
                continue;
            }
            if applied.reference.is_null() {
                self.resolved_sprites.insert((entity, applied.target), None);
                continue;
            }
            let guid = applied.reference.guid_hex();
            let path = self.sprite_assets.get(&guid).cloned();
            let texture = match path {
                Some(ref asset_path) => {
                    if let Some(cached) = self.sprite_textures.get(asset_path) {
                        Some(Arc::clone(cached))
                    } else if let Some(loaded) = load_sprite_texture_image(asset_path) {
                        let arc = Arc::new(loaded);
                        self.sprite_textures
                            .insert(asset_path.clone(), Arc::clone(&arc));
                        Some(arc)
                    } else {
                        None
                    }
                }
                None => None,
            };
            if let Some(tex) = texture {
                self.resolved_sprites
                    .insert((entity, applied.target), Some(tex));
            } else if let Some(entry) = self.entries.get_mut(&entity) {
                entry.push_diagnostic(AnimationRuntimeDiagnostic::UnresolvedObjectReference {
                    entity,
                    object_path: applied.target.object_path.clone(),
                    component: applied.target.component.clone(),
                    property: applied.target.property.clone(),
                    guid,
                    file_id: applied.reference.file_id,
                });
            }
        }
    }
}

fn load_sprite_texture_image(path: &Path) -> Option<Image> {
    let cooked = unitas_asset::texture::load_or_cook_texture(path)?;
    let mut image = Image::new(cooked.width, cooked.height);
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let offset = index * 4;
        pixel[0] = cooked.rgba.get(offset).copied().unwrap_or(0);
        pixel[1] = cooked.rgba.get(offset + 1).copied().unwrap_or(0);
        pixel[2] = cooked.rgba.get(offset + 2).copied().unwrap_or(0);
    }
    if cooked.srgb {
        for pixel in &mut image.pixels {
            for channel in pixel.iter_mut() {
                let linear = unitas_asset::texture::import::srgb_to_linear(*channel);
                *channel = (linear * 255.0).round() as u8;
            }
        }
    }
    Some(image)
}

fn current_state_index(entry: &EntityAnimatorEntry) -> usize {
    entry
        .animator
        .as_ref()
        .map(|animator| animator.current_state_info().state_index)
        .unwrap_or(0)
}

fn apply_bindings_to_transform(
    entity: EntityId,
    entry: &mut EntityAnimatorEntry,
    transform: &mut Transform,
    bindings: &[SampledBinding],
) -> Vec<SampledBinding> {
    let mut child_bindings = Vec::new();
    for binding in bindings {
        if let AnimatedValue::ObjectReference(reference) = binding.value {
            apply_object_reference(entry, &binding.target, reference);
            continue;
        }
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
            apply_transform_property(transform, prop, ch, binding.value);
        } else {
            child_bindings.push(binding.clone());
        }
    }
    child_bindings
}

fn apply_object_reference(
    entry: &mut EntityAnimatorEntry,
    target: &CurveBindingTarget,
    reference: AssetReference,
) {
    if let Some(applied) = entry
        .applied_object_references
        .iter_mut()
        .find(|applied| applied.target == *target)
    {
        if applied.reference != reference {
            applied.reference = reference;
            applied.changes = applied.changes.saturating_add(1);
        }
        return;
    }
    entry
        .applied_object_references
        .push(AppliedObjectReference {
            target: target.clone(),
            reference,
            changes: 0,
        });
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyCullingType {
    AlwaysAnimate,
    CullUpdateTransforms,
    CullCompletely,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyClipEntry {
    pub name: String,
    pub clip: AnimationClip,
    pub legacy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyAnimationState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyAnimationSample {
    pub bindings: Vec<SampledBinding>,
    pub fired_events: Vec<AnimationEvent>,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LegacyCrossFade {
    from: usize,
    elapsed: f32,
    duration: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyAnimationComponent {
    clips: Vec<LegacyClipEntry>,
    playing: Option<usize>,
    queue: Vec<usize>,
    cross_fade: Option<LegacyCrossFade>,
    blended: Vec<(usize, f32)>,
    time: f32,
    weight: f32,
    layer: usize,
    culling: LegacyCullingType,
    visible: bool,
    errors: Vec<String>,
}

impl Default for LegacyAnimationComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyAnimationComponent {
    pub fn new() -> Self {
        Self {
            clips: Vec::new(),
            playing: None,
            queue: Vec::new(),
            cross_fade: None,
            blended: Vec::new(),
            time: 0.0,
            weight: 1.0,
            layer: 0,
            culling: LegacyCullingType::AlwaysAnimate,
            visible: true,
            errors: Vec::new(),
        }
    }

    pub fn add_clip(&mut self, name: impl Into<String>, clip: AnimationClip, legacy: bool) {
        let name = name.into();
        self.clips.retain(|entry| entry.name != name);
        self.clips.push(LegacyClipEntry { name, clip, legacy });
    }

    pub fn clip_names(&self) -> Vec<String> {
        self.clips.iter().map(|entry| entry.name.clone()).collect()
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn set_weight(&mut self, weight: f32) {
        if weight.is_finite() {
            self.weight = weight.clamp(0.0, 1.0);
        }
    }

    pub fn layer(&self) -> usize {
        self.layer
    }

    pub fn set_layer(&mut self, layer: usize) {
        self.layer = layer;
    }

    pub fn culling_type(&self) -> LegacyCullingType {
        self.culling
    }

    pub fn set_culling_type(&mut self, culling: LegacyCullingType) {
        self.culling = culling;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn state(&self, name: &str) -> LegacyAnimationState {
        let Some(index) = self.clips.iter().position(|entry| entry.name == name) else {
            return LegacyAnimationState::Stopped;
        };
        if self.playing == Some(index) {
            LegacyAnimationState::Playing
        } else {
            LegacyAnimationState::Stopped
        }
    }

    pub fn is_playing(&self, name: &str) -> bool {
        self.state(name) == LegacyAnimationState::Playing
    }

    pub fn play(&mut self, name: &str) -> Result<(), String> {
        let index = match self.resolve(name) {
            Ok(index) => index,
            Err(message) => return self.fail(message),
        };
        self.playing = Some(index);
        self.time = 0.0;
        self.cross_fade = None;
        self.blended.clear();
        Ok(())
    }

    pub fn cross_fade(&mut self, name: &str, fade_length: f32) -> Result<(), String> {
        let index = match self.resolve(name) {
            Ok(index) => index,
            Err(message) => return self.fail(message),
        };
        let duration = if fade_length.is_finite() {
            fade_length.max(0.0)
        } else {
            0.0
        };
        if duration <= 0.0 {
            self.playing = Some(index);
            self.time = 0.0;
            self.cross_fade = None;
            return Ok(());
        }
        if let Some(previous) = self.playing {
            self.cross_fade = Some(LegacyCrossFade {
                from: previous,
                elapsed: 0.0,
                duration,
            });
        }
        self.playing = Some(index);
        self.time = 0.0;
        self.blended.clear();
        Ok(())
    }

    pub fn blend(&mut self, name: &str, weight: f32) -> Result<(), String> {
        let index = match self.resolve(name) {
            Ok(index) => index,
            Err(message) => return self.fail(message),
        };
        let weight = if weight.is_finite() {
            weight.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.blended.retain(|(existing, _)| *existing != index);
        if weight > 0.0 {
            self.blended.push((index, weight));
        }
        Ok(())
    }

    pub fn play_queued(&mut self, name: &str) -> Result<(), String> {
        let index = match self.resolve(name) {
            Ok(index) => index,
            Err(message) => return self.fail(message),
        };
        self.queue.push(index);
        Ok(())
    }

    pub fn step(&mut self, delta_seconds: f32) -> LegacyAnimationSample {
        let delta = if delta_seconds.is_finite() && delta_seconds > 0.0 {
            delta_seconds
        } else {
            0.0
        };
        if self.culling == LegacyCullingType::CullCompletely && !self.visible {
            return LegacyAnimationSample {
                bindings: Vec::new(),
                fired_events: Vec::new(),
                errors: Vec::new(),
            };
        }
        let Some(index) = self.playing else {
            return LegacyAnimationSample {
                bindings: Vec::new(),
                fired_events: Vec::new(),
                errors: Vec::new(),
            };
        };
        let previous_time = self.time;
        self.time += delta;
        let sample = self.clips[index].clip.sample(previous_time, self.time);
        let mut bindings = sample.bindings;
        let mut fired_events = sample.fired_events;
        if let Some(mut fade) = self.cross_fade {
            fade.elapsed += delta;
            let alpha = if fade.duration > 0.0 {
                (fade.elapsed / fade.duration).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let from_sample = self.clips[fade.from].clip.sample(previous_time, self.time);
            bindings = blend_sampled_bindings(&from_sample.bindings, &bindings, alpha);
            fired_events.extend(from_sample.fired_events);
            if alpha >= 1.0 {
                self.cross_fade = None;
            } else {
                self.cross_fade = Some(fade);
            }
        }
        for (blend_index, weight) in self.blended.clone() {
            let blend_sample = self.clips[blend_index]
                .clip
                .sample(previous_time, self.time);
            bindings = blend_sampled_bindings(&bindings, &blend_sample.bindings, weight);
            fired_events.extend(blend_sample.fired_events);
        }
        let duration = self.clips[index].clip.effective_duration();
        let finished = duration > 0.0
            && self.time >= duration
            && matches!(
                self.clips[index].clip.wrap_mode,
                WrapMode::Once | WrapMode::ClampForever
            );
        if finished {
            if let Some(next) = self.queue.first().copied() {
                self.queue.remove(0);
                self.playing = Some(next);
                self.time = 0.0;
            } else {
                self.playing = None;
                self.time = duration;
            }
        }
        LegacyAnimationSample {
            bindings,
            fired_events,
            errors: Vec::new(),
        }
    }

    fn resolve(&self, name: &str) -> Result<usize, String> {
        let Some(index) = self.clips.iter().position(|entry| entry.name == name) else {
            return Err(format!(
                "animation clip '{name}' is not assigned to the legacy animation component"
            ));
        };
        if !self.clips[index].legacy {
            return Err(format!(
                "animation clip '{name}' is not marked legacy and does not play"
            ));
        }
        Ok(index)
    }

    fn fail(&mut self, message: String) -> Result<(), String> {
        if !self.errors.contains(&message) {
            self.errors.push(message.clone());
        }
        Err(message)
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
        let cos_alpha =
            ((l1 * l1 + clamped_d * clamped_d - l2 * l2) / (2.0 * l1 * clamped_d)).clamp(-1.0, 1.0);
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
        self.goals.get(&goal).map_or([0.0; 3], |pose| pose.position)
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
        self.hints.get(&hint).map_or([0.0; 3], |pose| pose.position)
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
    fn on_animator_ik(&mut self, _entity: EntityId, _layer_index: u32, _pose: &mut AnimatorIkPose) {
    }
    fn on_animator_move(&mut self, _entity: EntityId, _motion: &mut AnimatorRootMotion) -> bool {
        false
    }
    fn on_state_enter(&mut self, _entity: EntityId, _callback: &StateMachineCallback) {}
    fn on_state_update(&mut self, _entity: EntityId, _callback: &StateMachineCallback) {}
    fn on_state_exit(&mut self, _entity: EntityId, _callback: &StateMachineCallback) {}
    fn on_state_machine_enter(&mut self, _entity: EntityId, _machine: &str) {}
    fn on_state_machine_exit(&mut self, _entity: EntityId, _machine: &str) {}
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
        self.entries
            .entry(entity)
            .or_insert_with(|| AnimatorHookEntry {
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
        let delta = if scaled_delta_seconds.is_finite() && scaled_delta_seconds > 0.0 {
            scaled_delta_seconds as f32
        } else {
            0.0
        };
        self.tick_frame_with_hooks(&AnimatorFrame::new(delta, delta, 1.0 / 60.0), hooks)
    }

    pub fn tick_frame_with_hooks(
        &mut self,
        frame: &AnimatorFrame,
        hooks: &mut AnimatorHookWorld,
    ) -> Vec<(EntityId, Vec<SampledBinding>)> {
        let previous: Vec<(EntityId, Transform)> = self
            .entries
            .iter()
            .map(|(entity, entry)| (*entity, entry.transform.clone()))
            .collect();
        let mut child_bindings = self.tick_frame(frame);
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
            for callback in entry.callbacks.clone() {
                match callback.kind {
                    StateMachineCallbackKind::OnStateEnter => {
                        script.on_state_enter(entity, &callback);
                    }
                    StateMachineCallbackKind::OnStateUpdate => {
                        script.on_state_update(entity, &callback);
                    }
                    StateMachineCallbackKind::OnStateExit => {
                        script.on_state_exit(entity, &callback);
                    }
                    StateMachineCallbackKind::OnStateMachineEnter => {
                        if let Some(machine) = callback.machine.as_deref() {
                            script.on_state_machine_enter(entity, machine);
                        }
                    }
                    StateMachineCallbackKind::OnStateMachineExit => {
                        if let Some(machine) = callback.machine.as_deref() {
                            script.on_state_machine_exit(entity, machine);
                        }
                    }
                    StateMachineCallbackKind::OnStateMove | StateMachineCallbackKind::OnStateIK => {
                    }
                }
            }
            let sampled_translation = entry.transform.translation;
            let sampled_rotation = entry.transform.rotation;
            let mut motion = entry.root_motion;
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
                if !entry.writes_transforms {
                    break;
                }
                ik_ran = true;
                populate_pose_from_skeleton(
                    &mut pose,
                    &entry.transform,
                    &skeleton_locals,
                    &hook_entry.skeleton_chains,
                    hook_entry.look_at_bone_path.as_deref(),
                );
                entry.callbacks.push(StateMachineCallback {
                    kind: StateMachineCallbackKind::OnStateIK,
                    state_index: current_state_index(entry),
                    state_name: String::new(),
                    machine: None,
                });
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
            let ik_override_translation = if entry.transform.translation != sampled_translation {
                Some(entry.transform.translation)
            } else {
                None
            };
            let ik_override_rotation = if entry.transform.rotation != sampled_rotation {
                Some(entry.transform.rotation)
            } else {
                None
            };
            entry.callbacks.push(StateMachineCallback {
                kind: StateMachineCallbackKind::OnStateMove,
                state_index: current_state_index(entry),
                state_name: String::new(),
                machine: None,
            });
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

fn blend_vector(a: [f32; 3], b: [f32; 3], weight: f32) -> [f32; 3] {
    match AnimatedValue::Vector3(a).blend(&AnimatedValue::Vector3(b), weight) {
        AnimatedValue::Vector3(value) => value,
        _ => b,
    }
}

fn blend_quaternion(a: [f32; 4], b: [f32; 4], weight: f32) -> [f32; 4] {
    match AnimatedValue::Quaternion(a).blend(&AnimatedValue::Quaternion(b), weight) {
        AnimatedValue::Quaternion(value) => value,
        _ => b,
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

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_animation::curve::{ObjectReferenceCurve, ObjectReferenceKey};
    use unitas_animation::{AnimatorState, CurveBinding, PropertyCurve, StateMotion};

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

    #[test]
    fn ani_001_animation_world_resolves_sprite_swap_and_records_missing_sprite_diagnostic()
    -> Result<(), Box<dyn std::error::Error>> {
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
        std::fs::write(
            dir.join("a.tga.meta"),
            format!("fileFormatVersion: 2\nguid: {guid_a}\nTextureImporter:\n  sRGBTexture: 0\n"),
        )?;
        std::fs::write(
            dir.join("b.tga.meta"),
            format!("fileFormatVersion: 2\nguid: {guid_b}\nTextureImporter:\n  sRGBTexture: 0\n"),
        )?;

        let ref_a = AssetReference::from_hex(guid_a, 21300000).ok_or("invalid guid_a")?;
        let ref_b = AssetReference::from_hex(guid_b, 21300000).ok_or("invalid guid_b")?;
        let ref_missing =
            AssetReference::from_hex(missing_guid, 21300099).ok_or("invalid missing_guid")?;
        let target = CurveBindingTarget::new("", "SpriteRenderer", "m_Sprite");
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
        let controller = AnimatorController::new(
            "SwapCtrl",
            Vec::new(),
            vec![AnimatorState::new("Swap", StateMotion::Clip(clip))],
            0,
            Vec::new(),
        );

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
}
