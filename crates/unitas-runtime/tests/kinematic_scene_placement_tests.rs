use std::error::Error;
use std::path::PathBuf;

use unitas_core::EntityId;
use unitas_physics::BodyRole;
use unitas_runtime::Runtime;

const SAMPLE_SCENE_RELATIVE: &str = "../../samples/roll-a-ball/Assets/Scenes/PhysicsLab.unity";
const KINEMATIC_NAME: &str = "KinematicPlatform";
const DYNAMIC_NAME: &str = "DynamicMarble";
const SAMPLE_KINEMATIC_NAME: &str = "RotatingPlatform";
const KINEMATIC_POSITION: [f64; 3] = [-10.0, 0.25, 0.0];
const DYNAMIC_POSITION: [f64; 3] = [-8.5, 1.0, 0.0];
const TOLERANCE: f64 = 1e-6;
const REST_TOLERANCE: f64 = 5e-2;
const TICK_DELTA: f64 = 0.02;
const TICK_COUNT: usize = 60;

fn scene_yaml() -> String {
    String::from(
        r"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  m_Layer: 0
  m_Name: KinematicPlatform
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -10, y: 0.25, z: 0}
  m_LocalScale: {x: 4, y: 0.5, z: 4}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!54 &102
Rigidbody:
  m_GameObject: {fileID: 100}
  m_Mass: 1
  m_UseGravity: 0
  m_IsKinematic: 1
  m_IsTrigger: 0
--- !u!65 &103
BoxCollider:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Size: {x: 4, y: 0.5, z: 4}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &200
GameObject:
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  - component: {fileID: 203}
  m_Layer: 0
  m_Name: DynamicMarble
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -8.5, y: 1, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!54 &202
Rigidbody:
  m_GameObject: {fileID: 200}
  m_Mass: 1
  m_UseGravity: 1
  m_IsKinematic: 0
  m_IsTrigger: 0
--- !u!135 &203
SphereCollider:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Radius: 0.5
  m_Center: {x: 0, y: 0, z: 0}
",
    )
}

fn write_scene() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_kinematic_placement_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let scene_path = temp_dir.join("kinematic_placement.unity");
    std::fs::write(&scene_path, scene_yaml())?;
    Ok((temp_dir, scene_path))
}

fn sample_scene_path() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAMPLE_SCENE_RELATIVE);
    if path.is_file() { Some(path) } else { None }
}

fn entity_handle(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph loaded").into()),
    };
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(entity.id.as_u64());
        }
    }
    Err(String::from("entity not found in scene graph").into())
}

fn graph_translation(runtime: &Runtime, entity: EntityId) -> Result<[f64; 3], Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("no scene graph loaded").into()),
    };
    match graph.transform(entity) {
        Some(transform) => Ok([
            f64::from(transform.translation[0]),
            f64::from(transform.translation[1]),
            f64::from(transform.translation[2]),
        ]),
        None => Err(String::from("entity has no scene graph transform").into()),
    }
}

fn body_position(runtime: &Runtime, entity: EntityId) -> Result<[f64; 3], Box<dyn Error>> {
    match runtime.body_position(entity) {
        Some(position) => Ok(position),
        None => Err(String::from("entity has no physics body").into()),
    }
}

fn assert_vector_close(actual: [f64; 3], expected: [f64; 3], label: &str, tolerance: f64) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= tolerance,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn kinematic_rigidbody_keeps_serialized_transform_and_body_pose_at_scene_load()
-> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene()?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let kinematic = EntityId::new(entity_handle(&runtime, KINEMATIC_NAME)?);
    let dynamic = EntityId::new(entity_handle(&runtime, DYNAMIC_NAME)?);
    assert_eq!(runtime.body_role(kinematic), Some(BodyRole::Kinematic));
    assert_eq!(runtime.body_role(dynamic), Some(BodyRole::Dynamic));

    assert_vector_close(
        graph_translation(&runtime, kinematic)?,
        KINEMATIC_POSITION,
        "kinematic scene graph transform",
        TOLERANCE,
    );
    assert_vector_close(
        body_position(&runtime, kinematic)?,
        KINEMATIC_POSITION,
        "kinematic physics body pose",
        TOLERANCE,
    );
    assert_vector_close(
        graph_translation(&runtime, dynamic)?,
        DYNAMIC_POSITION,
        "dynamic scene graph transform",
        TOLERANCE,
    );
    assert_vector_close(
        body_position(&runtime, dynamic)?,
        DYNAMIC_POSITION,
        "dynamic physics body pose",
        TOLERANCE,
    );
    Ok(())
}

#[test]
fn sample_scene_kinematic_platform_keeps_serialized_transform_and_body_pose()
-> Result<(), Box<dyn Error>> {
    let scene_path = match sample_scene_path() {
        Some(path) => path,
        None => {
            eprintln!(
                "the sample scene is absent at {SAMPLE_SCENE_RELATIVE}: the runtime kinematic \
                 placement fixture covers the same serialized placement"
            );
            return Ok(());
        }
    };
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    let kinematic = EntityId::new(entity_handle(&runtime, SAMPLE_KINEMATIC_NAME)?);
    assert_eq!(runtime.body_role(kinematic), Some(BodyRole::Kinematic));
    assert_vector_close(
        graph_translation(&runtime, kinematic)?,
        KINEMATIC_POSITION,
        "sample kinematic scene graph transform",
        TOLERANCE,
    );
    assert_vector_close(
        body_position(&runtime, kinematic)?,
        KINEMATIC_POSITION,
        "sample kinematic physics body pose",
        TOLERANCE,
    );
    Ok(())
}

#[test]
fn kinematic_platform_at_its_serialized_pose_supports_the_dynamic_marble()
-> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene()?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
    load_res?;

    let kinematic = EntityId::new(entity_handle(&runtime, KINEMATIC_NAME)?);
    let dynamic = EntityId::new(entity_handle(&runtime, DYNAMIC_NAME)?);
    for _ in 0..TICK_COUNT {
        runtime.tick(TICK_DELTA)?;
    }

    assert_vector_close(
        body_position(&runtime, kinematic)?,
        KINEMATIC_POSITION,
        "kinematic body pose after fixed steps",
        TOLERANCE,
    );
    let marble = body_position(&runtime, dynamic)?;
    assert!(
        (marble[1] - DYNAMIC_POSITION[1]).abs() <= REST_TOLERANCE,
        "the marble must rest on the kinematic platform at y = {}, got {marble:?}",
        DYNAMIC_POSITION[1]
    );
    assert!(
        (marble[0] - DYNAMIC_POSITION[0]).abs() <= REST_TOLERANCE,
        "the marble must stay over the kinematic platform at x = {}, got {marble:?}",
        DYNAMIC_POSITION[0]
    );
    Ok(())
}
