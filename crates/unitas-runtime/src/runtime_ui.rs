use unitas_core::EntityId;

use super::{DEFAULT_UI_REFERENCE_RESOLUTION, Runtime};
use crate::canvas::{TextLayoutConfig, UiDrawItem};

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

    pub fn take_ui_clicks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_ui_clicks)
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
        if pointer.primary_pressed {
            self.ui_pressed_button = hovered;
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
        }
    }
}
