use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use unitas_core::EntityId;

use crate::meta::GuidIndex;
use crate::scene::{AssetGuid, GameObjectRecord};
use crate::terrain_import::{import_terrain_data_file, ImportedTerrainData};
use crate::unity_yaml::{Node, Stream};

pub const TERRAIN_CLASS: u32 = 218;
pub const TERRAIN_COLLIDER_CLASS: u32 = 154;

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainBinding {
    pub entity: EntityId,
    pub enabled: bool,
    pub terrain_data_guid: Option<AssetGuid>,
    pub terrain_data_path: Option<PathBuf>,
    pub terrain_data: ImportedTerrainData,
    pub diagnostics: Vec<String>,
}

pub fn collect_scene_terrains(stream: &Stream, index: Option<&GuidIndex>) -> Vec<TerrainBinding> {
    let instance_roots = collect_prefab_instance_roots(stream);
    let mut game_objects = HashMap::new();
    let mut next_entity = 0u64;
    for document in &stream.documents {
        if document.class_id != 1 || document.stripped {
            continue;
        }
        if instance_roots.contains(&document.file_id) {
            continue;
        }
        let body = document.root.get("GameObject");
        let name = body
            .and_then(|node| node.get("m_Name"))
            .and_then(Node::as_str)
            .unwrap_or("GameObject")
            .to_string();
        let layer = body
            .and_then(|node| node.get("m_Layer"))
            .and_then(Node::as_i64)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);
        game_objects.insert(
            document.file_id,
            GameObjectRecord {
                entity: EntityId::new(next_entity),
                name,
                layer,
            },
        );
        next_entity += 1;
    }
    collect_terrains(stream, &game_objects, &instance_roots, index)
}

pub(crate) fn collect_terrains(
    stream: &Stream,
    game_objects: &HashMap<i64, GameObjectRecord>,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> Vec<TerrainBinding> {
    let mut terrains = Vec::new();
    for document in &stream.documents {
        if document.stripped || document.class_id != TERRAIN_CLASS {
            continue;
        }
        let Some(body) = document.root.get("Terrain") else {
            continue;
        };
        let Some(entity) = resolve_component_entity(body, game_objects, instance_roots) else {
            continue;
        };
        let enabled = parse_bool(body.get("m_Enabled"), true);
        let terrain_data_guid = extract_guid(body.get("m_TerrainData"));
        let (terrain_data_path, terrain_data, diagnostics) =
            resolve_terrain_data(entity, terrain_data_guid.as_deref(), index);
        terrains.push(TerrainBinding {
            entity,
            enabled,
            terrain_data_guid,
            terrain_data_path,
            terrain_data,
            diagnostics,
        });
    }
    terrains
}

fn collect_prefab_instance_roots(stream: &Stream) -> HashSet<i64> {
    let instance_ids: HashSet<i64> = stream
        .documents
        .iter()
        .filter(|doc| doc.class_id == 1001 && !doc.stripped)
        .map(|doc| doc.file_id)
        .collect();
    let mut roots = HashSet::new();
    for doc in &stream.documents {
        if doc.class_id != 1 || doc.stripped {
            continue;
        }
        let file_id = doc
            .root
            .get("GameObject")
            .and_then(|body| body.get("m_PrefabInstance"))
            .and_then(|node| node.get("fileID"))
            .and_then(Node::as_i64)
            .unwrap_or(0);
        if instance_ids.contains(&file_id) {
            roots.insert(doc.file_id);
        }
    }
    roots
}

fn resolve_terrain_data(
    entity: EntityId,
    guid: Option<&str>,
    index: Option<&GuidIndex>,
) -> (Option<PathBuf>, ImportedTerrainData, Vec<String>) {
    let Some(guid) = guid else {
        return (None, ImportedTerrainData::default(), Vec::new());
    };
    let Some(path) = index.and_then(|guids| {
        guids
            .get(guid)
            .or_else(|| guids.get(&guid.to_ascii_lowercase()))
    }) else {
        return (
            None,
            ImportedTerrainData::default(),
            vec![format!(
                "Terrain on entity {} references TerrainData guid {} that could not be resolved",
                entity.as_u64(),
                guid
            )],
        );
    };
    let path_buf = Path::to_path_buf(path);
    match import_terrain_data_file(&path_buf) {
        Ok(imported) => {
            let diagnostics = imported.diagnostics.clone();
            (Some(path_buf), imported, diagnostics)
        }
        Err(error) => (
            Some(path_buf),
            ImportedTerrainData::default(),
            vec![format!(
                "Terrain on entity {} failed to import TerrainData guid {}: {error}",
                entity.as_u64(),
                guid
            )],
        ),
    }
}

fn resolve_component_entity(
    body: &Node,
    game_objects: &HashMap<i64, GameObjectRecord>,
    instance_roots: &HashSet<i64>,
) -> Option<EntityId> {
    let file_id = body
        .get("m_GameObject")
        .and_then(|node| node.get("fileID"))
        .and_then(Node::as_i64)?;
    if instance_roots.contains(&file_id) {
        return None;
    }
    game_objects.get(&file_id).map(|record| record.entity)
}

fn extract_guid(node: Option<&Node>) -> Option<String> {
    let node = node?;
    if node.get("fileID").and_then(Node::as_i64) == Some(0) {
        return None;
    }
    let guid = node.get("guid").and_then(Node::as_str).map(str::trim)?;
    if guid.is_empty() {
        return None;
    }
    Some(guid.to_string())
}

fn parse_bool(node: Option<&Node>, default: bool) -> bool {
    match node.and_then(Node::as_f64) {
        Some(value) => value != 0.0,
        None => match node.and_then(Node::as_str) {
            Some("true" | "True" | "1") => true,
            Some("false" | "False" | "0") => false,
            _ => default,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use unitas_core::EntityId;

    use super::{collect_scene_terrains, TERRAIN_CLASS};
    use crate::meta::GuidIndex;
    use crate::unity_yaml;

    const TERRAIN_SCENE_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_Name: GroundTerrain
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &1001
Transform:
  m_GameObject: {fileID: 1000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!218 &1002
Terrain:
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_TerrainData: {fileID: 15600000, guid: 90000000000000000000000000000099, type: 2}
";

    const TERRAIN_DATA_ASSET_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!156 &15600000
TerrainData:
  m_Name: MeadowData
  m_Heightmap:
    m_Resolution: 2
    m_Size: {x: 50, y: 20, z: 50}
    m_Heights:
    - 0.25
    - 0.5
    - 0.75
    - 1.0
  m_DetailDatabase:
    m_TreePrototypes:
    - prefab: {fileID: 1, guid: 11111111111111111111111111111111, type: 3}
";

    #[test]
    fn collects_terrain_and_imports_referenced_terrain_data() -> Result<(), Box<dyn Error>> {
        assert_eq!(TERRAIN_CLASS, 218);
        let temp_dir =
            std::env::temp_dir().join(format!("unitas_scene_terrain_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir)?;
        let asset_path = temp_dir.join("MeadowData.asset");
        let meta_path = temp_dir.join("MeadowData.asset.meta");
        fs::write(&asset_path, TERRAIN_DATA_ASSET_YAML)?;
        fs::write(
            &meta_path,
            "fileFormatVersion: 2\nguid: 90000000000000000000000000000099\n",
        )?;

        let index = GuidIndex::build(&temp_dir)?;
        let stream = unity_yaml::parse(TERRAIN_SCENE_YAML)?;
        let bindings = collect_scene_terrains(&stream, Some(&index));
        let _ = fs::remove_dir_all(&temp_dir);

        assert_eq!(bindings.len(), 1);
        let binding = &bindings[0];
        assert_eq!(binding.entity, EntityId::new(0));
        assert!(binding.enabled);
        assert_eq!(
            binding.terrain_data_guid.as_deref(),
            Some("90000000000000000000000000000099")
        );
        assert_eq!(binding.terrain_data.heightmap_resolution, 2);
        assert_eq!(binding.terrain_data.size, [50.0, 20.0, 50.0]);
        assert_eq!(binding.terrain_data.heights, vec![0.25, 0.5, 0.75, 1.0]);
        assert_eq!(
            binding.diagnostics,
            vec![String::from("TerrainData tree prototypes are not imported")]
        );
        Ok(())
    }

    #[test]
    fn unresolved_terrain_data_falls_back_to_default_and_records_diagnostic(
    ) -> Result<(), Box<dyn Error>> {
        let stream = unity_yaml::parse(TERRAIN_SCENE_YAML)?;
        let bindings = collect_scene_terrains(&stream, None);
        assert_eq!(bindings.len(), 1);
        let binding = &bindings[0];
        assert_eq!(binding.terrain_data.heightmap_resolution, 33);
        assert_eq!(binding.diagnostics.len(), 1);
        assert!(binding.diagnostics[0].contains("could not be resolved"));
        Ok(())
    }
}
