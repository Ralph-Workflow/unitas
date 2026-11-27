use unitas_audio::{decode_audio, AudioError, AudioWorld};
use unitas_core::EntityId;

const OGG_FIXTURE: &[u8] = include_bytes!("fixtures/sine_mono.ogg");
const MP3_FIXTURE: &[u8] = include_bytes!("fixtures/sine_mono.mp3");

fn build_wav(
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    pcm_samples: &[i16],
    extra_chunk: Option<(&[u8; 4], &[u8])>,
) -> Vec<u8> {
    let byte_rate = sample_rate * u32::from(channels) * 2;
    let block_align = channels * 2;
    let data_size = (pcm_samples.len() * 2) as u32;
    let extra_len = extra_chunk
        .map(|(_, data)| 8 + data.len() as u32 + (data.len() as u32 % 2))
        .unwrap_or(0);
    let riff_size = 4 + (8 + 16) + extra_len + (8 + data_size);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&audio_format.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());

    if let Some((tag, chunk_data)) = extra_chunk {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&(chunk_data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(chunk_data);
        if chunk_data.len() % 2 == 1 {
            bytes.push(0);
        }
    }

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in pcm_samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn build_aiff(channels: u16, sample_rate: u32, pcm_samples: &[i16]) -> Vec<u8> {
    let frames = (pcm_samples.len() / usize::from(channels)) as u32;

    let mut comm = Vec::new();
    comm.extend_from_slice(&channels.to_be_bytes());
    comm.extend_from_slice(&frames.to_be_bytes());
    comm.extend_from_slice(&16u16.to_be_bytes());
    comm.extend_from_slice(&extended80(sample_rate));

    let mut ssnd = Vec::new();
    ssnd.extend_from_slice(&0u32.to_be_bytes());
    ssnd.extend_from_slice(&0u32.to_be_bytes());
    for sample in pcm_samples {
        ssnd.extend_from_slice(&sample.to_be_bytes());
    }

    let mut body = Vec::new();
    body.extend_from_slice(b"AIFF");
    body.extend_from_slice(b"COMM");
    body.extend_from_slice(&(comm.len() as u32).to_be_bytes());
    body.extend_from_slice(&comm);
    body.extend_from_slice(b"SSND");
    body.extend_from_slice(&(ssnd.len() as u32).to_be_bytes());
    body.extend_from_slice(&ssnd);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FORM");
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&body);
    bytes
}

fn extended80(sample_rate: u32) -> [u8; 10] {
    let bits = 32 - sample_rate.leading_zeros();
    let exponent = 16383 + (bits - 1);
    let mantissa = u64::from(sample_rate) << (63 - (bits - 1));
    let mut bytes = [0u8; 10];
    bytes[0] = ((exponent >> 8) & 0x7f) as u8;
    bytes[1] = (exponent & 0xff) as u8;
    bytes[2..].copy_from_slice(&mantissa.to_be_bytes());
    bytes
}

#[test]
fn wav_clip_decodes_16bit_mono_and_stereo_with_extra_chunks() -> Result<(), AudioError> {
    let mono_bytes = build_wav(
        1,
        1,
        44100,
        &[0, 16384, -16384, -32768],
        Some((b"JUNK", &[1, 2, 3])),
    );
    let mono = decode_audio(&mono_bytes)?;
    assert_eq!(mono.channels, 1);
    assert_eq!(mono.sample_rate, 44100);
    assert_eq!(mono.frame_count(), 4);
    assert!((mono.duration - (4.0 / 44100.0)).abs() < 1e-9);
    assert!((mono.samples[0] - 0.0).abs() < 1e-6);
    assert!((mono.samples[1] - 0.5).abs() < 1e-6);
    assert!((mono.samples[2] - (-0.5)).abs() < 1e-6);
    assert!((mono.samples[3] - (-1.0)).abs() < 1e-6);

    let stereo_bytes = build_wav(1, 2, 48000, &[16384, -16384, 8192, -8192], None);
    let stereo = decode_audio(&stereo_bytes)?;
    assert_eq!(stereo.channels, 2);
    assert_eq!(stereo.sample_rate, 48000);
    assert_eq!(stereo.frame_count(), 2);
    assert!((stereo.duration - (2.0 / 48000.0)).abs() < 1e-9);
    assert_eq!(stereo.samples.len(), 4);
    Ok(())
}

#[test]
fn ogg_clip_decodes_through_shared_decoder() -> Result<(), AudioError> {
    let clip = decode_audio(OGG_FIXTURE)?;
    assert_eq!(clip.channels, 1);
    assert_eq!(clip.sample_rate, 44100);
    assert!(!clip.samples.is_empty());
    assert!(clip.samples.iter().any(|sample| *sample != 0.0));
    Ok(())
}

#[test]
fn mp3_clip_decodes_through_shared_decoder() -> Result<(), AudioError> {
    let clip = decode_audio(MP3_FIXTURE)?;
    assert_eq!(clip.channels, 1);
    assert_eq!(clip.sample_rate, 44100);
    assert!(!clip.samples.is_empty());
    assert!(clip.samples.iter().any(|sample| *sample != 0.0));
    Ok(())
}

#[test]
fn aiff_clip_decodes_through_shared_decoder() -> Result<(), AudioError> {
    let bytes = build_aiff(1, 44100, &[0, 16384, -16384, -32768]);
    let clip = decode_audio(&bytes)?;
    assert_eq!(clip.channels, 1);
    assert_eq!(clip.sample_rate, 44100);
    assert_eq!(clip.frame_count(), 4);
    assert!((clip.samples[1] - 0.5).abs() < 1e-6);
    assert!((clip.samples[3] - (-1.0)).abs() < 1e-6);
    Ok(())
}

#[test]
fn decoded_clip_mixes_through_audio_world() -> Result<(), AudioError> {
    let bytes = build_wav(1, 1, 44100, &[16384, -16384, 8192, -8192], None);
    let mut world = AudioWorld::new();
    let clip_id = world.load_wav_bytes(&bytes)?;
    let entity = EntityId::new(42);
    world.ensure_source(entity);
    world.set_source_clip(entity, Some(clip_id));
    world.play(entity, false);

    let mixed = world.mix(4);
    assert_eq!(mixed.len(), 8);
    let expected = [0.5, -0.5, 0.25, -0.25];
    for (frame_idx, expected_sample) in expected.iter().enumerate() {
        assert!((mixed[frame_idx * 2] - expected_sample).abs() < 1e-6);
        assert!((mixed[frame_idx * 2 + 1] - expected_sample).abs() < 1e-6);
    }
    Ok(())
}

#[test]
fn unsupported_containers_and_encodings_are_rejected() {
    let flac = decode_audio(b"fLaC\x00\x00\x00\x22");
    assert!(matches!(flac, Err(AudioError::UnsupportedFormat(_))));

    let garbage = decode_audio(b"not audio at all");
    assert!(matches!(garbage, Err(AudioError::UnsupportedFormat(_))));

    let float_wav = build_wav(3, 1, 44100, &[0, 0], None);
    assert!(matches!(
        decode_audio(&float_wav),
        Err(AudioError::UnsupportedFormat(_))
    ));

    let quad_wav = build_wav(1, 4, 44100, &[0, 0, 0, 0], None);
    assert!(matches!(
        decode_audio(&quad_wav),
        Err(AudioError::UnsupportedFormat(_))
    ));
}

#[test]
fn cooked_audio_round_trip_matches_source_decoded_samples_across_all_formats(
) -> Result<(), Box<dyn std::error::Error>> {
    use unitas_asset::audio_import::{
        import_audio, AudioImportSettings, CookedAudio, COOKED_AUDIO_FORMAT_VERSION,
    };
    use unitas_audio::AudioClip;

    let settings = AudioImportSettings::default();
    let wav_bytes = build_wav(1, 2, 48000, &[0, 16384, -16384, -32768, 8192, -8192], None);
    let aiff_bytes = build_aiff(1, 44100, &[0, 16384, -16384, -32768]);
    let fixtures: [(&str, &[u8]); 4] = [
        ("clip.wav", &wav_bytes),
        ("clip.aiff", &aiff_bytes),
        ("clip.ogg", OGG_FIXTURE),
        ("clip.mp3", MP3_FIXTURE),
    ];

    for (name, source_bytes) in fixtures {
        let direct_clip = decode_audio(source_bytes)?;
        let imported = import_audio(source_bytes, name, &settings)?;
        let cooked = CookedAudio::from_import(&imported);
        assert_eq!(cooked.format_version, COOKED_AUDIO_FORMAT_VERSION);
        let encoded = cooked.encode();
        let reloaded_clip = AudioClip::from_cooked_bytes(&encoded)?;
        assert_eq!(
            reloaded_clip.channels, direct_clip.channels,
            "{name} channels"
        );
        assert_eq!(
            reloaded_clip.sample_rate, direct_clip.sample_rate,
            "{name} sample_rate"
        );
        assert_eq!(
            reloaded_clip.samples, direct_clip.samples,
            "{name} samples must be bit-exact after cook round-trip"
        );
    }
    Ok(())
}
