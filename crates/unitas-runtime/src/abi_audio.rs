use super::{AudioClip, AudioMixerGroup, EntityId, Path, read_path_buffer, with_runtime_state};

ffi_export! {
pub extern "C" fn unitas_runtime_audio_create(handle: u64, entity: u64) -> u64 {
    with_runtime_state(handle, |state| {
        let id = state.runtime.ensure_audio_source(entity);
        state.clear_error();
        id.as_u64()
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_destroy(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.audio_mut().stop(id);
        state.runtime.audio_mut().remove_source(id);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_group(handle: u64, entity: u64, group: u32) -> i32 {
    if entity == 0 || group > 2 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let group = match group {
            1 => AudioMixerGroup::Music,
            2 => AudioMixerGroup::Menu,
            _ => AudioMixerGroup::Sfx,
        };
        state.runtime.audio_mut().set_source_group(EntityId::new(entity), group);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_group(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    with_runtime_state(handle, |state| {
        state.clear_error();
        match state.runtime.audio().source_group(EntityId::new(entity)) {
            AudioMixerGroup::Sfx => 0,
            AudioMixerGroup::Music => 1,
            AudioMixerGroup::Menu => 2,
        }
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_load_music(handle: u64, address: u64, length: u64) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Ok(path_str) = read_path_buffer(state, address, len) else {
            return 0;
        };
        match state
            .runtime
            .audio_mut()
            .load_music_file(Path::new(&path_str))
        {
            Ok(stream_id) => {
                state.clear_error();
                stream_id
            }
            Err(err) => {
                state.set_error(&err.to_string());
                0
            }
        }
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_music(handle: u64, entity: u64, stream: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let stream = if stream == 0 {
            None
        } else {
            Some(stream)
        };
        state
            .runtime
            .audio_mut()
            .set_source_music(EntityId::new(entity), stream);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_play(handle: u64, entity: u64, one_shot: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .play(EntityId::new(entity), one_shot != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_play_scheduled(
    handle: u64,
    entity: u64,
    one_shot: u32,
    start_sample: u64,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .play_scheduled(EntityId::new(entity), one_shot != 0, start_sample);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
ffi_export! {
pub extern "C" fn unitas_runtime_audio_play_delayed(
    handle: u64,
    entity: u64,
    one_shot: u32,
    delay_seconds: f64,
) -> i32 {
    if entity == 0 || !delay_seconds.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .play_delayed(EntityId::new(entity), one_shot != 0, delay_seconds);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
ffi_export! {
pub extern "C" fn unitas_runtime_audio_dsp_time(handle: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.audio().dsp_time()
    })
    .unwrap_or(0.0)
}
}
ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_device_sample_rate(
    handle: u64,
    sample_rate: u32,
) -> i32 {
    if sample_rate == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().set_device_sample_rate(sample_rate);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
ffi_export! {
pub extern "C" fn unitas_runtime_audio_pause(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().pause(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_unpause(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().unpause(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_stop(handle: u64, entity: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().stop(EntityId::new(entity));
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_volume(handle: u64, entity: u64, volume: f32) -> i32 {
    if entity == 0 || !volume.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_volume(EntityId::new(entity), volume);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_pitch(handle: u64, entity: u64, pitch: f32) -> i32 {
    if entity == 0 || !pitch.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_pitch(EntityId::new(entity), pitch);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_mute(handle: u64, entity: u64, mute: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_mute(EntityId::new(entity), mute != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_loop(handle: u64, entity: u64, looping: u32) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_looping(EntityId::new(entity), looping != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_priority(
    handle: u64,
    entity: u64,
    priority: u32,
) -> i32 {
    if entity == 0 {
        return -2;
    }
    let clamped = priority.min(255) as u8;
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_priority(EntityId::new(entity), clamped);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_spatial_blend(
    handle: u64,
    entity: u64,
    blend: f32,
) -> i32 {
    if entity == 0 || !blend.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_spatial_blend(EntityId::new(entity), blend);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_min_max_distance(
    handle: u64,
    entity: u64,
    min_distance: f32,
    max_distance: f32,
) -> i32 {
    if entity == 0 || !min_distance.is_finite() || !max_distance.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let id = EntityId::new(entity);
        state.runtime.audio_mut().set_min_distance(id, min_distance);
        state.runtime.audio_mut().set_max_distance(id, max_distance);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_time(handle: u64, entity: u64, time: f64) -> i32 {
    if entity == 0 || !time.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_source_time(EntityId::new(entity), time);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_clip(handle: u64, entity: u64, clip_id: u64) -> i32 {
    if entity == 0 {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let clip_opt = if clip_id == 0 { None } else { Some(clip_id) };
        state
            .runtime
            .audio_mut()
            .set_source_clip(EntityId::new(entity), clip_opt);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_custom_duration(
    handle: u64,
    entity: u64,
    duration: f64,
) -> i32 {
    if entity == 0 || !duration.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        let dur_opt = if duration <= 0.0 {
            None
        } else {
            Some(duration)
        };
        state
            .runtime
            .audio_mut()
            .set_custom_duration(EntityId::new(entity), dur_opt);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_position(
    handle: u64,
    entity: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    if entity == 0 || !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_source_position(EntityId::new(entity), [x, y, z]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_state(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state
            .runtime
            .audio()
            .source_state(EntityId::new(entity))
            .as_i32()
    })
    .unwrap_or(-1)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_time(handle: u64, entity: u64) -> f64 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state.runtime.audio().source_time(EntityId::new(entity))
    })
    .unwrap_or(0.0)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_silence_reason(handle: u64, entity: u64) -> i32 {
    with_runtime_state(handle, |state| {
        state.clear_error();
        state
            .runtime
            .audio()
            .inaudible_reason(EntityId::new(entity))
            .map_or(-1, |r| r.as_i32())
    })
    .unwrap_or(-2)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_listener(
    handle: u64,
    x: f32,
    y: f32,
    z: f32,
    fx: f32,
    fy: f32,
    fz: f32,
) -> i32 {
    if !x.is_finite()
        || !y.is_finite()
        || !z.is_finite()
        || !fx.is_finite()
        || !fy.is_finite()
        || !fz.is_finite()
    {
        return -2;
    }
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_listener([x, y, z], [fx, fy, fz]);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_clear_listener(handle: u64) -> i32 {
    match with_runtime_state(handle, |state| {
        state.runtime.audio_mut().clear_listener();
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_set_paused(handle: u64, paused: u32) -> i32 {
    match with_runtime_state(handle, |state| {
        state
            .runtime
            .audio_mut()
            .set_application_paused(paused != 0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_mix(handle: u64, frames: u64, out_buffer: u64) -> u64 {
    let Ok(n_frames) = usize::try_from(frames) else {
        return 0;
    };
    let Some(byte_len) = n_frames.checked_mul(8) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        if out_buffer != 0 {
            if out_buffer != state.path_buffer.as_ptr() as usize as u64
                || state.path_buffer.len() < byte_len
            {
                state.set_error("buffer address mismatch");
                return 0;
            }
        } else if state.path_buffer.len() < byte_len {
            state.path_buffer.resize(byte_len, 0);
        }
        let samples = state.runtime.audio_mut().mix(n_frames);
        for (idx, sample) in samples.iter().enumerate() {
            let start = idx * 4;
            if start + 4 <= state.path_buffer.len() {
                state.path_buffer[start..start + 4].copy_from_slice(&sample.to_le_bytes());
            }
        }
        state.clear_error();
        frames
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_load_clip(handle: u64, address: u64, length: u64) -> u64 {
    let Ok(len) = usize::try_from(length) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let Ok(path_str) = read_path_buffer(state, address, len) else {
            return 0;
        };
        match state
            .runtime
            .audio_mut()
            .load_wav_file(Path::new(&path_str))
        {
            Ok(clip_id) => {
                state.clear_error();
                clip_id
            }
            Err(err) => {
                state.set_error(&err.to_string());
                0
            }
        }
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_register_clip_samples(
    handle: u64,
    channels: u32,
    sample_rate: u32,
    frame_count: u64,
    in_buffer: u64,
) -> u64 {
    if (channels != 1 && channels != 2) || sample_rate == 0 {
        return 0;
    }
    let Ok(frames) = usize::try_from(frame_count) else {
        return 0;
    };
    let ch = channels as usize;
    let Some(sample_count) = frames.checked_mul(ch) else {
        return 0;
    };
    let Some(byte_len) = sample_count.checked_mul(4) else {
        return 0;
    };
    with_runtime_state(handle, |state| {
        let samples = if in_buffer != 0 {
            if in_buffer != state.path_buffer.as_ptr() as usize as u64
                || state.path_buffer.len() < byte_len
            {
                state.set_error("buffer address mismatch");
                return 0;
            }
            let mut decoded = Vec::with_capacity(sample_count);
            for idx in 0..sample_count {
                let start = idx * 4;
                let Ok(bytes) = state.path_buffer[start..start + 4].try_into() else {
                    return 0;
                };
                decoded.push(f32::from_le_bytes(bytes));
            }
            decoded
        } else {
            vec![0.5f32; sample_count]
        };
        let clip = AudioClip::new(channels as u16, sample_rate, samples);
        let clip_id = state.runtime.audio_mut().register_clip(clip);
        state.clear_error();
        clip_id
    })
    .unwrap_or_default()
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_clip_info(
    handle: u64,
    clip_id: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 16
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let Some(clip) = state.runtime.audio().clip(clip_id) else {
            state.set_error("audio clip not found");
            return -4;
        };
        let duration = clip.duration;
        let sample_rate = clip.sample_rate;
        let channels = u32::from(clip.channels);
        state.path_buffer[0..8].copy_from_slice(&duration.to_le_bytes());
        state.path_buffer[8..12].copy_from_slice(&sample_rate.to_le_bytes());
        state.path_buffer[12..16].copy_from_slice(&channels.to_le_bytes());
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_audio_source_config(
    handle: u64,
    entity: u64,
    out_buffer: u64,
) -> i32 {
    match with_runtime_state(handle, |state| {
        if out_buffer == 0
            || out_buffer != state.path_buffer.as_ptr() as usize as u64
            || state.path_buffer.len() < 44
        {
            state.set_error("buffer address mismatch");
            return -2;
        }
        let id = EntityId::new(entity);
        let Some(config) = state.runtime.audio_source_config(id).cloned() else {
            state.set_error("audio source not found");
            return -4;
        };
        let clip_id = state
            .runtime
            .audio()
            .source(id)
            .and_then(|s| s.clip)
            .unwrap_or(0);
        let clip_duration = if clip_id != 0 {
            state
                .runtime
                .audio()
                .clip(clip_id)
                .map_or(0.0, |c| c.duration)
        } else {
            0.0
        };
        state.path_buffer[0..4].copy_from_slice(&config.volume.to_le_bytes());
        state.path_buffer[4..8].copy_from_slice(&config.pitch.to_le_bytes());
        state.path_buffer[8..12].copy_from_slice(&config.spatial_blend.to_le_bytes());
        state.path_buffer[12..16].copy_from_slice(&config.min_distance.to_le_bytes());
        state.path_buffer[16..20].copy_from_slice(&config.max_distance.to_le_bytes());
        state.path_buffer[20..28].copy_from_slice(&clip_id.to_le_bytes());
        state.path_buffer[28..36].copy_from_slice(&clip_duration.to_le_bytes());
        state.path_buffer[36] = config.priority;
        state.path_buffer[37] = u8::from(config.looping);
        state.path_buffer[38] = u8::from(config.play_on_awake);
        state.path_buffer[39] = u8::from(config.mute);
        state.path_buffer[40] = u8::from(config.enabled);
        state.path_buffer[41..44].fill(0);
        state.clear_error();
        0
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}
}
