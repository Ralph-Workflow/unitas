#[cfg(test)]
extern crate self as unitas_asset;

pub mod material;
pub mod meta;
pub mod prefab;
pub mod project_settings;
pub mod scene;
pub mod unity_yaml;

pub use material::{
    parse_material, parse_material_stream, parse_material_stream_with_guids,
    parse_material_with_guids, Material, MaterialError,
};
pub use meta::{GuidIndex, MetaError, MetaIndex};
pub use project_settings::{
    parse_dynamics_manager, parse_dynamics_manager_stream, parse_input_manager,
    parse_input_manager_stream, parse_time_manager, parse_time_manager_stream, DynamicsSettings,
    InputAxis, InputManagerSettings, ProjectSettingsError, TimeSettings, DEFAULT_FIXED_TIMESTEP,
    DEFAULT_GRAVITY,
};
pub use scene::{
    assemble_scene, assemble_scene_file, assemble_scene_file_with_guids,
    assemble_scene_file_with_physics, assemble_scene_with_guids, assemble_scene_with_physics,
    BoxColliderComponent, PhysicsBinding, RigidbodyComponent, SceneError, SphereColliderComponent,
};
pub use unity_yaml::{AssetError, Document, Header, Node, Stream, YamlError, YamlNode};
