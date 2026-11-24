use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::error::RuntimeError;
use crate::runtime::{Callbacks, QueryRecord, Runtime};

const QUERY_RECORD_SIZE: usize = 64;

pub struct RuntimeState {
    pub runtime: Runtime,
    pub path_buffer: Vec<u8>,
    pub query_buffer: Vec<u8>,
    pub query_records: Vec<QueryRecord>,
    pub last_error: Option<Vec<u8>>,
}

impl RuntimeState {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            path_buffer: Vec::new(),
            query_buffer: Vec::new(),
            query_records: Vec::new(),
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
static INVALID_HANDLE_ERROR_TARGET: AtomicU64 = AtomicU64::new(0);
const INVALID_HANDLE_MESSAGE: &[u8] = b"runtime handle not found\0";

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
        None => {
            if handle != 0 && INVALID_HANDLE_ERROR_TARGET.load(Ordering::Relaxed) == handle {
                return INVALID_HANDLE_MESSAGE.as_ptr() as usize as u64;
            }
            return 0;
        }
    };
    match state.last_error.as_ref() {
        Some(bytes) => bytes.as_ptr() as usize as u64,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_fixed_timestep(handle: u64) -> f64 {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return 0.0,
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return 0.0,
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            return 0.0;
        }
    };
    state.clear_error();
    state.runtime.fixed_timestep()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_set_time_scale(handle: u64, time_scale: f64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.set_time_scale(time_scale);
        state.clear_error();
    }) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_time_scale(handle: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.time_scale()
    })
    .unwrap_or(0.0)
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

#[no_mangle]
pub extern "C" fn unitas_runtime_query_buffer(handle: u64, capacity: u32) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let size = match usize::try_from(capacity) {
        Ok(value) => value,
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
    state.query_buffer = vec![0u8; size];
    write_query_records(state);
    state.query_buffer.as_ptr() as usize as u64
}

#[no_mangle]
pub extern "C" fn unitas_runtime_raycast(
    handle: u64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    direction_x: f64,
    direction_y: f64,
    direction_z: f64,
    max_distance: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let records = state.runtime.raycast(
            [origin_x, origin_y, origin_z],
            [direction_x, direction_y, direction_z],
            max_distance,
            layer_mask,
            include_triggers != 0,
        );
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_linecast(
    handle: u64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    direction_x: f64,
    direction_y: f64,
    direction_z: f64,
    max_distance: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let records = state.runtime.linecast(
            [origin_x, origin_y, origin_z],
            [direction_x, direction_y, direction_z],
            max_distance,
            layer_mask,
            include_triggers != 0,
        );
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_sphere_cast(
    handle: u64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    direction_x: f64,
    direction_y: f64,
    direction_z: f64,
    radius: f64,
    max_distance: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let records = state.runtime.sphere_cast(
            [origin_x, origin_y, origin_z],
            radius,
            [direction_x, direction_y, direction_z],
            max_distance,
            layer_mask,
            include_triggers != 0,
        );
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_box_cast(
    handle: u64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    direction_x: f64,
    direction_y: f64,
    direction_z: f64,
    half_x: f64,
    half_y: f64,
    half_z: f64,
    max_distance: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let records = state.runtime.box_cast(
            [origin_x, origin_y, origin_z],
            [half_x, half_y, half_z],
            [direction_x, direction_y, direction_z],
            max_distance,
            layer_mask,
            include_triggers != 0,
        );
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_overlap_sphere(
    handle: u64,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    radius: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let records = state.runtime.overlap_sphere(
            [center_x, center_y, center_z],
            radius,
            layer_mask,
            include_triggers != 0,
        );
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}

pub fn test_read_query_buffer(handle: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
    if !data.len().is_multiple_of(QUERY_RECORD_SIZE) {
        return Err(RuntimeError::InvalidAddress);
    }
    let reg = registry()?;
    let map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(RuntimeError::Poisoned),
    };
    let state = match map.get(&handle) {
        Some(s) => s,
        None => return Err(RuntimeError::NotFound),
    };
    let length = data.len().min(state.query_buffer.len());
    data[..length].copy_from_slice(&state.query_buffer[..length]);
    Ok(())
}

fn with_runtime_state<T>(
    handle: u64,
    action: impl FnOnce(&mut RuntimeState) -> T,
) -> Result<T, i32> {
    let reg = match registry() {
        Ok(r) => r,
        Err(_) => return Err(-1),
    };
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(-1),
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return Err(-1),
    };
    Ok(action(state))
}

fn store_query_records(state: &mut RuntimeState, records: Vec<QueryRecord>) -> i32 {
    let count = records.len();
    state.query_records = records;
    write_query_records(state);
    let Ok(count_value) = i32::try_from(count) else {
        return -1;
    };
    count_value
}

fn write_query_records(state: &mut RuntimeState) {
    state.query_buffer.fill(0);
    let capacity = state.query_buffer.len() / QUERY_RECORD_SIZE;
    let count = state.query_records.len().min(capacity);
    for (index, record) in state.query_records.iter().take(count).enumerate() {
        let offset = index * QUERY_RECORD_SIZE;
        let fields = [
            record.entity.to_le_bytes(),
            record.distance.to_le_bytes(),
            record.point[0].to_le_bytes(),
            record.point[1].to_le_bytes(),
            record.point[2].to_le_bytes(),
            record.normal[0].to_le_bytes(),
            record.normal[1].to_le_bytes(),
            record.normal[2].to_le_bytes(),
        ];
        for (slot, field) in fields.iter().enumerate() {
            let start = offset + slot * 8;
            state.query_buffer[start..start + 8].copy_from_slice(field);
        }
    }
}
