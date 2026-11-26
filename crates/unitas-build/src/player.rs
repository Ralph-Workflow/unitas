use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
#[cfg(feature = "device")]
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use unitas_asset::project::{build_compatibility_report, unity_project_root, Project};
use unitas_asset::trust::TrustStore;
use unitas_render::Image;
#[cfg(feature = "device")]
use unitas_render::TextOverlayItem;
#[cfg(feature = "device")]
use unitas_render::GPU_CPU_MEAN_ERROR_TOLERANCE;
#[cfg(feature = "device")]
use unitas_runtime::window::{run_windowed_gpu, GpuFramePresenter, WindowConfig};
#[cfg(feature = "device")]
use unitas_runtime::KEY_RIGHT_ARROW;

use crate::bundle::{
    content_hash_hex, stage_bundle_for_host, BundleError, BundleManifest, ContentBuildResult,
    StagedBundle, BUNDLE_MANIFEST_FILENAME, CONTENT_CATALOG_FILENAME,
    DEVELOPMENT_DIAGNOSTICS_MARKER_FILENAME,
};
use crate::config::BuildConfiguration;
use crate::report::{
    BuildDiagnostic, BuildReport, CompatibilityFinding, SizeCategoryEntry, SizeContributor,
    StartupMeasurement, STARTUP_BASIS,
};
#[cfg(feature = "device")]
use crate::sample::stage_roll_a_ball_bundle;
use crate::targets::BuildTarget;
use crate::toolchain::target_availability_report;

pub const BUILD_REPORT_FILENAME: &str = "build-report.json";
pub const DEFAULT_DEVICE_VERIFY_WIDTH: u32 = 320;
pub const DEFAULT_DEVICE_VERIFY_HEIGHT: u32 = 240;
pub const STARTUP_FAILURE_EXIT_CODE: u8 = 1;
pub const STARTUP_MEASUREMENT_TIMEOUT_SECONDS: u64 = 20;

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
    RuntimeLibraryUnavailable {
        path: PathBuf,
    },
    ScriptHostBootFailed {
        message: String,
    },
    ScriptHostInitFailed {
        stage: i32,
    },
    ScriptHostTickFailed {
        frame: usize,
        stage: i32,
    },
    ScriptHostShutdownFailed {
        stage: i32,
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
            PlayerStartupError::RuntimeLibraryUnavailable { path } => {
                write!(
                    formatter,
                    "the runtime library does not exist at '{}'",
                    path.display()
                )
            }
            PlayerStartupError::ScriptHostBootFailed { message } => {
                write!(formatter, "the script host failed to boot: {message}")
            }
            PlayerStartupError::ScriptHostInitFailed { stage } => {
                write!(
                    formatter,
                    "the script host failed to initialize: stage {stage}"
                )
            }
            PlayerStartupError::ScriptHostTickFailed { frame, stage } => {
                write!(
                    formatter,
                    "the script host failed to tick frame {frame}: stage {stage}"
                )
            }
            PlayerStartupError::ScriptHostShutdownFailed { stage } => {
                write!(
                    formatter,
                    "the script host failed to shut down: stage {stage}"
                )
            }
        }
    }
}

impl std::error::Error for PlayerStartupError {}

pub fn window_failure<E: fmt::Display>(error: E) -> PlayerStartupError {
    PlayerStartupError::WindowFailed {
        message: error.to_string(),
    }
}

pub fn render_failure<E: fmt::Display>(error: E) -> PlayerStartupError {
    PlayerStartupError::RenderFailed {
        message: error.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptHost {
    CoreClr,
    None,
}

impl ScriptHost {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoreClr => "coreclr",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLaunchResult {
    pub manifest: BundleManifest,
    pub startup_scene_path: PathBuf,
    pub loaded_scene_count: usize,
    pub root_object_count: usize,
    pub frames_stepped: usize,
    pub development_diagnostics_enabled: bool,
    pub script_host: ScriptHost,
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
    pub won: bool,
    pub hud: bool,
    pub hud_text: String,
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
    pub total: u32,
    pub won: bool,
    pub hud: bool,
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
    let development_diagnostics_enabled = manifest.development_diagnostics;

    if let Some(scripted) =
        crate::scripted::launch_scripted(bundle_dir, &startup_scene_path, frames, delta_seconds)?
    {
        return Ok(PlayerLaunchResult {
            manifest,
            startup_scene_path,
            loaded_scene_count: scripted.loaded_scene_count,
            root_object_count: scripted.root_object_count,
            frames_stepped: scripted.frames_stepped,
            development_diagnostics_enabled,
            script_host: ScriptHost::CoreClr,
        });
    }

    let native = launch_native_scene(&startup_scene_path, frames, delta_seconds)?;
    Ok(PlayerLaunchResult {
        manifest,
        startup_scene_path,
        loaded_scene_count: native.loaded_scene_count,
        root_object_count: native.root_object_count,
        frames_stepped: native.frames_stepped,
        development_diagnostics_enabled,
        script_host: ScriptHost::None,
    })
}

struct NativeSceneLaunch {
    loaded_scene_count: usize,
    root_object_count: usize,
    frames_stepped: usize,
}

fn launch_native_scene(
    startup_scene_path: &Path,
    frames: usize,
    delta_seconds: f64,
) -> Result<NativeSceneLaunch, PlayerStartupError> {
    let mut runtime =
        unitas_runtime::Runtime::new().map_err(|error| PlayerStartupError::RuntimeInit {
            message: error.to_string(),
        })?;

    runtime.load_scene(startup_scene_path).map_err(|error| {
        PlayerStartupError::SceneLoadFailed {
            scene_path: startup_scene_path.to_path_buf(),
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
    let _ = runtime.shutdown();

    Ok(NativeSceneLaunch {
        loaded_scene_count,
        root_object_count,
        frames_stepped: frames,
    })
}

pub fn apply_content_update_to_player(
    player_bundle_dir: &Path,
    content_update: &ContentBuildResult,
    frames: usize,
    delta_seconds: f64,
) -> Result<PlayerLaunchResult, PlayerStartupError> {
    let (mut manifest, _startup_scene_path) = load_and_validate_bundle_manifest(player_bundle_dir)?;
    if !content_update
        .catalog
        .is_compatible_with_player(&manifest.version, manifest.target)
    {
        return Err(PlayerStartupError::VerificationFailed {
            message: format!(
                "content update catalog '{}' (target '{}', compatibility '{}') is incompatible with shipped player version '{}' on target '{}'",
                content_update.catalog.catalog_id,
                content_update.catalog.target,
                content_update.catalog.player_compatibility_version,
                manifest.version,
                manifest.target
            ),
        });
    }

    for emitted in &content_update.emitted_bundles {
        let updated_bundle_path = content_update.output_dir.join(&emitted.staged_path);
        let bytes =
            fs::read(&updated_bundle_path).map_err(|error| PlayerStartupError::ManifestLoad {
                bundle_dir: player_bundle_dir.to_path_buf(),
                source: BundleError::Io {
                    path: updated_bundle_path.clone(),
                    message: error.to_string(),
                },
            })?;
        let dest_content_bundle = player_bundle_dir.join(&emitted.staged_path);
        if let Some(parent) = dest_content_bundle.parent() {
            fs::create_dir_all(parent).map_err(|error| PlayerStartupError::ManifestLoad {
                bundle_dir: player_bundle_dir.to_path_buf(),
                source: BundleError::Io {
                    path: parent.to_path_buf(),
                    message: error.to_string(),
                },
            })?;
        }
        fs::write(&dest_content_bundle, &bytes).map_err(|error| {
            PlayerStartupError::ManifestLoad {
                bundle_dir: player_bundle_dir.to_path_buf(),
                source: BundleError::Io {
                    path: dest_content_bundle,
                    message: error.to_string(),
                },
            }
        })?;

        for entry in &mut manifest.entries {
            if entry.source_path == emitted.source_path
                || entry.staged_path.ends_with(&emitted.source_path)
            {
                let staged_scene_dest = player_bundle_dir.join(&entry.staged_path);
                fs::write(&staged_scene_dest, &bytes).map_err(|error| {
                    PlayerStartupError::ManifestLoad {
                        bundle_dir: player_bundle_dir.to_path_buf(),
                        source: BundleError::Io {
                            path: staged_scene_dest,
                            message: error.to_string(),
                        },
                    }
                })?;
                entry.content_hash = emitted.content_hash.clone();
                entry.size_bytes = emitted.size_bytes;
                entry.root_object_count = emitted.root_object_count;
            }
        }
    }

    manifest.total_content_bytes = manifest
        .entries
        .iter()
        .fold(0u64, |acc, item| acc.saturating_add(item.size_bytes));
    manifest.manifest_hash = manifest.compute_manifest_hash();
    let manifest_path = player_bundle_dir.join(BUNDLE_MANIFEST_FILENAME);
    fs::write(&manifest_path, manifest.to_canonical_string()).map_err(|error| {
        PlayerStartupError::ManifestLoad {
            bundle_dir: player_bundle_dir.to_path_buf(),
            source: BundleError::Io {
                path: manifest_path,
                message: error.to_string(),
            },
        }
    })?;

    let catalog_dest = player_bundle_dir.join(CONTENT_CATALOG_FILENAME);
    fs::write(&catalog_dest, content_update.catalog.to_canonical_string()).map_err(|error| {
        PlayerStartupError::ManifestLoad {
            bundle_dir: player_bundle_dir.to_path_buf(),
            source: BundleError::Io {
                path: catalog_dest,
                message: error.to_string(),
            },
        }
    })?;

    launch_bundle(player_bundle_dir, frames, delta_seconds)
}

#[cfg(feature = "device")]
pub fn launch_bundle_windowed(
    bundle_dir: &Path,
    options: &WindowedLaunchOptions,
) -> Result<WindowedPlayerResult, PlayerStartupError> {
    let (manifest, startup_scene_path) = load_and_validate_bundle_manifest(bundle_dir)?;

    let scripted = crate::scripted::ScriptedPlayer::boot(bundle_dir, &startup_scene_path)?
        .map(|player| Arc::new(Mutex::new(player)));

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

    let mut presenter = GpuFramePresenter::from_new_renderer().map_err(render_failure)?;

    let (audio_device_active, audio_device_status) = if options.start_audio_device {
        match presenter.start_default_audio_output() {
            Ok(()) => (true, String::from("active")),
            Err(error) => (false, format!("platform-limited ({error})")),
        }
    } else {
        (false, String::from("disabled"))
    };

    let width = options.width.max(1);
    let height = options.height.max(1);

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

    run_windowed_scripted(
        &mut runtime,
        &window_config,
        &mut presenter,
        scripted.clone(),
        options.inject_demo_input,
    )
    .map_err(window_failure)?;

    let frames_presented = presenter.frames_presented();

    let captured_frame = if options.capture_last_frame {
        Some(
            presenter
                .render_offscreen(&runtime, width, height)
                .map_err(render_failure)?,
        )
    } else {
        None
    };

    let (score, total_pickups, won, hud_text) =
        match scripted.as_ref().and_then(|player| player.lock().ok()) {
            Some(player) => (
                player.score(),
                player.total(),
                player.won(),
                player.hud_text(),
            ),
            None => (0, 0, false, String::new()),
        };

    let hud = presenter.overlay_drawn();
    let loaded_scene_count = runtime.scene_count();
    let root_object_count = runtime.scene_graph().map_or(0, |graph| graph.roots().len());
    let development_diagnostics_enabled = manifest.development_diagnostics;
    let audio_frames_pushed = presenter.total_audio_frames_pushed();
    let script_host = if scripted.is_some() {
        ScriptHost::CoreClr
    } else {
        ScriptHost::None
    };

    let _ = runtime.shutdown();
    if let Some(player) = scripted {
        if let Ok(mut player) = player.lock() {
            let _ = player.shutdown();
        }
    }

    Ok(WindowedPlayerResult {
        launch: PlayerLaunchResult {
            manifest,
            startup_scene_path,
            loaded_scene_count,
            root_object_count,
            frames_stepped: frames_presented as usize,
            development_diagnostics_enabled,
            script_host,
        },
        frames_presented,
        audio_device_active,
        audio_device_status,
        audio_frames_pushed,
        final_ball_position: [0.0, 0.5, 0.0],
        score,
        total_pickups,
        won,
        hud,
        hud_text,
        captured_frame,
    })
}

#[cfg(feature = "device")]
fn script_hud_overlay_items(
    hud_text: &str,
    won: bool,
    width: u32,
    height: u32,
) -> Vec<TextOverlayItem> {
    let mut items = Vec::new();
    if !hud_text.is_empty() {
        items.push(TextOverlayItem {
            text: hud_text.to_string(),
            x: 16.0,
            y: 16.0,
            size_px: 28.0,
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    if won {
        items.push(TextOverlayItem {
            text: String::from("You win!"),
            x: (width as f32) * 0.5 - 90.0,
            y: (height as f32) * 0.5 - 24.0,
            size_px: 48.0,
            color: [1.0, 0.85, 0.2, 1.0],
        });
    }
    items
}

#[cfg(feature = "device")]
fn run_windowed_scripted(
    runtime: &mut unitas_runtime::Runtime,
    window_config: &WindowConfig,
    presenter: &mut GpuFramePresenter<'_>,
    scripted: Option<Arc<Mutex<crate::scripted::ScriptedPlayer>>>,
    inject_demo_input: bool,
) -> Result<u64, unitas_runtime::window::WindowError> {
    let shared_overlay = Arc::new(Mutex::new(Vec::new()));
    let forwarded_keys: Vec<u32> = (0u32..=127).chain(273u32..=276).collect();
    let mut key_down = vec![false; forwarded_keys.len()];
    let mut mouse_down = [false; 3];

    presenter.set_overlay_source(Arc::clone(&shared_overlay));

    {
        let overlay = Arc::clone(&shared_overlay);
        let cb_player = scripted.clone();
        presenter.set_update_callback(move |rt, frame| {
            if inject_demo_input {
                rt.input_mut().inject_key(u32::from(b'w'), true);
                rt.input_mut().inject_key(KEY_RIGHT_ARROW, true);
                rt.input_mut()
                    .inject_pointer_move((frame.frame_index as f64) * 6.0, 3.0);
            }

            let Some(player) = cb_player.as_ref() else {
                return Ok(());
            };
            let Ok(mut player) = player.lock() else {
                return Ok(());
            };

            for (index, code) in forwarded_keys.iter().enumerate() {
                let down = rt.input_key_state(*code) & unitas_runtime::STATE_DOWN != 0;
                if down != key_down[index] {
                    key_down[index] = down;
                    player.inject_input(0, u64::from(*code), down, 0.0, 0.0);
                }
            }

            for button in 0..3u32 {
                let down = rt.input_mouse_state(button) & unitas_runtime::STATE_DOWN != 0;
                if down != mouse_down[button as usize] {
                    mouse_down[button as usize] = down;
                    player.inject_input(1, u64::from(button), down, 0.0, 0.0);
                }
            }

            let (pointer_x, pointer_y, _, _) = rt.input_pointer();
            player.inject_input(2, 0, false, pointer_x, pointer_y);

            player
                .tick(frame.delta_seconds)
                .map_err(|error| unitas_runtime::window::WindowError::Runtime(error.to_string()))?;

            let hud_text = player.hud_text();
            let won = player.won();
            let published = publish_script_hud(player.handle(), &hud_text);
            if let Ok(mut items) = overlay.lock() {
                *items = script_hud_overlay_items(&published, won, frame.width, frame.height);
            }
            Ok(())
        });
    }

    let result = run_windowed_gpu(runtime, window_config, presenter);
    presenter.set_update_callback(|_, _| Ok(()));
    result.map(|()| presenter.frames_presented())
}

#[cfg(feature = "device")]
fn publish_script_hud(handle: u64, hud_text: &str) -> String {
    if let Some(address) = unitas_runtime::abi_player::write_player_hud_buffer(handle, hud_text) {
        let _ = unitas_runtime::abi_player::unitas_runtime_player_publish_hud(
            handle,
            address,
            hud_text.len() as u64,
        );
    }
    unitas_runtime::abi_player::player_hud_text(handle).unwrap_or_default()
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
    let presenter = GpuFramePresenter::from_new_renderer().map_err(render_failure)?;

    runtime
        .tick(1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;

    let image = presenter
        .render_offscreen(&runtime, width.max(1), height.max(1))
        .map_err(render_failure)?;
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

    let presenter = GpuFramePresenter::from_new_renderer().map_err(render_failure)?;

    let initial_ball_position = runtime
        .find_entity_by_name("PlayerBall")
        .and_then(|ent| runtime.body_position(ent))
        .unwrap_or([0.0, 0.5, 0.0]);

    runtime
        .tick(1.0 / 60.0)
        .map_err(|error| PlayerStartupError::FrameTickFailed {
            frame: 0,
            message: error.to_string(),
        })?;

    let first_gpu_frame = presenter
        .render_offscreen(&runtime, width, height)
        .map_err(render_failure)?;
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
        total: windowed_result.total_pickups,
        won: windowed_result.won,
        hud: windowed_result.hud,
        reference_path: reference_bmp_path.to_path_buf(),
    })
}

/// Measures the time from launching a packaged player until it answers its
/// first control request.
pub fn measure_player_startup(launch_artifact: &Path) -> StartupMeasurement {
    let mut child = match Command::new(launch_artifact)
        .arg("--control-stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return StartupMeasurement::unavailable(
                format!(
                    "the player at '{}' did not start: {error}",
                    launch_artifact.display()
                ),
                STARTUP_BASIS,
            );
        }
    };
    let started = Instant::now();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"{\"cmd\":\"state\"}\n");
        let _ = stdin.flush();
    }
    let stdout = child.stdout.take();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let answer = stdout.and_then(|stdout| {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader.read_line(&mut line).ok().map(|read| (read, line))
        });
        let _ = sender.send(answer);
    });
    let answered = receiver.recv_timeout(Duration::from_secs(STARTUP_MEASUREMENT_TIMEOUT_SECONDS));
    let elapsed = started.elapsed();
    let _ = child.kill();
    let _ = child.wait();
    match answered {
        Ok(Some((read, line))) if read > 0 && line.trim_start().starts_with('{') => {
            StartupMeasurement::measured(
                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
                STARTUP_BASIS,
            )
        }
        Ok(Some((_, line))) => StartupMeasurement::unavailable(
            format!("the player answered with '{line}' instead of a control response"),
            STARTUP_BASIS,
        ),
        Ok(None) => StartupMeasurement::unavailable(
            "the player exited without answering a control request",
            STARTUP_BASIS,
        ),
        Err(_) => StartupMeasurement::unavailable(
            format!(
                "the player did not answer within {STARTUP_MEASUREMENT_TIMEOUT_SECONDS} seconds"
            ),
            STARTUP_BASIS,
        ),
    }
}

/// Measures the startup of the player a staged bundle produced, when the
/// bundle carries a launchable player on this host.
pub fn measure_packaged_startup(staged: &StagedBundle) -> StartupMeasurement {
    if staged.application_bundle_path.is_none() {
        return StartupMeasurement::unavailable(
            format!(
                "the {} target produced no launchable player on this host",
                staged.manifest.target
            ),
            STARTUP_BASIS,
        );
    }
    measure_player_startup(&staged.launch_artifact_path)
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
            )
            .with_compatibility_findings(project_compatibility_findings(config))
            .with_startup_time(measure_packaged_startup(&staged))
            .with_target_availability(target_availability_report());

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

fn project_compatibility_findings(config: &BuildConfiguration) -> Vec<CompatibilityFinding> {
    let Some(scene) = config.scenes.first() else {
        return Vec::new();
    };
    let Some(root) = unity_project_root(scene) else {
        return Vec::new();
    };
    let trust_store = TrustStore::new(
        std::env::temp_dir().join(format!("unitas_build_trust_{}", std::process::id())),
    );
    let Ok((_project, open_report)) = Project::open(&root, &trust_store) else {
        return Vec::new();
    };
    let assessment = build_compatibility_report(&root, &open_report);
    assessment
        .asset_findings
        .iter()
        .chain(assessment.package_findings.iter())
        .chain(assessment.setting_findings.iter())
        .filter_map(CompatibilityFinding::from_project_entry)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::stage_bundle_for_host;
    use crate::config::BuildConfiguration;

    #[test]
    fn headless_launch_of_a_bundle_without_a_managed_payload_runs_the_native_loop(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = std::env::temp_dir().join(format!(
            "unitas_player_native_loop_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&workspace);
        let scene_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scene_alpha.unity");
        let config = BuildConfiguration::for_host(vec![scene_path], "NativeLoopGame");
        stage_bundle_for_host(&config, &workspace, BuildTarget::current_host())?;

        let result = launch_bundle(&workspace, 4, 1.0 / 60.0)?;
        assert_eq!(result.frames_stepped, 4);
        assert_eq!(result.script_host, ScriptHost::None);
        assert_eq!(result.loaded_scene_count, 1);
        assert!(result.root_object_count >= 1);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }
}
