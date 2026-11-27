using System;
using System.IO;
using System.Runtime.CompilerServices;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class PhysicsSimulationTests
    {
        private static bool s_ran;
        private const string Probe3DGuid = "a1000000000000000000000000000001";
        private const string Probe2DGuid = "a2000000000000000000000000000002";

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestSimulationModeEnumValues();
            TestSimulationModeAndAutoSimulationSynchronization3D();
            TestSimulationModeAndAutoSimulationSynchronization2D();
            TestManualSimulation3DEndToEnd();
            TestManualSimulation2DEndToEnd();
        }

        private static void TestSimulationModeEnumValues()
        {
            Check.Equal("SimulationMode.FixedUpdate is 0", 0, (int)SimulationMode.FixedUpdate);
            Check.Equal("SimulationMode.Update is 1", 1, (int)SimulationMode.Update);
            Check.Equal("SimulationMode.Script is 2", 2, (int)SimulationMode.Script);

            Check.Equal("SimulationMode2D.FixedUpdate is 0", 0, (int)SimulationMode2D.FixedUpdate);
            Check.Equal("SimulationMode2D.Update is 1", 1, (int)SimulationMode2D.Update);
            Check.Equal("SimulationMode2D.Script is 2", 2, (int)SimulationMode2D.Script);
        }

        private static void TestSimulationModeAndAutoSimulationSynchronization3D()
        {
            SimulationMode savedMode = Physics.simulationMode;
            try
            {
                Physics.simulationMode = SimulationMode.FixedUpdate;
                Check.Equal("Physics.simulationMode defaults to FixedUpdate", (int)SimulationMode.FixedUpdate, (int)Physics.simulationMode);
                Check.Equal("Physics.autoSimulation is true in FixedUpdate mode", true, Physics.autoSimulation);

                Physics.autoSimulation = false;
                Check.Equal("Physics.autoSimulation = false selects SimulationMode.Script", (int)SimulationMode.Script, (int)Physics.simulationMode);
                Check.Equal("Physics.autoSimulation reads false in Script mode", false, Physics.autoSimulation);

                Physics.autoSimulation = true;
                Check.Equal("Physics.autoSimulation = true selects SimulationMode.FixedUpdate", (int)SimulationMode.FixedUpdate, (int)Physics.simulationMode);
                Check.Equal("Physics.autoSimulation reads true after re-enabling", true, Physics.autoSimulation);

                Physics.simulationMode = SimulationMode.Script;
                Check.Equal("Setting Physics.simulationMode = Script makes autoSimulation false", false, Physics.autoSimulation);

                Physics.simulationMode = SimulationMode.Update;
                Check.Equal("Physics.simulationMode stores Update", (int)SimulationMode.Update, (int)Physics.simulationMode);
                Check.Equal("Physics.autoSimulation reads true in Update mode", true, Physics.autoSimulation);
            }
            finally
            {
                Physics.simulationMode = savedMode;
            }
        }

        private static void TestSimulationModeAndAutoSimulationSynchronization2D()
        {
            SimulationMode2D savedMode = Physics2D.simulationMode;
            try
            {
                Physics2D.simulationMode = SimulationMode2D.FixedUpdate;
                Check.Equal("Physics2D.simulationMode defaults to FixedUpdate", (int)SimulationMode2D.FixedUpdate, (int)Physics2D.simulationMode);
                Check.Equal("Physics2D.autoSimulation is true in FixedUpdate mode", true, Physics2D.autoSimulation);

                Physics2D.autoSimulation = false;
                Check.Equal("Physics2D.autoSimulation = false selects SimulationMode2D.Script", (int)SimulationMode2D.Script, (int)Physics2D.simulationMode);
                Check.Equal("Physics2D.autoSimulation reads false in Script mode", false, Physics2D.autoSimulation);

                Physics2D.autoSimulation = true;
                Check.Equal("Physics2D.autoSimulation = true selects SimulationMode2D.FixedUpdate", (int)SimulationMode2D.FixedUpdate, (int)Physics2D.simulationMode);
                Check.Equal("Physics2D.autoSimulation reads true after re-enabling", true, Physics2D.autoSimulation);

                Physics2D.simulationMode = SimulationMode2D.Script;
                Check.Equal("Setting Physics2D.simulationMode = Script makes autoSimulation false", false, Physics2D.autoSimulation);

                Physics2D.simulationMode = SimulationMode2D.Update;
                Check.Equal("Physics2D.simulationMode stores Update", (int)SimulationMode2D.Update, (int)Physics2D.simulationMode);
                Check.Equal("Physics2D.autoSimulation reads true in Update mode", true, Physics2D.autoSimulation);
            }
            finally
            {
                Physics2D.simulationMode = savedMode;
            }
        }

        private static void TestManualSimulation3DEndToEnd()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_sim3d_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "sim3d.unity");
            File.WriteAllText(scenePath, Scene3DYaml);

            try
            {
                ScriptGlueRegistry.Clear();
                SimulationProbe3DScript probe = new SimulationProbe3DScript();
                ScriptGlueRegistry.Register(Probe3DGuid, () => probe);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    Rigidbody? rb = probe.gameObject.GetComponent<Rigidbody>();
                    Check.That("3D simulated body has Rigidbody", rb != null);
                    if (rb == null)
                    {
                        return;
                    }
                    VerifyScriptAndFixedModes3D(host, probe, rb);
                    VerifyUpdateMode3D(host, probe, rb);
                    host.Shutdown();
                }
            }
            finally
            {
                Physics.simulationMode = SimulationMode.FixedUpdate;
                ScriptGlueRegistry.Clear();
                LogRecorder.Clear();
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void VerifyScriptAndFixedModes3D(BridgeHost host, SimulationProbe3DScript probe, Rigidbody rb)
        {
            rb.velocity = new Vector3(0f, -5f, 0f);
            Physics.simulationMode = SimulationMode.Script;
            Check.Equal("BridgeHost reflects SimulationMode.Script for 3D", (int)SimulationMode.Script, (int)Physics.simulationMode);
            Check.Equal("Physics.autoSimulation is false in Script mode", false, Physics.autoSimulation);

            float yBeforeTicks = probe.transform.position.y;
            host.Tick(0.02);
            host.Tick(0.02);
            host.Tick(0.02);
            Check.Equal("3D body does not move during Tick in Script mode", yBeforeTicks, probe.transform.position.y);
            Check.Equal("3D trigger does not fire during Tick in Script mode", 0, probe.TriggerEnterCount);

            Physics.Simulate(0.1f);
            float yAfterSimulate = probe.transform.position.y;
            Check.That("3D body advances when Physics.Simulate is called in Script mode", yAfterSimulate < yBeforeTicks - 0.4f);
            Check.Equal("3D Rigidbody.position matches updated transform.position", yAfterSimulate, rb.position.y);
            Check.Equal("3D OnTriggerEnter callback fires during Physics.Simulate", 1, probe.TriggerEnterCount);

            Physics.simulationMode = SimulationMode.FixedUpdate;
            Check.Equal("Physics.autoSimulation is true in FixedUpdate mode", true, Physics.autoSimulation);
            LogRecorder.Clear();
            float yBeforeInvalidSim = probe.transform.position.y;
            Physics.Simulate(0.1f);
            Check.Equal("Physics.Simulate outside Script mode does not advance 3D body", yBeforeInvalidSim, probe.transform.position.y);
            Check.That("Physics.Simulate outside Script mode logs an error", HasLoggedError());
            LogRecorder.Clear();

            host.Tick(0.02);
            Check.That("3D body advances on Tick again in FixedUpdate mode", probe.transform.position.y < yBeforeInvalidSim - 0.05f);
        }

        private static void VerifyUpdateMode3D(BridgeHost host, SimulationProbe3DScript probe, Rigidbody rb)
        {
            rb.MovePosition(new Vector3(0f, 5f, 0f));
            rb.velocity = new Vector3(10f, 0f, 0f);
            Physics.simulationMode = SimulationMode.Update;
            Check.Equal("BridgeHost reflects SimulationMode.Update for 3D", (int)SimulationMode.Update, (int)Physics.simulationMode);

            float xBeforeUpdateTick = probe.transform.position.x;
            host.Tick(0.03);
            float xDelta = probe.transform.position.x - xBeforeUpdateTick;
            Check.Equal("3D body in Update mode advances by frame delta (10 * 0.03 = 0.3)", 0.3f, xDelta);
            Physics.simulationMode = SimulationMode.FixedUpdate;
        }

        private static void TestManualSimulation2DEndToEnd()
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "unitas_sim2d_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "sim2d.unity");
            File.WriteAllText(scenePath, Scene2DYaml);

            try
            {
                ScriptGlueRegistry.Clear();
                SimulationProbe2DScript probe = new SimulationProbe2DScript();
                ScriptGlueRegistry.Register(Probe2DGuid, () => probe);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    Rigidbody2D? rb2d = probe.gameObject.GetComponent<Rigidbody2D>();
                    Check.That("2D simulated body has Rigidbody2D", rb2d != null);
                    if (rb2d == null)
                    {
                        return;
                    }
                    VerifyScriptAndFixedModes2D(host, probe, rb2d);
                    VerifyUpdateMode2D(host, probe, rb2d);
                    host.Shutdown();
                }
            }
            finally
            {
                Physics2D.simulationMode = SimulationMode2D.FixedUpdate;
                ScriptGlueRegistry.Clear();
                LogRecorder.Clear();
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void VerifyScriptAndFixedModes2D(BridgeHost host, SimulationProbe2DScript probe, Rigidbody2D rb2d)
        {
            rb2d.velocity = new Vector2(0f, -5f);
            Physics2D.simulationMode = SimulationMode2D.Script;
            Check.Equal("BridgeHost reflects SimulationMode2D.Script for 2D", (int)SimulationMode2D.Script, (int)Physics2D.simulationMode);
            Check.Equal("Physics2D.autoSimulation is false in Script mode", false, Physics2D.autoSimulation);

            float yBeforeTicks = probe.transform.position.y;
            host.Tick(0.02);
            host.Tick(0.02);
            Check.Equal("2D body does not move during Tick in Script mode", yBeforeTicks, probe.transform.position.y);
            Check.Equal("2D trigger does not fire during Tick in Script mode", 0, probe.TriggerEnter2DCount);

            bool simulated = Physics2D.Simulate(0.1f);
            Check.Equal("Physics2D.Simulate returns true in Script mode", true, simulated);
            Check.That("2D body advances when Physics2D.Simulate is called in Script mode", probe.transform.position.y < yBeforeTicks - 0.4f);
            Check.Equal("2D OnTriggerEnter2D callback fires during Physics2D.Simulate", 1, probe.TriggerEnter2DCount);

            Physics2D.simulationMode = SimulationMode2D.FixedUpdate;
            Check.Equal("Physics2D.autoSimulation is true in FixedUpdate mode", true, Physics2D.autoSimulation);
            LogRecorder.Clear();
            float yBeforeInvalidSim = probe.transform.position.y;
            bool invalidSim = Physics2D.Simulate(0.1f);
            Check.Equal("Physics2D.Simulate returns false outside Script mode", false, invalidSim);
            Check.Equal("Physics2D.Simulate outside Script mode does not advance 2D body", yBeforeInvalidSim, probe.transform.position.y);
            Check.That("Physics2D.Simulate outside Script mode logs an error", HasLoggedError());
            LogRecorder.Clear();

            host.Tick(0.02);
            Check.That("2D body advances on Tick again in FixedUpdate mode", probe.transform.position.y < yBeforeInvalidSim - 0.05f);
        }

        private static void VerifyUpdateMode2D(BridgeHost host, SimulationProbe2DScript probe, Rigidbody2D rb2d)
        {
            rb2d.MovePosition(new Vector2(0f, 5f));
            rb2d.velocity = new Vector2(10f, 0f);
            Physics2D.simulationMode = SimulationMode2D.Update;
            Check.Equal("BridgeHost reflects SimulationMode2D.Update for 2D", (int)SimulationMode2D.Update, (int)Physics2D.simulationMode);

            float xBeforeUpdateTick = probe.transform.position.x;
            host.Tick(0.03);
            float xDelta = probe.transform.position.x - xBeforeUpdateTick;
            Check.Equal("2D body in Update mode advances by frame delta (10 * 0.03 = 0.3)", 0.3f, xDelta);
            Physics2D.simulationMode = SimulationMode2D.FixedUpdate;
        }

        private static bool HasLoggedError()
        {
            for (int i = 0; i < LogRecorder.Entries.Count; i++)
            {
                if (LogRecorder.Entries[i].Severity == LogSeverity.Error)
                {
                    return true;
                }
            }
            return false;
        }

        private sealed class SimulationProbe3DScript : MonoBehaviour
        {
            public int TriggerEnterCount { get; private set; }

            public override void OnTriggerEnter(Collider? other)
            {
                TriggerEnterCount++;
            }
        }

        private sealed class SimulationProbe2DScript : MonoBehaviour
        {
            public int TriggerEnter2DCount { get; private set; }

            public override void OnTriggerEnter2D(Collider2D? other)
            {
                TriggerEnter2DCount++;
            }
        }

        private const string Scene3DYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: SimBody3D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 2, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!54 &102
Rigidbody:
  m_GameObject: {fileID: 100}
  m_Mass: 1
  m_UseGravity: 0
  m_IsKinematic: 0
--- !u!135 &103
SphereCollider:
  m_GameObject: {fileID: 100}
  m_Radius: 0.5
  m_IsTrigger: 0
  m_Center: {x: 0, y: 0, z: 0}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: a1000000000000000000000000000001, type: 3}
--- !u!1 &200
GameObject:
  m_Name: SimTrigger3D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1.4, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!65 &202
BoxCollider:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 1
  m_Size: {x: 2, y: 0.5, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
";

        private const string Scene2DYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: SimBody2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 2, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
--- !u!58 &103
CircleCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Radius: 0.5
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: a2000000000000000000000000000002, type: 3}
--- !u!1 &200
GameObject:
  m_Name: SimTrigger2D
  m_Layer: 0
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1.4, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
--- !u!61 &202
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 2, y: 0.5}
";
    }

    internal static class PhysicsSimulationTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsTargetShard(args))
            {
                return;
            }
            PhysicsSimulationTests.Run();
        }

        private static bool IsPerfMode(string[] args)
        {
            for (int i = 0; i < args.Length; i++)
            {
                if (string.Equals(args[i], "--perf", StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool IsTargetShard(string[] args)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (!string.Equals(args[i], "--shard", StringComparison.Ordinal))
                {
                    continue;
                }
                string[] parts = args[i + 1].Split('/');
                return parts.Length == 2 && string.Equals(parts[0], "5", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
