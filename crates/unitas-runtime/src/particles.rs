use std::collections::BTreeMap;

use unitas_core::EntityId;
use unitas_physics::particles::{
    Curve, EmitParams, Particle, ParticleCollisionEvent, ParticleCollisionEvent2D,
    ParticlePropertyCurves, ParticleShape, ParticleSystem, ParticleSystem2D, ParticleSystemState,
    ParticleTriggerEvent, ParticleTriggerEvent2D, StopAction,
};
use unitas_physics::two_d::World2D;
use unitas_physics::World;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSystemSnapshot {
    pub entity: EntityId,
    pub seed: u64,
    pub duration: f32,
    pub looping: bool,
    pub state: ParticleSystemState,
    pub emitting: bool,
    pub emission_rate: f32,
    pub shape: ParticleShape,
    pub live_particle_count: usize,
    pub stopped_count: u32,
    pub disabled: bool,
    pub destroyed: bool,
}

#[derive(Default)]
pub struct ParticleRuntimeWorld {
    systems: BTreeMap<EntityId, ParticleSystem>,
}

impl ParticleRuntimeWorld {
    pub fn new() -> Self {
        Self {
            systems: BTreeMap::new(),
        }
    }

    pub fn register_system(&mut self, entity: EntityId, seed: u64) -> &mut ParticleSystem {
        self.systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem::new(seed))
    }

    pub fn insert_system(&mut self, entity: EntityId, system: ParticleSystem) {
        self.systems.insert(entity, system);
    }

    pub fn remove_system(&mut self, entity: EntityId) -> bool {
        self.systems.remove(&entity).is_some()
    }

    pub fn system(&self, entity: EntityId) -> Option<&ParticleSystem> {
        self.systems.get(&entity)
    }

    pub fn system_mut(&mut self, entity: EntityId) -> Option<&mut ParticleSystem> {
        self.systems.get_mut(&entity)
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn snapshot(&self, entity: EntityId) -> Option<ParticleSystemSnapshot> {
        let system = self.systems.get(&entity)?;
        Some(ParticleSystemSnapshot {
            entity,
            seed: system.seed(),
            duration: system.duration(),
            looping: system.is_looping(),
            state: system.state(),
            emitting: system.is_emitting(),
            emission_rate: system.emission_rate(),
            shape: system.shape(),
            live_particle_count: system.get_particles().len(),
            stopped_count: system.stopped_count(),
            disabled: system.is_disabled(),
            destroyed: system.is_destroyed(),
        })
    }

    pub fn play(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.play();
        true
    }

    pub fn pause(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.pause();
        true
    }

    pub fn stop(&mut self, entity: EntityId, action: StopAction) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.stop(action);
        true
    }

    pub fn clear(&mut self, entity: EntityId) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.clear();
        true
    }

    pub fn emit(&mut self, entity: EntityId, count: u32) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.emit(count);
        true
    }

    pub fn emit_with_params(&mut self, entity: EntityId, params: EmitParams, count: u32) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.emit_with_params(params, count);
        true
    }

    pub fn set_curves(&mut self, entity: EntityId, curves: ParticlePropertyCurves) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        if let Some(velocity) = curves.velocity {
            system.set_velocity_over_lifetime(velocity);
        }
        if let Some(force) = curves.force {
            system.set_force_over_lifetime(force);
        }
        if let Some(color) = curves.color {
            system.set_color_over_lifetime(color);
        }
        if let Some(size) = curves.size {
            system.set_size_over_lifetime(size);
        }
        true
    }

    pub fn set_velocity_curve(&mut self, entity: EntityId, curve: Curve<3>) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.set_velocity_over_lifetime(curve);
        true
    }

    pub fn particles(&self, entity: EntityId) -> Option<&[Particle]> {
        self.systems.get(&entity).map(ParticleSystem::get_particles)
    }

    pub fn set_particles(&mut self, entity: EntityId, particles: Vec<Particle>) -> bool {
        let Some(system) = self.systems.get_mut(&entity) else {
            return false;
        };
        system.set_particles(particles);
        true
    }

    pub fn step_all(&mut self, delta_time: f32, physics_world: Option<&World>) {
        let mut destroyed_entities = Vec::new();
        for (entity, system) in &mut self.systems {
            if !system.is_disabled() && !system.is_destroyed() {
                system.simulate(delta_time);
                if let Some(world) = physics_world {
                    system.update_collisions(world);
                }
            }
            if system.is_destroyed() {
                destroyed_entities.push(*entity);
            }
        }
        for entity in destroyed_entities {
            self.systems.remove(&entity);
        }
    }

    pub fn collision_events(&self, entity: EntityId) -> Option<&[ParticleCollisionEvent]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem::get_collision_events)
    }

    pub fn trigger_events(&self, entity: EntityId) -> Option<&[ParticleTriggerEvent]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem::get_trigger_events)
    }
}

/// The two-dimensional counterpart of `ParticleRuntimeWorld`.
#[derive(Default)]
pub struct ParticleRuntimeWorld2D {
    systems: BTreeMap<EntityId, ParticleSystem2D>,
}

impl ParticleRuntimeWorld2D {
    pub fn new() -> Self {
        Self {
            systems: BTreeMap::new(),
        }
    }

    pub fn register_system(&mut self, entity: EntityId, seed: u64) -> &mut ParticleSystem2D {
        self.systems
            .entry(entity)
            .or_insert_with(|| ParticleSystem2D::new(seed))
    }

    pub fn insert_system(&mut self, entity: EntityId, system: ParticleSystem2D) {
        self.systems.insert(entity, system);
    }

    pub fn remove_system(&mut self, entity: EntityId) -> bool {
        self.systems.remove(&entity).is_some()
    }

    pub fn system(&self, entity: EntityId) -> Option<&ParticleSystem2D> {
        self.systems.get(&entity)
    }

    pub fn system_mut(&mut self, entity: EntityId) -> Option<&mut ParticleSystem2D> {
        self.systems.get_mut(&entity)
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn step_all(&mut self, delta_time: f32, physics_world: Option<&World2D>) {
        let mut destroyed_entities = Vec::new();
        for (entity, system) in &mut self.systems {
            if !system.is_disabled() && !system.is_destroyed() {
                system.system_mut().simulate(delta_time);
                if let Some(world) = physics_world {
                    system.update_collisions(world);
                }
            }
            if system.is_destroyed() {
                destroyed_entities.push(*entity);
            }
        }
        for entity in destroyed_entities {
            self.systems.remove(&entity);
        }
    }

    pub fn collision_events(&self, entity: EntityId) -> Option<&[ParticleCollisionEvent2D]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem2D::get_collision_events)
    }

    pub fn trigger_events(&self, entity: EntityId) -> Option<&[ParticleTriggerEvent2D]> {
        self.systems
            .get(&entity)
            .map(ParticleSystem2D::get_trigger_events)
    }
}
