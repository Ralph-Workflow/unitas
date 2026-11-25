use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonSource {
    Key(u32),
    MouseButton(u32),
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
    prev_buttons_down: HashMap<String, bool>,
    buttons_pressed: HashSet<String>,
    buttons_released: HashSet<String>,
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
            prev_buttons_down: HashMap::new(),
            buttons_pressed: HashSet::new(),
            buttons_released: HashSet::new(),
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
        self.keys_down.clear();
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_down.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.queued_keys.clear();
        self.queued_mouse_buttons.clear();
        self.queued_pointer_move = None;
        self.queued_scroll = (0.0, 0.0);
        self.pointer_x = 0.0;
        self.pointer_y = 0.0;
        self.pointer_dx = 0.0;
        self.pointer_dy = 0.0;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        for entries in self.axes.values_mut() {
            for entry in entries {
                entry.reset();
            }
        }
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

    pub fn inject_scroll(&mut self, x: f64, y: f64) {
        self.queue_scroll(x, y);
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

        let keys_down = &self.keys_down;
        let mouse_down = &self.mouse_down;
        let keys_pressed = &self.keys_pressed;
        let mouse_pressed = &self.mouse_pressed;
        let keys_released = &self.keys_released;
        let mouse_released = &self.mouse_released;
        let pointer_dx = self.pointer_dx;
        let pointer_dy = self.pointer_dy;
        let scroll_y = self.scroll_y;

        for (name, entries) in &mut self.axes {
            let mut any_button_source_down = false;
            let mut any_button_source_pressed = false;
            let mut any_button_source_released = false;

            for entry in entries.iter_mut() {
                for src in entry
                    .positive_sources
                    .iter()
                    .chain(entry.negative_sources.iter())
                {
                    if is_source_in(src, keys_down, mouse_down) {
                        any_button_source_down = true;
                    }
                    if is_source_in(src, keys_pressed, mouse_pressed) {
                        any_button_source_pressed = true;
                    }
                    if is_source_in(src, keys_released, mouse_released) {
                        any_button_source_released = true;
                    }
                }

                match entry.config.axis_type {
                    0 => {
                        let pos_down = entry
                            .positive_sources
                            .iter()
                            .any(|src| is_source_in(src, keys_down, mouse_down));
                        let neg_down = entry
                            .negative_sources
                            .iter()
                            .any(|src| is_source_in(src, keys_down, mouse_down));

                        let raw_target = match (pos_down, neg_down) {
                            (true, false) => 1.0,
                            (false, true) => -1.0,
                            _ => 0.0,
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

fn is_source_in(src: &ButtonSource, keys: &HashSet<u32>, mouse: &HashSet<u32>) -> bool {
    match *src {
        ButtonSource::Key(key) => keys.contains(&key),
        ButtonSource::MouseButton(button) => mouse.contains(&button),
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
