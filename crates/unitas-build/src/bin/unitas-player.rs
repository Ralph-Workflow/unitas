use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use unitas_build::launch_bundle;

fn parse_cli_args<I>(args: I) -> Result<(PathBuf, usize, f64), String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut bundle_dir: Option<PathBuf> = None;
    let mut frames: usize = 1;
    let mut delta_seconds: f64 = 1.0 / 60.0;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--frames" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| String::from("missing value for '--frames'"))?;
                frames = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid '--frames' value '{raw}': {err}"))?;
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
            flag if flag.starts_with('-') => {
                return Err(format!("unrecognized argument '{flag}'"));
            }
            positional => {
                if bundle_dir.is_some() {
                    return Err(format!("unexpected extra positional argument '{positional}'"));
                }
                bundle_dir = Some(PathBuf::from(positional));
            }
        }
    }

    let resolved_dir =
        bundle_dir.ok_or_else(|| String::from("usage: unitas-player <bundle-dir> [--frames <N>] [--dt <seconds>]"))?;
    Ok((resolved_dir, frames, delta_seconds))
}

fn main() -> ExitCode {
    let args = env::args().skip(1);
    let (bundle_dir, frames, delta_seconds) = match parse_cli_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("unitas-player argument error: {message}");
            return ExitCode::from(2);
        }
    };

    match launch_bundle(&bundle_dir, frames, delta_seconds) {
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
}
