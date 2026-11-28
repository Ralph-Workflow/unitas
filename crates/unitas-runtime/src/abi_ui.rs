use unitas_asset::scene::{
    CanvasMatchMode, CanvasScaleMode, ContentSizeFitMode, LayoutGroupAxis, UiPadding,
};
use unitas_core::EntityId;

use crate::canvas::{
    CanvasScalerOverride, HorizontalWrapMode, PhysicalUnit, SliderDirection, TextAnchor,
    TextLayoutConfig, UiCanvasGroupComponent, UiInputFieldComponent, UiSliderComponent,
    UiToggleComponent, VerticalWrapMode,
};

use super::{read_path_buffer, with_runtime_state, write_string_to_path_buffer};

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

ffi_export! {
pub extern "C" fn unitas_runtime_ui_layout_group_set(
    handle: u64,
    object_id: u64,
    axis: u32,
    padding_packed: u64,
    spacing: f32,
    child_alignment: u32,
    flags: u32,
) -> i32 {
    let axis = match axis {
        0 => LayoutGroupAxis::Horizontal,
        1 => LayoutGroupAxis::Vertical,
        _ => return -2,
    };
    if !spacing.is_finite() || child_alignment > 8 {
        return -2;
    }
    let padding = UiPadding {
        left: (padding_packed & 0xFFFF) as i16 as i32,
        right: ((padding_packed >> 16) & 0xFFFF) as i16 as i32,
        top: ((padding_packed >> 32) & 0xFFFF) as i16 as i32,
        bottom: ((padding_packed >> 48) & 0xFFFF) as i16 as i32,
    };
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_layout_group(object_id, axis, padding, spacing, child_alignment, flags)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for layout group");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_content_size_fitter_set(
    handle: u64,
    object_id: u64,
    horizontal_fit: u32,
    vertical_fit: u32,
    enabled: u32,
) -> i32 {
    let Some(horizontal_fit) = ContentSizeFitMode::from_u32(horizontal_fit) else {
        return -2;
    };
    let Some(vertical_fit) = ContentSizeFitMode::from_u32(vertical_fit) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_content_size_fitter(
            object_id,
            horizontal_fit,
            vertical_fit,
            enabled != 0,
        ) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for content size fitter");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_input_char_event(handle: u64, codepoint: u32) -> i32 {
    let Some(ch) = char::from_u32(codepoint) else {
        return -2;
    };
    if ch.is_control() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.input_char_event(ch);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_create(
    handle: u64,
    object_id: u64,
    interactable: u32,
    character_limit: u32,
    font_size: f32,
) -> i32 {
    if object_id == 0 || !font_size.is_finite() || font_size <= 0.0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let mut field = state
            .runtime
            .ui_input_field(object_id)
            .cloned()
            .unwrap_or_else(|| UiInputFieldComponent {
                entity: EntityId::new(object_id),
                ..UiInputFieldComponent::default()
            });
        field.interactable = interactable != 0;
        field.character_limit = character_limit as usize;
        field.font_size = font_size;
        if state.runtime.set_ui_input_field(field) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for input field");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_set_text(
    handle: u64,
    object_id: u64,
    ptr: u64,
    len: u64,
    notify: u32,
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
        if state
            .runtime
            .set_ui_input_field_text(object_id, &text, notify != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_get_text(
    handle: u64,
    object_id: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(field) = state.runtime.ui_input_field(object_id) else {
            state.set_error("no input field component for that object");
            return -4;
        };
        let text = field.text.clone();
        write_string_to_path_buffer(state, &text, "input field text exceeds supported length")
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_set_placeholder(
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
        if state
            .runtime
            .set_ui_input_field_placeholder(object_id, &text)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_set_interactable(
    handle: u64,
    object_id: u64,
    interactable: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_input_field_interactable(object_id, interactable != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_set_character_limit(
    handle: u64,
    object_id: u64,
    character_limit: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_input_field_character_limit(object_id, character_limit as usize)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_get_caret_position(
    handle: u64,
    object_id: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(caret_position) = state
            .runtime
            .ui_input_field(object_id)
            .map(|field| field.caret_position)
        else {
            state.set_error("no input field component for that object");
            return -4;
        };
        state.clear_error();
        i32::try_from(caret_position).unwrap_or(0)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_set_caret_position(
    handle: u64,
    object_id: u64,
    caret_position: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_input_field_caret_position(object_id, caret_position as usize)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_focus(
    handle: u64,
    object_id: u64,
    focused: u32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .focus_ui_input_field(object_id, focused != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no input field component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_is_focused(
    handle: u64,
    object_id: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        state.clear_error();
        i32::from(state.runtime.is_ui_input_field_focused(object_id))
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_was_canceled(
    handle: u64,
    object_id: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(was_canceled) = state
            .runtime
            .ui_input_field(object_id)
            .map(|field| field.was_canceled)
        else {
            state.set_error("no input field component for that object");
            return -4;
        };
        state.clear_error();
        i32::from(was_canceled)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_take_value_changed(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let events = state.runtime.take_ui_input_field_value_changed();
        if events.is_empty() {
            state.clear_error();
            return 0;
        }
        let mut total_bytes = 0usize;
        for ev in &events {
            total_bytes = total_bytes
                .saturating_add(12)
                .saturating_add(ev.text.len());
        }
        if state.path_buffer.len() < total_bytes {
            state.path_buffer.resize(total_bytes, 0);
        }
        let mut offset = 0usize;
        for ev in &events {
            let text_bytes = ev.text.as_bytes();
            let text_len = u32::try_from(text_bytes.len()).unwrap_or(0);
            state.path_buffer[offset..offset + 8]
                .copy_from_slice(&ev.entity.as_u64().to_le_bytes());
            offset += 8;
            state.path_buffer[offset..offset + 4].copy_from_slice(&text_len.to_le_bytes());
            offset += 4;
            let len = text_len as usize;
            state.path_buffer[offset..offset + len].copy_from_slice(&text_bytes[..len]);
            offset += len;
        }
        state.clear_error();
        i32::try_from(events.len()).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_input_field_take_end_edit(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let events = state.runtime.take_ui_input_field_end_edit();
        if events.is_empty() {
            state.clear_error();
            return 0;
        }
        let mut total_bytes = 0usize;
        for ev in &events {
            total_bytes = total_bytes
                .saturating_add(16)
                .saturating_add(ev.text.len());
        }
        if state.path_buffer.len() < total_bytes {
            state.path_buffer.resize(total_bytes, 0);
        }
        let mut offset = 0usize;
        for ev in &events {
            let text_bytes = ev.text.as_bytes();
            let text_len = u32::try_from(text_bytes.len()).unwrap_or(0);
            let canceled = u32::from(ev.canceled);
            state.path_buffer[offset..offset + 8]
                .copy_from_slice(&ev.entity.as_u64().to_le_bytes());
            offset += 8;
            state.path_buffer[offset..offset + 4].copy_from_slice(&canceled.to_le_bytes());
            offset += 4;
            state.path_buffer[offset..offset + 4].copy_from_slice(&text_len.to_le_bytes());
            offset += 4;
            let len = text_len as usize;
            state.path_buffer[offset..offset + len].copy_from_slice(&text_bytes[..len]);
            offset += len;
        }
        state.clear_error();
        i32::try_from(events.len()).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_toggle_create(
    handle: u64,
    object_id: u64,
    is_on: u32,
    interactable: u32,
    graphic_id: u64,
    group_id: u64,
) -> i32 {
    if object_id == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let mut toggle = state
            .runtime
            .ui_toggle(object_id)
            .cloned()
            .unwrap_or_else(|| UiToggleComponent {
                entity: EntityId::new(object_id),
                ..UiToggleComponent::default()
            });
        toggle.is_on = is_on != 0;
        toggle.interactable = interactable != 0;
        toggle.graphic = (graphic_id != 0).then_some(EntityId::new(graphic_id));
        toggle.group = (group_id != 0).then_some(EntityId::new(group_id));
        if state.runtime.set_ui_toggle(toggle) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for toggle");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_toggle_set_value(
    handle: u64,
    object_id: u64,
    is_on: u32,
    notify: u32,
) -> i32 {
    if object_id == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_toggle_value(object_id, is_on != 0, notify != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no toggle component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_toggle_get(handle: u64, object_id: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let is_on = state.runtime.ui_toggle(object_id).map(|toggle| toggle.is_on);
        let Some(is_on) = is_on else {
            state.set_error("no toggle component for that object");
            return -4;
        };
        state.clear_error();
        i32::from(is_on)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_toggle_take_value_changed(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let events = state.runtime.take_ui_toggle_value_changed();
        if events.is_empty() {
            state.clear_error();
            return 0;
        }
        let total_bytes = events.len().saturating_mul(9);
        if state.path_buffer.len() < total_bytes {
            state.path_buffer.resize(total_bytes, 0);
        }
        let mut offset = 0usize;
        for ev in &events {
            state.path_buffer[offset..offset + 8]
                .copy_from_slice(&ev.entity.as_u64().to_le_bytes());
            offset += 8;
            state.path_buffer[offset] = u8::from(ev.is_on);
            offset += 1;
        }
        state.clear_error();
        i32::try_from(events.len()).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_slider_create(
    handle: u64,
    object_id: u64,
    min_value: f32,
    max_value: f32,
    value: f32,
    whole_numbers: u32,
    direction: u32,
    interactable: u32,
    fill_id: u64,
    handle_id: u64,
) -> i32 {
    if object_id == 0 || !min_value.is_finite() || !max_value.is_finite() || !value.is_finite() {
        return -2;
    }
    let Some(direction) = SliderDirection::from_u32(direction) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        let slider = UiSliderComponent {
            entity: EntityId::new(object_id),
            value: value.clamp(min_value.min(max_value), min_value.max(max_value)),
            min_value,
            max_value,
            whole_numbers: whole_numbers != 0,
            interactable: interactable != 0,
            enabled: true,
            direction,
            fill_rect: (fill_id != 0).then_some(EntityId::new(fill_id)),
            handle_rect: (handle_id != 0).then_some(EntityId::new(handle_id)),
        };
        if state.runtime.set_ui_slider(slider) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for slider");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_slider_set_value(
    handle: u64,
    object_id: u64,
    value: f32,
    notify: u32,
) -> i32 {
    if object_id == 0 || !value.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state
            .runtime
            .set_ui_slider_value(object_id, value, notify != 0)
        {
            state.clear_error();
            0
        } else {
            state.set_error("no slider component for that object");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_slider_get_value(handle: u64, object_id: u64) -> f32 {
    with_runtime_state(handle, |state| {
        let value = state.runtime.ui_slider(object_id).map(|slider| slider.value);
        let Some(value) = value else {
            state.set_error("no slider component for that object");
            return f32::NAN;
        };
        state.clear_error();
        value
    })
    .unwrap_or(f32::NAN)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_slider_take_value_changed(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let events = state.runtime.take_ui_slider_value_changed();
        if events.is_empty() {
            state.clear_error();
            return 0;
        }
        let total_bytes = events.len().saturating_mul(12);
        if state.path_buffer.len() < total_bytes {
            state.path_buffer.resize(total_bytes, 0);
        }
        let mut offset = 0usize;
        for ev in &events {
            state.path_buffer[offset..offset + 8]
                .copy_from_slice(&ev.entity.as_u64().to_le_bytes());
            offset += 8;
            state.path_buffer[offset..offset + 4].copy_from_slice(&ev.value.to_le_bytes());
            offset += 4;
        }
        state.clear_error();
        i32::try_from(events.len()).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_canvas_scaler_set(
    handle: u64,
    object_id: u64,
    scale_mode: u32,
    scale_factor: f32,
    reference_width: f32,
    reference_height: f32,
    match_mode: u32,
    match_value: f32,
    physical_unit: u32,
    fallback_dpi: f32,
) -> i32 {
    if object_id == 0 {
        return -2;
    }
    let ui_scale_mode = match scale_mode {
        0 => CanvasScaleMode::ConstantPixelSize,
        1 => CanvasScaleMode::ScaleWithScreenSize,
        2 => CanvasScaleMode::ConstantPhysicalSize,
        _ => return -2,
    };
    let screen_match_mode = match match_mode {
        0 => CanvasMatchMode::MatchWidthOrHeight,
        1 => CanvasMatchMode::Expand,
        2 => CanvasMatchMode::Shrink,
        _ => return -2,
    };
    let Some(physical_unit) = PhysicalUnit::from_u32(physical_unit) else {
        return -2;
    };
    if !scale_factor.is_finite()
        || !reference_width.is_finite()
        || !reference_height.is_finite()
        || !match_value.is_finite()
        || !fallback_dpi.is_finite()
        || fallback_dpi <= 0.0
    {
        return -2;
    }
    let scaler = CanvasScalerOverride {
        entity: EntityId::new(object_id),
        ui_scale_mode,
        scale_factor,
        reference_resolution: [reference_width, reference_height],
        screen_match_mode,
        match_width_or_height: match_value,
        physical_unit,
        fallback_dpi,
    };
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_canvas_scaler(scaler) {
            state.clear_error();
            0
        } else {
            state.set_error("no canvas for the scaler");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_canvas_group_set(
    handle: u64,
    object_id: u64,
    alpha: f32,
    interactable: u32,
    blocks_raycasts: u32,
    ignore_parent_groups: u32,
    enabled: u32,
) -> i32 {
    if object_id == 0 || !alpha.is_finite() {
        return -2;
    }
    let group = UiCanvasGroupComponent {
        entity: EntityId::new(object_id),
        alpha,
        interactable: interactable != 0,
        blocks_raycasts: blocks_raycasts != 0,
        ignore_parent_groups: ignore_parent_groups != 0,
        enabled: enabled != 0,
    };
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_canvas_group(group) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for canvas group");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_rect_mask_set(
    handle: u64,
    object_id: u64,
    enabled: u32,
) -> i32 {
    if object_id == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state.runtime.set_ui_rect_mask(object_id, enabled != 0) {
            state.clear_error();
            0
        } else {
            state.set_error("no ui target for rect mask");
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_ui_screen_dpi_set(handle: u64, dpi: f32) -> i32 {
    if !dpi.is_finite() || dpi < 0.0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.set_ui_screen_dpi(dpi);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
