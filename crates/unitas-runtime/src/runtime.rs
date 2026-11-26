use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use unitas_animation::SampledBinding;
use unitas_asset::collision::{CollisionMesh, cook_builtin_collision_mesh, cook_collision_mesh};
use unitas_asset::particle::ParticleSystemBinding;
use unitas_asset::project_settings::{
    PhysicsProjectSettings, QualityLevel, QualitySettings, parse_physics_manager,
    parse_quality_settings,
};
use unitas_asset::scene::{
    AudioSourceComponent, CharacterControllerComponent, ColliderComponent, MeshColliderComponent,
    MeshRendererComponent, PhysicsBinding, RenderBinding, RigidbodyComponent, ScriptBinding,
    assemble_scene_file_bundle,
};
use unitas_asset::trail::TrailRendererBinding;
use unitas_asset::{
    DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY, InputAxis, ProjectIdentity, parse_dynamics_manager,
    parse_input_manager, parse_player_settings, parse_time_manager,
};
use unitas_audio::{AudioSourceState, AudioWorld};
use unitas_core::{
    EntityId, IDENTITY_ROTATION, SceneGraph, Transform, multiply_rotation, normalize_rotation,
    rotate_vector,
};
use unitas_physics::navigation::NavMesh;
use unitas_physics::particles::ParticleSystem;
use unitas_physics::trails::Trail;
use unitas_physics::{
    BodyDesc, BodyRole, CharacterController, ColliderDesc, ColliderShape, CollisionDetectionMode3D,
    CollisionPayload, ForceMode, MoveResult, Phase, PhysicsMaterial3D, QueryHit, Shape, SolverType,
    World,
};
use unitas_render::image::Image;
use unitas_render::ui_pass::{GlyphTextPainter, UiTarget, composite_ui};
use unitas_render::{Camera, DirectionalLight, Mat4, Mesh, Renderer, Vec3, cube, look_at};

use self::ui_components::UiComponents;
use crate::animation::{AnimationRuntimeDiagnostic, AnimationWorld, apply_transform_property};
use crate::canvas::UiDrawItem;
use crate::error::{RenderError, RuntimeError};
use crate::input::{AxisConfig, InputState, default_axis_configs};
use crate::navigation::NavigationRuntimeWorld;
use crate::particles::{ParticleDrawItem, ParticleRuntimeWorld, ParticleRuntimeWorld2D};
use crate::physics::{PhysicsRuntimeWorld, layer_bits};
use crate::physics_settings::{
    collision_detection_of, interpolation_of, layer_matrix_of, material_of, solver_settings_of,
};
use crate::presenter::{GpuCameraBinding, scene_gpu_meshes};
use crate::terrain::TerrainWorld;
use crate::trails::{TrailDrawItem, TrailRuntimeWorld};

#[path = "ui_components.rs"]
pub(crate) mod ui_components;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryRecord {
    pub entity: u64,
    pub distance: f64,
    pub point: [f64; 3],
    pub normal: [f64; 3],
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

pub const DEFAULT_MATERIAL_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

/// The screen size the interface falls back to when no canvas scaler states a
/// reference resolution.
pub const DEFAULT_UI_REFERENCE_RESOLUTION: [f32; 2] = [800.0, 600.0];

pub const DEFAULT_MAXIMUM_DELTA_TIME: f64 = 1.0 / 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderCameraParams {
    pub orthographic: bool,
    pub orthographic_size: f32,
    pub field_of_view: f32,
    pub near_clip_plane: f32,
    pub far_clip_plane: f32,
    pub background_color: [f32; 3],
}

impl Default for RenderCameraParams {
    fn default() -> Self {
        Self {
            orthographic: false,
            orthographic_size: 5.0,
            field_of_view: 60.0,
            near_clip_plane: 0.3,
            far_clip_plane: 1000.0,
            background_color: [0.0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderMaterial {
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderCamera {
    pub entity: Option<EntityId>,
    pub params: RenderCameraParams,
    pub viewport: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderRenderer {
    pub entity: Option<EntityId>,
    pub mesh: Option<u64>,
    pub material: Option<u64>,
    pub enabled: bool,
}

/// One scene renderer a presenter can draw: the mesh it draws, the material
/// colour it uses and the entity it belongs to.
#[derive(Clone, Copy, Debug)]
pub struct SceneRenderable<'a> {
    pub entity: EntityId,
    pub mesh: &'a Mesh,
    pub color: [f32; 3],
}

/// The scene camera a presenter can draw with: the entity it follows and the
/// projection and background parameters.
#[derive(Clone, Copy, Debug)]
pub struct SceneCamera {
    pub entity: Option<EntityId>,
    pub params: RenderCameraParams,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderLight {
    pub entity: Option<EntityId>,
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Clone, Copy, Debug)]
struct CameraBasis {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    view: Mat4,
}

#[derive(Debug)]
pub struct RenderWorld {
    meshes: HashMap<u64, Mesh>,
    materials: HashMap<u64, RenderMaterial>,
    cameras: HashMap<u64, RenderCamera>,
    renderers: HashMap<u64, RenderRenderer>,
    lights: HashMap<u64, RenderLight>,
    camera_defaults: HashMap<EntityId, RenderCameraParams>,
    light_defaults: HashMap<EntityId, RenderLight>,
    effect_mesh: Mesh,
    next_handle: u64,
}

impl Default for RenderWorld {
    fn default() -> Self {
        Self {
            meshes: HashMap::new(),
            materials: HashMap::new(),
            cameras: HashMap::new(),
            renderers: HashMap::new(),
            lights: HashMap::new(),
            camera_defaults: HashMap::new(),
            light_defaults: HashMap::new(),
            effect_mesh: cube(1.0),
            next_handle: 0,
        }
    }
}

impl RenderWorld {
    fn allocate(&mut self) -> u64 {
        self.next_handle = self.next_handle.saturating_add(1);
        self.next_handle
    }
}

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
    pub(crate) quality_settings: QualitySettings,
    pub(crate) collision_payloads: HashMap<(u64, u64), CollisionPayload>,
    pub(crate) callbacks: Option<Callbacks>,
    pub(crate) fixed_timestep: f64,
    pub(crate) maximum_delta_time: f64,
    pub(crate) time_scale: f64,
    pub(crate) accumulator: f64,
    pub(crate) first_tick: bool,
    pub(crate) is_shutdown: bool,
    pub(crate) destroy_handle: Option<u64>,
    pub(crate) destroy_channel: Option<DestroyChannel>,
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
            quality_settings: QualitySettings::default(),
            collision_payloads: HashMap::new(),
            callbacks: None,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
            maximum_delta_time: DEFAULT_MAXIMUM_DELTA_TIME,
            time_scale: 1.0,
            accumulator: 0.0,
            first_tick: true,
            is_shutdown: false,
            destroy_handle: None,
            destroy_channel: None,
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
        let graph = assembled.graph;
        let bindings = assembled.scripts;
        let physics = assembled.physics;
        let audio_bindings = assembled.audio_bindings;
        let render_binding = assembled.render;
        let particle_bindings = assembled.particles;
        let trail_bindings = assembled.trails;
        let mut ui = assembled.ui;
        let settings = discover_project_settings(path);
        let scripts = build_script_instances(&bindings)?;
        let (descs, collider_diagnostics) = body_descs(&graph, &physics);
        validate_bodies(&descs)?;
        match mode {
            LoadSceneMode::Single => {
                self.unload_all_scenes(&mut hook);
                self.apply_project_settings(&settings);
            }
            LoadSceneMode::Additive => {
                if self.scenes.is_empty() {
                    self.apply_project_settings(&settings);
                }
            }
        }
        let entity_base = self.next_entity_base();
        let mut graph = graph;
        graph.shift_entity_ids(entity_base);
        // The UI binding carries the same object ids as the scene graph, so it
        // is shifted the same way before the canvas is built from it.
        ui.shift_entity_ids(entity_base);
        let ui = UiComponents::new(ui);
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
        let body_offsets = descs
            .iter()
            .map(|(entity, spec)| (EntityId::new(entity_base + entity.as_u64()), spec.offset))
            .collect::<Vec<_>>();
        let body_colliders = descs
            .iter()
            .map(|(entity, spec)| {
                (
                    EntityId::new(entity_base + entity.as_u64()),
                    spec.colliders.clone(),
                )
            })
            .collect::<Vec<_>>();
        let character_controllers = collect_character_controllers(&physics, entity_base);
        for (entity, config) in &character_controllers {
            self.character_states
                .insert(*entity, controller_from_component(Some(*entity), config));
        }
        let mut diagnostics = collect_material_diagnostics(&physics, entity_base);
        diagnostics.extend(
            collider_diagnostics
                .into_iter()
                .map(|(entity, text)| (EntityId::new(entity_base + entity.as_u64()), text)),
        );
        diagnostics.extend(self.populate_render_world(&render_binding, entity_base, &graph));
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
        let rigidbodies = physics
            .iter()
            .filter_map(|binding| {
                binding.rigidbody.as_ref().map(|rigidbody| {
                    (
                        EntityId::new(entity_base + binding.entity.as_u64()),
                        rigidbody.clone(),
                    )
                })
            })
            .collect::<HashMap<EntityId, RigidbodyComponent>>();
        for (body, (_, spec)) in bodies.iter().zip(descs.iter()) {
            self.physics.world_3d_mut().add_body_with_material(
                *body,
                spec.desc.clone(),
                spec.material,
            )?;
            // The body's own shape already is the first Unity collider, so
            // only the remaining ones become extra colliders. Adding the
            // first one again would put a second copy of it in the world.
            for collider in spec.colliders.iter().skip(1) {
                self.physics
                    .world_3d_mut()
                    .add_collider(*body, collider.clone())?;
            }
            if let Some(rigidbody) = rigidbodies.get(body) {
                let world = self.physics.world_3d_mut();
                world.set_body_drag(*body, rigidbody.drag, rigidbody.angular_drag);
                world.set_body_interpolation(*body, interpolation_of(rigidbody.interpolation));
                world.set_body_layer_overrides(
                    *body,
                    layer_override_mask(rigidbody.include_layers),
                    layer_override_mask(rigidbody.exclude_layers),
                    rigidbody.layer_override_priority,
                );
            }
        }
        let mut particle_entities = Vec::with_capacity(particle_bindings.len());
        for particle in &particle_bindings {
            let entity = EntityId::new(entity_base + particle.entity.as_u64());
            let system = self.register_particle_system(
                entity,
                particle_seed(particle.entity),
                &particle.binding,
            );
            system.play();
            particle_entities.push(entity);
        }
        let mut trail_entities = Vec::with_capacity(trail_bindings.len());
        for trail in &trail_bindings {
            let entity = EntityId::new(entity_base + trail.entity.as_u64());
            self.register_trail(entity, &trail.binding);
            trail_entities.push(entity);
        }
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
        self.quality_settings = settings.quality.clone();
        if let Some(physics) = settings.physics.as_ref() {
            let matrix = layer_matrix_of(physics);
            self.physics.set_layer_collision_matrix_3d(&matrix);
            let solver = solver_settings_of(physics);
            self.physics.set_solver_settings_3d(solver);
        }
    }

    pub fn physics_project_settings(&self) -> Option<&PhysicsProjectSettings> {
        self.physics_settings.as_ref()
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
        let entities = self.animation.entities();
        for entity in entities {
            self.write_back_animator_transform(entity);
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

    fn write_back_animator_transform(&mut self, entity: EntityId) {
        let Some(anim_transform) = self.animation.transform(entity).cloned() else {
            return;
        };
        for scene in &mut self.scenes {
            if let Some(graph_transform) = scene.graph.transform_mut(entity) {
                graph_transform.translation = anim_transform.translation;
                graph_transform.rotation = anim_transform.rotation;
                graph_transform.scale = anim_transform.scale;
                return;
            }
        }
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
        const EPSILON: f64 = 1e-9;
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
        self.input.begin_frame(scaled_delta);
        self.accumulator += scaled_delta;
        if self.accumulator > self.maximum_delta_time {
            self.accumulator = self.maximum_delta_time;
        }
        let fixed_step = self.fixed_timestep;
        while self.accumulator >= fixed_step - EPSILON {
            self.sync_kinematic_transforms();
            self.physics.world_3d_mut().step(fixed_step)?;
            self.write_back_dynamic_transforms();
            let contacts_2d = self.physics.step_2d_contacts(fixed_step as f32);
            let events = self.physics.world_3d_mut().drain_events();
            let trigger_events = self.physics.world_3d_mut().drain_trigger_events();
            if let Some(callbacks) = self.callbacks {
                let handles = self.all_script_handles();
                for handle in handles {
                    hook(self, &mut || {
                        (callbacks.fixed_update)(handle, fixed_step);
                    });
                }
                // A step's physics messages are dispatched before the next step
                // runs, so the message order does not depend on how many fixed
                // steps a rendered frame happens to batch.
                for event in events {
                    let a = event.a;
                    let b = event.b;
                    let payload = self.physics.world_3d().collision_payload(a, b).cloned();
                    if let Some(payload) = payload {
                        self.collision_payloads
                            .insert(ordered_pair(a.as_u64(), b.as_u64()), payload);
                    }
                    let a = a.as_u64();
                    let b = b.as_u64();
                    let phase = phase_code(event.phase);
                    hook(self, &mut || {
                        (callbacks.collision)(a, b, phase);
                    });
                }
                for event in trigger_events {
                    let has_rigidbody = self.entity_belongs_to_rigid_body(event.a)
                        || self.entity_belongs_to_rigid_body(event.b);
                    if !has_rigidbody {
                        continue;
                    }
                    let a = event.a.as_u64();
                    let b = event.b.as_u64();
                    let phase = phase_code(event.phase);
                    hook(self, &mut || {
                        (callbacks.trigger)(a, b, phase);
                    });
                }
                for contact in contacts_2d {
                    let a = contact.a.as_u64();
                    let b = contact.b.as_u64();
                    let phase = phase_code(contact.phase);
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
            self.accumulator -= fixed_step;
            if self.accumulator < 0.0 {
                self.accumulator = 0.0;
            }
        }
        if self.pending_scripts {
            if let Some(callbacks) = self.callbacks {
                self.init_pending_scripts(callbacks, &mut hook);
            }
            self.refresh_script_view();
        }
        let handles = self.all_script_handles();
        if let Some(callbacks) = self.callbacks {
            for handle in &handles {
                let h = *handle;
                hook(self, &mut || {
                    (callbacks.update)(h, scaled_delta);
                });
            }
        }
        self.step_animators(scaled_delta);
        self.particles
            .step_all(scaled_delta as f32, Some(self.physics.world_3d()));
        self.particles_2d
            .step_all(scaled_delta as f32, Some(self.physics.world_2d()));
        self.step_trails(scaled_delta as f32);
        if scaled_delta >= 0.0 {
            let _ = self.navigation.step(scaled_delta);
        }
        self.terrain.apply_heightfields(self.physics.world_3d_mut());
        self.sync_audio_source_positions();
        self.audio.tick(delta_seconds, self.time_scale);
        self.route_ui_pointer();
        if let Some(callbacks) = self.callbacks {
            let late_handles = self.all_script_handles();
            for handle in &late_handles {
                let h = *handle;
                hook(self, &mut || {
                    (callbacks.late_update)(h, scaled_delta);
                });
            }
        }
        if self.pending_scripts {
            if let Some(callbacks) = self.callbacks {
                self.init_pending_scripts(callbacks, &mut hook);
            }
            self.refresh_script_view();
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
        self.physics.world_3d().position(entity)
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

    pub fn input_axis_event(&mut self, name: &str, value: f64) -> bool {
        self.input.inject_axis(name, value)
    }

    pub fn input_injected_axis(&self, name: &str) -> Option<f64> {
        self.input.injected_axis(name)
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

    /// The screen size the interface is laid out and hit-tested against: the
    /// size the host sets for the live window, else the size of the last
    /// rendered frame, else the reference resolution of the canvas scaler.
    pub fn ui_screen_size(&self) -> [f32; 2] {
        if let Some(size) = self.ui_screen_size {
            return size;
        }
        if let Some((width, height)) = self.render_viewport() {
            return [width as f32, height as f32];
        }
        self.ui_reference_resolution()
    }

    /// Sets the size of the live screen the interface is laid out against.
    pub fn set_ui_screen_size(&mut self, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.ui_screen_size = Some([width, height]);
        }
    }

    /// The draw items of the visible canvas content of the loaded scenes.
    pub fn ui_draw_items(&self) -> Vec<UiDrawItem> {
        self.ui_draw_items_for(self.ui_screen_size())
    }

    /// The draw items of the visible canvas content, laid out against the
    /// given screen size.
    pub(crate) fn ui_draw_items_for(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        let mut items = Vec::new();
        for scene in &self.scenes {
            items.extend(scene.ui.draw_items(screen_size));
        }
        items
    }

    /// Whether the pointer is over an element of the interface.
    pub fn ui_pointer_over(&self) -> bool {
        let screen_size = self.ui_screen_size();
        let (x, y) = self.input.pointer_frame().position;
        self.scenes
            .iter()
            .rev()
            .any(|scene| scene.ui.pointer_over(screen_size, (x, y)))
    }

    /// Replaces the content of the `Text` component of an object.
    pub fn set_ui_text(&mut self, object_id: u64, text: &str) -> bool {
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_text(entity, text))
    }

    /// Replaces the font size and colour of the `Text` component of an object.
    pub fn set_ui_text_style(&mut self, object_id: u64, font_size: f32, color: [f32; 4]) -> bool {
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_style(entity, font_size, color))
    }

    /// The objects whose buttons were clicked since the last call.
    pub fn take_ui_clicks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_ui_clicks)
    }

    fn render_viewport(&self) -> Option<(u32, u32)> {
        self.render
            .cameras
            .values()
            .find_map(|camera| camera.viewport)
    }

    fn ui_reference_resolution(&self) -> [f32; 2] {
        for scene in &self.scenes {
            for canvas in &scene.ui.canvas().binding().canvases {
                if let Some(scaler) = &canvas.scaler {
                    return scaler.reference_resolution;
                }
            }
        }
        DEFAULT_UI_REFERENCE_RESOLUTION
    }

    /// Routes the pointer to the buttons of the loaded scenes: a button that is
    /// pressed and released under the pointer is clicked.
    fn route_ui_pointer(&mut self) {
        let pointer = self.input.pointer_frame();
        let screen_size = self.ui_screen_size();
        let hovered = self
            .scenes
            .iter()
            .rev()
            .find_map(|scene| scene.ui.hit_button(screen_size, pointer.position));
        if pointer.primary_pressed {
            self.ui_pressed_button = hovered;
        }
        if self.ui_pressed_button.is_some() && self.ui_pressed_button != hovered {
            self.ui_pressed_button = None;
        }
        if pointer.primary_released {
            if let Some(pressed) = self.ui_pressed_button
                && hovered == Some(pressed)
            {
                self.pending_ui_clicks.push(pressed.as_u64());
            }
            self.ui_pressed_button = None;
        }
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
        quality,
    }
}

struct DiscoveredProject {
    fixed_timestep: f64,
    gravity: [f64; 3],
    axes: Vec<AxisConfig>,
    identity: ProjectIdentity,
    physics: Option<PhysicsProjectSettings>,
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

struct BodySpec {
    desc: BodyDesc,
    colliders: Vec<ColliderDesc>,
    offset: [f32; 3],
    material: Option<PhysicsMaterial3D>,
}

type BodySpecs = Vec<(EntityId, BodySpec)>;
type ColliderDiagnostics = Vec<(EntityId, String)>;

fn body_descs(graph: &SceneGraph, bindings: &[PhysicsBinding]) -> (BodySpecs, ColliderDiagnostics) {
    let rigid_entities: std::collections::BTreeSet<EntityId> = bindings
        .iter()
        .filter(|binding| binding.rigidbody.is_some())
        .map(|binding| binding.entity)
        .collect();
    let mut descs = Vec::new();
    let mut diagnostics = Vec::new();
    for binding in bindings {
        let spec = match body_spec(graph, binding, &rigid_entities, &mut diagnostics) {
            Some(spec) => spec,
            None => continue,
        };
        descs.push((binding.entity, spec));
    }
    (descs, diagnostics)
}

fn validate_bodies(descs: &[(EntityId, BodySpec)]) -> Result<(), RuntimeError> {
    let mut world = World::new();
    for (entity, spec) in descs {
        world.add_body(*entity, spec.desc.clone())?;
        for collider in spec.colliders.iter().skip(1) {
            world.add_collider(*entity, collider.clone())?;
        }
    }
    Ok(())
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

fn layer_override_mask(mask: Option<u32>) -> Option<u32> {
    mask.filter(|bits| *bits != 0)
}

fn collect_character_controllers(
    bindings: &[PhysicsBinding],
    entity_base: u64,
) -> Vec<(EntityId, CharacterControllerComponent)> {
    let mut controllers = Vec::new();
    for binding in bindings {
        if let Some(controller) = &binding.character_controller {
            let entity = EntityId::new(entity_base + binding.entity.as_u64());
            controllers.push((entity, controller.clone()));
        }
    }
    controllers
}

fn controller_from_component(
    entity: Option<EntityId>,
    component: &CharacterControllerComponent,
) -> CharacterController {
    let mut controller = CharacterController::default();
    controller.radius = component.radius;
    controller.height = component.height;
    controller.center = component.center;
    controller.slope_limit_deg = component.slope_limit;
    controller.step_offset = component.step_offset;
    controller.skin_width = component.skin_width;
    controller.min_move_distance = if component.min_move_distance > 0.0 {
        component.min_move_distance
    } else {
        0.001
    };
    controller.layer_mask = u32::MAX;
    controller.enabled = component.enabled;
    controller.excluded_entity = entity;
    if controller.validate().is_err() {
        controller = CharacterController::default();
        controller.excluded_entity = entity;
    }
    controller
}

fn collect_material_diagnostics(
    bindings: &[PhysicsBinding],
    entity_base: u64,
) -> Vec<(EntityId, String)> {
    let mut diagnostics = Vec::new();
    for binding in bindings {
        let entity = EntityId::new(entity_base + binding.entity.as_u64());
        for collider in &binding.colliders {
            let (material_diagnostic, geometry_diagnostic) = match collider {
                ColliderComponent::Box(component) => (component.material_diagnostic.as_ref(), None),
                ColliderComponent::Sphere(component) => {
                    (component.material_diagnostic.as_ref(), None)
                }
                ColliderComponent::Capsule(component) => (
                    component.material_diagnostic.as_ref(),
                    component.geometry_diagnostic.as_ref(),
                ),
                ColliderComponent::Mesh(component) => {
                    (component.material_diagnostic.as_ref(), None)
                }
            };
            if let Some(diag) = material_diagnostic {
                diagnostics.push((entity, diag.clone()));
            }
            if let Some(diag) = geometry_diagnostic {
                diagnostics.push((entity, diag.clone()));
            }
        }
    }
    diagnostics
}

fn has_rigidbody_ancestor(
    graph: &SceneGraph,
    entity: EntityId,
    rigid_entities: &std::collections::BTreeSet<EntityId>,
) -> bool {
    let mut current = graph.parent(entity);
    while let Some(ancestor) = current {
        if rigid_entities.contains(&ancestor) {
            return true;
        }
        current = graph.parent(ancestor);
    }
    false
}

/// One Unity collider component, resolved into the shape the world needs.
struct ColliderPlan {
    shape: ColliderShape,
    center: [f64; 3],
    is_trigger: bool,
    material: Option<PhysicsMaterial3D>,
}

/// The colliders of one GameObject, in the order the scene lists them. A
/// collider whose mesh cannot be cooked is dropped with a diagnostic.
fn collider_plans(
    graph: &SceneGraph,
    binding: &PhysicsBinding,
    scale: [f32; 3],
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Vec<ColliderPlan> {
    let mut plans = Vec::new();
    for collider in &binding.colliders {
        let plan = match collider {
            ColliderComponent::Box(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Box {
                    half_extents: [
                        component.size[0] / 2.0,
                        component.size[1] / 2.0,
                        component.size[2] / 2.0,
                    ],
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Sphere(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Sphere {
                    radius: component.radius,
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Capsule(component) => ColliderPlan {
                shape: ColliderShape::Primitive(Shape::Capsule {
                    radius: component.radius,
                    height: component.height,
                    direction: component.direction as u8,
                }),
                center: component.center,
                is_trigger: component.is_trigger,
                material: component.material.as_ref().map(material_of),
            },
            ColliderComponent::Mesh(component) => {
                match mesh_shape(graph, binding.entity, component, scale, diagnostics) {
                    Some(shape) => ColliderPlan {
                        shape,
                        center: component.center,
                        is_trigger: component.is_trigger,
                        material: component.material.as_ref().map(material_of),
                    },
                    None => continue,
                }
            }
        };
        plans.push(plan);
    }
    plans
}

/// Cook the geometry a mesh collider names: Unity's built-in primitive mesh
/// by identity, otherwise the referenced asset file.
fn mesh_shape(
    graph: &SceneGraph,
    entity: EntityId,
    component: &MeshColliderComponent,
    scale: [f32; 3],
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Option<ColliderShape> {
    let cooked = match &component.mesh_path {
        Some(path) => match cook_file_mesh(path, component.mesh_file_id) {
            Ok(mesh) => Some(mesh),
            Err(reason) => {
                diagnostics.push((
                    entity,
                    mesh_diagnostic_text(graph, entity, component, Some(&reason)),
                ));
                return None;
            }
        },
        None => component
            .mesh_file_id
            .zip(component.mesh_guid.as_deref())
            .and_then(|(file_id, guid)| cook_builtin_collision_mesh(file_id, guid)),
    };
    let mesh = match cooked {
        Some(mesh) => mesh,
        None => {
            diagnostics.push((entity, mesh_diagnostic_text(graph, entity, component, None)));
            return None;
        }
    };
    Some(ColliderShape::Mesh {
        vertices: mesh
            .vertices
            .iter()
            .map(|vertex| {
                [
                    f64::from(vertex[0]) * f64::from(scale[0]),
                    f64::from(vertex[1]) * f64::from(scale[1]),
                    f64::from(vertex[2]) * f64::from(scale[2]),
                ]
            })
            .collect(),
        indices: mesh.indices,
        convex: component.convex,
    })
}

fn cook_file_mesh(path: &Path, file_id: Option<i64>) -> Result<CollisionMesh, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    cook_collision_mesh(&bytes, file_name, file_id).map_err(|error| error.to_string())
}

/// The diagnostic a mesh collider that names no cookable geometry produces.
/// It always names the object and the asset it could not resolve.
fn mesh_diagnostic_text(
    graph: &SceneGraph,
    entity: EntityId,
    component: &MeshColliderComponent,
    reason: Option<&str>,
) -> String {
    if let Some(diagnostic) = &component.mesh_diagnostic {
        return diagnostic.clone();
    }
    let name = graph
        .entity(entity)
        .map_or("?", |entity| entity.name.as_str());
    let owner = format!("MeshCollider on '{name}' (entity {})", entity.as_u64());
    match (&component.mesh_path, component.mesh_guid.as_deref(), reason) {
        (Some(path), _, Some(reason)) => {
            format!(
                "{owner} references mesh '{}' that could not be cooked: {reason}",
                path.display()
            )
        }
        (Some(path), _, None) => {
            format!(
                "{owner} references mesh '{}' that could not be cooked",
                path.display()
            )
        }
        (None, Some(guid), Some(reason)) => {
            format!("{owner} references mesh guid {guid} that could not be cooked: {reason}")
        }
        (None, Some(guid), None) => {
            format!("{owner} references mesh guid {guid} that could not be cooked")
        }
        (None, None, _) => format!("{owner} has no mesh reference"),
    }
}

/// The offset of a collider from the body, in the body's own frame. The body
/// sits at the centre of the first collider, so the first collider's offset is
/// the origin and every other collider keeps its distance from it.
fn collider_offset(center: [f64; 3], first_center: [f64; 3], scale: [f32; 3]) -> [f64; 3] {
    [
        ((center[0] as f32 - first_center[0] as f32) * scale[0]) as f64,
        ((center[1] as f32 - first_center[1] as f32) * scale[1]) as f64,
        ((center[2] as f32 - first_center[2] as f32) * scale[2]) as f64,
    ]
}

fn body_spec(
    graph: &SceneGraph,
    binding: &PhysicsBinding,
    rigid_entities: &std::collections::BTreeSet<EntityId>,
    diagnostics: &mut Vec<(EntityId, String)>,
) -> Option<BodySpec> {
    let (translation, rotation, scale) = match graph.world_transform(binding.entity) {
        Some(world) => (world.translation, world.rotation, world.scale),
        None => ([0.0f32; 3], IDENTITY_ROTATION, [1.0f32; 3]),
    };
    let plans = collider_plans(graph, binding, scale, diagnostics);
    let first_center = plans.first()?.center;
    let (mass, use_gravity, rigidbody_trigger, role) = match &binding.rigidbody {
        Some(rigidbody) if rigidbody.is_kinematic => (
            rigidbody.mass,
            rigidbody.use_gravity,
            rigidbody.is_trigger,
            BodyRole::Kinematic,
        ),
        Some(rigidbody) => (
            rigidbody.mass,
            rigidbody.use_gravity,
            rigidbody.is_trigger,
            BodyRole::Dynamic,
        ),
        None if has_rigidbody_ancestor(graph, binding.entity, rigid_entities) => {
            (0.0, false, false, BodyRole::Kinematic)
        }
        None => (0.0, false, false, BodyRole::Static),
    };
    let collision_detection_mode = binding
        .rigidbody
        .as_ref()
        .map_or(CollisionDetectionMode3D::Discrete, |rigidbody| {
            collision_detection_of(rigidbody.collision_detection)
        });
    let layer = layer_bits(binding.layer);
    let colliders: Vec<ColliderDesc> = plans
        .into_iter()
        .enumerate()
        .map(|(index, plan)| ColliderDesc {
            shape: plan.shape,
            offset: if index == 0 {
                [0.0; 3]
            } else {
                collider_offset(plan.center, first_center, scale)
            },
            rotation: [0.0, 0.0, 0.0, 1.0],
            is_trigger: plan.is_trigger || rigidbody_trigger,
            material: plan.material,
            layer,
        })
        .collect();
    let first = colliders.first()?;
    let material = first.material;
    let friction = material.map_or(0.6, |m| m.friction);
    let restitution = material.map_or(0.0, |m| m.restitution);
    let scaled_center = [
        first_center[0] as f32 * scale[0],
        first_center[1] as f32 * scale[1],
        first_center[2] as f32 * scale[2],
    ];
    let offset = rotate_vector(rotation, scaled_center);
    Some(BodySpec {
        desc: BodyDesc {
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
            is_trigger: first.is_trigger,
            collider_shape: first.shape.clone(),
            layer,
            role,
            friction,
            restitution,
            collision_detection_mode,
        },
        colliders,
        offset,
        material,
    })
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

fn normalized(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some([vector[0] / length, vector[1] / length, vector[2] / length])
}

fn rotations_equivalent(a: [f64; 4], b: [f64; 4]) -> bool {
    let a = normalize_rotation([a[0] as f32, a[1] as f32, a[2] as f32, a[3] as f32]);
    let b = normalize_rotation([b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32]);
    let dot = f64::from(a[0]) * f64::from(b[0])
        + f64::from(a[1]) * f64::from(b[1])
        + f64::from(a[2]) * f64::from(b[2])
        + f64::from(a[3]) * f64::from(b[3]);
    (1.0 - dot.abs()) <= 1e-6
}

fn ordered_pair(first: u64, second: u64) -> (u64, u64) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn phase_code(phase: Phase) -> u32 {
    match phase {
        Phase::Enter => 0,
        Phase::Stay => 1,
        Phase::Exit => 2,
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

fn toward_light_direction(rotation: [f32; 4], travel: [f32; 3]) -> [f32; 3] {
    let rotated = rotate_vector(rotation, travel);
    [-rotated[0], -rotated[1], -rotated[2]]
}

pub(crate) fn projection_for(params: &RenderCameraParams, aspect: f32) -> Camera {
    let near = params.near_clip_plane.max(0.001);
    let far = params.far_clip_plane.max(near + 0.001);
    if params.orthographic {
        Camera::orthographic(params.orthographic_size.max(0.001), aspect, near, far)
    } else {
        Camera::perspective(params.field_of_view.max(1.0), aspect, near, far)
    }
}

impl Runtime {
    pub(crate) fn create_render_mesh(
        &mut self,
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> u64 {
        let handle = self.render.allocate();
        self.render.meshes.insert(
            handle,
            Mesh {
                positions,
                normals,
                indices,
            },
        );
        handle
    }

    pub(crate) fn destroy_render_mesh(&mut self, handle: u64) -> bool {
        self.render.meshes.remove(&handle).is_some()
    }

    pub(crate) fn create_render_material(&mut self) -> u64 {
        let handle = self.render.allocate();
        self.render.materials.insert(
            handle,
            RenderMaterial {
                color: DEFAULT_MATERIAL_COLOR,
            },
        );
        handle
    }

    pub(crate) fn set_render_material_color(&mut self, handle: u64, color: [f32; 4]) -> bool {
        match self.render.materials.get_mut(&handle) {
            Some(material) => {
                material.color = color;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_camera(&mut self, entity: Option<EntityId>) -> u64 {
        let handle = self.render.allocate();
        let params = entity
            .and_then(|entity| self.render.camera_defaults.get(&entity).copied())
            .unwrap_or_default();
        self.render.cameras.insert(
            handle,
            RenderCamera {
                entity,
                params,
                viewport: None,
            },
        );
        handle
    }

    pub(crate) fn update_render_camera(&mut self, handle: u64, params: RenderCameraParams) -> bool {
        match self.render.cameras.get_mut(&handle) {
            Some(camera) => {
                camera.params = params;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_renderer(
        &mut self,
        entity: Option<EntityId>,
        mesh: Option<u64>,
        material: Option<u64>,
    ) -> u64 {
        let handle = self.render.allocate();
        self.render.renderers.insert(
            handle,
            RenderRenderer {
                entity,
                mesh,
                material,
                enabled: true,
            },
        );
        handle
    }

    pub(crate) fn set_render_renderer_enabled(&mut self, handle: u64, enabled: bool) -> bool {
        match self.render.renderers.get_mut(&handle) {
            Some(renderer) => {
                renderer.enabled = enabled;
                true
            }
            None => false,
        }
    }

    pub(crate) fn create_render_light(&mut self, entity: Option<EntityId>) -> u64 {
        let handle = self.render.allocate();
        let mut light = entity
            .and_then(|entity| self.render.light_defaults.get(&entity).copied())
            .unwrap_or(RenderLight {
                entity,
                direction: [0.0, 1.0, 0.0],
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            });
        light.entity = entity;
        self.render.lights.insert(handle, light);
        handle
    }

    pub(crate) fn update_render_light(
        &mut self,
        handle: u64,
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
    ) -> bool {
        match self.render.lights.get_mut(&handle) {
            Some(light) => {
                light.direction = direction;
                light.color = color;
                light.intensity = intensity;
                true
            }
            None => false,
        }
    }

    pub(crate) fn entity_transform(&self, entity: EntityId) -> Option<Transform> {
        for scene in &self.scenes {
            if let Some(transform) = scene.graph.world_transform(entity) {
                return Some(transform);
            }
        }
        None
    }

    fn populate_render_world(
        &mut self,
        binding: &RenderBinding,
        entity_base: u64,
        graph: &SceneGraph,
    ) -> Vec<(EntityId, String)> {
        let mut diagnostics = Vec::new();
        for camera in &binding.cameras {
            let entity = EntityId::new(entity_base + camera.entity.as_u64());
            let params = RenderCameraParams {
                orthographic: camera.orthographic,
                orthographic_size: camera.orthographic_size,
                field_of_view: camera.field_of_view,
                near_clip_plane: camera.near_clip_plane,
                far_clip_plane: camera.far_clip_plane,
                background_color: camera.background_color,
            };
            self.render.camera_defaults.insert(entity, params);
            let handle = self.create_render_camera(Some(entity));
            self.update_render_camera(handle, params);
        }
        for renderer in &binding.renderers {
            let entity = EntityId::new(entity_base + renderer.entity.as_u64());
            let material = self.create_render_material();
            self.set_render_material_color(
                material,
                [
                    renderer.material_color[0],
                    renderer.material_color[1],
                    renderer.material_color[2],
                    1.0,
                ],
            );
            let mesh = self.import_scene_mesh(renderer);
            let handle = self.create_render_renderer(Some(entity), mesh, Some(material));
            self.set_render_renderer_enabled(handle, renderer.enabled);
            if mesh.is_none() {
                if let Some(diagnostic) = &renderer.mesh_diagnostic {
                    diagnostics.push((entity, diagnostic.clone()));
                } else if let Some(guid) = &renderer.mesh_guid {
                    diagnostics.push((
                        entity,
                        format!(
                            "MeshRenderer on entity {} references mesh guid {} that could not be imported",
                            entity.as_u64(),
                            guid
                        ),
                    ));
                }
            }
            if let Some(diagnostic) = &renderer.material_diagnostic {
                diagnostics.push((entity, diagnostic.clone()));
            } else if let Some(guid) = &renderer.material_guid {
                diagnostics.push((
                    entity,
                    format!(
                        "MeshRenderer on entity {} references material guid {} that could not be imported",
                        entity.as_u64(),
                        guid
                    ),
                ));
            }
        }
        for light in &binding.lights {
            let entity = EntityId::new(entity_base + light.entity.as_u64());
            if !light.directional {
                if let Some(diagnostic) = &light.unsupported_diagnostic {
                    diagnostics.push((entity, diagnostic.clone()));
                }
                continue;
            }
            let rotation = graph
                .world_transform(entity)
                .map(|transform| transform.rotation)
                .unwrap_or(IDENTITY_ROTATION);
            let direction = toward_light_direction(rotation, light.direction);
            self.render.light_defaults.insert(
                entity,
                RenderLight {
                    entity: Some(entity),
                    direction,
                    color: light.color,
                    intensity: light.intensity,
                },
            );
            let handle = self.create_render_light(Some(entity));
            self.update_render_light(handle, direction, light.color, light.intensity);
        }
        diagnostics
    }

    // Resolve a scene-declared MeshFilter reference into a render mesh:
    // Unity built-in primitives by (fileID, guid), otherwise an imported
    // model file. Returns None when the mesh cannot be resolved.
    fn import_scene_mesh(&mut self, renderer: &MeshRendererComponent) -> Option<u64> {
        let mesh_data = if let (Some(file_id), Some(guid)) =
            (renderer.mesh_file_id, renderer.mesh_guid.as_deref())
        {
            let primitive = unitas_asset::primitive::builtin_primitive_by_identity(file_id, guid)?;
            let mut data = unitas_asset::primitive::primitive_mesh(primitive);
            if primitive == unitas_asset::primitive::BuiltinPrimitive::Sphere {
                // Unity's built-in sphere is radius 0.5; unitas generates radius 1.
                for position in &mut data.positions {
                    *position = [position[0] * 0.5, position[1] * 0.5, position[2] * 0.5];
                }
            }
            Some(data)
        } else if let Some(path) = &renderer.mesh_path {
            let bytes = std::fs::read(path).ok()?;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            unitas_asset::model::import_model(&bytes, name).ok()
        } else {
            None
        }?;
        let positions = mesh_data
            .positions
            .iter()
            .map(|p| Vec3::new(p[0], p[1], p[2]))
            .collect();
        let normals = mesh_data
            .normals
            .iter()
            .map(|n| Vec3::new(n[0], n[1], n[2]))
            .collect();
        Some(self.create_render_mesh(positions, normals, mesh_data.indices.clone()))
    }

    fn remove_scene_render_handles(&mut self, scene: &LoadedScene) {
        let base = scene.entity_base;
        let count = scene.entity_count;
        let in_scene = |entity: Option<EntityId>| match entity {
            Some(entity) => {
                let value = entity.as_u64();
                value >= base && value < base + count
            }
            None => false,
        };
        self.render
            .cameras
            .retain(|_, camera| !in_scene(camera.entity));
        self.render
            .renderers
            .retain(|_, renderer| !in_scene(renderer.entity));
        self.render
            .lights
            .retain(|_, light| !in_scene(light.entity));
        self.render
            .camera_defaults
            .retain(|entity, _| !in_scene(Some(*entity)));
        self.render
            .light_defaults
            .retain(|entity, _| !in_scene(Some(*entity)));
    }

    fn camera_basis(&self, entity: Option<EntityId>) -> Option<CameraBasis> {
        let entity = entity?;
        let transform = self.entity_transform(entity)?;
        let eye = Vec3::new(
            transform.translation[0],
            transform.translation[1],
            transform.translation[2],
        );
        let rotated = rotate_vector(transform.rotation, [0.0, 0.0, 1.0]);
        let mut forward = Vec3::new(rotated[0], rotated[1], rotated[2]);
        if forward.length_squared() < 1e-8 {
            forward = Vec3::new(0.0, 0.0, 1.0);
        }
        let forward = forward.normalize();
        let mut up = Vec3::new(0.0, 1.0, 0.0);
        if up.cross(forward).length_squared() < 1e-8 {
            up = Vec3::new(0.0, 0.0, 1.0);
        }
        let view = look_at(eye, eye + forward, up);
        let right = Vec3::new(view.get(0, 0), view.get(1, 0), view.get(2, 0));
        let up_axis = Vec3::new(view.get(0, 1), view.get(1, 1), view.get(2, 1));
        Some(CameraBasis {
            eye,
            right,
            up: up_axis,
            forward,
            view,
        })
    }

    /// The scene's directional light, or a default one when the scene has
    /// none.
    pub fn directional_light(&self) -> DirectionalLight {
        let mut selected: Option<(u64, RenderLight)> = None;
        for (handle, light) in &self.render.lights {
            let replace = match selected {
                Some((best, _)) => *handle > best,
                None => true,
            };
            if replace {
                selected = Some((*handle, *light));
            }
        }
        match selected {
            Some((_, light)) => DirectionalLight {
                direction: Vec3::new(light.direction[0], light.direction[1], light.direction[2]),
                color: light.color,
                intensity: light.intensity,
            },
            None => DirectionalLight {
                direction: Vec3::new(0.0, 1.0, 0.0),
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            },
        }
    }

    /// The loaded scene's enabled renderers with the mesh and material colour
    /// each one draws, in the deterministic order the renderer draws them.
    pub fn scene_renderables(&self) -> Vec<SceneRenderable<'_>> {
        // Renderer handles are sorted so the instance order, and therefore the
        // rendered image, is the same in every process: a HashMap's iteration
        // order is seeded per process, so consecutive captures of one scene
        // otherwise disagree on which coincident surface wins the depth test.
        let mut handles: Vec<u64> = self.render.renderers.keys().copied().collect();
        handles.sort_unstable();
        handles
            .iter()
            .filter_map(|handle| {
                let renderer = self.render.renderers.get(handle)?;
                if !renderer.enabled {
                    return None;
                }
                let entity = renderer.entity?;
                let mesh = renderer
                    .mesh
                    .and_then(|mesh| self.render.meshes.get(&mesh))?;
                let color = renderer
                    .material
                    .and_then(|material| self.render.materials.get(&material))
                    .map(|material| [material.color[0], material.color[1], material.color[2]])
                    .unwrap_or([
                        DEFAULT_MATERIAL_COLOR[0],
                        DEFAULT_MATERIAL_COLOR[1],
                        DEFAULT_MATERIAL_COLOR[2],
                    ]);
                Some(SceneRenderable {
                    entity,
                    mesh,
                    color,
                })
            })
            .collect()
    }

    /// The scene camera with the highest handle, so the choice does not depend
    /// on map iteration order.
    pub fn scene_camera(&self) -> Option<SceneCamera> {
        let mut selected: Option<(u64, RenderCamera)> = None;
        for (handle, camera) in &self.render.cameras {
            let replace = match selected {
                Some((best, _)) => *handle > best,
                None => true,
            };
            if replace {
                selected = Some((*handle, *camera));
            }
        }
        selected.map(|(_, camera)| SceneCamera {
            entity: camera.entity,
            params: camera.params,
        })
    }

    pub(crate) fn render_frame(
        &mut self,
        camera: u64,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        if let Some(entity) = state.entity
            && self.world_transform_of(entity).is_none()
        {
            return Err(RenderError::InvalidArgument(
                "camera entity is not in a loaded scene".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.set_meshes(scene_gpu_meshes(self));
        let default = GpuCameraBinding::default();
        presenter.set_camera_binding(GpuCameraBinding {
            entity: state.entity,
            camera: projection_for(&state.params, default.camera.aspect),
            view: default.view,
            background: state.params.background_color,
        });
        presenter.set_light(self.directional_light());
        presenter.set_shadow_settings(crate::presenter::shadow_settings_for_runtime(self));
        let image = presenter
            .render_offscreen(self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        let bytes = image.to_bmp_bytes();
        if let Some(camera_state) = self.render.cameras.get_mut(&camera) {
            camera_state.viewport = Some((width, height));
        }
        Ok(bytes)
    }

    pub(crate) fn render_id_map(
        &mut self,
        camera: u64,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        if let Some(entity) = state.entity
            && self.world_transform_of(entity).is_none()
        {
            return Err(RenderError::InvalidArgument(
                "camera entity is not in a loaded scene".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.set_meshes(scene_gpu_meshes(self));
        let default = GpuCameraBinding::default();
        presenter.set_camera_binding(GpuCameraBinding {
            entity: state.entity,
            camera: projection_for(&state.params, default.camera.aspect),
            view: default.view,
            background: state.params.background_color,
        });
        presenter.set_light(self.directional_light());
        presenter.set_shadow_settings(crate::presenter::shadow_settings_for_runtime(self));
        let frame = crate::capture::capture_offscreen(&mut presenter, self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        let mut bytes = Vec::with_capacity(frame.id_map.len() * 4);
        for id in frame.id_map {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        if let Some(camera_state) = self.render.cameras.get_mut(&camera) {
            camera_state.viewport = Some((width, height));
        }
        Ok(bytes)
    }

    pub fn capture_scene_frame(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<unitas_render::CaptureFrame, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidArgument(
                "frame size must be greater than zero".to_string(),
            ));
        }
        let renderer = self.gpu_renderer()?;
        let mut presenter = crate::presenter::GpuFramePresenter::new(renderer);
        presenter.bind_scene(self);
        crate::capture::capture_offscreen(&mut presenter, self, width, height)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))
    }

    /// Draws the canvas content of the loaded scenes onto a rendered frame, so
    /// the interface is part of the picture instead of only its state.
    pub(crate) fn composite_canvas_image(&self, image: Image, width: u32, height: u32) -> Image {
        let items = self.ui_draw_items_for([width as f32, height as f32]);
        if items.is_empty() {
            return image;
        }
        let mut rgba = image.to_rgba8();
        let mut target = UiTarget {
            image: &mut rgba,
            ids: None,
        };
        if items.iter().any(|item| item.text.is_some()) {
            let mut painter = self
                .ui_painter
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let painter = painter.get_or_insert_with(GlyphTextPainter::new);
            composite_ui(&mut target, &items, 1, Some(painter));
        } else {
            composite_ui(&mut target, &items, 1, None);
        }
        Image::from_rgba8(rgba)
    }

    /// The renderer every frame of this runtime is rendered by.
    pub(crate) fn gpu_renderer(&mut self) -> Result<Arc<Renderer>, RenderError> {
        if let Some(renderer) = &self.gpu_renderer {
            return Ok(Arc::clone(renderer));
        }
        let renderer = Renderer::new()
            .map(Arc::new)
            .map_err(|error| RenderError::DeviceFailed(error.to_string()))?;
        self.gpu_renderer = Some(Arc::clone(&renderer));
        Ok(renderer)
    }

    /// The mesh the particle and trail effects are drawn with.
    pub(crate) fn effect_mesh(&self) -> &Mesh {
        &self.render.effect_mesh
    }

    /// The name of an entity of the loaded scenes.
    pub(crate) fn entity_name(&self, entity: EntityId) -> Option<&str> {
        self.scene_graph()
            .and_then(|graph| graph.entity(entity))
            .map(|entity| entity.name())
    }

    pub(crate) fn camera_world_to_screen_point(
        &self,
        camera: u64,
        point: [f32; 3],
    ) -> Result<[f32; 3], RenderError> {
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        let (width, height) = match state.viewport {
            Some(viewport) => viewport,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera has no rendered frame".to_string(),
                ));
            }
        };
        let basis = match self.camera_basis(state.entity) {
            Some(basis) => basis,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera entity is not in a loaded scene".to_string(),
                ));
            }
        };
        let projection = projection_for(&state.params, width as f32 / height as f32);
        let clip = projection.projection_matrix() * basis.view;
        let world = Vec3::new(point[0], point[1], point[2]);
        let clip = clip.transform_point(world);
        if clip.w <= 1e-6 {
            return Err(RenderError::InvalidArgument(
                "point is behind the camera".to_string(),
            ));
        }
        let x_pixel = (clip.x / clip.w + 1.0) * 0.5 * width as f32;
        let y_pixel = (clip.y / clip.w + 1.0) * 0.5 * height as f32;
        let distance = (world - basis.eye).dot(basis.forward);
        Ok([x_pixel, y_pixel, distance])
    }

    pub(crate) fn camera_screen_point_to_ray(
        &self,
        camera: u64,
        point: [f32; 3],
    ) -> Result<[f32; 6], RenderError> {
        let state = match self.render.cameras.get(&camera) {
            Some(state) => *state,
            None => return Err(RenderError::UnknownHandle),
        };
        let (width, height) = match state.viewport {
            Some(viewport) => viewport,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera has no rendered frame".to_string(),
                ));
            }
        };
        let basis = match self.camera_basis(state.entity) {
            Some(basis) => basis,
            None => {
                return Err(RenderError::InvalidArgument(
                    "camera entity is not in a loaded scene".to_string(),
                ));
            }
        };
        let aspect = width as f32 / height as f32;
        let ndc_x = point[0] / (0.5 * width as f32) - 1.0;
        let ndc_y = point[1] / (0.5 * height as f32) - 1.0;
        let near = state.params.near_clip_plane.max(0.001);
        let (view_x, view_y) = if state.params.orthographic {
            (
                ndc_x * state.params.orthographic_size * aspect,
                ndc_y * state.params.orthographic_size,
            )
        } else {
            let half = (state.params.field_of_view.to_radians() * 0.5).tan();
            (ndc_x * near * half * aspect, ndc_y * near * half)
        };
        let direction =
            (basis.right * view_x + basis.up * view_y + basis.forward * near).normalize();
        Ok([
            basis.eye.x,
            basis.eye.y,
            basis.eye.z,
            direction.x,
            direction.y,
            direction.z,
        ])
    }
}
