use unitas_asset::scene::{
    CanvasComponent, CanvasRenderMode, RectTransformComponent, SpriteDrawMode, UiBinding,
    UiElementComponent, UiImageComponent,
};
use unitas_core::EntityId;
use unitas_render::Rgba8;
use unitas_render::ui_pass::register_ui_sprite_image;
use unitas_runtime::canvas::CanvasState;

const CANVAS_ID: EntityId = EntityId::new(10);
const PANEL_ID: EntityId = EntityId::new(11);
const SPRITE_GUID: &str = "33333333333333333333333333333333";

fn binding(image: UiImageComponent) -> UiBinding {
    UiBinding {
        canvases: vec![CanvasComponent {
            entity: CANVAS_ID,
            render_mode: CanvasRenderMode::ScreenSpaceOverlay,
            sorting_order: 0,
            scaler: None,
        }],
        rect_transforms: vec![
            RectTransformComponent {
                entity: CANVAS_ID,
                parent: None,
                anchor_min: [0.0, 0.0],
                anchor_max: [1.0, 1.0],
                anchored_position: [0.0, 0.0],
                size_delta: [0.0, 0.0],
                pivot: [0.5, 0.5],
            },
            RectTransformComponent {
                entity: PANEL_ID,
                parent: Some(CANVAS_ID),
                anchor_min: [0.0, 0.0],
                anchor_max: [0.0, 0.0],
                anchored_position: [10.0, 20.0],
                size_delta: [100.0, 40.0],
                pivot: [0.0, 0.0],
            },
        ],
        elements: vec![UiElementComponent::Image(image)],
        layout_groups: Vec::new(),
        content_size_fitters: Vec::new(),
    }
}

fn image(draw_mode: SpriteDrawMode, border: [f32; 4]) -> UiImageComponent {
    UiImageComponent {
        entity: PANEL_ID,
        color: [1.0, 1.0, 1.0, 1.0],
        sprite_guid: Some(SPRITE_GUID.to_string()),
        enabled: true,
        draw_mode,
        sprite_border: border,
    }
}

fn expected_sliced_rects() -> Vec<[f32; 4]> {
    vec![
        [10.0, 540.0, 8.0, 8.0],
        [18.0, 540.0, 84.0, 8.0],
        [102.0, 540.0, 8.0, 8.0],
        [10.0, 548.0, 8.0, 24.0],
        [18.0, 548.0, 84.0, 24.0],
        [102.0, 548.0, 8.0, 24.0],
        [10.0, 572.0, 8.0, 8.0],
        [18.0, 572.0, 84.0, 8.0],
        [102.0, 572.0, 8.0, 8.0],
    ]
}

fn expected_sliced_uvs(quarter: f32) -> Vec<[f32; 4]> {
    vec![
        [0.0, 0.0, quarter, quarter],
        [quarter, 0.0, 1.0 - quarter, quarter],
        [1.0 - quarter, 0.0, 1.0, quarter],
        [0.0, quarter, quarter, 1.0 - quarter],
        [quarter, quarter, 1.0 - quarter, 1.0 - quarter],
        [1.0 - quarter, quarter, 1.0, 1.0 - quarter],
        [0.0, 1.0 - quarter, quarter, 1.0],
        [quarter, 1.0 - quarter, 1.0 - quarter, 1.0],
        [1.0 - quarter, 1.0 - quarter, 1.0, 1.0],
    ]
}

#[test]
fn sliced_image_with_an_eight_pixel_border_draws_nine_quads_with_the_border_uvs() {
    let mut sprite = Rgba8::new(32, 32);
    sprite.pixels.fill(255);
    register_ui_sprite_image(SPRITE_GUID, sprite);

    let canvas = CanvasState::new(binding(image(SpriteDrawMode::Sliced, [8.0, 8.0, 8.0, 8.0])));
    let items = canvas.draw_items([800.0, 600.0]);

    assert_eq!(items.len(), 9, "a sliced image draws nine quads");
    let rects: Vec<[f32; 4]> = items.iter().map(|item| item.rect_px).collect();
    let uvs: Vec<[f32; 4]> = items
        .iter()
        .map(|item| {
            item.sprite
                .as_ref()
                .map(|sprite| sprite.uv_rect)
                .unwrap_or_default()
        })
        .collect();

    assert_eq!(
        rects,
        expected_sliced_rects(),
        "the corners keep the border size and the edges and centre stretch"
    );
    assert_eq!(
        uvs,
        expected_sliced_uvs(8.0 / 32.0),
        "the sprite uv region is split by the border"
    );
    assert!(
        items
            .iter()
            .all(|item| item.sprite.is_some() && item.text.is_none()),
        "every sliced quad shows the sprite"
    );
}

#[test]
fn simple_image_stays_one_stretched_quad_and_tiled_repeats_the_sprite() {
    let mut sprite = Rgba8::new(32, 32);
    sprite.pixels.fill(255);
    register_ui_sprite_image(SPRITE_GUID, sprite);

    let simple = CanvasState::new(binding(image(SpriteDrawMode::Simple, [8.0, 8.0, 8.0, 8.0])));
    let items = simple.draw_items([800.0, 600.0]);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].rect_px, [10.0, 540.0, 100.0, 40.0]);
    assert_eq!(
        items[0].sprite.as_ref().map(|sprite| sprite.uv_rect),
        Some([0.0, 0.0, 1.0, 1.0])
    );

    let tiled = CanvasState::new(binding(image(SpriteDrawMode::Tiled, [0.0; 4])));
    let items = tiled.draw_items([800.0, 600.0]);
    assert_eq!(items.len(), 8, "a 100x40 rect holds eight 32x32 tiles");
    assert_eq!(items[0].rect_px, [10.0, 540.0, 32.0, 32.0]);
    assert_eq!(items[1].rect_px, [42.0, 540.0, 32.0, 32.0]);
    assert_eq!(items[2].rect_px, [74.0, 540.0, 32.0, 32.0]);
    assert_eq!(items[5].rect_px, [42.0, 572.0, 32.0, 8.0]);
    assert!(
        items
            .iter()
            .all(|item| item.sprite.as_ref().map(|sprite| sprite.uv_rect)
                == Some([0.0, 0.0, 1.0, 1.0])),
        "every tile shows the whole sprite"
    );
}
