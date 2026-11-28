use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_asset::database::tilemap::scene::parse_scene_tilemaps;
use unitas_asset::meta::GuidIndex;

const TILE_ASSET_GUID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const TILE_SPRITE_GUID: &str = "cccccccccccccccccccccccccccccccc";

const SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Main Camera
--- !u!1 &3000
GameObject:
  m_ObjectHideFlags: 0
  m_Name: Ground
--- !u!4 &3001
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_LocalPosition: {x: 0, y: -0.5, z: 0}
--- !u!1839735485 &3005
Tilemap:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 3000}
  m_TileAssetArray:
  - m_RefCount: 2
    m_TileAsset: {fileID: 11400000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 2}
  m_TileAnchor: {x: 0.5, y: 0.5}
  m_SortingLayer: 2
  m_SortingOrder: -1
  m_Tiles:
  - m_X: 0
    m_Y: -1
    m_Z: 0
    m_TileIndex: 0
  - m_X: 1
    m_Y: -1
    m_Z: 0
    m_TileIndex: 0
";

const TILE_ASSET: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!114 &11400000
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 0}
  m_Script: {fileID: 11500000, guid: dddddddddddddddddddddddddddddddd, type: 3}
  m_Name: ground_tile
  m_Sprite: {fileID: 21300000, guid: cccccccccccccccccccccccccccccccc, type: 3}
  m_Color: {r: 1, g: 1, b: 1, a: 1}
  m_ColliderType: 2
";

fn fixture_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_scene_tilemap_{}_{}",
        std::process::id(),
        name
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_tile_asset(dir: &Path) -> Result<(), Box<dyn Error>> {
    fs::write(dir.join("ground_tile.asset"), TILE_ASSET)?;
    fs::write(
        dir.join("ground_tile.asset.meta"),
        format!("fileFormatVersion: 2\nguid: {TILE_ASSET_GUID}\n"),
    )?;
    Ok(())
}

#[test]
fn parses_scene_tilemaps_with_cells_sorting_and_entity_ordinals() -> Result<(), Box<dyn Error>> {
    let dir = fixture_dir("resolve")?;
    write_tile_asset(&dir)?;
    let index = GuidIndex::build(&dir)?;
    let maps = parse_scene_tilemaps(SCENE, Some(&index))?;
    assert_eq!(maps.len(), 1);
    let map = &maps[0];
    assert_eq!(map.game_object_file_id, 3000);
    assert_eq!(map.ordinal, 1);
    assert_eq!(map.sorting_layer, 2);
    assert_eq!(map.sorting_order, -1);
    assert_eq!(map.tile_anchor, [0.5, 0.5]);
    assert_eq!(map.cells.len(), 2);
    assert_eq!(map.cells[0].position, [0, -1, 0]);
    assert_eq!(map.cells[1].position, [1, -1, 0]);
    assert_eq!(map.tiles.len(), 1);
    let sprite = map.tiles[0]
        .sprite
        .as_ref()
        .ok_or("the resolved tile must carry a sprite")?;
    assert_eq!(sprite.guid, TILE_SPRITE_GUID);
    assert_eq!(sprite.file_id, 21300000);
    Ok(())
}

#[test]
fn parses_scene_tilemaps_without_a_guid_index_as_unresolved() -> Result<(), Box<dyn Error>> {
    let maps = parse_scene_tilemaps(SCENE, None)?;
    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].cells.len(), 2);
    assert!(maps[0].tiles.is_empty());
    Ok(())
}
