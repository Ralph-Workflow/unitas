use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::project::unity_project_root;
use unitas_asset::trust::{
    per_user_trust_store_path, require_code_execution, ProjectCapabilities, TrustRequiredError,
    TrustStore,
};
use unitas_build::control;
use unitas_build::package::package_desktop_targets;
use unitas_build::player::STARTUP_FAILURE_EXIT_CODE;
#[cfg(feature = "device")]
use unitas_build::{
    capture_sample_reference_bmp, launch_bundle_windowed, run_device_verification,
    DEFAULT_DEVICE_VERIFY_HEIGHT, DEFAULT_DEVICE_VERIFY_WIDTH,
};
use unitas_build::{
    launch_bundle, load_and_validate_bundle_manifest, roll_a_ball_build_configuration,
    stage_roll_a_ball_bundle, BuildReport, WindowedLaunchOptions, BUILD_REPORT_FILENAME,
};
use unitas_dotnet::install_code_execution_capabilities;

const USAGE: &str = "usage: unitas-player [<bundle-dir> | --sample] [--windowed] [--frames <N>] [--dt <seconds>] [--verify-device <ref.bmp>] [--capture-reference <out.bmp>] [--control-stdio] [--package-sample <dir>]";

const TRUST_REQUIRED_EXIT_CODE: u8 = 3;

const PLAYER_LAUNCH_OPERATION: &str = "player launch";

#[derive(Debug, Clone, PartialEq)]
struct CliOptions {
    bundle_dir: Option<PathBuf>,
    sample: bool,
    windowed: bool,
    package_sample: Option<PathBuf>,
    frames: Option<usize>,
    delta_seconds: f64,
    width: u32,
    height: u32,
    capture_reference: Option<PathBuf>,
    verify_device: Option<PathBuf>,
    demo_input: bool,
    control_stdio: bool,
}

fn parse_cli_args<I>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut bundle_dir: Option<PathBuf> = None;
    let mut sample = false;
    let mut windowed = false;
    let mut package_sample: Option<PathBuf> = None;
    let mut frames: Option<usize> = None;
    let mut delta_seconds: f64 = 1.0 / 60.0;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut capture_reference: Option<PathBuf> = None;
    let mut verify_device: Option<PathBuf> = None;
    let mut demo_input = false;
    let mut control_stdio = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--frames" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for '--frames'"))?;
                let parsed = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid '--frames' value '{raw}': {err}"))?;
                frames = Some(parsed);
            }
            "--dt" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for '--dt'"))?;
                delta_seconds = raw
                    .parse::<f64>()
                    .map_err(|err| format!("invalid '--dt' value '{raw}': {err}"))?;
                if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
                    return Err(format!(
                        "invalid '--dt' value '{raw}': must be a positive finite number"
                    ));
                }
            }
            "--width" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for '--width'"))?;
                width = raw
                    .parse::<u32>()
                    .map_err(|err| format!("invalid '--width' value '{raw}': {err}"))?;
                if width == 0 {
                    return Err(String::from(
                        "invalid '--width' value '0': must be positive",
                    ));
                }
            }
            "--height" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for '--height'"))?;
                height = raw
                    .parse::<u32>()
                    .map_err(|err| format!("invalid '--height' value '{raw}': {err}"))?;
                if height == 0 {
                    return Err(String::from(
                        "invalid '--height' value '0': must be positive",
                    ));
                }
            }
            "--sample" => {
                sample = true;
            }
            "--windowed" => {
                windowed = true;
            }
            "--package-sample" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing directory for '--package-sample'"))?;
                package_sample = Some(PathBuf::from(raw));
            }
            "--demo-input" => {
                demo_input = true;
            }
            "--control-stdio" => {
                control_stdio = true;
            }
            "--capture-reference" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing path for '--capture-reference'"))?;
                capture_reference = Some(PathBuf::from(raw));
            }
            "--verify-device" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing path for '--verify-device'"))?;
                verify_device = Some(PathBuf::from(raw));
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unrecognized argument '{flag}'"));
            }
            positional => {
                if bundle_dir.is_some() {
                    return Err(format!(
                        "unexpected extra positional argument '{positional}'"
                    ));
                }
                bundle_dir = Some(PathBuf::from(positional));
            }
        }
    }

    if bundle_dir.is_none()
        && !sample
        && capture_reference.is_none()
        && verify_device.is_none()
        && !control_stdio
        && package_sample.is_none()
    {
        return Err(String::from(USAGE));
    }

    Ok(CliOptions {
        bundle_dir,
        sample,
        windowed,
        package_sample,
        frames,
        delta_seconds,
        width,
        height,
        capture_reference,
        verify_device,
        demo_input,
        control_stdio,
    })
}

fn run_package_sample(output_dir: &Path) -> ExitCode {
    let config = match roll_a_ball_build_configuration(None) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("unitas-player packaging failed: {error}");
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };
    let packaged = match package_desktop_targets(&config, output_dir) {
        Ok(packaged) => packaged,
        Err(error) => {
            eprintln!("unitas-player packaging failed: {error}");
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };
    for target in &packaged {
        println!(
            "packaged_app[{}]={}",
            target.label,
            target.app_path.display()
        );
        println!(
            "build_report[{}]={}",
            target.label,
            target.build_report_path.display()
        );
        println!(
            "launch_artifact[{}]={}",
            target.label,
            target.launch_artifact.display()
        );
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args = env::args().skip(1);
    let cli = match parse_cli_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("unitas-player argument error: {message}");
            return ExitCode::from(2);
        }
    };

    if let Some(ref output_dir) = cli.package_sample {
        return run_package_sample(output_dir);
    }

    if cli.control_stdio {
        return run_control_stdio(&cli);
    }

    if let Some(ref reference_out) = cli.capture_reference {
        return run_capture_reference(reference_out, cli.width, cli.height);
    }

    if let Some(ref reference_path) = cli.verify_device {
        return run_verify_device(reference_path, cli.frames.unwrap_or(12) as u64);
    }

    let mut temp_sample_dir: Option<PathBuf> = None;
    let resolved_bundle_dir = match cli.bundle_dir {
        Some(ref dir) if cli.sample && !dir.exists() => {
            if let Err(error) = stage_roll_a_ball_bundle(dir) {
                eprintln!("unitas-player startup failed: {error}");
                return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
            }
            dir.clone()
        }
        Some(ref dir) => dir.clone(),
        None => {
            let temp_dir = std::env::temp_dir().join(format!(
                "unitas_player_sample_bundle_{}",
                std::process::id()
            ));
            if let Err(error) = stage_roll_a_ball_bundle(&temp_dir) {
                eprintln!("unitas-player startup failed: {error}");
                return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
            }
            temp_sample_dir = Some(temp_dir.clone());
            temp_dir
        }
    };

    let run_headless =
        !cli.windowed && (cli.frames.is_some() || (!cli.sample && cli.bundle_dir.is_some()));

    if let Err(error) = install_project_trust(&resolved_bundle_dir) {
        eprintln!("unitas-player refused to run project code: {error}");
        return ExitCode::from(TRUST_REQUIRED_EXIT_CODE);
    }

    let exit_code = if run_headless {
        let frames = cli.frames.unwrap_or(1);
        match launch_bundle(&resolved_bundle_dir, frames, cli.delta_seconds) {
            Ok(result) => {
                println!(
                    "launched_product={} startup_scene={} loaded_scenes={} root_objects={} frames_stepped={} development_diagnostics={} script_host={} hud=false",
                    result.manifest.product_name,
                    result.startup_scene_path.display(),
                    result.loaded_scene_count,
                    result.root_object_count,
                    result.frames_stepped,
                    result.development_diagnostics_enabled,
                    result.script_host.as_str()
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("unitas-player startup failed: {error}");
                ExitCode::from(STARTUP_FAILURE_EXIT_CODE)
            }
        }
    } else {
        let options = WindowedLaunchOptions {
            width: cli.width,
            height: cli.height,
            max_frames: cli.frames.map(|n| n as u64),
            fixed_delta_seconds: Some(cli.delta_seconds),
            capture_last_frame: false,
            start_audio_device: true,
            inject_demo_input: cli.demo_input,
        };
        run_windowed(&resolved_bundle_dir, &options)
    };

    if let Some(dir) = temp_sample_dir {
        let _ = fs::remove_dir_all(dir);
    }
    exit_code
}

#[cfg(feature = "device")]
fn run_windowed(bundle_dir: &Path, options: &WindowedLaunchOptions) -> ExitCode {
    match launch_bundle_windowed(bundle_dir, options) {
        Ok(result) => {
            println!(
                "launched_product={} startup_scene={} loaded_scenes={} root_objects={} frames_stepped={} development_diagnostics={} script_host={} mode=windowed frames_presented={} audio_status=\"{}\" audio_frames={} score={}/{} won={} hud={} hud_text=\"{}\"",
                result.launch.manifest.product_name,
                result.launch.startup_scene_path.display(),
                result.launch.loaded_scene_count,
                result.launch.root_object_count,
                result.launch.frames_stepped,
                result.launch.development_diagnostics_enabled,
                result.launch.script_host.as_str(),
                result.frames_presented,
                result.audio_device_status,
                result.audio_frames_pushed,
                result.score,
                result.total_pickups,
                result.won,
                result.hud,
                result.hud_text
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("unitas-player startup failed: {error}");
            ExitCode::from(STARTUP_FAILURE_EXIT_CODE)
        }
    }
}

#[cfg(not(feature = "device"))]
fn run_windowed(bundle_dir: &Path, _options: &WindowedLaunchOptions) -> ExitCode {
    if let Err(error) = load_and_validate_bundle_manifest(bundle_dir) {
        eprintln!("unitas-player startup failed: {error}");
        return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
    }
    eprintln!("unitas-player: the windowed player requires the device feature");
    ExitCode::from(2)
}

#[cfg(feature = "device")]
fn run_capture_reference(reference_out: &Path, width: u32, height: u32) -> ExitCode {
    let cap_w = if width == 1280 {
        DEFAULT_DEVICE_VERIFY_WIDTH
    } else {
        width
    };
    let cap_h = if height == 720 {
        DEFAULT_DEVICE_VERIFY_HEIGHT
    } else {
        height
    };
    match capture_sample_reference_bmp(reference_out, cap_w, cap_h) {
        Ok(image) => {
            println!(
                "captured_reference={} width={} height={}",
                reference_out.display(),
                image.width,
                image.height
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("unitas-player startup failed: {error}");
            ExitCode::from(STARTUP_FAILURE_EXIT_CODE)
        }
    }
}

#[cfg(not(feature = "device"))]
fn run_capture_reference(_reference_out: &Path, _width: u32, _height: u32) -> ExitCode {
    eprintln!("unitas-player: '--capture-reference' requires the device feature");
    ExitCode::from(2)
}

#[cfg(feature = "device")]
fn run_verify_device(reference_path: &Path, frames: u64) -> ExitCode {
    match run_device_verification(reference_path, frames) {
        Ok(report) => {
            println!(
                "device_verify=ok reference={} size={}x{} frames_presented={} mean_error={:.4} tolerance={:.4} audio_active={} audio_status=\"{}\" audio_frames={} ball_moved={} final_pos=({:.3},{:.3},{:.3}) score={}/{} won={} hud={}",
                report.reference_path.display(),
                report.width,
                report.height,
                report.frames_presented,
                report.mean_channel_error,
                report.tolerance,
                report.audio_device_active,
                report.audio_device_status,
                report.audio_frames_pushed,
                report.ball_moved,
                report.final_ball_position[0],
                report.final_ball_position[1],
                report.final_ball_position[2],
                report.score,
                report.total,
                report.won,
                report.hud
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("unitas-player startup failed: {error}");
            ExitCode::from(STARTUP_FAILURE_EXIT_CODE)
        }
    }
}

#[cfg(not(feature = "device"))]
fn run_verify_device(_reference_path: &Path, _frames: u64) -> ExitCode {
    eprintln!("unitas-player: '--verify-device' requires the device feature");
    ExitCode::from(2)
}

fn run_control_stdio(cli: &CliOptions) -> ExitCode {
    let (bundle_dir, scene_path, staged_dir) = match resolve_control_scene(cli) {
        Ok(resolved) => resolved,
        Err(message) => {
            eprintln!("unitas-player startup failed: {message}");
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };

    if let Err(error) = install_project_trust(&bundle_dir) {
        eprintln!("unitas-player refused to run project code: {error}");
        if let Some(dir) = staged_dir {
            let _ = fs::remove_dir_all(dir);
        }
        return ExitCode::from(TRUST_REQUIRED_EXIT_CODE);
    }

    let mut session = match control::ControlSession::start(
        &bundle_dir,
        &scene_path,
        cli.delta_seconds,
        control_seed(),
    ) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("unitas-player startup failed: {}", error.outcome());
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };

    let exit_code = serve_control_lines(&mut session);

    if let Some(dir) = staged_dir {
        let _ = fs::remove_dir_all(dir);
    }
    exit_code
}

fn resolve_control_scene(cli: &CliOptions) -> Result<(PathBuf, PathBuf, Option<PathBuf>), String> {
    match cli.bundle_dir {
        Some(ref dir) => {
            let (_manifest, startup_scene_path) =
                load_and_validate_bundle_manifest(dir).map_err(|error| error.to_string())?;
            Ok((dir.clone(), startup_scene_path, None))
        }
        None => {
            let staged_dir = env::temp_dir().join(format!(
                "unitas_player_control_bundle_{}",
                std::process::id()
            ));
            let staged =
                stage_roll_a_ball_bundle(&staged_dir).map_err(|error| error.to_string())?;
            let scene_path = staged_dir.join(&staged.manifest.startup_scene_staged_path);
            Ok((staged_dir.clone(), scene_path, Some(staged_dir)))
        }
    }
}

fn serve_control_lines(session: &mut control::ControlSession) -> ExitCode {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("unitas-player control read failed: {error}");
                return ExitCode::from(1);
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = control::handle_line(session, &line);
        if let Err(error) = writeln!(stdout, "{response}") {
            eprintln!("unitas-player control write failed: {error}");
            return ExitCode::from(1);
        }
        if let Err(error) = stdout.flush() {
            eprintln!("unitas-player control flush failed: {error}");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

fn control_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();
    nanos ^ (u64::from(std::process::id()) << 32)
}

fn project_capabilities(bundle_dir: &Path) -> Option<ProjectCapabilities> {
    let report = BuildReport::read_from_file(&bundle_dir.join(BUILD_REPORT_FILENAME)).ok()?;
    let scene_path = report.included_scenes.first()?;
    let project_root = unity_project_root(Path::new(scene_path))?;
    let store = TrustStore::new(per_user_trust_store_path());
    Some(store.capabilities(&project_root.to_string_lossy()))
}

fn install_project_trust(bundle_dir: &Path) -> Result<(), TrustRequiredError> {
    let capabilities = project_capabilities(bundle_dir).unwrap_or(ProjectCapabilities {
        may_run_code: true,
        may_fetch: true,
        restricted: false,
    });
    install_code_execution_capabilities(capabilities);
    require_code_execution(capabilities, PLAYER_LAUNCH_OPERATION)
}
