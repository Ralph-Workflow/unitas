use super::{EntityId, TrailSettings, path_buffer_offset, read_f32_le, with_runtime_state};

pub const TRAIL_SETTINGS_FLOATS: usize = 12;
const TRAIL_SETTINGS_BYTES: usize = TRAIL_SETTINGS_FLOATS * 4;

fn trail_settings_blob(settings: &TrailSettings) -> [f32; TRAIL_SETTINGS_FLOATS] {
    [
        settings.time,
        settings.width_start,
        settings.width_end,
        settings.min_vertex_distance,
        settings.start_color[0],
        settings.start_color[1],
        settings.start_color[2],
        settings.start_color[3],
        settings.end_color[0],
        settings.end_color[1],
        settings.end_color[2],
        settings.end_color[3],
    ]
}

fn trail_settings_from_blob(values: &[f32; TRAIL_SETTINGS_FLOATS]) -> TrailSettings {
    TrailSettings {
        time: values[0],
        width_start: values[1],
        width_end: values[2],
        min_vertex_distance: values[3],
        start_color: [values[4], values[5], values[6], values[7]],
        end_color: [values[8], values[9], values[10], values[11]],
    }
}

ffi_export! {
pub extern "C" fn unitas_runtime_trail_get_settings(handle: u64, entity: u64, blob: *mut f32) -> i32 {
    if entity == 0 || blob.is_null() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, blob as usize, TRAIL_SETTINGS_BYTES) else {
            return -2;
        };
        let Some(trail) = state.runtime.trails().trail(EntityId::new(entity)) else {
            return -3;
        };
        let values = trail_settings_blob(trail.settings());
        for (index, value) in values.iter().enumerate() {
            let start = offset + index * 4;
            state.path_buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
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
pub extern "C" fn unitas_runtime_trail_configure(handle: u64, entity: u64, blob: *const f32) -> i32 {
    if entity == 0 || blob.is_null() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, blob as usize, TRAIL_SETTINGS_BYTES) else {
            return -2;
        };
        let mut values = [0.0f32; TRAIL_SETTINGS_FLOATS];
        for (index, value) in values.iter_mut().enumerate() {
            *value = read_f32_le(&state.path_buffer, offset + index * 4);
        }
        let Some(trail) = state
            .runtime
            .trails_mut()
            .trail_mut(EntityId::new(entity))
        else {
            return -3;
        };
        trail.set_settings(trail_settings_from_blob(&values));
        state.runtime.trails_mut().rebuild_draw_list();
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_trail_set_emitting(handle: u64, entity: u64, emitting: u8) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .trails_mut()
            .set_emitting(EntityId::new(entity), emitting != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_trail_get_emitting(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let entity = EntityId::new(entity);
        if state.runtime.trails().trail(entity).is_none() {
            return -3;
        }
        let emitting = state.runtime.trails().emitting(entity);
        state.clear_error();
        i32::from(emitting)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_trail_set_full_width_speed(handle: u64, entity: u64, speed: f32) -> i32 {
    if entity == 0 || !speed.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .trails_mut()
            .set_full_width_speed(EntityId::new(entity), speed);
        state.runtime.trails_mut().rebuild_draw_list();
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
