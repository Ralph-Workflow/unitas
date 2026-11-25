use std::collections::BTreeMap;
use std::path::Path;
use unitas_core::EntityId;

#[cfg(feature = "device")]
pub mod output;

pub type AudioClipId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioError {
    UnsupportedFormat(String),
    InvalidWav(String),
    Io(String),
    Device(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat(detail) => {
                write!(formatter, "unsupported audio format: {detail}")
            }
            Self::InvalidWav(detail) => write!(formatter, "invalid wav data: {detail}"),
            Self::Io(detail) => write!(formatter, "audio io error: {detail}"),
            Self::Device(detail) => write!(formatter, "audio device error: {detail}"),
        }
    }
}

impl std::error::Error for AudioError {}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioSourceState {
    Scheduled = 0,
    Loading = 1,
    Playing = 2,
    Paused = 3,
    Virtualized = 4,
    Stopped = 5,
    Completed = 6,
    Failed = 7,
}

impl AudioSourceState {
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilenceReason {
    ZeroVolume = 0,
    Muted = 1,
    Paused = 2,
    Virtualized = 3,
    OutOfRange = 4,
    NoListener = 5,
    UnsupportedClip = 6,
    Stopped = 7,
    Completed = 8,
}

impl SilenceReason {
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioClip {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    pub duration: f64,
}

impl AudioClip {
    pub fn new(channels: u16, sample_rate: u32, samples: Vec<f32>) -> Self {
        let ch = usize::from(channels.max(1));
        let frame_count = samples.len() / ch;
        let duration = if sample_rate > 0 {
            frame_count as f64 / f64::from(sample_rate)
        } else {
            0.0
        };
        Self {
            channels,
            sample_rate,
            samples,
            duration,
        }
    }

    pub fn frame_count(&self) -> usize {
        let ch = usize::from(self.channels.max(1));
        self.samples.len() / ch
    }

    pub fn parse_wav(bytes: &[u8]) -> Result<Self, AudioError> {
        parse_wav(bytes)
    }
}

pub fn decode_audio(bytes: &[u8]) -> Result<AudioClip, AudioError> {
    use unitas_asset::audio_import as import;
    let pcm =
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
            import::decode_wav(bytes)
        } else if bytes.starts_with(b"FORM") {
            import::decode_aiff(bytes)
        } else if bytes.starts_with(b"OggS") {
            import::decode_ogg_vorbis(bytes)
        } else if bytes.starts_with(b"ID3")
            || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
        {
            import::decode_mp3(bytes)
        } else {
            return Err(AudioError::UnsupportedFormat(
                "unrecognized audio container".to_string(),
            ));
        }
        .map_err(|error| AudioError::UnsupportedFormat(error.to_string()))?;
    if pcm.channels == 0 || pcm.channels > 2 {
        return Err(AudioError::UnsupportedFormat(format!(
            "{} audio channels are not supported",
            pcm.channels
        )));
    }
    let samples = pcm
        .samples
        .iter()
        .map(|sample| f32::from(*sample) / 32768.0)
        .collect();
    Ok(AudioClip::new(pcm.channels, pcm.sample_rate, samples))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Option<i16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(i16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub fn parse_wav(bytes: &[u8]) -> Result<AudioClip, AudioError> {
    if bytes.starts_with(b"OggS") {
        return Err(AudioError::UnsupportedFormat("OGG container".to_string()));
    }
    if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2
            && bytes[0] == 0xFF
            && (bytes[1] == 0xFB
                || bytes[1] == 0xF3
                || bytes[1] == 0xF2
                || (bytes[1] & 0xE0) == 0xE0))
    {
        return Err(AudioError::UnsupportedFormat("MP3 audio".to_string()));
    }
    if bytes.starts_with(b"fLaC") {
        return Err(AudioError::UnsupportedFormat("FLAC audio".to_string()));
    }
    if bytes.len() < 12 {
        return Err(AudioError::InvalidWav(
            "buffer shorter than 12-byte RIFF header".to_string(),
        ));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::InvalidWav(
            "missing RIFF/WAVE header".to_string(),
        ));
    }

    let mut offset = 12usize;
    let mut fmt_fields: Option<(u16, u16, u32, u16)> = None;
    let mut data_slice: Option<&[u8]> = None;

    while offset + 8 <= bytes.len() {
        let Some(chunk_id) = bytes.get(offset..offset + 4) else {
            break;
        };
        let Some(chunk_size_u32) = read_u32_le(bytes, offset + 4) else {
            break;
        };
        let chunk_size = chunk_size_u32 as usize;
        let payload_start = offset + 8;
        let Some(payload_end) = payload_start.checked_add(chunk_size) else {
            return Err(AudioError::InvalidWav("chunk size overflow".to_string()));
        };
        if payload_end > bytes.len() {
            return Err(AudioError::InvalidWav(
                "chunk extends past end of buffer".to_string(),
            ));
        }

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err(AudioError::InvalidWav(
                    "fmt chunk smaller than 16 bytes".to_string(),
                ));
            }
            let Some(audio_format) = read_u16_le(bytes, payload_start) else {
                return Err(AudioError::InvalidWav(
                    "failed to read audio_format".to_string(),
                ));
            };
            let Some(channels) = read_u16_le(bytes, payload_start + 2) else {
                return Err(AudioError::InvalidWav(
                    "failed to read channels".to_string(),
                ));
            };
            let Some(sample_rate) = read_u32_le(bytes, payload_start + 4) else {
                return Err(AudioError::InvalidWav(
                    "failed to read sample_rate".to_string(),
                ));
            };
            let Some(bits_per_sample) = read_u16_le(bytes, payload_start + 14) else {
                return Err(AudioError::InvalidWav(
                    "failed to read bits_per_sample".to_string(),
                ));
            };
            fmt_fields = Some((audio_format, channels, sample_rate, bits_per_sample));
        } else if chunk_id == b"data" {
            data_slice = bytes.get(payload_start..payload_end);
        }

        let pad = chunk_size % 2;
        offset = payload_end.saturating_add(pad);
    }

    let Some((audio_format, channels, sample_rate, bits_per_sample)) = fmt_fields else {
        return Err(AudioError::InvalidWav("missing fmt chunk".to_string()));
    };
    if audio_format != 1 {
        return Err(AudioError::UnsupportedFormat(format!(
            "WAV audio_format {audio_format} (only PCM format 1 is supported)"
        )));
    }
    if channels != 1 && channels != 2 {
        return Err(AudioError::UnsupportedFormat(format!(
            "WAV channels {channels} (only mono 1 or stereo 2 supported)"
        )));
    }
    if bits_per_sample != 16 {
        return Err(AudioError::UnsupportedFormat(format!(
            "WAV bits_per_sample {bits_per_sample} (only 16-bit PCM supported)"
        )));
    }
    if sample_rate == 0 {
        return Err(AudioError::InvalidWav(
            "WAV sample_rate must be positive".to_string(),
        ));
    }

    let Some(raw_data) = data_slice else {
        return Err(AudioError::InvalidWav("missing data chunk".to_string()));
    };
    let bytes_per_frame = usize::from(channels) * 2;
    if raw_data.len() % bytes_per_frame != 0 {
        return Err(AudioError::InvalidWav(
            "data chunk not aligned to frame size".to_string(),
        ));
    }

    let sample_count = raw_data.len() / 2;
    let mut samples = Vec::with_capacity(sample_count);
    for idx in 0..sample_count {
        let Some(pcm) = read_i16_le(raw_data, idx * 2) else {
            return Err(AudioError::InvalidWav("truncated sample".to_string()));
        };
        samples.push((f32::from(pcm) / 32768.0).clamp(-1.0, 1.0));
    }

    Ok(AudioClip::new(channels, sample_rate, samples))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListenerPose {
    pub position: [f32; 3],
    pub forward: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioSource {
    pub clip: Option<AudioClipId>,
    pub custom_duration: Option<f64>,
    pub state: AudioSourceState,
    pub volume: f32,
    pub pitch: f32,
    pub mute: bool,
    pub looping: bool,
    pub play_on_awake: bool,
    pub priority: u8,
    pub spatial_blend: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub time: f64,
    pub position: [f32; 3],
    pub ignore_time_scale: bool,
    pub one_shot_active: bool,
    pub unsupported_clip: bool,
    pub wants_play: bool,
    pub sample_cursor: f64,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            clip: None,
            custom_duration: None,
            state: AudioSourceState::Stopped,
            volume: 1.0,
            pitch: 1.0,
            mute: false,
            looping: false,
            play_on_awake: true,
            priority: 128,
            spatial_blend: 0.0,
            min_distance: 1.0,
            max_distance: 500.0,
            time: 0.0,
            position: [0.0, 0.0, 0.0],
            ignore_time_scale: false,
            one_shot_active: false,
            unsupported_clip: false,
            wants_play: false,
            sample_cursor: 0.0,
        }
    }
}

pub struct AudioWorld {
    clips: BTreeMap<AudioClipId, AudioClip>,
    failed_clips: BTreeMap<AudioClipId, String>,
    next_clip_id: AudioClipId,
    sources: BTreeMap<EntityId, AudioSource>,
    listener: Option<ListenerPose>,
    max_real_voices: usize,
    application_paused: bool,
}

impl Default for AudioWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioWorld {
    pub fn new() -> Self {
        Self {
            clips: BTreeMap::new(),
            failed_clips: BTreeMap::new(),
            next_clip_id: 1,
            sources: BTreeMap::new(),
            listener: Some(ListenerPose {
                position: [0.0, 0.0, 0.0],
                forward: [0.0, 0.0, 1.0],
            }),
            max_real_voices: 32,
            application_paused: false,
        }
    }

    pub fn register_clip(&mut self, clip: AudioClip) -> AudioClipId {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        self.clips.insert(id, clip);
        id
    }

    pub fn register_failed_clip(&mut self, reason: impl Into<String>) -> AudioClipId {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        self.failed_clips.insert(id, reason.into());
        id
    }

    pub fn load_wav_bytes(&mut self, bytes: &[u8]) -> Result<AudioClipId, AudioError> {
        let clip = parse_wav(bytes)?;
        Ok(self.register_clip(clip))
    }

    pub fn load_wav_file(&mut self, path: &Path) -> Result<AudioClipId, AudioError> {
        let bytes = std::fs::read(path).map_err(|err| AudioError::Io(err.to_string()))?;
        self.load_wav_bytes(&bytes)
    }

    pub fn clip(&self, clip_id: AudioClipId) -> Option<&AudioClip> {
        self.clips.get(&clip_id)
    }

    pub fn ensure_source(&mut self, entity: EntityId) -> &mut AudioSource {
        self.sources.entry(entity).or_default()
    }

    pub fn remove_source(&mut self, entity: EntityId) -> bool {
        let removed = self.sources.remove(&entity).is_some();
        if removed {
            self.recompute_virtualization();
        }
        removed
    }

    pub fn source(&self, entity: EntityId) -> Option<&AudioSource> {
        self.sources.get(&entity)
    }

    pub fn set_source_clip(&mut self, entity: EntityId, clip_id: Option<AudioClipId>) {
        let is_failed = clip_id
            .map(|id| self.failed_clips.contains_key(&id))
            .unwrap_or(false);
        let source = self.ensure_source(entity);
        source.clip = clip_id;
        source.time = 0.0;
        source.sample_cursor = 0.0;
        if is_failed {
            source.unsupported_clip = true;
            source.wants_play = false;
            source.state = AudioSourceState::Failed;
        } else {
            source.unsupported_clip = false;
            if source.state == AudioSourceState::Failed {
                source.state = AudioSourceState::Stopped;
            }
        }
        self.recompute_virtualization();
    }

    pub fn set_custom_duration(&mut self, entity: EntityId, duration: Option<f64>) {
        let source = self.ensure_source(entity);
        source.custom_duration = duration.map(|d| d.max(0.0));
    }

    pub fn mark_source_unsupported_clip(&mut self, entity: EntityId) {
        let source = self.ensure_source(entity);
        source.unsupported_clip = true;
        source.wants_play = false;
        source.one_shot_active = false;
        source.state = AudioSourceState::Failed;
        self.recompute_virtualization();
    }

    pub fn set_listener(&mut self, position: [f32; 3], forward: [f32; 3]) {
        self.listener = Some(ListenerPose { position, forward });
    }

    pub fn clear_listener(&mut self) {
        self.listener = None;
    }

    pub fn listener(&self) -> Option<ListenerPose> {
        self.listener
    }

    pub fn set_max_real_voices(&mut self, max_voices: usize) {
        self.max_real_voices = max_voices;
        self.recompute_virtualization();
    }

    pub fn max_real_voices(&self) -> usize {
        self.max_real_voices
    }

    pub fn set_application_paused(&mut self, paused: bool) {
        self.application_paused = paused;
    }

    pub fn is_application_paused(&self) -> bool {
        self.application_paused
    }

    pub fn play(&mut self, entity: EntityId, one_shot: bool) {
        self.ensure_source(entity);
        let duration = self.effective_duration(entity);
        let sample_rate = self.source_sample_rate(entity);
        let source = self.ensure_source(entity);
        if source.unsupported_clip {
            source.state = AudioSourceState::Failed;
            source.wants_play = false;
            return;
        }
        if one_shot {
            source.time = 0.0;
            source.sample_cursor = 0.0;
        } else if source.state == AudioSourceState::Completed
            || source.state == AudioSourceState::Stopped
        {
            if let Some(d) = duration {
                if source.time >= d {
                    source.time = 0.0;
                    source.sample_cursor = 0.0;
                }
            }
        }
        source.sample_cursor = source.time * f64::from(sample_rate);
        source.one_shot_active = one_shot;
        source.wants_play = true;
        source.state = AudioSourceState::Playing;
        self.recompute_virtualization();
    }

    pub fn pause(&mut self, entity: EntityId) {
        let Some(source) = self.sources.get_mut(&entity) else {
            return;
        };
        if source.state == AudioSourceState::Playing
            || source.state == AudioSourceState::Virtualized
        {
            source.wants_play = false;
            source.state = AudioSourceState::Paused;
            self.recompute_virtualization();
        }
    }

    pub fn unpause(&mut self, entity: EntityId) {
        let Some(source) = self.sources.get_mut(&entity) else {
            return;
        };
        if source.state == AudioSourceState::Paused {
            source.wants_play = true;
            source.state = AudioSourceState::Playing;
            self.recompute_virtualization();
        }
    }

    pub fn stop(&mut self, entity: EntityId) {
        let source = self.ensure_source(entity);
        if source.unsupported_clip {
            source.state = AudioSourceState::Failed;
            source.wants_play = false;
            return;
        }
        source.wants_play = false;
        source.one_shot_active = false;
        source.time = 0.0;
        source.sample_cursor = 0.0;
        source.state = AudioSourceState::Stopped;
        self.recompute_virtualization();
    }

    pub fn set_volume(&mut self, entity: EntityId, volume: f32) {
        let source = self.ensure_source(entity);
        source.volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn volume(&self, entity: EntityId) -> f32 {
        self.sources.get(&entity).map(|s| s.volume).unwrap_or(1.0)
    }

    pub fn set_pitch(&mut self, entity: EntityId, pitch: f32) {
        let source = self.ensure_source(entity);
        if pitch.is_finite() {
            source.pitch = pitch;
        }
    }

    pub fn pitch(&self, entity: EntityId) -> f32 {
        self.sources.get(&entity).map(|s| s.pitch).unwrap_or(1.0)
    }

    pub fn set_mute(&mut self, entity: EntityId, mute: bool) {
        let source = self.ensure_source(entity);
        source.mute = mute;
    }

    pub fn mute(&self, entity: EntityId) -> bool {
        self.sources.get(&entity).map(|s| s.mute).unwrap_or(false)
    }

    pub fn set_looping(&mut self, entity: EntityId, looping: bool) {
        let source = self.ensure_source(entity);
        source.looping = looping;
    }

    pub fn looping(&self, entity: EntityId) -> bool {
        self.sources
            .get(&entity)
            .map(|s| s.looping)
            .unwrap_or(false)
    }

    pub fn set_play_on_awake(&mut self, entity: EntityId, play_on_awake: bool) {
        let source = self.ensure_source(entity);
        source.play_on_awake = play_on_awake;
    }

    pub fn play_on_awake(&self, entity: EntityId) -> bool {
        self.sources
            .get(&entity)
            .map(|s| s.play_on_awake)
            .unwrap_or(true)
    }

    pub fn set_priority(&mut self, entity: EntityId, priority: u8) {
        let source = self.ensure_source(entity);
        source.priority = priority;
        self.recompute_virtualization();
    }

    pub fn priority(&self, entity: EntityId) -> u8 {
        self.sources.get(&entity).map(|s| s.priority).unwrap_or(128)
    }

    pub fn set_spatial_blend(&mut self, entity: EntityId, blend: f32) {
        let source = self.ensure_source(entity);
        source.spatial_blend = if blend.is_finite() {
            blend.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn spatial_blend(&self, entity: EntityId) -> f32 {
        self.sources
            .get(&entity)
            .map(|s| s.spatial_blend)
            .unwrap_or(0.0)
    }

    pub fn set_min_distance(&mut self, entity: EntityId, min_distance: f32) {
        let source = self.ensure_source(entity);
        if min_distance.is_finite() {
            source.min_distance = min_distance.max(0.0);
        }
    }

    pub fn min_distance(&self, entity: EntityId) -> f32 {
        self.sources
            .get(&entity)
            .map(|s| s.min_distance)
            .unwrap_or(1.0)
    }

    pub fn set_max_distance(&mut self, entity: EntityId, max_distance: f32) {
        let source = self.ensure_source(entity);
        if max_distance.is_finite() {
            source.max_distance = max_distance.max(0.0);
        }
    }

    pub fn max_distance(&self, entity: EntityId) -> f32 {
        self.sources
            .get(&entity)
            .map(|s| s.max_distance)
            .unwrap_or(500.0)
    }

    pub fn set_source_time(&mut self, entity: EntityId, time: f64) {
        let sample_rate = self.source_sample_rate(entity);
        let duration = self.effective_duration(entity);
        let source = self.ensure_source(entity);
        let clamped = if time.is_finite() { time.max(0.0) } else { 0.0 };
        source.time = if let Some(d) = duration {
            clamped.min(d)
        } else {
            clamped
        };
        source.sample_cursor = source.time * f64::from(sample_rate);
    }

    pub fn source_time(&self, entity: EntityId) -> f64 {
        self.sources.get(&entity).map(|s| s.time).unwrap_or(0.0)
    }

    pub fn set_source_position(&mut self, entity: EntityId, position: [f32; 3]) {
        let source = self.ensure_source(entity);
        if position.iter().all(|v| v.is_finite()) {
            source.position = position;
        }
    }

    pub fn source_position(&self, entity: EntityId) -> [f32; 3] {
        self.sources
            .get(&entity)
            .map(|s| s.position)
            .unwrap_or([0.0, 0.0, 0.0])
    }

    pub fn set_ignore_time_scale(&mut self, entity: EntityId, ignore: bool) {
        let source = self.ensure_source(entity);
        source.ignore_time_scale = ignore;
    }

    pub fn ignore_time_scale(&self, entity: EntityId) -> bool {
        self.sources
            .get(&entity)
            .map(|s| s.ignore_time_scale)
            .unwrap_or(false)
    }

    pub fn source_state(&self, entity: EntityId) -> AudioSourceState {
        self.sources
            .get(&entity)
            .map(|s| s.state)
            .unwrap_or(AudioSourceState::Stopped)
    }

    pub fn inaudible_reason(&self, entity: EntityId) -> Option<SilenceReason> {
        let Some(source) = self.sources.get(&entity) else {
            return Some(SilenceReason::Stopped);
        };
        if source.unsupported_clip || source.state == AudioSourceState::Failed {
            return Some(SilenceReason::UnsupportedClip);
        }
        match source.state {
            AudioSourceState::Paused => return Some(SilenceReason::Paused),
            AudioSourceState::Virtualized => return Some(SilenceReason::Virtualized),
            AudioSourceState::Stopped | AudioSourceState::Scheduled | AudioSourceState::Loading => {
                return Some(SilenceReason::Stopped)
            }
            AudioSourceState::Completed => return Some(SilenceReason::Completed),
            AudioSourceState::Failed => return Some(SilenceReason::UnsupportedClip),
            AudioSourceState::Playing => {}
        }
        if self.listener.is_none() {
            return Some(SilenceReason::NoListener);
        }
        if source.mute {
            return Some(SilenceReason::Muted);
        }
        if source.volume <= 0.0 {
            return Some(SilenceReason::ZeroVolume);
        }
        if source.spatial_blend > 0.0 {
            let dist = self.distance_to_listener(source.position);
            if dist >= source.max_distance {
                return Some(SilenceReason::OutOfRange);
            }
        }
        None
    }

    pub fn source_gains(&self, entity: EntityId) -> Option<(f32, f32)> {
        let source = self.sources.get(&entity)?;
        if source.mute || source.volume <= 0.0 || self.listener.is_none() {
            return Some((0.0, 0.0));
        }
        let b = source.spatial_blend.clamp(0.0, 1.0);
        let dist = self.distance_to_listener(source.position);
        let attenuation_3d = if dist <= source.min_distance {
            1.0
        } else if dist >= source.max_distance || source.max_distance <= source.min_distance {
            0.0
        } else {
            (source.max_distance - dist) / (source.max_distance - source.min_distance)
        };
        let distance_gain = (1.0 - b) + b * attenuation_3d;
        let pan_3d = self.compute_pan_3d(source.position);
        let pan_left = (1.0 - b) + b * (0.5 * (1.0 - pan_3d));
        let pan_right = (1.0 - b) + b * (0.5 * (1.0 + pan_3d));
        let base = source.volume * distance_gain;
        Some((base * pan_left, base * pan_right))
    }

    pub fn tick(&mut self, scaled_dt: f64, time_scale: f64) {
        if self.application_paused || !scaled_dt.is_finite() || scaled_dt < 0.0 {
            return;
        }
        let scale = if time_scale.is_finite() {
            time_scale.max(0.0)
        } else {
            0.0
        };
        let entities: Vec<EntityId> = self.sources.keys().copied().collect();
        let mut any_completed = false;

        for entity in entities {
            let duration = self.effective_duration(entity);
            let sample_rate = self.source_sample_rate(entity);
            let Some(source) = self.sources.get_mut(&entity) else {
                continue;
            };
            if source.state != AudioSourceState::Playing
                && source.state != AudioSourceState::Virtualized
            {
                continue;
            }
            let pitch = f64::from(source.pitch).max(0.0);
            let dt = if source.ignore_time_scale {
                scaled_dt
            } else {
                scaled_dt * scale
            };
            let delta = dt * pitch;
            if delta <= 0.0 {
                continue;
            }
            source.time += delta;
            if let Some(d) = duration {
                if d > 0.0 && source.time >= d {
                    if source.looping && !source.one_shot_active {
                        source.time %= d;
                    } else {
                        source.time = d;
                        source.wants_play = false;
                        source.one_shot_active = false;
                        source.state = AudioSourceState::Completed;
                        any_completed = true;
                    }
                }
            }
            source.sample_cursor = source.time * f64::from(sample_rate);
        }

        if any_completed {
            self.recompute_virtualization();
        }
    }

    pub fn mix(&mut self, n_frames: usize) -> Vec<f32> {
        let mut output = vec![0.0f32; n_frames * 2];
        let entities: Vec<EntityId> = self.sources.keys().copied().collect();
        let mut any_completed = false;

        for entity in entities {
            if self.inaudible_reason(entity).is_some() {
                continue;
            }
            let Some((gain_l, gain_r)) = self.source_gains(entity) else {
                continue;
            };
            let Some(clip_id) = self.sources.get(&entity).and_then(|s| s.clip) else {
                continue;
            };
            let Some(clip) = self.clips.get(&clip_id).cloned() else {
                continue;
            };
            let total_frames = clip.frame_count();
            if total_frames == 0 || clip.sample_rate == 0 {
                continue;
            }
            let Some(source) = self.sources.get_mut(&entity) else {
                continue;
            };
            let step = f64::from(source.pitch).max(0.0);

            for frame_out in 0..n_frames {
                let mut frame_idx = source.sample_cursor.floor() as usize;
                if frame_idx >= total_frames {
                    if source.looping && !source.one_shot_active {
                        frame_idx %= total_frames;
                        source.sample_cursor %= total_frames as f64;
                    } else {
                        source.time = clip.duration;
                        source.sample_cursor = total_frames as f64;
                        source.wants_play = false;
                        source.one_shot_active = false;
                        source.state = AudioSourceState::Completed;
                        any_completed = true;
                        break;
                    }
                }

                let (sample_l, sample_r) = if clip.channels == 1 {
                    let s = clip.samples.get(frame_idx).copied().unwrap_or(0.0);
                    (s, s)
                } else {
                    let l = clip.samples.get(frame_idx * 2).copied().unwrap_or(0.0);
                    let r = clip.samples.get(frame_idx * 2 + 1).copied().unwrap_or(0.0);
                    (l, r)
                };

                output[frame_out * 2] += sample_l * gain_l;
                output[frame_out * 2 + 1] += sample_r * gain_r;

                source.sample_cursor += step;
                if source.sample_cursor >= total_frames as f64 {
                    if source.looping && !source.one_shot_active {
                        source.sample_cursor %= total_frames as f64;
                    } else if frame_out + 1 == n_frames {
                        source.time = clip.duration;
                        source.sample_cursor = total_frames as f64;
                        source.wants_play = false;
                        source.one_shot_active = false;
                        source.state = AudioSourceState::Completed;
                        any_completed = true;
                        break;
                    }
                }
                source.time = source.sample_cursor / f64::from(clip.sample_rate);
            }
        }

        if any_completed {
            self.recompute_virtualization();
        }
        output
    }

    fn effective_duration(&self, entity: EntityId) -> Option<f64> {
        let source = self.sources.get(&entity)?;
        if let Some(clip_id) = source.clip {
            if let Some(clip) = self.clips.get(&clip_id) {
                return Some(clip.duration);
            }
        }
        source.custom_duration
    }

    fn source_sample_rate(&self, entity: EntityId) -> u32 {
        self.sources
            .get(&entity)
            .and_then(|s| s.clip)
            .and_then(|id| self.clips.get(&id))
            .map(|c| c.sample_rate)
            .unwrap_or(44100)
    }

    fn distance_to_listener(&self, source_pos: [f32; 3]) -> f32 {
        let Some(listener) = self.listener else {
            return 0.0;
        };
        let dx = source_pos[0] - listener.position[0];
        let dy = source_pos[1] - listener.position[1];
        let dz = source_pos[2] - listener.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn compute_pan_3d(&self, source_pos: [f32; 3]) -> f32 {
        let Some(listener) = self.listener else {
            return 0.0;
        };
        let dx = source_pos[0] - listener.position[0];
        let dy = source_pos[1] - listener.position[1];
        let dz = source_pos[2] - listener.position[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist <= 1e-6 {
            return 0.0;
        }
        let dir = [dx / dist, dy / dist, dz / dist];
        let up = [0.0f32, 1.0, 0.0];
        let f = listener.forward;
        let rx = up[1] * f[2] - up[2] * f[1];
        let ry = up[2] * f[0] - up[0] * f[2];
        let rz = up[0] * f[1] - up[1] * f[0];
        let r_len = (rx * rx + ry * ry + rz * rz).sqrt();
        let right = if r_len > 1e-6 {
            [rx / r_len, ry / r_len, rz / r_len]
        } else {
            [1.0, 0.0, 0.0]
        };
        let dot = dir[0] * right[0] + dir[1] * right[1] + dir[2] * right[2];
        dot.clamp(-1.0, 1.0)
    }

    fn recompute_virtualization(&mut self) {
        let mut active: Vec<(u8, EntityId)> = self
            .sources
            .iter()
            .filter_map(|(entity, source)| {
                if source.wants_play
                    && (source.state == AudioSourceState::Playing
                        || source.state == AudioSourceState::Virtualized)
                {
                    Some((source.priority, *entity))
                } else {
                    None
                }
            })
            .collect();

        active.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        for (idx, (_, entity)) in active.into_iter().enumerate() {
            if let Some(source) = self.sources.get_mut(&entity) {
                if idx < self.max_real_voices {
                    source.state = AudioSourceState::Playing;
                } else {
                    source.state = AudioSourceState::Virtualized;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_core::EntityId;

    fn build_wav(
        audio_format: u16,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
        pcm_samples: &[i16],
        extra_chunk: Option<(&[u8; 4], &[u8])>,
    ) -> Vec<u8> {
        let byte_rate = sample_rate * u32::from(channels) * (u32::from(bits_per_sample) / 8);
        let block_align = channels * (bits_per_sample / 8);
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
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());

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

    #[test]
    fn parse_wav_parses_16bit_mono_and_stereo_with_extra_chunks() -> Result<(), AudioError> {
        let mono_bytes = build_wav(
            1,
            1,
            44100,
            16,
            &[0, 16384, -16384, -32768],
            Some((b"JUNK", &[1, 2, 3])),
        );
        let mono = parse_wav(&mono_bytes)?;
        assert_eq!(mono.channels, 1);
        assert_eq!(mono.sample_rate, 44100);
        assert_eq!(mono.frame_count(), 4);
        assert!((mono.duration - (4.0 / 44100.0)).abs() < 1e-9);
        assert!((mono.samples[0] - 0.0).abs() < 1e-6);
        assert!((mono.samples[1] - 0.5).abs() < 1e-6);
        assert!((mono.samples[2] - (-0.5)).abs() < 1e-6);
        assert!((mono.samples[3] - (-1.0)).abs() < 1e-6);

        let stereo_bytes = build_wav(1, 2, 48000, 16, &[16384, -16384, 8192, -8192], None);
        let stereo = parse_wav(&stereo_bytes)?;
        assert_eq!(stereo.channels, 2);
        assert_eq!(stereo.sample_rate, 48000);
        assert_eq!(stereo.frame_count(), 2);
        assert!((stereo.duration - (2.0 / 48000.0)).abs() < 1e-9);
        assert_eq!(stereo.samples.len(), 4);
        Ok(())
    }

    #[test]
    fn parse_wav_rejects_ogg_mp3_and_non_pcm_wav_with_named_unsupported_format() {
        let ogg_res = parse_wav(b"OggS\x00\x02\x00\x00\x00\x00\x00\x00");
        assert!(
            matches!(ogg_res, Err(AudioError::UnsupportedFormat(ref msg)) if msg.contains("OGG"))
        );

        let mp3_id3_res = parse_wav(b"ID3\x04\x00\x00\x00\x00\x00\x00");
        assert!(
            matches!(mp3_id3_res, Err(AudioError::UnsupportedFormat(ref msg)) if msg.contains("MP3"))
        );

        let mp3_sync_res = parse_wav(&[0xFF, 0xFB, 0x90, 0x64, 0x00]);
        assert!(
            matches!(mp3_sync_res, Err(AudioError::UnsupportedFormat(ref msg)) if msg.contains("MP3"))
        );

        let float_wav = build_wav(3, 1, 44100, 16, &[0, 0], None);
        let float_res = parse_wav(&float_wav);
        assert!(
            matches!(float_res, Err(AudioError::UnsupportedFormat(ref msg)) if msg.contains('3'))
        );

        let bit24_wav = build_wav(1, 1, 44100, 24, &[0, 0, 0], None);
        let bit24_res = parse_wav(&bit24_wav);
        assert!(matches!(bit24_res, Err(AudioError::UnsupportedFormat(_))));

        let ch4_wav = build_wav(1, 4, 44100, 16, &[0, 0, 0, 0], None);
        let ch4_res = parse_wav(&ch4_wav);
        assert!(matches!(ch4_res, Err(AudioError::UnsupportedFormat(_))));
    }

    #[test]
    fn loop_wraps_with_no_sample_loss_and_pause_resume_and_pitch_work() {
        let mut world = AudioWorld::new();
        let clip = AudioClip {
            channels: 1,
            sample_rate: 4,
            samples: vec![0.25, 0.5, 0.75, 1.0],
            duration: 1.0,
        };
        let clip_id = world.register_clip(clip);
        let entity = EntityId::new(1);
        world.ensure_source(entity);
        world.set_source_clip(entity, Some(clip_id));
        world.set_looping(entity, true);
        world.play(entity, false);

        let mixed = world.mix(8);
        assert_eq!(mixed.len(), 16);
        let expected_frames = [0.25, 0.5, 0.75, 1.0, 0.25, 0.5, 0.75, 1.0];
        for (frame_idx, expected) in expected_frames.iter().enumerate() {
            let left = mixed[frame_idx * 2];
            let right = mixed[frame_idx * 2 + 1];
            assert!((left - expected).abs() < 1e-6);
            assert!((right - expected).abs() < 1e-6);
        }

        world.stop(entity);
        assert_eq!(world.source_state(entity), AudioSourceState::Stopped);
        world.set_looping(entity, false);
        world.play(entity, false);
        world.tick(0.25, 1.0);
        assert!((world.source_time(entity) - 0.25).abs() < 1e-9);

        world.pause(entity);
        assert_eq!(world.source_state(entity), AudioSourceState::Paused);
        world.tick(0.5, 1.0);
        assert!((world.source_time(entity) - 0.25).abs() < 1e-9);

        world.unpause(entity);
        assert_eq!(world.source_state(entity), AudioSourceState::Playing);
        world.set_pitch(entity, 2.0);
        world.tick(0.25, 1.0);
        assert!((world.source_time(entity) - 0.75).abs() < 1e-9);

        world.tick(0.25, 1.0);
        assert_eq!(world.source_state(entity), AudioSourceState::Completed);
        assert!((world.source_time(entity) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn time_scale_zero_freezes_scaled_source_while_unscaled_advances() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 100,
            samples: vec![0.5; 200],
            duration: 2.0,
        });

        let scaled = EntityId::new(10);
        let unscaled = EntityId::new(11);
        world.ensure_source(scaled);
        world.set_source_clip(scaled, Some(clip_id));
        world.play(scaled, false);

        world.ensure_source(unscaled);
        world.set_source_clip(unscaled, Some(clip_id));
        world.set_ignore_time_scale(unscaled, true);
        world.play(unscaled, false);

        world.tick(0.5, 0.0);
        assert!((world.source_time(scaled) - 0.0).abs() < 1e-9);
        assert!((world.source_time(unscaled) - 0.5).abs() < 1e-9);

        world.set_application_paused(true);
        world.tick(0.5, 1.0);
        assert!((world.source_time(scaled) - 0.0).abs() < 1e-9);
        assert!((world.source_time(unscaled) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn spatial_gain_and_pan_move_monotonically_and_2d_ignores_distance() {
        let mut world = AudioWorld::new();
        world.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);

        let src_3d = EntityId::new(20);
        world.ensure_source(src_3d);
        world.set_spatial_blend(src_3d, 1.0);
        world.set_min_distance(src_3d, 1.0);
        world.set_max_distance(src_3d, 11.0);
        world.play(src_3d, false);

        let distances = [0.0f32, 1.0, 3.5, 6.0, 8.5, 11.0];
        let mut prev_total = f32::MAX;
        for d in distances {
            world.set_source_position(src_3d, [0.0, 0.0, d]);
            let (left, right) = world.source_gains(src_3d).unwrap_or((0.0, 0.0));
            let total = left + right;
            assert!(total <= prev_total + 1e-6);
            prev_total = total;
        }
        assert!(prev_total.abs() < 1e-6);
        assert_eq!(
            world.inaudible_reason(src_3d),
            Some(SilenceReason::OutOfRange)
        );

        let azimuth_x = [-5.0f32, -2.0, 0.0, 2.0, 5.0];
        let mut prev_pan_ratio = -1.0f32;
        for x in azimuth_x {
            world.set_source_position(src_3d, [x, 0.0, 5.0]);
            let (left, right) = world.source_gains(src_3d).unwrap_or((0.0, 0.0));
            let total = left + right;
            assert!(total > 0.0);
            let pan_ratio = right / total;
            assert!(pan_ratio > prev_pan_ratio);
            prev_pan_ratio = pan_ratio;
        }
        world.set_source_position(src_3d, [-5.0, 0.0, 0.0]);
        let (full_l_left, full_l_right) = world.source_gains(src_3d).unwrap_or((0.0, 0.0));
        assert!(full_l_left > 0.0);
        assert!(full_l_right.abs() < 1e-6);
        world.set_source_position(src_3d, [5.0, 0.0, 0.0]);
        let (full_r_left, full_r_right) = world.source_gains(src_3d).unwrap_or((0.0, 0.0));
        assert!(full_r_left.abs() < 1e-6);
        assert!(full_r_right > 0.0);

        let src_2d = EntityId::new(21);
        world.ensure_source(src_2d);
        world.set_spatial_blend(src_2d, 0.0);
        world.set_min_distance(src_2d, 1.0);
        world.set_max_distance(src_2d, 10.0);
        world.set_source_position(src_2d, [1000.0, 0.0, 1000.0]);
        world.play(src_2d, false);
        let (l2d, r2d) = world.source_gains(src_2d).unwrap_or((0.0, 0.0));
        assert!((l2d - 1.0).abs() < 1e-6);
        assert!((r2d - 1.0).abs() < 1e-6);
        assert_eq!(world.inaudible_reason(src_2d), None);
    }

    #[test]
    fn voice_limit_virtualizes_lowest_priority_and_promotes_when_freed() {
        let mut world = AudioWorld::new();
        world.set_max_real_voices(2);

        let high = EntityId::new(1);
        let mid = EntityId::new(2);
        let low = EntityId::new(3);

        for (entity, prio) in [(high, 10u8), (mid, 128u8), (low, 200u8)] {
            world.ensure_source(entity);
            world.set_priority(entity, prio);
            world.play(entity, false);
        }

        assert_eq!(world.source_state(high), AudioSourceState::Playing);
        assert_eq!(world.source_state(mid), AudioSourceState::Playing);
        assert_eq!(world.source_state(low), AudioSourceState::Virtualized);
        assert_eq!(
            world.inaudible_reason(low),
            Some(SilenceReason::Virtualized)
        );

        world.stop(high);
        assert_eq!(world.source_state(mid), AudioSourceState::Playing);
        assert_eq!(world.source_state(low), AudioSourceState::Playing);
        assert_eq!(world.inaudible_reason(low), None);
    }

    #[test]
    fn silence_reasons_cover_all_distinct_variants() {
        let mut world = AudioWorld::new();
        let e = EntityId::new(100);
        world.ensure_source(e);

        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::Stopped));

        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 10,
            samples: vec![1.0; 10],
            duration: 1.0,
        });
        world.set_source_clip(e, Some(clip_id));
        world.play(e, false);
        assert_eq!(world.inaudible_reason(e), None);

        world.pause(e);
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::Paused));
        world.unpause(e);

        world.set_mute(e, true);
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::Muted));
        world.set_mute(e, false);

        world.set_volume(e, 0.0);
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::ZeroVolume));
        world.set_volume(e, 1.0);

        world.set_spatial_blend(e, 1.0);
        world.set_max_distance(e, 5.0);
        world.set_source_position(e, [0.0, 0.0, 10.0]);
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::OutOfRange));
        world.set_source_position(e, [0.0, 0.0, 0.0]);

        world.clear_listener();
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::NoListener));
        world.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);

        world.tick(1.5, 1.0);
        assert_eq!(world.inaudible_reason(e), Some(SilenceReason::Completed));

        world.mark_source_unsupported_clip(e);
        assert_eq!(world.source_state(e), AudioSourceState::Failed);
        assert_eq!(
            world.inaudible_reason(e),
            Some(SilenceReason::UnsupportedClip)
        );
    }
}
