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

type InitEntry = extern "C" fn(u64, *const u8, *const u8, *const u8) -> i32;
type TickEntry = extern "C" fn(f64) -> i32;
type ShutdownEntry = extern "C" fn() -> i32;
type InjectInputEntry = extern "C" fn(u64, u64, u8, f64, f64) -> i32;

pub struct ScriptedPlayer {
    runtime: RuntimeDylib,
    handle: u64,
    tick: TickEntry,
    shutdown: ShutdownEntry,
    inject_input: InjectInputEntry,
    persist_root: PathBuf,
}

pub struct ScriptedLaunch {
    pub loaded_scene_count: usize,
    pub root_object_count: usize,
    pub frames_stepped: usize,
}

impl ScriptedPlayer {
    pub fn boot(
        bundle_dir: &Path,
        startup_scene_path: &Path,
    ) -> Result<Option<Self>, PlayerStartupError> {
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
        let inject_input = host
            .load_function::<InjectInputEntry>(
                &assembly_path,
                PLAYER_HOST_ENTRY_TYPE,
                "UnitasPlayerInjectInput",
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
        let scene_argument = nul_terminated(&absolute_path(startup_scene_path))?;

        let stage = init(
            handle,
            bundle_root_argument.as_ptr().cast::<u8>(),
            persist_root_argument.as_ptr().cast::<u8>(),
            scene_argument.as_ptr().cast::<u8>(),
        );
        if stage != 0 {
            let _ = runtime.destroy(handle);
            let _ = fs::remove_dir_all(&persist_root);
            return Err(PlayerStartupError::ScriptHostInitFailed { stage });
        }

        Ok(Some(Self {
            runtime,
            handle,
            tick,
            shutdown,
            inject_input,
            persist_root,
        }))
    }

    pub fn tick(&mut self, delta_seconds: f64) -> Result<(), PlayerStartupError> {
        let stage = (self.tick)(delta_seconds);
        if stage != 0 {
            return Err(PlayerStartupError::ScriptHostTickFailed { frame: 0, stage });
        }
        Ok(())
    }

    pub fn inject_input(&mut self, kind: u64, code: u64, pressed: bool, x: f64, y: f64) {
        let _ = (self.inject_input)(kind, code, u8::from(pressed), x, y);
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn loaded_scene_count(&self) -> Result<usize, PlayerStartupError> {
        self.runtime.scene_count(self.handle).map_err(|error| {
            PlayerStartupError::ScriptHostBootFailed {
                message: error.to_string(),
            }
        })
    }

    pub fn root_object_count(&self) -> Result<usize, PlayerStartupError> {
        self.runtime.root_count(self.handle).map_err(|error| {
            PlayerStartupError::ScriptHostBootFailed {
                message: error.to_string(),
            }
        })
    }

    pub fn capture_scene(
        &self,
        width: u32,
        height: u32,
    ) -> Result<unitas_render::CaptureFrame, PlayerStartupError> {
        self.runtime
            .capture_scene(self.handle, width, height)
            .map_err(|error| PlayerStartupError::ScriptHostBootFailed {
                message: error.to_string(),
            })
    }

    pub fn shutdown(&mut self) -> Result<(), PlayerStartupError> {
        let stage = (self.shutdown)();
        let _ = self.runtime.destroy(self.handle);
        let _ = fs::remove_dir_all(&self.persist_root);
        if stage != 0 {
            return Err(PlayerStartupError::ScriptHostShutdownFailed { stage });
        }
        Ok(())
    }
}

pub fn launch_scripted(
    bundle_dir: &Path,
    startup_scene_path: &Path,
    frames: usize,
    delta_seconds: f64,
) -> Result<Option<ScriptedLaunch>, PlayerStartupError> {
    let Some(mut player) = ScriptedPlayer::boot(bundle_dir, startup_scene_path)? else {
        return Ok(None);
    };

    for frame in 0..frames {
        if let Err(error) = player.tick(delta_seconds) {
            if let PlayerStartupError::ScriptHostTickFailed { stage, .. } = error {
                let _ = player.shutdown();
                return Err(PlayerStartupError::ScriptHostTickFailed { frame, stage });
            }

            return Err(error);
        }
    }

    let loaded_scene_count = player.loaded_scene_count()?;
    let root_object_count = player.root_object_count()?;
    player.shutdown()?;

    Ok(Some(ScriptedLaunch {
        loaded_scene_count,
        root_object_count,
        frames_stepped: frames,
    }))
}

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
