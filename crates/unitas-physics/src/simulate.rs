use crate::error::PhysicsError;
use crate::event::{CollisionEvent, TriggerEvent};
use crate::world::World;

pub const DEFAULT_FIXED_TIMESTEP: f64 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationMode {
    FixedUpdate,
    Update,
    Script,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepResult {
    pub delta_seconds: f64,
    pub collision_events: Vec<CollisionEvent>,
    pub trigger_events: Vec<TriggerEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SimulationError {
    NotScriptMode(SimulationMode),
    InvalidFixedTimestep(f64),
    World(PhysicsError),
}

impl From<PhysicsError> for SimulationError {
    fn from(error: PhysicsError) -> Self {
        Self::World(error)
    }
}

pub struct Simulation {
    world: World,
    mode: SimulationMode,
    fixed_timestep: f64,
}

impl Simulation {
    pub fn new(world: World) -> Self {
        Self {
            world,
            mode: SimulationMode::FixedUpdate,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
        }
    }

    pub fn local(world: World) -> Self {
        Self {
            mode: SimulationMode::Script,
            ..Self::new(world)
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn mode(&self) -> SimulationMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: SimulationMode) {
        self.mode = mode;
    }

    pub fn auto_simulation(&self) -> bool {
        self.mode != SimulationMode::Script
    }

    pub fn set_auto_simulation(&mut self, enabled: bool) {
        self.mode = if enabled {
            SimulationMode::FixedUpdate
        } else {
            SimulationMode::Script
        };
    }

    pub fn fixed_timestep(&self) -> f64 {
        self.fixed_timestep
    }

    pub fn set_fixed_timestep(&mut self, seconds: f64) -> Result<(), SimulationError> {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(SimulationError::InvalidFixedTimestep(seconds));
        }
        self.fixed_timestep = seconds;
        Ok(())
    }

    pub fn advance(
        &mut self,
        frame_delta_seconds: f64,
    ) -> Result<Option<StepResult>, SimulationError> {
        let delta_seconds = match self.mode {
            SimulationMode::Script => return Ok(None),
            SimulationMode::FixedUpdate => self.fixed_timestep,
            SimulationMode::Update => frame_delta_seconds,
        };
        Ok(Some(self.step(delta_seconds)?))
    }

    pub fn simulate(&mut self, delta_seconds: f64) -> Result<StepResult, SimulationError> {
        if self.mode != SimulationMode::Script {
            return Err(SimulationError::NotScriptMode(self.mode));
        }
        self.step(delta_seconds)
    }

    fn step(&mut self, delta_seconds: f64) -> Result<StepResult, SimulationError> {
        self.world.step(delta_seconds)?;
        Ok(StepResult {
            delta_seconds,
            collision_events: self.world.drain_events(),
            trigger_events: self.world.drain_trigger_events(),
        })
    }
}
