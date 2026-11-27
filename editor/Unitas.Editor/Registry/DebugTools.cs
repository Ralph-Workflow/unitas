using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using Unitas.Debugging;

namespace Unitas.Editor.Mcp
{
    /// <summary>
    /// The debug adapter commands of a play session. The tools drive the debug
    /// adapter client of the caller's play session; the session itself owns the
    /// connection, so a breakpoint stop never blocks the command channel.
    /// </summary>
    public static class DebugTools
    {
        public const string DebugSessionOrigin = "unitas://debug/session";

        private const int DefaultTimeoutMilliseconds = 5000;
        private const int MaximumTimeoutMilliseconds = 120000;

        public static void Register(McpHost host, PlaySessionManager sessionManager)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionManager);

            host.RegisterTool(
                new ToolDefinition(
                    "debug.set_breakpoints",
                    "Set breakpoints in the scripts of a debugged play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["path"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["lines"] = new JsonObject
                            {
                                ["type"] = "array",
                            },
                            ["conditions"] = new JsonObject
                            {
                                ["type"] = "array",
                            },
                        },
                        ["required"] = new JsonArray("path", "lines"),
                    },
                    Scope.ReadProject),
                arguments => HandleSetBreakpoints(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.wait_stopped",
                    "Wait for the debugged play session to stop at a breakpoint",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["timeoutMs"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleWaitStopped(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.stack",
                    "Read the stack frames of the stopped debugged play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleStack(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.locals",
                    "Read the local variables of the stopped debugged play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleLocals(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.continue",
                    "Resume the stopped debugged play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleContinue(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.step",
                    "Step the stopped debugged play session over, in or out",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["mode"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleStep(host, sessionManager, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "debug.evaluate",
                    "Evaluate an expression in the stopped debugged play session",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["expression"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("expression"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleEvaluate(host, sessionManager, arguments));
        }

        private static ToolOutput HandleSetBreakpoints(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            string path = RequireText(arguments, "path", "debug.set_breakpoints");
            int[] lines = RequireIntArray(arguments, "lines", "debug.set_breakpoints");
            string[]? conditions = ExtractOptionalStringArray(arguments, "conditions");

            JsonObject? response = session.SetBreakpoints(path, lines, conditions);
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["path"] = path,
                ["applied"] = response != null,
                ["pending"] = response == null,
                ["breakpoints"] = BuildBreakpoints(lines, response),
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleWaitStopped(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            int timeoutMs = ExtractTimeoutMilliseconds(arguments);
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["timeoutMs"] = timeoutMs,
            };

            DapClient client;
            try
            {
                client = session.EnsureDebugClient(TimeSpan.FromMilliseconds(timeoutMs));
            }
            catch (TimeoutException)
            {
                payload["stopped"] = false;
                payload["timedOut"] = true;
                payload["message"] =
                    "the play session did not open a debug adapter within " + timeoutMs + " ms";
                return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                    .WithAffectedIdentities(session.PlaySessionId);
            }

            JsonObject? stopped = client.WaitForStopped(TimeSpan.FromMilliseconds(timeoutMs));
            payload["stopped"] = stopped != null;
            payload["timedOut"] = stopped == null;
            if (stopped != null && stopped["body"] is JsonObject body)
            {
                payload["reason"] = body["reason"]?.DeepClone();
                payload["threadId"] = body["threadId"]?.DeepClone();
                payload["description"] = body["description"]?.DeepClone();
            }

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleStack(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            DapClient client = session.EnsureDebugClient(DefaultTimeout());
            JsonObject response = RequireSuccess(client.StackTrace(), "debug.stack");
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["frames"] = response["body"]?["stackFrames"]?.DeepClone(),
                ["totalFrames"] = response["body"]?["totalFrames"]?.DeepClone(),
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleLocals(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            DapClient client = session.EnsureDebugClient(DefaultTimeout());
            JsonObject scopes = RequireSuccess(client.Scopes(), "debug.locals");
            JsonObject variables = RequireSuccess(
                client.Variables(FirstScopeReference(scopes)),
                "debug.locals");
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["locals"] = variables["body"]?["variables"]?.DeepClone(),
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleContinue(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            DapClient client = session.EnsureDebugClient(DefaultTimeout());
            RequireSuccess(client.Continue(), "debug.continue");
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["continued"] = true,
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleStep(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            DapClient client = session.EnsureDebugClient(DefaultTimeout());
            string mode = ExtractOptionalString(arguments, "mode") ?? "over";
            JsonObject response;
            switch (mode)
            {
                case "over":
                    response = client.Next();
                    break;
                case "in":
                    response = client.StepIn();
                    break;
                case "out":
                    response = client.StepOut();
                    break;
                default:
                    throw new McpError.InvalidParams("debug.step mode must be one of over, in or out");
            }

            RequireSuccess(response, "debug.step");
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["stepped"] = true,
                ["mode"] = mode,
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static ToolOutput HandleEvaluate(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            PlaySession session = ResolveSession(host, manager, arguments);
            DapClient client = session.EnsureDebugClient(DefaultTimeout());
            string expression = RequireText(arguments, "expression", "debug.evaluate");
            JsonObject response = RequireSuccess(client.Evaluate(expression), "debug.evaluate");
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = session.PlaySessionId,
                ["expression"] = expression,
                ["result"] = response["body"]?["result"]?.DeepClone(),
            };

            return ToolOutput.Untrusted(DebugSessionOrigin, payload)
                .WithAffectedIdentities(session.PlaySessionId);
        }

        private static JsonArray BuildBreakpoints(int[] lines, JsonObject? response)
        {
            JsonArray entries = new JsonArray();
            if (response != null && response["body"]?["breakpoints"] is JsonArray verified)
            {
                foreach (JsonNode? node in verified)
                {
                    entries.Add(node?.DeepClone());
                }

                return entries;
            }

            foreach (int line in lines)
            {
                entries.Add(new JsonObject
                {
                    ["line"] = line,
                    ["verified"] = false,
                    ["pending"] = true,
                });
            }

            return entries;
        }

        private static int FirstScopeReference(JsonObject scopes)
        {
            if (scopes["body"]?["scopes"] is JsonArray list)
            {
                foreach (JsonNode? scope in list)
                {
                    if (scope?["variablesReference"] is JsonValue value && value.TryGetValue(out int reference))
                    {
                        return reference;
                    }
                }
            }

            return 0;
        }

        private static JsonObject RequireSuccess(JsonObject response, string toolName)
        {
            bool success = true;
            if (response["success"] is JsonValue value && value.TryGetValue(out bool parsed))
            {
                success = parsed;
            }

            if (!success)
            {
                string message = "the debug adapter refused the request";
                if (response["message"] is JsonValue messageValue
                    && messageValue.TryGetValue(out string? text)
                    && !string.IsNullOrEmpty(text))
                {
                    message = text!;
                }

                throw new McpError.InvalidRequest(toolName + ": " + message);
            }

            return response;
        }

        private static PlaySession ResolveSession(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            string? playSessionId = ExtractOptionalString(arguments, "playSessionId");
            return manager.GetSession(playSessionId, host.CurrentSessionId);
        }

        private static TimeSpan DefaultTimeout()
        {
            return TimeSpan.FromMilliseconds(DefaultTimeoutMilliseconds);
        }

        private static int ExtractTimeoutMilliseconds(JsonNode arguments)
        {
            int timeout = DefaultTimeoutMilliseconds;
            if (arguments is JsonObject obj && obj["timeoutMs"] is JsonValue value)
            {
                if (value.TryGetValue(out int parsed))
                {
                    timeout = parsed;
                }
                else if (value.TryGetValue(out double real) && real == Math.Floor(real))
                {
                    timeout = (int)real;
                }
            }

            if (timeout < 0)
            {
                throw new McpError.InvalidParams("timeoutMs must not be negative");
            }

            return timeout > MaximumTimeoutMilliseconds ? MaximumTimeoutMilliseconds : timeout;
        }

        private static string RequireText(JsonNode arguments, string propertyName, string toolName)
        {
            string? text = ExtractOptionalString(arguments, propertyName);
            if (text == null)
            {
                throw new McpError.InvalidParams(toolName + " requires a '" + propertyName + "' argument");
            }

            return text;
        }

        private static int[] RequireIntArray(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonArray array)
            {
                List<int> values = new List<int>();
                foreach (JsonNode? node in array)
                {
                    if (node is JsonValue value)
                    {
                        if (value.TryGetValue(out int number))
                        {
                            values.Add(number);
                            continue;
                        }

                        if (value.TryGetValue(out double real) && real == Math.Floor(real))
                        {
                            values.Add((int)real);
                            continue;
                        }
                    }

                    throw new McpError.InvalidParams(
                        toolName + " requires '" + propertyName + "' to hold line numbers");
                }

                if (values.Count == 0)
                {
                    throw new McpError.InvalidParams(
                        toolName + " requires at least one entry in '" + propertyName + "'");
                }

                return values.ToArray();
            }

            throw new McpError.InvalidParams(toolName + " requires a '" + propertyName + "' array");
        }

        private static string[]? ExtractOptionalStringArray(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonArray array)
            {
                List<string> values = new List<string>();
                foreach (JsonNode? node in array)
                {
                    if (node is JsonValue value && value.TryGetValue(out string? text))
                    {
                        values.Add(text!);
                    }
                    else
                    {
                        values.Add(string.Empty);
                    }
                }

                return values.ToArray();
            }

            return null;
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
    }
}
