using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class FixedStepDeterminismTests
    {
        private static readonly int[] FrameRates = { 30, 60, 144, 240 };

        private const double FixedStep = 0.02;
        private const double SimulatedSeconds = 1.0;
        private const int ExpectedFixedSteps = (int)(SimulatedSeconds / FixedStep); // 50

        private const string FixedTriple = "F:A,F:B,F:C";
        private const string UpdateAndLateUpdateTail = "U:A,U:B,U:C,L:A,L:B,L:C";

        public static void Run()
        {
            string scenePath = ExecutionOrderDispatchTests.ResolveFixture(ExecutionOrderDispatchTests.FixtureRelativePath);
            Check.That("Fixed-step determinism scene file exists", File.Exists(scenePath));

            Dictionary<int, string> fixedTraceByRate = new Dictionary<int, string>();
            string baselineFixedTrace = string.Empty;

            for (int r = 0; r < FrameRates.Length; r++)
            {
                int rate = FrameRates[r];
                List<string> trace = SimulateOneSecond(scenePath, rate);

                int fixedCount = trace.FindAll(entry => entry.StartsWith("F:", StringComparison.Ordinal)).Count;
                Check.Equal(
                    "Fixed-step accumulator runs exactly " + ExpectedFixedSteps + " steps for 1.0 s at " + rate + " fps",
                    ExpectedFixedSteps * 3,
                    fixedCount);

                string fixedTrace = string.Join(",", trace.FindAll(entry => entry.StartsWith("F:", StringComparison.Ordinal)));
                fixedTraceByRate[rate] = fixedTrace;
                if (r == 0)
                {
                    baselineFixedTrace = fixedTrace;
                }
                else
                {
                    Check.Equal(
                        "FixedUpdate event order at " + rate + " fps matches the " + FrameRates[0] + " fps baseline",
                        baselineFixedTrace,
                        fixedTrace);
                }

                Check.That(
                    "Every tick at " + rate + " fps dispatches whole A,B,C phases with FixedUpdate before Update before LateUpdate",
                    EveryTickHasWellFormedPhases(trace));
            }
        }

        private static List<string> SimulateOneSecond(string scenePath, int rate)
        {
            double deltaSeconds = 1.0 / rate;
            List<string> trace = new List<string>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidA, () => new ExecutionOrderRecorderScript("A", trace));
            ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidB, () => new ExecutionOrderRecorderScript("B", trace));
            ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidC, () => new ExecutionOrderRecorderScript("C", trace));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(deltaSeconds); // first tick only runs Awake/OnEnable/Start
                    trace.Clear();
                    for (int tick = 0; tick < rate; tick++)
                    {
                        host.Tick(deltaSeconds);
                        trace.Add("|");
                    }
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }

            return trace;
        }

        private static bool EveryTickHasWellFormedPhases(List<string> trace)
        {
            List<string> segment = new List<string>();
            for (int i = 0; i < trace.Count; i++)
            {
                if (trace[i] != "|")
                {
                    segment.Add(trace[i]);
                    continue;
                }
                if (!SegmentIsWellFormed(segment))
                {
                    return false;
                }
                segment.Clear();
            }
            return segment.Count == 0;
        }

        private static bool SegmentIsWellFormed(List<string> segment)
        {
            // One tick must be zero or more "F:A,F:B,F:C" fixed steps followed by
            // exactly one Update and one LateUpdate pass in the same script order.
            int fixedEntryCount = segment.FindAll(entry => entry.StartsWith("F:", StringComparison.Ordinal)).Count;
            if (fixedEntryCount % 3 != 0 || segment.Count != fixedEntryCount + 6)
            {
                return false;
            }
            string expected = string.Empty;
            for (int step = 0; step < fixedEntryCount / 3; step++)
            {
                expected += FixedTriple + ",";
            }
            expected += UpdateAndLateUpdateTail;
            return string.Join(",", segment) == expected;
        }
    }
}
