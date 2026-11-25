using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class ExecutionOrderRecorderScript : MonoBehaviour
    {
        private readonly string _tag;
        private readonly List<string> _trace;

        public ExecutionOrderRecorderScript(string tag, List<string> trace)
        {
            _tag = tag;
            _trace = trace;
        }

        public override void FixedUpdate()
        {
            _trace.Add("F:" + _tag);
        }

        public override void Update()
        {
            _trace.Add("U:" + _tag);
        }

        public override void LateUpdate()
        {
            _trace.Add("L:" + _tag);
        }
    }

    public sealed class ExecutionOrderScriptA : ExecutionOrderRecorderScript
    {
        public ExecutionOrderScriptA(List<string> trace)
            : base("A", trace)
        {
        }
    }

    public sealed class ExecutionOrderScriptB : ExecutionOrderRecorderScript
    {
        public ExecutionOrderScriptB(List<string> trace)
            : base("B", trace)
        {
        }

        public override int DefaultExecutionOrderValue => 50;
    }

    public sealed class ExecutionOrderScriptC : ExecutionOrderRecorderScript
    {
        public ExecutionOrderScriptC(List<string> trace)
            : base("C", trace)
        {
        }
    }

    public static class ExecutionOrderDispatchTests
    {
        internal const string FixtureRelativePath = "bridge/Unitas.Bridge.Tests/Fixtures/scene_execution_order.unity";

        internal const string GuidA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        internal const string GuidB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        internal const string GuidC = "cccccccccccccccccccccccccccccccc";

        public static void Run()
        {
            TestDispatchWithEqualOrdersKeepsSceneInsertionOrder();
            RunStrictExecutionOrderAssertions();
        }

        // Strict assertions: BridgeHost buffers each phase and dispatches sorted by
        // GetEffectiveExecutionOrder (the dispatch-ordering fix in BridgeHost.cs).
        public static void RunStrictExecutionOrderAssertions()
        {
            TestUpdateAndFixedUpdateDispatchHonorsRegisteredExecutionOrder();
        }

        private static void TestUpdateAndFixedUpdateDispatchHonorsRegisteredExecutionOrder()
        {
            string scenePath = ResolveFixture(FixtureRelativePath);
            Check.That("Execution-order scene file exists", File.Exists(scenePath));

            List<string> trace = new List<string>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(GuidA, () => new ExecutionOrderScriptA(trace));
            ScriptGlueRegistry.Register(GuidB, () => new ExecutionOrderScriptB(trace));
            ScriptGlueRegistry.Register(GuidC, () => new ExecutionOrderScriptC(trace));

            // Scene insertion order is A, B, C. Registered orders are
            // C = -100 (RegisterDefaultExecutionOrder), B = 50
            // (DefaultExecutionOrderValue), A = 100 (SetExecutionOrderOverride),
            // so dispatch must run C, B, A in every phase.
            MonoBehaviour.RegisterDefaultExecutionOrder<ExecutionOrderScriptC>(-100);
            MonoBehaviour.SetExecutionOrderOverride<ExecutionOrderScriptA>(100);
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02); // first tick only runs Awake/OnEnable/Start
                    trace.Clear();
                    host.Tick(0.02); // one fixed step, one update, one late update
                    host.Shutdown();
                }
            }
            finally
            {
                MonoBehaviour.ClearExecutionOrderOverrides();
                MonoBehaviour.ClearRegisteredDefaultExecutionOrders();
                ScriptGlueRegistry.Clear();
            }

            string observed = string.Join(",", trace);
            Check.Equal(
                "FixedUpdate dispatch honors registered execution order (C,B,A) over insertion order (A,B,C)",
                "F:C,F:B,F:A",
                string.Join(",", trace.FindAll(entry => entry.StartsWith("F:", StringComparison.Ordinal))));
            Check.Equal(
                "Update dispatch honors registered execution order (C,B,A) over insertion order (A,B,C)",
                "U:C,U:B,U:A",
                string.Join(",", trace.FindAll(entry => entry.StartsWith("U:", StringComparison.Ordinal))));
            Check.Equal(
                "LateUpdate dispatch honors registered execution order (C,B,A) over insertion order (A,B,C)",
                "L:C,L:B,L:A",
                string.Join(",", trace.FindAll(entry => entry.StartsWith("L:", StringComparison.Ordinal))));
            Check.Equal(
                "Every FixedUpdate precedes every Update which precedes every LateUpdate within a tick",
                "F:C,F:B,F:A,U:C,U:B,U:A,L:C,L:B,L:A",
                observed);
        }

        private static void TestDispatchWithEqualOrdersKeepsSceneInsertionOrder()
        {
            string scenePath = ResolveFixture(FixtureRelativePath);

            List<string> trace = new List<string>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(GuidA, () => new ExecutionOrderRecorderScript("A", trace));
            ScriptGlueRegistry.Register(GuidB, () => new ExecutionOrderRecorderScript("B", trace));
            ScriptGlueRegistry.Register(GuidC, () => new ExecutionOrderRecorderScript("C", trace));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    trace.Clear();
                    host.Tick(0.02);
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }

            Check.Equal(
                "Equal execution orders keep scene insertion order (stable sort)",
                "F:A,F:B,F:C,U:A,U:B,U:C,L:A,L:B,L:C",
                string.Join(",", trace));
        }

        internal static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
