#[macro_use]
mod abi;
pub mod abi_video;
pub mod animation;
pub mod canvas;
pub mod capture;
pub mod cinemachine;
#[cfg(not(target_arch = "wasm32"))]
pub mod dylib;
mod error;
pub mod input;
pub mod jobs;
pub mod navigation;
pub mod particles;
pub mod physics;
pub mod physics2d_colliders;
pub mod physics_settings;
pub mod player_loop;
pub mod presenter;
pub mod resources;
mod runtime;
mod runtime_objects;
mod runtime_script;
mod scene_query;
pub mod scriptable_object;
pub mod scripting;
pub mod terrain;
mod time;
pub mod timeline;
pub mod trails;
pub mod ui;
pub mod video;
pub mod web;
pub mod window;

pub use abi::*;
pub use abi_video::*;
pub use animation::{AnimationRuntimeDiagnostic, AnimationWorld, EntityAnimatorEntry};
pub use canvas::{CanvasRect, CanvasState, SpriteRef, UiDrawItem};
#[cfg(not(target_arch = "wasm32"))]
pub use dylib::{RuntimeDylib, RuntimeDylibError};
pub use error::RuntimeError;
pub use input::*;
pub use jobs::job_system;
pub use player_loop::{
    BuiltinWork, CustomSystemPosition, PlayerLoop, PlayerLoopCallback, PlayerLoopError,
    PlayerLoopSystem,
};
pub use presenter::{
    DisplaySettingsState, EffectiveRenderPipeline, FrameInfo, GpuCameraBinding, GpuFramePresenter,
    GpuMeshBinding, GpuWindowPresenter, WindowError, effective_render_pipeline,
    frame_pacing_interval, scene_gpu_meshes, set_runtime_quality_level,
    shadow_settings_for_quality, shadow_settings_for_runtime, transform_to_mat4,
    unitas_runtime_display_full_screen, unitas_runtime_display_height,
    unitas_runtime_display_quality_level, unitas_runtime_display_quit_requested,
    unitas_runtime_display_request_quit, unitas_runtime_display_reset,
    unitas_runtime_display_set_full_screen, unitas_runtime_display_set_quality_level,
    unitas_runtime_display_set_resolution, unitas_runtime_display_set_target_frame_rate,
    unitas_runtime_display_set_v_sync_count, unitas_runtime_display_target_frame_rate,
    unitas_runtime_display_v_sync_count, unitas_runtime_display_width, v_sync_count_for_runtime,
    view_matrix_from_transform, with_display_settings,
};
pub use runtime::{
    CONTACT_PHASE_2D_FLAG, Callbacks, DEFAULT_MAXIMUM_DELTA_TIME, LoadSceneMode, LoadedScene,
    PipelineSource, QualityPipelineReport, RenderPipelineReport, Runtime, ScriptInstance,
    UnsupportedFeatureReport, UnsupportedRenderFeature, unitas_runtime_bind_destroy_channel,
    unitas_runtime_mark_destroyed,
};
pub use unitas_physics::BodyRole;
pub use video::{
    DecodedVideoClip, VideoDecodeError, VideoEvents, VideoPlayerSnapshot, VideoPlayerState,
    decode_cooked_video, ffmpeg_libraries_available,
};
