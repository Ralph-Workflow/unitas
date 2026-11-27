use super::{INVALID_HANDLE_ERROR_TARGET, Ordering, with_runtime_state};
use crate::input::GAMEPAD_AXIS_COUNT;

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
pub extern "C" fn unitas_runtime_input_injected_axis(handle: u64, address: u64, length: u64) -> f64 {
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
        match state.runtime.input_injected_axis(&name) {
            Some(value) => {
                state.clear_error();
                value
            }
            None => {
                state.set_error(&format!("no injected input axis: {name}"));
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
pub extern "C" fn unitas_runtime_input_gamepad_set_connected(handle: u64, connected: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.input_gamepad_set_connected(connected != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_gamepad_button_event(
    handle: u64,
    button: u32,
    pressed: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.input_gamepad_button_event(button, pressed != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_gamepad_axis_event(
    handle: u64,
    axis: u32,
    value: f64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !value.is_finite() {
            state.set_error("non-finite gamepad axis value");
            return -2;
        }
        if axis as usize >= GAMEPAD_AXIS_COUNT {
            state.set_error(&format!("unknown gamepad axis: {axis}"));
            return -2;
        }
        state.runtime.input_gamepad_axis_event(axis, value);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_gamepad_button_state(handle: u64, button: u32) -> u32 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.input_gamepad_button_state(button)
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
pub extern "C" fn unitas_runtime_input_gamepad_axis(handle: u64, axis: u32) -> f64 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.input_gamepad_axis(axis)
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
pub extern "C" fn unitas_runtime_input_gamepad_connected(handle: u64) -> u32 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.input_gamepad_connected()
    }) {
        Ok(connected) => connected,
        Err(_) => {
            INVALID_HANDLE_ERROR_TARGET.store(handle, Ordering::Relaxed);
            0
        }
    }
}
}
