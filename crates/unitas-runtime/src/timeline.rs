use std::collections::{BTreeMap, BTreeSet};

use unitas_core::EntityId;

use crate::animation::AnimationWorld;
use crate::cinemachine::{CameraBlend, CinemachineWorld};

const TIME_EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineWrapMode {
    Once,
    Loop,
    PingPong,
    Hold,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineTrackKind {
    Animation,
    Audio,
    Activation,
    Signal,
    CinemachineShot,
}

impl TimelineTrackKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Animation => "animation",
            Self::Audio => "audio",
            Self::Activation => "activation",
            Self::Signal => "signal",
            Self::CinemachineShot => "cinemachine shot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Complete,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineClipPayload {
    Animation {
        entity: EntityId,
        clip: String,
    },
    Audio {
        clip: String,
        volume: f32,
    },
    Activation {
        entity: EntityId,
        active: bool,
    },
    Signal {
        signal: String,
        receiver: String,
        retroactive: bool,
        emit_once: bool,
    },
    CinemachineShot {
        camera: String,
        blend: Option<CameraBlend>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineClip {
    pub name: String,
    pub start: f64,
    pub duration: f64,
    pub payload: TimelineClipPayload,
}

impl TimelineClip {
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTrack {
    pub kind: TimelineTrackKind,
    pub clips: Vec<TimelineClip>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineSequence {
    pub name: String,
    pub tracks: Vec<TimelineTrack>,
}

impl TimelineSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tracks: Vec::new(),
        }
    }

    pub fn duration(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .map(|clip| clip.end())
            .fold(0.0, f64::max)
    }

    pub fn clip_count(&self) -> usize {
        self.tracks.iter().map(|track| track.clips.len()).sum()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSignalEvent {
    pub signal: String,
    pub receiver: String,
    pub time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineShot {
    pub camera: String,
    pub blend: Option<CameraBlend>,
    pub track_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineAudioState {
    pub clip: String,
    pub volume: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineAnimationState {
    pub entity: EntityId,
    pub clip: String,
    pub local_time: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineDiagnostic {
    MissingBinding {
        track: TimelineTrackKind,
        track_index: usize,
        target: String,
    },
}

impl std::fmt::Display for TimelineDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBinding {
                track,
                track_index,
                target,
            } => write!(
                formatter,
                "{} track {track_index} has no binding for '{target}'",
                track.label()
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineError {
    UnknownSequence(u64),
    UnknownPlayer(u64),
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSequence(handle) => {
                write!(formatter, "timeline sequence {handle} not found")
            }
            Self::UnknownPlayer(handle) => write!(formatter, "timeline player {handle} not found"),
        }
    }
}

impl std::error::Error for TimelineError {}

#[derive(Clone, Debug)]
struct SignalSpec {
    time: f64,
    signal: String,
    receiver: String,
    retroactive: bool,
    emit_once: bool,
}

#[derive(Clone, Debug)]
pub struct TimelinePlayer {
    sequence: TimelineSequence,
    wrap_mode: TimelineWrapMode,
    time: f64,
    state: PlaybackState,
    direction: f64,
    signals: Vec<SignalSpec>,
    emitted_once: Vec<bool>,
    signal_events: Vec<TimelineSignalEvent>,
    stopped_count: u32,
    wraps: u64,
}

impl TimelinePlayer {
    pub fn new(sequence: TimelineSequence, wrap_mode: TimelineWrapMode) -> Self {
        let signals = collect_signals(&sequence);
        let emitted_once = vec![false; signals.len()];
        Self {
            sequence,
            wrap_mode,
            time: 0.0,
            state: PlaybackState::Stopped,
            direction: 1.0,
            signals,
            emitted_once,
            signal_events: Vec::new(),
            stopped_count: 0,
            wraps: 0,
        }
    }

    pub fn sequence(&self) -> &TimelineSequence {
        &self.sequence
    }

    pub fn duration(&self) -> f64 {
        self.sequence.duration()
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn wrap_mode(&self) -> TimelineWrapMode {
        self.wrap_mode
    }

    pub fn set_wrap_mode(&mut self, wrap_mode: TimelineWrapMode) {
        self.wrap_mode = wrap_mode;
    }

    pub fn direction(&self) -> f64 {
        self.direction
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    pub fn is_complete(&self) -> bool {
        self.state == PlaybackState::Complete
    }

    pub fn at_end(&self) -> bool {
        let duration = self.duration();
        duration > 0.0 && self.time >= duration - TIME_EPSILON
    }

    pub fn wraps(&self) -> u64 {
        self.wraps
    }

    pub fn stopped_count(&self) -> u32 {
        self.stopped_count
    }

    pub fn signal_events(&self) -> &[TimelineSignalEvent] {
        &self.signal_events
    }

    pub fn play(&mut self) {
        match self.state {
            PlaybackState::Playing => {}
            PlaybackState::Paused => self.state = PlaybackState::Playing,
            PlaybackState::Stopped => {
                self.state = PlaybackState::Playing;
                self.fire_arrival(self.time);
                self.fire_retroactive(self.time);
            }
            PlaybackState::Complete => {
                self.time = 0.0;
                self.direction = 1.0;
                self.state = PlaybackState::Playing;
                self.fire_arrival(0.0);
                self.fire_retroactive(0.0);
            }
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.time = 0.0;
        self.direction = 1.0;
    }

    pub fn seek(&mut self, time: f64) {
        if !time.is_finite() {
            return;
        }
        self.time = time.clamp(0.0, self.duration());
    }

    pub fn update(&mut self, delta: f64) {
        if self.state != PlaybackState::Playing {
            return;
        }
        if !delta.is_finite() || delta <= 0.0 {
            return;
        }
        let duration = self.duration();
        if duration <= 0.0 {
            return;
        }
        match self.wrap_mode {
            TimelineWrapMode::Once | TimelineWrapMode::None => self.advance_once(delta, duration),
            TimelineWrapMode::Hold => self.advance_hold(delta, duration),
            TimelineWrapMode::Loop => self.advance_loop(delta, duration),
            TimelineWrapMode::PingPong => self.advance_pingpong(delta, duration),
        }
    }

    pub fn active_animation_clips(&self) -> Vec<TimelineAnimationState> {
        let mut states = Vec::new();
        for track in &self.sequence.tracks {
            if track.kind != TimelineTrackKind::Animation {
                continue;
            }
            for clip in &track.clips {
                let TimelineClipPayload::Animation { entity, clip: name } = &clip.payload else {
                    continue;
                };
                if !self.clip_is_active(clip) {
                    continue;
                }
                states.push(TimelineAnimationState {
                    entity: *entity,
                    clip: name.clone(),
                    local_time: self.time - clip.start,
                });
            }
        }
        states
    }

    pub fn active_audio_clips(&self) -> Vec<TimelineAudioState> {
        let mut states = Vec::new();
        for track in &self.sequence.tracks {
            if track.kind != TimelineTrackKind::Audio {
                continue;
            }
            for clip in &track.clips {
                let TimelineClipPayload::Audio { clip: name, volume } = &clip.payload else {
                    continue;
                };
                if !self.clip_is_active(clip) {
                    continue;
                }
                states.push(TimelineAudioState {
                    clip: name.clone(),
                    volume: *volume,
                });
            }
        }
        states
    }

    pub fn active_activation_clips(&self) -> Vec<(EntityId, bool)> {
        let mut states = Vec::new();
        for track in &self.sequence.tracks {
            if track.kind != TimelineTrackKind::Activation {
                continue;
            }
            for clip in &track.clips {
                let TimelineClipPayload::Activation { entity, active } = &clip.payload else {
                    continue;
                };
                if !self.clip_is_active(clip) {
                    continue;
                }
                states.push((*entity, *active));
            }
        }
        states
    }

    pub fn active_shot(&self) -> Option<TimelineShot> {
        let mut shot = None;
        for (track_index, track) in self.sequence.tracks.iter().enumerate() {
            if track.kind != TimelineTrackKind::CinemachineShot {
                continue;
            }
            for clip in &track.clips {
                let TimelineClipPayload::CinemachineShot { camera, blend } = &clip.payload else {
                    continue;
                };
                if !self.clip_is_active(clip) {
                    continue;
                }
                shot = Some(TimelineShot {
                    camera: camera.clone(),
                    blend: *blend,
                    track_index,
                });
            }
        }
        shot
    }

    fn clip_is_active(&self, clip: &TimelineClip) -> bool {
        clip.start <= self.time && self.time < clip.end()
    }

    fn advance_once(&mut self, delta: f64, duration: f64) {
        let from = self.time;
        let to = (from + delta).min(duration);
        self.fire_forward(from, to);
        self.time = to;
        if to >= duration - TIME_EPSILON {
            self.time = duration;
            self.state = PlaybackState::Complete;
            self.stopped_count += 1;
        }
    }

    fn advance_hold(&mut self, delta: f64, duration: f64) {
        let from = self.time;
        let to = (from + delta).min(duration);
        self.fire_forward(from, to);
        self.time = to;
        if to >= duration - TIME_EPSILON {
            self.time = duration;
        }
    }

    fn advance_loop(&mut self, delta: f64, duration: f64) {
        let mut remaining = delta;
        let mut from = self.time;
        loop {
            let to_end = duration - from;
            if remaining < to_end - TIME_EPSILON {
                let to = from + remaining;
                self.fire_forward(from, to);
                self.time = to;
                return;
            }
            self.fire_forward(from, duration);
            remaining -= to_end;
            self.wraps += 1;
            self.time = 0.0;
            self.fire_arrival(0.0);
            from = 0.0;
            if remaining <= TIME_EPSILON {
                return;
            }
        }
    }

    fn advance_pingpong(&mut self, delta: f64, duration: f64) {
        let mut remaining = delta;
        loop {
            let forward = self.direction > 0.0;
            let boundary = if forward { duration } else { 0.0 };
            let to_boundary = if forward {
                boundary - self.time
            } else {
                self.time - boundary
            };
            if remaining < to_boundary - TIME_EPSILON {
                let to = if forward {
                    self.time + remaining
                } else {
                    self.time - remaining
                };
                if forward {
                    self.fire_forward(self.time, to);
                } else {
                    self.fire_backward(self.time, to);
                }
                self.time = to;
                return;
            }
            if forward {
                self.fire_forward(self.time, duration);
            } else {
                self.fire_backward(self.time, 0.0);
            }
            remaining -= to_boundary;
            self.time = boundary;
            self.wraps += 1;
            self.direction = if forward { -1.0 } else { 1.0 };
            if remaining <= TIME_EPSILON {
                return;
            }
        }
    }

    fn fire_forward(&mut self, from: f64, to: f64) {
        if to <= from {
            return;
        }
        let crossed: Vec<usize> = self
            .signals
            .iter()
            .enumerate()
            .filter(|(_, spec)| spec.time > from && spec.time <= to)
            .map(|(index, _)| index)
            .collect();
        for index in crossed {
            self.emit(index);
        }
    }

    fn fire_backward(&mut self, from: f64, to: f64) {
        if to >= from {
            return;
        }
        let crossed: Vec<usize> = self
            .signals
            .iter()
            .enumerate()
            .filter(|(_, spec)| spec.time >= to && spec.time < from)
            .map(|(index, _)| index)
            .collect();
        for index in crossed {
            self.emit(index);
        }
    }

    fn fire_arrival(&mut self, time: f64) {
        let crossed: Vec<usize> = self
            .signals
            .iter()
            .enumerate()
            .filter(|(_, spec)| spec.time == time)
            .map(|(index, _)| index)
            .collect();
        for index in crossed {
            self.emit(index);
        }
    }

    fn fire_retroactive(&mut self, time: f64) {
        let crossed: Vec<usize> = self
            .signals
            .iter()
            .enumerate()
            .filter(|(_, spec)| spec.retroactive && spec.time < time)
            .map(|(index, _)| index)
            .collect();
        for index in crossed {
            self.emit(index);
        }
    }

    fn emit(&mut self, index: usize) {
        if self.signals[index].emit_once && self.emitted_once[index] {
            return;
        }
        self.emitted_once[index] = true;
        let spec = &self.signals[index];
        self.signal_events.push(TimelineSignalEvent {
            signal: spec.signal.clone(),
            receiver: spec.receiver.clone(),
            time: spec.time,
        });
    }
}

fn collect_signals(sequence: &TimelineSequence) -> Vec<SignalSpec> {
    let mut specs = Vec::new();
    for track in &sequence.tracks {
        if track.kind != TimelineTrackKind::Signal {
            continue;
        }
        for clip in &track.clips {
            let TimelineClipPayload::Signal {
                signal,
                receiver,
                retroactive,
                emit_once,
            } = &clip.payload
            else {
                continue;
            };
            specs.push(SignalSpec {
                time: clip.start,
                signal: signal.clone(),
                receiver: receiver.clone(),
                retroactive: *retroactive,
                emit_once: *emit_once,
            });
        }
    }
    specs
}

#[derive(Clone, Debug, Default)]
pub struct TimelineWorld {
    sequences: BTreeMap<u64, TimelineSequence>,
    players: BTreeMap<u64, TimelinePlayer>,
    entities: BTreeSet<EntityId>,
    diagnostics: Vec<TimelineDiagnostic>,
    next_sequence: u64,
    next_player: u64,
}

impl TimelineWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_sequence(&mut self, sequence: TimelineSequence) -> u64 {
        self.next_sequence += 1;
        let handle = self.next_sequence;
        self.sequences.insert(handle, sequence);
        handle
    }

    pub fn sequence(&self, handle: u64) -> Option<&TimelineSequence> {
        self.sequences.get(&handle)
    }

    pub fn create_player(
        &mut self,
        sequence: u64,
        wrap_mode: TimelineWrapMode,
    ) -> Result<u64, TimelineError> {
        let Some(asset) = self.sequences.get(&sequence).cloned() else {
            return Err(TimelineError::UnknownSequence(sequence));
        };
        self.next_player += 1;
        let handle = self.next_player;
        self.players
            .insert(handle, TimelinePlayer::new(asset, wrap_mode));
        Ok(handle)
    }

    pub fn player(&self, handle: u64) -> Option<&TimelinePlayer> {
        self.players.get(&handle)
    }

    pub fn player_mut(&mut self, handle: u64) -> Option<&mut TimelinePlayer> {
        self.players.get_mut(&handle)
    }

    pub fn register_entity(&mut self, entity: EntityId) {
        self.entities.insert(entity);
    }

    pub fn is_registered(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity)
    }

    pub fn diagnostics(&self) -> &[TimelineDiagnostic] {
        &self.diagnostics
    }

    pub fn update(&mut self, player: u64, delta: f64) -> Result<(), TimelineError> {
        let Some(timeline) = self.players.get_mut(&player) else {
            return Err(TimelineError::UnknownPlayer(player));
        };
        timeline.update(delta);
        self.check_activation_bindings(player);
        Ok(())
    }

    pub fn drive_animation(
        &mut self,
        player: u64,
        animations: &mut AnimationWorld,
    ) -> Result<(), TimelineError> {
        let Some(timeline) = self.players.get(&player) else {
            return Err(TimelineError::UnknownPlayer(player));
        };
        let now = timeline.time();
        let mut targets: BTreeMap<EntityId, bool> = BTreeMap::new();
        let mut findings = Vec::new();
        for (track_index, track) in timeline.sequence().tracks.iter().enumerate() {
            if track.kind != TimelineTrackKind::Animation {
                continue;
            }
            for clip in &track.clips {
                let TimelineClipPayload::Animation { entity, .. } = &clip.payload else {
                    continue;
                };
                if animations.entry(*entity).is_none() {
                    findings.push(TimelineDiagnostic::MissingBinding {
                        track: TimelineTrackKind::Animation,
                        track_index,
                        target: entity.to_string(),
                    });
                    continue;
                }
                let active = clip.start <= now && now < clip.end();
                let entry = targets.entry(*entity).or_insert(false);
                *entry = *entry || active;
            }
        }
        for (entity, active) in targets {
            let speed = if active { 1.0 } else { 0.0 };
            let _ = animations.set_speed(entity, speed);
        }
        for finding in findings {
            self.push_diagnostic(finding);
        }
        Ok(())
    }

    pub fn drive_cinemachine(
        &mut self,
        player: u64,
        cameras: &mut CinemachineWorld,
    ) -> Result<(), TimelineError> {
        let Some(timeline) = self.players.get(&player) else {
            return Err(TimelineError::UnknownPlayer(player));
        };
        let shot = timeline.active_shot();
        match shot {
            Some(shot) => {
                if !cameras.set_shot(Some(shot.camera.as_str()), shot.blend) {
                    self.push_diagnostic(TimelineDiagnostic::MissingBinding {
                        track: TimelineTrackKind::CinemachineShot,
                        track_index: shot.track_index,
                        target: shot.camera.clone(),
                    });
                }
            }
            None => {
                cameras.set_shot(None, None);
            }
        }
        Ok(())
    }

    fn check_activation_bindings(&mut self, player: u64) {
        let findings = {
            let Some(timeline) = self.players.get(&player) else {
                return;
            };
            let mut findings = Vec::new();
            for (track_index, track) in timeline.sequence().tracks.iter().enumerate() {
                if track.kind != TimelineTrackKind::Activation {
                    continue;
                }
                for clip in &track.clips {
                    let TimelineClipPayload::Activation { entity, .. } = &clip.payload else {
                        continue;
                    };
                    if self.entities.contains(entity) {
                        continue;
                    }
                    findings.push(TimelineDiagnostic::MissingBinding {
                        track: TimelineTrackKind::Activation,
                        track_index,
                        target: entity.to_string(),
                    });
                }
            }
            findings
        };
        for finding in findings {
            self.push_diagnostic(finding);
        }
    }

    fn push_diagnostic(&mut self, diagnostic: TimelineDiagnostic) {
        if !self.diagnostics.contains(&diagnostic) {
            self.diagnostics.push(diagnostic);
        }
    }
}
