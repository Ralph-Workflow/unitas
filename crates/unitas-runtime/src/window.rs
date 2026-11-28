#![cfg(feature = "device")]

macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use gilrs::ff::{
    BaseEffect, BaseEffectType, Effect, EffectBuilder, Envelope, Repeat, Replay, Ticks,
};
use gilrs::{Axis, Button, Error as GilrsError, EventType, GamepadId, Gilrs};
use unitas_audio::output::DeviceOutput;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::Runtime;
use crate::input::{
    FocusPause, GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_AXIS_RIGHT_X,
    GAMEPAD_AXIS_RIGHT_Y, GAMEPAD_BUTTON_DPAD_DOWN, GAMEPAD_BUTTON_DPAD_LEFT,
    GAMEPAD_BUTTON_DPAD_RIGHT, GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_EAST,
    GAMEPAD_BUTTON_LEFT_STICK, GAMEPAD_BUTTON_LEFT_TRIGGER, GAMEPAD_BUTTON_NORTH,
    GAMEPAD_BUTTON_RIGHT_STICK, GAMEPAD_BUTTON_RIGHT_TRIGGER, GAMEPAD_BUTTON_SELECT,
    GAMEPAD_BUTTON_SOUTH, GAMEPAD_BUTTON_START, GAMEPAD_BUTTON_WEST, InputState, KEY_BACKSPACE,
    KEY_DELETE, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ALT, KEY_LEFT_ARROW, KEY_LEFT_CONTROL,
    KEY_LEFT_SHIFT, KEY_RETURN, KEY_RIGHT_ALT, KEY_RIGHT_ARROW, KEY_RIGHT_CONTROL, KEY_RIGHT_SHIFT,
    KEY_SPACE, KEY_TAB, KEY_UP_ARROW, RumbleHandler,
};
pub use crate::presenter::{
    DisplaySettingsState, FrameInfo, GpuCameraBinding, GpuFramePresenter, GpuMeshBinding,
    GpuWindowPresenter, WindowError, frame_pacing_interval, scene_gpu_meshes,
    set_runtime_quality_level, shadow_settings_for_quality, shadow_settings_for_runtime,
    transform_to_mat4, unitas_runtime_display_full_screen, unitas_runtime_display_height,
    unitas_runtime_display_quality_level, unitas_runtime_display_quit_requested,
    unitas_runtime_display_request_quit, unitas_runtime_display_reset,
    unitas_runtime_display_set_full_screen, unitas_runtime_display_set_quality_level,
    unitas_runtime_display_set_resolution, unitas_runtime_display_set_target_frame_rate,
    unitas_runtime_display_set_v_sync_count, unitas_runtime_display_target_frame_rate,
    unitas_runtime_display_v_sync_count, unitas_runtime_display_width, v_sync_count_for_runtime,
    view_matrix_from_transform, with_display_settings,
};

pub const DEFAULT_WINDOW_TITLE: &str = "unitas";
pub const DEFAULT_WINDOW_WIDTH: u32 = 1280;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 720;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub frame_limit: u64,
    pub max_frames: Option<u64>,
    pub fixed_frame_delta_seconds: Option<f64>,
    pub exit_on_escape: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from(DEFAULT_WINDOW_TITLE),
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
            resizable: true,
            frame_limit: 0,
            max_frames: None,
            fixed_frame_delta_seconds: None,
            exit_on_escape: true,
        }
    }
}

impl WindowConfig {
    pub fn effective_max_frames(&self) -> Option<u64> {
        match (self.max_frames, self.frame_limit) {
            (Some(limit), _) if limit > 0 => Some(limit),
            (_, limit) if limit > 0 => Some(limit),
            _ => None,
        }
    }
}

pub trait FramePresenter {
    fn attach_window(&mut self, _window: &Arc<Window>) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn apply_focus(&mut self, _input: &mut InputState, _focused: bool) {}

    fn update(&mut self, _runtime: &mut Runtime, _frame: &FrameInfo) -> Result<(), WindowError> {
        Ok(())
    }

    fn present(&mut self, window: &Window, runtime: &Runtime, frame: &FrameInfo);
}

#[derive(Debug, Default)]
pub struct NullFramePresenter {
    pub frames: u64,
}

impl FramePresenter for NullFramePresenter {
    fn present(&mut self, _window: &Window, _runtime: &Runtime, frame: &FrameInfo) {
        self.frames = frame.frame_index;
    }
}

impl FramePresenter for GpuFramePresenter<'_> {
    fn attach_window(&mut self, window: &Arc<Window>) {
        self.attach_window_surface(window);
    }

    fn apply_focus(&mut self, input: &mut InputState, focused: bool) {
        self.apply_window_focus(input, focused);
    }

    fn update(&mut self, runtime: &mut Runtime, frame: &FrameInfo) -> Result<(), WindowError> {
        self.update_window_frame(runtime, frame)
    }

    fn present(&mut self, _window: &Window, runtime: &Runtime, frame: &FrameInfo) {
        self.present_window_frame(runtime, frame);
    }
}

pub fn sustained_frames_per_second(frames: u64, elapsed_seconds: f64) -> f64 {
    if frames > 0 && elapsed_seconds > 0.0 {
        frames as f64 / elapsed_seconds
    } else {
        0.0
    }
}

pub fn sustained_fps_report_line(frames: u64, elapsed_seconds: f64) -> String {
    let fps = sustained_frames_per_second(frames, elapsed_seconds);
    format!("sustained_fps={fps:.1} frames={frames} elapsed_seconds={elapsed_seconds:.3}")
}

pub fn frame_pacing_wait_duration(
    target_frame_rate: i32,
    v_sync_count: u32,
    elapsed: Duration,
) -> Option<Duration> {
    let interval = frame_pacing_interval(target_frame_rate, v_sync_count)?;
    interval.checked_sub(elapsed).filter(|wait| !wait.is_zero())
}

pub fn should_exit_window_loop(runtime: &Runtime, config: &WindowConfig, frame_index: u64) -> bool {
    if runtime.is_shutdown() {
        return true;
    }
    if with_display_settings(|display| display.quit_requested()) {
        return true;
    }
    if let Some(limit) = config.effective_max_frames()
        && frame_index >= limit
    {
        return true;
    }
    false
}

pub fn apply_window_resolution(
    window: &Window,
    presenter: &mut dyn FramePresenter,
    width: u32,
    height: u32,
    full_screen: bool,
) {
    let fullscreen_mode = if full_screen {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    };
    window.set_fullscreen(fullscreen_mode);
    if !full_screen {
        let _ = window.request_inner_size(LogicalSize::new(width.max(1), height.max(1)));
    }
    presenter.resize(width.max(1), height.max(1));
}

pub fn run_windowed(
    runtime: &mut Runtime,
    config: &WindowConfig,
    presenter: &mut dyn FramePresenter,
) -> Result<(), WindowError> {
    if config.width == 0 || config.height == 0 {
        return Err(WindowError::InvalidConfig(String::from(
            "window size must be greater than zero",
        )));
    }
    let event_loop = EventLoop::new().map_err(|error| WindowError::EventLoop(error.to_string()))?;
    let mut app = WindowApp::new(runtime, config, presenter);
    event_loop
        .run_app(&mut app)
        .map_err(|error| WindowError::EventLoop(error.to_string()))?;
    if let Some(line) = app.sustained_fps_line() {
        println!("{line}");
    }
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

pub fn run_windowed_gpu(
    runtime: &mut Runtime,
    config: &WindowConfig,
    presenter: &mut GpuFramePresenter<'_>,
) -> Result<(), WindowError> {
    run_windowed(runtime, config, presenter)?;
    if let Some(error) = presenter.take_last_error() {
        return Err(error);
    }
    Ok(())
}

const RUMBLE_DURATION_MILLIS: u32 = 200;

struct GamepadBackend {
    gilrs: Gilrs,
    active: Option<GamepadId>,
    effect: Option<Effect>,
}

impl GamepadBackend {
    fn new() -> Option<Self> {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(GilrsError::NotImplemented(gilrs)) => gilrs,
            Err(_) => return None,
        };
        let mut backend = Self {
            gilrs,
            active: None,
            effect: None,
        };
        let connected: Vec<GamepadId> = backend.gilrs.gamepads().map(|(id, _)| id).collect();
        if let Some(id) = connected.first().copied() {
            backend.active = Some(id);
        }
        Some(backend)
    }

    fn poll(&mut self, input: &mut InputState) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    self.active = Some(event.id);
                    input.connect_gamepad(gamepad_device_id(event.id));
                }
                EventType::Disconnected => {
                    if self.active == Some(event.id) {
                        self.active = None;
                        self.effect = None;
                    }
                    input.disconnect_gamepad(gamepad_device_id(event.id));
                }
                EventType::ButtonPressed(button, _) => {
                    if let Some(index) = gamepad_button_index(button) {
                        input.set_gamepad_button(index, true);
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(index) = gamepad_button_index(button) {
                        input.set_gamepad_button(index, false);
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    if let Some(index) = gamepad_axis_index(axis) {
                        input.set_gamepad_axis(index, f64::from(value));
                    }
                }
                _ => {}
            }
        }
    }

    fn rumble(&mut self, low: f64, high: f64) -> bool {
        let Some(id) = self.active else {
            return false;
        };
        let Some(gamepad) = self.gilrs.connected_gamepad(id) else {
            return false;
        };
        if !gamepad.is_ff_supported() {
            return false;
        }
        let duration = Ticks::from_ms(RUMBLE_DURATION_MILLIS);
        let mut builder = EffectBuilder::new();
        builder.add_effect(BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: rumble_magnitude(low),
            },
            scheduling: Replay {
                play_for: duration,
                with_delay: duration,
                ..Default::default()
            },
            envelope: Envelope::default(),
        });
        builder.add_effect(BaseEffect {
            kind: BaseEffectType::Weak {
                magnitude: rumble_magnitude(high),
            },
            scheduling: Replay {
                play_for: duration,
                with_delay: duration,
                ..Default::default()
            },
            envelope: Envelope::default(),
        });
        let Ok(effect) = builder
            .gamepads(&[id])
            .repeat(Repeat::For(duration))
            .finish(&mut self.gilrs)
        else {
            return false;
        };
        if effect.play().is_err() {
            return false;
        }
        self.effect = Some(effect);
        true
    }
}

fn gamepad_device_id(id: GamepadId) -> u32 {
    u32::try_from(usize::from(id)).unwrap_or(u32::MAX)
}

fn gamepad_axis_index(axis: Axis) -> Option<u32> {
    match axis {
        Axis::LeftStickX => Some(GAMEPAD_AXIS_LEFT_X),
        Axis::LeftStickY => Some(GAMEPAD_AXIS_LEFT_Y),
        Axis::RightStickX => Some(GAMEPAD_AXIS_RIGHT_X),
        Axis::RightStickY => Some(GAMEPAD_AXIS_RIGHT_Y),
        _ => None,
    }
}

fn gamepad_button_index(button: Button) -> Option<u32> {
    match button {
        Button::South => Some(GAMEPAD_BUTTON_SOUTH),
        Button::East => Some(GAMEPAD_BUTTON_EAST),
        Button::West => Some(GAMEPAD_BUTTON_WEST),
        Button::North => Some(GAMEPAD_BUTTON_NORTH),
        Button::LeftTrigger => Some(GAMEPAD_BUTTON_LEFT_TRIGGER),
        Button::RightTrigger => Some(GAMEPAD_BUTTON_RIGHT_TRIGGER),
        Button::Select => Some(GAMEPAD_BUTTON_SELECT),
        Button::Start => Some(GAMEPAD_BUTTON_START),
        Button::LeftThumb => Some(GAMEPAD_BUTTON_LEFT_STICK),
        Button::RightThumb => Some(GAMEPAD_BUTTON_RIGHT_STICK),
        Button::DPadUp => Some(GAMEPAD_BUTTON_DPAD_UP),
        Button::DPadDown => Some(GAMEPAD_BUTTON_DPAD_DOWN),
        Button::DPadLeft => Some(GAMEPAD_BUTTON_DPAD_LEFT),
        Button::DPadRight => Some(GAMEPAD_BUTTON_DPAD_RIGHT),
        _ => None,
    }
}

fn rumble_magnitude(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16
}

struct WindowApp<'a> {
    runtime: &'a mut Runtime,
    presenter: &'a mut dyn FramePresenter,
    config: WindowConfig,
    window: Option<Arc<Window>>,
    last_frame: Option<Instant>,
    frame_index: u64,
    error: Option<WindowError>,
    presented_frames: u64,
    presenting_started: Option<Instant>,
    presenting_finished: Option<Instant>,
    gamepad: Option<Arc<Mutex<GamepadBackend>>>,
    focus_pause: FocusPause,
}

impl<'a> WindowApp<'a> {
    fn new(
        runtime: &'a mut Runtime,
        config: &WindowConfig,
        presenter: &'a mut dyn FramePresenter,
    ) -> Self {
        let gamepad = GamepadBackend::new().map(|backend| Arc::new(Mutex::new(backend)));
        if let Some(backend) = &gamepad {
            let handler_backend = Arc::clone(backend);
            runtime
                .input_mut()
                .set_rumble_handler(Some(RumbleHandler::new(
                    move |low, high| match handler_backend.lock() {
                        Ok(mut backend) => backend.rumble(low, high),
                        Err(_) => false,
                    },
                )));
        }
        with_display_settings(|display| {
            display.sync_from_runtime(runtime, config.width, config.height, false);
        });
        Self {
            runtime,
            presenter,
            config: config.clone(),
            window: None,
            last_frame: None,
            frame_index: 0,
            error: None,
            presented_frames: 0,
            presenting_started: None,
            presenting_finished: None,
            gamepad,
            focus_pause: FocusPause::default(),
        }
    }

    fn poll_gamepad(&mut self) {
        let Some(gamepad) = &self.gamepad else {
            return;
        };
        let Ok(mut backend) = gamepad.lock() else {
            return;
        };
        backend.poll(self.runtime.input_mut());
    }

    fn sustained_fps_line(&self) -> Option<String> {
        if self.presented_frames == 0 {
            return None;
        }
        let elapsed_seconds = match (self.presenting_started, self.presenting_finished) {
            (Some(started), Some(finished)) => finished.duration_since(started).as_secs_f64(),
            _ => 0.0,
        };
        Some(sustained_fps_report_line(
            self.presented_frames,
            elapsed_seconds,
        ))
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }
        if should_exit_window_loop(self.runtime, &self.config, self.frame_index) {
            with_display_settings(|display| display.clear_quit_request());
            event_loop.exit();
            return;
        }
        let frame_started = Instant::now();
        self.poll_gamepad();
        let now = Instant::now();
        let delta_seconds = match self.config.fixed_frame_delta_seconds {
            Some(fixed) if fixed.is_finite() && fixed > 0.0 => fixed,
            _ => match self.last_frame {
                Some(last) => now.duration_since(last).as_secs_f64(),
                None => 1.0 / 60.0,
            },
        };
        self.last_frame = Some(now);
        if let Err(error) = self.runtime.tick(delta_seconds) {
            self.error = Some(WindowError::Runtime(error.to_string()));
            event_loop.exit();
            return;
        }
        self.frame_index += 1;
        let pending_window =
            with_display_settings(|display| display.take_pending_window_reconfigure());
        if let (Some((req_w, req_h, req_fs)), Some(window)) = (pending_window, &self.window) {
            let window = Arc::clone(window);
            apply_window_resolution(&window, self.presenter, req_w, req_h, req_fs);
        }
        let (width, height) = match &self.window {
            Some(window) => {
                let size = window.inner_size();
                (size.width, size.height)
            }
            None => (self.config.width, self.config.height),
        };
        let frame = FrameInfo {
            frame_index: self.frame_index,
            width,
            height,
            delta_seconds,
        };
        if let Err(error) = self.presenter.update(self.runtime, &frame) {
            self.error = Some(error);
            event_loop.exit();
            return;
        }
        let post_update_window =
            with_display_settings(|display| display.take_pending_window_reconfigure());
        if let (Some((req_w, req_h, req_fs)), Some(window)) = (post_update_window, &self.window) {
            let window = Arc::clone(window);
            apply_window_resolution(&window, self.presenter, req_w, req_h, req_fs);
        }
        if should_exit_window_loop(self.runtime, &self.config, 0) {
            with_display_settings(|display| display.clear_quit_request());
            event_loop.exit();
            return;
        }
        let Some(window) = &self.window else {
            return;
        };
        if self.presenting_started.is_none() {
            self.presenting_started = Some(Instant::now());
        }
        self.presenter.present(window, self.runtime, &frame);
        self.presented_frames += 1;
        self.presenting_finished = Some(Instant::now());
        let (target_fps, vsync) =
            with_display_settings(|display| (display.target_frame_rate(), display.v_sync_count()));
        if let Some(wait) = frame_pacing_wait_duration(target_fps, vsync, frame_started.elapsed()) {
            std::thread::sleep(wait);
        }
        window.request_redraw();
        if should_exit_window_loop(self.runtime, &self.config, self.frame_index) {
            with_display_settings(|display| display.clear_quit_request());
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for WindowApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_resizable(self.config.resizable);
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window.inner_size();
                self.presenter.attach_window(&window);
                self.presenter.resize(size.width, size.height);
                window.request_redraw();
                self.window = Some(window);
            }
            Err(error) => {
                self.error = Some(WindowError::WindowCreation(error.to_string()));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Resized(size) => {
                with_display_settings(|display| {
                    display.record_observed_size(size.width, size.height);
                });
                self.presenter.resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    with_display_settings(|display| {
                        display.record_observed_size(size.width, size.height);
                    });
                    self.presenter.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(focused) => {
                self.focus_pause.apply_focus(self.runtime, focused);
                self.presenter
                    .apply_focus(self.runtime.input_mut(), focused);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.config.exit_on_escape
                    && event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    && self.config.effective_max_frames().is_none()
                {
                    apply_key_event(self.runtime.input_mut(), event.physical_key, event.state);
                    event_loop.exit();
                    return;
                }
                apply_key_event(self.runtime.input_mut(), event.physical_key, event.state);
                if event.state == ElementState::Pressed
                    && let Some(text) = event.text.as_deref()
                {
                    apply_text_input(self.runtime.input_mut(), text);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                apply_mouse_button(self.runtime.input_mut(), button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                apply_pointer_move(
                    self.runtime.input_mut(),
                    position.x / scale_factor,
                    position.y / scale_factor,
                );
            }
            WindowEvent::MouseWheel { delta, .. } => {
                apply_scroll(self.runtime.input_mut(), delta);
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }
}

pub fn key_code_from_winit(code: KeyCode) -> Option<u32> {
    let key = match code {
        KeyCode::KeyA => u32::from(b'a'),
        KeyCode::KeyB => u32::from(b'b'),
        KeyCode::KeyC => u32::from(b'c'),
        KeyCode::KeyD => u32::from(b'd'),
        KeyCode::KeyE => u32::from(b'e'),
        KeyCode::KeyF => u32::from(b'f'),
        KeyCode::KeyG => u32::from(b'g'),
        KeyCode::KeyH => u32::from(b'h'),
        KeyCode::KeyI => u32::from(b'i'),
        KeyCode::KeyJ => u32::from(b'j'),
        KeyCode::KeyK => u32::from(b'k'),
        KeyCode::KeyL => u32::from(b'l'),
        KeyCode::KeyM => u32::from(b'm'),
        KeyCode::KeyN => u32::from(b'n'),
        KeyCode::KeyO => u32::from(b'o'),
        KeyCode::KeyP => u32::from(b'p'),
        KeyCode::KeyQ => u32::from(b'q'),
        KeyCode::KeyR => u32::from(b'r'),
        KeyCode::KeyS => u32::from(b's'),
        KeyCode::KeyT => u32::from(b't'),
        KeyCode::KeyU => u32::from(b'u'),
        KeyCode::KeyV => u32::from(b'v'),
        KeyCode::KeyW => u32::from(b'w'),
        KeyCode::KeyX => u32::from(b'x'),
        KeyCode::KeyY => u32::from(b'y'),
        KeyCode::KeyZ => u32::from(b'z'),
        KeyCode::Digit0 => u32::from(b'0'),
        KeyCode::Digit1 => u32::from(b'1'),
        KeyCode::Digit2 => u32::from(b'2'),
        KeyCode::Digit3 => u32::from(b'3'),
        KeyCode::Digit4 => u32::from(b'4'),
        KeyCode::Digit5 => u32::from(b'5'),
        KeyCode::Digit6 => u32::from(b'6'),
        KeyCode::Digit7 => u32::from(b'7'),
        KeyCode::Digit8 => u32::from(b'8'),
        KeyCode::Digit9 => u32::from(b'9'),
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::Tab => KEY_TAB,
        KeyCode::Enter => KEY_RETURN,
        KeyCode::Escape => KEY_ESCAPE,
        KeyCode::Space => KEY_SPACE,
        KeyCode::Delete => KEY_DELETE,
        KeyCode::ArrowUp => KEY_UP_ARROW,
        KeyCode::ArrowDown => KEY_DOWN_ARROW,
        KeyCode::ArrowRight => KEY_RIGHT_ARROW,
        KeyCode::ArrowLeft => KEY_LEFT_ARROW,
        KeyCode::Home => crate::canvas::KEY_HOME,
        KeyCode::End => crate::canvas::KEY_END,
        KeyCode::ShiftLeft => KEY_LEFT_SHIFT,
        KeyCode::ShiftRight => KEY_RIGHT_SHIFT,
        KeyCode::ControlLeft => KEY_LEFT_CONTROL,
        KeyCode::ControlRight => KEY_RIGHT_CONTROL,
        KeyCode::AltLeft => KEY_LEFT_ALT,
        KeyCode::AltRight => KEY_RIGHT_ALT,
        _ => return None,
    };
    Some(key)
}

pub fn mouse_button_code_from_winit(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
        MouseButton::Other(code) => 5 + u32::from(code),
    }
}

pub fn apply_key_event(input: &mut InputState, physical_key: PhysicalKey, state: ElementState) {
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };
    let Some(key) = key_code_from_winit(code) else {
        return;
    };
    input.inject_key(key, state == ElementState::Pressed);
}

pub fn apply_text_input(input: &mut InputState, text: &str) {
    for ch in text.chars() {
        if !ch.is_control() {
            input.queue_char_event(ch);
        }
    }
}

pub fn apply_mouse_button(input: &mut InputState, button: MouseButton, state: ElementState) {
    let code = mouse_button_code_from_winit(button);
    input.inject_mouse_button(code, state == ElementState::Pressed);
}

pub fn apply_focus_change(
    input: &mut InputState,
    audio_paused: &mut bool,
    audio_output: Option<&mut DeviceOutput>,
    focused: bool,
) {
    if focused {
        *audio_paused = false;
        if let Some(output) = audio_output {
            let _ = output.set_paused(false);
        }
        return;
    }
    input.reset();
    *audio_paused = true;
    if let Some(output) = audio_output {
        let _ = output.set_paused(true);
    }
}

pub fn apply_pointer_move(input: &mut InputState, x: f64, y: f64) {
    input.inject_mouse_move(x, y);
}

pub fn apply_scroll(input: &mut InputState, delta: MouseScrollDelta) {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(position) => (position.x, position.y),
    };
    input.inject_mouse_scroll(x, y);
}

static LAST_ERROR: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
static LAST_FRAME_COUNT: AtomicU64 = AtomicU64::new(0);

fn set_last_error(message: &str) {
    let store = LAST_ERROR.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(mut guard) = store.lock() else {
        return;
    };
    guard.clear();
    guard.extend_from_slice(message.as_bytes());
    guard.push(0);
}

ffi_export! {
pub extern "C" fn unitas_runtime_run_windowed(width: u32, height: u32, frame_limit: u32) -> i32 {
    let mut runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            set_last_error(&error.to_string());
            return -5;
        }
    };
    let limit = u64::from(frame_limit);
    let config = WindowConfig {
        title: String::from(DEFAULT_WINDOW_TITLE),
        width,
        height,
        resizable: true,
        frame_limit: limit,
        max_frames: if limit > 0 { Some(limit) } else { None },
        fixed_frame_delta_seconds: None,
        exit_on_escape: true,
    };
    let mut presenter = NullFramePresenter::default();
    match run_windowed(&mut runtime, &config, &mut presenter) {
        Ok(()) => {
            LAST_FRAME_COUNT.store(presenter.frames, Ordering::Relaxed);
            0
        }
        Err(error) => {
            set_last_error(&error.to_string());
            error.class()
        }
    }
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_window_frame_count() -> u64 {
    LAST_FRAME_COUNT.load(Ordering::Relaxed)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_window_error_message() -> u64 {
    let store = LAST_ERROR.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(guard) = store.lock() else {
        return 0;
    };
    if guard.is_empty() {
        return 0;
    }
    guard.as_ptr() as usize as u64
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeError;
    use crate::input::{KEY_MOUSE_0, STATE_DOWN, STATE_PRESSED, STATE_RELEASED};
    use std::path::{Path, PathBuf};
    use unitas_core::{EntityId, Transform};
    use unitas_render::{Mat4, cube};

    const SAMPLE_SCENE: &str = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";

    fn sample_scene_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(SAMPLE_SCENE)
    }

    fn input_state() -> InputState {
        InputState::with_default_axes()
    }

    #[test]
    fn bound_scene_renders_non_flat_frame_on_gpu() -> Result<(), Box<dyn std::error::Error>> {
        let mut presenter = match GpuFramePresenter::from_new_renderer() {
            Ok(presenter) => presenter,
            Err(error) => {
                eprintln!("skipping GPU scene test, no GPU device available: {error}");
                return Ok(());
            }
        };
        let mut runtime = Runtime::new()?;
        runtime.load_scene(&sample_scene_path())?;
        presenter.bind_scene(&runtime);
        assert!(
            !presenter.meshes().is_empty(),
            "the scene renderers were not bound to the presenter"
        );
        let first = presenter.render_offscreen(&runtime, 320, 240)?;
        let second = presenter.render_offscreen(&runtime, 320, 240)?;
        let distinct: std::collections::HashSet<_> = first.pixels.iter().copied().collect();
        assert!(
            distinct.len() > 1,
            "the captured frame is a single flat colour"
        );
        assert_eq!(first.pixels, second.pixels);
        Ok(())
    }

    #[test]
    fn key_press_maps_to_key_state() {
        let mut input = input_state();
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        input.begin_frame(1.0 / 60.0);
        let state = input.key_state(u32::from(b'w'));
        assert_eq!(state & STATE_DOWN, STATE_DOWN);
        assert_eq!(state & STATE_PRESSED, STATE_PRESSED);
        assert_eq!(state & STATE_RELEASED, 0);
        assert!(input.is_key_down(u32::from(b'w')));
        assert!(input.is_key_pressed(u32::from(b'w')));
        assert!(!input.is_key_released(u32::from(b'w')));
    }

    #[test]
    fn key_release_clears_key_state() {
        let mut input = input_state();
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        input.begin_frame(1.0 / 60.0);
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Released,
        );
        input.begin_frame(1.0 / 60.0);
        let state = input.key_state(u32::from(b'w'));
        assert_eq!(state & STATE_DOWN, 0);
        assert_eq!(state & STATE_RELEASED, STATE_RELEASED);
        assert!(!input.is_key_down(u32::from(b'w')));
        assert!(input.is_key_released(u32::from(b'w')));
    }

    #[test]
    fn named_keys_map_to_runtime_constants() {
        let mut input = input_state();
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::ArrowUp),
            ElementState::Pressed,
        );
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::ShiftLeft),
            ElementState::Pressed,
        );
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        );
        input.begin_frame(1.0 / 60.0);
        assert_ne!(input.key_state(KEY_UP_ARROW) & STATE_DOWN, 0);
        assert_ne!(input.key_state(KEY_LEFT_SHIFT) & STATE_DOWN, 0);
        assert_ne!(input.key_state(KEY_ESCAPE) & STATE_DOWN, 0);
    }

    #[test]
    fn horizontal_axis_follows_key_state() {
        let mut input = input_state();
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::KeyD),
            ElementState::Pressed,
        );
        input.begin_frame(1.0 / 60.0);
        assert!(input.axis("Horizontal").unwrap_or(0.0) > 0.0);
    }

    #[test]
    fn focus_change_clears_pressed_input_and_toggles_audio_pause() {
        let mut input = input_state();
        let mut audio_paused = false;
        apply_key_event(
            &mut input,
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        apply_mouse_button(&mut input, MouseButton::Left, ElementState::Pressed);
        input.begin_frame(1.0 / 60.0);
        apply_focus_change(&mut input, &mut audio_paused, None, false);
        assert!(!input.is_key_down(u32::from(b'w')));
        assert!(!input.is_key_pressed(u32::from(b'w')));
        assert!(!input.is_mouse_button_down(0));
        assert!(audio_paused);
        apply_focus_change(&mut input, &mut audio_paused, None, true);
        assert!(!audio_paused);
    }

    #[test]
    fn focus_loss_clears_input_and_freezes_the_player_until_focus_returns()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new()?;
        let mut audio_paused = false;
        let mut focus = FocusPause::default();
        apply_key_event(
            runtime.input_mut(),
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
        );
        runtime.input_mut().begin_frame(1.0 / 60.0);

        focus.apply_focus(&mut runtime, false);
        apply_focus_change(runtime.input_mut(), &mut audio_paused, None, false);

        assert!(!runtime.input().is_key_down(u32::from(b'w')));
        assert_eq!(runtime.time_scale(), 0.0);
        assert!(audio_paused);

        focus.apply_focus(&mut runtime, true);
        apply_focus_change(runtime.input_mut(), &mut audio_paused, None, true);

        assert_eq!(runtime.time_scale(), 1.0);
        assert!(!audio_paused);
        Ok(())
    }

    #[test]
    fn mouse_button_maps_to_mouse_state() {
        let mut input = input_state();
        apply_mouse_button(&mut input, MouseButton::Left, ElementState::Pressed);
        input.begin_frame(1.0 / 60.0);
        let state = input.mouse_button_state(0);
        assert_eq!(state & STATE_DOWN, STATE_DOWN);
        assert_eq!(state & STATE_PRESSED, STATE_PRESSED);
        assert_ne!(input.key_state(KEY_MOUSE_0) & STATE_DOWN, 0);
        assert!(input.is_mouse_button_down(0));
        assert!(input.is_mouse_button_pressed(0));
        assert!(!input.is_mouse_button_released(0));
    }

    #[test]
    fn pointer_move_updates_pointer_state() {
        let mut input = input_state();
        apply_pointer_move(&mut input, 120.0, 45.0);
        input.begin_frame(1.0 / 60.0);
        let (x, y, dx, dy) = input.pointer();
        assert_eq!(x, 120.0);
        assert_eq!(y, 45.0);
        assert_eq!(dx, 120.0);
        assert_eq!(dy, 45.0);
    }

    #[test]
    fn scroll_updates_scroll_state() {
        let mut input = input_state();
        apply_scroll(&mut input, MouseScrollDelta::LineDelta(0.0, 3.0));
        input.begin_frame(1.0 / 60.0);
        let (x, y) = input.scroll();
        assert_eq!(x, 0.0);
        assert_eq!(y, 3.0);
    }

    #[test]
    fn injection_api_feeds_the_same_path() {
        let mut input = input_state();
        input.inject_key(u32::from(b'a'), true);
        input.inject_mouse_button(1, true);
        input.inject_pointer_move(10.0, 20.0);
        input.inject_scroll(1.0, 2.0);
        input.begin_frame(1.0 / 60.0);
        assert_ne!(input.key_state(u32::from(b'a')) & STATE_DOWN, 0);
        assert_ne!(input.mouse_button_state(1) & STATE_DOWN, 0);
        let (x, y, _, _) = input.pointer();
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        let (_, scroll_y) = input.scroll();
        assert_eq!(scroll_y, 2.0);
    }

    #[test]
    fn window_config_defaults_and_max_frames_resolution() {
        let mut config = WindowConfig::default();
        assert!(config.width > 0);
        assert!(config.height > 0);
        assert!(config.resizable);
        assert_eq!(config.frame_limit, 0);
        assert_eq!(config.max_frames, None);
        assert_eq!(config.effective_max_frames(), None);
        assert_eq!(config.title, String::from(DEFAULT_WINDOW_TITLE));

        config.frame_limit = 12;
        assert_eq!(config.effective_max_frames(), Some(12));

        config.max_frames = Some(5);
        assert_eq!(config.effective_max_frames(), Some(5));
    }

    #[test]
    fn transform_and_view_matrix_helpers_produce_valid_matrices() {
        let transform = Transform {
            translation: [1.5, 2.5, -3.5],
            scale: [2.0, 2.0, 2.0],
            ..Default::default()
        };
        let mat = transform_to_mat4(&transform);
        assert_eq!(mat.get(3, 0), 1.5);
        assert_eq!(mat.get(3, 1), 2.5);
        assert_eq!(mat.get(3, 2), -3.5);
        assert_eq!(mat.get(0, 0), 2.0);

        let view = view_matrix_from_transform(&transform);
        assert_eq!(view.get(3, 3), 1.0);
    }

    #[test]
    fn sustained_frames_per_second_divides_presented_frames_by_elapsed_time() {
        let fps = sustained_frames_per_second(60, 1.0);
        assert!((fps - 60.0).abs() < 1e-9);
        assert_eq!(sustained_frames_per_second(0, 1.0), 0.0);
        assert_eq!(sustained_frames_per_second(60, 0.0), 0.0);
        assert_eq!(sustained_frames_per_second(60, -1.0), 0.0);
    }

    #[test]
    fn sustained_fps_report_line_uses_the_device_lane_greppable_format() {
        assert_eq!(
            sustained_fps_report_line(12, 0.2),
            String::from("sustained_fps=60.0 frames=12 elapsed_seconds=0.200")
        );
        assert_eq!(
            sustained_fps_report_line(0, 0.0),
            String::from("sustained_fps=0.0 frames=0 elapsed_seconds=0.000")
        );
    }

    #[test]
    fn gpu_mesh_binding_resolves_standalone_and_cpu_reference() -> Result<(), RuntimeError> {
        let runtime = Runtime::new()?;
        let binding =
            GpuMeshBinding::standalone(cube(1.0), Mat4::identity(), [0.8, 0.2, 0.2], true);
        assert!(binding.enabled);
        assert_eq!(binding.entity, None);
        assert_eq!(runtime.world_transform_of(EntityId::new(999)), None);
        Ok(())
    }

    #[test]
    fn frame_pacing_wait_duration_respects_target_frame_rate_only_when_vsync_is_zero() {
        let one_ms = std::time::Duration::from_millis(1);
        assert_eq!(frame_pacing_wait_duration(60, 1, one_ms), None);
        assert_eq!(frame_pacing_wait_duration(-1, 0, one_ms), None);
        assert_eq!(frame_pacing_wait_duration(0, 0, one_ms), None);
        let wait = frame_pacing_wait_duration(50, 0, std::time::Duration::from_millis(5));
        assert_eq!(wait, Some(std::time::Duration::from_millis(15)));
        let no_wait = frame_pacing_wait_duration(50, 0, std::time::Duration::from_millis(25));
        assert_eq!(no_wait, None);
    }

    #[test]
    fn window_loop_should_exit_on_runtime_shutdown_display_quit_or_frame_limit()
    -> Result<(), RuntimeError> {
        unitas_runtime_display_reset();
        let mut runtime = Runtime::new()?;
        let config = WindowConfig::default();
        assert!(!should_exit_window_loop(&runtime, &config, 1));

        unitas_runtime_display_request_quit();
        assert!(should_exit_window_loop(&runtime, &config, 1));
        unitas_runtime_display_reset();
        assert!(!should_exit_window_loop(&runtime, &config, 1));

        let limited = WindowConfig {
            max_frames: Some(3),
            ..WindowConfig::default()
        };
        assert!(!should_exit_window_loop(&runtime, &limited, 2));
        assert!(should_exit_window_loop(&runtime, &limited, 3));

        let _ = runtime.shutdown();
        assert!(should_exit_window_loop(&runtime, &config, 1));
        Ok(())
    }
}
