use std::error::Error;
use std::path::Path;

use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::{
    assemble_scene, assemble_scene_file, assemble_scene_file_with_guids, assemble_scene_with_guids,
    SceneError,
};
use unitas_asset::unity_yaml;
use unitas_core::{EntityId, SceneGraph};

const SCENE_FIXTURE: &str = "tests/fixtures/scene.unity";
const PREFAB_FIXTURE: &str = "tests/fixtures/prefab.prefab";
const PROJECT_DIR: &str = "tests/fixtures/project";

const MISSING_GAME_OBJECT: &str = "%YAML 1.1
--- !u!4 &10
Transform:
  m_GameObject: {fileID: 99}
  m_Father: {fileID: 0}
";

const DUPLICATE_TRANSFORM_OWNER: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Shared
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
--- !u!4 &3
Transform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
";

const MISSING_FATHER: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Child
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 77}
";

const MISSING_CHILD: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Root
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_Children:
  - {fileID: 88}
  m_Father: {fileID: 0}
";

const MALFORMED_FILE_ID: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Root
--- !u!4 &2
Transform:
  m_GameObject: {fileID: nope}
  m_Father: {fileID: 0}
";

type TestResult = Result<(), Box<dyn Error>>;

fn entity_named(graph: &SceneGraph, name: &str) -> Result<EntityId, Box<dyn Error>> {
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id) {
            if entity.name == name {
                return Ok(id);
            }
        }
    }
    Err(format!("no entity named {name}").into())
}

fn entity_names(graph: &SceneGraph) -> Vec<String> {
    let mut names = Vec::new();
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id) {
            names.push(entity.name.clone());
        }
    }
    names
}

#[test]
fn scene_game_objects_become_named_entities() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    assert_eq!(
        entity_names(&graph),
        vec![
            String::from("Root"),
            String::from("Child"),
            String::from("Sibling"),
            String::from("Grandchild"),
        ]
    );
    Ok(())
}

#[test]
fn scene_transforms_form_the_parent_child_hierarchy() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    let root = entity_named(&graph, "Root")?;
    let child = entity_named(&graph, "Child")?;
    let grandchild = entity_named(&graph, "Grandchild")?;
    let sibling = entity_named(&graph, "Sibling")?;
    assert_eq!(graph.parent(root), None);
    assert_eq!(graph.parent(child), Some(root));
    assert_eq!(graph.parent(grandchild), Some(child));
    assert_eq!(graph.parent(sibling), Some(root));
    assert_eq!(graph.children(root), &[child, sibling][..]);
    assert_eq!(graph.children(child), &[grandchild][..]);
    assert_eq!(graph.roots(), vec![root]);
    Ok(())
}

#[test]
fn scene_local_position_and_scale_are_applied() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    let root = entity_named(&graph, "Root")?;
    let child = entity_named(&graph, "Child")?;
    let transform = graph.transform(root).ok_or("root has no transform")?;
    assert_eq!(transform.translation, [1.0, 2.0, 3.0]);
    assert_eq!(transform.scale, [2.0, 2.0, 2.0]);
    let transform = graph.transform(child).ok_or("child has no transform")?;
    assert_eq!(transform.translation, [0.5, 0.0, -1.0]);
    assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    Ok(())
}

#[test]
fn scene_rotation_stays_at_the_default_value() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    for id in graph.breadth_first_order() {
        let transform = graph.transform(id).ok_or("entity has no transform")?;
        assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
    }
    Ok(())
}

#[test]
fn scene_depth_first_order_follows_the_hierarchy() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    let root = entity_named(&graph, "Root")?;
    let child = entity_named(&graph, "Child")?;
    let grandchild = entity_named(&graph, "Grandchild")?;
    let sibling = entity_named(&graph, "Sibling")?;
    assert_eq!(graph.depth_first_order(), vec![root, child, grandchild, sibling]);
    Ok(())
}

#[test]
fn scene_missing_game_object_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(MISSING_GAME_OBJECT)?;
    let result = assemble_scene(&stream);
    assert!(matches!(result, Err(SceneError::MissingGameObject { .. })));
    Ok(())
}

#[test]
fn scene_two_transforms_claiming_one_game_object_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(DUPLICATE_TRANSFORM_OWNER)?;
    let result = assemble_scene(&stream);
    assert!(matches!(result, Err(SceneError::DuplicateTransformOwner { .. })));
    Ok(())
}

#[test]
fn scene_missing_father_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(MISSING_FATHER)?;
    let result = assemble_scene(&stream);
    assert!(matches!(result, Err(SceneError::MissingFather { .. })));
    Ok(())
}

#[test]
fn scene_missing_child_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(MISSING_CHILD)?;
    let result = assemble_scene(&stream);
    assert!(matches!(result, Err(SceneError::MissingChild { .. })));
    Ok(())
}

#[test]
fn scene_malformed_file_id_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(MALFORMED_FILE_ID)?;
    let result = assemble_scene(&stream);
    assert!(matches!(result, Err(SceneError::InvalidFileId { .. })));
    Ok(())
}

#[test]
fn prefab_game_objects_are_assembled_like_a_scene() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PREFAB_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    assert_eq!(entity_names(&graph), vec![String::from("Cube")]);
    Ok(())
}

#[test]
fn prefab_source_reference_resolves_against_the_guid_index() -> TestResult {
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    let stream = unity_yaml::parse_file(Path::new(PREFAB_FIXTURE))?;
    let graph = assemble_scene_with_guids(&stream, &index)?;
    assert_eq!(entity_names(&graph), vec![String::from("Cube")]);
    Ok(())
}

#[test]
fn prefab_source_reference_without_a_match_is_an_error() -> TestResult {
    let index = GuidIndex::default();
    let stream = unity_yaml::parse_file(Path::new(PREFAB_FIXTURE))?;
    let result = assemble_scene_with_guids(&stream, &index);
    assert!(matches!(result, Err(SceneError::UnresolvedExternalReference { .. })));
    Ok(())
}

#[test]
fn scene_file_entry_points_assemble_the_fixtures() -> TestResult {
    let graph = assemble_scene_file(Path::new(SCENE_FIXTURE))?;
    assert_eq!(entity_names(&graph).len(), 4);
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    let graph = assemble_scene_file_with_guids(Path::new(PREFAB_FIXTURE), &index)?;
    assert_eq!(entity_names(&graph), vec![String::from("Cube")]);
    Ok(())
}
