//! One test binary for the runtime suite.
//!
//! Each area keeps its own file under `runtime/`; this root module binds them
//! so the workspace builds one test binary per crate instead of one per area
//! (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

#[path = "runtime/abi_scene_tests.rs"]
mod abi_scene_tests;
#[path = "runtime/animation_tests.rs"]
mod animation_tests;
#[path = "runtime/cinemachine_tests.rs"]
mod cinemachine_tests;
#[path = "runtime/input_tests.rs"]
mod input_tests;
#[path = "runtime/navigation_tests.rs"]
mod navigation_tests;
#[path = "runtime/physics_tests.rs"]
mod physics_tests;
#[path = "runtime/render_tests.rs"]
mod render_tests;
#[path = "runtime/runtime_tests.rs"]
mod runtime_tests;
#[path = "runtime/subsystem_entry_tests.rs"]
mod subsystem_entry_tests;
#[path = "runtime/terrain_tests.rs"]
mod terrain_tests;
#[path = "runtime/timeline_tests.rs"]
mod timeline_tests;
