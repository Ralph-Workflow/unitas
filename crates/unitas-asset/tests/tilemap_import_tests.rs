use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use unitas_asset::database::identify_asset_family;
use unitas_asset::database::tilemap::{
    import_tilemap, import_tilemap_file, import_tilemap_with_guids, CellLayout, CellSwizzle,
    TileColliderType, TilemapError, TilemapKind,
};
use unitas_asset::database::RecognizedAssetFamily;
use unitas_asset::dependencies::AssetImportState;
use unitas_asset::meta::GuidIndex;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const TILE_FIXTURE: &str = "tests/fixtures/tilemap/tilemap_tile.asset";
const TILE_PIVOT_FIXTURE: &str = "tests/fixtures/tilemap/tilemap_tile_pivot.asset";
const RULE_TILE_FIXTURE: &str = "tests/fixtures/tilemap/tilemap_rule_tile.asset";
const TILEMAP_FIXTURE: &str = "tests/fixtures/tilemap/tilemap_tilemap.asset";
const GRID_FIXTURE: &str = "tests/fixtures/tilemap/tilemap_grid.asset";

const GRASS_TILE_GUID: &str = "11111111111111111111111111111111";
const GRASS_TILE_SPRITE_GUID: &str = "66666666666666666666666666666666";
const STONE_TILE_SPRITE_GUID: &str = "77777777777777777777777777777777";
const RULE_TILE_GUID: &str = "33333333333333333333333333333333";
const TILEMAP_GUID: &str = "44444444444444444444444444444444";

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn fixture_bytes(relative: &str) -> TestResult<Vec<u8>> {
    Ok(fs::read(fixture_path(relative))?)
}

#[test]
fn imports_a_builtin_tile_with_its_sprite_and_collider_type() -> TestResult {
    let bytes = fixture_bytes(TILE_FIXTURE)?;
    let artifact = import_tilemap(&bytes, "tilemap_tile.asset")?;
    assert_eq!(artifact.identity.kind, TilemapKind::Tile);
    assert_eq!(artifact.identity.name, "grass_tile");
    let tile = artifact
        .tiles
        .first()
        .ok_or("the tile asset must carry one tile")?;
    assert_eq!(tile.name, "grass_tile");
    assert_eq!(tile.reference.guid, "");
    assert_eq!(tile.reference.file_id, 11400000);
    let sprite = tile.sprite.as_ref().ok_or("the tile must carry a sprite")?;
    assert_eq!(sprite.guid, GRASS_TILE_SPRITE_GUID);
    assert_eq!(sprite.file_id, 21300000);
    assert_eq!(tile.collider_type, TileColliderType::Sprite);
    assert_eq!(tile.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(tile.pivot, [0.5, 0.5]);
    assert!(tile.rules.is_empty());
    assert!(artifact.tilemap.is_none());
    assert!(artifact.grid.is_none());
    Ok(())
}

#[test]
fn imports_a_custom_tile_with_its_pivot_and_grid_collider_type() -> TestResult {
    let bytes = fixture_bytes(TILE_PIVOT_FIXTURE)?;
    let artifact = import_tilemap(&bytes, "tilemap_tile_pivot.asset")?;
    assert_eq!(artifact.identity.kind, TilemapKind::Tile);
    let tile = artifact
        .tiles
        .first()
        .ok_or("the tile asset must carry one tile")?;
    assert_eq!(tile.name, "stone_tile");
    assert_eq!(tile.pivot, [0.25, 0.75]);
    assert_eq!(tile.collider_type, TileColliderType::Grid);
    assert_eq!(tile.color, [0.8, 0.9, 1.0, 1.0]);
    let sprite = tile.sprite.as_ref().ok_or("the tile must carry a sprite")?;
    assert_eq!(sprite.guid, STONE_TILE_SPRITE_GUID);
    Ok(())
}

#[test]
fn maps_unity_collider_values_onto_the_physics_tile_vocabulary() -> TestResult {
    assert_eq!(
        TileColliderType::from_unity_value(0),
        Some(TileColliderType::None)
    );
    assert_eq!(
        TileColliderType::from_unity_value(1),
        Some(TileColliderType::Sprite)
    );
    assert_eq!(
        TileColliderType::from_unity_value(2),
        Some(TileColliderType::Grid)
    );
    assert_eq!(TileColliderType::from_unity_value(7), None);
    Ok(())
}

#[test]
fn imports_a_rule_tile_with_its_rules() -> TestResult {
    let bytes = fixture_bytes(RULE_TILE_FIXTURE)?;
    let artifact = import_tilemap(&bytes, "tilemap_rule_tile.asset")?;
    assert_eq!(artifact.identity.kind, TilemapKind::RuleTile);
    let tile = artifact
        .tiles
        .first()
        .ok_or("the rule tile must carry one tile")?;
    assert_eq!(tile.kind, TilemapKind::RuleTile);
    assert_eq!(tile.name, "grass_rule_tile");
    assert_eq!(tile.rules.len(), 2);
    let first = tile.rules.first().ok_or("the rule tile must carry rules")?;
    let sprite = first.sprite.as_ref().ok_or("a rule must carry a sprite")?;
    assert_eq!(sprite.guid, "99999999999999999999999999999999");
    assert_eq!(first.probability, 1.0);
    assert_eq!(first.transform, [1.0, 0.0, 0.0, 0.0]);
    assert!(first.override_tile.is_none());
    let second = tile
        .rules
        .get(1)
        .ok_or("the rule tile must carry two rules")?;
    assert_eq!(second.probability, 0.5);
    assert_eq!(second.transform, [0.0, 0.0, 1.0, 0.0]);
    let override_tile = second
        .override_tile
        .as_ref()
        .ok_or("the rule must override a tile")?;
    assert_eq!(override_tile.guid, GRASS_TILE_GUID);
    Ok(())
}

#[test]
fn imports_a_scriptable_rule_tile_with_its_neighbor_sprites() -> TestResult {
    let document = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!11400000 &11400000\nMonoBehaviour:\n  m_Name: scriptable_rule_tile\n  m_ScriptableRules:\n  - m_RuleID: rule_0\n    m_NeighborIDs: []\n    m_Sprites: []\n  - m_RuleID: rule_1\n    m_NeighborIDs:\n    - up\n    m_Sprites:\n    - {fileID: 21300000, guid: 66666666666666666666666666666666, type: 3}\n    - {fileID: 21300000, guid: 77777777777777777777777777777777, type: 3}\n";
    let artifact = import_tilemap(document, "scriptable_rule_tile.asset")?;
    assert_eq!(artifact.identity.kind, TilemapKind::RuleTile);
    let tile = artifact
        .tiles
        .first()
        .ok_or("the rule tile must carry one tile")?;
    assert_eq!(tile.rules.len(), 2);
    let rule = tile
        .rules
        .get(1)
        .ok_or("the rule tile must carry two rules")?;
    let sprite = rule.sprite.as_ref().ok_or("the rule must carry a sprite")?;
    assert_eq!(sprite.guid, GRASS_TILE_SPRITE_GUID);
    let neighbor = rule
        .neighbors
        .first()
        .ok_or("the rule must carry a neighbor")?;
    assert_eq!(neighbor.direction, "up");
    let neighbor_sprite = neighbor
        .sprite
        .as_ref()
        .ok_or("the neighbor must carry a sprite")?;
    assert_eq!(neighbor_sprite.guid, STONE_TILE_SPRITE_GUID);
    Ok(())
}

#[test]
fn imports_a_tilemap_with_its_anchor_cells_and_referenced_tiles() -> TestResult {
    let artifact = import_tilemap_file(&fixture_path(TILEMAP_FIXTURE))?;
    assert_eq!(artifact.identity.kind, TilemapKind::Tilemap);
    assert_eq!(artifact.identity.guid.as_deref(), Some(TILEMAP_GUID));
    let tilemap = artifact
        .tilemap
        .as_ref()
        .ok_or("the tilemap must carry its settings")?;
    assert_eq!(tilemap.tile_anchor, [0.5, 0.25]);
    assert_eq!(tilemap.tile_anchor_position, [0.5, 0.5]);
    assert_eq!(tilemap.tile_assets.len(), 2);
    assert_eq!(tilemap.tile_assets[0].guid, GRASS_TILE_GUID);
    assert_eq!(tilemap.tile_assets[1].guid, RULE_TILE_GUID);
    assert_eq!(tilemap.cells.len(), 2);
    assert_eq!(tilemap.cells[0].position, [0, 0, 0]);
    assert_eq!(tilemap.cells[0].tile_index, 0);
    assert_eq!(tilemap.cells[1].position, [1, 0, 0]);
    assert_eq!(tilemap.cells[1].tile_index, 1);
    assert_eq!(artifact.tiles.len(), 2);
    let grass = artifact
        .tiles
        .first()
        .ok_or("the tilemap must resolve its tiles")?;
    assert_eq!(grass.name, "grass_tile");
    assert_eq!(grass.collider_type, TileColliderType::Sprite);
    let sprite = grass
        .sprite
        .as_ref()
        .ok_or("the resolved tile must carry a sprite")?;
    assert_eq!(sprite.guid, GRASS_TILE_SPRITE_GUID);
    let rule_tile = artifact
        .tiles
        .get(1)
        .ok_or("the tilemap must resolve its rule tile")?;
    assert_eq!(rule_tile.kind, TilemapKind::RuleTile);
    assert_eq!(rule_tile.rules.len(), 2);
    Ok(())
}

#[test]
fn resolves_tilemap_tiles_through_a_caller_supplied_guid_index() -> TestResult {
    let mut index = GuidIndex::default();
    index.insert(GRASS_TILE_GUID.to_string(), fixture_path(TILE_FIXTURE))?;
    index.insert(RULE_TILE_GUID.to_string(), fixture_path(RULE_TILE_FIXTURE))?;
    index.insert(TILEMAP_GUID.to_string(), fixture_path(TILEMAP_FIXTURE))?;
    let bytes = fixture_bytes(TILEMAP_FIXTURE)?;
    let path = fixture_path(TILEMAP_FIXTURE);
    let artifact = import_tilemap_with_guids(&bytes, &path.to_string_lossy(), &index)?;
    assert_eq!(artifact.identity.guid.as_deref(), Some(TILEMAP_GUID));
    assert_eq!(artifact.tiles.len(), 2);
    let grass = artifact
        .tiles
        .first()
        .ok_or("the index must resolve the referenced tiles")?;
    assert_eq!(grass.name, "grass_tile");
    assert_eq!(grass.reference.guid, GRASS_TILE_GUID);
    Ok(())
}

#[test]
fn imports_a_grid_with_its_cell_layout() -> TestResult {
    let bytes = fixture_bytes(GRID_FIXTURE)?;
    let artifact = import_tilemap(&bytes, "tilemap_grid.asset")?;
    assert_eq!(artifact.identity.kind, TilemapKind::Grid);
    let grid = artifact
        .grid
        .as_ref()
        .ok_or("the grid must carry its settings")?;
    assert_eq!(grid.cell_size, [1.0, 0.5]);
    assert_eq!(grid.cell_gap, [0.0, 0.0]);
    assert_eq!(grid.cell_layout, CellLayout::Isometric);
    assert_eq!(grid.cell_swizzle, CellSwizzle::None);
    assert!(artifact.tiles.is_empty());
    Ok(())
}

#[test]
fn reimports_a_tilemap_to_the_same_identity() -> TestResult {
    let bytes = fixture_bytes(TILEMAP_FIXTURE)?;
    let first = import_tilemap_file(&fixture_path(TILEMAP_FIXTURE))?;
    let second = import_tilemap_file(&fixture_path(TILEMAP_FIXTURE))?;
    assert_eq!(first.identity, second.identity);
    assert_eq!(first.identity.content_hash, second.identity.content_hash);
    assert_eq!(first, second);
    let mut changed = bytes.clone();
    changed.push(b'\n');
    let third = import_tilemap(&changed, "tilemap_tilemap.asset")?;
    assert_ne!(first.identity.content_hash, third.identity.content_hash);
    Ok(())
}

#[test]
fn rejects_a_document_that_carries_no_tilemap() -> TestResult {
    let material = b"%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  m_Name: LitMaterial\n";
    let error = import_tilemap(material, "Material.mat")
        .err()
        .ok_or("a material must not import as a tilemap")?;
    assert_eq!(error, TilemapError::NoTilemapDocument);
    Ok(())
}

#[test]
fn identifies_tilemap_assets_as_their_own_families() -> TestResult {
    let cases = [
        (TILE_FIXTURE, RecognizedAssetFamily::Tile),
        (TILE_PIVOT_FIXTURE, RecognizedAssetFamily::Tile),
        (RULE_TILE_FIXTURE, RecognizedAssetFamily::Tile),
        (TILEMAP_FIXTURE, RecognizedAssetFamily::Tilemap),
        (GRID_FIXTURE, RecognizedAssetFamily::Grid),
    ];
    for (fixture, family) in cases {
        let bytes = fixture_bytes(fixture)?;
        let identified = identify_asset_family(Path::new(fixture), &bytes, None)
            .ok_or("the tilemap asset must be identified")?;
        assert_eq!(identified.family, family);
        assert_eq!(identified.outcome.state, AssetImportState::Ready);
    }
    Ok(())
}
