use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::OUTPUT_CHANNELS;

const PUSH_WAIT_DEADLINE: Duration = Duration::from_millis(50);
const PUSH_YIELD_INTERVAL: u32 = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PushReport {
    pub accepted_frames: u64,
    pub dropped_frames: u64,
}

#[derive(Debug)]
pub struct SampleRing {
    slots: Box<[AtomicU32]>,
    capacity_frames: usize,
    write_frame: AtomicUsize,
    read_frame: AtomicUsize,
    accepted_frames: AtomicU64,
    dropped_frames: AtomicU64,
    drained_frames: AtomicU64,
}

impl SampleRing {
    pub fn new(capacity_frames: usize) -> Self {
        let capacity_frames = capacity_frames.max(1);
        let slot_count = capacity_frames * OUTPUT_CHANNELS;
        let slots = (0..slot_count)
            .map(|_| AtomicU32::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            slots,
            capacity_frames,
            write_frame: AtomicUsize::new(0),
            read_frame: AtomicUsize::new(0),
            accepted_frames: AtomicU64::new(0),
            dropped_frames: AtomicU64::new(0),
            drained_frames: AtomicU64::new(0),
        }
    }

    pub fn capacity_frames(&self) -> usize {
        self.capacity_frames
    }

    pub fn push(&self, interleaved: &[f32]) -> PushReport {
        let total_frames = interleaved.len() / OUTPUT_CHANNELS;
        if total_frames == 0 {
            return PushReport::default();
        }
        let mut written = 0usize;
        while written < total_frames {
            let write = self.write_frame.load(Ordering::Relaxed);
            let read = self.read_frame.load(Ordering::Acquire);
            let used = write.saturating_sub(read);
            let space = self.capacity_frames.saturating_sub(used);
            if space == 0 {
                if !self.wait_for_space() {
                    let dropped = (total_frames - written) as u64;
                    self.dropped_frames.fetch_add(dropped, Ordering::Relaxed);
                    return PushReport {
                        accepted_frames: written as u64,
                        dropped_frames: dropped,
                    };
                }
                continue;
            }
            let chunk = space.min(total_frames - written);
            let base = write % self.capacity_frames;
            for (offset, frame) in (written..written + chunk).enumerate() {
                let slot = (base + offset) % self.capacity_frames;
                let left = interleaved[frame * OUTPUT_CHANNELS];
                let right = interleaved[frame * OUTPUT_CHANNELS + 1];
                self.slots[slot * OUTPUT_CHANNELS].store(left.to_bits(), Ordering::Relaxed);
                self.slots[slot * OUTPUT_CHANNELS + 1].store(right.to_bits(), Ordering::Relaxed);
            }
            self.write_frame.store(write + chunk, Ordering::Release);
            self.accepted_frames
                .fetch_add(chunk as u64, Ordering::Relaxed);
            written += chunk;
        }
        PushReport {
            accepted_frames: written as u64,
            dropped_frames: 0,
        }
    }

    pub fn fill(&self, out: &mut [f32]) -> usize {
        let capacity_frames = out.len() / OUTPUT_CHANNELS;
        out.fill(0.0);
        if capacity_frames == 0 {
            return 0;
        }
        let read = self.read_frame.load(Ordering::Relaxed);
        let write = self.write_frame.load(Ordering::Acquire);
        let available = write.saturating_sub(read).min(capacity_frames);
        for frame in 0..available {
            let slot = (read + frame) % self.capacity_frames;
            let left = f32::from_bits(self.slots[slot * OUTPUT_CHANNELS].load(Ordering::Relaxed));
            let right =
                f32::from_bits(self.slots[slot * OUTPUT_CHANNELS + 1].load(Ordering::Relaxed));
            out[frame * OUTPUT_CHANNELS] = left;
            out[frame * OUTPUT_CHANNELS + 1] = right;
        }
        self.read_frame.store(read + available, Ordering::Release);
        self.drained_frames
            .fetch_add(available as u64, Ordering::Relaxed);
        available
    }

    pub fn pending_frames(&self) -> usize {
        let write = self.write_frame.load(Ordering::Acquire);
        let read = self.read_frame.load(Ordering::Acquire);
        write.saturating_sub(read).min(self.capacity_frames)
    }

    pub fn accepted_frames(&self) -> u64 {
        self.accepted_frames.load(Ordering::Relaxed)
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    pub fn drained_frames(&self) -> u64 {
        self.drained_frames.load(Ordering::Relaxed)
    }

    fn wait_for_space(&self) -> bool {
        let deadline = Instant::now() + PUSH_WAIT_DEADLINE;
        let mut rounds = 0u32;
        while Instant::now() < deadline {
            if self.space_available() {
                return true;
            }
            rounds += 1;
            if rounds.is_multiple_of(PUSH_YIELD_INTERVAL) {
                std::thread::yield_now();
            } else {
                std::hint::spin_loop();
            }
        }
        false
    }

    fn space_available(&self) -> bool {
        let write = self.write_frame.load(Ordering::Relaxed);
        let read = self.read_frame.load(Ordering::Acquire);
        write.saturating_sub(read) < self.capacity_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_fill_move_every_frame_in_order() {
        let ring = SampleRing::new(8);
        let report = ring.push(&[0.25, -0.25, 0.5, -0.5]);
        assert_eq!(report.accepted_frames, 2);
        assert_eq!(report.dropped_frames, 0);
        assert_eq!(ring.pending_frames(), 2);

        let mut out = vec![9.0f32; 6];
        let frames = ring.fill(&mut out);
        assert_eq!(frames, 2);
        assert_eq!(out, vec![0.25, -0.25, 0.5, -0.5, 0.0, 0.0]);
        assert_eq!(ring.pending_frames(), 0);
        assert_eq!(ring.drained_frames(), 2);
    }

    #[test]
    fn a_full_ring_keeps_the_queued_frames_and_counts_the_overflow() {
        let ring = SampleRing::new(2);
        ring.push(&[1.0, 1.0, 2.0, 2.0]);
        let report = ring.push(&[3.0, 3.0]);
        assert_eq!(report.accepted_frames, 0);
        assert_eq!(report.dropped_frames, 1);
        assert_eq!(ring.dropped_frames(), 1);
        assert_eq!(ring.pending_frames(), 2);

        let mut out = vec![0.0f32; 4];
        assert_eq!(ring.fill(&mut out), 2);
        assert_eq!(out, vec![1.0, 1.0, 2.0, 2.0]);
    }

    #[test]
    fn fill_returns_every_queued_frame_while_the_producer_keeps_writing() {
        let ring = std::sync::Arc::new(SampleRing::new(256));
        let producer = std::sync::Arc::clone(&ring);
        let handle = std::thread::spawn(move || {
            for frame in 0..256u32 {
                let value = frame as f32 / 256.0;
                producer.push(&[value, value]);
            }
        });
        let mut drained = Vec::new();
        let mut out = vec![0.0f32; 8];
        while drained.len() < 256 {
            let frames = ring.fill(&mut out);
            for frame in 0..frames {
                drained.push(out[frame * OUTPUT_CHANNELS]);
            }
        }
        assert!(handle.join().is_ok());
        assert_eq!(drained.len(), 256);
        for (frame, value) in drained.iter().enumerate() {
            assert_eq!(*value, frame as f32 / 256.0);
        }
        assert_eq!(ring.dropped_frames(), 0);
    }

    #[test]
    fn a_full_ring_waits_for_the_consumer_instead_of_discarding_queued_frames() {
        let ring = std::sync::Arc::new(SampleRing::new(4));
        let consumer = std::sync::Arc::clone(&ring);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            let mut out = vec![0.0f32; 8];
            consumer.fill(&mut out);
            out
        });
        let report = ring.push(&[
            1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0, 6.0, 6.0, 7.0, 7.0, 8.0, 8.0,
        ]);
        let mut out = vec![0.0f32; 8];
        ring.fill(&mut out);
        let consumed = handle.join().unwrap_or_default();
        assert_eq!(report.accepted_frames, 8);
        assert_eq!(report.dropped_frames, 0);
        assert_eq!(ring.dropped_frames(), 0);
        assert_eq!(consumed[0], 1.0);
        assert_eq!(consumed[2], 2.0);
        assert_eq!(out[0], 5.0);
        assert_eq!(out[2], 6.0);
    }
}
