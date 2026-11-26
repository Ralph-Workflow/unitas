use std::error::Error;
use std::path::Path;

use unitas_asset::scene::{assemble_scene_file_bundle, AssembledScene, ColliderComponent};
use unitas_core::EntityId;

const OVERRIDE_SCENE_FIXTURE: &str =
    "tests/fixtures/prefab_project/Assets/Scenes/OverrideScene.unity";

type TestResult = Result<(), Box<dyn Error>>;

fn load_override_scene() -> Result<AssembledScene, Box<dyn Error>> {
    Ok(assemble_scene_file_bundle(Path::new(
        OVERRIDE_SCENE_FIXTURE,
    ))?)
}

fn entity_named(scene: &AssembledScene, name: &str) -> Result<EntityId, Box<dyn Error>> {
    for id in scene.graph.breadth_first_order() {
        if let Some(entity) = scene.graph.entity(id) {
            if entity.name == name {
                return Ok(id);
            }
        }
    }
    Err(format!("entity {name} missing").into())
}

#[test]
fn prefab_instance_structural_overrides_apply_at_load() -> TestResult {
    let scene = load_override_scene()?;
    let overrides = entity_named(&scene, "Overrides")?;
    assert!(entity_named(&scene, "RemovedChild").is_err());
    let added = entity_named(&scene, "AddedChild")?;
    assert_eq!(scene.graph.parent(added), Some(overrides));
    let binding = scene
        .physics
        .iter()
        .find(|binding| binding.entity == overrides)
        .ok_or("expanded prefab physics binding missing")?;
    assert!(!binding
        .colliders
        .iter()
        .any(|collider| matches!(collider, ColliderComponent::Box(_))));
    assert!(binding.rigidbody.is_some());
    let sphere = binding
        .colliders
        .iter()
        .find_map(|collider| match collider {
            ColliderComponent::Sphere(sphere) => Some(sphere),
            _ => None,
        })
        .ok_or("added sphere collider missing")?;
    assert_eq!(sphere.radius, 2.5);
    Ok(())
}

#[test]
fn prefab_instance_property_override_applies_at_load() -> TestResult {
    let scene = load_override_scene()?;
    let overrides = entity_named(&scene, "Overrides")?;
    let binding = scene
        .physics
        .iter()
        .find(|binding| binding.entity == overrides)
        .ok_or("expanded prefab physics binding missing")?;
    let rigidbody = binding.rigidbody.as_ref().ok_or("rigidbody missing")?;
    assert_eq!(rigidbody.mass, 7.5);
    Ok(())
}

#[test]
fn nested_instance_override_resolves_at_load() -> TestResult {
    let scene = load_override_scene()?;
    let composed = entity_named(&scene, "ComposedCannon")?;
    assert!(entity_named(&scene, "BigCannon").is_err());
    assert!(entity_named(&scene, "Cannon").is_err());
    let barrel = entity_named(&scene, "Barrel")?;
    assert_eq!(scene.graph.parent(composed), Some(barrel));
    Ok(())
}
