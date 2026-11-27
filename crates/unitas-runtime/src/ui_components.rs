use std::collections::HashMap;

use unitas_asset::scene::{UiBinding, UiElementComponent};
use unitas_core::EntityId;

use crate::canvas::{CanvasState, TextLayoutConfig, TextOverride, UiDrawItem};

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonState {
    pub interactable: bool,
}

pub struct UiComponents {
    canvas: CanvasState,
    texts: HashMap<EntityId, TextOverride>,
    buttons: HashMap<EntityId, ButtonState>,
}

impl UiComponents {
    pub fn new(binding: UiBinding) -> Self {
        let mut texts = HashMap::new();
        let mut buttons = HashMap::new();
        for element in &binding.elements {
            match element {
                UiElementComponent::Text(text) => {
                    texts.insert(
                        text.entity,
                        TextOverride {
                            text: text.text.clone(),
                            font_size: text.font_size,
                            color: text.color,
                            layout: TextLayoutConfig::default(),
                        },
                    );
                }
                UiElementComponent::Button(button) => {
                    buttons.insert(
                        button.entity,
                        ButtonState {
                            interactable: button.interactable,
                        },
                    );
                }
                UiElementComponent::Image(_) => {}
            }
        }
        Self {
            canvas: CanvasState::new(binding),
            texts,
            buttons,
        }
    }

    pub fn canvas(&self) -> &CanvasState {
        &self.canvas
    }

    pub fn set_text(&mut self, entity: EntityId, text: &str) -> bool {
        match self.texts.get_mut(&entity) {
            Some(state) => {
                state.text = text.to_string();
                true
            }
            None => false,
        }
    }

    pub fn set_style(&mut self, entity: EntityId, font_size: f32, color: [f32; 4]) -> bool {
        match self.texts.get_mut(&entity) {
            Some(state) => {
                state.font_size = font_size;
                state.color = color;
                true
            }
            None => false,
        }
    }

    pub fn set_layout(&mut self, entity: EntityId, layout: TextLayoutConfig) -> bool {
        match self.texts.get_mut(&entity) {
            Some(state) => {
                state.layout = layout;
                true
            }
            None => false,
        }
    }

    pub fn draw_items(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        self.canvas.draw_items_with(screen_size, &self.texts)
    }

    pub fn hit_button(&self, screen_size: [f32; 2], point: (f32, f32)) -> Option<EntityId> {
        for element in self.canvas.binding().elements.iter().rev() {
            let UiElementComponent::Button(button) = element else {
                continue;
            };
            let Some(state) = self.buttons.get(&button.entity) else {
                continue;
            };
            if !state.interactable {
                continue;
            }
            let Some((rect_px, _)) = self.canvas.element_rect(button.entity, screen_size) else {
                continue;
            };
            if rect_contains(rect_px, point) {
                return Some(button.entity);
            }
        }
        None
    }

    pub fn pointer_over(&self, screen_size: [f32; 2], point: (f32, f32)) -> bool {
        self.canvas.binding().elements.iter().any(|element| {
            self.canvas
                .element_rect(element.entity(), screen_size)
                .is_some_and(|(rect_px, _)| rect_contains(rect_px, point))
        })
    }
}

fn rect_contains(rect_px: [f32; 4], point: (f32, f32)) -> bool {
    let (x, top, width, height) = (rect_px[0], rect_px[1], rect_px[2], rect_px[3]);
    let (px, py) = point;
    px >= x && px < x + width && py >= top && py < top + height
}
