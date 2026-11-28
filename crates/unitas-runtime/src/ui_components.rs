use crate::canvas::{
    CanvasScalerOverride, CanvasState, InputFieldEndEditEvent, InputFieldValueChangedEvent,
    SliderValueChangedEvent, TextLayoutConfig, TextOverride, ToggleValueChangedEvent,
    UiCanvasGroupComponent, UiDrawItem, UiInputFieldComponent, UiSliderComponent,
    UiToggleComponent,
};
use std::collections::HashMap;
use unitas_asset::scene::{UiBinding, UiElementComponent};
use unitas_core::EntityId;
pub struct UiComponents {
    canvas: CanvasState,
    texts: HashMap<EntityId, TextOverride>,
}
impl UiComponents {
    pub fn new(binding: UiBinding) -> Self {
        let mut texts = HashMap::new();
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
                UiElementComponent::Button(_) => {}
                UiElementComponent::Image(_) => {}
            }
        }
        Self {
            canvas: CanvasState::new(binding),
            texts,
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
    pub fn set_toggle(&mut self, toggle: UiToggleComponent, screen_size: [f32; 2]) {
        self.canvas
            .ensure_canvas_and_rect(toggle.entity, screen_size);
        self.canvas.set_toggle(toggle);
    }
    pub fn toggle(&self, entity: EntityId) -> Option<&UiToggleComponent> {
        self.canvas.toggle(entity)
    }
    pub fn set_toggle_value(&mut self, entity: EntityId, is_on: bool, notify: bool) -> bool {
        self.canvas.set_toggle_value(entity, is_on, notify)
    }
    pub fn take_toggle_value_changed(&mut self) -> Vec<ToggleValueChangedEvent> {
        self.canvas.take_toggle_value_changed()
    }
    pub fn set_slider(&mut self, slider: UiSliderComponent, screen_size: [f32; 2]) {
        self.canvas
            .ensure_canvas_and_rect(slider.entity, screen_size);
        self.canvas.set_slider(slider);
    }
    pub fn slider(&self, entity: EntityId) -> Option<&UiSliderComponent> {
        self.canvas.slider(entity)
    }
    pub fn set_slider_value(&mut self, entity: EntityId, value: f32, notify: bool) -> bool {
        self.canvas.set_slider_value(entity, value, notify)
    }
    pub fn take_slider_value_changed(&mut self) -> Vec<SliderValueChangedEvent> {
        self.canvas.take_slider_value_changed()
    }
    pub fn set_rect_mask(&mut self, entity: EntityId, enabled: bool) {
        self.canvas.set_rect_mask(entity, enabled);
    }
    pub fn set_canvas_group(&mut self, group: UiCanvasGroupComponent) -> bool {
        self.canvas.set_canvas_group(group)
    }
    pub fn set_canvas_scaler(&self, scaler: CanvasScalerOverride) -> bool {
        self.canvas.set_canvas_scaler(scaler)
    }
    pub fn set_screen_dpi(&mut self, dpi: f32) {
        self.canvas.set_screen_dpi(dpi);
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
        self.canvas.hit_test_button(point, screen_size)
    }
    pub fn pointer_over(&self, screen_size: [f32; 2], point: (f32, f32)) -> bool {
        self.canvas.binding().elements.iter().any(|element| {
            self.canvas
                .element_receives_pointer(element.entity(), screen_size, point)
        }) || self
            .canvas
            .hit_test_input_field(point, screen_size)
            .is_some()
    }
}
