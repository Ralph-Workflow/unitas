using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Unitas.Debugging;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class DebuggingTests
    {
        public static void Run()
        {
            TestUnknownRequestReturnsErrorResponse();
            TestBreakpointStopsAndContinues();
            TestConditionalBreakpointHonorsCondition();
            TestDestroyedObjectRendersDestroyed();
            TestCompiledCoroutineConditionalBreakpointAndDestroyedObject();
            TestDebugAgentLibraryBreakpointNotificationLocalsAndContinue();
        }

        private static void TestUnknownRequestReturnsErrorResponse()
        {
            LoopbackPair pair = LoopbackPair.Create();
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                JsonObject response = client.Request("teleport");
                Check.That("an unknown request is answered with a response", ReadString(response["type"]) == "response");
                Check.That("an unknown request is rejected", ReadBool(response["success"]) == false);
                Check.That("an unknown request names the command", ReadString(response["message"]).Contains("teleport", StringComparison.Ordinal));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestBreakpointStopsAndContinues()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(11, "/game/Worker.cs", 42, "Worker.Run", new[] { "count", "label" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                Check.That("initialize succeeds", ReadBool(client.Request("initialize")["success"]));
                Check.That("attach succeeds", ReadBool(client.Request("attach")["success"]));
                JsonObject setResponse = client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Worker.cs" },
                    ["breakpoints"] = new JsonArray { new JsonObject { ["line"] = 42 } }
                });
                JsonArray verified = (JsonArray)setResponse["body"]!["breakpoints"]!;
                Check.That("setBreakpoints verifies the breakpoint", verified.Count == 1);
                Check.That("setBreakpoints reports the requested line", ReadInt(verified[0]?["line"]) == 42);
                Check.That("configurationDone succeeds", ReadBool(client.Request("configurationDone")["success"]));

                Task worker = Task.Run(() => DebugAgent.Hit(11, new object?[] { 7, "abc" }));
                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.That("a breakpoint emits the stopped event", ReadString(stopped["event"]) == "stopped");
                Check.That("the stopped event names the breakpoint reason", ReadString(stopped["body"]?["reason"]) == "breakpoint");

                JsonObject threads = client.Request("threads");
                Check.That("threads lists the debugged thread", ((JsonArray)threads["body"]!["threads"]!).Count == 1);

                JsonObject stack = client.Request("stackTrace");
                JsonArray frames = (JsonArray)stack["body"]!["stackFrames"]!;
                Check.That("stackTrace returns the stopped frame", frames.Count == 1);
                Check.That("stackTrace names the method", ReadString(frames[0]?["name"]) == "Worker.Run");
                Check.That("stackTrace reports the line", ReadInt(frames[0]?["line"]) == 42);
                Check.That("stackTrace reports the source path", ReadString(frames[0]?["source"]?["path"]) == "/game/Worker.cs");

                JsonObject scopes = client.Request("scopes");
                JsonArray scopeArray = (JsonArray)scopes["body"]!["scopes"]!;
                Check.That("scopes exposes the locals scope", scopeArray.Count == 1);
                int reference = ReadInt(scopeArray[0]?["variablesReference"]);

                JsonObject variables = client.Request("variables", new JsonObject { ["variablesReference"] = reference });
                JsonArray locals = (JsonArray)variables["body"]!["variables"]!;
                Check.That("variables exposes every captured local", locals.Count == 2);
                Check.That("a local keeps its name", ReadString(locals[0]?["name"]) == "count");
                Check.That("a numeric local renders its value", ReadString(locals[0]?["value"]) == "7");
                Check.That("a string local renders its value", ReadString(locals[1]?["value"]) == "abc");

                Check.That("continue succeeds", ReadBool(client.Request("continue")["success"]));
                Check.That("the instrumented thread resumes after continue", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestConditionalBreakpointHonorsCondition()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(21, "/game/Coroutine.cs", 7, "Coroutine.Step", new[] { "index" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                client.Request("initialize");
                client.Request("attach");
                client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Coroutine.cs" },
                    ["breakpoints"] = new JsonArray { new JsonObject { ["line"] = 7, ["condition"] = "index == 2" } }
                });

                DebugAgent.Hit(21, new object?[] { 1 });
                JsonObject missed = client.Receive(TimeSpan.FromMilliseconds(300));
                Check.That("a false condition does not stop the thread", ReadString(missed["event"]) != "stopped");

                Task worker = Task.Run(() => DebugAgent.Hit(21, new object?[] { 2 }));
                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.That("a true condition stops the thread", ReadString(stopped["event"]) == "stopped");
                Check.That("continue succeeds after a conditional stop", ReadBool(client.Request("continue")["success"]));
                Check.That("the conditional stop resumes", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestDestroyedObjectRendersDestroyed()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(31, "/game/Spawner.cs", 5, "Spawner.Tick", new[] { "victim" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                client.Request("initialize");
                client.Request("attach");
                client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Spawner.cs" },
                    ["breakpoints"] = new JsonArray { new JsonObject { ["line"] = 5 } }
                });

                GameObject victim = new GameObject("Victim");
                UnityEngine.Object.DestroyImmediate(victim);

                Task worker = Task.Run(() => DebugAgent.Hit(31, new object?[] { victim }));
                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.That("the destroyed object breakpoint stops the thread", ReadString(stopped["event"]) == "stopped");

                JsonObject scopes = client.Request("scopes");
                JsonArray scopeArray = (JsonArray)scopes["body"]!["scopes"]!;
                int reference = ReadInt(scopeArray[0]?["variablesReference"]);
                JsonObject variables = client.Request("variables", new JsonObject { ["variablesReference"] = reference });
                JsonArray locals = (JsonArray)variables["body"]!["variables"]!;
                Check.That("a destroyed object renders as destroyed", ReadString(locals[0]?["value"]) == "destroyed");

                client.Request("continue");
                Check.That("the destroyed object stop resumes", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestCompiledCoroutineConditionalBreakpointAndDestroyedObject()
        {
            string tempProject = Path.Combine(
                Path.GetTempPath(),
                "unitas_debug_coroutine_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(Path.Combine(tempProject, "Assets", "Gameplay"));
            LoopbackPair pair = LoopbackPair.Create();
            DebugSession? session = null;
            try
            {
                string asmdefPath = Path.Combine(tempProject, "Assets", "Gameplay", "MyGame.Gameplay.asmdef");
                File.WriteAllText(
                    asmdefPath,
                    "{\n  \"name\": \"MyGame.Gameplay\",\n  \"rootNamespace\": \"MyGame.Gameplay\"\n}\n");

                string scriptPath = Path.GetFullPath(Path.Combine(tempProject, "Assets", "Gameplay", "CoroutineProbe.cs"));
                string scriptSource =
                    "using System.Collections;\n"
                    + "using UnityEngine;\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    [System.Serializable]\n"
                    + "    public class CoroutineProbe : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int CompletedSteps;\n"
                    + "        public string DestroyedVictimName = string.Empty;\n"
                    + "        public override void Start()\n"
                    + "        {\n"
                    + "            StartCoroutine(RunSteps());\n"
                    + "        }\n"
                    + "        private IEnumerator RunSteps()\n"
                    + "        {\n"
                    + "            GameObject victim = new GameObject(\"CoroutineVictim\");\n"
                    + "            DestroyedVictimName = victim.name;\n"
                    + "            Object.DestroyImmediate(victim);\n"
                    + "            for (int step = 1; step <= 3; step++)\n"
                    + "            {\n"
                    + "                CompletedSteps = step;\n"
                    + "                yield return null;\n"
                    + "            }\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";
                File.WriteAllText(scriptPath, scriptSource);
                File.WriteAllText(
                    scriptPath + ".meta",
                    "fileFormatVersion: 2\nguid: 30000000000000000000000000000077\nMonoImporter:\n  serializedVersion: 2\n");

                Unitas.Scripting.ScriptCompilationResult compileResult = Unitas.Scripting.ScriptCompilation.CompileProject(
                    tempProject,
                    new Unitas.Scripting.ScriptCompilationOptions
                    {
                        IsEditor = false,
                        IsDevelopmentBuild = true,
                        EmitGeneratedGlue = true,
                        AutoLoadGeneratedGlue = false
                    });
                Check.Equal("compiled coroutine debug: script compilation succeeds", true, compileResult.Succeeded);

                Unitas.Scripting.CompiledAssemblyRecord? gameplayAsm = compileResult.FindAssembly("MyGame.Gameplay");
                Check.That("compiled coroutine debug: MyGame.Gameplay assembly exists", gameplayAsm != null && File.Exists(gameplayAsm.OutputPath));

                SequencePointTable table = PdbSequencePointExtractor.ExtractFromAssembly(
                    gameplayAsm!.OutputPath,
                    new[] { scriptPath });
                Check.That("compiled coroutine debug: sequence point table extracts line 21 inside coroutine MoveNext", table.ContainsLocation(scriptPath, 21));

                SequencePoint? line21Point = table.FindByLocation(scriptPath, 21);
                Check.That("compiled coroutine debug: line 21 sequence point is resolved", line21Point != null);
                Check.Equal(
                    "compiled coroutine debug: coroutine frame method maps to MyGame.Gameplay.CoroutineProbe.RunSteps",
                    "MyGame.Gameplay.CoroutineProbe.RunSteps",
                    line21Point != null ? line21Point.Method : string.Empty);
                Check.That(
                    "compiled coroutine debug: extracted coroutine sequence point includes hoisted locals step and victim",
                    line21Point != null && ContainsLocalName(line21Point.LocalNames, "step") && ContainsLocalName(line21Point.LocalNames, "victim"));

                DebugAgent.RegisterSequencePoints(table);
                session = DebugAgent.BeginSession(pair.Server);

                using DapClient client = new DapClient(pair.Client);
                Check.That("compiled coroutine debug: initialize succeeds", ReadBool(client.Request("initialize")["success"]));
                Check.That("compiled coroutine debug: attach succeeds", ReadBool(client.Request("attach")["success"]));

                JsonObject setResponse = client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = scriptPath },
                    ["breakpoints"] = new JsonArray
                    {
                        new JsonObject
                        {
                            ["line"] = 21,
                            ["condition"] = "step == 2"
                        }
                    }
                });
                JsonArray verified = (JsonArray)setResponse["body"]!["breakpoints"]!;
                Check.Equal("compiled coroutine debug: breakpoint count is 1", 1, verified.Count);
                Check.That("compiled coroutine debug: breakpoint at coroutine line 21 is verified", ReadBool(verified[0]?["verified"]));
                Check.Equal("compiled coroutine debug: verified breakpoint line is 21", 21, ReadInt(verified[0]?["line"]));
                Check.That("compiled coroutine debug: configurationDone succeeds", ReadBool(client.Request("configurationDone")["success"]));

                Unitas.Bridge.Scripting.ScriptGlueRegistry.LoadGlueAssembly(compileResult.GeneratedGlueAssemblyPath!);
                bool created = Unitas.Bridge.Scripting.ScriptGlueRegistry.TryCreate(
                    "30000000000000000000000000000077",
                    out MonoBehaviour? instance,
                    out IUnitasMessageHandler? _);
                Check.Equal("compiled coroutine debug: CoroutineProbe created via generated glue", true, created && instance != null);

                GameObject hostObject = new GameObject("CoroutineProbeHost");
                hostObject.AttachComponent(instance!);

                GameObject destroyedRef = new GameObject("CoroutineVictim");
                UnityEngine.Object.DestroyImmediate(destroyedRef);

                Task coroutineDriver = Task.Run(() =>
                {
                    instance!.Start();
                    DebugAgent.HitLocation(
                        scriptPath,
                        21,
                        BuildLocalsForPoint(line21Point!, 1, destroyedRef));

                    CoroutineScheduler.PumpFrame();
                    DebugAgent.HitLocation(
                        scriptPath,
                        21,
                        BuildLocalsForPoint(line21Point!, 2, destroyedRef));

                    CoroutineScheduler.PumpFrame();
                    DebugAgent.HitLocation(
                        scriptPath,
                        21,
                        BuildLocalsForPoint(line21Point!, 3, destroyedRef));
                });

                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("compiled coroutine debug: conditional breakpoint emits stopped event", "stopped", ReadString(stopped["event"]));
                Check.Equal("compiled coroutine debug: stop reason is breakpoint", "breakpoint", ReadString(stopped["body"]?["reason"]));

                JsonObject stack = client.Request("stackTrace");
                JsonArray frames = (JsonArray)stack["body"]!["stackFrames"]!;
                Check.Equal("compiled coroutine debug: stackTrace has 1 frame", 1, frames.Count);
                Check.Equal(
                    "compiled coroutine debug: stackTrace frame name is MyGame.Gameplay.CoroutineProbe.RunSteps",
                    "MyGame.Gameplay.CoroutineProbe.RunSteps",
                    ReadString(frames[0]?["name"]));
                Check.Equal("compiled coroutine debug: stackTrace frame line is 21", 21, ReadInt(frames[0]?["line"]));
                Check.Equal("compiled coroutine debug: stackTrace frame source path matches script", scriptPath, ReadString(frames[0]?["source"]?["path"]));

                JsonObject scopes = client.Request("scopes");
                JsonArray scopeArray = (JsonArray)scopes["body"]!["scopes"]!;
                int localsRef = ReadInt(scopeArray[0]?["variablesReference"]);
                JsonObject variables = client.Request("variables", new JsonObject { ["variablesReference"] = localsRef });
                JsonArray locals = (JsonArray)variables["body"]!["variables"]!;

                string stepVal = FindVariableValue(locals, "step");
                string victimVal = FindVariableValue(locals, "victim");
                Check.Equal("compiled coroutine debug: condition stopped when step == 2", "2", stepVal);
                Check.Equal("compiled coroutine debug: destroyed referenced object renders as destroyed", "destroyed", victimVal);

                Check.That("compiled coroutine debug: continue succeeds", ReadBool(client.Request("continue")["success"]));
                Check.That("compiled coroutine debug: coroutine finishes after continue", coroutineDriver.Wait(TimeSpan.FromSeconds(5)));

                CoroutineScheduler.StopAll();
                UnityEngine.Object.DestroyImmediate(hostObject);
            }
            finally
            {
                session?.Dispose();
                if (Directory.Exists(tempProject))
                {
                    try
                    {
                        Directory.Delete(tempProject, true);
                    }
                    catch (IOException)
                    {
                    }
                }
            }
        }

        private static bool ContainsLocalName(IReadOnlyList<string> names, string expected)
        {
            for (int index = 0; index < names.Count; index++)
            {
                if (string.Equals(names[index], expected, StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static object?[] BuildLocalsForPoint(SequencePoint point, int stepValue, GameObject victimValue)
        {
            object?[] values = new object?[point.LocalNames.Count];
            for (int index = 0; index < point.LocalNames.Count; index++)
            {
                string localName = point.LocalNames[index];
                if (string.Equals(localName, "step", StringComparison.Ordinal))
                {
                    values[index] = stepValue;
                }
                else if (string.Equals(localName, "victim", StringComparison.Ordinal))
                {
                    values[index] = victimValue;
                }
            }
            return values;
        }

        private static readonly string[] AgentPlayLocalNames = new[] { "frameIndex", "status", "destroyedActor" };

        private static void TestDebugAgentLibraryBreakpointNotificationLocalsAndContinue()
        {
            SequencePointTable table = new SequencePointTable();
            table.Add(501, "/headless/AgentPlay.cs", 30, "AgentPlay.Tick", AgentPlayLocalNames);
            DebugAgent.RegisterSequencePoints(table);

            using BlockingCollection<DebugFrame> notifications = new BlockingCollection<DebugFrame>();
            using DebugSession session = DebugAgent.BeginHeadlessSession(frame => notifications.Add(frame));

            Check.That(
                "debug agent library: TrySetBreakpoint verifies valid sequence point",
                DebugAgent.TrySetBreakpoint("/headless/AgentPlay.cs", 30, "frameIndex == 3", out string error) && string.IsNullOrEmpty(error));
            Check.That(
                "debug agent library: TrySetBreakpoint rejects invalid line with explanation",
                !DebugAgent.TrySetBreakpoint("/headless/AgentPlay.cs", 999, null, out string unboundError)
                && unboundError.Contains("No executable sequence point at line 999.", StringComparison.Ordinal));

            GameObject actor = new GameObject("DestroyedAgentActor");
            UnityEngine.Object.DestroyImmediate(actor);

            DebugAgent.Hit(501, new object?[] { 1, "warming", actor });
            Check.Equal("debug agent library: false condition does not notify stop", 0, notifications.Count);

            Task playTask = Task.Run(() =>
            {
                DebugAgent.Hit(501, new object?[] { 3, "active", actor });
            });

            bool notified = notifications.TryTake(out DebugFrame? stoppedFrame, TimeSpan.FromSeconds(5));
            Check.That("debug agent library: agent receives stop notification on breakpoint hit", notified && stoppedFrame != null);
            Check.Equal("debug agent library: notification frame method matches", "AgentPlay.Tick", stoppedFrame!.Method);
            Check.Equal("debug agent library: notification frame file matches", "/headless/AgentPlay.cs", stoppedFrame.File);
            Check.Equal("debug agent library: notification frame line matches", 30, stoppedFrame.Line);

            IReadOnlyDictionary<string, string> renderedLocals = DebugAgent.ReadCurrentLocals();
            Check.Equal("debug agent library: ReadCurrentLocals count is 3", 3, renderedLocals.Count);
            Check.Equal("debug agent library: ReadCurrentLocals reads frameIndex", "3", renderedLocals["frameIndex"]);
            Check.Equal("debug agent library: ReadCurrentLocals reads status", "active", renderedLocals["status"]);
            Check.Equal("debug agent library: ReadCurrentLocals renders destroyed object as destroyed", "destroyed", renderedLocals["destroyedActor"]);

            DebugAgent.Continue();
            Check.That("debug agent library: play session continues and completes after Continue()", playTask.Wait(TimeSpan.FromSeconds(5)));
        }

        private static string FindVariableValue(JsonArray variables, string name)
        {
            foreach (JsonNode? node in variables)
            {
                if (string.Equals(ReadString(node?["name"]), name, StringComparison.Ordinal))
                {
                    return ReadString(node?["value"]);
                }
            }
            return string.Empty;
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

        private sealed class LoopbackPair
        {
            private LoopbackPair(Stream server, Stream client)
            {
                Server = server;
                Client = client;
            }

            public Stream Server { get; }
            public Stream Client { get; }

            public static LoopbackPair Create()
            {
                Channel toServer = new Channel();
                Channel toClient = new Channel();
                return new LoopbackPair(new LoopbackStream(toServer, toClient), new LoopbackStream(toClient, toServer));
            }
        }

        private sealed class Channel
        {
            public BlockingCollection<byte[]> Items { get; } = new BlockingCollection<byte[]>();
        }

        private sealed class LoopbackStream : Stream
        {
            private readonly Channel _inbound;
            private readonly Channel _outbound;
            private byte[] _pending = Array.Empty<byte>();
            private int _offset;

            public LoopbackStream(Channel inbound, Channel outbound)
            {
                _inbound = inbound;
                _outbound = outbound;
            }

            public override bool CanRead => true;
            public override bool CanSeek => false;
            public override bool CanWrite => true;
            public override long Length => throw new NotSupportedException();

            public override long Position
            {
                get => throw new NotSupportedException();
                set => throw new NotSupportedException();
            }

            public override int Read(byte[] buffer, int offset, int count)
            {
                if (_pending.Length - _offset == 0)
                {
                    try
                    {
                        _pending = _inbound.Items.Take();
                    }
                    catch (InvalidOperationException)
                    {
                        return 0;
                    }
                    _offset = 0;
                }
                int copied = Math.Min(count, _pending.Length - _offset);
                Array.Copy(_pending, _offset, buffer, offset, copied);
                _offset += copied;
                return copied;
            }

            public override void Write(byte[] buffer, int offset, int count)
            {
                byte[] copy = new byte[count];
                Array.Copy(buffer, offset, copy, 0, count);
                _outbound.Items.Add(copy);
            }

            public override void Flush()
            {
            }

            public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();

            public override void SetLength(long value) => throw new NotSupportedException();

            protected override void Dispose(bool disposing)
            {
                if (disposing)
                {
                    _inbound.Items.CompleteAdding();
                }
                base.Dispose(disposing);
            }
        }

        private sealed class DapClient : IDisposable
        {
            private readonly Stream _stream;
            private readonly BlockingCollection<JsonObject> _messages = new BlockingCollection<JsonObject>();
            private int _sequence;

            public DapClient(Stream stream)
            {
                _stream = stream;
                _ = Task.Run(ReadLoopAsync);
            }

            public void Dispose()
            {
                _messages.Dispose();
                _stream.Dispose();
            }

            public JsonObject Request(string command, JsonObject? arguments = null)
            {
                int seq = Interlocked.Increment(ref _sequence);
                JsonObject request = new JsonObject
                {
                    ["seq"] = seq,
                    ["type"] = "request",
                    ["command"] = command
                };
                if (arguments != null)
                {
                    request["arguments"] = arguments;
                }
                Send(request);
                return AwaitResponse(seq);
            }

            public JsonObject Receive(TimeSpan timeout)
            {
                if (!_messages.TryTake(out JsonObject? message, timeout))
                {
                    return new JsonObject();
                }
                return message;
            }

            private JsonObject AwaitResponse(int seq)
            {
                while (true)
                {
                    JsonObject message = Receive(TimeSpan.FromSeconds(5));
                    if (ReadString(message["type"]) == "response" && ReadInt(message["request_seq"]) == seq)
                    {
                        return message;
                    }
                }
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
                while (true)
                {
                    string? message = await ReadMessageAsync(_stream).ConfigureAwait(false);
                    if (message == null)
                    {
                        return;
                    }
                    try
                    {
                        JsonNode? node = JsonNode.Parse(message);
                        if (node is JsonObject parsed)
                        {
                            _messages.Add(parsed);
                        }
                    }
                    catch (JsonException)
                    {
                    }
                }
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
}
