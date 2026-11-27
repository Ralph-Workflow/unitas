use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_core::EntityId;
use unitas_physics::two_d::{BodyDesc2D, ColliderDesc2D, Effector2D};
use unitas_runtime::Runtime;
use unitas_runtime::physics2d_colliders::attach_scene_physics_2d;

fn write_temp_scene(name: &str, yaml: &str) -> Result<PathBuf, Box<dyn Error>> {
    let dir =
        std::env::temp_dir().join(format!("unitas_u1_effectors_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.unity"));
    std::fs::write(&path, yaml)?;
    Ok(path)
}

#[test]
fn physics2d_scene_platform_effector_2d_allows_upward_pass_through_and_supports_falling_body()
-> Result<(), Box<dyn Error>> {
    let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: OneWayBridge
  m_Layer: 8
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &102
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 0.4}
--- !u!251 &103
PlatformEffector2D:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_UseOneWay: 1
  m_SurfaceArc: 180
  m_RotationalOffset: 0
  m_SideArc: 0
  m_UseSideFriction: 0
  m_UseSideBounce: 0
";

    let scene_path = write_temp_scene("one_way_bridge", scene_yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_file(&scene_path);
    load_res?;

    let jumper_entity = EntityId::new(900);
    let jumper_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(jumper_entity, [0.0, -1.2]).with_velocity([0.0, 12.0]));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(jumper_entity, [0.3, 0.3]).with_attached_body(jumper_body),
    );

    let mut reached_above = false;
    for _ in 0..30 {
        runtime.physics_mut().step_2d(0.02);
        let y = runtime
            .physics_mut()
            .body_position_2d(jumper_entity)
            .ok_or("missing jumper position")?[1];
        if y > 0.6 {
            reached_above = true;
            break;
        }
    }
    assert!(
        reached_above,
        "dynamic Rigidbody2D moving upward from below must pass through one-way PlatformEffector2D"
    );

    for _ in 0..120 {
        runtime.physics_mut().step_2d(0.02);
    }
    let rest_y = runtime
        .physics_mut()
        .body_position_2d(jumper_entity)
        .ok_or("missing jumper position after landing")?[1];
    assert!(
        (rest_y - 0.5).abs() < 0.12,
        "dynamic Rigidbody2D falling from above must collide and rest on top of platform (~0.5), got {rest_y}"
    );
    Ok(())
}

#[test]
fn physics2d_scene_surface_effector_2d_drives_conveyor_velocity_and_skips_when_unused_or_disabled()
-> Result<(), Box<dyn Error>> {
    let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &200
GameObject:
  m_Name: ActiveConveyor
  m_Layer: 6
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &202
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 0.4}
--- !u!252 &203
SurfaceEffector2D:
  m_GameObject: {fileID: 200}
  m_Enabled: 1
  m_Speed: 4.0
  m_ForceScale: 0.5
  m_UseFriction: 1
  m_UseBounce: 0
--- !u!1 &300
GameObject:
  m_Name: UnusedByEffectorLedge
  m_Layer: 6
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 20, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &302
BoxCollider2D:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 0
  m_UsedByEffector: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 0.4}
--- !u!252 &303
SurfaceEffector2D:
  m_GameObject: {fileID: 300}
  m_Enabled: 1
  m_Speed: 4.0
  m_ForceScale: 0.5
--- !u!1 &400
GameObject:
  m_Name: DisabledEffectorLedge
  m_Layer: 6
--- !u!4 &401
Transform:
  m_GameObject: {fileID: 400}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 40, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &402
BoxCollider2D:
  m_GameObject: {fileID: 400}
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 0.4}
--- !u!252 &403
SurfaceEffector2D:
  m_GameObject: {fileID: 400}
  m_Enabled: 0
  m_Speed: 4.0
  m_ForceScale: 0.5
";

    let stream = unitas_asset::unity_yaml::parse(scene_yaml)?;
    let assembled = unitas_asset::scene::assemble_scene_bundle(&stream)?;

    let mut runtime = Runtime::new()?;
    attach_scene_physics_2d(
        runtime.physics_mut(),
        &assembled.graph,
        &assembled.physics_2d,
        0,
    );

    let active_cols = runtime
        .physics_mut()
        .collider_ids_2d(EntityId::new(0))
        .to_vec();
    let unused_cols = runtime
        .physics_mut()
        .collider_ids_2d(EntityId::new(1))
        .to_vec();
    let disabled_cols = runtime
        .physics_mut()
        .collider_ids_2d(EntityId::new(2))
        .to_vec();
    assert_eq!(active_cols.len(), 1);
    assert_eq!(unused_cols.len(), 1);
    assert_eq!(disabled_cols.len(), 1);

    assert!(matches!(
        runtime.physics_mut().world_2d().effectors(active_cols[0]),
        [Effector2D::Surface(_)]
    ));
    assert!(
        runtime
            .physics_mut()
            .world_2d()
            .effectors(unused_cols[0])
            .is_empty(),
        "m_UsedByEffector: 0 collider must NOT have the effector attached"
    );
    assert!(
        runtime
            .physics_mut()
            .world_2d()
            .effectors(disabled_cols[0])
            .is_empty(),
        "m_Enabled: 0 effector must NOT be attached to collider"
    );

    let crate_active = EntityId::new(910);
    let body_active = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(crate_active, [0.0, 0.55]));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(crate_active, [0.3, 0.3]).with_attached_body(body_active),
    );

    let crate_unused = EntityId::new(911);
    let body_unused = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(crate_unused, [20.0, 0.55]));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(crate_unused, [0.3, 0.3]).with_attached_body(body_unused),
    );

    for _ in 0..15 {
        runtime.physics_mut().step_2d(0.02);
    }

    let vx_active = runtime
        .physics_mut()
        .body_velocity_2d(crate_active)
        .ok_or("missing active crate velocity")?[0];
    let vx_unused = runtime
        .physics_mut()
        .body_velocity_2d(crate_unused)
        .ok_or("missing unused crate velocity")?[0];

    assert!(
        vx_active > 3.0,
        "SurfaceEffector2D on m_UsedByEffector=1 collider must accelerate resting body toward speed=4.0, got vx={vx_active}"
    );
    assert!(
        vx_unused.abs() < 1e-4,
        "m_UsedByEffector=0 collider must not apply conveyor velocity, got vx={vx_unused}"
    );
    Ok(())
}

#[test]
fn physics2d_rootway_scene_loaded_via_runtime_load_scene_attaches_one_way_bridge_and_conveyor_ledge()
-> Result<(), Box<dyn Error>> {
    let rootway = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/2d-platformer/Assets/Scenes/Rootway.unity");
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&rootway)?;

    let bridge_entity = runtime
        .find_entity_by_name("OneWayBridge")
        .ok_or("missing OneWayBridge entity")?;
    let conveyor_entity = runtime
        .find_entity_by_name("ConveyorLedge")
        .ok_or("missing ConveyorLedge entity")?;

    let bridge_cols = runtime
        .physics_mut()
        .collider_ids_2d(bridge_entity)
        .to_vec();
    let conveyor_cols = runtime
        .physics_mut()
        .collider_ids_2d(conveyor_entity)
        .to_vec();
    assert_eq!(bridge_cols.len(), 1);
    assert_eq!(conveyor_cols.len(), 1);

    assert!(matches!(
        runtime.physics_mut().world_2d().effectors(bridge_cols[0]),
        [Effector2D::Platform(p)] if p.use_one_way && (p.surface_arc - 200.0).abs() < 1e-4
    ));
    assert!(matches!(
        runtime.physics_mut().world_2d().effectors(conveyor_cols[0]),
        [Effector2D::Surface(s)] if (s.speed - 3.0).abs() < 1e-4 && (s.force_scale - 1.0).abs() < 1e-4
    ));

    let jumper = EntityId::new(920);
    let jumper_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(jumper, [6.5, 0.3]).with_velocity([0.0, 12.0]));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(jumper, [0.3, 0.3]).with_attached_body(jumper_body),
    );

    let rider = EntityId::new(921);
    let rider_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(rider, [11.5, 0.75]));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(rider, [0.3, 0.3]).with_attached_body(rider_body),
    );

    let mut passed_bridge = false;
    let mut peak_rider_vx = 0.0f32;
    for _ in 0..35 {
        runtime.physics_mut().step_2d(0.02);
        let y = runtime
            .physics_mut()
            .body_position_2d(jumper)
            .ok_or("missing jumper pos")?[1];
        if y > 2.1 {
            passed_bridge = true;
        }
        let vx = runtime
            .physics_mut()
            .body_velocity_2d(rider)
            .ok_or("missing rider velocity")?[0];
        if vx > peak_rider_vx {
            peak_rider_vx = vx;
        }
    }
    assert!(
        passed_bridge,
        "body jumping from below OneWayBridge (y=1.5) in Rootway.unity must pass through upward"
    );
    assert!(
        peak_rider_vx > 2.5,
        "rider on ConveyorLedge in Rootway.unity must be driven toward speed=3.0, got peak vx={peak_rider_vx}"
    );

    for _ in 0..120 {
        runtime.physics_mut().step_2d(0.02);
    }

    let bridge_rest_y = runtime
        .physics_mut()
        .body_position_2d(jumper)
        .ok_or("missing jumper rest pos")?[1];
    assert!(
        (bridge_rest_y - 2.0).abs() < 0.15,
        "jumper must land and rest on top of OneWayBridge (y=1.5 + 0.2 + 0.3 = 2.0), got {bridge_rest_y}"
    );
    Ok(())
}

#[test]
fn physics2d_scene_area_point_and_buoyancy_effectors_2d_apply_forces_on_load_scene()
-> Result<(), Box<dyn Error>> {
    let scene_yaml = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &10
GameObject:
  m_Name: WindZone
  m_Layer: 0
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &12
BoxCollider2D:
  m_GameObject: {fileID: 10}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 6}
--- !u!249 &13
AreaEffector2D:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_UseGlobalAngle: 1
  m_ForceAngle: 90
  m_ForceMagnitude: 20
  m_Drag: 0.5
--- !u!1 &20
GameObject:
  m_Name: MagnetNode
  m_Layer: 0
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 30, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!58 &22
CircleCollider2D:
  m_GameObject: {fileID: 20}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Radius: 5
--- !u!250 &23
PointEffector2D:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_ForceMagnitude: -35
  m_DistanceScale: 1
--- !u!1 &30
GameObject:
  m_Name: WaterPool
  m_Layer: 0
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 60, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &32
BoxCollider2D:
  m_GameObject: {fileID: 30}
  m_IsTrigger: 1
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 10, y: 6}
--- !u!253 &33
BuoyancyEffector2D:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_SurfaceLevel: 2
  m_Density: 2.5
  m_LinearDrag: 2
  m_AngularDrag: 2
  m_FlowAngle: 0
  m_FlowMagnitude: 6
";

    let scene_path = write_temp_scene("area_point_buoyancy", scene_yaml)?;
    let mut runtime = Runtime::new()?;
    let load_res = runtime.load_scene(&scene_path);
    let _ = std::fs::remove_file(&scene_path);
    load_res?;

    let wind_ball = EntityId::new(930);
    let wind_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(wind_ball, [0.0, 0.0]).with_gravity_scale(0.0));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::circle_collider(wind_ball, 0.3).with_attached_body(wind_body),
    );

    let metal_ball = EntityId::new(931);
    let metal_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(metal_ball, [33.0, 0.0]).with_gravity_scale(0.0));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::circle_collider(metal_ball, 0.3).with_attached_body(metal_body),
    );

    let cork = EntityId::new(932);
    let cork_body = runtime
        .physics_mut()
        .add_body_2d(BodyDesc2D::dynamic(cork, [60.0, 0.0]).with_mass(1.0));
    runtime.physics_mut().add_collider_2d(
        ColliderDesc2D::box_collider(cork, [0.4, 0.4]).with_attached_body(cork_body),
    );

    for _ in 0..20 {
        runtime.physics_mut().step_2d(0.02);
    }

    let wind_vy = runtime
        .physics_mut()
        .body_velocity_2d(wind_ball)
        .ok_or("missing wind_ball velocity")?[1];
    assert!(
        wind_vy > 4.0,
        "AreaEffector2D loaded from scene must accelerate body upward (+Y), got vy={wind_vy}"
    );

    let metal_x = runtime
        .physics_mut()
        .body_position_2d(metal_ball)
        .ok_or("missing metal_ball position")?[0];
    assert!(
        metal_x < 32.5,
        "PointEffector2D with negative force loaded from scene must pull body toward x=30, got x={metal_x}"
    );

    let cork_pos = runtime
        .physics_mut()
        .body_position_2d(cork)
        .ok_or("missing cork position")?;
    assert!(
        cork_pos[1] > 0.2 && cork_pos[0] > 60.1,
        "BuoyancyEffector2D loaded from scene must float submerged cork upward and push rightward with flow, got ({}, {})",
        cork_pos[0],
        cork_pos[1]
    );
    Ok(())
}
