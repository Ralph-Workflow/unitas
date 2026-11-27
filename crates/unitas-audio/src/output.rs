use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::{AudioError, AudioWorld};

pub const OUTPUT_CHANNELS: usize = 2;

const DEFAULT_CAPACITY_FRAMES: usize = 4096;
const STREAM_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PushReport {
    pub accepted_frames: u64,
    pub dropped_frames: u64,
}

/// The samples the device output accepted, kept for the device lane's audio proof.
#[derive(Clone, Debug)]
pub struct DeviceCapture {
    state: Arc<Mutex<CaptureState>>,
}

#[derive(Debug, Default)]
struct CaptureState {
    seconds: u32,
    sample_rate: u32,
    capacity_samples: usize,
    samples: VecDeque<f32>,
}

/// The capture the device lane arms before the player starts the device.
static ARMED_CAPTURE: Mutex<Option<DeviceCapture>> = Mutex::new(None);

/// Arms a capture of the samples the device output accepts; the device lane
/// reads the captured audio after the run.
pub fn arm_capture(seconds: u32) -> DeviceCapture {
    let capture = DeviceCapture {
        state: Arc::new(Mutex::new(CaptureState {
            seconds,
            ..CaptureState::default()
        })),
    };
    let mut armed = match ARMED_CAPTURE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *armed = Some(capture.clone());
    capture
}

fn take_armed_capture() -> Option<DeviceCapture> {
    let mut armed = match ARMED_CAPTURE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    armed.take()
}

impl DeviceCapture {
    /// The mix sample rate the capture recorded at.
    pub fn sample_rate(&self) -> u32 {
        self.lock().sample_rate
    }

    /// The captured samples as mono, the interleaved pairs averaged.
    pub fn mono_samples(&self) -> Vec<f32> {
        let captured = self.lock();
        let samples: Vec<f32> = captured.samples.iter().copied().collect();
        drop(captured);
        samples
            .chunks_exact(OUTPUT_CHANNELS)
            .map(|frame| (frame[0] + frame[1]) * 0.5)
            .collect()
    }

    /// Writes the captured audio as a 16-bit PCM mono WAV file.
    pub fn write_wav(&self, path: &Path) -> Result<(), io::Error> {
        let mono = self.mono_samples();
        let sample_rate = self.sample_rate();
        let data_bytes = mono.len() * 2;
        let mut bytes: Vec<u8> = Vec::with_capacity(44 + data_bytes);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        for sample in mono {
            let scaled = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            bytes.extend_from_slice(&scaled.to_le_bytes());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    }

    fn lock(&self) -> MutexGuard<'_, CaptureState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[derive(Debug, Default)]
struct HandoffState {
    samples: VecDeque<f32>,
    accepted_frames: u64,
    dropped_frames: u64,
    drained_frames: u64,
    capture: Option<DeviceCapture>,
}

#[derive(Clone, Debug)]
pub struct FrameHandoff {
    state: Arc<Mutex<HandoffState>>,
    capacity_frames: usize,
}

impl FrameHandoff {
    pub fn new(capacity_frames: usize) -> Self {
        let capacity_frames = capacity_frames.max(1);
        Self {
            state: Arc::new(Mutex::new(HandoffState::default())),
            capacity_frames,
        }
    }

    pub fn capacity_frames(&self) -> usize {
        self.capacity_frames
    }

    /// Records the accepted samples into the capture, bounded to the last
    /// seconds of audio the capture asked for.
    pub fn enable_capture(&self, capture: DeviceCapture, sample_rate: u32) {
        let capacity_samples =
            capture.lock().seconds as usize * sample_rate as usize * OUTPUT_CHANNELS;
        {
            let mut captured = capture.lock();
            captured.sample_rate = sample_rate;
            captured.capacity_samples = capacity_samples;
        }
        self.lock().capture = Some(capture);
    }

    pub fn push(&self, interleaved: &[f32]) -> PushReport {
        let incoming_frames = interleaved.len() / OUTPUT_CHANNELS;
        if incoming_frames == 0 {
            return PushReport::default();
        }
        let mut state = self.lock();
        let mut dropped_frames = 0u64;
        while !state.samples.is_empty()
            && state.samples.len() / OUTPUT_CHANNELS + incoming_frames > self.capacity_frames
        {
            state.samples.pop_front();
            state.samples.pop_front();
            dropped_frames += 1;
        }
        let mut kept_frames = incoming_frames;
        if kept_frames > self.capacity_frames {
            dropped_frames += (kept_frames - self.capacity_frames) as u64;
            kept_frames = self.capacity_frames;
        }
        let start = (incoming_frames - kept_frames) * OUTPUT_CHANNELS;
        let end = start + kept_frames * OUTPUT_CHANNELS;
        state
            .samples
            .extend(interleaved[start..end].iter().copied());
        if let Some(capture) = &state.capture {
            let mut captured = capture.lock();
            captured
                .samples
                .extend(interleaved[start..end].iter().copied());
            while captured.samples.len() > captured.capacity_samples {
                captured.samples.pop_front();
            }
        }
        state.accepted_frames += kept_frames as u64;
        state.dropped_frames += dropped_frames;
        PushReport {
            accepted_frames: kept_frames as u64,
            dropped_frames,
        }
    }

    pub fn fill(&self, out: &mut [f32]) -> usize {
        let capacity_frames = out.len() / OUTPUT_CHANNELS;
        if capacity_frames == 0 {
            return 0;
        }
        let Ok(mut state) = self.state.try_lock() else {
            return 0;
        };
        let available_frames = state.samples.len() / OUTPUT_CHANNELS;
        let frames = available_frames.min(capacity_frames);
        let samples = frames * OUTPUT_CHANNELS;
        for (index, slot) in out.iter_mut().take(samples).enumerate() {
            *slot = state.samples[index];
        }
        out[samples..].fill(0.0);
        for _ in 0..samples {
            state.samples.pop_front();
        }
        state.drained_frames += frames as u64;
        frames
    }

    pub fn pending_frames(&self) -> usize {
        let state = self.lock();
        state.samples.len() / OUTPUT_CHANNELS
    }

    pub fn accepted_frames(&self) -> u64 {
        self.lock().accepted_frames
    }

    pub fn dropped_frames(&self) -> u64 {
        self.lock().dropped_frames
    }

    pub fn drained_frames(&self) -> u64 {
        self.lock().drained_frames
    }

    fn lock(&self) -> MutexGuard<'_, HandoffState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

pub struct DeviceOutput {
    stream: Stream,
    handoff: FrameHandoff,
    sample_rate: u32,
    channels: u16,
    paused: bool,
}

impl DeviceOutput {
    pub fn start() -> Result<Self, AudioError> {
        Self::start_with_capacity(DEFAULT_CAPACITY_FRAMES)
    }

    pub fn start_with_capacity(capacity_frames: usize) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            AudioError::Device("no default output device is available".to_string())
        })?;
        let supported = device
            .default_output_config()
            .map_err(|error| AudioError::Device(error.to_string()))?;
        let sample_rate = supported.sample_rate();
        let channels = supported.channels();
        let config = supported.config();
        let handoff = FrameHandoff::new(capacity_frames);
        if let Some(capture) = take_armed_capture() {
            handoff.enable_capture(capture, sample_rate);
        }
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(&device, config, handoff.clone())?,
            SampleFormat::I16 => build_stream::<i16>(&device, config, handoff.clone())?,
            SampleFormat::U16 => build_stream::<u16>(&device, config, handoff.clone())?,
            format => {
                return Err(AudioError::Device(format!(
                    "unsupported output sample format {format}"
                )))
            }
        };
        stream
            .play()
            .map_err(|error| AudioError::Device(error.to_string()))?;
        Ok(Self {
            stream,
            handoff,
            sample_rate,
            channels,
            paused: false,
        })
    }

    pub fn handoff(&self) -> &FrameHandoff {
        &self.handoff
    }

    pub fn push_mixed(&self, interleaved: &[f32]) -> PushReport {
        self.handoff.push(interleaved)
    }

    pub fn pump_world(&self, world: &mut AudioWorld, n_frames: usize) -> PushReport {
        let mixed = world.mix(n_frames);
        self.handoff.push(&mixed)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn pending_frames(&self) -> usize {
        self.handoff.pending_frames()
    }

    pub fn accepted_frames(&self) -> u64 {
        self.handoff.accepted_frames()
    }

    pub fn dropped_frames(&self) -> u64 {
        self.handoff.dropped_frames()
    }

    pub fn drained_frames(&self) -> u64 {
        self.handoff.drained_frames()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<(), AudioError> {
        if paused == self.paused {
            return Ok(());
        }
        let outcome = if paused {
            self.stream.pause()
        } else {
            self.stream.play()
        };
        outcome.map_err(|error| AudioError::Device(error.to_string()))?;
        self.paused = paused;
        Ok(())
    }
}

fn build_stream<T>(
    device: &Device,
    config: StreamConfig,
    handoff: FrameHandoff,
) -> Result<Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels.max(1));
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_output_stream::<T, _, _>(
            config,
            move |output: &mut [T], _info: &cpal::OutputCallbackInfo| {
                let frames = output.len() / channels;
                let needed = frames * OUTPUT_CHANNELS;
                if scratch.len() != needed {
                    scratch.resize(needed, 0.0);
                }
                handoff.fill(&mut scratch);
                write_output(output, &scratch, channels);
            },
            |_error| {},
            Some(STREAM_TIMEOUT),
        )
        .map_err(|error| AudioError::Device(error.to_string()))
}

fn write_output<T>(output: &mut [T], mixed: &[f32], channels: usize)
where
    T: SizedSample + FromSample<f32>,
{
    for (frame_index, frame) in output.chunks_mut(channels).enumerate() {
        let left = sample_at(mixed, frame_index * OUTPUT_CHANNELS);
        let right = sample_at(mixed, frame_index * OUTPUT_CHANNELS + 1);
        let first = if channels == 1 {
            (left + right) * 0.5
        } else {
            left
        };
        if let Some(slot) = frame.first_mut() {
            *slot = T::from_sample(first);
        }
        if channels > 1 {
            let second = if channels == 2 { right } else { 0.0 };
            if let Some(slot) = frame.get_mut(1) {
                *slot = T::from_sample(second);
            }
        }
    }
}

fn sample_at(mixed: &[f32], index: usize) -> f32 {
    match mixed.get(index) {
        Some(value) => *value,
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_reports_pending_frames_after_push() {
        let handoff = FrameHandoff::new(8);
        assert_eq!(handoff.pending_frames(), 0);
        let report = handoff.push(&[0.25, -0.5, 1.0, -1.0]);
        assert_eq!(report.accepted_frames, 2);
        assert_eq!(report.dropped_frames, 0);
        assert_eq!(handoff.pending_frames(), 2);
        assert_eq!(handoff.accepted_frames(), 2);
        assert_eq!(handoff.dropped_frames(), 0);
    }

    #[test]
    fn handoff_drains_the_exact_frames_that_were_pushed() {
        let handoff = FrameHandoff::new(8);
        handoff.push(&[0.25, -0.5, 1.0, -1.0]);
        let mut out = vec![9.0f32; 8];
        let frames = handoff.fill(&mut out);
        assert_eq!(frames, 2);
        assert_eq!(out, vec![0.25, -0.5, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(handoff.pending_frames(), 0);
        assert_eq!(handoff.drained_frames(), 2);
    }

    #[test]
    fn handoff_fills_the_remainder_with_silence() {
        let handoff = FrameHandoff::new(8);
        handoff.push(&[0.25, -0.5]);
        let mut out = vec![9.0f32; 4];
        let frames = handoff.fill(&mut out);
        assert_eq!(frames, 1);
        assert_eq!(out, vec![0.25, -0.5, 0.0, 0.0]);
    }

    #[test]
    fn handoff_drops_the_oldest_frames_when_full() {
        let handoff = FrameHandoff::new(2);
        handoff.push(&[1.0, 1.0, 2.0, 2.0]);
        let report = handoff.push(&[3.0, 3.0]);
        assert_eq!(report.accepted_frames, 1);
        assert_eq!(report.dropped_frames, 1);
        assert_eq!(handoff.dropped_frames(), 1);
        assert_eq!(handoff.pending_frames(), 2);
        let mut out = vec![0.0f32; 4];
        handoff.fill(&mut out);
        assert_eq!(out, vec![2.0, 2.0, 3.0, 3.0]);
    }

    #[test]
    fn handoff_keeps_the_newest_frames_when_a_batch_exceeds_capacity() {
        let handoff = FrameHandoff::new(2);
        let report = handoff.push(&[1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
        assert_eq!(report.accepted_frames, 2);
        assert_eq!(report.dropped_frames, 1);
        let mut out = vec![0.0f32; 4];
        let frames = handoff.fill(&mut out);
        assert_eq!(frames, 2);
        assert_eq!(out, vec![2.0, 2.0, 3.0, 3.0]);
    }

    #[test]
    fn handoff_is_shared_between_the_producer_and_the_consumer() {
        let handoff = FrameHandoff::new(8);
        let producer = handoff.clone();
        let handle = std::thread::spawn(move || producer.push(&[0.25, -0.5, 1.0, -1.0]));
        assert!(handle.join().is_ok());
        assert_eq!(handoff.pending_frames(), 2);
        let mut out = vec![0.0f32; 4];
        assert_eq!(handoff.fill(&mut out), 2);
        assert_eq!(out, vec![0.25, -0.5, 1.0, -1.0]);
    }

    #[test]
    fn write_output_emits_the_mixed_samples_verbatim() {
        let handoff = FrameHandoff::new(8);
        handoff.push(&[0.25, -0.5, 1.0, -1.0]);
        let mut mixed = vec![0.0f32; 4];
        handoff.fill(&mut mixed);
        let mut device_samples = vec![0.0f32; 4];
        write_output(&mut device_samples, &mixed, 2);
        assert_eq!(device_samples, vec![0.25, -0.5, 1.0, -1.0]);
    }

    #[test]
    fn write_output_downmixes_to_mono() {
        let mut device_samples = vec![0.0f32; 1];
        write_output(&mut device_samples, &[0.25, -0.75], 1);
        assert_eq!(device_samples, vec![-0.25]);
    }

    #[test]
    fn capture_records_the_accepted_samples_as_mono() {
        let handoff = FrameHandoff::new(8);
        let capture = DeviceCapture {
            state: Arc::new(Mutex::new(CaptureState {
                seconds: 1,
                ..CaptureState::default()
            })),
        };
        handoff.enable_capture(capture.clone(), 48000);
        handoff.push(&[0.5, -0.5, 1.0, 1.0]);
        assert_eq!(capture.sample_rate(), 48000);
        assert_eq!(capture.mono_samples(), vec![0.0, 1.0]);
    }

    #[test]
    fn capture_keeps_the_newest_samples_within_its_bound() {
        let handoff = FrameHandoff::new(8);
        let capture = DeviceCapture {
            state: Arc::new(Mutex::new(CaptureState {
                seconds: 1,
                ..CaptureState::default()
            })),
        };
        handoff.enable_capture(capture.clone(), 2);
        handoff.push(&[0.1, 0.1, 0.2, 0.2]);
        handoff.push(&[0.3, 0.3, 0.4, 0.4]);
        assert_eq!(capture.mono_samples(), vec![0.3, 0.4]);
    }

    #[test]
    fn capture_writes_a_16_bit_mono_wav() -> Result<(), Box<dyn std::error::Error>> {
        let handoff = FrameHandoff::new(8);
        let capture = DeviceCapture {
            state: Arc::new(Mutex::new(CaptureState {
                seconds: 1,
                ..CaptureState::default()
            })),
        };
        handoff.enable_capture(capture.clone(), 48000);
        handoff.push(&[0.5, 0.5]);
        let path =
            std::env::temp_dir().join(format!("unitas_capture_test_{}.wav", std::process::id()));
        capture.write_wav(&path)?;
        let bytes = std::fs::read(&path)?;
        let _ = std::fs::remove_file(&path);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1);
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            48000
        );
        assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
        assert_eq!(bytes.len(), 46);
        Ok(())
    }

    #[test]
    fn device_start_returns_a_typed_error_instead_of_panicking() {
        match DeviceOutput::start() {
            Ok(output) => {
                assert!(output.sample_rate() > 0);
                assert!(output.channels() >= 1);
                let report = output.push_mixed(&[0.25, -0.5]);
                assert_eq!(report.accepted_frames, 1);
            }
            Err(error) => {
                assert!(matches!(error, AudioError::Device(_)));
            }
        }
    }

    #[test]
    fn capture_records_gameplay_and_menu_audio_separation_across_pause() {
        let mut world = AudioWorld::new();
        let clip_id = world.register_clip(crate::AudioClip::new(1, 8, vec![0.5, 0.5, 0.5, 0.5]));
        let gameplay = unitas_core::EntityId::new(1);
        let menu = unitas_core::EntityId::new(2);
        for entity in [gameplay, menu] {
            world.ensure_source(entity);
            world.set_source_clip(entity, Some(clip_id));
        }
        world.set_source_group(gameplay, crate::AudioMixerGroup::Sfx);
        world.set_source_group(menu, crate::AudioMixerGroup::Menu);

        let handoff = FrameHandoff::new(32);
        let capture = DeviceCapture {
            state: Arc::new(Mutex::new(CaptureState {
                seconds: 2,
                ..CaptureState::default()
            })),
        };
        handoff.enable_capture(capture.clone(), 8);

        let lead_in = world.mix(2);
        handoff.push(&lead_in);

        world.play(gameplay, true);
        let gameplay_active = world.mix(2);
        handoff.push(&gameplay_active);

        world.set_gameplay_paused(true);
        world.play(gameplay, true);
        let gameplay_paused = world.mix(2);
        handoff.push(&gameplay_paused);

        world.play(menu, true);
        let menu_paused = world.mix(2);
        handoff.push(&menu_paused);

        assert_eq!(
            capture.mono_samples(),
            vec![0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.5, 0.5]
        );
    }
}
