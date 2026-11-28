use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::dependencies::{
    ImportCandidate, ImportDiagnostic, ImportFingerprint, ImportOutcome, ImportSession,
    ImportSessionReport,
};
use crate::import_cache::{ImportCache, ImportCacheError};
use crate::json::{self, JsonValue};
use crate::meta::guid_from_meta_text;

pub const VIDEO_IMPORTER_SECTION: &str = "VideoClipImporter:";
pub const COOKED_VIDEO_DIR_NAME: &str = "Cooked";
pub const COOKED_VIDEO_MANIFEST_FILENAME: &str = "video-manifest.json";
pub const COOKED_VIDEO_MAGIC: [u8; 4] = *b"UVID";
pub const COOKED_VIDEO_FORMAT_VERSION: u32 = 1;
pub const COOKED_VIDEO_IMPORTER: &str = "unitas-video-cook-v1";
pub const COOKED_VIDEO_EXTENSION: &str = "uvid";
pub const COOKED_VIDEO_MANIFEST_FORMAT: &str = "unitas-cooked-video-manifest";
pub const COOKED_VIDEO_MANIFEST_FORMAT_VERSION: u32 = 1;
pub const VIDEO_IMPORTER_VERSION: &str = COOKED_VIDEO_IMPORTER;

const COOKED_VIDEO_HEADER_BYTES: usize = 52;
const BUNDLE_MANIFEST_FILENAME: &str = "bundle.manifest";

static VIDEO_COOK_TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static VIDEO_COOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_video_cook() -> std::sync::MutexGuard<'static, ()> {
    VIDEO_COOK_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoContainerFormat {
    Mp4,
    Mov,
    WebM,
    Matroska,
    Ogg,
    Ivf,
}

impl VideoContainerFormat {
    pub const fn code(self) -> u32 {
        match self {
            Self::Mp4 => 1,
            Self::Mov => 2,
            Self::WebM => 3,
            Self::Matroska => 4,
            Self::Ogg => 5,
            Self::Ivf => 6,
        }
    }

    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::Mp4),
            2 => Some(Self::Mov),
            3 => Some(Self::WebM),
            4 => Some(Self::Matroska),
            5 => Some(Self::Ogg),
            6 => Some(Self::Ivf),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::WebM => "webm",
            Self::Matroska => "mkv",
            Self::Ogg => "ogg",
            Self::Ivf => "ivf",
        }
    }
}

impl fmt::Display for VideoContainerFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoImportSettings {
    pub keep_alpha: bool,
    pub deinterlace: u32,
    pub flip_horizontally: bool,
    pub flip_vertically: bool,
    pub import_audio: bool,
}

impl Default for VideoImportSettings {
    fn default() -> Self {
        Self {
            keep_alpha: false,
            deinterlace: 0,
            flip_horizontally: false,
            flip_vertically: false,
            import_audio: true,
        }
    }
}

impl VideoImportSettings {
    pub fn from_meta_text(text: &str) -> Result<Self, VideoImportError> {
        let mut settings = Self::default();
        if !text
            .lines()
            .any(|line| line.trim() == VIDEO_IMPORTER_SECTION)
        {
            return Ok(settings);
        }
        let mut inside = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.len() == trimmed.len() {
                inside = trimmed == VIDEO_IMPORTER_SECTION;
                continue;
            }
            if !inside {
                continue;
            }
            let Some((key, raw_value)) = trimmed.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = raw_value.trim();
            match key {
                "keepAlpha" => settings.keep_alpha = parse_meta_bool("keepAlpha", value)?,
                "deinterlace" => {
                    settings.deinterlace = value.parse::<u32>().map_err(|_| {
                        VideoImportError::InvalidMetaText("deinterlace is not a number".to_string())
                    })?;
                }
                "flipHorizontally" => {
                    settings.flip_horizontally = parse_meta_bool("flipHorizontally", value)?;
                }
                "flipVertically" => {
                    settings.flip_vertically = parse_meta_bool("flipVertically", value)?;
                }
                "importAudio" => settings.import_audio = parse_meta_bool("importAudio", value)?,
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn to_meta_text(&self) -> String {
        let mut text = String::from("VideoClipImporter:\n");
        text.push_str(&format!("  keepAlpha: {}\n", u8::from(self.keep_alpha)));
        text.push_str(&format!("  deinterlace: {}\n", self.deinterlace));
        text.push_str(&format!(
            "  flipHorizontally: {}\n",
            u8::from(self.flip_horizontally)
        ));
        text.push_str(&format!(
            "  flipVertically: {}\n",
            u8::from(self.flip_vertically)
        ));
        text.push_str(&format!("  importAudio: {}\n", u8::from(self.import_audio)));
        text
    }
}

fn parse_meta_bool(key: &str, value: &str) -> Result<bool, VideoImportError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(VideoImportError::InvalidMetaText(format!(
            "{key} is {other}"
        ))),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoImportError {
    Truncated,
    UnsupportedContainer(String),
    CorruptContainer(String),
    InvalidDimensions { width: u32, height: u32 },
    InvalidFrameRate { num: u32, den: u32 },
    InvalidCookedSignature,
    UnsupportedCookedVersion(u32),
    InvalidMetaText(String),
}

impl fmt::Display for VideoImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("video data is truncated"),
            Self::UnsupportedContainer(detail) => {
                write!(formatter, "unsupported video container: {detail}")
            }
            Self::CorruptContainer(detail) => {
                write!(formatter, "corrupt video container: {detail}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(formatter, "invalid video dimensions {width}x{height}")
            }
            Self::InvalidFrameRate { num, den } => {
                write!(formatter, "invalid video frame rate {num}/{den}")
            }
            Self::InvalidCookedSignature => {
                formatter.write_str("invalid cooked video signature (expected UVID)")
            }
            Self::UnsupportedCookedVersion(version) => {
                write!(
                    formatter,
                    "unsupported cooked video format version {version}"
                )
            }
            Self::InvalidMetaText(detail) => {
                write!(formatter, "invalid video importer meta: {detail}")
            }
        }
    }
}

impl std::error::Error for VideoImportError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoStreamMetadata {
    pub container: VideoContainerFormat,
    pub codec_tag: [u8; 4],
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub frame_count: u64,
    pub duration_micros: u64,
}

impl VideoStreamMetadata {
    pub fn frame_rate(&self) -> f64 {
        if self.fps_den == 0 {
            0.0
        } else {
            f64::from(self.fps_num) / f64::from(self.fps_den)
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        self.duration_micros as f64 / 1_000_000.0
    }

    pub fn codec_name(&self) -> String {
        String::from_utf8_lossy(&self.codec_tag).trim().to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoImportResult {
    pub metadata: VideoStreamMetadata,
    pub settings: VideoImportSettings,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookedVideo {
    pub format_version: u32,
    pub container: VideoContainerFormat,
    pub codec_tag: [u8; 4],
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub frame_count: u64,
    pub duration_micros: u64,
    pub payload: Vec<u8>,
}

impl CookedVideo {
    pub fn from_import(result: &VideoImportResult) -> Self {
        Self {
            format_version: COOKED_VIDEO_FORMAT_VERSION,
            container: result.metadata.container,
            codec_tag: result.metadata.codec_tag,
            width: result.metadata.width,
            height: result.metadata.height,
            fps_num: result.metadata.fps_num,
            fps_den: result.metadata.fps_den,
            frame_count: result.metadata.frame_count,
            duration_micros: result.metadata.duration_micros,
            payload: result.payload.clone(),
        }
    }

    pub fn frame_rate(&self) -> f64 {
        if self.fps_den == 0 {
            0.0
        } else {
            f64::from(self.fps_num) / f64::from(self.fps_den)
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        self.duration_micros as f64 / 1_000_000.0
    }

    pub fn codec_name(&self) -> String {
        String::from_utf8_lossy(&self.codec_tag).trim().to_string()
    }

    pub fn encode(&self) -> Vec<u8> {
        let payload_len = u32::try_from(self.payload.len()).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(COOKED_VIDEO_HEADER_BYTES + self.payload.len());
        out.extend_from_slice(&COOKED_VIDEO_MAGIC);
        out.extend_from_slice(&self.format_version.to_le_bytes());
        out.extend_from_slice(&self.container.code().to_le_bytes());
        out.extend_from_slice(&self.codec_tag);
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.fps_num.to_le_bytes());
        out.extend_from_slice(&self.fps_den.to_le_bytes());
        out.extend_from_slice(&self.frame_count.to_le_bytes());
        out.extend_from_slice(&self.duration_micros.to_le_bytes());
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, VideoImportError> {
        if bytes.len() < COOKED_VIDEO_HEADER_BYTES {
            return Err(VideoImportError::Truncated);
        }
        if bytes.get(0..4) != Some(&COOKED_VIDEO_MAGIC) {
            return Err(VideoImportError::InvalidCookedSignature);
        }
        let format_version = read_u32_le(&bytes[4..8]);
        if format_version != COOKED_VIDEO_FORMAT_VERSION {
            return Err(VideoImportError::UnsupportedCookedVersion(format_version));
        }
        let container_code = read_u32_le(&bytes[8..12]);
        let container = VideoContainerFormat::from_code(container_code).ok_or_else(|| {
            VideoImportError::UnsupportedContainer(format!("container code {container_code}"))
        })?;
        let mut codec_tag = [0u8; 4];
        codec_tag.copy_from_slice(&bytes[12..16]);
        let width = read_u32_le(&bytes[16..20]);
        let height = read_u32_le(&bytes[20..24]);
        if width == 0 || height == 0 {
            return Err(VideoImportError::InvalidDimensions { width, height });
        }
        let fps_num = read_u32_le(&bytes[24..28]);
        let fps_den = read_u32_le(&bytes[28..32]);
        if fps_num == 0 || fps_den == 0 {
            return Err(VideoImportError::InvalidFrameRate {
                num: fps_num,
                den: fps_den,
            });
        }
        let frame_count = read_u64_le(&bytes[32..40]);
        let duration_micros = read_u64_le(&bytes[40..48]);
        let payload_len = read_u32_le(&bytes[48..52]) as usize;
        let expected_total = COOKED_VIDEO_HEADER_BYTES
            .checked_add(payload_len)
            .ok_or(VideoImportError::Truncated)?;
        if bytes.len() != expected_total {
            return Err(VideoImportError::Truncated);
        }
        Ok(Self {
            format_version,
            container,
            codec_tag,
            width,
            height,
            fps_num,
            fps_den,
            frame_count,
            duration_micros,
            payload: bytes[COOKED_VIDEO_HEADER_BYTES..expected_total].to_vec(),
        })
    }
}

pub fn is_supported_video_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "mp4" | "m4v" | "mov" | "webm" | "mkv" | "ogv" | "ogg" | "ivf"
    )
}

pub fn is_video_source_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(is_supported_video_extension)
}

pub fn import_video(
    bytes: &[u8],
    file_name: &str,
    settings: &VideoImportSettings,
) -> Result<VideoImportResult, VideoImportError> {
    if bytes.starts_with(&COOKED_VIDEO_MAGIC) {
        let cooked = CookedVideo::decode(bytes)?;
        return Ok(VideoImportResult {
            metadata: VideoStreamMetadata {
                container: cooked.container,
                codec_tag: cooked.codec_tag,
                width: cooked.width,
                height: cooked.height,
                fps_num: cooked.fps_num,
                fps_den: cooked.fps_den,
                frame_count: cooked.frame_count,
                duration_micros: cooked.duration_micros,
            },
            settings: settings.clone(),
            payload: cooked.payload,
        });
    }
    let metadata = probe_video_stream(bytes, file_name)?;
    Ok(VideoImportResult {
        metadata,
        settings: settings.clone(),
        payload: bytes.to_vec(),
    })
}

pub fn video_import_outcome(
    path: &str,
    result: Result<&VideoImportResult, &VideoImportError>,
) -> ImportOutcome {
    match result {
        Ok(_) => ImportOutcome::ready(),
        Err(error) => ImportOutcome::failed(ImportDiagnostic::new(
            path,
            error.to_string(),
            "the video clip cannot be imported; the asset stays failed and cannot play",
        )),
    }
}

pub fn probe_video_stream(
    bytes: &[u8],
    file_name: &str,
) -> Result<VideoStreamMetadata, VideoImportError> {
    if bytes.len() < 12 {
        return Err(VideoImportError::Truncated);
    }
    let ext = extension_of(file_name).to_ascii_lowercase();
    if bytes.starts_with(b"DKIF") {
        return parse_ivf_header(bytes);
    }
    if &bytes[4..8] == b"ftyp"
        || &bytes[4..8] == b"moov"
        || &bytes[4..8] == b"mdat"
        || &bytes[4..8] == b"wide"
    {
        let container = if ext == "mov" || (&bytes[4..8] == b"ftyp" && &bytes[8..12] == b"qt  ") {
            VideoContainerFormat::Mov
        } else {
            VideoContainerFormat::Mp4
        };
        return parse_bmff_container(bytes, container);
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let container = if ext == "mkv" {
            VideoContainerFormat::Matroska
        } else {
            VideoContainerFormat::WebM
        };
        return parse_ebml_container(bytes, container);
    }
    if bytes.starts_with(b"OggS") {
        return parse_ogg_theora_container(bytes);
    }
    Err(VideoImportError::UnsupportedContainer(format!(
        "{file_name} does not carry a recognized MP4, MOV, WebM, Matroska, Ogg, or IVF header"
    )))
}

fn parse_ivf_header(bytes: &[u8]) -> Result<VideoStreamMetadata, VideoImportError> {
    if bytes.len() < 32 {
        return Err(VideoImportError::Truncated);
    }
    let version = read_u16_le(&bytes[4..6]);
    if version != 0 {
        return Err(VideoImportError::CorruptContainer(format!(
            "unsupported IVF version {version}"
        )));
    }
    let mut codec_tag = [0u8; 4];
    codec_tag.copy_from_slice(&bytes[8..12]);
    let width = u32::from(read_u16_le(&bytes[12..14]));
    let height = u32::from(read_u16_le(&bytes[14..16]));
    if width == 0 || height == 0 {
        return Err(VideoImportError::InvalidDimensions { width, height });
    }
    let fps_num = read_u32_le(&bytes[16..20]).max(1);
    let fps_den = read_u32_le(&bytes[20..24]).max(1);
    let frame_count = u64::from(read_u32_le(&bytes[24..28])).max(1);
    let duration_micros = frame_count
        .saturating_mul(u64::from(fps_den))
        .saturating_mul(1_000_000)
        / u64::from(fps_num);
    Ok(VideoStreamMetadata {
        container: VideoContainerFormat::Ivf,
        codec_tag,
        width,
        height,
        fps_num,
        fps_den,
        frame_count,
        duration_micros,
    })
}

#[derive(Default)]
struct Mp4ProbeState {
    mvhd_timescale: u32,
    mvhd_duration: u64,
    width: u32,
    height: u32,
    mdhd_timescale: u32,
    mdhd_duration: u64,
    codec_tag: Option<[u8; 4]>,
    sample_count: u64,
    stts_total_count: u64,
    stts_total_delta: u64,
    is_video_track: bool,
}

fn parse_bmff_container(
    bytes: &[u8],
    container: VideoContainerFormat,
) -> Result<VideoStreamMetadata, VideoImportError> {
    let mut state = Mp4ProbeState::default();
    let mut found_moov = false;
    walk_bmff_boxes(bytes, &mut state, &mut found_moov)?;
    if !found_moov {
        return Err(VideoImportError::CorruptContainer(
            "missing moov box in ISO BMFF container".to_string(),
        ));
    }
    let width = state.width;
    let height = state.height;
    if width == 0 || height == 0 {
        return Err(VideoImportError::InvalidDimensions { width, height });
    }
    let codec_tag = state.codec_tag.unwrap_or(*b"avc1");
    let timescale = if state.mdhd_timescale > 0 {
        state.mdhd_timescale
    } else if state.mvhd_timescale > 0 {
        state.mvhd_timescale
    } else {
        30
    };
    let raw_duration = if state.mdhd_duration > 0 {
        state.mdhd_duration
    } else if state.stts_total_delta > 0 {
        state.stts_total_delta
    } else {
        state.mvhd_duration
    };
    let duration_micros = if timescale > 0 && raw_duration > 0 {
        raw_duration.saturating_mul(1_000_000) / u64::from(timescale)
    } else {
        0
    };
    let frame_count = if state.sample_count > 0 {
        state.sample_count
    } else if state.stts_total_count > 0 {
        state.stts_total_count
    } else {
        1
    };
    let (fps_num, fps_den) =
        if state.stts_total_count > 0 && state.stts_total_delta > 0 && state.mdhd_timescale > 0 {
            let num = u64::from(state.mdhd_timescale).saturating_mul(state.stts_total_count);
            let den = state.stts_total_delta;
            reduce_rational(num, den)
        } else if duration_micros > 0 && frame_count > 0 {
            reduce_rational(frame_count.saturating_mul(1_000_000), duration_micros)
        } else {
            (30, 1)
        };
    Ok(VideoStreamMetadata {
        container,
        codec_tag,
        width,
        height,
        fps_num,
        fps_den,
        frame_count,
        duration_micros,
    })
}

fn reduce_rational(num: u64, den: u64) -> (u32, u32) {
    if num == 0 || den == 0 {
        return (30, 1);
    }
    let g = gcd_u64(num, den).max(1);
    let mut n = num / g;
    let mut d = den / g;
    while n > u64::from(u32::MAX) || d > u64::from(u32::MAX) {
        n = (n >> 1).max(1);
        d = (d >> 1).max(1);
    }
    (n as u32, d as u32)
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn walk_bmff_boxes(
    bytes: &[u8],
    state: &mut Mp4ProbeState,
    found_moov: &mut bool,
) -> Result<(), VideoImportError> {
    let mut pos = 0usize;
    while pos + 8 <= bytes.len() {
        let size32 = read_u32_be(&bytes[pos..pos + 4]) as usize;
        let box_type = &bytes[pos + 4..pos + 8];
        let (header_len, box_size) = if size32 == 1 {
            if pos + 16 > bytes.len() {
                return Err(VideoImportError::Truncated);
            }
            let size64 = usize::try_from(read_u64_be(&bytes[pos + 8..pos + 16]))
                .map_err(|_| VideoImportError::Truncated)?;
            (16usize, size64)
        } else if size32 == 0 {
            (8usize, bytes.len() - pos)
        } else {
            (8usize, size32)
        };
        if box_size < header_len {
            return Err(VideoImportError::CorruptContainer(
                "ISO BMFF box size smaller than header".to_string(),
            ));
        }
        let end = pos
            .checked_add(box_size)
            .ok_or(VideoImportError::Truncated)?;
        if end > bytes.len() {
            return Err(VideoImportError::Truncated);
        }
        let body = &bytes[pos + header_len..end];
        match box_type {
            b"moov" => {
                *found_moov = true;
                walk_bmff_boxes(body, state, found_moov)?;
            }
            b"trak" => {
                let mut track_state = Mp4ProbeState::default();
                walk_bmff_boxes(body, &mut track_state, found_moov)?;
                if track_state.is_video_track
                    || (track_state.width > 0 && track_state.height > 0 && state.width == 0)
                {
                    if track_state.width > 0 && track_state.height > 0 {
                        state.width = track_state.width;
                        state.height = track_state.height;
                    }
                    if track_state.mdhd_timescale > 0 {
                        state.mdhd_timescale = track_state.mdhd_timescale;
                        state.mdhd_duration = track_state.mdhd_duration;
                    }
                    if track_state.codec_tag.is_some() {
                        state.codec_tag = track_state.codec_tag;
                    }
                    if track_state.sample_count > 0 {
                        state.sample_count = track_state.sample_count;
                    }
                    if track_state.stts_total_count > 0 {
                        state.stts_total_count = track_state.stts_total_count;
                        state.stts_total_delta = track_state.stts_total_delta;
                    }
                }
            }
            b"mdia" | b"minf" | b"stbl" => {
                walk_bmff_boxes(body, state, found_moov)?;
            }
            b"mvhd" => parse_mvhd_box(body, state),
            b"tkhd" => parse_tkhd_box(body, state),
            b"mdhd" => parse_mdhd_box(body, state),
            b"hdlr" => parse_hdlr_box(body, state),
            b"stsd" => parse_stsd_box(body, state),
            b"stsz" => parse_stsz_box(body, state),
            b"stts" => parse_stts_box(body, state),
            _ => {}
        }
        pos = end;
    }
    Ok(())
}

fn parse_mvhd_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() < 4 {
        return;
    }
    let version = body[0];
    if version == 1 && body.len() >= 32 {
        state.mvhd_timescale = read_u32_be(&body[20..24]);
        state.mvhd_duration = read_u64_be(&body[24..32]);
    } else if version == 0 && body.len() >= 20 {
        state.mvhd_timescale = read_u32_be(&body[12..16]);
        state.mvhd_duration = u64::from(read_u32_be(&body[16..20]));
    }
}

fn parse_tkhd_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() < 4 {
        return;
    }
    let version = body[0];
    let offset = if version == 1 { 88usize } else { 76usize };
    if body.len() >= offset + 8 {
        let width_fixed = read_u32_be(&body[offset..offset + 4]);
        let height_fixed = read_u32_be(&body[offset + 4..offset + 8]);
        let w = width_fixed >> 16;
        let h = height_fixed >> 16;
        if w > 0 && h > 0 && state.width == 0 {
            state.width = w;
            state.height = h;
        }
    }
}

fn parse_mdhd_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() < 4 {
        return;
    }
    let version = body[0];
    if version == 1 && body.len() >= 32 {
        state.mdhd_timescale = read_u32_be(&body[20..24]);
        state.mdhd_duration = read_u64_be(&body[24..32]);
    } else if version == 0 && body.len() >= 20 {
        state.mdhd_timescale = read_u32_be(&body[12..16]);
        state.mdhd_duration = u64::from(read_u32_be(&body[16..20]));
    }
}

fn parse_hdlr_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() >= 12 && &body[8..12] == b"vide" {
        state.is_video_track = true;
    }
}

fn parse_stsd_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() < 16 {
        return;
    }
    let entry_count = read_u32_be(&body[4..8]);
    if entry_count == 0 {
        return;
    }
    let entry_size = read_u32_be(&body[8..12]) as usize;
    if entry_size < 16 || body.len() < 8 + entry_size {
        return;
    }
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&body[12..16]);
    state.codec_tag = Some(tag);
    if entry_size >= 40 && body.len() >= 8 + 40 {
        let w = u32::from(read_u16_be(&body[8 + 32..8 + 34]));
        let h = u32::from(read_u16_be(&body[8 + 34..8 + 36]));
        if w > 0 && h > 0 {
            state.width = w;
            state.height = h;
        }
    }
}

fn parse_stsz_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() >= 12 {
        state.sample_count = u64::from(read_u32_be(&body[8..12]));
    }
}

fn parse_stts_box(body: &[u8], state: &mut Mp4ProbeState) {
    if body.len() < 8 {
        return;
    }
    let entry_count = read_u32_be(&body[4..8]) as usize;
    let mut pos = 8usize;
    let mut total_count = 0u64;
    let mut total_delta = 0u64;
    for _ in 0..entry_count {
        if pos + 8 > body.len() {
            break;
        }
        let sample_count = u64::from(read_u32_be(&body[pos..pos + 4]));
        let sample_delta = u64::from(read_u32_be(&body[pos + 4..pos + 8]));
        total_count = total_count.saturating_add(sample_count);
        total_delta = total_delta.saturating_add(sample_count.saturating_mul(sample_delta));
        pos += 8;
    }
    state.stts_total_count = total_count;
    state.stts_total_delta = total_delta;
}

fn parse_ebml_container(
    bytes: &[u8],
    container: VideoContainerFormat,
) -> Result<VideoStreamMetadata, VideoImportError> {
    let mut width = 0u32;
    let mut height = 0u32;
    let mut codec_tag = *b"vp08";
    let mut default_duration_ns = 0u64;
    let mut i = 0usize;
    while i + 3 < bytes.len() {
        if bytes[i] == 0xB0 && (bytes[i + 1] == 0x82 || bytes[i + 1] == 0x81) {
            let len = usize::from(bytes[i + 1] & 0x7F);
            if i + 2 + len <= bytes.len() && width == 0 {
                width = read_uint_be(&bytes[i + 2..i + 2 + len]) as u32;
            }
        } else if bytes[i] == 0xBA && (bytes[i + 1] == 0x82 || bytes[i + 1] == 0x81) {
            let len = usize::from(bytes[i + 1] & 0x7F);
            if i + 2 + len <= bytes.len() && height == 0 {
                height = read_uint_be(&bytes[i + 2..i + 2 + len]) as u32;
            }
        } else if i + 6 < bytes.len()
            && bytes[i] == 0x23
            && bytes[i + 1] == 0xE3
            && bytes[i + 2] == 0x83
            && (0x81..=0x88).contains(&bytes[i + 3])
        {
            let len = usize::from(bytes[i + 3] & 0x7F);
            if i + 4 + len <= bytes.len() && default_duration_ns == 0 {
                default_duration_ns = read_uint_be(&bytes[i + 4..i + 4 + len]);
            }
        } else if bytes[i..].starts_with(b"V_VP9") {
            codec_tag = *b"vp09";
        } else if bytes[i..].starts_with(b"V_VP8") {
            codec_tag = *b"vp08";
        } else if bytes[i..].starts_with(b"V_AV1") {
            codec_tag = *b"av01";
        } else if bytes[i..].starts_with(b"V_MPEG4/ISO/AVC") {
            codec_tag = *b"avc1";
        } else if bytes[i..].starts_with(b"V_MPEGH/ISO/HEVC") {
            codec_tag = *b"hvc1";
        }
        i += 1;
    }
    if width == 0 || height == 0 {
        return Err(VideoImportError::CorruptContainer(
            "missing PixelWidth or PixelHeight in WebM/Matroska stream".to_string(),
        ));
    }
    let (fps_num, fps_den) = if default_duration_ns > 0 {
        reduce_rational(1_000_000_000, default_duration_ns)
    } else {
        (30, 1)
    };
    Ok(VideoStreamMetadata {
        container,
        codec_tag,
        width,
        height,
        fps_num,
        fps_den,
        frame_count: 1,
        duration_micros: 1_000_000 / u64::from(fps_num.max(1)) * u64::from(fps_den.max(1)),
    })
}

fn parse_ogg_theora_container(bytes: &[u8]) -> Result<VideoStreamMetadata, VideoImportError> {
    for i in 0..bytes.len().saturating_sub(28) {
        if &bytes[i..i + 7] == b"\x80theora" {
            let width = ((u32::from(bytes[i + 14]) << 16)
                | (u32::from(bytes[i + 15]) << 8)
                | u32::from(bytes[i + 16]))
                & 0x00FF_FFFF;
            let height = ((u32::from(bytes[i + 17]) << 16)
                | (u32::from(bytes[i + 18]) << 8)
                | u32::from(bytes[i + 19]))
                & 0x00FF_FFFF;
            let fps_num = read_u32_be(&bytes[i + 22..i + 26]);
            let fps_den = read_u32_be(&bytes[i + 26..i + 30]);
            if width == 0 || height == 0 {
                return Err(VideoImportError::InvalidDimensions { width, height });
            }
            if fps_num == 0 || fps_den == 0 {
                return Err(VideoImportError::InvalidFrameRate {
                    num: fps_num,
                    den: fps_den,
                });
            }
            return Ok(VideoStreamMetadata {
                container: VideoContainerFormat::Ogg,
                codec_tag: *b"theo",
                width,
                height,
                fps_num,
                fps_den,
                frame_count: 1,
                duration_micros: u64::from(fps_den) * 1_000_000 / u64::from(fps_num),
            });
        }
    }
    Err(VideoImportError::CorruptContainer(
        "Ogg container holds no Theora identification header".to_string(),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedVideoEntry {
    pub asset_path: String,
    pub guid: String,
    pub cooked_path: String,
    pub content_hash: String,
    pub format_version: u32,
    pub container: String,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub frame_count: u64,
    pub duration_micros: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookedVideoManifest {
    pub format_version: u32,
    pub entries: BTreeMap<String, CookedVideoEntry>,
}

impl CookedVideoManifest {
    pub fn new() -> Self {
        Self {
            format_version: COOKED_VIDEO_MANIFEST_FORMAT_VERSION,
            entries: BTreeMap::new(),
        }
    }

    pub fn get_by_asset_path(&self, asset_path: &str) -> Option<&CookedVideoEntry> {
        self.entries.get(&normalize_slashes(asset_path))
    }

    pub fn get_by_guid(&self, guid: &str) -> Option<&CookedVideoEntry> {
        self.entries.values().find(|entry| entry.guid == guid)
    }

    pub fn load_from_dir(cooked_dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(cooked_dir.join(COOKED_VIDEO_MANIFEST_FILENAME)).ok()?;
        Self::decode(&text)
    }

    pub fn save_to_dir(&self, cooked_dir: &Path) -> Result<PathBuf, ImportCacheError> {
        fs::create_dir_all(cooked_dir).map_err(|error| ImportCacheError {
            path: cooked_dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = cooked_dir.join(COOKED_VIDEO_MANIFEST_FILENAME);
        write_file_atomic(&path, self.encode().as_bytes()).map_err(|error| ImportCacheError {
            path: path.clone(),
            message: error.to_string(),
        })?;
        Ok(path)
    }

    pub fn encode(&self) -> String {
        let entries: Vec<JsonValue> = self
            .entries
            .values()
            .map(encode_video_manifest_entry)
            .collect();
        json::write(&JsonValue::Object(vec![
            (
                "format".to_string(),
                JsonValue::String(COOKED_VIDEO_MANIFEST_FORMAT.to_string()),
            ),
            (
                "formatVersion".to_string(),
                JsonValue::Number(f64::from(self.format_version)),
            ),
            ("entries".to_string(), JsonValue::Array(entries)),
        ]))
    }

    pub fn decode(text: &str) -> Option<Self> {
        let value = json::parse(text).ok()?;
        if value.get("format").and_then(JsonValue::as_str) != Some(COOKED_VIDEO_MANIFEST_FORMAT) {
            return None;
        }
        let version = u32::try_from(value.get("formatVersion").and_then(JsonValue::as_i64)?)
            .ok()
            .filter(|v| *v == COOKED_VIDEO_MANIFEST_FORMAT_VERSION)?;
        let mut entries = BTreeMap::new();
        for item in value.get("entries").and_then(JsonValue::as_array)? {
            let entry = decode_video_manifest_entry(item)?;
            entries.insert(entry.asset_path.clone(), entry);
        }
        Some(Self {
            format_version: version,
            entries,
        })
    }
}

fn encode_video_manifest_entry(entry: &CookedVideoEntry) -> JsonValue {
    JsonValue::Object(vec![
        (
            "assetPath".to_string(),
            JsonValue::String(entry.asset_path.clone()),
        ),
        ("guid".to_string(), JsonValue::String(entry.guid.clone())),
        (
            "cookedPath".to_string(),
            JsonValue::String(entry.cooked_path.clone()),
        ),
        (
            "contentHash".to_string(),
            JsonValue::String(entry.content_hash.clone()),
        ),
        (
            "formatVersion".to_string(),
            JsonValue::Number(f64::from(entry.format_version)),
        ),
        (
            "container".to_string(),
            JsonValue::String(entry.container.clone()),
        ),
        ("codec".to_string(), JsonValue::String(entry.codec.clone())),
        (
            "width".to_string(),
            JsonValue::Number(f64::from(entry.width)),
        ),
        (
            "height".to_string(),
            JsonValue::Number(f64::from(entry.height)),
        ),
        (
            "fpsNum".to_string(),
            JsonValue::Number(f64::from(entry.fps_num)),
        ),
        (
            "fpsDen".to_string(),
            JsonValue::Number(f64::from(entry.fps_den)),
        ),
        (
            "frameCount".to_string(),
            JsonValue::Number(entry.frame_count as f64),
        ),
        (
            "durationMicros".to_string(),
            JsonValue::Number(entry.duration_micros as f64),
        ),
    ])
}

fn decode_video_manifest_entry(value: &JsonValue) -> Option<CookedVideoEntry> {
    Some(CookedVideoEntry {
        asset_path: value.get("assetPath")?.as_str()?.to_string(),
        guid: value.get("guid")?.as_str()?.to_string(),
        cooked_path: value.get("cookedPath")?.as_str()?.to_string(),
        content_hash: value.get("contentHash")?.as_str()?.to_string(),
        format_version: u32::try_from(value.get("formatVersion")?.as_i64()?).ok()?,
        container: value.get("container")?.as_str()?.to_string(),
        codec: value.get("codec")?.as_str()?.to_string(),
        width: u32::try_from(value.get("width")?.as_i64()?).ok()?,
        height: u32::try_from(value.get("height")?.as_i64()?).ok()?,
        fps_num: u32::try_from(value.get("fpsNum")?.as_i64()?).ok()?,
        fps_den: u32::try_from(value.get("fpsDen")?.as_i64()?).ok()?,
        frame_count: u64::try_from(value.get("frameCount")?.as_i64()?).ok()?,
        duration_micros: u64::try_from(value.get("durationMicros")?.as_i64()?).ok()?,
    })
}

pub fn build_video_candidate(
    source_path: &Path,
    logical_path: PathBuf,
) -> Result<ImportCandidate, ImportCacheError> {
    let source = fs::read(source_path).map_err(|error| ImportCacheError {
        path: source_path.to_path_buf(),
        message: error.to_string(),
    })?;
    let meta_path = meta_path_for(source_path);
    let metadata = fs::read_to_string(&meta_path).unwrap_or_default();
    let settings_parsed = VideoImportSettings::from_meta_text(&metadata).unwrap_or_default();
    let settings = settings_parsed.to_meta_text();
    let guid = guid_from_meta_text(&metadata).unwrap_or_else(|| {
        format!(
            "{:016x}",
            fnv1a_hash(normalize_slashes(&logical_path.to_string_lossy()).as_bytes())
        )
    });
    Ok(ImportCandidate {
        path: logical_path,
        guid,
        source,
        metadata,
        settings,
        importer_version: VIDEO_IMPORTER_VERSION.to_string(),
        dependencies: Vec::new(),
    })
}

pub fn cook_project_videos(
    project_root: &Path,
    cache_dir: &Path,
    cooked_out_dir: &Path,
) -> Result<(CookedVideoManifest, ImportSessionReport), ImportCacheError> {
    let assets_dir = project_root.join("Assets");
    let mut source_paths = Vec::new();
    if assets_dir.is_dir() {
        collect_video_files(&assets_dir, &mut source_paths)?;
        source_paths.sort();
    }
    let mut candidates = Vec::with_capacity(source_paths.len());
    for source_path in &source_paths {
        let rel = source_path
            .strip_prefix(project_root)
            .unwrap_or(source_path);
        let rel_key = normalize_slashes(&rel.to_string_lossy());
        candidates.push(build_video_candidate(source_path, PathBuf::from(rel_key))?);
    }
    cook_video_candidates(&candidates, cache_dir, Some(cooked_out_dir))
}

pub fn cook_video_candidates(
    candidates: &[ImportCandidate],
    cache_dir: &Path,
    cooked_out_dir: Option<&Path>,
) -> Result<(CookedVideoManifest, ImportSessionReport), ImportCacheError> {
    let _cook_guard = lock_video_cook();
    let cache_cooked_dir = cache_dir.join(COOKED_VIDEO_DIR_NAME);
    fs::create_dir_all(&cache_cooked_dir).map_err(|error| ImportCacheError {
        path: cache_cooked_dir.clone(),
        message: error.to_string(),
    })?;
    let cached_manifest = CookedVideoManifest::load_from_dir(&cache_cooked_dir).unwrap_or_default();
    let mut cache = ImportCache::open(cache_dir);
    prune_missing_cached_video_artifacts(
        &mut cache,
        &cache_cooked_dir,
        &cached_manifest,
        candidates,
    );
    let mut session = ImportSession::with_cache(cache);
    let mut freshly_cooked: BTreeMap<String, CookedVideoEntry> = BTreeMap::new();

    let report = session.run(candidates, &mut |candidate| {
        cook_single_video_candidate(candidate, &cache_cooked_dir, &mut freshly_cooked)
    });
    if let Some(error) = session.cache_error() {
        return Err(error.clone());
    }

    let mut manifest = CookedVideoManifest::new();
    for (key, entry) in &cached_manifest.entries {
        manifest.entries.insert(key.clone(), entry.clone());
    }
    for candidate in candidates {
        let key = normalize_slashes(&candidate.path.to_string_lossy());
        if let Some(entry) = freshly_cooked.remove(&key) {
            manifest.entries.insert(key, entry);
        }
    }
    manifest.save_to_dir(&cache_cooked_dir)?;
    if let Some(out_dir) = cooked_out_dir {
        let mut requested_manifest = CookedVideoManifest::new();
        for candidate in candidates {
            let key = normalize_slashes(&candidate.path.to_string_lossy());
            if let Some(entry) = manifest.entries.get(&key) {
                requested_manifest.entries.insert(key, entry.clone());
            }
        }
        stage_cooked_video_outputs(&cache_cooked_dir, out_dir, &requested_manifest)?;
        return Ok((requested_manifest, report));
    }
    Ok((manifest, report))
}

fn prune_missing_cached_video_artifacts(
    cache: &mut ImportCache,
    cache_cooked_dir: &Path,
    cached_manifest: &CookedVideoManifest,
    candidates: &[ImportCandidate],
) {
    let mut missing = Vec::new();
    for candidate in candidates {
        let key = normalize_slashes(&candidate.path.to_string_lossy());
        let artifact_exists = cached_manifest
            .get_by_asset_path(&key)
            .is_some_and(|entry| cache_cooked_dir.join(&entry.cooked_path).is_file());
        if !artifact_exists {
            missing.push(candidate.path.clone());
        }
    }
    if !missing.is_empty() {
        let retained: Vec<_> = cache
            .records()
            .filter(|record| !missing.contains(&record.path))
            .cloned()
            .collect();
        cache.replace(retained);
    }
}

fn cook_single_video_candidate(
    candidate: &ImportCandidate,
    cache_cooked_dir: &Path,
    freshly_cooked: &mut BTreeMap<String, CookedVideoEntry>,
) -> ImportOutcome {
    let asset_key = normalize_slashes(&candidate.path.to_string_lossy());
    let file_name = candidate
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clip.mp4");
    let settings = match VideoImportSettings::from_meta_text(&candidate.metadata) {
        Ok(settings) => settings,
        Err(error) => {
            return ImportOutcome::failed(ImportDiagnostic::new(
                asset_key,
                error.to_string(),
                "the video clip stays failed",
            ));
        }
    };
    let imported = match import_video(&candidate.source, file_name, &settings) {
        Ok(imported) => imported,
        Err(error) => {
            return ImportOutcome::failed(ImportDiagnostic::new(
                asset_key,
                error.to_string(),
                "the video clip stays failed",
            ));
        }
    };
    let cooked = CookedVideo::from_import(&imported);
    let encoded = cooked.encode();
    let content_hash = format!("{:016x}", fnv1a_hash(&encoded));
    let cooked_filename = format!(
        "{}_{}.{COOKED_VIDEO_EXTENSION}",
        sanitize_id(&candidate.guid),
        content_hash
    );
    let target_path = cache_cooked_dir.join(&cooked_filename);
    if let Err(error) = write_file_atomic(&target_path, &encoded) {
        return ImportOutcome::failed(ImportDiagnostic::new(
            asset_key,
            error.to_string(),
            "the cooked video clip could not be written",
        ));
    }
    freshly_cooked.insert(
        asset_key.clone(),
        CookedVideoEntry {
            asset_path: asset_key,
            guid: candidate.guid.clone(),
            cooked_path: cooked_filename,
            content_hash,
            format_version: cooked.format_version,
            container: cooked.container.as_str().to_string(),
            codec: cooked.codec_name(),
            width: cooked.width,
            height: cooked.height,
            fps_num: cooked.fps_num,
            fps_den: cooked.fps_den,
            frame_count: cooked.frame_count,
            duration_micros: cooked.duration_micros,
        },
    );
    ImportOutcome::ready()
}

fn stage_cooked_video_outputs(
    cache_cooked_dir: &Path,
    cooked_out_dir: &Path,
    manifest: &CookedVideoManifest,
) -> Result<(), ImportCacheError> {
    if cache_cooked_dir == cooked_out_dir {
        return Ok(());
    }
    fs::create_dir_all(cooked_out_dir).map_err(|error| ImportCacheError {
        path: cooked_out_dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in manifest.entries.values() {
        let src = cache_cooked_dir.join(&entry.cooked_path);
        let dst = cooked_out_dir.join(&entry.cooked_path);
        if src != dst {
            copy_file_atomic(&src, &dst).map_err(|error| ImportCacheError {
                path: dst,
                message: error.to_string(),
            })?;
        }
    }
    manifest.save_to_dir(cooked_out_dir)?;
    Ok(())
}

pub fn load_or_cook_video(path: &Path) -> Option<CookedVideo> {
    load_or_cook_video_with_cache(path, None)
}

pub fn load_or_cook_video_with_cache(
    path: &Path,
    explicit_cache_dir: Option<&Path>,
) -> Option<CookedVideo> {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(COOKED_VIDEO_EXTENSION))
        && path.is_file()
    {
        let bytes = fs::read(path).ok()?;
        return CookedVideo::decode(&bytes).ok();
    }
    if let Some((root, cooked_dir, manifest)) = find_ancestor_cooked_video_manifest(path) {
        if let Ok(rel) = path.strip_prefix(&root) {
            let rel_key = normalize_slashes(&rel.to_string_lossy());
            if let Some(entry) = manifest.get_by_asset_path(&rel_key) {
                let bytes = fs::read(cooked_dir.join(&entry.cooked_path)).ok()?;
                return CookedVideo::decode(&bytes).ok();
            }
        }
        if let Some(guid) = read_sidecar_guid(path) {
            if let Some(entry) = manifest.get_by_guid(&guid) {
                let bytes = fs::read(cooked_dir.join(&entry.cooked_path)).ok()?;
                return CookedVideo::decode(&bytes).ok();
            }
        }
        if root.join(BUNDLE_MANIFEST_FILENAME).is_file() {
            return None;
        }
    }
    if is_inside_staged_bundle(path) && !path.is_file() {
        return None;
    }
    let logical_path = logical_asset_path(path);
    let candidate = build_video_candidate(path, logical_path).ok()?;
    let cache_dir = explicit_cache_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_video_cache_dir_for_path(path, &candidate.fingerprint()));
    let (manifest, _) =
        cook_video_candidates(std::slice::from_ref(&candidate), &cache_dir, None).ok()?;
    let key = normalize_slashes(&candidate.path.to_string_lossy());
    let entry = manifest.get_by_asset_path(&key)?;
    let cooked_bytes = fs::read(
        cache_dir
            .join(COOKED_VIDEO_DIR_NAME)
            .join(&entry.cooked_path),
    )
    .ok()?;
    CookedVideo::decode(&cooked_bytes).ok()
}

pub fn default_video_cache_dir(project_root: &Path) -> PathBuf {
    let hash = fnv1a_hash(project_root.to_string_lossy().as_bytes());
    std::env::temp_dir()
        .join("unitas_video_import_cache")
        .join(format!("{}_{hash:016x}", std::process::id()))
}

fn default_video_cache_dir_for_path(path: &Path, fingerprint: &ImportFingerprint) -> PathBuf {
    if let Some(root) = find_project_root(path) {
        return default_video_cache_dir(&root);
    }
    let parent_key = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{:016x}", fingerprint.source));
    let hash = fnv1a_hash(parent_key.as_bytes());
    std::env::temp_dir()
        .join("unitas_video_import_cache")
        .join(format!("{}_{hash:016x}", std::process::id()))
}

fn find_ancestor_cooked_video_manifest(
    path: &Path,
) -> Option<(PathBuf, PathBuf, CookedVideoManifest)> {
    let mut current = path.parent()?;
    loop {
        let cooked_dir = current.join(COOKED_VIDEO_DIR_NAME);
        if let Some(manifest) = CookedVideoManifest::load_from_dir(&cooked_dir) {
            return Some((current.to_path_buf(), cooked_dir, manifest));
        }
        current = current.parent()?;
    }
}

fn is_inside_staged_bundle(path: &Path) -> bool {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(BUNDLE_MANIFEST_FILENAME).is_file() {
            return true;
        }
        current = dir.parent();
    }
    false
}

fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn logical_asset_path(path: &Path) -> PathBuf {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.file_name().and_then(|n| n.to_str()) == Some("Assets") {
            if let Some(root) = dir.parent() {
                if let Ok(rel) = path.strip_prefix(root) {
                    return PathBuf::from(normalize_slashes(&rel.to_string_lossy()));
                }
            }
        }
        current = dir.parent();
    }
    path.file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
}

fn read_sidecar_guid(path: &Path) -> Option<String> {
    let text = fs::read_to_string(meta_path_for(path)).ok()?;
    guid_from_meta_text(&text)
}

fn meta_path_for(path: &Path) -> PathBuf {
    let mut meta = path.as_os_str().to_os_string();
    meta.push(".meta");
    PathBuf::from(meta)
}

fn collect_video_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ImportCacheError> {
    let entries = fs::read_dir(dir).map_err(|error| ImportCacheError {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ImportCacheError {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_video_files(&path, out)?;
        } else if is_video_source_path(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn extension_of(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => "",
    }
}

fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

fn sanitize_id(raw: &str) -> String {
    let filtered: String = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();
    if filtered.is_empty() {
        "video".to_string()
    } else {
        filtered
    }
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let seq = VIDEO_COOK_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut temp = path.file_name().unwrap_or_default().to_os_string();
    temp.push(format!(".{}.{seq}.tmp", std::process::id()));
    let temp_path = path.with_file_name(temp);
    fs::write(&temp_path, bytes)?;
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn copy_file_atomic(src: &Path, dst: &Path) -> std::io::Result<()> {
    let seq = VIDEO_COOK_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut temp = dst.file_name().unwrap_or_default().to_os_string();
    temp.push(format!(".{}.{seq}.tmp", std::process::id()));
    let temp_path = dst.with_file_name(temp);
    fs::copy(src, &temp_path)?;
    if let Err(error) = fs::rename(&temp_path, dst) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u16_be(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u32_be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn read_u64_be(bytes: &[u8]) -> u64 {
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn read_uint_be(bytes: &[u8]) -> u64 {
    let mut value = 0u64;
    for byte in bytes {
        value = (value << 8) | u64::from(*byte);
    }
    value
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
