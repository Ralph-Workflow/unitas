pub mod bundle;
pub mod config;
pub mod control;
pub mod licensing;
pub mod package;
pub mod player;
pub mod report;
pub mod sample;
pub mod scripted;
pub mod targets;
pub mod toolchain;

pub use bundle::{
    build_addressables_content, build_addressables_content_for_host,
    build_addressables_content_update, build_addressables_content_update_for_host,
    build_content_catalog, build_content_catalog_for_host, build_content_update,
    build_content_update_for_host, content_hash_hex, derive_compatible_player_versions,
    derive_player_compatibility_version, stage_bundle, stage_bundle_for_host,
    validate_content_bundle, AddressablesGroupConfig, AddressablesProfile, BundleError,
    BundleManifest, BundleManifestEntry, ContentBuildMode, ContentBuildResult, ContentBundleInput,
    ContentCatalog, ContentCatalogEntry, ContentRejectionReason, RejectedContentBundle,
    StagedBundle, BUNDLE_MANIFEST_FILENAME, BUNDLE_MANIFEST_FORMAT_VERSION,
    CONTENT_CATALOG_FILENAME, CONTENT_CATALOG_FORMAT_VERSION,
    DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_VALUE,
};
pub use config::{BuildConfiguration, BuildMode, BuildValidationError, ProductIdentity};
pub use player::{
    apply_content_update_to_player, build_project, build_project_for_host, decode_bmp_bytes,
    diagnostics_from_bundle_error, execute_build, execute_build_for_host, launch_bundle,
    load_and_validate_bundle_manifest, mean_image_channel_error, measure_packaged_startup,
    measure_player_startup, render_failure, window_failure, BuildOutcomeBundle, DeviceVerifyReport,
    PlayerLaunchResult, PlayerSession, PlayerStartupError, ScriptHost, WindowedLaunchOptions,
    WindowedPlayerResult, BUILD_REPORT_FILENAME, DEFAULT_DEVICE_VERIFY_HEIGHT,
    DEFAULT_DEVICE_VERIFY_WIDTH, STARTUP_FAILURE_EXIT_CODE,
};
#[cfg(feature = "device")]
pub use player::{capture_sample_reference_bmp, launch_bundle_windowed, run_device_verification};
pub use report::{
    BuildDiagnostic, BuildOutcome, BuildReport, ContentBuildReport, ContentBundleReportEntry,
    RejectedBundleReportEntry, ReportParseError, SizeCategoryEntry, SizeContributor,
    StartupMeasurement, StartupStatus, STARTUP_BASIS,
};
pub use sample::{
    imported_gem_mesh, roll_a_ball_build_configuration, roll_a_ball_scene_yaml,
    stage_roll_a_ball_bundle, stage_roll_a_ball_bundle_for_host, synthesize_chime_clip,
    synthesize_chime_wav_bytes, PickupSlot, RollABallController, PICKUP_COLLECT_RADIUS,
    PICKUP_NAMES, PICKUP_POSITIONS, PLAYER_START_POSITION, ROLL_A_BALL_APPLICATION_ID,
    ROLL_A_BALL_COMPANY_NAME, ROLL_A_BALL_PRODUCT_NAME, ROLL_A_BALL_SCENE_FILENAME,
    ROLL_A_BALL_VERSION, WALL_NAMES,
};
pub use targets::{BuildTarget, TargetArch, TargetOs, TargetStatus};
pub use toolchain::{
    evaluate_availability, packaging_targets, probe_target_availability, required_toolchain,
    target_availability_report, TargetAvailability, TargetToolchain, ToolPresence, ToolchainCause,
    ToolchainProbe, ToolchainRequirement, UnavailableToolchain,
};
