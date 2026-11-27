use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

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

use unitas_asset::project_settings::QualityLevel;
#[cfg(feature = "device")]
use unitas_audio::output::DeviceOutput;
use unitas_core::{EntityId, Transform, rotate_vector};
use unitas_render::{
    Camera, DirectionalLight, GpuInstance, Image, Mat4, Mesh, RenderError, RenderScene, Renderer,
    Rgba8, ShadowSettings, TextOverlayItem, Vec3, look_at,
};
#[cfg(feature = "device")]
use winit::window::Window;

use crate::Runtime;
#[cfg(feature = "device")]
use crate::input::InputState;
use crate::runtime::RenderCameraParams;

#[cfg(feature = "device")]
const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 44_100;

pub const DESKTOP_QUALITY_TARGET_GROUP: &str = "Standalone";
pub const WEB_QUALITY_TARGET_GROUP: &str = "WebGL";
pub const DEFAULT_DISPLAY_WIDTH: u32 = 1280;
pub const DEFAULT_DISPLAY_HEIGHT: u32 = 720;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySettingsState {
    width: u32,
    height: u32,
    full_screen: bool,
    v_sync_count: u32,
    target_frame_rate: i32,
    quality_level: usize,
    quit_requested: bool,
    pending_window_reconfigure: Option<(u32, u32, bool)>,
    pending_v_sync_count: Option<u32>,
    pending_quality_level: Option<usize>,
}

impl Default for DisplaySettingsState {
    fn default() -> Self {
        Self::new(
            DEFAULT_DISPLAY_WIDTH,
            DEFAULT_DISPLAY_HEIGHT,
            false,
            1,
            -1,
            0,
        )
    }
}

impl DisplaySettingsState {
    pub fn new(
        width: u32,
        height: u32,
        full_screen: bool,
        v_sync_count: u32,
        target_frame_rate: i32,
        quality_level: usize,
    ) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
            full_screen,
            v_sync_count,
            target_frame_rate,
            quality_level,
            quit_requested: false,
            pending_window_reconfigure: None,
            pending_v_sync_count: None,
            pending_quality_level: None,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn full_screen(&self) -> bool {
        self.full_screen
    }

    pub fn v_sync_count(&self) -> u32 {
        self.v_sync_count
    }

    pub fn target_frame_rate(&self) -> i32 {
        self.target_frame_rate
    }

    pub fn quality_level(&self) -> usize {
        self.quality_level
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn set_resolution(&mut self, width: u32, height: u32, full_screen: bool) -> bool {
        if width == 0 || height == 0 {
            return false;
        }
        self.width = width;
        self.height = height;
        self.full_screen = full_screen;
        self.pending_window_reconfigure = Some((width, height, full_screen));
        true
    }

    pub fn record_observed_size(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
        }
    }

    pub fn set_full_screen(&mut self, full_screen: bool) {
        self.full_screen = full_screen;
        self.pending_window_reconfigure = Some((self.width, self.height, full_screen));
    }

    pub fn set_v_sync_count(&mut self, v_sync_count: u32) {
        self.v_sync_count = v_sync_count;
        self.pending_v_sync_count = Some(v_sync_count);
    }

    pub fn set_target_frame_rate(&mut self, target_frame_rate: i32) {
        self.target_frame_rate = target_frame_rate;
    }

    pub fn set_quality_level(&mut self, quality_level: usize) {
        self.quality_level = quality_level;
        self.pending_quality_level = Some(quality_level);
    }

    pub fn sync_from_runtime(
        &mut self,
        runtime: &Runtime,
        width: u32,
        height: u32,
        full_screen: bool,
    ) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
        }
        self.full_screen = full_screen;
        self.v_sync_count = v_sync_count_for_runtime(runtime);
        if let Some(index) = quality_level_index_for_runtime(runtime) {
            self.quality_level = index;
        }
        self.pending_window_reconfigure = None;
        self.pending_v_sync_count = None;
        self.pending_quality_level = None;
        self.quit_requested = false;
    }

    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn clear_quit_request(&mut self) {
        self.quit_requested = false;
    }

    pub fn take_pending_window_reconfigure(&mut self) -> Option<(u32, u32, bool)> {
        self.pending_window_reconfigure.take()
    }

    pub fn take_pending_v_sync_count(&mut self) -> Option<u32> {
        self.pending_v_sync_count.take()
    }

    pub fn take_pending_quality_level(&mut self) -> Option<usize> {
        self.pending_quality_level.take()
    }

    pub fn frame_interval(&self) -> Option<Duration> {
        frame_pacing_interval(self.target_frame_rate, self.v_sync_count)
    }
}

pub fn frame_pacing_interval(target_frame_rate: i32, v_sync_count: u32) -> Option<Duration> {
    if v_sync_count == 0 && target_frame_rate > 0 {
        Some(Duration::from_secs_f64(1.0 / f64::from(target_frame_rate)))
    } else {
        None
    }
}

pub fn quality_level_index_for_runtime(runtime: &Runtime) -> Option<usize> {
    if runtime.quality_settings.levels.is_empty() {
        return None;
    }
    let index = runtime
        .quality_settings
        .per_platform_default_quality
        .iter()
        .find_map(|(platform, level_index)| {
            (platform == DESKTOP_QUALITY_TARGET_GROUP).then_some(*level_index)
        })
        .unwrap_or(runtime.quality_settings.current_quality);
    (index < runtime.quality_settings.levels.len()).then_some(index)
}

static GLOBAL_DISPLAY_SETTINGS: OnceLock<Mutex<DisplaySettingsState>> = OnceLock::new();

pub fn global_display_settings() -> &'static Mutex<DisplaySettingsState> {
    GLOBAL_DISPLAY_SETTINGS.get_or_init(|| Mutex::new(DisplaySettingsState::default()))
}

pub fn with_display_settings<R>(action: impl FnOnce(&mut DisplaySettingsState) -> R) -> R {
    let mutex = global_display_settings();
    let mut guard = match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    action(&mut guard)
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_reset() -> i32 {
    with_display_settings(|state| {
        *state = DisplaySettingsState::default();
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_set_resolution(
    width: u32,
    height: u32,
    full_screen: u32,
) -> i32 {
    with_display_settings(|state| {
        if state.set_resolution(width, height, full_screen != 0) {
            0
        } else {
            -2
        }
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_width() -> u32 {
    with_display_settings(|state| state.width())
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_height() -> u32 {
    with_display_settings(|state| state.height())
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_set_full_screen(full_screen: u32) -> i32 {
    with_display_settings(|state| {
        state.set_full_screen(full_screen != 0);
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_full_screen() -> u32 {
    with_display_settings(|state| u32::from(state.full_screen()))
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_set_v_sync_count(v_sync_count: u32) -> i32 {
    with_display_settings(|state| {
        state.set_v_sync_count(v_sync_count);
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_v_sync_count() -> u32 {
    with_display_settings(|state| state.v_sync_count())
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_set_target_frame_rate(target_frame_rate: i32) -> i32 {
    with_display_settings(|state| {
        state.set_target_frame_rate(target_frame_rate);
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_target_frame_rate() -> i32 {
    with_display_settings(|state| state.target_frame_rate())
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_set_quality_level(index: u32) -> i32 {
    with_display_settings(|state| {
        state.set_quality_level(index as usize);
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_quality_level() -> u32 {
    with_display_settings(|state| state.quality_level() as u32)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_request_quit() -> i32 {
    with_display_settings(|state| {
        state.request_quit();
        0
    })
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_display_quit_requested() -> u32 {
    with_display_settings(|state| u32::from(state.quit_requested()))
}
}

pub fn shadow_settings_for_quality(level: &QualityLevel) -> ShadowSettings {
    ShadowSettings {
        enabled: level.shadows != 0,
        soft: level.shadows == 2,
        resolution_index: level.shadow_resolution,
        distance: level.shadow_distance as f32,
    }
}

pub fn shadow_settings_for_runtime(runtime: &Runtime) -> ShadowSettings {
    runtime
        .quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP)
        .map_or_else(ShadowSettings::disabled, shadow_settings_for_quality)
}

pub fn v_sync_count_for_runtime(runtime: &Runtime) -> u32 {
    runtime
        .quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP)
        .map_or(1, |level| level.v_sync_count)
}

pub fn set_runtime_quality_level(runtime: &mut Runtime, index: usize) -> Option<&QualityLevel> {
    if index >= runtime.quality_settings.levels.len() {
        return None;
    }
    runtime.quality_settings.current_quality = index;
    let mut updated = false;
    for (platform, level_index) in &mut runtime.quality_settings.per_platform_default_quality {
        if platform == DESKTOP_QUALITY_TARGET_GROUP {
            *level_index = index;
            updated = true;
        }
    }
    if !updated {
        runtime
            .quality_settings
            .per_platform_default_quality
            .push((DESKTOP_QUALITY_TARGET_GROUP.to_string(), index));
    }
    if let Some(renderer) = &runtime.gpu_renderer
        && let Some(level) = runtime.quality_settings.levels.get(index)
    {
        renderer.set_shadow_settings(shadow_settings_for_quality(level));
        renderer.set_v_sync_count(level.v_sync_count);
    }
    runtime.quality_settings.levels.get(index)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameInfo {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub delta_seconds: f64,
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

pub type FrameUpdateCallback<'a> =
    Box<dyn FnMut(&mut Runtime, &FrameInfo) -> Result<(), WindowError> + 'a>;

/// Presents a composed frame on the window surface.
pub type SurfaceBlit = Box<dyn FnMut(&Renderer, &Rgba8) -> Result<(), RenderError>>;

pub fn resolve_camera_and_view(
    binding: &GpuCameraBinding,
    runtime: &Runtime,
    width: u32,
    height: u32,
) -> (Camera, Mat4, [f32; 3]) {
    let aspect = if height > 0 {
        width.max(1) as f32 / height as f32
    } else {
        1.0
    };
    let mut camera = binding.camera;
    camera.aspect = aspect;
    let view = match binding
        .entity
        .and_then(|entity| runtime.world_transform_of(entity))
    {
        Some(transform) => view_matrix_from_transform(&transform),
        None => binding.view,
    };
    (camera, view, binding.background)
}

pub fn collect_gpu_instances_from<'b>(
    meshes: &'b [GpuMeshBinding],
    runtime: &'b Runtime,
) -> Vec<GpuInstance<'b>> {
    let mut instances = Vec::with_capacity(meshes.len());
    for entry in meshes {
        if !entry.enabled {
            continue;
        }
        let model = match entry.entity {
            Some(entity) => match runtime.world_transform_of(entity) {
                Some(transform) => transform_to_mat4(&transform),
                None => continue,
            },
            None => entry.model,
        };
        instances.push(GpuInstance {
            mesh: &entry.mesh,
            model,
            color: entry.color,
            lit: entry.lit,
        });
    }
    let effect_mesh = runtime.effect_mesh();
    for item in runtime.particle_draw_list() {
        instances.push(GpuInstance {
            mesh: effect_mesh,
            model: effect_model_mat4(item.position, item.size),
            color: [item.color[0], item.color[1], item.color[2]],
            lit: true,
        });
    }
    for item in runtime.trail_draw_list() {
        instances.push(GpuInstance {
            mesh: effect_mesh,
            model: effect_model_mat4(item.position, item.width),
            color: [item.color[0], item.color[1], item.color[2]],
            lit: true,
        });
    }
    instances
}

pub fn effect_model_mat4(position: [f32; 3], size: f32) -> Mat4 {
    let transform = Transform {
        translation: position,
        rotation: unitas_core::IDENTITY_ROTATION,
        scale: [size, size, size],
        parent: None,
        children: Vec::new(),
    };
    transform_to_mat4(&transform)
}

pub fn scene_gpu_meshes(runtime: &Runtime) -> Vec<GpuMeshBinding> {
    runtime
        .scene_renderables()
        .iter()
        .map(|renderable| {
            GpuMeshBinding::for_entity(
                renderable.entity,
                renderable.mesh.clone(),
                renderable.color,
                true,
            )
        })
        .collect()
}

pub fn is_transient_surface_error(error: &RenderError) -> bool {
    match error {
        RenderError::SurfaceUnavailable(message) => {
            matches!(
                message.as_str(),
                "Timeout"
                    | "Outdated"
                    | "Occluded"
                    | "CurrentSurfaceTexture::Timeout"
                    | "CurrentSurfaceTexture::Outdated"
                    | "CurrentSurfaceTexture::Occluded"
            )
        }
        _ => false,
    }
}

pub struct GpuFramePresenter<'a> {
    renderer: Arc<Renderer>,
    meshes: Vec<GpuMeshBinding>,
    camera_binding: GpuCameraBinding,
    light: DirectionalLight,
    #[cfg(feature = "device")]
    audio_output: Option<DeviceOutput>,
    update_callback: Option<FrameUpdateCallback<'a>>,
    capture_every_frame: bool,
    last_captured_frame: Option<Image>,
    last_mixed_samples: Vec<f32>,
    total_audio_frames_pushed: u64,
    frames_presented: u64,
    surface_blit: Option<SurfaceBlit>,
    overlay: Vec<TextOverlayItem>,
    overlay_source: Option<Arc<Mutex<Vec<TextOverlayItem>>>>,
    overlay_drawn: bool,
    audio_paused: bool,
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
            #[cfg(feature = "device")]
            audio_output: None,
            update_callback: None,
            capture_every_frame: false,
            last_captured_frame: None,
            last_mixed_samples: Vec::new(),
            total_audio_frames_pushed: 0,
            frames_presented: 0,
            surface_blit: None,
            overlay: Vec::new(),
            overlay_source: None,
            overlay_drawn: false,
            audio_paused: false,
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

    pub fn bind_scene(&mut self, runtime: &Runtime) {
        self.set_meshes(scene_gpu_meshes(runtime));
        if let Some(camera) = runtime.scene_camera() {
            let default = GpuCameraBinding::default();
            self.set_camera_binding(GpuCameraBinding {
                entity: camera.entity,
                camera: crate::runtime::projection_for(&camera.params, default.camera.aspect),
                view: default.view,
                background: camera.params.background_color,
            });
        }
        self.set_light(runtime.directional_light());
        self.set_shadow_settings(shadow_settings_for_runtime(runtime));
        let vsync = v_sync_count_for_runtime(runtime);
        self.set_v_sync_count(vsync);
        let quality_index = quality_level_index_for_runtime(runtime).unwrap_or(0);
        with_display_settings(|display| {
            display.v_sync_count = vsync;
            display.quality_level = quality_index;
            display.pending_v_sync_count = None;
            display.pending_quality_level = None;
        });
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

    pub fn set_shadow_settings(&mut self, settings: ShadowSettings) {
        self.renderer.set_shadow_settings(settings);
    }

    pub fn shadow_settings(&self) -> ShadowSettings {
        self.renderer.shadow_settings()
    }

    pub fn set_v_sync_count(&mut self, v_sync_count: u32) {
        self.renderer.set_v_sync_count(v_sync_count);
        with_display_settings(|display| {
            display.v_sync_count = v_sync_count;
            display.pending_v_sync_count = None;
        });
    }

    pub fn v_sync_count(&self) -> u32 {
        self.renderer.v_sync_count()
    }

    pub fn apply_quality_level(&mut self, runtime: &mut Runtime, index: usize) -> bool {
        let Some(level) = set_runtime_quality_level(runtime, index) else {
            return false;
        };
        let shadow = shadow_settings_for_quality(level);
        let vsync = level.v_sync_count;
        self.set_shadow_settings(shadow);
        self.set_v_sync_count(vsync);
        with_display_settings(|display| {
            display.quality_level = index;
            display.v_sync_count = vsync;
            display.pending_quality_level = None;
            display.pending_v_sync_count = None;
        });
        true
    }

    pub fn sync_quality_from_runtime(&mut self, runtime: &Runtime) {
        self.set_shadow_settings(shadow_settings_for_runtime(runtime));
        let vsync = v_sync_count_for_runtime(runtime);
        self.set_v_sync_count(vsync);
    }

    pub fn apply_pending_display_settings(&mut self, runtime: &mut Runtime) {
        let (pending_quality, pending_vsync) = with_display_settings(|display| {
            (
                display.take_pending_quality_level(),
                display.take_pending_v_sync_count(),
            )
        });
        if let Some(index) = pending_quality {
            self.apply_quality_level(runtime, index);
        }
        if let Some(vsync) = pending_vsync {
            self.set_v_sync_count(vsync);
        }
    }

    #[cfg(feature = "device")]
    pub fn set_audio_output(&mut self, output: Option<DeviceOutput>) {
        self.audio_output = output;
    }

    #[cfg(feature = "device")]
    pub fn start_default_audio_output(&mut self) -> Result<(), WindowError> {
        let output =
            DeviceOutput::start().map_err(|error| WindowError::Audio(error.to_string()))?;
        self.audio_output = Some(output);
        Ok(())
    }

    #[cfg(feature = "device")]
    pub fn audio_output(&self) -> Option<&DeviceOutput> {
        self.audio_output.as_ref()
    }

    pub fn audio_paused(&self) -> bool {
        self.audio_paused
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

    pub fn set_overlay(&mut self, items: Vec<TextOverlayItem>) {
        self.overlay = items;
    }

    pub fn overlay(&self) -> &[TextOverlayItem] {
        &self.overlay
    }

    pub fn set_overlay_source(&mut self, source: Arc<Mutex<Vec<TextOverlayItem>>>) {
        self.overlay_source = Some(source);
    }

    pub fn overlay_drawn(&self) -> bool {
        self.overlay_drawn
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
        resolve_camera_and_view(&self.camera_binding, runtime, width, height)
    }

    pub fn collect_gpu_instances<'b>(&'b self, runtime: &'b Runtime) -> Vec<GpuInstance<'b>> {
        collect_gpu_instances_from(&self.meshes, runtime)
    }

    pub fn render_offscreen(
        &self,
        runtime: &Runtime,
        width: u32,
        height: u32,
    ) -> Result<Image, WindowError> {
        let (camera, view, background) = self.resolved_camera_and_view(runtime, width, height);
        let instances = self.collect_gpu_instances(runtime);
        let scene = RenderScene {
            camera: &camera,
            view,
            light: &self.light,
            background,
        };
        let image = self.renderer.render_offscreen_with_overlay(
            &instances,
            scene,
            (width, height),
            &self.overlay,
        )?;
        Ok(runtime.composite_canvas_image(image, width, height))
    }

    #[cfg(feature = "device")]
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
        if !self.audio_paused
            && let Some(output) = &self.audio_output
        {
            let report = output.push_mixed(&samples);
            self.total_audio_frames_pushed += report.accepted_frames;
        }
        self.last_mixed_samples = samples;
    }

    #[cfg(feature = "device")]
    pub(crate) fn attach_window_surface(&mut self, window: &Arc<Window>) {
        match self.renderer.instance().create_surface(Arc::clone(window)) {
            Ok(surface) => {
                let mut configured: Option<(u32, u32, u32)> = None;
                self.surface_blit = Some(Box::new(
                    move |renderer: &Renderer, image: &Rgba8| -> Result<(), RenderError> {
                        let v_sync_count = renderer.v_sync_count();
                        if configured != Some((image.width, image.height, v_sync_count)) {
                            renderer.configure_surface_with_v_sync(
                                &surface,
                                image.width,
                                image.height,
                                v_sync_count,
                            )?;
                            configured = Some((image.width, image.height, v_sync_count));
                        }
                        renderer.present_rgba_to_surface(&surface, image)
                    },
                ));
            }
            Err(error) => {
                self.last_error = Some(WindowError::Render(error.to_string()));
            }
        }
    }

    #[cfg(feature = "device")]
    pub(crate) fn apply_window_focus(&mut self, input: &mut InputState, focused: bool) {
        crate::window::apply_focus_change(
            input,
            &mut self.audio_paused,
            self.audio_output.as_mut(),
            focused,
        );
    }

    #[cfg(feature = "device")]
    pub(crate) fn update_window_frame(
        &mut self,
        runtime: &mut Runtime,
        frame: &FrameInfo,
    ) -> Result<(), WindowError> {
        if let Some(callback) = &mut self.update_callback {
            callback(runtime, frame)?;
        }
        self.apply_pending_display_settings(runtime);
        self.mix_and_push_audio(runtime, frame.delta_seconds);
        Ok(())
    }

    /// The frame the windowed player presents: the scene with the interface
    /// drawn on it by the composition the capture path uses as well.
    pub fn render_window_frame(
        &self,
        runtime: &Runtime,
        width: u32,
        height: u32,
        overlay: &[TextOverlayItem],
    ) -> Result<Rgba8, WindowError> {
        let (camera, view, background) = self.resolved_camera_and_view(runtime, width, height);
        let instances = self.collect_gpu_instances(runtime);
        let scene = RenderScene {
            camera: &camera,
            view,
            light: &self.light,
            background,
        };
        let image = self.renderer.render_offscreen_rgba_with_overlay(
            &instances,
            scene,
            (width, height),
            overlay,
        )?;
        Ok(runtime.composite_canvas_rgba(image, width, height))
    }

    /// Presents the frame of the windowed player on the window surface.
    pub fn present_window_frame(&mut self, runtime: &Runtime, frame: &FrameInfo) {
        let width = frame.width.max(1);
        let height = frame.height.max(1);
        if self.surface_blit.is_none() {
            self.last_error = Some(WindowError::Render(String::from(
                "window surface is not attached",
            )));
            return;
        }
        let overlay_items = self.current_overlay();
        let image = match self.render_window_frame(runtime, width, height, &overlay_items) {
            Ok(image) => image,
            Err(error) => {
                self.last_error = Some(error);
                return;
            }
        };
        let Some(blit) = self.surface_blit.as_mut() else {
            self.last_error = Some(WindowError::Render(String::from(
                "window surface is not attached",
            )));
            return;
        };
        if let Err(error) = blit(&self.renderer, &image)
            && !is_transient_surface_error(&error)
        {
            self.overlay_drawn = false;
            self.last_error = Some(WindowError::Render(error.to_string()));
            return;
        }
        self.overlay_drawn = !overlay_items.is_empty() && self.renderer.has_overlay_fonts();
        if self.capture_every_frame {
            let (camera, view, background) =
                resolve_camera_and_view(&self.camera_binding, runtime, width, height);
            let instances = collect_gpu_instances_from(&self.meshes, runtime);
            match self.renderer.render_offscreen_with(
                &instances,
                &camera,
                view,
                &self.light,
                background,
                (width, height),
            ) {
                Ok(image) => {
                    self.last_captured_frame =
                        Some(runtime.composite_canvas_image(image, width, height));
                }
                Err(error) => {
                    self.last_error = Some(WindowError::Render(error.to_string()));
                    return;
                }
            }
        }
        self.frames_presented = frame.frame_index;
    }

    fn current_overlay(&self) -> Vec<TextOverlayItem> {
        match &self.overlay_source {
            Some(source) => source.lock().map(|items| items.clone()).unwrap_or_default(),
            None => self.overlay.clone(),
        }
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use unitas_asset::project_settings::{QualityLevel, QualitySettings};
    use unitas_render::{RenderScene, Renderer, Rgba8, ShadowSettings};

    use super::{
        DESKTOP_QUALITY_TARGET_GROUP, FrameInfo, GpuFramePresenter, WEB_QUALITY_TARGET_GROUP,
        shadow_settings_for_quality, shadow_settings_for_runtime,
    };
    use crate::Runtime;

    fn sample_quality_level(
        name: &str,
        shadows: u32,
        shadow_resolution: u32,
        shadow_distance: f64,
    ) -> QualityLevel {
        QualityLevel {
            name: name.to_string(),
            pixel_light_count: 2,
            shadows,
            shadow_resolution,
            shadow_distance,
            texture_quality: 0,
            anisotropic_textures: 0,
            anti_aliasing: 0,
            v_sync_count: 1,
            lod_bias: 1.0,
        }
    }

    #[test]
    fn windowed_present_shows_the_canvas_the_capture_path_composes() {
        let Ok(mut runtime) = Runtime::new() else {
            return;
        };
        let scene_path = Path::new("../../samples/roll-a-ball/Assets/Scenes/RollABall.unity");
        let Ok(_) = runtime.load_scene(scene_path) else {
            return;
        };
        let width = 800u32;
        let height = 600u32;
        let items = runtime.ui_draw_items_for([width as f32, height as f32]);
        assert!(
            !items.is_empty(),
            "the scene has no canvas content to present"
        );
        let Ok(renderer) = Renderer::new() else {
            return;
        };
        let renderer = Arc::new(renderer);
        let mut presenter = GpuFramePresenter::new(Arc::clone(&renderer));
        presenter.bind_scene(&runtime);
        let captured: Arc<Mutex<Option<Rgba8>>> = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&captured);
        presenter.surface_blit = Some(Box::new(move |_renderer: &Renderer, image: &Rgba8| {
            let mut guard = sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(image.clone());
            Ok(())
        }));
        presenter.present_window_frame(
            &runtime,
            &FrameInfo {
                frame_index: 1,
                width,
                height,
                delta_seconds: 1.0 / 60.0,
            },
        );
        let presented = match captured.lock().ok().and_then(|guard| guard.clone()) {
            Some(image) => image,
            None => return,
        };
        assert_eq!(presented.width, width);
        assert_eq!(presented.height, height);
        let (camera, view, background) =
            presenter.resolved_camera_and_view(&runtime, width, height);
        let instances = presenter.collect_gpu_instances(&runtime);
        let scene = RenderScene {
            camera: &camera,
            view,
            light: presenter.light(),
            background,
        };
        let Ok(scene_frame) =
            renderer.render_offscreen_with_overlay(&instances, scene, (width, height), &[])
        else {
            return;
        };
        let scene_rgba = scene_frame.to_rgba8();
        let canvas_pixels = presented
            .pixels
            .chunks_exact(4)
            .zip(scene_rgba.pixels.chunks_exact(4))
            .filter(|pair| pair.0[..3] != pair.1[..3])
            .count();
        assert!(
            canvas_pixels > 0,
            "the presented frame has no canvas pixels"
        );
        let capture_frame = runtime
            .composite_canvas_image(scene_frame, width, height)
            .to_rgba8();
        let different = presented
            .pixels
            .chunks_exact(4)
            .zip(capture_frame.pixels.chunks_exact(4))
            .filter(|pair| pair.0 != pair.1)
            .count();
        assert_eq!(
            different, 0,
            "the presented frame differs from the capture composition"
        );
    }

    #[test]
    fn quality_shadow_settings_mapping_covers_shadows_resolutions_and_distances() {
        for &(shadows, expected_enabled, expected_soft) in &[
            (0_u32, false, false),
            (1_u32, true, false),
            (2_u32, true, true),
        ] {
            for resolution_index in 0..=3 {
                for &distance in &[40.0_f64, 50.0_f64, 70.0_f64] {
                    let level = sample_quality_level("Test", shadows, resolution_index, distance);
                    let mapped = shadow_settings_for_quality(&level);
                    assert_eq!(mapped.enabled, expected_enabled);
                    assert_eq!(mapped.soft, expected_soft);
                    assert_eq!(mapped.resolution_index, resolution_index);
                    assert!((mapped.distance - distance as f32).abs() < 1e-5);
                }
            }
        }
    }

    #[test]
    fn quality_level_for_target_prefers_per_platform_and_falls_back_to_current_quality() {
        let Ok(mut runtime) = Runtime::new() else {
            return;
        };
        assert_eq!(runtime.quality_settings(), &QualitySettings::default());
        assert!(
            runtime
                .quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP)
                .is_none()
        );
        assert_eq!(
            shadow_settings_for_runtime(&runtime),
            ShadowSettings::disabled()
        );

        runtime.quality_settings = QualitySettings {
            current_quality: 0,
            levels: vec![
                sample_quality_level("Low", 0, 0, 40.0),
                sample_quality_level("Medium", 1, 1, 70.0),
                sample_quality_level("High", 2, 2, 50.0),
            ],
            per_platform_default_quality: vec![
                (WEB_QUALITY_TARGET_GROUP.to_string(), 1),
                (DESKTOP_QUALITY_TARGET_GROUP.to_string(), 2),
            ],
        };

        let standalone = runtime.quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP);
        assert_eq!(standalone.map(|level| level.name.as_str()), Some("High"));

        let web = runtime.quality_level_for_target(WEB_QUALITY_TARGET_GROUP);
        assert_eq!(web.map(|level| level.name.as_str()), Some("Medium"));

        let fallback = runtime.quality_level_for_target("UnknownPlatform");
        assert_eq!(fallback.map(|level| level.name.as_str()), Some("Low"));
    }

    #[test]
    fn quality_settings_loaded_from_roll_a_ball_scene() {
        let Ok(mut runtime) = Runtime::new() else {
            return;
        };
        let scene_path = Path::new("../../samples/roll-a-ball/Assets/Scenes/RollABall.unity");
        let load_result = runtime.load_scene(scene_path);
        assert!(load_result.is_ok());
        assert_eq!(runtime.quality_settings().levels.len(), 3);
        assert_eq!(runtime.quality_settings().current_quality, 2);

        let standalone = runtime.quality_level_for_target(DESKTOP_QUALITY_TARGET_GROUP);
        assert!(standalone.is_some());
        if let Some(level) = standalone {
            assert_eq!(level.name, "High");
            assert_eq!(level.shadows, 2);
            assert_eq!(level.shadow_resolution, 2);
            assert!((level.shadow_distance - 50.0).abs() < 1e-6);
            assert_eq!(
                shadow_settings_for_quality(level),
                ShadowSettings {
                    enabled: true,
                    soft: true,
                    resolution_index: 2,
                    distance: 50.0,
                }
            );
        }

        let web = runtime.quality_level_for_target(WEB_QUALITY_TARGET_GROUP);
        assert!(web.is_some());
        if let Some(level) = web {
            assert_eq!(level.name, "Medium");
            assert_eq!(level.shadows, 1);
            assert_eq!(level.shadow_resolution, 1);
            assert!((level.shadow_distance - 70.0).abs() < 1e-6);
            assert_eq!(
                shadow_settings_for_quality(level),
                ShadowSettings {
                    enabled: true,
                    soft: false,
                    resolution_index: 1,
                    distance: 70.0,
                }
            );
        }
    }

    #[test]
    fn switching_quality_level_dynamically_reapplies_shadow_settings_and_v_sync_count() {
        let Ok(mut runtime) = Runtime::new() else {
            return;
        };
        let mut low = sample_quality_level("Low", 0, 0, 40.0);
        low.v_sync_count = 0;
        let mut medium = sample_quality_level("Medium", 1, 1, 70.0);
        medium.v_sync_count = 1;
        let mut high = sample_quality_level("High", 2, 2, 50.0);
        high.v_sync_count = 2;
        runtime.quality_settings = QualitySettings {
            current_quality: 2,
            levels: vec![low, medium, high],
            per_platform_default_quality: vec![(DESKTOP_QUALITY_TARGET_GROUP.to_string(), 2)],
        };
        let Ok(renderer) = Renderer::new() else {
            return;
        };
        let mut presenter = GpuFramePresenter::new(Arc::new(renderer));
        presenter.bind_scene(&runtime);
        assert_eq!(
            presenter.shadow_settings(),
            ShadowSettings {
                enabled: true,
                soft: true,
                resolution_index: 2,
                distance: 50.0,
            }
        );
        assert_eq!(presenter.v_sync_count(), 2);

        assert!(presenter.apply_quality_level(&mut runtime, 0));
        assert_eq!(runtime.quality_settings().current_quality, 0);
        assert_eq!(
            presenter.shadow_settings(),
            ShadowSettings {
                enabled: false,
                soft: false,
                resolution_index: 0,
                distance: 40.0,
            }
        );
        assert_eq!(presenter.v_sync_count(), 0);

        assert!(presenter.apply_quality_level(&mut runtime, 1));
        assert_eq!(runtime.quality_settings().current_quality, 1);
        assert_eq!(
            presenter.shadow_settings(),
            ShadowSettings {
                enabled: true,
                soft: false,
                resolution_index: 1,
                distance: 70.0,
            }
        );
        assert_eq!(presenter.v_sync_count(), 1);

        presenter.set_v_sync_count(0);
        assert_eq!(presenter.v_sync_count(), 0);
        assert_eq!(super::v_sync_count_for_runtime(&runtime), 1);

        assert!(!presenter.apply_quality_level(&mut runtime, 99));
        assert_eq!(runtime.quality_settings().current_quality, 1);
    }

    #[test]
    fn display_settings_state_tracks_resolution_fullscreen_pacing_and_quit() {
        let mut display = super::DisplaySettingsState::new(1280, 720, false, 1, -1, 0);
        assert_eq!(display.width(), 1280);
        assert_eq!(display.height(), 720);
        assert!(!display.full_screen());
        assert_eq!(display.v_sync_count(), 1);
        assert_eq!(display.target_frame_rate(), -1);
        assert_eq!(display.frame_interval(), None);
        assert!(!display.quit_requested());

        assert!(display.set_resolution(1920, 1080, true));
        assert_eq!(display.width(), 1920);
        assert_eq!(display.height(), 1080);
        assert!(display.full_screen());
        assert_eq!(
            display.take_pending_window_reconfigure(),
            Some((1920, 1080, true))
        );
        assert_eq!(display.take_pending_window_reconfigure(), None);

        assert!(!display.set_resolution(0, 1080, false));
        assert!(!display.set_resolution(1920, 0, false));
        assert_eq!(display.width(), 1920);
        assert_eq!(display.height(), 1080);

        display.set_full_screen(false);
        assert!(!display.full_screen());
        assert_eq!(
            display.take_pending_window_reconfigure(),
            Some((1920, 1080, false))
        );

        display.set_target_frame_rate(60);
        assert_eq!(display.target_frame_rate(), 60);
        assert_eq!(display.frame_interval(), None);

        display.set_v_sync_count(0);
        assert_eq!(display.v_sync_count(), 0);
        let interval = display.frame_interval().unwrap_or_default();
        assert!((interval.as_secs_f64() - (1.0 / 60.0)).abs() < 1e-6);

        display.set_target_frame_rate(0);
        assert_eq!(display.frame_interval(), None);
        display.set_target_frame_rate(-1);
        assert_eq!(display.frame_interval(), None);

        display.request_quit();
        assert!(display.quit_requested());
        display.clear_quit_request();
        assert!(!display.quit_requested());
    }

    #[test]
    fn runtime_display_ffi_controls_resolution_vsync_frame_rate_quality_and_quit() {
        super::unitas_runtime_display_reset();
        assert_eq!(super::unitas_runtime_display_width(), 1280);
        assert_eq!(super::unitas_runtime_display_height(), 720);
        assert_eq!(super::unitas_runtime_display_full_screen(), 0);
        assert_eq!(super::unitas_runtime_display_v_sync_count(), 1);
        assert_eq!(super::unitas_runtime_display_target_frame_rate(), -1);
        assert_eq!(super::unitas_runtime_display_quit_requested(), 0);

        assert_eq!(
            super::unitas_runtime_display_set_resolution(1600, 900, 1),
            0
        );
        assert_eq!(super::unitas_runtime_display_width(), 1600);
        assert_eq!(super::unitas_runtime_display_height(), 900);
        assert_eq!(super::unitas_runtime_display_full_screen(), 1);
        assert_eq!(super::unitas_runtime_display_set_resolution(0, 900, 0), -2);

        assert_eq!(super::unitas_runtime_display_set_full_screen(0), 0);
        assert_eq!(super::unitas_runtime_display_full_screen(), 0);

        assert_eq!(super::unitas_runtime_display_set_v_sync_count(0), 0);
        assert_eq!(super::unitas_runtime_display_v_sync_count(), 0);
        assert_eq!(super::unitas_runtime_display_set_v_sync_count(2), 0);
        assert_eq!(super::unitas_runtime_display_v_sync_count(), 2);

        assert_eq!(super::unitas_runtime_display_set_target_frame_rate(120), 0);
        assert_eq!(super::unitas_runtime_display_target_frame_rate(), 120);

        assert_eq!(super::unitas_runtime_display_set_quality_level(1), 0);
        assert_eq!(super::unitas_runtime_display_quality_level(), 1);

        assert_eq!(super::unitas_runtime_display_request_quit(), 0);
        assert_eq!(super::unitas_runtime_display_quit_requested(), 1);
        super::unitas_runtime_display_reset();
        assert_eq!(super::unitas_runtime_display_quit_requested(), 0);
    }
}
