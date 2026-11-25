//! One test binary for the render suite.
//!
//! Each area keeps its own file under `render/`; this root module binds them
//! so the workspace builds one test binary per crate instead of one per area
//! (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

#[path = "render/integration_tests.rs"]
mod integration_tests;
#[path = "render/render_tests.rs"]
mod render_tests;
