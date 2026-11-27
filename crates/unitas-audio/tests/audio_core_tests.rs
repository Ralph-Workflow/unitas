use unitas_audio::{AudioClip, AudioError, AudioMixerGroup, AudioWorld};
use unitas_core::EntityId;

const OGG_FIXTURE: &[u8] = include_bytes!("fixtures/sine_mono.ogg");

fn reference_ogg_samples() -> Result<Vec<i16>, AudioError> {
    let clip = unitas_asset::audio_import::decode_ogg_vorbis(OGG_FIXTURE)
        .map_err(|err| AudioError::UnsupportedFormat(err.to_string()))?;
    Ok(clip.samples)
}

#[test]
fn the_music_loop_repeats_without_a_gap_at_the_seam() -> Result<(), AudioError> {
    let reference = reference_ogg_samples()?;
    let total_frames = reference.len();
    assert!(total_frames > 0);

    let mut world = AudioWorld::new();
    let stream_id = world.load_music_bytes(OGG_FIXTURE)?;
    let entity = EntityId::new(1);
    world.ensure_source(entity);
    world.set_source_music(entity, Some(stream_id));
    world.set_source_group(entity, AudioMixerGroup::Music);
    world.set_looping(entity, true);
    world.play(entity, false);

    let mixed = world.mix(total_frames * 2);
    assert_eq!(mixed.len(), total_frames * 4);
    for frame in 0..total_frames * 2 {
        let expected = (f32::from(reference[frame % total_frames]) / 32768.0).clamp(-1.0, 1.0);
        assert_eq!(
            mixed[frame * 2],
            expected,
            "left sample at frame {frame} carries a loop seam"
        );
        assert_eq!(
            mixed[frame * 2 + 1],
            expected,
            "right sample at frame {frame} carries a loop seam"
        );
    }
    Ok(())
}

#[test]
fn a_clip_loaded_after_the_device_rate_is_resampled_to_it() -> Result<(), AudioError> {
    let mut world = AudioWorld::new();
    world.set_device_sample_rate(44100);
    let clip_id = world.register_clip(AudioClip::new(
        1,
        22050,
        (0..22050)
            .map(|index| (index % 100) as f32 / 100.0)
            .collect(),
    ));
    let clip = world
        .clip(clip_id)
        .ok_or_else(|| AudioError::Io("clip not registered".to_string()))?;
    assert_eq!(clip.sample_rate, 44100);
    assert_eq!(clip.frame_count(), 44100);
    assert!((clip.duration - 1.0).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_clip_loaded_before_the_device_rate_is_resampled_when_the_rate_lands() -> Result<(), AudioError>
{
    let mut world = AudioWorld::new();
    let clip_id = world.register_clip(AudioClip::new(1, 22050, vec![0.5; 22050]));
    assert_eq!(
        world.clip(clip_id).map(|clip| clip.sample_rate),
        Some(22050)
    );
    world.set_device_sample_rate(44100);
    let clip = world
        .clip(clip_id)
        .ok_or_else(|| AudioError::Io("clip not registered".to_string()))?;
    assert_eq!(clip.sample_rate, 44100);
    assert_eq!(clip.frame_count(), 44100);
    assert!((clip.duration - 1.0).abs() < 1e-6);
    Ok(())
}

#[test]
fn a_delayed_source_starts_after_the_delay_in_mixer_frames() {
    let mut world = AudioWorld::new();
    world.set_device_sample_rate(8);
    let clip_id = world.register_clip(AudioClip::new(1, 8, vec![1.0; 8]));
    let entity = EntityId::new(2);
    world.ensure_source(entity);
    world.set_source_clip(entity, Some(clip_id));

    world.mix(2);
    world.play_delayed(entity, false, 0.5);

    let mixed = world.mix(6);
    for frame in 0..4 {
        assert_eq!(
            mixed[frame * 2],
            0.0,
            "frame {frame} played before the delay"
        );
    }
    for frame in 4..6 {
        assert_eq!(mixed[frame * 2], 1.0, "frame {frame} stayed silent");
    }
}

#[test]
fn the_dsp_clock_follows_the_mixer_sample_counter() {
    let mut world = AudioWorld::new();
    world.set_device_sample_rate(8);
    assert_eq!(world.dsp_time(), 0.0);
    world.mix(16);
    assert_eq!(world.mixer_sample(), 16);
    assert!((world.dsp_time() - 2.0).abs() < 1e-9);
}

#[test]
fn mixing_into_a_caller_buffer_matches_the_allocating_mix() {
    let mut allocating = AudioWorld::new();
    let clip_id = allocating.register_clip(AudioClip::new(1, 4, vec![0.25, 0.5, 0.75, 1.0]));
    let entity = EntityId::new(3);
    allocating.ensure_source(entity);
    allocating.set_source_clip(entity, Some(clip_id));
    allocating.play(entity, false);
    let expected = allocating.mix(8);

    let mut buffer = vec![9.0f32; 16];
    let mut writing = AudioWorld::new();
    let clip_id = writing.register_clip(AudioClip::new(1, 4, vec![0.25, 0.5, 0.75, 1.0]));
    let entity = EntityId::new(3);
    writing.ensure_source(entity);
    writing.set_source_clip(entity, Some(clip_id));
    writing.play(entity, false);
    writing.mix_into(&mut buffer);

    assert_eq!(buffer, expected);
}
