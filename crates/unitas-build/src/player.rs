use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "device")]
use std::sync::{Arc, Mutex};

use unitas_render::Image;
#[cfg(feature = "device")]
use unitas_render::GPU_CPU_MEAN_ERROR_TOLERANCE;
#[cfg(feature = "device")]
use unitas_runtime::window::{run_windowed_gpu, FramePresenter, GpuFramePresenter, WindowConfig};
#[cfg(feature = "device")]
use unitas_runtime::KEY_RIGHT_ARROW;

use crate::bundle::{
    content_hash_hex, stage_bundle_for_host, BundleError, BundleManifest, StagedBundle,
    BUNDLE_MANIFEST_FILENAME, DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
};
use crate::config::BuildConfiguration;
use crate::report::{BuildDiagnostic, BuildReport, SizeCategoryEntry, SizeContributor};
#[cfg(feature = "device")]
use crate::sample::{stage_roll_a_ball_bundle, RollABallController};
use crate::targets::BuildTarget;

pub const BUILD_REPORT_FILENAME: &str = "build-report.json";
pub const DEFAULT_DEVICE_VERIFY_WIDTH: u32 = 320;
pub const DEFAULT_DEVICE_VERIFY_HEIGHT: u32 = 240;

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
    WindowFailed {
        message: String,
    },
    RenderFailed {
        message: String,
    },
    AudioFailed {
        message: String,
    },
    VerificationFailed {
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
            PlayerStartupError::WindowFailed { message } => {
                write!(formatter, "windowed player failed: {message}")
            }
            PlayerStartupError::RenderFailed { message } => {
                write!(formatter, "GPU renderer failed: {message}")
            }
            PlayerStartupError::AudioFailed { message } => {
                write!(formatter, "audio output failed: {message}")
            }
            PlayerStartupError::VerificationFailed { message } => {
                write!(formatter, "device verification failed: {message}")
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

#[derive(Debug, Clone, PartialEq)]
pub struct WindowedLaunchOptions {
    pub width: u32,
    pub height: u32,
    pub max_frames: Option<u64>,
    pub fixed_delta_seconds: Option<f64>,
    pub capture_last_frame: bool,
    pub start_audio_device: bool,
    pub inject_demo_input: bool,
}

impl Default for WindowedLaunchOptions {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            max_frames: None,
            fixed_delta_seconds: None,
            capture_last_frame: false,
            start_audio_device: true,
            inject_demo_input: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowedPlayerResult {
    pub launch: PlayerLaunchResult,
    pub frames_presented: u64,
    pub audio_device_active: bool,
    pub audio_device_status: String,
    pub audio_frames_pushed: u64,
    pub final_ball_position: [f64; 3],
    pub score: u32,
    pub total_pickups: u32,
    pub captured_frame: Option<Image>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceVerifyReport {
    pub width: u32,
    pub height: u32,
    pub frames_presented: u64,
    pub mean_channel_error: f32,
    pub tolerance: f32,
    pub audio_device_active: bool,
    pub audio_device_status: String,
    pub audio_frames_pushed: u64,
    pub ball_moved: bool,
    pub initial_ball_position: [f64; 3],
    pub final_ball_position: [f64; 3],
    pub score: u32,
    pub reference_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcomeBundle {
    pub bundle: Option<StagedBundle>,
    pub report: BuildReport,
}

pub fn load_and_validate_bundle_manifest(
    bundle_dir: &Path,
) -> Result<(BundleManifest, PathBuf), PlayerStartupError> {
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

    Ok((manifest, startup_scene_path))
}

pub fn launch_bundle(
    bundle_dir: &Path,
    frames: usize,
    delta_seconds: f64,
) -> Result<PlayerLaunchResult, PlayerStartupError> {
    let (manifest, startup_scene_path) = load_and_validate_bundle_manifest(bundle_dir)?;

    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;

    runtime.load_scene(&startup_scene_path).map_err(|error| {
        PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path.clone(),
            message: error.to_string(),
        }
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
    let root_object_count = runtime.scene_graph().map_or(0, |graph| graph.roots().len());
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

#[cfg(feature = "device")]
pub fn launch_bundle_windowed(
    bundle_dir: &Path,
    options: &WindowedLaunchOptions,
) -> Result<WindowedPlayerResult, PlayerStartupError> {
    let (manifest, startup_scene_path) = load_and_validate_bundle_manifest(bundle_dir)?;

    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;

    runtime.load_scene(&startup_scene_path).map_err(|error| {
        PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path.clone(),
            message: error.to_string(),
        }
    })?;

    let mut presenter = GpuFramePresenter::from_new_renderer().map_err(|error| {
        PlayerStartupError::RenderFailed {
            message: error.to_string(),
        }
    })?;

    let (audio_device_active, audio_device_status) = if options.start_audio_device {
        match presenter.start_default_audio_output() {
            Ok(()) => (true, String::from("active")),
            Err(error) => (false, format!("platform-limited ({error})")),
        }
    } else {
        (false, String::from("disabled"))
    };

    let mut controller =
        RollABallController::bind_to_scene(&mut runtime, &mut presenter).map_err(|error| {
            PlayerStartupError::AudioFailed {
                message: error.to_string(),
            }
        })?;
    let total_pickups = controller.total_pickups;

    let width = options.width.max(1);
    let height = options.height.max(1);
    let dt = options
        .fixed_delta_seconds
        .filter(|val| val.is_finite() && *val > 0.0)
        .unwrap_or(1.0 / 60.0);

    let window_config = WindowConfig {
        title: format!("{} ({})", manifest.product_name, manifest.version),
        width,
        height,
        resizable: true,
        frame_limit: options.max_frames.unwrap_or(0),
        max_frames: options.max_frames,
        fixed_frame_delta_seconds: options.fixed_delta_seconds,
        exit_on_escape: true,
    };

    if options.inject_demo_input {
        runtime.input_mut().inject_key(u32::from(b'w'), true);
        runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
        runtime.input_mut().inject_mouse_button(0, true);
        runtime.input_mut().inject_pointer_move(24.0, 12.0);
    }

    let window_result = run_windowed_with_controller(
        &mut runtime,
        &window_config,
        &mut presenter,
        &mut controller,
        options.inject_demo_input,
    );

    let mut frames_presented = presenter.frames_presented();
    if let Err(window_err) = window_result {
        let fallback_frames = options.max_frames.unwrap_or(1).max(1);
        for frame_idx in 0..fallback_frames {
            if options.inject_demo_input {
                runtime.input_mut().inject_key(u32::from(b'w'), true);
                runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
                runtime
                    .input_mut()
                    .inject_pointer_move((frame_idx as f64 + 1.0) * 8.0, 4.0);
            }
            runtime
                .tick(dt)
                .map_err(|error| PlayerStartupError::FrameTickFailed {
                    frame: frame_idx as usize,
                    message: error.to_string(),
                })?;
            controller
                .step(&mut runtime, &mut presenter, dt)
                .map_err(|error| PlayerStartupError::FrameTickFailed {
                    frame: frame_idx as usize,
                    message: error.to_string(),
                })?;
            let frame_info = unitas_runtime::window::FrameInfo {
                frame_index: frame_idx + 1,
                width,
                height,
                delta_seconds: dt,
            };
            presenter.update(&mut runtime, &frame_info).map_err(|err| {
                PlayerStartupError::WindowFailed {
                    message: format!("{window_err}; fallback update failed: {err}"),
                }
            })?;
            frames_presented = frame_idx + 1;
        }
    }

    let captured_frame = if options.capture_last_frame {
        Some(
            presenter
                .render_offscreen(&runtime, width, height)
                .map_err(|error| PlayerStartupError::RenderFailed {
                    message: error.to_string(),
                })?,
        )
    } else {
        None
    };

    let score = controller.score;
    let player_entity = controller.player_entity;

    let final_ball_position = player_entity
        .and_then(|ent| runtime.body_position(ent))
        .unwrap_or([0.0, 0.5, 0.0]);

    let loaded_scene_count = runtime.scene_count();
    let root_object_count = runtime.scene_graph().map_or(0, |graph| graph.roots().len());
    let development_diagnostics_enabled = manifest.development_diagnostics;
    let audio_frames_pushed = presenter.total_audio_frames_pushed();

    let _ = runtime.shutdown();

    Ok(WindowedPlayerResult {
        launch: PlayerLaunchResult {
            manifest,
            startup_scene_path,
            loaded_scene_count,
            root_object_count,
            frames_stepped: frames_presented as usize,
            development_diagnostics_enabled,
        },
        frames_presented,
        audio_device_active,
        audio_device_status,
        audio_frames_pushed,
        final_ball_position,
        score,
        total_pickups,
        captured_frame,
    })
}

#[cfg(feature = "device")]
fn run_windowed_with_controller(
    runtime: &mut unitas_runtime::Runtime,
    window_config: &WindowConfig,
    presenter: &mut GpuFramePresenter<'_>,
    controller: &mut RollABallController,
    inject_demo_input: bool,
) -> Result<(), unitas_runtime::window::WindowError> {
    let shared_meshes = Arc::new(Mutex::new(presenter.meshes().to_vec()));
    let shared_camera = Arc::new(Mutex::new(*presenter.camera_binding()));
    let shared_controller = Arc::new(Mutex::new(controller.clone()));

    let cb_meshes = Arc::clone(&shared_meshes);
    let cb_camera = Arc::clone(&shared_camera);
    let cb_controller = Arc::clone(&shared_controller);

    presenter.set_update_callback(move |rt, frame| {
        if inject_demo_input {
            rt.input_mut().inject_key(u32::from(b'w'), true);
            rt.input_mut().inject_key(KEY_RIGHT_ARROW, true);
            rt.input_mut()
                .inject_pointer_move((frame.frame_index as f64) * 6.0, 3.0);
        }
        if let (Ok(mut ctrl), Ok(mut meshes), Ok(mut cam)) =
            (cb_controller.lock(), cb_meshes.lock(), cb_camera.lock())
        {
            ctrl.step_on_bindings(rt, &mut meshes, &mut cam, frame.delta_seconds)
                .map_err(|error| unitas_runtime::window::WindowError::Runtime(error.to_string()))?;
        }
        Ok(())
    });

    let res = run_windowed_gpu(runtime, window_config, presenter);
    presenter.set_update_callback(|_, _| Ok(()));

    if let Ok(ctrl) = shared_controller.lock() {
        *controller = ctrl.clone();
    }
    if let Ok(meshes) = shared_meshes.lock() {
        presenter.set_meshes(meshes.clone());
    }
    if let Ok(cam) = shared_camera.lock() {
        presenter.set_camera_binding(*cam);
    }
    res
}

pub fn decode_bmp_bytes(bytes: &[u8]) -> Result<Image, String> {
    if bytes.len() < 54 {
        return Err(String::from("BMP buffer shorter than 54-byte header"));
    }
    if &bytes[0..2] != b"BM" {
        return Err(String::from("missing 'BM' signature"));
    }
    let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
    let width_i32 = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
    let height_i32 = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
    let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
    if width_i32 <= 0 || height_i32 == 0 {
        return Err(format!("invalid BMP dimensions {width_i32}x{height_i32}"));
    }
    if bpp != 24 {
        return Err(format!(
            "unsupported BMP bits_per_pixel {bpp} (expected 24)"
        ));
    }
    let width = width_i32 as u32;
    let top_down = height_i32 < 0;
    let height = height_i32.unsigned_abs();
    let row_bytes = (width as usize) * 3;
    let row_stride = (row_bytes + 3) & !3;
    let needed = data_offset.saturating_add(row_stride * (height as usize));
    if bytes.len() < needed {
        return Err(format!(
            "BMP buffer truncated: need {needed} bytes, found {}",
            bytes.len()
        ));
    }

    let mut image = Image::new(width, height);
    for row in 0..height {
        let target_y = if top_down { row } else { height - 1 - row };
        let row_start = data_offset + (row as usize) * row_stride;
        for x in 0..width {
            let pixel_offset = row_start + (x as usize) * 3;
            let b = bytes[pixel_offset];
            let g = bytes[pixel_offset + 1];
            let r = bytes[pixel_offset + 2];
            image.set_pixel(x, target_y, [r, g, b]);
        }
    }
    Ok(image)
}

pub fn mean_image_channel_error(left: &Image, right: &Image) -> Result<f32, String> {
    if left.width != right.width || left.height != right.height {
        return Err(format!(
            "image dimension mismatch: {}x{} vs {}x{}",
            left.width, left.height, right.width, right.height
        ));
    }
    if left.pixels.is_empty() {
        return Ok(0.0);
    }
    let mut sum = 0.0f32;
    let mut count = 0usize;
    for (p_a, p_b) in left.pixels.iter().zip(right.pixels.iter()) {
        for ch in 0..3 {
            sum += (f32::from(p_a[ch]) - f32::from(p_b[ch])).abs();
            count += 1;
        }
    }
    Ok(sum / (count.max(1) as f32))
}

#[cfg(feature = "device")]
pub fn capture_sample_reference_bmp(
    reference_bmp_path: &Path,
    width: u32,
    height: u32,
) -> Result<Image, PlayerStartupError> {
    let temp_bundle_dir =
        std::env::temp_dir().join(format!("unitas_sample_ref_capture_{}", std::process::id()));
    let staged = stage_roll_a_ball_bundle(&temp_bundle_dir).map_err(|source| {
        PlayerStartupError::ManifestLoad {
            bundle_dir: temp_bundle_dir.clone(),
            source,
        }
    })?;
    let startup_scene_path = temp_bundle_dir.join(&staged.manifest.startup_scene_staged_path);

    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;
    runtime.load_scene(&startup_scene_path).map_err(|error| {
        PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path.clone(),
            message: error.to_string(),
        }
    })?;
    let mut presenter = GpuFramePresenter::from_new_renderer().map_err(|error| {
        PlayerStartupError::RenderFailed {
            message: error.to_string(),
        }
    })?;
    let mut controller =
        RollABallController::bind_to_scene(&mut runtime, &mut presenter).map_err(|error| {
            PlayerStartupError::AudioFailed {
                message: error.to_string(),
            }
        })?;

    runtime
        .tick(1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;
    controller
        .step(&mut runtime, &mut presenter, 1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;

    let image = presenter
        .render_offscreen(&runtime, width.max(1), height.max(1))
        .map_err(|error| PlayerStartupError::RenderFailed {
            message: error.to_string(),
        })?;
    if let Some(parent) = reference_bmp_path.parent() {
        fs::create_dir_all(parent).map_err(|error| PlayerStartupError::VerificationFailed {
            message: format!(
                "failed to create reference parent directory '{}': {error}",
                parent.display()
            ),
        })?;
    }
    image.write_bmp(reference_bmp_path).map_err(|error| {
        PlayerStartupError::VerificationFailed {
            message: format!(
                "failed to write reference BMP '{}': {error}",
                reference_bmp_path.display()
            ),
        }
    })?;

    let _ = runtime.shutdown();
    let _ = fs::remove_dir_all(&temp_bundle_dir);
    Ok(image)
}

#[cfg(feature = "device")]
pub fn run_device_verification(
    reference_bmp_path: &Path,
    frames: u64,
) -> Result<DeviceVerifyReport, PlayerStartupError> {
    let reference_bytes =
        fs::read(reference_bmp_path).map_err(|error| PlayerStartupError::VerificationFailed {
            message: format!(
                "failed to read approved reference BMP '{}': {error}",
                reference_bmp_path.display()
            ),
        })?;
    let reference_image = decode_bmp_bytes(&reference_bytes).map_err(|message| {
        PlayerStartupError::VerificationFailed {
            message: format!(
                "invalid approved reference BMP '{}': {message}",
                reference_bmp_path.display()
            ),
        }
    })?;

    let width = reference_image.width;
    let height = reference_image.height;

    let temp_bundle_dir = std::env::temp_dir().join(format!(
        "unitas_device_verify_bundle_{}",
        std::process::id()
    ));
    let staged = stage_roll_a_ball_bundle(&temp_bundle_dir).map_err(|source| {
        PlayerStartupError::ManifestLoad {
            bundle_dir: temp_bundle_dir.clone(),
            source,
        }
    })?;
    let startup_scene_path = temp_bundle_dir.join(&staged.manifest.startup_scene_staged_path);

    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;
    runtime.load_scene(&startup_scene_path).map_err(|error| {
        PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path,
            message: error.to_string(),
        }
    })?;

    let mut presenter = GpuFramePresenter::from_new_renderer().map_err(|error| {
        PlayerStartupError::RenderFailed {
            message: error.to_string(),
        }
    })?;
    let mut controller =
        RollABallController::bind_to_scene(&mut runtime, &mut presenter).map_err(|error| {
            PlayerStartupError::AudioFailed {
                message: error.to_string(),
            }
        })?;

    let initial_ball_position = controller
        .player_entity
        .and_then(|ent| runtime.body_position(ent))
        .unwrap_or([0.0, 0.5, 0.0]);

    runtime
        .tick(1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;
    controller
        .step(&mut runtime, &mut presenter, 1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;

    let first_gpu_frame = presenter
        .render_offscreen(&runtime, width, height)
        .map_err(|error| PlayerStartupError::RenderFailed {
            message: error.to_string(),
        })?;
    let mean_channel_error = mean_image_channel_error(&first_gpu_frame, &reference_image)
        .map_err(|message| PlayerStartupError::VerificationFailed { message })?;

    let tolerance = GPU_CPU_MEAN_ERROR_TOLERANCE;
    if mean_channel_error > tolerance {
        let _ = runtime.shutdown();
        let _ = fs::remove_dir_all(&temp_bundle_dir);
        return Err(PlayerStartupError::VerificationFailed {
            message: format!(
                "GPU frame mean channel error {mean_channel_error:.3} exceeds tolerance {tolerance:.3} against '{}'",
                reference_bmp_path.display()
            ),
        });
    }

    let _ = runtime.shutdown();

    let options = WindowedLaunchOptions {
        width,
        height,
        max_frames: Some(frames.max(4)),
        fixed_delta_seconds: Some(1.0 / 60.0),
        capture_last_frame: true,
        start_audio_device: true,
        inject_demo_input: true,
    };

    let windowed_result = launch_bundle_windowed(&temp_bundle_dir, &options)?;
    let _ = fs::remove_dir_all(&temp_bundle_dir);

    let dx = windowed_result.final_ball_position[0] - initial_ball_position[0];
    let dz = windowed_result.final_ball_position[2] - initial_ball_position[2];
    let ball_moved = (dx * dx + dz * dz).sqrt() > 0.01;
    if !ball_moved {
        return Err(PlayerStartupError::VerificationFailed {
            message: format!(
                "synthetic keyboard/mouse input did not move PlayerBall (initial={initial_ball_position:?}, final={:?})",
                windowed_result.final_ball_position
            ),
        });
    }

    Ok(DeviceVerifyReport {
        width,
        height,
        frames_presented: windowed_result.frames_presented,
        mean_channel_error,
        tolerance,
        audio_device_active: windowed_result.audio_device_active,
        audio_device_status: windowed_result.audio_device_status,
        audio_frames_pushed: windowed_result.audio_frames_pushed,
        ball_moved,
        initial_ball_position,
        final_ball_position: windowed_result.final_ball_position,
        score: windowed_result.score,
        reference_path: reference_bmp_path.to_path_buf(),
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
            Some(String::from(
                "rebuild the output bundle to regenerate a valid manifest",
            )),
        )],
    }
}
