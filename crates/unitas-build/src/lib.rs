pub mod bundle;
pub mod config;
pub mod control;
pub mod cooked;
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
pub use package::{
    attempt_desktop_target, attempt_desktop_targets, desktop_package_targets,
    package_desktop_target, package_desktop_targets, package_linux_app, package_macos_app,
    package_web_app, package_windows_app, PackageError, PackagedApp, PackagedPlayer,
    PackagedTarget,
};
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
    DesktopPackageOutcome, RejectedBundleReportEntry, ReportParseError, SizeCategoryEntry,
    SizeContributor, StartupMeasurement, StartupStatus,
    DESKTOP_PACKAGE_STATE_EXTERNALLY_UNAVAILABLE, DESKTOP_PACKAGE_STATE_PACKAGED, STARTUP_BASIS,
};
pub use sample::{
    imported_gem_mesh, roll_a_ball_build_configuration, roll_a_ball_scene_yaml,
    stage_roll_a_ball_bundle, stage_roll_a_ball_bundle_for_host, ROLL_A_BALL_APPLICATION_ID,
    ROLL_A_BALL_COMPANY_NAME, ROLL_A_BALL_PRODUCT_NAME, ROLL_A_BALL_SCENE_FILENAME,
    ROLL_A_BALL_VERSION,
};
pub use targets::{BuildTarget, TargetArch, TargetOs, TargetStatus};
pub use toolchain::{
    evaluate_availability, packaging_targets, probe_target_availability, required_toolchain,
    target_availability_report, web_runtime_wasm32_gaps, TargetAvailability, TargetToolchain,
    ToolPresence, ToolchainCause, ToolchainProbe, ToolchainRequirement, UnavailableToolchain,
    WebRuntimeCompileGap, WEB_RUNTIME_WASM32_GAPS,
};
