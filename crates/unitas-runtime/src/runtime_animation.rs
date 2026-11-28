use std::path::{Path, PathBuf};

use unitas_animation::curve::PropertyCurve;
use unitas_animation::{
    AnimationClip, AnimatorController, AnimatorCullingMode, AnimatorUpdateMode, SampledBinding,
    StateMotion,
};
use unitas_asset::anim_import::import_animator_controller;
use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::AnimatorBinding;
use unitas_core::{EntityId, SceneGraph, Transform};

use super::Runtime;
use crate::animation::{AnimationRuntimeDiagnostic, AnimationWorld, apply_transform_property};

impl Runtime {
    fn bind_scene_animator_controller(
        &mut self,
        entity: EntityId,
        binding: &AnimatorBinding,
        index: Option<&GuidIndex>,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) {
        let Some(guid) = binding.controller_guid.as_deref() else {
            return;
        };
        let Some(index) = index else {
            diagnostics.push((entity, unresolved_controller(entity, guid, None::<&str>)));
            return;
        };
        let Some(controller_path) = resolve_guid_path(index, guid) else {
            diagnostics.push((entity, unresolved_controller(entity, guid, None::<&str>)));
            return;
        };
        let controller = match import_animator_controller(&controller_path, index) {
            Ok(controller) => controller,
            Err(error) => {
                diagnostics.push((entity, unresolved_controller(entity, guid, Some(error))));
                return;
            }
        };
        register_controller_sprite_assets(&mut self.animation, &controller, index);
        let handle = self.animation.register_controller(controller);
        if let Err(error) = self.animation.set_controller(entity, handle) {
            diagnostics.push((
                entity,
                format!(
                    "Animator on entity {} could not bind its controller: {error}",
                    entity.as_u64()
                ),
            ));
            return;
        }
        if !binding.enabled {
            let _ = self.animation.disable_animator(entity);
        }
    }

    pub(crate) fn register_scene_animators(
        &mut self,
        bindings: &[AnimatorBinding],
        entity_base: u64,
        graph: &SceneGraph,
        scene_path: &Path,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) -> Vec<EntityId> {
        let mut animator_entities = Vec::with_capacity(bindings.len());
        if bindings.is_empty() {
            return animator_entities;
        }
        let index = project_guid_index(scene_path);
        for binding in bindings {
            let entity = EntityId::new(entity_base + binding.entity.as_u64());
            let transform = graph.transform(entity).cloned().unwrap_or_default();
            self.animation.ensure_animator(entity, transform);
            if let Some(mode) = animator_update_mode(binding.update_mode) {
                let _ = self.animation.set_update_mode(entity, mode);
            }
            if let Some(mode) = animator_culling_mode(binding.culling_mode) {
                let _ = self.animation.set_culling_mode(entity, mode);
            }
            if !binding.enabled {
                let _ = self.animation.disable_animator(entity);
            }
            animator_entities.push(entity);
            self.bind_scene_animator_controller(entity, binding, index.as_ref(), diagnostics);
        }
        animator_entities
    }

    pub fn ensure_animator(&mut self, entity: u64) -> EntityId {
        let id = if entity == 0 {
            let allocated = self.next_entity_base();
            self.dynamic_entity_floor = allocated + 1;
            EntityId::new(allocated)
        } else {
            if entity >= self.dynamic_entity_floor {
                self.dynamic_entity_floor = entity + 1;
            }
            EntityId::new(entity)
        };
        let mut initial_transform = Transform::default();
        for scene in &mut self.scenes {
            if let Some(transform) = scene.graph.transform(id) {
                initial_transform = transform.clone();
                if !scene.animators.contains(&id) {
                    scene.animators.push(id);
                }
                break;
            }
        }
        if let Some(existing) = self.animation.transform(id).cloned() {
            initial_transform = existing;
        }
        self.animation.ensure_animator(id, initial_transform);
        id
    }

    pub fn remove_animator(&mut self, entity: EntityId) -> bool {
        for scene in &mut self.scenes {
            scene.animators.retain(|id| *id != entity);
        }
        self.persistent_animators.retain(|id| *id != entity);
        self.animation.remove_animator(entity)
    }

    pub fn set_animator_controller(
        &mut self,
        entity: EntityId,
        controller_handle: u64,
    ) -> Result<(), String> {
        let child_bindings = self.animation.set_controller(entity, controller_handle)?;
        self.write_back_animator_transform(entity);
        self.apply_child_animator_bindings(entity, &child_bindings);
        Ok(())
    }

    pub fn animation(&self) -> &AnimationWorld {
        &self.animation
    }

    pub fn animation_mut(&mut self) -> &mut AnimationWorld {
        &mut self.animation
    }

    pub(crate) fn step_animators(&mut self, scaled_delta: f64) {
        for scene in &self.scenes {
            for entity in &scene.animators {
                if let Some(graph_transform) = scene.graph.transform(*entity)
                    && let Some(entry) = self.animation.entry_mut(*entity)
                {
                    entry.transform.parent = graph_transform.parent;
                    entry.transform.children = graph_transform.children.clone();
                }
            }
        }
        let child_updates = self.animation.tick_parallel(scaled_delta, &self.job_system);
        for entity in self.animation.take_pending_transform_writes() {
            if !self.write_back_animator_transform(entity) {
                self.animation.queue_transform_write(entity);
            }
        }
        for (entity, child_bindings) in child_updates {
            self.apply_child_animator_bindings(entity, &child_bindings);
        }
    }

    pub(crate) fn write_back_animator_transform(&mut self, entity: EntityId) -> bool {
        let Some(anim_transform) = self.animation.transform(entity) else {
            return true;
        };
        for scene in &mut self.scenes {
            if let Some(graph_transform) = scene.graph.transform_mut(entity) {
                graph_transform.translation = anim_transform.translation;
                graph_transform.rotation = anim_transform.rotation;
                graph_transform.scale = anim_transform.scale;
                return true;
            }
        }
        false
    }

    pub(crate) fn apply_child_animator_bindings(
        &mut self,
        entity: EntityId,
        child_bindings: &[SampledBinding],
    ) {
        for binding in child_bindings {
            let Some((prop, ch)) = binding.target.parse_transform_property() else {
                continue;
            };
            let mut applied = false;
            for scene in &mut self.scenes {
                if !scene.graph.contains(entity) {
                    continue;
                }
                if let Some(child_id) =
                    resolve_child_path_in_graph(&scene.graph, entity, &binding.target.object_path)
                    && let Some(child_transform) = scene.graph.transform_mut(child_id)
                {
                    apply_transform_property(child_transform, prop, ch, binding.value);
                    applied = true;
                }
                break;
            }
            if !applied {
                self.animation.record_diagnostic(
                    entity,
                    AnimationRuntimeDiagnostic::UnresolvedChildPath {
                        entity,
                        object_path: binding.target.object_path.clone(),
                    },
                );
            }
        }
    }
}

fn register_controller_sprite_assets(
    animation: &mut AnimationWorld,
    controller: &AnimatorController,
    index: &GuidIndex,
) {
    for state in &controller.states {
        match &state.motion {
            StateMotion::None => {}
            StateMotion::Clip(clip) => register_clip_sprite_assets(animation, clip, index),
            StateMotion::BlendTree1D { thresholds, .. } => {
                for (_, clip) in thresholds {
                    register_clip_sprite_assets(animation, clip, index);
                }
            }
            StateMotion::BlendTree2D { children, .. } => {
                for (_, clip) in children {
                    register_clip_sprite_assets(animation, clip, index);
                }
            }
            StateMotion::BlendTreeDirect { children } => {
                for (_, clip) in children {
                    register_clip_sprite_assets(animation, clip, index);
                }
            }
        }
    }
}

fn register_clip_sprite_assets(
    animation: &mut AnimationWorld,
    clip: &AnimationClip,
    index: &GuidIndex,
) {
    for binding in &clip.bindings {
        if let PropertyCurve::ObjectReference(curve) = &binding.curve {
            for key in curve.keyframes() {
                if key.reference.is_null() {
                    continue;
                }
                let guid = key.reference.guid_hex();
                if let Some(path) = resolve_guid_path(index, &guid) {
                    animation.register_sprite_asset(&guid, &path);
                }
            }
        }
    }
}

fn project_guid_index(scene_path: &Path) -> Option<GuidIndex> {
    let parent = scene_path.parent()?;
    let mut index_root = parent.to_path_buf();
    for ancestor in parent.ancestors() {
        if ancestor.join("ProjectSettings").is_dir() {
            let assets = ancestor.join("Assets");
            index_root = if assets.is_dir() {
                assets
            } else {
                ancestor.to_path_buf()
            };
            break;
        }
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("Assets") {
            index_root = ancestor.to_path_buf();
            break;
        }
    }
    GuidIndex::build(&index_root).ok()
}

fn resolve_guid_path(index: &GuidIndex, guid: &str) -> Option<PathBuf> {
    let trimmed = guid.trim();
    index
        .get(trimmed)
        .or_else(|| index.get(&trimmed.to_ascii_lowercase()))
        .map(Path::to_path_buf)
}

fn unresolved_controller(
    entity: EntityId,
    guid: &str,
    error: Option<impl std::fmt::Display>,
) -> String {
    match error {
        Some(error) => format!(
            "Animator on entity {} references controller guid {} that could not be imported: {error}",
            entity.as_u64(),
            guid
        ),
        None => format!(
            "Animator on entity {} references controller guid {} that could not be imported",
            entity.as_u64(),
            guid
        ),
    }
}

fn animator_update_mode(value: i32) -> Option<AnimatorUpdateMode> {
    match value {
        0 => Some(AnimatorUpdateMode::Normal),
        1 => Some(AnimatorUpdateMode::AnimatePhysics),
        2 => Some(AnimatorUpdateMode::UnscaledTime),
        _ => None,
    }
}

fn animator_culling_mode(value: i32) -> Option<AnimatorCullingMode> {
    match value {
        0 => Some(AnimatorCullingMode::AlwaysAnimate),
        1 => Some(AnimatorCullingMode::CullUpdateTransforms),
        2 => Some(AnimatorCullingMode::CullCompletely),
        _ => None,
    }
}

fn resolve_child_path_in_graph(
    graph: &SceneGraph,
    root: EntityId,
    object_path: &str,
) -> Option<EntityId> {
    let mut current = root;
    for segment in object_path.split('/') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut next = None;
        for child_id in graph.children(current) {
            if let Some(entity) = graph.entity(*child_id)
                && entity.name == trimmed
            {
                next = Some(*child_id);
                break;
            }
        }
        current = next?;
    }
    if current == root { None } else { Some(current) }
}
