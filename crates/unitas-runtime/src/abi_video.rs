use std::path::Path;
use std::time::Duration;

use unitas_core::EntityId;

use crate::abi::{test_read_path_buffer, test_write_path_buffer, unitas_runtime_path_buffer};
use crate::video::{
    VideoEvents, VideoPlayerSnapshot, advance_video_player_by_id, create_video_player_by_id,
    destroy_video_player_by_id, load_video_clip_by_id, pause_video_player_by_id,
    play_video_player_by_id, prepare_video_player_by_id, set_video_player_frame_by_id,
    set_video_player_looping_by_id, set_video_player_playback_speed_by_id,
    set_video_player_send_frame_ready_events_by_id, set_video_player_target_entity_by_id,
    set_video_player_time_by_id, step_forward_video_player_by_id, stop_video_player_by_id,
    take_video_player_snapshot_by_id, video_player_current_frame_rgba_by_id,
    wait_video_player_prepared_by_id,
};

pub const VIDEO_STATE_BLOB_BYTES: usize = 64;

fn valid_handle(handle: u64) -> bool {
    handle != 0 && unitas_runtime_path_buffer(handle, 1) != 0
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_create(handle: u64, entity: u64) -> u64 {
    if !valid_handle(handle) {
        return 0;
    }
    create_video_player_by_id(EntityId::new(entity))
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_destroy(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if destroy_video_player_by_id(player) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_load_clip(
    handle: u64,
    player: u64,
    path_ptr: u64,
    path_len: u64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    let Ok(len) = usize::try_from(path_len) else {
        return -2;
    };
    if len == 0 {
        return -2;
    }
    let base = unitas_runtime_path_buffer(handle, path_len);
    if base == 0 || path_ptr != base {
        return -2;
    }
    let mut raw = vec![0u8; len];
    if test_read_path_buffer(handle, &mut raw).is_err() {
        return -2;
    }
    let Ok(path_str) = std::str::from_utf8(&raw) else {
        return -3;
    };
    let trimmed = path_str.trim_end_matches('\0');
    if load_video_clip_by_id(player, Path::new(trimmed)) {
        0
    } else {
        -4
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_prepare(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if prepare_video_player_by_id(player) {
        0
    } else {
        -4
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_wait_prepared(
    handle: u64,
    player: u64,
    timeout_ms: u32,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if wait_video_player_prepared_by_id(player, Duration::from_millis(u64::from(timeout_ms))) {
        0
    } else {
        -4
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_play(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if play_video_player_by_id(player) {
        0
    } else {
        -4
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_pause(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if pause_video_player_by_id(player) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_stop(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if stop_video_player_by_id(player) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_step_forward(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if step_forward_video_player_by_id(player) {
        0
    } else {
        -4
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_time(
    handle: u64,
    player: u64,
    seconds: f64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_time_by_id(player, seconds) {
        0
    } else {
        -2
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_frame(
    handle: u64,
    player: u64,
    frame: i64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_frame_by_id(player, frame) {
        0
    } else {
        -2
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_looping(
    handle: u64,
    player: u64,
    looping: u32,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_looping_by_id(player, looping != 0) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_playback_speed(
    handle: u64,
    player: u64,
    speed: f32,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_playback_speed_by_id(player, speed) {
        0
    } else {
        -2
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_send_frame_ready_events(
    handle: u64,
    player: u64,
    enabled: u32,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_send_frame_ready_events_by_id(player, enabled != 0) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_set_target_entity(
    handle: u64,
    player: u64,
    target_entity: u64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if set_video_player_target_entity_by_id(player, EntityId::new(target_entity)) {
        0
    } else {
        -1
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_advance(
    handle: u64,
    player: u64,
    delta_seconds: f64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    if advance_video_player_by_id(player, delta_seconds) {
        0
    } else {
        -2
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_get_state(
    handle: u64,
    player: u64,
    out_ptr: u64,
) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    let base = unitas_runtime_path_buffer(handle, VIDEO_STATE_BLOB_BYTES as u64);
    if base == 0 || out_ptr != base {
        return -2;
    }
    let Some(snap) = take_video_player_snapshot_by_id(player) else {
        return -1;
    };
    let blob = encode_video_state_blob(&snap);
    if test_write_path_buffer(handle, &blob).is_err() {
        return -2;
    }
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_video_get_frame_rgba(handle: u64, player: u64) -> i32 {
    if !valid_handle(handle) {
        return -1;
    }
    let Some(rgba) = video_player_current_frame_rgba_by_id(player) else {
        return -4;
    };
    let count = rgba.len();
    let ptr = unitas_runtime_path_buffer(handle, count as u64);
    if ptr == 0 || test_write_path_buffer(handle, &rgba).is_err() {
        return -2;
    }
    i32::try_from(count).unwrap_or(-2)
}
}

fn encode_video_state_blob(snap: &VideoPlayerSnapshot) -> [u8; VIDEO_STATE_BLOB_BYTES] {
    let mut blob = [0u8; VIDEO_STATE_BLOB_BYTES];
    blob[0..4].copy_from_slice(&snap.width.to_le_bytes());
    blob[4..8].copy_from_slice(&snap.height.to_le_bytes());
    blob[8..16].copy_from_slice(&snap.frame_count.to_le_bytes());
    blob[16..24].copy_from_slice(&snap.frame_rate.to_le_bytes());
    blob[24..32].copy_from_slice(&snap.length_seconds.to_le_bytes());
    blob[32..40].copy_from_slice(&snap.current_frame.to_le_bytes());
    blob[40..48].copy_from_slice(&snap.current_time.to_le_bytes());
    blob[48..52].copy_from_slice(&snap.playback_speed.to_le_bytes());
    let mut flags = 0u32;
    if snap.is_prepared {
        flags |= 1 << 0;
    }
    if snap.is_playing {
        flags |= 1 << 1;
    }
    if snap.is_paused {
        flags |= 1 << 2;
    }
    if snap.is_looping {
        flags |= 1 << 3;
    }
    if snap.send_frame_ready_events {
        flags |= 1 << 4;
    }
    blob[52..56].copy_from_slice(&flags.to_le_bytes());
    let events: VideoEvents = snap.events;
    blob[56..60].copy_from_slice(&events.to_bits().to_le_bytes());
    let has_error = u32::from(snap.last_error.is_some());
    blob[60..64].copy_from_slice(&has_error.to_le_bytes());
    blob
}
