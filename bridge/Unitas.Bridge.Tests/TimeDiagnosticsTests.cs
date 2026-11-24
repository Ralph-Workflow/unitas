using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class TimeScaleScript : MonoBehaviour
    {
        private int _updateCount;

        public bool StartScaledWait { get; set; }

        public float WaitSeconds { get; set; } = 0.2f;

        public int FixedUpdateCount { get; private set; }

        public int UpdateCount => _updateCount;

        public float ObservedDeltaTime { get; private set; }

        public int WaitCompletedAtUpdate { get; private set; } = -1;

        public int CollisionExitCount { get; private set; }

        public override void Start()
        {
            if (StartScaledWait)
            {
                StartCoroutine(ScaledWaitRoutine());
            }
        }

        public override void FixedUpdate()
        {
            FixedUpdateCount++;
        }

        public override void Update()
        {
            _updateCount++;
            ObservedDeltaTime = Time.deltaTime;
        }

        public override void OnCollisionExit(Collision collision)
        {
            CollisionExitCount++;
        }

        private IEnumerator ScaledWaitRoutine()
        {
            yield return new WaitForSeconds(WaitSeconds);
            WaitCompletedAtUpdate = _updateCount;
        }
    }

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
            SetFixedDeltaTimeRejectsInvalidValues();
            TimeScaleScalesTheNativeFixedStepsAndDeltaTime();
            TimeScaleZeroFreezesTheNativeWorld();
            TimeScaleHalfMakesTheFallingBodyFallHalfAsFarPerTick();
            WaitForSecondsCompletesLaterAtHalfScale();
        }

        private struct TimeScaleProbe
        {
            public int Ticks;
            public int FixedSteps;
            public int Updates;
            public float DeltaTime;
            public float NativeTimeScale;
            public int WaitCompletedAtUpdate;
            public int CollisionExitCount;
        }

        private static TimeScaleProbe ProbeTimeScale(string scenePath, float scale, int ticks, bool startWait)
        {
            Time.timeScale = scale;
            ScriptRegistry.Clear();
            TimeScaleScript script = new TimeScaleScript { StartScaledWait = startWait };
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => script);

            float nativeTimeScale = -1f;

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < ticks; tick++)
                {
                    host.Tick(0.02);
                }

                nativeTimeScale = (float)RuntimeInterop.unitas_runtime_time_scale(host.Handle);
            }

            return new TimeScaleProbe
            {
                Ticks = ticks,
                FixedSteps = script.FixedUpdateCount,
                Updates = script.UpdateCount,
                DeltaTime = script.ObservedDeltaTime,
                NativeTimeScale = nativeTimeScale,
                WaitCompletedAtUpdate = script.WaitCompletedAtUpdate,
                CollisionExitCount = script.CollisionExitCount,
            };
        }

        private static TimeScaleProbe FallUntilCollisionExit(string scenePath, float scale, int maxTicks)
        {
            Time.timeScale = scale;
            ScriptRegistry.Clear();
            TimeScaleScript script = new TimeScaleScript();
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => script);

            int ticks = 0;
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                while (ticks < maxTicks && script.CollisionExitCount == 0)
                {
                    host.Tick(0.02);
                    ticks++;
                }
            }

            return new TimeScaleProbe
            {
                Ticks = ticks,
                FixedSteps = script.FixedUpdateCount,
                Updates = script.UpdateCount,
                DeltaTime = script.ObservedDeltaTime,
                WaitCompletedAtUpdate = script.WaitCompletedAtUpdate,
                CollisionExitCount = script.CollisionExitCount,
            };
        }

        private static void TimeScaleScalesTheNativeFixedStepsAndDeltaTime()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            Check.That("Collision scene file exists for the native time scale test", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            try
            {
                TimeScaleProbe full = ProbeTimeScale(scenePath, 1f, 21, false);
                TimeScaleProbe half = ProbeTimeScale(scenePath, 0.5f, 21, false);
                Check.Equal("native fixed steps at timeScale one", 20, full.FixedSteps);
                Check.Equal("native fixed steps at timeScale half", 10, half.FixedSteps);
                Check.Equal("scaled deltaTime at timeScale one", 0.02f, full.DeltaTime);
                Check.Equal("scaled deltaTime at timeScale half", 0.01f, half.DeltaTime);
                Check.Equal("the native runtime holds timeScale one", 1f, full.NativeTimeScale);
                Check.Equal("the native runtime holds timeScale half", 0.5f, half.NativeTimeScale);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TimeScaleZeroFreezesTheNativeWorld()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            float savedScale = Time.timeScale;
            try
            {
                TimeScaleProbe frozen = ProbeTimeScale(scenePath, 0f, 21, false);
                Check.Equal("no native fixed step runs at timeScale zero", 0, frozen.FixedSteps);
                Check.Equal("no collision resolves at timeScale zero", 0, frozen.CollisionExitCount);
                Check.Equal("update still runs at timeScale zero", 20, frozen.Updates);
                Check.Equal("deltaTime is zero at timeScale zero", 0f, frozen.DeltaTime);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TimeScaleHalfMakesTheFallingBodyFallHalfAsFarPerTick()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            float savedScale = Time.timeScale;
            try
            {
                TimeScaleProbe full = FallUntilCollisionExit(scenePath, 1f, 200);
                TimeScaleProbe half = FallUntilCollisionExit(scenePath, 0.5f, 400);
                Check.Equal("the falling body leaves the crate at full speed", 1, full.CollisionExitCount);
                Check.Equal("the falling body leaves the crate at half speed", 1, half.CollisionExitCount);
                Check.Equal(
                    "the same fall distance takes the same number of fixed steps at half speed",
                    full.FixedSteps,
                    half.FixedSteps);
                Check.That(
                    "the same fall distance takes about twice as many ticks at half speed",
                    half.Ticks >= (2 * full.Ticks) - 2);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void WaitForSecondsCompletesLaterAtHalfScale()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            float savedScale = Time.timeScale;
            try
            {
                TimeScaleProbe full = ProbeTimeScale(scenePath, 1f, 30, true);
                TimeScaleProbe half = ProbeTimeScale(scenePath, 0.5f, 30, true);
                Check.Equal(
                    "WaitForSeconds completes after ten scaled frames at timeScale one",
                    10,
                    full.WaitCompletedAtUpdate);
                Check.Equal(
                    "WaitForSeconds completes after twenty scaled frames at timeScale half",
                    20,
                    half.WaitCompletedAtUpdate);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private static void SetFixedDeltaTimeRejectsInvalidValues()
        {
            Time.SetFixedDeltaTime(0.05f);
            Check.Equal("SetFixedDeltaTime accepts positive finite value", 0.05f, Time.fixedDeltaTime);
            Check.Equal("fixedUnscaledDeltaTime matches SetFixedDeltaTime", 0.05f, Time.fixedUnscaledDeltaTime);

            Time.SetFixedDeltaTime(0f);
            Check.Equal("SetFixedDeltaTime resets to 0.02 on zero", 0.02f, Time.fixedDeltaTime);

            Time.SetFixedDeltaTime(0.05f);
            Time.SetFixedDeltaTime(-0.01f);
            Check.Equal("SetFixedDeltaTime resets to 0.02 on negative", 0.02f, Time.fixedDeltaTime);

            Time.SetFixedDeltaTime(0.05f);
            Time.SetFixedDeltaTime(float.NaN);
            Check.Equal("SetFixedDeltaTime resets to 0.02 on NaN", 0.02f, Time.fixedDeltaTime);

            Time.SetFixedDeltaTime(0.05f);
            Time.SetFixedDeltaTime(float.PositiveInfinity);
            Check.Equal("SetFixedDeltaTime resets to 0.02 on PositiveInfinity", 0.02f, Time.fixedDeltaTime);
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
