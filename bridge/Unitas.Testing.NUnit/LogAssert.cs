using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using NUnit.Framework;

namespace UnityEngine.TestTools
{
    public static class LogAssert
    {
        private static readonly List<Expectation> Pending = new List<Expectation>();
        private static readonly List<string> Unexpected = new List<string>();
        private static int _consumedRecorderIndex;

        public static bool ignoreFailingMessages { get; set; }

        public static void Expect(LogType type, string message)
        {
            ArgumentNullException.ThrowIfNull(message);

            Pending.Add(new Expectation(type, message, null));
        }

        public static void Expect(LogType type, Regex message)
        {
            ArgumentNullException.ThrowIfNull(message);

            Pending.Add(new Expectation(type, message.ToString(), message));
        }

        public static void NoUnexpectedReceived()
        {
            DrainFromLogRecorder();
            if (!ignoreFailingMessages && Unexpected.Count > 0)
            {
                throw new AssertionException(Unexpected[0]);
            }
        }

        public static void DrainFromLogRecorder()
        {
            IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
            while (_consumedRecorderIndex < entries.Count)
            {
                LogEntry entry = entries[_consumedRecorderIndex];
                _consumedRecorderIndex++;

                LogType logType = entry.Severity switch
                {
                    LogSeverity.Error => LogType.Error,
                    LogSeverity.Warning => LogType.Warning,
                    _ => LogType.Log
                };

                Record(logType, entry.Message);
            }
        }

        public static void Record(LogType type, string message)
        {
            string safeMessage = message ?? string.Empty;
            for (int index = 0; index < Pending.Count; index++)
            {
                Expectation expectation = Pending[index];
                if (expectation.Matches(type, safeMessage))
                {
                    Pending.RemoveAt(index);
                    return;
                }
            }

            if (!ignoreFailingMessages
                && (type == LogType.Error || type == LogType.Assert || type == LogType.Exception))
            {
                Unexpected.Add("Unexpected log message received: [" + type + "] " + safeMessage);
            }
        }

        public static IReadOnlyList<string> Verify()
        {
            DrainFromLogRecorder();
            List<string> problems = new List<string>();
            for (int index = 0; index < Pending.Count; index++)
            {
                Expectation expectation = Pending[index];
                problems.Add("Expected log not received: [" + expectation.Type + "] '" + expectation.Message + "'");
            }

            if (!ignoreFailingMessages)
            {
                problems.AddRange(Unexpected);
            }

            return problems;
        }

        public static void Reset()
        {
            Pending.Clear();
            Unexpected.Clear();
            _consumedRecorderIndex = LogRecorder.Entries.Count;
            ignoreFailingMessages = false;
        }

        private sealed class Expectation
        {
            public Expectation(LogType type, string message, Regex? pattern)
            {
                Type = type;
                Message = message;
                Pattern = pattern;
            }

            public LogType Type { get; }

            public string Message { get; }

            public Regex? Pattern { get; }

            public bool Matches(LogType type, string candidate)
            {
                if (Type != type)
                {
                    return false;
                }

                if (Pattern != null)
                {
                    return Pattern.IsMatch(candidate);
                }

                return string.Equals(candidate, Message, StringComparison.Ordinal)
                    || candidate.Contains(Message, StringComparison.Ordinal);
            }
        }
    }
}
