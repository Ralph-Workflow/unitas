use unitas_audio::fx::FilterChain;
use unitas_audio::{AudioClip, AudioWorld, RolloffMode};
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

#[test]
fn logarithmic_linear_and_custom_rolloff_curves_follow_unity_distance_attenuation() {
    let mut world = AudioWorld::new();
    world.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);

    let (log_src, lin_src) = setup_log_and_linear_rolloff(&mut world);
    assert_log_and_linear_attenuation(&mut world, log_src, lin_src);
    assert_custom_rolloff(&mut world);
}

fn setup_log_and_linear_rolloff(world: &mut AudioWorld) -> (EntityId, EntityId) {
    let log_src = EntityId::new(80);
    world.ensure_source(log_src);
    assert_eq!(world.rolloff_mode(log_src), RolloffMode::Logarithmic);
    world.set_spatial_blend(log_src, 1.0);
    world.set_min_distance(log_src, 2.0);
    world.set_max_distance(log_src, 20.0);
    world.play(log_src, false);

    let lin_src = EntityId::new(81);
    world.ensure_source(lin_src);
    world.set_rolloff_mode(lin_src, RolloffMode::Linear);
    assert_eq!(world.rolloff_mode(lin_src), RolloffMode::Linear);
    world.set_spatial_blend(lin_src, 1.0);
    world.set_min_distance(lin_src, 2.0);
    world.set_max_distance(lin_src, 20.0);
    world.play(lin_src, false);

    (log_src, lin_src)
}

fn assert_log_and_linear_attenuation(world: &mut AudioWorld, log_src: EntityId, lin_src: EntityId) {
    for entity in [log_src, lin_src] {
        world.set_source_position(entity, [0.0, 0.0, 2.0]);
        let (l, r) = world.source_gains(entity).unwrap_or((0.0, 0.0));
        assert!((l - 0.5).abs() < 1e-6);
        assert!((r - 0.5).abs() < 1e-6);
    }

    world.set_source_position(log_src, [0.0, 0.0, 4.0]);
    world.set_source_position(lin_src, [0.0, 0.0, 4.0]);
    let (log_l, log_r) = world.source_gains(log_src).unwrap_or((0.0, 0.0));
    let (lin_l, lin_r) = world.source_gains(lin_src).unwrap_or((0.0, 0.0));
    let log_total = log_l + log_r;
    let lin_total = lin_l + lin_r;
    assert!((log_total - 0.5).abs() < 1e-6);
    assert!((lin_total - (16.0 / 18.0)).abs() < 1e-6);
    assert!(log_total < lin_total);

    world.set_source_position(log_src, [0.0, 0.0, 8.0]);
    let (log8_l, log8_r) = world.source_gains(log_src).unwrap_or((0.0, 0.0));
    assert!(((log8_l + log8_r) - 0.25).abs() < 1e-6);
}

fn assert_custom_rolloff(world: &mut AudioWorld) {
    let custom_src = EntityId::new(82);
    world.ensure_source(custom_src);
    world.set_spatial_blend(custom_src, 1.0);
    world.set_min_distance(custom_src, 1.0);
    world.set_max_distance(custom_src, 10.0);
    world.set_custom_rolloff(
        custom_src,
        vec![(0.0, 1.0), (0.2, 0.8), (0.5, 0.2), (1.0, 0.0)],
    );
    assert_eq!(world.rolloff_mode(custom_src), RolloffMode::Custom);
    assert_eq!(
        world.custom_rolloff(custom_src),
        &[(0.0, 1.0), (0.2, 0.8), (0.5, 0.2), (1.0, 0.0)]
    );
    world.play(custom_src, false);

    world.set_source_position(custom_src, [0.0, 0.0, 3.5]);
    let (c_l, c_r) = world.source_gains(custom_src).unwrap_or((0.0, 0.0));
    assert!(((c_l + c_r) - 0.5).abs() < 1e-6);

    world.set_source_position(custom_src, [0.0, 0.0, 10.0]);
    let (c_end_l, c_end_r) = world.source_gains(custom_src).unwrap_or((0.0, 0.0));
    assert!((c_end_l + c_end_r).abs() < 1e-6);
}

fn impulse_world(reverb: Option<(f32, f32, f32)>) -> AudioWorld {
    let mut world = AudioWorld::new();
    let mut samples = vec![0.0f32; 44100];
    samples[0] = 1.0;
    let clip_id = world.register_clip(AudioClip {
        channels: 1,
        sample_rate: 44100,
        samples,
        duration: 1.0,
    });
    let entity = EntityId::new(90);
    world.ensure_source(entity);
    world.set_source_clip(entity, Some(clip_id));
    if let Some((room_size, damping, wet_level)) = reverb {
        assert!(world.set_reverb(room_size, damping, wet_level));
    }
    world.play(entity, false);
    world
}

#[test]
fn reverb_send_produces_a_decaying_tail_after_an_impulse() {
    let mut world = impulse_world(Some((0.9, 0.2, 1.0)));
    let mixed = world.mix(44100);
    let left: Vec<f32> = mixed.iter().step_by(2).copied().collect();
    assert_eq!(left[0], 1.0);
    let tail_end = left
        .iter()
        .rposition(|sample| sample.abs() > 1e-7)
        .map_or(0, |i| i);
    assert!(tail_end > 10_000, "reverb tail ended at frame {tail_end}");
    let early = left[1000..2000]
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    let late = left[30_000..31_000]
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(late > 0.0);
    assert!(
        late < early,
        "tail did not decay: early {early} late {late}"
    );
}

#[test]
fn reverb_with_zero_wet_is_bit_identical_to_the_dry_mix() {
    let mut dry = impulse_world(None);
    let mut wet = impulse_world(Some((0.9, 0.5, 0.0)));
    let dry_mix = dry.mix(44100);
    let wet_mix = wet.mix(44100);
    assert_eq!(dry_mix, wet_mix);
}

#[test]
fn reverb_full_wet_impulse_output_does_not_clip() {
    let mut world = impulse_world(Some((1.0, 0.0, 1.0)));
    let mixed = world.mix(44100);
    assert!(
        mixed.iter().all(|sample| sample.abs() <= 1.0),
        "full-wet reverb output exceeded the unit range"
    );
}

#[test]
fn source_reverb_mix_zero_sends_nothing_to_the_reverb() {
    let mut world = impulse_world(Some((0.9, 0.5, 1.0)));
    let entity = EntityId::new(90);
    assert_eq!(world.reverb_mix(entity), 1.0);
    world.set_reverb_mix(entity, 0.0);
    assert_eq!(world.reverb_mix(entity), 0.0);
    let mut dry = impulse_world(None);
    let reverbed = world.mix(44100);
    let dry_mix = dry.mix(44100);
    assert_eq!(reverbed, dry_mix);
}

#[test]
fn reverb_parameters_round_trip_through_the_world_api() {
    let mut world = AudioWorld::new();
    assert_eq!(world.reverb(), None);
    assert!(!world.set_reverb(f32::NAN, 0.5, 1.0));
    assert!(!world.set_reverb(0.7, f32::INFINITY, 1.0));
    assert!(world.set_reverb(0.7, 0.25, 0.5));
    assert_eq!(world.reverb(), Some((0.7, 0.25, 0.5)));
    world.clear_reverb();
    assert_eq!(world.reverb(), None);
}
