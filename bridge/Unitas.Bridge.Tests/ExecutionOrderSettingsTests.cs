using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [DefaultExecutionOrder(-100)]
    public sealed class AssetOrderScriptA : MonoBehaviour
    {
        private readonly List<string> _trace;

        public AssetOrderScriptA(List<string> trace)
        {
            _trace = trace;
        }

        public override int DefaultExecutionOrderValue => -100;

        public override void Awake()
        {
            _trace.Add("W:A");
        }

        public override void Update()
        {
            _trace.Add("U:A");
        }
    }

    [DefaultExecutionOrder(100)]
    public sealed class AssetOrderScriptB : MonoBehaviour
    {
        private readonly List<string> _trace;

        public AssetOrderScriptB(List<string> trace)
        {
            _trace = trace;
        }

        public override int DefaultExecutionOrderValue => 100;

        public override void Awake()
        {
            _trace.Add("W:B");
        }

        public override void Update()
        {
            _trace.Add("U:B");
        }
    }

    public sealed class AssetOrderScriptC : MonoBehaviour
    {
        private readonly List<string> _trace;

        public AssetOrderScriptC(List<string> trace)
        {
            _trace = trace;
        }

        public override int DefaultExecutionOrderValue => 0;

        public override void Awake()
        {
            _trace.Add("W:C");
        }

        public override void Update()
        {
            _trace.Add("U:C");
        }
    }

    public static class ExecutionOrderSettingsTests
    {
        private const string AssetFileName = "ScriptExecutionOrder.asset";
        private const string UnknownGuid = "dddddddddddddddddddddddddddddddd";

        public static void Run()
        {
            TestProjectAssetOrderOverridesAttributeOrder();
            TestAssetParsingIgnoresMalformedRecords();
        }

        private static void TestProjectAssetOrderOverridesAttributeOrder()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_exec_order_" + Guid.NewGuid().ToString("N"));
            string settingsDir = Path.Combine(tempRoot, "ProjectSettings");
            string scenesDir = Path.Combine(tempRoot, "Assets", "Scenes");
            Directory.CreateDirectory(settingsDir);
            Directory.CreateDirectory(scenesDir);

            string tempScene = Path.Combine(scenesDir, "scene_execution_order.unity");
            File.Copy(
                ExecutionOrderDispatchTests.ResolveFixture(ExecutionOrderDispatchTests.FixtureRelativePath),
                tempScene,
                true);

            // The attribute order is A(-100) < C(0) < B(100); the project asset reverses A and B
            // and names a guid no script uses, which Unity ignores.
            File.WriteAllText(
                Path.Combine(settingsDir, AssetFileName),
                "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n"
                    + "--- !u!114 &1\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: " + ExecutionOrderDispatchTests.GuidA + ", type: 3}\n  m_ExecutionOrder: 100\n"
                    + "--- !u!114 &2\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: " + ExecutionOrderDispatchTests.GuidB + ", type: 3}\n  m_ExecutionOrder: -100\n"
                    + "--- !u!114 &3\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: " + UnknownGuid + ", type: 3}\n  m_ExecutionOrder: -500\n");

            List<string> trace = new List<string>();
            try
            {
                ScriptGlueRegistry.Clear();
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidA, () => new AssetOrderScriptA(trace));
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidB, () => new AssetOrderScriptB(trace));
                ScriptGlueRegistry.Register(ExecutionOrderDispatchTests.GuidC, () => new AssetOrderScriptC(trace));

                MonoBehaviour scriptA = ScriptGlueRegistry.Create(ExecutionOrderDispatchTests.GuidA)!;
                MonoBehaviour scriptB = ScriptGlueRegistry.Create(ExecutionOrderDispatchTests.GuidB)!;
                Check.Equal("Attribute order puts A before B", -100, MonoBehaviour.GetEffectiveExecutionOrder(scriptA));
                Check.Equal("Attribute order puts B after A", 100, MonoBehaviour.GetEffectiveExecutionOrder(scriptB));

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScene);

                    Check.Equal("Project asset order overrides the attribute order for A", 100, MonoBehaviour.GetEffectiveExecutionOrder(scriptA));
                    Check.Equal("Project asset order overrides the attribute order for B", -100, MonoBehaviour.GetEffectiveExecutionOrder(scriptB));

                    GameObject ownerA = new GameObject("AssetOrderA");
                    GameObject ownerB = new GameObject("AssetOrderB");
                    ownerA.AttachComponent(scriptA, typeof(AssetOrderScriptA));
                    ownerB.AttachComponent(scriptB, typeof(AssetOrderScriptB));

                    trace.Clear();
                    MonoBehaviour.RunLifecycleBatch(new MonoBehaviour[] { scriptA, scriptB });
                    Check.Equal(
                        "Awake dispatch follows the project asset order (B before A)",
                        "W:B,W:A",
                        string.Join(",", trace.FindAll(entry => entry.StartsWith("W:", StringComparison.Ordinal))));

                    trace.Clear();
                    host.Tick(0.02);
                    trace.Clear();
                    host.Tick(0.02);
                    host.Shutdown();
                }

                Check.Equal(
                    "ScriptExecutionOrder.asset order (B before A) wins over the attribute order in the Update dispatch",
                    "U:B,U:C,U:A",
                    string.Join(",", trace.FindAll(entry => entry.StartsWith("U:", StringComparison.Ordinal))));
            }
            finally
            {
                MonoBehaviour.ClearExecutionOrderOverrides();
                ScriptGlueRegistry.Clear();
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void TestAssetParsingIgnoresMalformedRecords()
        {
            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_exec_order_parse_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempRoot);
            string assetPath = Path.Combine(tempRoot, AssetFileName);

            try
            {
                File.WriteAllText(
                    assetPath,
                    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n"
                        + "--- !u!114 &1\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: not-a-guid, type: 3}\n  m_ExecutionOrder: 10\n"
                        + "--- !u!114 &2\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: " + ExecutionOrderDispatchTests.GuidA + ", type: 3}\n  m_ExecutionOrder: not-a-number\n"
                        + "--- !u!114 &3\nMonoBehaviour:\n  m_Script: {fileID: 11500000, guid: " + ExecutionOrderDispatchTests.GuidB + ", type: 3}\n  m_ExecutionOrder: -7\n");

                bool parsed = ExecutionOrderSettings.TryParse(assetPath, out Dictionary<string, int> orders);
                Check.That("Malformed ScriptExecutionOrder.asset parses without throwing", parsed);
                Check.Equal("Malformed ScriptExecutionOrder.asset yields only the valid record", 1, orders.Count);
                bool hasValidRecord = orders.TryGetValue(ExecutionOrderDispatchTests.GuidB, out int validOrder);
                Check.That("Valid ScriptExecutionOrder.asset record survives the malformed records", hasValidRecord);
                Check.Equal("Valid ScriptExecutionOrder.asset record keeps its order", -7, hasValidRecord ? validOrder : 0);
            }
            finally
            {
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }
    }
}
