use super::{
    AbiCharacterMoveInput, AbiCharacterMoveOutput, CharacterMoveRequest, CollisionContactPoint,
    CollisionDetectionMode3D, CollisionSummary, EntityId, read_force_from_path_buffer,
    read_vec3_from_path_buffer, read_vec4_from_path_buffer, to_f32, with_active_runtime,
    with_collision_payload, with_registered_runtime, with_runtime_state, write_output,
};

ffi_export! {
pub extern "C" fn unitas_runtime_body_position(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 24
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some([x, y, z]) = state.runtime.body_position(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&z.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_body_velocity(handle: u64, entity: u64, out_buffer: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 24
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some([x, y, z]) = state.runtime.body_velocity(EntityId::new(entity)) else {
            state.set_error("body not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&x.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&y.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&z.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_set_body_position(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vector) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_position(EntityId::new(entity), vector)
        {
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
pub extern "C" fn unitas_runtime_teleport_body_position(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vector) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .teleport_body_position(EntityId::new(entity), vector)
        {
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
pub extern "C" fn unitas_runtime_set_body_velocity(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(vector) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_velocity(EntityId::new(entity), vector)
        {
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
pub extern "C" fn unitas_runtime_set_body_rotation(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(rotation) = read_vec4_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .set_body_rotation(EntityId::new(entity), rotation)
        {
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
pub extern "C" fn unitas_runtime_body_set_collision_detection_mode(
    handle: u64,
    entity: u64,
    mode: u32,
) -> i32 {
    let mode = match mode {
        0 => CollisionDetectionMode3D::Discrete,
        1 => CollisionDetectionMode3D::Continuous,
        _ => {
            return match with_runtime_state(handle, |state| {
                state.set_error("unknown collision detection mode");
                -2
            }) {
                Ok(code) => code,
                Err(code) => code,
            };
        }
    };
    match with_runtime_state(handle, |state| {
        match state
            .runtime
            .physics_mut()
            .set_collision_detection_mode_3d(EntityId::new(entity), mode)
        {
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
pub extern "C" fn unitas_runtime_collision_summary(
    a: u64,
    b: u64,
    out: *mut CollisionSummary,
) -> i32 {
    let Some(summary) = with_collision_payload(a, b, &|payload| CollisionSummary {
        impulse: payload.total_impulse as f32,
        relative_velocity: to_f32(payload.relative_velocity),
        contact_count: payload.contacts.len() as u32,
        point: payload.contacts.first().map_or([0.0; 3], |contact| to_f32(contact.point)),
        normal: payload
            .contacts
            .first()
            .map_or([0.0; 3], |contact| to_f32(contact.normal)),
    }) else {
        return -1;
    };
    if !write_output(out, summary) {
        return -2;
    }
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_collision_contact(
    a: u64,
    b: u64,
    index: u32,
    out: *mut CollisionContactPoint,
) -> i32 {
    let Some(contact) = with_collision_payload(a, b, &|payload| {
        payload
            .contacts
            .get(index as usize)
            .map(|contact| CollisionContactPoint {
                point: contact.point,
                normal: contact.normal,
                impulse: contact.impulse as f32,
            })
    }) else {
        return -1;
    };
    let Some(contact) = contact else {
        return -1;
    };
    if !write_output(out, contact) {
        return -2;
    }
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_move_body_position(entity: u64, x: f32, y: f32, z: f32) -> i32 {
    let entity = EntityId::new(entity);
    let target = [f64::from(x), f64::from(y), f64::from(z)];
    let moved = with_active_runtime(|runtime| runtime.move_body_position(entity, target).ok())
        .or_else(|| {
            with_registered_runtime(|runtime| runtime.move_body_position(entity, target).ok())
        });
    match moved {
        Some(()) => 0,
        None => -1,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_add_body_force(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some((force, mode)) = read_force_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        match state
            .runtime
            .add_body_force(EntityId::new(entity), force, mode)
        {
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
pub extern "C" fn unitas_runtime_character_move(
    handle: u64,
    input: Option<&AbiCharacterMoveInput>,
    output: Option<&mut AbiCharacterMoveOutput>,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let (Some(input), Some(output)) = (input, output) else {
            state.set_error("null character move buffer pointer");
            return -2;
        };
        let entity = if input.entity_id == 0 {
            None
        } else {
            Some(EntityId::new(input.entity_id))
        };
        let request = CharacterMoveRequest {
            entity,
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
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_character_teleport(
    handle: u64,
    entity: u64,
    in_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        let Some(position) = read_vec3_from_path_buffer(state, in_buffer) else {
            state.set_error("buffer address mismatch");
            return -2;
        };
        let entity_opt = if entity == 0 {
            None
        } else {
            Some(EntityId::new(entity))
        };
        match state.runtime.character_teleport(entity_opt, position) {
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
pub extern "C" fn unitas_runtime_character_controller_config(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 64
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(config) = state
            .runtime
            .character_controller_config(EntityId::new(entity))
            .cloned()
        else {
            state.set_error("character controller not found");
            return -4;
        };
        state.path_buffer[0..8].copy_from_slice(&config.radius.to_le_bytes());
        state.path_buffer[8..16].copy_from_slice(&config.height.to_le_bytes());
        state.path_buffer[16..24].copy_from_slice(&config.center[0].to_le_bytes());
        state.path_buffer[24..32].copy_from_slice(&config.center[1].to_le_bytes());
        state.path_buffer[32..40].copy_from_slice(&config.center[2].to_le_bytes());
        state.path_buffer[40..48].copy_from_slice(&config.slope_limit.to_le_bytes());
        state.path_buffer[48..56].copy_from_slice(&config.step_offset.to_le_bytes());
        state.path_buffer[56..64].copy_from_slice(&config.skin_width.to_le_bytes());
        if state.path_buffer.len() >= 76 {
            state.path_buffer[64..72].copy_from_slice(&config.min_move_distance.to_le_bytes());
            state.path_buffer[72] = u8::from(config.enabled);
            state.path_buffer[73..76].fill(0);
        }
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
