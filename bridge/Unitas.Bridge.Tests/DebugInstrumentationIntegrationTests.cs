using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Unitas.Bridge.Scripting;
using Unitas.Debugging;
using Unitas.Scripting;

namespace Unitas.Bridge.Tests
{
    public static class DebugInstrumentationIntegrationTests
    {
        private const string ScriptSource = @"using UnityEngine;

public class ProbeScript
{
    [EditorCallableMethod]
    public static int Run()
    {
        int total = 0;
        total = total + 1;
        total = total + 2;
        return total;
    }
}
";

        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestInstrumentedScriptBreakpointStepAndLocals();
        }

        private static void TestInstrumentedScriptBreakpointStepAndLocals()
        {
            string tempProject = Path.Combine(
                Path.GetTempPath(),
                "unitas_dap_instrumentation_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            string scriptPath = Path.Combine(tempProject, "Assets", "ProbeScript.cs");
            int breakpointLine = LineOf(ScriptSource, "total = total + 2;");
            int stepLine = LineOf(ScriptSource, "return total;");
            Environment.SetEnvironmentVariable(DebugAgent.DapPortEnvironmentVariable, "0");
            try
            {
                Directory.CreateDirectory(Path.Combine(tempProject, "Assets"));

                string instrumented = ScriptAnalyzerAndGlueGenerator.GenerateInstrumentedScriptSource(
                    ScriptSource,
                    scriptPath,
                    true);
                Check.That(
                    "dap instrumentation: generated source emits DebugAgent.Hit probes",
                    instrumented.Contains("DebugAgent.Hit(", StringComparison.Ordinal));
                Check.That(
                    "dap instrumentation: probe carries the source line of the statement",
                    instrumented.Contains(", " + breakpointLine.ToString(CultureInfo.InvariantCulture) + ", ", StringComparison.Ordinal));
                Check.Equal(
                    "dap instrumentation: debugging disabled leaves the script source unchanged",
                    ScriptSource,
                    ScriptAnalyzerAndGlueGenerator.GenerateInstrumentedScriptSource(ScriptSource, scriptPath, false));

                File.WriteAllText(scriptPath, instrumented);
                string diagSource = "DIAG instrumented source:";
                Check.That(diagSource + Environment.NewLine + instrumented, true);

                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64",
                    EmitGeneratedGlue = true,
                    AutoLoadGeneratedGlue = false
                };
                options.AdditionalReferencePaths.Add(typeof(DebugAgent).Assembly.Location);

                ScriptCompilationResult result = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("dap instrumentation: instrumented script project compiles", true, result.Succeeded);
                Check.That(
                    "dap instrumentation: generated glue assembly exists",
                    !string.IsNullOrEmpty(result.GeneratedGlueAssemblyPath) && File.Exists(result.GeneratedGlueAssemblyPath));
                if (result.Succeeded && !string.IsNullOrEmpty(result.GeneratedGlueAssemblyPath))
                {
                    ScriptGlueRegistry.LoadGlueAssembly(result.GeneratedGlueAssemblyPath);
                    RunInstrumentedScript(scriptPath, breakpointLine, stepLine);
                }
            }
            finally
            {
                Environment.SetEnvironmentVariable(DebugAgent.DapPortEnvironmentVariable, null);
                DebugAgent.StopSelfStartedListener();
                DeleteDirectory(tempProject);
            }
        }

        private static void RunInstrumentedScript(string scriptPath, int breakpointLine, int stepLine)
        {
            object? invokeResult = null;
            Task worker = Task.Run(() =>
            {
                EditorMethodRegistry.TryInvoke("ProbeScript.Run", out invokeResult);
            });

            bool portReady = SpinWait.SpinUntil(() => DebugAgent.DapPort.HasValue, 5000);
            int port = DebugAgent.DapPort ?? 0;
            Check.That(
                "dap instrumentation: DebugAgent self-started a loopback DAP listener",
                portReady && port > 0);
            if (!portReady || port <= 0)
            {
                worker.Wait(TimeSpan.FromSeconds(2));
                return;
            }

            using TcpClient socket = new TcpClient();
            string diag = "DIAG";
            Check.That(diag + " connecting to port " + port.ToString(CultureInfo.InvariantCulture), true);
            socket.Connect(IPAddress.Loopback, port);
            Check.That(diag + " connected", true);
            using DapTcpClient client = new DapTcpClient(socket);
            Check.That(diag + " client created", true);

            Check.That(
                "dap instrumentation: initialize succeeds",
                ReadBool(client.Request("initialize")["success"]));
            Check.That(
                "dap instrumentation: attach succeeds",
                ReadBool(client.Request("attach")["success"]));

            JsonObject setResponse = client.Request("setBreakpoints", new JsonObject
            {
                ["source"] = new JsonObject { ["path"] = scriptPath },
                ["breakpoints"] = new JsonArray
                {
                    new JsonObject { ["line"] = breakpointLine }
                }
            });
            JsonArray breakpoints = (JsonArray)setResponse["body"]!["breakpoints"]!;
            Check.Equal("dap instrumentation: one breakpoint entry returned", 1, breakpoints.Count);
            Check.That(
                "dap instrumentation: breakpoint is verified",
                ReadBool(breakpoints[0]?["verified"]));
            Check.That(
                "dap instrumentation: configurationDone succeeds",
                ReadBool(client.Request("configurationDone")["success"]));

            JsonObject stopped = client.WaitForEvent("stopped", TimeSpan.FromSeconds(15));
            Check.Equal("dap instrumentation: breakpoint stop event", "stopped", ReadString(stopped["event"]));
            Check.Equal(
                "dap instrumentation: breakpoint stop reason",
                "breakpoint",
                ReadString(stopped["body"]?["reason"]));

            JsonArray frames = (JsonArray)client.Request("stackTrace")["body"]!["stackFrames"]!;
            Check.Equal("dap instrumentation: stack frame count", 1, frames.Count);
            Check.Equal(
                "dap instrumentation: stack frame reports the breakpoint line",
                breakpointLine,
                ReadInt(frames[0]?["line"]));
            Check.Equal(
                "dap instrumentation: stack frame reports the instrumented method",
                "ProbeScript.Run",
                ReadString(frames[0]?["name"]));

            JsonArray scopes = (JsonArray)client.Request("scopes")["body"]!["scopes"]!;
            Check.Equal("dap instrumentation: one scope returned", 1, scopes.Count);
            int variablesReference = ReadInt(scopes[0]?["variablesReference"]);
            JsonArray variables = (JsonArray)client.Request("variables", new JsonObject
            {
                ["variablesReference"] = variablesReference
            })["body"]!["variables"]!;
            Check.Equal("dap instrumentation: one local variable reported", 1, variables.Count);
            Check.Equal("dap instrumentation: local variable name", "total", ReadString(variables[0]?["name"]));
            Check.Equal("dap instrumentation: local variable value", "1", ReadString(variables[0]?["value"]));

            Check.That(
                "dap instrumentation: step-over request succeeds",
                ReadBool(client.Request("next")["success"]));
            JsonObject stepped = client.WaitForEvent("stopped", TimeSpan.FromSeconds(15));
            Check.Equal(
                "dap instrumentation: step-over stop reason",
                "step",
                ReadString(stepped["body"]?["reason"]));
            JsonArray stepFrames = (JsonArray)client.Request("stackTrace")["body"]!["stackFrames"]!;
            Check.Equal(
                "dap instrumentation: step-over advances to the next statement",
                stepLine,
                ReadInt(stepFrames[0]?["line"]));
            JsonArray stepVariables = (JsonArray)client.Request("variables", new JsonObject
            {
                ["variablesReference"] = variablesReference
            })["body"]!["variables"]!;
            Check.Equal("dap instrumentation: local value after step-over", "3", ReadString(stepVariables[0]?["value"]));

            Check.That(
                "dap instrumentation: continue request succeeds",
                ReadBool(client.Request("continue")["success"]));
            Check.That(
                "dap instrumentation: the instrumented script finishes",
                worker.Wait(TimeSpan.FromSeconds(15)));
            Check.Equal(
                "dap instrumentation: the instrumented script returns its computed value",
                3,
                invokeResult is int intResult ? intResult : -1);

            client.Request("disconnect");
        }

        private static int LineOf(string source, string marker)
        {
            int index = source.IndexOf(marker, StringComparison.Ordinal);
            if (index < 0)
            {
                return 0;
            }
            int line = 1;
            for (int i = 0; i < index; i++)
            {
                if (source[i] == '\n')
                {
                    line++;
                }
            }
            return line;
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

        private static bool ReadBool(JsonNode? node)
        {
            if (node is JsonValue value && value.TryGetValue<bool>(out bool flag))
            {
                return flag;
            }
            return false;
        }

        private static void DeleteDirectory(string path)
        {
            try
            {
                if (Directory.Exists(path))
                {
                    Directory.Delete(path, true);
                }
            }
            catch (IOException)
            {
            }
            catch (UnauthorizedAccessException)
            {
            }
        }

        private sealed class DapTcpClient : IDisposable
        {
            private readonly TcpClient _socket;
            private readonly NetworkStream _stream;
            private readonly BlockingCollection<JsonObject> _messages = new BlockingCollection<JsonObject>();
            private readonly List<JsonObject> _pending = new List<JsonObject>();
            private int _sequence;

            public DapTcpClient(TcpClient socket)
            {
                _socket = socket;
                _stream = socket.GetStream();
                _ = Task.Run(ReadLoopAsync);
            }

            public JsonObject Request(string command, JsonObject? arguments = null)
            {
                int sequence = Interlocked.Increment(ref _sequence);
                JsonObject request = new JsonObject
                {
                    ["seq"] = sequence,
                    ["type"] = "request",
                    ["command"] = command
                };
                if (arguments != null)
                {
                    request["arguments"] = arguments;
                }
                Send(request);
                while (true)
                {
                    JsonObject message = Next(TimeSpan.FromSeconds(15));
                    if (ReadString(message["type"]) == "response" && ReadInt(message["request_seq"]) == sequence)
                    {
                        return message;
                    }
                    _pending.Add(message);
                }
            }

            public JsonObject WaitForEvent(string name, TimeSpan timeout)
            {
                for (int i = 0; i < _pending.Count; i++)
                {
                    if (ReadString(_pending[i]["event"]) == name)
                    {
                        JsonObject pending = _pending[i];
                        _pending.RemoveAt(i);
                        return pending;
                    }
                }
                while (true)
                {
                    JsonObject message = Next(timeout);
                    if (ReadString(message["event"]) == name)
                    {
                        return message;
                    }
                    _pending.Add(message);
                }
            }

            public void Dispose()
            {
                _messages.Dispose();
                _stream.Dispose();
                _socket.Dispose();
            }

            private JsonObject Next(TimeSpan timeout)
            {
                if (_messages.TryTake(out JsonObject? message, timeout))
                {
                    return message;
                }
                return new JsonObject();
            }

            private void Send(JsonObject message)
            {
                byte[] payload = Encoding.UTF8.GetBytes(message.ToJsonString());
                byte[] header = Encoding.ASCII.GetBytes(
                    "Content-Length: " + payload.Length.ToString(CultureInfo.InvariantCulture) + "\r\n\r\n");
                _stream.Write(header, 0, header.Length);
                _stream.Write(payload, 0, payload.Length);
                _stream.Flush();
            }

            private async Task ReadLoopAsync()
            {
                try
                {
                    while (true)
                    {
                        string? message = await ReadMessageAsync().ConfigureAwait(false);
                        if (message == null)
                        {
                            return;
                        }
                        if (JsonNode.Parse(message) is JsonObject parsed)
                        {
                            _messages.Add(parsed);
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
                catch (JsonException)
                {
                }
            }

            private async Task<string?> ReadMessageAsync()
            {
                int contentLength = -1;
                while (true)
                {
                    string? line = await ReadLineAsync().ConfigureAwait(false);
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
                    int read = await _stream.ReadAsync(buffer.AsMemory(offset, contentLength - offset)).ConfigureAwait(false);
                    if (read <= 0)
                    {
                        return null;
                    }
                    offset += read;
                }
                return Encoding.UTF8.GetString(buffer);
            }

            private async Task<string?> ReadLineAsync()
            {
                StringBuilder builder = new StringBuilder();
                byte[] single = new byte[1];
                while (true)
                {
                    int read = await _stream.ReadAsync(single.AsMemory(0, 1)).ConfigureAwait(false);
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

    internal static class DebugInstrumentationTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnFirstShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsFirstShard(args))
            {
                return;
            }
            DebugInstrumentationIntegrationTests.Run();
        }

        private static bool IsPerfMode(string[] args)
        {
            for (int i = 0; i < args.Length; i++)
            {
                if (string.Equals(args[i], "--perf", StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool IsFirstShard(string[] args)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (!string.Equals(args[i], "--shard", StringComparison.Ordinal))
                {
                    continue;
                }
                string[] parts = args[i + 1].Split('/');
                return parts.Length == 2 && string.Equals(parts[0], "1", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
