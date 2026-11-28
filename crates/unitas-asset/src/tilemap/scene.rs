use std::collections::HashMap;

use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Node, Stream};

use super::{
    node_int, node_vector2, parse_tilemap, resolve_tiles, TileAsset, TilemapCell, TilemapError,
};

pub const TILEMAP_CLASS: u32 = 1839735485;
const GAME_OBJECT_CLASS: u32 = 1;
const DEFAULT_TILE_ANCHOR: [f32; 2] = [0.5, 0.5];

#[derive(Debug, Clone, PartialEq)]
pub struct SceneTilemap {
    pub game_object_file_id: i64,
    pub ordinal: u64,
    pub sorting_layer: i32,
    pub sorting_order: i32,
    pub tile_anchor: [f32; 2],
    pub tiles: Vec<TileAsset>,
    pub cells: Vec<TilemapCell>,
}

pub fn parse_scene_tilemaps(
    text: &str,
    index: Option<&GuidIndex>,
) -> Result<Vec<SceneTilemap>, TilemapError> {
    let stream = unity_yaml::parse(text)?;
    let ordinals = game_object_ordinals(&stream);
    let mut maps = Vec::new();
    for document in &stream.documents {
        if document.class_id != TILEMAP_CLASS || document.stripped {
            continue;
        }
        let Some(body) = document.root.get("Tilemap") else {
            continue;
        };
        let settings = parse_tilemap(body);
        let owner = game_object_file_id(body);
        maps.push(SceneTilemap {
            game_object_file_id: owner,
            ordinal: ordinals.get(&owner).copied().unwrap_or(0),
            sorting_layer: node_int(body, "m_SortingLayer").unwrap_or(0) as i32,
            sorting_order: node_int(body, "m_SortingOrder").unwrap_or(0) as i32,
            tile_anchor: node_vector2(body, "m_TileAnchor").unwrap_or(DEFAULT_TILE_ANCHOR),
            tiles: resolve_tiles(&settings.tile_assets, index),
            cells: settings.cells,
        });
    }
    Ok(maps)
}

fn game_object_file_id(body: &Node) -> i64 {
    body.get("m_GameObject")
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64)
        .unwrap_or(0)
}

fn game_object_ordinals(stream: &Stream) -> HashMap<i64, u64> {
    let mut ordinals = HashMap::new();
    let mut next = 0u64;
    for document in &stream.documents {
        if document.class_id == GAME_OBJECT_CLASS && !document.stripped {
            ordinals.insert(document.file_id, next);
            next += 1;
        }
    }
    ordinals
}
