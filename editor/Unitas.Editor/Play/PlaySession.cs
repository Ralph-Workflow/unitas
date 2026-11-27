using System;
using System.Collections.Generic;
using System.Net;
using System.Text.Json.Nodes;
using Unitas.Debugging;

namespace Unitas.Editor.Mcp
{
    public sealed class PlaySession : IDisposable
    {
        public const string StatePlaying = "playing";
        public const string StatePaused = "paused";
        public const string StateStopped = "stopped";
        public const string StateSessionEnded = "session-ended";

        // The worker opens its debug adapter at the first instrumented hit, so a
        // connection attempt that fails right away means the adapter is not up yet.
        private static readonly TimeSpan DebugConnectProbe = TimeSpan.FromMilliseconds(250);

        private readonly IPlayWorker worker;
        private readonly object sessionLock = new object();
        private readonly object debugLock = new object();
        private readonly Dictionary<string, PendingBreakpoint> pendingBreakpoints =
            new Dictionary<string, PendingBreakpoint>(StringComparer.Ordinal);
        private DapClient? debugClient;
        private bool debugConfigured;

        public PlaySession(
            string playSessionId,
            string ownerSessionId,
            string operationId,
            IPlayWorker worker,
            int? debugPort = null)
        {
            ArgumentNullException.ThrowIfNull(playSessionId);
            ArgumentNullException.ThrowIfNull(ownerSessionId);
            ArgumentNullException.ThrowIfNull(operationId);
            ArgumentNullException.ThrowIfNull(worker);

            PlaySessionId = playSessionId;
            OwnerSessionId = ownerSessionId;
            OperationId = operationId;
            this.worker = worker;
            DebugPort = debugPort;
            State = StatePaused;
            DeltaSeconds = 1.0 / 60.0;
        }

        public string PlaySessionId { get; }

        public string OwnerSessionId { get; }

        public string OperationId { get; }

        public IPlayWorker Worker => worker;

        public string State { get; private set; }

        public long Frames { get; private set; }

        public double DeltaSeconds { get; private set; }

        public ulong RandomSeed { get; private set; }

        public bool Crashed { get; private set; }

        public int? ExitCode => worker.ExitCode;

        /// <summary>
        /// The loopback port the worker exposes its debug adapter on, or null when
        /// the session runs without debugging.
        /// </summary>
        public int? DebugPort { get; }

        public bool DebugEnabled => DebugPort.HasValue;

        public JsonObject Play()
        {
            return SendAndUpdateState(new JsonObject
            {
                ["cmd"] = "play",
            });
        }

        public JsonObject Pause()
        {
            return SendAndUpdateState(new JsonObject
            {
                ["cmd"] = "pause",
            });
        }

        public JsonObject Step(long frames)
        {
            if (frames <= 0)
            {
                throw new McpError.InvalidParams("frames must be a positive integer");
            }

            return SendAndUpdateState(new JsonObject
            {
                ["cmd"] = "step",
                ["frames"] = frames,
            });
        }

        public JsonObject InjectInput(JsonArray events)
        {
            ArgumentNullException.ThrowIfNull(events);
            return SendAndUpdateState(new JsonObject
            {
                ["cmd"] = "inject_input",
                ["events"] = events.DeepClone(),
            });
        }

        public JsonObject QueryState()
        {
            lock (sessionLock)
            {
                if (State == StateStopped)
                {
                    return BuildStatePayloadLocked();
                }

                if (State == StateSessionEnded || worker.HasExited)
                {
                    MarkSessionEndedLocked();
                    return BuildStatePayloadLocked();
                }

                try
                {
                    JsonObject response = worker.SendCommand(new JsonObject
                    {
                        ["cmd"] = "state",
                    });
                    ApplyWorkerStateLocked(response);
                }
                catch (McpError)
                {
                    if (worker.HasExited)
                    {
                        MarkSessionEndedLocked();
                    }
                    else
                    {
                        throw;
                    }
                }

                return BuildStatePayloadLocked();
            }
        }

        public JsonObject Stop()
        {
            lock (sessionLock)
            {
                if (State == StateSessionEnded || worker.HasExited)
                {
                    MarkSessionEndedLocked();
                    worker.Dispose();
                    return BuildStatePayloadLocked();
                }

                if (State != StateStopped)
                {
                    try
                    {
                        JsonObject response = worker.SendCommand(new JsonObject
                        {
                            ["cmd"] = "stop",
                        });
                        ApplyWorkerStateLocked(response);
                    }
                    catch (McpError)
                    {
                    }

                    State = StateStopped;
                }

                worker.Dispose();
                DisposeDebugClient();
                return BuildStatePayloadLocked();
            }
        }

        public JsonObject Capture(int width, int height, bool idMap = true)
        {
            if (width <= 0 || height <= 0)
            {
                throw new McpError.InvalidParams("capture width and height must be positive integers");
            }

            lock (sessionLock)
            {
                if (State == StateSessionEnded || worker.HasExited)
                {
                    MarkSessionEndedLocked();
                    throw new McpError.InvalidRequest(
                        "play session " + PlaySessionId + " ended: worker process exited");
                }

                if (State == StateStopped)
                {
                    throw new McpError.InvalidRequest(
                        "play session " + PlaySessionId + " is stopped");
                }

                try
                {
                    return worker.SendCommand(new JsonObject
                    {
                        ["cmd"] = "capture",
                        ["width"] = width,
                        ["height"] = height,
                        ["id_map"] = idMap,
                    });
                }
                catch (McpError)
                {
                    if (worker.HasExited)
                    {
                        MarkSessionEndedLocked();
                    }

                    throw;
                }
            }
        }

        public void Dispose()
        {
            worker.Dispose();
            DisposeDebugClient();
        }

        /// <summary>
        /// Sets breakpoints in the debugged worker. The response is null when the
        /// worker has not opened its debug adapter yet; the breakpoints are then
        /// applied as soon as the adapter answers.
        /// </summary>
        public JsonObject? SetBreakpoints(string path, IReadOnlyList<int> lines, IReadOnlyList<string>? conditions)
        {
            ArgumentNullException.ThrowIfNull(path);
            ArgumentNullException.ThrowIfNull(lines);
            lock (debugLock)
            {
                if (debugClient != null)
                {
                    return debugClient.SetBreakpoints(path, lines, conditions);
                }

                pendingBreakpoints[path] = new PendingBreakpoint(path, lines, conditions);
                if (TryConnectDebugClientLocked(DebugConnectProbe) == null)
                {
                    return null;
                }

                Dictionary<string, JsonObject> applied = FlushPendingBreakpointsLocked();
                return applied.TryGetValue(path, out JsonObject? response) ? response : null;
            }
        }

        /// <summary>
        /// The debug adapter client of this session, connected on first use.
        /// </summary>
        public DapClient EnsureDebugClient(TimeSpan timeout)
        {
            lock (debugLock)
            {
                if (debugClient != null)
                {
                    return debugClient;
                }

                if (!DebugPort.HasValue)
                {
                    throw new McpError.InvalidRequest(
                        "play session " + PlaySessionId + " was not started with debugging enabled");
                }

                debugClient = ConnectDebugClientLocked(timeout);
                FlushPendingBreakpointsLocked();
                return debugClient;
            }
        }

        private JsonObject SendAndUpdateState(JsonObject command)
        {
            lock (sessionLock)
            {
                if (State == StateSessionEnded || worker.HasExited)
                {
                    MarkSessionEndedLocked();
                    throw new McpError.InvalidRequest(
                        "play session " + PlaySessionId + " ended: worker process exited");
                }

                try
                {
                    JsonObject response = worker.SendCommand(command);
                    ApplyWorkerStateLocked(response);
                    return BuildStatePayloadLocked();
                }
                catch (McpError)
                {
                    if (worker.HasExited)
                    {
                        MarkSessionEndedLocked();
                    }

                    throw;
                }
            }
        }

        private void ApplyWorkerStateLocked(JsonObject response)
        {
            if (response["state"] is JsonValue stateValue
                && stateValue.TryGetValue(out string? parsedState)
                && !string.IsNullOrEmpty(parsedState))
            {
                State = parsedState!;
            }

            if (response["frames"] is JsonValue framesValue
                && framesValue.TryGetValue(out long parsedFrames))
            {
                Frames = parsedFrames;
            }

            if (response["delta_seconds"] is JsonValue deltaValue
                && deltaValue.TryGetValue(out double parsedDelta))
            {
                DeltaSeconds = parsedDelta;
            }

            if (response["random_seed"] is JsonValue seedValue)
            {
                if (seedValue.TryGetValue(out ulong parsedUlong))
                {
                    RandomSeed = parsedUlong;
                }
                else if (seedValue.TryGetValue(out long parsedLong) && parsedLong >= 0)
                {
                    RandomSeed = (ulong)parsedLong;
                }
            }
        }

        private void MarkSessionEndedLocked()
        {
            State = StateSessionEnded;
            Crashed = true;
        }

        private JsonObject BuildStatePayloadLocked()
        {
            JsonObject payload = new JsonObject
            {
                ["playSessionId"] = PlaySessionId,
                ["ownerSessionId"] = OwnerSessionId,
                ["operationId"] = OperationId,
                ["state"] = State,
                ["frames"] = Frames,
                ["delta_seconds"] = DeltaSeconds,
                ["random_seed"] = RandomSeed,
                ["crashed"] = Crashed,
                ["debug"] = DebugEnabled,
            };

            if (DebugPort.HasValue)
            {
                payload["debugPort"] = DebugPort.Value;
            }

            if (worker.ExitCode.HasValue)
            {
                payload["exitCode"] = worker.ExitCode.Value;
            }

            return payload;
        }

        private DapClient? TryConnectDebugClientLocked(TimeSpan timeout)
        {
            if (debugClient != null)
            {
                return debugClient;
            }

            if (!DebugPort.HasValue)
            {
                return null;
            }

            try
            {
                debugClient = ConnectDebugClientLocked(timeout);
            }
            catch (TimeoutException)
            {
                return null;
            }

            return debugClient;
        }

        private DapClient ConnectDebugClientLocked(TimeSpan timeout)
        {
            DapClient client = DapClient.Connect(IPAddress.Loopback, DebugPort!.Value, timeout);
            try
            {
                client.Initialize();
                client.Attach();
            }
            catch (Exception error)
            {
                client.Dispose();
                throw new McpError.Io(error);
            }

            return client;
        }

        private Dictionary<string, JsonObject> FlushPendingBreakpointsLocked()
        {
            Dictionary<string, JsonObject> applied = new Dictionary<string, JsonObject>(StringComparer.Ordinal);
            if (debugClient == null)
            {
                return applied;
            }

            foreach (KeyValuePair<string, PendingBreakpoint> entry in pendingBreakpoints)
            {
                applied[entry.Key] = debugClient.SetBreakpoints(
                    entry.Value.Path,
                    entry.Value.Lines,
                    entry.Value.Conditions);
            }

            pendingBreakpoints.Clear();
            EnsureConfiguredLocked();
            return applied;
        }

        private void EnsureConfiguredLocked()
        {
            if (debugConfigured || debugClient == null)
            {
                return;
            }

            debugClient.ConfigurationDone();
            debugConfigured = true;
        }

        private void DisposeDebugClient()
        {
            lock (debugLock)
            {
                debugClient?.Dispose();
                debugClient = null;
            }
        }

        private sealed class PendingBreakpoint
        {
            public PendingBreakpoint(string path, IReadOnlyList<int> lines, IReadOnlyList<string>? conditions)
            {
                Path = path;
                Lines = lines;
                Conditions = conditions;
            }

            public string Path { get; }

            public IReadOnlyList<int> Lines { get; }

            public IReadOnlyList<string>? Conditions { get; }
        }
    }

    public sealed class PlaySessionManager : IDisposable
    {
        private readonly PlayWorkerFactoryWithDebugPort workerFactory;
        private readonly Dictionary<string, PlaySession> sessions =
            new Dictionary<string, PlaySession>(StringComparer.Ordinal);
        private readonly Dictionary<string, string> activeByOwner =
            new Dictionary<string, string>(StringComparer.Ordinal);
        private readonly object managerLock = new object();
        private long nextPlaySessionId;

        public PlaySessionManager()
            : this((bundleDir, dapPort) => WorkerProcess.StartDefault(bundleDir, dapPort))
        {
        }

        public PlaySessionManager(PlayWorkerFactory workerFactory)
            : this((bundleDir, dapPort) => workerFactory(bundleDir))
        {
        }

        public PlaySessionManager(PlayWorkerFactoryWithDebugPort workerFactory)
        {
            ArgumentNullException.ThrowIfNull(workerFactory);
            this.workerFactory = workerFactory;
        }

        public PlayWorkerFactory WorkerFactory => bundleDir => workerFactory(bundleDir, null);

        public PlaySession StartSession(
            string ownerSessionId,
            string operationId,
            string? bundleDirectory = null,
            bool debug = false)
        {
            ArgumentNullException.ThrowIfNull(ownerSessionId);
            ArgumentNullException.ThrowIfNull(operationId);

            int? debugPort = debug ? WorkerProcess.PickFreeLoopbackPort() : null;
            IPlayWorker worker = workerFactory(bundleDirectory, debugPort);
            PlaySession session;
            lock (managerLock)
            {
                nextPlaySessionId++;
                string playSessionId = "play-session-" + nextPlaySessionId;
                session = new PlaySession(playSessionId, ownerSessionId, operationId, worker, debugPort);
                sessions[playSessionId] = session;
                activeByOwner[ownerSessionId] = playSessionId;
            }

            try
            {
                session.Play();
                return session;
            }
            catch
            {
                session.Dispose();
                lock (managerLock)
                {
                    sessions.Remove(session.PlaySessionId);
                    if (activeByOwner.TryGetValue(ownerSessionId, out string? activeId)
                        && string.Equals(activeId, session.PlaySessionId, StringComparison.Ordinal))
                    {
                        activeByOwner.Remove(ownerSessionId);
                    }
                }

                throw;
            }
        }

        public PlaySession GetSession(string? playSessionId, string? callerSessionId)
        {
            lock (managerLock)
            {
                string? targetId = playSessionId;
                if (string.IsNullOrWhiteSpace(targetId))
                {
                    if (!string.IsNullOrWhiteSpace(callerSessionId)
                        && activeByOwner.TryGetValue(callerSessionId!, out string? activeId))
                    {
                        targetId = activeId;
                    }
                }

                if (string.IsNullOrWhiteSpace(targetId))
                {
                    throw new McpError.InvalidParams("no active playSessionId for session");
                }

                if (!sessions.TryGetValue(targetId!, out PlaySession? session))
                {
                    throw new McpError.InvalidParams("unknown playSessionId " + targetId);
                }

                return session;
            }
        }

        public PlaySession? TryGetSession(string? playSessionId, string? callerSessionId)
        {
            lock (managerLock)
            {
                string? targetId = playSessionId;
                if (string.IsNullOrWhiteSpace(targetId))
                {
                    if (!string.IsNullOrWhiteSpace(callerSessionId)
                        && activeByOwner.TryGetValue(callerSessionId!, out string? activeId))
                    {
                        targetId = activeId;
                    }
                }

                if (string.IsNullOrWhiteSpace(targetId))
                {
                    return null;
                }

                sessions.TryGetValue(targetId!, out PlaySession? session);
                return session;
            }
        }

        public IReadOnlyList<PlaySession> AllSessions()
        {
            lock (managerLock)
            {
                return new List<PlaySession>(sessions.Values);
            }
        }

        public void Dispose()
        {
            List<PlaySession> toDispose;
            lock (managerLock)
            {
                toDispose = new List<PlaySession>(sessions.Values);
                sessions.Clear();
                activeByOwner.Clear();
            }

            foreach (PlaySession session in toDispose)
            {
                session.Dispose();
            }
        }
    }
}
