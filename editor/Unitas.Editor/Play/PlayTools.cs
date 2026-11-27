using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class PlayTools
    {
        public const string PlaySessionOrigin = "unitas://play/session";
        public const string PermissionGrantOrigin = "unitas://permissions/grant";

        public static void Register(McpHost host, PlaySessionManager sessionManager)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionManager);

            host.RegisterTool(
                new ToolDefinition(
                    "play.enter",
                    "Start an isolated play-mode worker session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["bundleDirectory"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["debug"] = new JsonObject
                            {
                                ["type"] = "boolean",
                            },
                        },
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleEnter(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.pause",
                    "Pause an active play-mode worker session",
                    PlaySessionSchema(),
                    Scope.ExecuteProjectCode),
                arguments => HandlePause(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.step",
                    "Advance a play-mode worker session by a deterministic frame count",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["frames"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleStep(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.inject_input",
                    "Inject deterministic input events into a play-mode worker session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["events"] = new JsonObject
                            {
                                ["type"] = "array",
                            },
                        },
                        ["required"] = new JsonArray("events"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleInjectInput(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.set_runtime_property",
                    "Change one property of the open scene while a play session runs; the change is a runtime change and the editor discards it when the play session ends",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["path"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["value"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("fileId", "path", "value"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleSetRuntimeProperty(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.state",
                    "Query current state of a play-mode worker session",
                    PlaySessionSchema(),
                    Scope.ReadProject),
                arguments => HandleState(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "play.exit",
                    "Stop and dispose a play-mode worker session",
                    PlaySessionSchema(),
                    Scope.ExecuteProjectCode),
                arguments => HandleExit(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "permissions.request_grant",
                    "Discover and request a person-only permission scope grant without performing it",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["scope"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["reason"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("scope"),
                    },
                    Scope.ReadProject),
                arguments => HandleRequestGrant(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "permissions.person_grant",
                    "Perform a person-only scope grant using an out-of-band person credential",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["sessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["scope"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("scope", "personCredential"),
                    },
                    Scope.ReadProject),
                arguments => HandlePersonGrant(host, arguments));
        }

        private static ToolOutput HandleEnter(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            string callerSessionId = RequireCallerSession(host);
            string? bundleDirectory = ExtractOptionalString(arguments, "bundleDirectory");
            bool debug = ExtractOptionalBool(arguments, "debug");
            string operationId = host.StartOperation(callerSessionId, "play.session", Scope.ExecuteProjectCode);
            host.Operations.UpdateStage(operationId, "starting");

            PlaySession session;
            try
            {
                session = manager.StartSession(callerSessionId, operationId, bundleDirectory, debug);
            }
            catch (McpError error)
            {
                host.FailOperation(operationId, error.Message);
                throw;
            }

            host.Operations.UpdateStage(operationId, PlaySession.StatePlaying);
            JsonObject payload = session.QueryState();

            // The edit-time state of the open scene is what play mode comes back to.
            JsonObject? editState = SceneTools.TryCaptureEditState();
            if (editState != null)
            {
                payload["editState"] = editState;
            }

            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(operationId);
        }

        private static ToolOutput HandleSetRuntimeProperty(McpHost host, JsonNode arguments)
        {
            long fileId = RequireLong(arguments, "fileId", "play.set_runtime_property");
            string path = RequireText(arguments, "path", "play.set_runtime_property");
            string value = RequireValue(arguments, "value", "play.set_runtime_property");

            JsonObject payload = SceneTools.ApplyRuntimeProperty(fileId, path, value);
            return ToolOutput.Untrusted(PlaySessionOrigin, payload);
        }

        private static ToolOutput HandlePause(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            JsonObject payload = ExecuteSessionAction(host, session, () => session.Pause());
            UpdateOperationIfPending(host, session.OperationId, session.State);
            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(session.OperationId);
        }

        private static ToolOutput HandleStep(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            long frames = ExtractOptionalLong(arguments, "frames", 1L);
            JsonObject payload = ExecuteSessionAction(host, session, () => session.Step(frames));
            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(session.OperationId);
        }

        private static ToolOutput HandleInjectInput(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            JsonArray? events = (arguments as JsonObject)?["events"] as JsonArray;
            if (events == null)
            {
                throw new McpError.InvalidParams("play.inject_input requires an 'events' array");
            }

            JsonObject payload = ExecuteSessionAction(host, session, () => session.InjectInput(events));
            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(session.OperationId);
        }

        private static ToolOutput HandleState(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            JsonObject payload = session.QueryState();
            if (session.Crashed)
            {
                FailOperationIfPending(host, session.OperationId, "worker process exited");
            }

            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(session.OperationId);
        }

        private static ToolOutput HandleExit(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            JsonObject payload = session.Stop();

            // Leaving play mode restores the edit-time state the developer left behind,
            // so the runtime changes of the session are gone.
            JsonObject? editState = SceneTools.TryRestoreEditState();
            if (editState != null)
            {
                payload["editState"] = editState;
            }

            CompleteOperationIfPending(host, session.OperationId, PlaySessionOrigin, payload);
            return ToolOutput.Untrusted(PlaySessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId)
                .WithOperation(session.OperationId);
        }

        private static ToolOutput HandleRequestGrant(McpHost host, JsonNode arguments)
        {
            string callerSessionId = RequireCallerSession(host);
            Scope requestedScope = RequireScopeArgument(arguments);
            string? reason = ExtractOptionalString(arguments, "reason");

            JsonObject payload = new JsonObject
            {
                ["sessionId"] = callerSessionId,
                ["requestedScope"] = Scopes.AsString(requestedScope),
                ["status"] = "requested-person-action",
                ["personOnly"] = true,
                ["granted"] = false,
                ["reason"] = reason ?? "agent requested person-only scope grant",
            };

            return ToolOutput.Untrusted(PermissionGrantOrigin, payload)
                .WithAffectedIdentities(callerSessionId);
        }

        private static ToolOutput HandlePersonGrant(McpHost host, JsonNode arguments)
        {
            string callerSessionId = RequireCallerSession(host);
            SessionIdentity callerIdentity = host.GetSessionIdentity(callerSessionId);
            string targetSessionId = ExtractOptionalString(arguments, "sessionId") ?? callerSessionId;
            Scope scope = RequireScopeArgument(arguments);
            string? token = ExtractOptionalString(arguments, "personCredential");

            host.GrantScopeWithCredential(
                targetSessionId,
                scope,
                token,
                callerIdentity.Client);

            JsonObject payload = new JsonObject
            {
                ["sessionId"] = targetSessionId,
                ["grantedScope"] = Scopes.AsString(scope),
                ["status"] = "granted",
                ["authorizedBy"] = "person",
            };

            return ToolOutput.Untrusted(PermissionGrantOrigin, payload)
                .WithAffectedIdentities(targetSessionId);
        }

        private static PlaySession ResolveSession(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            string? playSessionId = ExtractOptionalString(arguments, "playSessionId");
            return manager.GetSession(playSessionId, host.CurrentSessionId);
        }

        private static JsonObject ExecuteSessionAction(
            McpHost host,
            PlaySession session,
            Func<JsonObject> action)
        {
            try
            {
                return action();
            }
            catch (McpError error)
            {
                if (session.Crashed)
                {
                    FailOperationIfPending(host, session.OperationId, error.Message);
                }

                throw;
            }
        }

        private static void UpdateOperationIfPending(McpHost host, string operationId, string stage)
        {
            OperationHandle handle = host.GetOperation(operationId);
            if (handle.State.IsPending)
            {
                host.Operations.UpdateStage(operationId, stage);
            }
        }

        private static void CompleteOperationIfPending(
            McpHost host,
            string operationId,
            string origin,
            JsonNode result)
        {
            OperationHandle handle = host.GetOperation(operationId);
            if (handle.State.IsPending)
            {
                host.CompleteOperation(operationId, origin, result);
            }
        }

        private static void FailOperationIfPending(McpHost host, string operationId, string error)
        {
            OperationHandle handle = host.GetOperation(operationId);
            if (handle.State.IsPending)
            {
                host.FailOperation(operationId, error);
            }
        }

        private static string RequireCallerSession(McpHost host)
        {
            string? sessionId = host.CurrentSessionId;
            if (string.IsNullOrEmpty(sessionId))
            {
                throw new McpError.InvalidRequest("tool invocation requires an active MCP session");
            }

            return sessionId!;
        }

        private static long RequireLong(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out long number))
                {
                    return number;
                }

                if (value.TryGetValue(out double real)
                    && real == Math.Floor(real)
                    && real >= long.MinValue
                    && real <= long.MaxValue)
                {
                    return (long)real;
                }
            }

            throw new McpError.InvalidParams(toolName + " requires a '" + propertyName + "' argument");
        }

        private static string RequireText(JsonNode arguments, string propertyName, string toolName)
        {
            string text = RequireValue(arguments, propertyName, toolName);
            if (text.Trim().Length == 0)
            {
                throw new McpError.InvalidParams(toolName + " requires a non-empty '" + propertyName + "' argument");
            }

            return text;
        }

        private static string RequireValue(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out string? text))
                {
                    return text;
                }

                if (value.TryGetValue(out double number))
                {
                    return number.ToString(System.Globalization.CultureInfo.InvariantCulture);
                }
            }

            throw new McpError.InvalidParams(toolName + " requires a '" + propertyName + "' argument");
        }

        private static Scope RequireScopeArgument(JsonNode arguments)
        {
            string? rawScope = ExtractOptionalString(arguments, "scope");
            if (string.IsNullOrWhiteSpace(rawScope) || !Scopes.TryParse(rawScope!, out Scope parsed))
            {
                throw new McpError.InvalidParams("valid 'scope' argument is required");
            }

            return parsed;
        }

        private static string? ExtractOptionalString(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            return null;
        }

        private static long ExtractOptionalLong(JsonNode arguments, string propertyName, long defaultValue)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out long longValue))
                {
                    return longValue;
                }

                if (value.TryGetValue(out int intValue))
                {
                    return intValue;
                }
            }

            return defaultValue;
        }

        private static bool ExtractOptionalBool(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out bool parsed))
            {
                return parsed;
            }

            return false;
        }

        private static JsonObject PlaySessionSchema()
        {
            return new JsonObject
            {
                ["type"] = "object",
                ["properties"] = new JsonObject
                {
                    ["playSessionId"] = new JsonObject
                    {
                        ["type"] = "string",
                    },
                },
            };
        }
    }
}
