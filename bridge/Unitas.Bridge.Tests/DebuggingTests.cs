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
        }

        private static void TestUnknownRequestReturnsErrorResponse()
        {
            LoopbackPair pair = LoopbackPair.Create();
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                DapClient client = new DapClient(pair.Client);
                JsonObject response = client.Request("teleport");
                Check.That("an unknown request is answered with a response", ReadString(response["type"]) == "response");
                Check.That("an unknown request is rejected", ReadBool(response["success"]) == false);
                Check.That("an unknown request names the command", ReadString(response["message"]).Contains("teleport"));
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
                DapClient client = new DapClient(pair.Client);
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
                DapClient client = new DapClient(pair.Client);
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
                DapClient client = new DapClient(pair.Client);
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

        private sealed class DapClient
        {
            private readonly Stream _stream;
            private readonly BlockingCollection<JsonObject> _messages = new BlockingCollection<JsonObject>();
            private int _sequence;

            public DapClient(Stream stream)
            {
                _stream = stream;
                _ = Task.Run(ReadLoopAsync);
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
