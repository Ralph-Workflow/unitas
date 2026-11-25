using System;
using System.Collections.Concurrent;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Unitas.Debugging;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class DapSteppingTests
    {
        public static void Run()
        {
            TestLaunchAndUnboundBreakpoints();
            TestSteppingAcrossSequencePoints();
            TestPauseStopsAtNextSequencePoint();
            TestEvaluateLocalsDestroyedAndRejectsSideEffects();
            TestLoopbackBindingRejectsNonLoopbackAddresses();
        }

        private static void TestLaunchAndUnboundBreakpoints()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(101, "/game/Player.cs", 15, "Player.Update", new[] { "hp" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                Check.That("dap stepping: initialize succeeds", ReadBool(client.Request("initialize")["success"]));
                JsonObject launchResponse = client.Request("launch");
                Check.That("dap stepping: launch succeeds", ReadBool(launchResponse["success"]));
                Check.That(
                    "dap stepping: launch reports that the adapter attaches to an existing play session",
                    ReadString(launchResponse["body"]!["message"]).Contains("does not start a play session", StringComparison.Ordinal));
                Check.That("dap stepping: session marks launched", session.IsLaunched && session.IsAttached);

                JsonObject setResponse = client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Player.cs" },
                    ["breakpoints"] = new JsonArray
                    {
                        new JsonObject { ["line"] = 15 },
                        new JsonObject { ["line"] = 99 }
                    }
                });
                JsonArray breakpoints = (JsonArray)setResponse["body"]!["breakpoints"]!;
                Check.Equal("dap stepping: two breakpoint entries returned", 2, breakpoints.Count);
                Check.That("dap stepping: bound breakpoint is verified", ReadBool(breakpoints[0]?["verified"]));
                Check.Equal("dap stepping: bound breakpoint line matches", 15, ReadInt(breakpoints[0]?["line"]));
                Check.That("dap stepping: unbound breakpoint is unverified", !ReadBool(breakpoints[1]?["verified"]));
                Check.Equal("dap stepping: unbound breakpoint line matches", 99, ReadInt(breakpoints[1]?["line"]));
                Check.That(
                    "dap stepping: unbound breakpoint explains missing sequence point",
                    ReadString(breakpoints[1]?["message"]).Contains("No executable sequence point at line 99.", StringComparison.Ordinal));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestSteppingAcrossSequencePoints()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(201, "/game/Stepper.cs", 10, "Stepper.Tick", new[] { "stage" });
            table.Add(202, "/game/Stepper.cs", 20, "Stepper.Helper", new[] { "stage" });
            table.Add(203, "/game/Stepper.cs", 21, "Stepper.Helper", new[] { "stage" });
            table.Add(204, "/game/Stepper.cs", 11, "Stepper.Tick", new[] { "stage" });
            table.Add(205, "/game/Stepper.cs", 30, "Stepper.Deep", new[] { "stage" });
            table.Add(206, "/game/Stepper.cs", 12, "Stepper.Tick", new[] { "stage" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                client.Request("initialize");
                client.Request("launch");
                client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Stepper.cs" },
                    ["breakpoints"] = new JsonArray { new JsonObject { ["line"] = 10 } }
                });
                client.Request("configurationDone");

                Task worker = Task.Run(() =>
                {
                    DebugAgent.Hit(201, new object?[] { 1 }, 0);
                    DebugAgent.Hit(202, new object?[] { 2 }, 1);
                    DebugAgent.Hit(203, new object?[] { 3 }, 1);
                    DebugAgent.Hit(204, new object?[] { 4 }, 0);
                    DebugAgent.Hit(205, new object?[] { 5 }, 1);
                    DebugAgent.Hit(206, new object?[] { 6 }, 0);
                });

                JsonObject firstStop = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: initial breakpoint stop reason", "breakpoint", ReadString(firstStop["body"]?["reason"]));

                Check.That("dap stepping: stepIn request succeeds", ReadBool(client.Request("stepIn")["success"]));
                JsonObject stepInStop = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: stepIn emits step reason", "step", ReadString(stepInStop["body"]?["reason"]));
                JsonArray stepInFrames = (JsonArray)client.Request("stackTrace")["body"]!["stackFrames"]!;
                Check.Equal("dap stepping: stepIn lands on helper line 20", 20, ReadInt(stepInFrames[0]?["line"]));

                Check.That("dap stepping: stepOut request succeeds", ReadBool(client.Request("stepOut")["success"]));
                JsonObject stepOutStop = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: stepOut emits step reason", "step", ReadString(stepOutStop["body"]?["reason"]));
                JsonArray stepOutFrames = (JsonArray)client.Request("stackTrace")["body"]!["stackFrames"]!;
                Check.Equal("dap stepping: stepOut skips sibling line 21 and lands on caller line 11", 11, ReadInt(stepOutFrames[0]?["line"]));

                Check.That("dap stepping: next request succeeds", ReadBool(client.Request("next")["success"]));
                JsonObject nextStop = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: next emits step reason", "step", ReadString(nextStop["body"]?["reason"]));
                JsonArray nextFrames = (JsonArray)client.Request("stackTrace")["body"]!["stackFrames"]!;
                Check.Equal("dap stepping: next steps over deeper frame 30 and lands on line 12", 12, ReadInt(nextFrames[0]?["line"]));

                Check.That("dap stepping: continue after stepping succeeds", ReadBool(client.Request("continue")["success"]));
                Check.That("dap stepping: worker finishes after stepping", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestPauseStopsAtNextSequencePoint()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(301, "/game/Loop.cs", 8, "Loop.Step", new[] { "tick" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                client.Request("initialize");
                client.Request("attach");
                Check.That("dap stepping: pause request succeeds", ReadBool(client.Request("pause")["success"]));

                Task worker = Task.Run(() => DebugAgent.Hit(301, new object?[] { 42 }));
                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: pause emits stopped event", "stopped", ReadString(stopped["event"]));
                Check.Equal("dap stepping: pause reason is pause", "pause", ReadString(stopped["body"]?["reason"]));

                client.Request("continue");
                Check.That("dap stepping: paused worker resumes on continue", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestEvaluateLocalsDestroyedAndRejectsSideEffects()
        {
            LoopbackPair pair = LoopbackPair.Create();
            SequencePointTable table = new SequencePointTable();
            table.Add(401, "/game/Eval.cs", 25, "Eval.Inspect", new[] { "count", "title", "destroyedTarget" });
            DebugAgent.RegisterSequencePoints(table);
            DebugSession session = DebugAgent.BeginSession(pair.Server);
            try
            {
                using DapClient client = new DapClient(pair.Client);
                client.Request("initialize");
                client.Request("launch");
                client.Request("setBreakpoints", new JsonObject
                {
                    ["source"] = new JsonObject { ["path"] = "/game/Eval.cs" },
                    ["breakpoints"] = new JsonArray { new JsonObject { ["line"] = 25 } }
                });

                GameObject target = new GameObject("TempEvalTarget");
                UnityEngine.Object.DestroyImmediate(target);

                Task worker = Task.Run(() => DebugAgent.Hit(401, new object?[] { 12, "ready", target }));
                JsonObject stopped = client.Receive(TimeSpan.FromSeconds(5));
                Check.Equal("dap stepping: evaluate test stops at breakpoint", "stopped", ReadString(stopped["event"]));

                JsonObject evalCount = client.Request("evaluate", new JsonObject { ["expression"] = "count" });
                Check.That("dap stepping: evaluate local variable succeeds", ReadBool(evalCount["success"]));
                Check.Equal("dap stepping: evaluate local variable result", "12", ReadString(evalCount["body"]?["result"]));

                JsonObject evalDestroyed = client.Request("evaluate", new JsonObject { ["expression"] = "destroyedTarget" });
                Check.That("dap stepping: evaluate destroyed object succeeds", ReadBool(evalDestroyed["success"]));
                Check.Equal("dap stepping: evaluate destroyed object renders destroyed", "destroyed", ReadString(evalDestroyed["body"]?["result"]));

                JsonObject evalComparison = client.Request("evaluate", new JsonObject { ["expression"] = "count >= 10 && title == 'ready'" });
                Check.That("dap stepping: evaluate boolean comparison succeeds", ReadBool(evalComparison["success"]));
                Check.Equal("dap stepping: evaluate boolean comparison result", "True", ReadString(evalComparison["body"]?["result"]));

                JsonObject evalMutation = client.Request("evaluate", new JsonObject { ["expression"] = "count = 99" });
                Check.That("dap stepping: evaluate assignment is rejected", !ReadBool(evalMutation["success"]));
                Check.That(
                    "dap stepping: evaluate assignment rejection explains read-only policy",
                    ReadString(evalMutation["message"]).Contains("Side-effecting", StringComparison.Ordinal));

                client.Request("continue");
                Check.That("dap stepping: evaluate worker resumes", worker.Wait(TimeSpan.FromSeconds(5)));
            }
            finally
            {
                session.Dispose();
            }
        }

        private static void TestLoopbackBindingRejectsNonLoopbackAddresses()
        {
            DebugAgent.ValidateLoopbackEndpoint(IPAddress.Loopback);
            DebugAgent.ValidateLoopbackEndpoint(IPAddress.IPv6Loopback);

            bool anyRejected = false;
            try
            {
                DebugAgent.ValidateLoopbackEndpoint(IPAddress.Any);
            }
            catch (InvalidOperationException exception)
            {
                anyRejected = exception.Message.Contains("loopback", StringComparison.Ordinal);
            }
            Check.That("dap stepping: IPAddress.Any is rejected by loopback guard", anyRejected);

            bool externalRejected = false;
            try
            {
                DebugAgent.ValidateLoopbackEndpoint(IPAddress.Parse("192.168.1.1"));
            }
            catch (InvalidOperationException exception)
            {
                externalRejected = exception.Message.Contains("loopback", StringComparison.Ordinal);
            }
            Check.That("dap stepping: external IPv4 address is rejected by loopback guard", externalRejected);

            TcpListener listener = DebugAgent.BeginLoopbackListener(IPAddress.Loopback, 0);
            try
            {
                IPEndPoint endpoint = (IPEndPoint)listener.LocalEndpoint;
                Check.That("dap stepping: loopback listener binds an ephemeral port", endpoint.Port > 0);
                Check.That("dap stepping: loopback listener address is loopback", IPAddress.IsLoopback(endpoint.Address));
            }
            finally
            {
                listener.Stop();
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
