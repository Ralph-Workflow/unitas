using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class CoroutineTests
    {
        public static void Run()
        {
            TestStartStopAndStopAllCoroutines();
            TestWaitFormsResumeOnExpectedPhases();
            TestScaledAndUnscaledWaitDurations();
            TestStopAndNonStopLifecycleRules();
            TestStartCoroutineOnInactiveGameObjectLogsErrorNamingObject();
            TestNativeBridgeHostCoroutineLifecycle();
            TestNativeBridgeHostMultiStepFixedWait();
            TestNestedEnumeratorPredicateExceptionAndSceneReload();
            TestDeterministicReplay();
        }

        private static void TestStartStopAndStopAllCoroutines()
        {
            CoroutineRecorderScript ownerA = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            CoroutineRecorderScript ownerB = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            try
            {
                List<string> trace = new List<string>();
                Coroutine first = ownerA.StartCoroutine(StepRoutine(trace, "A1", 3));
                Coroutine second = ownerA.StartCoroutine(StepRoutine(trace, "A2", 3));
                Coroutine third = ownerB.StartCoroutine(StepRoutine(trace, "B1", 3));

                Check.That("StartCoroutine returns non-null handle for first", first != null);
                Check.That("StartCoroutine returns non-null handle for second", second != null);
                Check.That("StartCoroutine returns non-null handle for third", third != null);
                Check.Equal(
                    "StartCoroutine executes initial synchronous segment up to first yield",
                    "A1:0, A2:0, B1:0",
                    string.Join(", ", trace));

                ownerA.StopCoroutine(first!);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "StopCoroutine halts target handle while sibling and other owner continue",
                    "A1:0, A2:0, B1:0, A2:1, B1:1",
                    string.Join(", ", trace));

                Coroutine fourth = ownerA.StartCoroutine(StepRoutine(trace, "A3", 3));
                Check.That("Fourth coroutine handle is non-null", fourth != null);
                ownerA.StopAllCoroutines();
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "StopAllCoroutines halts all coroutines on ownerA while ownerB continues",
                    "A1:0, A2:0, B1:0, A2:1, B1:1, A3:0, B1:2",
                    string.Join(", ", trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(ownerA);
                CoroutineScheduler.OwnerDestroyed(ownerB);
            }
        }

        private static void TestWaitFormsResumeOnExpectedPhases()
        {
            CoroutineRecorderScript owner = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            try
            {
                List<string> trace = new List<string>();
                bool gate = false;

                owner.StartCoroutine(NullWaitRoutine(trace));
                owner.StartCoroutine(FixedWaitSingleRoutine(trace));
                owner.StartCoroutine(EndOfFrameSingleRoutine(trace));
                owner.StartCoroutine(WaitUntilRoutine(trace, () => gate));
                owner.StartCoroutine(OuterCoroutineWaitRoutine(owner, trace));

                Check.Equal(
                    "Wait forms execute initial segments before first yield",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init",
                    string.Join(", ", trace));

                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "First PumpFrame resumes null and eof in order while fixed and false WaitUntil wait",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init, null:resumed, inner:step1, eof:resumed",
                    string.Join(", ", trace));

                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Second PumpFrame completes inner coroutine and resumes outer coroutine on same frame",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init, null:resumed, inner:step1, eof:resumed, inner:done, outer:resumed",
                    string.Join(", ", trace));

                CoroutineScheduler.PumpFixed();
                Check.Equal(
                    "PumpFixed resumes WaitForFixedUpdate",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init, null:resumed, inner:step1, eof:resumed, inner:done, outer:resumed, fixed:resumed",
                    string.Join(", ", trace));

                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "WaitUntil remains suspended while predicate is false",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init, null:resumed, inner:step1, eof:resumed, inner:done, outer:resumed, fixed:resumed",
                    string.Join(", ", trace));

                gate = true;
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "WaitUntil resumes on PumpFrame after predicate becomes true",
                    "null:init, fixed:init, eof:init, until:init, outer:init, inner:init, null:resumed, inner:step1, eof:resumed, inner:done, outer:resumed, fixed:resumed, until:resumed",
                    string.Join(", ", trace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
            }
        }

        private static void TestScaledAndUnscaledWaitDurations()
        {
            CoroutineRecorderScript owner = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            float savedScale = Time.timeScale;
            try
            {
                List<string> pausedTrace = new List<string>();
                Time.timeScale = 0f;
                owner.StartCoroutine(WaitSecondsRoutine(pausedTrace, "scaled", 0.2f));
                owner.StartCoroutine(WaitRealtimeRoutine(pausedTrace, "realtime", 0.2f));

                for (int frame = 1; frame <= 5; frame++)
                {
                    Time.Advance(0.1);
                    CoroutineScheduler.PumpFrame();
                    if (frame == 1)
                    {
                        Check.Equal(
                            "At timeScale 0 after 0.1s neither 0.2s wait has completed",
                            "scaled:start, realtime:start",
                            string.Join(", ", pausedTrace));
                    }
                    else if (frame == 2)
                    {
                        Check.Equal(
                            "At timeScale 0 after 0.2s unscaled WaitForSecondsRealtime completes while WaitForSeconds stays suspended",
                            "scaled:start, realtime:start, realtime:done",
                            string.Join(", ", pausedTrace));
                    }
                }

                Check.Equal(
                    "At timeScale 0 after 5 frames WaitForSeconds still has not completed",
                    "scaled:start, realtime:start, realtime:done",
                    string.Join(", ", pausedTrace));
                owner.StopAllCoroutines();

                List<string> halfScaleTrace = new List<string>();
                Time.timeScale = 0.5f;
                owner.StartCoroutine(WaitSecondsRoutine(halfScaleTrace, "scaled", 0.2f));
                owner.StartCoroutine(WaitRealtimeRoutine(halfScaleTrace, "realtime", 0.2f));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "At timeScale 0.5 frame 1 (0.05s scaled, 0.1s unscaled) neither wait completes",
                    "scaled:start, realtime:start",
                    string.Join(", ", halfScaleTrace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "At timeScale 0.5 frame 2 (0.10s scaled, 0.2s unscaled) realtime completes",
                    "scaled:start, realtime:start, realtime:done",
                    string.Join(", ", halfScaleTrace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "At timeScale 0.5 frame 3 (0.15s scaled) scaled wait still pending",
                    "scaled:start, realtime:start, realtime:done",
                    string.Join(", ", halfScaleTrace));

                Time.Advance(0.1);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "At timeScale 0.5 frame 4 (0.20s scaled) scaled wait completes",
                    "scaled:start, realtime:start, realtime:done, scaled:done",
                    string.Join(", ", halfScaleTrace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                Time.timeScale = savedScale;
            }
        }

        private static void TestStopAndNonStopLifecycleRules()
        {
            CoroutineRecorderScript ownerA = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            GameObject goB = new GameObject("OwnerB");
            CoroutineRecorderScript ownerB = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            goB.AttachComponent(ownerB, typeof(CoroutineRecorderScript));
            CoroutineRecorderScript ownerC = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            try
            {
                List<string> naturalTrace = new List<string>();
                ownerA.StartCoroutine(StepRoutine(naturalTrace, "nat", 1));
                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();
                Check.Equal("Natural completion runs once and stops", "nat:0, nat:1", string.Join(", ", naturalTrace));

                List<string> disabledTrace = new List<string>();
                ownerA.StartCoroutine(StepRoutine(disabledTrace, "dis", 2));
                ownerA.enabled = false;
                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Disabling MonoBehaviour.enabled does not stop running coroutine",
                    "dis:0, dis:1, dis:2",
                    string.Join(", ", disabledTrace));
                ownerA.enabled = true;

                List<string> deactivatedTrace = new List<string>();
                ownerB.StartCoroutine(StepRoutine(deactivatedTrace, "deact", 3));
                ownerB.gameObject.SetActive(false);
                CoroutineScheduler.PumpFrame();
                ownerB.gameObject.SetActive(true);
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Deactivating GameObject stops coroutines permanently even after SetActive(true)",
                    "deact:0",
                    string.Join(", ", deactivatedTrace));

                List<string> destroyedTrace = new List<string>();
                ownerC.StartCoroutine(StepRoutine(destroyedTrace, "dest", 3));
                ownerC.StartCoroutine(FixedWaitSingleRoutine(destroyedTrace));
                CoroutineScheduler.OwnerDestroyed(ownerC);
                CoroutineScheduler.PumpFixed();
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "OwnerDestroyed stops frame and fixed coroutines on that owner",
                    "dest:0, fixed:init",
                    string.Join(", ", destroyedTrace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(ownerA);
                CoroutineScheduler.OwnerDestroyed(ownerB);
                CoroutineScheduler.OwnerDestroyed(ownerC);
                UnityEngine.Object.Destroy(goB);
            }
        }

        private static void TestStartCoroutineOnInactiveGameObjectLogsErrorNamingObject()
        {
            GameObject inactiveRoot = new GameObject("InactiveLauncherObject");
            CoroutineRecorderScript rootScript = inactiveRoot.AddComponent<CoroutineRecorderScript>();
            inactiveRoot.SetActive(false);

            GameObject activeParent = new GameObject("DeactivatedAncestorRoot");
            GameObject childUnderInactive = new GameObject("ChildOnInactiveBranch");
            childUnderInactive.transform.SetParent(activeParent.transform, false);
            CoroutineRecorderScript childScript = childUnderInactive.AddComponent<CoroutineRecorderScript>();
            activeParent.SetActive(false);

            LogRecorder.Clear();
            try
            {
                List<string> trace = new List<string>();
                rootScript.StartCoroutine(StepRoutine(trace, "directInactive", 2));
                CoroutineScheduler.PumpFrame();

                Check.Equal("Coroutine started on directly inactive GameObject does not run", 0, trace.Count);
                IReadOnlyList<LogEntry> directErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("StartCoroutine on directly inactive GameObject logs an error", 1, directErrors.Count);
                if (directErrors.Count == 1)
                {
                    Check.That(
                        "Error message names the inactive GameObject InactiveLauncherObject",
                        directErrors[0].Message.Contains("InactiveLauncherObject", StringComparison.Ordinal));
                    Check.That(
                        "Error message explains game object is inactive",
                        directErrors[0].Message.Contains("inactive", StringComparison.OrdinalIgnoreCase));
                }

                LogRecorder.Clear();
                childScript.StartCoroutine(StepRoutine(trace, "ancestorInactive", 2));
                CoroutineScheduler.PumpFrame();

                Check.Equal("Coroutine started on child under inactive ancestor does not run", 0, trace.Count);
                IReadOnlyList<LogEntry> childErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("StartCoroutine on child under inactive ancestor logs an error", 1, childErrors.Count);
                if (childErrors.Count == 1)
                {
                    Check.That(
                        "Error message names the inactive child GameObject ChildOnInactiveBranch",
                        childErrors[0].Message.Contains("ChildOnInactiveBranch", StringComparison.Ordinal));
                }
            }
            finally
            {
                LogRecorder.Clear();
                CoroutineScheduler.OwnerDestroyed(rootScript);
                CoroutineScheduler.OwnerDestroyed(childScript);
                UnityEngine.Object.Destroy(inactiveRoot);
                UnityEngine.Object.Destroy(activeParent);
                UnityEngine.Object.Destroy(childUnderInactive);
            }
        }

        private static void TestNativeBridgeHostCoroutineLifecycle()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Scene with script exists for coroutine lifecycle test", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            ScriptGlueRegistry.Clear();
            CoroutineRecorderScript recorded = new CoroutineRecorderScript();
            ScriptGlueRegistry.Register(0UL, 1UL, () => recorded);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);
                    host.Shutdown();
                }

                string expected = "Awake, OnEnable, Start, NextFrame:Init, FixedWait:Init, EndOfFrame:Init, FixedUpdate, FixedWait:1, Update, LateUpdate, NextFrame:1, EndOfFrame:1, OnDisable, OnDestroy";
                Check.Equal("BridgeHost pumps fixed and frame coroutines in exact lifecycle order", expected, recorded.ObservedOrder);

                int countAfterShutdown = recorded.Events.Count;
                CoroutineScheduler.PumpFixed();
                CoroutineScheduler.PumpFrame();
                Check.Equal("Shutdown / OnDestroy stops active coroutines on the destroyed script", countAfterShutdown, recorded.Events.Count);
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(recorded);
                Time.timeScale = savedScale;
            }
        }

        private static void TestNativeBridgeHostMultiStepFixedWait()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Physics query scene exists for multi-step fixed coroutine test", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            ScriptGlueRegistry.Clear();
            Collection<string> shared = new Collection<string>();
            CoroutineRecorderScript scriptA = new CoroutineRecorderScript(shared, "A", false);
            CoroutineRecorderScript scriptB = new CoroutineRecorderScript(shared, "B", false);
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => scriptA);
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => scriptB);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    scriptA.StartFixedLoop();
                    scriptB.StartFixedLoop();
                    shared.Clear();

                    host.Tick(0.05);
                    host.Shutdown();
                }

                string expected = "A:FixedUpdate, B:FixedUpdate, A:FixedWait:1, B:FixedWait:1, A:FixedUpdate, B:FixedUpdate, A:FixedWait:2, B:FixedWait:2, A:Update, B:Update, A:LateUpdate, B:LateUpdate, B:OnDisable, A:OnDisable, B:OnDestroy, A:OnDestroy";
                Check.Equal(
                    "Multi-step tick with two scripts pumps WaitForFixedUpdate once per native fixed step after both scripts run FixedUpdate",
                    expected,
                    string.Join(", ", shared));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(scriptA);
                CoroutineScheduler.OwnerDestroyed(scriptB);
                Time.timeScale = savedScale;
            }
        }

        private static void TestNestedEnumeratorPredicateExceptionAndSceneReload()
        {
            CoroutineRecorderScript owner = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            try
            {
                List<string> nestedTrace = new List<string>();
                owner.StartCoroutine(RawOuterEnumeratorRoutine(nestedTrace));
                Check.Equal("Raw nested IEnumerator runs synchronously to first inner yield", "rawOuter:start, rawInner:start", string.Join(", ", nestedTrace));

                CoroutineScheduler.PumpFrame();
                Check.Equal("Raw nested IEnumerator advances inner step on PumpFrame", "rawOuter:start, rawInner:start, rawInner:mid", string.Join(", ", nestedTrace));

                CoroutineScheduler.PumpFrame();
                Check.Equal("Raw nested IEnumerator pops to parent and resumes parent on completion", "rawOuter:start, rawInner:start, rawInner:mid, rawInner:end, rawOuter:afterInner", string.Join(", ", nestedTrace));

                CoroutineScheduler.PumpFrame();
                Check.Equal("Parent completes after its final yield", "rawOuter:start, rawInner:start, rawInner:mid, rawInner:end, rawOuter:afterInner, rawOuter:end", string.Join(", ", nestedTrace));

                LogRecorder.Clear();
                List<string> faultTrace = new List<string>();
                owner.StartCoroutine(FaultingWaitUntilRoutine(faultTrace));
                owner.StartCoroutine(StepRoutine(faultTrace, "sibling", 2));

                CoroutineScheduler.PumpFrame();
                CoroutineScheduler.PumpFrame();

                Check.Equal(
                    "WaitUntil predicate exception terminates faulting coroutine while sibling continues",
                    "fault:init, sibling:0, sibling:1, sibling:2",
                    string.Join(", ", faultTrace));

                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Predicate exception logs one error entry via Debug.LogException", 1, errors.Count);
                if (errors.Count > 0)
                {
                    Check.That("Logged exception message contains predicate boom", errors[0].Message.Contains("predicate boom", StringComparison.Ordinal));
                    Check.That("Logged exception origin is the coroutine owner", ReferenceEquals(errors[0].Origin, owner));
                }

                LogRecorder.Clear();
                List<string> bodyFaultTrace = new List<string>();
                owner.StartCoroutine(FaultingBodyRoutine(bodyFaultTrace));
                owner.StartCoroutine(StepRoutine(bodyFaultTrace, "bodySibling", 1));
                CoroutineScheduler.PumpFrame();
                Check.Equal(
                    "Enumerator MoveNext exception terminates faulting coroutine while sibling continues",
                    "bodyFault:init, bodySibling:0, bodySibling:1",
                    string.Join(", ", bodyFaultTrace));
                IReadOnlyList<LogEntry> bodyErrors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Enumerator MoveNext exception logs one error entry via Debug.LogException", 1, bodyErrors.Count);
                if (bodyErrors.Count > 0)
                {
                    Check.That("Logged body exception message contains body boom", bodyErrors[0].Message.Contains("body boom", StringComparison.Ordinal));
                    Check.That("Logged body exception stack identifies the coroutine", bodyErrors[0].CallStack.Contains("FaultingBodyRoutine", StringComparison.Ordinal));
                    Check.That("Logged body exception origin is the coroutine owner", ReferenceEquals(bodyErrors[0].Origin, owner));
                }

                List<string> completedChildTrace = new List<string>();
                owner.StartCoroutine(YieldCompletedCoroutineRoutine(owner, completedChildTrace));
                Check.Equal(
                    "Yielding an already-completed Coroutine continues without waiting a frame",
                    "completedOuter:start, completedInner:run, completedOuter:resumed",
                    string.Join(", ", completedChildTrace));
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
            }

            string scriptedScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            string plainScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision_no_script.unity");
            Check.That("Scripted scene exists for reload test", File.Exists(scriptedScene));
            Check.That("Plain scene exists for reload test", File.Exists(plainScene));

            ScriptGlueRegistry.Clear();
            CoroutineRecorderScript sceneScript = new CoroutineRecorderScript();
            ScriptGlueRegistry.Register(0UL, 1UL, () => sceneScript);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scriptedScene);
                    host.Tick(0.02);
                    host.Tick(0.02);
                    int countBeforeReload = sceneScript.Events.Count;

                    host.LoadScene(plainScene);
                    host.Tick(0.02);
                    host.Tick(0.02);
                    int countAfterReload = sceneScript.Events.Count;
                    string teardownEvents = string.Empty;
                    for (int index = countBeforeReload; index < sceneScript.Events.Count; index++)
                    {
                        string recorded = sceneScript.Events[index];
                        teardownEvents = teardownEvents.Length == 0 ? recorded : teardownEvents + ", " + recorded;
                    }

                    Check.Equal(
                        "LoadScene tears the previous scene script down at the reload boundary",
                        "OnDisable, OnDestroy",
                        teardownEvents);
                    Check.Equal("LoadScene stops coroutines from previous scene", countBeforeReload + 2, countAfterReload);
                    host.Shutdown();
                }
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(sceneScript);
            }
        }

        private static void TestDeterministicReplay()
        {
            string firstRun = RunDeterministicScenario();
            string secondRun = RunDeterministicScenario();
            Check.That("Deterministic scenario produces non-empty trace", !string.IsNullOrEmpty(firstRun));
            Check.Equal("Two runs of the coroutine scenario produce identical event sequences", firstRun, secondRun);
        }

        private static string RunDeterministicScenario()
        {
            CoroutineRecorderScript owner = new CoroutineRecorderScript(new Collection<string>(), string.Empty, false);
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                List<string> events = new List<string>();
                bool ready = false;
                owner.StartCoroutine(StepRoutine(events, "step", 2));
                owner.StartCoroutine(WaitSecondsRoutine(events, "sec", 0.03f));
                owner.StartCoroutine(FixedWaitSingleRoutine(events));
                owner.StartCoroutine(EndOfFrameSingleRoutine(events));
                owner.StartCoroutine(WaitUntilRoutine(events, () => ready));

                Time.Advance(0.02);
                CoroutineScheduler.PumpFixed();
                CoroutineScheduler.PumpFrame();

                ready = true;
                Time.Advance(0.02);
                CoroutineScheduler.PumpFixed();
                CoroutineScheduler.PumpFrame();

                return string.Join(", ", events);
            }
            finally
            {
                CoroutineScheduler.OwnerDestroyed(owner);
                Time.timeScale = savedScale;
            }
        }

        private static IEnumerator StepRoutine(List<string> trace, string label, int count)
        {
            trace.Add(label + ":0");
            for (int i = 1; i <= count; i++)
            {
                yield return null;
                trace.Add(label + ":" + i.ToString(CultureInfo.InvariantCulture));
            }
        }

        private static IEnumerator NullWaitRoutine(List<string> trace)
        {
            trace.Add("null:init");
            yield return null;
            trace.Add("null:resumed");
        }

        private static IEnumerator FixedWaitSingleRoutine(List<string> trace)
        {
            trace.Add("fixed:init");
            yield return new WaitForFixedUpdate();
            trace.Add("fixed:resumed");
        }

        private static IEnumerator EndOfFrameSingleRoutine(List<string> trace)
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

        private static IEnumerator OuterCoroutineWaitRoutine(MonoBehaviour owner, List<string> trace)
        {
            trace.Add("outer:init");
            yield return owner.StartCoroutine(InnerCoroutineRoutine(trace));
            trace.Add("outer:resumed");
        }

        private static IEnumerator InnerCoroutineRoutine(List<string> trace)
        {
            trace.Add("inner:init");
            yield return null;
            trace.Add("inner:step1");
            yield return null;
            trace.Add("inner:done");
        }

        private static IEnumerator WaitSecondsRoutine(List<string> trace, string label, float seconds)
        {
            trace.Add(label + ":start");
            yield return new WaitForSeconds(seconds);
            trace.Add(label + ":done");
        }

        private static IEnumerator WaitRealtimeRoutine(List<string> trace, string label, float seconds)
        {
            trace.Add(label + ":start");
            yield return new WaitForSecondsRealtime(seconds);
            trace.Add(label + ":done");
        }

        private static IEnumerator RawOuterEnumeratorRoutine(List<string> trace)
        {
            trace.Add("rawOuter:start");
            yield return RawInnerEnumeratorRoutine(trace);
            trace.Add("rawOuter:afterInner");
            yield return null;
            trace.Add("rawOuter:end");
        }

        private static IEnumerator RawInnerEnumeratorRoutine(List<string> trace)
        {
            trace.Add("rawInner:start");
            yield return null;
            trace.Add("rawInner:mid");
            yield return null;
            trace.Add("rawInner:end");
        }

        private static IEnumerator FaultingWaitUntilRoutine(List<string> trace)
        {
            trace.Add("fault:init");
            yield return new WaitUntil(() => throw new InvalidOperationException("predicate boom"));
            trace.Add("fault:unreachable");
        }

        private static IEnumerator FaultingBodyRoutine(List<string> trace)
        {
            trace.Add("bodyFault:init");
            yield return null;
            throw new InvalidOperationException("body boom");
        }

        private static IEnumerator ImmediateRoutine(List<string> trace)
        {
            trace.Add("completedInner:run");
            yield break;
        }

        private static IEnumerator YieldCompletedCoroutineRoutine(MonoBehaviour owner, List<string> trace)
        {
            trace.Add("completedOuter:start");
            Coroutine completed = owner.StartCoroutine(ImmediateRoutine(trace));
            yield return completed;
            trace.Add("completedOuter:resumed");
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
