use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::bundle::{
    content_hash_hex, stage_bundle_for_host, BundleError, BundleManifest, StagedBundle,
    BUNDLE_MANIFEST_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
};
use crate::config::BuildConfiguration;
use crate::report::{BuildDiagnostic, BuildReport, SizeCategoryEntry, SizeContributor};
use crate::targets::BuildTarget;

pub const BUILD_REPORT_FILENAME: &str = "build-report.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerStartupError {
    ManifestLoad {
        bundle_dir: PathBuf,
        source: BundleError,
    },
    NoStartupSceneConfigured {
        bundle_dir: PathBuf,
    },
    StartupSceneMissing {
        scene_path: PathBuf,
    },
    RuntimeInit {
        message: String,
    },
    SceneLoadFailed {
        scene_path: PathBuf,
        message: String,
    },
    FrameTickFailed {
        frame: usize,
        message: String,
    },
}

impl fmt::Display for PlayerStartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerStartupError::ManifestLoad { bundle_dir, source } => {
                write!(
                    formatter,
                    "failed to load bundle manifest from '{}': {source}",
                    bundle_dir.display()
                )
            }
            PlayerStartupError::NoStartupSceneConfigured { bundle_dir } => {
                write!(
                    formatter,
                    "bundle '{}' has no startup scene configured",
                    bundle_dir.display()
                )
            }
            PlayerStartupError::StartupSceneMissing { scene_path } => {
                write!(
                    formatter,
                    "startup scene does not exist at '{}'",
                    scene_path.display()
                )
            }
            PlayerStartupError::RuntimeInit { message } => {
                write!(formatter, "failed to initialize runtime: {message}")
            }
            PlayerStartupError::SceneLoadFailed {
                scene_path,
                message,
            } => {
                write!(
                    formatter,
                    "failed to load startup scene '{}': {message}",
                    scene_path.display()
                )
            }
            PlayerStartupError::FrameTickFailed { frame, message } => {
                write!(formatter, "frame tick {frame} failed: {message}")
            }
        }
    }
}

impl std::error::Error for PlayerStartupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLaunchResult {
    pub manifest: BundleManifest,
    pub startup_scene_path: PathBuf,
    pub loaded_scene_count: usize,
    pub root_object_count: usize,
    pub frames_stepped: usize,
    pub development_diagnostics_enabled: bool,
}

pub type PlayerSession = PlayerLaunchResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcomeBundle {
    pub bundle: Option<StagedBundle>,
    pub report: BuildReport,
}

pub fn launch_bundle(
    bundle_dir: &Path,
    frames: usize,
    delta_seconds: f64,
) -> Result<PlayerLaunchResult, PlayerStartupError> {
    let manifest = BundleManifest::load_from_dir(bundle_dir).map_err(|source| {
        PlayerStartupError::ManifestLoad {
            bundle_dir: bundle_dir.to_path_buf(),
            source,
        }
    })?;

    if manifest.startup_scene_staged_path.trim().is_empty() || manifest.entries.is_empty() {
        return Err(PlayerStartupError::NoStartupSceneConfigured {
            bundle_dir: bundle_dir.to_path_buf(),
        });
    }

    let startup_scene_path = bundle_dir.join(&manifest.startup_scene_staged_path);
    if !startup_scene_path.is_file() {
        return Err(PlayerStartupError::StartupSceneMissing {
            scene_path: startup_scene_path,
        });
    }

    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;

    runtime
        .load_scene(&startup_scene_path)
        .map_err(|error| PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path.clone(),
            message: error.to_string(),
        })?;

    for frame in 0..frames {
        runtime
            .tick(delta_seconds)
            .map_err(|error| PlayerStartupError::FrameTickFailed {
                frame,
                message: error.to_string(),
            })?;
    }

    let loaded_scene_count = runtime.scene_count();
    let root_object_count = runtime
        .scene_graph()
        .map_or(0, |graph| graph.roots().len());
    let development_diagnostics_enabled = manifest.development_diagnostics;

    let _ = runtime.shutdown();

    Ok(PlayerLaunchResult {
        manifest,
        startup_scene_path,
        loaded_scene_count,
        root_object_count,
        frames_stepped: frames,
        development_diagnostics_enabled,
    })
}

pub fn build_project(
    config: &BuildConfiguration,
    output_dir: &Path,
    elapsed_ms: u64,
) -> Result<(StagedBundle, BuildReport), Box<(BundleError, BuildReport)>> {
    build_project_for_host(config, output_dir, elapsed_ms, BuildTarget::current_host())
}

pub fn build_project_for_host(
    config: &BuildConfiguration,
    output_dir: &Path,
    elapsed_ms: u64,
    host: Option<BuildTarget>,
) -> Result<(StagedBundle, BuildReport), Box<(BundleError, BuildReport)>> {
    match stage_bundle_for_host(config, output_dir, host) {
        Ok(staged) => {
            let mut contributors = Vec::with_capacity(staged.manifest.entries.len() + 3);
            let mut scenes_size = 0u64;
            for entry in &staged.manifest.entries {
                scenes_size = scenes_size.saturating_add(entry.size_bytes);
                contributors.push(SizeContributor::new(
                    entry.staged_path.clone(),
                    "scenes",
                    entry.size_bytes,
                    entry.content_hash.clone(),
                ));
            }

            let manifest_bytes = fs::read(&staged.manifest_path).unwrap_or_default();
            let metadata_size = manifest_bytes.len() as u64;
            contributors.push(SizeContributor::new(
                BUNDLE_MANIFEST_FILENAME,
                "metadata",
                metadata_size,
                content_hash_hex(&manifest_bytes),
            ));

            let launcher_bytes = fs::read(&staged.launch_artifact_path).unwrap_or_default();
            let launcher_size = launcher_bytes.len() as u64;
            let launcher_name = staged
                .launch_artifact_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unitas-player");
            contributors.push(SizeContributor::new(
                launcher_name,
                "launcher",
                launcher_size,
                content_hash_hex(&launcher_bytes),
            ));

            let mut category_sizes = vec![
                SizeCategoryEntry::new("scenes", scenes_size),
                SizeCategoryEntry::new("metadata", metadata_size),
                SizeCategoryEntry::new("launcher", launcher_size),
            ];

            let mut total_size_bytes = scenes_size
                .saturating_add(metadata_size)
                .saturating_add(launcher_size);

            if let Some(ref marker_path) = staged.diagnostics_marker_path {
                let marker_bytes = fs::read(marker_path).unwrap_or_default();
                let diagnostics_size = marker_bytes.len() as u64;
                total_size_bytes = total_size_bytes.saturating_add(diagnostics_size);
                category_sizes.push(SizeCategoryEntry::new("diagnostics", diagnostics_size));
                contributors.push(SizeContributor::new(
                    DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
                    "diagnostics",
                    diagnostics_size,
                    content_hash_hex(&marker_bytes),
                ));
            }

            let report = BuildReport::succeeded(
                config,
                elapsed_ms,
                staged.output_dir.clone(),
                staged.launch_artifact_path.clone(),
                total_size_bytes,
                category_sizes,
                contributors,
            );

            let report_path = output_dir.join(BUILD_REPORT_FILENAME);
            if let Err(io_error) = report.write_to_file(&report_path) {
                let err = BundleError::Io {
                    path: report_path,
                    message: io_error.to_string(),
                };
                let failed_report =
                    BuildReport::failed(config, elapsed_ms, diagnostics_from_bundle_error(&err));
                return Err(Box::new((err, failed_report)));
            }

            Ok((staged, report))
        }
        Err(error) => {
            let diagnostics = diagnostics_from_bundle_error(&error);
            let report = BuildReport::failed(config, elapsed_ms, diagnostics);
            Err(Box::new((error, report)))
        }
    }
}

pub fn execute_build(
    config: &BuildConfiguration,
    output_dir: &Path,
    elapsed_ms: u64,
) -> BuildOutcomeBundle {
    execute_build_for_host(config, output_dir, elapsed_ms, BuildTarget::current_host())
}

pub fn execute_build_for_host(
    config: &BuildConfiguration,
    output_dir: &Path,
    elapsed_ms: u64,
    host: Option<BuildTarget>,
) -> BuildOutcomeBundle {
    match build_project_for_host(config, output_dir, elapsed_ms, host) {
        Ok((staged, report)) => BuildOutcomeBundle {
            bundle: Some(staged),
            report,
        },
        Err(boxed) => {
            let (_error, report) = *boxed;
            BuildOutcomeBundle {
                bundle: None,
                report,
            }
        }
    }
}

pub fn diagnostics_from_bundle_error(error: &BundleError) -> Vec<BuildDiagnostic> {
    match error {
        BundleError::Validation(validation_errors) => validation_errors
            .iter()
            .map(BuildDiagnostic::from_validation_error)
            .collect(),
        BundleError::SceneMissing { path } => vec![BuildDiagnostic::for_content(
            "content.scene_missing",
            path.to_string_lossy().into_owned(),
            format!("scene file does not exist at '{}'", path.display()),
            Some(String::from(
                "ensure the configured '.unity' scene file exists on disk before building",
            )),
        )],
        BundleError::SceneAssembly { path, message } => vec![BuildDiagnostic::for_content(
            "content.scene_assembly",
            path.to_string_lossy().into_owned(),
            message.clone(),
            Some(String::from(
                "fix the syntax or object references in the '.unity' scene file",
            )),
        )],
        BundleError::Io { path, message } => vec![BuildDiagnostic::for_content(
            "io.filesystem",
            path.to_string_lossy().into_owned(),
            message.clone(),
            Some(String::from(
                "verify output directory permissions and available disk space",
            )),
        )],
        BundleError::ManifestParse { message } => vec![BuildDiagnostic::new(
            "bundle.manifest_parse",
            message.clone(),
            None,
            Some(String::from("rebuild the output bundle to regenerate a valid manifest")),
        )],
    }
}
