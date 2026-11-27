use std::f32::consts::TAU;

const MIN_CUTOFF_HZ: f32 = 10.0;
const MAX_CUTOFF_HZ: f32 = 22000.0;
const MIN_RESONANCE_Q: f32 = 0.05;
const MAX_RESONANCE_Q: f32 = 20.0;
const DEFAULT_RESONANCE_Q: f32 = 1.0;
const MIN_DELAY_SECONDS: f32 = 0.001;
const MAX_DELAY_SECONDS: f32 = 2.0;
const MIN_DECIBELS: f32 = -60.0;
const MAX_DECIBELS: f32 = 20.0;
const MAX_FEEDBACK_GAIN: f32 = 10.0;
const CHANNELS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiquadKind {
    LowPass,
    HighPass,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    kind: BiquadKind,
    cutoff_hz: f32,
    resonance_q: f32,
    coefficients: [f32; 5],
    z1: [f32; CHANNELS],
    z2: [f32; CHANNELS],
}

impl Biquad {
    pub fn low_pass(sample_rate: u32, cutoff_hz: f32, resonance_q: f32) -> Self {
        Self::new(BiquadKind::LowPass, sample_rate, cutoff_hz, resonance_q)
    }

    pub fn high_pass(sample_rate: u32, cutoff_hz: f32, resonance_q: f32) -> Self {
        Self::new(BiquadKind::HighPass, sample_rate, cutoff_hz, resonance_q)
    }

    fn new(kind: BiquadKind, sample_rate: u32, cutoff_hz: f32, resonance_q: f32) -> Self {
        let mut biquad = Self {
            kind,
            cutoff_hz: MIN_CUTOFF_HZ,
            resonance_q: DEFAULT_RESONANCE_Q,
            coefficients: [0.0; 5],
            z1: [0.0; CHANNELS],
            z2: [0.0; CHANNELS],
        };
        biquad.configure(sample_rate, cutoff_hz, resonance_q);
        biquad
    }

    pub fn kind(&self) -> BiquadKind {
        self.kind
    }

    pub fn cutoff_hz(&self) -> f32 {
        self.cutoff_hz
    }

    pub fn resonance_q(&self) -> f32 {
        self.resonance_q
    }

    pub fn configure(&mut self, sample_rate: u32, cutoff_hz: f32, resonance_q: f32) {
        let rate = (sample_rate as f32).max(1.0);
        let ceiling = (rate * 0.49).clamp(MIN_CUTOFF_HZ, MAX_CUTOFF_HZ);
        let cutoff = if cutoff_hz.is_finite() {
            cutoff_hz.clamp(MIN_CUTOFF_HZ, ceiling)
        } else {
            MAX_CUTOFF_HZ
        };
        let q = if resonance_q.is_finite() && resonance_q > 0.0 {
            resonance_q.clamp(MIN_RESONANCE_Q, MAX_RESONANCE_Q)
        } else {
            DEFAULT_RESONANCE_Q
        };
        let omega = TAU * cutoff / rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = match self.kind {
            BiquadKind::LowPass => {
                let half = (1.0 - cos_omega) * 0.5;
                (half, half * 2.0, half)
            }
            BiquadKind::HighPass => {
                let half = (1.0 + cos_omega) * 0.5;
                (half, -half * 2.0, half)
            }
        };
        self.cutoff_hz = cutoff;
        self.resonance_q = q;
        self.coefficients = [
            b0 / a0,
            b1 / a0,
            b2 / a0,
            -2.0 * cos_omega / a0,
            (1.0 - alpha) / a0,
        ];
    }

    pub fn process(&mut self, channel: usize, input: f32) -> f32 {
        let index = usize::from(channel != 0);
        let coefficients = self.coefficients;
        let output = coefficients[0] * input + self.z1[index];
        self.z1[index] = coefficients[1] * input - coefficients[3] * output + self.z2[index];
        self.z2[index] = coefficients[2] * input - coefficients[4] * output;
        output
    }

    pub fn reset(&mut self) {
        self.z1 = [0.0; CHANNELS];
        self.z2 = [0.0; CHANNELS];
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EchoFilter {
    delay_seconds: f32,
    dry_db: f32,
    feedback_db: f32,
    wet_db: f32,
    delay_frames: usize,
    dry_gain: f32,
    wet_gain: f32,
    feedback_gain: f32,
    line: Vec<f32>,
    write_index: usize,
}

impl EchoFilter {
    pub fn new(
        sample_rate: u32,
        delay_seconds: f32,
        dry_db: f32,
        feedback_db: f32,
        wet_db: f32,
    ) -> Self {
        let rate = (sample_rate as f32).max(1.0);
        let delay = if delay_seconds.is_finite() {
            delay_seconds.clamp(MIN_DELAY_SECONDS, MAX_DELAY_SECONDS)
        } else {
            MIN_DELAY_SECONDS
        };
        let delay_frames = ((delay * rate).round() as usize).max(1);
        Self {
            delay_seconds: delay,
            dry_db,
            feedback_db,
            wet_db,
            delay_frames,
            dry_gain: decibels_to_gain(dry_db, 1.0),
            wet_gain: decibels_to_gain(wet_db, 1.0),
            feedback_gain: decibels_to_gain(feedback_db, 0.0).min(MAX_FEEDBACK_GAIN),
            line: vec![0.0; delay_frames * CHANNELS],
            write_index: 0,
        }
    }

    pub fn delay_seconds(&self) -> f32 {
        self.delay_seconds
    }

    pub fn dry_db(&self) -> f32 {
        self.dry_db
    }

    pub fn feedback_db(&self) -> f32 {
        self.feedback_db
    }

    pub fn wet_db(&self) -> f32 {
        self.wet_db
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let index = self.write_index * CHANNELS;
        let delayed_left = self.line[index];
        let delayed_right = self.line[index + 1];
        self.line[index] = left + delayed_left * self.feedback_gain;
        self.line[index + 1] = right + delayed_right * self.feedback_gain;
        self.write_index = (self.write_index + 1) % self.delay_frames;
        (
            left * self.dry_gain + delayed_left * self.wet_gain,
            right * self.dry_gain + delayed_right * self.wet_gain,
        )
    }

    pub fn reset(&mut self) {
        self.line.iter_mut().for_each(|sample| *sample = 0.0);
        self.write_index = 0;
    }
}

fn decibels_to_gain(decibels: f32, fallback: f32) -> f32 {
    if !decibels.is_finite() {
        return fallback;
    }
    let clamped = decibels.clamp(MIN_DECIBELS, MAX_DECIBELS);
    10.0f32.powf(clamped / 20.0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilterChain {
    sample_rate: u32,
    low_pass: Option<Biquad>,
    high_pass: Option<Biquad>,
    echo: Option<EchoFilter>,
}

impl FilterChain {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate.max(1),
            low_pass: None,
            high_pass: None,
            echo: None,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn set_low_pass(&mut self, cutoff_hz: f32, resonance_q: f32) {
        let sample_rate = self.sample_rate;
        let mut biquad = match self.low_pass {
            Some(existing) => existing,
            None => Biquad::low_pass(sample_rate, cutoff_hz, resonance_q),
        };
        biquad.configure(sample_rate, cutoff_hz, resonance_q);
        self.low_pass = Some(biquad);
    }

    pub fn set_high_pass(&mut self, cutoff_hz: f32, resonance_q: f32) {
        let sample_rate = self.sample_rate;
        let mut biquad = match self.high_pass {
            Some(existing) => existing,
            None => Biquad::high_pass(sample_rate, cutoff_hz, resonance_q),
        };
        biquad.configure(sample_rate, cutoff_hz, resonance_q);
        self.high_pass = Some(biquad);
    }

    pub fn set_echo(&mut self, delay_seconds: f32, dry_db: f32, feedback_db: f32, wet_db: f32) {
        let echo = EchoFilter::new(self.sample_rate, delay_seconds, dry_db, feedback_db, wet_db);
        self.echo = Some(echo);
    }

    pub fn clear_low_pass(&mut self) {
        self.low_pass = None;
    }

    pub fn clear_high_pass(&mut self) {
        self.high_pass = None;
    }

    pub fn clear_echo(&mut self) {
        self.echo = None;
    }

    pub fn is_active(&self) -> bool {
        self.low_pass.is_some() || self.high_pass.is_some() || self.echo.is_some()
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        let rate = sample_rate.max(1);
        if rate == self.sample_rate {
            return;
        }
        self.sample_rate = rate;
        if let Some(mut biquad) = self.low_pass {
            let cutoff = biquad.cutoff_hz();
            let resonance = biquad.resonance_q();
            biquad.configure(rate, cutoff, resonance);
            self.low_pass = Some(biquad);
        }
        if let Some(mut biquad) = self.high_pass {
            let cutoff = biquad.cutoff_hz();
            let resonance = biquad.resonance_q();
            biquad.configure(rate, cutoff, resonance);
            self.high_pass = Some(biquad);
        }
        if let Some(echo) = self.echo.take() {
            let rebuilt = EchoFilter::new(
                rate,
                echo.delay_seconds(),
                echo.dry_db(),
                echo.feedback_db(),
                echo.wet_db(),
            );
            self.echo = Some(rebuilt);
        }
    }

    pub fn reset(&mut self) {
        if let Some(mut biquad) = self.low_pass {
            biquad.reset();
            self.low_pass = Some(biquad);
        }
        if let Some(mut biquad) = self.high_pass {
            biquad.reset();
            self.high_pass = Some(biquad);
        }
        if let Some(mut echo) = self.echo.take() {
            echo.reset();
            self.echo = Some(echo);
        }
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let mut out_left = left;
        let mut out_right = right;
        if let Some(biquad) = self.low_pass.as_mut() {
            out_left = biquad.process(0, out_left);
            out_right = biquad.process(1, out_right);
        }
        if let Some(biquad) = self.high_pass.as_mut() {
            out_left = biquad.process(0, out_left);
            out_right = biquad.process(1, out_right);
        }
        if let Some(echo) = self.echo.as_mut() {
            let (delayed_left, delayed_right) = echo.process(out_left, out_right);
            out_left = delayed_left;
            out_right = delayed_right;
        }
        (out_left, out_right)
    }
}
