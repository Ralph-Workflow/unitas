use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt;
use std::path::{Path, PathBuf};

use libloading::Library;

const GET_FUNCTION_POINTER: c_int = 6;
const HOSTFXR_LIBRARY_NAMES: [&str; 3] = ["libhostfxr.dylib", "libhostfxr.so", "hostfxr.dll"];
const HOSTFXR_SUCCESS: c_int = 0;
const HOSTFXR_SUCCESS_ALREADY_INITIALIZED: c_int = 1;

type CStr = c_char;
type VSlot = *mut *mut c_void;
type CmdInitFn = unsafe extern "C" fn(c_int, *const *const CStr, *const c_void, VSlot) -> i32;
type GetFunctionPointerFn = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
    *mut c_void,
    *mut *mut c_void,
) -> c_int;
type HostFxrDelegateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_void) -> c_int;

#[derive(Debug)]
pub enum DotnetHostError {
    HostFxrNotFound { searched: Vec<PathBuf> },
    RuntimeConfigInvalid { path: PathBuf, detail: String },
    InitFailed { code: i32 },
    LoadAssemblyFailed { assembly: PathBuf, detail: String },
}

impl fmt::Display for DotnetHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostFxrNotFound { searched } => {
                write!(f, "hostfxr not found; searched {searched:?}")
            }
            Self::RuntimeConfigInvalid { path, detail } => {
                write!(f, "invalid runtime config {}: {detail}", path.display())
            }
            Self::InitFailed { code } => write!(f, "hostfxr init failed with code {code}"),
            Self::LoadAssemblyFailed { assembly, detail } => {
                write!(f, "load from {} failed: {detail}", assembly.display())
            }
        }
    }
}

impl std::error::Error for DotnetHostError {}

pub struct DotnetHost {
    _library: Library,
    _handle: *mut c_void,
    get_function_pointer: GetFunctionPointerFn,
}

impl DotnetHost {
    pub fn boot(runtime_config: &Path) -> Result<DotnetHost, DotnetHostError> {
        DotnetHost::boot_with_search_paths(runtime_config, &default_search_roots())
    }

    pub fn boot_with_search_paths(
        runtime_config: &Path,
        roots: &[PathBuf],
    ) -> Result<DotnetHost, DotnetHostError> {
        let library_path = find_hostfxr(roots)?;
        let _config_path = validate_runtime_config(runtime_config)?;
        let library = load_library(&library_path)?;
        let initialize =
            hostfxr_symbol::<CmdInitFn>(&library, b"hostfxr_initialize_for_dotnet_command_line\0")?;
        let get_delegate =
            hostfxr_symbol::<HostFxrDelegateFn>(&library, b"hostfxr_get_runtime_delegate\0")?;
        let application = application_assembly_path(runtime_config);
        let application_argument = path_argument(&application)?;
        let arguments: [*const c_char; 1] = [application_argument.as_ptr()];
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: hostfxr writes a hostfxr_handle into the out parameter and the pointer stays valid for the process lifetime.
        let status = unsafe {
            initialize(
                1,
                arguments.as_ptr(),
                std::ptr::null::<c_void>(),
                std::ptr::from_mut(&mut handle),
            )
        };
        if status != HOSTFXR_SUCCESS && status != HOSTFXR_SUCCESS_ALREADY_INITIALIZED {
            return Err(DotnetHostError::InitFailed { code: status });
        }
        let mut pointer_slot: *mut c_void = std::ptr::null_mut();
        // SAFETY: hostfxr writes the get_function_pointer entry point into the out parameter and the pointer stays valid for the process lifetime.
        let status = unsafe {
            get_delegate(
                handle,
                GET_FUNCTION_POINTER,
                std::ptr::from_mut(&mut pointer_slot),
            )
        };
        if status != HOSTFXR_SUCCESS || pointer_slot.is_null() {
            return Err(DotnetHostError::InitFailed { code: status });
        }
        // SAFETY: hostfxr wrote a get_function_pointer entry point into the slot.
        let slot: *mut c_void = pointer_slot;
        // SAFETY: hostfxr wrote a get_function_pointer entry point into the slot.
        let get_function_pointer: GetFunctionPointerFn = unsafe { std::mem::transmute_copy(&slot) };
        Ok(DotnetHost {
            _library: library,
            _handle: handle,
            get_function_pointer,
        })
    }

    pub fn load_function<T: Copy>(
        &self,
        assembly: &Path,
        type_name: &str,
        method: &str,
    ) -> Result<T, DotnetHostError> {
        if std::mem::size_of::<T>() != std::mem::size_of::<*mut c_void>()
            || std::mem::align_of::<T>() > std::mem::align_of::<*mut c_void>()
        {
            return Err(DotnetHostError::LoadAssemblyFailed {
                assembly: assembly.to_path_buf(),
                detail: "delegate type must have the size and alignment of a function pointer"
                    .to_owned(),
            });
        }
        let type_argument = text_argument(&qualified_type_name(type_name, assembly), assembly)?;
        let method_argument = text_argument(method, assembly)?;
        let unmanaged_callers_only: *const c_char = usize::MAX as *const c_char;
        let mut slot: *mut c_void = std::ptr::null_mut();
        // SAFETY: the runtime writes the managed entry point into the out parameter and the pointer stays valid for the process lifetime.
        let status = unsafe {
            (self.get_function_pointer)(
                type_argument.as_ptr(),
                method_argument.as_ptr(),
                unmanaged_callers_only,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::from_mut(&mut slot),
            )
        };
        if status != HOSTFXR_SUCCESS {
            return Err(DotnetHostError::LoadAssemblyFailed {
                assembly: assembly.to_path_buf(),
                detail: format!("the runtime reported status {status}"),
            });
        }
        if slot.is_null() {
            return Err(DotnetHostError::LoadAssemblyFailed {
                assembly: assembly.to_path_buf(),
                detail: "the runtime returned a null delegate".to_owned(),
            });
        }
        // SAFETY: the slot holds the managed entry point the runtime resolved for the requested type and method.
        Ok(unsafe { std::mem::transmute_copy(&slot) })
    }
}

/// The assembly-qualified type name the runtime resolves the entry point by.
fn qualified_type_name(type_name: &str, assembly: &Path) -> String {
    if type_name.contains(',') {
        return type_name.to_owned();
    }
    let assembly_name = assembly
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    format!("{type_name}, {assembly_name}")
}

/// The application assembly the runtime resolves types against: the assembly
/// that sits next to the runtime configuration.
fn application_assembly_path(runtime_config: &Path) -> PathBuf {
    let file_name = runtime_config
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let stem = file_name
        .strip_suffix(".runtimeconfig.json")
        .unwrap_or(file_name);
    let assembly_name = format!("{stem}.dll");
    match runtime_config.parent() {
        Some(directory) => directory.join(assembly_name),
        None => PathBuf::from(assembly_name),
    }
}

fn default_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("DOTNET_ROOT").filter(|value| !value.is_empty()) {
        roots.push(PathBuf::from(root));
    }
    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        roots.push(PathBuf::from(home).join(".dotnet"));
    }
    if let Some(dotnet) = program_on_path("dotnet") {
        roots.push(install_root(&dotnet));
    }
    roots
}

fn program_on_path(program: &str) -> Option<PathBuf> {
    let search_path = std::env::var_os("PATH")?;
    std::env::split_paths(&search_path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

fn install_root(program: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(program).unwrap_or_else(|_| program.to_path_buf());
    resolved
        .parent()
        .map_or_else(|| resolved.clone(), Path::to_path_buf)
}

fn find_hostfxr(roots: &[PathBuf]) -> Result<PathBuf, DotnetHostError> {
    let mut searched = Vec::new();
    for root in roots {
        let versions = root.join("host").join("fxr");
        searched.push(versions.clone());
        if let Some(library) = highest_version_hostfxr(&versions) {
            return Ok(library);
        }
    }
    Err(DotnetHostError::HostFxrNotFound { searched })
}

fn highest_version_hostfxr(versions: &Path) -> Option<PathBuf> {
    let mut best: Option<((u32, u32, u32, bool), PathBuf)> = None;
    for entry in std::fs::read_dir(versions).ok()?.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(version) = parse_version(name) else {
            continue;
        };
        for candidate in HOSTFXR_LIBRARY_NAMES {
            let library = directory.join(candidate);
            if !library.is_file() {
                continue;
            }
            let newer = best.as_ref().is_none_or(|(best, _)| version > *best);
            if newer {
                best = Some((version, library));
            }
            break;
        }
    }
    best.map(|(_, library)| library)
}

fn parse_version(name: &str) -> Option<(u32, u32, u32, bool)> {
    let (numbers, prerelease) = match name.split_once('-') {
        Some((numbers, suffix)) => (numbers, !suffix.is_empty()),
        None => (name, false),
    };
    let mut parts = numbers.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch, !prerelease))
}

fn validate_runtime_config(path: &Path) -> Result<CString, DotnetHostError> {
    let invalid = |detail: &str| DotnetHostError::RuntimeConfigInvalid {
        path: path.to_path_buf(),
        detail: detail.to_owned(),
    };
    if !path.is_file() {
        return Err(invalid("the runtime config file does not exist"));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| invalid(&format!("the runtime config is unreadable: {error}")))?;
    if !text.contains("\"runtimeOptions\"") || !text.contains("\"framework\"") {
        return Err(invalid(
            "the runtime config declares no runtimeOptions.framework",
        ));
    }
    let text = path
        .to_str()
        .ok_or_else(|| invalid("the runtime config path is not valid UTF-8"))?;
    CString::new(text).map_err(|_| invalid("the runtime config path holds an interior NUL byte"))
}

fn load_library(path: &Path) -> Result<Library, DotnetHostError> {
    // SAFETY: the hostfxr library is a trusted runtime component shipped by the .NET installation.
    let library = unsafe { Library::new(path) };
    library.map_err(|_| DotnetHostError::InitFailed { code: -1 })
}

fn hostfxr_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, DotnetHostError> {
    // SAFETY: the symbol name is a NUL terminated string naming an exported hostfxr entry point.
    let symbol = unsafe { library.get::<T>(name) };
    let value = *symbol.map_err(|_| DotnetHostError::InitFailed { code: -1 })?;
    Ok(value)
}

fn path_argument(path: &Path) -> Result<CString, DotnetHostError> {
    let failed = |detail: &str| DotnetHostError::LoadAssemblyFailed {
        assembly: path.to_path_buf(),
        detail: detail.to_owned(),
    };
    let text = path
        .to_str()
        .ok_or_else(|| failed("the assembly path is not valid UTF-8"))?;
    CString::new(text).map_err(|_| failed("the assembly path holds an interior NUL byte"))
}

fn text_argument(text: &str, assembly: &Path) -> Result<CString, DotnetHostError> {
    CString::new(text).map_err(|_| DotnetHostError::LoadAssemblyFailed {
        assembly: assembly.to_path_buf(),
        detail: "the type or method name holds an interior NUL byte".to_owned(),
    })
}
