using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Threading;

namespace Unitas.Debugging
{
    public static class DebugAgent
    {
        private static readonly object Gate = new object();
        private static readonly Dictionary<(string File, int Line), string?> Breakpoints = new Dictionary<(string File, int Line), string?>();
        private static SequencePointTable _sequencePoints = new SequencePointTable();
        private static DebugSession? _session;
        private static int _callDepth;

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
