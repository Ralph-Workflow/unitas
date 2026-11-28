use super::{EntityId, with_runtime_state};

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_ensure(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.particles_mut().ensure_system(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_has(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let exists = state.runtime.particles_mut().has_system(EntityId::new(entity));
        state.clear_error();
        i32::from(exists)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_play(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        state.runtime.particles_mut().play(id);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_pause(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        state.runtime.particles_mut().pause(id);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_stop(
    handle: u64,
    entity: u64,
    stop_behavior: u32,
) -> i32 {
    if entity == 0 || stop_behavior > 1 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        let clear = stop_behavior == 0;
        state.runtime.particles_mut().stop_with_behavior(id, clear);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_clear(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        state.runtime.particles_mut().clear(id);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_simulate(
    handle: u64,
    entity: u64,
    delta_time: f32,
    restart: u8,
) -> i32 {
    if entity == 0 || !delta_time.is_finite() || delta_time < 0.0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        if !state
            .runtime
            .particles_mut()
            .simulate(id, delta_time, restart != 0)
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
pub extern "C" fn unitas_runtime_particle_system_emit(handle: u64, entity: u64, count: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.particles_mut().ensure_system(id);
        state.runtime.particles_mut().emit(id, count);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_is_playing(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.particles_mut().has_system(id) {
            return -3;
        }
        let playing = state.runtime.particles_mut().is_playing(id);
        state.clear_error();
        i32::from(playing)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_is_paused(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.particles_mut().has_system(id) {
            return -3;
        }
        let paused = state.runtime.particles_mut().is_paused(id);
        state.clear_error();
        i32::from(paused)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_is_stopped(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.particles_mut().has_system(id) {
            return -3;
        }
        let stopped = state.runtime.particles_mut().is_stopped(id);
        state.clear_error();
        i32::from(stopped)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_particle_system_particle_count(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        if !state.runtime.particles_mut().has_system(id) {
            return -3;
        }
        let count = state.runtime.particles_mut().particle_count(id);
        state.clear_error();
        i32::try_from(count).unwrap_or(i32::MAX)
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
