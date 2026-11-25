use std::f32::consts::PI;
use std::path::Path;

use unitas_audio::{parse_wav, AudioClip, AudioClipId, AudioError};
use unitas_core::EntityId;
#[cfg(feature = "device")]
use unitas_render::{cube, look_at, uv_sphere, Camera, DirectionalLight, Mat4};
use unitas_render::{Mesh, Vec3};
use unitas_runtime::{
    CharacterMoveRequest, Runtime, RuntimeError, KEY_DOWN_ARROW, KEY_LEFT_ARROW, KEY_RIGHT_ARROW,
    KEY_SPACE, KEY_UP_ARROW,
};

#[cfg(feature = "device")]
use unitas_runtime::window::{
    view_matrix_from_transform, GpuCameraBinding, GpuFramePresenter, GpuMeshBinding,
};

use crate::bundle::{stage_bundle_for_host, BundleError, StagedBundle};
use crate::config::{BuildConfiguration, BuildMode};
use crate::targets::BuildTarget;

pub const ROLL_A_BALL_SCENE_FILENAME: &str = "RollABall.unity";
pub const ROLL_A_BALL_PRODUCT_NAME: &str = "RollABall";
pub const ROLL_A_BALL_COMPANY_NAME: &str = "Unitas";
pub const ROLL_A_BALL_APPLICATION_ID: &str = "com.unitas.rollaball";
pub const ROLL_A_BALL_VERSION: &str = "1.0.0";

pub const PICKUP_NAMES: [&str; 4] = ["Pickup_1", "Pickup_2", "Pickup_3", "Pickup_4"];
pub const WALL_NAMES: [&str; 4] = ["Wall_North", "Wall_South", "Wall_East", "Wall_West"];
pub const PICKUP_POSITIONS: [[f64; 3]; 4] = [
    [3.0, 0.75, 3.0],
    [-3.0, 0.75, 3.0],
    [-3.0, 0.75, -3.0],
    [3.0, 0.75, -3.0],
];
pub const PLAYER_START_POSITION: [f64; 3] = [0.0, 0.5, 0.0];
pub const PICKUP_COLLECT_RADIUS: f64 = 1.15;
pub const PLAYER_MOVE_SPEED: f64 = 7.5;
pub const PLAYER_BOOST_MULTIPLIER: f64 = 1.65;
pub const PLAYER_JUMP_IMPULSE_Y: f64 = 4.2;
pub const PLAYER_GRAVITY_Y: f64 = -9.81;
pub const PLAYER_CHARACTER_RADIUS: f64 = 0.5;
pub const PLAYER_CHARACTER_HEIGHT: f64 = 1.0;
pub const PLAYER_CHARACTER_CENTER: [f64; 3] = [0.0, 0.05, 0.0];
pub const PLAYER_CHARACTER_SLOPE_LIMIT: f64 = 45.0;
pub const PLAYER_CHARACTER_STEP_OFFSET: f64 = 0.25;
pub const PLAYER_CHARACTER_SKIN_WIDTH: f64 = 0.05;
pub const PLAYER_CHARACTER_MIN_MOVE_DISTANCE: f64 = 0.002;
pub const CAMERA_OFFSET: [f32; 3] = [0.0, 9.5, -11.5];
pub const PICKUP_ROTATE_SPEED_DEGREES_PER_SECOND: f64 = 60.0;
pub const PICKUP_BOB_HEIGHT: f64 = 0.2;
pub const PICKUP_BOB_SPEED: f64 = 2.0;

pub fn roll_a_ball_scene_yaml() -> &'static str {
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
--- !u!1 &2000
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
--- !u!1 &4100
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
}

pub fn stage_roll_a_ball_bundle(output_dir: &Path) -> Result<StagedBundle, BundleError> {
    stage_roll_a_ball_bundle_for_host(output_dir, BuildTarget::current_host())
}

/// Stages the real `samples/roll-a-ball` project — its committed scene,
/// scripts compiled at build time, and the managed player-host payload —
/// behind the signature the player shell already calls.
pub fn stage_roll_a_ball_bundle_for_host(
    output_dir: &Path,
    host: Option<BuildTarget>,
) -> Result<StagedBundle, BundleError> {
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
    let config = BuildConfiguration::new(
        vec![scene_path],
        ROLL_A_BALL_COMPANY_NAME,
        ROLL_A_BALL_PRODUCT_NAME,
        ROLL_A_BALL_APPLICATION_ID,
        ROLL_A_BALL_VERSION,
        target,
    )
    .with_mode(BuildMode::Development);

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

pub fn synthesize_chime_wav_bytes(
    sample_rate: u32,
    frequency_hz: f32,
    duration_seconds: f32,
) -> Vec<u8> {
    let rate = sample_rate.max(8_000);
    let frame_count = ((duration_seconds.max(0.02) * rate as f32).round() as usize).max(1);
    let channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let bytes_per_sample = (bits_per_sample / 8) as usize;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = rate * u32::from(block_align);
    let data_len = frame_count * usize::from(channels) * bytes_per_sample;
    let riff_size = 36u32.saturating_add(data_len as u32);

    let mut wav = Vec::with_capacity(44 + data_len);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_len as u32).to_le_bytes());

    let total_f = frame_count as f32;
    for frame in 0..frame_count {
        let t = frame as f32 / rate as f32;
        let progress = frame as f32 / total_f;
        let envelope = (1.0 - progress).max(0.0);
        let fundamental = (2.0 * PI * frequency_hz * t).sin();
        let overtone = 0.35 * (2.0 * PI * (frequency_hz * 1.5) * t).sin();
        let sample = ((fundamental + overtone) * 0.45 * envelope).clamp(-1.0, 1.0);
        let pcm = (sample * 32767.0).round() as i16;
        let bytes = pcm.to_le_bytes();
        wav.extend_from_slice(&bytes);
        wav.extend_from_slice(&bytes);
    }

    wav
}

pub fn synthesize_chime_clip(
    sample_rate: u32,
    frequency_hz: f32,
    duration_seconds: f32,
) -> Result<AudioClip, AudioError> {
    let bytes = synthesize_chime_wav_bytes(sample_rate, frequency_hz, duration_seconds);
    parse_wav(&bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSnapshot {
    pub score: u32,
    pub total: u32,
    pub won: bool,
}

#[derive(Debug, Clone)]
pub struct PickupSlot {
    pub name: String,
    pub entity: Option<EntityId>,
    pub mesh_index: usize,
    pub position: [f64; 3],
    pub collected: bool,
}

#[derive(Debug, Clone)]
pub struct RollABallController {
    pub player_entity: Option<EntityId>,
    pub camera_entity: Option<EntityId>,
    pub audio_entity: EntityId,
    pub pickup_clip_id: AudioClipId,
    pub ambient_clip_id: AudioClipId,
    pub win_clip_id: AudioClipId,
    pub pickups: Vec<PickupSlot>,
    pub score: u32,
    pub total_pickups: u32,
    pub won: bool,
    pub camera_yaw_radians: f32,
    pub elapsed_seconds: f64,
    pub restart_count: u32,
    pub vertical_velocity: f64,
    pub grounded: bool,
}

impl RollABallController {
    #[cfg(feature = "device")]
    pub fn bind_to_scene(
        runtime: &mut Runtime,
        presenter: &mut GpuFramePresenter<'_>,
    ) -> Result<Self, AudioError> {
        let camera_entity = runtime.find_entity_by_name("Main Camera");
        let player_entity = runtime.find_entity_by_name("PlayerBall");
        let floor_entity = runtime.find_entity_by_name("ArenaFloor");

        let mut meshes = Vec::new();

        if let Some(floor) = floor_entity {
            meshes.push(GpuMeshBinding::for_entity(
                floor,
                cube(1.0),
                [0.22, 0.27, 0.34],
                true,
            ));
        } else {
            let floor_model = Mat4::from_cols_array([
                12.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 12.0, 0.0, 0.0, -0.25, 0.0, 1.0,
            ]);
            meshes.push(GpuMeshBinding::standalone(
                cube(1.0),
                floor_model,
                [0.22, 0.27, 0.34],
                true,
            ));
        }

        for wall_name in WALL_NAMES {
            if let Some(wall) = runtime.find_entity_by_name(wall_name) {
                meshes.push(GpuMeshBinding::for_entity(
                    wall,
                    cube(1.0),
                    [0.36, 0.42, 0.52],
                    true,
                ));
            }
        }

        if let Some(player) = player_entity {
            meshes.push(GpuMeshBinding::for_entity(
                player,
                uv_sphere(0.5, 24, 16),
                [0.95, 0.35, 0.2],
                true,
            ));
        } else {
            let ball_model = Mat4::from_cols_array([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 0.0, 1.0,
            ]);
            meshes.push(GpuMeshBinding::standalone(
                uv_sphere(0.5, 24, 16),
                ball_model,
                [0.95, 0.35, 0.2],
                true,
            ));
        }

        let gem_mesh = imported_gem_mesh();
        let pickup_colors: [[f32; 3]; 4] = [
            [0.98, 0.82, 0.2],
            [0.25, 0.9, 0.55],
            [0.3, 0.75, 0.98],
            [0.95, 0.45, 0.85],
        ];
        let mut pickups = Vec::with_capacity(PICKUP_NAMES.len());

        for (idx, pickup_name) in PICKUP_NAMES.iter().enumerate() {
            let entity = runtime.find_entity_by_name(pickup_name);
            let default_pos = PICKUP_POSITIONS[idx];
            let position = entity
                .and_then(|ent| runtime.world_transform_of(ent))
                .map(|t| {
                    [
                        f64::from(t.translation[0]),
                        f64::from(t.translation[1]),
                        f64::from(t.translation[2]),
                    ]
                })
                .unwrap_or(default_pos);
            let lit = idx % 2 == 0;
            let mesh_index = meshes.len();
            if let Some(ent) = entity {
                meshes.push(GpuMeshBinding::for_entity(
                    ent,
                    gem_mesh.clone(),
                    pickup_colors[idx],
                    lit,
                ));
            } else {
                let model = Mat4::from_cols_array([
                    0.75,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.75,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.75,
                    0.0,
                    position[0] as f32,
                    position[1] as f32,
                    position[2] as f32,
                    1.0,
                ]);
                meshes.push(GpuMeshBinding::standalone(
                    gem_mesh.clone(),
                    model,
                    pickup_colors[idx],
                    lit,
                ));
            }
            pickups.push(PickupSlot {
                name: (*pickup_name).to_string(),
                entity,
                mesh_index,
                position,
                collected: false,
            });
        }

        presenter.set_meshes(meshes);

        let camera = Camera::perspective(60.0, 16.0 / 9.0, 0.1, 200.0);
        let initial_view = camera_entity
            .and_then(|ent| runtime.world_transform_of(ent))
            .map(|transform| view_matrix_from_transform(&transform))
            .unwrap_or_else(|| {
                look_at(
                    Vec3::new(CAMERA_OFFSET[0], CAMERA_OFFSET[1], CAMERA_OFFSET[2]),
                    Vec3::new(0.0, 0.5, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                )
            });
        presenter.set_camera_binding(GpuCameraBinding {
            entity: None,
            camera,
            view: initial_view,
            background: [0.08, 0.11, 0.16],
        });
        presenter.set_light(DirectionalLight {
            direction: Vec3::new(0.4, 0.85, 0.35),
            color: [1.0, 0.98, 0.92],
            intensity: 0.9,
        });

        let pickup_wav = synthesize_chime_wav_bytes(44_100, 660.0, 0.25);
        let ambient_wav = synthesize_chime_wav_bytes(44_100, 330.0, 0.35);
        let win_wav = synthesize_chime_wav_bytes(44_100, 784.0, 0.6);
        let pickup_clip_id = runtime.audio_mut().load_wav_bytes(&pickup_wav)?;
        let ambient_clip_id = runtime.audio_mut().load_wav_bytes(&ambient_wav)?;
        let win_clip_id = runtime.audio_mut().load_wav_bytes(&win_wav)?;

        let audio_entity = player_entity.unwrap_or_else(|| runtime.ensure_audio_source(0));
        runtime.audio_mut().ensure_source(audio_entity);
        runtime
            .audio_mut()
            .set_source_clip(audio_entity, Some(ambient_clip_id));
        runtime.audio_mut().set_volume(audio_entity, 0.85);
        runtime.audio_mut().set_looping(audio_entity, true);
        runtime.audio_mut().play(audio_entity, false);

        let total_pickups = pickups.len() as u32;
        Ok(Self {
            player_entity,
            camera_entity,
            audio_entity,
            pickup_clip_id,
            ambient_clip_id,
            win_clip_id,
            pickups,
            score: 0,
            total_pickups,
            won: false,
            camera_yaw_radians: 0.0,
            elapsed_seconds: 0.0,
            restart_count: 0,
            vertical_velocity: 0.0,
            grounded: false,
        })
    }

    #[cfg(feature = "device")]
    pub fn step(
        &mut self,
        runtime: &mut Runtime,
        presenter: &mut GpuFramePresenter<'_>,
        dt: f64,
    ) -> Result<(), RuntimeError> {
        let mut meshes = presenter.meshes().to_vec();
        let mut camera_binding = *presenter.camera_binding();
        self.step_on_bindings(runtime, &mut meshes, &mut camera_binding, dt)?;
        presenter.set_meshes(meshes);
        presenter.set_camera_binding(camera_binding);
        Ok(())
    }

    pub fn hud_snapshot(&self) -> HudSnapshot {
        HudSnapshot {
            score: self.score,
            total: self.total_pickups,
            won: self.won,
        }
    }

    #[cfg(feature = "device")]
    pub fn step_on_bindings(
        &mut self,
        runtime: &mut Runtime,
        meshes: &mut [GpuMeshBinding],
        camera_binding: &mut GpuCameraBinding,
        dt: f64,
    ) -> Result<(), RuntimeError> {
        self.step_world(runtime, dt)?;
        self.sync_pickup_meshes(meshes);
        if let Some(player) = self.player_entity {
            if let Some(position) = runtime.body_position(player) {
                self.update_camera_follow_on_binding(position, camera_binding);
            }
        }
        Ok(())
    }

    pub fn step_world(&mut self, runtime: &mut Runtime, dt: f64) -> Result<(), RuntimeError> {
        let step_dt = if dt.is_finite() && dt > 0.0 {
            dt
        } else {
            1.0 / 60.0
        };
        self.elapsed_seconds += step_dt;

        if runtime.input().is_key_pressed(u32::from(b'r')) {
            self.reset_world(runtime)?;
            return Ok(());
        }

        let (_, _, pointer_dx, _) = runtime.input().pointer();
        if pointer_dx.abs() > 0.0 {
            self.camera_yaw_radians += (pointer_dx as f32) * 0.005;
        }

        let mut move_x = runtime.input().axis("Horizontal").unwrap_or(0.0);
        let mut move_z = runtime.input().axis("Vertical").unwrap_or(0.0);

        if runtime.input().is_key_down(u32::from(b'd'))
            || runtime.input().is_key_down(KEY_RIGHT_ARROW)
        {
            move_x = move_x.max(1.0);
        }
        if runtime.input().is_key_down(u32::from(b'a'))
            || runtime.input().is_key_down(KEY_LEFT_ARROW)
        {
            move_x = move_x.min(-1.0);
        }
        if runtime.input().is_key_down(u32::from(b'w')) || runtime.input().is_key_down(KEY_UP_ARROW)
        {
            move_z = move_z.max(1.0);
        }
        if runtime.input().is_key_down(u32::from(b's'))
            || runtime.input().is_key_down(KEY_DOWN_ARROW)
        {
            move_z = move_z.min(-1.0);
        }

        let boost = if runtime.input().is_mouse_button_down(0) {
            PLAYER_BOOST_MULTIPLIER
        } else {
            1.0
        };

        let yaw = f64::from(self.camera_yaw_radians);
        let cos_yaw = yaw.cos();
        let sin_yaw = yaw.sin();
        let world_dir_x = move_x * cos_yaw + move_z * sin_yaw;
        let world_dir_z = move_z * cos_yaw - move_x * sin_yaw;

        if let Some(player) = self.player_entity {
            let current_pos = runtime
                .body_position(player)
                .or_else(|| {
                    runtime.world_transform_of(player).map(|t| {
                        [
                            f64::from(t.translation[0]),
                            f64::from(t.translation[1]),
                            f64::from(t.translation[2]),
                        ]
                    })
                })
                .unwrap_or(PLAYER_START_POSITION);

            let speed = PLAYER_MOVE_SPEED * boost;
            let motion = [
                world_dir_x * speed * step_dt,
                self.vertical_motion(runtime, step_dt),
                world_dir_z * speed * step_dt,
            ];

            let request =
                self.character_move_request(runtime, player, current_pos, motion, step_dt);
            let result = runtime.character_move(request)?;
            self.grounded = result.is_grounded;
            runtime.set_body_position(player, result.position)?;

            self.collect_pickups(result.position, runtime);
        }
        self.animate_pickups(runtime);
        Ok(())
    }

    fn vertical_motion(&mut self, runtime: &Runtime, step_dt: f64) -> f64 {
        if self.grounded {
            self.vertical_velocity = if runtime.input().is_key_pressed(KEY_SPACE) {
                PLAYER_JUMP_IMPULSE_Y
            } else {
                0.0
            };
        } else {
            self.vertical_velocity += PLAYER_GRAVITY_Y * step_dt;
        }
        self.vertical_velocity * step_dt
    }

    fn character_move_request(
        &self,
        runtime: &Runtime,
        entity: EntityId,
        position: [f64; 3],
        motion: [f64; 3],
        step_dt: f64,
    ) -> CharacterMoveRequest {
        let config = runtime.character_controller_config(entity);
        CharacterMoveRequest {
            entity: Some(entity),
            position,
            motion,
            radius: config.map_or(PLAYER_CHARACTER_RADIUS, |value| value.radius),
            height: config.map_or(PLAYER_CHARACTER_HEIGHT, |value| value.height),
            center: config.map_or(PLAYER_CHARACTER_CENTER, |value| value.center),
            slope_limit_deg: config.map_or(PLAYER_CHARACTER_SLOPE_LIMIT, |value| value.slope_limit),
            step_offset: config.map_or(PLAYER_CHARACTER_STEP_OFFSET, |value| value.step_offset),
            skin_width: config.map_or(PLAYER_CHARACTER_SKIN_WIDTH, |value| value.skin_width),
            min_move_distance: config.map_or(PLAYER_CHARACTER_MIN_MOVE_DISTANCE, |value| {
                value.min_move_distance
            }),
            layer_mask: u32::MAX,
            enabled: config.is_none_or(|value| value.enabled),
            delta_seconds: step_dt,
        }
    }

    fn collect_pickups(&mut self, player_pos: [f64; 3], runtime: &mut Runtime) {
        if self.won {
            return;
        }
        let radius_sq = PICKUP_COLLECT_RADIUS * PICKUP_COLLECT_RADIUS;
        let mut newly_collected = false;

        for slot in &mut self.pickups {
            if slot.collected {
                continue;
            }
            let dx = player_pos[0] - slot.position[0];
            let dy = player_pos[1] - slot.position[1];
            let dz = player_pos[2] - slot.position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= radius_sq {
                slot.collected = true;
                self.score += 1;
                newly_collected = true;
            }
        }

        if !newly_collected {
            return;
        }
        let clip_id = if self.score >= self.total_pickups {
            self.won = true;
            self.win_clip_id
        } else {
            self.pickup_clip_id
        };
        let audio = runtime.audio_mut();
        audio.set_source_clip(self.audio_entity, Some(clip_id));
        audio.set_looping(self.audio_entity, false);
        audio.play(self.audio_entity, true);
    }

    fn animate_pickups(&mut self, runtime: &mut Runtime) {
        let angle = self.elapsed_seconds * PICKUP_ROTATE_SPEED_DEGREES_PER_SECOND.to_radians();
        let half_angle = (angle * 0.5) as f32;
        let rotation = [0.0, half_angle.sin(), 0.0, half_angle.cos()];
        let bob_offset = (self.elapsed_seconds * PICKUP_BOB_SPEED).sin() * PICKUP_BOB_HEIGHT;
        for slot in &self.pickups {
            if slot.collected {
                continue;
            }
            let Some(entity) = slot.entity else {
                continue;
            };
            let Some(graph) = runtime.scene_graph_mut() else {
                break;
            };
            if let Some(transform) = graph.transform_mut(entity) {
                transform.rotation = rotation;
                transform.translation = [
                    slot.position[0] as f32,
                    (slot.position[1] + bob_offset) as f32,
                    slot.position[2] as f32,
                ];
            }
        }
    }

    #[cfg(feature = "device")]
    fn sync_pickup_meshes(&self, meshes: &mut [GpuMeshBinding]) {
        let angle = self.elapsed_seconds * PICKUP_ROTATE_SPEED_DEGREES_PER_SECOND.to_radians();
        let (sin_angle, cos_angle) = angle.sin_cos();
        let bob_offset = (self.elapsed_seconds * PICKUP_BOB_SPEED).sin() * PICKUP_BOB_HEIGHT;
        for slot in &self.pickups {
            if let Some(binding) = meshes.get_mut(slot.mesh_index) {
                binding.enabled = !slot.collected;
                if binding.entity.is_none() {
                    let x = slot.position[0] as f32;
                    let y = (slot.position[1] + bob_offset) as f32;
                    let z = slot.position[2] as f32;
                    binding.model = Mat4::from_cols_array([
                        0.75 * cos_angle as f32,
                        0.0,
                        -0.75 * sin_angle as f32,
                        0.0,
                        0.0,
                        0.75,
                        0.0,
                        0.0,
                        0.75 * sin_angle as f32,
                        0.0,
                        0.75 * cos_angle as f32,
                        0.0,
                        x,
                        y,
                        z,
                        1.0,
                    ]);
                }
            }
        }
    }

    #[cfg(feature = "device")]
    fn update_camera_follow_on_binding(
        &self,
        player_pos: [f64; 3],
        camera_binding: &mut GpuCameraBinding,
    ) {
        let target = Vec3::new(
            player_pos[0] as f32,
            (player_pos[1] as f32).max(0.5),
            player_pos[2] as f32,
        );
        let horiz_dist = (-CAMERA_OFFSET[2]).max(1.0);
        let eye = Vec3::new(
            target.x - self.camera_yaw_radians.sin() * horiz_dist,
            target.y + CAMERA_OFFSET[1],
            target.z - self.camera_yaw_radians.cos() * horiz_dist,
        );
        camera_binding.view = look_at(eye, target, Vec3::new(0.0, 1.0, 0.0));
    }

    #[cfg(feature = "device")]
    pub fn reset(
        &mut self,
        runtime: &mut Runtime,
        presenter: &mut GpuFramePresenter<'_>,
    ) -> Result<(), RuntimeError> {
        let mut meshes = presenter.meshes().to_vec();
        let mut camera_binding = *presenter.camera_binding();
        self.reset_on_bindings(runtime, &mut meshes, &mut camera_binding)?;
        presenter.set_meshes(meshes);
        presenter.set_camera_binding(camera_binding);
        Ok(())
    }

    #[cfg(feature = "device")]
    pub fn reset_on_bindings(
        &mut self,
        runtime: &mut Runtime,
        meshes: &mut [GpuMeshBinding],
        camera_binding: &mut GpuCameraBinding,
    ) -> Result<(), RuntimeError> {
        self.reset_world(runtime)?;
        self.sync_pickup_meshes(meshes);
        self.update_camera_follow_on_binding(PLAYER_START_POSITION, camera_binding);
        Ok(())
    }

    pub fn reset_world(&mut self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        self.teleport_player(runtime, PLAYER_START_POSITION)?;
        for slot in &mut self.pickups {
            slot.collected = false;
        }
        self.restore_pickup_transforms(runtime);
        self.score = 0;
        self.won = false;
        self.camera_yaw_radians = 0.0;
        self.restart_count += 1;
        runtime
            .audio_mut()
            .set_source_clip(self.audio_entity, Some(self.ambient_clip_id));
        runtime.audio_mut().set_looping(self.audio_entity, true);
        runtime.audio_mut().play(self.audio_entity, false);
        Ok(())
    }

    fn restore_pickup_transforms(&mut self, runtime: &mut Runtime) {
        for slot in &self.pickups {
            let Some(entity) = slot.entity else {
                continue;
            };
            let Some(graph) = runtime.scene_graph_mut() else {
                break;
            };
            if let Some(transform) = graph.transform_mut(entity) {
                transform.rotation = [0.0, 0.0, 0.0, 1.0];
                transform.translation = [
                    slot.position[0] as f32,
                    slot.position[1] as f32,
                    slot.position[2] as f32,
                ];
            }
        }
    }

    pub fn teleport_player(
        &mut self,
        runtime: &mut Runtime,
        position: [f64; 3],
    ) -> Result<(), RuntimeError> {
        let Some(player) = self.player_entity else {
            return Ok(());
        };
        let target = runtime.character_teleport(Some(player), position)?;
        runtime.set_body_position(player, target)?;
        self.vertical_velocity = 0.0;
        self.grounded = false;
        Ok(())
    }
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
    fn synthesized_chime_wav_parses_and_produces_non_silent_stereo_samples(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let clip = synthesize_chime_clip(44_100, 523.25, 0.2)?;
        assert_eq!(clip.channels, 2);
        assert_eq!(clip.sample_rate, 44_100);
        assert!(clip.frame_count() > 4_000);
        let peak = clip
            .samples
            .iter()
            .fold(0.0f32, |acc, sample| acc.max(sample.abs()));
        assert!(peak > 0.1);
        Ok(())
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
        for pickup_name in PICKUP_NAMES {
            assert!(runtime.find_entity_by_name(pickup_name).is_some());
        }

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    fn roll_a_ball_gameplay_steps_headless_without_device_bindings(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_dir("controller_headless")?;
        let bundle = stage_roll_a_ball_bundle(&workspace)?;
        let scene_path = workspace.join(&bundle.manifest.startup_scene_staged_path);

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;

        let pickup_wav = synthesize_chime_wav_bytes(44_100, 660.0, 0.25);
        let win_wav = synthesize_chime_wav_bytes(44_100, 784.0, 0.6);
        let pickup_clip_id = runtime.audio_mut().load_wav_bytes(&pickup_wav)?;
        let win_clip_id = runtime.audio_mut().load_wav_bytes(&win_wav)?;
        let audio_entity = runtime.ensure_audio_source(0);
        runtime.audio_mut().ensure_source(audio_entity);

        let mut pickups = Vec::with_capacity(PICKUP_NAMES.len());
        for (idx, pickup_name) in PICKUP_NAMES.iter().enumerate() {
            pickups.push(PickupSlot {
                name: (*pickup_name).to_string(),
                entity: runtime.find_entity_by_name(pickup_name),
                mesh_index: idx,
                position: PICKUP_POSITIONS[idx],
                collected: false,
            });
        }

        let mut controller = RollABallController {
            player_entity: runtime.find_entity_by_name("PlayerBall"),
            camera_entity: runtime.find_entity_by_name("Main Camera"),
            audio_entity,
            pickup_clip_id,
            ambient_clip_id: pickup_clip_id,
            win_clip_id,
            pickups,
            score: 0,
            total_pickups: 4,
            won: false,
            camera_yaw_radians: 0.0,
            elapsed_seconds: 0.0,
            restart_count: 0,
            vertical_velocity: 0.0,
            grounded: false,
        };
        assert_eq!(
            controller.hud_snapshot(),
            HudSnapshot {
                score: 0,
                total: 4,
                won: false
            }
        );

        runtime.tick(1.0 / 60.0)?;
        controller.step_world(&mut runtime, 1.0 / 60.0)?;
        assert_eq!(controller.score, 0);
        assert!(!controller.won);

        let pickup_entity = controller.pickups[0]
            .entity
            .ok_or("missing Pickup_1 entity")?;
        let before = runtime
            .world_transform_of(pickup_entity)
            .ok_or("missing Pickup_1 transform")?;
        controller.step_world(&mut runtime, 1.0 / 50.0)?;
        let after = runtime
            .world_transform_of(pickup_entity)
            .ok_or("missing Pickup_1 transform after step")?;
        assert_ne!(before.rotation, after.rotation);
        assert_ne!(before.translation, after.translation);

        for position in PICKUP_POSITIONS {
            controller.teleport_player(&mut runtime, position)?;
            controller.step_world(&mut runtime, 1.0 / 60.0)?;
        }
        assert_eq!(controller.score, 4);
        assert!(controller.won);
        assert_eq!(
            controller.hud_snapshot(),
            HudSnapshot {
                score: 4,
                total: 4,
                won: true
            }
        );
        assert_eq!(
            runtime
                .audio()
                .source(audio_entity)
                .and_then(|source| source.clip),
            Some(win_clip_id)
        );

        runtime.input_mut().inject_key(u32::from(b'r'), true);
        runtime.tick(1.0 / 60.0)?;
        controller.step_world(&mut runtime, 1.0 / 60.0)?;
        assert_eq!(controller.score, 0);
        assert!(!controller.won);
        assert_eq!(controller.restart_count, 1);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    #[cfg(feature = "device")]
    fn roll_a_ball_controller_collects_all_pickups_wins_and_restart_clears(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Ok(mut presenter) = GpuFramePresenter::from_new_renderer() else {
            return Ok(());
        };
        let workspace = temp_dir("controller_win")?;
        let bundle = stage_roll_a_ball_bundle(&workspace)?;
        let scene_path = workspace.join(&bundle.manifest.startup_scene_staged_path);

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;

        let mut controller = RollABallController::bind_to_scene(&mut runtime, &mut presenter)?;
        let player = controller
            .player_entity
            .ok_or("missing PlayerBall entity")?;

        runtime.tick(1.0 / 60.0)?;
        for position in PICKUP_POSITIONS {
            controller.teleport_player(&mut runtime, position)?;
            controller.step(&mut runtime, &mut presenter, 1.0 / 60.0)?;
        }
        assert_eq!(controller.score, 4);
        assert!(controller.won);
        let hud = controller.hud_snapshot();
        assert_eq!(hud.score, 4);
        assert_eq!(hud.total, 4);
        assert!(hud.won);
        let queued_clip = runtime
            .audio()
            .source(controller.audio_entity)
            .and_then(|source| source.clip);
        assert_eq!(queued_clip, Some(controller.win_clip_id));
        for slot in &controller.pickups {
            assert!(!presenter.meshes()[slot.mesh_index].enabled);
        }

        runtime.input_mut().inject_key(u32::from(b'r'), true);
        runtime.tick(1.0 / 60.0)?;
        controller.step(&mut runtime, &mut presenter, 1.0 / 60.0)?;
        assert_eq!(controller.score, 0);
        assert!(!controller.won);
        assert_eq!(controller.restart_count, 1);
        assert_eq!(
            controller.hud_snapshot(),
            HudSnapshot {
                score: 0,
                total: 4,
                won: false
            }
        );
        for slot in &controller.pickups {
            assert!(presenter.meshes()[slot.mesh_index].enabled);
        }
        let reset_pos = runtime
            .body_position(player)
            .ok_or("missing PlayerBall physics body after restart")?;
        assert_eq!(reset_pos, PLAYER_START_POSITION);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }

    #[test]
    #[cfg(feature = "device")]
    fn roll_a_ball_controller_moves_ball_collects_pickup_and_resets_on_r_key(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Ok(mut presenter) = GpuFramePresenter::from_new_renderer() else {
            return Ok(());
        };
        let workspace = temp_dir("controller_gameplay")?;
        let bundle = stage_roll_a_ball_bundle(&workspace)?;
        let scene_path = workspace.join(&bundle.manifest.startup_scene_staged_path);

        let mut runtime = Runtime::new()?;
        runtime.load_scene(&scene_path)?;

        let mut controller = RollABallController::bind_to_scene(&mut runtime, &mut presenter)?;
        assert_eq!(controller.score, 0);
        assert_eq!(controller.total_pickups, 4);

        let player = controller
            .player_entity
            .ok_or("missing PlayerBall entity")?;
        let initial_pos = runtime
            .body_position(player)
            .ok_or("missing PlayerBall physics body")?;

        runtime.tick(1.0 / 60.0)?;
        runtime.input_mut().inject_key(u32::from(b'w'), true);
        runtime.input_mut().inject_key(u32::from(b'd'), true);
        runtime.input_mut().inject_mouse_button(0, true);
        runtime.input_mut().inject_pointer_move(20.0, 0.0);

        for _ in 0..8 {
            runtime.tick(1.0 / 50.0)?;
            controller.step(&mut runtime, &mut presenter, 1.0 / 50.0)?;
        }

        let moved_pos = runtime
            .body_position(player)
            .ok_or("missing PlayerBall physics body after step")?;
        assert!(moved_pos[0] > initial_pos[0] || moved_pos[2] > initial_pos[2]);
        assert!(controller.camera_yaw_radians > 0.0);

        controller.teleport_player(&mut runtime, PICKUP_POSITIONS[0])?;
        controller.step(&mut runtime, &mut presenter, 1.0 / 60.0)?;
        assert_eq!(controller.score, 1);
        assert!(controller.pickups[0].collected);
        let pickup_mesh_idx = controller.pickups[0].mesh_index;
        assert!(!presenter.meshes()[pickup_mesh_idx].enabled);

        runtime.input_mut().inject_key(u32::from(b'r'), true);
        runtime.tick(1.0 / 60.0)?;
        controller.step(&mut runtime, &mut presenter, 1.0 / 60.0)?;
        assert_eq!(controller.score, 0);
        assert_eq!(controller.restart_count, 1);
        assert!(!controller.pickups[0].collected);
        assert!(presenter.meshes()[pickup_mesh_idx].enabled);
        let reset_pos = runtime
            .body_position(player)
            .ok_or("missing PlayerBall physics body after reset")?;
        assert_eq!(reset_pos, PLAYER_START_POSITION);

        fs::remove_dir_all(&workspace)?;
        Ok(())
    }
}
