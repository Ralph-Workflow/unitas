use std::path::{Path, PathBuf};
use unitas_asset::scene::{assemble_scene_file_with_physics, PhysicsBinding, ScriptBinding};
use unitas_asset::{
    parse_dynamics_manager, parse_input_manager, parse_player_settings, parse_time_manager,
    InputAxis, ProjectIdentity, DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY,
};
use unitas_core::{rotate_vector, EntityId, SceneGraph, IDENTITY_ROTATION};
use unitas_physics::{BodyDesc, Phase, QueryHit, Shape, World};

use crate::error::RuntimeError;
use crate::input::{default_axis_configs, AxisConfig, InputState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadSceneMode {
    Single,
    Additive,
}

pub struct LoadedScene {
    name: String,
    graph: SceneGraph,
    scripts: Vec<ScriptInstance>,
    bodies: Vec<EntityId>,
    entity_base: u64,
    entity_count: u64,
}

impl LoadedScene {
    pub fn scripts(&self) -> &[ScriptInstance] {
        &self.scripts
    }

    pub fn bodies(&self) -> &[EntityId] {
        &self.bodies
    }
}

#[derive(Clone, Copy)]
pub struct Callbacks {
    pub create_script: extern "C" fn(u64, u64, u64) -> u64,
    pub awake: extern "C" fn(u64),
    pub on_enable: extern "C" fn(u64),
    pub start: extern "C" fn(u64),
    pub fixed_update: extern "C" fn(u64, f64),
    pub update: extern "C" fn(u64, f64),
    pub late_update: extern "C" fn(u64, f64),
    pub on_disable: extern "C" fn(u64),
    pub destroy: extern "C" fn(u64),
    pub collision: extern "C" fn(u64, u64, u32),
    pub trigger: extern "C" fn(u64, u64, u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryRecord {
    pub entity: u64,
    pub distance: f64,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Debug, Clone)]
pub struct ScriptInstance {
    pub entity: EntityId,
    pub guid: String,
    pub guid_hi: u64,
    pub guid_lo: u64,
    pub script_handle: u64,
    pub awake_called: bool,
    pub on_enable_called: bool,
    pub start_called: bool,
}

/// The headless runtime. Physics lives in one global world: unloading a scene removes exactly that
/// scene's bodies, so cross-scene contacts can still occur (Unity isolates physics per scene).
pub struct Runtime {
    scenes: Vec<LoadedScene>,
    active_scene: Option<usize>,
    persistent: Vec<ScriptInstance>,
    persistent_entities: Vec<EntityId>,
    script_view: Vec<ScriptInstance>,
    world: World,
    input: InputState,
    project_identity: ProjectIdentity,
    callbacks: Option<Callbacks>,
    fixed_timestep: f64,
    time_scale: f64,
    accumulator: f64,
    first_tick: bool,
    is_shutdown: bool,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            scenes: Vec::new(),
            active_scene: None,
            persistent: Vec::new(),
            persistent_entities: Vec::new(),
            script_view: Vec::new(),
            world: World::new(),
            input: InputState::with_default_axes(),
            project_identity: ProjectIdentity::default(),
            callbacks: None,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
            time_scale: 1.0,
            accumulator: 0.0,
            first_tick: true,
            is_shutdown: false,
        })
    }

    /// Loads a scene in Single mode, replacing every loaded scene. Returns the number of scripts
    /// in the newly loaded scene.
    pub fn load_scene(&mut self, path: &Path) -> Result<usize, RuntimeError> {
        self.load_scene_with_hook(path, LoadSceneMode::Single, |_, f| f())?;
        Ok(self.scenes.last().map_or(0, |scene| scene.scripts.len()))
    }

    /// Loads a scene in the given mode. Returns the number of loaded scenes after the operation
    /// (not the script count, which [`Runtime::load_scene`] reports).
    pub fn load_scene_with_mode(
        &mut self,
        path: &Path,
        mode: LoadSceneMode,
    ) -> Result<usize, RuntimeError> {
        self.load_scene_with_hook(path, mode, |_, f| f())
    }

    pub fn load_scene_with_hook<H>(
        &mut self,
        path: &Path,
        mode: LoadSceneMode,
        mut hook: H,
    ) -> Result<usize, RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let (graph, bindings, physics) = assemble_scene_file_with_physics(path)?;
        let (fixed_timestep, gravity, axes, identity) = discover_project_settings(path);
        let scripts = build_script_instances(&bindings)?;
        let descs = body_descs(&graph, &physics);
        validate_bodies(&descs)?;
        match mode {
            LoadSceneMode::Single => {
                self.unload_all_scenes(&mut hook);
                self.apply_project_settings(fixed_timestep, gravity, axes, identity);
            }
            LoadSceneMode::Additive => {
                if self.scenes.is_empty() {
                    self.apply_project_settings(fixed_timestep, gravity, axes, identity);
                }
            }
        }
        let entity_base = self.next_entity_base();
        let mut graph = graph;
        graph.shift_entity_ids(entity_base);
        let scripts = scripts
            .into_iter()
            .map(|script| ScriptInstance {
                entity: EntityId::new(entity_base + script.entity.as_u64()),
                ..script
            })
            .collect::<Vec<_>>();
        let bodies = descs
            .iter()
            .map(|(entity, _)| EntityId::new(entity_base + entity.as_u64()))
            .collect::<Vec<_>>();
        for (body, desc) in bodies.iter().zip(descs.iter()) {
            // The atomicity guarantee ends here: the previous scene set is already torn down. The insert
            // can only fail on a duplicate entity id, and next_entity_base keeps every new id above every
            // live id, so the failure is unreachable in practice.
            self.world.add_body(*body, desc.1)?;
        }
        let entity_count = graph.len() as u64;
        self.scenes.push(LoadedScene {
            name: scene_name_from_path(path),
            graph,
            scripts,
            bodies,
            entity_base,
            entity_count,
        });
        if self.active_scene.is_none() {
            self.active_scene = Some(self.scenes.len() - 1);
        }
        self.accumulator = 0.0;
        self.first_tick = true;
        self.is_shutdown = false;
        self.refresh_script_view();
        Ok(self.scenes.len())
    }

    pub fn unload_scene(&mut self, name: &str) -> Result<(), RuntimeError> {
        self.unload_scene_with_hook(name, |_, f| f())
    }

    pub fn unload_scene_with_hook<H>(
        &mut self,
        name: &str,
        mut hook: H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let index = match self.scenes.iter().position(|scene| scene.name == name) {
            Some(index) => index,
            None => return Err(RuntimeError::SceneNotFound(name.to_string())),
        };
        let scene = self.scenes.remove(index);
        self.teardown_scene(&scene, &mut hook);
        for body in &scene.bodies {
            self.world.remove_body(*body);
        }
        match self.active_scene {
            Some(active) if active == index => {
                self.active_scene = if self.scenes.is_empty() {
                    None
                } else {
                    Some(index.min(self.scenes.len() - 1))
                };
            }
            Some(active) if active > index => self.active_scene = Some(active - 1),
            _ => {}
        }
        self.refresh_script_view();
        Ok(())
    }

    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    pub fn scene_name(&self, index: usize) -> Option<&str> {
        self.scenes.get(index).map(|scene| scene.name.as_str())
    }

    pub fn scene(&self, index: usize) -> Option<&LoadedScene> {
        self.scenes.get(index)
    }

    pub fn set_active_scene(&mut self, index: usize) -> Result<(), RuntimeError> {
        if index >= self.scenes.len() {
            return Err(RuntimeError::InvalidSceneIndex(index));
        }
        self.active_scene = Some(index);
        self.refresh_script_view();
        Ok(())
    }

    pub fn active_scene_index(&self) -> Option<usize> {
        self.active_scene
    }

    pub fn dont_destroy_on_load(&mut self, entity: EntityId) -> Result<(), RuntimeError> {
        if self.persistent_entities.contains(&entity) {
            return Ok(());
        }
        let mut found = false;
        for index in 0..self.scenes.len() {
            let script_index = self.scenes[index]
                .scripts
                .iter()
                .position(|script| script.entity == entity);
            if let Some(script_index) = script_index {
                let script = self.scenes[index].scripts.remove(script_index);
                self.persistent.push(script);
                found = true;
                break;
            }
        }
        for index in 0..self.scenes.len() {
            let body_index = self.scenes[index]
                .bodies
                .iter()
                .position(|body| *body == entity);
            if let Some(body_index) = body_index {
                self.scenes[index].bodies.remove(body_index);
                found = true;
                break;
            }
        }
        if !found {
            return Err(RuntimeError::EntityNotFound(entity));
        }
        self.persistent_entities.push(entity);
        self.refresh_script_view();
        Ok(())
    }

    pub fn is_persistent(&self, entity: EntityId) -> bool {
        self.persistent_entities.contains(&entity)
    }

    fn apply_project_settings(
        &mut self,
        fixed_timestep: f64,
        gravity: [f64; 3],
        axes: Vec<AxisConfig>,
        identity: ProjectIdentity,
    ) {
        self.world.set_gravity(gravity);
        self.fixed_timestep = fixed_timestep;
        self.input.set_axes(axes);
        self.input.reset();
        self.project_identity = identity;
    }

    pub fn project_identity(&self) -> &ProjectIdentity {
        &self.project_identity
    }

    pub fn company_name(&self) -> Option<&str> {
        self.project_identity.company_name.as_deref()
    }

    pub fn product_name(&self) -> Option<&str> {
        self.project_identity.product_name.as_deref()
    }

    pub fn project_version(&self) -> Option<&str> {
        self.project_identity.bundle_version.as_deref()
    }

    fn unload_all_scenes<H>(&mut self, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let scenes = std::mem::take(&mut self.scenes);
        self.active_scene = None;
        for scene in &scenes {
            self.teardown_scene(scene, hook);
        }
        for scene in &scenes {
            for body in &scene.bodies {
                self.world.remove_body(*body);
            }
        }
    }

    fn teardown_scene<H>(&mut self, scene: &LoadedScene, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let Some(callbacks) = self.callbacks else {
            return;
        };
        let (disable, destroy) = teardown_handles(&scene.scripts);
        self.fire_teardown(disable, destroy, callbacks, hook);
    }

    fn fire_teardown<H>(
        &mut self,
        disable: Vec<u64>,
        destroy: Vec<u64>,
        callbacks: Callbacks,
        hook: &mut H,
    ) where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        for handle in disable {
            hook(self, &mut || {
                (callbacks.on_disable)(handle);
            });
        }
        for handle in destroy {
            hook(self, &mut || {
                (callbacks.destroy)(handle);
            });
        }
    }

    fn next_entity_base(&self) -> u64 {
        // Entity id 0 is reserved as "no entity" for the managed bridge, so the first scene starts at 1.
        let mut highest = 1u64;
        for scene in &self.scenes {
            highest = highest.max(scene.entity_base + scene.entity_count);
        }
        for entity in &self.persistent_entities {
            highest = highest.max(entity.as_u64() + 1);
        }
        highest
    }

    fn refresh_script_view(&mut self) {
        let scene_scripts = match self.active_scene.and_then(|index| self.scenes.get(index)) {
            Some(scene) => scene.scripts.clone(),
            None => Vec::new(),
        };
        let persistent = self.persistent.clone();
        self.script_view = scene_scripts;
        self.script_view.extend(persistent);
    }

    fn all_script_handles(&self) -> Vec<u64> {
        let mut handles = Vec::new();
        for scene in &self.scenes {
            for script in &scene.scripts {
                handles.push(script.script_handle);
            }
        }
        for script in &self.persistent {
            handles.push(script.script_handle);
        }
        handles
    }

    fn find_script(&self, entity: EntityId) -> Option<&ScriptInstance> {
        for scene in &self.scenes {
            if let Some(script) = scene.scripts.iter().find(|script| script.entity == entity) {
                return Some(script);
            }
        }
        self.persistent.iter().find(|script| script.entity == entity)
    }

    fn find_script_mut(&mut self, entity: EntityId) -> Option<&mut ScriptInstance> {
        for scene in &mut self.scenes {
            if let Some(script) = scene.scripts.iter_mut().find(|script| script.entity == entity) {
                return Some(script);
            }
        }
        self.persistent.iter_mut().find(|script| script.entity == entity)
    }

    fn init_pending_scripts<H>(&mut self, callbacks: Callbacks, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let mut pending: Vec<(EntityId, u64, u64)> = Vec::new();
        for scene in &self.scenes {
            for script in &scene.scripts {
                if !script.awake_called {
                    pending.push((script.entity, script.guid_hi, script.guid_lo));
                }
            }
        }
        for script in &self.persistent {
            if !script.awake_called {
                pending.push((script.entity, script.guid_hi, script.guid_lo));
            }
        }
        // Pending scripts are tracked by entity, not by slot index: a callback may mark a script
        // persistent and move it out of its scene while that script is being initialised.
        for (entity, guid_hi, guid_lo) in pending.iter().copied() {
            let mut created = 0;
            hook(self, &mut || {
                created = (callbacks.create_script)(entity.as_u64(), guid_hi, guid_lo);
            });
            if let Some(script) = self.find_script_mut(entity) {
                script.script_handle = created;
            }
        }
        for (entity, _, _) in pending.iter().copied() {
            let Some(handle) = self.find_script(entity).map(|script| script.script_handle) else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.awake)(handle);
            });
            if let Some(script) = self.find_script_mut(entity) {
                script.awake_called = true;
            }
        }
        for (entity, _, _) in pending.iter().copied() {
            let Some(handle) = self.find_script(entity).map(|script| script.script_handle) else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.on_enable)(handle);
            });
            if let Some(script) = self.find_script_mut(entity) {
                script.on_enable_called = true;
            }
        }
        for (entity, _, _) in pending.iter().copied() {
            let Some(handle) = self.find_script(entity).map(|script| script.script_handle) else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.start)(handle);
            });
            if let Some(script) = self.find_script_mut(entity) {
                script.start_called = true;
            }
        }
    }

    pub fn set_callbacks(&mut self, callbacks: Callbacks) {
        self.callbacks = Some(callbacks);
    }

    pub fn tick(&mut self, delta_seconds: f64) -> Result<(), RuntimeError> {
        self.tick_with_hook(delta_seconds, |_, f| f())
    }

    pub fn tick_with_hook<H>(&mut self, delta_seconds: f64, mut hook: H) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.is_shutdown {
            return Err(RuntimeError::AlreadyShutdown);
        }
        if self.first_tick {
            self.first_tick = false;
            if let Some(callbacks) = self.callbacks {
                self.init_pending_scripts(callbacks, &mut hook);
            }
            self.refresh_script_view();
            return Ok(());
        }
        let scaled_delta = delta_seconds * self.time_scale;
        self.input.begin_frame(scaled_delta);
        self.accumulator += scaled_delta;
        let fixed_step = self.fixed_timestep;
        const EPSILON: f64 = 1e-9;
        while self.accumulator >= fixed_step - EPSILON {
            self.world.step(fixed_step)?;
            if let Some(callbacks) = self.callbacks {
                let handles = self.all_script_handles();
                for handle in handles {
                    hook(self, &mut || {
                        (callbacks.fixed_update)(handle, fixed_step);
                    });
                }
            }
            self.accumulator -= fixed_step;
            if self.accumulator < 0.0 {
                self.accumulator = 0.0;
            }
        }
        let events = self.world.drain_events();
        let trigger_events = self.world.drain_trigger_events();
        if let Some(callbacks) = self.callbacks {
            for event in events {
                let a = event.a.as_u64();
                let b = event.b.as_u64();
                let phase = phase_code(event.phase);
                hook(self, &mut || {
                    (callbacks.collision)(a, b, phase);
                });
            }
            for event in trigger_events {
                let a = event.a.as_u64();
                let b = event.b.as_u64();
                let phase = phase_code(event.phase);
                hook(self, &mut || {
                    (callbacks.trigger)(a, b, phase);
                });
            }
            let handles = self.all_script_handles();
            for handle in &handles {
                let h = *handle;
                hook(self, &mut || {
                    (callbacks.update)(h, scaled_delta);
                });
            }
            for handle in &handles {
                let h = *handle;
                hook(self, &mut || {
                    (callbacks.late_update)(h, scaled_delta);
                });
            }
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), RuntimeError> {
        self.shutdown_with_hook(|_, f| f())
    }

    pub fn shutdown_with_hook<H>(&mut self, mut hook: H) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.is_shutdown {
            return Ok(());
        }
        self.is_shutdown = true;
        if let Some(callbacks) = self.callbacks {
            for index in (0..self.scenes.len()).rev() {
                let (disable, destroy) = teardown_handles(&self.scenes[index].scripts);
                self.fire_teardown(disable, destroy, callbacks, &mut hook);
            }
            let (disable, destroy) = teardown_handles(&self.persistent);
            self.fire_teardown(disable, destroy, callbacks, &mut hook);
        }
        Ok(())
    }

    /// The scripts of the active scene, followed by the scripts that are marked to survive scene
    /// changes. Scripts of other loaded scenes are not reported here.
    pub fn scripts(&self) -> &[ScriptInstance] {
        &self.script_view
    }

    pub fn scene_graph(&self) -> Option<&SceneGraph> {
        self.active_scene
            .and_then(|index| self.scenes.get(index))
            .map(|scene| &scene.graph)
    }

    pub fn raycast(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        self.records_from_hits(self.world.raycast_with_triggers(
            origin,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        ))
    }

    pub fn linecast(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        let Some(direction) = normalized(direction) else {
            return Vec::new();
        };
        if max_distance.is_nan() || max_distance <= 0.0 {
            return Vec::new();
        }
        let end = [
            origin[0] + direction[0] * max_distance,
            origin[1] + direction[1] * max_distance,
            origin[2] + direction[2] * max_distance,
        ];
        self.records_from_hits(
            self.world
                .linecast(origin, end, layer_mask, include_triggers),
        )
    }

    pub fn sphere_cast(
        &self,
        origin: [f64; 3],
        radius: f64,
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        self.records_from_hits(self.world.sphere_cast(
            origin,
            radius,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        ))
    }

    pub fn box_cast(
        &self,
        origin: [f64; 3],
        half_extents: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        self.records_from_hits(self.world.box_cast(
            origin,
            half_extents,
            direction,
            max_distance,
            layer_mask,
            include_triggers,
        ))
    }

    pub fn overlap_sphere(
        &self,
        center: [f64; 3],
        radius: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        let mut records: Vec<QueryRecord> = self
            .world
            .overlap_sphere_with_triggers(center, radius, layer_mask, include_triggers)
            .into_iter()
            .map(|entity| QueryRecord {
                entity: entity.as_u64(),
                distance: 0.0,
                point: self.world.position(entity).unwrap_or(center),
                normal: [0.0, 0.0, 0.0],
            })
            .collect();
        records.sort_by(|first, second| first.entity.cmp(&second.entity));
        records
    }

    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    pub fn fixed_timestep(&self) -> f64 {
        self.fixed_timestep
    }

    pub fn time_scale(&self) -> f64 {
        self.time_scale
    }

    pub fn set_time_scale(&mut self, time_scale: f64) {
        self.time_scale = time_scale;
    }

    pub fn body_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.world.position(entity)
    }

    pub fn body_velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.world.velocity(entity)
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    pub fn input_key_event(&mut self, key: u32, pressed: bool) {
        self.input.queue_key_event(key, pressed);
    }

    pub fn input_mouse_button_event(&mut self, button: u32, pressed: bool) {
        self.input.queue_mouse_button_event(button, pressed);
    }

    pub fn input_pointer_move(&mut self, x: f64, y: f64) {
        self.input.queue_pointer_move(x, y);
    }

    pub fn input_scroll(&mut self, x: f64, y: f64) {
        self.input.queue_scroll(x, y);
    }

    pub fn input_key_state(&self, key: u32) -> u32 {
        self.input.key_state(key)
    }

    pub fn input_mouse_state(&self, button: u32) -> u32 {
        self.input.mouse_button_state(button)
    }

    pub fn input_pointer(&self) -> (f64, f64, f64, f64) {
        self.input.pointer()
    }

    pub fn input_scroll_state(&self) -> (f64, f64) {
        self.input.scroll()
    }

    pub fn input_axis(&self, name: &str) -> Option<f64> {
        self.input.axis(name)
    }

    pub fn input_axis_raw(&self, name: &str) -> Option<f64> {
        self.input.axis_raw(name)
    }

    pub fn input_button_state(&self, name: &str) -> Option<u32> {
        self.input.button_state(name)
    }

    fn records_from_hits(&self, hits: Vec<QueryHit>) -> Vec<QueryRecord> {
        let mut records: Vec<QueryRecord> = hits
            .into_iter()
            .map(|hit| QueryRecord {
                entity: hit.entity.as_u64(),
                distance: hit.distance,
                point: hit.point,
                normal: hit.normal,
            })
            .collect();
        records.sort_by(|first, second| {
            first
                .distance
                .total_cmp(&second.distance)
                .then(first.entity.cmp(&second.entity))
        });
        records
    }
}

fn discover_project_settings(
    scene_path: &Path,
) -> (f64, [f64; 3], Vec<AxisConfig>, ProjectIdentity) {
    let Some(settings_dir) = find_project_settings_dir(scene_path) else {
        return (
            DEFAULT_FIXED_TIMESTEP,
            DEFAULT_GRAVITY,
            default_axis_configs(),
            ProjectIdentity::default(),
        );
    };
    let fixed_timestep = match parse_time_manager(&settings_dir.join("TimeManager.asset")) {
        Ok(Some(settings)) => settings.fixed_timestep,
        Ok(None) | Err(_) => DEFAULT_FIXED_TIMESTEP,
    };
    let gravity = match parse_dynamics_manager(&settings_dir.join("DynamicsManager.asset")) {
        Ok(Some(settings)) => settings.gravity,
        Ok(None) | Err(_) => DEFAULT_GRAVITY,
    };
    let axes = match parse_input_manager(&settings_dir.join("InputManager.asset")) {
        Ok(Some(settings)) => map_input_axes(settings.axes),
        Ok(None) | Err(_) => default_axis_configs(),
    };
    let identity = match parse_player_settings(&settings_dir.join("ProjectSettings.asset")) {
        Ok(Some(settings)) => settings,
        Ok(None) | Err(_) => ProjectIdentity::default(),
    };
    (fixed_timestep, gravity, axes, identity)
}

fn map_input_axes(axes: Vec<InputAxis>) -> Vec<AxisConfig> {
    axes.into_iter()
        .map(|axis| AxisConfig {
            name: axis.name,
            negative_button: axis.negative_button,
            positive_button: axis.positive_button,
            alt_negative_button: axis.alt_negative_button,
            alt_positive_button: axis.alt_positive_button,
            gravity: axis.gravity,
            dead: axis.dead,
            sensitivity: axis.sensitivity,
            snap: axis.snap,
            invert: axis.invert,
            axis_type: axis.axis_type,
            axis: axis.axis,
        })
        .collect()
}

fn find_project_settings_dir(scene_path: &Path) -> Option<PathBuf> {
    let parent = scene_path.parent()?;
    for ancestor in parent.ancestors() {
        let candidate = ancestor.join("ProjectSettings");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn build_script_instances(
    bindings: &[ScriptBinding],
) -> Result<Vec<ScriptInstance>, RuntimeError> {
    let mut scripts = Vec::new();
    for binding in bindings {
        if binding.guid.len() != 32 {
            return Err(RuntimeError::InvalidGuid(binding.guid.clone()));
        }
        let guid_hi = match u64::from_str_radix(&binding.guid[0..16], 16) {
            Ok(val) => val,
            Err(_) => return Err(RuntimeError::InvalidGuid(binding.guid.clone())),
        };
        let guid_lo = match u64::from_str_radix(&binding.guid[16..32], 16) {
            Ok(val) => val,
            Err(_) => return Err(RuntimeError::InvalidGuid(binding.guid.clone())),
        };
        scripts.push(ScriptInstance {
            entity: binding.entity,
            guid: binding.guid.clone(),
            guid_hi,
            guid_lo,
            script_handle: 0,
            awake_called: false,
            on_enable_called: false,
            start_called: false,
        });
    }
    Ok(scripts)
}

fn body_descs(graph: &SceneGraph, bindings: &[PhysicsBinding]) -> Vec<(EntityId, BodyDesc)> {
    let mut descs = Vec::new();
    for binding in bindings {
        let desc = match body_desc(graph, binding) {
            Some(desc) => desc,
            None => continue,
        };
        descs.push((binding.entity, desc));
    }
    descs
}

fn validate_bodies(descs: &[(EntityId, BodyDesc)]) -> Result<(), RuntimeError> {
    let mut world = World::new();
    for (entity, desc) in descs {
        world.add_body(*entity, *desc)?;
    }
    Ok(())
}

fn teardown_handles(scripts: &[ScriptInstance]) -> (Vec<u64>, Vec<u64>) {
    let disable = scripts
        .iter()
        .rev()
        .filter(|s| s.on_enable_called)
        .map(|s| s.script_handle)
        .collect();
    let destroy = scripts
        .iter()
        .rev()
        .filter(|s| s.script_handle != 0)
        .map(|s| s.script_handle)
        .collect();
    (disable, destroy)
}

fn scene_name_from_path(path: &Path) -> String {
    match path.file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) => stem.to_string(),
        None => String::from("scene"),
    }
}

fn body_desc(graph: &SceneGraph, binding: &PhysicsBinding) -> Option<BodyDesc> {
    let (shape, center, collider_trigger) = match (&binding.sphere_collider, &binding.box_collider)
    {
        (Some(sphere), _) => (
            Shape::Sphere {
                radius: sphere.radius,
            },
            sphere.center,
            sphere.is_trigger,
        ),
        (None, Some(collider)) => (
            Shape::Box {
                half_extents: [
                    collider.size[0] / 2.0,
                    collider.size[1] / 2.0,
                    collider.size[2] / 2.0,
                ],
            },
            collider.center,
            collider.is_trigger,
        ),
        (None, None) => return None,
    };
    let (translation, rotation, scale) = match graph.world_transform(binding.entity) {
        Some(world) => (world.translation, world.rotation, world.scale),
        None => ([0.0f32; 3], IDENTITY_ROTATION, [1.0f32; 3]),
    };
    let scaled_center = [
        center[0] as f32 * scale[0],
        center[1] as f32 * scale[1],
        center[2] as f32 * scale[2],
    ];
    let offset = rotate_vector(rotation, scaled_center);
    let (mass, use_gravity, rigidbody_trigger) = match &binding.rigidbody {
        Some(rigidbody) => (rigidbody.mass, rigidbody.use_gravity, rigidbody.is_trigger),
        None => (0.0, false, false),
    };
    Some(BodyDesc {
        position: [
            translation[0] as f64 + offset[0] as f64,
            translation[1] as f64 + offset[1] as f64,
            translation[2] as f64 + offset[2] as f64,
        ],
        rotation: [
            rotation[0] as f64,
            rotation[1] as f64,
            rotation[2] as f64,
            rotation[3] as f64,
        ],
        mass,
        velocity: [0.0; 3],
        use_gravity,
        is_trigger: collider_trigger || rigidbody_trigger,
        shape,
        layer: layer_bits(binding.layer),
    })
}

fn layer_bits(layer: u32) -> u32 {
    if layer >= 32 {
        return layer;
    }
    1 << layer
}

fn normalized(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length =
        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([
        vector[0] / length,
        vector[1] / length,
        vector[2] / length,
    ])
}

fn phase_code(phase: Phase) -> u32 {
    match phase {
        Phase::Enter => 0,
        Phase::Stay => 1,
        Phase::Exit => 2,
    }
}
