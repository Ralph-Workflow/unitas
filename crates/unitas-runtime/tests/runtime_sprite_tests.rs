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
    write_test_tga(&texture, 100, 50, [255, 0, 0])?;
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

    assert!(runtime.set_runtime_sprite_enabled(entity, false));
    assert_eq!(runtime.sprite_instances().len(), 1);
    assert!(runtime.set_runtime_sprite_enabled(entity, true));
    assert_eq!(runtime.sprite_instances().len(), 2);

    assert!(runtime.set_runtime_sprite_flip(entity, false, true));
    let sprites = runtime.sprite_instances();
    assert!((sprites[1].model.get(1, 1) + 0.5).abs() < 1e-6);

    assert!(runtime
        .set_runtime_object_transform(
            entity,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [4.0, 4.0, 1.0],
        )
        .is_ok());
    assert!(runtime.set_runtime_sprite_color(entity, [0.0, 1.0, 0.0, 1.0]));
    assert!(runtime.set_runtime_sprite_sorting(entity, 0, 10));
    let front_frame = runtime.capture_scene_frame(64, 64)?;
    let center_idx = ((32 * 64 + 32) * 3) as usize;
    let front_pixel = &front_frame.image.pixels[center_idx..center_idx + 3];

    assert!(runtime.set_runtime_sprite_color(entity, [0.0, 0.0, 1.0, 1.0]));
    let recolored_frame = runtime.capture_scene_frame(64, 64)?;
    let recolored_pixel = &recolored_frame.image.pixels[center_idx..center_idx + 3];
    assert_ne!(front_pixel, recolored_pixel);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn script_sprite_reports_the_draw_modes_the_renderer_does_not_draw()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_mode_{}",
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

    assert!(runtime.set_runtime_sprite_draw_mode(entity, SpriteDrawMode::Sliced));
    let diagnostics = runtime.diagnostics();
    assert!(diagnostics.iter().any(|(id, message)| *id == entity
        && message.contains("draws in the Sliced mode")
        && message.contains("does not draw yet")));

    assert!(runtime.set_runtime_sprite_draw_mode(entity, SpriteDrawMode::Simple));
    assert!(
        !runtime
            .diagnostics()
            .iter()
            .any(|(id, message)| *id == entity && message.contains("mode"))
    );

    let missing = dir.join("missing.tga");
    let missing_entity = runtime.create_runtime_object("MissingSprite")?;
    assert!(runtime.create_runtime_sprite(missing_entity, Some(&missing)));
    assert!(runtime
        .diagnostics()
        .iter()
        .any(|(id, message)| *id == missing_entity
            && message.contains("could not be imported")));
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

#[test]
fn probe_sprite_frame_layout() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "unitas_runtime_script_sprite_probe_{}",
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
    assert!(runtime.set_runtime_sprite_color(entity, [0.0, 1.0, 0.0, 1.0]));
    assert!(runtime.set_runtime_object_transform(
        entity,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [4.0, 4.0, 1.0],
    ).is_ok());

    let frame = runtime.capture_scene_frame(64, 64)?;
    println!("image {}x{} pixels len {}", frame.image.width, frame.image.height, frame.image.pixels.len());
    let mut seen: std::collections::BTreeMap<Vec<u8>, usize> = Default::default();
    for chunk in frame.image.pixels.chunks_exact(4) {
        *seen.entry(chunk.to_vec()).or_default() += 1;
    }
    for (color, count) in &seen {
        println!("color {color:?} count {count}");
    }
    for y in 0..64usize {
        let mut row = String::new();
        for x in 0..64usize {
            let i = (y * 64 + x) * 4;
            let p = &frame.image.pixels[i..i + 4];
            row.push(if p[0] > p[1] && p[0] > p[2] { 'R' } else if p[1] > p[0] && p[1] > p[2] { 'G' } else if p[2] > p[0] && p[2] > p[1] { 'B' } else { '.' });
        }
        println!("{row}");
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
