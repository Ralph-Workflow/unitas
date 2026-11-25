//! One test binary for the simulation suite.
//!
//! Each area keeps its own file under `simulation/`; this root module binds
//! them so the workspace builds one test binary per crate instead of one per
//! area (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

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
