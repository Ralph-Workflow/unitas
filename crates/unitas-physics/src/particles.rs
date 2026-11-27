use unitas_core::EntityId;

#[path = "particles_core.rs"]
mod particles_core;
#[path = "particles_modules.rs"]
mod particles_modules;
#[path = "particles_sim.rs"]
mod particles_sim;

pub use particles_core::{
    Curve, CustomParticleDataStream, EmitParams, ForceFieldShape, Particle, ParticleBurst,
    ParticleCollisionEvent, ParticlePropertyCurves, ParticleShape, ParticleSystemForceField,
    ParticleSystemState, ParticleTriggerEvent, StopAction,
};
pub use particles_modules::{
    ParticleCollisionEvent2D, ParticleSystem2D, ParticleTriggerEvent2D, ParticleUv,
    SubEmitterConfig, SubEmitterEvent, SubEmitterTrigger, TextureSheetAnimation,
    TextureSheetAnimationType,
};

use particles_core::{sample_shape, ParticleRuntime, SplitMix64};

const DEFAULT_DURATION: f32 = 5.0;
const IDENTITY_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const ALL_LAYERS: u32 = u32::MAX;

pub struct ParticleSystem {
    pub(crate) seed: u64,
    pub(crate) rng: SplitMix64,
    pub(crate) duration: f32,
    pub(crate) looping: bool,
    pub(crate) start_state: ParticleSystemState,
    pub(crate) state: ParticleSystemState,
    pub(crate) emission_rate: f32,
    pub(crate) emission_accumulator: f64,
    pub(crate) bursts: Vec<ParticleBurst>,
    pub(crate) burst_index: usize,
    pub(crate) burst_cycle: u32,
    pub(crate) max_particles: usize,
    pub(crate) emitting: bool,
    pub(crate) system_time: f32,
    pub(crate) shape: ParticleShape,
    pub(crate) emit_params: EmitParams,
    pub(crate) curves: ParticlePropertyCurves,
    pub(crate) emit_position: [f32; 3],
    pub(crate) emit_rotation: [f32; 4],
    pub(crate) collision_layer_mask: u32,
    pub(crate) external_forces_enabled: bool,
    pub(crate) external_forces_multiplier: f32,
    pub(crate) external_forces_layer_mask: u32,
    pub(crate) force_fields: Vec<ParticleSystemForceField>,
    pub(crate) sub_emitters: Vec<SubEmitterConfig>,
    pub(crate) sub_emitter_events: Vec<SubEmitterEvent>,
    pub(crate) texture_sheet_animation: Option<TextureSheetAnimation>,
    pub(crate) particle_uvs: Vec<ParticleUv>,
    pub(crate) particles: Vec<Particle>,
    pub(crate) runtime: Vec<ParticleRuntime>,
    pub(crate) custom_data_1: Vec<[f32; 4]>,
    pub(crate) custom_data_2: Vec<[f32; 4]>,
    pub(crate) collision_events: Vec<ParticleCollisionEvent>,
    pub(crate) trigger_events: Vec<ParticleTriggerEvent>,
    pub(crate) overlap_hits: Vec<(EntityId, bool)>,
    pub(crate) pending_stop_action: Option<StopAction>,
    pub(crate) stopped_callback: Option<Box<dyn FnMut() + Send>>,
    pub(crate) stopped_count: u32,
    pub(crate) disabled: bool,
    pub(crate) destroyed: bool,
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
            sub_emitters: Vec::new(),
            sub_emitter_events: Vec::new(),
            texture_sheet_animation: None,
            particle_uvs: Vec::new(),
            particles: Vec::new(),
            runtime: Vec::new(),
            custom_data_1: Vec::new(),
            custom_data_2: Vec::new(),
            collision_events: Vec::new(),
            trigger_events: Vec::new(),
            overlap_hits: Vec::new(),
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

    pub fn add_sub_emitter(&mut self, config: SubEmitterConfig) {
        self.sub_emitters.push(config);
    }

    pub fn set_sub_emitters(&mut self, sub_emitters: Vec<SubEmitterConfig>) {
        self.sub_emitters = sub_emitters;
    }

    pub fn clear_sub_emitters(&mut self) {
        self.sub_emitters.clear();
    }

    pub fn sub_emitters(&self) -> &[SubEmitterConfig] {
        &self.sub_emitters
    }

    pub fn sub_emitter_events(&self) -> &[SubEmitterEvent] {
        &self.sub_emitter_events
    }

    pub fn drain_sub_emitter_events(&mut self) -> Vec<SubEmitterEvent> {
        std::mem::take(&mut self.sub_emitter_events)
    }

    pub fn set_texture_sheet_animation(&mut self, animation: Option<TextureSheetAnimation>) {
        self.texture_sheet_animation = animation;
        self.refresh_particle_uvs();
    }

    pub fn texture_sheet_animation(&self) -> Option<TextureSheetAnimation> {
        self.texture_sheet_animation
    }

    pub fn particle_uvs(&self) -> &[ParticleUv] {
        &self.particle_uvs
    }

    pub(crate) fn refresh_particle_uvs(&mut self) {
        self.particle_uvs
            .resize(self.particles.len(), ParticleUv::default());
        for (index, particle) in self.particles.iter().enumerate() {
            let progress = if particle.lifetime > 0.0 {
                (particle.age / particle.lifetime).clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.particle_uvs[index] = match self.texture_sheet_animation {
                Some(sheet) => sheet.evaluate_uv(progress),
                None => ParticleUv::default(),
            };
        }
    }

    pub(crate) fn record_sub_emitter_event(
        &mut self,
        trigger: SubEmitterTrigger,
        position: [f32; 3],
        velocity: [f32; 3],
    ) {
        if self.sub_emitters.is_empty() {
            return;
        }
        for index in 0..self.sub_emitters.len() {
            let config = self.sub_emitters[index];
            if config.trigger != trigger || config.emit_count == 0 {
                continue;
            }
            let fires = if config.emit_probability >= 1.0 {
                true
            } else if config.emit_probability <= 0.0 {
                false
            } else {
                self.rng.next_f32() < config.emit_probability
            };
            if fires {
                self.sub_emitter_events.push(SubEmitterEvent {
                    trigger,
                    sub_emitter: config.sub_emitter,
                    position,
                    velocity,
                    emit_count: config.emit_count,
                });
            }
        }
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
