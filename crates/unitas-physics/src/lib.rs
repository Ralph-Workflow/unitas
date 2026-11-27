pub mod articulation;
mod backend;
mod body;
mod character;
pub mod cloth;
mod error;
mod event;
pub mod joint;
pub mod matrix;
pub mod navigation;
pub mod particles;
mod query;
pub mod simulate;
pub mod terrain;
pub mod trails;
pub mod two_d;
pub mod wheel;
pub mod world;

pub use body::{BodyDesc, BodyRole, ColliderShape, CollisionDetectionMode3D, Shape};
pub use character::{CharacterController, CollisionFlags, MoveResult};
pub use error::PhysicsError;
pub use event::{CollisionEvent, Phase, TriggerEvent};
pub use joint::{
    BrokenJointEvent3D, CharacterJoint3D, ConfigurableJoint3D, ConfigurableJointMotion,
    FixedJoint3D, HingeJoint3D, Joint3D, JointDrive3D, JointId3D, JointLimits3D, JointMotor3D,
    JointSpring3D, SpringJoint3D,
};
pub use query::QueryHit;
pub use world::{
    combine_materials, BodyInterpolation, BounceCombine, Capsule, ColliderDesc, CollisionContact,
    CollisionPayload, ForceMode, FrictionCombine, PhysicsMaterial3D, SolverSettings, SolverType,
    World,
};
