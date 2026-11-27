use crate::world::World;
use unitas_core::EntityId;

use super::{
    EmitParams, Particle, ParticleCollisionEvent, ParticleRuntime, ParticleSystem,
    ParticleSystemState, ParticleTriggerEvent, ParticleUv, SplitMix64, StopAction,
    SubEmitterTrigger, sample_shape,
};

impl ParticleSystem {
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
        self.particle_uvs.clear();
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
        self.refresh_particle_uvs();
    }

    pub fn update_collisions(&mut self, world: &World) {
        self.collision_events.clear();
        self.trigger_events.clear();
        if self.disabled || self.destroyed {
            return;
        }
        let layer_mask = self.collision_layer_mask;
        let bodies = world.query_bodies();
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
            world.overlap_sphere_classified(
                &bodies,
                center,
                radius,
                layer_mask,
                &mut self.overlap_hits,
            );
            self.record_3d_contacts(world, index, particle, center);
        }
    }

    fn record_3d_contacts(
        &mut self,
        world: &World,
        index: usize,
        particle: Particle,
        center: [f64; 3],
    ) {
        for hit_index in 0..self.overlap_hits.len() {
            let (entity, is_trigger) = self.overlap_hits[hit_index];
            if is_trigger {
                self.trigger_events.push(ParticleTriggerEvent {
                    particle_index: index,
                    entity,
                    position: particle.position,
                });
                self.record_sub_emitter_event(
                    SubEmitterTrigger::Trigger,
                    particle.position,
                    particle.velocity,
                );
            } else {
                let normal = self.contact_normal(world, entity, center);
                self.collision_events.push(ParticleCollisionEvent {
                    particle_index: index,
                    entity,
                    position: particle.position,
                    normal,
                });
                self.record_sub_emitter_event(
                    SubEmitterTrigger::Collision,
                    particle.position,
                    particle.velocity,
                );
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
            self.integrate_particle(index, delta_time);
        }
    }

    fn integrate_particle(&mut self, index: usize, delta_time: f32) {
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
            self.apply_force_fields(index, delta_time);
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
        if let Some(sheet) = self.texture_sheet_animation {
            if index < self.particle_uvs.len() {
                self.particle_uvs[index] = sheet.evaluate_uv(progress);
            }
        }
    }

    fn apply_force_fields(&mut self, index: usize, delta_time: f32) {
        let pos = self.particles[index].position;
        let mult = self.external_forces_multiplier;
        for field_idx in 0..self.force_fields.len() {
            let field = self.force_fields[field_idx];
            if field.layer < 32 && (self.external_forces_layer_mask & (1u32 << field.layer)) == 0 {
                continue;
            }
            if !field.shape.contains(field.position, pos) {
                continue;
            }
            self.apply_single_force_field(index, pos, mult, delta_time, &field);
        }
    }

    fn apply_single_force_field(
        &mut self,
        index: usize,
        pos: [f32; 3],
        mult: f32,
        delta_time: f32,
        field: &super::ParticleSystemForceField,
    ) {
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

    fn remove_expired(&mut self) {
        let mut index = 0;
        while index < self.particles.len() {
            if self.particles[index].age >= self.particles[index].lifetime {
                let expired = self.particles.remove(index);
                self.runtime.remove(index);
                if index < self.particle_uvs.len() {
                    self.particle_uvs.remove(index);
                }
                if index < self.custom_data_1.len() {
                    self.custom_data_1.remove(index);
                }
                if index < self.custom_data_2.len() {
                    self.custom_data_2.remove(index);
                }
                self.record_sub_emitter_event(
                    SubEmitterTrigger::Death,
                    expired.position,
                    expired.velocity,
                );
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
            let (offset, direction) = sample_shape(&mut self.rng, self.shape, self.emit_rotation);
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
            let uv = match self.texture_sheet_animation {
                Some(sheet) => sheet.evaluate_uv(0.0),
                None => ParticleUv::default(),
            };
            self.particle_uvs.push(uv);
            self.custom_data_1.push([0.0; 4]);
            self.custom_data_2.push([0.0; 4]);
            self.record_sub_emitter_event(SubEmitterTrigger::Birth, position, velocity);
        }
    }
}
