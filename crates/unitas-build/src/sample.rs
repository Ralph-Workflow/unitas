use std::path::Path;

use unitas_render::{Mesh, Vec3};

use crate::bundle::{stage_bundle_for_host, BundleError, StagedBundle};
use crate::config::{BuildConfiguration, BuildMode};
use crate::targets::BuildTarget;

pub const ROLL_A_BALL_SCENE_FILENAME: &str = "RollABall.unity";
pub const ROLL_A_BALL_PRODUCT_NAME: &str = "RollABall";
pub const ROLL_A_BALL_COMPANY_NAME: &str = "Unitas";
pub const ROLL_A_BALL_APPLICATION_ID: &str = "com.unitas.rollaball";
pub const ROLL_A_BALL_VERSION: &str = "1.0.0";

const ROLL_A_BALL_SCENE_YAML: &str = concat!(
    r"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1000
GameObject:
  m_Name: Main Camera
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &1001
Transform:
  m_GameObject: {fileID: 1000}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: 9.5, z: -11.5}
  m_LocalRotation: {x: 0.34202015, y: 0, z: 0, w: 0.9396926}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!20 &1002
Camera:
  m_GameObject: {fileID: 1000}
  orthographic: 0
  orthographic size: 5
  field of view: 60
  near clip plane: 0.1
  far clip plane: 200
  m_BackGroundColor: {r: 0.08, g: 0.11, b: 0.16, a: 1}
--- !u!1 &1100
GameObject:
  m_Name: Directional Light
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &1101
Transform:
  m_GameObject: {fileID: 1100}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: 12, z: 0}
  m_LocalRotation: {x: 0.40821788, y: -0.23456968, z: 0.10938163, w: 0.8754261}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!108 &1102
Light:
  m_GameObject: {fileID: 1100}
  m_Type: 1
  m_Color: {r: 1, g: 0.98, b: 0.92, a: 1}
  m_Intensity: 0.9
",
    r"--- !u!1 &2000
GameObject:
  m_Name: ArenaFloor
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2001
Transform:
  m_GameObject: {fileID: 2000}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: -0.25, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 12, y: 0.5, z: 12}
--- !u!65 &2002
BoxCollider:
  m_GameObject: {fileID: 2000}
  m_IsTrigger: 0
  m_Size: {x: 12, y: 0.5, z: 12}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &2100
GameObject:
  m_Name: Wall_North
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2101
Transform:
  m_GameObject: {fileID: 2100}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: 0.5, z: 6.25}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 13, y: 1, z: 0.5}
--- !u!65 &2102
BoxCollider:
  m_GameObject: {fileID: 2100}
  m_IsTrigger: 0
  m_Size: {x: 13, y: 1, z: 0.5}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &2200
GameObject:
  m_Name: Wall_South
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2201
Transform:
  m_GameObject: {fileID: 2200}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: 0.5, z: -6.25}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 13, y: 1, z: 0.5}
--- !u!65 &2202
BoxCollider:
  m_GameObject: {fileID: 2200}
  m_IsTrigger: 0
  m_Size: {x: 13, y: 1, z: 0.5}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &2300
GameObject:
  m_Name: Wall_East
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2301
Transform:
  m_GameObject: {fileID: 2300}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 6.25, y: 0.5, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.5, y: 1, z: 12}
--- !u!65 &2302
BoxCollider:
  m_GameObject: {fileID: 2300}
  m_IsTrigger: 0
  m_Size: {x: 0.5, y: 1, z: 12}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &2400
GameObject:
  m_Name: Wall_West
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &2401
Transform:
  m_GameObject: {fileID: 2400}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: -6.25, y: 0.5, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.5, y: 1, z: 12}
--- !u!65 &2402
BoxCollider:
  m_GameObject: {fileID: 2400}
  m_IsTrigger: 0
  m_Size: {x: 0.5, y: 1, z: 12}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &3000
GameObject:
  m_Name: PlayerBall
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &3001
Transform:
  m_GameObject: {fileID: 3000}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 0, y: 0.5, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!54 &3002
Rigidbody:
  m_GameObject: {fileID: 3000}
  m_Mass: 1
  m_UseGravity: 1
  m_IsKinematic: 1
  m_IsTrigger: 0
--- !u!135 &3003
SphereCollider:
  m_GameObject: {fileID: 3000}
  m_IsTrigger: 0
  m_Radius: 0.5
  m_Center: {x: 0, y: 0, z: 0}
--- !u!143 &3004
CharacterController:
  m_GameObject: {fileID: 3000}
  m_Enabled: 1
  m_Radius: 0.5
  m_Height: 1
  m_Center: {x: 0, y: 0.05, z: 0}
  m_SlopeLimit: 45
  m_StepOffset: 0.25
  m_SkinWidth: 0.05
  m_MinMoveDistance: 0.002
",
    r"--- !u!1 &4100
GameObject:
  m_Name: Pickup_1
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &4101
Transform:
  m_GameObject: {fileID: 4100}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 3, y: 0.75, z: 3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.75, y: 0.75, z: 0.75}
--- !u!135 &4102
SphereCollider:
  m_GameObject: {fileID: 4100}
  m_IsTrigger: 1
  m_Radius: 0.45
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &4200
GameObject:
  m_Name: Pickup_2
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &4201
Transform:
  m_GameObject: {fileID: 4200}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: -3, y: 0.75, z: 3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.75, y: 0.75, z: 0.75}
--- !u!135 &4202
SphereCollider:
  m_GameObject: {fileID: 4200}
  m_IsTrigger: 1
  m_Radius: 0.45
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &4300
GameObject:
  m_Name: Pickup_3
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &4301
Transform:
  m_GameObject: {fileID: 4300}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: -3, y: 0.75, z: -3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.75, y: 0.75, z: 0.75}
--- !u!135 &4302
SphereCollider:
  m_GameObject: {fileID: 4300}
  m_IsTrigger: 1
  m_Radius: 0.45
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &4400
GameObject:
  m_Name: Pickup_4
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &4401
Transform:
  m_GameObject: {fileID: 4400}
  m_Father: {fileID: 0}
  m_Children: []
  m_LocalPosition: {x: 3, y: 0.75, z: -3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 0.75, y: 0.75, z: 0.75}
--- !u!135 &4402
SphereCollider:
  m_GameObject: {fileID: 4400}
  m_IsTrigger: 1
  m_Radius: 0.45
  m_Center: {x: 0, y: 0, z: 0}
"
);

pub fn roll_a_ball_scene_yaml() -> &'static str {
    ROLL_A_BALL_SCENE_YAML
}

pub fn stage_roll_a_ball_bundle(output_dir: &Path) -> Result<StagedBundle, BundleError> {
    stage_roll_a_ball_bundle_for_host(output_dir, BuildTarget::current_host())
}

/// The build configuration the roll-a-ball sample stages and packages with:
/// its committed scene, scripts compiled at build time, and the managed
/// player-host payload behind the signature the player shell already calls.
pub fn roll_a_ball_build_configuration(
    host: Option<BuildTarget>,
) -> Result<BuildConfiguration, BundleError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| BundleError::Io {
            path: manifest_dir.to_path_buf(),
            message: String::from("the crate directory has no repository root"),
        })?;
    let scene_path = workspace_root
        .join("samples")
        .join("roll-a-ball")
        .join("Assets")
        .join("Scenes")
        .join(ROLL_A_BALL_SCENE_FILENAME);

    let target = host.unwrap_or_else(BuildTarget::host);
    Ok(BuildConfiguration::new(
        vec![scene_path],
        ROLL_A_BALL_COMPANY_NAME,
        ROLL_A_BALL_PRODUCT_NAME,
        ROLL_A_BALL_APPLICATION_ID,
        ROLL_A_BALL_VERSION,
        target,
    )
    .with_mode(BuildMode::Development))
}

pub fn stage_roll_a_ball_bundle_for_host(
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<StagedBundle, BundleError> {
    let target = host.unwrap_or_else(BuildTarget::host);
    let config = roll_a_ball_build_configuration(Some(target))?;
    stage_bundle_for_host(&config, output_dir, Some(target))
}

pub fn imported_gem_mesh() -> Mesh {
    let raw_faces: [[[f32; 3]; 3]; 8] = [
        [[0.0, 0.65, 0.0], [0.55, 0.0, 0.0], [0.0, 0.0, -0.55]],
        [[0.0, 0.65, 0.0], [0.0, 0.0, -0.55], [-0.55, 0.0, 0.0]],
        [[0.0, 0.65, 0.0], [-0.55, 0.0, 0.0], [0.0, 0.0, 0.55]],
        [[0.0, 0.65, 0.0], [0.0, 0.0, 0.55], [0.55, 0.0, 0.0]],
        [[0.0, -0.65, 0.0], [0.0, 0.0, -0.55], [0.55, 0.0, 0.0]],
        [[0.0, -0.65, 0.0], [-0.55, 0.0, 0.0], [0.0, 0.0, -0.55]],
        [[0.0, -0.65, 0.0], [0.0, 0.0, 0.55], [-0.55, 0.0, 0.0]],
        [[0.0, -0.65, 0.0], [0.55, 0.0, 0.0], [0.0, 0.0, 0.55]],
    ];

    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(24);

    for (face_index, tri) in raw_faces.iter().enumerate() {
        let a = Vec3::new(tri[0][0], tri[0][1], tri[0][2]);
        let b = Vec3::new(tri[1][0], tri[1][1], tri[1][2]);
        let c = Vec3::new(tri[2][0], tri[2][1], tri[2][2]);
        let normal = (b - a).cross(c - a).normalize();
        let n_arr = [normal.x, normal.y, normal.z];
        let base = (face_index * 3) as u32;
        positions.push(tri[0]);
        positions.push(tri[1]);
        positions.push(tri[2]);
        normals.push(n_arr);
        normals.push(n_arr);
        normals.push(n_arr);
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    Mesh::from_buffers(&positions, &normals, &indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use unitas_runtime::Runtime;

    static SAMPLE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let id = SAMPLE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "unitas_build_sample_test_{label}_{}_{id}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn imported_gem_mesh_builds_from_buffers_with_valid_normals_and_indices() {
        let mesh = imported_gem_mesh();
        assert_eq!(mesh.positions.len(), 24);
        assert_eq!(mesh.normals.len(), 24);
        assert_eq!(mesh.indices.len(), 24);
        for normal in &mesh.normals {
            assert!((normal.length() - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn roll_a_ball_bundle_stages_and_loads_into_runtime() -> Result<(), Box<dyn std::error::Error>>
    {
        let workspace = temp_dir("stage_sample")?;
        let bundle = stage_roll_a_ball_bundle(&workspace)?;
        assert_eq!(bundle.manifest.product_name, ROLL_A_BALL_PRODUCT_NAME);
        let scene_path = workspace.join(&bundle.manifest.startup_scene_staged_path);
        assert!(scene_path.is_file());

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;
        assert!(runtime.find_entity_by_name("Main Camera").is_some());
        assert!(runtime.find_entity_by_name("Directional Light").is_some());
        assert!(runtime.find_entity_by_name("ArenaFloor").is_some());
        assert!(runtime.find_entity_by_name("PlayerBall").is_some());

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }
}
