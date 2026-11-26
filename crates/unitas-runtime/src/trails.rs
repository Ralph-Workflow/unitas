use std::collections::BTreeMap;

use unitas_asset::trail::TrailRendererBinding;
use unitas_core::EntityId;
use unitas_physics::trails::{Trail, TrailPoint, TrailSettings};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailDrawItem {
    pub entity: EntityId,
    pub position: [f32; 3],
    pub width: f32,
    pub color: [f32; 4],
}

/// The emission state a trail carries beside its authored settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailEmission {
    /// Whether the trail still records new ribbon points.
    pub emitting: bool,
    /// The emitter's speed in metres per second, measured from its movement.
    pub speed: f32,
    /// The speed at which the ribbon reaches its authored width. Zero keeps
    /// the ribbon at its authored width at every speed.
    pub full_width_speed: f32,
}

impl Default for TrailEmission {
    fn default() -> Self {
        Self {
            emitting: true,
            speed: 0.0,
            full_width_speed: 0.0,
        }
    }
}

#[derive(Default)]
pub struct TrailRuntimeWorld {
    trails: BTreeMap<EntityId, Trail>,
    draw_list: Vec<TrailDrawItem>,
    emission: BTreeMap<EntityId, TrailEmission>,
    last_positions: BTreeMap<EntityId, [f32; 3]>,
}

impl TrailRuntimeWorld {
    pub fn new() -> Self {
        Self {
            trails: BTreeMap::new(),
            draw_list: Vec::new(),
            emission: BTreeMap::new(),
            last_positions: BTreeMap::new(),
        }
    }

    pub fn register_trail(&mut self, entity: EntityId, settings: TrailSettings) -> &mut Trail {
        self.trails
            .entry(entity)
            .or_insert_with(|| Trail::new(settings))
    }

    pub fn register_binding(
        &mut self,
        entity: EntityId,
        binding: &TrailRendererBinding,
    ) -> &mut Trail {
        self.register_trail(entity, settings_of(binding))
    }

    pub fn remove_trail(&mut self, entity: EntityId) -> bool {
        self.trails.remove(&entity).is_some()
    }

    pub fn trail(&self, entity: EntityId) -> Option<&Trail> {
        self.trails.get(&entity)
    }

    pub fn trail_mut(&mut self, entity: EntityId) -> Option<&mut Trail> {
        self.trails.get_mut(&entity)
    }

    pub fn trail_count(&self) -> usize {
        self.trails.len()
    }

    pub fn trail_entities(&self) -> Vec<EntityId> {
        self.trails.keys().copied().collect()
    }

    /// Sets whether a trail records new ribbon points. A trail that stops
    /// emitting keeps the ribbon it already holds until that ribbon fades.
    pub fn set_emitting(&mut self, entity: EntityId, emitting: bool) {
        self.emission.entry(entity).or_default().emitting = emitting;
    }

    pub fn emitting(&self, entity: EntityId) -> bool {
        self.emission
            .get(&entity)
            .is_none_or(|state| state.emitting)
    }

    /// Sets the speed at which a trail's ribbon reaches its authored width.
    pub fn set_full_width_speed(&mut self, entity: EntityId, speed: f32) {
        if !speed.is_finite() || speed < 0.0 {
            return;
        }
        self.emission.entry(entity).or_default().full_width_speed = speed;
    }

    /// The emitter's measured speed in metres per second.
    pub fn emitter_speed(&self, entity: EntityId) -> f32 {
        self.emission.get(&entity).map_or(0.0, |state| state.speed)
    }

    /// The fraction of the authored width a trail's ribbon reaches at its
    /// emitter's current speed.
    pub fn width_factor(&self, entity: EntityId) -> f32 {
        let Some(state) = self.emission.get(&entity) else {
            return 1.0;
        };
        if state.full_width_speed <= 0.0 {
            return 1.0;
        }
        (state.speed / state.full_width_speed).clamp(0.0, 1.0)
    }

    pub fn points(&self, entity: EntityId) -> Option<&[TrailPoint]> {
        self.trails.get(&entity).map(Trail::points)
    }

    pub fn draw_list(&self) -> &[TrailDrawItem] {
        &self.draw_list
    }

    pub fn step_all(&mut self, delta_time: f32, positions: &BTreeMap<EntityId, [f32; 3]>) {
        let emission = &self.emission;
        for (entity, trail) in &mut self.trails {
            let Some(position) = positions.get(entity) else {
                continue;
            };
            if !emission.get(entity).is_none_or(|state| state.emitting) {
                continue;
            }
            trail.update(delta_time, *position);
        }
        self.measure_speeds(delta_time, positions);
        self.rebuild_draw_list();
    }

    /// Measures each emitter's speed from its own movement so a trail can
    /// follow the speed of the object that carries it.
    fn measure_speeds(&mut self, delta_time: f32, positions: &BTreeMap<EntityId, [f32; 3]>) {
        if delta_time <= 0.0 || !delta_time.is_finite() {
            return;
        }
        for (entity, position) in positions {
            let previous = self.last_positions.insert(*entity, *position);
            let Some(previous) = previous else {
                self.emission.entry(*entity).or_default().speed = 0.0;
                continue;
            };
            let dx = position[0] - previous[0];
            let dy = position[1] - previous[1];
            let dz = position[2] - previous[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            self.emission.entry(*entity).or_default().speed = distance / delta_time;
        }
    }

    pub fn rebuild_draw_list(&mut self) {
        let mut draw_list = Vec::new();
        for (entity, trail) in &self.trails {
            let factor = self.width_factor(*entity);
            for point in trail.points() {
                draw_list.push(TrailDrawItem {
                    entity: *entity,
                    position: point.position,
                    width: point.width * factor,
                    color: point.color,
                });
            }
        }
        self.draw_list = draw_list;
    }
}

pub fn settings_of(binding: &TrailRendererBinding) -> TrailSettings {
    TrailSettings {
        time: binding.time,
        width_start: binding.width_start,
        width_end: binding.width_end,
        start_color: binding.start_color,
        end_color: binding.end_color,
        min_vertex_distance: binding.min_vertex_distance,
    }
}
