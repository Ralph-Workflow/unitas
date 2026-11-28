//! The native UI components of a loaded scene: the canvas content they draw,
//! the text they take and the clicks they dispatch.

use std::error::Error;
use std::path::{Path, PathBuf};

use unitas_runtime::input::{KEY_RETURN, KEY_TAB};
use unitas_runtime::{
    PRIMARY_MOUSE_BUTTON, Runtime, test_read_path_buffer, test_read_render_buffer,
    test_write_path_buffer, unitas_runtime_input_mouse_button_event,
    unitas_runtime_input_pointer_move, unitas_runtime_load_scene, unitas_runtime_new,
    unitas_runtime_path_buffer, unitas_runtime_render_buffer, unitas_runtime_render_camera_create,
    unitas_runtime_render_frame, unitas_runtime_tick, unitas_runtime_ui_pointer_over,
    unitas_runtime_ui_poll_clicks, unitas_runtime_ui_text_set,
};

/// A scene with a canvas, a text element and a button. The canvas root is
/// 800 by 600 reference pixels, the label sits in the top left corner and the
/// button in the middle of the canvas.
const UI_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: HudCanvas
  m_Layer: 5
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!224 &3
RectTransform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!223 &4
Canvas:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_RenderMode: 0
  m_SortingOrder: 0
--- !u!114 &5
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 800, y: 600}
  m_ScreenMatchMode: 0
  m_MatchWidthOrHeight: 0.5
--- !u!1 &10
GameObject:
  m_Name: ScoreLabel
  m_Layer: 5
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &12
RectTransform:
  m_GameObject: {fileID: 10}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 16, y: -12}
  m_SizeDelta: {x: 320, y: 28}
  m_Pivot: {x: 0, y: 1}
--- !u!114 &13
MonoBehaviour:
  m_GameObject: {fileID: 10}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: \"Score 0\"
  m_FontSize: 18
  m_Color: {r: 1, g: 1, b: 1, a: 1}
--- !u!1 &20
GameObject:
  m_Name: PlayButton
  m_Layer: 5
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &22
RectTransform:
  m_GameObject: {fileID: 20}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 200, y: 60}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!1 &30
GameObject:
  m_Name: ButtonFace
  m_Layer: 5
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 21}
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 30}
  m_Father: {fileID: 22}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &33
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.9, g: 0.9, b: 0.9, a: 1}
--- !u!114 &34
MonoBehaviour:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}
  m_TargetGraphic: {fileID: 33}
  m_Interactable: 1
";

/// The middle of the button, in screen pixels.
const BUTTON_CENTER: (f64, f64) = (400.0, 300.0);
const FRAME_SIZE: u32 = 256;

const TICK_DELTA: f64 = 0.016;

fn write_scene(label: &str) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!(
        "unitas_runtime_ui_components_{label}_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)?;
    let scene_path = temp_dir.join("ui_components.unity");
    std::fs::write(&scene_path, UI_SCENE)?;
    Ok((temp_dir, scene_path))
}

fn entity_id(runtime: &Runtime, name: &str) -> Result<u64, Box<dyn Error>> {
    let graph = runtime
        .scene_graph()
        .ok_or(String::from("the scene holds no graph"))?;
    for id in graph.breadth_first_order() {
        if let Some(entity) = graph.entity(id)
            && entity.name == name
        {
            return Ok(entity.id.as_u64());
        }
    }
    Err(String::from("the scene holds no object with that name").into())
}

fn entity_id_for(scene_path: &Path, name: &str) -> Result<u64, Box<dyn Error>> {
    let mut runtime = Runtime::new()?;
    runtime.load_scene(scene_path)?;
    entity_id(&runtime, name)
}

#[test]
fn the_rendered_frame_carries_the_canvas_content() -> Result<(), Box<dyn Error>> {
    let (_temp_dir, scene_path) = write_scene("frame")?;
    let scene = scene_path
        .to_str()
        .ok_or(String::from("non-utf8 temp path"))?
        .to_string();
    let canvas_entity = entity_id_for(&scene_path, "HudCanvas")?;
    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let address = unitas_runtime_path_buffer(handle, 512);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, scene.as_bytes())?;
    if unitas_runtime_load_scene(handle, address, scene.len() as u64) != 0 {
        return Err(String::from("the scene did not load").into());
    }
    let camera = unitas_runtime_render_camera_create(handle, canvas_entity);
    if camera == 0 {
        return Err(String::from("camera creation failed").into());
    }
    let count = unitas_runtime_render_frame(handle, camera, FRAME_SIZE, FRAME_SIZE);
    if count < 0 {
        return Err(format!("the render frame failed with code {count}").into());
    }
    let render_address = unitas_runtime_render_buffer(handle, count as u64);
    if render_address == 0 {
        return Err(String::from("render buffer allocation failed").into());
    }
    let mut bytes = vec![0u8; count as usize];
    test_read_render_buffer(handle, &mut bytes)?;

    let painted = painted_pixels(&bytes);
    assert!(
        painted > 0,
        "the canvas content must reach the rendered frame, not only the runtime state"
    );
    Ok(())
}

/// The number of pixels of a 24 bit BMP that carry something other than the
/// black background.
fn painted_pixels(bytes: &[u8]) -> usize {
    let read = |offset: usize| -> usize {
        usize::from(bytes[offset])
            | (usize::from(bytes[offset + 1]) << 8)
            | (usize::from(bytes[offset + 2]) << 16)
    };
    let data_offset = read(10);
    let width = read(18);
    let height = read(22);
    let stride = (width * 3).div_ceil(4) * 4;
    let mut painted = 0;
    for row in 0..height {
        let start = data_offset + row * stride;
        for column in 0..width {
            let pixel = start + column * 3;
            if bytes[pixel] != 0 || bytes[pixel + 1] != 0 || bytes[pixel + 2] != 0 {
                painted += 1;
            }
        }
    }
    painted
}

#[test]
fn the_canvas_draws_its_text_and_takes_a_new_content() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene("draw")?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    let label = entity_id(&runtime, "ScoreLabel")?;

    let items = runtime.ui_draw_items();
    let text = items
        .iter()
        .find_map(|item| item.text.as_ref())
        .ok_or(String::from("the canvas draws no text"))?;
    assert_eq!(text.text, "Score 0");
    assert_eq!(text.size_px, 18.0);

    assert!(runtime.set_ui_text(label, "Score 42"));
    let items = runtime.ui_draw_items();
    let text = items
        .iter()
        .find_map(|item| item.text.as_ref())
        .ok_or(String::from("the canvas draws no text"))?;
    assert_eq!(text.text, "Score 42");

    assert!(runtime.set_ui_text_style(label, 24.0, [1.0, 0.5, 0.25, 1.0]));
    let items = runtime.ui_draw_items();
    let text = items
        .iter()
        .find_map(|item| item.text.as_ref())
        .ok_or(String::from("the canvas draws no text"))?;
    assert_eq!(text.size_px, 24.0);
    assert_eq!(text.color, [1.0, 0.5, 0.25, 1.0]);

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn a_pointer_click_on_a_button_is_dispatched() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene("click")?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    let button = entity_id(&runtime, "PlayButton")?;

    runtime.tick(TICK_DELTA)?;
    runtime.input_pointer_move(BUTTON_CENTER.0, BUTTON_CENTER.1);
    runtime.input_mouse_button_event(PRIMARY_MOUSE_BUTTON, true);
    runtime.tick(TICK_DELTA)?;
    assert!(
        runtime.take_ui_clicks().is_empty(),
        "a press alone must not click the button"
    );

    runtime.input_mouse_button_event(PRIMARY_MOUSE_BUTTON, false);
    runtime.tick(TICK_DELTA)?;
    assert_eq!(runtime.take_ui_clicks(), vec![button]);

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn a_click_away_from_the_button_is_not_dispatched() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene("miss")?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;

    runtime.tick(TICK_DELTA)?;
    runtime.input_pointer_move(10.0, 500.0);
    runtime.input_mouse_button_event(PRIMARY_MOUSE_BUTTON, true);
    runtime.tick(TICK_DELTA)?;
    runtime.input_mouse_button_event(PRIMARY_MOUSE_BUTTON, false);
    runtime.tick(TICK_DELTA)?;
    assert!(runtime.take_ui_clicks().is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn keyboard_focus_and_enter_submit_the_focused_button() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene("focus")?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    let button = entity_id(&runtime, "PlayButton")?;

    runtime.tick(TICK_DELTA)?;
    runtime.input_key_event(KEY_TAB, true);
    runtime.tick(TICK_DELTA)?;
    runtime.input_key_event(KEY_TAB, false);
    runtime.tick(TICK_DELTA)?;
    assert!(
        runtime.take_ui_clicks().is_empty(),
        "moving the focus onto a button must not click it"
    );

    runtime.input_key_event(KEY_RETURN, true);
    runtime.tick(TICK_DELTA)?;
    runtime.input_key_event(KEY_RETURN, false);
    runtime.tick(TICK_DELTA)?;
    assert_eq!(runtime.take_ui_clicks(), vec![button]);

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn the_ui_abi_sets_the_text_and_reports_the_click() -> Result<(), Box<dyn Error>> {
    let (temp_dir, scene_path) = write_scene("abi")?;
    let scene = scene_path
        .to_str()
        .ok_or(String::from("non-utf8 temp path"))?
        .to_string();
    let label = entity_id_for(&scene_path, "ScoreLabel")?;
    let button = entity_id_for(&scene_path, "PlayButton")?;

    let handle = unitas_runtime_new();
    if handle == 0 {
        return Err(String::from("runtime handle creation failed").into());
    }
    let address = unitas_runtime_path_buffer(handle, 512);
    if address == 0 {
        return Err(String::from("path buffer allocation failed").into());
    }
    test_write_path_buffer(handle, scene.as_bytes())?;
    assert_eq!(
        unitas_runtime_load_scene(handle, address, scene.len() as u64),
        0
    );

    test_write_path_buffer(handle, b"Ready")?;
    assert_eq!(unitas_runtime_ui_text_set(handle, label, address, 5), 0);
    assert_eq!(
        unitas_runtime_ui_text_set(handle, button, address, 5),
        -4,
        "a button holds no text"
    );

    assert_eq!(unitas_runtime_tick(handle, TICK_DELTA), 0);
    unitas_runtime_input_pointer_move(handle, BUTTON_CENTER.0, BUTTON_CENTER.1);
    assert_eq!(unitas_runtime_tick(handle, TICK_DELTA), 0);
    assert_eq!(unitas_runtime_ui_pointer_over(handle), 1);

    unitas_runtime_input_mouse_button_event(handle, PRIMARY_MOUSE_BUTTON, 1);
    assert_eq!(unitas_runtime_tick(handle, TICK_DELTA), 0);
    unitas_runtime_input_mouse_button_event(handle, PRIMARY_MOUSE_BUTTON, 0);
    assert_eq!(unitas_runtime_tick(handle, TICK_DELTA), 0);

    assert_eq!(unitas_runtime_ui_poll_clicks(handle, 0, 16), 1);
    let mut clicked = [0u8; 8];
    test_read_path_buffer(handle, &mut clicked)?;
    assert_eq!(u64::from_le_bytes(clicked), button);
    assert_eq!(
        unitas_runtime_ui_poll_clicks(handle, 0, 16),
        0,
        "the clicks are consumed by the poll"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}
