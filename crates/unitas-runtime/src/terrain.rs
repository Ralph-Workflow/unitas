use std::collections::BTreeMap;

use unitas_core::EntityId;
use unitas_physics::terrain::{
    LargeWorldImpact, Terrain, TerrainBody, TerrainContact, TerrainData, TerrainInstancedDrawData,
    TerrainRayHit, TerrainWindReport, WindZone,
};
use unitas_physics::World;

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
        self.terrain(entity)?.raycast(origin, direction, max_distance)
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
        Ok(())
    }

    pub fn apply_heightfields(&self, world: &mut World) -> Vec<TerrainContact> {
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
