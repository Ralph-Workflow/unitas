use std::collections::BTreeMap;
use std::sync::Mutex;

use unitas_physics::{BodyInterpolation, ForceMode};

use super::{
    AbiCharacterMoveInput, AbiCharacterMoveOutput, CharacterMoveRequest, CollisionContactPoint,
    CollisionDetectionMode3D, CollisionSummary, EntityId, RuntimeState,
    read_force_from_path_buffer, read_vec3_from_path_buffer, read_vec4_from_path_buffer, to_f32,
    with_active_runtime, with_collision_payload, with_registered_runtime, with_runtime_state,
    write_output,
};

static BODY_CONSTRAINTS_3D: Mutex<BTreeMap<(u64, u64), u32>> = Mutex::new(BTreeMap::new());
static BODY_INTERPOLATION_3D: Mutex<BTreeMap<(u64, u64), u32>> = Mutex::new(BTreeMap::new());

fn constraints_for(handle: u64, entity: u64) -> u32 {
    BODY_CONSTRAINTS_3D
        .lock()
        .ok()
        .and_then(|map| map.get(&(handle, entity)).copied())
        .unwrap_or(0)
}

fn mask_linear(v: [f64; 3], c: u32) -> [f64; 3] {
    [
        if c & 2 != 0 { 0.0 } else { v[0] },
        if c & 4 != 0 { 0.0 } else { v[1] },
        if c & 8 != 0 { 0.0 } else { v[2] },
    ]
}

fn mask_angular(v: [f64; 3], c: u32) -> [f64; 3] {
    [
        if c & 16 != 0 { 0.0 } else { v[0] },
        if c & 32 != 0 { 0.0 } else { v[1] },
        if c & 64 != 0 { 0.0 } else { v[2] },
    ]
}

fn finish_res<E: std::fmt::Display>(state: &mut RuntimeState, res: Result<(), E>) -> i32 {
    match res {
        Ok(()) => {
            state.clear_error();
            0
        }
        Err(err) => {
            state.set_error(&err.to_string());
            -4
        }
    }
}

fn write_vec3_to_path_buffer(state: &mut RuntimeState, out_buffer: u64, v: [f64; 3]) -> i32 {
    if out_buffer == 0
        || out_buffer != state.path_buffer.as_ptr() as usize as u64
        || state.path_buffer.len() < 24
    {
        state.set_error("buffer address mismatch");
        return -2;
    }
    state.path_buffer[0..8].copy_from_slice(&v[0].to_le_bytes());
    state.path_buffer[8..16].copy_from_slice(&v[1].to_le_bytes());
    state.path_buffer[16..24].copy_from_slice(&v[2].to_le_bytes());
    state.clear_error();
    0
}

fn rotate_quat(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let [qx, qy, qz, qw] = q;
    let tx = 2.0 * (qy * v[2] - qz * v[1]);
    let ty = 2.0 * (qz * v[0] - qx * v[2]);
    let tz = 2.0 * (qx * v[1] - qy * v[0]);
    [
        v[0] + qw * tx + (qy * tz - qz * ty),
        v[1] + qw * ty + (qz * tx - qx * tz),
        v[2] + qw * tz + (qx * ty - qy * tx),
    ]
}

fn read_f64_at(state: &RuntimeState, offset: usize) -> Option<f64> {
    let bytes: [u8; 8] = state.path_buffer.get(offset..offset + 8)?.try_into().ok()?;
    Some(f64::from_le_bytes(bytes))
}

fn decode_force_mode(val: f64) -> Option<ForceMode> {
    match val {
        0.0 => Some(ForceMode::Force),
        1.0 => Some(ForceMode::Impulse),
        2.0 => Some(ForceMode::Acceleration),
        3.0 => Some(ForceMode::VelocityChange),
        _ => None,
    }
}

fn read_7_f64(state: &mut RuntimeState, in_buffer: u64) -> Option<([f64; 6], ForceMode)> {
    if in_buffer == 0
        || in_buffer != state.path_buffer.as_ptr() as usize as u64
        || state.path_buffer.len() < 56
    {
        state.set_error("buffer address mismatch");
        return None;
    }
    let vals = [
        read_f64_at(state, 0)?,
        read_f64_at(state, 8)?,
        read_f64_at(state, 16)?,
        read_f64_at(state, 24)?,
        read_f64_at(state, 32)?,
        read_f64_at(state, 40)?,
    ];
    let mode = decode_force_mode(read_f64_at(state, 48)?)?;
    Some((vals, mode))
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_position(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(pos) = state.runtime.body_position(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        write_vec3_to_path_buffer(state, out_buffer, pos)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_velocity(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(vel) = state.runtime.body_velocity(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        let masked = mask_linear(vel, constraints_for(handle, entity));
        write_vec3_to_path_buffer(state, out_buffer, masked)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_angular_velocity(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(vel) = state.runtime.physics.world_3d().angular_velocity(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        let masked = mask_angular(vel, constraints_for(handle, entity));
        write_vec3_to_path_buffer(state, out_buffer, masked)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_position(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(v) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let res = state.runtime.set_body_position(EntityId::new(entity), v);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_teleport_body_position(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(v) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let res = state.runtime.teleport_body_position(EntityId::new(entity), v);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_velocity(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(v) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let masked = mask_linear(v, constraints_for(handle, entity));
        let res = state.runtime.set_body_velocity(EntityId::new(entity), masked);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_angular_velocity(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(v) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let masked = mask_angular(v, constraints_for(handle, entity));
        let res = state.runtime.physics_mut().world_3d_mut().set_angular_velocity(EntityId::new(entity), masked);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_rotation(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(rot) = read_vec4_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let res = state.runtime.set_body_rotation(EntityId::new(entity), rot);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_teleport_body_rotation(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(rot) = read_vec4_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let res = state
            .runtime
            .physics_mut()
            .world_3d_mut()
            .teleport_rotation(EntityId::new(entity), rot);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_set_collision_detection_mode(handle: u64, entity: u64, mode: u32) -> i32 {
    let mode = match mode {
        0 => CollisionDetectionMode3D::Discrete,
        1 => CollisionDetectionMode3D::Continuous,
        _ => {
            return with_runtime_state(handle, |state| {
                state.set_error("unknown collision detection mode");
                -2
            })
            .unwrap_or_else(|code| code);
        }
    };
    with_runtime_state(handle, |state| {
        let res = state.runtime.physics_mut().set_collision_detection_mode_3d(EntityId::new(entity), mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collision_summary(a: u64, b: u64, out: *mut CollisionSummary) -> i32 {
    let Some(summary) = with_collision_payload(a, b, &|payload| CollisionSummary {
        impulse: payload.total_impulse as f32,
        relative_velocity: to_f32(payload.relative_velocity),
        contact_count: payload.contacts.len() as u32,
        point: payload.contacts.first().map_or([0.0; 3], |c| to_f32(c.point)),
        normal: payload.contacts.first().map_or([0.0; 3], |c| to_f32(c.normal)),
    }) else {
        return -1;
    };
    if !write_output(out, summary) { -2 } else { 0 }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collision_contact(a: u64, b: u64, index: u32, out: *mut CollisionContactPoint) -> i32 {
    let Some(Some(contact)) = with_collision_payload(a, b, &|payload| {
        payload.contacts.get(index as usize).map(|c| CollisionContactPoint {
            point: c.point,
            normal: c.normal,
            impulse: c.impulse as f32,
        })
    }) else {
        return -1;
    };
    if !write_output(out, contact) { -2 } else { 0 }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_move_body_position(entity: u64, x: f32, y: f32, z: f32) -> i32 {
    let entity = EntityId::new(entity);
    let target = [f64::from(x), f64::from(y), f64::from(z)];
    let moved = with_active_runtime(|r| r.move_body_position(entity, target).ok())
        .or_else(|| with_registered_runtime(|r| r.move_body_position(entity, target).ok()));
    if moved.is_some() { 0 } else { -1 }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_force(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some((force, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let masked = mask_linear(force, constraints_for(handle, entity));
        let res = state.runtime.add_body_force(EntityId::new(entity), masked, mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_torque(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some((torque, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let masked = mask_angular(torque, constraints_for(handle, entity));
        let res = state.runtime.physics_mut().world_3d_mut().add_torque(EntityId::new(entity), masked, mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_relative_force(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some((local_force, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let id = EntityId::new(entity);
        let Some(rot) = state.runtime.physics.world_3d().rotation(id) else {
            state.set_error("body not found");
            return -4;
        };
        let world_force = mask_linear(rotate_quat(rot, local_force), constraints_for(handle, entity));
        let res = state.runtime.add_body_force(id, world_force, mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_relative_torque(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some((local_torque, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let id = EntityId::new(entity);
        let Some(rot) = state.runtime.physics.world_3d().rotation(id) else {
            state.set_error("body not found");
            return -4;
        };
        let world_torque = mask_angular(rotate_quat(rot, local_torque), constraints_for(handle, entity));
        let res = state.runtime.physics_mut().world_3d_mut().add_torque(id, world_torque, mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_force_at_position(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(([fx, fy, fz, px, py, pz], m)) = read_7_f64(state, in_buffer) else {
            return -2;
        };
        let id = EntityId::new(entity);
        let Some(pos) = state.runtime.body_position(id) else {
            state.set_error("body not found");
            return -4;
        };
        let c = constraints_for(handle, entity);
        let r = [px - pos[0], py - pos[1], pz - pos[2]];
        let torque = [r[1] * fz - r[2] * fy, r[2] * fx - r[0] * fz, r[0] * fy - r[1] * fx];
        let res = state
            .runtime
            .add_body_force(id, mask_linear([fx, fy, fz], c), m)
            .and_then(|()| {
                state.runtime.physics_mut().world_3d_mut().add_torque(id, mask_angular(torque, c), m).map_err(Into::into)
            });
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_explosion_force(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(([force, ex, ey, ez, radius, upwards], mode)) = read_7_f64(state, in_buffer) else {
            return -2;
        };
        let id = EntityId::new(entity);
        let Some(pos) = state.runtime.body_position(id) else {
            state.set_error("body not found");
            return -4;
        };
        let dir = [pos[0] - ex, pos[1] - (ey - upwards), pos[2] - ez];
        let dist = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if radius > 0.0 && dist > radius {
            state.clear_error();
            return 0;
        }
        let atten = if radius <= 0.0 { 1.0 } else { (1.0 - dist / radius).clamp(0.0, 1.0) };
        let unit = if dist > 1e-9 { [dir[0] / dist, dir[1] / dist, dir[2] / dist] } else { [0.0, 1.0, 0.0] };
        let mag = force * atten;
        let f = mask_linear([unit[0] * mag, unit[1] * mag, unit[2] * mag], constraints_for(handle, entity));
        let res = state.runtime.add_body_force(id, f, mode);
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_sleep(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let res = state.runtime.physics_mut().world_3d_mut().sleep(EntityId::new(entity));
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_wake_up(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if state.runtime.body_position(id).is_none() {
            state.set_error("body not found");
            return -4;
        }
        state.runtime.physics_mut().world_3d_mut().wake_up(id);
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_is_sleeping(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if state.runtime.body_position(id).is_none() {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        i32::from(state.runtime.physics.world_3d().is_sleeping(id))
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_set_interpolation(handle: u64, entity: u64, mode: u32) -> i32 {
    let interp = match mode {
        0 => BodyInterpolation::None,
        1 => BodyInterpolation::Interpolate,
        2 => BodyInterpolation::Extrapolate,
        _ => return -2,
    };
    with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if state.runtime.body_position(id).is_none() {
            state.set_error("body not found");
            return -4;
        }
        state.runtime.physics_mut().world_3d_mut().set_body_interpolation(id, interp);
        if let Ok(mut map) = BODY_INTERPOLATION_3D.lock() {
            map.insert((handle, entity), mode);
        }
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_get_interpolation(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        if state.runtime.body_position(EntityId::new(entity)).is_none() {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        BODY_INTERPOLATION_3D.lock().ok().and_then(|m| m.get(&(handle, entity)).copied()).unwrap_or(0) as i32
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_set_constraints(handle: u64, entity: u64, constraints: u32) -> i32 {
    with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if state.runtime.body_position(id).is_none() {
            state.set_error("body not found");
            return -4;
        }
        if let Ok(mut map) = BODY_CONSTRAINTS_3D.lock() {
            map.insert((handle, entity), constraints);
        }
        if let Some(v) = state.runtime.body_velocity(id) {
            let _ = state.runtime.set_body_velocity(id, mask_linear(v, constraints));
        }
        if let Some(av) = state.runtime.physics.world_3d().angular_velocity(id) {
            let _ = state.runtime.physics_mut().world_3d_mut().set_angular_velocity(id, mask_angular(av, constraints));
        }
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_get_constraints(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        if state.runtime.body_position(EntityId::new(entity)).is_none() {
            state.set_error("body not found");
            return -4;
        }
        state.clear_error();
        constraints_for(handle, entity) as i32
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_character_move(
    handle: u64,
    input: Option<&AbiCharacterMoveInput>,
    output: Option<&mut AbiCharacterMoveOutput>,
) -> i32 {
    with_runtime_state(handle, |state| {
        let (Some(input), Some(output)) = (input, output) else {
            state.set_error("null character move buffer pointer");
            return -2;
        };
        let request = CharacterMoveRequest {
            entity: (input.entity_id != 0).then(|| EntityId::new(input.entity_id)),
            position: input.position,
            motion: input.motion,
            radius: input.radius,
            height: input.height,
            center: input.center,
            slope_limit_deg: input.slope_limit_deg,
            step_offset: input.step_offset,
            skin_width: input.skin_width,
            min_move_distance: input.min_move_distance,
            layer_mask: input.layer_mask,
            enabled: input.enabled != 0,
            delta_seconds: input.delta_seconds,
        };
        match state.runtime.character_move(request) {
            Ok(result) => {
                output.position = result.position;
                output.ground_entity_id = result.ground_entity.map_or(0, |id| id.as_u64());
                output.collision_flags = result.collision_flags.bits();
                output.is_grounded = u8::from(result.is_grounded);
                output.reserved = [0; 3];
                state.clear_error();
                0
            }
            Err(err) => {
                state.set_error(&err.to_string());
                -4
            }
        }
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_character_teleport(handle: u64, entity: u64, in_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        let Some(pos) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let res = state.runtime.character_teleport((entity != 0).then(|| EntityId::new(entity)), pos).map(|_| ());
        finish_res(state, res)
    })
    .unwrap_or_else(|code| code)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_character_controller_config(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    with_runtime_state(handle, |state| {
        if out_buffer == 0 || out_buffer != state.path_buffer.as_ptr() as usize as u64 || state.path_buffer.len() < 64 {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(cfg) = state.runtime.character_controller_config(EntityId::new(entity)).cloned() else {
            state.set_error("character controller not found");
            return -4;
        };
        for (i, val) in [cfg.radius, cfg.height, cfg.center[0], cfg.center[1], cfg.center[2], cfg.slope_limit, cfg.step_offset, cfg.skin_width].iter().enumerate() {
            state.path_buffer[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
        }
        if state.path_buffer.len() >= 76 {
            state.path_buffer[64..72].copy_from_slice(&cfg.min_move_distance.to_le_bytes());
            state.path_buffer[72] = u8::from(cfg.enabled);
            state.path_buffer[73..76].fill(0);
        }
        state.clear_error();
        0
    })
    .unwrap_or_else(|code| code)
}
}
