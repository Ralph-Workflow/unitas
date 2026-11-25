use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_dotnet::DotnetHost;
use unitas_runtime::RuntimeDylib;

use crate::bundle::{
    MANAGED_DIR_NAME, PLAYER_HOST_ASSEMBLY_FILENAME, PLAYER_HOST_RUNTIME_CONFIG_FILENAME,
    RUNTIME_LIBRARY_FILENAME,
};
use crate::player::PlayerStartupError;

const PLAYER_HOST_ENTRY_TYPE: &str = "Unitas.PlayerHost.PlayerHostEntry";
const RUNTIME_LIBRARY_ENVIRONMENT_VARIABLE: &str = "UNITAS_RUNTIME_LIBRARY";

type InitEntry = extern "C" fn(u64, *const u8, *const u8) -> i32;
type TickEntry = extern "C" fn(f64) -> i32;
type ShutdownEntry = extern "C" fn() -> i32;

pub struct ScriptedLaunch {
    pub loaded_scene_count: usize,
    pub root_object_count: usize,
    pub frames_stepped: usize,
}

/// Runs the bundle's managed payload in process: the native side creates the
/// runtime handle inside the runtime library the managed side binds, boots
/// CoreCLR, and drives the game's own scripts through the player host.
/// A bundle without a managed payload reports `None` and keeps the native loop.
pub fn launch_scripted(
    bundle_dir: &Path,
    frames: usize,
    delta_seconds: f64,
) -> Result<Option<ScriptedLaunch>, PlayerStartupError> {
    let managed_dir = bundle_dir.join(MANAGED_DIR_NAME);
    let assembly_path = managed_dir.join(PLAYER_HOST_ASSEMBLY_FILENAME);
    let runtime_config_path = managed_dir.join(PLAYER_HOST_RUNTIME_CONFIG_FILENAME);
    if !assembly_path.is_file() || !runtime_config_path.is_file() {
        return Ok(None);
    }

    let bundle_root = absolute_path(bundle_dir);
    let library_path = resolve_runtime_library(&bundle_root)?;
    let runtime = RuntimeDylib::load(&library_path).map_err(|error| {
        PlayerStartupError::ScriptHostBootFailed {
            message: error.to_string(),
        }
    })?;
    let handle = runtime.new_handle();
    if handle == 0 {
        return Err(PlayerStartupError::ScriptHostBootFailed {
            message: String::from("the runtime library did not create a runtime handle"),
        });
    }

    let host = DotnetHost::boot(&runtime_config_path).map_err(|error| {
        PlayerStartupError::ScriptHostBootFailed {
            message: error.to_string(),
        }
    })?;
    let init = host
        .load_function::<InitEntry>(&assembly_path, PLAYER_HOST_ENTRY_TYPE, "UnitasPlayerInit")
        .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
            message: error.to_string(),
        })?;
    let tick = host
        .load_function::<TickEntry>(&assembly_path, PLAYER_HOST_ENTRY_TYPE, "UnitasPlayerTick")
        .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
            message: error.to_string(),
        })?;
    let shutdown = host
        .load_function::<ShutdownEntry>(
            &assembly_path,
            PLAYER_HOST_ENTRY_TYPE,
            "UnitasPlayerShutdown",
        )
        .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
            message: error.to_string(),
        })?;

    let persist_root = persist_root_directory();
    fs::create_dir_all(&persist_root).map_err(|error| {
        PlayerStartupError::ScriptHostBootFailed {
            message: format!(
                "the persistent data directory '{}' could not be created: {error}",
                persist_root.display()
            ),
        }
    })?;
    let bundle_root_argument = nul_terminated(&bundle_root)?;
    let persist_root_argument = nul_terminated(&persist_root)?;

    let stage = init(
        handle,
        bundle_root_argument.as_ptr().cast::<u8>(),
        persist_root_argument.as_ptr().cast::<u8>(),
    );
    if stage != 0 {
        let _ = runtime.destroy(handle);
        return Err(PlayerStartupError::ScriptHostInitFailed { stage });
    }

    for frame in 0..frames {
        let stage = tick(delta_seconds);
        if stage != 0 {
            let _ = shutdown();
            let _ = runtime.destroy(handle);
            let _ = fs::remove_dir_all(&persist_root);
            return Err(PlayerStartupError::ScriptHostTickFailed { frame, stage });
        }
    }

    let loaded_scene_count =
        runtime
            .scene_count(handle)
            .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
                message: error.to_string(),
            })?;
    let root_object_count =
        runtime
            .root_count(handle)
            .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
                message: error.to_string(),
            })?;

    let stage = shutdown();
    let _ = fs::remove_dir_all(&persist_root);
    if stage != 0 {
        return Err(PlayerStartupError::ScriptHostShutdownFailed { stage });
    }

    Ok(Some(ScriptedLaunch {
        loaded_scene_count,
        root_object_count,
        frames_stepped: frames,
    }))
}

/// The runtime library the native side and the managed side bind together:
/// the environment override when it is set, otherwise the copy the bundle
/// carries at its root.
fn resolve_runtime_library(bundle_root: &Path) -> Result<PathBuf, PlayerStartupError> {
    let override_path = std::env::var_os(RUNTIME_LIBRARY_ENVIRONMENT_VARIABLE)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let path = match override_path {
        Some(path) => path,
        None => bundle_root.join(RUNTIME_LIBRARY_FILENAME),
    };
    if !path.is_file() {
        return Err(PlayerStartupError::RuntimeLibraryUnavailable { path });
    }
    Ok(path)
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map_or_else(|_| path.to_path_buf(), |directory| directory.join(path))
}

fn persist_root_directory() -> PathBuf {
    std::env::temp_dir().join(format!("unitas-player-persist-{}", std::process::id()))
}

fn nul_terminated(path: &Path) -> Result<CString, PlayerStartupError> {
    CString::new(path.to_string_lossy().as_bytes()).map_err(|_| {
        PlayerStartupError::ScriptHostBootFailed {
            message: format!("the path '{}' holds an interior NUL byte", path.display()),
        }
    })
}
