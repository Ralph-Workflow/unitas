using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class TimeSemanticsProbeBehaviour : MonoBehaviour
    {
        public readonly List<string> Trace = new List<string>();
    }

    public static class TimeSemanticsTests
    {
        public static void Run()
        {
            TestTimeScaleZeroHoldsScaledWaitButNotRealtimeWait();
            TestNestedCoroutineResumesCallerAfterChildCompletes();
            TestStopCoroutineAndDestroyCancelPendingWaits();
            TestMaximumDeltaTimeClampsNativeFixedStepCatchUp();
        }

        private static void TestTimeScaleZeroHoldsScaledWaitButNotRealtimeWait()
        {
            GameObject host = new GameObject("TimeSemanticsPauseHost");
            TimeSemanticsProbeBehaviour owner = host.AddComponent<TimeSemanticsProbeBehaviour>();
            float savedScale = Time.timeScale;
            try
            {
                List<string> trace = owner.Trace;
                Time.timeScale = 0f;
                owner.StartCoroutine(ScaledWaitRoutine(trace, "scaled", 0.2f));
                owner.StartCoroutine(RealtimeWaitRoutine(trace, "realtime", 0.2f));

                Check.Equal(
                    "timeScale 0 runs both coroutines up to their first yield",
                    "scaled:start, realtime:start",
                    string.Join(", ", trace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "timeScale 0 holds WaitForSeconds and WaitForSecondsRealtime after 0.1s of wall time",
                    "scaled:start, realtime:start",
                    string.Join(", ", trace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "timeScale 0 releases WaitForSecondsRealtime after 0.2s of wall time while WaitForSeconds stays held",
                    "scaled:start, realtime:start, realtime:done",
                    string.Join(", ", trace));

                for (int frame = 0; frame < 3; frame++)
                {
                    Time.Advance(0.1);
                    CoroutineScheduler.PumpFrame();
                }

                Check.Equal(
                    "timeScale 0 keeps WaitForSeconds held after 0.5s of wall time",
                    "scaled:start, realtime:start, realtime:done",
                    string.Join(", ", trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                UnityEngine.Object.Destroy(host);
                Time.timeScale = savedScale;
            }
        }

        private static void TestNestedCoroutineResumesCallerAfterChildCompletes()
        {
            GameObject host = new GameObject("TimeSemanticsNestedHost");
            TimeSemanticsProbeBehaviour owner = host.AddComponent<TimeSemanticsProbeBehaviour>();
            try
            {
                List<string> handleTrace = owner.Trace;
                owner.StartCoroutine(OuterCoroutineHandleRoutine(owner, handleTrace));
                Check.Equal(
                    "A nested StartCoroutine runs the child up to its first yield before the caller suspends",
                    "outer:start, inner:start",
                    string.Join(", ", handleTrace));

                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "The caller resumes on the frame the nested coroutine completes",
                    "outer:start, inner:start, inner:end, outer:resumed",
                    string.Join(", ", handleTrace));

                List<string> enumeratorTrace = new List<string>();
                owner.StartCoroutine(OuterEnumeratorRoutine(enumeratorTrace));
                Check.Equal(
                    "A nested IEnumerator runs the child up to its first yield before the caller suspends",
                    "rawOuter:start, inner:start",
                    string.Join(", ", enumeratorTrace));

                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "The caller resumes after a nested IEnumerator completes",
                    "rawOuter:start, inner:start, inner:end, rawOuter:resumed",
                    string.Join(", ", enumeratorTrace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                UnityEngine.Object.Destroy(host);
            }
        }

        private static void TestStopCoroutineAndDestroyCancelPendingWaits()
        {
            GameObject stoppedHost = new GameObject("TimeSemanticsStopHost");
            TimeSemanticsProbeBehaviour stoppedOwner = stoppedHost.AddComponent<TimeSemanticsProbeBehaviour>();
            GameObject destroyedHost = new GameObject("TimeSemanticsDestroyHost");
            TimeSemanticsProbeBehaviour destroyedOwner = destroyedHost.AddComponent<TimeSemanticsProbeBehaviour>();
            float savedScale = Time.timeScale;
            try
            {
                Time.timeScale = 1f;
                Coroutine stopped = stoppedOwner.StartCoroutine(ScaledWaitRoutine(stoppedOwner.Trace, "stopped", 0.1f));
                Coroutine destroyed = destroyedOwner.StartCoroutine(ScaledWaitRoutine(destroyedOwner.Trace, "destroyed", 0.1f));
                Check.That("StopCoroutine target handle is non-null", stopped != null);
                Check.That("Destroyed-owner coroutine handle is non-null", destroyed != null);

                stoppedOwner.StopCoroutine(stopped!);
                CoroutineScheduler.OwnerDestroyed(destroyedOwner);

                Time.Advance(0.2);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "StopCoroutine cancels the pending WaitForSeconds wait",
                    "stopped:start",
                    string.Join(", ", stoppedOwner.Trace));
                Check.Equal(
                    "Destroying the owner cancels the pending WaitForSeconds wait",
                    "destroyed:start",
                    string.Join(", ", destroyedOwner.Trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(stoppedOwner);
                CoroutineScheduler.OwnerDestroyed(destroyedOwner);
                UnityEngine.Object.Destroy(stoppedHost);
                UnityEngine.Object.Destroy(destroyedHost);
                Time.timeScale = savedScale;
            }
        }

        private static void TestMaximumDeltaTimeClampsNativeFixedStepCatchUp()
        {
            string? libraryPath = Environment.GetEnvironmentVariable("UNITAS_RUNTIME_LIBRARY");
            string exportName = "unitas_runtime_set_maximum_delta_time";
            if (!RuntimeLibraryExports(libraryPath, exportName))
            {
                Console.WriteLine(
                    "DEFERRED: MaximumDeltaTime clamps native fixed-step catch-up — the runtime library at "
                    + (libraryPath ?? "<default library search path>")
                    + " does not export "
                    + exportName
                    + "; rebuild it with: cargo build -p unitas-runtime");
                return;
            }

            string scenePath = ExecutionOrderDispatchTests.ResolveFixture(ExecutionOrderDispatchTests.FixtureRelativePath);
            Check.That("Execution-order scene file exists for the maximumDeltaTime clamp test", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            float savedMaximumDeltaTime = Time.maximumDeltaTime;
            List<string> trace = new List<string>();
            try
            {
                Time.timeScale = 1f;
                Time.maximumDeltaTime = 0.1f;
                Check.Equal("Time.maximumDeltaTime reads back the configured clamp", 0.1f, Time.maximumDeltaTime);

                ScriptGlueRegistry.Clear();
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidA, () => new ExecutionOrderRecorderScript("A", trace));
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidB, () => new ExecutionOrderRecorderScript("B", trace));
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidC, () => new ExecutionOrderRecorderScript("C", trace));

                using (BridgeHost host = new BridgeHost())
                {
                    Check.Equal("Time.maximumDeltaTime reads the clamp back from the runtime", 0.1f, Time.maximumDeltaTime);
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    trace.Clear();
                    host.Tick(10.0);
                    host.Shutdown();
                }

                int fixedCount = trace.FindAll(entry => entry.StartsWith("F:", StringComparison.Ordinal)).Count;
                Check.Equal(
                    "A 10s frame runs only the 5 fixed steps that maximumDeltaTime 0.1s allows (15 FixedUpdate callbacks across 3 scripts), not the 500 steps an unclamped frame would run (1500 callbacks)",
                    15,
                    fixedCount);
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.maximumDeltaTime = savedMaximumDeltaTime;
                Time.timeScale = savedScale;
            }
        }

        private static bool RuntimeLibraryExports(string? libraryPath, string exportName)
        {
            if (!string.IsNullOrEmpty(libraryPath) && NativeLibrary.TryLoad(libraryPath, out IntPtr loaded))
            {
                return NativeLibrary.TryGetExport(loaded, exportName, out _);
            }

            return NativeLibrary.TryLoad("unitas_runtime", out IntPtr fallback)
                && NativeLibrary.TryGetExport(fallback, exportName, out _);
        }

        private static IEnumerator ScaledWaitRoutine(List<string> trace, string label, float seconds)
        {
            trace.Add(label + ":start");
            yield return new WaitForSeconds(seconds);
            trace.Add(label + ":done");
        }

        private static IEnumerator RealtimeWaitRoutine(List<string> trace, string label, float seconds)
        {
            trace.Add(label + ":start");
            yield return new WaitForSecondsRealtime(seconds);
            trace.Add(label + ":done");
        }

        private static IEnumerator OuterCoroutineHandleRoutine(MonoBehaviour owner, List<string> trace)
        {
            trace.Add("outer:start");
            yield return owner.StartCoroutine(InnerRoutine(trace));
            trace.Add("outer:resumed");
        }

        private static IEnumerator OuterEnumeratorRoutine(List<string> trace)
        {
            trace.Add("rawOuter:start");
            yield return InnerRoutine(trace);
            trace.Add("rawOuter:resumed");
        }

        private static IEnumerator InnerRoutine(List<string> trace)
        {
            trace.Add("inner:start");
            yield return null;
            trace.Add("inner:end");
        }
    }
}
