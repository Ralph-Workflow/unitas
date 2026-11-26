use std::error::Error;

use unitas_asset::collision::{
    cook_builtin_collision_mesh, cook_collision_mesh, CollisionCookError,
};
use unitas_asset::scene::{assemble_scene_bundle, ColliderComponent};
use unitas_asset::unity_yaml;

type TestResult = Result<(), Box<dyn Error>>;

const BUILTIN_MESH_GUID: &str = "0000000000000000e000000000000000";
const BUILTIN_CUBE_FILE_ID: i64 = 10202;
const BUILTIN_SPHERE_FILE_ID: i64 = 10207;
const MODEL_SUB_ASSET_FILE_ID: i64 = 4300000;

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

const COMPOUND_COLLIDER_SCENE: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Name: Compound
  m_Layer: 0
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_Children: []
--- !u!65 &3
BoxCollider:
  m_GameObject: {fileID: 1}
  m_Size: {x: 2, y: 1, z: 1}
  m_Center: {x: 0, y: 0, z: 0}
  m_IsTrigger: 0
  m_Material: {fileID: 0}
--- !u!135 &4
SphereCollider:
  m_GameObject: {fileID: 1}
  m_Radius: 0.5
  m_Center: {x: 1, y: 0, z: 0}
  m_IsTrigger: 1
  m_Material: {fileID: 0}
--- !u!64 &5
MeshCollider:
  m_GameObject: {fileID: 1}
  m_Mesh: {fileID: 10207, guid: 0000000000000000e000000000000000, type: 3}
  m_Convex: 1
  m_IsTrigger: 0
  m_Center: {x: 0, y: 1, z: 0}
  m_Material: {fileID: 0}
--- !u!1 &6
GameObject:
  m_Name: Solo
  m_Layer: 0
--- !u!4 &7
Transform:
  m_GameObject: {fileID: 6}
  m_Father: {fileID: 0}
  m_Children: []
--- !u!65 &8
BoxCollider:
  m_GameObject: {fileID: 6}
  m_Size: {x: 1, y: 1, z: 1}
  m_Center: {x: 0, y: 0, z: 0}
  m_IsTrigger: 0
  m_Material: {fileID: 0}
";

const TRIANGLE_OBJ: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

#[test]
fn mesh_collider_resolves_its_named_mesh_and_cooks_collision_geometry() -> TestResult {
    let stream = unity_yaml::parse(MESH_COLLIDER_SCENE)?;
    let assembled = assemble_scene_bundle(&stream)?;
    assert!(
        assembled.diagnostics.is_empty(),
        "a resolved mesh collider must not raise diagnostics: {:?}",
        assembled.diagnostics
    );
    let binding = assembled
        .physics
        .first()
        .ok_or("the mesh collider must bind a physics body")?;
    assert_eq!(binding.colliders.len(), 1, "the frame carries one collider");
    let collider = match &binding.colliders[0] {
        ColliderComponent::Mesh(collider) => collider,
        _ => return Err("the frame must carry a mesh collider".into()),
    };
    assert_eq!(collider.mesh_guid.as_deref(), Some(BUILTIN_MESH_GUID));
    assert_eq!(collider.mesh_file_id, Some(BUILTIN_CUBE_FILE_ID));
    assert!(
        collider.mesh_path.is_none(),
        "a built-in mesh names no asset on disk"
    );
    assert_eq!(collider.mesh_diagnostic, None);
    assert!(!collider.convex);
    assert!(!collider.is_trigger);
    assert_eq!(collider.center, [0.0, 1.0, 0.0]);
    assert_eq!(collider.material, None);

    let cooked = cook_builtin_collision_mesh(BUILTIN_CUBE_FILE_ID, BUILTIN_MESH_GUID)
        .ok_or("the built-in cube must cook into collision geometry")?;
    assert_eq!(cooked.vertices.len(), 24, "the cube keeps its own vertices");
    assert_eq!(
        cooked.indices.len(),
        36,
        "the cube keeps its twelve triangles"
    );
    assert_eq!(
        cooked
            .vertices
            .iter()
            .flatten()
            .fold(0.0f32, |extent, value| extent.max(*value)),
        0.5,
        "the built-in cube is one unit wide"
    );
    Ok(())
}

#[test]
fn a_game_object_keeps_every_one_of_its_colliders() -> TestResult {
    let stream = unity_yaml::parse(COMPOUND_COLLIDER_SCENE)?;
    let assembled = assemble_scene_bundle(&stream)?;
    let compound = assembled
        .physics
        .iter()
        .find(|binding| {
            assembled
                .graph
                .entity(binding.entity)
                .is_some_and(|entity| entity.name == "Compound")
        })
        .ok_or("the compound object must bind a physics body")?;
    assert_eq!(
        compound.colliders.len(),
        3,
        "the compound keeps every collider"
    );
    match &compound.colliders[0] {
        ColliderComponent::Box(collider) => {
            assert_eq!(collider.size, [2.0, 1.0, 1.0]);
            assert_eq!(collider.center, [0.0, 0.0, 0.0]);
            assert!(!collider.is_trigger);
        }
        _ => return Err("the first collider must be the box".into()),
    }
    match &compound.colliders[1] {
        ColliderComponent::Sphere(collider) => {
            assert_eq!(collider.radius, 0.5);
            assert_eq!(collider.center, [1.0, 0.0, 0.0]);
            assert!(collider.is_trigger);
        }
        _ => return Err("the second collider must be the sphere".into()),
    }
    match &compound.colliders[2] {
        ColliderComponent::Mesh(collider) => {
            assert_eq!(collider.mesh_file_id, Some(BUILTIN_SPHERE_FILE_ID));
            assert!(collider.convex);
            assert_eq!(collider.center, [0.0, 1.0, 0.0]);
        }
        _ => return Err("the third collider must be the mesh".into()),
    }
    let solo = assembled
        .physics
        .iter()
        .find(|binding| {
            assembled
                .graph
                .entity(binding.entity)
                .is_some_and(|entity| entity.name == "Solo")
        })
        .ok_or("the solo object must bind a physics body")?;
    assert_eq!(
        solo.colliders.len(),
        1,
        "another object's colliders stay its own"
    );
    Ok(())
}

#[test]
fn the_built_in_sphere_cooks_at_unitys_half_radius() -> TestResult {
    let cooked = cook_builtin_collision_mesh(BUILTIN_SPHERE_FILE_ID, BUILTIN_MESH_GUID)
        .ok_or("the built-in sphere must cook into collision geometry")?;
    assert_eq!(
        cooked
            .vertices
            .iter()
            .flatten()
            .fold(0.0f32, |extent, value| extent.max(*value)),
        0.5,
        "Unity's built-in sphere has radius 0.5"
    );
    assert!(
        cooked.vertices.len() > 8,
        "the sphere keeps its own vertices, not its bounds"
    );
    Ok(())
}

#[test]
fn a_model_mesh_cooks_into_collision_geometry() -> TestResult {
    let cooked = cook_collision_mesh(
        TRIANGLE_OBJ.as_bytes(),
        "Frame.obj",
        Some(MODEL_SUB_ASSET_FILE_ID),
    )?;
    assert_eq!(cooked.vertices.len(), 3);
    assert_eq!(cooked.indices, vec![0, 1, 2]);
    let fallback = cook_collision_mesh(
        TRIANGLE_OBJ.as_bytes(),
        "Frame.obj",
        Some(MODEL_SUB_ASSET_FILE_ID + 2),
    )?;
    assert_eq!(
        fallback, cooked,
        "an unknown sub-asset falls back to the first mesh"
    );
    let error = cook_collision_mesh(b"not a model", "Frame.txt", None)
        .err()
        .ok_or("an unsupported model file must not cook")?;
    assert!(
        matches!(error, CollisionCookError::Import { .. }),
        "an unreadable model must report an import error: {error:?}"
    );
    Ok(())
}

#[path = "asset_import/audio_import_tests.rs"]
mod audio_import_tests;
#[path = "asset_import/binary_ser_tests.rs"]
mod binary_ser_tests;
#[path = "asset_import/database_tests.rs"]
mod database_tests;
#[path = "asset_import/dependencies_tests.rs"]
mod dependencies_tests;
#[path = "asset_import/fbx_tests.rs"]
mod fbx_tests;
#[path = "asset_import/import_cache_tests.rs"]
mod import_cache_tests;
#[path = "asset_import/import_pool_tests.rs"]
mod import_pool_tests;
#[path = "asset_import/inflate_tests.rs"]
mod inflate_tests;
#[path = "asset_import/json_tests.rs"]
mod json_tests;
#[path = "asset_import/model_tests.rs"]
mod model_tests;
#[path = "asset_import/packages_tests.rs"]
mod packages_tests;
#[path = "asset_import/particle_tests.rs"]
mod particle_tests;
#[path = "asset_import/perf_import.rs"]
mod perf_import;
#[path = "asset_import/perf_scene_load.rs"]
mod perf_scene_load;
#[path = "asset_import/prefab_variant_tests.rs"]
mod prefab_variant_tests;
#[path = "asset_import/primitive_tests.rs"]
mod primitive_tests;
#[path = "asset_import/project_tests.rs"]
mod project_tests;
#[path = "asset_import/roll_a_ball_asset_breadth_tests.rs"]
mod roll_a_ball_asset_breadth_tests;
#[path = "asset_import/roll_a_ball_project_tests.rs"]
mod roll_a_ball_project_tests;
#[path = "asset_import/scene_particle_trail_tests.rs"]
mod scene_particle_trail_tests;
#[path = "asset_import/scene_save_tests.rs"]
mod scene_save_tests;
#[path = "asset_import/texture_tests.rs"]
mod texture_tests;
#[path = "asset_import/trail_tests.rs"]
mod trail_tests;
#[path = "asset_import/trust_tests.rs"]
mod trust_tests;
