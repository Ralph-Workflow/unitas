use std::collections::BTreeMap;

use unitas_core::EntityId;
use unitas_physics::World;
use unitas_physics::terrain::{
    LargeWorldImpact, Terrain, TerrainBody, TerrainContact, TerrainData, TerrainInstancedDrawData,
    TerrainRayHit, TerrainWindReport, WindZone,
};

pub const DEFAULT_TERRAIN_RESOLUTION: usize = 33;
pub const DEFAULT_TERRAIN_SIZE: [f64; 3] = [1000.0, 600.0, 1000.0];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainRuntimeError {
    DuplicateTerrain(EntityId),
    UnknownTerrain(EntityId),
}

impl std::fmt::Display for TerrainRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateTerrain(entity) => {
                write!(formatter, "terrain entity {entity} is already registered")
            }
            Self::UnknownTerrain(entity) => {
                write!(formatter, "terrain entity {entity} is not registered")
            }
        }
    }
}

impl std::error::Error for TerrainRuntimeError {}

#[derive(Clone, Debug, Default)]
pub struct TerrainWorld {
    terrains: BTreeMap<EntityId, Terrain>,
    heightfields: Vec<(EntityId, TerrainBody)>,
    wind_zones: BTreeMap<EntityId, WindZone>,
    heightfield_apply_pending: bool,
    heightfield_apply_count: u64,
}

impl TerrainWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_terrain(
        &mut self,
        entity: EntityId,
        data: &TerrainData,
        position: [f64; 3],
    ) -> Result<(), TerrainRuntimeError> {
        if self.terrains.contains_key(&entity) {
            return Err(TerrainRuntimeError::DuplicateTerrain(entity));
        }
        self.terrains
            .insert(entity, Terrain::new(data.clone()).with_position(position));
        Ok(())
    }

    pub fn ensure_terrain(&mut self, entity: EntityId, resolution: usize) -> bool {
        let res = if resolution == 0 {
            DEFAULT_TERRAIN_RESOLUTION
        } else {
            resolution
        };
        if self.terrains.contains_key(&entity) {
            if resolution > 0 {
                return self.set_resolution(entity, res);
            }
            return true;
        }
        let Ok(data) = TerrainData::new(res, DEFAULT_TERRAIN_SIZE) else {
            return false;
        };
        self.terrains.insert(entity, Terrain::new(data));
        true
    }

    pub fn set_terrain_data(
        &mut self,
        entity: EntityId,
        resolution: usize,
        size: [f64; 3],
        position: [f64; 3],
    ) -> bool {
        let Ok(data) = TerrainData::new(resolution, size) else {
            return false;
        };
        self.terrains
            .insert(entity, Terrain::new(data).with_position(position));
        true
    }

    pub fn has_terrain(&self, entity: EntityId) -> bool {
        self.terrains.contains_key(&entity)
    }

    pub fn position(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.terrains.get(&entity).map(Terrain::position)
    }

    pub fn set_position(&mut self, entity: EntityId, position: [f64; 3]) -> bool {
        if !position.iter().all(|c| c.is_finite()) {
            return false;
        }
        let Some(existing) = self.terrains.get(&entity) else {
            return false;
        };
        let updated = existing.clone().with_position(position);
        self.terrains.insert(entity, updated);
        true
    }

    pub fn resolution(&self, entity: EntityId) -> Option<[usize; 2]> {
        let terrain = self.terrains.get(&entity)?;
        let res = terrain.data().heightmap_resolution();
        Some([res, res])
    }

    pub fn set_resolution(&mut self, entity: EntityId, resolution: usize) -> bool {
        if resolution == 0 {
            return false;
        }
        let Some(existing) = self.terrains.get(&entity) else {
            return false;
        };
        if existing.data().heightmap_resolution() == resolution {
            return true;
        }
        let size = existing.size();
        let position = existing.position();
        self.set_terrain_data(entity, resolution, size, position)
    }

    pub fn size(&self, entity: EntityId) -> Option<[f64; 3]> {
        self.terrains.get(&entity).map(Terrain::size)
    }

    pub fn set_size(&mut self, entity: EntityId, size: [f64; 3]) -> bool {
        if !size.iter().all(|c| c.is_finite() && *c > 0.0) {
            return false;
        }
        let Some(existing) = self.terrains.get(&entity) else {
            return false;
        };
        let old_size = existing.size();
        let res = existing.data().heightmap_resolution();
        let pos = existing.position();
        let old_heights = existing.data().heights();
        let Ok(new_data) = TerrainData::new(res, size) else {
            return false;
        };
        let scale_y = if old_size[1] > 0.0 {
            size[1] / old_size[1]
        } else {
            1.0
        };
        let mut updates = Vec::with_capacity(old_heights.len());
        for row in 0..res {
            for col in 0..res {
                let idx = row * res + col;
                if let Some(&h) = old_heights.get(idx) {
                    updates.push(([col, row], h * scale_y));
                }
            }
        }
        new_data.set_heights(&updates);
        self.terrains
            .insert(entity, Terrain::new(new_data).with_position(pos));
        true
    }

    pub fn set_resolution_and_size(
        &mut self,
        entity: EntityId,
        resolution: usize,
        size: [f64; 3],
    ) -> bool {
        let pos = self.position(entity).unwrap_or([0.0; 3]);
        self.set_terrain_data(entity, resolution, size, pos)
    }

    pub fn get_heights(
        &self,
        entity: EntityId,
        x_base: usize,
        y_base: usize,
        width: usize,
        height: usize,
    ) -> Option<Vec<f32>> {
        let terrain = self.terrains.get(&entity)?;
        let res = terrain.data().heightmap_resolution();
        if x_base.checked_add(width)? > res || y_base.checked_add(height)? > res {
            return None;
        }
        let max_y = terrain.size()[1];
        let inv_y = if max_y > 0.0 { 1.0 / max_y } else { 0.0 };
        let mut out = Vec::with_capacity(width * height);
        for row in 0..height {
            for col in 0..width {
                let world_h = terrain.data().height_at(x_base + col, y_base + row);
                out.push((world_h * inv_y) as f32);
            }
        }
        Some(out)
    }

    pub fn set_heights_block(
        &self,
        entity: EntityId,
        x_base: usize,
        y_base: usize,
        width: usize,
        height: usize,
        heights_normalized: &[f32],
    ) -> bool {
        let Some(terrain) = self.terrains.get(&entity) else {
            return false;
        };
        let res = terrain.data().heightmap_resolution();
        let Some(end_x) = x_base.checked_add(width) else {
            return false;
        };
        let Some(end_y) = y_base.checked_add(height) else {
            return false;
        };
        if end_x > res || end_y > res || heights_normalized.len() < width * height {
            return false;
        }
        let max_y = terrain.size()[1];
        let mut updates = Vec::with_capacity(width * height);
        for row in 0..height {
            for col in 0..width {
                let sample = f64::from(heights_normalized[row * width + col]);
                if !sample.is_finite() {
                    return false;
                }
                updates.push(([x_base + col, y_base + row], sample.clamp(0.0, 1.0) * max_y));
            }
        }
        terrain.data().set_heights(&updates);
        true
    }

    pub fn sample_height(&self, entity: EntityId, world_position: [f64; 3]) -> Option<f64> {
        let terrain = self.terrains.get(&entity)?;
        let size = terrain.size();
        let pos = terrain.position();
        let half_x = size[0] * 0.5;
        let half_z = size[2] * 0.5;
        let clamped = [
            world_position[0].clamp(pos[0] - half_x, pos[0] + half_x),
            world_position[1],
            world_position[2].clamp(pos[2] - half_z, pos[2] + half_z),
        ];
        terrain.height_at(clamped).map(|world_y| world_y - pos[1])
    }

    pub fn remove_terrain(&mut self, entity: EntityId) -> Option<Terrain> {
        self.heightfields.retain(|(terrain, _)| *terrain != entity);
        self.terrains.remove(&entity)
    }

    pub fn terrain(&self, entity: EntityId) -> Option<&Terrain> {
        self.terrains.get(&entity)
    }

    pub fn terrain_mut(&mut self, entity: EntityId) -> Option<&mut Terrain> {
        self.terrains.get_mut(&entity)
    }

    pub fn set_heights(
        &self,
        entity: EntityId,
        updates: &[([usize; 2], f64)],
    ) -> Result<(), TerrainRuntimeError> {
        let terrain = self
            .terrains
            .get(&entity)
            .ok_or(TerrainRuntimeError::UnknownTerrain(entity))?;
        terrain.data().set_heights(updates);
        Ok(())
    }

    pub fn height_at(&self, entity: EntityId, world_position: [f64; 3]) -> Option<f64> {
        self.terrain(entity)?.height_at(world_position)
    }

    pub fn normal_at(&self, entity: EntityId, world_position: [f64; 3]) -> Option<[f64; 3]> {
        self.terrain(entity)?.normal_at(world_position)
    }

    pub fn is_hole(&self, entity: EntityId, world_position: [f64; 3]) -> bool {
        self.terrain(entity)
            .is_some_and(|terrain| terrain.is_hole(world_position))
    }

    pub fn raycast(
        &self,
        entity: EntityId,
        origin: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
    ) -> Option<TerrainRayHit> {
        self.terrain(entity)?
            .raycast(origin, direction, max_distance)
    }

    pub fn register_wind_zone(&mut self, entity: EntityId, wind_zone: WindZone) {
        self.wind_zones.insert(entity, wind_zone);
    }

    pub fn wind_zone(&self, entity: EntityId) -> Option<&WindZone> {
        self.wind_zones.get(&entity)
    }

    pub fn evaluate_wind(
        &self,
        terrain_entity: EntityId,
        wind_entity: EntityId,
        time_seconds: f64,
    ) -> Result<TerrainWindReport, TerrainRuntimeError> {
        let terrain = self
            .terrains
            .get(&terrain_entity)
            .ok_or(TerrainRuntimeError::UnknownTerrain(terrain_entity))?;
        let zone = self
            .wind_zones
            .get(&wind_entity)
            .copied()
            .unwrap_or_else(|| WindZone::directional([1.0, 0.0, 0.0], 0.0, 0.0));
        Ok(terrain.evaluate_wind(&zone, time_seconds))
    }

    pub fn instanced_draw_data(
        &self,
        terrain_entity: EntityId,
    ) -> Result<TerrainInstancedDrawData, TerrainRuntimeError> {
        let terrain = self
            .terrains
            .get(&terrain_entity)
            .ok_or(TerrainRuntimeError::UnknownTerrain(terrain_entity))?;
        Ok(terrain.instanced_draw_data())
    }

    pub fn register_heightfield(
        &mut self,
        terrain_entity: EntityId,
        body: TerrainBody,
    ) -> Result<(), TerrainRuntimeError> {
        if !self.terrains.contains_key(&terrain_entity) {
            return Err(TerrainRuntimeError::UnknownTerrain(terrain_entity));
        }
        self.heightfields.push((terrain_entity, body));
        self.heightfield_apply_pending = true;
        Ok(())
    }

    pub fn needs_heightfield_apply(&self) -> bool {
        !self.heightfields.is_empty()
            && (self.heightfield_apply_pending || self.heightfield_apply_count == 0)
    }

    pub fn mark_heightfields_dirty(&mut self) {
        self.heightfield_apply_pending = true;
    }

    pub fn apply_count(&self) -> u64 {
        self.heightfield_apply_count
    }

    pub fn apply_heightfields(&mut self, world: &mut World) -> Vec<TerrainContact> {
        self.heightfield_apply_count += 1;
        self.heightfield_apply_pending = false;
        let mut contacts = Vec::new();
        for (terrain_entity, body) in &self.heightfields {
            let Some(terrain) = self.terrains.get(terrain_entity) else {
                continue;
            };
            if let Some(contact) = terrain.support_body(world, body) {
                contacts.push(contact);
            }
        }
        contacts
    }
}

pub fn far_position_impact(position: [f64; 3]) -> LargeWorldImpact {
    LargeWorldImpact::for_position(position)
}

#[cfg(test)]
mod tests {
    use unitas_core::EntityId;

    use super::{DEFAULT_TERRAIN_RESOLUTION, DEFAULT_TERRAIN_SIZE, TerrainWorld};

    #[test]
    fn ensure_terrain_get_set_heights_block_and_sample_height_round_trip() {
        let mut world = TerrainWorld::new();
        let entity = EntityId::new(42);
        assert!(world.ensure_terrain(entity, 0));
        assert!(world.has_terrain(entity));
        assert_eq!(
            world.resolution(entity),
            Some([DEFAULT_TERRAIN_RESOLUTION, DEFAULT_TERRAIN_RESOLUTION])
        );
        assert_eq!(world.size(entity), Some(DEFAULT_TERRAIN_SIZE));

        assert!(world.set_resolution(entity, 3));
        assert_eq!(world.resolution(entity), Some([3, 3]));
        assert!(world.set_size(entity, [30.0, 40.0, 30.0]));
        assert_eq!(world.size(entity), Some([30.0, 40.0, 30.0]));
        assert!(world.set_position(entity, [15.0, 0.0, 15.0]));
        assert_eq!(world.position(entity), Some([15.0, 0.0, 15.0]));

        let heights = [0.25f32, 0.25, 0.25, 0.50, 0.50, 0.50, 0.75, 0.75, 0.75];
        assert!(world.set_heights_block(entity, 0, 0, 3, 3, &heights));
        let fetched = world.get_heights(entity, 0, 0, 3, 3).unwrap_or_default();
        assert_eq!(fetched.len(), 9);
        assert!((fetched[0] - 0.25).abs() < 1e-5);
        assert!((fetched[4] - 0.50).abs() < 1e-5);
        assert!((fetched[8] - 0.75).abs() < 1e-5);

        let sampled = world
            .sample_height(entity, [15.0, 0.0, 15.0])
            .unwrap_or(-1.0);
        assert!((sampled - 20.0).abs() < 1e-5);
    }
}
