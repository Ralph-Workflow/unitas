use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use unitas_audio::AudioClip;
use unitas_core::EntityId;

use crate::error::RuntimeError;
pub use crate::runtime::CharacterMoveRequest;
use crate::runtime::{Callbacks, LoadSceneMode, QueryRecord, Runtime};

const QUERY_RECORD_SIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbiCharacterMoveInput {
    pub entity_id: u64,
    pub position: [f64; 3],
    pub motion: [f64; 3],
    pub radius: f64,
    pub height: f64,
    pub center: [f64; 3],
    pub slope_limit_deg: f64,
    pub step_offset: f64,
    pub skin_width: f64,
    pub min_move_distance: f64,
    pub delta_seconds: f64,
    pub layer_mask: u32,
    pub enabled: u8,
    pub reserved: [u8; 3],
}

impl Default for AbiCharacterMoveInput {
    fn default() -> Self {
        Self {
            entity_id: 0,
            position: [0.0; 3],
            motion: [0.0; 3],
            radius: 0.5,
            height: 2.0,
            center: [0.0; 3],
            slope_limit_deg: 45.0,
            step_offset: 0.3,
            skin_width: 0.08,
            min_move_distance: 0.001,
            delta_seconds: 0.0,
            layer_mask: u32::MAX,
            enabled: 1,
            reserved: [0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AbiCharacterMoveOutput {
    pub position: [f64; 3],
    pub ground_entity_id: u64,
    pub collision_flags: u32,
    pub is_grounded: u8,
    pub reserved: [u8; 3],
}

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

thread_local! {
    static ACTIVE_STATES: RefCell<Vec<(u64, RuntimeState)>> = const { RefCell::new(Vec::new()) };
}

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
    with_runtime_state(handle, |state| {
        if state.path_buffer.len() < cap {
            state.path_buffer.resize(cap, 0);
        }
        state.path_buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_load_scene(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    let path_str = match read_path_argument(handle, address, len) {
        Ok(text) => text,
        Err(code) => return code,
    };
    run_scene_operation(handle, |runtime, hook| {
        runtime
            .load_scene_with_hook(Path::new(&path_str), LoadSceneMode::Single, hook)
            .map(|_| ())
    })
}

#[no_mangle]
pub extern "C" fn unitas_runtime_load_scene_mode(
    handle: u64,
    address: u64,
    length: u64,
    mode: u32,
) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    let load_mode = match mode {
        0 => LoadSceneMode::Single,
        1 => LoadSceneMode::Additive,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown load scene mode");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    let path_str = match read_path_argument(handle, address, len) {
        Ok(text) => text,
        Err(code) => return code,
    };
    run_scene_operation(handle, |runtime, hook| {
        runtime
            .load_scene_with_hook(Path::new(&path_str), load_mode, hook)
            .map(|_| ())
    })
}

#[no_mangle]
pub extern "C" fn unitas_runtime_unload_scene(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    let name = match read_path_argument(handle, address, len) {
        Ok(text) => scene_name_from_input(&text),
        Err(code) => return code,
    };
    run_scene_operation(handle, |runtime, hook| runtime.unload_scene_with_hook(&name, hook))
}

#[no_mangle]
pub extern "C" fn unitas_runtime_scene_count(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        match i32::try_from(state.runtime.scene_count()) {
            Ok(count) => count,
            Err(_) => {
                state.set_error("scene count exceeds supported range");
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_scene_name(handle: u64, index: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        let name = match state.runtime.scene_name(index as usize) {
            Some(name) => name.to_string(),
            None => {
                state.set_error("scene index out of range");
                return -4;
            }
        };
        write_string_to_path_buffer(state, &name, "scene name exceeds supported length")
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_company_name(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let name = match state.runtime.company_name() {
            Some(name) => name.to_string(),
            None => {
                state.set_error("company name not available");
                return -4;
            }
        };
        write_string_to_path_buffer(state, &name, "company name exceeds supported length")
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_product_name(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let name = match state.runtime.product_name() {
            Some(name) => name.to_string(),
            None => {
                state.set_error("product name not available");
                return -4;
            }
        };
        write_string_to_path_buffer(state, &name, "product name exceeds supported length")
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_project_version(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let version = match state.runtime.project_version() {
            Some(version) => version.to_string(),
            None => {
                state.set_error("project version not available");
                return -4;
            }
        };
        write_string_to_path_buffer(state, &version, "project version exceeds supported length")
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

fn write_string_to_path_buffer(
    state: &mut RuntimeState,
    value: &str,
    overflow_error: &str,
) -> i32 {
    let bytes = value.as_bytes();
    let needed = bytes.len() + 1;
    if state.path_buffer.len() < needed {
        state.path_buffer.resize(needed, 0);
    }
    state.path_buffer[..bytes.len()].copy_from_slice(bytes);
    state.path_buffer[bytes.len()] = 0;
    match i32::try_from(bytes.len()) {
        Ok(length) => {
            state.clear_error();
            length
        }
        Err(_) => {
            state.set_error(overflow_error);
            -4
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_set_active_scene(handle: u64, index: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.set_active_scene(index as usize) {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_active_scene_index(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| match state.runtime.active_scene_index() {
        Some(index) => match i32::try_from(index) {
            Ok(value) => value,
            Err(_) => {
                state.set_error("scene index exceeds supported range");
                -4
            }
        },
        None => -1,
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_dont_destroy_on_load(handle: u64, entity: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.dont_destroy_on_load(EntityId::new(entity)) {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
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
    match with_runtime_state(handle, |state| {
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
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_tick(handle: u64, delta_seconds: f64) -> i32 {
    let mut state = match take_from_registry(handle) {
        Some(s) => s,
        None => return -1,
    };
    let result = run_reentrant_operation(handle, &mut state, |runtime, hook| {
        runtime.tick_with_hook(delta_seconds, hook)
    });
    let code = match result {
        Ok(()) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -5
        }
    };
    if !restore_to_registry(handle, state) {
        return -1;
    }
    code
}

#[no_mangle]
pub extern "C" fn unitas_runtime_destroy(handle: u64) -> i32 {
    let mut state = match take_from_registry(handle) {
        Some(s) => s,
        None => return -1,
    };
    match run_reentrant_operation(handle, &mut state, |runtime, hook| {
        runtime.shutdown_with_hook(hook)
    }) {
        Ok(()) => 0,
        Err(_) => -6,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_error_message(handle: u64) -> u64 {
    if let Some(result) = with_active_state(handle, |state| {
        state
            .last_error
            .as_ref()
            .map_or(0, |bytes| bytes.as_ptr() as usize as u64)
    }) {
        return result;
    }
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
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.fixed_timestep()
    }) {
        Ok(val) => val,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0.0
        }
    }
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

#[no_mangle]
pub extern "C" fn unitas_runtime_body_position(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 24
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some([x, y, z]) = state.runtime.body_position(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&z.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_body_velocity(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 24
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some([x, y, z]) = state.runtime.body_velocity(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&z.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_set_body_position(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vector) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_position(EntityId::new(entity), vector)
        {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_set_body_velocity(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vector) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_velocity(EntityId::new(entity), vector)
        {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

fn read_vec3_from_path_buffer(state: &RuntimeState, in_buffer: u64) -> Option<[f64; 3]> {
    if in_buffer == 0
        || in_buffer != state.path_buffer.as_ptr() as usize as u64
        || state.path_buffer.len() < 24
    {
        return None;
    }
    let x_bytes: [u8; 8] = state.path_buffer[0..8].try_into().ok()?;
    let y_bytes: [u8; 8] = state.path_buffer[8..16].try_into().ok()?;
    let z_bytes: [u8; 8] = state.path_buffer[16..24].try_into().ok()?;
    Some([
        f64::from_le_bytes(x_bytes),
        f64::from_le_bytes(y_bytes),
        f64::from_le_bytes(z_bytes),
    ])
}

pub fn test_write_path_buffer(handle: u64, data: &[u8]) -> Result<(), RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        if data.len() > state.path_buffer.len() {
            return Err(RuntimeError::InvalidAddress);
        }
        state.path_buffer[..data.len()].copy_from_slice(data);
        Ok(())
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

pub fn test_is_persistent_entity(handle: u64, entity: u64) -> Result<bool, RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        Ok(state.runtime.is_persistent(EntityId::new(entity)))
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

pub fn test_read_path_buffer(handle: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        if data.len() > state.path_buffer.len() {
            return Err(RuntimeError::InvalidAddress);
        }
        data.copy_from_slice(&state.path_buffer[..data.len()]);
        Ok(())
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_key_event(handle: u64, key: u32, pressed: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.input_key_event(key, pressed != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_mouse_button_event(
    handle: u64,
    btn: u32,
    pressed: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.input_mouse_button_event(btn, pressed != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_pointer_move(handle: u64, x: f64, y: f64) -> i32 {
    match with_runtime_state(handle, |state| {
        if !x.is_finite() || !y.is_finite() {
            state.set_error("non-finite pointer coordinate");
            return -2;
        }
        state.runtime.input_pointer_move(x, y);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_scroll(handle: u64, x: f64, y: f64) -> i32 {
    match with_runtime_state(handle, |state| {
        if !x.is_finite() || !y.is_finite() {
            state.set_error("non-finite scroll delta");
            return -2;
        }
        state.runtime.input_scroll(x, y);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_key_state(handle: u64, key: u32) -> u32 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.input_key_state(key)
    }) {
        Ok(mask) => mask,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_mouse_state(handle: u64, btn: u32) -> u32 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.input_mouse_state(btn)
    }) {
        Ok(mask) => mask,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_pointer(handle: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 32
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let (x, y, dx, dy) = state.runtime.input_pointer();
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&dx.to_le_bytes());
        state.path_buffer[24..32].copy_from_slice(&dy.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_scroll_state(handle: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 16
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let (x, y) = state.runtime.input_scroll_state();
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_axis(handle: u64, address: u64, length: u64) -> f64 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return 0.0,
    };
    match with_runtime_state(handle, |state| {
        if address == 0
            || address != state.path_buffer.as_ptr() as usize as u64
            || len > state.path_buffer.len()
        {
            state.set_error("buffer address mismatch");
            return 0.0;
        }
        let name = match std::str::from_utf8(&state.path_buffer[..len]) {
            Ok(s) => s.trim_end_matches('\0').to_string(),
            Err(err) => {
                let msg = err.to_string();
                state.set_error(&msg);
                return 0.0;
            }
        };
        match state.runtime.input_axis(&name) {
            Some(value) => {
                state.clear_error();
                value
            }
            None => {
                state.set_error(&format!("unknown input axis: {name}"));
                0.0
            }
        }
    }) {
        Ok(val) => val,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0.0
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_axis_raw(handle: u64, address: u64, length: u64) -> f64 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return 0.0,
    };
    match with_runtime_state(handle, |state| {
        if address == 0
            || address != state.path_buffer.as_ptr() as usize as u64
            || len > state.path_buffer.len()
        {
            state.set_error("buffer address mismatch");
            return 0.0;
        }
        let name = match std::str::from_utf8(&state.path_buffer[..len]) {
            Ok(s) => s.trim_end_matches('\0').to_string(),
            Err(err) => {
                let msg = err.to_string();
                state.set_error(&msg);
                return 0.0;
            }
        };
        match state.runtime.input_axis_raw(&name) {
            Some(value) => {
                state.clear_error();
                value
            }
            None => {
                state.set_error(&format!("unknown input axis: {name}"));
                0.0
            }
        }
    }) {
        Ok(val) => val,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0.0
        }
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_input_button_state(
    handle: u64,
    address: u64,
    length: u64,
) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        if address == 0
            || address != state.path_buffer.as_ptr() as usize as u64
            || len > state.path_buffer.len()
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let name = match std::str::from_utf8(&state.path_buffer[..len]) {
            Ok(s) => s.trim_end_matches('\0').to_string(),
            Err(err) => {
                let msg = err.to_string();
                state.set_error(&msg);
                return -3;
            }
        };
        match state.runtime.input_button_state(&name) {
            Some(mask) => {
                state.clear_error();
                i32::try_from(mask).unwrap_or(0)
            }
            None => {
                state.set_error(&format!("unknown input button: {name}"));
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
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
    with_runtime_state(handle, |state| {
        state.query_buffer = vec![0u8; size];
        write_query_records(state);
        state.query_buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
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

#[no_mangle]
pub extern "C" fn unitas_runtime_character_move(
    handle: u64,
    input: Option<&AbiCharacterMoveInput>,
    output: Option<&mut AbiCharacterMoveOutput>,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let (Some(input), Some(output)) = (input, output) else {
            state.set_error("null character move buffer pointer");
            return -2;
        };
        let entity = if input.entity_id == 0 {
            None
        } else {
            Some(EntityId::new(input.entity_id))
        };
        let request = CharacterMoveRequest {
            entity,
            position: input.position,
            motion: input.motion,
            radius: input.radius,
            height: input.height,
            center: input.center,
            slope_limit_deg: input.slope_limit_deg,
            step_offset: input.step_offset,
            skin_width: input.skin_width,
            min_move_distance: input.min_move_distance,
            layer_mask: input.layer_mask,
            enabled: input.enabled != 0,
            delta_seconds: input.delta_seconds,
        };
        match state.runtime.character_move(request) {
            Ok(result) => {
                output.position = result.position;
                output.ground_entity_id = result.ground_entity.map_or(0, |id| id.as_u64());
                output.collision_flags = result.collision_flags.bits();
                output.is_grounded = u8::from(result.is_grounded);
                output.reserved = [0; 3];
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_character_teleport(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(position) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let entity_opt = if entity == 0 {
            None
        } else {
            Some(EntityId::new(entity))
        };
        match state.runtime.character_teleport(entity_opt, position) {
            Ok(_) => {
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_character_controller_config(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 64
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(config) = state
            .runtime
            .character_controller_config(EntityId::new(entity))
            .cloned()
        else {
            state.set_error("character controller not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&config.radius.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&config.height.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&config.center[0].to_le_bytes());
        state.path_buffer[24..32].copy_from_slice(&config.center[1].to_le_bytes());
        state.path_buffer[32..40].copy_from_slice(&config.center[2].to_le_bytes());
        state.path_buffer[40..48].copy_from_slice(&config.slope_limit.to_le_bytes());
        state.path_buffer[48..56].copy_from_slice(&config.step_offset.to_le_bytes());
        state.path_buffer[56..64].copy_from_slice(&config.skin_width.to_le_bytes());
        if state.path_buffer.len() >= 76 {
            state.path_buffer[64..72].copy_from_slice(&config.min_move_distance.to_le_bytes());
            state.path_buffer[72] = u8::from(config.enabled);
            state.path_buffer[73..76].fill(0);
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        let id = state.runtime.ensure_audio_source(entity);
        state.clear_error();
        id.as_u64()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_destroy(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.audio_mut().stop(id);
        state.runtime.audio_mut().remove_source(id);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_play(handle: u64, entity: u64, one_shot: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .play(EntityId::new(entity), one_shot != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_pause(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().pause(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_unpause(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().unpause(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_stop(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().stop(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_volume(handle: u64, entity: u64, volume: f32) -> i32 {
    if entity == 0 || !volume.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_volume(EntityId::new(entity), volume);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_pitch(handle: u64, entity: u64, pitch: f32) -> i32 {
    if entity == 0 || !pitch.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_pitch(EntityId::new(entity), pitch);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_mute(handle: u64, entity: u64, mute: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_mute(EntityId::new(entity), mute != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_loop(handle: u64, entity: u64, looping: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_looping(EntityId::new(entity), looping != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_priority(
    handle: u64,
    entity: u64,
    priority: u32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let clamped = priority.min(255) as u8;
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_priority(EntityId::new(entity), clamped);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_spatial_blend(
    handle: u64,
    entity: u64,
    blend: f32,
) -> i32 {
    if entity == 0 || !blend.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_spatial_blend(EntityId::new(entity), blend);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_min_max_distance(
    handle: u64,
    entity: u64,
    min_distance: f32,
    max_distance: f32,
) -> i32 {
    if entity == 0 || !min_distance.is_finite() || !max_distance.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.audio_mut().set_min_distance(id, min_distance);
        state.runtime.audio_mut().set_max_distance(id, max_distance);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_time(handle: u64, entity: u64, time: f64) -> i32 {
    if entity == 0 || !time.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_source_time(EntityId::new(entity), time);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_clip(handle: u64, entity: u64, clip_id: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let clip_opt = if clip_id == 0 { None } else { Some(clip_id) };
        state
            .runtime
            .audio_mut()
            .set_source_clip(EntityId::new(entity), clip_opt);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_custom_duration(
    handle: u64,
    entity: u64,
    duration: f64,
) -> i32 {
    if entity == 0 || !duration.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let dur_opt = if duration <= 0.0 {
            None
        } else {
            Some(duration)
        };
        state
            .runtime
            .audio_mut()
            .set_custom_duration(EntityId::new(entity), dur_opt);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_position(
    handle: u64,
    entity: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    if entity == 0 || !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_source_position(EntityId::new(entity), [x, y, z]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_state(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state
            .runtime
            .audio()
            .source_state(EntityId::new(entity))
            .as_i32()
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_time(handle: u64, entity: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.audio().source_time(EntityId::new(entity))
    })
    .unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_silence_reason(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state
            .runtime
            .audio()
            .inaudible_reason(EntityId::new(entity))
            .map_or(-1, |r| r.as_i32())
    })
    .unwrap_or(-2)
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_listener(
    handle: u64,
    x: f32,
    y: f32,
    z: f32,
    fx: f32,
    fy: f32,
    fz: f32,
) -> i32 {
    if !x.is_finite()
        || !y.is_finite()
        || !z.is_finite()
        || !fx.is_finite()
        || !fy.is_finite()
        || !fz.is_finite()
    {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_listener([x, y, z], [fx, fy, fz]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_clear_listener(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().clear_listener();
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_set_paused(handle: u64, paused: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().set_application_paused(paused != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_mix(handle: u64, frames: u64, out_buffer: u64) -> u64 {
    let Ok(n_frames) = usize::try_from(frames) else {
        return 0;
    };
    let Some(byte_len) = n_frames.checked_mul(8) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        if out_buffer != 0 {
            if out_buffer != state.path_buffer.as_ptr() as usize as u64
                || state.path_buffer.len() < byte_len
            {
                state.set_error("buffer address mismatch");
                return 0;
            }
        } else if state.path_buffer.len() < byte_len {
            state.path_buffer.resize(byte_len, 0);
        }
        let samples = state.runtime.audio_mut().mix(n_frames);
        for (idx, sample) in samples.iter().enumerate() {
            let start = idx * 4;
            if start + 4 <= state.path_buffer.len() {
                state.path_buffer[start..start + 4].copy_from_slice(&sample.to_le_bytes());
            }
        }
        state.clear_error();
        frames
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_load_clip(
    handle: u64,
    address: u64,
    length: u64,
) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Ok(path_str) = read_path_buffer(state, address, len) else {
            return 0;
        };
        match state.runtime.audio_mut().load_wav_file(Path::new(&path_str)) {
            Ok(clip_id) => {
                state.clear_error();
                clip_id
            }
            Err(err) => {
                state.set_error(&err.to_string());
                0
            }
        }
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_register_clip_samples(
    handle: u64,
    channels: u32,
    sample_rate: u32,
    frame_count: u64,
    in_buffer: u64,
) -> u64 {
    if (channels != 1 && channels != 2) || sample_rate == 0 {
        return 0;
    }
    let Ok(frames) = usize::try_from(frame_count) else {
        return 0;
    };
    let ch = channels as usize;
    let Some(sample_count) = frames.checked_mul(ch) else {
        return 0;
    };
    let Some(byte_len) = sample_count.checked_mul(4) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let samples = if in_buffer != 0 {
            if in_buffer != state.path_buffer.as_ptr() as usize as u64
                || state.path_buffer.len() < byte_len
            {
                state.set_error("buffer address mismatch");
                return 0;
            }
            let mut decoded = Vec::with_capacity(sample_count);
            for idx in 0..sample_count {
                let start = idx * 4;
                let Ok(bytes) = state.path_buffer[start..start + 4].try_into() else {
                    return 0;
                };
                decoded.push(f32::from_le_bytes(bytes));
            }
            decoded
        } else {
            vec![0.5f32; sample_count]
        };
        let clip = AudioClip::new(channels as u16, sample_rate, samples);
        let clip_id = state.runtime.audio_mut().register_clip(clip);
        state.clear_error();
        clip_id
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_clip_info(
    handle: u64,
    clip_id: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 16
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(clip) = state.runtime.audio().clip(clip_id) else {
            state.set_error("audio clip not found");
            return -4;
        };
        let duration = clip.duration;
        let sample_rate = clip.sample_rate;
        let channels = u32::from(clip.channels);
        state.path_buffer[0..8].copy_from_slice(&duration.to_le_bytes());
        state.path_buffer[8..12].copy_from_slice(&sample_rate.to_le_bytes());
        state.path_buffer[12..16].copy_from_slice(&channels.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn unitas_runtime_audio_source_config(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 44
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let id = EntityId::new(entity);
        let Some(config) = state.runtime.audio_source_config(id).cloned() else {
            state.set_error("audio source not found");
            return -4;
        };
        let clip_id = state
            .runtime
            .audio()
            .source(id)
            .and_then(|s| s.clip)
            .unwrap_or(0);
        let clip_duration = if clip_id != 0 {
            state
                .runtime
                .audio()
                .clip(clip_id)
                .map_or(0.0, |c| c.duration)
        } else {
            0.0
        };
        state.path_buffer[0..4].copy_from_slice(&config.volume.to_le_bytes());
        state.path_buffer[4..8].copy_from_slice(&config.pitch.to_le_bytes());
        state.path_buffer[8..12].copy_from_slice(&config.spatial_blend.to_le_bytes());
        state.path_buffer[12..16].copy_from_slice(&config.min_distance.to_le_bytes());
        state.path_buffer[16..20].copy_from_slice(&config.max_distance.to_le_bytes());
        state.path_buffer[20..28].copy_from_slice(&clip_id.to_le_bytes());
        state.path_buffer[28..36].copy_from_slice(&clip_duration.to_le_bytes());
        state.path_buffer[36] = config.priority;
        state.path_buffer[37] = u8::from(config.looping);
        state.path_buffer[38] = u8::from(config.play_on_awake);
        state.path_buffer[39] = u8::from(config.mute);
        state.path_buffer[40] = u8::from(config.enabled);
        state.path_buffer[41..44].fill(0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

pub fn test_read_query_buffer(handle: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
    if !data.len().is_multiple_of(QUERY_RECORD_SIZE) {
        return Err(RuntimeError::InvalidAddress);
    }
    match with_runtime_state_err(handle, |state| {
        let length = data.len().min(state.query_buffer.len());
        data[..length].copy_from_slice(&state.query_buffer[..length]);
        Ok(())
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

fn run_reentrant_operation<T, F>(handle: u64, state: &mut RuntimeState, op: F) -> T
where
    F: FnOnce(&mut Runtime, &mut dyn FnMut(&mut Runtime, &mut dyn FnMut())) -> T,
{
    let mut hook = |runtime: &mut Runtime, callback: &mut dyn FnMut()| {
        let Ok(placeholder_runtime) = Runtime::new() else {
            callback();
            return;
        };
        let active_runtime = std::mem::replace(runtime, placeholder_runtime);
        let mut temp_state = RuntimeState::new(active_runtime);
        std::mem::swap(&mut temp_state.path_buffer, &mut state.path_buffer);
        std::mem::swap(&mut temp_state.query_buffer, &mut state.query_buffer);
        std::mem::swap(&mut temp_state.query_records, &mut state.query_records);
        std::mem::swap(&mut temp_state.last_error, &mut state.last_error);

        ACTIVE_STATES.with(|cell| {
            cell.borrow_mut().push((handle, temp_state));
        });

        callback();

        let popped = ACTIVE_STATES.with(|cell| cell.borrow_mut().pop());
        if let Some((_, mut returned_state)) = popped {
            std::mem::swap(&mut returned_state.path_buffer, &mut state.path_buffer);
            std::mem::swap(&mut returned_state.query_buffer, &mut state.query_buffer);
            std::mem::swap(&mut returned_state.query_records, &mut state.query_records);
            std::mem::swap(&mut returned_state.last_error, &mut state.last_error);
            *runtime = returned_state.runtime;
        }
    };
    op(&mut state.runtime, &mut hook)
}

fn with_active_state<T>(
    handle: u64,
    action: impl FnOnce(&mut RuntimeState) -> T,
) -> Option<T> {
    ACTIVE_STATES.with(|cell| {
        let mut stack = cell.borrow_mut();
        for (active_handle, state) in stack.iter_mut().rev() {
            if *active_handle == handle {
                return Some(action(state));
            }
        }
        None
    })
}

fn take_from_registry(handle: u64) -> Option<RuntimeState> {
    let reg = registry().ok()?;
    let mut map = reg.lock().ok()?;
    map.remove(&handle)
}

fn read_path_argument(handle: u64, address: u64, length: usize) -> Result<String, i32> {
    match with_runtime_state(handle, |state| read_path_buffer(state, address, length)) {
        Ok(result) => result,
        Err(code) => Err(code),
    }
}

fn run_scene_operation(
    handle: u64,
    op: impl FnOnce(&mut Runtime, &mut dyn FnMut(&mut Runtime, &mut dyn FnMut())) -> Result<(), RuntimeError>,
) -> i32 {
    if with_active_state(handle, |_| ()).is_some() {
        let _ = with_active_state(handle, |state| {
            state.set_error("scene operations cannot run inside a runtime callback");
        });
        return -5;
    }
    let mut state = match take_from_registry(handle) {
        Some(state) => state,
        None => return -1,
    };
    let result = run_reentrant_operation(handle, &mut state, |runtime, hook| op(runtime, hook));
    let code = match result {
        Ok(()) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -4
        }
    };
    if !restore_to_registry(handle, state) {
        return -1;
    }
    code
}

fn restore_to_registry(handle: u64, state: RuntimeState) -> bool {
    let Ok(reg) = registry() else {
        return false;
    };
    let Ok(mut map) = reg.lock() else {
        return false;
    };
    map.insert(handle, state);
    true
}

fn scene_name_from_input(text: &str) -> String {
    let path_like = text.contains('/') || text.contains('\\') || text.ends_with(".unity");
    if !path_like {
        return text.to_string();
    }
    Path::new(text)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(text)
        .to_string()
}

fn read_path_buffer(state: &mut RuntimeState, address: u64, length: usize) -> Result<String, i32> {
    if address == 0 || address != state.path_buffer.as_ptr() as usize as u64 {
        state.set_error("buffer address mismatch");
        return Err(-2);
    }
    if length > state.path_buffer.len() {
        state.set_error("buffer length exceeds capacity");
        return Err(-2);
    }
    match std::str::from_utf8(&state.path_buffer[..length]) {
        Ok(text) => Ok(text.trim_end_matches('\0').to_string()),
        Err(err) => {
            let msg = err.to_string();
            state.set_error(&msg);
            Err(-3)
        }
    }
}

fn with_runtime_state<T>(
    handle: u64,
    action: impl FnOnce(&mut RuntimeState) -> T,
) -> Result<T, i32> {
    let mut action_opt = Some(action);
    if let Some(result) = ACTIVE_STATES.with(|cell| {
        let mut stack = cell.borrow_mut();
        for (active_handle, state) in stack.iter_mut().rev() {
            if *active_handle == handle {
                if let Some(f) = action_opt.take() {
                    return Some(f(state));
                }
            }
        }
        None
    }) {
        return Ok(result);
    }
    let Some(f) = action_opt else {
        return Err(-1);
    };
    let reg = registry().map_err(|_| -1)?;
    let mut map = reg.lock().map_err(|_| -1)?;
    let state = map.get_mut(&handle).ok_or(-1)?;
    Ok(f(state))
}

fn with_runtime_state_err<T>(
    handle: u64,
    action: impl FnOnce(&mut RuntimeState) -> Result<T, RuntimeError>,
) -> Result<Result<T, RuntimeError>, RuntimeError> {
    let mut action_opt = Some(action);
    if let Some(result) = ACTIVE_STATES.with(|cell| {
        let mut stack = cell.borrow_mut();
        for (active_handle, state) in stack.iter_mut().rev() {
            if *active_handle == handle {
                if let Some(f) = action_opt.take() {
                    return Some(f(state));
                }
            }
        }
        None
    }) {
        return Ok(result);
    }
    let Some(f) = action_opt else {
        return Err(RuntimeError::NotFound);
    };
    let reg = registry()?;
    let mut map = match reg.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(RuntimeError::Poisoned),
    };
    let state = match map.get_mut(&handle) {
        Some(s) => s,
        None => return Err(RuntimeError::NotFound),
    };
    Ok(f(state))
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
