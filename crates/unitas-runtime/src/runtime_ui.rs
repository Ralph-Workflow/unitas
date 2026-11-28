use unitas_asset::scene::{
    ContentSizeFitMode, LayoutGroupAxis, UiContentSizeFitterComponent, UiLayoutGroupComponent,
    UiPadding,
};
use unitas_core::EntityId;

use super::{DEFAULT_UI_REFERENCE_RESOLUTION, Runtime};
use crate::canvas::{
    CanvasScalerOverride, InputFieldEndEditEvent, InputFieldValueChangedEvent,
    SliderValueChangedEvent, TextLayoutConfig, ToggleValueChangedEvent, UiCanvasGroupComponent,
    UiDrawItem, UiInputFieldComponent, UiSliderComponent, UiToggleComponent,
};

impl Runtime {
    pub fn ui_screen_size(&self) -> [f32; 2] {
        if let Some(size) = self.ui_screen_size {
            return size;
        }
        if let Some((width, height)) = self.render_viewport() {
            return [width as f32, height as f32];
        }
        self.ui_reference_resolution()
    }

    pub fn set_ui_screen_size(&mut self, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.ui_screen_size = Some([width, height]);
        }
    }

    pub fn ui_draw_items(&self) -> Vec<UiDrawItem> {
        self.ui_draw_items_for(self.ui_screen_size())
    }

    pub(crate) fn ui_draw_items_for(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        let mut items = Vec::new();
        for scene in &self.scenes {
            items.extend(scene.ui.draw_items(screen_size));
        }
        items
    }

    pub fn ui_pointer_over(&self) -> bool {
        let screen_size = self.ui_screen_size();
        let (x, y) = self.input.pointer_frame().position;
        self.scenes
            .iter()
            .rev()
            .any(|scene| scene.ui.pointer_over(screen_size, (x, y)))
    }

    pub fn set_ui_text(&mut self, object_id: u64, text: &str) -> bool {
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_text(entity, text))
    }

    pub fn set_ui_text_style(&mut self, object_id: u64, font_size: f32, color: [f32; 4]) -> bool {
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_style(entity, font_size, color))
    }

    pub fn set_ui_text_layout(&mut self, object_id: u64, layout: TextLayoutConfig) -> bool {
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_layout(entity, layout))
    }

    pub fn set_ui_layout_group(
        &mut self,
        object_id: u64,
        axis: LayoutGroupAxis,
        padding: UiPadding,
        spacing: f32,
        child_alignment: u32,
        flags: u32,
    ) -> bool {
        if object_id == 0 || !spacing.is_finite() || child_alignment > 8 {
            return false;
        }
        let entity = EntityId::new(object_id);
        let group = UiLayoutGroupComponent {
            entity,
            axis,
            padding,
            spacing,
            child_alignment,
            child_control_width: (flags & 1) != 0,
            child_control_height: (flags & 2) != 0,
            child_force_expand_width: (flags & 4) != 0,
            child_force_expand_height: (flags & 8) != 0,
            enabled: (flags & 16) != 0,
        };
        let scene_idx = self
            .scenes
            .iter()
            .position(|s| {
                s.graph.contains(entity)
                    || s.ui
                        .canvas()
                        .binding()
                        .rect_transforms
                        .iter()
                        .any(|r| r.entity == entity)
            })
            .or(self.active_scene);
        let Some(idx) = scene_idx else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.canvas().set_layout_group(group)
    }

    pub fn set_ui_content_size_fitter(
        &mut self,
        object_id: u64,
        horizontal_fit: ContentSizeFitMode,
        vertical_fit: ContentSizeFitMode,
        enabled: bool,
    ) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        let fitter = UiContentSizeFitterComponent {
            entity,
            horizontal_fit,
            vertical_fit,
            enabled,
        };
        let scene_idx = self
            .scenes
            .iter()
            .position(|s| {
                s.graph.contains(entity)
                    || s.ui
                        .canvas()
                        .binding()
                        .rect_transforms
                        .iter()
                        .any(|r| r.entity == entity)
            })
            .or(self.active_scene);
        let Some(idx) = scene_idx else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.canvas().set_content_size_fitter(fitter)
    }

    pub fn take_ui_clicks(&mut self) -> Vec<u64> {
        let mut clicks = std::mem::take(&mut self.pending_ui_clicks);
        for scene in &mut self.scenes {
            clicks.extend(scene.ui.canvas_mut().take_focused_clicks());
        }
        clicks
    }

    fn resolve_ui_scene_index(&self, entity: EntityId) -> Option<usize> {
        self.scenes
            .iter()
            .position(|s| {
                s.graph.contains(entity)
                    || s.ui.input_field(entity).is_some()
                    || s.ui
                        .canvas()
                        .binding()
                        .rect_transforms
                        .iter()
                        .any(|r| r.entity == entity)
            })
            .or(self.active_scene)
    }

    pub fn set_ui_input_field(&mut self, field: UiInputFieldComponent) -> bool {
        if field.entity.as_u64() == 0 {
            return false;
        }
        let screen_size = self.ui_screen_size();
        let Some(idx) = self.resolve_ui_scene_index(field.entity) else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_input_field(field, screen_size);
        true
    }
    pub fn set_ui_toggle(&mut self, toggle: UiToggleComponent) -> bool {
        if toggle.entity.as_u64() == 0 {
            return false;
        }
        let screen_size = self.ui_screen_size();
        let Some(idx) = self.resolve_ui_scene_index(toggle.entity) else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_toggle(toggle, screen_size);
        true
    }
    pub fn ui_toggle(&self, object_id: u64) -> Option<&UiToggleComponent> {
        if object_id == 0 {
            return None;
        }
        let entity = EntityId::new(object_id);
        self.scenes.iter().find_map(|scene| scene.ui.toggle(entity))
    }
    pub fn set_ui_toggle_value(&mut self, object_id: u64, is_on: bool, notify: bool) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_toggle_value(entity, is_on, notify))
    }
    pub fn take_ui_toggle_value_changed(&mut self) -> Vec<ToggleValueChangedEvent> {
        let mut events = Vec::new();
        for scene in &mut self.scenes {
            events.extend(scene.ui.take_toggle_value_changed());
        }
        events
    }
    pub fn set_ui_slider(&mut self, slider: UiSliderComponent) -> bool {
        if slider.entity.as_u64() == 0 {
            return false;
        }
        let screen_size = self.ui_screen_size();
        let Some(idx) = self.resolve_ui_scene_index(slider.entity) else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_slider(slider, screen_size);
        true
    }
    pub fn ui_slider(&self, object_id: u64) -> Option<&UiSliderComponent> {
        if object_id == 0 {
            return None;
        }
        let entity = EntityId::new(object_id);
        self.scenes.iter().find_map(|scene| scene.ui.slider(entity))
    }
    pub fn set_ui_slider_value(&mut self, object_id: u64, value: f32, notify: bool) -> bool {
        if object_id == 0 || !value.is_finite() {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_slider_value(entity, value, notify))
    }
    pub fn take_ui_slider_value_changed(&mut self) -> Vec<SliderValueChangedEvent> {
        let mut events = Vec::new();
        for scene in &mut self.scenes {
            events.extend(scene.ui.take_slider_value_changed());
        }
        events
    }
    pub fn set_ui_canvas_scaler(&mut self, scaler: CanvasScalerOverride) -> bool {
        let entity = scaler.entity;
        let scene_idx = self
            .scenes
            .iter()
            .position(|s| {
                s.ui.canvas()
                    .binding()
                    .canvases
                    .iter()
                    .any(|c| c.entity == entity)
            })
            .or(self.active_scene);
        let Some(idx) = scene_idx else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_canvas_scaler(scaler)
    }
    pub fn set_ui_canvas_group(&mut self, group: UiCanvasGroupComponent) -> bool {
        let entity = group.entity;
        let scene_idx = self
            .scenes
            .iter()
            .position(|s| {
                s.graph.contains(entity)
                    || s.ui
                        .canvas()
                        .binding()
                        .rect_transforms
                        .iter()
                        .any(|r| r.entity == entity)
            })
            .or(self.active_scene);
        let Some(idx) = scene_idx else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_canvas_group(group)
    }
    pub fn set_ui_rect_mask(&mut self, object_id: u64, enabled: bool) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        let scene_idx = self
            .scenes
            .iter()
            .position(|s| {
                s.ui.canvas()
                    .binding()
                    .rect_transforms
                    .iter()
                    .any(|r| r.entity == entity)
            })
            .or(self.active_scene);
        let Some(idx) = scene_idx else {
            return false;
        };
        let Some(scene) = self.scenes.get_mut(idx) else {
            return false;
        };
        scene.ui.set_rect_mask(entity, enabled);
        true
    }
    pub fn set_ui_screen_dpi(&mut self, dpi: f32) {
        if !dpi.is_finite() || dpi < 0.0 {
            return;
        }
        for scene in &mut self.scenes {
            scene.ui.set_screen_dpi(dpi);
        }
    }

    pub fn ui_input_field(&self, object_id: u64) -> Option<&UiInputFieldComponent> {
        if object_id == 0 {
            return None;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter()
            .find_map(|scene| scene.ui.input_field(entity))
    }

    pub fn set_ui_input_field_text(&mut self, object_id: u64, text: &str, notify: bool) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_input_field_text(entity, text, notify))
    }

    pub fn set_ui_input_field_placeholder(&mut self, object_id: u64, placeholder: &str) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_input_field_placeholder(entity, placeholder))
    }

    pub fn set_ui_input_field_interactable(&mut self, object_id: u64, interactable: bool) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_input_field_interactable(entity, interactable))
    }

    pub fn set_ui_input_field_character_limit(&mut self, object_id: u64, limit: usize) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_input_field_character_limit(entity, limit))
    }

    pub fn set_ui_input_field_caret_position(&mut self, object_id: u64, caret: usize) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.set_input_field_caret_position(entity, caret))
    }

    pub fn focus_ui_input_field(&mut self, object_id: u64, focused: bool) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        self.scenes
            .iter_mut()
            .any(|scene| scene.ui.focus_input_field(entity, focused))
    }

    pub fn is_ui_input_field_focused(&self, object_id: u64) -> bool {
        if object_id == 0 {
            return false;
        }
        let entity = EntityId::new(object_id);
        let screen_size = self.ui_screen_size();
        self.scenes.iter().any(|scene| {
            scene.ui.input_field(entity).is_some()
                && scene.ui.canvas().focused_button(screen_size) == Some(entity)
        })
    }

    pub fn take_ui_input_field_value_changed(&mut self) -> Vec<InputFieldValueChangedEvent> {
        let mut events = Vec::new();
        for scene in &mut self.scenes {
            events.extend(scene.ui.take_input_field_value_changed());
        }
        events
    }

    pub fn take_ui_input_field_end_edit(&mut self) -> Vec<InputFieldEndEditEvent> {
        let mut events = Vec::new();
        for scene in &mut self.scenes {
            events.extend(scene.ui.take_input_field_end_edit());
        }
        events
    }

    fn ui_reference_resolution(&self) -> [f32; 2] {
        for scene in &self.scenes {
            for canvas in &scene.ui.canvas().binding().canvases {
                if let Some(scaler) = &canvas.scaler {
                    return scaler.reference_resolution;
                }
            }
        }
        DEFAULT_UI_REFERENCE_RESOLUTION
    }

    pub(super) fn route_ui_pointer(&mut self) {
        let pointer = self.input.pointer_frame();
        let screen_size = self.ui_screen_size();
        let hovered = self
            .scenes
            .iter()
            .rev()
            .find_map(|scene| scene.ui.hit_button(screen_size, pointer.position));
        let hovered_input_field = self
            .scenes
            .iter()
            .rev()
            .find_map(|scene| scene.ui.hit_input_field(screen_size, pointer.position));
        if pointer.primary_pressed {
            self.ui_pressed_button = hovered;
            if let Some(field_entity) = hovered_input_field {
                for scene in &mut self.scenes {
                    if scene.ui.input_field(field_entity).is_some() {
                        let _ = scene.ui.focus_input_field(field_entity, true);
                    }
                }
            }
            for scene in &mut self.scenes {
                scene
                    .ui
                    .canvas_mut()
                    .press_pointer(pointer.position, screen_size);
            }
        }
        for scene in &mut self.scenes {
            scene
                .ui
                .canvas_mut()
                .move_pointer(pointer.position, screen_size);
        }
        if self.ui_pressed_button.is_some() && self.ui_pressed_button != hovered {
            self.ui_pressed_button = None;
        }
        if pointer.primary_released {
            if let Some(pressed) = self.ui_pressed_button
                && hovered == Some(pressed)
            {
                self.pending_ui_clicks.push(pressed.as_u64());
            }
            self.ui_pressed_button = None;
            for scene in &mut self.scenes {
                scene
                    .ui
                    .canvas_mut()
                    .release_pointer(pointer.position, screen_size);
            }
        }
        let mut active_scene_idx = None;
        for (idx, scene) in self.scenes.iter().enumerate() {
            let focusables = scene.ui.canvas().focusable_buttons(screen_size);
            if !focusables.is_empty() {
                active_scene_idx = Some(idx);
                break;
            }
        }
        if let Some(idx) = active_scene_idx
            && let Some(scene) = self.scenes.get_mut(idx)
        {
            let _ = scene.ui.canvas_mut().handle_focus(&self.input, screen_size);
        }
    }
}
