use std::fmt;
use std::path::{Path, PathBuf};

use libloading::Library;

type NewHandleFn = extern "C" fn() -> u64;
type CountFn = extern "C" fn(u64) -> i32;
type DestroyFn = extern "C" fn(u64) -> i32;

#[derive(Debug)]
pub enum RuntimeDylibError {
    LoadFailed { path: PathBuf, message: String },
    SymbolMissing { path: PathBuf, symbol: String },
    OperationFailed { operation: &'static str, code: i32 },
}

impl fmt::Display for RuntimeDylibError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed { path, message } => {
                write!(
                    formatter,
                    "failed to load the runtime library '{}': {message}",
                    path.display()
                )
            }
            Self::SymbolMissing { path, symbol } => {
                write!(
                    formatter,
                    "the runtime library '{}' does not export '{symbol}'",
                    path.display()
                )
            }
            Self::OperationFailed { operation, code } => {
                write!(
                    formatter,
                    "the runtime library call '{operation}' failed with code {code}"
                )
            }
        }
    }
}

impl std::error::Error for RuntimeDylibError {}

pub struct RuntimeDylib {
    _library: Library,
    new_handle: NewHandleFn,
    scene_count: CountFn,
    root_count: CountFn,
    destroy: DestroyFn,
}

impl RuntimeDylib {
    pub fn load(path: &Path) -> Result<Self, RuntimeDylibError> {
        // SAFETY: the caller resolves the path to the project's own runtime library artifact.
        let opened = unsafe { Library::new(path) };
        let library = opened.map_err(|error| RuntimeDylibError::LoadFailed {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let new_handle = symbol::<NewHandleFn>(&library, path, "unitas_runtime_new")?;
        let scene_count = symbol::<CountFn>(&library, path, "unitas_runtime_scene_count")?;
        let root_count = symbol::<CountFn>(&library, path, "unitas_runtime_root_count")?;
        let destroy = symbol::<DestroyFn>(&library, path, "unitas_runtime_destroy")?;
        Ok(Self {
            _library: library,
            new_handle,
            scene_count,
            root_count,
            destroy,
        })
    }

    pub fn new_handle(&self) -> u64 {
        (self.new_handle)()
    }

    pub fn scene_count(&self, handle: u64) -> Result<usize, RuntimeDylibError> {
        let code = (self.scene_count)(handle);
        if code < 0 {
            return Err(RuntimeDylibError::OperationFailed {
                operation: "unitas_runtime_scene_count",
                code,
            });
        }
        Ok(code as usize)
    }

    pub fn root_count(&self, handle: u64) -> Result<usize, RuntimeDylibError> {
        let code = (self.root_count)(handle);
        if code < 0 {
            return Err(RuntimeDylibError::OperationFailed {
                operation: "unitas_runtime_root_count",
                code,
            });
        }
        Ok(code as usize)
    }

    pub fn destroy(&self, handle: u64) -> Result<(), RuntimeDylibError> {
        let code = (self.destroy)(handle);
        if code != 0 {
            return Err(RuntimeDylibError::OperationFailed {
                operation: "unitas_runtime_destroy",
                code,
            });
        }
        Ok(())
    }
}

fn symbol<T: Copy>(library: &Library, path: &Path, name: &str) -> Result<T, RuntimeDylibError> {
    let mut terminated = name.as_bytes().to_vec();
    terminated.push(0);
    // SAFETY: the symbol name is a NUL terminated string naming an exported function of the loaded runtime library.
    let found = unsafe { library.get::<T>(&terminated) };
    let symbol = found.map_err(|_| RuntimeDylibError::SymbolMissing {
        path: path.to_path_buf(),
        symbol: name.to_string(),
    })?;
    Ok(*symbol)
}
