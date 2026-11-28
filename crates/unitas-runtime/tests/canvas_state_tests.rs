use std::collections::HashMap;

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, CanvasScalerComponent,
    ContentSizeFitMode, LayoutGroupAxis, RectTransformComponent, SpriteDrawMode, UiBinding,
    UiButtonComponent, UiContentSizeFitterComponent, UiElementComponent, UiImageComponent,
    UiLayoutGroupComponent, UiPadding, UiTextComponent,
};
use unitas_core::EntityId;
use unitas_render::image::Rgba8;
use unitas_render::ui_pass::{FOCUS_INDICATOR_COLOR, UiTarget, composite_ui};
use unitas_runtime::canvas::{
    CanvasState, HorizontalWrapMode, TextAnchor, TextLayoutConfig, TextOverride, UiDrawItem,
    VerticalWrapMode,
};
use unitas_runtime::input::{
    GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_BUTTON_DPAD_DOWN, GAMEPAD_BUTTON_DPAD_RIGHT,
    GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_SOUTH, InputState, KEY_DOWN_ARROW, KEY_LEFT_ARROW,
    KEY_LEFT_SHIFT, KEY_RETURN, KEY_RIGHT_ARROW, KEY_SPACE, KEY_TAB, KEY_UP_ARROW,
};

const SCREEN: [f32; 2] = [800.0, 600.0];
const CANVAS_ID: EntityId = EntityId::new(100);
const TITLE_PANEL_ID: EntityId = EntityId::new(200);
const PLAY_BUTTON_ID: EntityId = EntityId::new(210);
const QUIT_BUTTON_ID: EntityId = EntityId::new(220);
const DISABLED_BUTTON_ID: EntityId = EntityId::new(230);
const RESULTS_PANEL_ID: EntityId = EntityId::new(300);
const RETRY_BUTTON_ID: EntityId = EntityId::new(310);
const ARENA_SELECT_BUTTON_ID: EntityId = EntityId::new(320);

fn centered_rect(
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

fn simple_image_element(entity: EntityId, enabled: bool, color: [f32; 4]) -> UiElementComponent {
    UiElementComponent::Image(UiImageComponent {
        entity,
        enabled,
        color,
        sprite_guid: None,
        draw_mode: SpriteDrawMode::Simple,
        sprite_border: [0.0; 4],
    })
}

fn button_element(entity: EntityId, interactable: bool) -> UiElementComponent {
    UiElementComponent::Button(UiButtonComponent {
        entity,
        target_graphic: Some(entity),
        interactable,
    })
}

fn sample_rect_transforms() -> Vec<RectTransformComponent> {
    vec![
        centered_rect(CANVAS_ID, None, [0.0, 0.0], SCREEN),
        centered_rect(TITLE_PANEL_ID, Some(CANVAS_ID), [0.0, 0.0], [420.0, 260.0]),
        centered_rect(
            PLAY_BUTTON_ID,
            Some(TITLE_PANEL_ID),
            [0.0, 10.0],
            [200.0, 48.0],
        ),
        centered_rect(
            QUIT_BUTTON_ID,
            Some(TITLE_PANEL_ID),
            [0.0, -50.0],
            [200.0, 48.0],
        ),
        centered_rect(
            DISABLED_BUTTON_ID,
            Some(TITLE_PANEL_ID),
            [0.0, -110.0],
            [200.0, 48.0],
        ),
        centered_rect(
            RESULTS_PANEL_ID,
            Some(CANVAS_ID),
            [0.0, 0.0],
            [440.0, 320.0],
        ),
        centered_rect(
            RETRY_BUTTON_ID,
            Some(RESULTS_PANEL_ID),
            [-100.0, -70.0],
            [180.0, 44.0],
        ),
        centered_rect(
            ARENA_SELECT_BUTTON_ID,
            Some(RESULTS_PANEL_ID),
            [100.0, -70.0],
            [180.0, 44.0],
        ),
    ]
}

fn sample_canvas() -> CanvasState {
    let binding = UiBinding {
        canvases: vec![CanvasComponent {
            entity: CANVAS_ID,
            render_mode: CanvasRenderMode::ScreenSpaceOverlay,
            sorting_order: 0,
            scaler: Some(CanvasScalerComponent {
                ui_scale_mode: CanvasScaleMode::ScaleWithScreenSize,
                reference_resolution: SCREEN,
                screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
                match_width_or_height: 0.5,
            }),
        }],
        rect_transforms: sample_rect_transforms(),
        elements: vec![
            simple_image_element(TITLE_PANEL_ID, true, [0.08, 0.1, 0.16, 0.88]),
            simple_image_element(PLAY_BUTTON_ID, true, [0.2, 0.65, 0.32, 1.0]),
            button_element(PLAY_BUTTON_ID, true),
            UiElementComponent::Text(UiTextComponent {
                entity: PLAY_BUTTON_ID,
                enabled: true,
                text: "PLAY".to_string(),
                font_size: 22.0,
                color: [1.0, 1.0, 1.0, 1.0],
            }),
            simple_image_element(QUIT_BUTTON_ID, true, [0.55, 0.2, 0.22, 1.0]),
            button_element(QUIT_BUTTON_ID, true),
            simple_image_element(DISABLED_BUTTON_ID, true, [0.3, 0.3, 0.3, 1.0]),
            button_element(DISABLED_BUTTON_ID, false),
            simple_image_element(RESULTS_PANEL_ID, false, [0.08, 0.1, 0.16, 0.92]),
            simple_image_element(RETRY_BUTTON_ID, true, [0.2, 0.65, 0.32, 1.0]),
            button_element(RETRY_BUTTON_ID, true),
            simple_image_element(ARENA_SELECT_BUTTON_ID, true, [0.22, 0.45, 0.72, 1.0]),
            button_element(ARENA_SELECT_BUTTON_ID, true),
        ],
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    };
    CanvasState::new(binding)
}

#[test]
fn focusable_registry_filters_disabled_and_hidden_controls_and_auto_focuses_first() {
    let mut canvas = sample_canvas();
    assert_eq!(
        canvas.focusable_buttons(SCREEN),
        vec![PLAY_BUTTON_ID, QUIT_BUTTON_ID]
    );
    assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));

    canvas.set_element_enabled(TITLE_PANEL_ID, false);
    canvas.set_element_enabled(RESULTS_PANEL_ID, true);
    assert_eq!(
        canvas.focusable_buttons(SCREEN),
        vec![RETRY_BUTTON_ID, ARENA_SELECT_BUTTON_ID]
    );
    assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
}

fn assert_results_panel_navigation(canvas: &mut CanvasState, input: &mut InputState) {
    canvas.set_element_enabled(TITLE_PANEL_ID, false);
    canvas.set_element_enabled(RESULTS_PANEL_ID, true);
    assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));

    input.queue_key_event(KEY_RIGHT_ARROW, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
    input.queue_key_event(KEY_RIGHT_ARROW, false);
    input.begin_frame(1.0 / 60.0);

    input.queue_key_event(KEY_LEFT_ARROW, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
    input.queue_key_event(KEY_LEFT_ARROW, false);
    input.begin_frame(1.0 / 60.0);

    input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
    input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, false);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);

    input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, -0.85);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
    input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 0.0);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);

    input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_UP, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
}

#[test]
fn directional_and_tab_navigation_move_focus_deterministically_across_panels() {
    let mut canvas = sample_canvas();
    let mut input = InputState::with_default_axes();
    input.connect_gamepad(1);

    input.queue_key_event(KEY_DOWN_ARROW, true);
    input.begin_frame(1.0 / 60.0);
    assert!(canvas.handle_focus(&input, SCREEN).is_empty());
    assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
    input.queue_key_event(KEY_DOWN_ARROW, false);
    input.begin_frame(1.0 / 60.0);

    input.queue_key_event(KEY_UP_ARROW, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
    input.queue_key_event(KEY_UP_ARROW, false);
    input.begin_frame(1.0 / 60.0);

    input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
    input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, false);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);

    input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_Y, 0.9);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
    input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_Y, 0.0);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);

    input.queue_key_event(KEY_TAB, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
    input.queue_key_event(KEY_TAB, false);
    input.begin_frame(1.0 / 60.0);

    input.queue_key_event(KEY_LEFT_SHIFT, true);
    input.queue_key_event(KEY_TAB, true);
    input.begin_frame(1.0 / 60.0);
    canvas.handle_focus(&input, SCREEN);
    assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
    input.queue_key_event(KEY_TAB, false);
    input.queue_key_event(KEY_LEFT_SHIFT, false);
    input.begin_frame(1.0 / 60.0);

    assert_results_panel_navigation(&mut canvas, &mut input);
}

#[test]
fn draw_items_include_visible_focus_indicator_border_around_focused_button() {
    let mut canvas = sample_canvas();
    let items = canvas.draw_items(SCREEN);
    let focus_items: Vec<&UiDrawItem> = items
        .iter()
        .filter(|item| item.color == FOCUS_INDICATOR_COLOR)
        .collect();
    assert_eq!(focus_items.len(), 4);
    let (play_rect, _) = canvas
        .element_rect(PLAY_BUTTON_ID, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert_eq!(focus_items[0].rect_px[0], play_rect[0]);
    assert_eq!(focus_items[0].rect_px[1], play_rect[1]);
    assert_eq!(focus_items[0].rect_px[2], play_rect[2]);

    let mut image = Rgba8::new(800, 600);
    let mut target = UiTarget {
        image: &mut image,
        ids: None,
    };
    composite_ui(&mut target, &items, 1, None);
    let border_x = play_rect[0] as usize + 1;
    let border_y = play_rect[1] as usize;
    let pixel_idx = (border_y * 800 + border_x) * 4;
    assert_eq!(image.pixels[pixel_idx], 255);
    assert_eq!(image.pixels[pixel_idx + 1], 230);
    assert_eq!(image.pixels[pixel_idx + 2], 0);

    canvas.set_focused_button(Some(QUIT_BUTTON_ID));
    let moved_items = canvas.draw_items(SCREEN);
    let moved_focus: Vec<&UiDrawItem> = moved_items
        .iter()
        .filter(|item| item.color == FOCUS_INDICATOR_COLOR)
        .collect();
    assert_eq!(moved_focus.len(), 4);
    let (quit_rect, _) = canvas
        .element_rect(QUIT_BUTTON_ID, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert_eq!(moved_focus[0].rect_px[1], quit_rect[1]);
}

#[test]
fn submit_keys_and_gamepad_south_enqueue_click_identical_to_pointer_hit_test() {
    let mut canvas = sample_canvas();
    let mut input = InputState::with_default_axes();
    input.connect_gamepad(1);

    let (play_rect, _) = canvas
        .element_rect(PLAY_BUTTON_ID, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    let pointer_center = (
        play_rect[0] + play_rect[2] * 0.5,
        play_rect[1] + play_rect[3] * 0.5,
    );
    let pointer_hit = canvas.hit_test_button(pointer_center, SCREEN);
    assert_eq!(pointer_hit, Some(PLAY_BUTTON_ID));

    input.queue_key_event(KEY_RETURN, true);
    input.begin_frame(1.0 / 60.0);
    let clicks = canvas.handle_focus(&input, SCREEN);
    assert_eq!(clicks, vec![PLAY_BUTTON_ID]);
    assert_eq!(canvas.take_focused_clicks(), vec![PLAY_BUTTON_ID.as_u64()]);
    input.queue_key_event(KEY_RETURN, false);
    input.begin_frame(1.0 / 60.0);

    input.queue_key_event(KEY_DOWN_ARROW, true);
    input.begin_frame(1.0 / 60.0);
    assert!(canvas.handle_focus(&input, SCREEN).is_empty());
    assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
    input.queue_key_event(KEY_DOWN_ARROW, false);
    input.begin_frame(1.0 / 60.0);

    input.queue_key_event(KEY_SPACE, true);
    input.begin_frame(1.0 / 60.0);
    assert_eq!(canvas.handle_focus(&input, SCREEN), vec![QUIT_BUTTON_ID]);
    assert_eq!(canvas.take_focused_clicks(), vec![QUIT_BUTTON_ID.as_u64()]);
    input.queue_key_event(KEY_SPACE, false);
    input.begin_frame(1.0 / 60.0);

    input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, true);
    input.begin_frame(1.0 / 60.0);
    assert_eq!(canvas.handle_focus(&input, SCREEN), vec![QUIT_BUTTON_ID]);
    assert_eq!(canvas.take_focused_clicks(), vec![QUIT_BUTTON_ID.as_u64()]);
}

#[test]
fn text_override_layout_plumbs_alignment_wrap_and_truncate_clip_into_ui_text() {
    use unitas_render::ui_pass::{GlyphTextPainter, HorizontalAlign, UiElement, VerticalAlign};

    let canvas = sample_canvas();
    let default_items = canvas.draw_items(SCREEN);
    let default_play = default_items
        .iter()
        .find(|item| item.text.is_some())
        .and_then(|item| item.text());
    assert!(default_play.is_some());
    if let Some(ui_text) = default_play {
        assert_eq!(ui_text.horizontal_align, HorizontalAlign::Left);
        assert_eq!(ui_text.vertical_align, VerticalAlign::Top);
        assert!(!ui_text.wrap);
        assert_eq!(ui_text.max_width, None);
        assert_eq!(ui_text.line_spacing, 1.0);
        assert_eq!(ui_text.clip_rect, None);
    }

    let overrides = HashMap::from([(
        PLAY_BUTTON_ID,
        TextOverride {
            text: "START GAME NOW".to_string(),
            font_size: 20.0,
            color: [1.0, 1.0, 1.0, 1.0],
            layout: TextLayoutConfig {
                alignment: TextAnchor::MiddleCenter,
                horizontal_overflow: HorizontalWrapMode::Wrap,
                vertical_overflow: VerticalWrapMode::Truncate,
                line_spacing: 1.25,
            },
        },
    )]);
    let overridden_items = canvas.draw_items_with(SCREEN, &overrides);
    let play_item = overridden_items.iter().find(|item| item.text.is_some());
    assert!(play_item.is_some());
    if let Some(item) = play_item
        && let Some(ui_text) = item.text()
    {
        assert_eq!(ui_text.horizontal_align, HorizontalAlign::Center);
        assert_eq!(ui_text.vertical_align, VerticalAlign::Middle);
        assert!(ui_text.wrap);
        assert_eq!(ui_text.max_width, Some(item.rect_px[2]));
        assert_eq!(ui_text.line_spacing, 1.25);
        assert_eq!(ui_text.clip_rect, Some(item.rect_px));
    }

    let mut painter = GlyphTextPainter::new();
    let mut image = Rgba8::new(800, 600);
    let mut target = UiTarget {
        image: &mut image,
        ids: None,
    };
    composite_ui(&mut target, &overridden_items, 1, Some(&mut painter));
}

fn layout_group_test_binding(
    panel_id: EntityId,
    row_id: EntityId,
    icon_id: EntityId,
    label_id: EntityId,
    footer_id: EntityId,
) -> UiBinding {
    UiBinding {
        canvases: vec![CanvasComponent {
            entity: CANVAS_ID,
            render_mode: CanvasRenderMode::ScreenSpaceOverlay,
            sorting_order: 0,
            scaler: None,
        }],
        rect_transforms: vec![
            centered_rect(CANVAS_ID, None, [0.0, 0.0], SCREEN),
            RectTransformComponent {
                entity: panel_id,
                parent: Some(CANVAS_ID),
                anchor_min: [0.0, 1.0],
                anchor_max: [0.0, 1.0],
                anchored_position: [50.0, -40.0],
                size_delta: [10.0, 10.0],
                pivot: [0.0, 1.0],
            },
            centered_rect(row_id, Some(panel_id), [0.0, 0.0], [10.0, 10.0]),
            centered_rect(icon_id, Some(row_id), [0.0, 0.0], [32.0, 32.0]),
            centered_rect(label_id, Some(row_id), [0.0, 0.0], [80.0, 20.0]),
            centered_rect(footer_id, Some(panel_id), [0.0, 0.0], [100.0, 24.0]),
        ],
        elements: vec![
            simple_image_element(panel_id, true, [0.1, 0.1, 0.1, 1.0]),
            simple_image_element(icon_id, true, [1.0, 0.0, 0.0, 1.0]),
            UiElementComponent::Text(UiTextComponent {
                entity: label_id,
                enabled: true,
                text: "PLAY GAME".to_string(),
                font_size: 20.0,
                color: [1.0, 1.0, 1.0, 1.0],
            }),
            simple_image_element(footer_id, true, [0.0, 0.5, 1.0, 1.0]),
        ],
        layout_groups: layout_group_test_groups(panel_id, row_id),
        content_size_fitters: vec![UiContentSizeFitterComponent {
            entity: panel_id,
            horizontal_fit: ContentSizeFitMode::PreferredSize,
            vertical_fit: ContentSizeFitMode::PreferredSize,
            enabled: true,
        }],
    }
}

fn layout_group_test_groups(panel_id: EntityId, row_id: EntityId) -> Vec<UiLayoutGroupComponent> {
    vec![
        UiLayoutGroupComponent {
            entity: panel_id,
            axis: LayoutGroupAxis::Vertical,
            padding: UiPadding {
                left: 10,
                right: 14,
                top: 8,
                bottom: 12,
            },
            spacing: 6.0,
            child_alignment: 1,
            child_control_width: true,
            child_control_height: true,
            child_force_expand_width: true,
            child_force_expand_height: false,
            enabled: true,
        },
        UiLayoutGroupComponent {
            entity: row_id,
            axis: LayoutGroupAxis::Horizontal,
            padding: UiPadding {
                left: 4,
                right: 4,
                top: 2,
                bottom: 2,
            },
            spacing: 8.0,
            child_alignment: 3,
            child_control_width: true,
            child_control_height: false,
            child_force_expand_width: false,
            child_force_expand_height: false,
            enabled: true,
        },
    ]
}

fn assert_layout_group_overrides(canvas: &CanvasState, panel_id: EntityId, expected_panel_h: f32) {
    canvas.set_layout_group(UiLayoutGroupComponent {
        entity: panel_id,
        axis: LayoutGroupAxis::Vertical,
        padding: UiPadding {
            left: 10,
            right: 14,
            top: 8,
            bottom: 12,
        },
        spacing: 20.0,
        child_alignment: 1,
        child_control_width: false,
        child_control_height: true,
        child_force_expand_width: false,
        child_force_expand_height: false,
        enabled: true,
    });
    let (updated_panel, _) = canvas
        .element_rect(panel_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((updated_panel[3] - (expected_panel_h + 14.0)).abs() < 0.01);

    canvas.set_content_size_fitter(UiContentSizeFitterComponent {
        entity: panel_id,
        horizontal_fit: ContentSizeFitMode::Unconstrained,
        vertical_fit: ContentSizeFitMode::Unconstrained,
        enabled: true,
    });
    let (unconstrained_panel, _) = canvas
        .element_rect(panel_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((unconstrained_panel[2] - 10.0).abs() < 0.01);
    assert!((unconstrained_panel[3] - 10.0).abs() < 0.01);
}

#[test]
fn vertical_and_horizontal_layout_groups_with_content_size_fitter_resolve_unity_child_rects() {
    let panel_id = EntityId::new(200);
    let row_id = EntityId::new(210);
    let icon_id = EntityId::new(211);
    let label_id = EntityId::new(212);
    let footer_id = EntityId::new(220);

    let binding = layout_group_test_binding(panel_id, row_id, icon_id, label_id, footer_id);
    let canvas = CanvasState::new(binding);
    let (label_rect, _) = canvas
        .element_rect(label_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!(label_rect[2] > 50.0);
    let expected_row_pref_w = 4.0 + 32.0 + 8.0 + label_rect[2] + 4.0;
    let expected_row_pref_h = 2.0 + 32.0_f32.max(label_rect[3]) + 2.0;
    let expected_panel_w = 10.0 + expected_row_pref_w.max(100.0) + 14.0;
    let expected_panel_h = 8.0 + expected_row_pref_h + 6.0 + 24.0 + 12.0;

    let (panel_rect, _) = canvas
        .element_rect(panel_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((panel_rect[0] - 50.0).abs() < 0.01);
    assert!((panel_rect[1] - 40.0).abs() < 0.01);
    assert!((panel_rect[2] - expected_panel_w).abs() < 0.01);
    assert!((panel_rect[3] - expected_panel_h).abs() < 0.01);

    let (row_rect, _) = canvas
        .element_rect(row_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((row_rect[0] - 60.0).abs() < 0.01);
    assert!((row_rect[1] - 48.0).abs() < 0.01);
    assert!((row_rect[2] - (expected_panel_w - 24.0)).abs() < 0.01);
    assert!((row_rect[3] - expected_row_pref_h).abs() < 0.01);

    let (icon_rect, _) = canvas
        .element_rect(icon_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((icon_rect[0] - 64.0).abs() < 0.01);
    assert!((icon_rect[2] - 32.0).abs() < 0.01);
    assert!((icon_rect[3] - 32.0).abs() < 0.01);
    assert!((label_rect[0] - (64.0 + 32.0 + 8.0)).abs() < 0.01);

    let (footer_rect, _) = canvas
        .element_rect(footer_id, SCREEN)
        .unwrap_or(([0.0; 4], 1.0));
    assert!((footer_rect[0] - 60.0).abs() < 0.01);
    assert!((footer_rect[1] - (48.0 + expected_row_pref_h + 6.0)).abs() < 0.01);
    assert!((footer_rect[2] - (expected_panel_w - 24.0)).abs() < 0.01);
    assert!((footer_rect[3] - 24.0).abs() < 0.01);

    assert_layout_group_overrides(&canvas, panel_id, expected_panel_h);
}

fn assert_abi_layout_group_and_fitter() {
    let handle = unitas_runtime::unitas_runtime_new();
    assert_ne!(handle, 0);
    let packed_padding = 6u64 | (8u64 << 16) | (10u64 << 32) | (12u64 << 48);
    let obj_id = unitas_runtime::unitas_runtime_object_create(handle, 0, 0);
    assert_ne!(obj_id, 0);
    assert_eq!(
        unitas_runtime::unitas_runtime_ui_layout_group_set(
            handle,
            obj_id,
            1,
            packed_padding,
            4.0,
            1,
            1 | 2 | 4 | 16,
        ),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_ui_layout_group_set(
            handle,
            obj_id,
            9,
            packed_padding,
            4.0,
            1,
            0
        ),
        -2
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_ui_content_size_fitter_set(handle, obj_id, 2, 1, 1),
        0
    );
    assert_eq!(
        unitas_runtime::unitas_runtime_ui_content_size_fitter_set(handle, obj_id, 9, 1, 1),
        -2
    );
    assert_eq!(unitas_runtime::unitas_runtime_destroy(handle), 0);
}

#[test]
fn runtime_ui_and_abi_configure_layout_groups_and_content_size_fitter() {
    let mut runtime = unitas_runtime::Runtime::new().unwrap_or_else(|_| std::process::abort());
    let canvas_id = runtime
        .create_runtime_object("HudCanvas")
        .unwrap_or(EntityId::new(1));
    assert!(runtime.set_ui_layout_group(
        canvas_id.as_u64(),
        LayoutGroupAxis::Vertical,
        UiPadding {
            left: 6,
            right: 8,
            top: 10,
            bottom: 12,
        },
        4.0,
        1,
        1 | 2 | 4 | 16,
    ));
    assert!(runtime.set_ui_content_size_fitter(
        canvas_id.as_u64(),
        ContentSizeFitMode::PreferredSize,
        ContentSizeFitMode::MinSize,
        true,
    ));
    assert!(!runtime.set_ui_layout_group(
        0,
        LayoutGroupAxis::Horizontal,
        UiPadding::default(),
        0.0,
        0,
        0,
    ));
    assert!(!runtime.set_ui_content_size_fitter(
        0,
        ContentSizeFitMode::Unconstrained,
        ContentSizeFitMode::Unconstrained,
        false,
    ));
    assert_abi_layout_group_and_fitter();
}
