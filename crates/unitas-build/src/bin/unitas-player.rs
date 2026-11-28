use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_build::control;
use unitas_build::package::{package_desktop_targets, package_web_target};
#[cfg(feature = "device")]
use unitas_build::player::CANVAS_TEXT_SEPARATOR;
use unitas_build::player::{install_project_trust, STARTUP_FAILURE_EXIT_CODE};
#[cfg(feature = "device")]
use unitas_build::{
    capture_sample_reference_bmp, launch_bundle_windowed, run_device_verification,
    DEFAULT_DEVICE_VERIFY_HEIGHT, DEFAULT_DEVICE_VERIFY_WIDTH,
};
use unitas_build::{
    launch_bundle, load_and_validate_bundle_manifest, marble_rush_build_configuration,
    sample::stage_marble_rush_bundle_for_host_with_script_debugging, WindowedLaunchOptions,
};

const USAGE: &str = "usage: unitas-player [<bundle-dir> | --sample] [--windowed] [--frames <N>] [--dt <seconds>] [--verify-device <ref.bmp>] [--capture-reference <out.bmp>] [--control-stdio] [--package-sample <dir>] [--package-web <dir>] [--debug]";

const HELP: &str = "--debug   compile the staged Managed/ payload with script debugging enabled";

const TRUST_REQUIRED_EXIT_CODE: u8 = 3;

#[derive(Debug, Clone, PartialEq)]
struct CliOptions {
    bundle_dir: Option<PathBuf>,
    sample: bool,
    windowed: bool,
    package_sample: Option<PathBuf>,
    package_web: Option<PathBuf>,
    frames: Option<usize>,
    delta_seconds: f64,
    width: u32,
    height: u32,
    capture_reference: Option<PathBuf>,
    verify_device: Option<PathBuf>,
    demo_input: bool,
    control_stdio: bool,
    script_debugging: bool,
    help: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            bundle_dir: None,
            sample: false,
            windowed: false,
            package_sample: None,
            package_web: None,
            frames: None,
            delta_seconds: 1.0 / 60.0,
            width: 1280,
            height: 720,
            capture_reference: None,
            verify_device: None,
            demo_input: false,
            control_stdio: false,
            script_debugging: false,
            help: false,
        }
    }
}

fn parse_cli_args<I>(args: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut options = CliOptions::default();
    while let Some(arg) = iter.next() {
        apply_cli_argument(&mut options, &mut iter, &arg)?;
    }

    if options.bundle_dir.is_none()
        && !options.sample
        && options.capture_reference.is_none()
        && options.verify_device.is_none()
        && !options.control_stdio
        && options.package_sample.is_none()
        && options.package_web.is_none()
        && !options.help
    {
        return Err(String::from(USAGE));
    }

    Ok(options)
}

fn next_value<I>(iter: &mut I, message: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    iter.next().ok_or_else(|| String::from(message))
}

fn apply_cli_argument<I>(options: &mut CliOptions, iter: &mut I, arg: &str) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    match arg {
        "--sample" => options.sample = true,
        "--windowed" => options.windowed = true,
        "--demo-input" => options.demo_input = true,
        "--control-stdio" => options.control_stdio = true,
        "--debug" => options.script_debugging = true,
        "--help" => options.help = true,
        "--package-sample" => {
            let raw = next_value(iter, "missing directory for '--package-sample'")?;
            options.package_sample = Some(PathBuf::from(raw));
        }
        "--package-web" => {
            let raw = next_value(iter, "missing directory for '--package-web'")?;
            options.package_web = Some(PathBuf::from(raw));
        }
        "--capture-reference" => {
            let raw = next_value(iter, "missing path for '--capture-reference'")?;
            options.capture_reference = Some(PathBuf::from(raw));
        }
        "--verify-device" => {
            let raw = next_value(iter, "missing path for '--verify-device'")?;
            options.verify_device = Some(PathBuf::from(raw));
        }
        _ => apply_cli_measured_argument(options, iter, arg)?,
    }
    Ok(())
}

fn apply_cli_measured_argument<I>(
    options: &mut CliOptions,
    iter: &mut I,
    arg: &str,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    match arg {
        "--frames" => {
            let raw = next_value(iter, "missing value for '--frames'")?;
            options.frames = Some(parse_frames(&raw)?);
        }
        "--dt" => {
            let raw = next_value(iter, "missing value for '--dt'")?;
            options.delta_seconds = parse_delta_seconds(&raw)?;
        }
        "--width" => {
            let raw = next_value(iter, "missing value for '--width'")?;
            options.width = parse_dimension(&raw, "--width")?;
        }
        "--height" => {
            let raw = next_value(iter, "missing value for '--height'")?;
            options.height = parse_dimension(&raw, "--height")?;
        }
        flag if flag.starts_with('-') => {
            return Err(format!("unrecognized argument '{flag}'"));
        }
        positional => {
            if options.bundle_dir.is_some() {
                return Err(format!(
                    "unexpected extra positional argument '{positional}'"
                ));
            }
            options.bundle_dir = Some(PathBuf::from(positional));
        }
    }
    Ok(())
}

fn parse_frames(raw: &str) -> Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|err| format!("invalid '--frames' value '{raw}': {err}"))
}

fn parse_delta_seconds(raw: &str) -> Result<f64, String> {
    let delta_seconds = raw
        .parse::<f64>()
        .map_err(|err| format!("invalid '--dt' value '{raw}': {err}"))?;
    if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
        return Err(format!(
            "invalid '--dt' value '{raw}': must be a positive finite number"
        ));
    }
    Ok(delta_seconds)
}

fn parse_dimension(raw: &str, flag: &str) -> Result<u32, String> {
    let value = raw
        .parse::<u32>()
        .map_err(|err| format!("invalid '{flag}' value '{raw}': {err}"))?;
    if value == 0 {
        return Err(format!("invalid '{flag}' value '0': must be positive"));
    }
    Ok(value)
}

fn run_package_sample(output_dir: &Path, script_debugging: bool) -> ExitCode {
    let config = match marble_rush_build_configuration(None) {
        Ok(config) => config.with_script_debugging(script_debugging),
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

fn run_package_web(output_dir: &Path, script_debugging: bool) -> ExitCode {
    let config = match marble_rush_build_configuration(None) {
        Ok(config) => config.with_script_debugging(script_debugging),
        Err(error) => {
            eprintln!("unitas-player packaging failed: {error}");
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };
    let packaged = match package_web_target(&config, output_dir) {
        Ok(packaged) => packaged,
        Err(error) => {
            eprintln!("unitas-player packaging failed: {error}");
            return ExitCode::from(STARTUP_FAILURE_EXIT_CODE);
        }
    };
    println!("packaged_app[web]={}", packaged.app_path.display());
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

    if cli.help {
        println!("{USAGE}\n{HELP}");
        return ExitCode::SUCCESS;
    }

    if let Some(ref output_dir) = cli.package_sample {
        return run_package_sample(output_dir, cli.script_debugging);
    }

    if let Some(ref output_dir) = cli.package_web {
        return run_package_web(output_dir, cli.script_debugging);
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

    let (resolved_bundle_dir, temp_sample_dir) = match resolve_bundle_dir(&cli) {
        Ok(resolved) => resolved,
        Err(code) => return code,
    };

    let run_headless =
        !cli.windowed && (cli.frames.is_some() || (!cli.sample && cli.bundle_dir.is_some()));

    if let Err(error) = install_project_trust(&resolved_bundle_dir) {
        eprintln!("unitas-player refused to run project code: {error}");
        return ExitCode::from(TRUST_REQUIRED_EXIT_CODE);
    }

    let exit_code = if run_headless {
        run_headless_launch(&resolved_bundle_dir, &cli)
    } else {
        run_windowed_launch(&resolved_bundle_dir, &cli)
    };

    if let Some(dir) = temp_sample_dir {
        let _ = fs::remove_dir_all(dir);
    }
    exit_code
}

fn resolve_bundle_dir(cli: &CliOptions) -> Result<(PathBuf, Option<PathBuf>), ExitCode> {
    let mut temp_sample_dir: Option<PathBuf> = None;
    let resolved_bundle_dir = match cli.bundle_dir {
        Some(ref dir) if cli.sample && !dir.exists() => {
            if let Err(error) = stage_marble_rush_bundle_for_host_with_script_debugging(
                dir,
                None,
                cli.script_debugging,
            ) {
                eprintln!("unitas-player startup failed: {error}");
                return Err(ExitCode::from(STARTUP_FAILURE_EXIT_CODE));
            }
            dir.clone()
        }
        Some(ref dir) => dir.clone(),
        None => {
            let temp_dir = std::env::temp_dir().join(format!(
                "unitas_player_sample_bundle_{}",
                std::process::id()
            ));
            if let Err(error) = stage_marble_rush_bundle_for_host_with_script_debugging(
                &temp_dir,
                None,
                cli.script_debugging,
            ) {
                eprintln!("unitas-player startup failed: {error}");
                return Err(ExitCode::from(STARTUP_FAILURE_EXIT_CODE));
            }
            temp_sample_dir = Some(temp_dir.clone());
            temp_dir
        }
    };
    Ok((resolved_bundle_dir, temp_sample_dir))
}

fn run_headless_launch(bundle_dir: &Path, cli: &CliOptions) -> ExitCode {
    let frames = cli.frames.unwrap_or(1);
    match launch_bundle(bundle_dir, frames, cli.delta_seconds) {
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
}

fn run_windowed_launch(bundle_dir: &Path, cli: &CliOptions) -> ExitCode {
    let options = WindowedLaunchOptions {
        width: cli.width,
        height: cli.height,
        max_frames: cli.frames.map(|n| n as u64),
        fixed_delta_seconds: Some(cli.delta_seconds),
        capture_last_frame: false,
        start_audio_device: true,
        inject_demo_input: cli.demo_input,
    };
    run_windowed(bundle_dir, &options)
}

#[cfg(feature = "device")]
fn run_windowed(bundle_dir: &Path, options: &WindowedLaunchOptions) -> ExitCode {
    match launch_bundle_windowed(bundle_dir, options) {
        Ok(result) => {
            println!(
                "launched_product={} startup_scene={} loaded_scenes={} root_objects={} frames_stepped={} development_diagnostics={} script_host={} mode=windowed frames_presented={} audio_status=\"{}\" audio_frames={} canvas_texts=\"{}\"",
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
                result.canvas_texts.join(CANVAS_TEXT_SEPARATOR)
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
                "device_verify=ok reference={} size={}x{} frames_presented={} mean_error={:.4} tolerance={:.4} audio_active={} audio_status=\"{}\" audio_frames={} final_pos=({:.3},{:.3},{:.3}) canvas={} canvas_texts=\"{}\" shadow={}",
                report.reference_path.display(),
                report.width,
                report.height,
                report.frames_presented,
                report.mean_channel_error,
                report.tolerance,
                report.audio_device_active,
                report.audio_device_status,
                report.audio_frames_pushed,
                report.final_ball_position[0],
                report.final_ball_position[1],
                report.final_ball_position[2],
                report.canvas,
                report.canvas_texts.join(CANVAS_TEXT_SEPARATOR),
                report.shadow
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
            let staged = stage_marble_rush_bundle_for_host_with_script_debugging(
                &staged_dir,
                None,
                cli.script_debugging,
            )
            .map_err(|error| error.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_flag_is_parsed_for_the_staging_paths() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_args([String::from("--sample"), String::from("--debug")])
            .map_err(std::io::Error::other)?;
        assert!(cli.sample);
        assert!(cli.script_debugging);

        let plain = parse_cli_args([String::from("--sample")]).map_err(std::io::Error::other)?;
        assert!(!plain.script_debugging);
        Ok(())
    }

    #[test]
    fn help_output_names_the_debug_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_args([String::from("--help")]).map_err(std::io::Error::other)?;
        assert!(cli.help);
        assert!(USAGE.contains("--debug"));
        assert!(HELP.contains("--debug"));
        Ok(())
    }

    #[test]
    fn package_web_flag_is_parsed_for_the_web_package() -> Result<(), Box<dyn std::error::Error>> {
        let cli = parse_cli_args([String::from("--package-web"), String::from("target/web")])
            .map_err(std::io::Error::other)?;
        assert_eq!(cli.package_web, Some(PathBuf::from("target/web")));
        assert!(cli.package_sample.is_none());
        assert!(USAGE.contains("--package-web"));
        Ok(())
    }

    #[test]
    fn package_web_flag_requires_a_directory() {
        let error = parse_cli_args([String::from("--package-web")]);
        assert_eq!(
            error,
            Err(String::from("missing directory for '--package-web'"))
        );
    }
}
