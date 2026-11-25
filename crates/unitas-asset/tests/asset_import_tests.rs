//! One test binary for the asset-import suite.
//!
//! Each import area keeps its own file under `asset_import/`; this root module
//! binds them so the workspace builds one test binary per crate instead of one
//! per area (each extra binary costs a link and a first-exec cost in the
//! shared verification budget).

#[path = "asset_import/audio_import_tests.rs"]
mod audio_import_tests;
#[path = "asset_import/binary_ser_tests.rs"]
mod binary_ser_tests;
#[path = "asset_import/database_tests.rs"]
mod database_tests;
#[path = "asset_import/dependencies_tests.rs"]
mod dependencies_tests;
#[path = "asset_import/fbx_tests.rs"]
mod fbx_tests;
#[path = "asset_import/inflate_tests.rs"]
mod inflate_tests;
#[path = "asset_import/json_tests.rs"]
mod json_tests;
#[path = "asset_import/model_tests.rs"]
mod model_tests;
#[path = "asset_import/packages_tests.rs"]
mod packages_tests;
#[path = "asset_import/prefab_variant_tests.rs"]
mod prefab_variant_tests;
#[path = "asset_import/primitive_tests.rs"]
mod primitive_tests;
#[path = "asset_import/project_tests.rs"]
mod project_tests;
#[path = "asset_import/scene_save_tests.rs"]
mod scene_save_tests;
#[path = "asset_import/texture_tests.rs"]
mod texture_tests;
#[path = "asset_import/trust_tests.rs"]
mod trust_tests;
