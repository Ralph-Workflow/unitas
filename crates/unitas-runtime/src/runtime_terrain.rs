use std::path::Path;

use unitas_asset::meta::GuidIndex;
use unitas_asset::scene_terrain::{TerrainBinding, collect_scene_terrains};
use unitas_asset::unity_yaml;
use unitas_core::{EntityId, SceneGraph};
use unitas_physics::terrain::TerrainData;

use super::Runtime;

impl Runtime {
    pub fn register_scene_terrains_from_path(
        &mut self,
        scene_path: &Path,
        entity_base: u64,
        graph: &SceneGraph,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) -> Vec<EntityId> {
        let Ok(stream) = unity_yaml::parse_file(scene_path) else {
            return Vec::new();
        };
        let index = discover_terrain_guid_index(scene_path);
        let bindings = collect_scene_terrains(&stream, index.as_ref());
        self.register_scene_terrains(&bindings, entity_base, graph, diagnostics)
    }

    pub fn register_scene_terrains(
        &mut self,
        bindings: &[TerrainBinding],
        entity_base: u64,
        graph: &SceneGraph,
        diagnostics: &mut Vec<(EntityId, String)>,
    ) -> Vec<EntityId> {
        let mut terrain_entities = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let entity = EntityId::new(entity_base + binding.entity.as_u64());
            for diag in &binding.diagnostics {
                diagnostics.push((entity, diag.clone()));
            }
            let res = binding.terrain_data.heightmap_resolution.max(1);
            let size = binding.terrain_data.size;
            let Ok(data) = TerrainData::new(res, size) else {
                continue;
            };
            let max_y = size[1];
            let mut updates = Vec::with_capacity(binding.terrain_data.heights.len());
            for row in 0..res {
                for col in 0..res {
                    let idx = row * res + col;
                    if let Some(&norm_h) = binding.terrain_data.heights.get(idx) {
                        updates.push(([col, row], norm_h.clamp(0.0, 1.0) * max_y));
                    }
                }
            }
            data.set_heights(&updates);
            let world_origin = graph
                .world_transform(entity)
                .map_or([0.0; 3], |t| {
                    [
                        f64::from(t.translation[0]),
                        f64::from(t.translation[1]),
                        f64::from(t.translation[2]),
                    ]
                });
            let center_position = [
                world_origin[0] + size[0] * 0.5,
                world_origin[1],
                world_origin[2] + size[2] * 0.5,
            ];
            let _ = self.terrain.remove_terrain(entity);
            if self
                .terrain
                .register_terrain(entity, &data, center_position)
                .is_ok()
            {
                if !binding.enabled
                    && let Some(terrain) = self.terrain.terrain_mut(entity)
                {
                    *terrain = terrain.clone().with_collider_enabled(false);
                }
                terrain_entities.push(entity);
            }
        }
        terrain_entities
    }
}

fn discover_terrain_guid_index(scene_path: &Path) -> Option<GuidIndex> {
    let parent = scene_path.parent()?;
    let mut index_root = parent.to_path_buf();
    for ancestor in parent.ancestors() {
        if ancestor.join("ProjectSettings").is_dir() {
            let assets = ancestor.join("Assets");
            index_root = if assets.is_dir() {
                assets
            } else {
                ancestor.to_path_buf()
            };
            break;
        }
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("Assets") {
            index_root = ancestor.to_path_buf();
            break;
        }
    }
    GuidIndex::build(&index_root).ok()
}
