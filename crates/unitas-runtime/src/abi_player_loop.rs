use super::{
    CustomSystemPosition, RuntimeError, RuntimeState, player_loop, read_runtime_bytes,
    with_runtime_state, with_runtime_state_err,
};

fn read_runtime_string(state: &RuntimeState, address: u64, length: usize) -> Option<String> {
    let bytes = read_runtime_bytes(state, address, length)?;
    String::from_utf8(bytes).ok()
}

pub fn test_read_player_loop_buffer(handle: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
    match with_runtime_state_err(handle, |state| {
        if data.len() > state.player_loop_buffer.len() {
            return Err(RuntimeError::InvalidAddress);
        }
        data.copy_from_slice(&state.player_loop_buffer[..data.len()]);
        Ok(())
    }) {
        Ok(res) => res,
        Err(err) => Err(err),
    }
}

ffi_export! {
pub extern "C" fn unitas_runtime_player_loop_tree(handle: u64) -> u64 {
    with_runtime_state(handle, |state| {
        state.player_loop_buffer = player_loop::serialize(state.runtime.player_loop_tree());
        u64::try_from(state.player_loop_buffer.len()).unwrap_or(0)
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_player_loop_default_tree(handle: u64) -> u64 {
    with_runtime_state(handle, |state| {
        state.player_loop_buffer = player_loop::serialize(&player_loop::default_tree());
        u64::try_from(state.player_loop_buffer.len()).unwrap_or(0)
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_player_loop_buffer(handle: u64, capacity: u64) -> u64 {
    if capacity == 0 {
        return 0;
    }
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    with_runtime_state(handle, |state| {
        if state.player_loop_buffer.len() < cap {
            state.player_loop_buffer.resize(cap, 0);
        }
        state.player_loop_buffer.as_ptr() as usize as u64
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_player_loop_set_tree(handle: u64, address: u64, length: u64) -> i32 {
    let len = match usize::try_from(length) {
        Ok(value) => value,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let Some(bytes) = read_runtime_bytes(state, address, len) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let Some(tree) = player_loop::deserialize(&bytes) else {
            state.set_error("malformed player loop tree");
            return -3;
        };
        match state.runtime.set_player_loop_tree(tree) {
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
pub extern "C" fn unitas_runtime_player_loop_register_custom_system(
    handle: u64,
    name_address: u64,
    name_length: u64,
    type_address: u64,
    type_length: u64,
    anchor_address: u64,
    anchor_length: u64,
    position: u32,
    callback: Option<extern "C" fn(u64, f64)>,
    user_data: u64,
) -> i32 {
    let Some(callback) = callback else {
        return match with_runtime_state(handle, |state| {
            state.set_error("custom system needs a callback");
            -2
        }) {
            Ok(code) => code,
            Err(code) => code,
        };
    };
    let position = match position {
        0 => CustomSystemPosition::Before,
        1 => CustomSystemPosition::After,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown custom system position");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    let lengths = match (
        usize::try_from(name_length),
        usize::try_from(type_length),
        usize::try_from(anchor_length),
    ) {
        (Ok(name), Ok(type_name), Ok(anchor)) => (name, type_name, anchor),
        _ => return -2,
    };
    match with_runtime_state(handle, |state| {
        let Some(name) = read_runtime_string(state, name_address, lengths.0) else {
            state.set_error("custom system name is not valid utf-8");
            return -3;
        };
        let Some(type_name) = read_runtime_string(state, type_address, lengths.1) else {
            state.set_error("custom system type is not valid utf-8");
            return -3;
        };
        let Some(anchor_type) = read_runtime_string(state, anchor_address, lengths.2) else {
            state.set_error("custom system anchor is not valid utf-8");
            return -3;
        };
        match state.runtime.register_player_loop_custom_system(
            &name,
            &type_name,
            &anchor_type,
            position,
            callback,
            user_data,
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
pub extern "C" fn unitas_runtime_player_loop_unregister_custom_system(
    handle: u64,
    type_address: u64,
    type_length: u64,
) -> i32 {
    let len = match usize::try_from(type_length) {
        Ok(value) => value,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let Some(type_name) = read_runtime_string(state, type_address, len) else {
            state.set_error("custom system type is not valid utf-8");
            return -3;
        };
        if state.runtime.unregister_player_loop_custom_system(&type_name) {
            state.clear_error();
            0
        } else {
            state.set_error("custom system not found");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
