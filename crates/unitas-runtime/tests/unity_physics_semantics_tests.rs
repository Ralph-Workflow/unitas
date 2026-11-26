use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use unitas_core::EntityId;
use unitas_runtime::{Callbacks, Runtime};

type TestResult = Result<(), Box<dyn Error>>;

const FIXED_STEP: f64 = 0.02;
const IMPULSE: [f64; 3] = [2.0, 0.0, 0.0];
const DRAG_SPEED: [f64; 3] = [10.0, 0.0, 0.0];
const SPIN_SPEED: [f64; 3] = [0.0, 10.0, 0.0];
const MASS_EXPECTED_VELOCITY: f64 = 0.5;
const VELOCITY_TOLERANCE: f64 = 1e-6;

static LAYER_COLLISIONS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static PAYLOAD_COLLISIONS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());
static TRIGGER_EVENTS: Mutex<Vec<(u64, u64, u32)>> = Mutex::new(Vec::new());

extern "C" fn record_layer_collision(first: u64, second: u64, phase: u32) {
    if let Ok(mut events) = LAYER_COLLISIONS.lock() {
        events.push((first, second, phase));
    }
}

extern "C" fn record_payload_collision(first: u64, second: u64, phase: u32) {
    if let Ok(mut events) = PAYLOAD_COLLISIONS.lock() {
        events.push((first, second, phase));
    }
}

extern "C" fn record_trigger(first: u64, second: u64, phase: u32) {
    if let Ok(mut events) = TRIGGER_EVENTS.lock() {
        events.push((first, second, phase));
    }
}

extern "C" fn noop_create_script(_entity: u64, _guid_hi: u64, _guid_lo: u64) -> u64 {
    1
}

extern "C" fn noop_handle(_handle: u64) {}

extern "C" fn noop_handle_delta(_handle: u64, _delta: f64) {}

fn callbacks(
    collision: extern "C" fn(u64, u64, u32),
    trigger: extern "C" fn(u64, u64, u32),
) -> Callbacks {
    Callbacks {
        create_script: noop_create_script,
        awake: noop_handle,
        on_enable: noop_handle,
        start: noop_handle,
        fixed_update: noop_handle_delta,
        update: noop_handle_delta,
        late_update: noop_handle_delta,
        on_disable: noop_handle,
        destroy: noop_handle,
        collision,
        trigger,
    }
}

struct Rigidbody {
    mass: f64,
    drag: f64,
    angular_drag: f64,
    use_gravity: bool,
    is_kinematic: bool,
    interpolate: u32,
    collision_detection: u32,
}

enum Collider {
    Sphere { radius: f32, is_trigger: bool },
    Cube { size: [f32; 3], is_trigger: bool },
}

struct Object {
    name: &'static str,
    layer: u32,
    position: [f32; 3],
    rigidbody: Option<Rigidbody>,
    collider: Collider,
}

fn dynamic(mass: f64, drag: f64, angular_drag: f64, interpolate: u32, detection: u32) -> Rigidbody {
    Rigidbody {
        mass,
        drag,
        angular_drag,
        use_gravity: false,
        is_kinematic: false,
        interpolate,
        collision_detection: detection,
    }
}

fn write_scene(path: &Path, objects: &[Object]) -> TestResult {
    let mut yaml = String::from("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n");
    let mut next_id = 1_000_000_u64;
    for object in objects {
        let game_object_id = next_id;
        let transform_id = next_id + 1;
        let rigidbody_id = next_id + 2;
        let collider_id = next_id + 3;
        next_id += 4;
        let name = object.name;
        let layer = object.layer;
        let position = object.position;
        yaml.push_str(&format!("--- !u!1 &{game_object_id}\n"));
        yaml.push_str("GameObject:\n");
        yaml.push_str("  m_ObjectHideFlags: 0\n");
        yaml.push_str("  m_CorrespondingSourceObject: {fileID: 0}\n");
        yaml.push_str("  m_PrefabInstance: {fileID: 0}\n");
        yaml.push_str("  m_PrefabAsset: {fileID: 0}\n");
        yaml.push_str("  serializedVersion: 6\n");
        yaml.push_str("  m_Component:\n");
        yaml.push_str(&format!("  - component: {{fileID: {transform_id}}}\n"));
        if object.rigidbody.is_some() {
            yaml.push_str(&format!("  - component: {{fileID: {rigidbody_id}}}\n"));
        }
        yaml.push_str(&format!("  - component: {{fileID: {collider_id}}}\n"));
        yaml.push_str(&format!("  m_Layer: {layer}\n"));
        yaml.push_str(&format!("  m_Name: {name}\n"));
        yaml.push_str("  m_TagString: Untagged\n");
        yaml.push_str("  m_Icon: {fileID: 0}\n");
        yaml.push_str("  m_NavMeshLayer: 0\n");
        yaml.push_str("  m_StaticEditorFlags: 0\n");
        yaml.push_str("  m_IsActive: 1\n");
        yaml.push_str(&format!("--- !u!4 &{transform_id}\n"));
        yaml.push_str("Transform:\n");
        yaml.push_str("  m_ObjectHideFlags: 0\n");
        yaml.push_str("  m_CorrespondingSourceObject: {fileID: 0}\n");
        yaml.push_str("  m_PrefabInstance: {fileID: 0}\n");
        yaml.push_str("  m_PrefabAsset: {fileID: 0}\n");
        yaml.push_str(&format!("  m_GameObject: {{fileID: {game_object_id}}}\n"));
        yaml.push_str("  serializedVersion: 2\n");
        yaml.push_str("  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n");
        yaml.push_str(&format!(
            "  m_LocalPosition: {{x: {}, y: {}, z: {}}}\n",
            position[0], position[1], position[2]
        ));
        yaml.push_str("  m_LocalScale: {x: 1, y: 1, z: 1}\n");
        yaml.push_str("  m_Children: []\n");
        yaml.push_str("  m_Father: {fileID: 0}\n");
        if let Some(rigidbody) = &object.rigidbody {
            let mass = rigidbody.mass;
            let drag = rigidbody.drag;
            let angular_drag = rigidbody.angular_drag;
            let use_gravity = u8::from(rigidbody.use_gravity);
            let is_kinematic = u8::from(rigidbody.is_kinematic);
            let interpolate = rigidbody.interpolate;
            let collision_detection = rigidbody.collision_detection;
            yaml.push_str(&format!("--- !u!54 &{rigidbody_id}\n"));
            yaml.push_str("Rigidbody:\n");
            yaml.push_str("  m_ObjectHideFlags: 0\n");
            yaml.push_str("  m_CorrespondingSourceObject: {fileID: 0}\n");
            yaml.push_str("  m_PrefabInstance: {fileID: 0}\n");
            yaml.push_str("  m_PrefabAsset: {fileID: 0}\n");
            yaml.push_str(&format!("  m_GameObject: {{fileID: {game_object_id}}}\n"));
            yaml.push_str(&format!("  m_Mass: {mass}\n"));
            yaml.push_str(&format!("  m_Drag: {drag}\n"));
            yaml.push_str(&format!("  m_AngularDrag: {angular_drag}\n"));
            yaml.push_str(&format!("  m_UseGravity: {use_gravity}\n"));
            yaml.push_str(&format!("  m_IsKinematic: {is_kinematic}\n"));
            yaml.push_str(&format!("  m_Interpolate: {interpolate}\n"));
            yaml.push_str(&format!("  m_CollisionDetection: {collision_detection}\n"));
        }
        yaml.push_str(&format!("--- !u!{class} &{collider_id}\n", class = collider_class(&object.collider)));
        yaml.push_str(&format!(
            "{}:\n",
            match &object.collider {
                Collider::Sphere { .. } => "SphereCollider",
                Collider::Cube { .. } => "BoxCollider",
            }
        ));
        yaml.push_str("  m_ObjectHideFlags: 0\n");
        yaml.push_str("  m_CorrespondingSourceObject: {fileID: 0}\n");
        yaml.push_str("  m_PrefabInstance: {fileID: 0}\n");
        yaml.push_str("  m_PrefabAsset: {fileID: 0}\n");
        yaml.push_str(&format!("  m_GameObject: {{fileID: {game_object_id}}}\n"));
        match &object.collider {
            Collider::Sphere { radius, is_trigger } => {
                let radius = *radius;
                let is_trigger = u8::from(*is_trigger);
                yaml.push_str(&format!("  m_IsTrigger: {is_trigger}\n"));
                yaml.push_str(&format!("  m_Radius: {radius}\n"));
                yaml.push_str("  m_Center: {x: 0, y: 0, z: 0}\n");
            }
            Collider::Cube { size, is_trigger } => {
                let size = *size;
                let is_trigger = u8::from(*is_trigger);
                yaml.push_str(&format!("  m_IsTrigger: {is_trigger}\n"));
                yaml.push_str(&format!(
                    "  m_Size: {{x: {}, y: {}, z: {}}}\n",
                    size[0], size[1], size[2]
                ));
                yaml.push_str("  m_Center: {x: 0, y: 0, z: 0}\n");
            }
        }
    }
    fs::write(path, yaml)?;
    Ok(())
}

fn collider_class(collider: &Collider) -> u32 {
    match collider {
        Collider::Sphere { .. } => 135,
        Collider::Cube { .. } => 65,
    }
}

fn layer_collision_matrix_hex() -> String {
    let mut text = String::with_capacity(256);
    for layer in 0..32_u32 {
        let mut mask = u32::MAX;
        if layer == 0 {
            mask &= !(1 << 1);
        }
        if layer == 1 {
            mask &= !(1 << 0);
        }
        text.push_str(&format!("{:08x}", mask.swap_bytes()));
    }
    text
}

fn write_physics_settings(dir: &Path) -> TestResult {
    let mut yaml = String::from("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!55 &1\n");
    yaml.push_str("PhysicsManager:\n");
    yaml.push_str("  m_ObjectHideFlags: 0\n");
    yaml.push_str("  serializedVersion: 13\n");
    yaml.push_str("  m_Gravity: {x: 0, y: 0, z: 0}\n");
    yaml.push_str("  m_DefaultMaxAngularSpeed: 7\n");
    yaml.push_str(&format!(
        "  m_LayerCollisionMatrix: {}\n",
        layer_collision_matrix_hex()
    ));
    fs::write(dir.join("ProjectSettings").join("DynamicsManager.asset"), yaml)?;
    Ok(())
}

fn project_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir = std::env::temp_dir().join(format!("unitas-u7-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Assets").join("Scenes"))?;
    fs::create_dir_all(dir.join("ProjectSettings"))?;
    Ok(dir)
}

fn load_scene(dir: &Path, scene: &str, objects: &[Object]) -> Result<Runtime, Box<dyn Error>> {
    let path = dir.join("Assets").join("Scenes").join(scene);
    write_scene(&path, objects)?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&path)?;
    Ok(runtime)
}

fn entity_id(runtime: &Runtime, name: &str) -> Result<EntityId, Box<dyn Error>> {
    let graph = match runtime.scene_graph() {
        Some(graph) => graph,
        None => return Err(String::from("runtime exposes no scene graph").into()),
    };
    graph
        .iter_breadth_first()
        .find(|id| graph.entity(*id).is_some_and(|entity| entity.name == name))
        .ok_or_else(|| format!("scene has no entity named {name}").into())
}

fn tick(runtime: &mut Runtime, count: usize) -> TestResult {
    for _ in 0..count {
        runtime.tick(FIXED_STEP)?;
    }
    Ok(())
}

fn recorded(events: &Mutex<Vec<(u64, u64, u32)>>) -> Vec<(u64, u64, u32)> {
    match events.lock() {
        Ok(events) => events.clone(),
        Err(_) => Vec::new(),
    }
}

fn collides(first: u64, second: u64, events: &[(u64, u64, u32)]) -> bool {
    events
        .iter()
        .any(|(left, right, _)| (*left == first && *right == second) || (*left == second && *right == first))
}

#[test]
fn scene_authored_rigidbody_reaches_the_world_with_its_physics_settings() -> TestResult {
    let dir = project_dir("authored")?;
    write_physics_settings(&dir)?;
    let mut runtime = load_scene(
        &dir,
        "Authored.unity",
        &[
            Object {
                name: "Authored",
                layer: 0,
                position: [0.0, 0.0, 0.0],
                rigidbody: Some(dynamic(4.0, 0.0, 0.0, 1, 1)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
            Object {
                name: "Damped",
                layer: 0,
                position: [0.0, 0.0, 50.0],
                rigidbody: Some(dynamic(4.0, 2.0, 2.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
        ],
    )?;
    let authored = entity_id(&runtime, "Authored")?;
    let damped = entity_id(&runtime, "Damped")?;

    let mode = runtime
        .physics_mut()
        .world_3d()
        .collision_detection_mode(authored)
        .map(|mode| format!("{mode:?}"));
    assert_eq!(mode.as_deref(), Some("Continuous"));

    runtime
        .physics_mut()
        .world_3d_mut()
        .add_impulse(authored, IMPULSE)?;
    tick(&mut runtime, 2)?;
    let velocity = runtime
        .body_velocity(authored)
        .ok_or_else(|| String::from("authored body has no velocity"))?;
    assert!(
        (velocity[0] - MASS_EXPECTED_VELOCITY).abs() < VELOCITY_TOLERANCE,
        "expected the authored mass of 4 to scale the impulse to {MASS_EXPECTED_VELOCITY}, got {velocity:?}"
    );

    runtime.set_body_velocity(authored, DRAG_SPEED)?;
    runtime.set_body_velocity(damped, DRAG_SPEED)?;
    runtime
        .physics_mut()
        .world_3d_mut()
        .set_angular_velocity(authored, SPIN_SPEED)?;
    runtime
        .physics_mut()
        .world_3d_mut()
        .set_angular_velocity(damped, SPIN_SPEED)?;
    tick(&mut runtime, 2)?;

    let authored_position = runtime
        .body_position(authored)
        .ok_or_else(|| String::from("authored body has no position"))?;
    let damped_position = runtime
        .body_position(damped)
        .ok_or_else(|| String::from("damped body has no position"))?;
    let authored_pose = runtime
        .physics_mut()
        .presented_pose_3d(authored)
        .ok_or_else(|| String::from("authored body has no presented pose"))?;
    let damped_pose = runtime
        .physics_mut()
        .presented_pose_3d(damped)
        .ok_or_else(|| String::from("damped body has no presented pose"))?;
    assert_ne!(
        authored_pose.0, authored_position,
        "expected the authored interpolation to present a pose between the previous and current transform"
    );
    assert_eq!(
        damped_pose.0, damped_position,
        "expected the body without interpolation to present its current transform"
    );

    tick(&mut runtime, 60)?;
    let authored_velocity = runtime
        .body_velocity(authored)
        .ok_or_else(|| String::from("authored body has no velocity"))?;
    let damped_velocity = runtime
        .body_velocity(damped)
        .ok_or_else(|| String::from("damped body has no velocity"))?;
    assert!(
        authored_velocity[0] > 9.9,
        "expected the body without drag to keep its speed, got {authored_velocity:?}"
    );
    assert!(
        damped_velocity[0] < 9.0,
        "expected the authored drag to slow the body, got {damped_velocity:?}"
    );
    let authored_spin = runtime
        .physics_mut()
        .world_3d()
        .angular_velocity(authored)
        .ok_or_else(|| String::from("authored body has no angular velocity"))?;
    let damped_spin = runtime
        .physics_mut()
        .world_3d()
        .angular_velocity(damped)
        .ok_or_else(|| String::from("damped body has no angular velocity"))?;
    assert!(
        authored_spin[1] > 9.9,
        "expected the body without angular drag to keep its spin, got {authored_spin:?}"
    );
    println!("SPIN authored {authored_spin:?} damped {damped_spin:?}");
    println!("DRAG authored {authored_velocity:?} damped {damped_velocity:?}");
    assert!(
        damped_spin[1] < 9.0,
        "expected the authored angular drag to slow the spin, got {damped_spin:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn layer_pair_separated_by_the_project_collision_matrix_does_not_collide() -> TestResult {
    let dir = project_dir("layers")?;
    write_physics_settings(&dir)?;
    let mut runtime = load_scene(
        &dir,
        "Layers.unity",
        &[
            Object {
                name: "LayerZero",
                layer: 0,
                position: [0.0, 0.0, 0.0],
                rigidbody: Some(dynamic(1.0, 0.0, 0.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
            Object {
                name: "LayerOne",
                layer: 1,
                position: [2.0, 0.0, 0.0],
                rigidbody: None,
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: false,
                },
            },
            Object {
                name: "LayerZeroControl",
                layer: 0,
                position: [0.0, 0.0, 10.0],
                rigidbody: Some(dynamic(1.0, 0.0, 0.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
            Object {
                name: "LayerTwo",
                layer: 2,
                position: [2.0, 0.0, 10.0],
                rigidbody: None,
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: false,
                },
            },
        ],
    )?;
    runtime.set_callbacks(callbacks(record_layer_collision, record_trigger));
    let layer_zero = entity_id(&runtime, "LayerZero")?;
    let layer_one = entity_id(&runtime, "LayerOne")?;
    let control = entity_id(&runtime, "LayerZeroControl")?;
    let layer_two = entity_id(&runtime, "LayerTwo")?;
    runtime.set_body_velocity(layer_zero, [5.0, 0.0, 0.0])?;
    runtime.set_body_velocity(control, [5.0, 0.0, 0.0])?;
    tick(&mut runtime, 40)?;
    let events = recorded(&LAYER_COLLISIONS);
    assert!(
        !collides(layer_zero.as_u64(), layer_one.as_u64(), &events),
        "expected the collision matrix to separate layers 0 and 1, got {events:?}"
    );
    assert!(
        collides(control.as_u64(), layer_two.as_u64(), &events),
        "expected the collision matrix to keep layers 0 and 2 colliding, got {events:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn collision_callback_receives_a_payload_with_impulse_and_contacts() -> TestResult {
    let dir = project_dir("payload")?;
    write_physics_settings(&dir)?;
    let mut runtime = load_scene(
        &dir,
        "Payload.unity",
        &[
            Object {
                name: "PayloadBall",
                layer: 0,
                position: [0.0, 0.0, 0.0],
                rigidbody: Some(dynamic(1.0, 0.0, 0.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
            Object {
                name: "PayloadWall",
                layer: 0,
                position: [2.0, 0.0, 0.0],
                rigidbody: None,
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: false,
                },
            },
        ],
    )?;
    runtime.set_callbacks(callbacks(record_payload_collision, record_trigger));
    let ball = entity_id(&runtime, "PayloadBall")?;
    runtime.set_body_velocity(ball, [5.0, 0.0, 0.0])?;
    let mut hit = None;
    let mut payload_impulse = 0.0;
    let mut payload_contacts = 0_usize;
    let mut contact_impulse = 0.0;
    for _ in 0..40 {
        runtime.tick(FIXED_STEP)?;
        let events = recorded(&PAYLOAD_COLLISIONS);
        if let Some(event) = events.first() {
            hit = Some((event.0, event.1));
        }
        let Some((first, second)) = hit else {
            continue;
        };
        let Some(payload) = runtime.collision_payload(first, second) else {
            continue;
        };
        payload_impulse = payload.total_impulse;
        payload_contacts = payload.contacts.len();
        contact_impulse = payload
            .contacts
            .iter()
            .map(|contact| contact.impulse)
            .fold(0.0, f64::max);
        if payload_impulse > 0.0 {
            break;
        }
    }
    assert!(
        hit.is_some(),
        "expected the ball to collide with the wall it was aimed at"
    );
    assert!(
        payload_impulse > 0.0,
        "expected a non-zero collision impulse, got {payload_impulse}"
    );
    assert!(
        payload_contacts >= 1,
        "expected at least one contact point in the collision payload, got {payload_contacts}"
    );
    assert!(
        contact_impulse > 0.0,
        "expected a contact point with a non-zero impulse, got {contact_impulse}"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn move_position_on_a_kinematic_body_pushes_a_dynamic_body() -> TestResult {
    let dir = project_dir("move")?;
    write_physics_settings(&dir)?;
    let mut runtime = load_scene(
        &dir,
        "Move.unity",
        &[
            Object {
                name: "Mover",
                layer: 0,
                position: [0.0, 0.0, 0.0],
                rigidbody: Some(Rigidbody {
                    mass: 1.0,
                    drag: 0.0,
                    angular_drag: 0.0,
                    use_gravity: false,
                    is_kinematic: true,
                    interpolate: 0,
                    collision_detection: 0,
                }),
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: false,
                },
            },
            Object {
                name: "Pushed",
                layer: 0,
                position: [2.4, 0.0, 0.0],
                rigidbody: Some(dynamic(1.0, 0.0, 0.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: false,
                },
            },
        ],
    )?;
    let mover = entity_id(&runtime, "Mover")?;
    let pushed = entity_id(&runtime, "Pushed")?;
    let before = runtime
        .body_position(pushed)
        .ok_or_else(|| String::from("the pushed body has no position"))?;
    runtime.move_body_position(mover, [2.0, 0.0, 0.0])?;
    tick(&mut runtime, 4)?;
    let after = runtime
        .body_position(pushed)
        .ok_or_else(|| String::from("the pushed body has no position"))?;
    let velocity = runtime
        .body_velocity(pushed)
        .ok_or_else(|| String::from("the pushed body has no velocity"))?;
    let moved = (after[0] - before[0]).abs() + (after[1] - before[1]).abs() + (after[2] - before[2]).abs();
    let speed = (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]).sqrt();
    println!("MOVE before {before:?} after {after:?} velocity {velocity:?}");
    assert!(
        speed > 0.0,
        "expected the kinematic move to push the dynamic body, got velocity {velocity:?}"
    );
    assert!(
        moved > 0.0,
        "expected the kinematic move to displace the dynamic body, got {before:?} then {after:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn trigger_pair_without_a_rigidbody_produces_no_trigger_event() -> TestResult {
    let dir = project_dir("trigger")?;
    write_physics_settings(&dir)?;
    let mut runtime = load_scene(
        &dir,
        "Trigger.unity",
        &[
            Object {
                name: "TriggerBox",
                layer: 0,
                position: [0.0, 0.0, 0.0],
                rigidbody: None,
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: true,
                },
            },
            Object {
                name: "TriggerSphere",
                layer: 0,
                position: [0.5, 0.0, 0.0],
                rigidbody: None,
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: true,
                },
            },
            Object {
                name: "TriggerBody",
                layer: 0,
                position: [10.0, 0.0, 0.0],
                rigidbody: Some(dynamic(1.0, 0.0, 0.0, 0, 0)),
                collider: Collider::Sphere {
                    radius: 0.5,
                    is_trigger: true,
                },
            },
            Object {
                name: "TriggerTarget",
                layer: 0,
                position: [10.5, 0.0, 0.0],
                rigidbody: None,
                collider: Collider::Cube {
                    size: [1.0, 1.0, 1.0],
                    is_trigger: true,
                },
            },
        ],
    )?;
    runtime.set_callbacks(callbacks(record_payload_collision, record_trigger));
    let trigger_box = entity_id(&runtime, "TriggerBox")?;
    let trigger_sphere = entity_id(&runtime, "TriggerSphere")?;
    let trigger_body = entity_id(&runtime, "TriggerBody")?;
    let trigger_target = entity_id(&runtime, "TriggerTarget")?;
    tick(&mut runtime, 10)?;
    let events = recorded(&TRIGGER_EVENTS);
    assert!(
        !collides(
            trigger_box.as_u64(),
            trigger_sphere.as_u64(),
            &events
        ),
        "expected no trigger event without a rigidbody, got {events:?}"
    );
    assert!(
        collides(trigger_body.as_u64(), trigger_target.as_u64(), &events),
        "expected a trigger event once one of the pair carries a rigidbody, got {events:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
