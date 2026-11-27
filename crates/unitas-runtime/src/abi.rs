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

#[path = "abi_animator.rs"]
mod abi_animator;
#[path = "abi_audio.rs"]
mod abi_audio;
#[path = "abi_input.rs"]
mod abi_input;
#[path = "abi_jobs.rs"]
mod abi_jobs;
#[path = "abi_physics2d.rs"]
mod abi_physics2d;
#[path = "abi_physics3d.rs"]
mod abi_physics3d;
#[path = "abi_player_loop.rs"]
mod abi_player_loop;
#[path = "abi_queries.rs"]
mod abi_queries;
#[path = "abi_render.rs"]
mod abi_render;
#[path = "abi_trails.rs"]
mod abi_trails;
#[path = "abi_ui.rs"]
mod abi_ui;

pub use abi_animator::*;
pub use abi_audio::*;
pub use abi_input::*;
pub use abi_jobs::*;
pub use abi_physics2d::*;
pub use abi_physics3d::*;
pub use abi_player_loop::*;
pub use abi_queries::*;
pub use abi_render::*;
pub use abi_trails::*;
pub use abi_ui::*;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use unitas_animation::decode_controller_binary;
use unitas_audio::{AudioClip, AudioMixerGroup};
use unitas_core::EntityId;
use unitas_jobs::JobHandle;
use unitas_physics::trails::TrailSettings;
use unitas_physics::two_d::body::{
    BodyDesc2D, BodyType2D, CollisionDetectionMode2D, RigidbodyInterpolation2D,
};
use unitas_physics::two_d::{
    ColliderDesc2D, ColliderId2D, ContactFilter2D, PlatformEffector2D, SurfaceEffector2D,
};
use unitas_physics::{CollisionDetectionMode3D, CollisionPayload, ForceMode};
use unitas_render::Vec3;

use crate::error::RuntimeError;
use crate::jobs::job_system;
use crate::physics::CollisionPayload2D;
use crate::player_loop::{self, CustomSystemPosition};
pub use crate::runtime::CharacterMoveRequest;
use crate::runtime::{
    Callbacks, LoadSceneMode, QueryRecord, QueryRecord2D, RenderCameraParams, Runtime,
};

const QUERY_RECORD_SIZE: usize = 64;
const QUERY_RECORD_2D_SIZE: usize = 32;
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionSummary {
    pub impulse: f32,
    pub relative_velocity: [f32; 3],
    pub contact_count: u32,
    pub point: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionContactPoint {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub impulse: f32,
}

/// The 2D collision summary the managed `Collision2D` reads.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionSummary2D {
    pub impulse: f32,
    pub relative_velocity: [f32; 2],
    pub contact_count: u32,
    pub point: [f32; 2],
    pub normal: [f32; 2],
}

/// One 2D contact point the managed `ContactPoint2D` reads.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionContactPoint2D {
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub impulse: f32,
}

pub struct RuntimeState {
    pub runtime: Runtime,
    pub path_buffer: Vec<u8>,
    pub query_buffer: Vec<u8>,
    pub query_buffer_2d: Vec<u8>,
    pub render_buffer: Vec<u8>,
    pub player_loop_buffer: Vec<u8>,
    pub query_records: Vec<QueryRecord>,
    pub query_records_2d: Vec<QueryRecord2D>,
    pub last_error: Option<Vec<u8>>,
}

impl RuntimeState {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            path_buffer: Vec::new(),
            query_buffer: Vec::new(),
            query_buffer_2d: Vec::new(),
            render_buffer: Vec::new(),
            player_loop_buffer: Vec::new(),
            query_records: Vec::new(),
            query_records_2d: Vec::new(),
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

static REGISTRY: Mutex<BTreeMap<u64, RuntimeState>> = Mutex::new(BTreeMap::new());
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
static INVALID_HANDLE_ERROR_TARGET: AtomicU64 = AtomicU64::new(0);
const INVALID_HANDLE_MESSAGE: &[u8] = b"runtime handle not found\0";

thread_local! {
    static ACTIVE_STATES: RefCell<Vec<(u64, RuntimeState)>> = const { RefCell::new(Vec::new()) };
}

fn registry() -> &'static Mutex<BTreeMap<u64, RuntimeState>> {
    &REGISTRY
}

ffi_export! {
pub extern "C" fn unitas_runtime_new() -> u64 {
    let runtime = match Runtime::new() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let reg = registry();
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
pub extern "C" fn unitas_runtime_object_create(
    handle: u64,
    name_address: u64,
    name_length: u64,
) -> u64 {
    let Ok(len) = usize::try_from(name_length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let name = if len == 0 {
            String::from("GameObject")
        } else {
            let Ok(text) = read_path_buffer(state, name_address, len) else {
                return 0;
            };
            text
        };
        match state.runtime.create_runtime_object(&name) {
            Ok(entity) => {
                state.clear_error();
                entity.as_u64()
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
pub extern "C" fn unitas_runtime_object_attach_script(
    handle: u64,
    entity: u64,
    guid_hi: u64,
    guid_lo: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    if with_active_state(handle, |_| ()).is_some() {
        let mut active_callbacks = None;
        let _ = with_active_state(handle, |state| {
            let scene_index = state
                .runtime
                .scenes
                .iter()
                .position(|scene| scene.graph.contains(EntityId::new(entity)));
            let is_persistent = state
                .runtime
                .persistent_entities
                .contains(&EntityId::new(entity));
            if scene_index.is_some() || is_persistent {
                active_callbacks = Some((state.runtime.callbacks, scene_index));
            } else {
                state.set_error(&RuntimeError::EntityNotFound(EntityId::new(entity)).to_string());
            }
        });
        let Some((callbacks, scene_index)) = active_callbacks else {
            return -4;
        };
        let script_handle = match callbacks {
            Some(cb) => (cb.create_script)(entity, guid_hi, guid_lo),
            None => 0,
        };
        let instance = crate::runtime::ScriptInstance {
            entity: EntityId::new(entity),
            guid: format!("{guid_hi:016x}{guid_lo:016x}"),
            guid_hi,
            guid_lo,
            script_handle,
            awake_called: false,
            on_enable_called: false,
            start_called: false,
        };
        let mut attached = false;
        let _ = with_active_state(handle, |state| {
            if let Some(index) = scene_index {
                if let Some(scene) = state.runtime.scenes.get_mut(index) {
                    scene.scripts.push(instance);
                    attached = true;
                }
            } else {
                state.runtime.persistent.push(instance);
                attached = true;
            }
            if attached {
                state.runtime.refresh_script_view();
                state.clear_error();
            }
        });
        return if attached { 0 } else { -4 };
    }
    let mut state = match take_from_registry(handle) {
        Some(s) => s,
        None => return -1,
    };
    let result = run_reentrant_operation(handle, &mut state, |runtime, hook| {
        runtime.attach_runtime_script_with_hook(EntityId::new(entity), guid_hi, guid_lo, hook)
    });
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
}

ffi_export! {
pub extern "C" fn unitas_runtime_object_set_transform(
    handle: u64,
    entity: u64,
    position_x: f64,
    position_y: f64,
    position_z: f64,
    rotation_x: f64,
    rotation_y: f64,
    rotation_z: f64,
    rotation_w: f64,
    scale_x: f64,
    scale_y: f64,
    scale_z: f64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        match state.runtime.set_runtime_object_transform(
            EntityId::new(entity),
            [position_x, position_y, position_z],
            [rotation_x, rotation_y, rotation_z, rotation_w],
            [scale_x, scale_y, scale_z],
        ) {
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
pub extern "C" fn unitas_runtime_object_create_body(
    handle: u64,
    entity: u64,
    shape: u32,
    radius: f64,
    half_x: f64,
    half_y: f64,
    half_z: f64,
    height: f64,
    is_trigger: u32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if shape > 2 {
            state.set_error("invalid runtime body shape");
            return -2;
        }
        match state.runtime.create_runtime_body(
            EntityId::new(entity),
            shape,
            radius,
            [half_x, half_y, half_z],
            height,
            is_trigger != 0,
        ) {
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
    let reg = registry();
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
pub extern "C" fn unitas_runtime_set_fixed_timestep(handle: u64, seconds: f64) -> i32 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 1;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.set_fixed_timestep(seconds);
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

fn to_f32(vector: [f64; 3]) -> [f32; 3] {
    [vector[0] as f32, vector[1] as f32, vector[2] as f32]
}

fn write_output<T>(out: *mut T, value: T) -> bool {
    if out.is_null() {
        return false;
    }
    // SAFETY: the caller owns the memory the pointer names and passes a pointer to a
    // writable value of the same repr(C) type the export declares.
    unsafe { out.write(value) };
    true
}

fn with_collision_payload<T>(
    first: u64,
    second: u64,
    action: &impl Fn(&CollisionPayload) -> T,
) -> Option<T> {
    if let Some(value) =
        with_active_runtime(|runtime| runtime.collision_payload(first, second).map(action))
    {
        return Some(value);
    }
    with_registered_runtime(|runtime| runtime.collision_payload(first, second).map(action))
}

fn with_active_runtime<T>(action: impl Fn(&mut Runtime) -> Option<T>) -> Option<T> {
    ACTIVE_STATES.with(|cell| {
        let mut stack = cell.borrow_mut();
        for (_, state) in stack.iter_mut().rev() {
            if let Some(value) = action(&mut state.runtime) {
                return Some(value);
            }
        }
        None
    })
}

fn with_registered_runtime<T>(action: impl Fn(&mut Runtime) -> Option<T>) -> Option<T> {
    let Ok(mut map) = registry().lock() else {
        return None;
    };
    for state in map.values_mut() {
        if let Some(value) = action(&mut state.runtime) {
            return Some(value);
        }
    }
    None
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
        std::mem::swap(&mut temp_state.query_buffer_2d, &mut state.query_buffer_2d);
        std::mem::swap(&mut temp_state.render_buffer, &mut state.render_buffer);
        std::mem::swap(&mut temp_state.query_records, &mut state.query_records);
        std::mem::swap(
            &mut temp_state.query_records_2d,
            &mut state.query_records_2d,
        );
        std::mem::swap(&mut temp_state.last_error, &mut state.last_error);

        ACTIVE_STATES.with(|cell| {
            cell.borrow_mut().push((handle, temp_state));
        });

        callback();

        let popped = ACTIVE_STATES.with(|cell| cell.borrow_mut().pop());
        if let Some((_, mut returned_state)) = popped {
            std::mem::swap(&mut returned_state.path_buffer, &mut state.path_buffer);
            std::mem::swap(&mut returned_state.query_buffer, &mut state.query_buffer);
            std::mem::swap(
                &mut returned_state.query_buffer_2d,
                &mut state.query_buffer_2d,
            );
            std::mem::swap(&mut returned_state.render_buffer, &mut state.render_buffer);
            std::mem::swap(&mut returned_state.query_records, &mut state.query_records);
            std::mem::swap(
                &mut returned_state.query_records_2d,
                &mut state.query_records_2d,
            );
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
    let mut map = registry().lock().ok()?;
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
    let Ok(mut map) = registry().lock() else {
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
    let Ok(mut map) = registry().lock() else {
        return Err(-1);
    };
    let Some(state) = map.get_mut(&handle) else {
        return Err(-1);
    };
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
    let mut map = match registry().lock() {
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
