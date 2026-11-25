use unitas_asset::audio_import::{
    decode_aiff, detect_audio_state, import_audio, AudioCompressionFormat, AudioImportError,
    AudioImportSettings, AudioImportState, AudioLoadType,
};

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
        matches!(decode_aiff(&bytes[..bytes.len() - 2]), Err(AudioImportError::Truncated)),
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
    let ogg = b"OggS-not-a-real-stream";
    let mp3 = b"ID3-not-a-real-frame";
    assert!(
        detect_audio_state(ogg, "music.ogg") == AudioImportState::NotYetSupported,
        "expected OGG to be reported as not yet supported"
    );
    assert!(
        detect_audio_state(mp3, "music.mp3") == AudioImportState::NotYetSupported,
        "expected MP3 to be reported as not yet supported"
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
fn detects_unsupported_container() {
    assert!(
        detect_audio_state(b"not-audio-at-all", "mystery.bin") == AudioImportState::UnsupportedContainer,
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
