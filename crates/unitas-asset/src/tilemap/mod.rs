use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dependencies::{ImportDiagnostic, ImportOutcome};
use crate::meta::GuidIndex;
use crate::unity_yaml::{self, Document, Node, Stream, YamlError};

const DEFAULT_TILE_PIVOT: [f32; 2] = [0.5, 0.5];
const DEFAULT_TILE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_CELL_SIZE: [f32; 2] = [1.0, 1.0];
const DEFAULT_CELL_GAP: [f32; 2] = [0.0, 0.0];
const DEFAULT_TILE_ANCHOR: [f32; 2] = [0.5, 0.5];
const IDENTITY_TRANSFORM: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TilemapKind {
    Tile,
    RuleTile,
    Tilemap,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileColliderType {
    None,
    Sprite,
    Grid,
}

impl TileColliderType {
    pub fn from_unity_value(value: i64) -> Option<Self> {
        match value {
            0 => Some(TileColliderType::None),
            1 => Some(TileColliderType::Sprite),
            2 => Some(TileColliderType::Grid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReference {
    pub guid: String,
    pub file_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleNeighbor {
    pub direction: String,
    pub sprite: Option<AssetReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TilingRule {
    pub sprite: Option<AssetReference>,
    pub probability: f32,
    pub transform: [f32; 4],
    pub override_tile: Option<AssetReference>,
    pub neighbors: Vec<RuleNeighbor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileAsset {
    pub kind: TilemapKind,
    pub name: String,
    pub reference: AssetReference,
    pub sprite: Option<AssetReference>,
    pub collider_type: TileColliderType,
    pub pivot: [f32; 2],
    pub color: [f32; 4],
    pub rules: Vec<TilingRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellLayout {
    Rectangle,
    Hexagon,
    Isometric,
    IsometricZAsY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSwizzle {
    None,
    YX,
    XY,
    YXY,
    ZXY,
    ZYX,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSettings {
    pub cell_size: [f32; 2],
    pub cell_gap: [f32; 2],
    pub cell_layout: CellLayout,
    pub cell_swizzle: CellSwizzle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileOrientation {
    Horizontal,
    Vertical,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TilemapCell {
    pub position: [i32; 3],
    pub tile_index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TilemapSettings {
    pub tile_anchor: [f32; 2],
    pub tile_anchor_position: [f32; 2],
    pub orientation: TileOrientation,
    pub tile_assets: Vec<AssetReference>,
    pub cells: Vec<TilemapCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilemapIdentity {
    pub kind: TilemapKind,
    pub name: String,
    pub guid: Option<String>,
    pub content_hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TilemapArtifact {
    pub identity: TilemapIdentity,
    pub grid: Option<GridSettings>,
    pub tilemap: Option<TilemapSettings>,
    pub tiles: Vec<TileAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TilemapError {
    Yaml { error: YamlError },
    Text { message: String },
    Io { path: PathBuf, message: String },
    NoTilemapDocument,
}

impl fmt::Display for TilemapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TilemapError::Yaml { error } => write!(formatter, "{error}"),
            TilemapError::Text { message } => {
                write!(formatter, "the tilemap asset is not valid UTF-8: {message}")
            }
            TilemapError::Io { path, message } => {
                write!(formatter, "cannot read {}: {}", path.display(), message)
            }
            TilemapError::NoTilemapDocument => {
                write!(formatter, "stream contains no tilemap document")
            }
        }
    }
}

impl std::error::Error for TilemapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TilemapError::Yaml { error } => Some(error),
            _ => None,
        }
    }
}

impl From<YamlError> for TilemapError {
    fn from(error: YamlError) -> Self {
        TilemapError::Yaml { error }
    }
}

pub fn import_tilemap(bytes: &[u8], file_name: &str) -> Result<TilemapArtifact, TilemapError> {
    let stream = parse_stream(bytes)?;
    build_artifact(&stream, bytes, file_name, None, None)
}

pub fn import_tilemap_with_guids(
    bytes: &[u8],
    file_name: &str,
    index: &GuidIndex,
) -> Result<TilemapArtifact, TilemapError> {
    let stream = parse_stream(bytes)?;
    let own_guid = guid_for_path(index, Path::new(file_name));
    build_artifact(&stream, bytes, file_name, Some(index), own_guid.as_deref())
}

pub fn import_tilemap_file(path: &Path) -> Result<TilemapArtifact, TilemapError> {
    let bytes = fs::read(path).map_err(|error| TilemapError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let index = path
        .parent()
        .and_then(|parent| GuidIndex::build(parent).ok());
    let own_guid = index.as_ref().and_then(|index| guid_for_path(index, path));
    let stream = parse_stream(&bytes)?;
    build_artifact(
        &stream,
        &bytes,
        &asset_name(&path.to_string_lossy()),
        index.as_ref(),
        own_guid.as_deref(),
    )
}

pub fn tilemap_import_outcome(
    path: &str,
    result: Result<&TilemapArtifact, &TilemapError>,
) -> ImportOutcome {
    match result {
        Ok(_) => ImportOutcome::ready(),
        Err(error) => ImportOutcome::failed(ImportDiagnostic::new(
            path,
            error.to_string(),
            "the tilemap cannot be imported; the asset stays failed and no tile data is available",
        )),
    }
}

pub fn classify_body(body: &Node) -> Option<TilemapKind> {
    if body.get("m_TileAssetArray").is_some() {
        return Some(TilemapKind::Tilemap);
    }
    if body.get("m_CellSize").is_some() {
        return Some(TilemapKind::Grid);
    }
    if body.get("m_TilingRules").is_some() || body.get("m_ScriptableRules").is_some() {
        return Some(TilemapKind::RuleTile);
    }
    if body.get("m_ColliderType").is_some() && body.get("m_Sprite").is_some() {
        return Some(TilemapKind::Tile);
    }
    None
}

fn parse_stream(bytes: &[u8]) -> Result<Stream, TilemapError> {
    let text = std::str::from_utf8(bytes).map_err(|error| TilemapError::Text {
        message: error.to_string(),
    })?;
    Ok(unity_yaml::parse(text)?)
}

fn build_artifact(
    stream: &Stream,
    bytes: &[u8],
    file_name: &str,
    index: Option<&GuidIndex>,
    own_guid: Option<&str>,
) -> Result<TilemapArtifact, TilemapError> {
    let (document, kind) = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| classify_document(document).map(|kind| (document, kind)))
        .ok_or(TilemapError::NoTilemapDocument)?;
    let body = root_body(document)
        .map(|(_, body)| body)
        .ok_or(TilemapError::NoTilemapDocument)?;
    let name = node_string(body, "m_Name").unwrap_or_else(|| asset_name(file_name));
    let mut grid = None;
    let mut tilemap = None;
    let mut tiles = Vec::new();
    match kind {
        TilemapKind::Tile | TilemapKind::RuleTile => {
            let reference = AssetReference {
                guid: own_guid.unwrap_or_default().to_string(),
                file_id: document.file_id,
            };
            tiles.push(parse_tile(body, kind, reference, name.clone()));
        }
        TilemapKind::Grid => {
            grid = Some(parse_grid(body));
        }
        TilemapKind::Tilemap => {
            let settings = parse_tilemap(body);
            tiles = resolve_tiles(&settings.tile_assets, index);
            tilemap = Some(settings);
        }
    }
    Ok(TilemapArtifact {
        identity: TilemapIdentity {
            kind,
            name,
            guid: own_guid.map(str::to_string),
            content_hash: hash_bytes(bytes),
        },
        grid,
        tilemap,
        tiles,
    })
}

fn classify_document(document: &Document) -> Option<TilemapKind> {
    let (class_name, body) = root_body(document)?;
    match class_name {
        "Tile" | "TileBase" => Some(TilemapKind::Tile),
        "RuleTile" | "ScriptableRuleTile" | "IsometricRuleTile" => Some(TilemapKind::RuleTile),
        "Tilemap" => Some(TilemapKind::Tilemap),
        "Grid" => Some(TilemapKind::Grid),
        "MonoBehaviour" => classify_body(body),
        _ => None,
    }
}

fn root_body(document: &Document) -> Option<(&str, &Node)> {
    let entries = document.root.as_mapping()?;
    let (class_name, body) = entries.first()?;
    Some((class_name.as_str(), body))
}

fn parse_tile(
    body: &Node,
    kind: TilemapKind,
    reference: AssetReference,
    name: String,
) -> TileAsset {
    TileAsset {
        kind,
        name,
        reference,
        sprite: node_reference(body, "m_Sprite"),
        collider_type: node_int(body, "m_ColliderType")
            .and_then(TileColliderType::from_unity_value)
            .unwrap_or(TileColliderType::None),
        pivot: node_vector2(body, "m_Pivot").unwrap_or(DEFAULT_TILE_PIVOT),
        color: node_color(body, "m_Color").unwrap_or(DEFAULT_TILE_COLOR),
        rules: parse_rules(body),
    }
}

fn parse_rules(body: &Node) -> Vec<TilingRule> {
    if let Some(entries) = body.get("m_TilingRules").and_then(Node::as_sequence) {
        return entries.iter().map(parse_tiling_rule).collect();
    }
    if let Some(entries) = body.get("m_ScriptableRules").and_then(Node::as_sequence) {
        return entries.iter().map(parse_scriptable_rule).collect();
    }
    Vec::new()
}

fn parse_tiling_rule(node: &Node) -> TilingRule {
    TilingRule {
        sprite: node_reference(node, "m_Sprite"),
        probability: node_float(node, "m_Probability").unwrap_or(1.0),
        transform: node_quaternion(node, "m_Transform").unwrap_or(IDENTITY_TRANSFORM),
        override_tile: node_reference(node, "m_OverrideTile"),
        neighbors: Vec::new(),
    }
}

fn parse_scriptable_rule(node: &Node) -> TilingRule {
    let sprites: Vec<Option<AssetReference>> = node
        .get("m_Sprites")
        .and_then(Node::as_sequence)
        .map(|entries| entries.iter().map(parse_reference).collect())
        .unwrap_or_default();
    let directions: Vec<String> = node
        .get("m_NeighborIDs")
        .and_then(Node::as_sequence)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_scalar().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let neighbors = directions
        .iter()
        .enumerate()
        .map(|(index, direction)| RuleNeighbor {
            direction: direction.clone(),
            sprite: sprites.get(index + 1).cloned().flatten(),
        })
        .collect();
    TilingRule {
        sprite: sprites.first().cloned().flatten(),
        probability: node_float(node, "m_Probability").unwrap_or(1.0),
        transform: node_quaternion(node, "m_Transform").unwrap_or(IDENTITY_TRANSFORM),
        override_tile: node_reference(node, "m_OverrideTile"),
        neighbors,
    }
}

fn parse_tilemap(body: &Node) -> TilemapSettings {
    let tile_assets = body
        .get("m_TileAssetArray")
        .and_then(Node::as_sequence)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.get("m_TileAsset").and_then(parse_reference))
                .collect()
        })
        .unwrap_or_default();
    let cells = body
        .get("m_Tiles")
        .and_then(Node::as_sequence)
        .map(|entries| entries.iter().map(parse_cell).collect())
        .unwrap_or_default();
    TilemapSettings {
        tile_anchor: node_vector2(body, "m_TileAnchor").unwrap_or(DEFAULT_TILE_ANCHOR),
        tile_anchor_position: node_vector2(body, "m_TileAnchorPosition")
            .unwrap_or(DEFAULT_TILE_ANCHOR),
        orientation: parse_orientation(node_int(body, "m_TileOrientation")),
        tile_assets,
        cells,
    }
}

fn parse_cell(node: &Node) -> TilemapCell {
    TilemapCell {
        position: [
            node_int(node, "m_X").unwrap_or(0) as i32,
            node_int(node, "m_Y").unwrap_or(0) as i32,
            node_int(node, "m_Z").unwrap_or(0) as i32,
        ],
        tile_index: node_int(node, "m_TileIndex").unwrap_or(0) as i32,
    }
}

fn parse_orientation(value: Option<i64>) -> TileOrientation {
    match value {
        Some(1) => TileOrientation::Vertical,
        Some(2) => TileOrientation::Custom,
        _ => TileOrientation::Horizontal,
    }
}

fn parse_grid(body: &Node) -> GridSettings {
    GridSettings {
        cell_size: node_vector2(body, "m_CellSize").unwrap_or(DEFAULT_CELL_SIZE),
        cell_gap: node_vector2(body, "m_CellGap").unwrap_or(DEFAULT_CELL_GAP),
        cell_layout: parse_cell_layout(node_int(body, "m_CellLayout")),
        cell_swizzle: parse_cell_swizzle(node_int(body, "m_CellSwizzle")),
    }
}

fn parse_cell_layout(value: Option<i64>) -> CellLayout {
    match value {
        Some(1) => CellLayout::Hexagon,
        Some(2) => CellLayout::Isometric,
        Some(3) => CellLayout::IsometricZAsY,
        _ => CellLayout::Rectangle,
    }
}

fn parse_cell_swizzle(value: Option<i64>) -> CellSwizzle {
    match value {
        Some(1) => CellSwizzle::YX,
        Some(2) => CellSwizzle::XY,
        Some(3) => CellSwizzle::YXY,
        Some(4) => CellSwizzle::ZXY,
        Some(5) => CellSwizzle::ZYX,
        _ => CellSwizzle::None,
    }
}

fn resolve_tiles(references: &[AssetReference], index: Option<&GuidIndex>) -> Vec<TileAsset> {
    match index {
        Some(index) => references
            .iter()
            .filter_map(|reference| resolve_tile(reference, index))
            .collect(),
        None => Vec::new(),
    }
}

fn resolve_tile(reference: &AssetReference, index: &GuidIndex) -> Option<TileAsset> {
    let path = index.get(&reference.guid)?;
    let bytes = fs::read(path).ok()?;
    let stream = parse_stream(&bytes).ok()?;
    let (document, kind) = stream
        .documents
        .iter()
        .filter(|document| !document.stripped)
        .find_map(|document| classify_document(document).map(|kind| (document, kind)))?;
    if matches!(kind, TilemapKind::Tilemap | TilemapKind::Grid) {
        return None;
    }
    let body = root_body(document).map(|(_, body)| body)?;
    let name = node_string(body, "m_Name").unwrap_or_else(|| asset_name(&path.to_string_lossy()));
    Some(parse_tile(body, kind, reference.clone(), name))
}

fn guid_for_path(index: &GuidIndex, path: &Path) -> Option<String> {
    index
        .iter()
        .find(|(_, candidate)| *candidate == path)
        .map(|(guid, _)| guid.to_string())
}

fn node_string(node: &Node, key: &str) -> Option<String> {
    node.get(key).and_then(Node::as_scalar).map(str::to_string)
}

fn node_int(node: &Node, key: &str) -> Option<i64> {
    node.get(key).and_then(Node::as_i64)
}

fn node_float(node: &Node, key: &str) -> Option<f32> {
    node.get(key)
        .and_then(Node::as_f64)
        .map(|value| value as f32)
}

fn node_vector2(node: &Node, key: &str) -> Option<[f32; 2]> {
    let value = node.get(key)?;
    Some([
        value.get("x").and_then(Node::as_f64)? as f32,
        value.get("y").and_then(Node::as_f64)? as f32,
    ])
}

fn node_color(node: &Node, key: &str) -> Option<[f32; 4]> {
    let value = node.get(key)?;
    Some([
        value.get("r").and_then(Node::as_f64)? as f32,
        value.get("g").and_then(Node::as_f64)? as f32,
        value.get("b").and_then(Node::as_f64)? as f32,
        value.get("a").and_then(Node::as_f64)? as f32,
    ])
}

fn node_quaternion(node: &Node, key: &str) -> Option<[f32; 4]> {
    let value = node.get(key)?;
    Some([
        value.get("x").and_then(Node::as_f64)? as f32,
        value.get("y").and_then(Node::as_f64)? as f32,
        value.get("z").and_then(Node::as_f64)? as f32,
        value.get("w").and_then(Node::as_f64)? as f32,
    ])
}

fn node_reference(node: &Node, key: &str) -> Option<AssetReference> {
    node.get(key).and_then(parse_reference)
}

fn parse_reference(node: &Node) -> Option<AssetReference> {
    let guid = node
        .get("guid")
        .and_then(Node::as_scalar)
        .filter(|guid| !guid.is_empty())?;
    Some(AssetReference {
        guid: guid.to_string(),
        file_id: node.get("fileID").and_then(Node::as_i64).unwrap_or(0),
    })
}

fn asset_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_string()
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
