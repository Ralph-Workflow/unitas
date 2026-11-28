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
const HIERARCHY_ROW_COUNT: usize = 12;
const PROPERTY_ROW_COUNT: usize = 8;

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
    play_status_text: Option<EntityId>,
    select_next_button: Option<EntityId>,
    play_toggle_button: Option<EntityId>,
    hierarchy_row_buttons: [Option<EntityId>; HIERARCHY_ROW_COUNT],
    hierarchy_row_labels: [Option<EntityId>; HIERARCHY_ROW_COUNT],
    property_row_buttons: [Option<EntityId>; PROPERTY_ROW_COUNT],
    property_row_labels: [Option<EntityId>; PROPERTY_ROW_COUNT],
    step_up_button: Option<EntityId>,
    step_down_button: Option<EntityId>,
    toggle_value_button: Option<EntityId>,
    create_object_button: Option<EntityId>,
    duplicate_object_button: Option<EntityId>,
    delete_object_button: Option<EntityId>,
    active_toggle_button: Option<EntityId>,
    undo_button: Option<EntityId>,
    redo_button: Option<EntityId>,
    search_input: Option<EntityId>,
    search_placeholder_text: Option<EntityId>,
    page_prev_button: Option<EntityId>,
    page_next_button: Option<EntityId>,
    result_count_text: Option<EntityId>,
    save_button: Option<EntityId>,
    save_status_text: Option<EntityId>,
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
        serde_json::from_str::<Value>(&raw).map_err(|e| {
            (
                "invalid-args",
                format!("optionsJson is not valid JSON: {e}"),
            )
        })?
    };
    Ok(extract_shell_options(&parsed))
}

fn extract_shell_options(parsed: &Value) -> ShellOptions {
    let env_hl = std::env::var("UNITAS_SHELL_HEADLESS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let headless = parsed
        .get("headless")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || env_hl;
    let env_fr = std::env::var("UNITAS_SHELL_MAX_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let max_frames = parsed.get("maxFrames").and_then(Value::as_u64).or(env_fr);
    let dim = |k, def| {
        parsed
            .get(k)
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|&v| v > 0)
            .unwrap_or(def)
    };
    let capture_out = parsed
        .get("captureOut")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| std::env::var("UNITAS_SHELL_CAPTURE_PATH").ok())
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from);
    let target_project_path = parsed
        .get("targetProjectPath")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let events = parsed
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    ShellOptions {
        target_project_path,
        headless,
        max_frames,
        width: dim("width", DEFAULT_WIDTH),
        height: dim("height", DEFAULT_HEIGHT),
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
    let find = |name: &str| runtime.find_entity_by_name(name);
    ShellEntityMap {
        title_text: find("WorkspaceTitleText"),
        play_status_text: find("PlayStatusText"),
        select_next_button: find("SelectNextButton"),
        play_toggle_button: find("PlayToggleButton"),
        hierarchy_row_buttons: std::array::from_fn(|i| find(&format!("HierarchyRowButton{i}"))),
        hierarchy_row_labels: std::array::from_fn(|i| find(&format!("HierarchyRowLabel{i}"))),
        property_row_buttons: std::array::from_fn(|i| find(&format!("PropertyRowButton{i}"))),
        property_row_labels: std::array::from_fn(|i| find(&format!("PropertyRowLabel{i}"))),
        step_up_button: find("StepUpButton"),
        step_down_button: find("StepDownButton"),
        toggle_value_button: find("ToggleValueButton"),
        create_object_button: find("CreateObjectButton"),
        duplicate_object_button: find("DuplicateObjectButton"),
        delete_object_button: find("DeleteObjectButton"),
        active_toggle_button: find("ActiveToggleButton"),
        undo_button: find("UndoButton"),
        redo_button: find("RedoButton"),
        search_input: find("SearchInput"),
        search_placeholder_text: find("SearchPlaceholderText"),
        page_prev_button: find("PagePrevButton"),
        page_next_button: find("PageNextButton"),
        result_count_text: find("ResultCountText"),
        save_button: find("SaveButton"),
        save_status_text: find("SaveStatusText"),
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
        pending_events.push(json!({"type": "init", "projectPath": options.target_project_path}));
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
    let Some(cb) = state.callback else { return };
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
    if let (Some(id), Some(t)) = (
        entities.title_text,
        state.get("titleText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), t);
    }
    if let (Some(id), Some(t)) = (
        entities.play_status_text,
        state.get("playStateText").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), t);
    }
    apply_search_view_fields(runtime, entities, state);
    let set_rows = |rt: &mut Runtime,
                    labels: &[Option<EntityId>],
                    btns: &[Option<EntityId>],
                    rows: &[Value]| {
        for (idx, (lbl, btn)) in labels.iter().zip(btns.iter()).enumerate() {
            let text = rows.get(idx).and_then(Value::as_str).unwrap_or("");
            if let Some(id) = lbl {
                rt.set_ui_text(id.as_u64(), text);
            }
            if let Some(id) = btn {
                rt.set_ui_text(id.as_u64(), text);
            }
        }
    };
    if let Some(rows) = state.get("hierarchyRows").and_then(Value::as_array) {
        set_rows(
            runtime,
            &entities.hierarchy_row_labels,
            &entities.hierarchy_row_buttons,
            rows,
        );
    }
    if let Some(rows) = state.get("propertyRows").and_then(Value::as_array) {
        set_rows(
            runtime,
            &entities.property_row_labels,
            &entities.property_row_buttons,
            rows,
        );
    }
}

fn apply_search_view_fields(runtime: &mut Runtime, entities: &ShellEntityMap, state: &Value) {
    if let (Some(id), Some(val)) = (entities.result_count_text, state.get("resultCount")) {
        let s = match val {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => val.to_string(),
        };
        runtime.set_ui_text(id.as_u64(), &s);
    }
    if let (Some(id), Some(t)) = (
        entities.save_status_text,
        state.get("saveStatus").and_then(Value::as_str),
    ) {
        runtime.set_ui_text(id.as_u64(), t);
    }
    if let Some(q) = state.get("searchQuery").and_then(Value::as_str) {
        let applied = entities
            .search_input
            .is_some_and(|id| runtime.set_ui_input_field_text(id.as_u64(), q, false));
        if !applied
            && !q.is_empty()
            && let Some(id) = entities.search_placeholder_text
        {
            runtime.set_ui_text(id.as_u64(), q);
        }
    }
}

fn map_clicked_entity(e: &ShellEntityMap, clicked: EntityId) -> Option<Value> {
    if let Some(idx) = e
        .hierarchy_row_buttons
        .iter()
        .position(|&b| b == Some(clicked))
    {
        return Some(json!({"type": "select", "index": idx}));
    }
    if let Some(idx) = e
        .property_row_buttons
        .iter()
        .position(|&b| b == Some(clicked))
    {
        return Some(json!({"type": "select_property", "index": idx}));
    }
    let t = Some(clicked);
    let direct = [
        (
            e.step_up_button,
            json!({"type": "step_property", "delta": 1}),
        ),
        (
            e.step_down_button,
            json!({"type": "step_property", "delta": -1}),
        ),
        (e.toggle_value_button, json!({"type": "toggle_property"})),
        (e.active_toggle_button, json!({"type": "toggle_active"})),
        (e.select_next_button, json!({"type": "select_next"})),
        (e.page_prev_button, json!({"type": "page", "delta": -1})),
        (e.page_next_button, json!({"type": "page", "delta": 1})),
        (e.save_button, json!({"type": "save"})),
    ];
    if let Some((_, event)) = direct.into_iter().find(|(button, _)| *button == t) {
        return Some(event);
    }
    let cmds = [
        (e.create_object_button, "scene.create_object"),
        (e.duplicate_object_button, "scene.duplicate_object"),
        (e.delete_object_button, "scene.destroy_object"),
        (e.undo_button, "editor.undo"),
        (e.redo_button, "editor.redo"),
        (e.play_toggle_button, "play.toggle"),
    ];
    cmds.into_iter()
        .find_map(|(b, cmd)| (b == t).then(|| json!({"type": "command", "command": cmd})))
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
        if let Some(ev) = map_clicked_entity(&state.entities, clicked) {
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
    run_windowed_gpu(runtime, &config, &mut presenter)
        .map_err(|err| ("window-loop", err.to_string()))?;
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
            (i32::from(px[0]) - i32::from(BACKGROUND_RGB[0])).abs() > 3
                || (i32::from(px[1]) - i32::from(BACKGROUND_RGB[1])).abs() > 3
                || (i32::from(px[2]) - i32::from(BACKGROUND_RGB[2])).abs() > 3
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

#[cfg(test)]
fn click_test_entity_map() -> ShellEntityMap {
    let f = |i| Some(EntityId::new(i));
    ShellEntityMap {
        title_text: f(1),
        play_status_text: f(2),
        select_next_button: f(10),
        play_toggle_button: f(11),
        hierarchy_row_buttons: std::array::from_fn(|i| f(100 + i as u64)),
        hierarchy_row_labels: std::array::from_fn(|i| f(200 + i as u64)),
        property_row_buttons: std::array::from_fn(|i| f(300 + i as u64)),
        property_row_labels: std::array::from_fn(|i| f(400 + i as u64)),
        step_up_button: f(20),
        step_down_button: f(21),
        toggle_value_button: f(22),
        create_object_button: f(23),
        duplicate_object_button: f(24),
        delete_object_button: f(25),
        active_toggle_button: f(26),
        undo_button: f(27),
        redo_button: f(28),
        search_input: f(29),
        search_placeholder_text: f(30),
        page_prev_button: f(31),
        page_next_button: f(32),
        result_count_text: f(33),
        save_button: f(34),
        save_status_text: f(35),
    }
}

#[cfg(test)]
#[test]
fn map_clicked_entity_covers_all_rows_and_action_buttons() {
    let e = click_test_entity_map();
    let m = |id| map_clicked_entity(&e, EntityId::new(id));
    assert_eq!(m(100), Some(json!({"type": "select", "index": 0})));
    assert_eq!(m(111), Some(json!({"type": "select", "index": 11})));
    assert_eq!(m(300), Some(json!({"type": "select_property", "index": 0})));
    assert_eq!(m(307), Some(json!({"type": "select_property", "index": 7})));
    assert_eq!(m(20), Some(json!({"type": "step_property", "delta": 1})));
    assert_eq!(m(21), Some(json!({"type": "step_property", "delta": -1})));
    assert_eq!(m(22), Some(json!({"type": "toggle_property"})));
    assert_eq!(m(26), Some(json!({"type": "toggle_active"})));
    assert_eq!(m(10), Some(json!({"type": "select_next"})));
    assert_eq!(m(31), Some(json!({"type": "page", "delta": -1})));
    assert_eq!(m(32), Some(json!({"type": "page", "delta": 1})));
    assert_eq!(m(34), Some(json!({"type": "save"})));
    for (id, c) in [
        (23, "scene.create_object"),
        (24, "scene.duplicate_object"),
        (25, "scene.destroy_object"),
        (27, "editor.undo"),
        (28, "editor.redo"),
        (11, "play.toggle"),
    ] {
        assert_eq!(m(id), Some(json!({"type": "command", "command": c})));
    }
    assert_eq!(m(999), None);
}

#[cfg(test)]
const SHELL_VIEW_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: HudCanvas
  m_Layer: 5
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!224 &3
RectTransform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!223 &4
Canvas:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_RenderMode: 0
  m_SortingOrder: 0
--- !u!114 &5
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 800, y: 600}
  m_ScreenMatchMode: 0
  m_MatchWidthOrHeight: 0.5
--- !u!1 &10
GameObject:
  m_Name: SearchInput
  m_Layer: 5
--- !u!4 &11
Transform:
  m_GameObject: {fileID: 10}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &12
RectTransform:
  m_GameObject: {fileID: 10}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 16, y: -48}
  m_SizeDelta: {x: 240, y: 28}
  m_Pivot: {x: 0, y: 1}
--- !u!1 &20
GameObject:
  m_Name: SearchPlaceholderText
  m_Layer: 5
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &22
RectTransform:
  m_GameObject: {fileID: 20}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 20, y: -52}
  m_SizeDelta: {x: 232, y: 20}
  m_Pivot: {x: 0, y: 1}
--- !u!114 &23
MonoBehaviour:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: Search
  m_FontSize: 14
  m_Color: {r: 1, g: 1, b: 1, a: 1}
--- !u!1 &30
GameObject:
  m_Name: PagePrevButton
  m_Layer: 5
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 30}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 0, y: 0}
  m_AnchoredPosition: {x: 16, y: 12}
  m_SizeDelta: {x: 32, y: 24}
  m_Pivot: {x: 0, y: 0}
--- !u!1 &40
GameObject:
  m_Name: PageNextButton
  m_Layer: 5
--- !u!4 &41
Transform:
  m_GameObject: {fileID: 40}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &42
RectTransform:
  m_GameObject: {fileID: 40}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 0, y: 0}
  m_AnchoredPosition: {x: 56, y: 12}
  m_SizeDelta: {x: 32, y: 24}
  m_Pivot: {x: 0, y: 0}
--- !u!1 &50
GameObject:
  m_Name: ResultCountText
  m_Layer: 5
--- !u!4 &51
Transform:
  m_GameObject: {fileID: 50}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &52
RectTransform:
  m_GameObject: {fileID: 50}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 0, y: 0}
  m_AnchoredPosition: {x: 96, y: 12}
  m_SizeDelta: {x: 120, y: 24}
  m_Pivot: {x: 0, y: 0}
--- !u!114 &53
MonoBehaviour:
  m_GameObject: {fileID: 50}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: 0 results
  m_FontSize: 14
  m_Color: {r: 1, g: 1, b: 1, a: 1}
--- !u!1 &60
GameObject:
  m_Name: SaveButton
  m_Layer: 5
--- !u!4 &61
Transform:
  m_GameObject: {fileID: 60}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &62
RectTransform:
  m_GameObject: {fileID: 60}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 1, y: 1}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: -16, y: -12}
  m_SizeDelta: {x: 80, y: 28}
  m_Pivot: {x: 1, y: 1}
--- !u!1 &70
GameObject:
  m_Name: SaveStatusText
  m_Layer: 5
--- !u!4 &71
Transform:
  m_GameObject: {fileID: 70}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &72
RectTransform:
  m_GameObject: {fileID: 70}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 1, y: 1}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: -104, y: -16}
  m_SizeDelta: {x: 120, y: 20}
  m_Pivot: {x: 1, y: 1}
--- !u!114 &73
MonoBehaviour:
  m_GameObject: {fileID: 70}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: clean
  m_FontSize: 14
  m_Color: {r: 1, g: 1, b: 1, a: 1}
";

#[cfg(test)]
static RECORDED_SHELL_EVENTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
extern "C" fn record_shell_event(event_ptr: usize) -> usize {
    let Some(Ok(event)) = read_c_string(event_ptr) else {
        return 0;
    };
    if let Ok(mut recorded) = RECORDED_SHELL_EVENTS.lock() {
        recorded.push(event);
    }
    allocate_c_string(
        "{\"titleText\":\"Workspace\",\"searchQuery\":\"boss\",\"resultCount\":7,\
         \"pageIndex\":1,\"pageCount\":3,\"saveStatus\":\"dirty\"}",
    )
}

#[cfg(test)]
fn assert_shell_events_and_view(
    runtime: &Runtime,
    state: &ShellLoopState,
) -> Result<(), Box<dyn std::error::Error>> {
    let recorded = RECORDED_SHELL_EVENTS
        .lock()
        .map_err(|_| "event log lock poisoned")?
        .clone();
    let events: Vec<Value> = recorded
        .iter()
        .map(|raw| serde_json::from_str(raw))
        .collect::<Result<_, _>>()?;
    assert_eq!(
        events,
        vec![
            json!({"type": "search.set", "query": "x"}),
            json!({"type": "page", "delta": 1}),
            json!({"type": "save"}),
        ]
    );
    let texts = collect_canvas_texts(runtime);
    for expected in ["boss", "7", "dirty"] {
        assert!(
            texts.iter().any(|text| text == expected),
            "the canvas texts miss {expected}"
        );
    }
    assert_eq!(state.last_workspace_state["pageIndex"], json!(1));
    assert_eq!(state.last_workspace_state["pageCount"], json!(3));
    Ok(())
}

#[cfg(test)]
#[test]
fn shell_events_pass_through_and_view_fields_render() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir().join(format!("unitas_shell_view_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;
    let scene_path = temp_dir.join("shell_view.unity");
    std::fs::write(&scene_path, SHELL_VIEW_SCENE)?;
    let mut runtime = Runtime::new()?;
    runtime.load_scene(&scene_path)?;
    runtime.set_ui_screen_size(800.0, 600.0);
    let entities = discover_shell_entities(&runtime);
    assert!(entities.search_input.is_some());
    assert!(entities.save_button.is_some());
    if let Ok(mut recorded) = RECORDED_SHELL_EVENTS.lock() {
        recorded.clear();
    }
    let mut state = ShellLoopState {
        callback: Some(record_shell_event),
        entities,
        pending_events: vec![
            json!({"type": "search.set", "query": "x"}),
            json!({"type": "page", "delta": 1}),
            json!({"type": "save"}),
        ],
        event_cursor: 0,
        last_workspace_state: json!({}),
    };
    for frame in 0..3 {
        dispatch_pending_events_for_frame(&mut runtime, &mut state, frame);
    }
    assert_shell_events_and_view(&runtime, &state)?;
    let _ = runtime.shutdown();
    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}
