using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;

namespace Unitas.Debugging
{
    /// <summary>
    /// A debug adapter protocol client that talks to a debug adapter server over a
    /// loopback TCP connection. Requests are correlated with their responses by
    /// sequence number and every wait is bounded, so a caller never hangs.
    /// </summary>
    public sealed class DapClient : IDisposable
    {
        public const string StoppedEvent = "stopped";

        public static readonly TimeSpan DefaultTimeout = TimeSpan.FromSeconds(10);

        private static readonly TimeSpan ConnectRetryInterval = TimeSpan.FromMilliseconds(50);

        private readonly TcpClient transport;
        private readonly NetworkStream stream;
        private readonly object writeGate = new object();
        private readonly object pendingGate = new object();
        private readonly Dictionary<int, TaskCompletionSource<JsonObject>> pending =
            new Dictionary<int, TaskCompletionSource<JsonObject>>();
        private readonly object eventGate = new object();
        private readonly List<JsonObject> events = new List<JsonObject>();
        private readonly Task reader;
        private int sequence;
        private bool disposed;

        private DapClient(IPAddress address, int port)
        {
            transport = new TcpClient();
            try
            {
                transport.NoDelay = true;
                transport.Connect(address, port);
                stream = transport.GetStream();
            }
            catch (Exception)
            {
                transport.Dispose();
                throw;
            }

            reader = Task.Run(ReadLoopAsync);
        }

        /// <summary>
        /// Connects to a debug adapter on the loopback interface. The connection is
        /// retried until the timeout expires, because the adapter opens its listener
        /// only once the debugged process starts.
        /// </summary>
        public static DapClient Connect(int port, TimeSpan? timeout = null)
        {
            return Connect(IPAddress.Loopback, port, timeout);
        }

        public static DapClient Connect(IPAddress address, int port, TimeSpan? timeout = null)
        {
            ArgumentNullException.ThrowIfNull(address);
            DebugAgent.ValidateLoopbackEndpoint(address);
            TimeSpan budget = timeout ?? DefaultTimeout;
            Stopwatch watch = Stopwatch.StartNew();
            while (true)
            {
                try
                {
                    return new DapClient(address, port);
                }
                catch (SocketException)
                {
                    if (watch.Elapsed >= budget)
                    {
                        throw new TimeoutException(
                            "no debug adapter accepted a connection on " + address + ":" +
                            port.ToString(CultureInfo.InvariantCulture) + " within " +
                            budget.TotalSeconds.ToString("0.###", CultureInfo.InvariantCulture) + " s");
                    }

                    Thread.Sleep(ConnectRetryInterval);
                }
            }
        }

        public JsonObject Request(string command, JsonObject? arguments = null, TimeSpan? timeout = null)
        {
            ArgumentNullException.ThrowIfNull(command);
            lock (pendingGate)
            {
                ObjectDisposedException.ThrowIf(disposed, this);
            }

            int requestSequence = Interlocked.Increment(ref sequence);
            TaskCompletionSource<JsonObject> completion =
                new TaskCompletionSource<JsonObject>(TaskCreationOptions.RunContinuationsAsynchronously);
            lock (pendingGate)
            {
                pending[requestSequence] = completion;
            }

            JsonObject request = new JsonObject
            {
                ["seq"] = requestSequence,
                ["type"] = "request",
                ["command"] = command,
            };
            if (arguments != null)
            {
                request["arguments"] = arguments.DeepClone();
            }

            try
            {
                Send(request);
            }
            catch (Exception)
            {
                lock (pendingGate)
                {
                    pending.Remove(requestSequence);
                }

                throw;
            }

            TimeSpan budget = timeout ?? DefaultTimeout;
            bool answered;
            try
            {
                answered = completion.Task.Wait(budget);
            }
            catch (AggregateException error)
            {
                throw error.InnerException ?? error;
            }

            if (!answered)
            {
                lock (pendingGate)
                {
                    pending.Remove(requestSequence);
                }

                throw new TimeoutException(
                    "the debug adapter did not answer '" + command + "' within " +
                    budget.TotalSeconds.ToString("0.###", CultureInfo.InvariantCulture) + " s");
            }

            return completion.Task.GetAwaiter().GetResult();
        }

        public JsonObject Initialize()
        {
            return Request("initialize");
        }

        public JsonObject Attach()
        {
            return Request("attach");
        }

        public JsonObject SetBreakpoints(string path, IReadOnlyList<int> lines, IReadOnlyList<string>? conditions = null)
        {
            ArgumentNullException.ThrowIfNull(path);
            ArgumentNullException.ThrowIfNull(lines);
            JsonArray breakpoints = new JsonArray();
            for (int index = 0; index < lines.Count; index++)
            {
                JsonObject entry = new JsonObject
                {
                    ["line"] = lines[index],
                };
                if (conditions != null && index < conditions.Count && !string.IsNullOrEmpty(conditions[index]))
                {
                    entry["condition"] = conditions[index];
                }

                breakpoints.Add(entry);
            }

            return Request("setBreakpoints", new JsonObject
            {
                ["source"] = new JsonObject { ["path"] = path },
                ["breakpoints"] = breakpoints,
            });
        }

        public JsonObject ConfigurationDone()
        {
            return Request("configurationDone");
        }

        public JsonObject Threads()
        {
            return Request("threads");
        }

        public JsonObject StackTrace()
        {
            return Request("stackTrace");
        }

        public JsonObject Scopes()
        {
            return Request("scopes");
        }

        public JsonObject Variables(int variablesReference)
        {
            return Request("variables", new JsonObject
            {
                ["variablesReference"] = variablesReference,
            });
        }

        public JsonObject Evaluate(string expression)
        {
            ArgumentNullException.ThrowIfNull(expression);
            return Request("evaluate", new JsonObject
            {
                ["expression"] = expression,
            });
        }

        public JsonObject Continue()
        {
            return Request("continue");
        }

        public JsonObject Next()
        {
            return Request("next");
        }

        public JsonObject StepIn()
        {
            return Request("stepIn");
        }

        public JsonObject StepOut()
        {
            return Request("stepOut");
        }

        public JsonObject Disconnect()
        {
            return Request("disconnect");
        }

        /// <summary>
        /// Waits for one event with the given name. Returns null when the event does
        /// not arrive before the timeout expires.
        /// </summary>
        public JsonObject? WaitForEvent(string eventName, TimeSpan timeout)
        {
            ArgumentNullException.ThrowIfNull(eventName);
            Stopwatch watch = Stopwatch.StartNew();
            lock (eventGate)
            {
                while (true)
                {
                    for (int index = 0; index < events.Count; index++)
                    {
                        if (ReadString(events[index]["event"]) == eventName)
                        {
                            JsonObject found = events[index];
                            events.RemoveAt(index);
                            return found;
                        }
                    }

                    TimeSpan remaining = timeout - watch.Elapsed;
                    if (remaining <= TimeSpan.Zero)
                    {
                        return null;
                    }

                    Monitor.Wait(eventGate, remaining);
                }
            }
        }

        public JsonObject? WaitForStopped(TimeSpan timeout)
        {
            return WaitForEvent(StoppedEvent, timeout);
        }

        public void Dispose()
        {
            lock (pendingGate)
            {
                if (disposed)
                {
                    return;
                }

                disposed = true;
            }

            try
            {
                stream.Dispose();
            }
            catch (ObjectDisposedException)
            {
            }

            try
            {
                transport.Close();
            }
            catch (SocketException)
            {
            }

            try
            {
                reader.Wait(TimeSpan.FromMilliseconds(200));
            }
            catch (AggregateException)
            {
            }

            FailPending(new IOException("the debug adapter connection closed"));
        }

        private void Send(JsonObject message)
        {
            byte[] payload = Encoding.UTF8.GetBytes(message.ToJsonString());
            byte[] header = Encoding.ASCII.GetBytes(
                "Content-Length: " + payload.Length.ToString(CultureInfo.InvariantCulture) + "\r\n\r\n");
            lock (writeGate)
            {
                stream.Write(header, 0, header.Length);
                stream.Write(payload, 0, payload.Length);
                stream.Flush();
            }
        }

        private async Task ReadLoopAsync()
        {
            try
            {
                while (true)
                {
                    string? message = await ReadMessageAsync(stream).ConfigureAwait(false);
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

                    if (node is JsonObject parsed)
                    {
                        Dispatch(parsed);
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
                FailPending(new IOException("the debug adapter connection closed"));
            }
        }

        private void Dispatch(JsonObject message)
        {
            if (ReadString(message["type"]) == "response")
            {
                int requestSequence = ReadInt(message["request_seq"]);
                TaskCompletionSource<JsonObject>? completion = null;
                lock (pendingGate)
                {
                    if (pending.TryGetValue(requestSequence, out completion))
                    {
                        pending.Remove(requestSequence);
                    }
                }

                completion?.TrySetResult(message);
                return;
            }

            if (ReadString(message["type"]) == "event")
            {
                lock (eventGate)
                {
                    events.Add(message);
                    Monitor.PulseAll(eventGate);
                }
            }
        }

        private void FailPending(Exception error)
        {
            List<TaskCompletionSource<JsonObject>> waiting;
            lock (pendingGate)
            {
                waiting = new List<TaskCompletionSource<JsonObject>>(pending.Values);
                pending.Clear();
            }

            foreach (TaskCompletionSource<JsonObject> completion in waiting)
            {
                completion.TrySetException(error);
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
    }
}
