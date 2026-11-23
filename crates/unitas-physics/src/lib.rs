mod body;
mod error;
mod event;
mod world;

pub use body::{BodyDesc, Shape};
pub use error::PhysicsError;
pub use event::{CollisionEvent, Phase, TriggerEvent};
pub use world::World;
