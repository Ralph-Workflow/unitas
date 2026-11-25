use unitas_asset::audio_import::{
    audio_import_outcome, decode_aiff, decode_mp3, decode_ogg_vorbis, decode_wav,
    detect_audio_state, import_audio, import_audio_for_target, AudioCompressionFormat,
    AudioImportError, AudioImportSettings, AudioImportState, AudioLoadType, AudioPlatformSettings,
};
use unitas_asset::dependencies::AssetImportState;

fn extended80(rate: u32) -> [u8; 10] {
    let mut bytes = [0u8; 10];
    bytes[0] = 0x40;
    bytes[1] = 0x0e;
    bytes[2..10].copy_from_slice(&((rate as u64) << 48).to_be_bytes());
    bytes
}

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + body.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn comm_body(
    channels: u16,
    frames: u32,
    sample_size: u16,
    rate: u32,
    compression: Option<&[u8; 4]>,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&channels.to_be_bytes());
    body.extend_from_slice(&frames.to_be_bytes());
    body.extend_from_slice(&sample_size.to_be_bytes());
    body.extend_from_slice(&extended80(rate));
    if let Some(compression) = compression {
        body.extend_from_slice(compression);
        body.push(0);
    }
    body
}

fn ssnd_body(data: &[u8], offset: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&offset.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.resize(body.len() + offset as usize, 0u8);
    body.extend_from_slice(data);
    body
}

fn build_form(form_type: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(form_type);
    for chunk in chunks {
        body.extend_from_slice(chunk);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

fn pcm16(samples: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        out.extend_from_slice(&sample.to_be_bytes());
    }
    out
}

fn pcm16_le(samples: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn pcm8(samples: &[i16]) -> Vec<u8> {
    samples.iter().map(|sample| (sample + 128) as u8).collect()
}

fn stereo_aiff() -> Vec<u8> {
    let samples = vec![0, 1000, -1000, 32767, -32768, 5, -5, 0];
    build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(2, 4, 16, 44100, None)),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    )
}

#[test]
fn decodes_16_bit_stereo_aiff_at_44100() {
    let samples = vec![0, 1000, -1000, 32767, -32768, 5, -5, 0];
    let bytes = stereo_aiff();
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip)
            if clip.channels == 2 && clip.sample_rate == 44100 && clip.samples == samples),
        "expected the interleaved stereo samples at 44100 Hz"
    );
}

#[test]
fn decodes_8_bit_mono_aiff() {
    let samples = vec![-128, -1, 0, 1, 127];
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(1, 5, 8, 22050, None)),
            chunk(b"SSND", &ssnd_body(&pcm8(&samples), 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip)
            if clip.channels == 1 && clip.sample_rate == 22050 && clip.samples == samples),
        "expected unsigned 8-bit samples decoded around zero"
    );
}

#[test]
fn decodes_aifc_with_none_compression() {
    let samples = vec![10, -10, 20, -20];
    let bytes = build_form(
        b"AIFC",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 48000, Some(b"NONE"))),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip)
            if clip.channels == 2 && clip.sample_rate == 48000 && clip.samples == samples),
        "expected AIFF-C NONE samples to decode"
    );
}

#[test]
fn decodes_aifc_with_sowt_little_endian_samples() {
    let samples = vec![10, -10, 20, -20];
    let bytes = build_form(
        b"AIFC",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 48000, Some(b"sowt"))),
            chunk(b"SSND", &ssnd_body(&pcm16_le(&samples), 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip)
            if clip.channels == 2 && clip.sample_rate == 48000 && clip.samples == samples),
        "expected AIFF-C sowt little-endian samples to decode"
    );
}

#[test]
fn decodes_aifc_with_twos_compression_as_big_endian() {
    let samples = vec![10, -10, 20, -20];
    let bytes = build_form(
        b"AIFC",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 48000, Some(b"twos"))),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip) if clip.samples == samples),
        "expected AIFF-C twos samples to decode as big-endian PCM"
    );
}

#[test]
fn aifc_with_unsupported_compression_names_the_type() {
    let bytes = build_form(
        b"AIFC",
        &[
            chunk(b"COMM", &comm_body(1, 1, 16, 44100, Some(b"fl32"))),
            chunk(b"SSND", &ssnd_body(&pcm16(&[0]), 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Err(AudioImportError::UnsupportedCompression(name))
            if name == "fl32"),
        "expected the unsupported compression type to be named"
    );
}

#[test]
fn truncated_file_reports_truncated() {
    let bytes = stereo_aiff();
    let truncated = &bytes[..bytes.len() - 4];
    assert!(
        matches!(decode_aiff(truncated), Err(AudioImportError::Truncated)),
        "expected a truncated file to be reported"
    );
}

#[test]
fn truncated_sample_data_reports_truncated() {
    let samples = vec![0i16; 4];
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 44100, None)),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    );
    assert!(
        matches!(
            decode_aiff(&bytes[..bytes.len() - 2]),
            Err(AudioImportError::Truncated)
        ),
        "expected short sample data to be reported"
    );
}

#[test]
fn missing_comm_chunk_reports_missing_chunk() {
    let bytes = build_form(b"AIFF", &[chunk(b"SSND", &ssnd_body(&pcm16(&[0]), 0))]);
    assert!(
        matches!(decode_aiff(&bytes), Err(AudioImportError::MissingChunk(name)) if name == "COMM"),
        "expected the missing COMM chunk to be named"
    );
}

#[test]
fn rejects_24_bit_sample_size() {
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(1, 1, 24, 44100, None)),
            chunk(b"SSND", &ssnd_body(&[0, 0, 0], 0)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Err(AudioImportError::UnsupportedSampleSize(bits)) if bits == 24),
        "expected the 24-bit sample size to be reported"
    );
}

#[test]
fn honours_the_ssnd_offset() {
    let samples = vec![1i16, 2, 3, 4];
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 44100, None)),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 4)),
        ],
    );
    assert!(
        matches!(decode_aiff(&bytes), Ok(clip) if clip.samples == samples),
        "expected the SSND offset bytes to be skipped"
    );
}

#[test]
fn detects_imported_aiff() {
    let bytes = stereo_aiff();
    assert!(
        detect_audio_state(&bytes, "clip.aiff") == AudioImportState::Imported,
        "expected a valid AIFF to be detected as imported"
    );
}

#[test]
fn detects_not_yet_supported_containers() {
    let opus_ogg = [
        b'O', b'g', b'g', b'S', 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x86, 0x99, 0x62, 0x35, 0x01, 0x08, b'O', b'p',
        b'u', b's', b'H', b'e', b'a', b'd',
    ];
    assert!(
        detect_audio_state(&opus_ogg, "voice.opus") == AudioImportState::NotYetSupported,
        "expected non-Vorbis Opus OGG to be reported as not yet supported"
    );
    assert!(
        detect_audio_state(b"not-audio", "tune.xm") == AudioImportState::NotYetSupported,
        "expected a tracker module to be reported as not yet supported"
    );
    assert!(
        detect_audio_state(b"not-audio", "tune.it") == AudioImportState::NotYetSupported,
        "expected a tracker module to be reported as not yet supported"
    );
    assert!(
        detect_audio_state(b"not-audio", "tune.mod") == AudioImportState::NotYetSupported,
        "expected a tracker module to be reported as not yet supported"
    );
}

#[test]
fn decodes_mono_ogg_vorbis_fixture() {
    let bytes = include_bytes!("../fixtures/sine_mono.ogg");
    assert!(
        matches!(decode_ogg_vorbis(bytes), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 44100
                && !clip.samples.is_empty()
                && clip.samples.iter().any(|sample| *sample != 0)),
        "expected mono OGG Vorbis fixture to decode non-silent PCM samples at 44100 Hz"
    );
    assert!(
        detect_audio_state(bytes, "sine_mono.ogg") == AudioImportState::Imported,
        "expected mono OGG Vorbis fixture to be detected as imported"
    );
}

#[test]
fn decodes_stereo_ogg_vorbis_fixture_and_applies_import_settings() {
    let bytes = include_bytes!("../fixtures/sine_stereo.ogg");
    assert!(
        matches!(decode_ogg_vorbis(bytes), Ok(clip)
            if clip.channels == 2
                && clip.sample_rate == 48000
                && clip.samples.len() >= 64
                && clip.samples.len() % 2 == 0
                && clip.samples.iter().any(|sample| *sample != 0)),
        "expected stereo OGG Vorbis fixture to decode interleaved non-silent PCM samples at 48000 Hz"
    );
    let settings = AudioImportSettings {
        force_to_mono: true,
        compression_format: AudioCompressionFormat::Vorbis,
        ..AudioImportSettings::default()
    };
    assert!(
        matches!(import_audio(bytes, "sine_stereo.ogg", &settings), Ok(result)
            if result.state == AudioImportState::Imported
                && result.clip.channels == 1
                && result.clip.sample_rate == 48000
                && !result.clip.samples.is_empty()
                && result.clip.samples.iter().any(|sample| *sample != 0)),
        "expected import_audio to decode stereo OGG Vorbis and apply force_to_mono"
    );
}

#[test]
fn decodes_mono_mp3_fixture() {
    let bytes = include_bytes!("../fixtures/sine_mono.mp3");
    assert!(
        matches!(decode_mp3(bytes), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 44100
                && !clip.samples.is_empty()
                && clip.samples.iter().any(|sample| *sample != 0)),
        "expected mono MP3 fixture to decode non-silent PCM samples at 44100 Hz"
    );
    assert!(
        detect_audio_state(bytes, "sine_mono.mp3") == AudioImportState::Imported,
        "expected mono MP3 fixture to be detected as imported"
    );
}

#[test]
fn decodes_stereo_mp3_fixture_and_applies_import_settings() {
    let bytes = include_bytes!("../fixtures/sine_stereo.mp3");
    assert!(
        matches!(decode_mp3(bytes), Ok(clip)
            if clip.channels == 2
                && clip.sample_rate == 48000
                && !clip.samples.is_empty()
                && clip.samples.len() % 2 == 0
                && clip.samples.iter().any(|sample| *sample != 0)),
        "expected stereo MP3 fixture to decode interleaved non-silent PCM samples at 48000 Hz"
    );
    let settings = AudioImportSettings {
        force_to_mono: true,
        compression_format: AudioCompressionFormat::MP3,
        ..AudioImportSettings::default()
    };
    assert!(
        matches!(import_audio(bytes, "sine_stereo.mp3", &settings), Ok(result)
            if result.state == AudioImportState::Imported
                && result.clip.channels == 1
                && result.clip.sample_rate == 48000
                && !result.clip.samples.is_empty()
                && result.clip.samples.iter().any(|sample| *sample != 0)),
        "expected import_audio to decode stereo MP3 and apply force_to_mono"
    );
}

#[test]
fn rejects_corrupt_and_truncated_ogg_and_mp3_streams() {
    let ogg = include_bytes!("../fixtures/sine_mono.ogg");
    assert!(
        matches!(
            decode_ogg_vorbis(&ogg[..16]),
            Err(AudioImportError::Truncated)
        ),
        "expected truncated OGG header to report Truncated"
    );
    assert!(
        matches!(
            decode_ogg_vorbis(b"OggS-not-a-valid-page-header-at-all"),
            Err(AudioImportError::CorruptOgg(_) | AudioImportError::Truncated)
        ),
        "expected corrupt OGG stream to return a diagnostic error"
    );
    assert!(
        detect_audio_state(b"OggS-not-a-real-stream", "music.ogg")
            == AudioImportState::ImportFailed,
        "expected corrupt OGG to report ImportFailed"
    );

    let mp3 = include_bytes!("../fixtures/sine_mono.mp3");
    assert!(
        matches!(decode_mp3(&mp3[..8]), Err(AudioImportError::Truncated)),
        "expected truncated ID3 header to report Truncated"
    );
    assert!(
        matches!(
            decode_mp3(b"ID3-not-a-real-frame"),
            Err(AudioImportError::Truncated | AudioImportError::CorruptMp3(_))
        ),
        "expected corrupt MP3 stream to return a diagnostic error"
    );
    assert!(
        detect_audio_state(b"ID3-not-a-real-frame", "music.mp3") == AudioImportState::ImportFailed,
        "expected corrupt MP3 to report ImportFailed"
    );
}

#[test]
fn detects_unsupported_container() {
    assert!(
        detect_audio_state(b"not-audio-at-all", "mystery.bin")
            == AudioImportState::UnsupportedContainer,
        "expected an unknown container to be reported"
    );
}

#[test]
fn detects_failed_import_for_broken_aiff() {
    let bytes = stereo_aiff();
    let truncated = &bytes[..bytes.len() - 4];
    assert!(
        detect_audio_state(truncated, "clip.aiff") == AudioImportState::ImportFailed,
        "expected a broken AIFF to be reported as a failed import"
    );
}

#[test]
fn import_applies_force_to_mono() {
    let samples = vec![100, 300, -100, -300, 10, -10];
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(2, 3, 16, 44100, None)),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    );
    let settings = AudioImportSettings {
        force_to_mono: true,
        ..AudioImportSettings::default()
    };
    assert!(
        matches!(import_audio(&bytes, "clip.aiff", &settings), Ok(result)
            if result.state == AudioImportState::Imported
                && result.clip.channels == 1
                && result.clip.samples == vec![200, -200, 0]
                && result.settings == settings),
        "expected the stereo frames to be averaged into mono"
    );
}

#[test]
fn import_reports_the_sample_rate_override() {
    let bytes = stereo_aiff();
    let settings = AudioImportSettings {
        sample_rate_override: Some(22050),
        ..AudioImportSettings::default()
    };
    assert!(
        matches!(import_audio(&bytes, "clip.aiff", &settings), Ok(result)
            if result.clip.sample_rate == 22050
                && result.settings == settings
                && result.notes.iter().any(|note| note.contains("22050"))),
        "expected the sample rate override to be reported"
    );
}

#[test]
fn import_keeps_the_requested_settings_and_names_the_compression_format() {
    let bytes = stereo_aiff();
    let settings = AudioImportSettings {
        load_type: AudioLoadType::Streaming,
        compression_format: AudioCompressionFormat::Vorbis,
        preload_audio_data: false,
        quality: 0.5,
        ..AudioImportSettings::default()
    };
    assert!(
        matches!(import_audio(&bytes, "clip.aiff", &settings), Ok(result)
            if result.settings.load_type == AudioLoadType::Streaming
                && result.settings.compression_format == AudioCompressionFormat::Vorbis
                && !result.settings.preload_audio_data
                && result.settings.quality == 0.5
                && result.notes.iter().any(|note| note.contains("Vorbis"))),
        "expected the import settings to survive the round trip"
    );
}

#[test]
fn import_reports_not_yet_supported_with_the_file_name() {
    let settings = AudioImportSettings::default();
    assert!(
        matches!(import_audio(b"OggS-not-a-real-stream", "tune.xm", &settings), Ok(result)
            if result.state == AudioImportState::NotYetSupported
                && result.clip.samples.is_empty()
                && result.notes.iter().any(|note| note.contains("tune.xm"))),
        "expected the unsupported file to be named"
    );
}

#[test]
fn import_reports_an_unsupported_container_as_an_error() {
    let settings = AudioImportSettings::default();
    assert!(
        matches!(
            import_audio(b"not-audio-at-all", "mystery.bin", &settings),
            Err(AudioImportError::NotAiff)
        ),
        "expected an unsupported container to fail the import"
    );
}

#[test]
fn prj004_outcome16_xm_modules_are_reported_unsupported() {
    let xm_header = b"Extended Module: ChipTune       \x1aFastTracker v2.00   \x04\x01";
    let settings = AudioImportSettings::default();
    assert_eq!(
        detect_audio_state(xm_header, "Assets/Audio/level1.xm"),
        AudioImportState::NotYetSupported
    );
    let result = match import_audio(xm_header, "Assets/Audio/level1.xm", &settings) {
        Ok(result) => result,
        Err(error) => {
            assert_eq!(
                error,
                AudioImportError::NotAiff,
                "unexpected error: {error}"
            );
            return;
        }
    };
    assert_eq!(result.state, AudioImportState::NotYetSupported);
    assert!(result.clip.samples.is_empty());
    assert!(
        result.notes.iter().any(|note| {
            note.contains("Assets/Audio/level1.xm")
                && note.contains("xm")
                && note.contains("Unsupported")
        }),
        "expected note naming file, format 'xm', and 'Unsupported': {:?}",
        result.notes
    );
}

fn wav_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + body.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if !body.len().is_multiple_of(2) {
        out.push(0);
    }
    out
}

fn wav_fmt_body(format_tag: u16, channels: u16, sample_rate: u32, bits_per_sample: u16) -> Vec<u8> {
    let bytes_per_sample = (bits_per_sample / 8).max(1);
    let block_align = channels * bytes_per_sample;
    let byte_rate = sample_rate * u32::from(block_align);
    let mut body = Vec::with_capacity(16);
    body.extend_from_slice(&format_tag.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&bits_per_sample.to_le_bytes());
    body
}

fn wav_extensible_fmt_body(
    sub_format_tag: u16,
    channels: u16,
    sample_rate: u32,
    container_bits: u16,
    valid_bits: u16,
) -> Vec<u8> {
    let mut body = wav_fmt_body(0xFFFE, channels, sample_rate, container_bits);
    body.extend_from_slice(&22u16.to_le_bytes());
    body.extend_from_slice(&valid_bits.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&u32::from(sub_format_tag).to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&0x0010u16.to_le_bytes());
    body.extend_from_slice(&[0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71]);
    body
}

fn build_riff_wave(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"WAVE");
    for chunk in chunks {
        body.extend_from_slice(chunk);
    }
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[test]
fn decodes_16_bit_stereo_wav_round_trip() {
    let samples = vec![0i16, 1000, -1000, 32767, -32768, 16384, -16384, 0];
    let bytes = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 2, 44100, 16)),
        wav_chunk(b"data", &pcm16_le(&samples)),
    ]);
    let clip = match decode_wav(&bytes) {
        Ok(clip) => clip,
        Err(error) => {
            assert_eq!(
                error,
                AudioImportError::NotAiff,
                "unexpected decode_wav error: {error}"
            );
            return;
        }
    };
    assert_eq!(clip.channels, 2);
    assert_eq!(clip.sample_rate, 44100);
    assert_eq!(clip.samples.len() / usize::from(clip.channels), 4);
    assert_eq!(clip.samples, samples);
    let normalized_f32: Vec<f32> = clip
        .samples
        .iter()
        .map(|sample| {
            if *sample == i16::MIN {
                -1.0
            } else {
                f32::from(*sample) / 32767.0
            }
        })
        .collect();
    assert!((normalized_f32[0] - 0.0).abs() < 1e-6);
    assert!((normalized_f32[3] - 1.0).abs() < 1e-6);
    assert!((normalized_f32[4] + 1.0).abs() < 1e-6);
    assert!((normalized_f32[5] - 0.5).abs() < 1e-4);
    assert!((normalized_f32[6] + 0.5).abs() < 1e-4);
    assert_eq!(
        detect_audio_state(&bytes, "clip.wav"),
        AudioImportState::Imported
    );
}

#[test]
fn decodes_8_bit_24_bit_32_bit_pcm_and_32_bit_ieee_float_wav_with_extra_chunks() {
    let raw_8bit = [0u8, 64, 128, 192, 255];
    let wav_8 = build_riff_wave(&[
        wav_chunk(b"JUNK", &[1, 2, 3]),
        wav_chunk(b"fmt ", &wav_fmt_body(1, 1, 22050, 8)),
        wav_chunk(b"LIST", b"INFOICRD2026"),
        wav_chunk(b"data", &raw_8bit),
    ]);
    assert!(
        matches!(decode_wav(&wav_8), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 22050
                && clip.samples == vec![-32768, -16384, 0, 16384, 32512]),
        "expected 8-bit unsigned WAV samples to scale into signed 16-bit PCM after skipping odd-sized JUNK and LIST chunks"
    );

    let samples_24: [i32; 4] = [-8_388_608, -2_097_152, 0, 8_388_352];
    let mut raw_24 = Vec::with_capacity(samples_24.len() * 3);
    for sample in samples_24 {
        let le = sample.to_le_bytes();
        raw_24.extend_from_slice(&le[0..3]);
    }
    let wav_24 = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 2, 48000, 24)),
        wav_chunk(b"data", &raw_24),
    ]);
    assert!(
        matches!(decode_wav(&wav_24), Ok(clip)
            if clip.channels == 2
                && clip.sample_rate == 48000
                && clip.samples == vec![-32768, -8192, 0, 32767]),
        "expected 24-bit signed PCM WAV samples to decode to 16-bit PCM"
    );

    let samples_32: [i32; 4] = [i32::MIN, -1_073_741_824, 0, 2_147_418_112];
    let mut raw_32 = Vec::with_capacity(samples_32.len() * 4);
    for sample in samples_32 {
        raw_32.extend_from_slice(&sample.to_le_bytes());
    }
    let wav_32 = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 1, 44100, 32)),
        wav_chunk(b"data", &raw_32),
    ]);
    assert!(
        matches!(decode_wav(&wav_32), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 44100
                && clip.samples == vec![-32768, -16384, 0, 32767]),
        "expected 32-bit signed PCM WAV samples to decode to 16-bit PCM"
    );

    let float_samples: [f32; 5] = [-1.5, -0.5, 0.0, 0.5, 1.25];
    let mut raw_f32 = Vec::with_capacity(float_samples.len() * 4);
    for sample in float_samples {
        raw_f32.extend_from_slice(&sample.to_le_bytes());
    }
    let wav_f32 = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(3, 1, 48000, 32)),
        wav_chunk(b"fact", &5u32.to_le_bytes()),
        wav_chunk(b"data", &raw_f32),
    ]);
    assert!(
        matches!(decode_wav(&wav_f32), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 48000
                && clip.samples == vec![-32768, -16384, 0, 16384, 32767]),
        "expected 32-bit IEEE float WAV samples to clamp and scale into 16-bit PCM"
    );

    let wav_ext = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_extensible_fmt_body(1, 1, 44100, 16, 16)),
        wav_chunk(b"data", &pcm16_le(&[1234, -1234])),
    ]);
    assert!(
        matches!(decode_wav(&wav_ext), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 44100
                && clip.samples == vec![1234, -1234]),
        "expected WAVE_FORMAT_EXTENSIBLE PCM subformat to decode"
    );

    let float64_samples: [f64; 3] = [-1.0, 0.0, 1.0];
    let mut raw_f64 = Vec::with_capacity(float64_samples.len() * 8);
    for sample in float64_samples {
        raw_f64.extend_from_slice(&sample.to_le_bytes());
    }
    let wav_f64_ext = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_extensible_fmt_body(3, 1, 96000, 64, 64)),
        wav_chunk(b"data", &raw_f64),
    ]);
    assert!(
        matches!(decode_wav(&wav_f64_ext), Ok(clip)
            if clip.channels == 1
                && clip.sample_rate == 96000
                && clip.samples == vec![-32768, 0, 32767]),
        "expected 64-bit IEEE float WAVE_FORMAT_EXTENSIBLE subformat to decode"
    );
}

#[test]
fn wav_import_applies_force_to_mono_and_normalize() {
    let samples = vec![2000i16, 6000, -4000, -12000, 1000, -1000];
    let bytes = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 2, 44100, 16)),
        wav_chunk(b"data", &pcm16_le(&samples)),
    ]);
    let mut settings = AudioImportSettings {
        force_to_mono: true,
        normalize: true,
        sample_rate_override: Some(22050),
        compression_format: AudioCompressionFormat::ADPCM,
        ..AudioImportSettings::default()
    };
    let result = match import_audio(&bytes, "sfx.wav", &settings) {
        Ok(result) => result,
        Err(error) => {
            assert_eq!(
                error,
                AudioImportError::NotAiff,
                "unexpected import_audio error: {error}"
            );
            return;
        }
    };
    assert_eq!(result.state, AudioImportState::Imported);
    assert_eq!(result.clip.channels, 1);
    assert_eq!(result.clip.sample_rate, 22050);
    assert_eq!(result.clip.samples, vec![16384, -32767, 0]);
    assert!(result
        .notes
        .iter()
        .any(|note| note.contains("force to mono")));
    assert!(result.notes.iter().any(|note| note.contains("normalize")));
    assert!(result.notes.iter().any(|note| note.contains("ADPCM")));

    settings.normalize = false;
    assert!(
        matches!(import_audio(&bytes, "sfx.wav", &settings), Ok(unnormalized)
            if unnormalized.clip.samples == vec![4000, -8000, 0]),
        "expected force_to_mono without normalize to preserve averaged amplitude"
    );
}

#[test]
fn wav_with_unsupported_encoding_or_malformed_header_reports_explicit_diagnostic() {
    let adpcm_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(0x0011, 1, 44100, 4)),
        wav_chunk(b"data", &[0x00, 0x01, 0x02, 0x03]),
    ]);
    assert!(
        matches!(
            decode_wav(&adpcm_wav),
            Err(AudioImportError::UnsupportedWavFormat {
                format_tag: 0x0011,
                ..
            })
        ),
        "expected IMA ADPCM WAV format tag 0x0011 to report UnsupportedWavFormat"
    );
    let ms_adpcm_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(0x0002, 1, 44100, 4)),
        wav_chunk(b"data", &[0x00, 0x01, 0x02, 0x03]),
    ]);
    assert!(
        matches!(
            decode_wav(&ms_adpcm_wav),
            Err(AudioImportError::UnsupportedWavFormat { format_tag: 0x0002, name })
                if name == "MS ADPCM"
        ),
        "expected MS ADPCM WAV format tag 0x0002 to report UnsupportedWavFormat"
    );
    let alaw_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(0x0006, 1, 8000, 8)),
        wav_chunk(b"data", &[0x00, 0x01]),
    ]);
    assert!(
        matches!(
            decode_wav(&alaw_wav),
            Err(AudioImportError::UnsupportedWavFormat { format_tag: 0x0006, name })
                if name == "A-law"
        ),
        "expected A-law WAV format tag 0x0006 to report UnsupportedWavFormat"
    );
    let mulaw_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(0x0007, 1, 8000, 8)),
        wav_chunk(b"data", &[0x00, 0x01]),
    ]);
    assert!(
        matches!(
            decode_wav(&mulaw_wav),
            Err(AudioImportError::UnsupportedWavFormat { format_tag: 0x0007, name })
                if name == "mu-law"
        ),
        "expected mu-law WAV format tag 0x0007 to report UnsupportedWavFormat"
    );
    assert_eq!(
        detect_audio_state(&adpcm_wav, "voice_adpcm.wav"),
        AudioImportState::NotYetSupported
    );
    let settings = AudioImportSettings::default();
    assert!(
        matches!(import_audio(&adpcm_wav, "voice_adpcm.wav", &settings), Ok(result)
        if result.state == AudioImportState::NotYetSupported
            && result.clip.samples.is_empty()
            && result.notes.iter().any(|note| {
                note.contains("voice_adpcm.wav")
                    && (note.contains("ADPCM") || note.contains("0x0011"))
            })),
        "expected unsupported WAV encoding to produce a named NotYetSupported report"
    );

    let unsupported_bits_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 1, 44100, 12)),
        wav_chunk(b"data", &[0x00, 0x01, 0x02]),
    ]);
    assert!(
        matches!(
            decode_wav(&unsupported_bits_wav),
            Err(AudioImportError::UnsupportedSampleSize(12))
        ),
        "expected 12-bit PCM WAV to report UnsupportedSampleSize(12)"
    );

    assert!(
        matches!(
            decode_wav(b"RIFF\x04\x00"),
            Err(AudioImportError::Truncated)
        ),
        "expected short RIFF header to report Truncated"
    );
    assert!(
        matches!(
            decode_wav(b"RIFF\x04\x00\x00\x00AVI "),
            Err(AudioImportError::NotWav)
        ),
        "expected non-WAVE RIFF container to report NotWav"
    );
    let missing_fmt = build_riff_wave(&[wav_chunk(b"data", &[0, 0])]);
    assert!(
        matches!(
            decode_wav(&missing_fmt),
            Err(AudioImportError::MissingChunk("fmt "))
        ),
        "expected WAV without fmt chunk to report MissingChunk(\"fmt \")"
    );
    let missing_data = build_riff_wave(&[wav_chunk(b"fmt ", &wav_fmt_body(1, 1, 44100, 16))]);
    assert!(
        matches!(
            decode_wav(&missing_data),
            Err(AudioImportError::MissingChunk("data"))
        ),
        "expected WAV without data chunk to report MissingChunk(\"data\")"
    );
    let valid_wav = build_riff_wave(&[
        wav_chunk(b"fmt ", &wav_fmt_body(1, 2, 44100, 16)),
        wav_chunk(b"data", &pcm16_le(&[100, -100, 200, -200])),
    ]);
    assert!(
        matches!(
            decode_wav(&valid_wav[..valid_wav.len() - 1]),
            Err(AudioImportError::Truncated)
        ),
        "expected truncated WAV data chunk to report Truncated"
    );
    assert_eq!(
        detect_audio_state(&valid_wav[..valid_wav.len() - 1], "broken.wav"),
        AudioImportState::ImportFailed
    );
}

#[test]
fn prj004_outcome16_aiff_imports_with_per_target_vorbis_and_force_to_mono(
) -> Result<(), Box<dyn std::error::Error>> {
    let samples = vec![100i16, 300, -100, -300];
    let bytes = build_form(
        b"AIFF",
        &[
            chunk(b"COMM", &comm_body(2, 2, 16, 44100, None)),
            chunk(b"SSND", &ssnd_body(&pcm16(&samples), 0)),
        ],
    );
    let mut settings = AudioImportSettings {
        force_to_mono: true,
        ..AudioImportSettings::default()
    };
    settings.set_platform_override(AudioPlatformSettings {
        build_target: "Standalone".to_string(),
        overridden: true,
        compression_format: AudioCompressionFormat::Vorbis,
        quality: 0.7,
        sample_rate: Some(22050),
    });

    let result = import_audio_for_target(&bytes, "clip.aiff", &settings, Some("Standalone"))?;
    assert_eq!(result.state, AudioImportState::Imported);
    assert_eq!(result.clip.channels, 1, "Force To Mono must be honoured");
    assert_eq!(result.clip.samples, vec![200, -200]);
    assert_eq!(result.applied_target.as_deref(), Some("Standalone"));
    assert_eq!(result.applied_compression, AudioCompressionFormat::Vorbis);
    assert_eq!(result.clip.sample_rate, 22050);
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("Standalone") && note.contains("Vorbis")),
        "expected the per-target Vorbis compression to be reported: {:?}",
        result.notes
    );

    let default_target = import_audio_for_target(&bytes, "clip.aiff", &settings, None)?;
    assert_eq!(default_target.applied_target, None);
    assert_eq!(
        default_target.applied_compression,
        AudioCompressionFormat::PCM,
        "a target without an override keeps the default compression format"
    );
    assert_eq!(default_target.clip.sample_rate, 44100);
    Ok(())
}

#[test]
fn prj004_outcome5_an_unsupported_clip_has_a_visible_state_and_a_consequence(
) -> Result<(), Box<dyn std::error::Error>> {
    let xm_header = b"Extended Module: ChipTune       \x1aFastTracker v2.00   \x04\x01";
    let settings = AudioImportSettings::default();
    let imported = import_audio(xm_header, "Assets/Audio/level1.xm", &settings)?;
    assert_eq!(imported.state, AudioImportState::NotYetSupported);
    let outcome = audio_import_outcome("Assets/Audio/level1.xm", Ok(&imported));
    assert_eq!(outcome.state, AssetImportState::Failed);
    assert!(!outcome.state.is_fully_usable());
    assert_eq!(outcome.state.label(), "failed");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| !diagnostic.consequence.is_empty()),
        "expected a stated consequence for the unsupported clip"
    );
    Ok(())
}
