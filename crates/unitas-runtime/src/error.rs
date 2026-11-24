use std::fmt;
use std::io;
use unitas_asset::scene::SceneError;
use unitas_audio::AudioError;
use unitas_core::EntityId;
use unitas_physics::PhysicsError;

#[derive(Debug)]
pub enum RuntimeError {
    Scene(SceneError),
    Physics(PhysicsError),
    Audio(AudioError),
    Io(io::Error),
    InvalidGuid(String),
    InvalidAddress,
    InvalidUtf8,
    NotFound,
    AlreadyShutdown,
    Poisoned,
    SceneNotFound(String),
    InvalidSceneIndex(usize),
    EntityNotFound(EntityId),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Scene(err) => write!(formatter, "scene error: {err}"),
            RuntimeError::Physics(err) => write!(formatter, "physics error: {err}"),
            RuntimeError::Audio(err) => write!(formatter, "audio error: {err}"),
            RuntimeError::Io(err) => write!(formatter, "io error: {err}"),
            RuntimeError::InvalidGuid(guid) => write!(formatter, "invalid guid: {guid}"),
            RuntimeError::InvalidAddress => write!(formatter, "invalid buffer address or length"),
            RuntimeError::InvalidUtf8 => write!(formatter, "invalid utf-8 path"),
            RuntimeError::NotFound => write!(formatter, "runtime handle not found"),
            RuntimeError::AlreadyShutdown => write!(formatter, "runtime is already shut down"),
            RuntimeError::Poisoned => write!(formatter, "runtime mutex poisoned"),
            RuntimeError::SceneNotFound(name) => write!(formatter, "scene not found: {name}"),
            RuntimeError::InvalidSceneIndex(index) => {
                write!(formatter, "invalid scene index: {index}")
            }
            RuntimeError::EntityNotFound(entity) => write!(formatter, "entity not found: {entity}"),
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeError::Scene(err) => Some(err),
            RuntimeError::Physics(err) => Some(err),
            RuntimeError::Audio(err) => Some(err),
            RuntimeError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<SceneError> for RuntimeError {
    fn from(err: SceneError) -> Self {
        RuntimeError::Scene(err)
    }
}

impl From<PhysicsError> for RuntimeError {
    fn from(err: PhysicsError) -> Self {
        RuntimeError::Physics(err)
    }
}

impl From<AudioError> for RuntimeError {
    fn from(err: AudioError) -> Self {
        RuntimeError::Audio(err)
    }
}

impl From<io::Error> for RuntimeError {
    fn from(err: io::Error) -> Self {
        RuntimeError::Io(err)
    }
}
