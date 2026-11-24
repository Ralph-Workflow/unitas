use std::error::Error;
use std::f32::consts::FRAC_1_SQRT_2;
use std::path::Path;

use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::{
    assemble_scene, assemble_scene_file, assemble_scene_file_with_guids,
    assemble_scene_file_with_physics, assemble_scene_with_guids, assemble_scene_with_physics,
    SceneError,
};
use unitas_asset::unity_yaml;
use unitas_core::{EntityId, SceneGraph};

const SCENE_FIXTURE: &str = "tests/fixtures/scene.unity";
const PREFAB_FIXTURE: &str = "tests/fixtures/prefab.prefab";
const PHYSICS_FIXTURE: &str = "tests/fixtures/physics.unity";
const PHYSICS_TRIGGER_FIXTURE: &str = "tests/fixtures/physics_trigger.unity";
const PHYSICS_LAYER_FIXTURE: &str = "tests/fixtures/physics_layer.unity";
const PROJECT_DIR: &str = "tests/fixtures/project";
const PREFAB_SCENE_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/PrefabScene.unity";
const PREFAB_CYCLE_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/CycleScene.unity";
const PREFAB_UNKNOWN_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/UnknownScene.unity";
const PREFAB_UNITY_SCENE_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/UnityPrefabScene.unity";
const PREFAB_NO_SOURCE_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/NoSourceScene.unity";
const PREFAB_SCRIPT_GUID: &str = "11111111111111111111111111110004";

const NEGATIVE_LAYER: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Negative
  m_Layer: -3
--- !u!54 &2
Rigidbody:
  m_GameObject: {fileID: 1}
";

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

const PHYSICS_MISSING_GAME_OBJECT: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Ball
--- !u!54 &2
Rigidbody:
  m_GameObject: {fileID: 99}
";

type TestResult = Result<(), Box<dyn Error>>;

fn scene_with_local_rotation(rotation: Option<&str>) -> String {
    let rotation_line = match rotation {
        Some(rotation) => format!("  m_LocalRotation: {rotation}\n"),
        None => String::new(),
    };
    format!(
        "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Rotated
--- !u!4 &2
Transform:
  m_GameObject: {{fileID: 1}}
{rotation_line}  m_Father: {{fileID: 0}}
"
    )
}

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
        assert_eq!(transform.rotation, [0.0, 0.0, 0.0, 1.0]);
    }
    Ok(())
}

#[test]
fn scene_local_rotation_is_applied() -> TestResult {
    let stream = unity_yaml::parse(&scene_with_local_rotation(Some(
        "{x: 0, y: 0, z: 0.70710678, w: 0.70710678}",
    )))?;
    let graph = assemble_scene(&stream)?;
    let rotated = entity_named(&graph, "Rotated")?;
    let transform = graph.transform(rotated).ok_or("entity has no transform")?;
    assert_eq!(
        transform.rotation,
        [0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]
    );
    Ok(())
}

#[test]
fn scene_unnormalized_local_rotation_stays_at_the_identity() -> TestResult {
    let stream = unity_yaml::parse(&scene_with_local_rotation(Some(
        "{x: 0.5, y: -0.5, z: 0.25, w: 0.707}",
    )))?;
    let graph = assemble_scene(&stream)?;
    let rotated = entity_named(&graph, "Rotated")?;
    let transform = graph.transform(rotated).ok_or("entity has no transform")?;
    assert_eq!(transform.rotation, [0.0, 0.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn scene_missing_local_rotation_stays_at_the_identity() -> TestResult {
    let stream = unity_yaml::parse(&scene_with_local_rotation(None))?;
    let graph = assemble_scene(&stream)?;
    let rotated = entity_named(&graph, "Rotated")?;
    let transform = graph.transform(rotated).ok_or("entity has no transform")?;
    assert_eq!(transform.rotation, [0.0, 0.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn scene_malformed_local_rotation_stays_at_the_identity() -> TestResult {
    let stream = unity_yaml::parse(&scene_with_local_rotation(Some("{x: 0.5, y: -0.5, z: 0.25}")))?;
    let graph = assemble_scene(&stream)?;
    let rotated = entity_named(&graph, "Rotated")?;
    let transform = graph.transform(rotated).ok_or("entity has no transform")?;
    assert_eq!(transform.rotation, [0.0, 0.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn scene_non_finite_local_rotation_stays_at_the_identity() -> TestResult {
    let stream = unity_yaml::parse(&scene_with_local_rotation(Some("{x: inf, y: 0, z: 0, w: 1}")))?;
    let graph = assemble_scene(&stream)?;
    let rotated = entity_named(&graph, "Rotated")?;
    let transform = graph.transform(rotated).ok_or("entity has no transform")?;
    assert_eq!(transform.rotation, [0.0, 0.0, 0.0, 1.0]);
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
fn physics_components_bind_to_their_game_object_entity() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PHYSICS_FIXTURE))?;
    let (graph, scripts, bindings) = assemble_scene_with_physics(&stream)?;
    let ball = entity_named(&graph, "Ball")?;
    let crate_entity = entity_named(&graph, "Crate")?;
    let defaults = entity_named(&graph, "Defaults")?;
    assert!(scripts.is_empty());
    assert_eq!(
        bindings.iter().map(|binding| binding.entity).collect::<Vec<_>>(),
        vec![ball, crate_entity, defaults]
    );
    let ball_binding = &bindings[0];
    let rigidbody = ball_binding.rigidbody.as_ref().ok_or("Ball has no rigidbody")?;
    assert_eq!(rigidbody.mass, 2.5);
    assert!(!rigidbody.use_gravity);
    let sphere = ball_binding.sphere_collider.as_ref().ok_or("Ball has no sphere collider")?;
    assert_eq!(sphere.radius, 0.75);
    assert_eq!(sphere.center, [0.0, 1.0, 0.0]);
    assert!(ball_binding.box_collider.is_none());
    let crate_binding = &bindings[1];
    let box_collider = crate_binding.box_collider.as_ref().ok_or("Crate has no box collider")?;
    assert_eq!(box_collider.size, [2.0, 3.0, 4.0]);
    assert_eq!(box_collider.center, [0.5, 0.0, -0.5]);
    assert!(crate_binding.rigidbody.is_none());
    assert!(crate_binding.sphere_collider.is_none());
    Ok(())
}

#[test]
fn physics_defaults_apply_when_a_field_is_absent() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PHYSICS_FIXTURE))?;
    let (graph, _scripts, bindings) = assemble_scene_with_physics(&stream)?;
    let defaults = entity_named(&graph, "Defaults")?;
    let binding = bindings
        .iter()
        .find(|binding| binding.entity == defaults)
        .ok_or("no physics binding for Defaults")?;
    let rigidbody = binding.rigidbody.as_ref().ok_or("Defaults has no rigidbody")?;
    assert_eq!(rigidbody.mass, 1.0);
    assert!(rigidbody.use_gravity);
    assert!(!rigidbody.is_trigger);
    let box_collider = binding.box_collider.as_ref().ok_or("Defaults has no box collider")?;
    assert_eq!(box_collider.size, [1.0, 1.0, 1.0]);
    assert_eq!(box_collider.center, [0.0, 0.0, 0.0]);
    assert!(!box_collider.is_trigger);
    let sphere = binding.sphere_collider.as_ref().ok_or("Defaults has no sphere collider")?;
    assert_eq!(sphere.radius, 0.5);
    assert_eq!(sphere.center, [0.0, 0.0, 0.0]);
    assert!(!sphere.is_trigger);
    Ok(())
}

#[test]
fn physics_is_trigger_flag_parses_from_scene_components() -> TestResult {
    let (graph, _scripts, bindings) =
        assemble_scene_file_with_physics(Path::new(PHYSICS_TRIGGER_FIXTURE))?;
    let trigger_zone = entity_named(&graph, "TriggerZone")?;
    let ball = entity_named(&graph, "Ball")?;
    let trigger_rig = entity_named(&graph, "TriggerRig")?;

    let zone_binding = bindings
        .iter()
        .find(|binding| binding.entity == trigger_zone)
        .ok_or("no physics binding for TriggerZone")?;
    let zone_box = zone_binding
        .box_collider
        .as_ref()
        .ok_or("TriggerZone has no box collider")?;
    assert!(zone_box.is_trigger);

    let ball_binding = bindings
        .iter()
        .find(|binding| binding.entity == ball)
        .ok_or("no physics binding for Ball")?;
    let ball_rb = ball_binding
        .rigidbody
        .as_ref()
        .ok_or("Ball has no rigidbody")?;
    let ball_sphere = ball_binding
        .sphere_collider
        .as_ref()
        .ok_or("Ball has no sphere collider")?;
    assert!(!ball_rb.is_trigger);
    assert!(!ball_sphere.is_trigger);

    let rig_binding = bindings
        .iter()
        .find(|binding| binding.entity == trigger_rig)
        .ok_or("no physics binding for TriggerRig")?;
    let rig_rb = rig_binding
        .rigidbody
        .as_ref()
        .ok_or("TriggerRig has no rigidbody")?;
    let rig_sphere = rig_binding
        .sphere_collider
        .as_ref()
        .ok_or("TriggerRig has no sphere collider")?;
    assert!(rig_rb.is_trigger);
    assert!(rig_sphere.is_trigger);
    Ok(())
}

#[test]
fn physics_layer_parses_from_the_game_object() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PHYSICS_LAYER_FIXTURE))?;
    let (graph, _scripts, bindings) = assemble_scene_with_physics(&stream)?;
    let layered = entity_named(&graph, "Layered")?;
    let binding = bindings
        .iter()
        .find(|binding| binding.entity == layered)
        .ok_or("no physics binding for Layered")?;
    assert_eq!(binding.layer, 5);
    Ok(())
}

#[test]
fn physics_layer_defaults_to_zero_when_absent() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PHYSICS_LAYER_FIXTURE))?;
    let (graph, _scripts, bindings) = assemble_scene_with_physics(&stream)?;
    let unlayered = entity_named(&graph, "Unlayered")?;
    let binding = bindings
        .iter()
        .find(|binding| binding.entity == unlayered)
        .ok_or("no physics binding for Unlayered")?;
    assert_eq!(binding.layer, 0);
    Ok(())
}

#[test]
fn physics_negative_layer_falls_back_to_zero() -> TestResult {
    let stream = unity_yaml::parse(NEGATIVE_LAYER)?;
    let (_graph, _scripts, bindings) = assemble_scene_with_physics(&stream)?;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].layer, 0);
    Ok(())
}

#[test]
fn physics_documents_do_not_change_the_plain_scene_assembly() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PHYSICS_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    assert_eq!(
        entity_names(&graph),
        vec![String::from("Ball"), String::from("Crate"), String::from("Defaults")]
    );
    Ok(())
}

#[test]
fn physics_reference_to_an_unknown_game_object_is_an_error() -> TestResult {
    let stream = unity_yaml::parse(PHYSICS_MISSING_GAME_OBJECT)?;
    let result = assemble_scene_with_physics(&stream);
    assert!(matches!(result, Err(SceneError::MissingGameObject { .. })));
    Ok(())
}

#[test]
fn physics_file_entry_point_assembles_the_fixture() -> TestResult {
    let (graph, scripts, bindings) = assemble_scene_file_with_physics(Path::new(PHYSICS_FIXTURE))?;
    assert!(scripts.is_empty());
    assert_eq!(bindings.len(), 3);
    assert_eq!(entity_names(&graph).len(), 3);
    Ok(())
}

#[test]
fn scene_file_entry_points_assemble_the_fixtures() -> TestResult {
    let graph = assemble_scene_file(Path::new(SCENE_FIXTURE))?;
    assert_eq!(entity_names(&graph).len(), 4);
    let index = GuidIndex::build(Path::new(PROJECT_DIR))?;
    let graph = assemble_scene_file_with_guids(Path::new(PREFAB_FIXTURE), &index)?;
    assert_eq!(
        entity_names(&graph),
        vec![String::from("Cube"), String::from("Camera")]
    );
    Ok(())
}

#[test]
fn prefab_instance_is_expanded_into_the_scene_graph() -> TestResult {
    let graph = assemble_scene_file(Path::new(PREFAB_SCENE_FIXTURE))?;
    assert_eq!(
        entity_names(&graph),
        vec![
            String::from("Stage"),
            String::from("GuardTower"),
            String::from("Barrel"),
            String::from("BigCannon"),
        ]
    );
    let stage = entity_named(&graph, "Stage")?;
    let guard = entity_named(&graph, "GuardTower")?;
    let barrel = entity_named(&graph, "Barrel")?;
    let cannon = entity_named(&graph, "BigCannon")?;
    assert_eq!(graph.parent(guard), Some(stage));
    assert_eq!(graph.parent(barrel), Some(guard));
    assert_eq!(graph.parent(cannon), Some(barrel));
    assert_eq!(graph.children(stage), &[guard][..]);
    assert_eq!(graph.children(guard), &[barrel][..]);
    assert_eq!(graph.roots(), vec![stage]);
    Ok(())
}

#[test]
fn prefab_instance_applies_name_and_transform_modifications() -> TestResult {
    let graph = assemble_scene_file(Path::new(PREFAB_SCENE_FIXTURE))?;
    let guard = entity_named(&graph, "GuardTower")?;
    let transform = graph.transform(guard).ok_or("prefab root has no transform")?;
    assert_eq!(transform.translation, [5.0, 2.0, 0.0]);
    assert_eq!(transform.scale, [1.0, 1.0, 4.0]);
    Ok(())
}

#[test]
fn prefab_instance_merges_script_and_physics_records() -> TestResult {
    let (graph, scripts, bindings) =
        assemble_scene_file_with_physics(Path::new(PREFAB_SCENE_FIXTURE))?;
    let guard = entity_named(&graph, "GuardTower")?;
    let barrel = entity_named(&graph, "Barrel")?;
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].entity, guard);
    assert_eq!(scripts[0].guid, PREFAB_SCRIPT_GUID);
    let guard_binding = bindings
        .iter()
        .find(|binding| binding.entity == guard)
        .ok_or("no physics binding for GuardTower")?;
    let rigidbody = guard_binding.rigidbody.as_ref().ok_or("GuardTower has no rigidbody")?;
    assert_eq!(rigidbody.mass, 3.5);
    assert!(!rigidbody.use_gravity);
    let barrel_binding = bindings
        .iter()
        .find(|binding| binding.entity == barrel)
        .ok_or("no physics binding for Barrel")?;
    let collider = barrel_binding.box_collider.as_ref().ok_or("Barrel has no box collider")?;
    assert_eq!(collider.size, [1.0, 2.0, 3.0]);
    assert_eq!(collider.center, [0.0, 0.5, 0.0]);
    Ok(())
}

#[test]
fn cyclic_prefab_reference_is_an_error() -> TestResult {
    let result = assemble_scene_file(Path::new(PREFAB_CYCLE_FIXTURE));
    assert!(matches!(result, Err(SceneError::CyclicPrefabReference { .. })));
    Ok(())
}

#[test]
fn prefab_instance_with_an_unknown_source_is_an_error() -> TestResult {
    let result = assemble_scene_file(Path::new(PREFAB_UNKNOWN_FIXTURE));
    assert!(matches!(result, Err(SceneError::UnresolvedExternalReference { .. })));
    Ok(())
}

#[test]
fn in_memory_assembly_does_not_expand_prefab_instances() -> TestResult {
    let stream = unity_yaml::parse_file(Path::new(PREFAB_SCENE_FIXTURE))?;
    let graph = assemble_scene(&stream)?;
    assert_eq!(entity_names(&graph), vec![String::from("Stage")]);
    Ok(())
}

#[test]
fn unity_prefab_instance_root_is_assembled_once() -> TestResult {
    let graph = assemble_scene_file(Path::new(PREFAB_UNITY_SCENE_FIXTURE))?;
    assert_eq!(
        entity_names(&graph),
        vec![
            String::from("Stage"),
            String::from("GuardTower"),
            String::from("Barrel"),
            String::from("BigCannon"),
        ]
    );
    let stage = entity_named(&graph, "Stage")?;
    let guard = entity_named(&graph, "GuardTower")?;
    assert_eq!(graph.parent(guard), Some(stage));
    assert_eq!(graph.roots(), vec![stage]);
    Ok(())
}

#[test]
fn prefab_instance_without_a_source_is_an_error() -> TestResult {
    let result = assemble_scene_file(Path::new(PREFAB_NO_SOURCE_FIXTURE));
    assert!(matches!(result, Err(SceneError::MissingPrefabSource { .. })));
    Ok(())
}
