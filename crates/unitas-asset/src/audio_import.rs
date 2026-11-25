use std::fmt;
use std::io::Cursor;

use lewton::header::HeaderReadError;
use lewton::inside_ogg::OggStreamReader;
use lewton::{OggReadError, VorbisError};
use minimp3::{Decoder as Mp3Decoder, Error as Mp3Error};

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
    pub normalize: bool,
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
            normalize: false,
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
    NotWav,
    NotOgg,
    NotMp3,
    CorruptOgg(String),
    CorruptMp3(String),
    MissingChunk(&'static str),
    UnsupportedSampleSize(u16),
    UnsupportedCompression(String),
    UnsupportedWavFormat { format_tag: u16, name: &'static str },
    Truncated,
    NotYetSupported(&'static str),
    InvalidSampleRate,
}

impl fmt::Display for AudioImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioImportError::NotAiff => formatter.write_str("file is not an AIFF or AIFF-C container"),
            AudioImportError::NotWav => formatter.write_str("file is not a RIFF WAVE container"),
            AudioImportError::NotOgg => formatter.write_str("file is not an OGG container"),
            AudioImportError::NotMp3 => formatter.write_str("file is not an MP3 stream"),
            AudioImportError::CorruptOgg(detail) => {
                write!(formatter, "corrupt OGG Vorbis stream: {detail}")
            }
            AudioImportError::CorruptMp3(detail) => {
                write!(formatter, "corrupt MP3 stream: {detail}")
            }
            AudioImportError::MissingChunk(name) => write!(formatter, "missing {name} chunk"),
            AudioImportError::UnsupportedSampleSize(bits) => {
                write!(formatter, "unsupported sample size {bits} bits")
            }
            AudioImportError::UnsupportedCompression(name) => {
                write!(formatter, "unsupported AIFC compression type {name}")
            }
            AudioImportError::UnsupportedWavFormat { format_tag, name } => {
                write!(
                    formatter,
                    "unsupported WAV encoding {name} (format tag 0x{format_tag:04X})"
                )
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

pub fn decode_ogg_vorbis(bytes: &[u8]) -> Result<PcmClip, AudioImportError> {
    if bytes.len() < 4 {
        return Err(AudioImportError::Truncated);
    }
    if !is_ogg(bytes) {
        return Err(AudioImportError::NotOgg);
    }
    if bytes.len() < 27 {
        return Err(AudioImportError::Truncated);
    }

    let mut reader = OggStreamReader::new(Cursor::new(bytes)).map_err(map_vorbis_error)?;
    let channels = u16::from(reader.ident_hdr.audio_channels);
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if channels == 0 {
        return Err(AudioImportError::CorruptOgg(
            "zero audio channels".to_string(),
        ));
    }
    if sample_rate == 0 {
        return Err(AudioImportError::InvalidSampleRate);
    }

    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(map_vorbis_error)? {
        samples.extend_from_slice(&packet);
    }

    Ok(PcmClip {
        channels,
        sample_rate,
        samples,
    })
}

fn map_vorbis_error(error: VorbisError) -> AudioImportError {
    match error {
        VorbisError::BadHeader(HeaderReadError::NotVorbisHeader) => {
            AudioImportError::NotYetSupported("non-Vorbis OGG codec is not yet supported")
        }
        VorbisError::BadHeader(HeaderReadError::EndOfPacket) => AudioImportError::Truncated,
        VorbisError::OggError(OggReadError::ReadError(ref io_err))
            if io_err.kind() == std::io::ErrorKind::UnexpectedEof =>
        {
            AudioImportError::Truncated
        }
        other => AudioImportError::CorruptOgg(other.to_string()),
    }
}

pub fn decode_mp3(bytes: &[u8]) -> Result<PcmClip, AudioImportError> {
    if bytes.len() < 3 {
        return Err(AudioImportError::Truncated);
    }
    if !is_mp3(bytes) {
        return Err(AudioImportError::NotMp3);
    }
    if bytes.starts_with(b"ID3") {
        if bytes.len() < 10 {
            return Err(AudioImportError::Truncated);
        }
        let tag_size = ((usize::from(bytes[6] & 0x7f)) << 21)
            | ((usize::from(bytes[7] & 0x7f)) << 14)
            | ((usize::from(bytes[8] & 0x7f)) << 7)
            | usize::from(bytes[9] & 0x7f);
        let footer_size = if bytes[5] & 0x10 != 0 { 10usize } else { 0usize };
        let header_end = 10usize
            .checked_add(tag_size)
            .and_then(|end| end.checked_add(footer_size))
            .ok_or(AudioImportError::Truncated)?;
        if bytes.len() <= header_end {
            return Err(AudioImportError::Truncated);
        }
    } else if bytes.len() < 4 {
        return Err(AudioImportError::Truncated);
    }

    let mut decoder = Mp3Decoder::new(Cursor::new(bytes));
    let mut channels: Option<u16> = None;
    let mut sample_rate: Option<u32> = None;
    let mut samples = Vec::new();

    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                let frame_channels = u16::try_from(frame.channels).map_err(|_| {
                    AudioImportError::CorruptMp3("invalid channel count".to_string())
                })?;
                if frame_channels == 0 {
                    return Err(AudioImportError::CorruptMp3(
                        "zero audio channels".to_string(),
                    ));
                }
                let frame_rate =
                    u32::try_from(frame.sample_rate).map_err(|_| AudioImportError::InvalidSampleRate)?;
                if frame_rate == 0 {
                    return Err(AudioImportError::InvalidSampleRate);
                }
                if channels.is_none() {
                    channels = Some(frame_channels);
                }
                if sample_rate.is_none() {
                    sample_rate = Some(frame_rate);
                }
                samples.extend_from_slice(&frame.data);
            }
            Err(Mp3Error::Eof) => break,
            Err(Mp3Error::InsufficientData) => return Err(AudioImportError::Truncated),
            Err(other) => return Err(AudioImportError::CorruptMp3(other.to_string())),
        }
    }

    let Some(channels) = channels else {
        return Err(AudioImportError::CorruptMp3(
            "no valid MPEG audio frames decoded".to_string(),
        ));
    };
    let Some(sample_rate) = sample_rate else {
        return Err(AudioImportError::InvalidSampleRate);
    };
    if samples.is_empty() {
        return Err(AudioImportError::CorruptMp3(
            "decoded MP3 stream produced zero samples".to_string(),
        ));
    }

    Ok(PcmClip {
        channels,
        sample_rate,
        samples,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WavEncoding {
    Pcm,
    IeeeFloat,
}

struct WavFmt {
    encoding: WavEncoding,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
}

pub fn decode_wav(bytes: &[u8]) -> Result<PcmClip, AudioImportError> {
    if bytes.len() < 12 {
        return Err(AudioImportError::Truncated);
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioImportError::NotWav);
    }

    let mut fmt_chunk: Option<WavFmt> = None;
    let mut data_chunk: Option<&[u8]> = None;
    let mut position = 12usize;
    while position + 8 <= bytes.len() {
        let id = &bytes[position..position + 4];
        let size = read_u32_le(&bytes[position + 4..position + 8]) as usize;
        let body_start = position + 8;
        let body_end = body_start
            .checked_add(size)
            .ok_or(AudioImportError::Truncated)?;
        if body_end > bytes.len() {
            return Err(AudioImportError::Truncated);
        }
        let body = &bytes[body_start..body_end];
        match id {
            b"fmt " => fmt_chunk = Some(parse_wav_fmt(body)?),
            b"data" => data_chunk = Some(body),
            _ => {}
        }
        position = body_end + (size & 1);
    }
    if position < bytes.len() {
        return Err(AudioImportError::Truncated);
    }

    let fmt_chunk = fmt_chunk.ok_or(AudioImportError::MissingChunk("fmt "))?;
    let data_chunk = data_chunk.ok_or(AudioImportError::MissingChunk("data"))?;
    decode_wav_samples(&fmt_chunk, data_chunk)
}

fn parse_wav_fmt(body: &[u8]) -> Result<WavFmt, AudioImportError> {
    if body.len() < 16 {
        return Err(AudioImportError::Truncated);
    }
    let raw_format_tag = read_u16_le(&body[0..2]);
    let channels = read_u16_le(&body[2..4]);
    let sample_rate = read_u32_le(&body[4..8]);
    let container_bits = read_u16_le(&body[14..16]);
    if channels == 0 {
        return Err(AudioImportError::Truncated);
    }
    if sample_rate == 0 {
        return Err(AudioImportError::InvalidSampleRate);
    }
    let (format_tag, bits_per_sample) = if raw_format_tag == 0xFFFE {
        if body.len() < 40 {
            return Err(AudioImportError::Truncated);
        }
        let valid_bits = read_u16_le(&body[18..20]);
        if valid_bits != 0 && valid_bits != container_bits {
            return Err(AudioImportError::UnsupportedSampleSize(valid_bits));
        }
        (read_u16_le(&body[24..26]), container_bits)
    } else {
        (raw_format_tag, container_bits)
    };
    let encoding = match format_tag {
        0x0001 => WavEncoding::Pcm,
        0x0003 => WavEncoding::IeeeFloat,
        other => {
            return Err(AudioImportError::UnsupportedWavFormat {
                format_tag: other,
                name: wav_format_name(other),
            })
        }
    };
    Ok(WavFmt {
        encoding,
        channels,
        sample_rate,
        bits_per_sample,
    })
}

fn wav_format_name(format_tag: u16) -> &'static str {
    match format_tag {
        0x0002 => "MS ADPCM",
        0x0006 => "A-law",
        0x0007 => "mu-law",
        0x0011 => "IMA ADPCM",
        0x0055 => "MPEG Layer 3",
        _ => "compressed WAV codec",
    }
}

fn decode_wav_samples(fmt_chunk: &WavFmt, data: &[u8]) -> Result<PcmClip, AudioImportError> {
    let bytes_per_sample = match (fmt_chunk.encoding, fmt_chunk.bits_per_sample) {
        (WavEncoding::Pcm, 8) => 1usize,
        (WavEncoding::Pcm, 16) => 2usize,
        (WavEncoding::Pcm, 24) => 3usize,
        (WavEncoding::Pcm, 32) | (WavEncoding::IeeeFloat, 32) => 4usize,
        (WavEncoding::IeeeFloat, 64) => 8usize,
        (_, other) => return Err(AudioImportError::UnsupportedSampleSize(other)),
    };
    let channel_count = usize::from(fmt_chunk.channels);
    let frame_bytes = channel_count
        .checked_mul(bytes_per_sample)
        .ok_or(AudioImportError::Truncated)?;
    if frame_bytes == 0 || !data.len().is_multiple_of(frame_bytes) {
        return Err(AudioImportError::Truncated);
    }
    let sample_count = data.len() / bytes_per_sample;
    let mut samples = Vec::with_capacity(sample_count);
    for chunk in data.chunks_exact(bytes_per_sample) {
        let sample = match (fmt_chunk.encoding, fmt_chunk.bits_per_sample) {
            (WavEncoding::Pcm, 8) => (i16::from(chunk[0]) - 128) * 256,
            (WavEncoding::Pcm, 16) => i16::from_le_bytes([chunk[0], chunk[1]]),
            (WavEncoding::Pcm, 24) => {
                let signed =
                    ((i32::from(chunk[2]) << 24) | (i32::from(chunk[1]) << 16) | (i32::from(chunk[0]) << 8))
                        >> 8;
                (signed >> 8) as i16
            }
            (WavEncoding::Pcm, 32) => {
                let signed = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                (signed >> 16) as i16
            }
            (WavEncoding::IeeeFloat, 32) => {
                let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                float_to_i16(f64::from(value))
            }
            (WavEncoding::IeeeFloat, 64) => {
                let value = f64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]);
                float_to_i16(value)
            }
            (_, other) => return Err(AudioImportError::UnsupportedSampleSize(other)),
        };
        samples.push(sample);
    }
    Ok(PcmClip {
        channels: fmt_chunk.channels,
        sample_rate: fmt_chunk.sample_rate,
        samples,
    })
}

fn float_to_i16(value: f64) -> i16 {
    if !value.is_finite() {
        return 0;
    }
    let clamped = value.clamp(-1.0, 1.0);
    if clamped <= -1.0 {
        i16::MIN
    } else {
        (clamped * 32767.0).round() as i16
    }
}

pub fn detect_audio_state(bytes: &[u8], file_name: &str) -> AudioImportState {
    if recognized_unsupported(bytes, file_name).is_some() {
        return AudioImportState::NotYetSupported;
    }
    let extension = extension_of(file_name).to_ascii_lowercase();
    if is_aiff(bytes) || matches!(extension.as_str(), "aif" | "aiff" | "aifc") {
        return match decode_aiff(bytes) {
            Ok(_) => AudioImportState::Imported,
            Err(_) => AudioImportState::ImportFailed,
        };
    }
    if is_wav(bytes) || matches!(extension.as_str(), "wav" | "wave") {
        return match decode_wav(bytes) {
            Ok(_) => AudioImportState::Imported,
            Err(AudioImportError::UnsupportedWavFormat { .. })
            | Err(AudioImportError::NotYetSupported(_)) => AudioImportState::NotYetSupported,
            Err(_) => AudioImportState::ImportFailed,
        };
    }
    if is_ogg(bytes) || extension == "ogg" {
        return match decode_ogg_vorbis(bytes) {
            Ok(_) => AudioImportState::Imported,
            Err(AudioImportError::NotYetSupported(_)) => AudioImportState::NotYetSupported,
            Err(_) => AudioImportState::ImportFailed,
        };
    }
    if is_mp3(bytes) || extension == "mp3" {
        return match decode_mp3(bytes) {
            Ok(_) => AudioImportState::Imported,
            Err(_) => AudioImportState::ImportFailed,
        };
    }
    AudioImportState::UnsupportedContainer
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

    let extension = extension_of(file_name).to_ascii_lowercase();
    let mut clip = if is_aiff(bytes) || matches!(extension.as_str(), "aif" | "aiff" | "aifc") {
        decode_aiff(bytes)?
    } else if is_wav(bytes) || matches!(extension.as_str(), "wav" | "wave") {
        decode_wav(bytes)?
    } else if is_ogg(bytes) || extension == "ogg" {
        decode_ogg_vorbis(bytes)?
    } else if is_mp3(bytes) || extension == "mp3" {
        decode_mp3(bytes)?
    } else {
        return Err(AudioImportError::NotAiff);
    };
    let mut notes = Vec::new();
    apply_import_transforms(&mut clip, settings, &mut notes);

    Ok(AudioImportResult {
        clip,
        state: AudioImportState::Imported,
        settings: settings.clone(),
        notes,
    })
}

fn apply_import_transforms(
    clip: &mut PcmClip,
    settings: &AudioImportSettings,
    notes: &mut Vec<String>,
) {
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

    if settings.normalize && !clip.samples.is_empty() {
        let peak = clip
            .samples
            .iter()
            .map(|sample| i32::from(*sample).abs())
            .max()
            .unwrap_or(0);
        if peak > 0 && peak < 32767 {
            let scale = 32767.0 / f64::from(peak);
            for sample in &mut clip.samples {
                let scaled = (f64::from(*sample) * scale).round();
                *sample = scaled.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
            }
            notes.push(format!("normalize applied: peak {peak} scaled to 32767"));
        }
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
}

fn is_aiff(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"FORM" && (&bytes[8..12] == b"AIFF" || &bytes[8..12] == b"AIFC")
}

fn is_ogg(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"OggS"
}

fn is_mp3(bytes: &[u8]) -> bool {
    (bytes.len() >= 3 && &bytes[0..3] == b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

fn is_xm(bytes: &[u8]) -> bool {
    bytes.starts_with(b"Extended Module:")
}

fn extension_of(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => "",
    }
}

fn recognized_unsupported(bytes: &[u8], file_name: &str) -> Option<&'static str> {
    if is_xm(bytes) {
        return Some("xm tracker module format is Unsupported (not yet supported)");
    }
    let extension = extension_of(file_name).to_ascii_lowercase();
    match extension.as_str() {
        "oga" | "ogv" | "opus" => Some("non-Vorbis OGG codec is not yet supported"),
        "mp2" => Some("MP2 import is not yet supported"),
        "xm" => Some("xm tracker module format is Unsupported (not yet supported)"),
        "mod" | "it" | "s3m" => Some("tracker module import is not yet supported"),
        _ => None,
    }
}

fn unsupported_reason(bytes: &[u8], file_name: &str) -> String {
    if let Some(reason) = recognized_unsupported(bytes, file_name) {
        return reason.to_string();
    }
    if is_wav(bytes) {
        if let Err(error @ AudioImportError::UnsupportedWavFormat { .. }) = decode_wav(bytes) {
            return error.to_string();
        }
    }
    if is_ogg(bytes) {
        if let Err(AudioImportError::NotYetSupported(reason)) = decode_ogg_vorbis(bytes) {
            return reason.to_string();
        }
    }
    "audio container is not yet supported".to_string()
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
