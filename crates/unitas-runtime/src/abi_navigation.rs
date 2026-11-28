use unitas_asset::scene::NavMeshObstacleShape;
use unitas_physics::navigation::{NavMeshQueryFilter, NavMeshTriangle, PathQueryStatus};

use super::{EntityId, RuntimeState, path_buffer_offset, with_runtime_state};
use crate::navigation::RuntimeObstacleEntry;

fn read_f64_le(bytes: &[u8], offset: usize) -> f64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[offset..offset + 8]);
    f64::from_le_bytes(raw)
}

fn write_f64_slice(state: &mut RuntimeState, offset: usize, values: &[f64]) {
    for (idx, val) in values.iter().enumerate() {
        let start = offset + idx * 8;
        state.path_buffer[start..start + 8].copy_from_slice(&val.to_le_bytes());
    }
}

fn path_status_code(status: Option<PathQueryStatus>) -> i32 {
    match status {
        Some(PathQueryStatus::Complete) => 0,
        Some(PathQueryStatus::Partial) => 1,
        Some(PathQueryStatus::Invalid) | None => 2,
    }
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_ensure(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.ensure_navmesh_agent(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_has(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let exists = state.runtime.navigation.has_agent(EntityId::new(entity));
        state.clear_error();
        i32::from(exists)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_destination(
    handle: u64,
    entity: u64,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if entity == 0 || !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        match state.runtime.navigation_mut().set_destination(id, [x, y, z]) {
            Ok(result) => {
                state.clear_error();
                path_status_code(Some(result.status))
            }
            Err(err) => {
                state.set_error(&err.to_string());
                2
            }
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_destination(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 24) else {
            return -2;
        };
        let id = EntityId::new(entity);
        let Some(agent) = state.runtime.navigation.agent(id) else {
            return -3;
        };
        let dest = agent.destination().unwrap_or_else(|| agent.position());
        write_f64_slice(state, offset, &dest);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_warp(
    handle: u64,
    entity: u64,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if entity == 0 || !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        match state.runtime.warp_navmesh_agent(id, [x, y, z]) {
            Ok(()) => {
                state.clear_error();
                i32::from(state.runtime.navigation.is_on_navmesh(id))
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
pub extern "C" fn unitas_runtime_navmesh_agent_move(
    handle: u64,
    entity: u64,
    dx: f64,
    dy: f64,
    dz: f64,
) -> i32 {
    if entity == 0 || !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if state.runtime.move_navmesh_agent(EntityId::new(entity), [dx, dy, dz]) {
            state.clear_error();
            0
        } else {
            -4
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_reset_path(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(agent) = state.runtime.navigation_mut().agent_mut(id) {
            agent.reset_path();
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_stopped(
    handle: u64,
    entity: u64,
    stopped: u8,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(agent) = state.runtime.navigation_mut().agent_mut(id) {
            agent.is_stopped = stopped != 0;
            if agent.is_stopped {
                let _ = agent.velocity();
            }
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_stopped(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        let is_stopped = agent.is_stopped;
        state.clear_error();
        i32::from(is_stopped)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_speed(
    handle: u64,
    entity: u64,
    speed: f64,
) -> i32 {
    if entity == 0 || !speed.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(agent) = state.runtime.navigation_mut().agent_mut(id) {
            agent.speed = speed.max(0.0);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_speed(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[agent.speed]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_acceleration(
    handle: u64,
    entity: u64,
    acceleration: f64,
) -> i32 {
    if entity == 0 || !acceleration.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(props) = state.runtime.navigation_mut().agent_props_mut(id) {
            props.acceleration = acceleration.max(0.0);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_acceleration(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(props) = state.runtime.navigation.agent_props(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[props.acceleration]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_angular_speed(
    handle: u64,
    entity: u64,
    angular_speed: f64,
) -> i32 {
    if entity == 0 || !angular_speed.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(props) = state.runtime.navigation_mut().agent_props_mut(id) {
            props.angular_speed = angular_speed.max(0.0);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_angular_speed(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(props) = state.runtime.navigation.agent_props(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[props.angular_speed]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_stopping_distance(
    handle: u64,
    entity: u64,
    dist: f64,
) -> i32 {
    if entity == 0 || !dist.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(props) = state.runtime.navigation_mut().agent_props_mut(id) {
            props.stopping_distance = dist.max(0.0);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_stopping_distance(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(props) = state.runtime.navigation.agent_props(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[props.stopping_distance]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_radius(
    handle: u64,
    entity: u64,
    radius: f64,
) -> i32 {
    if entity == 0 || !radius.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(agent) = state.runtime.navigation_mut().agent_mut(id) {
            agent.radius = radius.max(0.001);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_radius(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[agent.radius]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_set_height(
    handle: u64,
    entity: u64,
    height: f64,
) -> i32 {
    if entity == 0 || !height.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.ensure_navmesh_agent(id);
        if let Some(props) = state.runtime.navigation_mut().agent_props_mut(id) {
            props.height = height.max(0.001);
            state.clear_error();
            0
        } else {
            -3
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_height(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(props) = state.runtime.navigation.agent_props(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[props.height]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_remaining_distance(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 8) else {
            return -2;
        };
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &[agent.remaining_distance()]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_path_status(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        let path_status = agent.path_status();
        state.clear_error();
        match path_status {
            Some(PathQueryStatus::Complete) | None => 0,
            Some(PathQueryStatus::Partial) => 1,
            Some(PathQueryStatus::Invalid) => 2,
        }
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_is_on_navmesh(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.navigation.has_agent(id) {
            return -3;
        }
        state.clear_error();
        i32::from(state.runtime.navigation.is_on_navmesh(id))
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_has_path(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        let has_path = agent.has_path();
        state.clear_error();
        i32::from(has_path)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_position(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 24) else {
            return -2;
        };
        let Some(pos) = state.runtime.navigation.position(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &pos);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_agent_get_velocity(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    if entity == 0 || out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 24) else {
            return -2;
        };
        let Some(agent) = state.runtime.navigation.agent(EntityId::new(entity)) else {
            return -3;
        };
        write_f64_slice(state, offset, &agent.velocity());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_sample_position(
    handle: u64,
    x: f64,
    y: f64,
    z: f64,
    max_distance: f64,
    area_mask: i32,
    out_buffer: u64,
) -> i32 {
    if out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 64) else {
            return -2;
        };
        let mask = area_mask as u32;
        let Some(hit) = state
            .runtime
            .navigation
            .sample_position([x, y, z], max_distance, mask)
        else {
            state.clear_error();
            return 0;
        };
        write_f64_slice(
            state,
            offset,
            &[
                hit.position[0],
                hit.position[1],
                hit.position[2],
                hit.normal[0],
                hit.normal[1],
                hit.normal[2],
                hit.distance,
                f64::from(hit.mask),
            ],
        );
        state.clear_error();
        1
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_raycast(
    handle: u64,
    sx: f64,
    sy: f64,
    sz: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    area_mask: i32,
    out_buffer: u64,
) -> i32 {
    if out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, out_buffer as usize, 64) else {
            return -2;
        };
        let mask = area_mask as u32;
        let hit = state
            .runtime
            .navigation
            .raycast([sx, sy, sz], [tx, ty, tz], mask);
        write_f64_slice(
            state,
            offset,
            &[
                hit.position[0],
                hit.position[1],
                hit.position[2],
                hit.normal[0],
                hit.normal[1],
                hit.normal[2],
                hit.distance,
                f64::from(hit.mask),
            ],
        );
        state.clear_error();
        i32::from(hit.hit)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_calculate_path(
    handle: u64,
    sx: f64,
    sy: f64,
    sz: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    area_mask: i32,
    out_buffer: u64,
) -> i32 {
    if out_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let filter = NavMeshQueryFilter::new(area_mask as u32);
        let result = state
            .runtime
            .navigation
            .calculate_path([sx, sy, sz], [tx, ty, tz], &filter);
        let count = result.corners.len();
        let needed = 8 + count * 24;
        if state.path_buffer.len() < needed {
            state.path_buffer.resize(needed, 0);
        }
        let status = path_status_code(Some(result.status));
        let count_i32 = i32::try_from(count).unwrap_or(0);
        state.path_buffer[0..4].copy_from_slice(&status.to_le_bytes());
        state.path_buffer[4..8].copy_from_slice(&count_i32.to_le_bytes());
        for (idx, corner) in result.corners.iter().enumerate() {
            write_f64_slice(state, 8 + idx * 24, corner);
        }
        state.clear_error();
        status
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_obstacle_set_carving(
    handle: u64,
    entity: u64,
    carving: u8,
    cx: f64,
    cy: f64,
    cz: f64,
    sx: f64,
    sy: f64,
    sz: f64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        let world_tf = state.runtime.entity_transform(id).unwrap_or_default();
        let world_center = [
            f64::from(world_tf.translation[0]) + cx,
            f64::from(world_tf.translation[1]) + cy,
            f64::from(world_tf.translation[2]) + cz,
        ];
        let existing = state
            .runtime
            .navigation
            .entity_obstacle(id)
            .copied()
            .unwrap_or_default();
        let entry = RuntimeObstacleEntry {
            enabled: true,
            shape: existing.shape,
            center: world_center,
            size: [sx.max(0.02), sy.max(0.02), sz.max(0.02)],
            radius: (sx * 0.5).max(0.01),
            height: sy.max(0.02),
            carving: carving != 0,
            carve_only_stationary: existing.carve_only_stationary,
            is_stationary: true,
            navmesh_index: existing.navmesh_index,
        };
        match state.runtime.navigation_mut().upsert_entity_obstacle(id, entry) {
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
pub extern "C" fn unitas_runtime_navmesh_obstacle_get_carving(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(entry) = state.runtime.navigation.entity_obstacle(EntityId::new(entity)) else {
            return -3;
        };
        let carving = entry.carving;
        state.clear_error();
        i32::from(carving)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_navmesh_obstacle_configure(
    handle: u64,
    entity: u64,
    enabled: u8,
    carving: u8,
    carve_only_stationary: u8,
    shape: i32,
    in_buffer: u64,
) -> i32 {
    if entity == 0 || in_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let Some(offset) = path_buffer_offset(state, in_buffer as usize, 64) else {
            return -2;
        };
        let cx = read_f64_le(&state.path_buffer, offset);
        let cy = read_f64_le(&state.path_buffer, offset + 8);
        let cz = read_f64_le(&state.path_buffer, offset + 16);
        let sx = read_f64_le(&state.path_buffer, offset + 24);
        let sy = read_f64_le(&state.path_buffer, offset + 32);
        let sz = read_f64_le(&state.path_buffer, offset + 40);
        let radius = read_f64_le(&state.path_buffer, offset + 48);
        let height = read_f64_le(&state.path_buffer, offset + 56);
        let id = EntityId::new(entity);
        let world_tf = state.runtime.entity_transform(id).unwrap_or_default();
        let world_center = [
            f64::from(world_tf.translation[0]) + cx,
            f64::from(world_tf.translation[1]) + cy,
            f64::from(world_tf.translation[2]) + cz,
        ];
        let existing_idx = state
            .runtime
            .navigation
            .entity_obstacle(id)
            .and_then(|e| e.navmesh_index);
        let entry = RuntimeObstacleEntry {
            enabled: enabled != 0,
            shape: NavMeshObstacleShape::from_i32(shape),
            center: world_center,
            size: [sx.max(0.02), sy.max(0.02), sz.max(0.02)],
            radius: radius.max(0.01),
            height: height.max(0.02),
            carving: carving != 0,
            carve_only_stationary: carve_only_stationary != 0,
            is_stationary: true,
            navmesh_index: existing_idx,
        };
        match state.runtime.navigation_mut().upsert_entity_obstacle(id, entry) {
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
pub extern "C" fn unitas_runtime_navmesh_set_surface_triangles(
    handle: u64,
    in_buffer: u64,
    triangle_count: u32,
) -> i32 {
    if in_buffer == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let count = triangle_count as usize;
        let bytes_needed = count * 80;
        let Some(offset) = path_buffer_offset(state, in_buffer as usize, bytes_needed) else {
            return -2;
        };
        let mut triangles = Vec::with_capacity(count);
        for i in 0..count {
            let base = offset + i * 80;
            let v0 = [
                read_f64_le(&state.path_buffer, base),
                read_f64_le(&state.path_buffer, base + 8),
                read_f64_le(&state.path_buffer, base + 16),
            ];
            let v1 = [
                read_f64_le(&state.path_buffer, base + 24),
                read_f64_le(&state.path_buffer, base + 32),
                read_f64_le(&state.path_buffer, base + 40),
            ];
            let v2 = [
                read_f64_le(&state.path_buffer, base + 48),
                read_f64_le(&state.path_buffer, base + 56),
                read_f64_le(&state.path_buffer, base + 64),
            ];
            let area = read_f64_le(&state.path_buffer, base + 72) as u32;
            triangles.push(NavMeshTriangle::new([v0, v1, v2], area));
        }
        match state.runtime.navigation_mut().set_surface_triangles(triangles) {
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
