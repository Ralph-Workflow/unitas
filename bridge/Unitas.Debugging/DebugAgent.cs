using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;

namespace Unitas.Debugging
{
    public static class DebugAgent
    {
        public const string DapPortEnvironmentVariable = "UNITAS_DAP_PORT";

        private const int SelfStartTimeoutMilliseconds = 10000;

        /// <summary>
        /// How long a self-started session waits for its client to configure it.
        /// The script thread parks on <see cref="SessionConfigured"/> instead of
        /// polling, so a loaded host cannot starve the adapter reader thread and
        /// lose the breakpoint stop.
        /// </summary>
        private const int SessionConfigurationTimeoutMilliseconds = 120000;

        private static readonly object Gate = new object();
        private static readonly ManualResetEventSlim SessionConfigured = new ManualResetEventSlim(false);
        private static readonly Dictionary<(string File, int Line), string?> Breakpoints = new Dictionary<(string File, int Line), string?>();
        private static SequencePointTable _sequencePoints = new SequencePointTable();
        private static DebugSession? _session;
        private static int _callDepth;
        private static int _selfStartState;
        private static int _selfStartedPort;
        private static TcpListener? _selfStartedListener;
        private static TcpClient? _selfStartedClient;

        public static void RegisterSequencePoints(SequencePointTable table)
        {
            lock (Gate)
            {
                _sequencePoints = table ?? new SequencePointTable();
                _callDepth = 0;
            }
        }

        public static void ValidateLoopbackEndpoint(IPAddress address)
        {
            ArgumentNullException.ThrowIfNull(address);
            if (!IPAddress.IsLoopback(address))
            {
                throw new InvalidOperationException(
                    "DAP endpoint can only bind to loopback interface (" + address + " is not loopback).");
            }
        }

        public static TcpListener BeginLoopbackListener(IPAddress address, int port)
        {
            ValidateLoopbackEndpoint(address);
            TcpListener listener = new TcpListener(address, port);
            listener.Start();
            return listener;
        }

        internal static bool TryVerifyBreakpointLocation(string file, int line, out string message)
        {
            lock (Gate)
            {
                if (_sequencePoints.Count == 0 || _sequencePoints.ContainsLocation(file, line))
                {
                    message = string.Empty;
                    return true;
                }
            }
            message = "No executable sequence point at line " + line + ".";
            return false;
        }

        public static void EnterFrame()
        {
            lock (Gate)
            {
                _callDepth++;
            }
        }

        public static void LeaveFrame()
        {
            lock (Gate)
            {
                if (_callDepth > 0)
                {
                    _callDepth--;
                }
            }
        }

        public static void SetBreakpoint(string file, int line, string? condition)
        {
            lock (Gate)
            {
                Breakpoints[(file ?? string.Empty, line)] = condition;
            }
        }

        public static bool TrySetBreakpoint(string file, int line, string? condition, out string message)
        {
            if (!TryVerifyBreakpointLocation(file, line, out message))
            {
                return false;
            }
            SetBreakpoint(file, line, condition);
            return true;
        }

        public static void ClearBreakpoints(string file)
        {
            lock (Gate)
            {
                List<(string File, int Line)> stale = new List<(string File, int Line)>();
                foreach ((string File, int Line) key in Breakpoints.Keys)
                {
                    if (string.Equals(key.File, file ?? string.Empty, StringComparison.Ordinal))
                    {
                        stale.Add(key);
                    }
                }
                foreach ((string File, int Line) key in stale)
                {
                    Breakpoints.Remove(key);
                }
            }
        }

        public static DebugSession BeginSession(Stream dapTransport)
        {
            DapServer server = new DapServer(dapTransport);
            DebugSession session = new DebugSession(server);
            lock (Gate)
            {
                _session = session;
                _callDepth = 0;
            }
            server.Start();
            return session;
        }

        public static DebugSession BeginHeadlessSession(Action<DebugFrame> onStopped)
        {
            ArgumentNullException.ThrowIfNull(onStopped);
            DebugSession session = new DebugSession(onStopped);
            lock (Gate)
            {
                _session = session;
                _callDepth = 0;
            }
            return session;
        }

        public static IReadOnlyDictionary<string, string> ReadCurrentLocals()
        {
            DebugFrame? frame = CurrentFrame;
            Dictionary<string, string> rendered = new Dictionary<string, string>(StringComparer.Ordinal);
            if (frame == null)
            {
                return rendered;
            }
            for (int index = 0; index < frame.Locals.Length; index++)
            {
                string name = index < frame.LocalNames.Count
                    ? frame.LocalNames[index]
                    : "local" + index.ToString(System.Globalization.CultureInfo.InvariantCulture);
                rendered[name] = DapServer.RenderValue(frame.Locals[index]);
            }
            return rendered;
        }

        public static void Continue()
        {
            Resume();
        }

        public static void HitLocation(string file, int line, object?[] locals)
        {
            EnsureDebugSession();
            SequencePoint? point;
            int depth;
            lock (Gate)
            {
                point = _sequencePoints.FindByLocation(file, line);
                depth = _callDepth;
            }
            if (point == null)
            {
                return;
            }
            Hit(point.Id, locals, depth);
        }

        public static void Hit(int sequencePointId)
        {
            Hit(sequencePointId, Array.Empty<object?>());
        }

        public static void Hit(int sequencePointId, object?[] locals)
        {
            int depth;
            lock (Gate)
            {
                depth = _callDepth;
            }
            Hit(sequencePointId, locals, depth);
        }

        public static void Hit(int sequencePointId, object?[] locals, int depth)
        {
            EnsureDebugSession();
            SequencePoint? point;
            DebugSession? session;
            lock (Gate)
            {
                point = _sequencePoints.Find(sequencePointId);
                session = _session;
            }
            if (point == null)
            {
                return;
            }
            string stopReason = string.Empty;
            bool shouldStop = false;
            if (session != null && session.TryConsumeStepOrPause(depth, out string stepOrPauseReason))
            {
                shouldStop = true;
                stopReason = stepOrPauseReason;
            }
            else
            {
                string? condition;
                bool hasBreakpoint;
                lock (Gate)
                {
                    hasBreakpoint = Breakpoints.TryGetValue((point.File, point.Line), out condition);
                }
                if (hasBreakpoint && (string.IsNullOrEmpty(condition) || ConditionEvaluator.Evaluate(condition, point.LocalNames, locals)))
                {
                    shouldStop = true;
                    stopReason = "breakpoint";
                    session?.ClearStepAndPause();
                }
            }
            if (!shouldStop)
            {
                return;
            }
            DebugFrame frame = new DebugFrame(sequencePointId, point.Method, point.File, point.Line, point.LocalNames, locals, depth);
            session?.Stop(frame, stopReason);
        }

        public static void Hit(string file, int line, string method, IReadOnlyList<string> localNames, object?[] locals)
        {
            ArgumentNullException.ThrowIfNull(localNames);
            ArgumentNullException.ThrowIfNull(locals);
            EnsureDebugSession();
            DebugSession? session;
            SequencePoint? point;
            int depth;
            lock (Gate)
            {
                session = _session;
                depth = _callDepth;
                point = _sequencePoints.FindByLocation(file, line);
            }
            if (session == null)
            {
                return;
            }
            string stopReason = string.Empty;
            bool shouldStop = false;
            if (session.TryConsumeStepOrPause(depth, out string stepOrPauseReason))
            {
                shouldStop = true;
                stopReason = stepOrPauseReason;
            }
            else
            {
                string? condition;
                bool hasBreakpoint;
                lock (Gate)
                {
                    hasBreakpoint = Breakpoints.TryGetValue((file ?? string.Empty, line), out condition);
                }
                if (hasBreakpoint && (string.IsNullOrEmpty(condition) || ConditionEvaluator.Evaluate(condition, localNames, locals)))
                {
                    shouldStop = true;
                    stopReason = "breakpoint";
                    session.ClearStepAndPause();
                }
            }
            if (!shouldStop)
            {
                return;
            }
            DebugFrame frame = new DebugFrame(
                point?.Id ?? 0,
                point != null && !string.IsNullOrEmpty(point.Method) ? point.Method : method,
                file ?? string.Empty,
                line,
                localNames,
                locals,
                depth);
            session.Stop(frame, stopReason);
        }

        public static int? DapPort
        {
            get
            {
                int port = Volatile.Read(ref _selfStartedPort);
                return port > 0 ? port : (int?)null;
            }
        }

        public static void StopSelfStartedListener()
        {
            TcpListener? listener;
            TcpClient? client;
            DebugSession? session;
            lock (Gate)
            {
                listener = _selfStartedListener;
                client = _selfStartedClient;
                session = _session;
                _selfStartedListener = null;
                _selfStartedClient = null;
                Volatile.Write(ref _selfStartedPort, 0);
                Volatile.Write(ref _selfStartState, 0);
            }
            SessionConfigured.Set();
            session?.Dispose();
            client?.Close();
            listener?.Stop();
        }

        internal static void MarkConfigured()
        {
            Volatile.Write(ref _selfStartState, 2);
            SessionConfigured.Set();
        }

        private static void EnsureDebugSession()
        {
            int state = Volatile.Read(ref _selfStartState);
            if (state == 2)
            {
                return;
            }
            if (state == 0)
            {
                bool hasSession;
                lock (Gate)
                {
                    hasSession = _session != null;
                }
                if (hasSession)
                {
                    return;
                }
                string? portText = Environment.GetEnvironmentVariable(DapPortEnvironmentVariable);
                if (string.IsNullOrEmpty(portText))
                {
                    return;
                }
                lock (Gate)
                {
                    if (Volatile.Read(ref _selfStartState) == 0)
                    {
                        StartSelfStartedListener(portText);
                    }
                }
                state = Volatile.Read(ref _selfStartState);
            }
            if (state != 1)
            {
                return;
            }
            if (!SessionConfigured.Wait(SessionConfigurationTimeoutMilliseconds))
            {
                throw new TimeoutException(
                    "the DAP client did not configure the session within "
                    + (SessionConfigurationTimeoutMilliseconds / 1000).ToString(CultureInfo.InvariantCulture)
                    + " s");
            }
        }

        private static void StartSelfStartedListener(string portText)
        {
            if (!int.TryParse(portText, NumberStyles.Integer, CultureInfo.InvariantCulture, out int port) || port < 0 || port > 65535)
            {
                throw new InvalidOperationException(
                    DapPortEnvironmentVariable + " must be a TCP port number between 0 and 65535.");
            }
            TcpListener listener = BeginLoopbackListener(IPAddress.Loopback, port);
            _selfStartedListener = listener;
            SessionConfigured.Reset();
            Volatile.Write(ref _selfStartedPort, ((IPEndPoint)listener.LocalEndpoint).Port);
            Volatile.Write(ref _selfStartState, 1);
            _ = Task.Run(() => AcceptSelfStartedClientAsync(listener));
        }

        private static async Task AcceptSelfStartedClientAsync(TcpListener listener)
        {
            try
            {
                using CancellationTokenSource acceptTimeout = new CancellationTokenSource(
                    TimeSpan.FromMilliseconds(SelfStartTimeoutMilliseconds * 3));
                TcpClient client = await listener.AcceptTcpClientAsync(acceptTimeout.Token).ConfigureAwait(false);
                _selfStartedClient = client;
                BeginSession(client.GetStream());
            }
            catch (SocketException)
            {
            }
            catch (OperationCanceledException)
            {
            }
            catch (ObjectDisposedException)
            {
            }
            catch (InvalidOperationException)
            {
            }
        }

        internal static void MarkLaunched()
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.MarkLaunched();
        }

        internal static void MarkAttached()
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.MarkAttached();
        }

        internal static void RequestPause()
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.RequestPause();
        }

        internal static void Step(DebugStepMode mode)
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.ArmStep(mode);
        }

        internal static void Resume()
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.ClearStepAndPause();
            session?.Resume();
        }

        internal static DebugFrame? CurrentFrame
        {
            get
            {
                DebugSession? session;
                lock (Gate)
                {
                    session = _session;
                }
                return session?.CurrentFrame;
            }
        }

        internal static void ReleaseSession(DebugSession session)
        {
            lock (Gate)
            {
                if (ReferenceEquals(_session, session))
                {
                    _session = null;
                }
            }
        }
    }
}
