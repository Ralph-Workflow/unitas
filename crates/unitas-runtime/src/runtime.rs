use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use unitas_animation::SampledBinding;
use unitas_asset::particle::ParticleSystemBinding;
use unitas_asset::project_settings::{
    Physics2DProjectSettings, PhysicsProjectSettings, QualityLevel, QualitySettings,
    parse_physics_manager, parse_physics2d_settings, parse_quality_settings,
};
use unitas_asset::scene::{
    AudioSourceComponent, CharacterControllerComponent, ParticleBinding, ScriptBinding,
    TrailBinding, assemble_scene_file_bundle,
};
use unitas_asset::trail::TrailRendererBinding;
use unitas_asset::{
    DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY, InputAxis, ProjectIdentity, parse_dynamics_manager,
    parse_input_manager, parse_player_settings, parse_time_manager,
};
use unitas_audio::{AudioSourceState, AudioWorld};
use unitas_core::{
    EntityId, SceneGraph, Transform, multiply_rotation, normalize_rotation, rotate_vector,
};
use unitas_physics::navigation::NavMesh;
use unitas_physics::particles::ParticleSystem;
use unitas_physics::trails::Trail;
use unitas_physics::{
    BodyRole, CharacterController, ColliderDesc, CollisionEvent, CollisionPayload, ForceMode,
    MoveResult, QueryHit, SolverType, TriggerEvent,
};
use unitas_render::Renderer;
use unitas_render::ui_pass::GlyphTextPainter;

use self::runtime_physics::{
    body_descs, collect_character_controllers, collect_material_diagnostics,
    collect_scene_rigidbodies, contact_phase_code, controller_from_component, normalized,
    ordered_pair, rotations_equivalent, shift_scene_bodies, validate_bodies,
};
use self::ui_components::UiComponents;
use crate::animation::{AnimationRuntimeDiagnostic, AnimationWorld, apply_transform_property};
use crate::error::RuntimeError;
use crate::input::{AxisConfig, InputState, default_axis_configs};
use crate::navigation::NavigationRuntimeWorld;
use crate::particles::{ParticleDrawItem, ParticleRuntimeWorld, ParticleRuntimeWorld2D};
use crate::physics::{CollisionPayload2D, ContactEvent, PhysicsRuntimeWorld};
use crate::physics_settings::{
    apply_physics2d_project_settings, layer_matrix_of, solver_settings_of,
};
use crate::player_loop::{
    BuiltinWork, CustomSystemPosition, PlayerLoop, PlayerLoopCallback, PlayerLoopError,
    PlayerLoopStep, PlayerLoopSystem,
};
use crate::terrain::TerrainWorld;
use crate::trails::{TrailDrawItem, TrailRuntimeWorld};

#[path = "ui_components.rs"]
pub(crate) mod ui_components;

#[path = "runtime_input.rs"]
mod runtime_input;
#[path = "runtime_physics.rs"]
mod runtime_physics;
#[path = "runtime_render.rs"]
mod runtime_render;
#[path = "runtime_ui.rs"]
mod runtime_ui;

pub use runtime_physics::*;
pub use runtime_render::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadSceneMode {
    Single,
    Additive,
}

type DestroyChannel = Arc<Mutex<Vec<EntityId>>>;

static DESTROY_CHANNELS: OnceLock<Mutex<HashMap<u64, DestroyChannel>>> = OnceLock::new();

thread_local! {
    static BOUND_DESTROY_CHANNEL: RefCell<Option<(u64, DestroyChannel)>> = const { RefCell::new(None) };
}

fn destroy_channels() -> &'static Mutex<HashMap<u64, DestroyChannel>> {
    DESTROY_CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[unsafe(export_name = "unitas_runtime_bind_destroy_channel")]
pub extern "C" fn unitas_runtime_bind_destroy_channel(handle: u64) -> i32 {
    #[used]
    static KEEP: extern "C" fn(u64) -> i32 = unitas_runtime_bind_destroy_channel;
    let channel = {
        let Ok(mut channels) = destroy_channels().lock() else {
            return -1;
        };
        channels
            .entry(handle)
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
            .clone()
    };
    BOUND_DESTROY_CHANNEL.with(|cell| {
        *cell.borrow_mut() = Some((handle, channel));
    });
    0
}

#[unsafe(export_name = "unitas_runtime_mark_destroyed")]
pub extern "C" fn unitas_runtime_mark_destroyed(handle: u64, entity: u64) -> i32 {
    #[used]
    static KEEP: extern "C" fn(u64, u64) -> i32 = unitas_runtime_mark_destroyed;
    let Ok(channels) = destroy_channels().lock() else {
        return -1;
    };
    let Some(channel) = channels.get(&handle) else {
        return -1;
    };
    let Ok(mut requests) = channel.lock() else {
        return -1;
    };
    requests.push(EntityId::new(entity));
    0
}

pub struct LoadedScene {
    pub(crate) name: String,
    pub(crate) graph: SceneGraph,
    pub(crate) scripts: Vec<ScriptInstance>,
    pub(crate) bodies: Vec<EntityId>,
    pub(crate) body_offsets: Vec<(EntityId, [f32; 3])>,
    pub(crate) body_colliders: Vec<(EntityId, Vec<ColliderDesc>)>,
    pub(crate) character_controllers: Vec<(EntityId, CharacterControllerComponent)>,
    pub(crate) audio_sources: Vec<(EntityId, AudioSourceComponent)>,
    pub(crate) animators: Vec<EntityId>,
    pub(crate) particles: Vec<EntityId>,
    pub(crate) trails: Vec<EntityId>,
    pub(crate) ui: UiComponents,
    pub(crate) diagnostics: Vec<(EntityId, String)>,
    pub(crate) entity_base: u64,
    pub(crate) entity_count: u64,
    pub(crate) rigid_bodies: std::collections::BTreeSet<EntityId>,
}

impl LoadedScene {
    pub fn scripts(&self) -> &[ScriptInstance] {
        &self.scripts
    }

    pub fn bodies(&self) -> &[EntityId] {
        &self.bodies
    }

    /// The colliders the scene bound to one body, one per Unity collider
    /// component of the GameObject.
    pub fn body_colliders(&self, entity: EntityId) -> &[ColliderDesc] {
        self.body_colliders
            .iter()
            .find(|(body, _)| *body == entity)
            .map_or(&[], |(_, colliders)| colliders.as_slice())
    }

    pub fn character_controllers(&self) -> &[(EntityId, CharacterControllerComponent)] {
        &self.character_controllers
    }

    pub fn audio_sources(&self) -> &[(EntityId, AudioSourceComponent)] {
        &self.audio_sources
    }

    pub fn animators(&self) -> &[EntityId] {
        &self.animators
    }

    pub fn diagnostics(&self) -> &[(EntityId, String)] {
        &self.diagnostics
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

/// The physics messages one fixed step produced. They are dispatched after the
/// step's script callbacks, the way the engine dispatched them before the player
/// loop tree existed.
pub(crate) struct PhysicsMessages {
    events: Vec<CollisionEvent>,
    trigger_events: Vec<TriggerEvent>,
    contacts_2d: Vec<ContactEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryRecord {
    pub entity: u64,
    pub distance: f64,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QueryRecord2D {
    pub entity: u64,
    pub distance: f32,
    pub point: [f32; 2],
    pub normal: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterMoveRequest {
    pub entity: Option<EntityId>,
    pub position: [f64; 3],
    pub motion: [f64; 3],
    pub radius: f64,
    pub height: f64,
    pub center: [f64; 3],
    pub slope_limit_deg: f64,
    pub step_offset: f64,
    pub skin_width: f64,
    pub min_move_distance: f64,
    pub layer_mask: u32,
    pub enabled: bool,
    pub delta_seconds: f64,
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

/// The screen size the interface falls back to when no canvas scaler states a
/// reference resolution.
pub const DEFAULT_UI_REFERENCE_RESOLUTION: [f32; 2] = [800.0, 600.0];

pub const DEFAULT_MAXIMUM_DELTA_TIME: f64 = 1.0 / 3.0;

pub struct Runtime {
    pub(crate) scenes: Vec<LoadedScene>,
    pub(crate) active_scene: Option<usize>,
    pub(crate) persistent: Vec<ScriptInstance>,
    pub(crate) persistent_entities: Vec<EntityId>,
    pub(crate) persistent_character_controllers: Vec<(EntityId, CharacterControllerComponent)>,
    pub(crate) persistent_audio_sources: Vec<(EntityId, AudioSourceComponent)>,
    pub(crate) persistent_animators: Vec<EntityId>,
    pub(crate) pending_play_on_awake: Vec<EntityId>,
    pub(crate) dynamic_entity_floor: u64,
    pub(crate) character_states: HashMap<EntityId, CharacterController>,
    pub(crate) default_character_state: CharacterController,
    pub(crate) script_view: Vec<ScriptInstance>,
    pub(crate) pending_scripts: bool,
    pub(crate) physics: PhysicsRuntimeWorld,
    pub(crate) particles: ParticleRuntimeWorld,
    pub(crate) particles_2d: ParticleRuntimeWorld2D,
    pub(crate) trails: TrailRuntimeWorld,
    pub(crate) navigation: NavigationRuntimeWorld,
    pub(crate) terrain: TerrainWorld,
    pub(crate) audio: AudioWorld,
    pub(crate) animation: AnimationWorld,
    pub(crate) input: InputState,
    pub(crate) render: RenderWorld,
    pub(crate) ui_screen_size: Option<[f32; 2]>,
    pub(crate) ui_painter: Mutex<Option<GlyphTextPainter>>,
    pub(crate) gpu_renderer: Option<Arc<Renderer>>,
    pub(crate) pending_ui_clicks: Vec<u64>,
    pub(crate) ui_pressed_button: Option<EntityId>,
    pub(crate) project_identity: ProjectIdentity,
    pub(crate) physics_settings: Option<PhysicsProjectSettings>,
    pub(crate) physics_2d_settings: Option<Physics2DProjectSettings>,
    pub(crate) quality_settings: QualitySettings,
    pub(crate) collision_payloads: HashMap<(u64, u64), CollisionPayload>,
    pub(crate) collision_payloads_2d: HashMap<(u64, u64), CollisionPayload2D>,
    pub(crate) callbacks: Option<Callbacks>,
    pub(crate) fixed_timestep: f64,
    pub(crate) maximum_delta_time: f64,
    pub(crate) time_scale: f64,
    pub(crate) accumulator: f64,
    pub(crate) first_tick: bool,
    pub(crate) is_shutdown: bool,
    pub(crate) destroy_handle: Option<u64>,
    pub(crate) destroy_channel: Option<DestroyChannel>,
    pub(crate) player_loop: PlayerLoop,
    pub(crate) pending_physics_messages: Option<PhysicsMessages>,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            scenes: Vec::new(),
            active_scene: None,
            persistent: Vec::new(),
            persistent_entities: Vec::new(),
            persistent_character_controllers: Vec::new(),
            persistent_audio_sources: Vec::new(),
            persistent_animators: Vec::new(),
            pending_play_on_awake: Vec::new(),
            dynamic_entity_floor: 1,
            character_states: HashMap::new(),
            default_character_state: CharacterController::default(),
            script_view: Vec::new(),
            pending_scripts: false,
            physics: PhysicsRuntimeWorld::new(),
            particles: ParticleRuntimeWorld::new(),
            particles_2d: ParticleRuntimeWorld2D::new(),
            trails: TrailRuntimeWorld::new(),
            navigation: NavigationRuntimeWorld::new(NavMesh::default()),
            terrain: TerrainWorld::new(),
            audio: AudioWorld::new(),
            animation: AnimationWorld::new(),
            input: InputState::with_default_axes(),
            render: RenderWorld::default(),
            ui_screen_size: None,
            ui_painter: Mutex::new(None),
            gpu_renderer: None,
            pending_ui_clicks: Vec::new(),
            ui_pressed_button: None,
            project_identity: ProjectIdentity::default(),
            physics_settings: None,
            physics_2d_settings: None,
            quality_settings: QualitySettings::default(),
            collision_payloads: HashMap::new(),
            collision_payloads_2d: HashMap::new(),
            callbacks: None,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
            maximum_delta_time: DEFAULT_MAXIMUM_DELTA_TIME,
            time_scale: 1.0,
            accumulator: 0.0,
            first_tick: true,
            is_shutdown: false,
            destroy_handle: None,
            destroy_channel: None,
            player_loop: PlayerLoop::new(),
            pending_physics_messages: None,
        })
    }

    pub fn load_scene(&mut self, path: &Path) -> Result<usize, RuntimeError> {
        self.load_scene_with_hook(path, LoadSceneMode::Single, |_, f| f())?;
        Ok(self.scenes.last().map_or(0, |scene| scene.scripts.len()))
    }

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
        let assembled = assemble_scene_file_bundle(path)?;
        let settings = discover_project_settings(path);
        let scripts = build_script_instances(&assembled.scripts)?;
        let (descs, collider_diagnostics) = body_descs(&assembled.graph, &assembled.physics);
        validate_bodies(&descs)?;
        self.prepare_scene_load(mode, &settings, &mut hook);
        let entity_base = self.next_entity_base();
        let mut graph = assembled.graph;
        graph.shift_entity_ids(entity_base);
        let mut ui = assembled.ui;
        ui.shift_entity_ids(entity_base);
        let ui = UiComponents::new(ui);
        let scripts = shift_scene_scripts(scripts, entity_base);
        let (bodies, body_offsets, body_colliders) = shift_scene_bodies(&descs, entity_base);
        let character_controllers = collect_character_controllers(&assembled.physics, entity_base);
        for (entity, config) in &character_controllers {
            self.character_states
                .insert(*entity, controller_from_component(Some(*entity), config));
        }
        let mut diagnostics = collect_material_diagnostics(&assembled.physics, entity_base);
        diagnostics.extend(
            collider_diagnostics
                .into_iter()
                .map(|(entity, text)| (EntityId::new(entity_base + entity.as_u64()), text)),
        );
        diagnostics.extend(self.populate_render_world(&assembled.render, entity_base, &graph));
        let audio_sources = self.register_scene_audio_sources(
            assembled.audio_bindings,
            entity_base,
            &graph,
            &mut diagnostics,
        );
        let rigidbodies = collect_scene_rigidbodies(&assembled.physics, entity_base);
        self.register_scene_bodies(&bodies, &descs, &rigidbodies)?;
        let particle_entities = self.register_scene_particles(&assembled.particles, entity_base);
        let trail_entities = self.register_scene_trails(&assembled.trails, entity_base);
        let entity_count = graph.len() as u64;
        let rigid_body_ids = rigidbodies.keys().copied().collect();
        self.scenes.push(LoadedScene {
            name: scene_name_from_path(path),
            graph,
            scripts,
            bodies,
            body_offsets,
            body_colliders,
            character_controllers,
            audio_sources,
            animators: Vec::new(),
            particles: particle_entities,
            trails: trail_entities,
            ui,
            diagnostics,
            entity_base,
            entity_count,
            rigid_bodies: rigid_body_ids,
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

    fn prepare_scene_load<H>(
        &mut self,
        mode: LoadSceneMode,
        settings: &DiscoveredProject,
        hook: &mut H,
    ) where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        match mode {
            LoadSceneMode::Single => {
                self.unload_all_scenes(hook);
                self.apply_project_settings(settings);
            }
            LoadSceneMode::Additive => {
                if self.scenes.is_empty() {
                    self.apply_project_settings(settings);
                }
            }
        }
    }

    fn register_scene_audio_sources(
        &mut self,
        audio_bindings: Vec<(EntityId, AudioSourceComponent)>,
        entity_base: u64,
        graph: &SceneGraph,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) -> Vec<(EntityId, AudioSourceComponent)> {
        let mut audio_sources = Vec::with_capacity(audio_bindings.len());
        for (entity, comp) in audio_bindings {
            let shifted = EntityId::new(entity_base + entity.as_u64());
            self.audio.ensure_source(shifted);
            self.audio.set_volume(shifted, comp.volume);
            self.audio.set_pitch(shifted, comp.pitch);
            self.audio.set_looping(shifted, comp.looping);
            self.audio.set_play_on_awake(shifted, comp.play_on_awake);
            self.audio.set_priority(shifted, comp.priority);
            self.audio.set_spatial_blend(shifted, comp.spatial_blend);
            self.audio.set_min_distance(shifted, comp.min_distance);
            self.audio.set_max_distance(shifted, comp.max_distance);
            self.audio.set_mute(shifted, comp.mute);
            if let Some(world_transform) = graph.world_transform(shifted) {
                self.audio
                    .set_source_position(shifted, world_transform.translation);
            }
            if let Some(ref diag) = comp.clip_diagnostic {
                diagnostics.push((shifted, diag.clone()));
                self.audio.mark_source_unsupported_clip(shifted);
            } else if let Some(ref clip_path) = comp.clip_path {
                match self.audio.load_wav_file(clip_path) {
                    Ok(clip_id) => {
                        self.audio.set_source_clip(shifted, Some(clip_id));
                    }
                    Err(err) => {
                        diagnostics.push((
                            shifted,
                            format!(
                                "AudioSource on entity {} failed to load clip '{}': {err}",
                                shifted.as_u64(),
                                clip_path.display()
                            ),
                        ));
                        self.audio.mark_source_unsupported_clip(shifted);
                    }
                }
            }
            if comp.enabled && comp.play_on_awake {
                self.pending_play_on_awake.push(shifted);
            }
            audio_sources.push((shifted, comp));
        }
        audio_sources
    }

    fn register_scene_particles(
        &mut self,
        particles: &[ParticleBinding],
        entity_base: u64,
    ) -> Vec<EntityId> {
        let mut particle_entities = Vec::with_capacity(particles.len());
        for particle in particles {
            let entity = EntityId::new(entity_base + particle.entity.as_u64());
            let system = self.register_particle_system(
                entity,
                particle_seed(particle.entity),
                &particle.binding,
            );
            system.play();
            particle_entities.push(entity);
        }
        particle_entities
    }

    fn register_scene_trails(
        &mut self,
        trails: &[TrailBinding],
        entity_base: u64,
    ) -> Vec<EntityId> {
        let mut trail_entities = Vec::with_capacity(trails.len());
        for trail in trails {
            let entity = EntityId::new(entity_base + trail.entity.as_u64());
            self.register_trail(entity, &trail.binding);
            trail_entities.push(entity);
        }
        trail_entities
    }

    pub fn unload_scene(&mut self, name: &str) -> Result<(), RuntimeError> {
        self.unload_scene_with_hook(name, |_, f| f())
    }

    pub fn unload_scene_with_hook<H>(&mut self, name: &str, mut hook: H) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let index = match self.scenes.iter().position(|scene| scene.name == name) {
            Some(index) => index,
            None => return Err(RuntimeError::SceneNotFound(name.to_string())),
        };
        let scene = self.scenes.remove(index);
        self.remove_scene_render_handles(&scene);
        self.teardown_scene(&scene, &mut hook);
        for body in &scene.bodies {
            self.physics.world_3d_mut().remove_body(*body);
        }
        for entity in &scene.particles {
            self.particles.remove_system(*entity);
        }
        for entity in &scene.trails {
            self.trails.remove_trail(*entity);
        }
        self.trails.rebuild_draw_list();
        for (entity, _) in &scene.character_controllers {
            if !self.persistent_entities.contains(entity) {
                self.character_states.remove(entity);
            }
        }
        for (entity, _) in &scene.audio_sources {
            if !self.persistent_entities.contains(entity) {
                self.pending_play_on_awake.retain(|e| e != entity);
                self.audio.stop(*entity);
                self.audio.remove_source(*entity);
            }
        }
        for entity in &scene.animators {
            if !self.persistent_entities.contains(entity)
                && !self.persistent_animators.contains(entity)
            {
                self.animation.remove_animator(*entity);
            }
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
                self.scenes[index]
                    .body_offsets
                    .retain(|(id, _)| *id != entity);
                found = true;
                break;
            }
        }
        for index in 0..self.scenes.len() {
            let controller_index = self.scenes[index]
                .character_controllers
                .iter()
                .position(|(id, _)| *id == entity);
            if let Some(controller_index) = controller_index {
                let entry = self.scenes[index]
                    .character_controllers
                    .remove(controller_index);
                self.persistent_character_controllers.push(entry);
                found = true;
                break;
            }
        }
        for index in 0..self.scenes.len() {
            let audio_index = self.scenes[index]
                .audio_sources
                .iter()
                .position(|(id, _)| *id == entity);
            if let Some(audio_index) = audio_index {
                let entry = self.scenes[index].audio_sources.remove(audio_index);
                self.persistent_audio_sources.push(entry);
                found = true;
                break;
            }
        }
        for index in 0..self.scenes.len() {
            let anim_index = self.scenes[index]
                .animators
                .iter()
                .position(|id| *id == entity);
            if let Some(anim_index) = anim_index {
                let entry = self.scenes[index].animators.remove(anim_index);
                if !self.persistent_animators.contains(&entry) {
                    self.persistent_animators.push(entry);
                }
                found = true;
                break;
            }
        }
        if !found && self.animation.entry(entity).is_some() {
            if !self.persistent_animators.contains(&entity) {
                self.persistent_animators.push(entity);
            }
            found = true;
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

    fn apply_project_settings(&mut self, settings: &DiscoveredProject) {
        self.physics.world_3d_mut().set_gravity(settings.gravity);
        self.fixed_timestep = settings.fixed_timestep;
        self.input.set_axes(settings.axes.clone());
        self.input.reset();
        self.project_identity = settings.identity.clone();
        self.physics_settings = settings.physics;
        self.physics_2d_settings = settings.physics_2d;
        self.quality_settings = settings.quality.clone();
        if let Some(physics) = settings.physics.as_ref() {
            let matrix = layer_matrix_of(physics);
            self.physics.set_layer_collision_matrix_3d(&matrix);
            let solver = solver_settings_of(physics);
            self.physics.set_solver_settings_3d(solver);
        }
        let effective_2d = settings.physics_2d.unwrap_or_default();
        apply_physics2d_project_settings(self.physics.world_2d_mut(), &effective_2d);
    }

    pub fn physics_project_settings(&self) -> Option<&PhysicsProjectSettings> {
        self.physics_settings.as_ref()
    }

    pub fn physics_2d_project_settings(&self) -> Option<&Physics2DProjectSettings> {
        self.physics_2d_settings.as_ref()
    }

    pub fn quality_settings(&self) -> &QualitySettings {
        &self.quality_settings
    }

    pub fn quality_level_for_target(&self, target_group_id: &str) -> Option<&QualityLevel> {
        let index = self
            .quality_settings
            .per_platform_default_quality
            .iter()
            .find_map(|(platform, level_index)| {
                (platform == target_group_id).then_some(*level_index)
            })
            .unwrap_or(self.quality_settings.current_quality);
        self.quality_settings.levels.get(index)
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
                self.physics.world_3d_mut().remove_body(*body);
            }
            for entity in &scene.particles {
                self.particles.remove_system(*entity);
            }
            for entity in &scene.trails {
                self.trails.remove_trail(*entity);
            }
            self.trails.rebuild_draw_list();
            for (entity, _) in &scene.character_controllers {
                if !self.persistent_entities.contains(entity) {
                    self.character_states.remove(entity);
                }
            }
            for (entity, _) in &scene.audio_sources {
                if !self.persistent_entities.contains(entity) {
                    self.pending_play_on_awake.retain(|e| e != entity);
                    self.audio.stop(*entity);
                    self.audio.remove_source(*entity);
                }
            }
            for entity in &scene.animators {
                if !self.persistent_entities.contains(entity)
                    && !self.persistent_animators.contains(entity)
                {
                    self.animation.remove_animator(*entity);
                }
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

    pub(crate) fn next_entity_base(&self) -> u64 {
        let mut highest = self.dynamic_entity_floor.max(1);
        for scene in &self.scenes {
            highest = highest.max(scene.entity_base + scene.entity_count);
        }
        for entity in &self.persistent_entities {
            highest = highest.max(entity.as_u64() + 1);
        }
        highest
    }

    pub fn ensure_audio_source(&mut self, entity: u64) -> EntityId {
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
        self.audio.ensure_source(id);
        id
    }

    pub fn audio(&self) -> &AudioWorld {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioWorld {
        &mut self.audio
    }

    pub fn physics_mut(&mut self) -> &mut PhysicsRuntimeWorld {
        &mut self.physics
    }

    pub fn particles_mut(&mut self) -> &mut ParticleRuntimeWorld {
        &mut self.particles
    }

    pub fn register_particle_system(
        &mut self,
        entity: EntityId,
        seed: u64,
        binding: &ParticleSystemBinding,
    ) -> &mut ParticleSystem {
        self.particles.register_binding(entity, seed, binding)
    }

    pub fn particle_draw_list(&self) -> &[ParticleDrawItem] {
        self.particles.draw_list()
    }

    pub fn trails(&self) -> &TrailRuntimeWorld {
        &self.trails
    }

    pub fn trails_mut(&mut self) -> &mut TrailRuntimeWorld {
        &mut self.trails
    }

    pub fn register_trail(
        &mut self,
        entity: EntityId,
        binding: &TrailRendererBinding,
    ) -> &mut Trail {
        self.trails.register_binding(entity, binding)
    }

    pub fn trail_draw_list(&self) -> &[TrailDrawItem] {
        self.trails.draw_list()
    }

    pub fn particles_2d_mut(&mut self) -> &mut ParticleRuntimeWorld2D {
        &mut self.particles_2d
    }

    pub fn navigation_mut(&mut self) -> &mut NavigationRuntimeWorld {
        &mut self.navigation
    }

    pub fn terrain(&self) -> &TerrainWorld {
        &self.terrain
    }

    pub fn terrain_mut(&mut self) -> &mut TerrainWorld {
        &mut self.terrain
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

    fn step_animators(&mut self, scaled_delta: f64) {
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
        let child_updates = self.animation.tick(scaled_delta);
        for entity in self.animation.take_pending_transform_writes() {
            if !self.write_back_animator_transform(entity) {
                self.animation.queue_transform_write(entity);
            }
        }
        for (entity, child_bindings) in child_updates {
            self.apply_child_animator_bindings(entity, &child_bindings);
        }
    }

    fn step_trails(&mut self, delta_time: f32) {
        let entities = self.trails.trail_entities();
        let mut positions = BTreeMap::new();
        for entity in entities {
            if let Some(transform) = self.entity_transform(entity) {
                positions.insert(entity, transform.translation);
            }
        }
        self.trails.step_all(delta_time, &positions);
    }

    fn write_back_animator_transform(&mut self, entity: EntityId) -> bool {
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

    fn apply_child_animator_bindings(
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

    pub fn audio_source_config(&self, entity: EntityId) -> Option<&AudioSourceComponent> {
        for scene in &self.scenes {
            if let Some((_, config)) = scene.audio_sources.iter().find(|(id, _)| *id == entity) {
                return Some(config);
            }
        }
        self.persistent_audio_sources
            .iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, config)| config)
    }

    fn drain_pending_play_on_awake(&mut self) {
        self.sync_audio_source_positions();
        let pending = std::mem::take(&mut self.pending_play_on_awake);
        for entity in pending {
            if self.audio.source_state(entity) == AudioSourceState::Stopped {
                self.audio.play(entity, false);
            }
        }
    }

    fn sync_audio_source_positions(&mut self) {
        for scene in &self.scenes {
            for (entity, _) in &scene.audio_sources {
                if let Some(world_transform) = scene.graph.world_transform(*entity) {
                    self.audio
                        .set_source_position(*entity, world_transform.translation);
                }
            }
        }
    }

    pub(crate) fn refresh_script_view(&mut self) {
        let scene_scripts = match self.active_scene.and_then(|index| self.scenes.get(index)) {
            Some(scene) => scene.scripts.clone(),
            None => Vec::new(),
        };
        let persistent = self.persistent.clone();
        self.script_view = scene_scripts;
        self.script_view.extend(persistent);
        self.pending_scripts = self.scripts_need_init();
    }

    fn scripts_need_init(&self) -> bool {
        if self
            .scenes
            .iter()
            .any(|scene| scene.scripts.iter().any(|script| !script.awake_called))
        {
            return true;
        }
        self.persistent.iter().any(|script| !script.awake_called)
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

    fn find_script_at(&self, entity: EntityId, ordinal: usize) -> Option<&ScriptInstance> {
        for scene in &self.scenes {
            if let Some(script) = scene.scripts.get(ordinal)
                && script.entity == entity
            {
                return Some(script);
            }
        }
        if let Some(script) = self.persistent.get(ordinal)
            && script.entity == entity
        {
            return Some(script);
        }
        for scene in &self.scenes {
            if let Some(script) = scene.scripts.iter().find(|script| script.entity == entity) {
                return Some(script);
            }
        }
        self.persistent
            .iter()
            .find(|script| script.entity == entity)
    }

    fn find_script_at_mut(
        &mut self,
        entity: EntityId,
        ordinal: usize,
    ) -> Option<&mut ScriptInstance> {
        if let Some(scene_idx) = self.scenes.iter().position(|scene| {
            scene
                .scripts
                .get(ordinal)
                .is_some_and(|script| script.entity == entity)
        }) {
            return self.scenes[scene_idx].scripts.get_mut(ordinal);
        }
        if self
            .persistent
            .get(ordinal)
            .is_some_and(|script| script.entity == entity)
        {
            return self.persistent.get_mut(ordinal);
        }
        for scene in &mut self.scenes {
            if let Some(script) = scene
                .scripts
                .iter_mut()
                .find(|script| script.entity == entity)
            {
                return Some(script);
            }
        }
        self.persistent
            .iter_mut()
            .find(|script| script.entity == entity)
    }

    fn init_pending_scripts<H>(&mut self, callbacks: Callbacks, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let mut pending: Vec<(EntityId, usize, u64, u64, u64)> = Vec::new();
        for scene in &self.scenes {
            for (ordinal, script) in scene.scripts.iter().enumerate() {
                if !script.awake_called {
                    pending.push((
                        script.entity,
                        ordinal,
                        script.guid_hi,
                        script.guid_lo,
                        script.script_handle,
                    ));
                }
            }
        }
        for (ordinal, script) in self.persistent.iter().enumerate() {
            if !script.awake_called {
                pending.push((
                    script.entity,
                    ordinal,
                    script.guid_hi,
                    script.guid_lo,
                    script.script_handle,
                ));
            }
        }
        for (entity, ordinal, guid_hi, guid_lo, existing_handle) in pending.iter().copied() {
            if existing_handle != 0 {
                continue;
            }
            let mut created = 0;
            hook(self, &mut || {
                created = (callbacks.create_script)(entity.as_u64(), guid_hi, guid_lo);
            });
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.script_handle = created;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|script| script.script_handle)
            else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.awake)(handle);
            });
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.awake_called = true;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|script| script.script_handle)
            else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.on_enable)(handle);
            });
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.on_enable_called = true;
            }
        }
        for (entity, ordinal, _, _, _) in pending.iter().copied() {
            let Some(handle) = self
                .find_script_at(entity, ordinal)
                .map(|script| script.script_handle)
            else {
                continue;
            };
            hook(self, &mut || {
                (callbacks.start)(handle);
            });
            if let Some(script) = self.find_script_at_mut(entity, ordinal) {
                script.start_called = true;
            }
        }
    }

    pub fn set_callbacks(&mut self, callbacks: Callbacks) {
        self.callbacks = Some(callbacks);
    }

    /// The player loop the runtime walks every frame.
    pub fn player_loop(&self) -> &PlayerLoop {
        &self.player_loop
    }

    /// The current player loop tree.
    pub fn player_loop_tree(&self) -> &PlayerLoopSystem {
        self.player_loop.root()
    }

    /// Replaces the player loop tree. A tree that drops a subsystem drops the
    /// work that subsystem carried.
    pub fn set_player_loop_tree(&mut self, tree: PlayerLoopSystem) -> Result<(), PlayerLoopError> {
        self.player_loop.set_root(tree)
    }

    /// Registers a custom player loop system next to the system it is
    /// anchored to. It runs every frame at that position.
    pub fn register_player_loop_custom_system(
        &mut self,
        name: &str,
        type_name: &str,
        anchor_type: &str,
        position: CustomSystemPosition,
        callback: PlayerLoopCallback,
        user_data: u64,
    ) -> Result<(), PlayerLoopError> {
        self.player_loop.register_custom_system(
            name,
            type_name,
            anchor_type,
            position,
            callback,
            user_data,
        )
    }

    /// Removes a custom player loop system and the node it runs in.
    pub fn unregister_player_loop_custom_system(&mut self, type_name: &str) -> bool {
        self.player_loop.unregister_custom_system(type_name)
    }

    pub fn tick(&mut self, delta_seconds: f64) -> Result<(), RuntimeError> {
        self.tick_with_hook(delta_seconds, |_, f| f())
    }

    pub fn tick_with_hook<H>(&mut self, delta_seconds: f64, hook: H) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.is_shutdown {
            return Err(RuntimeError::AlreadyShutdown);
        }
        self.adopt_destroy_channel();
        self.apply_destroy_requests();
        self.sweep_destroyed_entities();
        let result = self.tick_frame(delta_seconds, hook);
        self.drain_scene_changes();
        result
    }

    fn tick_frame<H>(&mut self, delta_seconds: f64, mut hook: H) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.first_tick {
            self.first_tick = false;
            if let Some(callbacks) = self.callbacks {
                self.init_pending_scripts(callbacks, &mut hook);
            }
            self.drain_pending_play_on_awake();
            self.refresh_script_view();
            return Ok(());
        }
        if self.pending_scripts {
            if let Some(callbacks) = self.callbacks {
                self.init_pending_scripts(callbacks, &mut hook);
            }
            self.refresh_script_view();
        }
        if !self.pending_play_on_awake.is_empty() {
            self.drain_pending_play_on_awake();
        }
        let scaled_delta = delta_seconds * self.time_scale;
        self.run_player_loop(scaled_delta, delta_seconds, &mut hook)?;
        Ok(())
    }

    fn run_player_loop<H>(
        &mut self,
        scaled_delta: f64,
        delta_seconds: f64,
        hook: &mut H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let plan = self.player_loop.plan();
        self.run_player_loop_steps(&plan, scaled_delta, delta_seconds, hook)
    }

    fn run_player_loop_steps<H>(
        &mut self,
        steps: &[PlayerLoopStep],
        scaled_delta: f64,
        delta_seconds: f64,
        hook: &mut H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        for step in steps {
            match step {
                PlayerLoopStep::Builtin(work) => {
                    self.run_player_loop_work(*work, scaled_delta, delta_seconds, hook)?;
                }
                PlayerLoopStep::Custom {
                    callback,
                    user_data,
                } => {
                    let callback = *callback;
                    let user_data = *user_data;
                    hook(self, &mut || {
                        callback(user_data, scaled_delta);
                    });
                }
                PlayerLoopStep::FixedPhase { steps } => {
                    self.run_fixed_update_phase(steps, scaled_delta, delta_seconds, hook)?;
                }
            }
        }
        Ok(())
    }

    fn run_fixed_update_phase<H>(
        &mut self,
        steps: &[PlayerLoopStep],
        scaled_delta: f64,
        delta_seconds: f64,
        hook: &mut H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        const EPSILON: f64 = 1e-9;
        let fixed_step = self.fixed_timestep;
        while self.accumulator >= fixed_step - EPSILON {
            self.run_player_loop_steps(steps, scaled_delta, delta_seconds, hook)?;
            self.dispatch_physics_messages(hook);
            self.accumulator -= fixed_step;
            if self.accumulator < 0.0 {
                self.accumulator = 0.0;
            }
        }
        Ok(())
    }

    fn run_player_loop_work<H>(
        &mut self,
        work: BuiltinWork,
        scaled_delta: f64,
        delta_seconds: f64,
        hook: &mut H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        match work {
            BuiltinWork::PlayerUpdateTime => {
                self.accumulator += scaled_delta;
                if self.accumulator > self.maximum_delta_time {
                    self.accumulator = self.maximum_delta_time;
                }
            }
            BuiltinWork::SynchronizeInputs => {
                self.input.begin_frame(scaled_delta);
            }
            BuiltinWork::PhysicsFixedUpdate => {
                let fixed_step = self.fixed_timestep;
                self.sync_kinematic_transforms();
                self.physics.world_3d_mut().step(fixed_step)?;
                self.write_back_dynamic_transforms();
                let events = self.physics.world_3d_mut().drain_events();
                let trigger_events = self.physics.world_3d_mut().drain_trigger_events();
                self.pending_physics_messages = Some(PhysicsMessages {
                    events,
                    trigger_events,
                    contacts_2d: Vec::new(),
                });
            }
            BuiltinWork::Physics2DFixedUpdate => {
                let contacts_2d = self.physics.step_2d_contacts(self.fixed_timestep as f32);
                if let Some(messages) = &mut self.pending_physics_messages {
                    messages.contacts_2d = contacts_2d;
                }
            }
            BuiltinWork::ScriptRunBehaviourFixedUpdate => {
                let fixed_step = self.fixed_timestep;
                if let Some(callbacks) = self.callbacks {
                    let handles = self.all_script_handles();
                    for handle in handles {
                        hook(self, &mut || {
                            (callbacks.fixed_update)(handle, fixed_step);
                        });
                    }
                }
            }
            BuiltinWork::ScriptRunBehaviourUpdate => {
                if let Some(callbacks) = self.callbacks {
                    let handles = self.all_script_handles();
                    for handle in handles {
                        hook(self, &mut || {
                            (callbacks.update)(handle, scaled_delta);
                        });
                    }
                }
            }
            BuiltinWork::LegacyAnimationUpdate => {
                self.step_animators(scaled_delta);
            }
            BuiltinWork::VFXUpdate => {
                self.particles
                    .step_all(scaled_delta as f32, Some(self.physics.world_3d()));
                self.particles_2d
                    .step_all(scaled_delta as f32, Some(self.physics.world_2d()));
                self.step_trails(scaled_delta as f32);
                if scaled_delta >= 0.0 {
                    let _ = self.navigation.step(scaled_delta);
                }
                self.terrain.apply_heightfields(self.physics.world_3d_mut());
            }
            BuiltinWork::UpdateAudio => {
                self.sync_audio_source_positions();
                self.audio.tick(delta_seconds, self.time_scale);
            }
            BuiltinWork::UpdateCanvas => {
                self.route_ui_pointer();
            }
            BuiltinWork::ScriptLateUpdate => {
                if let Some(callbacks) = self.callbacks {
                    let late_handles = self.all_script_handles();
                    for handle in late_handles {
                        hook(self, &mut || {
                            (callbacks.late_update)(handle, scaled_delta);
                        });
                    }
                }
            }
            BuiltinWork::InitPendingScripts => {
                if self.pending_scripts {
                    if let Some(callbacks) = self.callbacks {
                        self.init_pending_scripts(callbacks, hook);
                    }
                    self.refresh_script_view();
                }
            }
        }
        Ok(())
    }

    fn dispatch_physics_messages<H>(&mut self, hook: &mut H)
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        let Some(messages) = self.pending_physics_messages.take() else {
            return;
        };
        let Some(callbacks) = self.callbacks else {
            return;
        };
        // A step's physics messages are dispatched before the next step
        // runs, so the message order does not depend on how many fixed
        // steps a rendered frame happens to batch. The 2D contacts join
        // the 3D ones on one contact-event path, and the `is_2d`
        // discriminator keeps a 2D contact off the 3D handlers.
        let mut contacts: Vec<ContactEvent> = Vec::new();
        for event in messages.events {
            contacts.push(ContactEvent {
                a: event.a,
                b: event.b,
                trigger: false,
                phase: event.phase,
                is_2d: false,
                contacts: Vec::new(),
                relative_velocity: [0.0; 2],
            });
        }
        for event in messages.trigger_events {
            let has_rigidbody = self.entity_belongs_to_rigid_body(event.a)
                || self.entity_belongs_to_rigid_body(event.b);
            if !has_rigidbody {
                continue;
            }
            contacts.push(ContactEvent {
                a: event.a,
                b: event.b,
                trigger: true,
                phase: event.phase,
                is_2d: false,
                contacts: Vec::new(),
                relative_velocity: [0.0; 2],
            });
        }
        contacts.extend(messages.contacts_2d);
        for contact in contacts {
            let a = contact.a;
            let b = contact.b;
            if !contact.is_2d && !contact.trigger {
                let payload = self.physics.world_3d().collision_payload(a, b).cloned();
                if let Some(payload) = payload {
                    self.collision_payloads
                        .insert(ordered_pair(a.as_u64(), b.as_u64()), payload);
                }
            }
            if contact.is_2d && !contact.trigger {
                self.collision_payloads_2d.insert(
                    (a.as_u64(), b.as_u64()),
                    CollisionPayload2D {
                        impulse: contact
                            .contacts
                            .iter()
                            .map(|point| point.normal_impulse)
                            .sum(),
                        relative_velocity: contact.relative_velocity,
                        contacts: contact.contacts,
                    },
                );
            }
            let a = a.as_u64();
            let b = b.as_u64();
            let phase = contact_phase_code(contact.phase, contact.is_2d);
            if contact.trigger {
                hook(self, &mut || {
                    (callbacks.trigger)(a, b, phase);
                });
            } else {
                hook(self, &mut || {
                    (callbacks.collision)(a, b, phase);
                });
            }
        }
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
        self.pending_play_on_awake.clear();
        for scene in &self.scenes {
            for (entity, _) in &scene.audio_sources {
                self.audio.stop(*entity);
                self.audio.remove_source(*entity);
            }
            for entity in &scene.animators {
                self.animation.remove_animator(*entity);
            }
        }
        for (entity, _) in &self.persistent_audio_sources {
            self.audio.stop(*entity);
            self.audio.remove_source(*entity);
        }
        for entity in &self.persistent_animators {
            self.animation.remove_animator(*entity);
        }
        Ok(())
    }

    pub fn scripts(&self) -> &[ScriptInstance] {
        &self.script_view
    }

    pub fn scene_graph(&self) -> Option<&SceneGraph> {
        self.active_scene
            .and_then(|index| self.scenes.get(index))
            .map(|scene| &scene.graph)
    }

    pub fn scene_graph_mut(&mut self) -> Option<&mut SceneGraph> {
        self.active_scene
            .and_then(|index| self.scenes.get_mut(index))
            .map(|scene| &mut scene.graph)
    }

    pub fn mark_destroyed(&mut self, entity: EntityId) -> Result<(), RuntimeError> {
        for scene in &mut self.scenes {
            if scene.graph.contains(entity) {
                return match scene.graph.mark_destroyed(entity) {
                    Ok(()) => Ok(()),
                    Err(_) => Err(RuntimeError::EntityNotFound(entity)),
                };
            }
        }
        Err(RuntimeError::EntityNotFound(entity))
    }

    fn adopt_destroy_channel(&mut self) {
        BOUND_DESTROY_CHANNEL.with(|cell| {
            let Some((handle, channel)) = cell.borrow_mut().take() else {
                return;
            };
            if let Some(current) = self.destroy_handle
                && current != handle
            {
                return;
            }
            self.destroy_handle = Some(handle);
            self.destroy_channel = Some(channel);
        });
    }

    fn apply_destroy_requests(&mut self) {
        let Some(channel) = self.destroy_channel.clone() else {
            return;
        };
        let requests = {
            let Ok(mut requests) = channel.lock() else {
                return;
            };
            std::mem::take(&mut *requests)
        };
        for entity in requests {
            let _ = self.mark_destroyed(entity);
        }
    }

    fn sweep_destroyed_entities(&mut self) {
        let mut swept_all = Vec::new();
        for scene in &mut self.scenes {
            let swept = scene.graph.sweep_destroyed();
            if !swept.is_empty() {
                scene.scripts.retain(|s| !swept.contains(&s.entity));
                scene.bodies.retain(|b| !swept.contains(b));
                scene.body_offsets.retain(|(id, _)| !swept.contains(id));
                swept_all.extend(swept);
            }
        }
        if !swept_all.is_empty() {
            self.persistent.retain(|s| !swept_all.contains(&s.entity));
            self.persistent_entities.retain(|e| !swept_all.contains(e));
            for entity in swept_all {
                self.physics.world_3d_mut().remove_body(entity);
            }
            self.refresh_script_view();
        }
    }

    fn drain_scene_changes(&mut self) {
        for scene in &mut self.scenes {
            let _changes = scene.graph.take_changes();
        }
    }

    pub fn diagnostics(&self) -> Vec<(EntityId, &str)> {
        let mut items = Vec::new();
        for scene in &self.scenes {
            for (entity, msg) in &scene.diagnostics {
                items.push((*entity, msg.as_str()));
            }
        }
        for entity in self.animation.entities() {
            if let Some(diags) = self.animation.diagnostics(entity) {
                for msg in diags {
                    items.push((entity, msg.as_str()));
                }
            }
        }
        items
    }

    pub fn entity_diagnostics(&self, entity: EntityId) -> Vec<&str> {
        let mut items = Vec::new();
        for scene in &self.scenes {
            for (id, msg) in &scene.diagnostics {
                if *id == entity {
                    items.push(msg.as_str());
                }
            }
        }
        if let Some(diags) = self.animation.diagnostics(entity) {
            for msg in diags {
                items.push(msg.as_str());
            }
        }
        items
    }

    pub fn raycast(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        layer_mask: u32,
        include_triggers: bool,
    ) -> Vec<QueryRecord> {
        self.records_from_hits(self.physics.world_3d().raycast_with_triggers(
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
        self.records_from_hits(self.physics.world_3d().linecast(
            origin,
            end,
            layer_mask,
            include_triggers,
        ))
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
        self.records_from_hits(self.physics.world_3d().sphere_cast(
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
        self.records_from_hits(self.physics.world_3d().box_cast(
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
            .physics
            .world_3d()
            .overlap_sphere_with_triggers(center, radius, layer_mask, include_triggers)
            .into_iter()
            .map(|entity| QueryRecord {
                entity: entity.as_u64(),
                distance: 0.0,
                point: self.physics.world_3d().position(entity).unwrap_or(center),
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

    pub fn set_fixed_timestep(&mut self, seconds: f64) -> bool {
        if !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        self.fixed_timestep = seconds;
        true
    }

    pub fn solver_type(&self) -> SolverType {
        self.physics.world_3d().solver_type()
    }

    pub fn entity_has_rigidbody(&self, entity: EntityId) -> bool {
        self.scenes
            .iter()
            .any(|scene| scene.rigid_bodies.contains(&entity))
    }

    fn entity_belongs_to_rigid_body(&self, entity: EntityId) -> bool {
        if self.entity_has_rigidbody(entity) {
            return true;
        }
        self.scenes.iter().any(|scene| {
            let mut current = scene.graph.parent(entity);
            while let Some(parent) = current {
                if scene.rigid_bodies.contains(&parent) {
                    return true;
                }
                current = scene.graph.parent(parent);
            }
            false
        })
    }

    pub fn time_scale(&self) -> f64 {
        self.time_scale
    }

    pub fn set_time_scale(&mut self, time_scale: f64) {
        self.time_scale = time_scale;
        // A frozen scaled clock is the gameplay-pause signal: the audio bus
        // silences every mixer group but Menu until time flows again.
        self.audio.set_gameplay_paused(time_scale == 0.0);
    }

    pub fn maximum_delta_time(&self) -> f64 {
        self.maximum_delta_time
    }

    pub fn set_maximum_delta_time(&mut self, seconds: f64) {
        self.maximum_delta_time = if seconds.is_finite() && seconds > 0.0 {
            seconds
        } else {
            DEFAULT_MAXIMUM_DELTA_TIME
        };
    }

    pub fn body_position(&self, entity: EntityId) -> Option<[f64; 3]> {
        if let Some(pos) = self.physics.world_3d().position(entity) {
            return Some(pos);
        }
        if let Some(pos_2d) = self.physics.body_position_2d(entity) {
            let z = self
                .entity_transform(entity)
                .map_or(0.0, |t| f64::from(t.translation[2]));
            return Some([f64::from(pos_2d[0]), f64::from(pos_2d[1]), z]);
        }
        None
    }

    pub fn body_velocity(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.physics.world_3d().velocity(entity)
    }

    pub fn set_body_position(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
    ) -> Result<(), RuntimeError> {
        self.physics.world_3d_mut().set_position(entity, position)?;
        self.write_back_body_transform(entity, position);
        Ok(())
    }

    pub fn teleport_body_position(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
    ) -> Result<(), RuntimeError> {
        self.physics
            .world_3d_mut()
            .teleport_position(entity, position)?;
        self.write_back_body_transform(entity, position);
        Ok(())
    }

    pub fn move_body_position(
        &mut self,
        entity: EntityId,
        position: [f64; 3],
    ) -> Result<(), RuntimeError> {
        self.physics
            .world_3d_mut()
            .move_position(entity, position)?;
        self.write_back_body_transform(entity, position);
        Ok(())
    }

    pub fn collision_payload(&self, first: u64, second: u64) -> Option<&CollisionPayload> {
        self.collision_payloads.get(&ordered_pair(first, second))
    }

    pub fn collision_payload_2d(&self, first: u64, second: u64) -> Option<&CollisionPayload2D> {
        self.collision_payloads_2d
            .get(&(first, second))
            .or_else(|| self.collision_payloads_2d.get(&(second, first)))
    }

    pub fn set_body_velocity(
        &mut self,
        entity: EntityId,
        velocity: [f64; 3],
    ) -> Result<(), RuntimeError> {
        self.physics.world_3d_mut().set_velocity(entity, velocity)?;
        Ok(())
    }

    pub fn set_body_rotation(
        &mut self,
        entity: EntityId,
        rotation: [f64; 4],
    ) -> Result<(), RuntimeError> {
        self.physics.world_3d_mut().set_rotation(entity, rotation)?;
        self.write_back_body_rotation(entity, rotation);
        Ok(())
    }

    pub fn add_body_force(
        &mut self,
        entity: EntityId,
        force: [f64; 3],
        mode: ForceMode,
    ) -> Result<(), RuntimeError> {
        self.physics.world_3d_mut().add_force(entity, force, mode)?;
        Ok(())
    }

    pub fn body_role(&self, entity: EntityId) -> Option<BodyRole> {
        self.physics.world_3d().body_role(entity)
    }

    pub fn body_is_dynamic(&self, entity: EntityId) -> bool {
        self.physics.world_3d().body_role(entity) == Some(BodyRole::Dynamic)
    }

    pub fn character_controller_config(
        &self,
        entity: EntityId,
    ) -> Option<&CharacterControllerComponent> {
        for scene in &self.scenes {
            if let Some((_, config)) = scene
                .character_controllers
                .iter()
                .find(|(id, _)| *id == entity)
            {
                return Some(config);
            }
        }
        self.persistent_character_controllers
            .iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, config)| config)
    }

    pub fn character_move(
        &mut self,
        request: CharacterMoveRequest,
    ) -> Result<MoveResult, RuntimeError> {
        let delta_seconds = if request.delta_seconds > 0.0 {
            request.delta_seconds
        } else {
            self.fixed_timestep
        };
        let mut controller = match request.entity {
            Some(entity) => self
                .character_states
                .get(&entity)
                .copied()
                .unwrap_or_default(),
            None => self.default_character_state,
        };
        controller.radius = request.radius;
        controller.height = request.height;
        controller.center = request.center;
        controller.slope_limit_deg = request.slope_limit_deg;
        controller.step_offset = request.step_offset;
        controller.skin_width = request.skin_width;
        controller.min_move_distance = if request.min_move_distance > 0.0 {
            request.min_move_distance
        } else if request.min_move_distance == 0.0 {
            0.001
        } else {
            request.min_move_distance
        };
        controller.layer_mask = request.layer_mask;
        controller.enabled = request.enabled;
        controller.excluded_entity = request.entity;

        let result = controller.move_character_with_dt(
            self.physics.world_3d(),
            request.position,
            request.motion,
            delta_seconds,
        )?;
        match request.entity {
            Some(entity) => {
                self.character_states.insert(entity, controller);
                self.write_back_entity_transform(entity, result.position);
            }
            None => {
                self.default_character_state = controller;
            }
        }
        Ok(result)
    }

    pub fn character_teleport(
        &mut self,
        entity: Option<EntityId>,
        position: [f64; 3],
    ) -> Result<[f64; 3], RuntimeError> {
        let mut controller = match entity {
            Some(id) => self.character_states.get(&id).copied().unwrap_or_default(),
            None => self.default_character_state,
        };
        let target = controller.teleport(position)?;
        match entity {
            Some(id) => {
                self.character_states.insert(id, controller);
                self.write_back_entity_transform(id, target);
            }
            None => {
                self.default_character_state = controller;
            }
        }
        Ok(target)
    }

    fn write_back_entity_transform(&mut self, entity: EntityId, world_pos: [f64; 3]) {
        for scene in &mut self.scenes {
            if scene.graph.entity(entity).is_none() {
                continue;
            }
            let world_translation = [
                world_pos[0] as f32,
                world_pos[1] as f32,
                world_pos[2] as f32,
            ];
            let local_translation = match scene
                .graph
                .parent(entity)
                .and_then(|parent| scene.graph.world_transform(parent))
            {
                Some(parent_world) => world_to_local_translation(&parent_world, world_translation),
                None => world_translation,
            };
            if let Some(transform) = scene.graph.transform_mut(entity) {
                transform.translation = local_translation;
            }
            return;
        }
    }

    fn sync_kinematic_transforms(&mut self) {
        for scene in &self.scenes {
            for (entity, offset) in &scene.body_offsets {
                if self.physics.world_3d().body_role(*entity) != Some(BodyRole::Kinematic) {
                    continue;
                }
                let Some(world_transform) = scene.graph.world_transform(*entity) else {
                    continue;
                };
                let graph_pos = [
                    f64::from(world_transform.translation[0]) + f64::from(offset[0]),
                    f64::from(world_transform.translation[1]) + f64::from(offset[1]),
                    f64::from(world_transform.translation[2]) + f64::from(offset[2]),
                ];
                let should_update = match self.physics.world_3d().position(*entity) {
                    Some(current) => {
                        (graph_pos[0] - current[0]).abs() > 1e-6
                            || (graph_pos[1] - current[1]).abs() > 1e-6
                            || (graph_pos[2] - current[2]).abs() > 1e-6
                    }
                    None => false,
                };
                if should_update {
                    let _ = self.physics.world_3d_mut().set_position(*entity, graph_pos);
                }
                let graph_rotation = normalize_rotation(world_transform.rotation);
                let graph_rotation = [
                    f64::from(graph_rotation[0]),
                    f64::from(graph_rotation[1]),
                    f64::from(graph_rotation[2]),
                    f64::from(graph_rotation[3]),
                ];
                let rotation_changed = match self.physics.world_3d().rotation(*entity) {
                    Some(current) => !rotations_equivalent(current, graph_rotation),
                    None => false,
                };
                if rotation_changed {
                    let _ = self
                        .physics
                        .world_3d_mut()
                        .set_rotation(*entity, graph_rotation);
                }
            }
        }
    }

    fn write_back_body_transform(&mut self, entity: EntityId, world_pos: [f64; 3]) {
        for scene in &mut self.scenes {
            let Some((_, offset)) = scene
                .body_offsets
                .iter()
                .find(|(body_entity, _)| *body_entity == entity)
                .copied()
            else {
                continue;
            };
            let world_translation = [
                world_pos[0] as f32 - offset[0],
                world_pos[1] as f32 - offset[1],
                world_pos[2] as f32 - offset[2],
            ];
            let local_translation = match scene
                .graph
                .parent(entity)
                .and_then(|parent| scene.graph.world_transform(parent))
            {
                Some(parent_world) => world_to_local_translation(&parent_world, world_translation),
                None => world_translation,
            };
            if let Some(transform) = scene.graph.transform_mut(entity) {
                transform.translation = local_translation;
            }
            return;
        }
    }

    fn write_back_body_rotation(&mut self, entity: EntityId, world_rotation: [f64; 4]) {
        for scene in &mut self.scenes {
            if scene.graph.entity(entity).is_none() {
                continue;
            }
            let world = [
                world_rotation[0] as f32,
                world_rotation[1] as f32,
                world_rotation[2] as f32,
                world_rotation[3] as f32,
            ];
            let local = match scene
                .graph
                .parent(entity)
                .and_then(|parent| scene.graph.world_transform(parent))
            {
                Some(parent_world) => {
                    let parent = normalize_rotation(parent_world.rotation);
                    multiply_rotation([-parent[0], -parent[1], -parent[2], parent[3]], world)
                }
                None => world,
            };
            if let Some(transform) = scene.graph.transform_mut(entity) {
                transform.rotation = normalize_rotation(local);
            }
            return;
        }
    }

    fn write_back_dynamic_transforms(&mut self) {
        for scene in &mut self.scenes {
            for (entity, offset) in &scene.body_offsets {
                if self.physics.world_3d().body_role(*entity) != Some(BodyRole::Dynamic) {
                    continue;
                }
                let Some(world_pos) = self.physics.world_3d().position(*entity) else {
                    continue;
                };
                let world_translation = [
                    world_pos[0] as f32 - offset[0],
                    world_pos[1] as f32 - offset[1],
                    world_pos[2] as f32 - offset[2],
                ];
                let local_translation = match scene
                    .graph
                    .parent(*entity)
                    .and_then(|parent| scene.graph.world_transform(parent))
                {
                    Some(parent_world) => {
                        world_to_local_translation(&parent_world, world_translation)
                    }
                    None => world_translation,
                };
                if let Some(transform) = scene.graph.transform_mut(*entity) {
                    transform.translation = local_translation;
                }
            }
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub(crate) fn entity_transform(&self, entity: EntityId) -> Option<Transform> {
        for scene in &self.scenes {
            if let Some(transform) = scene.graph.world_transform(entity) {
                return Some(transform);
            }
        }
        None
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

fn discover_project_settings(scene_path: &Path) -> DiscoveredProject {
    let Some(settings_dir) = find_project_settings_dir(scene_path) else {
        return DiscoveredProject {
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
            gravity: DEFAULT_GRAVITY,
            axes: default_axis_configs(),
            identity: ProjectIdentity::default(),
            physics: None,
            physics_2d: None,
            quality: QualitySettings::default(),
        };
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
    let physics = match parse_physics_manager(&settings_dir.join("DynamicsManager.asset")) {
        Ok(Some(settings)) => Some(settings),
        Ok(None) | Err(_) => None,
    };
    let physics_2d = match parse_physics2d_settings(&settings_dir.join("Physics2DSettings.asset")) {
        Ok(Some(settings)) => Some(settings),
        Ok(None) | Err(_) => None,
    };
    let quality = match parse_quality_settings(&settings_dir.join("QualitySettings.asset")) {
        Ok(Some(settings)) => settings,
        Ok(None) | Err(_) => QualitySettings::default(),
    };
    DiscoveredProject {
        fixed_timestep,
        gravity,
        axes,
        identity,
        physics,
        physics_2d,
        quality,
    }
}

struct DiscoveredProject {
    fixed_timestep: f64,
    gravity: [f64; 3],
    axes: Vec<AxisConfig>,
    identity: ProjectIdentity,
    physics: Option<PhysicsProjectSettings>,
    physics_2d: Option<Physics2DProjectSettings>,
    quality: QualitySettings,
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

fn build_script_instances(bindings: &[ScriptBinding]) -> Result<Vec<ScriptInstance>, RuntimeError> {
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

fn shift_scene_scripts(scripts: Vec<ScriptInstance>, entity_base: u64) -> Vec<ScriptInstance> {
    scripts
        .into_iter()
        .map(|script| ScriptInstance {
            entity: EntityId::new(entity_base + script.entity.as_u64()),
            ..script
        })
        .collect::<Vec<_>>()
}

fn particle_seed(entity: EntityId) -> u64 {
    entity.as_u64().wrapping_mul(0x9E37_79B9_7F4A_7C15)
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

pub(crate) fn world_to_local_translation(
    parent_world: &unitas_core::Transform,
    world_translation: [f32; 3],
) -> [f32; 3] {
    let delta = [
        world_translation[0] - parent_world.translation[0],
        world_translation[1] - parent_world.translation[1],
        world_translation[2] - parent_world.translation[2],
    ];
    let unit = normalize_rotation(parent_world.rotation);
    let inv_rotation = [-unit[0], -unit[1], -unit[2], unit[3]];
    let unrotated = rotate_vector(inv_rotation, delta);
    [
        if parent_world.scale[0] != 0.0 {
            unrotated[0] / parent_world.scale[0]
        } else {
            unrotated[0]
        },
        if parent_world.scale[1] != 0.0 {
            unrotated[1] / parent_world.scale[1]
        } else {
            unrotated[1]
        },
        if parent_world.scale[2] != 0.0 {
            unrotated[2] / parent_world.scale[2]
        } else {
            unrotated[2]
        },
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_audio::{AudioClip, AudioMixerGroup, SilenceReason};

    #[test]
    fn roll_a_ball_scene_binds_collect_chime_audio_source_clip()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/roll-a-ball/Assets/Scenes/RollABall.unity");
        runtime.load_scene(&scene_path)?;

        let scene_text = std::fs::read_to_string(&scene_path)?;
        let expected_guid = scene_text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("m_audioClip: "))
            .find_map(|value| {
                value
                    .split_once("guid: ")
                    .map(|(_, guid)| guid.split(',').next().unwrap_or_default().trim())
            })
            .ok_or("scene audio clip reference not found")?;

        let scene = runtime.scenes.first().ok_or("scene not loaded")?;
        let chime_entity = scene
            .graph
            .breadth_first_order()
            .into_iter()
            .find(|entity| runtime.entity_name(*entity) == Some("CollectChime"))
            .ok_or("CollectChime entity not found")?;

        let (_, config) = scene
            .audio_sources
            .iter()
            .find(|(entity, _)| *entity == chime_entity)
            .ok_or("CollectChime audio source not bound")?;
        assert_eq!(config.clip_guid.as_deref(), Some(expected_guid));

        let source = runtime
            .audio()
            .source(chime_entity)
            .ok_or("CollectChime audio source missing")?;
        let clip_id = source.clip.ok_or("CollectChime clip unresolved")?;
        assert!(clip_id > 0);
        let clip = runtime
            .audio()
            .clip(clip_id)
            .ok_or("CollectChime clip not loaded")?;
        assert!(clip.duration > 0.0);
        Ok(())
    }

    #[test]
    fn runtime_time_scale_pause_routes_sfx_music_and_menu_groups()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        let clip_id = runtime.audio_mut().register_clip(AudioClip {
            channels: 1,
            sample_rate: 100,
            samples: vec![0.4; 10],
            duration: 0.10,
        });
        let sfx = EntityId::new(100);
        let music = EntityId::new(101);
        let menu = EntityId::new(102);
        for entity in [sfx, music, menu] {
            runtime.audio_mut().ensure_source(entity);
            runtime.audio_mut().set_source_clip(entity, Some(clip_id));
        }
        runtime
            .audio_mut()
            .set_source_group(music, AudioMixerGroup::Music);
        runtime
            .audio_mut()
            .set_source_group(menu, AudioMixerGroup::Menu);
        for entity in [sfx, music, menu] {
            runtime.audio_mut().play(entity, false);
        }

        runtime.tick(0.0)?;
        runtime.set_time_scale(0.0);
        assert!(runtime.audio().is_gameplay_paused());
        assert_eq!(
            runtime.audio().inaudible_reason(sfx),
            Some(SilenceReason::GameplayPaused)
        );
        assert_eq!(
            runtime.audio().inaudible_reason(music),
            Some(SilenceReason::GameplayPaused)
        );
        assert_eq!(runtime.audio().inaudible_reason(menu), None);

        runtime.tick(0.04)?;
        assert_eq!(runtime.audio().source(sfx).map(|s| s.time), Some(0.0));
        assert_eq!(runtime.audio().source(music).map(|s| s.time), Some(0.0));
        assert_eq!(runtime.audio().source(menu).map(|s| s.time), Some(0.04));

        runtime.set_time_scale(1.0);
        assert!(!runtime.audio().is_gameplay_paused());
        assert_eq!(runtime.audio().inaudible_reason(sfx), None);
        assert_eq!(runtime.audio().inaudible_reason(music), None);
        Ok(())
    }

    #[test]
    fn physics2d_project_settings_and_abi_entry_points_configure_world_2d_and_survive_clear()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            unitas_runtime_destroy, unitas_runtime_new, unitas_runtime_physics2d_apply_settings,
            unitas_runtime_physics2d_clear, unitas_runtime_physics2d_get_layer_collision,
            unitas_runtime_physics2d_set_layer_collision,
        };

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_root = std::env::temp_dir().join(format!(
            "unitas_runtime_p2d_settings_{}_{nanos}",
            std::process::id()
        ));
        let settings_dir = temp_root.join("ProjectSettings");
        let scenes_dir = temp_root.join("Assets").join("Scenes");
        std::fs::create_dir_all(&settings_dir)?;
        std::fs::create_dir_all(&scenes_dir)?;

        let mut matrix_bits = [0xFFFF_FFFFu32; 32];
        matrix_bits[0] &= !(1u32 << 2);
        matrix_bits[2] &= !(1u32 << 0);
        let matrix_hex =
            unitas_asset::project_settings::format_layer_collision_matrix(&matrix_bits);
        let physics2d_yaml = format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!19 &1\nPhysics2DSettings:\n  m_ObjectHideFlags: 0\n  serializedVersion: 5\n  m_Gravity: {{x: 2.5, y: -14.0}}\n  m_VelocityIterations: 11\n  m_PositionIterations: 6\n  m_QueriesHitTriggers: 0\n  m_QueriesStartInColliders: 0\n  m_CallbacksOnDisable: 0\n  m_AutoSyncTransforms: 0\n  m_LayerCollisionMatrix: {matrix_hex}\n"
        );
        std::fs::write(settings_dir.join("Physics2DSettings.asset"), physics2d_yaml)?;

        let scene_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Root\n  m_Layer: 0\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n";
        let scene_path = scenes_dir.join("Test2D.unity");
        std::fs::write(&scene_path, scene_yaml)?;

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;

        let world_2d = runtime.physics_mut().world_2d();
        assert_eq!(world_2d.gravity(), [2.5, -14.0]);
        assert_eq!(world_2d.velocity_iterations(), 11);
        assert_eq!(world_2d.position_iterations(), 6);
        assert!(!world_2d.queries_hit_triggers());
        assert!(!world_2d.queries_start_in_colliders());
        assert!(!world_2d.callbacks_on_disable());
        assert!(!world_2d.auto_sync_transforms());
        assert!(world_2d.get_ignore_layer_collision(0, 2));
        assert!(!world_2d.get_ignore_layer_collision(0, 1));

        runtime.physics_mut().clear_2d();
        let cleared_2d = runtime.physics_mut().world_2d();
        assert_eq!(cleared_2d.gravity(), [2.5, -14.0]);
        assert_eq!(cleared_2d.velocity_iterations(), 11);
        assert_eq!(cleared_2d.position_iterations(), 6);
        assert!(!cleared_2d.queries_hit_triggers());
        assert!(cleared_2d.get_ignore_layer_collision(0, 2));

        std::fs::remove_dir_all(&temp_root)?;

        let handle = unitas_runtime_new();
        assert_ne!(handle, 0);
        assert_eq!(
            unitas_runtime_physics2d_apply_settings(handle, 0.0, -20.0, 9, 4, 0),
            0
        );
        assert_eq!(
            unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
            1
        );
        assert_eq!(
            unitas_runtime_physics2d_set_layer_collision(handle, 1, 2, 0),
            0
        );
        assert_eq!(
            unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
            0
        );
        assert_eq!(
            unitas_runtime_physics2d_get_layer_collision(handle, 2, 1),
            0
        );
        assert_eq!(unitas_runtime_physics2d_clear(handle), 0);
        assert_eq!(
            unitas_runtime_physics2d_get_layer_collision(handle, 1, 2),
            0
        );
        assert_eq!(
            unitas_runtime_physics2d_set_layer_collision(handle, 32, 0, 1),
            -2
        );
        assert_eq!(
            unitas_runtime_physics2d_get_layer_collision(handle, 0, 32),
            -2
        );
        assert_eq!(unitas_runtime_destroy(handle), 0);
        Ok(())
    }
}
