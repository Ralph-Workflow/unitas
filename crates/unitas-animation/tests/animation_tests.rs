//! One test binary for the animation suite.
//!
//! Each area keeps its own file under `animation/`; this root module binds
//! them so the workspace builds one test binary per crate instead of one per
//! area (each extra binary costs a link and a first-exec cost in the shared
//! verification budget).

#[path = "animation/avatar_mask_tests.rs"]
mod avatar_mask_tests;
#[path = "animation/clip_tests.rs"]
mod clip_tests;
#[path = "animation/controller_tests.rs"]
mod controller_tests;
#[path = "animation/curve_tests.rs"]
mod curve_tests;
#[path = "animation/playable_tests.rs"]
mod playable_tests;
#[path = "animation/wire_tests.rs"]
mod wire_tests;
