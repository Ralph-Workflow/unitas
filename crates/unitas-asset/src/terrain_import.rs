use std::fmt;
use std::fs;
use std::path::Path;

use crate::unity_yaml::{self, Document, Node, Stream, YamlError};

pub const TERRAIN_DATA_CLASS_ID: u32 = 156;
pub const DEFAULT_TERRAIN_RESOLUTION: usize = 33;
pub const DEFAULT_TERRAIN_SIZE: [f64; 3] = [1000.0, 600.0, 1000.0];
const UNITY_HEIGHTMAP_MAX_I16: f64 = 32766.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTerrainData {
    pub name: String,
    pub heightmap_resolution: usize,
    pub size: [f64; 3],
    pub heights: Vec<f64>,
    pub diagnostics: Vec<String>,
}

impl Default for ImportedTerrainData {
    fn default() -> Self {
        let cells = DEFAULT_TERRAIN_RESOLUTION * DEFAULT_TERRAIN_RESOLUTION;
        Self {
            name: String::from("TerrainData"),
            heightmap_resolution: DEFAULT_TERRAIN_RESOLUTION,
            size: DEFAULT_TERRAIN_SIZE,
            heights: vec![0.0; cells],
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainImportError {
    InvalidUtf8,
    Yaml(YamlError),
    Read { asset: String, message: String },
    MissingTerrainDataDocument { asset: String },
    InvalidHeightmap { asset: String, reason: String },
}

impl fmt::Display for TerrainImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => write!(formatter, "TerrainData asset is not valid UTF-8"),
            Self::Yaml(error) => write!(formatter, "invalid TerrainData YAML: {error}"),
            Self::Read { asset, message } => {
                write!(formatter, "cannot read TerrainData '{asset}': {message}")
            }
            Self::MissingTerrainDataDocument { asset } => {
                write!(
                    formatter,
                    "TerrainData '{asset}' does not contain a class 156 TerrainData document"
                )
            }
            Self::InvalidHeightmap { asset, reason } => {
                write!(
                    formatter,
                    "TerrainData '{asset}' heightmap is invalid: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for TerrainImportError {}

impl From<YamlError> for TerrainImportError {
    fn from(error: YamlError) -> Self {
        Self::Yaml(error)
    }
}

pub fn import_terrain_data_file(path: &Path) -> Result<ImportedTerrainData, TerrainImportError> {
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("TerrainData"))
        .to_string();
    let bytes = fs::read(path).map_err(|error| TerrainImportError::Read {
        asset: label.clone(),
        message: error.to_string(),
    })?;
    import_terrain_data_bytes(&bytes, &label)
}

pub fn import_terrain_data_bytes(
    bytes: &[u8],
    asset_label: &str,
) -> Result<ImportedTerrainData, TerrainImportError> {
    let without_bom = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(without_bom).map_err(|_| TerrainImportError::InvalidUtf8)?;
    import_terrain_data_str(text, asset_label)
}

pub fn import_terrain_data_str(
    text: &str,
    asset_label: &str,
) -> Result<ImportedTerrainData, TerrainImportError> {
    let stream = unity_yaml::parse(text)?;
    import_terrain_data_stream(&stream, asset_label)
}

pub fn import_terrain_data_stream(
    stream: &Stream,
    asset_label: &str,
) -> Result<ImportedTerrainData, TerrainImportError> {
    let doc = find_terrain_data_doc(stream).ok_or_else(|| {
        TerrainImportError::MissingTerrainDataDocument {
            asset: asset_label.to_string(),
        }
    })?;
    let body = doc.root.get("TerrainData").unwrap_or(&doc.root);
    if body.as_mapping().is_none() {
        return Err(TerrainImportError::MissingTerrainDataDocument {
            asset: asset_label.to_string(),
        });
    }
    let name = parse_name(body, asset_label);
    let heightmap = body.get("m_Heightmap").unwrap_or(body);
    let raw_samples = parse_heights_node(heightmap, asset_label)?;
    let resolution = resolve_resolution(heightmap, raw_samples.len(), asset_label)?;
    let size = resolve_world_size(body, heightmap, resolution);
    let heights = resize_samples(raw_samples, resolution * resolution);
    let diagnostics = collect_sub_database_diagnostics(body);
    Ok(ImportedTerrainData {
        name,
        heightmap_resolution: resolution,
        size,
        heights,
        diagnostics,
    })
}

fn find_terrain_data_doc(stream: &Stream) -> Option<&Document> {
    stream.documents.iter().find(|doc| {
        !doc.stripped
            && (doc.class_id == TERRAIN_DATA_CLASS_ID || doc.root.get("TerrainData").is_some())
    })
}

fn parse_name(body: &Node, asset_label: &str) -> String {
    let fallback = Path::new(asset_label)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(asset_label)
        .to_string();
    body.get("m_Name")
        .and_then(Node::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or(fallback)
}

fn resolve_resolution(
    heightmap: &Node,
    sample_count: usize,
    asset_label: &str,
) -> Result<usize, TerrainImportError> {
    let explicit = heightmap
        .get("m_Resolution")
        .or_else(|| heightmap.get("m_Width"))
        .or_else(|| heightmap.get("m_Height"))
        .and_then(Node::as_i64);
    if let Some(value) = explicit {
        if value <= 0 {
            return Err(TerrainImportError::InvalidHeightmap {
                asset: asset_label.to_string(),
                reason: format!("heightmap resolution {value} must be positive"),
            });
        }
        return usize::try_from(value).map_err(|_| TerrainImportError::InvalidHeightmap {
            asset: asset_label.to_string(),
            reason: format!("heightmap resolution {value} out of range"),
        });
    }
    if sample_count > 0 {
        let root = (sample_count as f64).sqrt().round() as usize;
        if root > 0 && root * root == sample_count {
            return Ok(root);
        }
    }
    Ok(DEFAULT_TERRAIN_RESOLUTION)
}

fn resolve_world_size(body: &Node, heightmap: &Node, resolution: usize) -> [f64; 3] {
    if let Some(size) = heightmap
        .get("m_Size")
        .or_else(|| body.get("m_Size"))
        .and_then(parse_positive_vec3)
    {
        return size;
    }
    if let Some(scale) = heightmap
        .get("m_Scale")
        .or_else(|| body.get("m_Scale"))
        .and_then(parse_positive_vec3)
    {
        let span = resolution.saturating_sub(1).max(1) as f64;
        return [scale[0] * span, scale[1], scale[2] * span];
    }
    DEFAULT_TERRAIN_SIZE
}

fn parse_positive_vec3(node: &Node) -> Option<[f64; 3]> {
    let x = node.get("x").and_then(Node::as_f64)?;
    let y = node.get("y").and_then(Node::as_f64)?;
    let z = node.get("z").and_then(Node::as_f64)?;
    if x.is_finite() && x > 0.0 && y.is_finite() && y > 0.0 && z.is_finite() && z > 0.0 {
        Some([x, y, z])
    } else {
        None
    }
}

fn parse_heights_node(heightmap: &Node, asset_label: &str) -> Result<Vec<f64>, TerrainImportError> {
    let Some(node) = heightmap
        .get("m_Heights")
        .or_else(|| heightmap.get("_typelessdata"))
    else {
        return Ok(Vec::new());
    };
    if let Some(items) = node.as_sequence() {
        return parse_height_sequence(items, asset_label);
    }
    if let Some(typeless) = node.get("_typelessdata").and_then(Node::as_str) {
        return parse_height_hex(typeless.trim(), asset_label);
    }
    if let Some(scalar) = node.as_str() {
        let trimmed = scalar.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        return parse_height_hex(trimmed, asset_label);
    }
    Ok(Vec::new())
}

fn parse_height_sequence(
    items: &[Node],
    asset_label: &str,
) -> Result<Vec<f64>, TerrainImportError> {
    let mut raw = Vec::with_capacity(items.len());
    let mut any_exceeds_one = false;
    for (idx, item) in items.iter().enumerate() {
        let value = item.as_f64().filter(|v| v.is_finite()).ok_or_else(|| {
            TerrainImportError::InvalidHeightmap {
                asset: asset_label.to_string(),
                reason: format!("m_Heights[{idx}] is not a finite number"),
            }
        })?;
        if value > 1.0 {
            any_exceeds_one = true;
        }
        raw.push(value);
    }
    if any_exceeds_one {
        for value in &mut raw {
            *value = (*value / UNITY_HEIGHTMAP_MAX_I16).clamp(0.0, 1.0);
        }
    } else {
        for value in &mut raw {
            *value = value.clamp(0.0, 1.0);
        }
    }
    Ok(raw)
}

fn parse_height_hex(hex: &str, asset_label: &str) -> Result<Vec<f64>, TerrainImportError> {
    let clean: Vec<u8> = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if clean.is_empty() {
        return Ok(Vec::new());
    }
    if !clean.len().is_multiple_of(4) {
        return Err(TerrainImportError::InvalidHeightmap {
            asset: asset_label.to_string(),
            reason: format!(
                "hex heightmap length {} is not a multiple of 4 hex digits (16-bit samples)",
                clean.len()
            ),
        });
    }
    let mut samples = Vec::with_capacity(clean.len() / 4);
    for chunk in clean.chunks_exact(4) {
        let b0 = decode_hex_byte(chunk[0], chunk[1], asset_label)?;
        let b1 = decode_hex_byte(chunk[2], chunk[3], asset_label)?;
        let sample = u16::from_le_bytes([b0, b1]);
        let normalized = (f64::from(sample) / UNITY_HEIGHTMAP_MAX_I16).clamp(0.0, 1.0);
        samples.push(normalized);
    }
    Ok(samples)
}

fn decode_hex_byte(hi: u8, lo: u8, asset_label: &str) -> Result<u8, TerrainImportError> {
    let Some(h) = hex_nibble(hi) else {
        return Err(TerrainImportError::InvalidHeightmap {
            asset: asset_label.to_string(),
            reason: format!("invalid hex digit '{}'", char::from(hi)),
        });
    };
    let Some(l) = hex_nibble(lo) else {
        return Err(TerrainImportError::InvalidHeightmap {
            asset: asset_label.to_string(),
            reason: format!("invalid hex digit '{}'", char::from(lo)),
        });
    };
    Ok((h << 4) | l)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn resize_samples(mut samples: Vec<f64>, target_len: usize) -> Vec<f64> {
    samples.resize(target_len, 0.0);
    samples
}

fn collect_sub_database_diagnostics(body: &Node) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let splat_db = body.get("m_SplatDatabase").unwrap_or(body);
    let has_splats = has_non_empty_sequence(splat_db.get("m_Splats"))
        || has_non_empty_sequence(splat_db.get("m_TerrainLayers"))
        || has_non_empty_sequence(body.get("m_TerrainLayers"))
        || has_non_empty_sequence(splat_db.get("m_AlphaTextures"));
    if has_splats {
        diagnostics.push(String::from("TerrainData splat layers are not imported"));
    }
    let detail_db = body.get("m_DetailDatabase").unwrap_or(body);
    let has_trees = has_non_empty_sequence(detail_db.get("m_TreePrototypes"))
        || has_non_empty_sequence(detail_db.get("m_TreeInstances"))
        || has_non_empty_sequence(body.get("m_TreePrototypes"));
    if has_trees {
        diagnostics.push(String::from("TerrainData tree prototypes are not imported"));
    }
    let has_details = has_non_empty_sequence(detail_db.get("m_DetailPrototypes"))
        || has_non_empty_sequence(detail_db.get("m_Patches"))
        || has_non_empty_sequence(body.get("m_DetailPrototypes"));
    if has_details {
        diagnostics.push(String::from(
            "TerrainData detail prototypes are not imported",
        ));
    }
    diagnostics
}

fn has_non_empty_sequence(node: Option<&Node>) -> bool {
    node.and_then(Node::as_sequence)
        .is_some_and(|items| !items.is_empty())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{import_terrain_data_str, DEFAULT_TERRAIN_RESOLUTION, DEFAULT_TERRAIN_SIZE};

    #[test]
    fn imports_float_sequence_heightmap_and_scale() -> Result<(), Box<dyn Error>> {
        let yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!156 &15600000
TerrainData:
  m_ObjectHideFlags: 0
  m_Name: Hills
  m_Heightmap:
    m_Width: 3
    m_Height: 3
    m_Scale: {x: 50, y: 200, z: 50}
    m_Heights:
    - 0
    - 0.25
    - 0.5
    - 0.5
    - 0.75
    - 1.0
    - 0.1
    - 0.2
    - 0.3
";
        let imported = import_terrain_data_str(yaml, "Hills.asset")?;
        assert_eq!(imported.name, "Hills");
        assert_eq!(imported.heightmap_resolution, 3);
        assert_eq!(imported.size, [100.0, 200.0, 100.0]);
        assert_eq!(imported.heights.len(), 9);
        assert!((imported.heights[1] - 0.25).abs() < 1e-6);
        assert!((imported.heights[4] - 0.75).abs() < 1e-6);
        assert!((imported.heights[5] - 1.0).abs() < 1e-6);
        assert!(imported.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn imports_hex_encoded_16bit_heightmap_and_explicit_size() -> Result<(), Box<dyn Error>> {
        let half_sample = 16383u16.to_le_bytes();
        let full_sample = 32766u16.to_le_bytes();
        let hex = format!(
            "0000{:02x}{:02x}{:02x}{:02x}0000",
            half_sample[0], half_sample[1], full_sample[0], full_sample[1]
        );
        let yaml = format!(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!156 &15600000
TerrainData:
  m_Name: HexTerrain
  m_Heightmap:
    m_Resolution: 2
    m_Size: {{x: 400, y: 80, z: 400}}
    m_Heights: {hex}
"
        );
        let imported = import_terrain_data_str(&yaml, "HexTerrain.asset")?;
        assert_eq!(imported.heightmap_resolution, 2);
        assert_eq!(imported.size, [400.0, 80.0, 400.0]);
        assert_eq!(imported.heights.len(), 4);
        assert!((imported.heights[0] - 0.0).abs() < 1e-6);
        assert!((imported.heights[1] - 0.5).abs() < 1e-4);
        assert!((imported.heights[2] - 1.0).abs() < 1e-4);
        assert!((imported.heights[3] - 0.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn emits_diagnostics_for_splat_tree_and_detail_databases() -> Result<(), Box<dyn Error>> {
        let yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!156 &15600000
TerrainData:
  m_Name: RichTerrain
  m_SplatDatabase:
    m_TerrainLayers:
    - {fileID: 11400000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 2}
  m_DetailDatabase:
    m_TreePrototypes:
    - prefab: {fileID: 1000, guid: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, type: 3}
    m_DetailPrototypes:
    - prototype: {fileID: 2000, guid: cccccccccccccccccccccccccccccccc, type: 3}
";
        let imported = import_terrain_data_str(yaml, "RichTerrain.asset")?;
        assert_eq!(imported.heightmap_resolution, DEFAULT_TERRAIN_RESOLUTION);
        assert_eq!(imported.size, DEFAULT_TERRAIN_SIZE);
        assert_eq!(imported.diagnostics.len(), 3);
        assert!(imported
            .diagnostics
            .contains(&String::from("TerrainData splat layers are not imported")));
        assert!(imported.diagnostics.contains(&String::from(
            "TerrainData tree prototypes are not imported"
        )));
        assert!(imported.diagnostics.contains(&String::from(
            "TerrainData detail prototypes are not imported"
        )));
        Ok(())
    }
}
