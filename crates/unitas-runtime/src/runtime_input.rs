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

    pub fn input_gamepad_set_connected(&mut self, connected: bool) {
        self.input.set_gamepad_connected(connected);
    }

    pub fn input_gamepad_connected(&self) -> u32 {
        u32::from(self.input.is_gamepad_connected())
    }

    pub fn input_gamepad_button_event(&mut self, button: u32, pressed: bool) {
        self.input.set_gamepad_button(button, pressed);
    }

    pub fn input_gamepad_button_state(&self, button: u32) -> u32 {
        self.input.gamepad_button_state(button)
    }

    pub fn input_gamepad_axis_event(&mut self, axis: u32, value: f64) {
        self.input.set_gamepad_axis(axis, value);
    }

    pub fn input_gamepad_axis(&self, axis: u32) -> f64 {
        self.input.gamepad_axis(axis)
    }
}

#[cfg(test)]
mod tests {
    use crate::abi::{
        test_read_error_message, unitas_runtime_destroy, unitas_runtime_input_gamepad_axis,
        unitas_runtime_input_gamepad_axis_event, unitas_runtime_input_gamepad_button_event,
        unitas_runtime_input_gamepad_button_state, unitas_runtime_input_gamepad_connected,
        unitas_runtime_input_gamepad_set_connected, unitas_runtime_new, unitas_runtime_tick,
    };
    use crate::input::{
        GAMEPAD_AXIS_COUNT, GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_RIGHT_Y, GAMEPAD_BUTTON_NORTH,
        GAMEPAD_BUTTON_SOUTH, STATE_DOWN, STATE_PRESSED, STATE_RELEASED,
    };

    #[test]
    fn gamepad_ffi_exports_track_connection_button_transitions_and_axes() {
        let handle = unitas_runtime_new();
        assert_ne!(handle, 0);
        assert_eq!(unitas_runtime_tick(handle, 1.0 / 60.0), 0);

        assert_eq!(unitas_runtime_input_gamepad_connected(handle), 0);
        assert_eq!(unitas_runtime_input_gamepad_set_connected(handle, 1), 0);
        assert_eq!(unitas_runtime_input_gamepad_connected(handle), 1);

        assert_eq!(
            unitas_runtime_input_gamepad_button_event(handle, GAMEPAD_BUTTON_SOUTH, 1),
            0
        );
        assert_eq!(
            unitas_runtime_input_gamepad_axis_event(handle, GAMEPAD_AXIS_LEFT_X, 0.75),
            0
        );
        assert_eq!(
            unitas_runtime_input_gamepad_axis_event(handle, GAMEPAD_AXIS_RIGHT_Y, -0.5),
            0
        );
        assert_eq!(unitas_runtime_tick(handle, 1.0 / 60.0), 0);

        assert_eq!(
            unitas_runtime_input_gamepad_button_state(handle, GAMEPAD_BUTTON_SOUTH),
            STATE_DOWN | STATE_PRESSED
        );
        assert_eq!(
            unitas_runtime_input_gamepad_button_state(handle, GAMEPAD_BUTTON_NORTH),
            0
        );
        assert!(
            (unitas_runtime_input_gamepad_axis(handle, GAMEPAD_AXIS_LEFT_X) - 0.75).abs() < 1e-9
        );
        assert!(
            (unitas_runtime_input_gamepad_axis(handle, GAMEPAD_AXIS_RIGHT_Y) - (-0.5)).abs() < 1e-9
        );

        assert_eq!(unitas_runtime_tick(handle, 1.0 / 60.0), 0);
        assert_eq!(
            unitas_runtime_input_gamepad_button_state(handle, GAMEPAD_BUTTON_SOUTH),
            STATE_DOWN
        );

        assert_eq!(
            unitas_runtime_input_gamepad_button_event(handle, GAMEPAD_BUTTON_SOUTH, 0),
            0
        );
        assert_eq!(unitas_runtime_tick(handle, 1.0 / 60.0), 0);
        assert_eq!(
            unitas_runtime_input_gamepad_button_state(handle, GAMEPAD_BUTTON_SOUTH),
            STATE_RELEASED
        );

        assert_eq!(unitas_runtime_tick(handle, 1.0 / 60.0), 0);
        assert_eq!(
            unitas_runtime_input_gamepad_button_state(handle, GAMEPAD_BUTTON_SOUTH),
            0
        );

        assert_eq!(unitas_runtime_input_gamepad_set_connected(handle, 0), 0);
        assert_eq!(unitas_runtime_input_gamepad_connected(handle), 0);
        assert_eq!(
            unitas_runtime_input_gamepad_axis(handle, GAMEPAD_AXIS_LEFT_X),
            0.0
        );

        assert_eq!(unitas_runtime_destroy(handle), 0);
    }

    #[test]
    fn gamepad_ffi_exports_validate_arguments_and_invalid_handles() {
        let handle = unitas_runtime_new();
        assert_ne!(handle, 0);

        assert_eq!(
            unitas_runtime_input_gamepad_axis_event(handle, GAMEPAD_AXIS_LEFT_X, f64::NAN),
            -2
        );
        assert_eq!(
            test_read_error_message(handle).ok().as_deref(),
            Some("non-finite gamepad axis value")
        );

        assert_eq!(
            unitas_runtime_input_gamepad_axis_event(handle, GAMEPAD_AXIS_COUNT as u32, 0.5),
            -2
        );

        assert_eq!(unitas_runtime_destroy(handle), 0);

        let invalid = 999_999_999;
        assert_eq!(
            unitas_runtime_input_gamepad_button_state(invalid, GAMEPAD_BUTTON_SOUTH),
            0
        );
        assert_eq!(
            unitas_runtime_input_gamepad_axis(invalid, GAMEPAD_AXIS_LEFT_X),
            0.0
        );
        assert_eq!(unitas_runtime_input_gamepad_connected(invalid), 0);
        assert_eq!(unitas_runtime_input_gamepad_set_connected(invalid, 1), -1);
        assert_eq!(
            unitas_runtime_input_gamepad_button_event(invalid, GAMEPAD_BUTTON_SOUTH, 1),
            -1
        );
        assert_eq!(
            unitas_runtime_input_gamepad_axis_event(invalid, GAMEPAD_AXIS_LEFT_X, 0.5),
            -1
        );
    }
}
