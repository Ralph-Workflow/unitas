use unitas_core::EntityId;

use crate::two_d::collider::transform_point2;
use crate::two_d::{Collider2D, ContactFilter2D, World2D};

use super::{Curve, Particle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubEmitterTrigger {
    Birth,
    Collision,
    Death,
    Trigger,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubEmitterConfig {
    pub trigger: SubEmitterTrigger,
    pub sub_emitter: EntityId,
    pub emit_probability: f32,
    pub emit_count: u32,
}

impl SubEmitterConfig {
    pub fn new(trigger: SubEmitterTrigger, sub_emitter: EntityId, emit_count: u32) -> Self {
        Self {
            trigger,
            sub_emitter,
            emit_probability: 1.0,
            emit_count: emit_count.max(1),
        }
    }

    pub fn with_probability(mut self, probability: f32) -> Self {
        self.emit_probability = if probability.is_finite() {
            probability.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubEmitterEvent {
    pub trigger: SubEmitterTrigger,
    pub sub_emitter: EntityId,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub emit_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureSheetAnimationType {
    WholeSheet,
    SingleRow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleUv {
    pub frame_index: u32,
    pub uv_rect: [f32; 4],
}

impl Default for ParticleUv {
    fn default() -> Self {
        Self {
            frame_index: 0,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureSheetAnimation {
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub animation_type: TextureSheetAnimationType,
    pub row_index: u32,
    pub frame_over_time: Curve<1>,
    pub start_frame: f32,
    pub cycles: f32,
}

impl Default for TextureSheetAnimation {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

impl TextureSheetAnimation {
    pub fn new(tiles_x: u32, tiles_y: u32) -> Self {
        Self {
            tiles_x: tiles_x.max(1),
            tiles_y: tiles_y.max(1),
            animation_type: TextureSheetAnimationType::WholeSheet,
            row_index: 0,
            frame_over_time: Curve::new([0.0], [1.0]),
            start_frame: 0.0,
            cycles: 1.0,
        }
    }

    pub fn with_animation_type(mut self, animation_type: TextureSheetAnimationType) -> Self {
        self.animation_type = animation_type;
        self
    }

    pub fn with_row_index(mut self, row_index: u32) -> Self {
        self.row_index = row_index;
        self
    }

    pub fn with_frame_over_time(mut self, curve: Curve<1>) -> Self {
        self.frame_over_time = curve;
        self
    }

    pub fn with_start_frame(mut self, start_frame: f32) -> Self {
        self.start_frame = if start_frame.is_finite() {
            start_frame.max(0.0)
        } else {
            0.0
        };
        self
    }

    pub fn with_cycles(mut self, cycles: f32) -> Self {
        self.cycles = if cycles.is_finite() && cycles > 0.0 {
            cycles
        } else {
            1.0
        };
        self
    }

    pub fn evaluate_uv(&self, normalized_age: f32) -> ParticleUv {
        let tx = self.tiles_x.max(1);
        let ty = self.tiles_y.max(1);
        let cycle_t = self.normalized_cycle_time(normalized_age);
        let sampled = self.frame_over_time.sample(cycle_t)[0].clamp(0.0, 1.0);
        let start = if self.start_frame.is_finite() {
            self.start_frame.max(0.0)
        } else {
            0.0
        };
        let (frame_index, col, row) = self.resolve_frame(tx, ty, start, sampled);
        let w = 1.0 / tx as f32;
        let h = 1.0 / ty as f32;
        ParticleUv {
            frame_index,
            uv_rect: [col as f32 * w, row as f32 * h, w, h],
        }
    }

    fn normalized_cycle_time(&self, normalized_age: f32) -> f32 {
        let cycles = if self.cycles.is_finite() && self.cycles > 0.0 {
            self.cycles
        } else {
            1.0
        };
        let t = if normalized_age.is_finite() {
            normalized_age.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if t >= 1.0 {
            1.0
        } else {
            (t * cycles).fract()
        }
    }

    fn resolve_frame(&self, tx: u32, ty: u32, start: f32, sampled: f32) -> (u32, u32, u32) {
        match self.animation_type {
            TextureSheetAnimationType::WholeSheet => {
                let total = tx * ty;
                let raw = (start + sampled * total as f32).floor() as u32;
                let idx = if sampled >= 1.0 && start == 0.0 {
                    total - 1
                } else {
                    raw % total
                };
                (idx, idx % tx, idx / tx)
            }
            TextureSheetAnimationType::SingleRow => {
                let row = self.row_index.min(ty - 1);
                let raw = (start + sampled * tx as f32).floor() as u32;
                let col = if sampled >= 1.0 && start == 0.0 {
                    tx - 1
                } else {
                    raw % tx
                };
                (row * tx + col, col, row)
            }
        }
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
    system: super::ParticleSystem,
    collision_events: Vec<ParticleCollisionEvent2D>,
    trigger_events: Vec<ParticleTriggerEvent2D>,
}

impl ParticleSystem2D {
    pub fn new(seed: u64) -> Self {
        Self {
            system: super::ParticleSystem::new(seed),
            collision_events: Vec::new(),
            trigger_events: Vec::new(),
        }
    }

    pub fn system(&self) -> &super::ParticleSystem {
        &self.system
    }

    pub fn system_mut(&mut self) -> &mut super::ParticleSystem {
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
                self.record_2d_contact(world, collider, index, particle, center);
            }
        }
    }

    fn record_2d_contact(
        &mut self,
        world: &World2D,
        collider: &Collider2D,
        index: usize,
        particle: Particle,
        center: [f32; 2],
    ) {
        if collider.is_trigger {
            self.trigger_events.push(ParticleTriggerEvent2D {
                particle_index: index,
                entity: collider.game_object,
                position: center,
            });
            self.system.record_sub_emitter_event(
                SubEmitterTrigger::Trigger,
                particle.position,
                particle.velocity,
            );
        } else {
            self.collision_events.push(ParticleCollisionEvent2D {
                particle_index: index,
                entity: collider.game_object,
                position: center,
                normal: contact_normal_2d(world, collider, center),
            });
            self.system.record_sub_emitter_event(
                SubEmitterTrigger::Collision,
                particle.position,
                particle.velocity,
            );
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
