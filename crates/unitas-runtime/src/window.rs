#![cfg(feature = "device")]

macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use unitas_audio::output::DeviceOutput;
use unitas_core::{rotate_vector, EntityId, Transform};
use unitas_render::{
    look_at, render_scene, Camera, DirectionalLight, GpuInstance, Image, Mat4, Mesh, MeshInstance,
    RenderError, Renderer, Vec3,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::input::{
    InputState, KEY_BACKSPACE, KEY_DELETE, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ALT,
    KEY_LEFT_ARROW, KEY_LEFT_CONTROL, KEY_LEFT_SHIFT, KEY_RETURN, KEY_RIGHT_ALT, KEY_RIGHT_ARROW,
    KEY_RIGHT_CONTROL, KEY_RIGHT_SHIFT, KEY_SPACE, KEY_TAB, KEY_UP_ARROW,
};
use crate::runtime::RenderCameraParams;
use crate::Runtime;

pub const DEFAULT_WINDOW_TITLE: &str = "unitas";
pub const DEFAULT_WINDOW_WIDTH: u32 = 1280;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 720;
const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 44_100;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameInfo {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub delta_seconds: f64,
}

pub trait FramePresenter {
    fn resize(&mut self, _width: u32, _height: u32) {}

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

#[derive(Debug)]
pub enum WindowError {
    EventLoop(String),
    InvalidConfig(String),
    WindowCreation(String),
    Render(String),
    Audio(String),
    Runtime(String),
}

impl WindowError {
    pub fn class(&self) -> i32 {
        match self {
            WindowError::EventLoop(_) => -1,
            WindowError::InvalidConfig(_) => -2,
            WindowError::WindowCreation(_) => -3,
            WindowError::Render(_) => -4,
            WindowError::Runtime(_) => -5,
            WindowError::Audio(_) => -6,
        }
    }
}

impl fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::EventLoop(message) => {
                write!(formatter, "-1 event loop error: {message}")
            }
            WindowError::InvalidConfig(message) => {
                write!(formatter, "-2 invalid argument: {message}")
            }
            WindowError::WindowCreation(message) => {
                write!(formatter, "-3 window creation error: {message}")
            }
            WindowError::Render(message) => {
                write!(formatter, "-4 render error: {message}")
            }
            WindowError::Runtime(message) => write!(formatter, "-5 runtime error: {message}"),
            WindowError::Audio(message) => write!(formatter, "-6 audio error: {message}"),
        }
    }
}

impl std::error::Error for WindowError {}

impl From<RenderError> for WindowError {
    fn from(error: RenderError) -> Self {
        WindowError::Render(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuMeshBinding {
    pub entity: Option<EntityId>,
    pub mesh: Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
    pub lit: bool,
    pub enabled: bool,
}

impl GpuMeshBinding {
    pub fn for_entity(entity: EntityId, mesh: Mesh, color: [f32; 3], lit: bool) -> Self {
        Self {
            entity: Some(entity),
            mesh,
            model: Mat4::identity(),
            color,
            lit,
            enabled: true,
        }
    }

    pub fn standalone(mesh: Mesh, model: Mat4, color: [f32; 3], lit: bool) -> Self {
        Self {
            entity: None,
            mesh,
            model,
            color,
            lit,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuCameraBinding {
    pub entity: Option<EntityId>,
    pub camera: Camera,
    pub view: Mat4,
    pub background: [f32; 3],
}

impl Default for GpuCameraBinding {
    fn default() -> Self {
        Self {
            entity: None,
            camera: Camera::perspective(60.0, 16.0 / 9.0, 0.1, 1000.0),
            view: look_at(
                Vec3::new(0.0, 8.0, -10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            background: [0.08, 0.1, 0.14],
        }
    }
}

type FrameUpdateCallback<'a> =
    Box<dyn FnMut(&mut Runtime, &FrameInfo) -> Result<(), WindowError> + 'a>;

pub struct GpuFramePresenter<'a> {
    renderer: Arc<Renderer>,
    meshes: Vec<GpuMeshBinding>,
    camera_binding: GpuCameraBinding,
    light: DirectionalLight,
    audio_output: Option<DeviceOutput>,
    update_callback: Option<FrameUpdateCallback<'a>>,
    capture_every_frame: bool,
    last_captured_frame: Option<Image>,
    last_mixed_samples: Vec<f32>,
    total_audio_frames_pushed: u64,
    frames_presented: u64,
    last_error: Option<WindowError>,
}

impl<'a> GpuFramePresenter<'a> {
    pub fn new(renderer: Arc<Renderer>) -> Self {
        Self {
            renderer,
            meshes: Vec::new(),
            camera_binding: GpuCameraBinding::default(),
            light: DirectionalLight {
                direction: Vec3::new(0.4, 0.8, 0.5),
                color: [1.0, 1.0, 1.0],
                intensity: 0.85,
            },
            audio_output: None,
            update_callback: None,
            capture_every_frame: false,
            last_captured_frame: None,
            last_mixed_samples: Vec::new(),
            total_audio_frames_pushed: 0,
            frames_presented: 0,
            last_error: None,
        }
    }

    pub fn from_new_renderer() -> Result<Self, WindowError> {
        let renderer = Renderer::new()?;
        Ok(Self::new(Arc::new(renderer)))
    }

    pub fn renderer(&self) -> &Arc<Renderer> {
        &self.renderer
    }

    pub fn set_meshes(&mut self, meshes: Vec<GpuMeshBinding>) {
        self.meshes = meshes;
    }

    pub fn add_mesh(&mut self, binding: GpuMeshBinding) -> usize {
        let index = self.meshes.len();
        self.meshes.push(binding);
        index
    }

    pub fn meshes(&self) -> &[GpuMeshBinding] {
        &self.meshes
    }

    pub fn meshes_mut(&mut self) -> &mut [GpuMeshBinding] {
        &mut self.meshes
    }

    pub fn set_camera_binding(&mut self, binding: GpuCameraBinding) {
        self.camera_binding = binding;
    }

    pub fn camera_binding(&self) -> &GpuCameraBinding {
        &self.camera_binding
    }

    pub fn camera_binding_mut(&mut self) -> &mut GpuCameraBinding {
        &mut self.camera_binding
    }

    pub fn set_light(&mut self, light: DirectionalLight) {
        self.light = light;
    }

    pub fn light(&self) -> &DirectionalLight {
        &self.light
    }

    pub fn set_audio_output(&mut self, output: Option<DeviceOutput>) {
        self.audio_output = output;
    }

    pub fn start_default_audio_output(&mut self) -> Result<(), WindowError> {
        let output =
            DeviceOutput::start().map_err(|error| WindowError::Audio(error.to_string()))?;
        self.audio_output = Some(output);
        Ok(())
    }

    pub fn audio_output(&self) -> Option<&DeviceOutput> {
        self.audio_output.as_ref()
    }

    pub fn set_update_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Runtime, &FrameInfo) -> Result<(), WindowError> + 'a,
    {
        self.update_callback = Some(Box::new(callback));
    }

    pub fn set_capture_every_frame(&mut self, capture: bool) {
        self.capture_every_frame = capture;
    }

    pub fn last_captured_frame(&self) -> Option<&Image> {
        self.last_captured_frame.as_ref()
    }

    pub fn last_mixed_samples(&self) -> &[f32] {
        &self.last_mixed_samples
    }

    pub fn total_audio_frames_pushed(&self) -> u64 {
        self.total_audio_frames_pushed
    }

    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    pub fn take_last_error(&mut self) -> Option<WindowError> {
        self.last_error.take()
    }

    pub fn resolved_camera_and_view(
        &self,
        runtime: &Runtime,
        width: u32,
        height: u32,
    ) -> (Camera, Mat4, [f32; 3]) {
        let aspect = if height > 0 {
            width.max(1) as f32 / height as f32
        } else {
            1.0
        };
        let mut camera = self.camera_binding.camera;
        camera.aspect = aspect;
        let view = match self
            .camera_binding
            .entity
            .and_then(|entity| runtime.world_transform_of(entity))
        {
            Some(transform) => view_matrix_from_transform(&transform),
            None => self.camera_binding.view,
        };
        (camera, view, self.camera_binding.background)
    }

    pub fn collect_gpu_instances<'b>(&'b self, runtime: &Runtime) -> Vec<GpuInstance<'b>> {
        let mut instances = Vec::with_capacity(self.meshes.len());
        for entry in &self.meshes {
            if !entry.enabled {
                continue;
            }
            let model = match entry
                .entity
                .and_then(|entity| runtime.world_transform_of(entity))
            {
                Some(transform) => transform_to_mat4(&transform),
                None => entry.model,
            };
            instances.push(GpuInstance {
                mesh: &entry.mesh,
                model,
                color: entry.color,
                lit: entry.lit,
            });
        }
        instances
    }

    pub fn render_offscreen(
        &self,
        runtime: &Runtime,
        width: u32,
        height: u32,
    ) -> Result<Image, WindowError> {
        let (camera, view, background) = self.resolved_camera_and_view(runtime, width, height);
        let instances = self.collect_gpu_instances(runtime);
        let image = self.renderer.render_offscreen_with(
            &instances,
            &camera,
            view,
            &self.light,
            background,
            (width, height),
        )?;
        Ok(image)
    }

    pub fn render_cpu_reference(&self, runtime: &Runtime, width: u32, height: u32) -> Image {
        let (camera, view, background) = self.resolved_camera_and_view(runtime, width, height);
        let gpu_instances = self.collect_gpu_instances(runtime);
        let cpu_instances: Vec<MeshInstance<'_>> = gpu_instances
            .iter()
            .map(|inst| MeshInstance {
                mesh: inst.mesh,
                model: inst.model,
                color: inst.color,
            })
            .collect();
        render_scene(
            &cpu_instances,
            &camera,
            view,
            &self.light,
            background,
            width,
            height,
        )
    }

    fn mix_and_push_audio(&mut self, runtime: &mut Runtime, delta_seconds: f64) {
        let dt = if delta_seconds.is_finite() && delta_seconds > 0.0 {
            delta_seconds
        } else {
            1.0 / 60.0
        };
        let sample_rate = self
            .audio_output
            .as_ref()
            .map(|output| output.sample_rate())
            .unwrap_or(DEFAULT_AUDIO_SAMPLE_RATE);
        let frames = ((dt * f64::from(sample_rate)).round() as usize).clamp(1, 4096);
        let samples = runtime.audio_mut().mix(frames);
        if let Some(output) = &self.audio_output {
            let report = output.push_mixed(&samples);
            self.total_audio_frames_pushed += report.accepted_frames;
        }
        self.last_mixed_samples = samples;
    }
}

impl FramePresenter for GpuFramePresenter<'_> {
    fn update(&mut self, runtime: &mut Runtime, frame: &FrameInfo) -> Result<(), WindowError> {
        if let Some(callback) = &mut self.update_callback {
            callback(runtime, frame)?;
        }
        self.mix_and_push_audio(runtime, frame.delta_seconds);
        Ok(())
    }

    fn present(&mut self, window: &Window, runtime: &Runtime, frame: &FrameInfo) {
        let width = frame.width.max(1);
        let height = frame.height.max(1);
        let (camera, view, background) = self.resolved_camera_and_view(runtime, width, height);
        let instances = self.collect_gpu_instances(runtime);
        match self.renderer.instance().create_surface(window) {
            Ok(surface) => {
                if let Err(error) = self.renderer.configure_surface(&surface, width, height) {
                    self.last_error = Some(WindowError::Render(error.to_string()));
                    return;
                }
                if let Err(error) = self.renderer.render_to_surface_with(
                    &surface,
                    &instances,
                    &camera,
                    view,
                    &self.light,
                    background,
                ) {
                    self.last_error = Some(WindowError::Render(error.to_string()));
                    return;
                }
            }
            Err(error) => {
                self.last_error = Some(WindowError::Render(error.to_string()));
                return;
            }
        }
        if self.capture_every_frame {
            match self.renderer.render_offscreen_with(
                &instances,
                &camera,
                view,
                &self.light,
                background,
                (width, height),
            ) {
                Ok(image) => self.last_captured_frame = Some(image),
                Err(error) => {
                    self.last_error = Some(WindowError::Render(error.to_string()));
                    return;
                }
            }
        }
        self.frames_presented = frame.frame_index;
    }
}

pub type GpuWindowPresenter<'a> = GpuFramePresenter<'a>;

pub fn transform_to_mat4(transform: &Transform) -> Mat4 {
    let rotation = transform.rotation;
    let (x, y, z, w) = (rotation[0], rotation[1], rotation[2], rotation[3]);
    let (sx, sy, sz) = (transform.scale[0], transform.scale[1], transform.scale[2]);
    let mut values = [0.0f32; 16];
    values[0] = (1.0 - 2.0 * (y * y + z * z)) * sx;
    values[1] = (2.0 * (x * y + w * z)) * sx;
    values[2] = (2.0 * (x * z - w * y)) * sx;
    values[4] = (2.0 * (x * y - w * z)) * sy;
    values[5] = (1.0 - 2.0 * (x * x + z * z)) * sy;
    values[6] = (2.0 * (y * z + w * x)) * sy;
    values[8] = (2.0 * (x * z + w * y)) * sz;
    values[9] = (2.0 * (y * z - w * x)) * sz;
    values[10] = (1.0 - 2.0 * (x * x + y * y)) * sz;
    values[12] = transform.translation[0];
    values[13] = transform.translation[1];
    values[14] = transform.translation[2];
    values[15] = 1.0;
    Mat4::from_cols_array(values)
}

pub fn view_matrix_from_transform(transform: &Transform) -> Mat4 {
    let eye = Vec3::new(
        transform.translation[0],
        transform.translation[1],
        transform.translation[2],
    );
    let rotated = rotate_vector(transform.rotation, [0.0, 0.0, 1.0]);
    let mut forward = Vec3::new(rotated[0], rotated[1], rotated[2]);
    if forward.length_squared() < 1e-8 {
        forward = Vec3::new(0.0, 0.0, 1.0);
    }
    let forward = forward.normalize();
    let mut up = Vec3::new(0.0, 1.0, 0.0);
    if up.cross(forward).length_squared() < 1e-8 {
        up = Vec3::new(0.0, 0.0, 1.0);
    }
    look_at(eye, eye + forward, up)
}

impl Runtime {
    pub fn register_render_mesh(
        &mut self,
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> u64 {
        self.create_render_mesh(positions, normals, indices)
    }

    pub fn remove_render_mesh(&mut self, handle: u64) -> bool {
        self.destroy_render_mesh(handle)
    }

    pub fn register_render_material(&mut self, color: [f32; 4]) -> u64 {
        let handle = self.create_render_material();
        self.set_render_material_color(handle, color);
        handle
    }

    pub fn update_render_material_color(&mut self, handle: u64, color: [f32; 4]) -> bool {
        self.set_render_material_color(handle, color)
    }

    pub fn register_render_camera(&mut self, entity: Option<EntityId>) -> u64 {
        self.create_render_camera(entity)
    }

    pub fn set_render_camera_params(&mut self, handle: u64, params: RenderCameraParams) -> bool {
        self.update_render_camera(handle, params)
    }

    pub fn register_render_renderer(
        &mut self,
        entity: Option<EntityId>,
        mesh: Option<u64>,
        material: Option<u64>,
    ) -> u64 {
        self.create_render_renderer(entity, mesh, material)
    }

    pub fn update_render_renderer_enabled(&mut self, handle: u64, enabled: bool) -> bool {
        self.set_render_renderer_enabled(handle, enabled)
    }

    pub fn register_render_light(&mut self, entity: Option<EntityId>) -> u64 {
        self.create_render_light(entity)
    }

    pub fn set_render_light_params(
        &mut self,
        handle: u64,
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
    ) -> bool {
        self.update_render_light(handle, direction, color, intensity)
    }

    pub fn render_cpu_frame_bmp(
        &mut self,
        camera: u64,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, WindowError> {
        self.render_frame(camera, width, height)
            .map_err(|error| WindowError::Render(error.to_string()))
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

struct WindowApp<'a> {
    runtime: &'a mut Runtime,
    presenter: &'a mut dyn FramePresenter,
    config: WindowConfig,
    window: Option<Window>,
    last_frame: Option<Instant>,
    frame_index: u64,
    error: Option<WindowError>,
    presented_frames: u64,
    presenting_started: Option<Instant>,
    presenting_finished: Option<Instant>,
}

impl<'a> WindowApp<'a> {
    fn new(
        runtime: &'a mut Runtime,
        config: &WindowConfig,
        presenter: &'a mut dyn FramePresenter,
    ) -> Self {
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
        }
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
        let Some(window) = &self.window else {
            return;
        };
        if self.presenting_started.is_none() {
            self.presenting_started = Some(Instant::now());
        }
        self.presenter.present(window, self.runtime, &frame);
        self.presented_frames += 1;
        self.presenting_finished = Some(Instant::now());
        window.request_redraw();
        if let Some(limit) = self.config.effective_max_frames() {
            if self.frame_index >= limit {
                event_loop.exit();
            }
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
                let size = window.inner_size();
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
            WindowEvent::Resized(size) => self.presenter.resize(size.width, size.height),
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
            }
            WindowEvent::MouseInput { state, button, .. } => {
                apply_mouse_button(self.runtime.input_mut(), button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                apply_pointer_move(self.runtime.input_mut(), position.x, position.y);
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

pub fn apply_mouse_button(input: &mut InputState, button: MouseButton, state: ElementState) {
    let code = mouse_button_code_from_winit(button);
    input.inject_mouse_button(code, state == ElementState::Pressed);
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
    use crate::input::{KEY_MOUSE_0, STATE_DOWN, STATE_PRESSED, STATE_RELEASED};
    use crate::RuntimeError;
    use unitas_render::cube;

    fn input_state() -> InputState {
        InputState::with_default_axes()
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
}
