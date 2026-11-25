use crate::error::McpError;
use crate::operations::OperationId;
use crate::registry::{McpHost, ResourceContent, ResourceDefinition, ToolDefinition, ToolOutput};
use crate::scope::Scope;
use crate::session::{SessionId, SessionIdentity};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2024-11-05";

pub struct StdioServer {
    host: McpHost,
    active_session: Option<SessionId>,
}

impl StdioServer {
    pub fn new(host: McpHost) -> Self {
        Self {
            host,
            active_session: None,
        }
    }

    pub fn with_defaults() -> Result<Self, McpError> {
        let mut host = McpHost::new();
        register_builtin_defaults(&mut host)?;
        Ok(Self::new(host))
    }

    pub fn host(&self) -> &McpHost {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut McpHost {
        &mut self.host
    }

    pub fn active_session(&self) -> Option<&SessionId> {
        self.active_session.as_ref()
    }

    fn ensure_session(&mut self, params: &Value) -> SessionId {
        if let Some(requested_id) = params.get("sessionId").and_then(Value::as_str) {
            let candidate = SessionId::new(requested_id);
            if self.host.session(&candidate).is_ok() {
                return candidate;
            }
        }
        if let Some(existing) = &self.active_session {
            return existing.clone();
        }
        let client_name = params
            .get("clientInfo")
            .and_then(|info| info.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("mcp-stdio-client");
        let agent_name = params
            .get("agent")
            .and_then(Value::as_str)
            .unwrap_or(client_name);
        let session = self
            .host
            .create_session(SessionIdentity::new(agent_name, "stdio", "stdio-local"));
        self.active_session = Some(session.clone());
        session
    }

    pub fn handle_line(&mut self, line: &str) -> Result<Option<String>, McpError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(error) => {
                let response = json_rpc_error(&Value::Null, -32700, &error.to_string(), None);
                return Ok(Some(response.to_string()));
            }
        };
        let response = self.handle_message(&parsed);
        Ok(response.map(|value| value.to_string()))
    }

    pub fn handle_message(&mut self, request: &Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let jsonrpc = request.get("jsonrpc").and_then(Value::as_str);
        if jsonrpc != Some("2.0") {
            let response_id = id.unwrap_or(Value::Null);
            return Some(json_rpc_error(
                &response_id,
                -32600,
                "jsonrpc field must be \"2.0\"",
                None,
            ));
        }
        let method = match request.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => {
                let response_id = id.unwrap_or(Value::Null);
                return Some(json_rpc_error(
                    &response_id,
                    -32600,
                    "missing method field",
                    None,
                ));
            }
        };
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let is_notification = id.is_none();
        if is_notification && method == "notifications/initialized" {
            return None;
        }
        let response_id = id.unwrap_or(Value::Null);
        match self.dispatch(method, &params) {
            Ok(result) => {
                if is_notification {
                    None
                } else {
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": response_id,
                        "result": result,
                    }))
                }
            }
            Err(error) => Some(map_error_to_json_rpc(&response_id, &error)),
        }
    }

    fn dispatch(&mut self, method: &str, params: &Value) -> Result<Value, McpError> {
        match method {
            "initialize" => {
                let session = self.ensure_session(params);
                let granted: Vec<&str> = self
                    .host
                    .session(&session)?
                    .scopes()
                    .map(Scope::as_str)
                    .collect();
                let available_scopes: Vec<&str> =
                    Scope::ALL.iter().copied().map(Scope::as_str).collect();
                Ok(json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "serverInfo": {
                        "name": "unitas-mcp",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "operations": {},
                        "audit": { "appendOnly": true },
                    },
                    "sessionId": session.as_str(),
                    "grantedScopes": granted,
                    "availableScopes": available_scopes,
                }))
            }
            "tools/list" => {
                let _ = self.ensure_session(params);
                let tools: Vec<Value> = self
                    .host
                    .list_tools()
                    .into_iter()
                    .map(ToolDefinition::to_value)
                    .collect();
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let session = self.ensure_session(params);
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| McpError::InvalidParams("missing tool name".to_string()))?;
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let envelope = self.host.call_tool(&session, name, &arguments)?;
                let text = envelope.to_string();
                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": text,
                            "untrusted": true,
                        }
                    ],
                    "structuredContent": envelope,
                    "isError": false,
                }))
            }
            "resources/list" => {
                let _ = self.ensure_session(params);
                let resources: Vec<Value> = self
                    .host
                    .list_resources()
                    .into_iter()
                    .map(ResourceDefinition::to_value)
                    .collect();
                Ok(json!({ "resources": resources }))
            }
            "resources/read" => {
                let session = self.ensure_session(params);
                let uri = params
                    .get("uri")
                    .and_then(Value::as_str)
                    .ok_or_else(|| McpError::InvalidParams("missing resource uri".to_string()))?;
                let definition = self.host.list_resources().into_iter().find(|r| r.uri() == uri).cloned();
                let mime_type = definition
                    .as_ref()
                    .map(|def| def.mime_type().to_string())
                    .unwrap_or_else(|| "application/json".to_string());
                let envelope = self.host.read_resource(&session, uri)?;
                let text = envelope.to_string();
                Ok(json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": mime_type,
                            "text": text,
                            "untrusted": true,
                            "structuredContent": envelope,
                        }
                    ]
                }))
            }
            "operations/get" => {
                let _ = self.ensure_session(params);
                let id_str = params
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| McpError::InvalidParams("missing operation id".to_string()))?;
                let op = self.host.operation(&OperationId::new(id_str))?;
                Ok(op.to_value())
            }
            "audit/list" => {
                let _ = self.ensure_session(params);
                Ok(json!({
                    "records": self.host.audit_log().export_json(),
                }))
            }
            "permissions/grant" | "scopes/grant" => {
                let session = self.ensure_session(params);
                let requested = params
                    .get("scope")
                    .and_then(Value::as_str)
                    .and_then(Scope::from_name)
                    .unwrap_or(Scope::ReadProject);
                self.host.grant_scope(
                    &session,
                    requested,
                    crate::session::GrantAuthority::SessionInherited,
                )?;
                Ok(json!({}))
            }
            other => Err(McpError::UnknownMethod(other.to_string())),
        }
    }

    pub fn serve<R: BufRead, W: Write>(
        &mut self,
        reader: R,
        mut writer: W,
    ) -> Result<(), McpError> {
        for line_result in reader.lines() {
            let line = line_result.map_err(McpError::Io)?;
            if let Some(response) = self.handle_line(&line)? {
                writer
                    .write_all(response.as_bytes())
                    .map_err(McpError::Io)?;
                writer.write_all(b"\n").map_err(McpError::Io)?;
                writer.flush().map_err(McpError::Io)?;
            }
        }
        Ok(())
    }
}

pub fn register_builtin_defaults(host: &mut McpHost) -> Result<(), McpError> {
    host.register_tool(
        ToolDefinition::new(
            "project.status",
            "Read current unitas project status summary",
            json!({
                "type": "object",
                "properties": {},
            }),
            Scope::ReadProject,
        ),
        |_args| {
            Ok(ToolOutput::untrusted(
                "unitas://project/status",
                json!({
                    "state": "ready",
                }),
            ))
        },
    )?;
    host.register_resource(
        ResourceDefinition::new(
            "unitas://knowledge/scopes",
            "AUT-007 Permission Scopes",
            "Fixed permission scope catalog for unitas MCP sessions",
            "application/json",
            Scope::ReadProject,
        ),
        || {
            let scopes: Vec<&str> = Scope::ALL.iter().copied().map(Scope::as_str).collect();
            Ok(ResourceContent::untrusted(
                "unitas://knowledge/scopes",
                json!({ "scopes": scopes }),
            ))
        },
    )?;
    Ok(())
}

fn json_rpc_error(id: &Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error_obj = json!({
        "code": code,
        "message": message,
    });
    if let Some(extra) = data {
        if let Some(map) = error_obj.as_object_mut() {
            map.insert("data".to_string(), extra);
        }
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error_obj,
    })
}

fn map_error_to_json_rpc(id: &Value, error: &McpError) -> Value {
    match error {
        McpError::MissingScope { scope, session } => json_rpc_error(
            id,
            -32001,
            &error.to_string(),
            Some(json!({
                "kind": "missing-permission-scope",
                "missingScope": scope.as_str(),
                "sessionId": session.as_str(),
            })),
        ),
        McpError::SelfWidening { scope, source } => json_rpc_error(
            id,
            -32002,
            &error.to_string(),
            Some(json!({
                "kind": "self-widening-refused",
                "requestedScope": scope.as_str(),
                "source": source,
            })),
        ),
        McpError::UnknownMethod(_) => json_rpc_error(id, -32601, &error.to_string(), None),
        McpError::InvalidParams(_) => json_rpc_error(id, -32602, &error.to_string(), None),
        McpError::InvalidRequest(_) => json_rpc_error(id, -32600, &error.to_string(), None),
        McpError::ParseError(_) => json_rpc_error(id, -32700, &error.to_string(), None),
        _ => json_rpc_error(id, -32000, &error.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::StdioServer;
    use crate::clock::FixedClock;
    use crate::registry::{McpHost, ResourceContent, ResourceDefinition, ToolDefinition, ToolOutput};
    use crate::scope::Scope;
    use crate::session::GrantAuthority;
    use crate::untrusted::is_untrusted_envelope;
    use serde_json::{json, Value};
    use std::io::Cursor;

    fn test_server() -> StdioServer {
        let mut host = McpHost::with_clock(FixedClock::new(1_700_000_000_000));
        let _ = host.register_tool(
            ToolDefinition::new(
                "scene.inspect",
                "Inspect active scene",
                json!({"type": "object"}),
                Scope::ReadProject,
            ),
            |_args| {
                Ok(ToolOutput::untrusted(
                    "scene:Sample",
                    json!({"rootObjects": ["MainCamera", "Ball"]}),
                ))
            },
        );
        let _ = host.register_tool(
            ToolDefinition::new(
                "scene.edit",
                "Edit scene object",
                json!({"type": "object"}),
                Scope::EditContent,
            ),
            |args| {
                Ok(ToolOutput::untrusted(
                    "scene:Sample",
                    json!({"applied": args}),
                ))
            },
        );
        let _ = host.register_resource(
            ResourceDefinition::new(
                "unitas://project/manifest",
                "Project Manifest",
                "Manifest metadata",
                "application/json",
                Scope::ReadProject,
            ),
            || {
                Ok(ResourceContent::untrusted(
                    "project:manifest",
                    json!({"name": "RollABall"}),
                ))
            },
        );
        StdioServer::new(host)
    }

    #[test]
    fn stdio_initialize_tools_and_resources_round_trip() {
        let mut server = test_server();
        let input = [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "test-client", "version": "1.0" }
                }
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "scene.inspect",
                    "arguments": {}
                }
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "scene.edit",
                    "arguments": { "name": "Ball" }
                }
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "resources/list",
                "params": {}
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "resources/read",
                "params": {
                    "uri": "unitas://project/manifest"
                }
            })
            .to_string(),
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "permissions/grant",
                "params": {
                    "scope": "edit-content"
                }
            })
            .to_string(),
        ]
        .join("\n");

        let mut output = Vec::new();
        assert!(server.serve(Cursor::new(input), &mut output).is_ok());
        let text = String::from_utf8(output).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 7);

        let init_resp: Value = serde_json::from_str(lines[0]).unwrap_or_default();
        assert_eq!(
            init_resp
                .get("result")
                .and_then(|r| r.get("grantedScopes")),
            Some(&json!(["read-project"]))
        );

        let tools_resp: Value = serde_json::from_str(lines[1]).unwrap_or_default();
        let tools_len = tools_resp
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        assert_eq!(tools_len, 2);

        let call_ok: Value = serde_json::from_str(lines[2]).unwrap_or_default();
        let structured = call_ok
            .get("result")
            .and_then(|r| r.get("structuredContent"))
            .cloned()
            .unwrap_or_default();
        assert!(is_untrusted_envelope(&structured));

        let call_refused: Value = serde_json::from_str(lines[3]).unwrap_or_default();
        let err_msg = call_refused
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(err_msg.contains("edit-content"));

        let res_list: Value = serde_json::from_str(lines[4]).unwrap_or_default();
        let res_len = res_list
            .get("result")
            .and_then(|r| r.get("resources"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        assert_eq!(res_len, 1);

        let res_read: Value = serde_json::from_str(lines[5]).unwrap_or_default();
        let res_structured = res_read
            .get("result")
            .and_then(|r| r.get("contents"))
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("structuredContent"))
            .cloned()
            .unwrap_or_default();
        assert!(is_untrusted_envelope(&res_structured));

        let self_widen: Value = serde_json::from_str(lines[6]).unwrap_or_default();
        let widen_kind = self_widen
            .get("error")
            .and_then(|e| e.get("data"))
            .and_then(|d| d.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(widen_kind, "self-widening-refused");

        let session_id = server
            .active_session()
            .cloned()
            .unwrap_or_else(|| crate::session::SessionId::new("none"));
        assert!(server
            .host_mut()
            .grant_scope(&session_id, Scope::EditContent, GrantAuthority::Person)
            .is_ok());
        let retry = server.handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "scene.edit",
                "arguments": { "name": "Ball" }
            }
        }));
        assert!(retry.is_some_and(|resp| resp.get("result").is_some()));
        assert_eq!(server.host().audit_log().len(), 6);
    }
}
