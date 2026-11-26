use lewton::inside_ogg::OggStreamReader;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use unitas_asset::audio_import::{decode_aiff, decode_mp3, decode_ogg_vorbis, decode_wav};
use unitas_core::EntityId;

#[cfg(feature = "device")]
pub mod output;

pub type AudioClipId = u64;
pub type MusicStreamId = u64;

const DEFAULT_DOPPLER_FACTOR: f32 = 1.0;
const DEFAULT_SPEED_OF_SOUND: f32 = 343.0;
const MAX_DOPPLER_FACTOR: f32 = 5.0;
const MIN_SPEED_OF_SOUND: f32 = 1.0;
const MAX_EFFECTIVE_PITCH: f32 = 4.0;
const MIN_AXIS_DISTANCE: f32 = 1e-6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioMixerGroup {
    #[default]
    Sfx,
    Music,
    Menu,
}

impl AudioMixerGroup {
    const fn index(self) -> usize {
        match self {
            Self::Sfx => 0,
            Self::Music => 1,
            Self::Menu => 2,
        }
    }
}

pub struct MusicStream {
    data: Vec<u8>,
    reader: OggStreamReader<Cursor<Vec<u8>>>,
    channels: u16,
    sample_rate: u32,
    packet: Vec<f32>,
    packet_pos: usize,
}

impl MusicStream {
    fn new(bytes: &[u8]) -> Result<Self, AudioError> {
        let data = bytes.to_vec();
        let reader = OggStreamReader::new(Cursor::new(data.clone()))
            .map_err(|err| AudioError::UnsupportedFormat(err.to_string()))?;
        let channels = u16::from(reader.ident_hdr.audio_channels);
        let sample_rate = reader.ident_hdr.audio_sample_rate;
        if channels == 0 || channels > 2 {
            return Err(AudioError::UnsupportedFormat(format!(
                "{channels} channels (only mono or stereo supported)"
            )));
        }
        if sample_rate == 0 {
            return Err(AudioError::UnsupportedFormat(
                "zero sample rate".to_string(),
            ));
        }
        Ok(Self {
            data,
            reader,
            channels,
            sample_rate,
            packet: Vec::new(),
            packet_pos: 0,
        })
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn restart(&mut self) -> Result<(), AudioError> {
        self.reader = OggStreamReader::new(Cursor::new(self.data.clone()))
            .map_err(|err| AudioError::UnsupportedFormat(err.to_string()))?;
        self.packet.clear();
        self.packet_pos = 0;
        Ok(())
    }

    fn pull_packet(&mut self) -> Result<bool, AudioError> {
        match self.reader.read_dec_packet_itl() {
            Ok(Some(samples)) => {
                self.packet = samples
                    .iter()
                    .map(|sample| (f32::from(*sample) / 32768.0).clamp(-1.0, 1.0))
                    .collect();
                self.packet_pos = 0;
                Ok(true)
            }
            Ok(None) => Ok(false),
            Err(err) => Err(AudioError::UnsupportedFormat(err.to_string())),
        }
    }

    pub fn next_frame(&mut self, looping: bool) -> Option<(f32, f32)> {
        loop {
            let channel_count = usize::from(self.channels);
            if self.packet_pos + channel_count <= self.packet.len() {
                if channel_count == 1 {
                    let sample = self.packet[self.packet_pos];
                    self.packet_pos += 1;
                    return Some((sample, sample));
                }
                let left = self.packet[self.packet_pos];
                let right = self.packet[self.packet_pos + 1];
                self.packet_pos += 2;
                return Some((left, right));
            }
            match self.pull_packet() {
                Ok(true) => {}
                Ok(false) => {
                    if !looping {
                        return None;
                    }
                    if self.restart().is_err() {
                        return None;
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

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
    GameplayPaused = 9,
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

pub fn decode_audio(bytes: &[u8]) -> Result<AudioClip, AudioError> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return decode_shared(decode_wav(bytes));
    }
    if bytes.starts_with(b"OggS") {
        return decode_shared(decode_ogg_vorbis(bytes));
    }
    if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
    {
        return decode_shared(decode_mp3(bytes));
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"FORM" && &bytes[8..12] == b"AIFF" {
        return decode_shared(decode_aiff(bytes));
    }
    Err(AudioError::UnsupportedFormat(
        "unrecognized audio container".to_string(),
    ))
}

fn decode_shared(
    decoded: Result<
        unitas_asset::audio_import::PcmClip,
        unitas_asset::audio_import::AudioImportError,
    >,
) -> Result<AudioClip, AudioError> {
    let clip = decoded.map_err(|err| AudioError::UnsupportedFormat(err.to_string()))?;
    if clip.channels > 2 {
        return Err(AudioError::UnsupportedFormat(format!(
            "{} channels (only mono or stereo supported)",
            clip.channels
        )));
    }
    let samples = clip
        .samples
        .iter()
        .map(|sample| (f32::from(*sample) / 32768.0).clamp(-1.0, 1.0))
        .collect();
    Ok(AudioClip::new(clip.channels, clip.sample_rate, samples))
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
    pub velocity: Option<[f32; 3]>,
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
    pub group: AudioMixerGroup,
    pub music: Option<MusicStreamId>,
    pub velocity: [f32; 3],
    pub scheduled_sample: Option<u64>,
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
            group: AudioMixerGroup::Sfx,
            music: None,
            velocity: [0.0; 3],
            scheduled_sample: None,
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
    master_volume: f32,
    group_volumes: [f32; 3],
    gameplay_paused: bool,
    music_streams: BTreeMap<MusicStreamId, MusicStream>,
    next_music_stream_id: MusicStreamId,
    mixer_sample: u64,
    doppler_factor: f32,
    speed_of_sound: f32,
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
                velocity: None,
            }),
            max_real_voices: 32,
            application_paused: false,
            master_volume: 1.0,
            group_volumes: [1.0; 3],
            gameplay_paused: false,
            music_streams: BTreeMap::new(),
            next_music_stream_id: 1,
            mixer_sample: 0,
            doppler_factor: DEFAULT_DOPPLER_FACTOR,
            speed_of_sound: DEFAULT_SPEED_OF_SOUND,
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
        self.listener = Some(ListenerPose {
            position,
            forward,
            velocity: None,
        });
    }

    pub fn set_listener_with_velocity(
        &mut self,
        position: [f32; 3],
        forward: [f32; 3],
        velocity: [f32; 3],
    ) {
        self.listener = Some(ListenerPose {
            position,
            forward,
            velocity: Some(velocity),
        });
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

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn set_group_volume(&mut self, group: AudioMixerGroup, volume: f32) {
        self.group_volumes[group.index()] = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn group_volume(&self, group: AudioMixerGroup) -> f32 {
        self.group_volumes[group.index()]
    }

    pub fn set_gameplay_paused(&mut self, paused: bool) {
        self.gameplay_paused = paused;
    }

    pub fn is_gameplay_paused(&self) -> bool {
        self.gameplay_paused
    }

    pub fn set_source_group(&mut self, entity: EntityId, group: AudioMixerGroup) {
        let source = self.ensure_source(entity);
        source.group = group;
    }

    pub fn source_group(&self, entity: EntityId) -> AudioMixerGroup {
        self.sources
            .get(&entity)
            .map(|s| s.group)
            .unwrap_or_default()
    }

    fn bus_gain(&self, group: AudioMixerGroup) -> f32 {
        if self.gameplay_paused && group != AudioMixerGroup::Menu {
            return 0.0;
        }
        self.master_volume * self.group_volumes[group.index()]
    }

    fn bus_is_muted(&self, group: AudioMixerGroup) -> bool {
        self.master_volume <= 0.0 || self.group_volumes[group.index()] <= 0.0
    }

    pub fn load_music_bytes(&mut self, bytes: &[u8]) -> Result<MusicStreamId, AudioError> {
        let stream = MusicStream::new(bytes)?;
        let id = self.next_music_stream_id;
        self.next_music_stream_id += 1;
        self.music_streams.insert(id, stream);
        Ok(id)
    }

    pub fn load_music_file(&mut self, path: &Path) -> Result<MusicStreamId, AudioError> {
        let bytes = std::fs::read(path).map_err(|err| AudioError::Io(err.to_string()))?;
        self.load_music_bytes(&bytes)
    }

    pub fn music_stream(&self, stream_id: MusicStreamId) -> Option<&MusicStream> {
        self.music_streams.get(&stream_id)
    }

    pub fn set_source_music(&mut self, entity: EntityId, stream: Option<MusicStreamId>) {
        let valid = stream
            .map(|id| self.music_streams.contains_key(&id))
            .unwrap_or(true);
        let source = self.ensure_source(entity);
        if !valid {
            source.music = None;
            return;
        }
        source.music = stream;
        if stream.is_some() {
            source.clip = None;
        }
        source.time = 0.0;
        source.sample_cursor = 0.0;
        if source.state == AudioSourceState::Completed {
            source.state = AudioSourceState::Stopped;
        }
    }

    pub fn mixer_sample(&self) -> u64 {
        self.mixer_sample
    }

    pub fn play(&mut self, entity: EntityId, one_shot: bool) {
        self.start_playback(entity, one_shot, None);
    }

    pub fn play_scheduled(&mut self, entity: EntityId, one_shot: bool, start_sample: u64) {
        self.start_playback(entity, one_shot, Some(start_sample));
    }

    fn start_playback(&mut self, entity: EntityId, one_shot: bool, scheduled_sample: Option<u64>) {
        self.ensure_source(entity);
        let duration = self.effective_duration(entity);
        let sample_rate = self.source_sample_rate(entity);
        let source = self.ensure_source(entity);
        if source.unsupported_clip {
            source.state = AudioSourceState::Failed;
            source.wants_play = false;
            source.scheduled_sample = None;
            return;
        }
        let was_finished = source.state == AudioSourceState::Completed
            || source.state == AudioSourceState::Stopped;
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
        source.scheduled_sample = scheduled_sample;
        source.state = if scheduled_sample.is_some() {
            AudioSourceState::Scheduled
        } else {
            AudioSourceState::Playing
        };
        let restart_music = if was_finished { source.music } else { None };
        if let Some(stream_id) = restart_music {
            if let Some(stream) = self.music_streams.get_mut(&stream_id) {
                if stream.restart().is_ok() {
                    let source = self.ensure_source(entity);
                    source.time = 0.0;
                    source.sample_cursor = 0.0;
                }
            }
        }
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
        source.scheduled_sample = None;
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

    pub fn set_doppler_factor(&mut self, factor: f32) {
        if factor.is_finite() {
            self.doppler_factor = factor.clamp(0.0, MAX_DOPPLER_FACTOR);
        }
    }

    pub fn doppler_factor(&self) -> f32 {
        self.doppler_factor
    }

    pub fn set_speed_of_sound(&mut self, speed: f32) {
        if speed.is_finite() {
            self.speed_of_sound = speed.max(MIN_SPEED_OF_SOUND);
        }
    }

    pub fn speed_of_sound(&self) -> f32 {
        self.speed_of_sound
    }

    pub fn set_source_velocity(&mut self, entity: EntityId, velocity: [f32; 3]) {
        let source = self.ensure_source(entity);
        if velocity.iter().all(|value| value.is_finite()) {
            source.velocity = velocity;
        }
    }

    pub fn source_velocity(&self, entity: EntityId) -> [f32; 3] {
        self.sources
            .get(&entity)
            .map(|s| s.velocity)
            .unwrap_or([0.0; 3])
    }

    pub fn doppler_multiplier(&self, entity: EntityId) -> f32 {
        let Some(source) = self.sources.get(&entity) else {
            return 1.0;
        };
        let Some(listener) = self.listener else {
            return 1.0;
        };
        let listener_velocity = listener.velocity.unwrap_or([0.0; 3]);
        if !source.velocity.iter().all(|value| value.is_finite())
            || !listener_velocity.iter().all(|value| value.is_finite())
        {
            return 1.0;
        }
        let offset = [
            listener.position[0] - source.position[0],
            listener.position[1] - source.position[1],
            listener.position[2] - source.position[2],
        ];
        let distance =
            (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        if distance <= MIN_AXIS_DISTANCE {
            return 1.0;
        }
        let axis = [
            offset[0] / distance,
            offset[1] / distance,
            offset[2] / distance,
        ];
        let relative = [
            source.velocity[0] - listener_velocity[0],
            source.velocity[1] - listener_velocity[1],
            source.velocity[2] - listener_velocity[2],
        ];
        let radial = relative[0] * axis[0] + relative[1] * axis[1] + relative[2] * axis[2];
        let clamped = radial.clamp(-self.speed_of_sound, self.speed_of_sound);
        self.doppler_factor * (1.0 + clamped / self.speed_of_sound)
    }

    pub fn effective_pitch(&self, entity: EntityId) -> f32 {
        let Some(source) = self.sources.get(&entity) else {
            return 1.0;
        };
        let base = source.pitch.max(0.0);
        (base * self.doppler_multiplier(entity)).clamp(0.0, MAX_EFFECTIVE_PITCH)
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
        if self.gameplay_paused && source.group != AudioMixerGroup::Menu {
            return Some(SilenceReason::GameplayPaused);
        }
        if self.bus_is_muted(source.group) {
            return Some(SilenceReason::Muted);
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
            let pitch = f64::from(self.effective_pitch(entity)).max(0.0);
            let Some(source) = self.sources.get_mut(&entity) else {
                continue;
            };
            if source.state != AudioSourceState::Playing
                && source.state != AudioSourceState::Virtualized
            {
                continue;
            }
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
        let start_sample = self.mixer_sample;
        let end_sample = start_sample + n_frames as u64;

        if self.advance_scheduled_sources(start_sample, end_sample) {
            self.recompute_virtualization();
        }

        for entity in &entities {
            let Some(source_summary) = self.sources.get(entity) else {
                continue;
            };
            let render_offset = match source_summary.scheduled_sample {
                Some(scheduled) if scheduled > start_sample => (scheduled - start_sample) as usize,
                _ => 0,
            };
            if render_offset >= n_frames {
                continue;
            }
            if self.inaudible_reason(*entity).is_some() {
                continue;
            }
            let Some((gain_l, gain_r)) = self.source_gains(*entity) else {
                continue;
            };
            let bus = self.bus_gain(source_summary.group);
            if bus <= 0.0 {
                continue;
            }
            let (gain_l, gain_r) = (gain_l * bus, gain_r * bus);
            if let Some(stream_id) = source_summary.music {
                let Some(stream) = self.music_streams.get_mut(&stream_id) else {
                    continue;
                };
                let Some(source) = self.sources.get_mut(entity) else {
                    continue;
                };
                let looping = source.looping && !source.one_shot_active;
                for frame_out in render_offset..n_frames {
                    let Some((sample_l, sample_r)) = stream.next_frame(looping) else {
                        source.wants_play = false;
                        source.one_shot_active = false;
                        source.state = AudioSourceState::Completed;
                        any_completed = true;
                        break;
                    };
                    output[frame_out * 2] += sample_l * gain_l;
                    output[frame_out * 2 + 1] += sample_r * gain_r;
                    source.sample_cursor += 1.0;
                    source.time = source.sample_cursor / f64::from(stream.sample_rate());
                }
                continue;
            }
            let Some(clip_id) = source_summary.clip else {
                continue;
            };
            let Some(clip) = self.clips.get(&clip_id).cloned() else {
                continue;
            };
            let total_frames = clip.frame_count();
            if total_frames == 0 || clip.sample_rate == 0 {
                continue;
            }
            let step = f64::from(self.effective_pitch(*entity)).max(0.0);
            let Some(source) = self.sources.get_mut(entity) else {
                continue;
            };

            for frame_out in render_offset..n_frames {
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

        self.mixer_sample = end_sample;
        if any_completed {
            self.recompute_virtualization();
        }
        output
    }

    fn advance_scheduled_sources(&mut self, start_sample: u64, end_sample: u64) -> bool {
        let mut promoted = false;
        for source in self.sources.values_mut() {
            let Some(scheduled) = source.scheduled_sample else {
                continue;
            };
            if source.state == AudioSourceState::Scheduled && scheduled < end_sample {
                source.state = AudioSourceState::Playing;
                promoted = true;
            }
            if scheduled < start_sample {
                source.scheduled_sample = None;
            }
        }
        promoted
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

    const OGG_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/sine_mono.ogg");

    fn reference_ogg_samples() -> Result<(u16, Vec<i16>), AudioError> {
        let clip = decode_ogg_vorbis(OGG_FIXTURE)
            .map_err(|err| AudioError::UnsupportedFormat(err.to_string()))?;
        Ok((clip.channels, clip.samples))
    }

    #[test]
    fn streamed_ogg_loop_repeats_with_no_gap_or_overrun_at_the_seam() -> Result<(), AudioError> {
        let (channels, reference) = reference_ogg_samples()?;
        assert_eq!(channels, 1);
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
            assert_eq!(mixed[frame * 2], expected, "left sample at frame {frame}");
            assert_eq!(
                mixed[frame * 2 + 1],
                expected,
                "right sample at frame {frame}"
            );
        }
        assert_eq!(world.source_state(entity), AudioSourceState::Playing);

        world.set_gameplay_paused(true);
        let silenced = world.mix(total_frames);
        assert!(silenced.iter().all(|sample| *sample == 0.0));
        world.set_gameplay_paused(false);
        Ok(())
    }

    #[test]
    fn streamed_ogg_without_loop_completes_pads_silence_and_replays_from_start(
    ) -> Result<(), AudioError> {
        let (channels, reference) = reference_ogg_samples()?;
        let total_frames = reference.len() / usize::from(channels);

        let mut world = AudioWorld::new();
        let stream_id = world.load_music_bytes(OGG_FIXTURE)?;
        let entity = EntityId::new(2);
        world.ensure_source(entity);
        world.set_source_music(entity, Some(stream_id));
        world.set_source_group(entity, AudioMixerGroup::Music);
        world.play(entity, false);

        let mixed = world.mix(total_frames + 8);
        assert_eq!(mixed.len(), (total_frames + 8) * 2);
        for tail in 0..8 {
            let index = (total_frames + tail) * 2;
            assert_eq!(mixed[index], 0.0);
            assert_eq!(mixed[index + 1], 0.0);
        }
        assert_eq!(world.source_state(entity), AudioSourceState::Completed);

        world.play(entity, false);
        assert_eq!(world.source_state(entity), AudioSourceState::Playing);
        let remixed = world.mix(4);
        for frame in 0..4.min(total_frames) {
            let expected = f32::from(reference[frame]) / 32768.0;
            assert_eq!(remixed[frame * 2], expected);
        }
        Ok(())
    }

    #[test]
    fn group_gains_apply_only_to_their_own_bus() {
        let mut world = AudioWorld::new();
        assert_eq!(world.master_volume(), 1.0);
        assert_eq!(world.group_volume(AudioMixerGroup::Sfx), 1.0);

        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 100,
            samples: vec![0.5; 400],
            duration: 4.0,
        });
        let sfx = EntityId::new(10);
        let menu = EntityId::new(11);
        for entity in [sfx, menu] {
            world.ensure_source(entity);
            world.set_source_clip(entity, Some(clip_id));
            world.play(entity, false);
        }
        world.set_source_group(menu, AudioMixerGroup::Menu);
        assert_eq!(world.source_group(menu), AudioMixerGroup::Menu);
        assert_eq!(world.source_group(sfx), AudioMixerGroup::Sfx);

        let baseline = world.mix(4);

        world.set_group_volume(AudioMixerGroup::Music, 0.1);
        world.set_group_volume(AudioMixerGroup::Menu, 0.5);
        let after_menu = world.mix(4);
        for frame in 0..4 {
            assert!((after_menu[frame * 2] - 0.75).abs() < 1e-6);
            assert!((after_menu[frame * 2 + 1] - 0.75).abs() < 1e-6);
        }

        world.set_group_volume(AudioMixerGroup::Sfx, 0.25);
        let after_sfx = world.mix(4);
        for frame in 0..4 {
            assert!((after_sfx[frame * 2] - (0.125 + 0.25)).abs() < 1e-6);
        }

        world.set_master_volume(0.5);
        let after_master = world.mix(4);
        for frame in 0..4 {
            assert!((after_master[frame * 2] - (0.0625 + 0.125)).abs() < 1e-6);
        }

        world.set_group_volume(AudioMixerGroup::Sfx, 1.0);
        world.set_group_volume(AudioMixerGroup::Menu, 1.0);
        world.set_master_volume(1.0);
        let restored = world.mix(4);
        for frame in 0..4 {
            assert!((restored[frame * 2] - baseline[frame * 2]).abs() < 1e-6);
        }
    }

    #[test]
    fn gameplay_pause_silences_music_and_sfx_keeps_menu_and_freezes_gameplay() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 100,
            samples: vec![0.1, 0.2, 0.3, 0.4, 0.0, 0.0, 0.0, 0.0],
            duration: 0.08,
        });
        let music_entity = EntityId::new(20);
        let sfx_entity = EntityId::new(21);
        let menu_entity = EntityId::new(22);
        for entity in [music_entity, sfx_entity, menu_entity] {
            world.ensure_source(entity);
            world.set_source_clip(entity, Some(clip_id));
        }
        world.set_source_group(music_entity, AudioMixerGroup::Music);
        world.set_source_group(menu_entity, AudioMixerGroup::Menu);
        for entity in [music_entity, sfx_entity, menu_entity] {
            world.play(entity, false);
        }

        let active = world.mix(2);
        for frame in 0..2 {
            let expected = 3.0 * [0.1f32, 0.2][frame];
            assert!((active[frame * 2] - expected).abs() < 1e-6);
        }

        world.set_gameplay_paused(true);
        assert!(world.is_gameplay_paused());
        assert_eq!(
            world.inaudible_reason(music_entity),
            Some(SilenceReason::GameplayPaused)
        );
        assert_eq!(
            world.inaudible_reason(sfx_entity),
            Some(SilenceReason::GameplayPaused)
        );
        assert_eq!(world.inaudible_reason(menu_entity), None);
        let paused = world.mix(2);
        for frame in 0..2 {
            let menu_expected = [0.3f32, 0.4][frame];
            assert!((paused[frame * 2] - menu_expected).abs() < 1e-6);
            assert!((paused[frame * 2 + 1] - menu_expected).abs() < 1e-6);
        }

        world.set_gameplay_paused(false);
        assert!(!world.is_gameplay_paused());
        assert_eq!(world.inaudible_reason(music_entity), None);
        assert_eq!(world.inaudible_reason(sfx_entity), None);
        let resumed = world.mix(2);
        for frame in 0..2 {
            let expected = 2.0 * [0.3f32, 0.4][frame];
            assert!((resumed[frame * 2] - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn doppler_raises_pitch_for_an_approaching_source_and_lowers_it_for_a_receding_one() {
        let mut world = AudioWorld::new();
        world.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let entity = EntityId::new(50);
        world.ensure_source(entity);
        world.set_source_position(entity, [0.0, 0.0, -20.0]);

        world.set_source_velocity(entity, [0.0, 0.0, 100.0]);
        let approaching = world.doppler_multiplier(entity);
        assert!(approaching > 1.0);
        assert!((world.effective_pitch(entity) - approaching).abs() < 1e-6);
        assert!((world.effective_pitch(entity) - world.pitch(entity) * approaching).abs() < 1e-6);

        world.set_source_velocity(entity, [0.0, 0.0, -100.0]);
        let receding = world.doppler_multiplier(entity);
        assert!(receding < 1.0);
        assert!(receding > 0.0);
        assert!((world.effective_pitch(entity) - receding).abs() < 1e-6);

        world.set_source_velocity(entity, [0.0, 0.0, 0.0]);
        assert!((world.doppler_multiplier(entity) - 1.0).abs() < 1e-6);

        world.set_listener_with_velocity([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 100.0]);
        let listener_receding = world.doppler_multiplier(entity);
        assert!(listener_receding < 1.0);
        assert!((listener_receding - receding).abs() < 1e-6);
    }

    #[test]
    fn doppler_pitch_shift_changes_the_mixed_playback_rate() {
        let mut world = AudioWorld::new();
        world.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 100,
            samples: vec![0.5; 1000],
            duration: 10.0,
        });
        let entity = EntityId::new(51);
        world.ensure_source(entity);
        world.set_source_clip(entity, Some(clip_id));
        world.set_source_position(entity, [0.0, 0.0, -20.0]);
        world.set_source_velocity(entity, [0.0, 0.0, 343.0]);
        world.play(entity, false);

        let multiplier = world.doppler_multiplier(entity);
        assert!((multiplier - 2.0).abs() < 1e-6);

        world.mix(10);
        let expected_time = 10.0 * f64::from(multiplier) / 100.0;
        assert!((world.source_time(entity) - expected_time).abs() < 1e-6);
    }

    #[test]
    fn scheduled_playback_starts_at_the_scheduled_mixer_sample() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 8,
            samples: vec![1.0; 8],
            duration: 1.0,
        });
        let entity = EntityId::new(52);
        world.ensure_source(entity);
        world.set_source_clip(entity, Some(clip_id));

        world.mix(2);
        assert_eq!(world.mixer_sample(), 2);
        let scheduled_at = world.mixer_sample() + 3;
        world.play_scheduled(entity, false, scheduled_at);
        assert_eq!(world.source_state(entity), AudioSourceState::Scheduled);
        assert_eq!(world.inaudible_reason(entity), Some(SilenceReason::Stopped));

        let mixed = world.mix(6);
        assert_eq!(mixed.len(), 12);
        for frame in 0..3 {
            assert_eq!(mixed[frame * 2], 0.0);
            assert_eq!(mixed[frame * 2 + 1], 0.0);
        }
        for frame in 3..6 {
            assert_eq!(mixed[frame * 2], 1.0);
            assert_eq!(mixed[frame * 2 + 1], 1.0);
        }
        assert_eq!(world.source_state(entity), AudioSourceState::Playing);
        assert_eq!(world.mixer_sample(), 8);
    }

    #[test]
    fn sources_scheduled_at_adjacent_samples_render_without_gap_or_overlap() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 8,
            samples: vec![0.25; 8],
            duration: 1.0,
        });
        let first = EntityId::new(53);
        let second = EntityId::new(54);
        for entity in [first, second] {
            world.ensure_source(entity);
            world.set_source_clip(entity, Some(clip_id));
        }

        world.play_scheduled(first, false, world.mixer_sample());
        world.play_scheduled(second, false, world.mixer_sample() + 1);

        let mixed = world.mix(4);
        assert_eq!(mixed[0], 0.25);
        assert!((mixed[2] - 0.5).abs() < 1e-6);
        assert!((mixed[4] - 0.5).abs() < 1e-6);
        assert!((mixed[6] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn muted_mixer_group_reports_the_muted_inaudible_reason() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 10,
            samples: vec![0.5; 10],
            duration: 1.0,
        });
        let entity = EntityId::new(55);
        world.ensure_source(entity);
        world.set_source_clip(entity, Some(clip_id));
        world.set_source_group(entity, AudioMixerGroup::Music);
        world.play(entity, false);
        assert_eq!(world.inaudible_reason(entity), None);

        world.set_group_volume(AudioMixerGroup::Music, 0.0);
        assert_eq!(world.inaudible_reason(entity), Some(SilenceReason::Muted));
        assert!(world.mix(4).iter().all(|sample| *sample == 0.0));

        world.set_group_volume(AudioMixerGroup::Music, 1.0);
        assert_eq!(world.inaudible_reason(entity), None);
        world.set_master_volume(0.0);
        assert_eq!(world.inaudible_reason(entity), Some(SilenceReason::Muted));
    }

    fn chime_bus_world() -> (AudioWorld, EntityId, EntityId) {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(AudioClip {
            channels: 1,
            sample_rate: 8,
            samples: vec![0.5, 0.25, 0.125, 0.0625],
            duration: 0.5,
        });
        let sfx = EntityId::new(60);
        let menu = EntityId::new(61);
        for entity in [sfx, menu] {
            world.ensure_source(entity);
            world.set_source_clip(entity, Some(clip_id));
            world.play(entity, true);
        }
        world.set_source_group(sfx, AudioMixerGroup::Sfx);
        world.set_source_group(menu, AudioMixerGroup::Menu);
        (world, sfx, menu)
    }

    #[test]
    fn chime_mixes_through_the_sfx_bus_with_group_volume_and_gameplay_pause() {
        let (mut world, _sfx, _menu) = chime_bus_world();
        let mixed = world.mix(4);
        assert!((mixed[0] - 1.0).abs() < 1e-6);
        assert!((mixed[2] - 0.5).abs() < 1e-6);

        let (mut world, _sfx, _menu) = chime_bus_world();
        world.set_group_volume(AudioMixerGroup::Sfx, 0.25);
        let mixed = world.mix(4);
        assert!((mixed[0] - 0.625).abs() < 1e-6);
        assert!((mixed[2] - 0.3125).abs() < 1e-6);

        let (mut world, sfx, _menu) = chime_bus_world();
        world.set_group_volume(AudioMixerGroup::Sfx, 0.0);
        assert_eq!(world.inaudible_reason(sfx), Some(SilenceReason::Muted));
        let mixed = world.mix(4);
        assert!((mixed[0] - 0.5).abs() < 1e-6);
        assert!((mixed[2] - 0.25).abs() < 1e-6);

        let (mut world, sfx, menu) = chime_bus_world();
        world.set_gameplay_paused(true);
        assert_eq!(
            world.inaudible_reason(sfx),
            Some(SilenceReason::GameplayPaused)
        );
        assert_eq!(world.inaudible_reason(menu), None);
        let mixed = world.mix(4);
        assert!((mixed[0] - 0.5).abs() < 1e-6);
        assert!((mixed[2] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn shipped_collect_chime_asset_decodes_and_mixes_audible_samples() -> Result<(), AudioError> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/roll-a-ball/Assets/Audio/PickupChime.wav");
        let bytes = std::fs::read(&path).map_err(|err| AudioError::Io(err.to_string()))?;
        let clip = parse_wav(&bytes)?;
        assert!(clip.frame_count() > 0);
        assert!(clip.samples.iter().any(|sample| sample.abs() > 0.1));

        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(clip);
        let entity = EntityId::new(70);
        world.ensure_source(entity);
        world.set_source_clip(entity, Some(clip_id));
        world.play(entity, true);
        assert_eq!(world.inaudible_reason(entity), None);

        let mixed = world.mix(1024);
        assert!(mixed.iter().any(|sample| sample.abs() > 0.01));
        Ok(())
    }
}
