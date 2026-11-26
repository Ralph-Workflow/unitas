use std::collections::HashMap;

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, RectTransformComponent,
    UiBinding, UiElementComponent,
};
use unitas_core::EntityId;

use crate::ui::UiTextItem;

pub use unitas_render::ui_pass::SpriteRef;

const MAX_HIERARCHY_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct UiDrawItem {
    pub rect_px: [f32; 4],
    pub color: [f32; 4],
    pub sprite: Option<SpriteRef>,
    pub text: Option<UiTextItem>,
}

impl unitas_render::ui_pass::UiElement for UiDrawItem {
    fn rect(&self) -> [f32; 4] {
        self.rect_px
    }

    fn color(&self) -> [f32; 4] {
        self.color
    }

    fn sprite(&self) -> Option<&SpriteRef> {
        self.sprite.as_ref()
    }

    fn text(&self) -> Option<unitas_render::ui_pass::UiText<'_>> {
        self.text
            .as_ref()
            .map(|item| unitas_render::ui_pass::UiText {
                text: &item.text,
                x: item.x,
                y: item.y,
                size_px: item.size_px,
                color: item.color,
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

/// The runtime state of a `Text` component: its content, font size and colour.
#[derive(Clone, Debug, PartialEq)]
pub struct TextOverride {
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
}

pub fn resolve_canvas_rect(rect: &RectTransformComponent, parent: CanvasRect) -> CanvasRect {
    let anchor_size = [
        (rect.anchor_max[0] - rect.anchor_min[0]) * parent.size[0],
        (rect.anchor_max[1] - rect.anchor_min[1]) * parent.size[1],
    ];
    let size = [
        anchor_size[0] + rect.size_delta[0],
        anchor_size[1] + rect.size_delta[1],
    ];
    let pivot = [
        rect.anchor_min[0] * parent.size[0]
            + anchor_size[0] * rect.pivot[0]
            + rect.anchored_position[0],
        rect.anchor_min[1] * parent.size[1]
            + anchor_size[1] * rect.pivot[1]
            + rect.anchored_position[1],
    ];
    CanvasRect {
        min: [
            parent.min[0] + pivot[0] - rect.pivot[0] * size[0],
            parent.min[1] + pivot[1] - rect.pivot[1] * size[1],
        ],
        size,
    }
}

pub fn canvas_scale_factor(canvas: &CanvasComponent, screen_size: [f32; 2]) -> f32 {
    let scaler = match canvas.scaler.as_ref() {
        Some(scaler) => scaler,
        None => return 1.0,
    };
    if scaler.ui_scale_mode != CanvasScaleMode::ScaleWithScreenSize {
        return 1.0;
    }
    let reference = scaler.reference_resolution;
    if reference[0] <= 0.0 || reference[1] <= 0.0 {
        return 1.0;
    }
    let width_ratio = screen_size[0] / reference[0];
    let height_ratio = screen_size[1] / reference[1];
    match scaler.screen_match_mode {
        CanvasMatchMode::MatchWidthOrHeight => {
            if scaler.match_width_or_height < 0.5 {
                width_ratio
            } else if scaler.match_width_or_height > 0.5 {
                height_ratio
            } else {
                (width_ratio * height_ratio).sqrt()
            }
        }
        CanvasMatchMode::Shrink => width_ratio.min(height_ratio),
        CanvasMatchMode::Expand => width_ratio.max(height_ratio),
    }
}

pub struct CanvasState {
    binding: UiBinding,
    rects: HashMap<EntityId, RectTransformComponent>,
}

impl CanvasState {
    pub fn new(binding: UiBinding) -> Self {
        let rects = binding
            .rect_transforms
            .iter()
            .map(|rect| (rect.entity, rect.clone()))
            .collect();
        Self { binding, rects }
    }

    pub fn binding(&self) -> &UiBinding {
        &self.binding
    }

    pub fn draw_items(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        self.draw_items_with(screen_size, &HashMap::new())
    }

    /// The draw items of the visible canvas content, with the runtime text
    /// state applied: `texts` overrides the content, font size and colour of
    /// the `Text` components it names.
    pub fn draw_items_with(
        &self,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Vec<UiDrawItem> {
        let mut items = Vec::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return items;
        }
        for element in &self.binding.elements {
            let (rect_px, scale) = match self.element_rect(element.entity(), screen_size) {
                Some(resolved) => resolved,
                None => continue,
            };
            match element {
                UiElementComponent::Image(image) => {
                    if !image.enabled {
                        continue;
                    }
                    items.push(UiDrawItem {
                        rect_px,
                        color: image.color,
                        sprite: image.sprite_guid.as_ref().map(|guid| SpriteRef {
                            texture_guid: guid.clone(),
                            uv_rect: [0.0, 0.0, 1.0, 1.0],
                        }),
                        text: None,
                    });
                }
                UiElementComponent::Text(text) => {
                    if !text.enabled {
                        continue;
                    }
                    let state = texts.get(&text.entity);
                    let content = state.map_or(text.text.as_str(), |state| state.text.as_str());
                    let color = state.map_or(text.color, |state| state.color);
                    let font_size = state.map_or(text.font_size, |state| state.font_size);
                    items.push(UiDrawItem {
                        rect_px,
                        color,
                        sprite: None,
                        text: Some(UiTextItem {
                            text: content.to_string(),
                            x: rect_px[0],
                            y: rect_px[1],
                            size_px: font_size * scale,
                            color,
                        }),
                    });
                }
                // A button draws through its target graphic, which is an
                // element of its own.
                UiElementComponent::Button(_) => {}
            }
        }
        items
    }

    /// The pixel rectangle of one element under its canvas and the scale of
    /// that canvas, in screen pixels with the origin at the top left.
    pub fn element_rect(&self, entity: EntityId, screen_size: [f32; 2]) -> Option<([f32; 4], f32)> {
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return None;
        }
        let chain = self.chain_to_canvas(entity)?;
        let canvas = self
            .binding
            .canvases
            .iter()
            .find(|canvas| canvas.entity == chain[0])?;
        if canvas.render_mode != CanvasRenderMode::ScreenSpaceOverlay {
            return None;
        }
        let scale = canvas_scale_factor(canvas, screen_size);
        if scale <= 0.0 {
            return None;
        }
        let canvas_size = [screen_size[0] / scale, screen_size[1] / scale];
        let mut rect = CanvasRect {
            min: [0.0, 0.0],
            size: canvas_size,
        };
        for node in chain.iter().skip(1) {
            let transform = match self.rects.get(node) {
                Some(transform) => transform,
                None => continue,
            };
            rect = resolve_canvas_rect(transform, rect);
        }
        Some((canvas_rect_to_pixels(rect, canvas_size, scale), scale))
    }

    fn chain_to_canvas(&self, entity: EntityId) -> Option<Vec<EntityId>> {
        let mut chain = Vec::new();
        let mut current = Some(entity);
        while let Some(node) = current {
            if chain.len() >= MAX_HIERARCHY_DEPTH {
                return None;
            }
            chain.push(node);
            if self
                .binding
                .canvases
                .iter()
                .any(|canvas| canvas.entity == node)
            {
                chain.reverse();
                return Some(chain);
            }
            current = self.rects.get(&node).and_then(|rect| rect.parent);
        }
        None
    }
}

fn canvas_rect_to_pixels(rect: CanvasRect, canvas_size: [f32; 2], scale: f32) -> [f32; 4] {
    let x = rect.min[0] * scale;
    let width = rect.size[0] * scale;
    let top = (canvas_size[1] - rect.min[1] - rect.size[1]) * scale;
    let height = rect.size[1] * scale;
    [x, top, width, height]
}
