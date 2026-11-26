use unitas_physics::trails::{Trail, TrailSettings};

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
}

fn trail_settings() -> TrailSettings {
    TrailSettings {
        time: 1.0,
        width_start: 1.0,
        width_end: 0.0,
        start_color: [1.0, 0.0, 0.0, 1.0],
        end_color: [0.0, 0.0, 1.0, 0.0],
        min_vertex_distance: 0.0,
    }
}

fn seeded_path(seed: u64, steps: usize) -> Vec<[f32; 3]> {
    let mut rng = SplitMix64::new(seed);
    let mut position = [0.0f32; 3];
    let mut path = Vec::with_capacity(steps);
    for _ in 0..steps {
        position[0] += (rng.next_f32() - 0.5) * 0.5;
        position[1] += (rng.next_f32() - 0.5) * 0.5;
        path.push(position);
    }
    path
}

fn ribbon_positions(path: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut trail = Trail::new(trail_settings());
    for position in path {
        trail.update(1.0 / 60.0, *position);
    }
    trail
        .points()
        .iter()
        .map(|point| point.position)
        .collect::<Vec<[f32; 3]>>()
}

#[test]
fn a_seeded_emitter_produces_the_same_ribbon_on_every_run() {
    let path = seeded_path(42, 60);
    let first = ribbon_positions(&path);
    let second = ribbon_positions(&path);
    assert!(!first.is_empty());
    assert_eq!(first.len(), second.len());
    assert_eq!(first, second);
}

#[test]
fn a_different_seed_produces_a_different_ribbon() {
    let first = ribbon_positions(&seeded_path(42, 60));
    let second = ribbon_positions(&seeded_path(43, 60));
    assert_eq!(first.len(), second.len());
    assert_ne!(first, second);
}

#[test]
fn a_point_older_than_the_trail_time_expires() {
    let settings = trail_settings();
    let mut trail = Trail::new(settings);
    for step in 0..20 {
        trail.update(0.1, [step as f32, 0.0, 0.0]);
    }
    let points = trail.points();
    assert!(points.len() > 1);
    assert!(points.len() < 20);
    for point in points {
        assert!(point.age <= settings.time);
    }
    let oldest = points.first().map(|point| point.age).unwrap_or(0.0);
    assert!(oldest > settings.time - 0.1);
}

#[test]
fn an_emitter_within_the_min_vertex_distance_adds_no_point() {
    let mut trail = Trail::new(TrailSettings {
        min_vertex_distance: 0.5,
        ..trail_settings()
    });
    trail.update(0.016, [0.0, 0.0, 0.0]);
    assert_eq!(trail.point_count(), 1);
    for step in 0..9 {
        let offset = (step % 4) as f32 * 0.1;
        trail.update(0.016, [offset, 0.0, 0.0]);
    }
    assert_eq!(trail.point_count(), 1);
}

#[test]
fn an_emitter_above_the_min_vertex_distance_adds_a_point() {
    let mut trail = Trail::new(TrailSettings {
        min_vertex_distance: 0.5,
        ..trail_settings()
    });
    trail.update(0.016, [0.0, 0.0, 0.0]);
    trail.update(0.016, [1.0, 0.0, 0.0]);
    assert_eq!(trail.point_count(), 2);
}

#[test]
fn a_trail_fades_width_and_color_from_the_head_to_the_tail() {
    let mut trail = Trail::new(trail_settings());
    trail.update(0.25, [0.0, 0.0, 0.0]);
    trail.update(0.25, [1.0, 0.0, 0.0]);
    let points = trail.points();
    assert_eq!(points.len(), 2);
    let tail = points[0];
    let head = points[1];
    assert!((tail.width - 0.75).abs() < 1e-5);
    assert!((head.width - 1.0).abs() < 1e-5);
    assert!((tail.color[0] - 0.75).abs() < 1e-5);
    assert!((tail.color[2] - 0.25).abs() < 1e-5);
    assert!((tail.color[3] - 0.75).abs() < 1e-5);
    assert!((head.color[0] - 1.0).abs() < 1e-5);
}

#[test]
fn a_cleared_trail_keeps_no_points() {
    let mut trail = Trail::new(trail_settings());
    trail.update(0.016, [0.0, 0.0, 0.0]);
    trail.update(0.016, [1.0, 0.0, 0.0]);
    assert_eq!(trail.point_count(), 2);
    trail.clear();
    assert_eq!(trail.point_count(), 0);
    assert!(trail.is_empty());
}
