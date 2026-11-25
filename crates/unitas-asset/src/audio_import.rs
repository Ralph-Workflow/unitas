use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioImportState {
    Imported,
    NotYetSupported,
    UnsupportedContainer,
    ImportFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioLoadType {
    DecompressOnLoad,
    CompressedInMemory,
    Streaming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCompressionFormat {
    PCM,
    Vorbis,
    ADPCM,
    MP3,
}

impl fmt::Display for AudioCompressionFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            AudioCompressionFormat::PCM => "PCM",
            AudioCompressionFormat::Vorbis => "Vorbis",
            AudioCompressionFormat::ADPCM => "ADPCM",
            AudioCompressionFormat::MP3 => "MP3",
        };
        formatter.write_str(name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioImportSettings {
    pub force_to_mono: bool,
    pub load_type: AudioLoadType,
    pub compression_format: AudioCompressionFormat,
    pub preload_audio_data: bool,
    pub sample_rate_override: Option<u32>,
    pub quality: f32,
}

impl Default for AudioImportSettings {
    fn default() -> Self {
        Self {
            force_to_mono: false,
            load_type: AudioLoadType::DecompressOnLoad,
            compression_format: AudioCompressionFormat::PCM,
            preload_audio_data: true,
            sample_rate_override: None,
            quality: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcmClip {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioImportResult {
    pub clip: PcmClip,
    pub state: AudioImportState,
    pub settings: AudioImportSettings,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioImportError {
    NotAiff,
    MissingChunk(&'static str),
    UnsupportedSampleSize(u16),
    UnsupportedCompression(String),
    Truncated,
    NotYetSupported(&'static str),
    InvalidSampleRate,
}

impl fmt::Display for AudioImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioImportError::NotAiff => formatter.write_str("file is not an AIFF or AIFF-C container"),
            AudioImportError::MissingChunk(name) => write!(formatter, "missing {name} chunk"),
            AudioImportError::UnsupportedSampleSize(bits) => {
                write!(formatter, "unsupported sample size {bits} bits")
            }
            AudioImportError::UnsupportedCompression(name) => {
                write!(formatter, "unsupported AIFC compression type {name}")
            }
            AudioImportError::Truncated => formatter.write_str("audio data is truncated"),
            AudioImportError::NotYetSupported(reason) => write!(formatter, "{reason}"),
            AudioImportError::InvalidSampleRate => formatter.write_str("invalid sample rate"),
        }
    }
}

impl std::error::Error for AudioImportError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Compression {
    BigEndian,
    LittleEndian,
}

struct Comm {
    channels: u16,
    frames: u32,
    sample_size: u16,
    sample_rate: u32,
    compression: Compression,
}

struct Ssnd<'a> {
    data: &'a [u8],
}

pub fn decode_aiff(bytes: &[u8]) -> Result<PcmClip, AudioImportError> {
    if bytes.len() < 12 {
        return Err(AudioImportError::Truncated);
    }
    if &bytes[0..4] != b"FORM" {
        return Err(AudioImportError::NotAiff);
    }
    let aifc = match &bytes[8..12] {
        b"AIFF" => false,
        b"AIFC" => true,
        _ => return Err(AudioImportError::NotAiff),
    };

    let mut comm: Option<Comm> = None;
    let mut ssnd: Option<Ssnd<'_>> = None;
    let mut position = 12usize;
    while position + 8 <= bytes.len() {
        let id = &bytes[position..position + 4];
        let size = read_u32(&bytes[position + 4..position + 8]) as usize;
        let body_start = position + 8;
        let body_end = body_start
            .checked_add(size)
            .ok_or(AudioImportError::Truncated)?;
        if body_end > bytes.len() {
            return Err(AudioImportError::Truncated);
        }
        let body = &bytes[body_start..body_end];
        match id {
            b"COMM" => comm = Some(parse_comm(body, aifc)?),
            b"SSND" => ssnd = Some(parse_ssnd(body)?),
            _ => {}
        }
        position = body_end + (size & 1);
    }

    let comm = comm.ok_or(AudioImportError::MissingChunk("COMM"))?;
    let ssnd = ssnd.ok_or(AudioImportError::MissingChunk("SSND"))?;
    decode_samples(&comm, &ssnd)
}

fn parse_comm(body: &[u8], aifc: bool) -> Result<Comm, AudioImportError> {
    if body.len() < 18 {
        return Err(AudioImportError::Truncated);
    }
    let channels = read_u16(&body[0..2]);
    let frames = read_u32(&body[2..6]);
    let sample_size = read_u16(&body[6..8]);
    let sample_rate = decode_extended80(&body[8..18])?;
    let compression = if aifc {
        if body.len() < 22 {
            return Err(AudioImportError::Truncated);
        }
        match &body[18..22] {
            b"NONE" | b"twos" => Compression::BigEndian,
            b"sowt" => Compression::LittleEndian,
            other => {
                return Err(AudioImportError::UnsupportedCompression(
                    String::from_utf8_lossy(other).into_owned(),
                ))
            }
        }
    } else {
        Compression::BigEndian
    };
    Ok(Comm {
        channels,
        frames,
        sample_size,
        sample_rate,
        compression,
    })
}

fn parse_ssnd(body: &[u8]) -> Result<Ssnd<'_>, AudioImportError> {
    if body.len() < 8 {
        return Err(AudioImportError::Truncated);
    }
    let offset = read_u32(&body[0..4]) as usize;
    if body.len() < 8 + offset {
        return Err(AudioImportError::Truncated);
    }
    Ok(Ssnd {
        data: &body[8 + offset..],
    })
}

fn decode_samples(comm: &Comm, ssnd: &Ssnd<'_>) -> Result<PcmClip, AudioImportError> {
    let bytes_per_sample = match comm.sample_size {
        8 => 1usize,
        16 => 2usize,
        other => return Err(AudioImportError::UnsupportedSampleSize(other)),
    };
    if comm.channels == 0 {
        return Err(AudioImportError::Truncated);
    }
    let frame_count = comm.frames as usize;
    let channel_count = comm.channels as usize;
    let required = frame_count
        .checked_mul(channel_count)
        .and_then(|count| count.checked_mul(bytes_per_sample))
        .ok_or(AudioImportError::Truncated)?;
    if ssnd.data.len() < required {
        return Err(AudioImportError::Truncated);
    }

    let mut samples = Vec::with_capacity(frame_count * channel_count);
    let mut at = 0usize;
    for _ in 0..frame_count {
        for _ in 0..channel_count {
            let value = match (comm.sample_size, comm.compression) {
                (8, _) => i16::from(ssnd.data[at]) - 128,
                (16, Compression::BigEndian) => {
                    i16::from_be_bytes([ssnd.data[at], ssnd.data[at + 1]])
                }
                (16, Compression::LittleEndian) => {
                    i16::from_le_bytes([ssnd.data[at], ssnd.data[at + 1]])
                }
                (size, _) => return Err(AudioImportError::UnsupportedSampleSize(size)),
            };
            samples.push(value);
            at += bytes_per_sample;
        }
    }

    Ok(PcmClip {
        channels: comm.channels,
        sample_rate: comm.sample_rate,
        samples,
    })
}

fn decode_extended80(bytes: &[u8]) -> Result<u32, AudioImportError> {
    if bytes.len() < 10 {
        return Err(AudioImportError::Truncated);
    }
    if bytes[0] & 0x80 != 0 {
        return Err(AudioImportError::InvalidSampleRate);
    }
    let exponent = u16::from_be_bytes([bytes[0] & 0x7f, bytes[1]]);
    let mut significand = [0u8; 8];
    significand.copy_from_slice(&bytes[2..10]);
    let mantissa = u64::from_be_bytes(significand);
    if exponent == 0 || mantissa == 0 {
        return Err(AudioImportError::InvalidSampleRate);
    }

    let shift = i32::from(exponent) - 16383 - 63;
    let value = if shift >= 0 {
        if shift >= 64 {
            return Err(AudioImportError::InvalidSampleRate);
        }
        if shift > 0 && (mantissa >> (64 - shift)) != 0 {
            return Err(AudioImportError::InvalidSampleRate);
        }
        mantissa << shift
    } else {
        let dropped = u32::try_from(-shift).map_err(|_| AudioImportError::InvalidSampleRate)?;
        if dropped >= 64 {
            return Err(AudioImportError::InvalidSampleRate);
        }
        if mantissa & ((1u64 << dropped) - 1) != 0 {
            return Err(AudioImportError::InvalidSampleRate);
        }
        mantissa >> dropped
    };

    if value == 0 || value > u64::from(u32::MAX) {
        return Err(AudioImportError::InvalidSampleRate);
    }
    Ok(value as u32)
}

pub fn detect_audio_state(bytes: &[u8], file_name: &str) -> AudioImportState {
    if is_aiff(bytes) {
        return match decode_aiff(bytes) {
            Ok(_) => AudioImportState::Imported,
            Err(_) => AudioImportState::ImportFailed,
        };
    }
    match recognized_unsupported(bytes, file_name) {
        Some(_) => AudioImportState::NotYetSupported,
        None => AudioImportState::UnsupportedContainer,
    }
}

pub fn import_audio(
    bytes: &[u8],
    file_name: &str,
    settings: &AudioImportSettings,
) -> Result<AudioImportResult, AudioImportError> {
    let state = detect_audio_state(bytes, file_name);
    if state == AudioImportState::NotYetSupported {
        let reason = unsupported_reason(bytes, file_name);
        return Ok(AudioImportResult {
            clip: PcmClip {
                channels: 0,
                sample_rate: 0,
                samples: Vec::new(),
            },
            state,
            settings: settings.clone(),
            notes: vec![format!("{file_name}: {reason}")],
        });
    }
    if state == AudioImportState::UnsupportedContainer {
        return Err(AudioImportError::NotAiff);
    }

    let clip = decode_aiff(bytes)?;
    let mut notes = Vec::new();
    let mut clip = clip;

    if settings.force_to_mono && clip.channels > 1 {
        let channels = usize::from(clip.channels);
        let mut mono = Vec::with_capacity(clip.samples.len() / channels);
        for frame in clip.samples.chunks_exact(channels) {
            let sum: i64 = frame.iter().map(|value| i64::from(*value)).sum();
            let average = (sum as f64 / channels as f64).round() as i16;
            mono.push(average);
        }
        notes.push(format!(
            "force to mono applied: {} channels averaged into 1",
            clip.channels
        ));
        clip.channels = 1;
        clip.samples = mono;
    }

    if let Some(rate) = settings.sample_rate_override {
        if rate != clip.sample_rate {
            notes.push(format!(
                "sample rate override applied: {} -> {} (samples not resampled)",
                clip.sample_rate, rate
            ));
        }
        clip.sample_rate = rate;
    }

    if settings.compression_format != AudioCompressionFormat::PCM {
        notes.push(format!(
            "{} compression recorded as an import setting; decoded samples remain PCM",
            settings.compression_format
        ));
    }

    Ok(AudioImportResult {
        clip,
        state: AudioImportState::Imported,
        settings: settings.clone(),
        notes,
    })
}

fn is_aiff(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"FORM" && (&bytes[8..12] == b"AIFF" || &bytes[8..12] == b"AIFC")
}

fn is_ogg(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"OggS"
}

fn is_mp3(bytes: &[u8]) -> bool {
    bytes.len() >= 3
        && (&bytes[0..3] == b"ID3"
            || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

fn extension_of(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => "",
    }
}

fn recognized_unsupported(bytes: &[u8], file_name: &str) -> Option<&'static str> {
    let extension = extension_of(file_name).to_ascii_lowercase();
    match extension.as_str() {
        "ogg" | "oga" | "ogv" | "opus" => Some("OGG Vorbis import is not yet supported"),
        "mp3" | "mp2" => Some("MP3 import is not yet supported"),
        "mod" | "it" | "s3m" | "xm" => Some("tracker module import is not yet supported"),
        "wav" | "wave" => Some("WAV import is not yet supported by the AIFF decoder"),
        _ => {
            if is_ogg(bytes) {
                Some("OGG Vorbis import is not yet supported")
            } else if is_mp3(bytes) {
                Some("MP3 import is not yet supported")
            } else if is_wav(bytes) {
                Some("WAV import is not yet supported by the AIFF decoder")
            } else {
                None
            }
        }
    }
}

fn unsupported_reason(bytes: &[u8], file_name: &str) -> &'static str {
    recognized_unsupported(bytes, file_name)
        .unwrap_or("audio container is not yet supported")
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
