use unitas_asset::scene::UiBinding;
use unitas_core::{EntityId, IDENTITY_ROTATION, SceneGraph, multiply_rotation, normalize_rotation};
use unitas_physics::{
    BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, PhysicsError, Shape,
};

use crate::error::RuntimeError;
use crate::runtime::ui_components::UiComponents;
use crate::runtime::{LoadedScene, Runtime, ScriptInstance, world_to_local_translation};

impl Runtime {
    pub fn create_runtime_object(&mut self, name: &str) -> Result<EntityId, RuntimeError> {
        if self.active_scene.is_none() && self.scenes.is_empty() {
            let entity_base = self.next_entity_base();
            let mut graph = SceneGraph::new();
            graph.shift_entity_ids(entity_base);
            let mut ui = UiBinding::default();
            ui.shift_entity_ids(entity_base);
            self.scenes.push(LoadedScene {
                name: String::from("runtime"),
                graph,
                scripts: Vec::new(),
                bodies: Vec::new(),
                body_offsets: Vec::new(),
                body_colliders: Vec::new(),
                character_controllers: Vec::new(),
                audio_sources: Vec::new(),
                animators: Vec::new(),
                particles: Vec::new(),
                trails: Vec::new(),
                ui: UiComponents::new(ui),
                diagnostics: Vec::new(),
                entity_base,
                entity_count: 0,
                rigid_bodies: std::collections::BTreeSet::new(),
            });
            self.active_scene = Some(0);
        }
        let id_value = self.next_entity_base();
        let id = EntityId::new(id_value);
        self.create_runtime_object_with_id(id, name)
    }

    pub fn create_runtime_object_with_id(
        &mut self,
        id: EntityId,
        name: &str,
    ) -> Result<EntityId, RuntimeError> {
        let Some(active) = self.active_scene else {
            return Err(RuntimeError::InvalidSceneIndex(0));
        };
        let id_value = id.as_u64();
        for scene in &mut self.scenes {
            if scene.graph.contains(id) {
                return Err(RuntimeError::DuplicateEntityId(id));
            }
            scene.graph.reserve_entity_id(id_value);
        }
        let Some(scene) = self.scenes.get_mut(active) else {
            return Err(RuntimeError::InvalidSceneIndex(active));
        };
        if scene.graph.add_entity_with_id(id, name).is_none() {
            return Err(RuntimeError::DuplicateEntityId(id));
        }
        let end = id_value.saturating_add(1);
        if end > scene.entity_base + scene.entity_count {
            scene.entity_count = end.saturating_sub(scene.entity_base);
        }
        self.dynamic_entity_floor = self.dynamic_entity_floor.max(end);
        Ok(id)
    }

    pub fn attach_runtime_script(
        &mut self,
        entity: EntityId,
        guid_hi: u64,
        guid_lo: u64,
    ) -> Result<(), RuntimeError> {
        self.attach_runtime_script_with_hook(entity, guid_hi, guid_lo, |_, f| f())
    }

    pub fn attach_runtime_script_with_hook<H>(
        &mut self,
        entity: EntityId,
        guid_hi: u64,
        guid_lo: u64,
        mut hook: H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if entity.as_u64() == 0 {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        let scene_index = self
            .scenes
            .iter()
            .position(|scene| scene.graph.contains(entity));
        let is_persistent = self.persistent_entities.contains(&entity);
        if scene_index.is_none() && !is_persistent {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        let mut script_handle = 0;
        if let Some(callbacks) = self.callbacks {
            hook(self, &mut || {
                script_handle = (callbacks.create_script)(entity.as_u64(), guid_hi, guid_lo);
            });
        }
        let instance = ScriptInstance {
            entity,
            guid: format!("{guid_hi:016x}{guid_lo:016x}"),
            guid_hi,
            guid_lo,
            script_handle,
            awake_called: false,
            on_enable_called: false,
            start_called: false,
        };
        if let Some(index) = scene_index {
            if let Some(scene) = self.scenes.get_mut(index) {
                scene.scripts.push(instance);
            }
        } else {
            self.persistent.push(instance);
        }
        self.refresh_script_view();
        Ok(())
    }

    pub fn set_runtime_object_transform(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
        rotation: [f64; 4],
        scale: [f64; 3],
    ) -> Result<(), RuntimeError> {
        if entity.as_u64() == 0
            || !position.iter().all(|v| v.is_finite())
            || !rotation.iter().all(|v| v.is_finite())
            || !scale.iter().all(|v| v.is_finite())
        {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        let mut found_offset: Option<[f32; 3]> = None;
        let mut updated = false;
        for scene in &mut self.scenes {
            if !scene.graph.contains(entity) {
                continue;
            }
            let world_translation = [position[0] as f32, position[1] as f32, position[2] as f32];
            let world_rotation = normalize_rotation([
                rotation[0] as f32,
                rotation[1] as f32,
                rotation[2] as f32,
                rotation[3] as f32,
            ]);
            let local_scale = [scale[0] as f32, scale[1] as f32, scale[2] as f32];
            let parent_world = scene
                .graph
                .parent(entity)
                .and_then(|parent| scene.graph.world_transform(parent));
            let (local_translation, local_rotation) = match parent_world {
                Some(ref parent) => {
                    let p_rot = normalize_rotation(parent.rotation);
                    let inv_p = [-p_rot[0], -p_rot[1], -p_rot[2], p_rot[3]];
                    (
                        world_to_local_translation(parent, world_translation),
                        normalize_rotation(multiply_rotation(inv_p, world_rotation)),
                    )
                }
                None => (world_translation, world_rotation),
            };
            if let Some(transform) = scene.graph.transform_mut(entity) {
                transform.translation = local_translation;
                transform.rotation = local_rotation;
                transform.scale = local_scale;
            }
            found_offset = scene
                .body_offsets
                .iter()
                .find(|(id, _)| *id == entity)
                .map(|(_, offset)| *offset);
            updated = true;
            break;
        }
        if !updated {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        if self.physics.world_3d().position(entity).is_some() {
            let offset = found_offset.unwrap_or([0.0; 3]);
            let body_pos = [
                position[0] + f64::from(offset[0]),
                position[1] + f64::from(offset[1]),
                position[2] + f64::from(offset[2]),
            ];
            let norm_rot = normalize_rotation([
                rotation[0] as f32,
                rotation[1] as f32,
                rotation[2] as f32,
                rotation[3] as f32,
            ]);
            let body_rot = [
                f64::from(norm_rot[0]),
                f64::from(norm_rot[1]),
                f64::from(norm_rot[2]),
                f64::from(norm_rot[3]),
            ];
            let _ = self
                .physics
                .world_3d_mut()
                .teleport_position(entity, body_pos);
            let _ = self
                .physics
                .world_3d_mut()
                .teleport_rotation(entity, body_rot);
        }
        Ok(())
    }

    pub fn create_runtime_body(
        &mut self,
        entity: EntityId,
        shape_kind: u32,
        radius: f64,
        half_extents: [f64; 3],
        height: f64,
        is_trigger: bool,
    ) -> Result<(), RuntimeError> {
        if entity.as_u64() == 0 {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        let shape = match shape_kind {
            0 => Shape::Sphere { radius },
            1 => Shape::Box { half_extents },
            2 => Shape::Capsule {
                radius,
                height,
                direction: 1,
            },
            _ => return Err(RuntimeError::Physics(PhysicsError::InvalidBody(entity))),
        };
        let mut world_tf = None;
        let mut scene_index = None;
        for (idx, scene) in self.scenes.iter().enumerate() {
            if let Some(tf) = scene.graph.world_transform(entity) {
                world_tf = Some(tf);
                scene_index = Some(idx);
                break;
            }
        }
        let Some(world_tf) = world_tf else {
            return Err(RuntimeError::EntityNotFound(entity));
        };
        let norm_rot = normalize_rotation(world_tf.rotation);
        let desc = BodyDesc {
            position: [
                f64::from(world_tf.translation[0]),
                f64::from(world_tf.translation[1]),
                f64::from(world_tf.translation[2]),
            ],
            rotation: [
                f64::from(norm_rot[0]),
                f64::from(norm_rot[1]),
                f64::from(norm_rot[2]),
                f64::from(norm_rot[3]),
            ],
            mass: 0.0,
            velocity: [0.0; 3],
            use_gravity: false,
            is_trigger,
            collider_shape: ColliderShape::Primitive(shape),
            layer: 1,
            role: BodyRole::Static,
            friction: 0.6,
            restitution: 0.0,
            collision_detection_mode: CollisionDetectionMode3D::Discrete,
        };
        self.physics.world_3d_mut().remove_body(entity);
        self.physics
            .world_3d_mut()
            .add_body_with_default_material(entity, desc)?;
        if let Some(idx) = scene_index
            && let Some(scene) = self.scenes.get_mut(idx)
        {
            if !scene.bodies.contains(&entity) {
                scene.bodies.push(entity);
            }
            if let Some(entry) = scene.body_offsets.iter_mut().find(|(id, _)| *id == entity) {
                entry.1 = [0.0; 3];
            } else {
                scene.body_offsets.push((entity, [0.0; 3]));
            }
        }
        let _ = IDENTITY_ROTATION;
        Ok(())
    }
}
