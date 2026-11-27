use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::Runtime;

pub const GAMEPAD_AXIS_LEFT_X: u32 = 0;
pub const GAMEPAD_AXIS_LEFT_Y: u32 = 1;
pub const GAMEPAD_AXIS_RIGHT_X: u32 = 2;
pub const GAMEPAD_AXIS_RIGHT_Y: u32 = 3;
pub const GAMEPAD_AXIS_LEFT_TRIGGER: u32 = 4;
pub const GAMEPAD_AXIS_RIGHT_TRIGGER: u32 = 5;
pub const GAMEPAD_AXIS_COUNT: usize = 6;

pub const GAMEPAD_BUTTON_SOUTH: u32 = 0;
pub const GAMEPAD_BUTTON_EAST: u32 = 1;
pub const GAMEPAD_BUTTON_WEST: u32 = 2;
pub const GAMEPAD_BUTTON_NORTH: u32 = 3;
pub const GAMEPAD_BUTTON_LEFT_SHOULDER: u32 = 4;
pub const GAMEPAD_BUTTON_RIGHT_SHOULDER: u32 = 5;
pub const GAMEPAD_BUTTON_SELECT: u32 = 6;
pub const GAMEPAD_BUTTON_START: u32 = 7;
pub const GAMEPAD_BUTTON_LEFT_STICK: u32 = 8;
pub const GAMEPAD_BUTTON_RIGHT_STICK: u32 = 9;
pub const GAMEPAD_BUTTON_LEFT_TRIGGER: u32 = 10;
pub const GAMEPAD_BUTTON_RIGHT_TRIGGER: u32 = 11;
pub const GAMEPAD_BUTTON_DPAD_UP: u32 = 12;
pub const GAMEPAD_BUTTON_DPAD_DOWN: u32 = 13;
pub const GAMEPAD_BUTTON_DPAD_LEFT: u32 = 14;
pub const GAMEPAD_BUTTON_DPAD_RIGHT: u32 = 15;

pub const GAMEPAD_STICK_DEAD_ZONE: f64 = 0.2;

pub const STATE_DOWN: u32 = 1;
pub const STATE_PRESSED: u32 = 2;
pub const STATE_RELEASED: u32 = 4;

pub const KEY_BACKSPACE: u32 = 8;
pub const KEY_TAB: u32 = 9;
pub const KEY_RETURN: u32 = 13;
pub const KEY_ESCAPE: u32 = 27;
pub const KEY_SPACE: u32 = 32;
pub const KEY_DELETE: u32 = 127;
pub const KEY_UP_ARROW: u32 = 273;
pub const KEY_DOWN_ARROW: u32 = 274;
pub const KEY_RIGHT_ARROW: u32 = 275;
pub const KEY_LEFT_ARROW: u32 = 276;
pub const KEY_RIGHT_SHIFT: u32 = 303;
pub const KEY_LEFT_SHIFT: u32 = 304;
pub const KEY_RIGHT_CONTROL: u32 = 305;
pub const KEY_LEFT_CONTROL: u32 = 306;
pub const KEY_RIGHT_ALT: u32 = 307;
pub const KEY_LEFT_ALT: u32 = 308;
pub const KEY_MOUSE_0: u32 = 323;
pub const KEY_MOUSE_1: u32 = 324;
pub const KEY_MOUSE_2: u32 = 325;

/// Mouse button id of the primary (left) button.
pub const PRIMARY_MOUSE_BUTTON: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonSource {
    Key(u32),
    MouseButton(u32),
    GamepadButton(u32),
}

type RumbleFn = dyn Fn(f64, f64) -> bool + Send + Sync;

#[derive(Clone)]
pub struct RumbleHandler(Arc<RumbleFn>);

impl RumbleHandler {
    pub fn new(handler: impl Fn(f64, f64) -> bool + Send + Sync + 'static) -> Self {
        Self(Arc::new(handler))
    }

    pub fn play(&self, low: f64, high: f64) -> bool {
        (self.0)(low, high)
    }
}

impl fmt::Debug for RumbleHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RumbleHandler")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisConfig {
    pub name: String,
    pub negative_button: String,
    pub positive_button: String,
    pub alt_negative_button: String,
    pub alt_positive_button: String,
    pub gravity: f64,
    pub dead: f64,
    pub sensitivity: f64,
    pub snap: bool,
    pub invert: bool,
    pub axis_type: u32,
    pub axis: u32,
}

#[derive(Debug, Clone)]
struct AxisEntry {
    config: AxisConfig,
    positive_sources: Vec<ButtonSource>,
    negative_sources: Vec<ButtonSource>,
    current: f64,
    smoothed: f64,
    raw: f64,
}

impl AxisEntry {
    fn new(config: AxisConfig) -> Self {
        let mut positive_sources = Vec::new();
        if let Some(src) = parse_button_token(&config.positive_button) {
            positive_sources.push(src);
        }
        if let Some(src) = parse_button_token(&config.alt_positive_button)
            && !positive_sources.contains(&src)
        {
            positive_sources.push(src);
        }
        let mut negative_sources = Vec::new();
        if let Some(src) = parse_button_token(&config.negative_button) {
            negative_sources.push(src);
        }
        if let Some(src) = parse_button_token(&config.alt_negative_button)
            && !negative_sources.contains(&src)
        {
            negative_sources.push(src);
        }
        Self {
            config,
            positive_sources,
            negative_sources,
            current: 0.0,
            smoothed: 0.0,
            raw: 0.0,
        }
    }

    fn reset(&mut self) {
        self.current = 0.0;
        self.smoothed = 0.0;
        self.raw = 0.0;
    }
}

pub fn parse_button_token(name: &str) -> Option<ButtonSource> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.len() == 1 {
        let byte = normalized.as_bytes()[0];
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            return Some(ButtonSource::Key(u32::from(byte)));
        }
    }
    if let Some(index) = normalized
        .strip_prefix("joystick button ")
        .and_then(|index| index.trim().parse::<u32>().ok())
    {
        return Some(ButtonSource::GamepadButton(index));
    }
    match normalized.as_str() {
        "backspace" => Some(ButtonSource::Key(KEY_BACKSPACE)),
        "tab" => Some(ButtonSource::Key(KEY_TAB)),
        "return" | "enter" => Some(ButtonSource::Key(KEY_RETURN)),
        "escape" => Some(ButtonSource::Key(KEY_ESCAPE)),
        "space" => Some(ButtonSource::Key(KEY_SPACE)),
        "delete" => Some(ButtonSource::Key(KEY_DELETE)),
        "up" => Some(ButtonSource::Key(KEY_UP_ARROW)),
        "down" => Some(ButtonSource::Key(KEY_DOWN_ARROW)),
        "right" => Some(ButtonSource::Key(KEY_RIGHT_ARROW)),
        "left" => Some(ButtonSource::Key(KEY_LEFT_ARROW)),
        "right shift" => Some(ButtonSource::Key(KEY_RIGHT_SHIFT)),
        "left shift" => Some(ButtonSource::Key(KEY_LEFT_SHIFT)),
        "right ctrl" | "right control" => Some(ButtonSource::Key(KEY_RIGHT_CONTROL)),
        "left ctrl" | "left control" => Some(ButtonSource::Key(KEY_LEFT_CONTROL)),
        "right alt" => Some(ButtonSource::Key(KEY_RIGHT_ALT)),
        "left alt" => Some(ButtonSource::Key(KEY_LEFT_ALT)),
        "mouse 0" => Some(ButtonSource::MouseButton(0)),
        "mouse 1" => Some(ButtonSource::MouseButton(1)),
        "mouse 2" => Some(ButtonSource::MouseButton(2)),
        _ => None,
    }
}

pub fn default_axis_configs() -> Vec<AxisConfig> {
    vec![
        AxisConfig {
            name: String::from("Horizontal"),
            negative_button: String::from("left"),
            positive_button: String::from("right"),
            alt_negative_button: String::from("a"),
            alt_positive_button: String::from("d"),
            gravity: 3.0,
            dead: 0.001,
            sensitivity: 3.0,
            snap: true,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Vertical"),
            negative_button: String::from("down"),
            positive_button: String::from("up"),
            alt_negative_button: String::from("s"),
            alt_positive_button: String::from("w"),
            gravity: 3.0,
            dead: 0.001,
            sensitivity: 3.0,
            snap: false,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Fire1"),
            negative_button: String::new(),
            positive_button: String::from("left ctrl"),
            alt_negative_button: String::new(),
            alt_positive_button: String::from("mouse 0"),
            gravity: 1000.0,
            dead: 0.001,
            sensitivity: 1000.0,
            snap: false,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Jump"),
            negative_button: String::new(),
            positive_button: String::from("space"),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 1000.0,
            dead: 0.001,
            sensitivity: 1000.0,
            snap: false,
            invert: false,
            axis_type: 0,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Mouse X"),
            negative_button: String::new(),
            positive_button: String::new(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 0.0,
            dead: 0.0,
            sensitivity: 0.1,
            snap: false,
            invert: false,
            axis_type: 1,
            axis: 0,
        },
        AxisConfig {
            name: String::from("Mouse Y"),
            negative_button: String::new(),
            positive_button: String::new(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 0.0,
            dead: 0.0,
            sensitivity: 0.1,
            snap: false,
            invert: false,
            axis_type: 1,
            axis: 1,
        },
        AxisConfig {
            name: String::from("Mouse ScrollWheel"),
            negative_button: String::new(),
            positive_button: String::new(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 0.0,
            dead: 0.0,
            sensitivity: 0.1,
            snap: false,
            invert: false,
            axis_type: 1,
            axis: 2,
        },
    ]
}

/// One frame of pointer state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerFrame {
    pub position: (f32, f32),
    pub primary_held: bool,
    pub primary_pressed: bool,
    pub primary_released: bool,
}

#[derive(Debug, Clone)]
pub struct InputState {
    keys_down: HashSet<u32>,
    keys_pressed: HashSet<u32>,
    keys_released: HashSet<u32>,
    mouse_down: HashSet<u32>,
    mouse_pressed: HashSet<u32>,
    mouse_released: HashSet<u32>,
    queued_keys: Vec<(u32, bool)>,
    queued_mouse_buttons: Vec<(u32, bool)>,
    queued_pointer_move: Option<(f64, f64)>,
    queued_scroll: (f64, f64),
    pointer_x: f64,
    pointer_y: f64,
    pointer_dx: f64,
    pointer_dy: f64,
    scroll_x: f64,
    scroll_y: f64,
    axes: HashMap<String, Vec<AxisEntry>>,
    injected_axes: HashMap<String, f64>,
    prev_buttons_down: HashMap<String, bool>,
    buttons_pressed: HashSet<String>,
    buttons_released: HashSet<String>,
    gamepad_id: Option<u32>,
    gamepad_axes: [f64; GAMEPAD_AXIS_COUNT],
    gamepad_buttons_down: HashSet<u32>,
    gamepad_buttons_pressed: HashSet<u32>,
    gamepad_buttons_released: HashSet<u32>,
    queued_gamepad_buttons: Vec<(u32, bool)>,
    rumble_handler: Option<RumbleHandler>,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_pressed: HashSet::new(),
            mouse_released: HashSet::new(),
            queued_keys: Vec::new(),
            queued_mouse_buttons: Vec::new(),
            queued_pointer_move: None,
            queued_scroll: (0.0, 0.0),
            pointer_x: 0.0,
            pointer_y: 0.0,
            pointer_dx: 0.0,
            pointer_dy: 0.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            axes: HashMap::new(),
            injected_axes: HashMap::new(),
            prev_buttons_down: HashMap::new(),
            buttons_pressed: HashSet::new(),
            buttons_released: HashSet::new(),
            gamepad_id: None,
            gamepad_axes: [0.0; GAMEPAD_AXIS_COUNT],
            gamepad_buttons_down: HashSet::new(),
            gamepad_buttons_pressed: HashSet::new(),
            gamepad_buttons_released: HashSet::new(),
            queued_gamepad_buttons: Vec::new(),
            rumble_handler: None,
        }
    }

    pub fn with_default_axes() -> Self {
        let mut state = Self::new();
        state.set_axes(default_axis_configs());
        state
    }

    pub fn set_axes(&mut self, configs: Vec<AxisConfig>) {
        self.axes.clear();
        self.prev_buttons_down.clear();
        self.buttons_pressed.clear();
        self.buttons_released.clear();
        for config in configs {
            let name = config.name.clone();
            self.prev_buttons_down.insert(name.clone(), false);
            self.axes
                .entry(name)
                .or_default()
                .push(AxisEntry::new(config));
        }
    }

    pub fn reset(&mut self) {
        self.clear_held_input();
        self.queued_pointer_move = None;
        self.queued_scroll = (0.0, 0.0);
        self.pointer_x = 0.0;
        self.pointer_y = 0.0;
        self.pointer_dx = 0.0;
        self.pointer_dy = 0.0;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
    }

    pub fn clear_held_input(&mut self) {
        self.keys_down.clear();
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_down.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.queued_keys.clear();
        self.queued_mouse_buttons.clear();
        self.clear_gamepad_state();
        for entries in self.axes.values_mut() {
            for entry in entries {
                entry.reset();
            }
        }
        self.injected_axes.clear();
        for down in self.prev_buttons_down.values_mut() {
            *down = false;
        }
        self.buttons_pressed.clear();
        self.buttons_released.clear();
    }

    pub fn queue_key_event(&mut self, key: u32, pressed: bool) {
        if let Some(button) = mouse_button_from_keycode(key) {
            self.queued_mouse_buttons.push((button, pressed));
        }
        self.queued_keys.push((key, pressed));
    }

    pub fn queue_mouse_button_event(&mut self, button: u32, pressed: bool) {
        self.queued_mouse_buttons.push((button, pressed));
        if let Some(key) = keycode_from_mouse_button(button) {
            self.queued_keys.push((key, pressed));
        }
    }

    pub fn queue_pointer_move(&mut self, x: f64, y: f64) {
        self.queued_pointer_move = Some((x, y));
    }

    pub fn queue_scroll(&mut self, x: f64, y: f64) {
        self.queued_scroll.0 += x;
        self.queued_scroll.1 += y;
    }

    pub fn inject_key(&mut self, key: u32, pressed: bool) {
        self.queue_key_event(key, pressed);
    }

    pub fn inject_mouse_button(&mut self, button: u32, pressed: bool) {
        self.queue_mouse_button_event(button, pressed);
    }

    pub fn inject_mouse_move(&mut self, x: f64, y: f64) {
        self.queue_pointer_move(x, y);
    }

    pub fn inject_mouse_scroll(&mut self, x: f64, y: f64) {
        self.queue_scroll(x, y);
    }

    pub fn inject_pointer_move(&mut self, x: f64, y: f64) {
        self.queue_pointer_move(x, y);
    }

    /// Injects a whole pointer frame (position plus primary button state) so a
    /// headless scenario can drive the pointer without a window.
    pub fn inject_pointer_frame(&mut self, position: (f32, f32), primary_held: bool) {
        self.inject_mouse_move(f64::from(position.0), f64::from(position.1));
        if self.is_mouse_button_down(PRIMARY_MOUSE_BUTTON) != primary_held {
            self.inject_mouse_button(PRIMARY_MOUSE_BUTTON, primary_held);
        }
    }

    pub fn inject_scroll(&mut self, x: f64, y: f64) {
        self.queue_scroll(x, y);
    }

    pub fn connect_gamepad(&mut self, id: u32) {
        if self.gamepad_id == Some(id) {
            return;
        }
        self.gamepad_id = Some(id);
        self.clear_gamepad_state();
    }

    pub fn disconnect_gamepad(&mut self, id: u32) {
        if self.gamepad_id != Some(id) {
            return;
        }
        self.gamepad_id = None;
        self.clear_gamepad_state();
    }

    pub fn set_gamepad_axis(&mut self, axis: u32, value: f64) {
        let Some(slot) = self.gamepad_axes.get_mut(axis as usize) else {
            return;
        };
        *slot = if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn set_gamepad_button(&mut self, button: u32, pressed: bool) {
        self.queued_gamepad_buttons.push((button, pressed));
    }

    pub fn set_rumble_handler(&mut self, handler: Option<RumbleHandler>) {
        self.rumble_handler = handler;
    }

    pub fn rumble(&self, low: f64, high: f64) -> bool {
        let Some(handler) = &self.rumble_handler else {
            return false;
        };
        handler.play(clamp_rumble(low), clamp_rumble(high))
    }

    fn clear_gamepad_state(&mut self) {
        self.gamepad_axes = [0.0; GAMEPAD_AXIS_COUNT];
        self.gamepad_buttons_down.clear();
        self.gamepad_buttons_pressed.clear();
        self.gamepad_buttons_released.clear();
        self.queued_gamepad_buttons.clear();
    }

    /// Injects a device-style axis value (gamepad stick) for a configured axis.
    /// The injected value replaces the button-derived evaluation until the next
    /// injection, so the reported value does not depend on the frame rate.
    /// Returns false when the axis is not configured or the value is not finite.
    pub fn inject_axis(&mut self, name: &str, value: f64) -> bool {
        let axis = name.trim();
        if axis.is_empty() || !value.is_finite() || !self.axes.contains_key(axis) {
            return false;
        }
        self.injected_axes
            .insert(axis.to_string(), value.clamp(-1.0, 1.0));
        true
    }

    /// Returns the injected device value for a configured axis with the axis'
    /// inversion and dead-zone rules applied, or None when the axis is unknown
    /// or has no injection. Unlike axis()/axis_raw(), button-derived values are
    /// never reported, so callers stay frame-rate independent.
    pub fn injected_axis(&self, name: &str) -> Option<f64> {
        let axis = name.trim();
        let entries = self.axes.get(axis)?;
        let injected = self.injected_axes.get(axis).copied()?;
        let entry = entries.first()?;
        let sign = if entry.config.invert { -1.0 } else { 1.0 };
        let value = injected * sign;
        let dead = entry.config.dead.max(0.0);
        Some(if value.abs() <= dead { 0.0 } else { value })
    }

    pub fn is_key_down(&self, key: u32) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: u32) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_released(&self, key: u32) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn is_mouse_button_down(&self, button: u32) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn is_mouse_button_pressed(&self, button: u32) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn is_mouse_button_released(&self, button: u32) -> bool {
        self.mouse_released.contains(&button)
    }

    pub fn is_gamepad_button_down(&self, button: u32) -> bool {
        self.gamepad_buttons_down.contains(&button)
    }

    pub fn is_gamepad_button_pressed(&self, button: u32) -> bool {
        self.gamepad_buttons_pressed.contains(&button)
    }

    pub fn is_gamepad_button_released(&self, button: u32) -> bool {
        self.gamepad_buttons_released.contains(&button)
    }

    pub fn gamepad_axis(&self, axis: u32) -> f64 {
        self.gamepad_axes.get(axis as usize).copied().unwrap_or(0.0)
    }

    pub fn begin_frame(&mut self, delta_seconds: f64) {
        let dt = if delta_seconds.is_finite() && delta_seconds > 0.0 {
            delta_seconds
        } else {
            0.0
        };

        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.gamepad_buttons_pressed.clear();
        self.gamepad_buttons_released.clear();
        self.buttons_pressed.clear();
        self.buttons_released.clear();

        if let Some((next_x, next_y)) = self.queued_pointer_move.take() {
            self.pointer_dx = next_x - self.pointer_x;
            self.pointer_dy = next_y - self.pointer_y;
            self.pointer_x = next_x;
            self.pointer_y = next_y;
        } else {
            self.pointer_dx = 0.0;
            self.pointer_dy = 0.0;
        }

        self.scroll_x = self.queued_scroll.0;
        self.scroll_y = self.queued_scroll.1;
        self.queued_scroll = (0.0, 0.0);

        let queued_keys = std::mem::take(&mut self.queued_keys);
        for (key, pressed) in queued_keys {
            if pressed {
                if self.keys_down.insert(key) {
                    self.keys_pressed.insert(key);
                }
            } else if self.keys_down.remove(&key) {
                self.keys_released.insert(key);
            }
        }

        let queued_mouse = std::mem::take(&mut self.queued_mouse_buttons);
        for (button, pressed) in queued_mouse {
            if pressed {
                if self.mouse_down.insert(button) {
                    self.mouse_pressed.insert(button);
                }
            } else if self.mouse_down.remove(&button) {
                self.mouse_released.insert(button);
            }
        }

        let queued_gamepad = std::mem::take(&mut self.queued_gamepad_buttons);
        for (button, pressed) in queued_gamepad {
            if pressed {
                if self.gamepad_buttons_down.insert(button) {
                    self.gamepad_buttons_pressed.insert(button);
                }
            } else if self.gamepad_buttons_down.remove(&button) {
                self.gamepad_buttons_released.insert(button);
            }
        }

        let keys_down = &self.keys_down;
        let mouse_down = &self.mouse_down;
        let keys_pressed = &self.keys_pressed;
        let mouse_pressed = &self.mouse_pressed;
        let keys_released = &self.keys_released;
        let mouse_released = &self.mouse_released;
        let gamepad_down = &self.gamepad_buttons_down;
        let gamepad_pressed = &self.gamepad_buttons_pressed;
        let gamepad_released = &self.gamepad_buttons_released;
        let gamepad_axes = self.gamepad_axes;
        let pointer_dx = self.pointer_dx;
        let pointer_dy = self.pointer_dy;
        let scroll_y = self.scroll_y;

        for (name, entries) in &mut self.axes {
            let mut any_button_source_down = false;
            let mut any_button_source_pressed = false;
            let mut any_button_source_released = false;
            let injected = self.injected_axes.get(name).copied();

            let gamepad_button = gamepad_button_for_name(name);
            if gamepad_button.is_some_and(|button| gamepad_down.contains(&button)) {
                any_button_source_down = true;
            }
            if gamepad_button.is_some_and(|button| gamepad_pressed.contains(&button)) {
                any_button_source_pressed = true;
            }
            if gamepad_button.is_some_and(|button| gamepad_released.contains(&button)) {
                any_button_source_released = true;
            }
            let analog = gamepad_axis_value(&gamepad_axes, gamepad_down, name);

            for entry in entries.iter_mut() {
                for src in entry
                    .positive_sources
                    .iter()
                    .chain(entry.negative_sources.iter())
                {
                    if is_source_in(src, keys_down, mouse_down, gamepad_down) {
                        any_button_source_down = true;
                    }
                    if is_source_in(src, keys_pressed, mouse_pressed, gamepad_pressed) {
                        any_button_source_pressed = true;
                    }
                    if is_source_in(src, keys_released, mouse_released, gamepad_released) {
                        any_button_source_released = true;
                    }
                }

                if let Some(injected_value) = injected {
                    // An injected device axis replaces the button-derived value and
                    // stays frame-rate independent.
                    let sign = if entry.config.invert { -1.0 } else { 1.0 };
                    let value = injected_value * sign;
                    entry.raw = value;
                    let dead = entry.config.dead.max(0.0);
                    entry.smoothed = if value.abs() <= dead { 0.0 } else { value };
                    continue;
                }

                match entry.config.axis_type {
                    0 => {
                        let pos_down = entry
                            .positive_sources
                            .iter()
                            .any(|src| is_source_in(src, keys_down, mouse_down, gamepad_down));
                        let neg_down = entry
                            .negative_sources
                            .iter()
                            .any(|src| is_source_in(src, keys_down, mouse_down, gamepad_down));

                        let digital: f64 = match (pos_down, neg_down) {
                            (true, false) => 1.0,
                            (false, true) => -1.0,
                            _ => 0.0,
                        };
                        let raw_target = if analog.abs() > digital.abs() {
                            analog
                        } else {
                            digital
                        };

                        if raw_target == 0.0 {
                            let step = (entry.config.gravity * dt).max(0.0);
                            if entry.current > 0.0 {
                                entry.current = (entry.current - step).max(0.0);
                            } else if entry.current < 0.0 {
                                entry.current = (entry.current + step).min(0.0);
                            }
                        } else {
                            if entry.config.snap && (entry.current * raw_target < 0.0) {
                                entry.current = 0.0;
                            }
                            let step = (entry.config.sensitivity * dt).max(0.0);
                            if raw_target > entry.current {
                                entry.current = (entry.current + step).min(1.0);
                            } else if raw_target < entry.current {
                                entry.current = (entry.current - step).max(-1.0);
                            }
                        }

                        let sign = if entry.config.invert { -1.0 } else { 1.0 };
                        entry.raw = if raw_target == 0.0 {
                            0.0
                        } else {
                            raw_target * sign
                        };
                        let inverted_val = entry.current * sign;
                        let dead = entry.config.dead.max(0.0);
                        entry.smoothed = if inverted_val.abs() <= dead {
                            0.0
                        } else {
                            inverted_val
                        };
                    }
                    1 => {
                        let delta = match entry.config.axis {
                            0 => pointer_dx,
                            1 => pointer_dy,
                            2 => scroll_y,
                            _ => 0.0,
                        };
                        let dead = entry.config.dead.max(0.0);
                        let filtered = if delta.abs() <= dead { 0.0 } else { delta };
                        let sign = if entry.config.invert { -1.0 } else { 1.0 };
                        entry.raw = if filtered == 0.0 {
                            0.0
                        } else {
                            filtered * sign
                        };
                        entry.smoothed = entry.raw * entry.config.sensitivity;
                    }
                    _ => {
                        entry.raw = 0.0;
                        entry.smoothed = 0.0;
                    }
                }
            }

            let was_down = self.prev_buttons_down.get(name).copied().unwrap_or(false);
            if !was_down && (any_button_source_down || any_button_source_pressed) {
                self.buttons_pressed.insert(name.clone());
            }
            if !any_button_source_down && (was_down || any_button_source_released) {
                self.buttons_released.insert(name.clone());
            }
            self.prev_buttons_down
                .insert(name.clone(), any_button_source_down);
        }
    }

    pub fn key_state(&self, key: u32) -> u32 {
        let mut mask = 0;
        if self.keys_down.contains(&key) {
            mask |= STATE_DOWN;
        }
        if self.keys_pressed.contains(&key) {
            mask |= STATE_PRESSED;
        }
        if self.keys_released.contains(&key) {
            mask |= STATE_RELEASED;
        }
        mask
    }

    pub fn mouse_button_state(&self, button: u32) -> u32 {
        let mut mask = 0;
        if self.mouse_down.contains(&button) {
            mask |= STATE_DOWN;
        }
        if self.mouse_pressed.contains(&button) {
            mask |= STATE_PRESSED;
        }
        if self.mouse_released.contains(&button) {
            mask |= STATE_RELEASED;
        }
        mask
    }

    pub fn pointer(&self) -> (f64, f64, f64, f64) {
        (
            self.pointer_x,
            self.pointer_y,
            self.pointer_dx,
            self.pointer_dy,
        )
    }

    /// The pointer state of the current frame.
    pub fn pointer_frame(&self) -> PointerFrame {
        PointerFrame {
            position: (self.pointer_x as f32, self.pointer_y as f32),
            primary_held: self.is_mouse_button_down(PRIMARY_MOUSE_BUTTON),
            primary_pressed: self.is_mouse_button_pressed(PRIMARY_MOUSE_BUTTON),
            primary_released: self.is_mouse_button_released(PRIMARY_MOUSE_BUTTON),
        }
    }

    pub fn scroll(&self) -> (f64, f64) {
        (self.scroll_x, self.scroll_y)
    }

    pub fn axis(&self, name: &str) -> Option<f64> {
        let entries = self.axes.get(name)?;
        let mut best = 0.0f64;
        for entry in entries {
            if entry.smoothed.abs() > best.abs() {
                best = entry.smoothed;
            }
        }
        Some(best)
    }

    pub fn axis_raw(&self, name: &str) -> Option<f64> {
        let entries = self.axes.get(name)?;
        let mut best = 0.0f64;
        for entry in entries {
            if entry.raw.abs() > best.abs() {
                best = entry.raw;
            }
        }
        Some(best)
    }

    pub fn button_state(&self, name: &str) -> Option<u32> {
        if !self.axes.contains_key(name) {
            return None;
        }
        let mut mask = 0;
        if self.prev_buttons_down.get(name).copied().unwrap_or(false) {
            mask |= STATE_DOWN;
        }
        if self.buttons_pressed.contains(name) {
            mask |= STATE_PRESSED;
        }
        if self.buttons_released.contains(name) {
            mask |= STATE_RELEASED;
        }
        Some(mask)
    }
}

#[derive(Debug, Default)]
pub struct FocusPause {
    restore_time_scale: Option<f64>,
}

impl FocusPause {
    pub fn apply_focus(&mut self, runtime: &mut Runtime, focused: bool) {
        if focused {
            runtime.input_mut().clear_held_input();
            if let Some(time_scale) = self.restore_time_scale.take() {
                runtime.set_time_scale(time_scale);
            }
            return;
        }
        runtime.input_mut().reset();
        if self.restore_time_scale.is_none() {
            self.restore_time_scale = Some(runtime.time_scale());
            runtime.set_time_scale(0.0);
        }
    }
}

fn is_source_in(
    src: &ButtonSource,
    keys: &HashSet<u32>,
    mouse: &HashSet<u32>,
    gamepad: &HashSet<u32>,
) -> bool {
    match *src {
        ButtonSource::Key(key) => keys.contains(&key),
        ButtonSource::MouseButton(button) => mouse.contains(&button),
        ButtonSource::GamepadButton(button) => gamepad.contains(&button),
    }
}

fn gamepad_axis_for_name(name: &str) -> Option<u32> {
    match name {
        "Horizontal" => Some(GAMEPAD_AXIS_LEFT_X),
        "Vertical" => Some(GAMEPAD_AXIS_LEFT_Y),
        _ => None,
    }
}

fn gamepad_button_for_name(name: &str) -> Option<u32> {
    match name {
        "Fire1" => Some(GAMEPAD_BUTTON_SOUTH),
        _ => None,
    }
}

fn gamepad_axis_value(
    axes: &[f64; GAMEPAD_AXIS_COUNT],
    buttons_down: &HashSet<u32>,
    name: &str,
) -> f64 {
    let dpad: f64 = match name {
        "Horizontal" => match (
            buttons_down.contains(&GAMEPAD_BUTTON_DPAD_RIGHT),
            buttons_down.contains(&GAMEPAD_BUTTON_DPAD_LEFT),
        ) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        },
        "Vertical" => match (
            buttons_down.contains(&GAMEPAD_BUTTON_DPAD_UP),
            buttons_down.contains(&GAMEPAD_BUTTON_DPAD_DOWN),
        ) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        },
        _ => 0.0,
    };
    if dpad != 0.0 {
        return dpad;
    }
    let Some(index) = gamepad_axis_for_name(name) else {
        return 0.0;
    };
    let Some(value) = axes.get(index as usize) else {
        return 0.0;
    };
    if value.abs() <= GAMEPAD_STICK_DEAD_ZONE {
        return 0.0;
    }
    *value
}

fn clamp_rumble(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn mouse_button_from_keycode(key: u32) -> Option<u32> {
    match key {
        KEY_MOUSE_0 => Some(0),
        KEY_MOUSE_1 => Some(1),
        KEY_MOUSE_2 => Some(2),
        _ => None,
    }
}

fn keycode_from_mouse_button(button: u32) -> Option<u32> {
    match button {
        0 => Some(KEY_MOUSE_0),
        1 => Some(KEY_MOUSE_1),
        2 => Some(KEY_MOUSE_2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamepad_stick_and_buttons_drive_the_named_axes_and_buttons() {
        let mut input = InputState::with_default_axes();
        input.connect_gamepad(1);
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 1.0);
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_Y, -1.0);
        input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, true);
        input.begin_frame(0.5);

        let horizontal = input.axis("Horizontal").unwrap_or(-999.0);
        assert!((horizontal - 1.0).abs() < 1e-9);
        let vertical = input.axis("Vertical").unwrap_or(-999.0);
        assert!((vertical - (-1.0)).abs() < 1e-9);
        assert_eq!(input.axis_raw("Horizontal"), Some(1.0));
        assert_eq!(input.axis_raw("Vertical"), Some(-1.0));
        assert_eq!(
            input.button_state("Fire1"),
            Some(STATE_DOWN | STATE_PRESSED)
        );
    }

    #[test]
    fn gamepad_stick_dead_zone_zeroes_small_deflections() {
        let mut input = InputState::with_default_axes();
        input.connect_gamepad(1);
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, GAMEPAD_STICK_DEAD_ZONE * 0.5);
        input.begin_frame(0.5);
        assert_eq!(input.axis_raw("Horizontal"), Some(0.0));

        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 0.5);
        input.begin_frame(0.5);
        assert_eq!(input.axis_raw("Horizontal"), Some(0.5));
    }

    #[test]
    fn gamepad_hot_plug_leaves_the_keyboard_state_untouched() {
        let mut input = InputState::with_default_axes();
        input.queue_key_event(KEY_SPACE, true);
        input.begin_frame(0.5);
        assert_eq!(input.key_state(KEY_SPACE), STATE_DOWN | STATE_PRESSED);

        input.connect_gamepad(1);
        input.begin_frame(0.5);
        assert_eq!(input.key_state(KEY_SPACE), STATE_DOWN);

        input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, true);
        input.begin_frame(0.5);
        assert_eq!(
            input.button_state("Fire1"),
            Some(STATE_DOWN | STATE_PRESSED)
        );

        input.disconnect_gamepad(1);
        input.begin_frame(0.5);
        assert_eq!(input.key_state(KEY_SPACE), STATE_DOWN);
        assert_eq!(input.button_state("Fire1"), Some(STATE_RELEASED));
    }

    #[test]
    fn rumble_without_a_device_is_a_silent_no_op() {
        let input = InputState::new();
        assert!(!input.rumble(0.5, 0.25));
    }

    #[test]
    fn joystick_button_tokens_bind_gamepad_buttons_to_named_axes() {
        assert_eq!(
            parse_button_token("joystick button 0"),
            Some(ButtonSource::GamepadButton(0))
        );
        assert_eq!(
            parse_button_token("Joystick Button 3"),
            Some(ButtonSource::GamepadButton(3))
        );
        assert_eq!(parse_button_token("joystick button"), None);
    }

    #[test]
    fn injected_axis_replaces_the_button_derived_value() {
        let mut input = InputState::with_default_axes();
        assert!(input.inject_axis("Horizontal", 1.0));
        input.begin_frame(1.0 / 60.0);
        assert_eq!(input.axis("Horizontal"), Some(1.0));
        assert_eq!(input.axis_raw("Horizontal"), Some(1.0));

        assert!(input.inject_axis("Horizontal", -0.5));
        input.begin_frame(1.0 / 60.0);
        assert_eq!(input.axis("Horizontal"), Some(-0.5));
        assert_eq!(input.axis_raw("Horizontal"), Some(-0.5));
    }

    #[test]
    fn injected_axis_value_is_frame_rate_independent() {
        let mut input = InputState::with_default_axes();
        input.inject_axis("Horizontal", 1.0);
        input.begin_frame(1.0 / 30.0);
        assert_eq!(input.axis("Horizontal"), Some(1.0));
        for _ in 0..10 {
            input.begin_frame(1.0 / 144.0);
        }
        assert_eq!(input.axis("Horizontal"), Some(1.0));
        assert_eq!(input.axis_raw("Horizontal"), Some(1.0));
    }

    #[test]
    fn injected_axis_needs_a_configured_axis_and_is_cleared_by_reset() {
        let mut input = InputState::with_default_axes();
        assert!(!input.inject_axis("Gamepad Axis 1", 1.0));
        assert_eq!(input.axis("Gamepad Axis 1"), None);
        input.inject_axis("Horizontal", 1.0);
        input.reset();
        input.begin_frame(1.0 / 60.0);
        assert_eq!(input.axis("Horizontal"), Some(0.0));
    }

    #[test]
    fn button_derived_axis_is_unchanged_without_an_injection() {
        let mut input = InputState::with_default_axes();
        input.inject_key(KEY_RIGHT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        assert_eq!(input.axis_raw("Horizontal"), Some(1.0));
        assert!(input.axis("Horizontal").unwrap_or(0.0) > 0.0);
    }

    #[test]
    fn focus_loss_clears_held_keys_buttons_and_axes() -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
        runtime.input_mut().inject_mouse_button(0, true);
        runtime.input_mut().inject_axis("Horizontal", 1.0);
        runtime.input_mut().begin_frame(1.0 / 60.0);
        assert!(runtime.input().is_key_down(KEY_RIGHT_ARROW));
        assert!(runtime.input().is_mouse_button_down(0));
        assert_eq!(runtime.input().axis_raw("Horizontal"), Some(1.0));

        let mut focus = FocusPause::default();
        focus.apply_focus(&mut runtime, false);

        assert!(!runtime.input().is_key_down(KEY_RIGHT_ARROW));
        assert!(!runtime.input().is_mouse_button_down(0));
        assert_eq!(runtime.input().injected_axis("Horizontal"), None);
        runtime.input_mut().begin_frame(1.0 / 60.0);
        assert_eq!(runtime.input().axis("Horizontal"), Some(0.0));
        assert_eq!(runtime.input().axis_raw("Horizontal"), Some(0.0));
        Ok(())
    }

    #[test]
    fn focus_loss_pauses_the_simulation() -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        runtime.tick(0.5)?;
        runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
        runtime.tick(0.5)?;
        assert_eq!(runtime.input().axis("Horizontal"), Some(1.0));

        let mut focus = FocusPause::default();
        focus.apply_focus(&mut runtime, false);
        assert_eq!(runtime.time_scale(), 0.0);

        runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
        runtime.tick(0.5)?;
        assert_eq!(runtime.input().axis("Horizontal"), Some(0.0));
        Ok(())
    }

    #[test]
    fn focus_gain_restores_the_clock_and_drops_input_that_arrived_while_unfocused()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        let mut focus = FocusPause::default();
        focus.apply_focus(&mut runtime, false);
        assert_eq!(runtime.time_scale(), 0.0);

        runtime.input_mut().inject_key(KEY_RIGHT_ARROW, true);
        runtime.input_mut().begin_frame(1.0 / 60.0);
        assert!(runtime.input().is_key_down(KEY_RIGHT_ARROW));

        focus.apply_focus(&mut runtime, true);
        assert_eq!(runtime.time_scale(), 1.0);
        assert!(!runtime.input().is_key_down(KEY_RIGHT_ARROW));
        assert_eq!(
            runtime.input().key_state(KEY_RIGHT_ARROW) & STATE_RELEASED,
            0
        );
        Ok(())
    }

    #[test]
    fn focus_gain_restores_the_clock_that_was_in_effect() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut runtime = Runtime::new()?;
        let mut focus = FocusPause::default();

        runtime.set_time_scale(0.5);
        focus.apply_focus(&mut runtime, false);
        assert_eq!(runtime.time_scale(), 0.0);
        focus.apply_focus(&mut runtime, true);
        assert_eq!(runtime.time_scale(), 0.5);

        runtime.set_time_scale(0.0);
        focus.apply_focus(&mut runtime, false);
        focus.apply_focus(&mut runtime, true);
        assert_eq!(runtime.time_scale(), 0.0);
        Ok(())
    }

    #[test]
    fn injected_axis_reader_reports_injections_only() {
        let mut input = InputState::with_default_axes();
        assert_eq!(input.injected_axis("Horizontal"), None);
        input.inject_key(KEY_RIGHT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        assert_eq!(input.injected_axis("Horizontal"), None);
        assert!(input.inject_axis("Horizontal", 0.5));
        assert_eq!(input.injected_axis("Horizontal"), Some(0.5));
        input.inject_axis("Horizontal", 0.0);
        assert_eq!(input.injected_axis("Horizontal"), Some(0.0));
        assert_eq!(input.injected_axis("Unknown"), None);
    }

    #[test]
    fn gamepad_button_and_axis_queries_track_press_hold_release_and_dpad_axes() {
        let mut input = InputState::with_default_axes();
        input.connect_gamepad(1);
        input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, true);
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, true);
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, true);
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 0.75);
        input.begin_frame(0.5);

        assert!(input.is_gamepad_button_down(GAMEPAD_BUTTON_SOUTH));
        assert!(input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH));
        assert!(!input.is_gamepad_button_released(GAMEPAD_BUTTON_SOUTH));
        assert!((input.gamepad_axis(GAMEPAD_AXIS_LEFT_X) - 0.75).abs() < 1e-9);
        assert_eq!(input.axis_raw("Horizontal"), Some(1.0));
        assert_eq!(input.axis_raw("Vertical"), Some(-1.0));

        input.begin_frame(0.5);
        assert!(input.is_gamepad_button_down(GAMEPAD_BUTTON_SOUTH));
        assert!(!input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH));

        input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, false);
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, false);
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, false);
        input.begin_frame(0.5);
        assert!(!input.is_gamepad_button_down(GAMEPAD_BUTTON_SOUTH));
        assert!(!input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH));
        assert!(input.is_gamepad_button_released(GAMEPAD_BUTTON_SOUTH));
    }
}
