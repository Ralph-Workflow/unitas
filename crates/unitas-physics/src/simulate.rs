use crate::error::PhysicsError;
use crate::event::{CollisionEvent, TriggerEvent};
use crate::world::World;

/// The fixed timestep a scene advances by when its mode is `FixedUpdate`.
pub const DEFAULT_FIXED_TIMESTEP: f64 = 0.02;

/// When a physics scene advances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationMode {
    /// Physics advances once per fixed step.
    FixedUpdate,
    /// Physics advances once per frame by that frame's delta time.
    Update,
    /// Physics advances only when a script calls `simulate`.
    Script,
}

/// The outcome of one simulation step.
#[derive(Clone, Debug, PartialEq)]
pub struct StepResult {
    pub delta_seconds: f64,
    pub collision_events: Vec<CollisionEvent>,
    pub trigger_events: Vec<TriggerEvent>,
}

/// Why a simulation request did not advance the scene.
#[derive(Clone, Debug, PartialEq)]
pub enum SimulationError {
    /// `simulate` was called while the scene was not in `Script` mode.
    NotScriptMode(SimulationMode),
    /// The fixed timestep is not a finite positive number.
    InvalidFixedTimestep(f64),
    /// The world rejected the step.
    World(PhysicsError),
}

impl From<PhysicsError> for SimulationError {
    fn from(error: PhysicsError) -> Self {
        Self::World(error)
    }
}

/// A physics scene that owns a world and decides when that world advances.
///
/// The default scene and a scene-local scene are separate scenes: each owns its
/// own world, so stepping one never moves the bodies of the other.
pub struct Simulation {
    world: World,
    mode: SimulationMode,
    fixed_timestep: f64,
}

impl Simulation {
    /// The default physics scene, which advances on the fixed step.
    pub fn new(world: World) -> Self {
        Self {
            world,
            mode: SimulationMode::FixedUpdate,
            fixed_timestep: DEFAULT_FIXED_TIMESTEP,
        }
    }

    /// A scene-local physics scene, which advances only when a script asks it to.
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

    /// Whether the engine advances this scene without a script asking it to.
    pub fn auto_simulation(&self) -> bool {
        self.mode != SimulationMode::Script
    }

    /// Selects `FixedUpdate` when enabled and `Script` when disabled.
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

    /// Advances the scene the way the engine's frame loop drives it.
    ///
    /// A `FixedUpdate` scene advances by its fixed timestep and an `Update`
    /// scene by the frame delta. A `Script` scene does not advance; the
    /// returned result is `None`.
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

    /// Advances the scene by `delta_seconds` as an explicit script request.
    ///
    /// Only a `Script` scene advances; another mode reports an error and
    /// leaves the scene where it was.
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
