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

#[derive(Default)]
pub struct TrailRuntimeWorld {
    trails: BTreeMap<EntityId, Trail>,
    draw_list: Vec<TrailDrawItem>,
}

impl TrailRuntimeWorld {
    pub fn new() -> Self {
        Self {
            trails: BTreeMap::new(),
            draw_list: Vec::new(),
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

    pub fn points(&self, entity: EntityId) -> Option<&[TrailPoint]> {
        self.trails.get(&entity).map(Trail::points)
    }

    pub fn draw_list(&self) -> &[TrailDrawItem] {
        &self.draw_list
    }

    pub fn step_all(&mut self, delta_time: f32, positions: &BTreeMap<EntityId, [f32; 3]>) {
        for (entity, trail) in &mut self.trails {
            let Some(position) = positions.get(entity) else {
                continue;
            };
            trail.update(delta_time, *position);
        }
        self.rebuild_draw_list();
    }

    pub fn rebuild_draw_list(&mut self) {
        let mut draw_list = Vec::new();
        for (entity, trail) in &self.trails {
            for point in trail.points() {
                draw_list.push(TrailDrawItem {
                    entity: *entity,
                    position: point.position,
                    width: point.width,
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
