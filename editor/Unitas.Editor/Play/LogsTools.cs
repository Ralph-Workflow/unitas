using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class LogsTools
    {
        public const string LogsOrigin = "unitas://play/logs";
        public const string StatusOk = "ok";
        public const string StatusNoActivePlaySession = "no-active-play-session";
        public const int DefaultLimit = 100;
        public const int MaxLimit = 1000;

        public static void Register(McpHost host, PlaySessionManager manager)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(manager);

            host.RegisterTool(
                new ToolDefinition(
                    "logs.read",
                    "Read runtime log lines recorded by the active play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["sessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["severity"] = new JsonObject
                            {
                                ["type"] = "string",
                                ["enum"] = new JsonArray("info", "warning", "error"),
                            },
                            ["afterFrame"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["limit"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleRead(host, manager, arguments));
        }

        private static ToolOutput HandleRead(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            string callerSessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest("logs.read requires an active MCP session");
            string? requestedSessionId = ExtractOptionalString(arguments, "sessionId");
            string? severity = ExtractOptionalString(arguments, "severity");
            if (severity != null && severity != "info" && severity != "warning" && severity != "error")
            {
                throw new McpError.InvalidParams("logs.read 'severity' must be info, warning, or error");
            }

            long? afterFrame = TryExtractLong(arguments, "afterFrame");
            int limit = Math.Clamp(ExtractOptionalInt(arguments, "limit", DefaultLimit), 1, MaxLimit);

            // An explicitly named session must exist; without one, no active
            // play session is an empty result, not an error.
            PlaySession? session = !string.IsNullOrWhiteSpace(requestedSessionId)
                ? manager.GetSession(requestedSessionId, callerSessionId)
                : manager.TryGetSession(null, callerSessionId);
            if (session == null || !CanReadLogs(session))
            {
                return ToolOutput.Untrusted(LogsOrigin, EmptyPayload(session?.PlaySessionId ?? requestedSessionId));
            }

            JsonObject response;
            try
            {
                response = session.Worker.SendCommand(BuildLogsCommand(severity, afterFrame, limit));
            }
            catch (McpError) when (session.Worker.HasExited)
            {
                return ToolOutput.Untrusted(LogsOrigin, EmptyPayload(session.PlaySessionId));
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = StatusOk,
                ["playSessionId"] = session.PlaySessionId,
                ["entries"] = response["entries"]?.DeepClone() ?? new JsonArray(),
            };
            return ToolOutput.Untrusted(LogsOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static bool CanReadLogs(PlaySession session)
        {
            return session.State != PlaySession.StateStopped
                && session.State != PlaySession.StateSessionEnded
                && !session.Crashed
                && !session.Worker.HasExited;
        }

        private static JsonObject BuildLogsCommand(string? severity, long? afterFrame, int limit)
        {
            JsonObject command = new JsonObject
            {
                ["cmd"] = "logs",
                ["limit"] = limit,
            };
            if (severity != null)
            {
                command["severity"] = severity;
            }

            if (afterFrame.HasValue)
            {
                command["after_frame"] = afterFrame.Value;
            }

            return command;
        }

        private static JsonObject EmptyPayload(string? playSessionId)
        {
            JsonObject payload = new JsonObject
            {
                ["status"] = StatusNoActivePlaySession,
                ["entries"] = new JsonArray(),
            };
            if (!string.IsNullOrEmpty(playSessionId))
            {
                payload["playSessionId"] = playSessionId;
            }

            return payload;
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

        private static int ExtractOptionalInt(JsonNode arguments, string propertyName, int defaultValue)
        {
            long? extracted = TryExtractLong(arguments, propertyName);
            return extracted.HasValue ? (int)Math.Clamp(extracted.Value, 1L, int.MaxValue) : defaultValue;
        }

        private static long? TryExtractLong(JsonNode arguments, string propertyName)
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

            return null;
        }
    }
}
