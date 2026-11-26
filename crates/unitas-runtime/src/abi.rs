macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use unitas_animation::decode_controller_binary;
use unitas_audio::AudioClip;
use unitas_core::EntityId;
use unitas_physics::{CollisionDetectionMode3D, ForceMode};
use unitas_render::Vec3;

use crate::error::RuntimeError;
pub use crate::runtime::CharacterMoveRequest;
use crate::runtime::{Callbacks, LoadSceneMode, QueryRecord, RenderCameraParams, Runtime};

const QUERY_RECORD_SIZE: usize = 64;
const ANIMATOR_STATE_INFO_SIZE: usize = 80;
const CAMERA_BLOB_SIZE: usize = 32;
const CAMERA_BLOB_FLOATS: usize = 8;

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
    pub render_buffer: Vec<u8>,
    pub query_records: Vec<QueryRecord>,
    pub last_error: Option<Vec<u8>>,
}

impl RuntimeState {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            path_buffer: Vec::new(),
            query_buffer: Vec::new(),
            render_buffer: Vec::new(),
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

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_unload_scene(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    let name = match read_path_argument(handle, address, len) {
        Ok(text) => scene_name_from_input(&text),
        Err(code) => return code,
    };
    run_scene_operation(handle, |runtime, hook| {
        runtime.unload_scene_with_hook(&name, hook)
    })
}
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_root_count(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let roots = state.runtime.scene_graph().map_or(0, |graph| graph.roots().len());
        match i32::try_from(roots) {
            Ok(count) => count,
            Err(_) => {
                state.set_error("root object count exceeds supported range");
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

fn write_string_to_path_buffer(state: &mut RuntimeState, value: &str, overflow_error: &str) -> i32 {
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

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_time_scale(handle: u64, time_scale: f64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.set_time_scale(time_scale);
        state.clear_error();
    }) {
        Ok(()) => 0,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_time_scale(handle: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.time_scale()
    })
    .unwrap_or(0.0)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_maximum_delta_time(handle: u64, seconds: f64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.set_maximum_delta_time(seconds);
        state.clear_error();
    }) {
        Ok(()) => 0,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_maximum_delta_time(handle: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.maximum_delta_time()
    })
    .unwrap_or(0.0)
}
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_teleport_body_position(
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
            .teleport_body_position(EntityId::new(entity), vector)
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_rotation(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(rotation) = read_vec4_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_rotation(EntityId::new(entity), rotation)
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_set_collision_detection_mode(
    handle: u64,
    entity: u64,
    mode: u32,
) -> i32 {
    let mode = match mode {
        0 => CollisionDetectionMode3D::Discrete,
        1 => CollisionDetectionMode3D::Continuous,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown collision detection mode");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    match with_runtime_state(handle, |state| {
        match state
            .runtime
            .physics_mut()
            .set_collision_detection_mode_3d(EntityId::new(entity), mode)
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_force(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some((force, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .add_body_force(EntityId::new(entity), force, mode)
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

fn read_vec4_from_path_buffer(state: &RuntimeState, in_buffer: u64) -> Option<[f64; 4]> {
    if in_buffer == 0
        || in_buffer != state.path_buffer.as_ptr() as usize as u64
        || state.path_buffer.len() < 32
    {
        return None;
    }
    let x_bytes: [u8; 8] = state.path_buffer[0..8].try_into().ok()?;
    let y_bytes: [u8; 8] = state.path_buffer[8..16].try_into().ok()?;
    let z_bytes: [u8; 8] = state.path_buffer[16..24].try_into().ok()?;
    let w_bytes: [u8; 8] = state.path_buffer[24..32].try_into().ok()?;
    Some([
        f64::from_le_bytes(x_bytes),
        f64::from_le_bytes(y_bytes),
        f64::from_le_bytes(z_bytes),
        f64::from_le_bytes(w_bytes),
    ])
}

fn read_force_from_path_buffer(
    state: &RuntimeState,
    in_buffer: u64,
) -> Option<([f64; 3], ForceMode)> {
    if in_buffer == 0
        || in_buffer != state.path_buffer.as_ptr() as usize as u64
        || state.path_buffer.len() < 32
    {
        return None;
    }
    let x_bytes: [u8; 8] = state.path_buffer[0..8].try_into().ok()?;
    let y_bytes: [u8; 8] = state.path_buffer[8..16].try_into().ok()?;
    let z_bytes: [u8; 8] = state.path_buffer[16..24].try_into().ok()?;
    let mode_bytes: [u8; 8] = state.path_buffer[24..32].try_into().ok()?;
    let mode = match f64::from_le_bytes(mode_bytes) {
        0.0 => ForceMode::Force,
        1.0 => ForceMode::Impulse,
        2.0 => ForceMode::Acceleration,
        3.0 => ForceMode::VelocityChange,
        _ => return None,
    };
    Some((
        [
            f64::from_le_bytes(x_bytes),
            f64::from_le_bytes(y_bytes),
            f64::from_le_bytes(z_bytes),
        ],
        mode,
    ))
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

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_axis_event(
    handle: u64,
    address: u64,
    length: u64,
    value: f64,
) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        if !value.is_finite() {
            state.set_error("non-finite axis value");
            return -2;
        }
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
        if state.runtime.input_axis_event(&name, value) {
            state.clear_error();
            0
        } else {
            state.set_error(&format!("unknown input axis: {name}"));
            -5
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_button_state(handle: u64, address: u64, length: u64) -> i32 {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        let id = state.runtime.ensure_audio_source(entity);
        state.clear_error();
        id.as_u64()
    })
    .unwrap_or_default()
}
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_time(handle: u64, entity: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.audio().source_time(EntityId::new(entity))
    })
    .unwrap_or(0.0)
}
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_paused(handle: u64, paused: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_application_paused(paused != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_load_clip(handle: u64, address: u64, length: u64) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Ok(path_str) = read_path_buffer(state, address, len) else {
            return 0;
        };
        match state
            .runtime
            .audio_mut()
            .load_wav_file(Path::new(&path_str))
        {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

ffi_export! {
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
}

fn read_runtime_bytes(state: &RuntimeState, address: u64, length: usize) -> Option<Vec<u8>> {
    if address == 0 {
        return None;
    }
    let Ok(start) = usize::try_from(address) else {
        return None;
    };
    let base = state.path_buffer.as_ptr() as usize;
    let offset = start.checked_sub(base)?;
    let end = offset.checked_add(length)?;
    if end > state.path_buffer.len() {
        return None;
    }
    Some(state.path_buffer[offset..end].to_vec())
}

fn read_animator_name(
    state: &mut RuntimeState,
    address: u64,
    length: usize,
) -> Result<String, i32> {
    let Some(bytes) = read_runtime_bytes(state, address, length) else {
        state.set_error("buffer address mismatch");
        return Err(-2);
    };
    match String::from_utf8(bytes) {
        Ok(name) => Ok(name),
        Err(err) => {
            let message = err.to_string();
            state.set_error(&message);
            Err(-3)
        }
    }
}

fn animator_out_buffer_ready(state: &RuntimeState, out_buffer: u64, required: usize) -> bool {
    out_buffer != 0
        && out_buffer == state.path_buffer.as_ptr() as usize as u64
        && state.path_buffer.len() >= required
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_controller_create(
    handle: u64,
    address: u64,
    length: u64,
) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Some(bytes) = read_runtime_bytes(state, address, len) else {
            state.set_error("buffer address mismatch");
            return 0;
        };
        let controller = match decode_controller_binary(&bytes) {
            Ok(controller) => controller,
            Err(err) => {
                state.set_error(&err.to_string());
                return 0;
            }
        };
        let controller_handle = state
            .runtime
            .animation_mut()
            .register_controller(controller);
        state.clear_error();
        controller_handle
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_controller_destroy(handle: u64, controller: u64) -> i32 {
    if controller == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if !state.runtime.animation_mut().remove_controller(controller) {
            state.set_error("animator controller not found");
            return -4;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_attach(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        let id = state.runtime.ensure_animator(entity);
        state.clear_error();
        id.as_u64()
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_controller(
    handle: u64,
    entity: u64,
    controller: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let result = state
            .runtime
            .set_animator_controller(EntityId::new(entity), controller);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_float(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: f32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_float(id, &name, value);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_get_float(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_float(id, &name);
        match result {
            Ok(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_int(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: i32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_int(id, &name, value);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_get_int(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_int(id, &name);
        match result {
            Ok(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_bool(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: u32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state
            .runtime
            .animation_mut()
            .set_bool(id, &name, value != 0);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_get_bool(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_bool(id, &name);
        match result {
            Ok(value) => {
                let encoded = u32::from(value);
                state.path_buffer[0..4].copy_from_slice(&encoded.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_trigger(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_trigger(id, &name);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_reset_trigger(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().reset_trigger(id, &name);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_speed(handle: u64, entity: u64, speed: f32) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_speed(id, speed);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_get_speed(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let speed = state.runtime.animation().speed(id);
        match speed {
            Some(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            None => {
                state.set_error("animator not found");
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_state_info(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, ANIMATOR_STATE_INFO_SIZE) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let info = match state.runtime.animation().state_info(id) {
            Ok(info) => info,
            Err(message) => {
                state.set_error(&message);
                return -4;
            }
        };
        let transform = state
            .runtime
            .animation()
            .transform(id)
            .cloned()
            .unwrap_or_default();
        let fired_events = state
            .runtime
            .animation()
            .fired_event_count(id)
            .unwrap_or_default();
        let diagnostics = state
            .runtime
            .animation()
            .diagnostic_count(id)
            .unwrap_or_default();
        let next_state_index = match info.next_state_index {
            Some(index) => u32::try_from(index).unwrap_or(u32::MAX),
            None => u32::MAX,
        };
        let mut bytes = [0u8; ANIMATOR_STATE_INFO_SIZE];
        let state_index = u32::try_from(info.state_index).unwrap_or(u32::MAX);
        bytes[0..4].copy_from_slice(&state_index.to_le_bytes());
        bytes[4..8].copy_from_slice(&info.normalized_time.to_le_bytes());
        bytes[8..12].copy_from_slice(&info.length.to_le_bytes());
        bytes[12..16].copy_from_slice(&info.speed.to_le_bytes());
        bytes[16..20].copy_from_slice(&info.weight.to_le_bytes());
        bytes[20..24].copy_from_slice(&u32::from(info.is_in_transition).to_le_bytes());
        bytes[24..28].copy_from_slice(&next_state_index.to_le_bytes());
        bytes[28..32].copy_from_slice(&info.next_state_weight.to_le_bytes());
        bytes[32..36].copy_from_slice(&transform.translation[0].to_le_bytes());
        bytes[36..40].copy_from_slice(&transform.translation[1].to_le_bytes());
        bytes[40..44].copy_from_slice(&transform.translation[2].to_le_bytes());
        bytes[44..48].copy_from_slice(&transform.rotation[0].to_le_bytes());
        bytes[48..52].copy_from_slice(&transform.rotation[1].to_le_bytes());
        bytes[52..56].copy_from_slice(&transform.rotation[2].to_le_bytes());
        bytes[56..60].copy_from_slice(&transform.rotation[3].to_le_bytes());
        bytes[60..64].copy_from_slice(&transform.scale[0].to_le_bytes());
        bytes[64..68].copy_from_slice(&transform.scale[1].to_le_bytes());
        bytes[68..72].copy_from_slice(&transform.scale[2].to_le_bytes());
        let fired_event_count = u32::try_from(fired_events).unwrap_or(u32::MAX);
        let diagnostic_count = u32::try_from(diagnostics).unwrap_or(u32::MAX);
        bytes[72..76].copy_from_slice(&fired_event_count.to_le_bytes());
        bytes[76..80].copy_from_slice(&diagnostic_count.to_le_bytes());
        state.path_buffer[0..ANIMATOR_STATE_INFO_SIZE].copy_from_slice(&bytes);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_state_name(handle: u64, entity: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let info = match state.runtime.animation().state_info(id) {
            Ok(info) => info,
            Err(message) => {
                state.set_error(&message);
                return -4;
            }
        };
        let bytes = info.state_name.as_bytes();
        if state.path_buffer.len() < bytes.len() {
            state.set_error(&format!(
                "path buffer too small for state name: {} bytes needed",
                bytes.len()
            ));
            return -4;
        }
        state.path_buffer[..bytes.len()].copy_from_slice(bytes);
        state.clear_error();
        match i32::try_from(bytes.len()) {
            Ok(length) => length,
            Err(_) => {
                state.set_error("state name length exceeds supported range");
                -4
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

pub fn test_read_error_message(handle: u64) -> Result<String, RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        Ok(state.last_error.as_ref().map_or_else(String::new, |bytes| {
            let trimmed = bytes.strip_suffix(&[0]).unwrap_or(bytes);
            String::from_utf8_lossy(trimmed).to_string()
        }))
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

pub fn test_read_render_buffer(handle: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        let length = data.len().min(state.render_buffer.len());
        data[..length].copy_from_slice(&state.render_buffer[..length]);
        Ok(())
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
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
        std::mem::swap(&mut temp_state.render_buffer, &mut state.render_buffer);
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
            std::mem::swap(&mut returned_state.render_buffer, &mut state.render_buffer);
            std::mem::swap(&mut returned_state.query_records, &mut state.query_records);
            std::mem::swap(&mut returned_state.last_error, &mut state.last_error);
            *runtime = returned_state.runtime;
        }
    };
    op(&mut state.runtime, &mut hook)
}

fn with_active_state<T>(handle: u64, action: impl FnOnce(&mut RuntimeState) -> T) -> Option<T> {
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
    op: impl FnOnce(
        &mut Runtime,
        &mut dyn FnMut(&mut Runtime, &mut dyn FnMut()),
    ) -> Result<(), RuntimeError>,
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
            if *active_handle == handle
                && let Some(f) = action_opt.take()
            {
                return Some(f(state));
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
            if *active_handle == handle
                && let Some(f) = action_opt.take()
            {
                return Some(f(state));
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

fn buffer_offset(base: usize, length: usize, address: usize, size: usize) -> Option<usize> {
    let end = base.checked_add(length)?;
    let target_end = address.checked_add(size)?;
    if address < base || target_end > end {
        return None;
    }
    Some(address - base)
}

fn path_buffer_offset(state: &RuntimeState, address: usize, size: usize) -> Option<usize> {
    buffer_offset(
        state.path_buffer.as_ptr() as usize,
        state.path_buffer.len(),
        address,
        size,
    )
}

fn read_f32_le(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(raw)
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_buffer(handle: u64, capacity: u64) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let Ok(size) = usize::try_from(capacity) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        if state.render_buffer.len() < size {
            state.render_buffer.resize(size, 0);
        }
        state.render_buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_mesh_create(
    handle: u64,
    positions: *const f32,
    position_count: u32,
    normals: *const f32,
    normal_count: u32,
    indices: *const u32,
    index_count: u32,
) -> u64 {
    if !position_count.is_multiple_of(3)
        || !normal_count.is_multiple_of(3)
        || !index_count.is_multiple_of(3)
        || position_count != normal_count
    {
        return with_runtime_state(handle, |state| {
            state.set_error(
                "-2 invalid argument: mesh vertex, normal and index counts do not match",
            );
            0u64
        })
        .unwrap_or_default();
    }
    with_runtime_state(handle, |state| {
        let position_bytes = position_count as usize * 4;
        let normal_bytes = normal_count as usize * 4;
        let index_bytes = index_count as usize * 4;
        let Some(position_offset) = path_buffer_offset(state, positions as usize, position_bytes)
        else {
            state.set_error("-2 invalid argument: mesh position buffer is out of range");
            return 0u64;
        };
        let Some(normal_offset) = path_buffer_offset(state, normals as usize, normal_bytes) else {
            state.set_error("-2 invalid argument: mesh normal buffer is out of range");
            return 0u64;
        };
        let Some(index_offset) = path_buffer_offset(state, indices as usize, index_bytes) else {
            state.set_error("-2 invalid argument: mesh index buffer is out of range");
            return 0u64;
        };
        let mut position_values = Vec::with_capacity(position_count as usize / 3);
        for index in 0..position_count as usize / 3 {
            let start = position_offset + index * 12;
            position_values.push(Vec3::new(
                read_f32_le(&state.path_buffer, start),
                read_f32_le(&state.path_buffer, start + 4),
                read_f32_le(&state.path_buffer, start + 8),
            ));
        }
        let mut normal_values = Vec::with_capacity(normal_count as usize / 3);
        for index in 0..normal_count as usize / 3 {
            let start = normal_offset + index * 12;
            normal_values.push(Vec3::new(
                read_f32_le(&state.path_buffer, start),
                read_f32_le(&state.path_buffer, start + 4),
                read_f32_le(&state.path_buffer, start + 8),
            ));
        }
        let mut index_values = Vec::with_capacity(index_count as usize);
        for index in 0..index_count as usize {
            index_values.push(read_u32_le(&state.path_buffer, index_offset + index * 4));
        }
        state
            .runtime
            .create_render_mesh(position_values, normal_values, index_values)
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_mesh_destroy(handle: u64, mesh: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if state.runtime.destroy_render_mesh(mesh) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: mesh handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_material_create(handle: u64) -> u64 {
    with_runtime_state(handle, |state| state.runtime.create_render_material()).unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_material_set_color(
    handle: u64,
    material: u64,
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_render_material_color(material, [red, green, blue, alpha])
        {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: material handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: camera entity id is zero");
            return 0u64;
        }
        state
            .runtime
            .create_render_camera(Some(EntityId::new(entity)))
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_update(
    handle: u64,
    camera: u64,
    blob: *const f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if blob.is_null() {
            state.set_error("-2 invalid argument: camera blob pointer is null");
            return -2;
        }
        let base = state.path_buffer.as_ptr() as usize;
        let address = blob as usize;
        if address < base {
            state.set_error("-2 invalid argument: camera blob buffer address mismatch");
            return -2;
        }
        let offset = address - base;
        if offset > state.path_buffer.len() {
            state.set_error("-2 invalid argument: camera blob buffer address mismatch");
            return -2;
        }
        if state.path_buffer.len() - offset < CAMERA_BLOB_SIZE {
            state.set_error("-2 invalid argument: camera blob is shorter than 32 bytes");
            return -2;
        }
        let mut values = [0.0f32; CAMERA_BLOB_FLOATS];
        for (index, value) in values.iter_mut().enumerate() {
            *value = read_f32_le(&state.path_buffer, offset + index * 4);
        }
        let params = RenderCameraParams {
            orthographic: values[0] > 0.0,
            orthographic_size: values[0],
            field_of_view: values[1],
            near_clip_plane: values[2],
            far_clip_plane: values[3],
            background_color: [values[4], values[5], values[6]],
        };
        if state.runtime.update_render_camera(camera, params) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: camera handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_renderer_create(
    handle: u64,
    entity: u64,
    mesh: u64,
    material: u64,
) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: renderer entity id is zero");
            return 0u64;
        }
        if mesh == 0 {
            state.set_error("-2 invalid argument: renderer mesh handle is zero");
            return 0u64;
        }
        if material == 0 {
            state.set_error("-2 invalid argument: renderer material handle is zero");
            return 0u64;
        }
        state.runtime.create_render_renderer(
            Some(EntityId::new(entity)),
            Some(mesh),
            Some(material),
        )
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_renderer_set_enabled(
    handle: u64,
    renderer: u64,
    enabled: u8,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_render_renderer_enabled(renderer, enabled != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: renderer handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_light_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("-2 invalid argument: light entity id is zero");
            return 0u64;
        }
        state
            .runtime
            .create_render_light(Some(EntityId::new(entity)))
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_light_update(
    handle: u64,
    light: u64,
    direction_x: f32,
    direction_y: f32,
    direction_z: f32,
    color_red: f32,
    color_green: f32,
    color_blue: f32,
    intensity: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state.runtime.update_render_light(
            light,
            [direction_x, direction_y, direction_z],
            [color_red, color_green, color_blue],
            intensity,
        ) {
            state.clear_error();
            0
        } else {
            state.set_error("-1 unknown handle: light handle is not valid");
            -1
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_frame(
    handle: u64,
    camera: u64,
    width: u32,
    height: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        match state.runtime.render_frame(camera, width, height) {
            Ok(bytes) => {
                let count = bytes.len();
                state.render_buffer = bytes;
                state.clear_error();
                match i32::try_from(count) {
                    Ok(value) => value,
                    Err(_) => {
                        state.set_error(
                            "-2 invalid argument: frame size exceeds the supported range",
                        );
                        -2
                    }
                }
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_world_to_screen_point(
    handle: u64,
    camera: u64,
    x: f32,
    y: f32,
    z: f32,
    out: *mut f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out.is_null() {
            state.set_error("-2 invalid argument: output pointer is null");
            return -2;
        }
        let Some(offset) = path_buffer_offset(state, out as usize, 12) else {
            state.set_error("-2 invalid argument: output buffer is out of range");
            return -2;
        };
        match state
            .runtime
            .camera_world_to_screen_point(camera, [x, y, z])
        {
            Ok(values) => {
                for (index, value) in values.iter().enumerate() {
                    let start = offset + index * 4;
                    state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
                }
                state.clear_error();
                0
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_render_camera_screen_point_to_ray(
    handle: u64,
    camera: u64,
    x: f32,
    y: f32,
    _z: f32,
    out: *mut f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out.is_null() {
            state.set_error("-2 invalid argument: output pointer is null");
            return -2;
        }
        let Some(offset) = path_buffer_offset(state, out as usize, 24) else {
            state.set_error("-2 invalid argument: output buffer is out of range");
            return -2;
        };
        match state
            .runtime
            .camera_screen_point_to_ray(camera, [x, y, 0.0])
        {
            Ok(values) => {
                for (index, value) in values.iter().enumerate() {
                    let start = offset + index * 4;
                    state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
                }
                state.clear_error();
                0
            }
            Err(error) => {
                state.set_error(&error.to_string());
                error.class()
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_script_index_build(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let path_str = match read_path_buffer(state, address, len) {
            Ok(text) => text,
            Err(code) => return code,
        };
        let index = match crate::scripting::ProjectScriptIndex::build(Path::new(&path_str)) {
            Ok(idx) => idx,
            Err(err) => {
                state.set_error(&err.to_string());
                return -4;
            }
        };
        let manifest = index.to_manifest_tsv();
        write_string_to_path_buffer(
            state,
            &manifest,
            "script index manifest exceeds supported length",
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_resources_scan(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(l) => l,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let path_str = match read_path_buffer(state, address, len) {
            Ok(text) => text,
            Err(code) => return code,
        };
        let index = match crate::resources::ResourcesIndex::scan(Path::new(&path_str)) {
            Ok(idx) => idx,
            Err(err) => {
                state.set_error(&err.to_string());
                return -4;
            }
        };
        let mut lines = Vec::new();
        for entry in index.iter() {
            let guid = entry.guid.as_deref().unwrap_or("");
            lines.push(format!(
                "{}\t{}\t{}\t{}",
                entry.canonical_key,
                entry.resource_path,
                entry.file_path.display(),
                guid
            ));
        }
        let manifest = lines.join("\n");
        write_string_to_path_buffer(
            state,
            &manifest,
            "resources manifest exceeds supported length",
        )
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
