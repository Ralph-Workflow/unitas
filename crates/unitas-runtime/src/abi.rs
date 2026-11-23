use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::error::RuntimeError;
use crate::runtime::{Callbacks, Runtime};

pub struct RuntimeState {
    pub runtime: Runtime,
    pub path_buffer: Vec<u8>,
    pub last_error: Option<Vec<u8>>,
}

impl RuntimeState {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            path_buffer: Vec::new(),
            last_error: None,
        }
    }

    pub fn set_error(&mut self, message: &str) {
        let mut bytes = message.as_bytes().to_vec();
        bytes.push(0);
        self.last_error = Some(bytes);
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }
}

static REGISTRY: OnceLock<Mutex<HashMap<u64, RuntimeState>>> = OnceLock::new();
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn registry() -> Result<&'static Mutex<HashMap<u64, RuntimeState>>, RuntimeError> {
    Ok(REGISTRY.get_or_init(|| Mutex::new(HashMap::new())))
}

#[no_mangle]
pub extern "C" fn unitas_runtime_new() -> u64 {
    let runtime = match Runtime::new() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    map.insert(handle, RuntimeState::new(runtime));
    handle
}

#[no_mangle]
pub extern "C" fn unitas_runtime_path_buffer(handle: u64, capacity: u64) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let cap = match usize::try_from(capacity) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return 0,
    };
    state.path_buffer = vec![0u8; cap];
    state.path_buffer.as_ptr() as usize as u64
}

#[no_mangle]
pub extern "C" fn unitas_runtime_load_scene(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return -1,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return -1,
    };
    if address == 0 || address != state.path_buffer.as_ptr() as usize as u64 {
        state.set_error("buffer address mismatch");
        return -2;
    }
    if len > state.path_buffer.len() {
        state.set_error("buffer length exceeds capacity");
        return -2;
    }
    let slice = &state.path_buffer[..len];
    let path_str = match std::str::from_utf8(slice) {
        Ok(s) => s.trim_end_matches('\0'),
        Err(err) => {
            state.set_error(&err.to_string());
            return -3;
        }
    };
    let path = Path::new(path_str);
    match state.runtime.load_scene(path) {
        Ok(_) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -4
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_set_callbacks(
    handle: u64,
    create: extern "C" fn(u64, u64, u64) -> u64,
    awake: extern "C" fn(u64),
    on_enable: extern "C" fn(u64),
    start: extern "C" fn(u64),
    fixed_update: extern "C" fn(u64, f64),
    update: extern "C" fn(u64, f64),
    late_update: extern "C" fn(u64, f64),
    on_disable: extern "C" fn(u64),
    destroy: extern "C" fn(u64),
    collision: extern "C" fn(u64, u64, u32),
    trigger: extern "C" fn(u64, u64, u32),
) -> i32 {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return -1,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return -1,
    };
    state.runtime.set_callbacks(Callbacks {
        create_script: create,
        awake,
        on_enable,
        start,
        fixed_update,
        update,
        late_update,
        on_disable,
        destroy,
        collision,
        trigger,
    });
    state.clear_error();
    0
}

#[no_mangle]
pub extern "C" fn unitas_runtime_tick(handle: u64, delta_seconds: f64) -> i32 {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return -1,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return -1,
    };
    match state.runtime.tick(delta_seconds) {
        Ok(()) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -5
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_destroy(handle: u64) -> i32 {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return -1,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    let mut state = match map.remove(&handle) {
        Some(s) => s,
        None => return -1,
    };
    match state.runtime.shutdown() {
        Ok(()) => 0,
        Err(_) => -6,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_error_message(handle: u64) -> u64 {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let state = match map.get(&handle) {
        Some(s) => s,
        None => return 0,
    };
    match state.last_error.as_ref() {
        Some(bytes) => bytes.as_ptr() as usize as u64,
        None => 0,
    }
}

pub fn test_write_path_buffer(handle: u64, data: &[u8]) -> Result<(), RuntimeError> {
    let reg = registry()?;
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(RuntimeError::Poisoned),
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return Err(RuntimeError::NotFound),
    };
    if data.len() > state.path_buffer.len() {
        return Err(RuntimeError::InvalidAddress);
    }
    state.path_buffer[..data.len()].copy_from_slice(data);
    Ok(())
}
