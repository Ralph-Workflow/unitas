use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use unitas_build::{
    capture_sample_reference_bmp, launch_bundle, launch_bundle_windowed, run_device_verification,
    stage_roll_a_ball_bundle, WindowedLaunchOptions, DEFAULT_DEVICE_VERIFY_HEIGHT,
    DEFAULT_DEVICE_VERIFY_WIDTH,
};

#[derive(Debug, Clone, PartialEq)]
struct CliOptions {
    bundle_dir: Option<PathBuf>,
    sample: bool,
    windowed: bool,
    frames: Option<usize>,
    delta_seconds: f64,
    width: u32,
    height: u32,
    capture_reference: Option<PathBuf>,
    verify_device: Option<PathBuf>,
    demo_input: bool,
}

fn parse_cli_args<I>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut bundle_dir: Option<PathBuf> = None;
    let mut sample = false;
    let mut windowed = false;
    let mut frames: Option<usize> = None;
    let mut delta_seconds: f64 = 1.0 / 60.0;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut capture_reference: Option<PathBuf> = None;
    let mut verify_device: Option<PathBuf> = None;
    let mut demo_input = false;

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
            "--demo-input" => {
                demo_input = true;
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

    if bundle_dir.is_none() && !sample && capture_reference.is_none() && verify_device.is_none() {
        return Err(String::from(
            "usage: unitas-player [<bundle-dir> | --sample] [--windowed] [--frames <N>] [--dt <seconds>] [--verify-device <ref.bmp>] [--capture-reference <out.bmp>]",
        ));
    }

    Ok(CliOptions {
        bundle_dir,
        sample,
        windowed,
        frames,
        delta_seconds,
        width,
        height,
        capture_reference,
        verify_device,
        demo_input,
    })
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

    if let Some(ref reference_out) = cli.capture_reference {
        let cap_w = if cli.width == 1280 {
            DEFAULT_DEVICE_VERIFY_WIDTH
        } else {
            cli.width
        };
        let cap_h = if cli.height == 720 {
            DEFAULT_DEVICE_VERIFY_HEIGHT
        } else {
            cli.height
        };
        return match capture_sample_reference_bmp(reference_out, cap_w, cap_h) {
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
                ExitCode::from(1)
            }
        };
    }

    if let Some(ref reference_path) = cli.verify_device {
        let verify_frames = cli.frames.unwrap_or(12) as u64;
        return match run_device_verification(reference_path, verify_frames) {
            Ok(report) => {
                println!(
                    "device_verify=ok reference={} size={}x{} frames_presented={} mean_error={:.4} tolerance={:.4} audio_active={} audio_status=\"{}\" audio_frames={} ball_moved={} final_pos=({:.3},{:.3},{:.3}) score={}",
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
                    report.score
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("unitas-player startup failed: {error}");
                ExitCode::from(1)
            }
        };
    }

    let mut temp_sample_dir: Option<PathBuf> = None;
    let resolved_bundle_dir = match cli.bundle_dir {
        Some(ref dir) if cli.sample && !dir.exists() => {
            if let Err(error) = stage_roll_a_ball_bundle(dir) {
                eprintln!("unitas-player startup failed: {error}");
                return ExitCode::from(1);
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
                return ExitCode::from(1);
            }
            temp_sample_dir = Some(temp_dir.clone());
            temp_dir
        }
    };

    let run_headless = !cli.windowed && cli.frames.is_some();

    let exit_code = if !run_headless {
        let options = WindowedLaunchOptions {
            width: cli.width,
            height: cli.height,
            max_frames: cli.frames.map(|n| n as u64),
            fixed_delta_seconds: Some(cli.delta_seconds),
            capture_last_frame: false,
            start_audio_device: true,
            inject_demo_input: cli.demo_input,
        };
        match launch_bundle_windowed(&resolved_bundle_dir, &options) {
            Ok(result) => {
                println!(
                    "launched_product={} startup_scene={} loaded_scenes={} root_objects={} frames_stepped={} development_diagnostics={} mode=windowed frames_presented={} audio_status=\"{}\" audio_frames={} score={}/{}",
                    result.launch.manifest.product_name,
                    result.launch.startup_scene_path.display(),
                    result.launch.loaded_scene_count,
                    result.launch.root_object_count,
                    result.launch.frames_stepped,
                    result.launch.development_diagnostics_enabled,
                    result.frames_presented,
                    result.audio_device_status,
                    result.audio_frames_pushed,
                    result.score,
                    result.total_pickups
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("unitas-player startup failed: {error}");
                ExitCode::from(1)
            }
        }
    } else {
        let frames = cli.frames.unwrap_or(1);
        match launch_bundle(&resolved_bundle_dir, frames, cli.delta_seconds) {
            Ok(result) => {
                println!(
                    "launched_product={} startup_scene={} loaded_scenes={} root_objects={} frames_stepped={} development_diagnostics={}",
                    result.manifest.product_name,
                    result.startup_scene_path.display(),
                    result.loaded_scene_count,
                    result.root_object_count,
                    result.frames_stepped,
                    result.development_diagnostics_enabled
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("unitas-player startup failed: {error}");
                ExitCode::from(1)
            }
        }
    };

    if let Some(dir) = temp_sample_dir {
        let _ = fs::remove_dir_all(dir);
    }
    exit_code
}
