use std::path::Path;
use unitas_asset::scene::assemble_scene_file_with_scripts;
use unitas_core::{EntityId, SceneGraph};

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
    callbacks: Option<Callbacks>,
    accumulator: f64,
    first_tick: bool,
    is_shutdown: bool,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(Self {
            scene_graph: None,
            scripts: Vec::new(),
            callbacks: None,
            accumulator: 0.0,
            first_tick: true,
            is_shutdown: false,
        })
    }

    pub fn load_scene(&mut self, path: &Path) -> Result<usize, RuntimeError> {
        let (graph, bindings) = assemble_scene_file_with_scripts(path)?;
        self.scene_graph = Some(graph);
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
        self.accumulator += delta_seconds;
        const FIXED_TIME_STEP: f64 = 0.02;
        const EPSILON: f64 = 1e-9;
        while self.accumulator >= FIXED_TIME_STEP - EPSILON {
            if let Some(callbacks) = self.callbacks {
                for script in &self.scripts {
                    (callbacks.fixed_update)(script.script_handle, FIXED_TIME_STEP);
                }
            }
            self.accumulator -= FIXED_TIME_STEP;
            if self.accumulator < 0.0 {
                self.accumulator = 0.0;
            }
        }
        if let Some(callbacks) = self.callbacks {
            for script in &self.scripts {
                (callbacks.update)(script.script_handle, delta_seconds);
            }
            for script in &self.scripts {
                (callbacks.late_update)(script.script_handle, delta_seconds);
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

    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }
}
