use super::Runtime;
use crate::input::InputState;

impl Runtime {
    pub fn input(&self) -> &InputState {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    pub fn input_key_event(&mut self, key: u32, pressed: bool) {
        self.input.queue_key_event(key, pressed);
    }

    pub fn input_mouse_button_event(&mut self, button: u32, pressed: bool) {
        self.input.queue_mouse_button_event(button, pressed);
    }

    pub fn input_pointer_move(&mut self, x: f64, y: f64) {
        self.input.queue_pointer_move(x, y);
    }

    pub fn input_scroll(&mut self, x: f64, y: f64) {
        self.input.queue_scroll(x, y);
    }

    pub fn input_axis_event(&mut self, name: &str, value: f64) -> bool {
        self.input.inject_axis(name, value)
    }

    pub fn input_injected_axis(&self, name: &str) -> Option<f64> {
        self.input.injected_axis(name)
    }

    pub fn input_key_state(&self, key: u32) -> u32 {
        self.input.key_state(key)
    }

    pub fn input_mouse_state(&self, button: u32) -> u32 {
        self.input.mouse_button_state(button)
    }

    pub fn input_pointer(&self) -> (f64, f64, f64, f64) {
        self.input.pointer()
    }

    pub fn input_scroll_state(&self) -> (f64, f64) {
        self.input.scroll()
    }

    pub fn input_axis(&self, name: &str) -> Option<f64> {
        self.input.axis(name)
    }

    pub fn input_axis_raw(&self, name: &str) -> Option<f64> {
        self.input.axis_raw(name)
    }

    pub fn input_button_state(&self, name: &str) -> Option<u32> {
        self.input.button_state(name)
    }
}
