use std::error::Error;

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, CanvasScalerComponent,
    RectTransformComponent, SpriteDrawMode, UiBinding, UiElementComponent, UiImageComponent,
};
use unitas_core::EntityId;
use unitas_render::Rgba8;
use unitas_render::ui_pass::{UiTarget, composite_ui};
use unitas_runtime::canvas::{
    CanvasRect, CanvasState, UiDrawItem, canvas_scale_factor, resolve_canvas_rect,
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
