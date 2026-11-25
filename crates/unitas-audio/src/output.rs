use std::collections::VecDeque;
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

#[derive(Debug, Default)]
struct HandoffState {
    samples: VecDeque<f32>,
    accepted_frames: u64,
    dropped_frames: u64,
    drained_frames: u64,
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
}
