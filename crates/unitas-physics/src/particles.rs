use std::f32::consts::FRAC_PI_2;

use unitas_core::{rotate_vector, EntityId};

use crate::two_d::collider::transform_point2;
use crate::two_d::{Collider2D, ContactFilter2D, World2D};
use crate::world::World;

const DEFAULT_DURATION: f32 = 5.0;
const DEFAULT_BURST_INTERVAL: f32 = 0.1;
const IDENTITY_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const ALL_LAYERS: u32 = u32::MAX;

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
                let hx = if half_extents[0].is_finite() {
                    half_extents[0].max(0.0)
                } else {
                    return false;
                };
                let hy = if half_extents[1].is_finite() {
                    half_extents[1].max(0.0)
                } else {
                    return false;
                };
                let hz = if half_extents[2].is_finite() {
                    half_extents[2].max(0.0)
                } else {
                    return false;
                };
                dx.abs() <= hx && dy.abs() <= hy && dz.abs() <= hz
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

    fn occurrence(&self, cycle: u32) -> f32 {
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
struct ParticleRuntime {
    base_velocity: [f32; 3],
    force_velocity: [f32; 3],
}

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

pub struct ParticleSystem {
    seed: u64,
    rng: SplitMix64,
    duration: f32,
    looping: bool,
    start_state: ParticleSystemState,
    state: ParticleSystemState,
    emission_rate: f32,
    emission_accumulator: f64,
    bursts: Vec<ParticleBurst>,
    burst_index: usize,
    burst_cycle: u32,
    max_particles: usize,
    emitting: bool,
    system_time: f32,
    shape: ParticleShape,
    emit_params: EmitParams,
    curves: ParticlePropertyCurves,
    emit_position: [f32; 3],
    emit_rotation: [f32; 4],
    collision_layer_mask: u32,
    external_forces_enabled: bool,
    external_forces_multiplier: f32,
    external_forces_layer_mask: u32,
    force_fields: Vec<ParticleSystemForceField>,
    particles: Vec<Particle>,
    runtime: Vec<ParticleRuntime>,
    custom_data_1: Vec<[f32; 4]>,
    custom_data_2: Vec<[f32; 4]>,
    collision_events: Vec<ParticleCollisionEvent>,
    trigger_events: Vec<ParticleTriggerEvent>,
    pending_stop_action: Option<StopAction>,
    stopped_callback: Option<Box<dyn FnMut() + Send>>,
    stopped_count: u32,
    disabled: bool,
    destroyed: bool,
}

impl ParticleSystem {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: SplitMix64::new(seed),
            duration: DEFAULT_DURATION,
            looping: false,
            start_state: ParticleSystemState::Playing,
            state: ParticleSystemState::Playing,
            emission_rate: 0.0,
            emission_accumulator: 0.0,
            bursts: Vec::new(),
            burst_index: 0,
            burst_cycle: 0,
            max_particles: usize::MAX,
            emitting: true,
            system_time: 0.0,
            shape: ParticleShape::Point,
            emit_params: EmitParams::default(),
            curves: ParticlePropertyCurves::default(),
            emit_position: [0.0; 3],
            emit_rotation: IDENTITY_ROTATION,
            collision_layer_mask: ALL_LAYERS,
            external_forces_enabled: false,
            external_forces_multiplier: 1.0,
            external_forces_layer_mask: ALL_LAYERS,
            force_fields: Vec::new(),
            particles: Vec::new(),
            runtime: Vec::new(),
            custom_data_1: Vec::new(),
            custom_data_2: Vec::new(),
            collision_events: Vec::new(),
            trigger_events: Vec::new(),
            pending_stop_action: None,
            stopped_callback: None,
            stopped_count: 0,
            disabled: false,
            destroyed: false,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = SplitMix64::new(seed);
    }

    pub fn set_duration(&mut self, seconds: f32) {
        self.duration = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    pub fn set_start_state(&mut self, state: ParticleSystemState) {
        self.start_state = state;
        self.state = state;
        self.emitting = state == ParticleSystemState::Playing;
    }

    pub fn start_state(&self) -> ParticleSystemState {
        self.start_state
    }

    pub fn set_emission_rate(&mut self, rate: f32) {
        self.emission_rate = if rate.is_finite() { rate.max(0.0) } else { 0.0 };
    }

    pub fn emission_rate(&self) -> f32 {
        self.emission_rate
    }

    pub fn set_bursts(&mut self, bursts: Vec<ParticleBurst>) {
        self.bursts = bursts;
        self.bursts
            .sort_by(|left, right| left.time.total_cmp(&right.time));
        self.burst_index = 0;
        self.burst_cycle = 0;
    }

    pub fn bursts(&self) -> &[ParticleBurst] {
        &self.bursts
    }

    pub fn set_max_particles(&mut self, max_particles: usize) {
        self.max_particles = max_particles;
    }

    pub fn max_particles(&self) -> usize {
        self.max_particles
    }

    pub fn set_shape(&mut self, shape: ParticleShape) {
        self.shape = shape;
    }

    pub fn shape(&self) -> ParticleShape {
        self.shape
    }

    pub fn set_emit_params(&mut self, params: EmitParams) {
        self.emit_params = params;
    }

    pub fn emit_params(&self) -> EmitParams {
        self.emit_params
    }

    pub fn set_emit_position(&mut self, position: [f32; 3]) {
        self.emit_position = position;
    }

    pub fn set_emit_rotation(&mut self, rotation: [f32; 4]) {
        self.emit_rotation = rotation;
    }

    pub fn set_velocity_over_lifetime(&mut self, curve: Curve<3>) {
        self.curves.velocity = Some(curve);
    }

    pub fn set_force_over_lifetime(&mut self, curve: Curve<3>) {
        self.curves.force = Some(curve);
    }

    pub fn set_color_over_lifetime(&mut self, curve: Curve<4>) {
        self.curves.color = Some(curve);
    }

    pub fn set_size_over_lifetime(&mut self, curve: Curve<1>) {
        self.curves.size = Some(curve);
    }

    pub fn curves(&self) -> ParticlePropertyCurves {
        self.curves
    }

    pub fn set_collision_layer_mask(&mut self, layer_mask: u32) {
        self.collision_layer_mask = layer_mask;
    }

    pub fn collision_layer_mask(&self) -> u32 {
        self.collision_layer_mask
    }

    pub fn set_external_forces_enabled(&mut self, enabled: bool) {
        self.external_forces_enabled = enabled;
    }

    pub fn is_external_forces_enabled(&self) -> bool {
        self.external_forces_enabled
    }

    pub fn set_external_forces_multiplier(&mut self, multiplier: f32) {
        self.external_forces_multiplier = if multiplier.is_finite() {
            multiplier
        } else {
            0.0
        };
    }

    pub fn external_forces_multiplier(&self) -> f32 {
        self.external_forces_multiplier
    }

    pub fn set_external_forces_layer_mask(&mut self, layer_mask: u32) {
        self.external_forces_layer_mask = layer_mask;
    }

    pub fn external_forces_layer_mask(&self) -> u32 {
        self.external_forces_layer_mask
    }

    pub fn add_force_field(&mut self, field: ParticleSystemForceField) {
        self.force_fields.push(field);
    }

    pub fn clear_force_fields(&mut self) {
        self.force_fields.clear();
    }

    pub fn force_fields(&self) -> &[ParticleSystemForceField] {
        &self.force_fields
    }

    pub fn set_custom_particle_data(
        &mut self,
        stream: CustomParticleDataStream,
        data: &[[f32; 4]],
    ) {
        let count = self.particles.len();
        let mut synced = vec![[0.0; 4]; count];
        for (slot, value) in synced.iter_mut().zip(data.iter().copied()) {
            *slot = value;
        }
        match stream {
            CustomParticleDataStream::Custom1 => self.custom_data_1 = synced,
            CustomParticleDataStream::Custom2 => self.custom_data_2 = synced,
        }
    }

    pub fn get_custom_particle_data(&self, stream: CustomParticleDataStream) -> &[[f32; 4]] {
        match stream {
            CustomParticleDataStream::Custom1 => &self.custom_data_1,
            CustomParticleDataStream::Custom2 => &self.custom_data_2,
        }
    }

    pub fn set_stopped_callback(&mut self, callback: Box<dyn FnMut() + Send>) {
        self.stopped_callback = Some(callback);
    }

    pub fn stopped_count(&self) -> u32 {
        self.stopped_count
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn is_emitting(&self) -> bool {
        self.emitting
    }

    pub fn state(&self) -> ParticleSystemState {
        self.state
    }

    pub fn play(&mut self) {
        if self.disabled || self.destroyed {
            return;
        }
        match self.state {
            ParticleSystemState::Playing => {}
            ParticleSystemState::Paused => {
                self.emitting = true;
                self.state = ParticleSystemState::Playing;
            }
            ParticleSystemState::Stopped => {
                self.rng = SplitMix64::new(self.seed);
                self.system_time = 0.0;
                self.emission_accumulator = 0.0;
                self.burst_index = 0;
                self.burst_cycle = 0;
                self.emitting = true;
                self.state = ParticleSystemState::Playing;
            }
        }
    }

    pub fn pause(&mut self) {
        if self.state == ParticleSystemState::Playing {
            self.state = ParticleSystemState::Paused;
        }
    }

    pub fn stop(&mut self, action: StopAction) {
        self.emitting = false;
        self.state = ParticleSystemState::Stopped;
        self.pending_stop_action = Some(action);
        if self.particles.is_empty() {
            self.apply_pending_stop_action();
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.runtime.clear();
        self.custom_data_1.clear();
        self.custom_data_2.clear();
        if self.pending_stop_action.is_some() {
            self.apply_pending_stop_action();
        }
    }

    pub fn simulate(&mut self, delta_time: f32) {
        if !delta_time.is_finite() || delta_time <= 0.0 {
            return;
        }
        if self.state == ParticleSystemState::Paused || self.disabled || self.destroyed {
            return;
        }
        self.advance(delta_time);
    }

    pub fn emit(&mut self, count: u32) {
        let params = self.emit_params;
        self.spawn(count, params);
    }

    pub fn emit_with_params(&mut self, params: EmitParams, count: u32) {
        self.spawn(count, params);
    }

    pub fn get_particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn set_particles(&mut self, particles: Vec<Particle>) {
        let count = particles.len();
        self.runtime = particles
            .iter()
            .map(|particle| ParticleRuntime {
                base_velocity: particle.velocity,
                force_velocity: [0.0; 3],
            })
            .collect();
        self.particles = particles;
        self.custom_data_1.resize(count, [0.0; 4]);
        self.custom_data_2.resize(count, [0.0; 4]);
    }

    pub fn update_collisions(&mut self, world: &World) {
        self.collision_events.clear();
        self.trigger_events.clear();
        if self.disabled || self.destroyed {
            return;
        }
        for index in 0..self.particles.len() {
            let particle = self.particles[index];
            let radius = (particle.size * 0.5) as f64;
            if radius.is_nan() || radius <= 0.0 {
                continue;
            }
            let center = [
                particle.position[0] as f64,
                particle.position[1] as f64,
                particle.position[2] as f64,
            ];
            let solid = world.overlap_sphere(center, radius, self.collision_layer_mask);
            let overlapping =
                world.overlap_sphere_with_triggers(center, radius, self.collision_layer_mask, true);
            for entity in overlapping {
                if solid.contains(&entity) {
                    self.collision_events.push(ParticleCollisionEvent {
                        particle_index: index,
                        entity,
                        position: particle.position,
                        normal: self.contact_normal(world, entity, center),
                    });
                } else {
                    self.trigger_events.push(ParticleTriggerEvent {
                        particle_index: index,
                        entity,
                        position: particle.position,
                    });
                }
            }
        }
    }

    pub fn get_collision_events(&self) -> &[ParticleCollisionEvent] {
        &self.collision_events
    }

    pub fn get_trigger_events(&self) -> &[ParticleTriggerEvent] {
        &self.trigger_events
    }

    fn contact_normal(&self, world: &World, entity: EntityId, center: [f64; 3]) -> [f32; 3] {
        let Some(collider) = world.position(entity) else {
            return [0.0, 1.0, 0.0];
        };
        let offset = [
            center[0] - collider[0],
            center[1] - collider[1],
            center[2] - collider[2],
        ];
        let length = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        if length > 0.0 {
            [
                (offset[0] / length) as f32,
                (offset[1] / length) as f32,
                (offset[2] / length) as f32,
            ]
        } else {
            [0.0, 1.0, 0.0]
        }
    }

    fn advance(&mut self, delta_time: f32) {
        let emission_window = self.emission_window(delta_time);
        self.emit_over_time(emission_window);
        self.emit_bursts(emission_window);
        self.integrate(delta_time);
        self.system_time += delta_time;
        if self.duration > 0.0 {
            if self.looping {
                if self.system_time >= self.duration {
                    self.system_time %= self.duration;
                    self.emission_accumulator = 0.0;
                    self.burst_index = 0;
                    self.burst_cycle = 0;
                }
            } else if self.system_time >= self.duration {
                self.emitting = false;
            }
        }
        self.remove_expired();
        if !self.emitting && self.particles.is_empty() {
            self.state = ParticleSystemState::Stopped;
            if self.pending_stop_action.is_some() {
                self.apply_pending_stop_action();
            }
        }
    }

    fn emission_window(&self, delta_time: f32) -> f32 {
        if self.looping || self.duration <= 0.0 {
            return delta_time;
        }
        (self.duration - self.system_time).clamp(0.0, delta_time)
    }

    fn emit_over_time(&mut self, window: f32) {
        if !self.emitting || self.emission_rate <= 0.0 || window <= 0.0 {
            return;
        }
        let total = self.emission_accumulator + self.emission_rate as f64 * window as f64;
        let whole = (total + 1e-9).floor().max(0.0);
        self.emission_accumulator = (total - whole).max(0.0);
        if whole >= 1.0 {
            let params = self.emit_params;
            self.spawn(whole as u32, params);
        }
    }

    fn emit_bursts(&mut self, window: f32) {
        if !self.emitting || window <= 0.0 {
            return;
        }
        let end = self.system_time + window;
        while self.burst_index < self.bursts.len() {
            let burst = self.bursts[self.burst_index];
            let occurrence = burst.occurrence(self.burst_cycle);
            if occurrence > end {
                break;
            }
            if occurrence + 1e-6 >= self.system_time {
                let params = self.emit_params;
                self.spawn(burst.count, params);
            }
            self.burst_cycle += 1;
            if self.burst_cycle >= burst.cycles {
                self.burst_index += 1;
                self.burst_cycle = 0;
            }
        }
    }

    fn integrate(&mut self, delta_time: f32) {
        for index in 0..self.particles.len() {
            let lifetime = self.particles[index].lifetime;
            let age = self.particles[index].age + delta_time;
            let progress = if lifetime > 0.0 {
                (age / lifetime).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let mut velocity = self.runtime[index].base_velocity;
            if let Some(curve) = self.curves.velocity {
                velocity = curve.sample(progress);
            }
            if let Some(curve) = self.curves.force {
                let force = curve.sample(progress);
                self.runtime[index].force_velocity = [
                    self.runtime[index].force_velocity[0] + force[0] * delta_time,
                    self.runtime[index].force_velocity[1] + force[1] * delta_time,
                    self.runtime[index].force_velocity[2] + force[2] * delta_time,
                ];
            }
            if self.external_forces_enabled && self.external_forces_multiplier != 0.0 {
                let pos = self.particles[index].position;
                let mult = self.external_forces_multiplier;
                for field in &self.force_fields {
                    if field.layer < 32
                        && (self.external_forces_layer_mask & (1u32 << field.layer)) == 0
                    {
                        continue;
                    }
                    if !field.shape.contains(field.position, pos) {
                        continue;
                    }
                    let mut accel = [
                        (field.direction[0] + field.gravity[0]) * mult,
                        (field.direction[1] + field.gravity[1]) * mult,
                        (field.direction[2] + field.gravity[2]) * mult,
                    ];
                    if field.gravity_focus.is_finite() && field.gravity_focus != 0.0 {
                        let to_center = [
                            field.position[0] - pos[0],
                            field.position[1] - pos[1],
                            field.position[2] - pos[2],
                        ];
                        let dist = (to_center[0] * to_center[0]
                            + to_center[1] * to_center[1]
                            + to_center[2] * to_center[2])
                            .sqrt();
                        if dist > 1e-6 {
                            let scale = field.gravity_focus * mult / dist;
                            accel[0] += to_center[0] * scale;
                            accel[1] += to_center[1] * scale;
                            accel[2] += to_center[2] * scale;
                        }
                    }
                    self.runtime[index].force_velocity = [
                        self.runtime[index].force_velocity[0] + accel[0] * delta_time,
                        self.runtime[index].force_velocity[1] + accel[1] * delta_time,
                        self.runtime[index].force_velocity[2] + accel[2] * delta_time,
                    ];
                    if field.drag.is_finite() && field.drag > 0.0 {
                        let damp = (1.0 - field.drag * mult * delta_time).clamp(0.0, 1.0);
                        self.runtime[index].base_velocity = [
                            self.runtime[index].base_velocity[0] * damp,
                            self.runtime[index].base_velocity[1] * damp,
                            self.runtime[index].base_velocity[2] * damp,
                        ];
                        self.runtime[index].force_velocity = [
                            self.runtime[index].force_velocity[0] * damp,
                            self.runtime[index].force_velocity[1] * damp,
                            self.runtime[index].force_velocity[2] * damp,
                        ];
                    }
                }
                if self.curves.velocity.is_none() {
                    velocity = self.runtime[index].base_velocity;
                }
            }
            velocity = [
                velocity[0] + self.runtime[index].force_velocity[0],
                velocity[1] + self.runtime[index].force_velocity[1],
                velocity[2] + self.runtime[index].force_velocity[2],
            ];
            self.particles[index].velocity = velocity;
            self.particles[index].position = [
                self.particles[index].position[0] + velocity[0] * delta_time,
                self.particles[index].position[1] + velocity[1] * delta_time,
                self.particles[index].position[2] + velocity[2] * delta_time,
            ];
            self.particles[index].age = age;
            if let Some(curve) = self.curves.size {
                self.particles[index].size = curve.sample(progress)[0];
            }
            if let Some(curve) = self.curves.color {
                self.particles[index].color = curve.sample(progress);
            }
        }
    }

    fn remove_expired(&mut self) {
        let mut index = 0;
        while index < self.particles.len() {
            if self.particles[index].age >= self.particles[index].lifetime {
                self.particles.remove(index);
                self.runtime.remove(index);
                if index < self.custom_data_1.len() {
                    self.custom_data_1.remove(index);
                }
                if index < self.custom_data_2.len() {
                    self.custom_data_2.remove(index);
                }
            } else {
                index += 1;
            }
        }
    }

    fn apply_pending_stop_action(&mut self) {
        let Some(action) = self.pending_stop_action.take() else {
            return;
        };
        match action {
            StopAction::None => {}
            StopAction::Disable => self.disabled = true,
            StopAction::Destroy => self.destroyed = true,
            StopAction::Callback => {
                self.stopped_count += 1;
                if let Some(callback) = self.stopped_callback.as_mut() {
                    callback();
                }
            }
        }
    }

    fn spawn(&mut self, count: u32, params: EmitParams) {
        let room = self.max_particles.saturating_sub(self.particles.len());
        let count = (count as usize).min(room);
        let lifetime = if params.start_lifetime.is_finite() {
            params.start_lifetime.max(0.0)
        } else {
            0.0
        };
        let speed = if params.start_speed.is_finite() {
            params.start_speed
        } else {
            0.0
        };
        for _ in 0..count {
            let (offset, direction) = self.sample_shape();
            let position = [
                self.emit_position[0] + offset[0],
                self.emit_position[1] + offset[1],
                self.emit_position[2] + offset[2],
            ];
            let velocity = [
                direction[0] * speed,
                direction[1] * speed,
                direction[2] * speed,
            ];
            self.particles.push(Particle {
                position,
                velocity,
                size: params.start_size,
                color: params.start_color,
                lifetime,
                age: 0.0,
            });
            self.runtime.push(ParticleRuntime {
                base_velocity: velocity,
                force_velocity: [0.0; 3],
            });
            self.custom_data_1.push([0.0; 4]);
            self.custom_data_2.push([0.0; 4]);
        }
    }

    fn sample_shape(&mut self) -> ([f32; 3], [f32; 3]) {
        let (offset, direction) = match self.shape {
            ParticleShape::Point => ([0.0; 3], [0.0, 0.0, 1.0]),
            ParticleShape::Sphere { radius } => {
                let radius = if radius.is_finite() {
                    radius.max(0.0)
                } else {
                    0.0
                };
                (self.random_point_in_ball(radius), self.random_unit_vector())
            }
            ParticleShape::Cone { angle, radius } => {
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
                let fraction = cube_root(self.rng.next_f32());
                let disc = self.random_disc_point();
                (
                    [
                        disc[0] * radius * fraction,
                        height * fraction,
                        disc[1] * radius * fraction,
                    ],
                    [0.0, 1.0, 0.0],
                )
            }
            ParticleShape::Box { size } => {
                (self.random_point_in_box(size), self.random_unit_vector())
            }
            ParticleShape::Mesh { bounds } => {
                (self.random_point_in_box(bounds), self.random_unit_vector())
            }
        };
        (
            rotate_vector(self.emit_rotation, offset),
            rotate_vector(self.emit_rotation, direction),
        )
    }

    fn random_unit_vector(&mut self) -> [f32; 3] {
        loop {
            let x = self.rng.next_f32() * 2.0 - 1.0;
            let y = self.rng.next_f32() * 2.0 - 1.0;
            let squared = x * x + y * y;
            if squared < 1.0 {
                let scale = (2.0 - squared).sqrt();
                return [x * scale, y * scale, 1.0 - squared];
            }
        }
    }

    fn random_disc_point(&mut self) -> [f32; 2] {
        loop {
            let x = self.rng.next_f32() * 2.0 - 1.0;
            let y = self.rng.next_f32() * 2.0 - 1.0;
            if x * x + y * y <= 1.0 {
                return [x, y];
            }
        }
    }

    fn random_point_in_ball(&mut self, radius: f32) -> [f32; 3] {
        let direction = self.random_unit_vector();
        let distance = radius * cube_root(self.rng.next_f32());
        [
            direction[0] * distance,
            direction[1] * distance,
            direction[2] * distance,
        ]
    }

    fn random_point_in_box(&mut self, size: [f32; 3]) -> [f32; 3] {
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
            (self.rng.next_f32() * 2.0 - 1.0) * half[0],
            (self.rng.next_f32() * 2.0 - 1.0) * half[1],
            (self.rng.next_f32() * 2.0 - 1.0) * half[2],
        ]
    }
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

impl std::fmt::Debug for ParticleSystem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParticleSystem")
            .field("seed", &self.seed)
            .field("state", &self.state)
            .field("duration", &self.duration)
            .field("looping", &self.looping)
            .field("emission_rate", &self.emission_rate)
            .field("particle_count", &self.particles.len())
            .field("disabled", &self.disabled)
            .field("destroyed", &self.destroyed)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleCollisionEvent2D {
    pub particle_index: usize,
    pub entity: EntityId,
    pub position: [f32; 2],
    pub normal: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleTriggerEvent2D {
    pub particle_index: usize,
    pub entity: EntityId,
    pub position: [f32; 2],
}

pub struct ParticleSystem2D {
    system: ParticleSystem,
    collision_events: Vec<ParticleCollisionEvent2D>,
    trigger_events: Vec<ParticleTriggerEvent2D>,
}

impl ParticleSystem2D {
    pub fn new(seed: u64) -> Self {
        Self {
            system: ParticleSystem::new(seed),
            collision_events: Vec::new(),
            trigger_events: Vec::new(),
        }
    }

    pub fn system(&self) -> &ParticleSystem {
        &self.system
    }

    pub fn system_mut(&mut self) -> &mut ParticleSystem {
        &mut self.system
    }

    pub fn set_collision_layer_mask(&mut self, layer_mask: u32) {
        self.system.set_collision_layer_mask(layer_mask);
    }

    pub fn collision_layer_mask(&self) -> u32 {
        self.system.collision_layer_mask()
    }

    pub fn is_disabled(&self) -> bool {
        self.system.is_disabled()
    }

    pub fn is_destroyed(&self) -> bool {
        self.system.is_destroyed()
    }

    pub fn update_collisions(&mut self, world: &World2D) {
        self.collision_events.clear();
        self.trigger_events.clear();
        if self.system.is_disabled() || self.system.is_destroyed() {
            return;
        }
        let filter = ContactFilter2D::default().with_layer_mask(self.system.collision_layer_mask());
        for index in 0..self.system.get_particles().len() {
            let particle = self.system.get_particles()[index];
            let radius = particle.size * 0.5;
            if radius.is_nan() || radius <= 0.0 {
                continue;
            }
            let center = [particle.position[0], particle.position[1]];
            for collider_id in world.overlap_circle(center, radius, filter) {
                let Some(collider) = world.collider(collider_id) else {
                    continue;
                };
                if collider.is_trigger {
                    self.trigger_events.push(ParticleTriggerEvent2D {
                        particle_index: index,
                        entity: collider.game_object,
                        position: center,
                    });
                } else {
                    self.collision_events.push(ParticleCollisionEvent2D {
                        particle_index: index,
                        entity: collider.game_object,
                        position: center,
                        normal: contact_normal_2d(world, collider, center),
                    });
                }
            }
        }
    }

    pub fn get_collision_events(&self) -> &[ParticleCollisionEvent2D] {
        &self.collision_events
    }

    pub fn get_trigger_events(&self) -> &[ParticleTriggerEvent2D] {
        &self.trigger_events
    }
}

impl std::fmt::Debug for ParticleSystem2D {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParticleSystem2D")
            .field("seed", &self.system.seed())
            .field("collision_layer_mask", &self.system.collision_layer_mask())
            .field("particle_count", &self.system.get_particles().len())
            .field("collision_event_count", &self.collision_events.len())
            .field("trigger_event_count", &self.trigger_events.len())
            .finish()
    }
}

fn collider_center_2d(world: &World2D, collider: &Collider2D) -> [f32; 2] {
    match collider.attached_body.and_then(|body| world.body(body)) {
        Some(body) => transform_point2(collider.offset, body.position, body.rotation),
        None => collider.offset,
    }
}

fn contact_normal_2d(world: &World2D, collider: &Collider2D, center: [f32; 2]) -> [f32; 2] {
    let collider_center = collider_center_2d(world, collider);
    let offset = [
        center[0] - collider_center[0],
        center[1] - collider_center[1],
    ];
    let length = (offset[0] * offset[0] + offset[1] * offset[1]).sqrt();
    if length > 0.0 {
        [offset[0] / length, offset[1] / length]
    } else {
        [0.0, 1.0]
    }
}
