using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class TimeDiagnosticsTests
    {
        public static void Run()
        {
            TimeScaleDefaultsToOne();
            AdvanceSetsDeltaValues();
            TimeScaleScalesElapsedTime();
            FixedDeltaTimeConstants();
            FrameCountIncrementsPerAdvance();
            SmoothDeltaTimeConvergesTowardDelta();
            LogCaptureRetainsSeverityFrameMessageAndStack();
            LogSeverityFilterSelectsMatchingEntries();
            LogSearchMatchesMessagesCaseInsensitively();
            LogClearEmptiesTheRecorder();
            AmbientOriginIsCapturedOnTheEntry();
            LogExceptionRecordsSeverityMessageAndStack();
        }

        private static void LogExceptionRecordsSeverityMessageAndStack()
        {
            LogRecorder.Clear();
            InvalidOperationException exception = new InvalidOperationException("bridge failure");

            Debug.LogException(exception);

            Check.Equal("one exception entry recorded", 1, LogRecorder.Entries.Count);
            LogEntry entry = LogRecorder.Entries[0];
            Check.That("exception entry is an error", entry.Severity == LogSeverity.Error);
            Check.Equal("exception entry keeps the message", "bridge failure", entry.Message);
            Check.That("exception entry keeps a non-empty stack", !string.IsNullOrEmpty(entry.CallStack));
        }

        private static void TimeScaleDefaultsToOne()
        {
            Check.Equal("Time.timeScale defaults to one", 1f, Time.timeScale);
        }

        private static void AdvanceSetsDeltaValues()
        {
            float scale = Time.timeScale;
            Time.timeScale = 1f;
            Time.Advance(0.5);
            Check.Equal("unscaledDeltaTime after Advance(0.5)", 0.5f, Time.unscaledDeltaTime);
            Check.Equal("deltaTime after Advance(0.5)", 0.5f, Time.deltaTime);
            Time.timeScale = scale;
        }

        private static void TimeScaleScalesElapsedTime()
        {
            float scale = Time.timeScale;
            float unscaledBefore = Time.unscaledTime;
            float scaledBefore = Time.time;
            Time.timeScale = 2f;
            Time.Advance(0.5);
            Check.Equal("deltaTime is scaled by timeScale", 1f, Time.deltaTime);
            Check.Equal("unscaledDeltaTime ignores timeScale", 0.5f, Time.unscaledDeltaTime);
            Check.Equal("unscaledTime advances by the raw delta", unscaledBefore + 0.5f, Time.unscaledTime);
            Check.Equal("time advances by the scaled delta", scaledBefore + 1f, Time.time);
            Time.timeScale = scale;
        }

        private static void FixedDeltaTimeConstants()
        {
            Check.Equal("fixedDeltaTime is 0.02", 0.02f, Time.fixedDeltaTime);
            Check.Equal("fixedUnscaledDeltaTime is 0.02", 0.02f, Time.fixedUnscaledDeltaTime);
        }

        private static void FrameCountIncrementsPerAdvance()
        {
            int before = Time.frameCount;
            Time.Advance(0.25);
            Check.Equal("frameCount increments per Advance", before + 1, Time.frameCount);
            Time.Advance(0.25);
            Check.Equal("frameCount increments on every Advance", before + 2, Time.frameCount);
        }

        private static void SmoothDeltaTimeConvergesTowardDelta()
        {
            float scale = Time.timeScale;
            Time.timeScale = 1f;
            for (int step = 0; step < 300; step++)
            {
                Time.Advance(0.1);
            }

            Check.That(
                "smoothDeltaTime is finite",
                !float.IsNaN(Time.smoothDeltaTime) && !float.IsInfinity(Time.smoothDeltaTime));
            Check.That("smoothDeltaTime is not negative", Time.smoothDeltaTime >= 0f);
            Check.That(
                "smoothDeltaTime converges toward the frame delta",
                Math.Abs((double)Time.smoothDeltaTime - 0.1) < 0.001);
            Time.timeScale = scale;
        }

        private static void LogCaptureRetainsSeverityFrameMessageAndStack()
        {
            LogRecorder.Clear();
            int frameBefore = Time.frameCount;
            Debug.Log("info message");
            Debug.LogWarning("warning message");
            Debug.LogError("error message");

            IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
            Check.Equal("three entries recorded", 3, entries.Count);
            Check.That("first entry is Info", entries[0].Severity == LogSeverity.Info);
            Check.That("second entry is Warning", entries[1].Severity == LogSeverity.Warning);
            Check.That("third entry is Error", entries[2].Severity == LogSeverity.Error);
            Check.Equal("entry keeps the message", "info message", entries[0].Message);
            Check.Equal("entry keeps the frame at write time", frameBefore, entries[0].Frame);
            Check.Equal("entry keeps the scaled time at write time", Time.time, entries[0].Time);
            Check.That("entry keeps a non-empty stack", !string.IsNullOrEmpty(entries[0].CallStack));
            Check.That("origin is null outside a script callback", entries[0].Origin == null);
        }

        private static void LogSeverityFilterSelectsMatchingEntries()
        {
            Check.Equal("one Info entry", 1, LogRecorder.OfSeverity(LogSeverity.Info).Count);
            Check.Equal("one Warning entry", 1, LogRecorder.OfSeverity(LogSeverity.Warning).Count);
            Check.Equal("one Error entry", 1, LogRecorder.OfSeverity(LogSeverity.Error).Count);
        }

        private static void LogSearchMatchesMessagesCaseInsensitively()
        {
            IReadOnlyList<LogEntry> matches = LogRecorder.Search("WARNING");
            Check.Equal("case-insensitive search finds the warning entry", 1, matches.Count);
            Check.That("search match is the warning entry", matches[0].Severity == LogSeverity.Warning);
            Check.Equal("search excludes non-matching messages", 0, LogRecorder.Search("absent text").Count);
        }

        private static void LogClearEmptiesTheRecorder()
        {
            LogRecorder.Clear();
            Check.Equal("recorder is empty after Clear", 0, LogRecorder.Entries.Count);
        }

        private static void AmbientOriginIsCapturedOnTheEntry()
        {
            LogRecorder.Clear();
            UnityEngine.Object origin = new GameObject("OriginObject");
            LogRecorder.CurrentOrigin = origin;
            Debug.Log("origin message");
            LogRecorder.CurrentOrigin = null;

            Check.Equal("origin message recorded once", 1, LogRecorder.Entries.Count);
            Check.That(
                "entry records the ambient origin",
                ReferenceEquals(LogRecorder.Entries[0].Origin, origin));
        }
    }
}
