//! One test binary for the core suite.
//!
//! Each area keeps its own file under `core/`; this root module binds them so
//! the workspace builds one test binary per crate instead of one per area
//! (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

#[path = "core/compat_tests.rs"]
mod compat_tests;
#[path = "core/scene_graph.rs"]
mod scene_graph;
