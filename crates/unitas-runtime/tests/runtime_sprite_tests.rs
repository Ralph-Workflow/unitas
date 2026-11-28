use std::path::Path;

use unitas_asset::scene::SpriteDrawMode;
use unitas_runtime::{
    Runtime, unitas_runtime_destroy, unitas_runtime_new, unitas_runtime_object_create,
    unitas_runtime_sprite_create, unitas_runtime_sprite_set_color,
    unitas_runtime_sprite_set_draw_mode, unitas_runtime_sprite_set_enabled,
    unitas_runtime_sprite_set_flip, unitas_runtime_sprite_set_sorting,
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
  m_Name: SceneSprite
  m_Layer: 0
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalPosition: {x: -1, y: 0, z: 0}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!212 &3
SpriteRenderer:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}
  m_Color: {r: 1, g: 1, b: 1, a: 1}
  m_FlipX: 0
  m_FlipY: 0
  m_DrawMode: 0
  m_SortingLayerID: 0
  m_SortingOrder: 5
";

const SLICED_SCENE_YAML: &str = "%YAML 1.1
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
  m_Name: SlicedSprite
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
  m_Color: {r: 1, g: 1, b: 1, a: 1}
  m_FlipX: 0
  m_FlipY: 0
  m_DrawMode: 1
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

fn write_scene(dir: &Path, texture: &Path) -> std::io::Result<()> {
    std::fs::write(
        texture.with_extension("tga.meta"),
        "fileFormatVersion: 2\nguid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    )?;
    std::fs::write(dir.join("SpriteScene.unity"), SCENE_YAML)
}

fn write_sprite_meta(path: &Path, border: [f32; 4]) -> std::io::Result<()> {
    let text = format!(
        "fileFormatVersion: 2\nguid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nTextureImporter:\n  textureType: 8\n  sRGBTexture: 1\n  maxTextureSize: 2048\n  mipmapEnabled: 0\n  spriteMode: 1\n  alphaIsTransparency: 0\n  wrapMode: 0\n  filterMode: 1\n  spriteSheet:\n    serializedVersion: 2\n    sprites:\n    - serializedVersion: 2\n      name: sprite\n      rect:\n        serializedVersion: 2\n        x: 0\n        y: 0\n        width: 100\n        height: 100\n      pivot: {{x: 0.5, y: 0.5}}\n      border: {{x: {}, y: {}, z: {}, w: {}}}\n",
        border[0], border[1], border[2], border[3]
    );
    std::fs::write(path, text)
}

fn write_sliced_scene(dir: &Path, texture: &Path, border: [f32; 4]) -> std::io::Result<()> {
    write_sprite_meta(&texture.with_extension("tga.meta"), border)?;
    std::fs::write(dir.join("SlicedScene.unity"), SLICED_SCENE_YAML)
}

fn write_regioned_tga(path: &Path, size: u16, border: u16) -> std::io::Result<()> {
    let mut bytes = vec![0u8; 18 + (size as usize) * (size as usize) * 3];
    bytes[2] = 2;
    bytes[12..14].copy_from_slice(&size.to_le_bytes());
    bytes[14..16].copy_from_slice(&size.to_le_bytes());
    bytes[16] = 24;
    bytes[17] = 0x20;
    for row in 0..size {
        for column in 0..size {
            let row_band = row < border || row >= size - border;
            let column_band = column < border || column >= size - border;
            let rgb = match (row_band, column_band) {
                (true, true) => [255, 0, 0],
                (true, false) | (false, true) => [0, 255, 0],
                (false, false) => [0, 0, 255],
            };
            let offset = 18 + ((row as usize) * (size as usize) + (column as usize)) * 3;
            bytes[offset] = rgb[2];
            bytes[offset + 1] = rgb[1];
            bytes[offset + 2] = rgb[0];
        }
    }
    std::fs::write(path, bytes)
}

fn assert_initial_script_sprite_sorting(runtime: &mut Runtime, entity: EntityId) {
    let sprites = runtime.sprite_instances();
    assert_eq!(sprites.len(), 2);
    assert_eq!(sprites[0].sorting_layer, 0);
    assert_eq!(sprites[0].sorting_order, 1);
    assert_eq!(sprites[0].color, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(sprites[0].texture.width, 100);
    assert_eq!(sprites[0].texture.height, 50);
    assert!((sprites[0].model.get(3, 0) - 3.0).abs() < 1e-6);
    assert!((sprites[0].model.get(3, 1) - 4.0).abs() < 1e-6);
    assert!((sprites[0].model.get(0, 0) + 1.0).abs() < 1e-6);
    assert!((sprites[0].model.get(1, 1) - 0.5).abs() < 1e-6);
    assert_eq!(sprites[1].sorting_layer, 0);
    assert_eq!(sprites[1].sorting_order, 5);

    assert!(runtime.set_runtime_sprite_sorting(entity, 0, 10));
    let sprites = runtime.sprite_instances();
    assert_eq!(sprites[0].sorting_order, 5);
    assert_eq!(sprites[1].sorting_order, 10);
}

#[test]
fn script_sprite_draws_in_the_same_sorted_pass_as_the_scene_sprites()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 50, [255, 255, 255])?;
    write_scene(&dir, &texture)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SpriteScene.unity"))?;

    let entity = runtime.create_runtime_object("ScriptSprite")?;
    runtime.set_runtime_object_transform(
        entity,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    )?;
    assert!(runtime.create_runtime_sprite(entity, Some(&texture)));
    assert!(runtime.set_runtime_sprite_color(entity, [0.1, 0.2, 0.3, 0.4]));
    assert!(runtime.set_runtime_sprite_sorting(entity, 0, 1));
    assert!(runtime.set_runtime_sprite_flip(entity, true, false));
    assert_initial_script_sprite_sorting(&mut runtime, entity);

    assert!(runtime.set_runtime_sprite_enabled(entity, false));
    assert_eq!(runtime.sprite_instances().len(), 1);
    let scene_only_frame = runtime.capture_scene_frame(64, 64)?;
    assert!(runtime.set_runtime_sprite_enabled(entity, true));
    assert_eq!(runtime.sprite_instances().len(), 2);
    assert_eq!(
        scene_only_frame.image.get_pixel(16, 31),
        [255, 255, 255, 255]
    );

    assert!(runtime.set_runtime_sprite_flip(entity, false, true));
    let sprites = runtime.sprite_instances();
    assert!((sprites[1].model.get(1, 1) + 0.5).abs() < 1e-6);

    runtime.set_runtime_object_transform(
        entity,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [4.0, 4.0, 1.0],
    )?;
    assert!(runtime.set_runtime_sprite_color(entity, [0.0, 1.0, 0.0, 1.0]));
    assert!(runtime.set_runtime_sprite_sorting(entity, 0, 10));
    let front_frame = runtime.capture_scene_frame(64, 64)?;
    let front_pixel = front_frame.image.get_pixel(32, 32);
    assert_eq!(
        front_pixel,
        [0, 255, 0, 255],
        "the script sprite must be drawn into the captured frame"
    );
    assert_eq!(front_frame.image.get_pixel(16, 31), [0, 255, 0, 255]);

    assert!(runtime.set_runtime_sprite_color(entity, [0.0, 0.0, 1.0, 1.0]));
    let recolored_frame = runtime.capture_scene_frame(64, 64)?;
    let recolored_pixel = recolored_frame.image.get_pixel(32, 32);
    assert_ne!(front_pixel, recolored_pixel);
    assert_eq!(
        recolored_pixel,
        [0, 0, 255, 255],
        "recolouring the script sprite must change the captured pixels"
    );

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn script_sprite_sliced_draw_mode_draws_the_nine_slice_parts()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_sliced_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 100, [255, 0, 0])?;
    write_scene(&dir, &texture)?;
    write_sprite_meta(
        &texture.with_extension("tga.meta"),
        [20.0, 20.0, 20.0, 20.0],
    )?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SpriteScene.unity"))?;
    let entity = runtime.create_runtime_object("ScriptSprite")?;
    assert!(runtime.create_runtime_sprite(entity, Some(&texture)));

    assert!(runtime.set_runtime_sprite_draw_mode(entity, SpriteDrawMode::Sliced));
    let sprites = runtime.sprite_instances();
    assert_eq!(sprites.len(), 10);
    let parts = &sprites[..9];
    let expected = [
        (20u32, 20u32),
        (60, 20),
        (20, 20),
        (20, 60),
        (60, 60),
        (20, 60),
        (20, 20),
        (60, 20),
        (20, 20),
    ];
    for (index, (width, height)) in expected.iter().enumerate() {
        assert_eq!(parts[index].texture.width, *width, "part {index} width");
        assert_eq!(parts[index].texture.height, *height, "part {index} height");
    }
    assert!((parts[0].model.get(3, 0) - (-0.4)).abs() < 1e-6);
    assert!((parts[0].model.get(3, 1) - (-0.4)).abs() < 1e-6);
    assert!((parts[0].model.get(0, 0) - 0.2).abs() < 1e-6);
    assert!((parts[0].model.get(1, 1) - 0.2).abs() < 1e-6);
    assert!((parts[4].model.get(3, 0)).abs() < 1e-6);
    assert!((parts[4].model.get(3, 1)).abs() < 1e-6);
    assert!((parts[4].model.get(0, 0) - 0.6).abs() < 1e-6);
    assert!((parts[4].model.get(1, 1) - 0.6).abs() < 1e-6);
    assert!((parts[8].model.get(3, 0) - 0.4).abs() < 1e-6);
    assert!((parts[8].model.get(3, 1) - 0.4).abs() < 1e-6);

    assert!(runtime.set_runtime_sprite_draw_mode(entity, SpriteDrawMode::Simple));
    assert_eq!(runtime.sprite_instances().len(), 2);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn script_sprite_tiled_draw_mode_repeats_the_sprite_rect() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_tiled_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 100, [255, 0, 0])?;
    write_scene(&dir, &texture)?;
    write_sprite_meta(
        &texture.with_extension("tga.meta"),
        [20.0, 20.0, 20.0, 20.0],
    )?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SpriteScene.unity"))?;
    let entity = runtime.create_runtime_object("ScriptSprite")?;
    assert!(runtime.create_runtime_sprite(entity, Some(&texture)));
    runtime.set_runtime_object_transform(
        entity,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [2.0, 2.0, 1.0],
    )?;

    assert!(runtime.set_runtime_sprite_draw_mode(entity, SpriteDrawMode::Tiled));
    let sprites = runtime.sprite_instances();
    assert_eq!(sprites.len(), 5);
    let tiles = &sprites[..4];
    for tile in tiles {
        assert_eq!(tile.texture.width, 100);
        assert_eq!(tile.texture.height, 100);
        assert!((tile.model.get(0, 0) - 1.0).abs() < 1e-6);
        assert!((tile.model.get(1, 1) - 1.0).abs() < 1e-6);
    }
    let centres = [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)];
    for (index, (x, y)) in centres.iter().enumerate() {
        assert!((tiles[index].model.get(3, 0) - x).abs() < 1e-6);
        assert!((tiles[index].model.get(3, 1) - y).abs() < 1e-6);
    }

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn scene_sprite_sliced_draw_mode_draws_the_nine_slice_parts()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_scene_sprite_sliced_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 100, [255, 0, 0])?;
    write_sliced_scene(&dir, &texture, [20.0, 20.0, 20.0, 20.0])?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SlicedScene.unity"))?;
    let sprites = runtime.sprite_instances();
    assert_eq!(sprites.len(), 9);
    assert_eq!(sprites[4].texture.width, 60);
    assert_eq!(sprites[4].texture.height, 60);
    assert!(
        !runtime
            .diagnostics()
            .iter()
            .any(|(_, message)| message.contains("draws in the"))
    );

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn sliced_sprite_draws_the_nine_slice_regions_into_the_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_sliced_frame_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_regioned_tga(&texture, 100, 20)?;
    write_sliced_scene(&dir, &texture, [20.0, 20.0, 20.0, 20.0])?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SlicedScene.unity"))?;
    let frame = runtime.capture_scene_frame(512, 512)?;
    assert_eq!(
        frame.image.get_pixel(200, 310),
        [255, 0, 0, 255],
        "the bottom-left corner of a sliced sprite keeps the corner colour"
    );
    assert_eq!(
        frame.image.get_pixel(256, 256),
        [0, 0, 255, 255],
        "the centre of a sliced sprite keeps the centre colour"
    );
    assert_eq!(
        frame.image.get_pixel(256, 310),
        [0, 255, 0, 255],
        "the bottom edge of a sliced sprite keeps the edge colour"
    );

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn sprite_without_an_importable_texture_reports_the_import_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_missing_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let texture = dir.join("sprite.tga");
    write_test_tga(&texture, 100, 50, [255, 0, 0])?;
    write_scene(&dir, &texture)?;

    let mut runtime = Runtime::new()?;
    runtime.load_scene(&dir.join("SpriteScene.unity"))?;
    let entity = runtime.create_runtime_object("ScriptSprite")?;
    assert!(runtime.create_runtime_sprite(entity, Some(&texture)));
    let missing = dir.join("missing.tga");
    let missing_entity = runtime.create_runtime_object("MissingSprite")?;
    assert!(runtime.create_runtime_sprite(missing_entity, Some(&missing)));
    assert!(runtime
        .diagnostics()
        .iter()
        .any(|(id, message)| *id == missing_entity && message.contains("could not be imported")));
    assert_eq!(runtime.sprite_instances().len(), 2);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn sprite_abi_accepts_the_script_calls_and_refuses_invalid_arguments() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let entity = unitas_runtime_object_create(handle, 0, 0);
    assert_ne!(entity, 0);

    assert_eq!(unitas_runtime_sprite_create(handle, entity, 0, 0), 0);
    assert_eq!(
        unitas_runtime_sprite_set_color(handle, entity, 1.0, 0.5, 0.25, 1.0),
        0
    );
    assert_eq!(unitas_runtime_sprite_set_sorting(handle, entity, 2, 3), 0);
    assert_eq!(unitas_runtime_sprite_set_flip(handle, entity, 1, 0), 0);
    assert_eq!(unitas_runtime_sprite_set_enabled(handle, entity, 1), 0);
    assert_eq!(unitas_runtime_sprite_set_draw_mode(handle, entity, 1), 0);

    assert_eq!(unitas_runtime_sprite_create(handle, 0, 0, 0), -2);
    assert_eq!(
        unitas_runtime_sprite_set_color(handle, 0, 1.0, 1.0, 1.0, 1.0),
        -2
    );
    assert_eq!(unitas_runtime_sprite_set_draw_mode(handle, entity, 7), -2);
    assert_eq!(
        unitas_runtime_sprite_set_color(handle, entity + 100000, 1.0, 1.0, 1.0, 1.0),
        -3
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
}
