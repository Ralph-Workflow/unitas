use super::{
    ANIMATOR_STATE_INFO_SIZE, EntityId, RuntimeState, decode_controller_binary, read_runtime_bytes,
    with_runtime_state,
};

fn read_animator_name(
    state: &mut RuntimeState,
    address: u64,
    length: usize,
) -> Result<String, i32> {
    let Some(bytes) = read_runtime_bytes(state, address, length) else {
        state.set_error("buffer address mismatch");
        return Err(-2);
    };
    match String::from_utf8(bytes) {
        Ok(name) => Ok(name),
        Err(err) => {
            let message = err.to_string();
            state.set_error(&message);
            Err(-3)
        }
    }
}

fn animator_out_buffer_ready(state: &RuntimeState, out_buffer: u64, required: usize) -> bool {
    out_buffer != 0
        && out_buffer == state.path_buffer.as_ptr() as usize as u64
        && state.path_buffer.len() >= required
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_controller_create(
    handle: u64,
    address: u64,
    length: u64,
) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Some(bytes) = read_runtime_bytes(state, address, len) else {
            state.set_error("buffer address mismatch");
            return 0;
        };
        let controller = match decode_controller_binary(&bytes) {
            Ok(controller) => controller,
            Err(err) => {
                state.set_error(&err.to_string());
                return 0;
            }
        };
        let controller_handle = state
            .runtime
            .animation_mut()
            .register_controller(controller);
        state.clear_error();
        controller_handle
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_controller_destroy(handle: u64, controller: u64) -> i32 {
    if controller == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        if !state.runtime.animation_mut().remove_controller(controller) {
            state.set_error("animator controller not found");
            return -4;
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
pub extern "C" fn unitas_runtime_animator_attach(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        let id = state.runtime.ensure_animator(entity);
        state.clear_error();
        id.as_u64()
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_set_controller(
    handle: u64,
    entity: u64,
    controller: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let result = state
            .runtime
            .set_animator_controller(EntityId::new(entity), controller);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_set_float(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: f32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_float(id, &name, value);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_get_float(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_float(id, &name);
        match result {
            Ok(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_set_int(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: i32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_int(id, &name, value);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_get_int(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_int(id, &name);
        match result {
            Ok(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_set_bool(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    value: u32,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state
            .runtime
            .animation_mut()
            .set_bool(id, &name, value != 0);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_get_bool(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
    out_buffer: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation().get_bool(id, &name);
        match result {
            Ok(value) => {
                let encoded = u32::from(value);
                state.path_buffer[0..4].copy_from_slice(&encoded.to_le_bytes());
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_set_trigger(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_trigger(id, &name);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_reset_trigger(
    handle: u64,
    entity: u64,
    name_address: u64,
    name_length: u64,
) -> i32 {
    let Ok(length) = usize::try_from(name_length) else {
        return -2;
    };
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let name = match read_animator_name(state, name_address, length) {
            Ok(name) => name,
            Err(code) => return code,
        };
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().reset_trigger(id, &name);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_set_speed(handle: u64, entity: u64, speed: f32) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let result = state.runtime.animation_mut().set_speed(id, speed);
        match result {
            Ok(()) => {
                state.clear_error();
                0
            }
            Err(message) => {
                state.set_error(&message);
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
pub extern "C" fn unitas_runtime_animator_get_speed(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, 4) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let speed = state.runtime.animation().speed(id);
        match speed {
            Some(value) => {
                state.path_buffer[0..4].copy_from_slice(&value.to_le_bytes());
                state.clear_error();
                0
            }
            None => {
                state.set_error("animator not found");
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
pub extern "C" fn unitas_runtime_animator_state_info(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if !animator_out_buffer_ready(state, out_buffer, ANIMATOR_STATE_INFO_SIZE) {
            state.set_error("buffer address mismatch");
            return -2;
        }
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let info = match state.runtime.animation().state_info(id) {
            Ok(info) => info,
            Err(message) => {
                state.set_error(&message);
                return -4;
            }
        };
        let transform = state
            .runtime
            .animation()
            .transform(id)
            .cloned()
            .unwrap_or_default();
        let fired_events = state
            .runtime
            .animation()
            .fired_event_count(id)
            .unwrap_or_default();
        let diagnostics = state
            .runtime
            .animation()
            .diagnostic_count(id)
            .unwrap_or_default();
        let next_state_index = match info.next_state_index {
            Some(index) => u32::try_from(index).unwrap_or(u32::MAX),
            None => u32::MAX,
        };
        let mut bytes = [0u8; ANIMATOR_STATE_INFO_SIZE];
        let state_index = u32::try_from(info.state_index).unwrap_or(u32::MAX);
        bytes[0..4].copy_from_slice(&state_index.to_le_bytes());
        bytes[4..8].copy_from_slice(&info.normalized_time.to_le_bytes());
        bytes[8..12].copy_from_slice(&info.length.to_le_bytes());
        bytes[12..16].copy_from_slice(&info.speed.to_le_bytes());
        bytes[16..20].copy_from_slice(&info.weight.to_le_bytes());
        bytes[20..24].copy_from_slice(&u32::from(info.is_in_transition).to_le_bytes());
        bytes[24..28].copy_from_slice(&next_state_index.to_le_bytes());
        bytes[28..32].copy_from_slice(&info.next_state_weight.to_le_bytes());
        bytes[32..36].copy_from_slice(&transform.translation[0].to_le_bytes());
        bytes[36..40].copy_from_slice(&transform.translation[1].to_le_bytes());
        bytes[40..44].copy_from_slice(&transform.translation[2].to_le_bytes());
        bytes[44..48].copy_from_slice(&transform.rotation[0].to_le_bytes());
        bytes[48..52].copy_from_slice(&transform.rotation[1].to_le_bytes());
        bytes[52..56].copy_from_slice(&transform.rotation[2].to_le_bytes());
        bytes[56..60].copy_from_slice(&transform.rotation[3].to_le_bytes());
        bytes[60..64].copy_from_slice(&transform.scale[0].to_le_bytes());
        bytes[64..68].copy_from_slice(&transform.scale[1].to_le_bytes());
        bytes[68..72].copy_from_slice(&transform.scale[2].to_le_bytes());
        let fired_event_count = u32::try_from(fired_events).unwrap_or(u32::MAX);
        let diagnostic_count = u32::try_from(diagnostics).unwrap_or(u32::MAX);
        bytes[72..76].copy_from_slice(&fired_event_count.to_le_bytes());
        bytes[76..80].copy_from_slice(&diagnostic_count.to_le_bytes());
        state.path_buffer[0..ANIMATOR_STATE_INFO_SIZE].copy_from_slice(&bytes);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_animator_state_name(handle: u64, entity: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        if entity == 0 {
            state.set_error("invalid entity id");
            return -2;
        }
        let id = EntityId::new(entity);
        let info = match state.runtime.animation().state_info(id) {
            Ok(info) => info,
            Err(message) => {
                state.set_error(&message);
                return -4;
            }
        };
        let bytes = info.state_name.as_bytes();
        if state.path_buffer.len() < bytes.len() {
            state.set_error(&format!(
                "path buffer too small for state name: {} bytes needed",
                bytes.len()
            ));
            return -4;
        }
        state.path_buffer[..bytes.len()].copy_from_slice(bytes);
        state.clear_error();
        match i32::try_from(bytes.len()) {
            Ok(length) => length,
            Err(_) => {
                state.set_error("state name length exceeds supported range");
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
pub extern "C" fn unitas_runtime_animator_drain_fired_events(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        let entities = state.runtime.animation().entities();
        let mut records = Vec::new();
        for entity in entities {
            let events = state.runtime.animation_mut().take_fired_events(entity);
            for event in events {
                records.push((entity.as_u64(), event));
            }
        }
        let count = records.len();
        let mut payload = Vec::new();
        let count_u32 = u32::try_from(count).unwrap_or(u32::MAX);
        payload.extend_from_slice(&count_u32.to_le_bytes());
        for (entity_id, event) in &records {
            payload.extend_from_slice(&entity_id.to_le_bytes());
            payload.extend_from_slice(&event.float_parameter.to_le_bytes());
            payload.extend_from_slice(&event.int_parameter.to_le_bytes());
            let fn_bytes = event.function_name.as_bytes();
            let fn_len = u32::try_from(fn_bytes.len()).unwrap_or(0);
            payload.extend_from_slice(&fn_len.to_le_bytes());
            payload.extend_from_slice(&fn_bytes[..usize::try_from(fn_len).unwrap_or(0)]);
            let str_bytes = event.string_parameter.as_bytes();
            let str_len = u32::try_from(str_bytes.len()).unwrap_or(0);
            payload.extend_from_slice(&str_len.to_le_bytes());
            payload.extend_from_slice(&str_bytes[..usize::try_from(str_len).unwrap_or(0)]);
        }
        if state.path_buffer.len() < payload.len() {
            state.path_buffer.resize(payload.len(), 0);
        }
        state.path_buffer[..payload.len()].copy_from_slice(&payload);
        state.clear_error();
        i32::try_from(payload.len()).unwrap_or(-4)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
