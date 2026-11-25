using System;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;

namespace Unitas.Debugging
{
    public sealed class DapServer
    {
        private const int MainThreadId = 1;
        private const int LocalsReference = 1;

        /// <summary>The adapter attaches to a play session the play surface started; it never starts one itself.</summary>
        private const string LaunchMessage =
            "unitas attaches to a play session that the play surface started; "
                + "this debug adapter does not start a play session.";

        private readonly Stream _transport;
        private readonly object _writeGate = new object();
        private int _sequence;
        private Task? _reader;
        private volatile bool _running;

        public DapServer(Stream transport)
        {
            _transport = transport ?? throw new ArgumentNullException(nameof(transport));
        }

        public Task Completion => _reader ?? Task.CompletedTask;

        public void Start()
        {
            if (_reader != null)
            {
                return;
            }
            _running = true;
            _reader = Task.Run(ReadLoopAsync);
        }

        public void Stop()
        {
            _running = false;
            try
            {
                _transport.Dispose();
            }
            catch (ObjectDisposedException)
            {
            }
            catch (IOException)
            {
            }
        }

        public void NotifyStopped(DebugFrame frame)
        {
            NotifyStopped(frame, "breakpoint");
        }

        public void NotifyStopped(DebugFrame frame, string reason)
        {
            ArgumentNullException.ThrowIfNull(frame);
            JsonObject body = new JsonObject
            {
                ["reason"] = string.IsNullOrEmpty(reason) ? "breakpoint" : reason,
                ["threadId"] = MainThreadId,
                ["allThreadsStopped"] = true,
                ["description"] = frame.Method
            };
            SendEvent("stopped", body);
        }

        private async Task ReadLoopAsync()
        {
            try
            {
                while (_running)
                {
                    string? message = await ReadMessageAsync(_transport).ConfigureAwait(false);
                    if (message == null)
                    {
                        break;
                    }
                    JsonNode? node;
                    try
                    {
                        node = JsonNode.Parse(message);
                    }
                    catch (JsonException)
                    {
                        continue;
                    }
                    if (node is JsonObject request)
                    {
                        Handle(request);
                    }
                }
            }
            catch (IOException)
            {
            }
            catch (ObjectDisposedException)
            {
            }
            catch (InvalidOperationException)
            {
            }
            finally
            {
                _running = false;
            }
        }

        private void Handle(JsonObject request)
        {
            string command = ReadString(request["command"]);
            switch (command)
            {
                case "initialize":
                    SendResponse(request, true, new JsonObject
                    {
                        ["supportsConfigurationDoneRequest"] = true,
                        ["supportsConditionalBreakpoints"] = true,
                        ["supportsEvaluateForHovers"] = true,
                        ["supportsTerminateRequest"] = false
                    });
                    break;
                case "launch":
                    DebugAgent.MarkLaunched();
                    SendResponse(request, true, new JsonObject
                    {
                        ["message"] = LaunchMessage
                    });
                    break;
                case "attach":
                    DebugAgent.MarkAttached();
                    SendResponse(request, true, new JsonObject());
                    break;
                case "setBreakpoints":
                    HandleSetBreakpoints(request);
                    break;
                case "configurationDone":
                    SendResponse(request, true, new JsonObject());
                    break;
                case "threads":
                    SendResponse(request, true, new JsonObject
                    {
                        ["threads"] = new JsonArray
                        {
                            new JsonObject
                            {
                                ["id"] = MainThreadId,
                                ["name"] = "Main Thread"
                            }
                        }
                    });
                    break;
                case "stackTrace":
                    HandleStackTrace(request);
                    break;
                case "scopes":
                    HandleScopes(request);
                    break;
                case "variables":
                    HandleVariables(request);
                    break;
                case "evaluate":
                    HandleEvaluate(request);
                    break;
                case "pause":
                    DebugAgent.RequestPause();
                    SendResponse(request, true, new JsonObject());
                    break;
                case "next":
                case "stepOver":
                    SendResponse(request, true, new JsonObject());
                    DebugAgent.Step(DebugStepMode.StepOver);
                    break;
                case "stepIn":
                    SendResponse(request, true, new JsonObject());
                    DebugAgent.Step(DebugStepMode.StepIn);
                    break;
                case "stepOut":
                    SendResponse(request, true, new JsonObject());
                    DebugAgent.Step(DebugStepMode.StepOut);
                    break;
                case "continue":
                    SendResponse(request, true, new JsonObject { ["allThreadsContinued"] = true });
                    DebugAgent.Resume();
                    break;
                case "disconnect":
                    SendResponse(request, true, new JsonObject());
                    DebugAgent.Resume();
                    Stop();
                    break;
                default:
                    SendError(request, "unsupported request: " + command);
                    break;
            }
        }

        private void HandleSetBreakpoints(JsonObject request)
        {
            JsonObject? arguments = request["arguments"] as JsonObject;
            string path = ReadString(arguments?["source"]?["path"]);
            DebugAgent.ClearBreakpoints(path);
            JsonArray verified = new JsonArray();
            if (arguments?["breakpoints"] is JsonArray breakpoints)
            {
                foreach (JsonNode? node in breakpoints)
                {
                    int line = ReadInt(node?["line"]);
                    string? condition = null;
                    if (node?["condition"] is JsonValue conditionValue && conditionValue.TryGetValue<string>(out string? text))
                    {
                        condition = text;
                    }
                    if (DebugAgent.TryVerifyBreakpointLocation(path, line, out string unboundReason))
                    {
                        DebugAgent.SetBreakpoint(path, line, condition);
                        verified.Add(new JsonObject
                        {
                            ["verified"] = true,
                            ["line"] = line,
                            ["id"] = line
                        });
                    }
                    else
                    {
                        verified.Add(new JsonObject
                        {
                            ["verified"] = false,
                            ["line"] = line,
                            ["id"] = line,
                            ["message"] = unboundReason
                        });
                    }
                }
            }
            SendResponse(request, true, new JsonObject { ["breakpoints"] = verified });
        }

        private void HandleEvaluate(JsonObject request)
        {
            DebugFrame? frame = DebugAgent.CurrentFrame;
            if (frame == null)
            {
                SendError(request, "Cannot evaluate expression when execution is not stopped.");
                return;
            }
            JsonObject? arguments = request["arguments"] as JsonObject;
            string expression = ReadString(arguments?["expression"]);
            if (!ConditionEvaluator.TryEvaluateValue(expression, frame.LocalNames, frame.Locals, out object? value, out string error))
            {
                SendError(request, error);
                return;
            }
            SendResponse(request, true, new JsonObject
            {
                ["result"] = RenderValue(value),
                ["variablesReference"] = 0
            });
        }

        private void HandleStackTrace(JsonObject request)
        {
            DebugFrame? frame = DebugAgent.CurrentFrame;
            JsonArray frames = new JsonArray();
            if (frame != null)
            {
                frames.Add(new JsonObject
                {
                    ["id"] = 1,
                    ["name"] = frame.Method,
                    ["line"] = frame.Line,
                    ["column"] = 1,
                    ["source"] = new JsonObject
                    {
                        ["path"] = frame.File,
                        ["name"] = Path.GetFileName(frame.File)
                    }
                });
            }
            SendResponse(request, true, new JsonObject
            {
                ["stackFrames"] = frames,
                ["totalFrames"] = frames.Count
            });
        }

        private void HandleScopes(JsonObject request)
        {
            JsonArray scopes = new JsonArray();
            if (DebugAgent.CurrentFrame != null)
            {
                scopes.Add(new JsonObject
                {
                    ["name"] = "Locals",
                    ["variablesReference"] = LocalsReference,
                    ["expensive"] = false
                });
            }
            SendResponse(request, true, new JsonObject { ["scopes"] = scopes });
        }

        private void HandleVariables(JsonObject request)
        {
            JsonObject? arguments = request["arguments"] as JsonObject;
            int reference = ReadInt(arguments?["variablesReference"]);
            DebugFrame? frame = DebugAgent.CurrentFrame;
            JsonArray variables = new JsonArray();
            if (frame != null && reference == LocalsReference)
            {
                for (int index = 0; index < frame.Locals.Length; index++)
                {
                    string name = index < frame.LocalNames.Count
                        ? frame.LocalNames[index]
                        : "local" + index.ToString(CultureInfo.InvariantCulture);
                    variables.Add(new JsonObject
                    {
                        ["name"] = name,
                        ["value"] = RenderValue(frame.Locals[index]),
                        ["variablesReference"] = 0
                    });
                }
            }
            SendResponse(request, true, new JsonObject { ["variables"] = variables });
        }

        private static string RenderValue(object? value)
        {
            if (value == null)
            {
                return "null";
            }
            if (value is UnityEngine.Object unityObject && unityObject == null)
            {
                return "destroyed";
            }
            return value.ToString() ?? "null";
        }

        private void SendResponse(JsonObject request, bool success, JsonObject body)
        {
            JsonObject response = new JsonObject
            {
                ["seq"] = NextSequence(),
                ["type"] = "response",
                ["request_seq"] = request["seq"]?.DeepClone(),
                ["success"] = success,
                ["command"] = request["command"]?.DeepClone(),
                ["body"] = body
            };
            Write(response);
        }

        private void SendError(JsonObject request, string message)
        {
            JsonObject response = new JsonObject
            {
                ["seq"] = NextSequence(),
                ["type"] = "response",
                ["request_seq"] = request["seq"]?.DeepClone(),
                ["success"] = false,
                ["command"] = request["command"]?.DeepClone(),
                ["message"] = message,
                ["body"] = new JsonObject
                {
                    ["error"] = new JsonObject
                    {
                        ["id"] = 1,
                        ["format"] = message
                    }
                }
            };
            Write(response);
        }

        private void SendEvent(string name, JsonObject body)
        {
            JsonObject message = new JsonObject
            {
                ["seq"] = NextSequence(),
                ["type"] = "event",
                ["event"] = name,
                ["body"] = body
            };
            Write(message);
        }

        private void Write(JsonObject message)
        {
            byte[] payload = Encoding.UTF8.GetBytes(message.ToJsonString());
            byte[] header = Encoding.ASCII.GetBytes(
                "Content-Length: " + payload.Length.ToString(CultureInfo.InvariantCulture) + "\r\n\r\n");
            lock (_writeGate)
            {
                _transport.Write(header, 0, header.Length);
                _transport.Write(payload, 0, payload.Length);
                _transport.Flush();
            }
        }

        private int NextSequence()
        {
            return Interlocked.Increment(ref _sequence);
        }

        private static string ReadString(JsonNode? node)
        {
            if (node is JsonValue value && value.TryGetValue<string>(out string? text))
            {
                return text;
            }
            return string.Empty;
        }

        private static int ReadInt(JsonNode? node)
        {
            if (node is JsonValue value && value.TryGetValue<int>(out int number))
            {
                return number;
            }
            return 0;
        }

        private static async Task<string?> ReadMessageAsync(Stream stream)
        {
            int contentLength = -1;
            while (true)
            {
                string? line = await ReadLineAsync(stream).ConfigureAwait(false);
                if (line == null)
                {
                    return null;
                }
                if (line.Length == 0)
                {
                    break;
                }
                if (line.StartsWith("Content-Length:", StringComparison.OrdinalIgnoreCase))
                {
                    string value = line.Substring("Content-Length:".Length).Trim();
                    if (!int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out contentLength))
                    {
                        contentLength = -1;
                    }
                }
            }
            if (contentLength < 0)
            {
                return null;
            }
            byte[] buffer = new byte[contentLength];
            int offset = 0;
            while (offset < contentLength)
            {
                int read = await stream.ReadAsync(buffer.AsMemory(offset, contentLength - offset)).ConfigureAwait(false);
                if (read <= 0)
                {
                    return null;
                }
                offset += read;
            }
            return Encoding.UTF8.GetString(buffer);
        }

        private static async Task<string?> ReadLineAsync(Stream stream)
        {
            StringBuilder builder = new StringBuilder();
            byte[] single = new byte[1];
            while (true)
            {
                int read = await stream.ReadAsync(single.AsMemory(0, 1)).ConfigureAwait(false);
                if (read <= 0)
                {
                    return builder.Length == 0 ? null : builder.ToString();
                }
                if (single[0] == (byte)'\n')
                {
                    return builder.ToString().TrimEnd('\r');
                }
                builder.Append((char)single[0]);
            }
        }
    }
}
