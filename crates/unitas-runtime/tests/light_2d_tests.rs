use std::path::Path;

use unitas_render::Light2DKind;
use unitas_runtime::{
    Runtime, test_read_path_buffer, unitas_runtime_destroy, unitas_runtime_new,
    unitas_runtime_object_create, unitas_runtime_object_set_transform, unitas_runtime_path_buffer,
    unitas_runtime_render_light2d_create, unitas_runtime_render_light2d_destroy,
    unitas_runtime_render_light2d_get_kind, unitas_runtime_render_light2d_get_state,
    unitas_runtime_render_light2d_set_angles, unitas_runtime_render_light2d_set_color,
    unitas_runtime_render_light2d_set_direction, unitas_runtime_render_light2d_set_enabled,
    unitas_runtime_render_light2d_set_intensity, unitas_runtime_render_light2d_set_kind,
    unitas_runtime_render_light2d_set_position, unitas_runtime_render_light2d_set_radii,
};

const SCENE_YAML: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Main Camera
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalPosition: {x: 0, y: 0, z: -2}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!20 &102
Camera:
  m_GameObject: {fileID: 100}
  orthographic: 1
  orthographic size: 2
  field of view: 60
  near clip plane: 0.1
  far clip plane: 100
  m_BackGroundColor: {r: 0, g: 0, b: 0, a: 1}
--- !u!1 &1
GameObject:
  m_Name: CenterSprite
  m_Layer: 0
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!212 &3
SpriteRenderer:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
  m_Color: {r: 0.4, g: 0.6, b: 0.8, a: 1}
  m_FlipX: 0
  m_FlipY: 0
  m_DrawMode: 0
  m_SortingLayerID: 0
  m_SortingOrder: 0
";

fn write_test_tga(path: &Path, width: u16, height: u16, rgb: [u8; 3]) -> std::io::Result<()> {
    let mut bytes = vec![0u8; 18 + (width as usize) * (height as usize) * 3];
    bytes[2] = 2;
    bytes[12..14].copy_from_slice(&width.to_le_bytes());
    bytes[14..16].copy_from_slice(&height.to_le_bytes());
    bytes[16] = 24;
    bytes[17] = 0x20;
    for chunk in bytes[18..].chunks_exact_mut(3) {
        chunk[0] = rgb[2];
        chunk[1] = rgb[1];
        chunk[2] = rgb[0];
    }
    std::fs::write(path, bytes)
}

fn read_f32_le(bytes: &[u8], index: usize) -> f32 {
    let start = index * 4;
    let mut chunk = [0u8; 4];
    chunk.copy_from_slice(&bytes[start..start + 4]);
    f32::from_le_bytes(chunk)
}

fn assert_light_2d_abi_state(handle: u64, light: u64) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = unitas_runtime_path_buffer(handle, 128);
    assert_ne!(buffer, 0);
    assert_eq!(
        unitas_runtime_render_light2d_get_state(handle, light, buffer as *mut f32),
        0
    );

    let mut raw = [0u8; 56];
    test_read_path_buffer(handle, &mut raw)?;
    for (idx, expected) in [
        (0, 2.0),
        (1, 0.25),
        (2, 0.5),
        (3, 0.75),
        (4, 2.5),
        (5, 4.0),
        (6, -5.0),
        (7, 0.6),
        (8, 0.8),
        (9, 0.5),
        (10, 3.5),
        (11, 15.0),
        (12, 45.0),
        (13, 0.0),
    ] {
        assert!((read_f32_le(&raw, idx) - expected).abs() < 1e-6);
    }
    Ok(())
}

#[test]
fn light_2d_abi_roundtrip_and_invalid_argument_handling() -> Result<(), Box<dyn std::error::Error>>
{
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let entity = unitas_runtime_object_create(handle, 0, 0);
    assert_ne!(entity, 0);
    assert_eq!(
        unitas_runtime_object_set_transform(
            handle, entity, 1.5, -2.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
        ),
        0
    );

    let light = unitas_runtime_render_light2d_create(handle, entity);
    assert_ne!(light, 0);
    assert_eq!(unitas_runtime_render_light2d_get_kind(handle, light), 1);

    assert_eq!(unitas_runtime_render_light2d_set_kind(handle, light, 2), 0);
    assert_eq!(unitas_runtime_render_light2d_get_kind(handle, light), 2);
    assert_eq!(
        unitas_runtime_render_light2d_set_color(handle, light, 0.25, 0.5, 0.75),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_intensity(handle, light, 2.5),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_radii(handle, light, 0.5, 3.5),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_angles(handle, light, 15.0, 45.0),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_position(handle, light, 4.0, -5.0),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_direction(handle, light, 0.6, 0.8),
        0
    );
    assert_eq!(
        unitas_runtime_render_light2d_set_enabled(handle, light, 0),
        0
    );

    assert_light_2d_abi_state(handle, light)?;

    assert_eq!(unitas_runtime_render_light2d_set_kind(handle, light, 9), -2);
    assert_eq!(
        unitas_runtime_render_light2d_set_kind(handle, 999999, 0),
        -1
    );
    assert_eq!(unitas_runtime_render_light2d_destroy(handle, light), 0);
    assert_eq!(unitas_runtime_render_light2d_destroy(handle, light), -1);
    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

fn load_light_2d_capture_runtime(dir: &Path) -> Result<Runtime, Box<dyn std::error::Error>> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 100, [255, 255, 255])?;
    std::fs::write(
        texture.with_extension("tga.meta"),
        "fileFormatVersion: 2\nguid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nTextureImporter:\n  sRGBTexture: 0\n",
    )?;
    let scene_path = dir.join("Light2DScene.unity");
    std::fs::write(&scene_path, SCENE_YAML)?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    Ok(runtime)
}

#[test]
fn light_2d_extraction_uses_entity_world_transform_and_brightens_captured_sprite()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_light_2d_capture_{}",
        std::process::id()
    ));
    let mut runtime = load_light_2d_capture_runtime(&dir)?;

    let unlit_frame = runtime.capture_scene_frame(64, 64)?;
    let unlit_center = unlit_frame.image.get_pixel(32, 32);

    let light_entity = runtime.create_runtime_object("PointLight2D")?;
    runtime.set_runtime_object_transform(
        light_entity,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;
    let light_handle = runtime.create_render_light_2d(Some(light_entity));
    assert!(runtime.set_render_light_2d_kind(light_handle, Light2DKind::Point));
    assert!(runtime.set_render_light_2d_color(light_handle, [1.0, 0.5, 0.0]));
    assert!(runtime.set_render_light_2d_intensity(light_handle, 1.0));
    assert!(runtime.set_render_light_2d_radii(light_handle, 0.2, 1.5));

    let extracted = runtime.lights_2d();
    assert_eq!(extracted.len(), 1);
    assert_eq!(extracted[0].kind, Light2DKind::Point);
    assert_eq!(extracted[0].position, [0.0, 0.0]);
    assert_eq!(extracted[0].direction, [0.0, 1.0]);

    let lit_frame = runtime.capture_scene_frame(64, 64)?;
    let lit_center = lit_frame.image.get_pixel(32, 32);

    assert!(
        lit_center[0] > unlit_center[0] && lit_center[1] > unlit_center[1],
        "expected point Light2D to brighten the sprite at center: unlit={unlit_center:?}, lit={lit_center:?}"
    );
    assert_eq!(
        lit_center[2], unlit_center[2],
        "expected zero blue light channel to leave sprite blue unchanged: unlit={unlit_center:?}, lit={lit_center:?}"
    );

    assert!(runtime.set_render_light_2d_enabled(light_handle, false));
    assert!(runtime.lights_2d().is_empty());
    let disabled_frame = runtime.capture_scene_frame(64, 64)?;
    assert_eq!(disabled_frame.image.get_pixel(32, 32), unlit_center);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
