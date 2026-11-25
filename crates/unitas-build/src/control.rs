use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use unitas_runtime::Runtime;

#[cfg(feature = "device")]
use crate::RollABallController;
#[cfg(feature = "device")]
use unitas_render::{CaptureInstance, CaptureObject};
#[cfg(feature = "device")]
use unitas_runtime::window::{transform_to_mat4, GpuFramePresenter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    Playing,
    Paused,
    Stopped,
}

impl SessionState {
    fn as_str(self) -> &'static str {
        match self {
            SessionState::Playing => "playing",
            SessionState::Paused => "paused",
            SessionState::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InjectedInput {
    Key { key: u32, pressed: bool },
    MouseButton { button: u32, pressed: bool },
    PointerMove { x: f64, y: f64 },
    Scroll { x: f64, y: f64 },
}

impl InjectedInput {
    fn apply(self, runtime: &mut Runtime) {
        match self {
            InjectedInput::Key { key, pressed } => runtime.input_key_event(key, pressed),
            InjectedInput::MouseButton { button, pressed } => {
                runtime.input_mouse_button_event(button, pressed);
            }
            InjectedInput::PointerMove { x, y } => runtime.input_pointer_move(x, y),
            InjectedInput::Scroll { x, y } => runtime.input_scroll(x, y),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Request {
    Play,
    Pause,
    Stop,
    State,
    Step {
        frames: u64,
    },
    InjectInput {
        events: Vec<InjectedInput>,
    },
    Capture {
        width: u32,
        height: u32,
        id_map: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidRequest,
    UnknownCommand,
    InvalidStep,
    InvalidInputEvent,
    InvalidCapture,
    NotPlaying,
    CaptureWithoutIdMap,
    #[cfg(not(feature = "device"))]
    CaptureRequiresDevice,
    #[cfg(feature = "device")]
    CaptureFailed(String),
    FrameFailed(String),
    SessionStartFailed(String),
}

impl ProtocolError {
    pub fn outcome(&self) -> String {
        match self {
            ProtocolError::InvalidRequest => String::from("invalid request"),
            ProtocolError::UnknownCommand => String::from("unknown command"),
            ProtocolError::InvalidStep => String::from("invalid step request"),
            ProtocolError::InvalidInputEvent => String::from("invalid input event"),
            ProtocolError::InvalidCapture => String::from("invalid capture request"),
            ProtocolError::NotPlaying => String::from("not playing"),
            ProtocolError::CaptureWithoutIdMap => String::from("capture requires an id map"),
            #[cfg(not(feature = "device"))]
            ProtocolError::CaptureRequiresDevice => {
                String::from("capture requires the device feature")
            }
            #[cfg(feature = "device")]
            ProtocolError::CaptureFailed(message) => format!("capture failed: {message}"),
            ProtocolError::FrameFailed(message) => format!("frame failed: {message}"),
            ProtocolError::SessionStartFailed(message) => {
                format!("session start failed: {message}")
            }
        }
    }
}

pub struct ControlSession {
    runtime: Runtime,
    scene_path: PathBuf,
    state: SessionState,
    frames: u64,
    delta_seconds: f64,
    random_seed: u64,
    #[cfg(feature = "device")]
    presenter: GpuFramePresenter<'static>,
    #[cfg(feature = "device")]
    controller: RollABallController,
}

impl ControlSession {
    pub fn start(
        scene_path: &Path,
        delta_seconds: f64,
        random_seed: u64,
    ) -> Result<Self, ProtocolError> {
        let runtime = load_runtime(scene_path)?;
        #[cfg(feature = "device")]
        let (runtime, presenter, controller) = bind_scene(runtime)?;
        Ok(Self {
            runtime,
            scene_path: scene_path.to_path_buf(),
            state: SessionState::Paused,
            frames: 0,
            delta_seconds,
            random_seed,
            #[cfg(feature = "device")]
            presenter,
            #[cfg(feature = "device")]
            controller,
        })
    }

    fn dispatch(&mut self, request: &Request) -> Result<Value, ProtocolError> {
        match request {
            Request::Play => {
                self.play()?;
                Ok(self.state_payload())
            }
            Request::Pause => {
                self.pause()?;
                Ok(self.state_payload())
            }
            Request::Stop => {
                self.stop();
                Ok(self.state_payload())
            }
            Request::State => Ok(self.state_payload()),
            Request::Step { frames } => {
                self.advance(*frames)?;
                Ok(self.state_payload())
            }
            Request::InjectInput { events } => {
                self.inject(events)?;
                Ok(self.state_payload())
            }
            Request::Capture {
                width,
                height,
                id_map,
            } => self.capture(*width, *height, *id_map),
        }
    }

    fn play(&mut self) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            self.restart()?;
        }
        self.state = SessionState::Playing;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        self.state = SessionState::Paused;
        Ok(())
    }

    fn stop(&mut self) {
        let _ = self.runtime.shutdown();
        self.state = SessionState::Stopped;
    }

    fn restart(&mut self) -> Result<(), ProtocolError> {
        self.runtime = load_runtime(&self.scene_path)?;
        self.frames = 0;
        #[cfg(feature = "device")]
        {
            self.controller =
                RollABallController::bind_to_scene(&mut self.runtime, &mut self.presenter)
                    .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
        }
        Ok(())
    }

    fn advance(&mut self, frames: u64) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        for _ in 0..frames {
            self.runtime
                .tick(self.delta_seconds)
                .map_err(|error| ProtocolError::FrameFailed(error.to_string()))?;
            #[cfg(feature = "device")]
            self.controller
                .step(&mut self.runtime, &mut self.presenter, self.delta_seconds)
                .map_err(|error| ProtocolError::FrameFailed(error.to_string()))?;
            self.frames += 1;
        }
        Ok(())
    }

    fn inject(&mut self, events: &[InjectedInput]) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        for event in events {
            event.apply(&mut self.runtime);
        }
        Ok(())
    }

    fn capture(&mut self, width: u32, height: u32, id_map: bool) -> Result<Value, ProtocolError> {
        if !id_map {
            return Err(ProtocolError::CaptureWithoutIdMap);
        }
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        capture_frame(self, width, height)
    }

    fn state_payload(&self) -> Value {
        json!({
            "ok": true,
            "state": self.state.as_str(),
            "frames": self.frames,
            "delta_seconds": self.delta_seconds,
            "random_seed": self.random_seed,
        })
    }
}

fn load_runtime(scene_path: &Path) -> Result<Runtime, ProtocolError> {
    let mut runtime =
        Runtime::new().map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
    runtime
        .load_scene(scene_path)
        .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
    Ok(runtime)
}

#[cfg(feature = "device")]
fn bind_scene(
    mut runtime: Runtime,
) -> Result<(Runtime, GpuFramePresenter<'static>, RollABallController), ProtocolError> {
    let mut presenter = GpuFramePresenter::from_new_renderer()
        .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
    let controller = RollABallController::bind_to_scene(&mut runtime, &mut presenter)
        .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
    Ok((runtime, presenter, controller))
}

#[cfg(not(feature = "device"))]
fn capture_frame(
    _session: &ControlSession,
    _width: u32,
    _height: u32,
) -> Result<Value, ProtocolError> {
    Err(ProtocolError::CaptureRequiresDevice)
}

#[cfg(feature = "device")]
fn capture_frame(
    session: &ControlSession,
    width: u32,
    height: u32,
) -> Result<Value, ProtocolError> {
    let (camera, view, background) =
        session
            .presenter
            .resolved_camera_and_view(&session.runtime, width, height);
    let instances = capture_instances(session);
    let frame = unitas_render::render_offscreen_with_ids(
        session.presenter.renderer(),
        &instances,
        &camera,
        view,
        session.presenter.light(),
        background,
        (width, height),
    )
    .map_err(|error| ProtocolError::CaptureFailed(error.to_string()))?;
    let objects: Vec<Value> = frame.objects.iter().map(capture_object_json).collect();
    Ok(json!({
        "ok": true,
        "width": frame.image.width,
        "height": frame.image.height,
        "image": base64_encode(frame.image.as_bytes()),
        "id_map": frame.id_map,
        "objects": objects,
    }))
}

#[cfg(feature = "device")]
fn capture_instances(session: &ControlSession) -> Vec<CaptureInstance<'_>> {
    let mut instances: Vec<CaptureInstance<'_>> = Vec::new();
    for binding in session.presenter.meshes() {
        if !binding.enabled {
            continue;
        }
        let model = match binding
            .entity
            .and_then(|entity| session.runtime.world_transform_of(entity))
        {
            Some(transform) => transform_to_mat4(&transform),
            None => binding.model,
        };
        let name = binding
            .entity
            .and_then(|entity| {
                session
                    .runtime
                    .scene_graph()
                    .and_then(|graph| graph.entity(entity))
            })
            .map_or("standalone-object", |entity| entity.name.as_str());
        instances.push(CaptureInstance {
            name,
            mesh: &binding.mesh,
            model,
            color: binding.color,
            lit: binding.lit,
        });
    }
    instances
}

#[cfg(feature = "device")]
fn capture_object_json(object: &CaptureObject) -> Value {
    json!({
        "id": object.id,
        "name": object.name,
        "bounds": {
            "x": object.bounds.x,
            "y": object.bounds.y,
            "width": object.bounds.width,
            "height": object.bounds.height,
        },
    })
}

#[cfg(feature = "device")]
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (first << 16) | (second << 8) | third;
        encoded.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        encoded.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

fn parse_request(line: &str) -> Result<Request, ProtocolError> {
    let value: Value = serde_json::from_str(line).map_err(|_| ProtocolError::InvalidRequest)?;
    let object = value.as_object().ok_or(ProtocolError::InvalidRequest)?;
    let command = object
        .get("cmd")
        .and_then(Value::as_str)
        .ok_or(ProtocolError::InvalidRequest)?;
    match command {
        "play" => Ok(Request::Play),
        "pause" => Ok(Request::Pause),
        "stop" => Ok(Request::Stop),
        "state" => Ok(Request::State),
        "step" => {
            let frames = object
                .get("frames")
                .and_then(Value::as_u64)
                .ok_or(ProtocolError::InvalidStep)?;
            Ok(Request::Step { frames })
        }
        "inject_input" => {
            let events = object
                .get("events")
                .and_then(Value::as_array)
                .ok_or(ProtocolError::InvalidInputEvent)?;
            let parsed = events
                .iter()
                .map(parse_input_event)
                .collect::<Result<Vec<InjectedInput>, ProtocolError>>()?;
            Ok(Request::InjectInput { events: parsed })
        }
        "capture" => {
            let width = object
                .get("width")
                .and_then(Value::as_u64)
                .ok_or(ProtocolError::InvalidCapture)?;
            let height = object
                .get("height")
                .and_then(Value::as_u64)
                .ok_or(ProtocolError::InvalidCapture)?;
            let width = u32::try_from(width).map_err(|_| ProtocolError::InvalidCapture)?;
            let height = u32::try_from(height).map_err(|_| ProtocolError::InvalidCapture)?;
            if width == 0 || height == 0 {
                return Err(ProtocolError::InvalidCapture);
            }
            let id_map = object
                .get("id_map")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            Ok(Request::Capture {
                width,
                height,
                id_map,
            })
        }
        _ => Err(ProtocolError::UnknownCommand),
    }
}

fn parse_input_event(value: &Value) -> Result<InjectedInput, ProtocolError> {
    let object = value.as_object().ok_or(ProtocolError::InvalidInputEvent)?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(ProtocolError::InvalidInputEvent)?;
    match kind {
        "key" => Ok(InjectedInput::Key {
            key: parse_u32(object, "key")?,
            pressed: parse_bool(object, "pressed")?,
        }),
        "mouse_button" => Ok(InjectedInput::MouseButton {
            button: parse_u32(object, "button")?,
            pressed: parse_bool(object, "pressed")?,
        }),
        "pointer_move" => Ok(InjectedInput::PointerMove {
            x: parse_f64(object, "x")?,
            y: parse_f64(object, "y")?,
        }),
        "scroll" => Ok(InjectedInput::Scroll {
            x: parse_f64(object, "x")?,
            y: parse_f64(object, "y")?,
        }),
        _ => Err(ProtocolError::InvalidInputEvent),
    }
}

fn parse_u32(object: &Map<String, Value>, key: &str) -> Result<u32, ProtocolError> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(ProtocolError::InvalidInputEvent)
}

fn parse_f64(object: &Map<String, Value>, key: &str) -> Result<f64, ProtocolError> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(ProtocolError::InvalidInputEvent)
}

fn parse_bool(object: &Map<String, Value>, key: &str) -> Result<bool, ProtocolError> {
    object
        .get(key)
        .and_then(Value::as_bool)
        .ok_or(ProtocolError::InvalidInputEvent)
}

pub fn handle_line(session: &mut ControlSession, line: &str) -> String {
    let outcome = match parse_request(line) {
        Ok(request) => session.dispatch(&request),
        Err(error) => Err(error),
    };
    match outcome {
        Ok(payload) => payload.to_string(),
        Err(error) => json!({ "ok": false, "error": error.outcome() }).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unitas_runtime::STATE_DOWN;

    fn fixture_scene() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scene_alpha.unity")
    }

    fn session(random_seed: u64) -> Result<ControlSession, String> {
        ControlSession::start(&fixture_scene(), 1.0 / 60.0, random_seed)
            .map_err(|error| error.outcome())
    }

    #[test]
    fn every_command_form_parses() {
        assert_eq!(parse_request("{\"cmd\":\"play\"}"), Ok(Request::Play));
        assert_eq!(parse_request("{\"cmd\":\"pause\"}"), Ok(Request::Pause));
        assert_eq!(parse_request("{\"cmd\":\"stop\"}"), Ok(Request::Stop));
        assert_eq!(parse_request("{\"cmd\":\"state\"}"), Ok(Request::State));
        assert_eq!(
            parse_request("{\"cmd\":\"step\",\"frames\":3}"),
            Ok(Request::Step { frames: 3 })
        );
        assert_eq!(
            parse_request("{\"cmd\":\"inject_input\",\"events\":[{\"kind\":\"key\",\"key\":97,\"pressed\":true}]}"),
            Ok(Request::InjectInput {
                events: vec![InjectedInput::Key {
                    key: 97,
                    pressed: true
                }]
            })
        );
        assert_eq!(
            parse_request("{\"cmd\":\"capture\",\"width\":16,\"height\":8,\"id_map\":true}"),
            Ok(Request::Capture {
                width: 16,
                height: 8,
                id_map: true
            })
        );
    }

    #[test]
    fn input_events_cover_the_runtime_injection_calls() {
        let request = parse_request(
            "{\"cmd\":\"inject_input\",\"events\":[\
            {\"kind\":\"key\",\"key\":97,\"pressed\":true},\
            {\"kind\":\"mouse_button\",\"button\":0,\"pressed\":false},\
            {\"kind\":\"pointer_move\",\"x\":1.5,\"y\":2.5},\
            {\"kind\":\"scroll\",\"x\":0.0,\"y\":1.0}]}",
        );
        assert_eq!(
            request,
            Ok(Request::InjectInput {
                events: vec![
                    InjectedInput::Key {
                        key: 97,
                        pressed: true
                    },
                    InjectedInput::MouseButton {
                        button: 0,
                        pressed: false
                    },
                    InjectedInput::PointerMove { x: 1.5, y: 2.5 },
                    InjectedInput::Scroll { x: 0.0, y: 1.0 },
                ]
            })
        );
    }

    #[test]
    fn unknown_and_malformed_requests_produce_named_outcomes() {
        assert_eq!(
            parse_request("{\"cmd\":\"rewind\"}"),
            Err(ProtocolError::UnknownCommand)
        );
        assert_eq!(
            ProtocolError::UnknownCommand.outcome(),
            String::from("unknown command")
        );
        assert_eq!(
            parse_request("not json at all"),
            Err(ProtocolError::InvalidRequest)
        );
        assert_eq!(
            parse_request("{\"cmd\":42}"),
            Err(ProtocolError::InvalidRequest)
        );
        assert_eq!(
            parse_request("{\"cmd\":\"step\"}"),
            Err(ProtocolError::InvalidStep)
        );
        assert_eq!(
            parse_request("{\"cmd\":\"inject_input\",\"events\":[{\"kind\":\"key\"}]}"),
            Err(ProtocolError::InvalidInputEvent)
        );
        assert_eq!(
            parse_request("{\"cmd\":\"inject_input\",\"events\":[{\"kind\":\"joystick\"}]}"),
            Err(ProtocolError::InvalidInputEvent)
        );
        assert_eq!(
            parse_request("{\"cmd\":\"capture\",\"width\":0,\"height\":8}"),
            Err(ProtocolError::InvalidCapture)
        );
    }

    #[test]
    fn play_pause_step_and_stop_drive_the_session_state() -> Result<(), String> {
        let mut session = session(7)?;
        assert_eq!(session.state, SessionState::Paused);

        session
            .dispatch(&Request::Play)
            .map_err(|error| error.outcome())?;
        assert_eq!(session.state, SessionState::Playing);

        session
            .dispatch(&Request::Pause)
            .map_err(|error| error.outcome())?;
        assert_eq!(session.state, SessionState::Paused);

        session
            .dispatch(&Request::Step { frames: 4 })
            .map_err(|error| error.outcome())?;
        assert_eq!(session.frames, 4);
        assert_eq!(session.state, SessionState::Paused);

        session
            .dispatch(&Request::Stop)
            .map_err(|error| error.outcome())?;
        assert_eq!(session.state, SessionState::Stopped);
        assert_eq!(session.frames, 4);
        Ok(())
    }

    #[test]
    fn a_stopped_session_refuses_further_work() -> Result<(), String> {
        let mut session = session(9)?;
        session
            .dispatch(&Request::Stop)
            .map_err(|error| error.outcome())?;
        assert_eq!(
            session.dispatch(&Request::Pause),
            Err(ProtocolError::NotPlaying)
        );
        assert_eq!(
            session.dispatch(&Request::Step { frames: 1 }),
            Err(ProtocolError::NotPlaying)
        );
        assert_eq!(
            session.dispatch(&Request::InjectInput {
                events: vec![InjectedInput::Key {
                    key: 97,
                    pressed: true
                }]
            }),
            Err(ProtocolError::NotPlaying)
        );
        assert_eq!(
            ProtocolError::NotPlaying.outcome(),
            String::from("not playing")
        );
        Ok(())
    }

    #[test]
    fn injected_input_reaches_the_session_input_state() -> Result<(), String> {
        let mut session = session(13)?;
        session
            .dispatch(&Request::InjectInput {
                events: vec![InjectedInput::Key {
                    key: 97,
                    pressed: true,
                }],
            })
            .map_err(|error| error.outcome())?;
        session
            .dispatch(&Request::Step { frames: 2 })
            .map_err(|error| error.outcome())?;
        assert_eq!(session.runtime.input_key_state(97) & STATE_DOWN, STATE_DOWN);
        Ok(())
    }

    #[test]
    fn capture_without_an_id_map_is_refused() -> Result<(), String> {
        let mut session = session(17)?;
        assert_eq!(
            session.dispatch(&Request::Capture {
                width: 8,
                height: 8,
                id_map: false
            }),
            Err(ProtocolError::CaptureWithoutIdMap)
        );
        Ok(())
    }

    #[test]
    fn capture_on_a_stopped_session_is_refused() -> Result<(), String> {
        let mut session = session(19)?;
        session
            .dispatch(&Request::Stop)
            .map_err(|error| error.outcome())?;
        assert_eq!(
            session.dispatch(&Request::Capture {
                width: 8,
                height: 8,
                id_map: true
            }),
            Err(ProtocolError::NotPlaying)
        );
        Ok(())
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn capture_without_the_device_feature_is_refused() -> Result<(), String> {
        let mut session = session(23)?;
        let result = session.dispatch(&Request::Capture {
            width: 8,
            height: 8,
            id_map: true,
        });
        assert_eq!(result, Err(ProtocolError::CaptureRequiresDevice));
        assert_eq!(
            ProtocolError::CaptureRequiresDevice.outcome(),
            String::from("capture requires the device feature")
        );
        Ok(())
    }

    #[cfg(feature = "device")]
    #[test]
    fn base64_encoding_pads_the_last_group() {
        assert_eq!(base64_encode(b""), String::new());
        assert_eq!(base64_encode(b"f"), String::from("Zg=="));
        assert_eq!(base64_encode(b"fo"), String::from("Zm8="));
        assert_eq!(base64_encode(b"foo"), String::from("Zm9v"));
        assert_eq!(base64_encode(b"foob"), String::from("Zm9vYg=="));
    }

    #[test]
    fn two_sessions_do_not_share_input_time_or_random_state() -> Result<(), String> {
        let mut first = session(101)?;
        let mut second = session(202)?;

        first
            .dispatch(&Request::InjectInput {
                events: vec![InjectedInput::Key {
                    key: 97,
                    pressed: true,
                }],
            })
            .map_err(|error| error.outcome())?;
        first
            .dispatch(&Request::Step { frames: 3 })
            .map_err(|error| error.outcome())?;
        second
            .dispatch(&Request::Step { frames: 1 })
            .map_err(|error| error.outcome())?;

        assert_eq!(first.frames, 3);
        assert_eq!(second.frames, 1);
        assert_eq!(first.runtime.input_key_state(97) & STATE_DOWN, STATE_DOWN);
        assert_eq!(second.runtime.input_key_state(97) & STATE_DOWN, 0);
        assert_ne!(first.random_seed, second.random_seed);
        Ok(())
    }

    #[test]
    fn each_request_produces_one_json_response_object() -> Result<(), Box<dyn std::error::Error>> {
        let mut session = session(29)?;
        let response = handle_line(&mut session, "{\"cmd\":\"play\"}");
        let payload: Value = serde_json::from_str(&response)?;
        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["state"], json!("playing"));
        assert_eq!(payload["frames"], json!(0));

        let response = handle_line(&mut session, "{\"cmd\":\"rewind\"}");
        let payload: Value = serde_json::from_str(&response)?;
        assert_eq!(payload["ok"], json!(false));
        assert_eq!(payload["error"], json!("unknown command"));
        Ok(())
    }
}
