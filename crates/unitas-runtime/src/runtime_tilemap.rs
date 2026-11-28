use std::path::Path;
use std::sync::Arc;

use unitas_asset::database::tilemap::scene::{SceneTilemap, parse_scene_tilemaps};
use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::SpriteDrawMode;
use unitas_core::EntityId;
use unitas_render::image::Image;

use super::Runtime;
use super::runtime_render::RenderSprite;

const TILE_CELL_SIZE: [f32; 2] = [1.0, 1.0];

impl Runtime {
    pub(crate) fn populate_scene_tilemaps(
        &mut self,
        scene_path: &Path,
        entity_base: u64,
    ) -> Vec<(EntityId, String)> {
        let mut diagnostics = Vec::new();
        let Ok(text) = std::fs::read_to_string(scene_path) else {
            return diagnostics;
        };
        self.register_sprite_asset_root(scene_path);
        let index = assets_index(scene_path);
        let Ok(maps) = parse_scene_tilemaps(&text, index.as_ref()) else {
            return diagnostics;
        };
        for map in &maps {
            diagnostics.extend(self.populate_tilemap_sprites(map, entity_base));
        }
        diagnostics
    }

    fn populate_tilemap_sprites(
        &mut self,
        map: &SceneTilemap,
        entity_base: u64,
    ) -> Vec<(EntityId, String)> {
        let mut diagnostics = Vec::new();
        if map.cells.is_empty() {
            return diagnostics;
        }
        let entity = EntityId::new(entity_base + map.ordinal);
        for cell in &map.cells {
            let Some(tile) = map.tiles.get(cell.tile_index as usize) else {
                diagnostics.push((
                    entity,
                    format!(
                        "Tilemap on entity {} references tile index {} that could not be resolved",
                        entity.as_u64(),
                        cell.tile_index
                    ),
                ));
                continue;
            };
            let guid = tile.sprite.as_ref().map(|sprite| sprite.guid.clone());
            let texture = guid
                .as_deref()
                .and_then(|guid| self.tile_sprite_texture(guid));
            if texture.is_none() {
                diagnostics.push((
                    entity,
                    format!(
                        "Tilemap on entity {} references tile sprite guid {} that could not be imported",
                        entity.as_u64(),
                        guid.as_deref().unwrap_or("none")
                    ),
                ));
                continue;
            }
            let offset = [
                (cell.position[0] as f32 + map.tile_anchor[0]) * TILE_CELL_SIZE[0],
                (cell.position[1] as f32 + map.tile_anchor[1]) * TILE_CELL_SIZE[1],
            ];
            self.render.sprites.push(RenderSprite {
                entity,
                texture,
                color: tile.color,
                sorting_layer: map.sorting_layer,
                sorting_order: map.sorting_order,
                size: TILE_CELL_SIZE,
                flip_x: false,
                flip_y: false,
                enabled: true,
                draw_mode: SpriteDrawMode::Simple,
                border: [0.0; 4],
                local_offset: offset,
                world_size: Some(TILE_CELL_SIZE),
                slices: None,
            });
        }
        diagnostics
    }

    fn tile_sprite_texture(&self, guid: &str) -> Option<Arc<Image>> {
        let path = self
            .render
            .sprite_asset_paths
            .get(&guid.to_ascii_lowercase())?;
        self.load_cached_sprite_texture(Some(path))
    }
}

fn assets_index(scene_path: &Path) -> Option<GuidIndex> {
    for ancestor in scene_path.parent()?.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("Assets") {
            return GuidIndex::build(ancestor).ok();
        }
    }
    None
}
