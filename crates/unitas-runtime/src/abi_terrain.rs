use super::{EntityId, RuntimeState, read_f32_le, with_runtime_state};

fn terrain_buffer_ready(state: &RuntimeState, buffer: u64, required: usize) -> bool {
    buffer != 0
        && buffer == state.path_buffer.as_ptr() as usize as u64
        && state.path_buffer.len() >= required
}

ffi_export! {
pub extern "C" fn unitas_runtime_terrain_ensure(
    handle: u64,
    entity: u64,
    resolution: u32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if !state
            .runtime
            .terrain_mut()
            .ensure_terrain(EntityId::new(entity), resolution as usize)
        {
            state.set_error("invalid terrain resolution");
            return -2;
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
pub extern "C" fn unitas_runtime_terrain_has(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let exists = state.runtime.terrain().has_terrain(EntityId::new(entity));
        state.clear_error();
        i32::from(exists)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_terrain_get_resolution(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some([res, _]) = state.runtime.terrain().resolution(EntityId::new(entity)) else {
            return -3;
        };
        state.clear_error();
        i32::try_from(res).unwrap_or(i32::MAX)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_terrain_set_resolution(
    handle: u64,
    entity: u64,
    resolution: u32,
) -> i32 {
    if entity == 0 || resolution == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.terrain().has_terrain(id) {
            if !state.runtime.terrain_mut().ensure_terrain(id, resolution as usize) {
                return -2;
            }
        } else if !state
            .runtime
            .terrain_mut()
            .set_resolution(id, resolution as usize)
        {
            return -2;
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
pub extern "C" fn unitas_runtime_terrain_get_size(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if !terrain_buffer_ready(state, out_buffer, 24) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(size) = state.runtime.terrain().size(EntityId::new(entity)) else {
            return -3;
        };
        state.path_buffer[0..8].copy_from_slice(&size[0].to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&size[1].to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&size[2].to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_terrain_set_size(
    handle: u64,
    entity: u64,
    sx: f64,
    sy: f64,
    sz: f64,
) -> i32 {
    if entity == 0 || !sx.is_finite() || !sy.is_finite() || !sz.is_finite() {
        return -2;
    }
    if sx <= 0.0 || sy <= 0.0 || sz <= 0.0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.terrain().has_terrain(id)
            && !state.runtime.terrain_mut().ensure_terrain(id, 0)
        {
            return -2;
        }
        if !state.runtime.terrain_mut().set_size(id, [sx, sy, sz]) {
            return -2;
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
pub extern "C" fn unitas_runtime_terrain_set_position(
    handle: u64,
    entity: u64,
    px: f64,
    py: f64,
    pz: f64,
) -> i32 {
    if entity == 0 || !px.is_finite() || !py.is_finite() || !pz.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.terrain().has_terrain(id)
            && !state.runtime.terrain_mut().ensure_terrain(id, 0)
        {
            return -2;
        }
        if !state.runtime.terrain_mut().set_position(id, [px, py, pz]) {
            return -2;
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
pub extern "C" fn unitas_runtime_terrain_sample_height(
    handle: u64,
    entity: u64,
    x: f64,
    y: f64,
    z: f64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if !terrain_buffer_ready(state, out_buffer, 8) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(sampled) = state
            .runtime
            .terrain()
            .sample_height(EntityId::new(entity), [x, y, z])
        else {
            return -3;
        };
        state.path_buffer[0..8].copy_from_slice(&sampled.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_terrain_get_heights(
    handle: u64,
    entity: u64,
    x_base: u32,
    y_base: u32,
    width: u32,
    height: u32,
    out_buffer: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let count = (width as usize).saturating_mul(height as usize);
    let required_bytes = count.saturating_mul(4);
    match with_runtime_state(handle, |state| {
        if !terrain_buffer_ready(state, out_buffer, required_bytes) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(values) = state.runtime.terrain().get_heights(
            EntityId::new(entity),
            x_base as usize,
            y_base as usize,
            width as usize,
            height as usize,
        ) else {
            return -3;
        };
        for (idx, value) in values.iter().enumerate() {
            let start = idx * 4;
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
pub extern "C" fn unitas_runtime_terrain_set_heights(
    handle: u64,
    entity: u64,
    x_base: u32,
    y_base: u32,
    width: u32,
    height: u32,
    in_buffer: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let count = (width as usize).saturating_mul(height as usize);
    let required_bytes = count.saturating_mul(4);
    match with_runtime_state(handle, |state| {
        if !terrain_buffer_ready(state, in_buffer, required_bytes) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let mut samples = Vec::with_capacity(count);
        for idx in 0..count {
            samples.push(read_f32_le(&state.path_buffer, idx * 4));
        }
        if !state.runtime.terrain().set_heights_block(
            EntityId::new(entity),
            x_base as usize,
            y_base as usize,
            width as usize,
            height as usize,
            &samples,
        ) {
            return -3;
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
