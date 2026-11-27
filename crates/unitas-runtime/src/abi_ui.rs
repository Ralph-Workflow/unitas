use crate::canvas::{HorizontalWrapMode, TextAnchor, TextLayoutConfig, VerticalWrapMode};

use super::{read_path_buffer, with_runtime_state};

ffi_export! {
pub extern "C" fn unitas_runtime_ui_text_set(
    handle: u64,
    object_id: u64,
    ptr: u64,
    len: u64,
) -> i32 {
    let length = match usize::try_from(len) {
        Ok(length) => length,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let text = match read_path_buffer(state, ptr, length) {
            Ok(text) => text,
            Err(code) => return code,
        };
        if state.runtime.set_ui_text(object_id, &text) {
            state.clear_error();
            0
        } else {
            state.set_error("no text component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_text_style(
    handle: u64,
    object_id: u64,
    font_size: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_text_style(object_id, font_size, [r, g, b, a]) {
            state.clear_error();
            0
        } else {
            state.set_error("no text component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_text_layout(
    handle: u64,
    object_id: u64,
    alignment: u32,
    horizontal_overflow: u32,
    vertical_overflow: u32,
    line_spacing: f32,
) -> i32 {
    let Some(alignment) = TextAnchor::from_u32(alignment) else {
        return -2;
    };
    let Some(horizontal_overflow) = HorizontalWrapMode::from_u32(horizontal_overflow) else {
        return -2;
    };
    let Some(vertical_overflow) = VerticalWrapMode::from_u32(vertical_overflow) else {
        return -2;
    };
    if !line_spacing.is_finite() || line_spacing <= 0.0 {
        return -2;
    }
    let layout = TextLayoutConfig {
        alignment,
        horizontal_overflow,
        vertical_overflow,
        line_spacing,
    };
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_text_layout(object_id, layout) {
            state.clear_error();
            0
        } else {
            state.set_error("no text component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_pointer_over(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        i32::from(state.runtime.ui_pointer_over())
    }) {
        Ok(over) => over,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_poll_clicks(
    handle: u64,
    out_ptr: u64,
    capacity: u64,
) -> i32 {
    let limit = match usize::try_from(capacity) {
        Ok(limit) => limit,
        Err(_) => return -2,
    };
    match with_runtime_state(handle, |state| {
        let clicks = state.runtime.take_ui_clicks();
        let count = clicks.len().min(limit);
        let byte_len = count.saturating_mul(8);
        if out_ptr != 0 {
            if out_ptr != state.path_buffer.as_ptr() as usize as u64
                || state.path_buffer.len() < byte_len
            {
                state.set_error("buffer address mismatch");
                return -2;
            }
        } else if state.path_buffer.len() < byte_len {
            state.path_buffer.resize(byte_len, 0);
        }
        for (index, click) in clicks.iter().take(count).enumerate() {
            let start = index * 8;
            state.path_buffer[start..start + 8].copy_from_slice(&click.to_le_bytes());
        }
        state.clear_error();
        i32::try_from(count).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
