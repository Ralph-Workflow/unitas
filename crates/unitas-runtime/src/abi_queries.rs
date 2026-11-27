use super::{QueryRecord, store_query_records, with_runtime_state, write_query_records};

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
pub extern "C" fn unitas_runtime_capsule_cast(
    handle: u64,
    p1_x: f64,
    p1_y: f64,
    p1_z: f64,
    p2_x: f64,
    p2_y: f64,
    p2_z: f64,
    radius: f64,
    dir_x: f64,
    dir_y: f64,
    dir_z: f64,
    max_distance: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let hits = state.runtime.physics_mut().world_3d().capsule_cast(
            unitas_physics::Capsule {
                point1: [p1_x, p1_y, p1_z],
                point2: [p2_x, p2_y, p2_z],
                radius,
            },
            [dir_x, dir_y, dir_z],
            max_distance,
            layer_mask,
            include_triggers != 0,
        );
        let records = query_records_from_hits(hits);
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_overlap_box(
    handle: u64,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    half_x: f64,
    half_y: f64,
    half_z: f64,
    rot_x: f64,
    rot_y: f64,
    rot_z: f64,
    rot_w: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    let center = [center_x, center_y, center_z];
    let half = [half_x, half_y, half_z];
    let rot = [rot_x, rot_y, rot_z, rot_w];
    match with_runtime_state(handle, |state| {
        let entities = state
            .runtime
            .physics_mut()
            .world_3d()
            .overlap_box_with_triggers(center, half, rot, layer_mask, include_triggers != 0);
        let records = query_records_from_entities(state, entities, center);
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_overlap_capsule(
    handle: u64,
    p0_x: f64,
    p0_y: f64,
    p0_z: f64,
    p1_x: f64,
    p1_y: f64,
    p1_z: f64,
    radius: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    let p0 = [p0_x, p0_y, p0_z];
    let p1 = [p1_x, p1_y, p1_z];
    match with_runtime_state(handle, |state| {
        let entities = state
            .runtime
            .physics_mut()
            .world_3d()
            .overlap_capsule_with_triggers(p0, p1, radius, layer_mask, include_triggers != 0);
        let records = query_records_from_entities(state, entities, p0);
        store_query_records(state, records)
    }) {
        Ok(count) => count,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_check_box(
    handle: u64,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    half_x: f64,
    half_y: f64,
    half_z: f64,
    rot_x: f64,
    rot_y: f64,
    rot_z: f64,
    rot_w: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        i32::from(state.runtime.physics_mut().world_3d().check_box(
            [center_x, center_y, center_z],
            [half_x, half_y, half_z],
            [rot_x, rot_y, rot_z, rot_w],
            layer_mask,
            include_triggers != 0,
        ))
    }) {
        Ok(flag) => flag,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_check_sphere(
    handle: u64,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    radius: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        i32::from(state.runtime.physics_mut().world_3d().check_sphere(
            [center_x, center_y, center_z],
            radius,
            layer_mask,
            include_triggers != 0,
        ))
    }) {
        Ok(flag) => flag,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_check_capsule(
    handle: u64,
    p0_x: f64,
    p0_y: f64,
    p0_z: f64,
    p1_x: f64,
    p1_y: f64,
    p1_z: f64,
    radius: f64,
    layer_mask: u32,
    include_triggers: i32,
) -> i32 {
    match with_runtime_state(handle, |state| {
        i32::from(state.runtime.physics_mut().world_3d().check_capsule(
            [p0_x, p0_y, p0_z],
            [p1_x, p1_y, p1_z],
            radius,
            layer_mask,
            include_triggers != 0,
        ))
    }) {
        Ok(flag) => flag,
        Err(code) => code,
    }
}
}

fn query_records_from_hits(hits: Vec<unitas_physics::QueryHit>) -> Vec<QueryRecord> {
    let mut records: Vec<QueryRecord> = hits
        .into_iter()
        .map(|hit| QueryRecord {
            entity: hit.entity.as_u64(),
            distance: hit.distance,
            point: hit.point,
            normal: hit.normal,
        })
        .collect();
    records.sort_by(|first, second| {
        first
            .distance
            .total_cmp(&second.distance)
            .then(first.entity.cmp(&second.entity))
    });
    records
}

fn query_records_from_entities(
    state: &mut super::RuntimeState,
    entities: Vec<unitas_core::EntityId>,
    fallback_point: [f64; 3],
) -> Vec<QueryRecord> {
    let world = state.runtime.physics_mut().world_3d();
    let mut records: Vec<QueryRecord> = entities
        .into_iter()
        .map(|entity| QueryRecord {
            entity: entity.as_u64(),
            distance: 0.0,
            point: world.position(entity).unwrap_or(fallback_point),
            normal: [0.0, 0.0, 0.0],
        })
        .collect();
    records.sort_by(|first, second| first.entity.cmp(&second.entity));
    records
}
