using System;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class ScriptingPerfProbeBehaviour : MonoBehaviour
    {
        public int UpdateCount { get; private set; }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public static class ScriptingPerfTests
    {
        private const int CallCostWarmupCalls = 20000;
        private const int CallCostMeasuredCalls = 200000;
        private const int FrameWarmupTicks = 20;
        private const int FrameMeasuredTicks = 200;
        private const double FrameDeltaSeconds = 0.02;
        private const double CallCostCeilingMs = 0.000446;
        private const double FrameAllocationCeilingBytes = 6566.88;

        public static void Run()
        {
            MeasureScriptingCallCost();
            MeasurePlayerLoopFrameAllocations();
        }

        private static void MeasureScriptingCallCost()
        {
            ulong handle = RuntimeInterop.unitas_runtime_new();
            Check.That("Scripting perf native runtime handle is created", handle != 0);
            if (handle == 0)
            {
                return;
            }

            try
            {
                double observed = 0.0;
                for (int index = 0; index < CallCostWarmupCalls; index++)
                {
                    observed += RuntimeInterop.unitas_runtime_time_scale(handle);
                }

                Stopwatch stopwatch = Stopwatch.StartNew();
                for (int index = 0; index < CallCostMeasuredCalls; index++)
                {
                    observed += RuntimeInterop.unitas_runtime_time_scale(handle);
                }

                stopwatch.Stop();
                double valueMs = stopwatch.Elapsed.TotalMilliseconds / CallCostMeasuredCalls;
                Console.WriteLine(
                    "scripting_call_cost calls: " + CallCostMeasuredCalls.ToString(CultureInfo.InvariantCulture) +
                    ", observed time scale: " + observed.ToString(CultureInfo.InvariantCulture));
                Console.WriteLine(
                    "measurement=scripting_call_cost value_ms=" + Format(valueMs) +
                    " ceiling_ms=" + Format(CallCostCeilingMs));
            }
            finally
            {
                RuntimeInterop.unitas_runtime_destroy(handle);
            }
        }

        private static void MeasurePlayerLoopFrameAllocations()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            Check.That("PlayerLoop allocation scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            ScriptingPerfProbeBehaviour probe = new ScriptingPerfProbeBehaviour();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => probe);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int index = 0; index < FrameWarmupTicks; index++)
                {
                    host.Tick(FrameDeltaSeconds);
                }

                long before = GC.GetAllocatedBytesForCurrentThread();
                for (int index = 0; index < FrameMeasuredTicks; index++)
                {
                    host.Tick(FrameDeltaSeconds);
                }

                long after = GC.GetAllocatedBytesForCurrentThread();
                Check.That("PlayerLoop dispatch reaches the measured script", probe.UpdateCount > 0);

                long allocatedBytes = after - before;
                double valueBytes = (double)allocatedBytes / FrameMeasuredTicks;
                Console.WriteLine(
                    "playerloop_frame_allocations ticks: " + FrameMeasuredTicks.ToString(CultureInfo.InvariantCulture) +
                    ", allocated bytes: " + allocatedBytes.ToString(CultureInfo.InvariantCulture));
                Console.WriteLine(
                    "measurement=playerloop_frame_allocations value_bytes=" + Format(valueBytes) +
                    " ceiling_bytes=" + Format(FrameAllocationCeilingBytes));
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private static string Format(double value)
        {
            return value.ToString("0.000000", CultureInfo.InvariantCulture);
        }
    }
}
