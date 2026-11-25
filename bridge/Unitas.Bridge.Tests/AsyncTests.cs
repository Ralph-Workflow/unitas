using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class AsyncOrderProbeScript : MonoBehaviour
    {
        private readonly List<string> _trace;
        private readonly TaskCompletionSource<int> _delayGate = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);

        public AsyncOrderProbeScript(List<string> trace)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        }

        public int StartThreadId { get; private set; }

        public int ContinuationThreadId { get; private set; }

        public int ContinuationFrame { get; private set; }

        public bool SpawnedLeakTask { get; set; }

        public Task? LeakedTask { get; private set; }

        public void ReleaseAwaiter()
        {
            _delayGate.TrySetResult(1);
        }

        public override void Start()
        {
            StartThreadId = Environment.CurrentManagedThreadId;
            _trace.Add("Start: before-await");
            Task tracked = UnitySynchronizationContext.TrackTask(RunAwaitedContinuationAsync(), this, "AsyncOrderProbeScript.RunAwaitedContinuationAsync");
            _ = tracked;

            if (SpawnedLeakTask)
            {
                TaskCompletionSource<int> neverCompletes = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
                LeakedTask = UnitySynchronizationContext.TrackTask(neverCompletes.Task, this, "AsyncOrderProbeScript.BackgroundPoller");
            }
        }

        public override void FixedUpdate()
        {
            _trace.Add("FixedUpdate:" + Time.frameCount);
        }

        public override void Update()
        {
            _trace.Add("Update:" + Time.frameCount);
        }

        public override void LateUpdate()
        {
            _trace.Add("LateUpdate:" + Time.frameCount);
        }

        private async Task RunAwaitedContinuationAsync()
        {
            await _delayGate.Task;
            ContinuationThreadId = Environment.CurrentManagedThreadId;
            ContinuationFrame = Time.frameCount;
            _trace.Add("Continuation:frame=" + Time.frameCount);
        }
    }

    public static class AsyncTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            TestAwaitedDelayResumesOnMainThreadInsideFramePump();
            TestMainThreadGuardRejectsBackgroundThreadCallNamingApi();
            TestLiveTaskStillRunningAtPlayEndIsReportedWithOrigin();
        }

        private static void TestAwaitedDelayResumesOnMainThreadInsideFramePump()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Scene fixture exists for async continuation test", File.Exists(scenePath));

            List<string> trace = new List<string>();
            AsyncOrderProbeScript probe = new AsyncOrderProbeScript(trace);
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0UL, 1UL, () => probe);

            int mainThreadId = Environment.CurrentManagedThreadId;

            using (BridgeHost host = new BridgeHost())
            {
                Check.That(
                    "BridgeHost installs UnitySynchronizationContext on main thread",
                    SynchronizationContext.Current is UnitySynchronizationContext);

                host.LoadScene(scenePath);
                host.Tick(0.02);

                Check.Equal("Start ran on main thread", mainThreadId, probe.StartThreadId);
                Check.Equal("Continuation has not run before awaiter completes", 0, probe.ContinuationThreadId);

                Task.Run(async () =>
                {
                    await Task.Delay(5).ConfigureAwait(false);
                    probe.ReleaseAwaiter();
                }).GetAwaiter().GetResult();

                SpinWait.SpinUntil(() => UnitySynchronizationContext.PendingContinuationCount > 0, 500);
                Check.Equal("Continuation is queued and does not run outside Tick", 0, probe.ContinuationThreadId);

                int frameBeforeSecondTick = Time.frameCount;
                host.Tick(0.02);

                Check.Equal("Awaited continuation resumed on the main thread", mainThreadId, probe.ContinuationThreadId);
                Check.Equal("Awaited continuation observed the current frame inside Tick", frameBeforeSecondTick + 1, probe.ContinuationFrame);

                int fixedIdx = trace.FindIndex(entry => entry == "FixedUpdate:" + (frameBeforeSecondTick + 1));
                int contIdx = trace.FindIndex(entry => entry == "Continuation:frame=" + (frameBeforeSecondTick + 1));
                int updateIdx = trace.FindIndex(entry => entry == "Update:" + (frameBeforeSecondTick + 1));
                Check.That(
                    "Awaited continuation executed inside the tick's fixed pump after FixedUpdate and before Update",
                    fixedIdx >= 0 && contIdx > fixedIdx && updateIdx > contIdx);

                trace.Clear();
                TaskCompletionSource<int> frameGate = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
                int frameContThreadId = 0;
                Func<Task> runAfterUpdate = async () =>
                {
                    await frameGate.Task;
                    frameContThreadId = Environment.CurrentManagedThreadId;
                    trace.Add("FrameContinuation:frame=" + Time.frameCount);
                };
                _ = UnitySynchronizationContext.TrackTask(runAfterUpdate(), probe, "AsyncOrderProbeScript.FramePumpContinuation");
                frameGate.TrySetResult(1);
                SpinWait.SpinUntil(() => UnitySynchronizationContext.PendingContinuationCount > 0, 500);

                int frameBeforeThirdTick = Time.frameCount;
                host.Tick(0.005);

                Check.Equal("Frame-pump continuation resumed on main thread", mainThreadId, frameContThreadId);
                int thirdUpdateIdx = trace.FindIndex(entry => entry == "Update:" + (frameBeforeThirdTick + 1));
                int thirdLateIdx = trace.FindIndex(entry => entry == "LateUpdate:" + (frameBeforeThirdTick + 1));
                int thirdContIdx = trace.FindIndex(entry => entry == "FrameContinuation:frame=" + (frameBeforeThirdTick + 1));
                Check.That(
                    "Frame-pump continuation executed inside the tick after Update and LateUpdate",
                    thirdUpdateIdx >= 0 && thirdLateIdx > thirdUpdateIdx && thirdContIdx > thirdLateIdx);

                host.Shutdown();
            }
        }

        private static void TestMainThreadGuardRejectsBackgroundThreadCallNamingApi()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Exception? backgroundError = null;
                Task worker = Task.Run(() =>
                {
                    try
                    {
                        UnitySynchronizationContext.EnsureMainThread("GameObject.CreatePrimitive");
                    }
                    catch (Exception ex) when (!(ex is OutOfMemoryException))
                    {
                        backgroundError = ex;
                    }
                });
                worker.GetAwaiter().GetResult();

                Check.That("Calling main-thread-only API from worker task throws an exception", backgroundError != null);
                if (backgroundError != null)
                {
                    Check.That(
                        "Main-thread exception message names the API GameObject.CreatePrimitive",
                        backgroundError.Message.Contains("GameObject.CreatePrimitive", StringComparison.Ordinal));
                    Check.That(
                        "Main-thread exception message states main thread requirement",
                        backgroundError.Message.Contains("main thread", StringComparison.OrdinalIgnoreCase));
                }
            }
        }

        private static void TestLiveTaskStillRunningAtPlayEndIsReportedWithOrigin()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            List<string> trace = new List<string>();
            AsyncOrderProbeScript probe = new AsyncOrderProbeScript(trace)
            {
                SpawnedLeakTask = true,
            };
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0UL, 1UL, () => probe);
            LogRecorder.Clear();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);
                probe.ReleaseAwaiter();
                SpinWait.SpinUntil(() => UnitySynchronizationContext.PendingContinuationCount > 0, 500);
                host.Tick(0.02);

                Check.That("Leaked task is still running before Shutdown", probe.LeakedTask != null && !probe.LeakedTask.IsCompleted);
                Check.Equal("One live task remains tracked before Shutdown", 1, UnitySynchronizationContext.LiveTaskCount);

                host.Shutdown();
            }

            IReadOnlyList<LogEntry> warnings = LogRecorder.OfSeverity(LogSeverity.Warning);
            bool foundReport = false;
            for (int i = 0; i < warnings.Count; i++)
            {
                LogEntry entry = warnings[i];
                if (entry.Message.Contains("AsyncOrderProbeScript.BackgroundPoller", StringComparison.Ordinal)
                    && ReferenceEquals(entry.Origin, probe))
                {
                    foundReport = true;
                    break;
                }
            }

            Check.That("Task still running when play ends is reported with its origin and label", foundReport);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
