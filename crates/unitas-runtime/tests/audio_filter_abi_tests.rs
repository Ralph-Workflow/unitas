use std::error::Error;

use unitas_runtime::{
    test_read_path_buffer, test_write_path_buffer, unitas_runtime_audio_clear_filters,
    unitas_runtime_audio_create, unitas_runtime_audio_mix, unitas_runtime_audio_play,
    unitas_runtime_audio_register_clip_samples, unitas_runtime_audio_set_clip,
    unitas_runtime_audio_set_echo_filter, unitas_runtime_audio_set_high_pass_filter,
    unitas_runtime_audio_set_low_pass_filter, unitas_runtime_destroy, unitas_runtime_new,
    unitas_runtime_path_buffer,
};

const SAMPLE_RATE: u32 = 44100;
const TONE_HZ: f32 = 8000.0;
const CUTOFF_HZ: f32 = 1000.0;

fn build_sine(frequency: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|index| {
            let phase = std::f32::consts::TAU * frequency * index as f32 / SAMPLE_RATE as f32;
            phase.sin()
        })
        .collect()
}

fn rms(samples: &[f32], skip: usize) -> f32 {
    let tail = &samples[skip..];
    let energy: f32 = tail.iter().map(|sample| sample * sample).sum();
    (energy / tail.len() as f32).sqrt()
}

fn gain_db(input_rms: f32, output_rms: f32) -> f32 {
    20.0 * (output_rms / input_rms).log10()
}

fn mix_and_measure_left_attenuation(
    handle: u64,
    half_frames: usize,
    output_buffer: u64,
    samples: &[f32],
    skip: usize,
) -> Result<f32, Box<dyn Error>> {
    let mixed_frames = unitas_runtime_audio_mix(handle, half_frames as u64, output_buffer);
    assert_eq!(mixed_frames, half_frames as u64);
    let mut mixed_bytes = vec![0u8; half_frames * 2 * 4];
    test_read_path_buffer(handle, &mut mixed_bytes)?;
    let mixed: Vec<f32> = mixed_bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    let left: Vec<f32> = mixed.iter().step_by(2).copied().collect();
    Ok(gain_db(rms(samples, skip), rms(&left, skip)))
}

#[test]
fn runtime_abi_audio_filters_round_trip() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let frames = SAMPLE_RATE as usize;
    let samples = build_sine(TONE_HZ, frames);
    let mut sample_bytes = Vec::with_capacity(samples.len() * 4);
    for sample in &samples {
        sample_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let sample_buffer = unitas_runtime_path_buffer(handle, sample_bytes.len() as u64);
    assert_ne!(sample_buffer, 0);
    test_write_path_buffer(handle, &sample_bytes)?;
    let clip_id = unitas_runtime_audio_register_clip_samples(
        handle,
        1,
        SAMPLE_RATE,
        frames as u64,
        sample_buffer,
    );
    assert_ne!(clip_id, 0);

    let entity = unitas_runtime_audio_create(handle, 0);
    assert!(entity > 0);
    assert_eq!(unitas_runtime_audio_set_clip(handle, entity, clip_id), 0);
    assert_eq!(
        unitas_runtime_audio_set_low_pass_filter(handle, entity, 1, CUTOFF_HZ, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime_audio_set_low_pass_filter(handle, 0, 1, CUTOFF_HZ, 1.0),
        -2
    );
    assert_eq!(
        unitas_runtime_audio_set_low_pass_filter(handle, entity, 1, f32::NAN, 1.0),
        -2
    );
    assert_eq!(
        unitas_runtime_audio_set_echo_filter(handle, entity, 1, f32::NAN, 0.0, -6.0, 0.0),
        -2
    );
    assert_eq!(unitas_runtime_audio_play(handle, entity, 0), 0);

    let half_frames = frames / 2;
    let output_buffer = unitas_runtime_path_buffer(handle, (half_frames * 2 * 4) as u64);
    assert_ne!(output_buffer, 0);
    let skip = half_frames / 4;
    let attenuation =
        mix_and_measure_left_attenuation(handle, half_frames, output_buffer, &samples, skip)?;
    assert!(
        attenuation <= -30.0,
        "8 kHz sine through the ABI low-pass attenuated {attenuation} dB"
    );

    assert_eq!(
        unitas_runtime_audio_set_high_pass_filter(handle, entity, 1, 2000.0, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime_audio_set_echo_filter(handle, entity, 1, 0.1, 0.0, -6.0, 0.0),
        0
    );
    assert_eq!(
        unitas_runtime_audio_set_low_pass_filter(handle, entity, 0, CUTOFF_HZ, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime_audio_set_high_pass_filter(handle, entity, 0, 2000.0, 1.0),
        0
    );
    assert_eq!(
        unitas_runtime_audio_set_echo_filter(handle, entity, 0, 0.1, 0.0, -6.0, 0.0),
        0
    );
    let attenuation =
        mix_and_measure_left_attenuation(handle, half_frames, output_buffer, &samples, skip)?;
    assert!(
        attenuation.abs() < 1.0,
        "unfiltered 8 kHz sine changed by {attenuation} dB"
    );

    assert_eq!(unitas_runtime_audio_clear_filters(handle, entity), 0);
    assert_eq!(unitas_runtime_audio_clear_filters(handle, 0), -2);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
