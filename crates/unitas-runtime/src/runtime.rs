use std::path::{Path, PathBuf};
use unitas_asset::scene::{assemble_scene_file_with_physics, PhysicsBinding};
use unitas_asset::{
    parse_dynamics_manager, parse_time_manager, DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY,
};
use unitas_core::{rotate_vector, EntityId, SceneGraph, IDENTITY_ROTATION};
use unitas_physics::{BodyDesc, Phase, QueryHit, Shape, World};

use crate::error::RuntimeError;

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
        let (fixed_timestep, gravity) = discover_project_settings(path);
        let world = build_world(&graph, &physics, gravity)?;
        self.scene_graph = Some(graph);
        self.world = world;
        self.fixed_timestep = fixed_timestep;
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
        if self.is_shutdown {
            return Err(RuntimeError::AlreadyShutdown);
        }
        if self.first_tick {
            self.first_tick = false;
            if let Some(callbacks) = self.callbacks {
                for script in &mut self.scripts {
                    script.script_handle = (callbacks.create_script)(
                        script.entity.as_u64(),
                        script.guid_hi,
                        script.guid_lo,
                    );
                }
                for script in &mut self.scripts {
                    (callbacks.awake)(script.script_handle);
                    script.awake_called = true;
                }
                for script in &mut self.scripts {
                    (callbacks.on_enable)(script.script_handle);
                    script.on_enable_called = true;
                }
                for script in &mut self.scripts {
                    (callbacks.start)(script.script_handle);
                    script.start_called = true;
                }
            }
            return Ok(());
        }
        let scaled_delta = delta_seconds * self.time_scale;
        self.accumulator += scaled_delta;
        let fixed_step = self.fixed_timestep;
        const EPSILON: f64 = 1e-9;
        while self.accumulator >= fixed_step - EPSILON {
            self.world.step(fixed_step)?;
            if let Some(callbacks) = self.callbacks {
                for script in &self.scripts {
                    (callbacks.fixed_update)(script.script_handle, fixed_step);
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
                (callbacks.collision)(event.a.as_u64(), event.b.as_u64(), phase_code(event.phase));
            }
            for event in trigger_events {
                (callbacks.trigger)(event.a.as_u64(), event.b.as_u64(), phase_code(event.phase));
            }
            for script in &self.scripts {
                (callbacks.update)(script.script_handle, scaled_delta);
            }
            for script in &self.scripts {
                (callbacks.late_update)(script.script_handle, scaled_delta);
            }
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), RuntimeError> {
        if self.is_shutdown {
            return Ok(());
        }
        self.is_shutdown = true;
        if let Some(callbacks) = self.callbacks {
            for script in self.scripts.iter().rev() {
                if script.on_enable_called {
                    (callbacks.on_disable)(script.script_handle);
                }
            }
            for script in self.scripts.iter().rev() {
                if script.script_handle != 0 {
                    (callbacks.destroy)(script.script_handle);
                }
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

fn discover_project_settings(scene_path: &Path) -> (f64, [f64; 3]) {
    let Some(settings_dir) = find_project_settings_dir(scene_path) else {
        return (DEFAULT_FIXED_TIMESTEP, DEFAULT_GRAVITY);
    };
    let fixed_timestep = match parse_time_manager(&settings_dir.join("TimeManager.asset")) {
        Ok(Some(settings)) => settings.fixed_timestep,
        Ok(None) | Err(_) => DEFAULT_FIXED_TIMESTEP,
    };
    let gravity = match parse_dynamics_manager(&settings_dir.join("DynamicsManager.asset")) {
        Ok(Some(settings)) => settings.gravity,
        Ok(None) | Err(_) => DEFAULT_GRAVITY,
    };
    (fixed_timestep, gravity)
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
