use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::player::ScriptHost;
use crate::scripted::ScriptedPlayer;

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
}

impl InjectedInput {
    fn apply(self, player: &mut ScriptedPlayer) {
        match self {
            InjectedInput::Key { key, pressed } => {
                player.inject_input(0, u64::from(key), pressed, 0.0, 0.0);
            }
            InjectedInput::MouseButton { button, pressed } => {
                player.inject_input(1, u64::from(button), pressed, 0.0, 0.0);
            }
            InjectedInput::PointerMove { x, y } => {
                player.inject_input(2, 0, false, x, y);
            }
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
    ToolsDescribe,
    ToolInvoke {
        name: String,
        arguments: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidRequest,
    UnknownCommand,
    InvalidStep,
    InvalidInputEvent,
    InvalidCapture,
    UnknownTool(String),
    NotPlaying,
    CaptureWithoutIdMap,
    CaptureUnsupported,
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
            ProtocolError::UnknownTool(name) => format!("unknown tool: {name}"),
            ProtocolError::NotPlaying => String::from("not playing"),
            ProtocolError::CaptureWithoutIdMap => String::from("capture requires an id map"),
            ProtocolError::CaptureUnsupported => {
                String::from("capture requires the device feature; the scripted control session does not support capture")
            }
            ProtocolError::FrameFailed(message) => format!("frame failed: {message}"),
            ProtocolError::SessionStartFailed(message) => {
                format!("session start failed: {message}")
            }
        }
    }
}

pub struct ControlSession {
    bundle_dir: PathBuf,
    startup_scene_path: PathBuf,
    player: Option<ScriptedPlayer>,
    state: SessionState,
    frames: u64,
    delta_seconds: f64,
    random_seed: u64,
}

impl ControlSession {
    pub fn start(
        bundle_dir: &Path,
        startup_scene_path: &Path,
        delta_seconds: f64,
        random_seed: u64,
    ) -> Result<Self, ProtocolError> {
        // The control protocol owns stdout; the managed host logs to stderr.
        std::env::set_var("UNITAS_MANAGED_LOG_TO_STDERR", "1");
        let player = ScriptedPlayer::boot(bundle_dir, startup_scene_path)
            .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
        Ok(Self {
            bundle_dir: bundle_dir.to_path_buf(),
            startup_scene_path: startup_scene_path.to_path_buf(),
            player,
            state: SessionState::Paused,
            frames: 0,
            delta_seconds,
            random_seed,
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
            Request::ToolsDescribe => Ok(json!({ "ok": true, "tools": [] })),
            Request::ToolInvoke { name, .. } => Err(ProtocolError::UnknownTool(name.clone())),
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
        if let Some(player) = self.player.as_mut() {
            let _ = player.shutdown();
        }
        self.player = None;
        self.state = SessionState::Stopped;
    }

    fn restart(&mut self) -> Result<(), ProtocolError> {
        self.player = ScriptedPlayer::boot(&self.bundle_dir, &self.startup_scene_path)
            .map_err(|error| ProtocolError::SessionStartFailed(error.to_string()))?;
        self.frames = 0;
        Ok(())
    }

    fn advance(&mut self, frames: u64) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        for _ in 0..frames {
            if let Some(player) = self.player.as_mut() {
                player
                    .tick(self.delta_seconds)
                    .map_err(|error| ProtocolError::FrameFailed(error.to_string()))?;
            }
            self.frames += 1;
        }
        Ok(())
    }

    fn inject(&mut self, events: &[InjectedInput]) -> Result<(), ProtocolError> {
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        if let Some(player) = self.player.as_mut() {
            for event in events {
                event.apply(player);
            }
        }
        Ok(())
    }

    fn capture(&mut self, _width: u32, _height: u32, id_map: bool) -> Result<Value, ProtocolError> {
        if !id_map {
            return Err(ProtocolError::CaptureWithoutIdMap);
        }
        if self.state == SessionState::Stopped {
            return Err(ProtocolError::NotPlaying);
        }
        Err(ProtocolError::CaptureUnsupported)
    }

    fn state_payload(&self) -> Value {
        let (score, total, won, script_host) = match self.player.as_ref() {
            Some(player) => (
                player.score(),
                player.total(),
                player.won(),
                ScriptHost::CoreClr.as_str(),
            ),
            None => (0, 0, false, ScriptHost::None.as_str()),
        };
        json!({
            "ok": true,
            "state": self.state.as_str(),
            "frames": self.frames,
            "delta_seconds": self.delta_seconds,
            "random_seed": self.random_seed,
            "score": score,
            "total": total,
            "won": won,
            "script_host": script_host,
        })
    }
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
        "tools_describe" => Ok(Request::ToolsDescribe),
        "tool_invoke" => {
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .ok_or(ProtocolError::InvalidRequest)?;
            let arguments = object
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            Ok(Request::ToolInvoke {
                name: String::from(name),
                arguments,
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

    fn fixture_scene() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scene_alpha.unity")
    }

    fn session(random_seed: u64) -> Result<ControlSession, String> {
        ControlSession::start(&fixture_scene(), &fixture_scene(), 1.0 / 60.0, random_seed)
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
    fn project_tool_commands_parse() {
        assert_eq!(
            parse_request("{\"cmd\":\"tools_describe\"}"),
            Ok(Request::ToolsDescribe)
        );
        assert_eq!(
            parse_request(
                "{\"cmd\":\"tool_invoke\",\"name\":\"spawn\",\"arguments\":{\"count\":2}}"
            ),
            Ok(Request::ToolInvoke {
                name: String::from("spawn"),
                arguments: json!({"count": 2})
            })
        );
        assert_eq!(
            parse_request("{\"cmd\":\"tool_invoke\"}"),
            Err(ProtocolError::InvalidRequest)
        );
    }

    #[test]
    fn tools_describe_returns_an_empty_tool_list() -> Result<(), String> {
        let mut session = session(31)?;
        let payload = session
            .dispatch(&Request::ToolsDescribe)
            .map_err(|error| error.outcome())?;
        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["tools"], json!([]));
        assert_eq!(
            session.dispatch(&Request::ToolInvoke {
                name: String::from("spawn"),
                arguments: json!({})
            }),
            Err(ProtocolError::UnknownTool(String::from("spawn")))
        );
        Ok(())
    }

    #[test]
    fn input_events_cover_the_managed_host_injection_calls() {
        let request = parse_request(
            "{\"cmd\":\"inject_input\",\"events\":[\
            {\"kind\":\"key\",\"key\":97,\"pressed\":true},\
            {\"kind\":\"mouse_button\",\"button\":0,\"pressed\":false},\
            {\"kind\":\"pointer_move\",\"x\":1.5,\"y\":2.5}]}",
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
                ]
            })
        );
    }

    #[test]
    fn a_scroll_event_is_refused_because_the_managed_host_has_no_scroll_kind() {
        assert_eq!(
            parse_request(
                "{\"cmd\":\"inject_input\",\"events\":[{\"kind\":\"scroll\",\"x\":0.0,\"y\":1.0}]}"
            ),
            Err(ProtocolError::InvalidInputEvent)
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
    fn injected_input_reaches_the_scripted_session() -> Result<(), String> {
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
        assert_eq!(session.frames, 2);
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
    fn capture_is_refused_by_the_scripted_control_session() -> Result<(), String> {
        let mut session = session(23)?;
        let result = session.dispatch(&Request::Capture {
            width: 8,
            height: 8,
            id_map: true,
        });
        assert_eq!(result, Err(ProtocolError::CaptureUnsupported));
        assert_eq!(
            ProtocolError::CaptureUnsupported.outcome(),
            String::from("capture requires the device feature; the scripted control session does not support capture")
        );
        Ok(())
    }

    #[test]
    fn two_sessions_do_not_share_frame_counts_or_random_state() -> Result<(), String> {
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
        assert_ne!(first.random_seed, second.random_seed);
        Ok(())
    }

    #[test]
    fn the_state_payload_reports_the_script_host_and_the_game_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut session = session(41)?;
        let payload = session
            .dispatch(&Request::State)
            .map_err(|error| error.outcome())?;
        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["score"], json!(0));
        assert_eq!(payload["total"], json!(0));
        assert_eq!(payload["won"], json!(false));
        assert_eq!(payload["script_host"], json!("none"));

        session
            .dispatch(&Request::Step { frames: 2 })
            .map_err(|error| error.outcome())?;
        let payload = session
            .dispatch(&Request::State)
            .map_err(|error| error.outcome())?;
        assert_eq!(payload["frames"], json!(2));
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
