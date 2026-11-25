using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class CoroutineAcceptanceProbeBehaviour : MonoBehaviour
    {
        public readonly List<string> Trace = new List<string>();
    }

    public static class CoroutineAcceptanceTests
    {
        public static void Run()
        {
            TestStory3ScaledWaitForSecondsVsUnscaledWaitForSecondsRealtimeAcrossPauseAndDoubleSpeed();
            TestStory3And32WaitFormsFixedUpdateEndOfFrameWaitUntilWaitWhileAndNestedStartCoroutine();
            TestStory32CoroutineSurvivesBehaviourDisabledAndStopsPermanentlyOnDeactivateOrDestroy();
            TestStory34CaptureFramerateAdvancesGameTimeDeterministically();
        }

        private static void TestStory3ScaledWaitForSecondsVsUnscaledWaitForSecondsRealtimeAcrossPauseAndDoubleSpeed()
        {
            GameObject hostGo = new GameObject("Story3_TimeScaleHost");
            CoroutineAcceptanceProbeBehaviour owner = hostGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();
            float savedScale = Time.timeScale;
            try
            {
                Time.Reset();
                Time.timeScale = 0f;

                owner.StartCoroutine(ScaledWaitRoutine(owner.Trace, "scaled_pause", 0.2f));
                owner.StartCoroutine(RealtimeWaitRoutine(owner.Trace, "realtime_pause", 0.2f));

                Check.Equal(
                    "Story 3 both coroutines execute initial synchronous step before yield",
                    "scaled_pause:start, realtime_pause:start",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=0 after 0.1s wall time neither 0.2s wait has completed",
                    "scaled_pause:start, realtime_pause:start",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=0 after 0.2s wall time WaitForSecondsRealtime completes while WaitForSeconds stays paused",
                    "scaled_pause:start, realtime_pause:start, realtime_pause:done",
                    string.Join(", ", owner.Trace));

                for (int extra = 0; extra < 4; extra++)
                {
                    Time.Advance(0.1);
                    CoroutineScheduler.PumpFrame();
                }

                Check.Equal(
                    "Story 3 at timeScale=0 after 0.6s wall time WaitForSeconds remains suspended",
                    "scaled_pause:start, realtime_pause:start, realtime_pause:done",
                    string.Join(", ", owner.Trace));

                Time.timeScale = 2f;
                Time.Advance(0.05);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=2 after 0.05s wall time (0.10s scaled) 0.2s WaitForSeconds still has 0.10s remaining",
                    "scaled_pause:start, realtime_pause:start, realtime_pause:done",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.05);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=2 after 0.10s wall time (0.20s scaled) WaitForSeconds completes",
                    "scaled_pause:start, realtime_pause:start, realtime_pause:done, scaled_pause:done",
                    string.Join(", ", owner.Trace));

                owner.Trace.Clear();
                owner.StopAllCoroutines();

                Time.timeScale = 2f;
                owner.StartCoroutine(ScaledWaitRoutine(owner.Trace, "scaled_2x", 0.2f));
                owner.StartCoroutine(RealtimeWaitRoutine(owner.Trace, "realtime_2x", 0.2f));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=2 after 0.1s wall time (0.2s scaled) WaitForSeconds finishes in 1 frame while WaitForSecondsRealtime is at 0.1s",
                    "scaled_2x:start, realtime_2x:start, scaled_2x:done",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 3 at timeScale=2 after 0.2s wall time WaitForSecondsRealtime finishes on second frame",
                    "scaled_2x:start, realtime_2x:start, scaled_2x:done, realtime_2x:done",
                    string.Join(", ", owner.Trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                UnityEngine.Object.Destroy(hostGo);
                Time.timeScale = savedScale;
            }
        }

        private static void TestStory3And32WaitFormsFixedUpdateEndOfFrameWaitUntilWaitWhileAndNestedStartCoroutine()
        {
            GameObject hostGo = new GameObject("Story32_WaitFormsHost");
            CoroutineAcceptanceProbeBehaviour owner = hostGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();
            float savedScale = Time.timeScale;
            try
            {
                Time.Reset();
                Time.timeScale = 1f;

                bool untilReady = false;
                bool whileBusy = true;

                owner.StartCoroutine(FixedUpdateWaitRoutine(owner.Trace));
                owner.StartCoroutine(EndOfFrameWaitRoutine(owner.Trace));
                owner.StartCoroutine(WaitUntilRoutine(owner.Trace, () => untilReady));
                owner.StartCoroutine(WaitWhileRoutine(owner.Trace, () => whileBusy));
                owner.StartCoroutine(NestedOuterCoroutineRoutine(owner, owner.Trace));

                Check.Equal(
                    "Story 32 all wait forms and nested StartCoroutine run synchronously to their first yield",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init",
                    string.Join(", ", owner.Trace));

                CoroutineScheduler.PumpFixed();
                Check.Equal(
                    "Story 32 PumpFixed resumes WaitForFixedUpdate",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init, fixed:resumed",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.02);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 first PumpFrame advances nested inner coroutine and resumes WaitForEndOfFrame at end of frame",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init, fixed:resumed, inner:step1, eof:resumed",
                    string.Join(", ", owner.Trace));

                Time.Advance(0.02);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 second PumpFrame completes nested inner coroutine and resumes outer coroutine in the same frame",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init, fixed:resumed, inner:step1, eof:resumed, inner:done, outer:resumed",
                    string.Join(", ", owner.Trace));

                untilReady = true;
                Time.Advance(0.02);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 WaitUntil resumes when predicate becomes true while WaitWhile stays suspended while predicate is true",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init, fixed:resumed, inner:step1, eof:resumed, inner:done, outer:resumed, until:resumed",
                    string.Join(", ", owner.Trace));

                whileBusy = false;
                Time.Advance(0.02);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 WaitWhile resumes when predicate becomes false",
                    "fixed:init, eof:init, until:init, while:init, outer:init, inner:init, fixed:resumed, inner:step1, eof:resumed, inner:done, outer:resumed, until:resumed, while:resumed",
                    string.Join(", ", owner.Trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                UnityEngine.Object.Destroy(hostGo);
                Time.timeScale = savedScale;
            }
        }

        private static void TestStory32CoroutineSurvivesBehaviourDisabledAndStopsPermanentlyOnDeactivateOrDestroy()
        {
            GameObject disableGo = new GameObject("Story32_DisableBehaviourHost");
            CoroutineAcceptanceProbeBehaviour disableOwner = disableGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();

            GameObject deactivateGo = new GameObject("Story32_DeactivateGameObjectHost");
            CoroutineAcceptanceProbeBehaviour deactivateOwner = deactivateGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();

            GameObject destroyGo = new GameObject("Story32_DestroyGameObjectHost");
            CoroutineAcceptanceProbeBehaviour destroyOwner = destroyGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();

            GameObject inactiveStartGo = new GameObject("Story32_InactiveStartHost");
            CoroutineAcceptanceProbeBehaviour inactiveStartOwner = inactiveStartGo.AddComponent<CoroutineAcceptanceProbeBehaviour>();
            inactiveStartGo.SetActive(false);

            LogRecorder.Clear();
            try
            {
                disableOwner.StartCoroutine(MultiFrameCounterRoutine(disableOwner.Trace, "surviveDisable", 3));
                Check.Equal("Story 32 coroutine runs step 0 before behaviour.enabled=false", "surviveDisable:0", string.Join(", ", disableOwner.Trace));

                disableOwner.enabled = false;
                Check.That("Story 32 behaviour.enabled is false", !disableOwner.enabled);
                Check.That("Story 32 behaviour.isActiveAndEnabled is false", !disableOwner.isActiveAndEnabled);

                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();

                Check.Equal(
                    "Story 32 coroutine survives behaviour.enabled = false and continues ticking to completion",
                    "surviveDisable:0, surviveDisable:1, surviveDisable:2, surviveDisable:3",
                    string.Join(", ", disableOwner.Trace));

                deactivateOwner.StartCoroutine(MultiFrameCounterRoutine(deactivateOwner.Trace, "stopOnDeactivate", 3));
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 coroutine ticks once while GameObject is active",
                    "stopOnDeactivate:0, stopOnDeactivate:1",
                    string.Join(", ", deactivateOwner.Trace));

                deactivateGo.SetActive(false);
                CoroutineScheduler.PumpFrame();
                deactivateGo.SetActive(true);
                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();

                Check.Equal(
                    "Story 32 gameObject.SetActive(false) permanently stops coroutine even after SetActive(true)",
                    "stopOnDeactivate:0, stopOnDeactivate:1",
                    string.Join(", ", deactivateOwner.Trace));

                destroyOwner.StartCoroutine(MultiFrameCounterRoutine(destroyOwner.Trace, "stopOnDestroy", 3));
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Story 32 coroutine ticks once before Object.Destroy(gameObject)",
                    "stopOnDestroy:0, stopOnDestroy:1",
                    string.Join(", ", destroyOwner.Trace));

                UnityEngine.Object.Destroy(destroyGo);
                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();

                Check.Equal(
                    "Story 32 Object.Destroy(gameObject) permanently stops running coroutines on its scripts",
                    "stopOnDestroy:0, stopOnDestroy:1",
                    string.Join(", ", destroyOwner.Trace));

                LogRecorder.Clear();
                inactiveStartOwner.StartCoroutine(MultiFrameCounterRoutine(inactiveStartOwner.Trace, "neverRuns", 2));
                CoroutineScheduler.PumpFrame();
                Check.Equal("Story 32 StartCoroutine on inactive GameObject does not execute coroutine body", 0, inactiveStartOwner.Trace.Count);
                IReadOnlyList<LogEntry> inactiveErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Story 32 StartCoroutine on inactive GameObject logs Unity error", 1, inactiveErrors.Count);
                if (inactiveErrors.Count == 1)
                {
                    Check.That(
                        "Story 32 inactive StartCoroutine error names Story32_InactiveStartHost",
                        inactiveErrors[0].Message.Contains("Story32_InactiveStartHost", StringComparison.Ordinal));
                }
            }
            finally
            {
                LogRecorder.Clear();
                CoroutineScheduler.OwnerDestroyed(disableOwner);
                CoroutineScheduler.OwnerDestroyed(deactivateOwner);
                CoroutineScheduler.OwnerDestroyed(destroyOwner);
                CoroutineScheduler.OwnerDestroyed(inactiveStartOwner);
                UnityEngine.Object.Destroy(disableGo);
                UnityEngine.Object.Destroy(deactivateGo);
                UnityEngine.Object.Destroy(destroyGo);
                UnityEngine.Object.Destroy(inactiveStartGo);
            }
        }

        private static void TestStory34CaptureFramerateAdvancesGameTimeDeterministically()
        {
            float savedScale = Time.timeScale;
            try
            {
                Time.Reset();
                Time.captureFramerate = 30;
                float expectedDelta = 1f / 30f;

                Time.Advance(0.25);
                Check.Equal("Story 34 captureFramerate=30 advances deltaTime by 1/30 regardless of 0.25s wall time", expectedDelta, Time.deltaTime);
                Check.Equal("Story 34 captureFramerate=30 advances time to 1/30", expectedDelta, Time.time);

                Time.Advance(0.002);
                Check.Equal("Story 34 captureFramerate=30 advances deltaTime by 1/30 regardless of 0.002s wall time", expectedDelta, Time.deltaTime);
                Check.Equal("Story 34 captureFramerate=30 advances time to 2/30", expectedDelta * 2f, Time.time);
            }
            finally
            {
                Time.Reset();
                Time.timeScale = savedScale;
            }
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

        private static IEnumerator FixedUpdateWaitRoutine(List<string> trace)
        {
            trace.Add("fixed:init");
            yield return new WaitForFixedUpdate();
            trace.Add("fixed:resumed");
        }

        private static IEnumerator EndOfFrameWaitRoutine(List<string> trace)
        {
            trace.Add("eof:init");
            yield return new WaitForEndOfFrame();
            trace.Add("eof:resumed");
        }

        private static IEnumerator WaitUntilRoutine(List<string> trace, Func<bool> predicate)
        {
            trace.Add("until:init");
            yield return new WaitUntil(predicate);
            trace.Add("until:resumed");
        }

        private static IEnumerator WaitWhileRoutine(List<string> trace, Func<bool> predicate)
        {
            trace.Add("while:init");
            yield return new WaitWhile(predicate);
            trace.Add("while:resumed");
        }

        private static IEnumerator NestedOuterCoroutineRoutine(MonoBehaviour owner, List<string> trace)
        {
            trace.Add("outer:init");
            yield return owner.StartCoroutine(NestedInnerCoroutineRoutine(trace));
            trace.Add("outer:resumed");
        }

        private static IEnumerator NestedInnerCoroutineRoutine(List<string> trace)
        {
            trace.Add("inner:init");
            yield return null;
            trace.Add("inner:step1");
            yield return null;
            trace.Add("inner:done");
        }

        private static IEnumerator MultiFrameCounterRoutine(List<string> trace, string label, int steps)
        {
            trace.Add(label + ":0");
            for (int i = 1; i <= steps; i++)
            {
                yield return null;
                trace.Add(label + ":" + i.ToString(CultureInfo.InvariantCulture));
            }
        }
    }
}
