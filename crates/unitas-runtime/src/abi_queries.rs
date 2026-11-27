use super::{store_query_records, with_runtime_state, write_query_records};

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
