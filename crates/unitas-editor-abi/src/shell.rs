use crate::dispatch::error_json;
use crate::ffi::{ShellEventCallback, allocate_c_string, free_c_string, read_c_string};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use unitas_core::EntityId;
use unitas_render::Image;
use unitas_runtime::Runtime;
use unitas_runtime::window::{GpuFramePresenter, WindowConfig, WindowError, run_windowed_gpu};

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 720;
const DEFAULT_HEADLESS_FRAMES: u64 = 4;
const DEFAULT_SCENE_RELATIVE: &str = "Assets/Scenes/WorkspaceShell.unity";
const BACKGROUND_RGB: [u8; 3] = [28, 31, 38];

struct ShellOptions {
    target_project_path: String,
    headless: bool,
    max_frames: Option<u64>,
    width: u32,
    height: u32,
    capture_out: Option<PathBuf>,
    events: Vec<Value>,
}

struct ShellEntityMap {
    title_text: Option<EntityId>,
    hierarchy_text: Option<EntityId>,
    inspector_text: Option<EntityId>,
    play_status_text: Option<EntityId>,
    select_next_button: Option<EntityId>,
    play_toggle_button: Option<EntityId>,
}

struct ShellLoopState {
    callback: Option<ShellEventCallback>,
    entities: ShellEntityMap,
    pending_events: Vec<Value>,
    event_cursor: usize,
    last_workspace_state: Value,
}

pub fn shell_run_impl(
    shell_project_path_ptr: usize,
    options_json_ptr: usize,
    callback: Option<ShellEventCallback>,
) -> usize {
    match run_shell_inner(shell_project_path_ptr, options_json_ptr, callback) {
        Ok(payload) => allocate_c_string(&payload.to_string()),
        Err((kind, message)) => allocate_c_string(
            &error_json(kind, Some("unitas_editor_run_shell"), &message).to_string(),
        ),
    }
}

fn run_shell_inner(
    shell_project_path_ptr: usize,
    options_json_ptr: usize,
    callback: Option<ShellEventCallback>,
) -> Result<Value, (&'static str, String)> {
    let shell_project_path = resolve_required_string(shell_project_path_ptr, "shellProjectPath")?;
    let options = parse_shell_options(options_json_ptr)?;
    let scene_path = resolve_shell_scene_path(Path::new(&shell_project_path))?;
    let mut runtime = Runtime::new().map_err(|err| ("runtime-init", err.to_string()))?;
    runtime
        .load_scene(&scene_path)
        .map_err(|err| ("scene-load", err.to_string()))?;
    runtime.set_ui_screen_size(options.width as f32, options.height as f32);
    let entities = discover_shell_entities(&runtime);
    let mut state = build_initial_loop_state(callback, entities, &options);
    dispatch_pending_events_for_frame(&mut runtime, &mut state, 0);
    execute_shell_frames(&mut runtime, &options, &mut state)
}

fn resolve_required_string(ptr: usize, field_name: &str) -> Result<String, (&'static str, String)> {
    if ptr == 0 {
        return Err(("invalid-args", format!("{field_name} pointer is null")));
    }
    let Some(Ok(value)) = read_c_string(ptr) else {
        return Err(("invalid-args", format!("{field_name} is not valid UTF-8")));
    };
    if value.trim().is_empty() {
        return Err(("invalid-args", format!("{field_name} must not be empty")));
    }
    Ok(value)
}

fn parse_shell_options(options_json_ptr: usize) -> Result<ShellOptions, (&'static str, String)> {
    let parsed = if options_json_ptr == 0 {
        json!({})
    } else {
        let Some(Ok(raw)) = read_c_string(options_json_ptr) else {
            return Err(("invalid-args", "optionsJson is not valid UTF-8".to_string()));
        };
        serde_json::from_str::<Value>(&raw).map_err(|err| {
            (
                "invalid-args",
                format!("optionsJson is not valid JSON: {err}"),
            )
        })?
    };
    Ok(extract_shell_options(&parsed))
}

fn extract_shell_options(parsed: &Value) -> ShellOptions {
    let env_headless = std::env::var("UNITAS_SHELL_HEADLESS")
        .ok()
        .is_some_and(|val| val == "1" || val.eq_ignore_ascii_case("true"));
    let headless = parsed
        .get("headless")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || env_headless;
    let env_frames = std::env::var("UNITAS_SHELL_MAX_FRAMES")
        .ok()
        .and_then(|val| val.parse::<u64>().ok());
    let max_frames = parsed
        .get("maxFrames")
        .and_then(Value::as_u64)
        .or(env_frames);
    let width = parsed
        .get("width")
        .and_then(Value::as_u64)
        .and_then(|val| u32::try_from(val).ok())
        .filter(|&val| val > 0)
        .unwrap_or(DEFAULT_WIDTH);
    let height = parsed
        .get("height")
        .and_then(Value::as_u64)
        .and_then(|val| u32::try_from(val).ok())
        .filter(|&val| val > 0)
        .unwrap_or(DEFAULT_HEIGHT);
    let capture_out = parsed
        .get("captureOut")
        .and_then(Value::as_str)
        .filter(|val| !val.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("UNITAS_SHELL_CAPTURE_PATH")
                .ok()
                .filter(|val| !val.trim().is_empty())
                .map(PathBuf::from)
        });
    let events = parsed
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let target_project_path = parsed
        .get("targetProjectPath")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    ShellOptions {
        target_project_path,
        headless,
        max_frames,
        width,
        height,
        capture_out,
        events,
    }
}

fn resolve_shell_scene_path(shell_root: &Path) -> Result<PathBuf, (&'static str, String)> {
    if shell_root.is_file() {
        return Ok(shell_root.to_path_buf());
    }
    let candidate = shell_root.join(DEFAULT_SCENE_RELATIVE);
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err((
        "scene-missing",
        format!("shell scene does not exist at '{}'", candidate.display()),
    ))
}

fn discover_shell_entities(runtime: &Runtime) -> ShellEntityMap {
    ShellEntityMap {
        title_text: runtime.find_entity_by_name("WorkspaceTitleText"),
        hierarchy_text: runtime.find_entity_by_name("HierarchyListText"),
        inspector_text: runtime.find_entity_by_name("InspectorPanelText"),
        play_status_text: runtime.find_entity_by_name("PlayStatusText"),
        select_next_button: runtime.find_entity_by_name("SelectNextButton"),
        play_toggle_button: runtime.find_entity_by_name("PlayToggleButton"),
    }
}

fn build_initial_loop_state(
    callback: Option<ShellEventCallback>,
    entities: ShellEntityMap,
    options: &ShellOptions,
) -> ShellLoopState {
    let mut pending_events = Vec::new();
    let has_init = options
        .events
        .first()
        .and_then(|ev| ev.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "init");
    if !has_init {
        pending_events.push(json!({
            "type": "init",
            "projectPath": options.target_project_path,
        }));
    }
    pending_events.extend(options.events.iter().cloned());
    ShellLoopState {
        callback,
        entities,
        pending_events,
        event_cursor: 0,
        last_workspace_state: json!({}),
    }
}

fn dispatch_event_to_managed(runtime: &mut Runtime, state: &mut ShellLoopState, event: &Value) {
    let Some(cb) = state.callback else {
        return;
    };
    let event_ptr = allocate_c_string(&event.to_string());
    if event_ptr == 0 {
        return;
    }
    let response_ptr = cb(event_ptr);
    free_c_string(event_ptr);
    if response_ptr == 0 {
        return;
    }
    let Some(Ok(response_text)) = read_c_string(response_ptr) else {
        free_c_string(response_ptr);
        return;
    };
    free_c_string(response_ptr);
    if let Ok(parsed) = serde_json::from_str::<Value>(&response_text) {
        apply_workspace_state(runtime, &state.entities, &parsed);
        state.last_workspace_state = parsed;
    }
}

fn apply_workspace_state(runtime: &mut Runtime, entities: &ShellEntityMap, state: &Value) {
    if let (Some(id), Some(text)) = (
        entities.title_text,
        state.get("titleText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), text);
    }
    if let (Some(id), Some(text)) = (
        entities.hierarchy_text,
        state.get("hierarchyText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), text);
    }
    if let (Some(id), Some(text)) = (
        entities.inspector_text,
        state.get("inspectorText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), text);
    }
    if let (Some(id), Some(text)) = (
        entities.play_status_text,
        state.get("playStateText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), text);
    }
}

fn dispatch_pending_events_for_frame(
    runtime: &mut Runtime,
    state: &mut ShellLoopState,
    frame_index: u64,
) {
    if frame_index == 0 {
        if let Some(first) = state.pending_events.first().cloned() {
            state.event_cursor = 1;
            dispatch_event_to_managed(runtime, state, &first);
        }
        return;
    }
    if state.event_cursor < state.pending_events.len() {
        let event = state.pending_events[state.event_cursor].clone();
        state.event_cursor += 1;
        dispatch_event_to_managed(runtime, state, &event);
    }
    for clicked_id in runtime.take_ui_clicks() {
        let clicked = EntityId::new(clicked_id);
        let mapped = if Some(clicked) == state.entities.select_next_button {
            Some(json!({"type": "select_next"}))
        } else if Some(clicked) == state.entities.play_toggle_button {
            Some(json!({"type": "command", "command": "play.toggle"}))
        } else {
            None
        };
        if let Some(ev) = mapped {
            dispatch_event_to_managed(runtime, state, &ev);
        }
    }
}

fn execute_shell_frames(
    runtime: &mut Runtime,
    options: &ShellOptions,
    state: &mut ShellLoopState,
) -> Result<Value, (&'static str, String)> {
    let mut presenter =
        GpuFramePresenter::from_new_renderer().map_err(|err| ("render-init", err.to_string()))?;
    presenter.bind_scene(runtime);
    let frames_rendered = if options.headless {
        run_headless_loop(runtime, &presenter, options, state)?
    } else {
        run_windowed_loop(runtime, options, state)?
    };
    let final_image = presenter
        .render_offscreen(runtime, options.width, options.height)
        .map_err(|err| ("render-frame", err.to_string()))?;
    let capture_path = write_capture_if_requested(&final_image, options.capture_out.as_deref())?;
    let non_bg = count_non_background_pixels(&final_image);
    let texts = collect_canvas_texts(runtime);
    let _ = runtime.shutdown();
    Ok(json!({
        "ok": true,
        "framesRendered": frames_rendered,
        "nonBackgroundPixels": non_bg,
        "canvasTexts": texts,
        "capturePath": capture_path,
        "lastWorkspaceState": state.last_workspace_state,
    }))
}

fn run_headless_loop(
    runtime: &mut Runtime,
    presenter: &GpuFramePresenter<'_>,
    options: &ShellOptions,
    state: &mut ShellLoopState,
) -> Result<u64, (&'static str, String)> {
    let total_frames = options
        .max_frames
        .unwrap_or(DEFAULT_HEADLESS_FRAMES)
        .max(state.pending_events.len() as u64)
        .max(1);
    for frame in 1..=total_frames {
        runtime
            .tick(1.0 / 60.0)
            .map_err(|err| ("runtime-tick", err.to_string()))?;
        dispatch_pending_events_for_frame(runtime, state, frame);
        let _ = presenter
            .render_offscreen(runtime, options.width, options.height)
            .map_err(|err| ("render-frame", err.to_string()))?;
    }
    Ok(total_frames)
}

fn run_windowed_loop(
    runtime: &mut Runtime,
    options: &ShellOptions,
    state: &mut ShellLoopState,
) -> Result<u64, (&'static str, String)> {
    let title = state
        .last_workspace_state
        .get("titleText")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("unitas Editor")
        .to_string();
    let config = WindowConfig {
        title,
        width: options.width,
        height: options.height,
        resizable: true,
        frame_limit: options.max_frames.unwrap_or(0),
        max_frames: options.max_frames,
        fixed_frame_delta_seconds: Some(1.0 / 60.0),
        exit_on_escape: true,
    };
    let mut presenter =
        GpuFramePresenter::from_new_renderer().map_err(|err| ("render-init", err.to_string()))?;
    presenter.bind_scene(runtime);
    presenter.set_update_callback(|rt, frame| -> Result<(), WindowError> {
        dispatch_pending_events_for_frame(rt, state, frame.frame_index);
        Ok(())
    });
    let run_result = run_windowed_gpu(runtime, &config, &mut presenter);
    run_result.map_err(|err| ("window-loop", err.to_string()))?;
    Ok(presenter.frames_presented().max(1))
}

fn write_capture_if_requested(
    image: &Image,
    capture_out: Option<&Path>,
) -> Result<Option<String>, (&'static str, String)> {
    let Some(path) = capture_out else {
        return Ok(None);
    };
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| ("capture-io", err.to_string()))?;
    }
    image
        .write_bmp(path)
        .map_err(|err| ("capture-io", err.to_string()))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

fn count_non_background_pixels(image: &Image) -> usize {
    image
        .pixels
        .iter()
        .filter(|&&px| {
            let dr = i32::from(px[0]) - i32::from(BACKGROUND_RGB[0]);
            let dg = i32::from(px[1]) - i32::from(BACKGROUND_RGB[1]);
            let db = i32::from(px[2]) - i32::from(BACKGROUND_RGB[2]);
            dr.abs() > 3 || dg.abs() > 3 || db.abs() > 3
        })
        .count()
}

fn collect_canvas_texts(runtime: &Runtime) -> Vec<String> {
    runtime
        .ui_draw_items()
        .into_iter()
        .filter_map(|item| item.text.map(|t| t.text))
        .collect()
}
