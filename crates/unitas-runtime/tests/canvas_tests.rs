use std::error::Error;

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, CanvasScalerComponent,
    RectTransformComponent, SpriteDrawMode, UiBinding, UiElementComponent, UiImageComponent,
};
use unitas_core::EntityId;
use unitas_render::Rgba8;
use unitas_render::ui_pass::{GlyphTextPainter, UiTarget, composite_ui};
use unitas_runtime::canvas::{
    CanvasRect, CanvasState, InputFieldEndEditEvent, InputFieldValueChangedEvent, UiDrawItem,
    UiInputFieldComponent, canvas_scale_factor, resolve_canvas_rect,
};
use unitas_runtime::input::{
    InputState, KEY_BACKSPACE, KEY_DELETE, KEY_ESCAPE, KEY_LEFT_ARROW, KEY_RETURN, KEY_RIGHT_ARROW,
};

const UI_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: HudCanvas
  m_Layer: 5
--- !u!224 &2
RectTransform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!223 &3
Canvas:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_RenderMode: 0
  m_SortingOrder: 7
--- !u!114 &4
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 800, y: 600}
  m_ScreenMatchMode: 0
  m_MatchWidthOrHeight: 0.5
--- !u!1 &5
GameObject:
  m_Name: HealthBar
  m_Layer: 5
--- !u!224 &6
RectTransform:
  m_GameObject: {fileID: 5}
  m_Father: {fileID: 2}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: -120, y: 30}
  m_SizeDelta: {x: 200, y: 40}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &7
MonoBehaviour:
  m_GameObject: {fileID: 5}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 1, g: 0.5, b: 0.25, a: 0.75}
  m_Sprite: {fileID: 21300000, guid: 22222222222222222222222222222222, type: 3}
--- !u!1 &8
GameObject:
  m_Name: ScoreLabel
  m_Layer: 5
--- !u!224 &9
RectTransform:
  m_GameObject: {fileID: 8}
  m_Father: {fileID: 2}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 16, y: -12}
  m_SizeDelta: {x: 320, y: 28}
  m_Pivot: {x: 0, y: 1}
--- !u!114 &10
MonoBehaviour:
  m_GameObject: {fileID: 8}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: \"Score 0\"
  m_FontSize: 18
  m_Color: {r: 1, g: 1, b: 1, a: 1}
";

fn rect_transform(
    anchor_min: [f32; 2],
    anchor_max: [f32; 2],
    anchored_position: [f32; 2],
    size_delta: [f32; 2],
    pivot: [f32; 2],
) -> RectTransformComponent {
    RectTransformComponent {
        entity: EntityId::new(1),
        parent: None,
        anchor_min,
        anchor_max,
        anchored_position,
        size_delta,
        pivot,
    }
}

fn canvas() -> CanvasComponent {
    CanvasComponent {
        entity: EntityId::new(0),
        render_mode: CanvasRenderMode::ScreenSpaceOverlay,
        sorting_order: 0,
        scaler: Some(CanvasScalerComponent {
            ui_scale_mode: CanvasScaleMode::ScaleWithScreenSize,
            reference_resolution: [800.0, 600.0],
            screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
            match_width_or_height: 0.5,
        }),
    }
}

#[test]
fn canvas_anchor_math_matches_unity_layout() {
    let parent = CanvasRect {
        min: [0.0, 0.0],
        size: [800.0, 600.0],
    };
    let centered = resolve_canvas_rect(
        &rect_transform(
            [0.5, 0.5],
            [0.5, 0.5],
            [10.0, 20.0],
            [100.0, 50.0],
            [0.5, 0.5],
        ),
        parent,
    );
    assert_eq!(centered.size, [100.0, 50.0]);
    assert_eq!(centered.min, [360.0, 295.0]);
    let stretched = resolve_canvas_rect(
        &rect_transform([0.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 0.0], [0.5, 0.5]),
        parent,
    );
    assert_eq!(stretched.size, [800.0, 600.0]);
    assert_eq!(stretched.min, [0.0, 0.0]);
    let top_left = resolve_canvas_rect(
        &rect_transform(
            [0.0, 1.0],
            [0.0, 1.0],
            [16.0, -12.0],
            [320.0, 28.0],
            [0.0, 1.0],
        ),
        parent,
    );
    assert_eq!(top_left.size, [320.0, 28.0]);
    assert_eq!(top_left.min, [16.0, 560.0]);
}

#[test]
fn canvas_scale_factor_follows_the_unity_match_rule() {
    let canvas = canvas();
    assert_eq!(canvas_scale_factor(&canvas, [1600.0, 1200.0]), 2.0);
    assert_eq!(canvas_scale_factor(&canvas, [800.0, 600.0]), 1.0);
    let mut width = canvas.clone();
    let scaler = width.scaler.as_mut().ok_or("no scaler");
    if let Ok(scaler) = scaler {
        scaler.match_width_or_height = 0.0;
    }
    assert_eq!(canvas_scale_factor(&width, [1600.0, 600.0]), 2.0);
}

#[test]
fn canvas_draw_items_follow_the_scaler_and_the_anchors() -> Result<(), Box<dyn Error>> {
    let stream = unitas_asset::unity_yaml::parse(UI_SCENE)?;
    let assembled = unitas_asset::scene::assemble_scene_bundle(&stream)?;
    let state = CanvasState::new(assembled.ui);
    let items = state.draw_items([1600.0, 1200.0]);
    assert_eq!(items.len(), 2);
    let bar = &items[0];
    assert_eq!(bar.rect_px, [360.0, 500.0, 400.0, 80.0]);
    assert_eq!(bar.color, [1.0, 0.5, 0.25, 0.75]);
    assert_eq!(
        bar.sprite
            .as_ref()
            .map(|sprite| sprite.texture_guid.as_str()),
        Some("22222222222222222222222222222222")
    );
    assert!(bar.text.is_none());
    let label = &items[1];
    assert_eq!(label.rect_px, [32.0, 24.0, 640.0, 56.0]);
    let text = label.text.as_ref().ok_or("the label has no text")?;
    assert_eq!(text.text, "Score 0");
    assert_eq!(text.x, 32.0);
    assert_eq!(text.y, 24.0);
    assert_eq!(text.size_px, 36.0);
    Ok(())
}

#[test]
fn canvas_fixture_composites_ui_pixels_and_object_ids() -> Result<(), Box<dyn Error>> {
    let stream = unitas_asset::unity_yaml::parse(UI_SCENE)?;
    let assembled = unitas_asset::scene::assemble_scene_bundle(&stream)?;
    let items = CanvasState::new(assembled.ui).draw_items([800.0, 600.0]);
    let mut image = Rgba8::new(800, 600);
    let mut ids = vec![0u32; 800 * 600];
    {
        let mut target = UiTarget {
            image: &mut image,
            ids: Some(&mut ids),
        };
        let coverages = composite_ui(&mut target, &items, 1, None);
        assert_eq!(coverages.len(), 2);
        assert_eq!(coverages[0].id, 1);
        assert_eq!(coverages[0].width, 200);
        assert_eq!(coverages[0].height, 40);
    }
    let center = (270 * 800 + 270) * 4;
    assert_eq!(&image.pixels[center..center + 3], &[191, 96, 48]);
    assert_eq!(ids[270 * 800 + 270], 1);
    let label_center = (26 * 800 + 176) * 4;
    assert_eq!(&image.pixels[label_center..label_center + 3], &[0, 0, 0]);
    assert_eq!(ids[26 * 800 + 176], 2);
    Ok(())
}

#[test]
fn canvas_binding_without_a_canvas_yields_no_items() {
    let binding = UiBinding {
        canvases: Vec::new(),
        rect_transforms: vec![rect_transform(
            [0.5, 0.5],
            [0.5, 0.5],
            [0.0, 0.0],
            [100.0, 50.0],
            [0.5, 0.5],
        )],
        elements: vec![UiElementComponent::Image(UiImageComponent {
            entity: EntityId::new(1),
            color: [1.0, 1.0, 1.0, 1.0],
            sprite_guid: None,
            draw_mode: SpriteDrawMode::Simple,
            sprite_border: [0.0; 4],
            enabled: true,
        })],
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    };
    let items: Vec<UiDrawItem> = CanvasState::new(binding).draw_items([800.0, 600.0]);
    assert!(items.is_empty());
}

#[test]
fn canvas_disabled_elements_are_skipped() -> Result<(), Box<dyn Error>> {
    let stream = unitas_asset::unity_yaml::parse(UI_SCENE)?;
    let mut assembled = unitas_asset::scene::assemble_scene_bundle(&stream)?;
    if let UiElementComponent::Image(image) = &mut assembled.ui.elements[0] {
        image.enabled = false;
    }
    let items = CanvasState::new(assembled.ui).draw_items([800.0, 600.0]);
    assert_eq!(items.len(), 1);
    assert!(items[0].text.is_some());
    Ok(())
}

fn tap_focus_key(state: &mut CanvasState, input: &mut InputState, screen_size: [f32; 2], key: u32) {
    input.queue_key_event(key, true);
    input.begin_frame(1.0 / 60.0);
    state.handle_focus(input, screen_size);
    input.queue_key_event(key, false);
}

fn assert_input_field_initial_typing(
    state: &mut CanvasState,
    input: &mut InputState,
    field_entity: EntityId,
    screen_size: [f32; 2],
) {
    for ch in ['A', 'B', 'D', 'E'] {
        input.queue_char_event(ch);
    }
    input.begin_frame(1.0 / 60.0);
    state.handle_focus(input, screen_size);

    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABDE")
    );
    assert_eq!(
        state.input_field(field_entity).map(|f| f.caret_position),
        Some(4)
    );
    assert_eq!(
        state.take_input_field_value_changed(),
        ["A", "AB", "ABD", "ABDE"]
            .into_iter()
            .map(|t| InputFieldValueChangedEvent {
                entity: field_entity,
                text: String::from(t),
            })
            .collect::<Vec<_>>()
    );
}

fn assert_input_field_navigation_and_deletion(
    state: &mut CanvasState,
    input: &mut InputState,
    field_entity: EntityId,
    screen_size: [f32; 2],
) {
    tap_focus_key(state, input, screen_size, KEY_LEFT_ARROW);
    tap_focus_key(state, input, screen_size, KEY_LEFT_ARROW);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.caret_position),
        Some(2)
    );

    input.queue_char_event('C');
    input.queue_char_event('X');
    input.begin_frame(1.0 / 60.0);
    state.handle_focus(input, screen_size);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABCDE")
    );
    assert_eq!(
        state.input_field(field_entity).map(|f| f.caret_position),
        Some(3)
    );
    assert_eq!(
        state.take_input_field_value_changed(),
        vec![InputFieldValueChangedEvent {
            entity: field_entity,
            text: String::from("ABCDE"),
        }]
    );

    tap_focus_key(state, input, screen_size, KEY_BACKSPACE);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABDE")
    );
    assert_eq!(
        state.input_field(field_entity).map(|f| f.caret_position),
        Some(2)
    );

    tap_focus_key(state, input, screen_size, KEY_DELETE);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABE")
    );

    tap_focus_key(state, input, screen_size, KEY_RIGHT_ARROW);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.caret_position),
        Some(3)
    );
    let _ = state.take_input_field_value_changed();
}

#[test]
fn input_field_handles_char_entry_caret_movement_deletion_and_submit_cancel_events() {
    let screen_size = [400.0, 200.0];
    let field_entity = EntityId::new(42);
    let mut state = CanvasState::new(UiBinding {
        canvases: Vec::new(),
        rect_transforms: Vec::new(),
        elements: Vec::new(),
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    });
    state.ensure_canvas_and_rect(field_entity, screen_size);
    state.set_input_field(UiInputFieldComponent {
        entity: field_entity,
        placeholder: String::from("Enter name..."),
        character_limit: 5,
        ..UiInputFieldComponent::default()
    });
    assert!(state.focus_input_field(field_entity, true));

    let mut input = InputState::new();
    assert_input_field_initial_typing(&mut state, &mut input, field_entity, screen_size);
    assert_input_field_navigation_and_deletion(&mut state, &mut input, field_entity, screen_size);

    tap_focus_key(&mut state, &mut input, screen_size, KEY_RETURN);
    assert_eq!(
        state.take_input_field_end_edit(),
        vec![InputFieldEndEditEvent {
            entity: field_entity,
            text: String::from("ABE"),
            canceled: false,
        }]
    );

    input.queue_char_event('Z');
    input.begin_frame(1.0 / 60.0);
    state.handle_focus(&input, screen_size);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABEZ")
    );
    let _ = state.take_input_field_value_changed();

    tap_focus_key(&mut state, &mut input, screen_size, KEY_ESCAPE);
    assert_eq!(
        state.input_field(field_entity).map(|f| f.text.as_str()),
        Some("ABE")
    );
    assert_eq!(
        state.input_field(field_entity).map(|f| f.was_canceled),
        Some(true)
    );
    assert_eq!(
        state.take_input_field_value_changed(),
        vec![InputFieldValueChangedEvent {
            entity: field_entity,
            text: String::from("ABE"),
        }]
    );
    assert_eq!(
        state.take_input_field_end_edit(),
        vec![InputFieldEndEditEvent {
            entity: field_entity,
            text: String::from("ABE"),
            canceled: true,
        }]
    );
}

fn composite_ui_image(items: &[UiDrawItem], painter: &mut GlyphTextPainter) -> Rgba8 {
    let mut img = Rgba8::new(320, 120);
    let mut target = UiTarget {
        image: &mut img,
        ids: None,
    };
    composite_ui(&mut target, items, 1, Some(painter));
    img
}

#[test]
fn input_field_headless_canvas_capture_renders_entered_text_and_caret() {
    let screen_size = [320.0, 120.0];
    let field_entity = EntityId::new(11);
    let button_entity = EntityId::new(22);
    let mut state = CanvasState::new(UiBinding {
        canvases: Vec::new(),
        rect_transforms: Vec::new(),
        elements: vec![UiElementComponent::Button(
            unitas_asset::scene::UiButtonComponent {
                entity: button_entity,
                interactable: true,
                target_graphic: None,
            },
        )],
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    });
    state.ensure_canvas_and_rect(field_entity, screen_size);
    state.ensure_canvas_and_rect(button_entity, screen_size);
    state.set_input_field(UiInputFieldComponent {
        entity: field_entity,
        placeholder: String::from("Type here"),
        ..UiInputFieldComponent::default()
    });
    assert!(state.focus_input_field(field_entity, true));

    let mut painter = GlyphTextPainter::new();
    let empty_focused_img = composite_ui_image(&state.draw_items(screen_size), &mut painter);

    let mut input = InputState::new();
    for ch in "Hello".chars() {
        input.queue_char_event(ch);
    }
    input.begin_frame(1.0 / 60.0);
    state.handle_focus(&input, screen_size);

    let typed_focused_items = state.draw_items(screen_size);
    assert!(typed_focused_items.len() >= 7);
    let caret_item = &typed_focused_items[2];
    assert!(caret_item.text.is_none());
    assert_eq!(caret_item.color, [1.0, 1.0, 1.0, 1.0]);

    let typed_focused_img = composite_ui_image(&typed_focused_items, &mut painter);
    assert_ne!(empty_focused_img.pixels, typed_focused_img.pixels);
    let caret_cx = (caret_item.rect_px[0] + caret_item.rect_px[2] * 0.5) as u32;
    let caret_cy = (caret_item.rect_px[1] + caret_item.rect_px[3] * 0.5) as u32;
    assert_eq!(
        typed_focused_img.get_pixel(caret_cx, caret_cy),
        [255, 255, 255, 255]
    );

    let text_rect = typed_focused_items[1].rect_px;
    let glyph_x0 = text_rect[0].floor() as u32;
    let glyph_x1 = (caret_item.rect_px[0].floor() as u32).saturating_sub(1);
    let glyph_y0 = text_rect[1].floor() as u32;
    let glyph_y1 = (text_rect[1] + text_rect[3]).ceil() as u32;
    assert!((glyph_y0..glyph_y1).any(|y| {
        (glyph_x0..glyph_x1).any(|x| {
            let px = typed_focused_img.get_pixel(x, y);
            px[0] > 100 && px[1] > 100 && px[2] > 100
        })
    }));

    state.set_focused_button(Some(button_entity));
    let unfocused_img = composite_ui_image(&state.draw_items(screen_size), &mut painter);
    assert_ne!(
        unfocused_img.get_pixel(caret_cx, caret_cy),
        [255, 255, 255, 255]
    );
}

fn centered_transform(
    entity: EntityId,
    parent: Option<EntityId>,
    pos: [f32; 2],
    size: [f32; 2],
) -> RectTransformComponent {
    RectTransformComponent {
        entity,
        parent,
        anchor_min: [0.5, 0.5],
        anchor_max: [0.5, 0.5],
        anchored_position: pos,
        size_delta: size,
        pivot: [0.5, 0.5],
    }
}

const PANEL_ID: EntityId = EntityId::new(510);
const WIDE_IMAGE_ID: EntityId = EntityId::new(520);
const CLIPPED_BUTTON_ID: EntityId = EntityId::new(530);
const PANEL_BUTTON_ID: EntityId = EntityId::new(540);
const FREE_BUTTON_ID: EntityId = EntityId::new(550);
const PANEL_RECT: [f32; 4] = [300.0, 200.0, 200.0, 200.0];
const WIDE_IMAGE_RECT: [f32; 4] = [200.0, 100.0, 400.0, 400.0];
const CLIPPED_BUTTON_RECT: [f32; 4] = [320.0, 120.0, 160.0, 40.0];
const PANEL_BUTTON_RECT: [f32; 4] = [320.0, 340.0, 160.0, 40.0];

fn plain_image(color: [f32; 4]) -> UiElementComponent {
    UiElementComponent::Image(UiImageComponent {
        entity: EntityId::new(0),
        color,
        sprite_guid: None,
        draw_mode: SpriteDrawMode::Simple,
        sprite_border: [0.0; 4],
        enabled: true,
    })
}

fn panel_binding() -> UiBinding {
    let canvas_id = EntityId::new(500);
    let set_entity = |element: &mut UiElementComponent, entity: EntityId| match element {
        UiElementComponent::Image(image) => image.entity = entity,
        UiElementComponent::Text(text) => text.entity = entity,
        UiElementComponent::Button(button) => button.entity = entity,
    };
    let mut panel_image = plain_image([0.1, 0.1, 0.1, 1.0]);
    set_entity(&mut panel_image, PANEL_ID);
    let mut wide_image = plain_image([0.9, 0.2, 0.2, 1.0]);
    set_entity(&mut wide_image, WIDE_IMAGE_ID);
    let mut clipped_face = plain_image([0.2, 0.4, 0.6, 1.0]);
    set_entity(&mut clipped_face, CLIPPED_BUTTON_ID);
    let mut clipped_button = UiElementComponent::Button(unitas_asset::scene::UiButtonComponent {
        entity: CLIPPED_BUTTON_ID,
        target_graphic: None,
        interactable: true,
    });
    set_entity(&mut clipped_button, CLIPPED_BUTTON_ID);
    let mut panel_face = plain_image([0.2, 0.6, 0.4, 1.0]);
    set_entity(&mut panel_face, PANEL_BUTTON_ID);
    let mut panel_button = UiElementComponent::Button(unitas_asset::scene::UiButtonComponent {
        entity: PANEL_BUTTON_ID,
        target_graphic: None,
        interactable: true,
    });
    set_entity(&mut panel_button, PANEL_BUTTON_ID);
    let mut free_face = plain_image([0.6, 0.4, 0.2, 1.0]);
    set_entity(&mut free_face, FREE_BUTTON_ID);
    let mut free_button = UiElementComponent::Button(unitas_asset::scene::UiButtonComponent {
        entity: FREE_BUTTON_ID,
        target_graphic: None,
        interactable: true,
    });
    set_entity(&mut free_button, FREE_BUTTON_ID);
    UiBinding {
        canvases: vec![CanvasComponent {
            entity: canvas_id,
            render_mode: CanvasRenderMode::ScreenSpaceOverlay,
            sorting_order: 0,
            scaler: None,
        }],
        rect_transforms: vec![
            centered_transform(canvas_id, None, [0.0, 0.0], [800.0, 600.0]),
            centered_transform(PANEL_ID, Some(canvas_id), [0.0, 0.0], [200.0, 200.0]),
            centered_transform(WIDE_IMAGE_ID, Some(PANEL_ID), [0.0, 0.0], [400.0, 400.0]),
            centered_transform(
                CLIPPED_BUTTON_ID,
                Some(WIDE_IMAGE_ID),
                [0.0, 160.0],
                [160.0, 40.0],
            ),
            centered_transform(PANEL_BUTTON_ID, Some(PANEL_ID), [0.0, -60.0], [160.0, 40.0]),
            centered_transform(
                FREE_BUTTON_ID,
                Some(canvas_id),
                [0.0, -220.0],
                [160.0, 40.0],
            ),
        ],
        elements: vec![
            panel_image,
            wide_image,
            clipped_face,
            clipped_button,
            panel_face,
            panel_button,
            free_face,
            free_button,
        ],
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    }
}

#[test]
fn match_width_or_height_interpolates_ratios_in_logarithmic_space() {
    let base = canvas();
    let mut scaler = base.scaler.clone().unwrap_or(CanvasScalerComponent {
        ui_scale_mode: CanvasScaleMode::ScaleWithScreenSize,
        reference_resolution: [800.0, 600.0],
        screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
        match_width_or_height: 0.5,
    });
    scaler.match_width_or_height = 0.25;
    let quarter = CanvasComponent {
        scaler: Some(scaler.clone()),
        ..base.clone()
    };
    let expected = 2f32.powf(0.75);
    assert!(
        (canvas_scale_factor(&quarter, [1600.0, 600.0]) - expected).abs() < 1e-5,
        "a quarter match blends a quarter of the way from the width ratio to the height ratio in log space"
    );
    scaler.match_width_or_height = 1.0;
    let height = CanvasComponent {
        scaler: Some(scaler),
        ..base
    };
    assert_eq!(canvas_scale_factor(&height, [1600.0, 600.0]), 1.0);
    let mut shrink = canvas();
    if let Some(scaler) = shrink.scaler.as_mut() {
        scaler.screen_match_mode = CanvasMatchMode::Shrink;
    }
    assert_eq!(canvas_scale_factor(&shrink, [1600.0, 600.0]), 1.0);
    let mut expand = canvas();
    if let Some(scaler) = expand.scaler.as_mut() {
        scaler.screen_match_mode = CanvasMatchMode::Expand;
    }
    assert_eq!(canvas_scale_factor(&expand, [1600.0, 600.0]), 2.0);
}

#[test]
fn canvas_scaler_override_constant_pixel_size_scales_layout() {
    let state = CanvasState::new(panel_binding());
    state.set_canvas_scaler(unitas_runtime::canvas::CanvasScalerOverride {
        entity: EntityId::new(500),
        ui_scale_mode: CanvasScaleMode::ConstantPixelSize,
        scale_factor: 2.0,
        reference_resolution: [800.0, 600.0],
        screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
        match_width_or_height: 0.5,
        physical_unit: unitas_runtime::canvas::PhysicalUnit::Inches,
        fallback_dpi: 96.0,
    });
    let (rect, scale) = state
        .element_rect(WIDE_IMAGE_ID, [800.0, 600.0])
        .unwrap_or(([0.0; 4], 1.0));
    assert_eq!(scale, 2.0);
    assert_eq!(rect, [0.0, -100.0, 800.0, 800.0]);
}

#[test]
fn canvas_scaler_override_constant_physical_size_uses_dpi_then_fallback() {
    let mut state = CanvasState::new(panel_binding());
    state.set_canvas_scaler(unitas_runtime::canvas::CanvasScalerOverride {
        entity: EntityId::new(500),
        ui_scale_mode: CanvasScaleMode::ConstantPhysicalSize,
        scale_factor: 1.0,
        reference_resolution: [800.0, 600.0],
        screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
        match_width_or_height: 0.5,
        physical_unit: unitas_runtime::canvas::PhysicalUnit::Centimeters,
        fallback_dpi: 96.0,
    });
    let (_, fallback_scale) = state
        .element_rect(WIDE_IMAGE_ID, [800.0, 600.0])
        .unwrap_or(([0.0; 4], 1.0));
    assert!(
        (fallback_scale - 96.0 / 2.54).abs() < 1e-3,
        "with no reported screen dpi the fallback dpi drives the physical scale"
    );
    state.set_screen_dpi(254.0);
    let (_, dpi_scale) = state
        .element_rect(WIDE_IMAGE_ID, [800.0, 600.0])
        .unwrap_or(([0.0; 4], 1.0));
    assert!(
        (dpi_scale - 100.0).abs() < 1e-3,
        "a reported screen dpi overrides the fallback"
    );
}

#[test]
fn toggles_flip_and_groups_keep_a_single_toggle_on() {
    let mut binding = panel_binding();
    let first = EntityId::new(600);
    let second = EntityId::new(610);
    binding.rect_transforms.push(centered_transform(
        first,
        Some(EntityId::new(500)),
        [0.0, 100.0],
        [240.0, 36.0],
    ));
    binding.rect_transforms.push(centered_transform(
        second,
        Some(EntityId::new(500)),
        [0.0, -100.0],
        [240.0, 36.0],
    ));
    let mut state = CanvasState::new(binding);
    for entity in [first, second] {
        state.set_toggle(unitas_runtime::canvas::UiToggleComponent {
            entity,
            group: Some(EntityId::new(700)),
            ..unitas_runtime::canvas::UiToggleComponent::default()
        });
    }
    assert!(state.set_toggle_value(first, true, true));
    assert!(state.toggle(first).is_some_and(|toggle| toggle.is_on));
    assert!(state.set_toggle_value(second, true, true));
    assert!(
        state.toggle(first).is_some_and(|toggle| !toggle.is_on),
        "turning one grouped toggle on turns the others off"
    );
    assert_eq!(
        state.take_toggle_value_changed(),
        vec![
            unitas_runtime::canvas::ToggleValueChangedEvent {
                entity: first,
                is_on: true,
            },
            unitas_runtime::canvas::ToggleValueChangedEvent {
                entity: second,
                is_on: true,
            },
        ],
        "group exclusivity silences the toggle that was forced off"
    );
    let press = state
        .element_rect(first, [800.0, 600.0])
        .map(|(rect, _)| (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5))
        .unwrap_or((0.0, 0.0));
    state.press_pointer(press, [800.0, 600.0]);
    state.release_pointer(press, [800.0, 600.0]);
    assert!(
        state.toggle(first).is_some_and(|toggle| toggle.is_on),
        "a pointer press and release over a toggle flips it"
    );
}

#[test]
fn sliders_clamp_round_and_follow_the_pointer_while_dragged() {
    let mut state = CanvasState::new(panel_binding());
    let slider_id = EntityId::new(620);
    state.ensure_canvas_and_rect(slider_id, [800.0, 600.0]);
    state.set_slider(unitas_runtime::canvas::UiSliderComponent {
        entity: slider_id,
        min_value: 0.0,
        max_value: 10.0,
        whole_numbers: true,
        ..unitas_runtime::canvas::UiSliderComponent::default()
    });
    assert!(state.set_slider_value(slider_id, 7.6, false));
    assert_eq!(
        state.slider(slider_id).map(|slider| slider.value),
        Some(8.0),
        "whole number sliders round their value"
    );
    let (rect, _) = state
        .element_rect(slider_id, [800.0, 600.0])
        .unwrap_or(([0.0; 4], 1.0));
    let left = (rect[0], rect[1] + rect[3] * 0.5);
    let middle = (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
    let right = (rect[0] + rect[2], rect[1] + rect[3] * 0.5);
    state.press_pointer(left, [800.0, 600.0]);
    assert_eq!(state.slider(slider_id).map(|s| s.value), Some(0.0));
    state.move_pointer(middle, [800.0, 600.0]);
    assert_eq!(
        state.slider(slider_id).map(|s| s.value),
        Some(5.0),
        "dragging across the slider track maps the pointer to the value"
    );
    state.release_pointer(right, [800.0, 600.0]);
    assert_eq!(state.slider(slider_id).map(|s| s.value), Some(10.0));
    assert_eq!(
        state.take_slider_value_changed().len(),
        3,
        "each drag step notifies once"
    );
}

#[test]
fn rect_masks_clip_draw_items_and_pointer_hits_to_the_mask_area() {
    let mut state = CanvasState::new(panel_binding());
    let outside_point = (CLIPPED_BUTTON_RECT[0] + 20.0, CLIPPED_BUTTON_RECT[1] + 20.0);
    let inside_point = (PANEL_BUTTON_RECT[0] + 20.0, PANEL_BUTTON_RECT[1] + 20.0);
    assert_eq!(
        state
            .element_rect(WIDE_IMAGE_ID, [800.0, 600.0])
            .map(|(rect, _)| rect),
        Some(WIDE_IMAGE_RECT)
    );
    assert_eq!(
        state.hit_test_button(outside_point, [800.0, 600.0]),
        Some(CLIPPED_BUTTON_ID),
        "before masking, the child button answers hits outside the panel area"
    );
    state.set_rect_mask(PANEL_ID, true);
    let items = state.draw_items([800.0, 600.0]);
    let masked_item = items
        .iter()
        .find(|item| item.rect_px == WIDE_IMAGE_RECT)
        .ok_or("the masked image still draws");
    if let Ok(item) = masked_item {
        assert_eq!(
            item.clip_rect,
            Some(PANEL_RECT),
            "the draw item carries the mask rect"
        );
    }
    let mut image = Rgba8::new(800, 600);
    let mut target = UiTarget {
        image: &mut image,
        ids: None,
    };
    composite_ui(&mut target, &items, 1, None);
    let outside_pixel = ((outside_point.1 as u32) * 800 + outside_point.0 as u32) as usize * 4;
    assert_eq!(
        &image.pixels[outside_pixel..outside_pixel + 3],
        &[0, 0, 0],
        "pixels outside the mask area stay untouched"
    );
    assert_eq!(
        state.hit_test_button(inside_point, [800.0, 600.0]),
        Some(PANEL_BUTTON_ID),
        "hits inside the mask area still land"
    );
    assert_eq!(
        state.hit_test_button(outside_point, [800.0, 600.0]),
        None,
        "hits outside the mask area are clipped away"
    );
    assert_eq!(
        state.hit_test_button((400.0, 520.0), [800.0, 600.0]),
        Some(FREE_BUTTON_ID),
        "unmasked siblings keep their full hit area"
    );
}

#[test]
fn canvas_groups_multiply_alpha_and_gate_interaction_down_the_hierarchy() {
    let mut state = CanvasState::new(panel_binding());
    let panel_button_center = (PANEL_BUTTON_RECT[0] + 20.0, PANEL_BUTTON_RECT[1] + 20.0);
    state.set_canvas_group(unitas_runtime::canvas::UiCanvasGroupComponent {
        entity: PANEL_ID,
        alpha: 0.5,
        ..unitas_runtime::canvas::UiCanvasGroupComponent::default()
    });
    let items = state.draw_items([800.0, 600.0]);
    let panel_item = items
        .iter()
        .find(|item| item.rect_px == PANEL_RECT)
        .ok_or("the panel draws");
    if let Ok(item) = panel_item {
        assert_eq!(item.color[3], 0.5, "the group alpha multiplies the panel");
    }
    let child_item = items
        .iter()
        .find(|item| item.rect_px == WIDE_IMAGE_RECT)
        .ok_or("the child draws");
    if let Ok(item) = child_item {
        assert_eq!(item.color[3], 0.5, "the group alpha reaches descendants");
    }
    assert_eq!(
        state.hit_test_button(panel_button_center, [800.0, 600.0]),
        Some(PANEL_BUTTON_ID)
    );
    state.set_canvas_group(unitas_runtime::canvas::UiCanvasGroupComponent {
        entity: PANEL_ID,
        alpha: 0.5,
        interactable: false,
        ..unitas_runtime::canvas::UiCanvasGroupComponent::default()
    });
    assert_eq!(
        state.hit_test_button(panel_button_center, [800.0, 600.0]),
        None,
        "a non-interactable group disables its descendants"
    );
    assert_eq!(
        state.focusable_buttons([800.0, 600.0]),
        vec![FREE_BUTTON_ID],
        "a non-interactable group removes only its descendants from focus"
    );
    state.set_canvas_group(unitas_runtime::canvas::UiCanvasGroupComponent {
        entity: PANEL_BUTTON_ID,
        ignore_parent_groups: true,
        ..unitas_runtime::canvas::UiCanvasGroupComponent::default()
    });
    assert_eq!(
        state.hit_test_button(panel_button_center, [800.0, 600.0]),
        Some(PANEL_BUTTON_ID),
        "a nested group that ignores parent groups restores interaction"
    );
    state.set_canvas_group(unitas_runtime::canvas::UiCanvasGroupComponent {
        entity: PANEL_ID,
        alpha: 0.5,
        blocks_raycasts: false,
        ..unitas_runtime::canvas::UiCanvasGroupComponent::default()
    });
    assert_eq!(
        state.hit_test_button((400.0, 140.0), [800.0, 600.0]),
        None,
        "a group that blocks no raycasts makes descendants transparent to hits"
    );
    assert_eq!(
        state.hit_test_button(panel_button_center, [800.0, 600.0]),
        Some(PANEL_BUTTON_ID),
        "the group that ignores parent groups keeps its hits"
    );
    assert_eq!(
        state.hit_test_button((400.0, 520.0), [800.0, 600.0]),
        Some(FREE_BUTTON_ID),
        "siblings outside every group keep their hits"
    );
}
