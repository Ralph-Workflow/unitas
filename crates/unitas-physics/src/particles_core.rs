use std::f32::consts::FRAC_PI_2;

use unitas_core::{rotate_vector, EntityId};

const DEFAULT_BURST_INTERVAL: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleSystemState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopAction {
    None,
    Disable,
    Destroy,
    Callback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleShape {
    Point,
    Sphere { radius: f32 },
    Cone { angle: f32, radius: f32 },
    Box { size: [f32; 3] },
    Mesh { bounds: [f32; 3] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomParticleDataStream {
    Custom1,
    Custom2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ForceFieldShape {
    Sphere { radius: f32 },
    Box { half_extents: [f32; 3] },
}

impl ForceFieldShape {
    pub fn contains(&self, field_position: [f32; 3], point: [f32; 3]) -> bool {
        let dx = point[0] - field_position[0];
        let dy = point[1] - field_position[1];
        let dz = point[2] - field_position[2];
        if !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
            return false;
        }
        match *self {
            Self::Sphere { radius } => {
                if !radius.is_finite() || radius < 0.0 {
                    return false;
                }
                dx * dx + dy * dy + dz * dz <= radius * radius
            }
            Self::Box { half_extents } => {
                if !half_extents[0].is_finite()
                    || !half_extents[1].is_finite()
                    || !half_extents[2].is_finite()
                {
                    return false;
                }
                dx.abs() <= half_extents[0].max(0.0)
                    && dy.abs() <= half_extents[1].max(0.0)
                    && dz.abs() <= half_extents[2].max(0.0)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSystemForceField {
    pub position: [f32; 3],
    pub shape: ForceFieldShape,
    pub direction: [f32; 3],
    pub gravity: [f32; 3],
    pub gravity_focus: f32,
    pub drag: f32,
    pub layer: u8,
}

impl Default for ParticleSystemForceField {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            shape: ForceFieldShape::Sphere { radius: 1.0 },
            direction: [0.0; 3],
            gravity: [0.0; 3],
            gravity_focus: 0.0,
            drag: 0.0,
            layer: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub lifetime: f32,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitParams {
    pub start_speed: f32,
    pub start_size: f32,
    pub start_lifetime: f32,
    pub start_color: [f32; 4],
}

impl Default for EmitParams {
    fn default() -> Self {
        Self {
            start_speed: 0.0,
            start_size: 1.0,
            start_lifetime: 1.0,
            start_color: [1.0; 4],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleBurst {
    pub time: f32,
    pub count: u32,
    pub cycles: u32,
    pub interval: f32,
}

impl ParticleBurst {
    pub fn new(time: f32, count: u32, cycles: u32, interval: f32) -> Self {
        Self {
            time: if time.is_finite() { time.max(0.0) } else { 0.0 },
            count,
            cycles: cycles.max(1),
            interval: if interval.is_finite() && interval > 0.0 {
                interval
            } else {
                DEFAULT_BURST_INTERVAL
            },
        }
    }

    pub(crate) fn occurrence(&self, cycle: u32) -> f32 {
        self.time + cycle as f32 * self.interval
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve<const N: usize> {
    pub start: [f32; N],
    pub end: [f32; N],
}

impl<const N: usize> Curve<N> {
    pub const fn new(start: [f32; N], end: [f32; N]) -> Self {
        Self { start, end }
    }

    pub fn sample(&self, t: f32) -> [f32; N] {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut sampled = [0.0f32; N];
        for (index, slot) in sampled.iter_mut().enumerate() {
            *slot = self.start[index] + (self.end[index] - self.start[index]) * t;
        }
        sampled
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticlePropertyCurves {
    pub velocity: Option<Curve<3>>,
    pub force: Option<Curve<3>>,
    pub color: Option<Curve<4>>,
    pub size: Option<Curve<1>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleCollisionEvent {
    pub particle_index: usize,
    pub entity: EntityId,
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleTriggerEvent {
    pub particle_index: usize,
    pub entity: EntityId,
    pub position: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ParticleRuntime {
    pub(crate) base_velocity: [f32; 3],
    pub(crate) force_velocity: [f32; 3],
}

pub(crate) struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub(crate) fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
}

pub(crate) fn sample_shape(
    rng: &mut SplitMix64,
    shape: ParticleShape,
    emit_rotation: [f32; 4],
) -> ([f32; 3], [f32; 3]) {
    let (offset, direction) = match shape {
        ParticleShape::Point => ([0.0; 3], [0.0, 0.0, 1.0]),
        ParticleShape::Sphere { radius } => {
            let radius = if radius.is_finite() {
                radius.max(0.0)
            } else {
                0.0
            };
            (random_point_in_ball(rng, radius), random_unit_vector(rng))
        }
        ParticleShape::Cone { angle, radius } => sample_cone(rng, angle, radius),
        ParticleShape::Box { size } => (random_point_in_box(rng, size), random_unit_vector(rng)),
        ParticleShape::Mesh { bounds } => {
            (random_point_in_box(rng, bounds), random_unit_vector(rng))
        }
    };
    (
        rotate_vector(emit_rotation, offset),
        rotate_vector(emit_rotation, direction),
    )
}

fn sample_cone(rng: &mut SplitMix64, angle: f32, radius: f32) -> ([f32; 3], [f32; 3]) {
    let radius = if radius.is_finite() {
        radius.max(0.0)
    } else {
        0.0
    };
    let half_angle = if angle.is_finite() {
        (angle * 0.5).to_radians().clamp(0.0, FRAC_PI_2 - 0.001)
    } else {
        0.0
    };
    let height = if half_angle > 0.001 {
        radius / half_angle.tan()
    } else {
        0.0
    };
    let fraction = cube_root(rng.next_f32());
    let disc = random_disc_point(rng);
    (
        [
            disc[0] * radius * fraction,
            height * fraction,
            disc[1] * radius * fraction,
        ],
        [0.0, 1.0, 0.0],
    )
}

fn random_unit_vector(rng: &mut SplitMix64) -> [f32; 3] {
    loop {
        let x = rng.next_f32() * 2.0 - 1.0;
        let y = rng.next_f32() * 2.0 - 1.0;
        let squared = x * x + y * y;
        if squared < 1.0 {
            let scale = (2.0 - squared).sqrt();
            return [x * scale, y * scale, 1.0 - squared];
        }
    }
}

fn random_disc_point(rng: &mut SplitMix64) -> [f32; 2] {
    loop {
        let x = rng.next_f32() * 2.0 - 1.0;
        let y = rng.next_f32() * 2.0 - 1.0;
        if x * x + y * y <= 1.0 {
            return [x, y];
        }
    }
}

fn random_point_in_ball(rng: &mut SplitMix64, radius: f32) -> [f32; 3] {
    let direction = random_unit_vector(rng);
    let distance = radius * cube_root(rng.next_f32());
    [
        direction[0] * distance,
        direction[1] * distance,
        direction[2] * distance,
    ]
}

fn random_point_in_box(rng: &mut SplitMix64, size: [f32; 3]) -> [f32; 3] {
    let half = [
        if size[0].is_finite() {
            size[0].abs() * 0.5
        } else {
            0.0
        },
        if size[1].is_finite() {
            size[1].abs() * 0.5
        } else {
            0.0
        },
        if size[2].is_finite() {
            size[2].abs() * 0.5
        } else {
            0.0
        },
    ];
    [
        (rng.next_f32() * 2.0 - 1.0) * half[0],
        (rng.next_f32() * 2.0 - 1.0) * half[1],
        (rng.next_f32() * 2.0 - 1.0) * half[2],
    ]
}

fn cube_root(value: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        return 0.0;
    }
    let mut guess = if value >= 1.0 { value } else { 1.0 };
    for _ in 0..8 {
        guess = (2.0 * guess + value / (guess * guess)) / 3.0;
    }
    guess
}
