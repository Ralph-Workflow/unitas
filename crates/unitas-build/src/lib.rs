pub mod bundle;
pub mod config;
pub mod control;
pub mod licensing;
pub mod package;
pub mod player;
pub mod report;
pub mod sample;
pub mod targets;

pub use bundle::{
    content_hash_hex, stage_bundle, stage_bundle_for_host, BundleError, BundleManifest,
    BundleManifestEntry, StagedBundle, BUNDLE_MANIFEST_FILENAME, BUNDLE_MANIFEST_FORMAT_VERSION,
    DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE,
};
pub use config::{BuildConfiguration, BuildMode, BuildValidationError, ProductIdentity};
pub use player::{
    build_project, build_project_for_host, decode_bmp_bytes, diagnostics_from_bundle_error,
    execute_build, execute_build_for_host, launch_bundle, load_and_validate_bundle_manifest,
    mean_image_channel_error, BuildOutcomeBundle, DeviceVerifyReport, PlayerLaunchResult,
    PlayerSession, PlayerStartupError, WindowedLaunchOptions, WindowedPlayerResult,
    BUILD_REPORT_FILENAME, DEFAULT_DEVICE_VERIFY_HEIGHT, DEFAULT_DEVICE_VERIFY_WIDTH,
};
#[cfg(feature = "device")]
pub use player::{capture_sample_reference_bmp, launch_bundle_windowed, run_device_verification};
pub use report::{
    BuildDiagnostic, BuildOutcome, BuildReport, ReportParseError, SizeCategoryEntry,
    SizeContributor,
};
pub use sample::{
    imported_gem_mesh, roll_a_ball_scene_yaml, stage_roll_a_ball_bundle,
    stage_roll_a_ball_bundle_for_host, synthesize_chime_clip, synthesize_chime_wav_bytes,
    PickupSlot, RollABallController, PICKUP_COLLECT_RADIUS, PICKUP_NAMES, PICKUP_POSITIONS,
    PLAYER_START_POSITION, ROLL_A_BALL_APPLICATION_ID, ROLL_A_BALL_COMPANY_NAME,
    ROLL_A_BALL_PRODUCT_NAME, ROLL_A_BALL_SCENE_FILENAME, ROLL_A_BALL_VERSION, WALL_NAMES,
};
pub use targets::{BuildTarget, TargetArch, TargetOs, TargetStatus};
