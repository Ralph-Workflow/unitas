using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum LogSeverity
    {
        Info,
        Warning,
        Error
    }

    public sealed class LogEntry
    {
        public LogEntry(LogSeverity severity, int frame, float time, string? message, string? callStack, Object? origin)
        {
            Severity = severity;
            Frame = frame;
            Time = time;
            Message = message ?? string.Empty;
            CallStack = callStack ?? string.Empty;
            Origin = origin;
        }

        public LogSeverity Severity { get; }

        public int Frame { get; }

        public float Time { get; }

        public string Message { get; }

        public string CallStack { get; }

        public Object? Origin { get; }
    }

    public static class LogRecorder
    {
        private static readonly List<LogEntry> Recorded = new List<LogEntry>();

        public static Object? CurrentOrigin { get; set; }

        public static IReadOnlyList<LogEntry> Entries => Recorded;

        public static void Append(LogEntry entry)
        {
            if (entry == null)
            {
                throw new ArgumentNullException(nameof(entry));
            }

            Recorded.Add(entry);
        }

        public static void Clear()
        {
            Recorded.Clear();
        }

        public static IReadOnlyList<LogEntry> OfSeverity(LogSeverity severity)
        {
            List<LogEntry> matches = new List<LogEntry>();
            foreach (LogEntry entry in Recorded)
            {
                if (entry.Severity == severity)
                {
                    matches.Add(entry);
                }
            }

            return matches;
        }

        public static IReadOnlyList<LogEntry> Search(string text)
        {
            if (text == null)
            {
                throw new ArgumentNullException(nameof(text));
            }

            List<LogEntry> matches = new List<LogEntry>();
            foreach (LogEntry entry in Recorded)
            {
                if (entry.Message.IndexOf(text, StringComparison.OrdinalIgnoreCase) >= 0)
                {
                    matches.Add(entry);
                }
            }

            return matches;
        }
    }
}
