use std::path::{Path, PathBuf};
use unitas_asset::scene::{assemble_scene_file_with_physics, PhysicsBinding};
use unitas_asset::{
    parse_dynamics_manager, parse_input_manager, parse_time_manager, InputAxis,
    DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY,
};
use unitas_core::{rotate_vector, EntityId, SceneGraph, IDENTITY_ROTATION};
use unitas_physics::{BodyDesc, Phase, QueryHit, Shape, World};

use crate::error::RuntimeError;
use crate::input::{default_axis_configs, AxisConfig, InputState};

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

pub struct Runtime {
    scene_graph: Option<SceneGraph>,
    scripts: Vec<ScriptInstance>,
    world: World,
    input: InputState,
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
            scene_graph: None,
            scripts: Vec::new(),
            world: World::new(),
            input: InputState::with_default_axes(),
            callbacks: None,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
            time_scale: 1.0,
            accumulator: 0.0,
            first_tick: true,
            is_shutdown: false,
        })
    }

    pub fn load_scene(&mut self, path: &Path) -> Result<usize, RuntimeError> {
        let (graph, bindings, physics) = assemble_scene_file_with_physics(path)?;
        let (fixed_timestep, gravity, axes) = discover_project_settings(path);
        let world = build_world(&graph, &physics, gravity)?;
        self.scene_graph = Some(graph);
        self.world = world;
        self.fixed_timestep = fixed_timestep;
        self.input.set_axes(axes);
        self.input.reset();
        self.scripts.clear();
        for binding in bindings {
            if binding.guid.len() != 32 {
                return Err(RuntimeError::InvalidGuid(binding.guid));
            }
            let guid_hi = match u64::from_str_radix(&binding.guid[0..16], 16) {
                Ok(val) => val,
                Err(_) => return Err(RuntimeError::InvalidGuid(binding.guid)),
            };
            let guid_lo = match u64::from_str_radix(&binding.guid[16..32], 16) {
                Ok(val) => val,
                Err(_) => return Err(RuntimeError::InvalidGuid(binding.guid)),
            };
            self.scripts.push(ScriptInstance {
                entity: binding.entity,
                guid: binding.guid,
                guid_hi,
                guid_lo,
                script_handle: 0,
                awake_called: false,
                on_enable_called: false,
                start_called: false,
            });
        }
        self.accumulator = 0.0;
        self.first_tick = true;
        self.is_shutdown = false;
        Ok(self.scripts.len())
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
                let count = self.scripts.len();
                for index in 0..count {
                    let entity = self.scripts[index].entity.as_u64();
                    let guid_hi = self.scripts[index].guid_hi;
                    let guid_lo = self.scripts[index].guid_lo;
                    let mut created = 0;
                    hook(self, &mut || {
                        created = (callbacks.create_script)(entity, guid_hi, guid_lo);
                    });
                    self.scripts[index].script_handle = created;
                }
                for index in 0..count {
                    let handle = self.scripts[index].script_handle;
                    hook(self, &mut || {
                        (callbacks.awake)(handle);
                    });
                    self.scripts[index].awake_called = true;
                }
                for index in 0..count {
                    let handle = self.scripts[index].script_handle;
                    hook(self, &mut || {
                        (callbacks.on_enable)(handle);
                    });
                    self.scripts[index].on_enable_called = true;
                }
                for index in 0..count {
                    let handle = self.scripts[index].script_handle;
                    hook(self, &mut || {
                        (callbacks.start)(handle);
                    });
                    self.scripts[index].start_called = true;
                }
            }
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
                let handles: Vec<u64> = self.scripts.iter().map(|s| s.script_handle).collect();
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
            let handles: Vec<u64> = self.scripts.iter().map(|s| s.script_handle).collect();
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
            let disable_handles: Vec<u64> = self
                .scripts
                .iter()
                .rev()
                .filter(|s| s.on_enable_called)
                .map(|s| s.script_handle)
                .collect();
            for handle in disable_handles {
                hook(self, &mut || {
                    (callbacks.on_disable)(handle);
                });
            }
            let destroy_handles: Vec<u64> = self
                .scripts
                .iter()
                .rev()
                .filter(|s| s.script_handle != 0)
                .map(|s| s.script_handle)
                .collect();
            for handle in destroy_handles {
                hook(self, &mut || {
                    (callbacks.destroy)(handle);
                });
            }
        }
        Ok(())
    }

    pub fn scripts(&self) -> &[ScriptInstance] {
        &self.scripts
    }

    pub fn scene_graph(&self) -> Option<&SceneGraph> {
        self.scene_graph.as_ref()
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

fn discover_project_settings(scene_path: &Path) -> (f64, [f64; 3], Vec<AxisConfig>) {
    let Some(settings_dir) = find_project_settings_dir(scene_path) else {
        return (
            DEFAULT_FIXED_TIMESTEP,
            DEFAULT_GRAVITY,
            default_axis_configs(),
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
    (fixed_timestep, gravity, axes)
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

fn build_world(
    graph: &SceneGraph,
    bindings: &[PhysicsBinding],
    gravity: [f64; 3],
) -> Result<World, RuntimeError> {
    let mut world = World::with_gravity(gravity);
    for binding in bindings {
        let desc = match body_desc(graph, binding) {
            Some(desc) => desc,
            None => continue,
        };
        world.add_body(binding.entity, desc)?;
    }
    Ok(world)
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
