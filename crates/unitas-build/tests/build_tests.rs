//! One test binary for the build suite.
//!
//! Each area keeps its own file under `build/`; this root module binds them
//! so the workspace builds one test binary per crate instead of one per area
//! (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

#[path = "build/build_and_launch.rs"]
mod build_and_launch;
#[path = "build/content_catalog.rs"]
mod content_catalog;
#[path = "build/report_tests.rs"]
mod report_tests;
