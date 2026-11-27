use unitas_runtime::{
    unitas_runtime_audio_create, unitas_runtime_audio_dsp_time, unitas_runtime_audio_mix,
    unitas_runtime_audio_play_delayed, unitas_runtime_audio_play_scheduled,
    unitas_runtime_audio_register_clip_samples, unitas_runtime_audio_set_clip,
    unitas_runtime_audio_set_device_sample_rate, unitas_runtime_audio_state,
    unitas_runtime_destroy, unitas_runtime_new,
};

const SCHEDULED_STATE: i32 = 0;
const PLAYING_STATE: i32 = 2;

#[test]
fn a_delayed_source_starts_once_the_mixer_clock_reaches_the_delay() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0, "runtime handle creation failed");
    assert_eq!(unitas_runtime_audio_set_device_sample_rate(handle, 8), 0);

    let entity = unitas_runtime_audio_create(handle, 42);
    assert_ne!(entity, 0, "audio source creation failed");
    let clip = unitas_runtime_audio_register_clip_samples(handle, 1, 8, 8, 0);
    assert_ne!(clip, 0, "clip registration failed");
    assert_eq!(unitas_runtime_audio_set_clip(handle, entity, clip), 0);

    assert_eq!(unitas_runtime_audio_play_delayed(handle, entity, 0, 0.5), 0);
    assert_eq!(unitas_runtime_audio_state(handle, entity), SCHEDULED_STATE);

    assert_eq!(unitas_runtime_audio_mix(handle, 2, 0), 2);
    assert!((unitas_runtime_audio_dsp_time(handle) - 0.25).abs() < 1e-9);
    assert_eq!(unitas_runtime_audio_state(handle, entity), SCHEDULED_STATE);

    assert_eq!(unitas_runtime_audio_mix(handle, 4, 0), 4);
    assert!((unitas_runtime_audio_dsp_time(handle) - 0.75).abs() < 1e-9);
    assert_eq!(unitas_runtime_audio_state(handle, entity), PLAYING_STATE);

    assert_eq!(unitas_runtime_destroy(handle), 0);
}

#[test]
fn a_source_scheduled_at_a_mixer_sample_starts_at_that_sample() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0, "runtime handle creation failed");
    assert_eq!(unitas_runtime_audio_set_device_sample_rate(handle, 8), 0);

    let entity = unitas_runtime_audio_create(handle, 43);
    assert_ne!(entity, 0, "audio source creation failed");
    let clip = unitas_runtime_audio_register_clip_samples(handle, 1, 8, 8, 0);
    assert_ne!(clip, 0, "clip registration failed");
    assert_eq!(unitas_runtime_audio_set_clip(handle, entity, clip), 0);

    assert_eq!(unitas_runtime_audio_play_scheduled(handle, entity, 0, 3), 0);
    assert_eq!(unitas_runtime_audio_state(handle, entity), SCHEDULED_STATE);
    assert_eq!(unitas_runtime_audio_mix(handle, 3, 0), 3);
    assert_eq!(unitas_runtime_audio_state(handle, entity), SCHEDULED_STATE);
    assert_eq!(unitas_runtime_audio_mix(handle, 3, 0), 3);
    assert_eq!(unitas_runtime_audio_state(handle, entity), PLAYING_STATE);

    assert_eq!(unitas_runtime_destroy(handle), 0);
}

#[test]
fn the_abi_refuses_a_scheduled_playback_without_an_entity_or_a_finite_delay() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0, "runtime handle creation failed");
    assert_eq!(unitas_runtime_audio_play_delayed(handle, 0, 0, 0.5), -2);
    assert_eq!(
        unitas_runtime_audio_play_delayed(handle, 7, 0, f64::NAN),
        -2
    );
    assert_eq!(unitas_runtime_audio_play_scheduled(handle, 0, 0, 4), -2);
    assert_eq!(unitas_runtime_audio_set_device_sample_rate(handle, 0), -2);
    assert_eq!(unitas_runtime_destroy(handle), 0);
}
