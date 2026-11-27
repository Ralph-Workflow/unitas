use unitas_audio::fx::FilterChain;
use unitas_audio::{AudioClip, AudioWorld};
use unitas_core::EntityId;

const SAMPLE_RATE: u32 = 44100;
const TAU: f32 = std::f32::consts::TAU;

fn sine(frequency: f32, seconds: f32) -> Vec<f32> {
    let frames = (f32::from(SAMPLE_RATE as u16) * seconds) as usize;
    (0..frames)
        .map(|index| {
            let phase = TAU * frequency * index as f32 / SAMPLE_RATE as f32;
            phase.sin()
        })
        .collect()
}

fn rms(samples: &[f32], skip: usize) -> f32 {
    let tail = &samples[skip..];
    let energy: f32 = tail.iter().map(|sample| sample * sample).sum();
    (energy / tail.len() as f32).sqrt()
}

fn gain_db(input_rms: f32, output_rms: f32) -> f32 {
    20.0 * (output_rms / input_rms).log10()
}

#[test]
fn low_pass_filter_attenuates_8khz_by_30db_and_passes_200hz() {
    let skip = SAMPLE_RATE as usize / 10;

    let mut chain = FilterChain::new(SAMPLE_RATE);
    chain.set_low_pass(1000.0, 1.0);
    let high = sine(8000.0, 0.5);
    let filtered_high: Vec<f32> = high
        .iter()
        .map(|sample| chain.process(*sample, *sample).0)
        .collect();
    let attenuation = gain_db(rms(&high, skip), rms(&filtered_high, skip));
    assert!(
        attenuation <= -30.0,
        "8 kHz sine through a 1 kHz low-pass attenuated {attenuation} dB"
    );

    let mut chain = FilterChain::new(SAMPLE_RATE);
    chain.set_low_pass(1000.0, 1.0);
    let low = sine(200.0, 0.5);
    let filtered_low: Vec<f32> = low
        .iter()
        .map(|sample| chain.process(*sample, *sample).0)
        .collect();
    let attenuation = gain_db(rms(&low, skip), rms(&filtered_low, skip));
    assert!(
        attenuation > -1.0,
        "200 Hz sine through a 1 kHz low-pass attenuated {attenuation} dB"
    );
}

#[test]
fn high_pass_filter_attenuates_200hz_by_30db_and_passes_8khz() {
    let skip = SAMPLE_RATE as usize / 10;

    let mut chain = FilterChain::new(SAMPLE_RATE);
    chain.set_high_pass(2000.0, 1.0);
    let low = sine(200.0, 0.5);
    let filtered_low: Vec<f32> = low
        .iter()
        .map(|sample| chain.process(*sample, *sample).0)
        .collect();
    let attenuation = gain_db(rms(&low, skip), rms(&filtered_low, skip));
    assert!(
        attenuation <= -30.0,
        "200 Hz sine through a 2 kHz high-pass attenuated {attenuation} dB"
    );

    let mut chain = FilterChain::new(SAMPLE_RATE);
    chain.set_high_pass(2000.0, 1.0);
    let high = sine(8000.0, 0.5);
    let filtered_high: Vec<f32> = high
        .iter()
        .map(|sample| chain.process(*sample, *sample).0)
        .collect();
    let attenuation = gain_db(rms(&high, skip), rms(&filtered_high, skip));
    assert!(
        attenuation > -1.0,
        "8 kHz sine through a 2 kHz high-pass attenuated {attenuation} dB"
    );
}

#[test]
fn echo_filter_repeats_after_the_delay_and_decays() {
    let delay_seconds = 0.1f32;
    let delay_frames = (delay_seconds * SAMPLE_RATE as f32) as usize;
    let feedback_db = -6.0f32;
    let feedback_gain = 10.0f32.powf(feedback_db / 20.0);

    let mut chain = FilterChain::new(SAMPLE_RATE);
    chain.set_echo(delay_seconds, 0.0, feedback_db, 0.0);

    let mut output = Vec::with_capacity(delay_frames * 4);
    output.push(chain.process(1.0, 1.0).0);
    for _ in 1..delay_frames * 4 {
        output.push(chain.process(0.0, 0.0).0);
    }

    let first = output[delay_frames];
    assert!(
        (first - 1.0).abs() < 1e-3,
        "first echo repeat was {first}, expected 1.0"
    );
    let second = output[delay_frames * 2];
    assert!(
        (second - feedback_gain).abs() < 1e-3,
        "second echo repeat was {second}, expected {feedback_gain}"
    );
    let third = output[delay_frames * 3];
    assert!(
        (third - feedback_gain * feedback_gain).abs() < 1e-3,
        "third echo repeat was {third}, expected {}",
        feedback_gain * feedback_gain
    );
}

#[test]
fn source_low_pass_filter_attenuates_the_mixed_output() {
    let mut world = AudioWorld::new();
    let samples = sine(8000.0, 1.0);
    let clip = AudioClip::new(1, SAMPLE_RATE, samples.clone());
    let clip_id = world.register_clip(clip);
    let entity = EntityId::new(7);
    world.ensure_source(entity);
    world.set_source_clip(entity, Some(clip_id));
    world.set_low_pass_filter(entity, 1000.0, 1.0);
    world.play(entity, false);

    let mixed = world.mix(SAMPLE_RATE as usize);
    let left: Vec<f32> = mixed.iter().step_by(2).copied().collect();
    let skip = SAMPLE_RATE as usize / 4;
    let attenuation = gain_db(rms(&samples, skip), rms(&left, skip));
    assert!(
        attenuation <= -30.0,
        "mixed 8 kHz sine through a 1 kHz low-pass attenuated {attenuation} dB"
    );
}

#[test]
fn source_without_a_filter_mixes_the_clip_unchanged() {
    let mut world = AudioWorld::new();
    let samples = sine(8000.0, 1.0);
    let clip_id = world.register_clip(AudioClip::new(1, SAMPLE_RATE, samples.clone()));
    let entity = EntityId::new(3);
    world.ensure_source(entity);
    world.set_source_clip(entity, Some(clip_id));
    world.play(entity, false);

    let mixed = world.mix(SAMPLE_RATE as usize);
    let left: Vec<f32> = mixed.iter().step_by(2).copied().collect();
    assert_eq!(left.len(), samples.len());
    for (index, sample) in left.iter().enumerate() {
        assert!(
            (sample - samples[index]).abs() < 1e-6,
            "frame {index} mixed {sample}, expected {}",
            samples[index]
        );
    }
}
