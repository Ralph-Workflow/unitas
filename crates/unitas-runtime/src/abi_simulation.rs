use unitas_physics::PhysicsError;
use unitas_physics::simulate::SimulationMode;

use super::{
    Runtime, RuntimeError, restore_to_registry, run_reentrant_operation, take_from_registry,
    with_active_runtime, with_active_state, with_runtime_state,
};
use crate::runtime::PhysicsMessages;

const UPDATE_STEP_3D_TYPE: &str = "UnityEngine.PlayerLoop.Update.PhysicsUpdateStep3D";
const UPDATE_STEP_2D_TYPE: &str = "UnityEngine.PlayerLoop.Update.Physics2DUpdateStep";
const FIXED_STEP_3D_TYPE: &str = "UnityEngine.PlayerLoop.FixedUpdate.PhysicsFixedUpdate";
const FIXED_STEP_2D_TYPE: &str = "UnityEngine.PlayerLoop.FixedUpdate.Physics2DFixedUpdate";
const SCRIPT_UPDATE_TYPE: &str = "UnityEngine.PlayerLoop.Update.ScriptRunBehaviourUpdate";
const FIXED_PHASE_TYPE: &str = "UnityEngine.PlayerLoop.FixedUpdate";
const UPDATE_PHASE_TYPE: &str = "UnityEngine.PlayerLoop.Update";

fn decode_mode(mode: u32) -> Option<SimulationMode> {
    match mode {
        0 => Some(SimulationMode::FixedUpdate),
        1 => Some(SimulationMode::Update),
        2 => Some(SimulationMode::Script),
        _ => None,
    }
}

fn encode_mode(mode: SimulationMode) -> i32 {
    match mode {
        SimulationMode::FixedUpdate => 0,
        SimulationMode::Update => 1,
        SimulationMode::Script => 2,
    }
}

extern "C" fn update_step_3d_callback(_user_data: u64, delta_seconds: f64) {
    if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
        return;
    }
    let _ = with_active_runtime(|runtime| {
        if runtime.simulation_mode_3d() != SimulationMode::Update {
            return None;
        }
        if runtime.step_3d_into_pending(delta_seconds).is_ok() {
            runtime.dispatch_physics_messages(&mut |_, f| f());
        }
        Some(())
    });
}

extern "C" fn update_step_2d_callback(_user_data: u64, delta_seconds: f64) {
    if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
        return;
    }
    let _ = with_active_runtime(|runtime| {
        if runtime.simulation_mode_2d() != SimulationMode::Update {
            return None;
        }
        runtime.step_2d_into_pending(delta_seconds as f32);
        runtime.dispatch_physics_messages(&mut |_, f| f());
        Some(())
    });
}

impl Runtime {
    pub fn simulation_mode_3d(&self) -> SimulationMode {
        if self.player_loop.root().contains(UPDATE_STEP_3D_TYPE) {
            return SimulationMode::Update;
        }
        if !self.player_loop.root().contains(FIXED_STEP_3D_TYPE) {
            return SimulationMode::Script;
        }
        SimulationMode::FixedUpdate
    }

    pub fn set_simulation_mode_3d(&mut self, mode: SimulationMode) {
        self.apply_player_loop_mode_3d(mode);
    }

    pub fn simulation_mode_2d(&self) -> SimulationMode {
        if self.player_loop.root().contains(UPDATE_STEP_2D_TYPE) {
            return SimulationMode::Update;
        }
        if !self.player_loop.root().contains(FIXED_STEP_2D_TYPE) {
            return SimulationMode::Script;
        }
        SimulationMode::FixedUpdate
    }

    pub fn set_simulation_mode_2d(&mut self, mode: SimulationMode) {
        self.apply_player_loop_mode_2d(mode);
    }

    fn apply_player_loop_mode_3d(&mut self, mode: SimulationMode) {
        let _ = self
            .player_loop
            .unregister_custom_system(UPDATE_STEP_3D_TYPE);
        let mut tree = self.player_loop.root().clone();
        tree.remove(FIXED_STEP_3D_TYPE);
        tree.remove(UPDATE_STEP_3D_TYPE);
        if mode == SimulationMode::FixedUpdate
            && let Some(fixed) = tree.find_mut(FIXED_PHASE_TYPE)
        {
            fixed.children.insert(
                0,
                crate::player_loop::PlayerLoopSystem::new("PhysicsFixedUpdate", FIXED_STEP_3D_TYPE),
            );
        }
        let _ = self.player_loop.set_root(tree);
        if mode == SimulationMode::Update {
            let anchor = if self.player_loop.root().contains(SCRIPT_UPDATE_TYPE) {
                SCRIPT_UPDATE_TYPE
            } else {
                UPDATE_PHASE_TYPE
            };
            let _ = self.player_loop.register_custom_system(
                "PhysicsUpdateStep3D",
                UPDATE_STEP_3D_TYPE,
                anchor,
                crate::player_loop::CustomSystemPosition::After,
                update_step_3d_callback,
                0,
            );
        }
    }

    fn apply_player_loop_mode_2d(&mut self, mode: SimulationMode) {
        let _ = self
            .player_loop
            .unregister_custom_system(UPDATE_STEP_2D_TYPE);
        let mut tree = self.player_loop.root().clone();
        tree.remove(FIXED_STEP_2D_TYPE);
        tree.remove(UPDATE_STEP_2D_TYPE);
        if mode == SimulationMode::FixedUpdate
            && let Some(fixed) = tree.find_mut(FIXED_PHASE_TYPE)
        {
            let pos = usize::from(
                fixed
                    .children
                    .first()
                    .is_some_and(|c| c.type_name == FIXED_STEP_3D_TYPE),
            );
            fixed.children.insert(
                pos,
                crate::player_loop::PlayerLoopSystem::new(
                    "Physics2DFixedUpdate",
                    FIXED_STEP_2D_TYPE,
                ),
            );
        }
        let _ = self.player_loop.set_root(tree);
        if mode == SimulationMode::Update {
            let anchor = if self.player_loop.root().contains(UPDATE_STEP_3D_TYPE) {
                UPDATE_STEP_3D_TYPE
            } else if self.player_loop.root().contains(SCRIPT_UPDATE_TYPE) {
                SCRIPT_UPDATE_TYPE
            } else {
                UPDATE_PHASE_TYPE
            };
            let _ = self.player_loop.register_custom_system(
                "Physics2DUpdateStep",
                UPDATE_STEP_2D_TYPE,
                anchor,
                crate::player_loop::CustomSystemPosition::After,
                update_step_2d_callback,
                0,
            );
        }
    }

    pub(crate) fn run_physics_fixed_update_3d(&mut self) -> Result<(), RuntimeError> {
        if self.simulation_mode_3d() != SimulationMode::FixedUpdate {
            return Ok(());
        }
        self.step_3d_into_pending(self.fixed_timestep)
    }

    pub(crate) fn run_physics_fixed_update_2d(&mut self) {
        if self.simulation_mode_2d() != SimulationMode::FixedUpdate {
            return;
        }
        self.step_2d_into_pending(self.fixed_timestep as f32);
    }

    pub fn simulate_3d(&mut self, delta_seconds: f64) -> Result<(), RuntimeError> {
        self.simulate_3d_with_hook(delta_seconds, |_, f| f())
    }

    pub fn simulate_3d_with_hook<H>(
        &mut self,
        delta_seconds: f64,
        mut hook: H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.simulation_mode_3d() != SimulationMode::Script {
            return Err(RuntimeError::Physics(PhysicsError::InvalidDeltaTime(
                delta_seconds,
            )));
        }
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(RuntimeError::Physics(PhysicsError::InvalidDeltaTime(
                delta_seconds,
            )));
        }
        let saved = self.pending_physics_messages.take();
        let res = self.step_3d_into_pending(delta_seconds);
        if res.is_ok() {
            self.dispatch_physics_messages(&mut hook);
        }
        self.pending_physics_messages = saved;
        res
    }

    pub fn simulate_2d(&mut self, delta_seconds: f32) -> Result<(), RuntimeError> {
        self.simulate_2d_with_hook(delta_seconds, |_, f| f())
    }

    pub fn simulate_2d_with_hook<H>(
        &mut self,
        delta_seconds: f32,
        mut hook: H,
    ) -> Result<(), RuntimeError>
    where
        H: FnMut(&mut Self, &mut dyn FnMut()),
    {
        if self.simulation_mode_2d() != SimulationMode::Script {
            return Err(RuntimeError::Physics(PhysicsError::InvalidDeltaTime(
                f64::from(delta_seconds),
            )));
        }
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return Err(RuntimeError::Physics(PhysicsError::InvalidDeltaTime(
                f64::from(delta_seconds),
            )));
        }
        let saved = self.pending_physics_messages.take();
        self.step_2d_into_pending(delta_seconds);
        self.dispatch_physics_messages(&mut hook);
        self.pending_physics_messages = saved;
        Ok(())
    }

    fn step_3d_into_pending(&mut self, delta_seconds: f64) -> Result<(), RuntimeError> {
        self.sync_kinematic_transforms();
        self.physics.world_3d_mut().step(delta_seconds)?;
        self.write_back_dynamic_transforms();
        let events = self.physics.world_3d_mut().drain_events();
        let trigger_events = self.physics.world_3d_mut().drain_trigger_events();
        match &mut self.pending_physics_messages {
            Some(messages) => {
                messages.events.extend(events);
                messages.trigger_events.extend(trigger_events);
            }
            None => {
                self.pending_physics_messages = Some(PhysicsMessages {
                    events,
                    trigger_events,
                    contacts_2d: Vec::new(),
                });
            }
        }
        Ok(())
    }

    fn step_2d_into_pending(&mut self, delta_seconds: f32) {
        let contacts_2d = self.physics.step_2d_contacts(delta_seconds);
        match &mut self.pending_physics_messages {
            Some(messages) => {
                messages.contacts_2d.extend(contacts_2d);
            }
            None => {
                self.pending_physics_messages = Some(PhysicsMessages {
                    events: Vec::new(),
                    trigger_events: Vec::new(),
                    contacts_2d,
                });
            }
        }
    }
}

fn run_simulation_operation(
    handle: u64,
    op: impl FnOnce(
        &mut Runtime,
        &mut dyn FnMut(&mut Runtime, &mut dyn FnMut()),
    ) -> Result<(), RuntimeError>,
) -> i32 {
    let mut op_opt = Some(op);
    if let Some(code) = with_active_state(handle, |state| {
        let Some(f) = op_opt.take() else {
            return -1;
        };
        let result = run_reentrant_operation(handle, state, |runtime, hook| f(runtime, hook));
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        return code;
    }
    let Some(f) = op_opt else {
        return -1;
    };
    let Some(mut state) = take_from_registry(handle) else {
        return -1;
    };
    let result = run_reentrant_operation(handle, &mut state, |runtime, hook| f(runtime, hook));
    let code = match result {
        Ok(()) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -4
        }
    };
    if !restore_to_registry(handle, state) {
        return -1;
    }
    code
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics_set_simulation_mode(handle: u64, mode: u32) -> i32 {
    let Some(sim_mode) = decode_mode(mode) else {
        return with_runtime_state(handle, |state| {
            state.set_error("unknown physics simulation mode");
            -2
        })
        .unwrap_or_else(|code| code);
    };
    with_runtime_state(handle, |state| {
        state.runtime.set_simulation_mode_3d(sim_mode);
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics_get_simulation_mode(handle: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        encode_mode(state.runtime.simulation_mode_3d())
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics_simulate(handle: u64, step_seconds: f64) -> i32 {
    if !step_seconds.is_finite() || step_seconds <= 0.0 {
        return with_runtime_state(handle, |state| {
            state.set_error("simulation step must be positive and finite");
            -2
        })
        .unwrap_or_else(|code| code);
    }
    run_simulation_operation(handle, |runtime, hook| {
        runtime.simulate_3d_with_hook(step_seconds, hook)
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_set_simulation_mode(handle: u64, mode: u32) -> i32 {
    let Some(sim_mode) = decode_mode(mode) else {
        return with_runtime_state(handle, |state| {
            state.set_error("unknown 2D physics simulation mode");
            -2
        })
        .unwrap_or_else(|code| code);
    };
    with_runtime_state(handle, |state| {
        state.runtime.set_simulation_mode_2d(sim_mode);
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_get_simulation_mode(handle: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        encode_mode(state.runtime.simulation_mode_2d())
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_physics2d_simulate(handle: u64, step_seconds: f32) -> i32 {
    if !step_seconds.is_finite() || step_seconds <= 0.0 {
        return with_runtime_state(handle, |state| {
            state.set_error("2D simulation step must be positive and finite");
            -2
        })
        .unwrap_or_else(|code| code);
    }
    run_simulation_operation(handle, |runtime, hook| {
        runtime.simulate_2d_with_hook(step_seconds, hook)
    })
}
}
