//! One test binary for the physics suite.
//!
//! Each area keeps its own file under `simulation/` or `physics/`; this root
//! module binds them so the workspace builds one test binary per crate
//! instead of one per area (each extra binary costs a link and a first-exec
//! cost in the shared verification budget).

#[path = "simulation/matrix_tests.rs"]
mod matrix_tests;
#[path = "simulation/particles_tests.rs"]
mod particles_tests;
#[path = "simulation/sleep_tests.rs"]
mod sleep_tests;
#[path = "simulation/teleport_tests.rs"]
mod teleport_tests;
#[path = "simulation/two_d_effector_tests.rs"]
mod two_d_effector_tests;
#[path = "simulation/two_d_tests.rs"]
mod two_d_tests;

#[path = "simulation/sim_control_tests.rs"]
mod sim_control_tests;
#[path = "simulation/particles_2d_tests.rs"]
mod particles_2d_tests;
#[path = "simulation/articulation_tests.rs"]
mod articulation_tests;
#[path = "simulation/cloth_tests.rs"]
mod cloth_tests;

#[path = "physics/character_tests.rs"]
mod character_tests;
#[path = "physics/collision_tests.rs"]
mod collision_tests;
#[path = "physics/navigation_tests.rs"]
mod navigation_tests;
#[path = "physics/query_tests.rs"]
mod query_tests;
#[path = "physics/terrain_tests.rs"]
mod terrain_tests;
#[path = "physics/wheel_tests.rs"]
mod wheel_tests;
#[path = "physics/world_tests.rs"]
mod world_tests;
