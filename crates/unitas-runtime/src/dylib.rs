use std::fmt;
use std::path::{Path, PathBuf};

use libloading::Library;

type NewHandleFn = extern "C" fn() -> u64;
type CountFn = extern "C" fn(u64) -> i32;
type DestroyFn = extern "C" fn(u64) -> i32;
type CaptureSceneFn = extern "C" fn(u64, u32, u32) -> i32;
type RenderBufferWordFn = extern "C" fn(u64, u64) -> u64;

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
    capture_scene: CaptureSceneFn,
    render_buffer_word: RenderBufferWordFn,
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
        let capture_scene =
            symbol::<CaptureSceneFn>(&library, path, "unitas_runtime_capture_scene")?;
        let render_buffer_word =
            symbol::<RenderBufferWordFn>(&library, path, "unitas_runtime_render_buffer_word")?;
        Ok(Self {
            _library: library,
            new_handle,
            scene_count,
            root_count,
            destroy,
            capture_scene,
            render_buffer_word,
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

    pub fn capture_scene(
        &self,
        handle: u64,
        width: u32,
        height: u32,
    ) -> Result<unitas_render::CaptureFrame, RuntimeDylibError> {
        let code = (self.capture_scene)(handle, width, height);
        if code < 0 {
            return Err(RuntimeDylibError::OperationFailed {
                operation: "unitas_runtime_capture_scene",
                code,
            });
        }
        let byte_len = code as usize;
        let mut bytes = Vec::with_capacity(byte_len.div_ceil(8) * 8);
        let mut offset = 0usize;
        while offset < byte_len {
            let word = (self.render_buffer_word)(handle, offset as u64);
            bytes.extend_from_slice(&word.to_le_bytes());
            offset += 8;
        }
        bytes.truncate(byte_len);
        decode_capture_frame(&bytes).ok_or(RuntimeDylibError::OperationFailed {
            operation: "unitas_runtime_capture_scene",
            code: -2,
        })
    }
}

fn decode_capture_frame(bytes: &[u8]) -> Option<unitas_render::CaptureFrame> {
    if bytes.len() < 16 {
        return None;
    }
    let width = read_u32(bytes, 0)?;
    let height = read_u32(bytes, 4)?;
    let id_count = read_u32(bytes, 8)? as usize;
    let object_count = read_u32(bytes, 12)? as usize;
    let image_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    let mut cursor = 16usize;
    let image_end = cursor.checked_add(image_len)?;
    if image_end > bytes.len() {
        return None;
    }
    let image = unitas_render::Rgba8 {
        width,
        height,
        pixels: bytes[cursor..image_end].to_vec(),
    };
    cursor = image_end;
    let mut id_map = Vec::with_capacity(id_count);
    for _ in 0..id_count {
        let id = read_u32(bytes, cursor)?;
        id_map.push(id);
        cursor = cursor.checked_add(4)?;
    }
    let mut objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        let id = read_u32(bytes, cursor)?;
        let x = read_u32(bytes, cursor.checked_add(4)?)?;
        let y = read_u32(bytes, cursor.checked_add(8)?)?;
        let obj_width = read_u32(bytes, cursor.checked_add(12)?)?;
        let obj_height = read_u32(bytes, cursor.checked_add(16)?)?;
        let name_len = read_u32(bytes, cursor.checked_add(20)?)? as usize;
        cursor = cursor.checked_add(24)?;
        let name_end = cursor.checked_add(name_len)?;
        if name_end > bytes.len() {
            return None;
        }
        let name = std::str::from_utf8(&bytes[cursor..name_end])
            .ok()?
            .to_string();
        cursor = name_end;
        objects.push(unitas_render::CaptureObject {
            id,
            name,
            bounds: unitas_render::ScreenBounds {
                x,
                y,
                width: obj_width,
                height: obj_height,
            },
        });
    }
    Some(unitas_render::CaptureFrame {
        image,
        id_map,
        objects,
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    if end > bytes.len() {
        return None;
    }
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..end]);
    Some(u32::from_le_bytes(raw))
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
