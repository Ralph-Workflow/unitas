using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class LoggingTestComponent : MonoBehaviour
    {
    }

    public static class LoggingTests
    {
        public static void Run()
        {
            TestLogContextOverloadsRecordOriginAndPreserveCurrentOrigin();
            TestLogFormatFamilyFormatsWithInvariantCultureAndRecordsContext();
            TestAssertAndAssertFormatTrueIsNoOpAndFalseLogsErrorWithContext();
            TestGuardValidationOnFormatAndExceptionOverloads();
        }

        private static void TestLogContextOverloadsRecordOriginAndPreserveCurrentOrigin()
        {
            GameObject go = new GameObject("LogContextObject");
            GameObject sentinel = new GameObject("LogSentinelObject");
            LoggingTestComponent component = go.AddComponent<LoggingTestComponent>();
            LogRecorder.Clear();
            try
            {
                Debug.Log("plain info");
                Debug.Log("go info", go);
                Debug.LogWarning("comp warning", component);
                Debug.LogError("go error", go);

                LogRecorder.CurrentOrigin = sentinel;
                Debug.Log("explicit null context", null);
                Debug.LogWarning("explicit go overrides current origin", go);
                Debug.LogError("plain error inherits current origin");
                Check.That(
                    "Context overloads restore prior LogRecorder.CurrentOrigin",
                    ReferenceEquals(LogRecorder.CurrentOrigin, sentinel));
                LogRecorder.CurrentOrigin = null;

                IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
                Check.Equal("Seven log entries recorded in context test", 7, entries.Count);
                if (entries.Count == 7)
                {
                    Check.Equal("Entry 0 severity Info", (int)LogSeverity.Info, (int)entries[0].Severity);
                    Check.Equal("Entry 0 message", "plain info", entries[0].Message);
                    Check.That("Entry 0 origin is null", entries[0].Origin == null);
                    Check.That("Entry 0 callStack is non-empty", !string.IsNullOrEmpty(entries[0].CallStack));

                    Check.Equal("Entry 1 severity Info", (int)LogSeverity.Info, (int)entries[1].Severity);
                    Check.Equal("Entry 1 message", "go info", entries[1].Message);
                    Check.That("Entry 1 origin is GameObject", ReferenceEquals(entries[1].Origin, go));

                    Check.Equal("Entry 2 severity Warning", (int)LogSeverity.Warning, (int)entries[2].Severity);
                    Check.Equal("Entry 2 message", "comp warning", entries[2].Message);
                    Check.That("Entry 2 origin is Component", ReferenceEquals(entries[2].Origin, component));

                    Check.Equal("Entry 3 severity Error", (int)LogSeverity.Error, (int)entries[3].Severity);
                    Check.Equal("Entry 3 message", "go error", entries[3].Message);
                    Check.That("Entry 3 origin is GameObject", ReferenceEquals(entries[3].Origin, go));

                    Check.Equal("Entry 4 severity Info", (int)LogSeverity.Info, (int)entries[4].Severity);
                    Check.Equal("Entry 4 message", "explicit null context", entries[4].Message);
                    Check.That("Entry 4 explicit null context records null origin", entries[4].Origin == null);

                    Check.Equal("Entry 5 severity Warning", (int)LogSeverity.Warning, (int)entries[5].Severity);
                    Check.Equal("Entry 5 message", "explicit go overrides current origin", entries[5].Message);
                    Check.That("Entry 5 explicit context overrides CurrentOrigin", ReferenceEquals(entries[5].Origin, go));

                    Check.Equal("Entry 6 severity Error", (int)LogSeverity.Error, (int)entries[6].Severity);
                    Check.Equal("Entry 6 message", "plain error inherits current origin", entries[6].Message);
                    Check.That("Entry 6 plain LogError uses CurrentOrigin", ReferenceEquals(entries[6].Origin, sentinel));
                }
            }
            finally
            {
                LogRecorder.CurrentOrigin = null;
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(go);
                UnityEngine.Object.Destroy(sentinel);
            }
        }

        private static void TestLogFormatFamilyFormatsWithInvariantCultureAndRecordsContext()
        {
            GameObject go = new GameObject("FormatContextObject");
            LoggingTestComponent component = go.AddComponent<LoggingTestComponent>();
            LogRecorder.Clear();
            try
            {
                Debug.LogFormat("Pos ({0:F2}, {1})", 1.5, 42);
                Debug.LogFormat(go, "Target {0} count={1}", "Alpha", 3);
                Debug.LogWarningFormat("Low health: {0:F1}%", 12.5);
                Debug.LogWarningFormat(component, "Component {0} warn {1}", "Sensor", 7);
                Debug.LogErrorFormat("Code {0}: {1}", 500, "Failure");
                Debug.LogErrorFormat(go, "Fatal on {0} at {1:F2}", "Node", 9.25);

                IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
                Check.Equal("Six log format entries recorded", 6, entries.Count);
                if (entries.Count == 6)
                {
                    Check.Equal("LogFormat severity Info", (int)LogSeverity.Info, (int)entries[0].Severity);
                    Check.Equal("LogFormat invariant message", "Pos (1.50, 42)", entries[0].Message);
                    Check.That("LogFormat without context has null origin", entries[0].Origin == null);

                    Check.Equal("LogFormat(context) severity Info", (int)LogSeverity.Info, (int)entries[1].Severity);
                    Check.Equal("LogFormat(context) message", "Target Alpha count=3", entries[1].Message);
                    Check.That("LogFormat(context) origin matches go", ReferenceEquals(entries[1].Origin, go));

                    Check.Equal("LogWarningFormat severity Warning", (int)LogSeverity.Warning, (int)entries[2].Severity);
                    Check.Equal("LogWarningFormat invariant message", "Low health: 12.5%", entries[2].Message);
                    Check.That("LogWarningFormat without context has null origin", entries[2].Origin == null);

                    Check.Equal("LogWarningFormat(context) severity Warning", (int)LogSeverity.Warning, (int)entries[3].Severity);
                    Check.Equal("LogWarningFormat(context) message", "Component Sensor warn 7", entries[3].Message);
                    Check.That("LogWarningFormat(context) origin matches component", ReferenceEquals(entries[3].Origin, component));

                    Check.Equal("LogErrorFormat severity Error", (int)LogSeverity.Error, (int)entries[4].Severity);
                    Check.Equal("LogErrorFormat message", "Code 500: Failure", entries[4].Message);
                    Check.That("LogErrorFormat without context has null origin", entries[4].Origin == null);

                    Check.Equal("LogErrorFormat(context) severity Error", (int)LogSeverity.Error, (int)entries[5].Severity);
                    Check.Equal("LogErrorFormat(context) invariant message", "Fatal on Node at 9.25", entries[5].Message);
                    Check.That("LogErrorFormat(context) origin matches go", ReferenceEquals(entries[5].Origin, go));
                }
            }
            finally
            {
                LogRecorder.CurrentOrigin = null;
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(go);
            }
        }

        private static void TestAssertAndAssertFormatTrueIsNoOpAndFalseLogsErrorWithContext()
        {
            GameObject go = new GameObject("AssertContextObject");
            LoggingTestComponent component = go.AddComponent<LoggingTestComponent>();
            LogRecorder.Clear();
            try
            {
                Debug.Assert(true);
                Debug.Assert(true, go);
                Debug.Assert(true, "should not log");
                Debug.Assert(true, "should not log with context", component);
                Debug.AssertFormat(true, "should not format {0}", 123);
                Debug.AssertFormat(true, go, "should not format with context {0}", 456);

                Check.Equal("Passing assertions record zero log entries", 0, LogRecorder.Entries.Count);

                Debug.Assert(false);
                Debug.Assert(false, go);
                Debug.Assert(false, "Custom assert failure");
                Debug.Assert(false, "Contextual assert failure", component);
                Debug.AssertFormat(false, "Expected {0} got {1:F1}", 10, 4.5);
                Debug.AssertFormat(false, go, "Node {0} index {1} invalid", "Root", -1);

                IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
                Check.Equal("Six failing assertions record six log entries", 6, entries.Count);
                if (entries.Count == 6)
                {
                    Check.Equal("Assert(false) severity Error", (int)LogSeverity.Error, (int)entries[0].Severity);
                    Check.Equal("Assert(false) default message", "Assertion failed", entries[0].Message);
                    Check.That("Assert(false) origin is null", entries[0].Origin == null);

                    Check.Equal("Assert(false, context) severity Error", (int)LogSeverity.Error, (int)entries[1].Severity);
                    Check.Equal("Assert(false, context) default message", "Assertion failed", entries[1].Message);
                    Check.That("Assert(false, context) origin matches go", ReferenceEquals(entries[1].Origin, go));

                    Check.Equal("Assert(false, message) severity Error", (int)LogSeverity.Error, (int)entries[2].Severity);
                    Check.Equal("Assert(false, message) custom message", "Custom assert failure", entries[2].Message);
                    Check.That("Assert(false, message) origin is null", entries[2].Origin == null);

                    Check.Equal("Assert(false, message, context) severity Error", (int)LogSeverity.Error, (int)entries[3].Severity);
                    Check.Equal("Assert(false, message, context) custom message", "Contextual assert failure", entries[3].Message);
                    Check.That("Assert(false, message, context) origin matches component", ReferenceEquals(entries[3].Origin, component));

                    Check.Equal("AssertFormat(false) severity Error", (int)LogSeverity.Error, (int)entries[4].Severity);
                    Check.Equal("AssertFormat(false) formatted message", "Expected 10 got 4.5", entries[4].Message);
                    Check.That("AssertFormat(false) origin is null", entries[4].Origin == null);

                    Check.Equal("AssertFormat(false, context) severity Error", (int)LogSeverity.Error, (int)entries[5].Severity);
                    Check.Equal("AssertFormat(false, context) formatted message", "Node Root index -1 invalid", entries[5].Message);
                    Check.That("AssertFormat(false, context) origin matches go", ReferenceEquals(entries[5].Origin, go));
                }
            }
            finally
            {
                LogRecorder.CurrentOrigin = null;
                LogRecorder.Clear();
                UnityEngine.Object.Destroy(go);
            }
        }

        private static void TestGuardValidationOnFormatAndExceptionOverloads()
        {
            LogRecorder.Clear();
            try
            {
                CheckThrowsArgumentNull("LogFormat null format", () => Debug.LogFormat(null!, 1));
                CheckThrowsArgumentNull("LogFormat null args", () => Debug.LogFormat("{0}", null!));
                CheckThrowsArgumentNull("LogWarningFormat null format", () => Debug.LogWarningFormat(null!, 1));
                CheckThrowsArgumentNull("LogWarningFormat null args", () => Debug.LogWarningFormat("{0}", null!));
                CheckThrowsArgumentNull("LogErrorFormat null format", () => Debug.LogErrorFormat(null!, 1));
                CheckThrowsArgumentNull("LogErrorFormat null args", () => Debug.LogErrorFormat("{0}", null!));
                CheckThrowsArgumentNull("AssertFormat(false) null format", () => Debug.AssertFormat(false, null!, 1));
                CheckThrowsArgumentNull("AssertFormat(false) null args", () => Debug.AssertFormat(false, "{0}", null!));
                CheckThrowsArgumentNull("LogException null exception", () => Debug.LogException(null!));
                Check.Equal("Failed guard calls record zero entries", 0, LogRecorder.Entries.Count);
            }
            finally
            {
                LogRecorder.Clear();
            }
        }

        private static void CheckThrowsArgumentNull(string label, Action action)
        {
            bool threw = false;
            try
            {
                action();
            }
            catch (ArgumentNullException)
            {
                threw = true;
            }

            Check.That(label + " throws ArgumentNullException", threw);
        }
    }
}
