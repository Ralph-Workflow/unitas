use std::collections::HashMap;

use unitas_asset::scene::{UiBinding, UiElementComponent};
use unitas_core::EntityId;

use crate::canvas::{
    CanvasState, InputFieldEndEditEvent, InputFieldValueChangedEvent, TextLayoutConfig,
    TextOverride, UiDrawItem, UiInputFieldComponent,
};

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

    pub fn canvas_mut(&mut self) -> &mut CanvasState {
        &mut self.canvas
    }

    pub fn set_input_field(&mut self, field: UiInputFieldComponent, screen_size: [f32; 2]) {
        self.canvas
            .ensure_canvas_and_rect(field.entity, screen_size);
        self.canvas.set_input_field(field);
    }

    pub fn input_field(&self, entity: EntityId) -> Option<&UiInputFieldComponent> {
        self.canvas.input_field(entity)
    }

    pub fn set_input_field_text(&mut self, entity: EntityId, text: &str, notify: bool) -> bool {
        self.canvas.set_input_field_text(entity, text, notify)
    }

    pub fn set_input_field_placeholder(&mut self, entity: EntityId, placeholder: &str) -> bool {
        self.canvas.set_input_field_placeholder(entity, placeholder)
    }

    pub fn set_input_field_interactable(&mut self, entity: EntityId, interactable: bool) -> bool {
        self.canvas
            .set_input_field_interactable(entity, interactable)
    }

    pub fn set_input_field_character_limit(&mut self, entity: EntityId, limit: usize) -> bool {
        self.canvas.set_input_field_character_limit(entity, limit)
    }

    pub fn set_input_field_caret_position(&mut self, entity: EntityId, caret: usize) -> bool {
        self.canvas.set_input_field_caret_position(entity, caret)
    }

    pub fn focus_input_field(&mut self, entity: EntityId, focused: bool) -> bool {
        self.canvas.focus_input_field(entity, focused)
    }

    pub fn hit_input_field(&self, screen_size: [f32; 2], point: (f32, f32)) -> Option<EntityId> {
        self.canvas.hit_test_input_field(point, screen_size)
    }

    pub fn take_input_field_value_changed(&mut self) -> Vec<InputFieldValueChangedEvent> {
        self.canvas.take_input_field_value_changed()
    }

    pub fn take_input_field_end_edit(&mut self) -> Vec<InputFieldEndEditEvent> {
        self.canvas.take_input_field_end_edit()
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
        }) || self
            .canvas
            .hit_test_input_field(point, screen_size)
            .is_some()
    }
}

fn rect_contains(rect_px: [f32; 4], point: (f32, f32)) -> bool {
    let (x, top, width, height) = (rect_px[0], rect_px[1], rect_px[2], rect_px[3]);
    let (px, py) = point;
    px >= x && px < x + width && py >= top && py < top + height
}
