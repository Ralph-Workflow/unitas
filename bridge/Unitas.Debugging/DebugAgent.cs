using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;

namespace Unitas.Debugging
{
    public static class DebugAgent
    {
        private static readonly object Gate = new object();
        private static readonly Dictionary<(string File, int Line), string?> Breakpoints = new Dictionary<(string File, int Line), string?>();
        private static SequencePointTable _sequencePoints = new SequencePointTable();
        private static DebugSession? _session;

        public static void RegisterSequencePoints(SequencePointTable table)
        {
            lock (Gate)
            {
                _sequencePoints = table ?? new SequencePointTable();
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
            SequencePoint? point;
            lock (Gate)
            {
                point = _sequencePoints.Find(sequencePointId);
            }
            if (point == null)
            {
                return;
            }
            string? condition;
            lock (Gate)
            {
                if (!Breakpoints.TryGetValue((point.File, point.Line), out condition))
                {
                    return;
                }
            }
            if (!string.IsNullOrEmpty(condition) && !ConditionEvaluator.Evaluate(condition, point.LocalNames, locals))
            {
                return;
            }
            DebugFrame frame = new DebugFrame(sequencePointId, point.Method, point.File, point.Line, point.LocalNames, locals);
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
            session?.Stop(frame);
        }

        internal static void Resume()
        {
            DebugSession? session;
            lock (Gate)
            {
                session = _session;
            }
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
