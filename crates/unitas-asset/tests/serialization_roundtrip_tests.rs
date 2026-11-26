use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unitas_asset::binary_ser;
use unitas_asset::prefab_variant::PrefabVariantDefinition;
use unitas_asset::project_settings;
use unitas_asset::scene::{assemble_scene_file_bundle, ColliderComponent};
use unitas_asset::scene_save;
use unitas_asset::unity_yaml::{self, Node, Stream};

type TestResult = Result<(), Box<dyn Error>>;

const SCENE: &str = "Assets/Scenes/RollABall.unity";
const PHYSICS_SCENE: &str = "Assets/Scenes/PhysicsLab.unity";
const PREFAB: &str = "Assets/Prefabs/Pickup.prefab";
const VARIANT: &str = "Assets/Prefabs/PickupGold.prefab";
const PLAYER_SETTINGS: &str = "ProjectSettings/ProjectSettings.asset";
const QUALITY_SETTINGS: &str = "ProjectSettings/QualitySettings.asset";

const MESH_COLLIDER_SCENE: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Frame
  m_Layer: 0
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Father: {fileID: 0}
  m_Children: []
--- !u!64 &3
MeshCollider:
  m_GameObject: {fileID: 1}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 3}
  m_Convex: 0
  m_IsTrigger: 0
  m_Center: {x: 0, y: 1, z: 0}
  m_Material: {fileID: 0}
";

const SCENE_CAMERA: i64 = 1000000003;
const SCENE_BALL_RIGIDBODY: i64 = 1000000133;
const SCENE_BALL_COLLIDER: i64 = 1000000134;
const PREFAB_COLLIDER: i64 = 13500000;
const PREFAB_BEHAVIOUR: i64 = 11400000;
const VARIANT_INSTANCE: i64 = 100100000;
const VARIANT_AURA: i64 = 5400001;
const VARIANT_AURA_TRANSFORM: i64 = 5400002;
const VARIANT_AURA_RENDERER: i64 = 2300002;
const VARIANT_AURA_BEHAVIOUR: i64 = 5400003;

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/roll-a-ball")
}

fn sample(relative: &str) -> PathBuf {
    sample_root().join(relative)
}

fn temp_dir(name: &str) -> std::io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "unitas_serialization_roundtrip_{name}_{}_{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn copy_sample(relative: &str, name: &str) -> std::io::Result<PathBuf> {
    let root = temp_dir(name)?;
    let target = root.join(Path::new(relative).file_name().unwrap_or_default());
    fs::copy(sample(relative), &target)?;
    Ok(target)
}

fn document_identities(stream: &Stream) -> Vec<(u32, i64, bool)> {
    stream
        .documents
        .iter()
        .map(|document| (document.class_id, document.file_id, document.stripped))
        .collect()
}

fn mapping_entry_mut<'a>(node: &'a mut Node, key: &str) -> Option<&'a mut Node> {
    match node {
        Node::Mapping(entries) => entries
            .iter_mut()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

fn set_field(node: &mut Node, keys: &[&str], value: &str) -> bool {
    let (key, rest) = match keys.split_first() {
        Some(parts) => parts,
        None => return false,
    };
    match mapping_entry_mut(node, key) {
        Some(entry) => {
            if rest.is_empty() {
                *entry = Node::Scalar(value.to_string());
                true
            } else {
                set_field(entry, rest, value)
            }
        }
        None => false,
    }
}

fn field<'a>(stream: &'a Stream, file_id: i64, keys: &[&str]) -> Option<&'a Node> {
    let document = stream
        .documents
        .iter()
        .find(|document| document.file_id == file_id)?;
    let mut node = &document.root;
    for key in keys {
        node = node.get(key)?;
    }
    Some(node)
}

fn scalar_at(stream: &Stream, file_id: i64, keys: &[&str]) -> Option<String> {
    field(stream, file_id, keys)
        .and_then(Node::as_str)
        .map(String::from)
}

fn doubled(value: &str) -> String {
    let number: f64 = value.parse().unwrap_or_default();
    let doubled = number * 2.0;
    if doubled.fract() == 0.0 {
        format!("{}", doubled as i64)
    } else {
        format!("{doubled}")
    }
}

fn document_mut(stream: &mut Stream, file_id: i64) -> Option<&mut Node> {
    stream
        .documents
        .iter_mut()
        .find(|document| document.file_id == file_id)
        .map(|document| &mut document.root)
}

fn modification_value(stream: &Stream, property_path: &str) -> Option<String> {
    field(
        stream,
        VARIANT_INSTANCE,
        &["PrefabInstance", "m_Modification"],
    )
    .and_then(|body| body.get("m_Modifications"))
    .and_then(Node::as_sequence)
    .and_then(|items| {
        items.iter().find(|item| {
            item.get("propertyPath")
                .and_then(Node::as_str)
                .is_some_and(|path| path == property_path)
        })
    })
    .and_then(|item| item.get("value"))
    .and_then(Node::as_str)
    .map(String::from)
}

fn set_modification_value(stream: &mut Stream, property_path: &str, value: &str) -> bool {
    let instance = match stream
        .documents
        .iter_mut()
        .find(|document| document.file_id == VARIANT_INSTANCE)
    {
        Some(document) => document,
        None => return false,
    };
    let body = match mapping_entry_mut(&mut instance.root, "PrefabInstance") {
        Some(body) => body,
        None => return false,
    };
    let modification = match mapping_entry_mut(body, "m_Modification") {
        Some(modification) => modification,
        None => return false,
    };
    let items = match mapping_entry_mut(modification, "m_Modifications") {
        Some(Node::Sequence(items)) => items,
        _ => return false,
    };
    let index = match items.iter().position(|item| {
        item.get("propertyPath")
            .and_then(Node::as_str)
            .is_some_and(|path| path == property_path)
    }) {
        Some(index) => index,
        None => return false,
    };
    match mapping_entry_mut(&mut items[index], "value") {
        Some(value_node) => {
            *value_node = Node::Scalar(value.to_string());
            true
        }
        None => false,
    }
}

#[test]
fn real_scene_text_round_trip_preserves_every_document_and_the_edited_value() -> TestResult {
    let path = sample(SCENE);
    let target = copy_sample(SCENE, "scene")?;
    let original = unity_yaml::parse_file(&path)?;
    let mut edited = original.clone();

    let radius_before = scalar_at(
        &original,
        SCENE_BALL_COLLIDER,
        &["SphereCollider", "m_Radius"],
    )
    .ok_or("the scene must declare the player ball collider radius")?;
    let radius_after = doubled(&radius_before);
    assert_ne!(
        radius_before, radius_after,
        "the edited radius must differ from the value on disk"
    );
    assert!(
        set_field(
            document_mut(&mut edited, SCENE_BALL_COLLIDER).ok_or("missing ball collider")?,
            &["SphereCollider", "m_Radius"],
            &radius_after
        ),
        "the ball collider must expose m_Radius"
    );

    scene_save::save_stream(&target, &edited)?;
    let reloaded = unity_yaml::parse_file(&target)?;

    assert_eq!(
        document_identities(&reloaded),
        document_identities(&original),
        "every scene document class, fileID, and stripped flag must survive the edit and save"
    );
    assert_eq!(
        scalar_at(
            &reloaded,
            SCENE_BALL_COLLIDER,
            &["SphereCollider", "m_Radius"]
        ),
        Some(radius_after.clone()),
        "the edited collider radius must survive save and reload"
    );
    for (before, after) in original.documents.iter().zip(reloaded.documents.iter()) {
        if before.file_id == SCENE_BALL_COLLIDER {
            continue;
        }
        assert_eq!(
            before, after,
            "scene document {} must be untouched by the unrelated edit",
            before.file_id
        );
    }
    let mut reverted = reloaded.clone();
    assert!(
        set_field(
            document_mut(&mut reverted, SCENE_BALL_COLLIDER).ok_or("missing ball collider")?,
            &["SphereCollider", "m_Radius"],
            &radius_before
        ),
        "the ball collider must expose m_Radius"
    );
    assert_eq!(
        reverted, original,
        "the saved scene must differ from the file on disk only at the edited value"
    );
    Ok(())
}

#[test]
fn real_prefab_text_round_trip_preserves_every_document_and_the_edited_value() -> TestResult {
    let path = sample(PREFAB);
    let target = copy_sample(PREFAB, "prefab")?;
    let original = unity_yaml::parse_file(&path)?;
    let mut edited = original.clone();

    let radius_before = scalar_at(&original, PREFAB_COLLIDER, &["SphereCollider", "m_Radius"])
        .ok_or("the prefab must declare the pickup collider radius")?;
    let radius_after = doubled(&radius_before);
    assert!(
        set_field(
            document_mut(&mut edited, PREFAB_COLLIDER).ok_or("missing collider")?,
            &["SphereCollider", "m_Radius"],
            &radius_after
        ),
        "the pickup collider must expose m_Radius"
    );

    scene_save::save_stream(&target, &edited)?;
    let reloaded = unity_yaml::parse_file(&target)?;

    assert_eq!(
        document_identities(&reloaded),
        document_identities(&original),
        "every prefab document class, fileID, and stripped flag must survive the edit and save"
    );
    assert_eq!(
        scalar_at(&reloaded, PREFAB_COLLIDER, &["SphereCollider", "m_Radius"]),
        Some(radius_after),
        "the edited collider radius must survive save and reload"
    );
    for (before, after) in original.documents.iter().zip(reloaded.documents.iter()) {
        if before.file_id == PREFAB_COLLIDER {
            continue;
        }
        assert_eq!(
            before, after,
            "prefab document {} must be untouched by the unrelated edit",
            before.file_id
        );
    }
    Ok(())
}

#[test]
fn real_prefab_variant_text_round_trip_preserves_the_instance_and_its_modifications() -> TestResult
{
    let path = sample(VARIANT);
    let target = copy_sample(VARIANT, "variant")?;
    let original = unity_yaml::parse_file(&path)?;
    let mut edited = original.clone();

    let score_before = modification_value(&original, "scoreValue")
        .ok_or("the variant must override scoreValue")?;
    let score_after = doubled(&score_before);
    assert!(
        set_modification_value(&mut edited, "scoreValue", &score_after),
        "the variant must expose the scoreValue modification value"
    );

    scene_save::save_stream(&target, &edited)?;
    let reloaded = unity_yaml::parse_file(&target)?;

    assert_eq!(
        document_identities(&reloaded),
        document_identities(&original),
        "every variant document class, fileID, and stripped flag must survive the edit and save"
    );
    assert_eq!(
        modification_value(&reloaded, "scoreValue"),
        Some(score_after),
        "the edited scoreValue override must survive save and reload"
    );
    assert_eq!(
        field(
            &reloaded,
            VARIANT_INSTANCE,
            &["PrefabInstance", "m_SourcePrefab"]
        )
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str),
        field(
            &original,
            VARIANT_INSTANCE,
            &["PrefabInstance", "m_SourcePrefab"]
        )
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str),
        "the variant must keep the source prefab guid it declares on disk"
    );
    for (before, after) in original.documents.iter().zip(reloaded.documents.iter()) {
        if before.file_id == VARIANT_INSTANCE {
            continue;
        }
        assert_eq!(
            before, after,
            "variant document {} must be untouched by the unrelated edit",
            before.file_id
        );
    }
    Ok(())
}

#[test]
fn real_project_settings_round_trip_preserves_values_and_keys_the_model_does_not_know() -> TestResult
{
    let path = sample(PLAYER_SETTINGS);
    let target = copy_sample(PLAYER_SETTINGS, "player_settings")?;
    let original = unity_yaml::parse_file(&path)?;

    let identity =
        project_settings::parse_player_settings(&path)?.ok_or("missing player settings")?;
    assert_eq!(
        identity.product_name.as_deref(),
        scalar_at(&original, 1, &["PlayerSettings", "productName"]).as_deref(),
        "the model must report the product name the file declares"
    );
    assert!(
        field(&original, 1, &["PlayerSettings", "macAppStoreCategory"]).is_some(),
        "the file must declare a key the player settings model does not represent"
    );

    let mut edited = original.clone();
    let width_before = scalar_at(&original, 1, &["PlayerSettings", "defaultScreenWidth"])
        .ok_or("the file must declare the default screen width")?;
    let width_after = doubled(&width_before);
    assert!(
        set_field(
            document_mut(&mut edited, 1).ok_or("missing player settings document")?,
            &["PlayerSettings", "defaultScreenWidth"],
            &width_after
        ),
        "the player settings must expose defaultScreenWidth"
    );

    scene_save::save_stream(&target, &edited)?;
    let reloaded = unity_yaml::parse_file(&target)?;

    assert_eq!(
        document_identities(&reloaded),
        document_identities(&original),
        "the player settings document identity must survive the edit and save"
    );
    assert_eq!(
        scalar_at(&reloaded, 1, &["PlayerSettings", "defaultScreenWidth"]),
        Some(width_after),
        "the edited screen width must survive save and reload"
    );
    assert_eq!(
        field(&reloaded, 1, &["PlayerSettings", "macAppStoreCategory"]).and_then(Node::as_str),
        field(&original, 1, &["PlayerSettings", "macAppStoreCategory"]).and_then(Node::as_str),
        "a key the player settings model does not represent must survive the edit and save"
    );
    assert_eq!(
        field(&reloaded, 1, &["PlayerSettings", "productGUID"]).and_then(Node::as_str),
        field(&original, 1, &["PlayerSettings", "productGUID"]).and_then(Node::as_str),
        "the product guid must survive the edit and save"
    );
    assert_eq!(
        field(&reloaded, 1, &["PlayerSettings", "applicationIdentifier"])
            .and_then(|node| node.get("Standalone"))
            .and_then(Node::as_str),
        field(&original, 1, &["PlayerSettings", "applicationIdentifier"])
            .and_then(|node| node.get("Standalone"))
            .and_then(Node::as_str),
        "a nested value the model does not represent must survive the edit and save"
    );
    Ok(())
}

#[test]
fn real_quality_settings_round_trip_preserves_every_level_and_its_unknown_keys() -> TestResult {
    let path = sample(QUALITY_SETTINGS);
    let target = copy_sample(QUALITY_SETTINGS, "quality_settings")?;
    let original = unity_yaml::parse_file(&path)?;
    let settings =
        project_settings::parse_quality_settings(&path)?.ok_or("missing quality settings")?;

    let levels = field(&original, 1, &["QualitySettings", "m_QualitySettings"])
        .and_then(Node::as_sequence)
        .ok_or("the file must declare its quality levels")?;
    assert_eq!(
        levels.len(),
        settings.levels.len(),
        "every quality level on disk must be represented by the model"
    );

    let mut edited = original.clone();
    let current_before = scalar_at(&original, 1, &["QualitySettings", "m_CurrentQuality"])
        .ok_or("the file must declare the current quality")?;
    let current_after = doubled(&current_before);
    assert!(
        set_field(
            document_mut(&mut edited, 1).ok_or("missing quality settings document")?,
            &["QualitySettings", "m_CurrentQuality"],
            &current_after
        ),
        "the quality settings must expose m_CurrentQuality"
    );

    scene_save::save_stream(&target, &edited)?;
    let reloaded = unity_yaml::parse_file(&target)?;

    assert_eq!(
        document_identities(&reloaded),
        document_identities(&original),
        "the quality settings document identity must survive the edit and save"
    );
    assert_eq!(
        scalar_at(&reloaded, 1, &["QualitySettings", "m_CurrentQuality"]),
        Some(current_after),
        "the edited current quality must survive save and reload"
    );
    assert_eq!(
        field(&reloaded, 1, &["QualitySettings", "m_QualitySettings"]),
        field(&original, 1, &["QualitySettings", "m_QualitySettings"]),
        "every quality level and every key inside it must survive the edit and save"
    );
    Ok(())
}

#[test]
fn unknown_keys_in_the_real_assets_survive_load_edit_and_save() -> TestResult {
    let prefab_path = sample(PREFAB);
    let prefab_target = copy_sample(PREFAB, "unknown_prefab")?;
    let original = unity_yaml::parse_file(&prefab_path)?;
    let burst = scalar_at(
        &original,
        PREFAB_BEHAVIOUR,
        &["MonoBehaviour", "burstObjectName"],
    )
    .ok_or("the prefab must declare the burst object name")?;
    let mut edited = original.clone();
    let radius_before = scalar_at(&original, PREFAB_COLLIDER, &["SphereCollider", "m_Radius"])
        .ok_or("the prefab must declare the pickup collider radius")?;
    assert!(
        set_field(
            document_mut(&mut edited, PREFAB_COLLIDER).ok_or("missing collider")?,
            &["SphereCollider", "m_Radius"],
            &doubled(&radius_before)
        ),
        "the pickup collider must expose m_Radius"
    );
    scene_save::save_stream(&prefab_target, &edited)?;
    let reloaded = unity_yaml::parse_file(&prefab_target)?;
    assert_eq!(
        scalar_at(
            &reloaded,
            PREFAB_BEHAVIOUR,
            &["MonoBehaviour", "burstObjectName"]
        ),
        Some(burst),
        "a script field the models do not represent must survive the edit and save"
    );
    assert_eq!(
        field(&reloaded, PREFAB_COLLIDER, &["SphereCollider", "m_Center"]),
        field(&original, PREFAB_COLLIDER, &["SphereCollider", "m_Center"]),
        "a nested value the models do not represent must survive the edit and save"
    );

    let scene_path = sample(SCENE);
    let scene_target = copy_sample(SCENE, "unknown_scene")?;
    let scene_original = unity_yaml::parse_file(&scene_path)?;
    let mut scene_edited = scene_original.clone();
    let mass_before = scalar_at(
        &scene_original,
        SCENE_BALL_RIGIDBODY,
        &["Rigidbody", "m_Mass"],
    )
    .ok_or("the scene must declare the ball rigidbody mass")?;
    assert!(
        set_field(
            document_mut(&mut scene_edited, SCENE_BALL_RIGIDBODY).ok_or("missing rigidbody")?,
            &["Rigidbody", "m_Mass"],
            &doubled(&mass_before)
        ),
        "the ball rigidbody must expose m_Mass"
    );
    scene_save::save_stream(&scene_target, &scene_edited)?;
    let scene_reloaded = unity_yaml::parse_file(&scene_target)?;
    assert_eq!(
        field(&scene_reloaded, SCENE_CAMERA, &["Camera", "m_CullingMask"]),
        field(&scene_original, SCENE_CAMERA, &["Camera", "m_CullingMask"]),
        "a nested camera value the models do not represent must survive the edit and save"
    );
    assert_eq!(
        field(
            &scene_reloaded,
            SCENE_CAMERA,
            &["Camera", "near clip plane"]
        )
        .and_then(Node::as_str),
        field(
            &scene_original,
            SCENE_CAMERA,
            &["Camera", "near clip plane"]
        )
        .and_then(Node::as_str),
        "a camera key with spaces the models do not represent must survive the edit and save"
    );
    Ok(())
}

#[test]
fn real_prefab_variant_model_round_trip_keeps_the_added_game_object_values() -> TestResult {
    let path = sample(VARIANT);
    let original = unity_yaml::parse_file(&path)?;
    let base_guid =
        unitas_asset::meta::parse_meta_file(&sample("Assets/Prefabs/Pickup.prefab.meta"))?;
    assert_eq!(
        field(
            &original,
            VARIANT_INSTANCE,
            &["PrefabInstance", "m_SourcePrefab"]
        )
        .and_then(|node| node.get("guid"))
        .and_then(Node::as_str),
        Some(base_guid.as_str()),
        "the variant must name the base prefab the sample ships"
    );
    let definition =
        PrefabVariantDefinition::load_from_file(&path, base_guid.clone(), "PickupGold")?;
    let target = copy_sample(VARIANT, "variant_model")?;
    definition.save_to_file(&target)?;
    let saved = unity_yaml::parse_file(&target)?;

    assert_eq!(
        field(
            &saved,
            VARIANT_AURA_TRANSFORM,
            &["Transform", "m_LocalPosition"]
        ),
        field(
            &original,
            VARIANT_AURA_TRANSFORM,
            &["Transform", "m_LocalPosition"]
        ),
        "the added game object transform position must survive the model round trip"
    );
    assert_eq!(
        field(
            &saved,
            VARIANT_AURA_TRANSFORM,
            &["Transform", "m_LocalScale"]
        ),
        field(
            &original,
            VARIANT_AURA_TRANSFORM,
            &["Transform", "m_LocalScale"]
        ),
        "the added game object transform scale must survive the model round trip"
    );
    assert_eq!(
        field(
            &saved,
            VARIANT_AURA_RENDERER,
            &["MeshRenderer", "m_Materials"]
        ),
        field(
            &original,
            VARIANT_AURA_RENDERER,
            &["MeshRenderer", "m_Materials"]
        ),
        "the added game object material list must survive the model round trip"
    );
    assert_eq!(
        field(
            &saved,
            VARIANT_AURA_BEHAVIOUR,
            &["MonoBehaviour", "m_Script"]
        ),
        field(
            &original,
            VARIANT_AURA_BEHAVIOUR,
            &["MonoBehaviour", "m_Script"]
        ),
        "the added game object script reference must survive the model round trip"
    );
    assert_eq!(
        field(&saved, VARIANT_AURA, &["GameObject", "m_Name"]).and_then(Node::as_str),
        field(&original, VARIANT_AURA, &["GameObject", "m_Name"]).and_then(Node::as_str),
        "the added game object name must survive the model round trip"
    );
    let reloaded = PrefabVariantDefinition::load_from_file(&target, base_guid, "PickupGold")?;
    assert_eq!(
        reloaded.targeted_modifications, definition.targeted_modifications,
        "the variant overrides must survive the model round trip"
    );
    assert_eq!(
        reloaded.structural_overrides.added_children.len(),
        definition.structural_overrides.added_children.len(),
        "the added game objects must survive the model round trip"
    );
    Ok(())
}

#[test]
fn the_sample_project_carries_no_binary_unity_asset_to_round_trip() -> TestResult {
    let mut checked = 0;
    for relative in [
        SCENE,
        PHYSICS_SCENE,
        PREFAB,
        VARIANT,
        PLAYER_SETTINGS,
        QUALITY_SETTINGS,
        "ProjectSettings/InputManager.asset",
        "ProjectSettings/TimeManager.asset",
        "ProjectSettings/DynamicsManager.asset",
        "Assets/Materials/Gold.mat",
        "Assets/Materials/Pickup.mat",
        "Assets/Materials/Bumper.physicMaterial",
        "Assets/Materials/Frictionless.physicMaterial",
    ] {
        let bytes = fs::read(sample(relative))?;
        assert!(
            binary_ser::is_text_serialized(&bytes),
            "{relative} must be a text serialized asset"
        );
        checked += 1;
    }
    assert_eq!(checked, 13, "every sampled Unity asset must be checked");
    Ok(())
}

#[test]
fn a_scene_with_a_mesh_collider_survives_load_save_and_load() -> TestResult {
    let directory = temp_dir("mesh_collider")?;
    let path = directory.join("MeshColliderScene.unity");
    fs::write(&path, MESH_COLLIDER_SCENE)?;
    let loaded = assemble_scene_file_bundle(&path)?;
    let collider = mesh_collider_of(&loaded)?;
    assert_eq!(collider.mesh_file_id, Some(10202));
    assert_eq!(
        collider.mesh_guid.as_deref(),
        Some("0000000000000000e000000000000000")
    );
    assert_eq!(collider.center, [0.0, 1.0, 0.0]);

    let stream = unity_yaml::parse_file(&path)?;
    let saved = directory.join("SavedMeshColliderScene.unity");
    scene_save::save_stream(&saved, &stream)?;
    assert_eq!(
        fs::read(&saved)?,
        fs::read(&path)?,
        "the saved scene must be byte-identical to the scene on disk"
    );

    let reloaded = assemble_scene_file_bundle(&saved)?;
    let reloaded_collider = mesh_collider_of(&reloaded)?;
    assert_eq!(
        reloaded_collider, collider,
        "the mesh collider must survive load, save and load"
    );
    Ok(())
}

fn mesh_collider_of(
    scene: &unitas_asset::scene::AssembledScene,
) -> Result<unitas_asset::scene::MeshColliderComponent, Box<dyn Error>> {
    let binding = scene
        .physics
        .first()
        .ok_or("the scene must bind a physics body")?;
    match binding.colliders.first() {
        Some(ColliderComponent::Mesh(collider)) => Ok(collider.clone()),
        _ => Err("the scene must carry a mesh collider".into()),
    }
}
