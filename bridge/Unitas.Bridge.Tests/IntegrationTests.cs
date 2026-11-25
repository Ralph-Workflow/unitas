using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public class LoggingScript : MonoBehaviour
    {
        public int UpdateCount { get; private set; }

        public float ObservedDeltaTime { get; private set; }

        public int FixedUpdateCount { get; private set; }

        public override void FixedUpdate()
        {
            FixedUpdateCount++;
        }

        public override void Update()
        {
            UpdateCount++;
            ObservedDeltaTime = Time.deltaTime;
            Debug.Log("scripted update " + UpdateCount.ToString(CultureInfo.InvariantCulture));
        }
    }

    public class PassiveScript : MonoBehaviour
    {
    }

    public class MotionRecorderScript : MonoBehaviour
    {
        private readonly List<float> _positionY = new List<float>();
        private readonly List<float> _velocityY = new List<float>();

        public IReadOnlyList<float> PositionY => _positionY;

        public IReadOnlyList<float> VelocityY => _velocityY;

        public override void FixedUpdate()
        {
            _positionY.Add(transform.position.y);
            Rigidbody? body = GetComponent<Rigidbody>();
            _velocityY.Add(body == null ? 0f : body.velocity.y);
        }
    }

    public class VelocityWriterScript : MonoBehaviour
    {
        private readonly List<float> _positionY = new List<float>();
        private bool _written;

        public IReadOnlyList<float> PositionY => _positionY;

        public Vector3 ReadBackVelocity { get; private set; }

        public bool Wrote { get; private set; }

        public override void FixedUpdate()
        {
            _positionY.Add(transform.position.y);
            Rigidbody? body = GetComponent<Rigidbody>();
            if (_written || body == null)
            {
                return;
            }
            body.velocity = new Vector3(0f, 5f, 0f);
            ReadBackVelocity = body.velocity;
            _written = true;
            Wrote = true;
        }
    }

    public class SphereRestRecorderScript : MonoBehaviour
    {
        private readonly List<float> _positionY = new List<float>();

        public IReadOnlyList<float> PositionY => _positionY;

        public override void FixedUpdate()
        {
            _positionY.Add(transform.position.y);
        }
    }

    public class PlatformMoverScript : MonoBehaviour
    {
        private const float RisePerStep = 0.05f;
        private int _steps;

        public float PlatformY { get; private set; }

        public override void FixedUpdate()
        {
            _steps++;
            transform.position = new Vector3(0f, RisePerStep * _steps, 0f);
            PlatformY = transform.position.y;
        }
    }

    public static class IntegrationTests
    {
        public static void Run()
        {
            ScriptedUpdateLogsStructuredEntryAndSphereCastHits();
            TimeScaleReachesTheNativeFixedStep();
            FixedDeltaTimeFollowsProjectSettingsOnLoadScene();
            BallFallsUnderGravityAndComesToRestOnFloor();
            ScriptSetVelocityChangesNativeTrajectory();
            KinematicPlatformCarriesRestingDynamicBody();
        }

        private static void BallFallsUnderGravityAndComesToRestOnFloor()
        {
            string baseScenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            Check.That("Base collision scene file exists", File.Exists(baseScenePath));

            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_ball_falls_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string tempScenePath = Path.Combine(tempDir, "ball_falls.unity");
            string yaml = File.ReadAllText(baseScenePath)
                .Replace(
                    "  m_LocalPosition: {x: 0.9999999, y: 0.499, z: 0}",
                    "  m_LocalPosition: {x: 0, y: 2, z: 0}",
                    StringComparison.Ordinal)
                .Replace(
                    "  m_Size: {x: 1, y: 1.5, z: 1}",
                    "  m_Size: {x: 4, y: 1, z: 4}",
                    StringComparison.Ordinal);
            File.WriteAllText(tempScenePath, yaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptGlueRegistry.Clear();
                MotionRecorderScript? recorder = null;
                ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recorder = new MotionRecorderScript());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScenePath);
                    host.Tick(0.0);

                    Check.That("Ball script created on load", recorder != null);
                    Rigidbody? attachedBody = recorder!.GetComponent<Rigidbody>();
                    Check.That("Rigidbody parsed and attached on scene load", attachedBody != null);
                    Check.Equal("Parsed Rigidbody default isKinematic is false", false, attachedBody!.isKinematic);
                    Check.Equal("Initial ball transform position y is 2", 2f, recorder.transform.position.y);
                    Check.Equal("Initial ball velocity y is 0", 0f, attachedBody.velocity.y);

                    for (int tick = 0; tick < 60; tick++)
                    {
                        host.Tick(0.02);
                    }

                    IReadOnlyList<float> posHistory = recorder.PositionY;
                    IReadOnlyList<float> velHistory = recorder.VelocityY;
                    Check.That("FixedUpdate recorded motion samples", posHistory.Count >= 40);
                    Check.That("Ball falls downward on first fixed step", posHistory[0] < 2f && posHistory[0] > 1f);
                    Check.That("Ball velocity is negative while falling", velHistory[0] < -0.1f);
                    Check.That("Ball continues falling before impact", posHistory[5] < posHistory[0]);
                    Check.That("Ball downward speed increases while falling", velHistory[5] < velHistory[0]);

                    Check.Equal("Ball settles on floor at y = 1", 1f, recorder.transform.position.y);
                    Check.Equal("Ball velocity settles to 0 at rest", 0f, attachedBody.velocity.y);
                    Check.Equal("Last recorded FixedUpdate position y is 1", 1f, posHistory[posHistory.Count - 1]);
                    Check.Equal("Last recorded FixedUpdate velocity y is 0", 0f, velHistory[velHistory.Count - 1]);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void ScriptSetVelocityChangesNativeTrajectory()
        {
            string baseScenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            Check.That("Base collision scene file exists for velocity test", File.Exists(baseScenePath));

            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_velocity_push_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string tempScenePath = Path.Combine(tempDir, "velocity_push.unity");
            string yaml = File.ReadAllText(baseScenePath)
                .Replace(
                    "  m_LocalPosition: {x: 0.9999999, y: 0.499, z: 0}",
                    "  m_LocalPosition: {x: 0, y: 2, z: 0}",
                    StringComparison.Ordinal)
                .Replace(
                    "  m_Size: {x: 1, y: 1.5, z: 1}",
                    "  m_Size: {x: 4, y: 1, z: 4}",
                    StringComparison.Ordinal);
            File.WriteAllText(tempScenePath, yaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptGlueRegistry.Clear();
                VelocityWriterScript? writer = null;
                ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => writer = new VelocityWriterScript());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScenePath);
                    host.Tick(0.0);

                    Check.That("Ball script created on load", writer != null);
                    Rigidbody? attachedBody = writer!.GetComponent<Rigidbody>();
                    Check.That("Ball Rigidbody attached on load", attachedBody != null);
                    Check.Equal("Initial ball velocity y is 0", 0f, attachedBody!.velocity.y);

                    for (int tick = 0; tick < 30; tick++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That("Script authored a velocity write", writer!.Wrote);
                    Check.Equal("Velocity getter reads back the pushed value", 5f, writer!.ReadBackVelocity.y);

                    IReadOnlyList<float> posHistory = writer!.PositionY;
                    Check.That("FixedUpdate recorded motion samples", posHistory.Count >= 25);
                    Check.That("Ball rises after the scripted velocity write", posHistory[1] > posHistory[0]);

                    float highest = posHistory[0];
                    foreach (float sample in posHistory)
                    {
                        if (sample > highest)
                        {
                            highest = sample;
                        }
                    }
                    Check.That("Script-set velocity changes the native trajectory", highest > 2.05f);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void KinematicPlatformCarriesRestingDynamicBody()
        {
            string baseScenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            Check.That("Base collision scene file exists for kinematic platform test", File.Exists(baseScenePath));

            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_kinematic_platform_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string tempScenePath = Path.Combine(tempDir, "kinematic_platform.unity");
            string yaml = File.ReadAllText(baseScenePath)
                .Replace(
                    "  m_LocalPosition: {x: 0.9999999, y: 0.499, z: 0}",
                    "  m_LocalPosition: {x: 0, y: 0.75, z: 0}",
                    StringComparison.Ordinal)
                .Replace(
                    "  m_Size: {x: 1, y: 1.5, z: 1}",
                    "  m_Size: {x: 2, y: 0.5, z: 2}",
                    StringComparison.Ordinal)
                .Replace(
                    "  - component: {fileID: 1311639327}\n",
                    "  - component: {fileID: 1311639327}\n  - component: {fileID: 1311639328}\n  - component: {fileID: 1311639329}\n",
                    StringComparison.Ordinal)
                + KinematicPlatformDocuments;
            File.WriteAllText(tempScenePath, yaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptGlueRegistry.Clear();
                SphereRestRecorderScript? recorder = null;
                PlatformMoverScript? mover = null;
                ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recorder = new SphereRestRecorderScript());
                ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => mover = new PlatformMoverScript());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScenePath);
                    host.Tick(0.0);

                    Check.That("Resting sphere script created", recorder != null);
                    Check.That("Kinematic platform script created", mover != null);
                    Rigidbody? platformBody = mover!.GetComponent<Rigidbody>();
                    Check.That("Kinematic platform Rigidbody attached on load", platformBody != null);
                    Check.Equal("Parsed platform Rigidbody isKinematic is true", true, platformBody!.isKinematic);

                    for (int tick = 0; tick < 30; tick++)
                    {
                        host.Tick(0.02);
                    }

                    IReadOnlyList<float> sphereY = recorder!.PositionY;
                    Check.That("FixedUpdate recorded sphere samples", sphereY.Count >= 25);
                    Check.That("Kinematic platform rose across fixed updates", mover!.PlatformY > 1f);

                    float firstY = sphereY[0];
                    float lastY = sphereY[sphereY.Count - 1];
                    Check.That("Dynamic sphere is carried upward by the platform", lastY > firstY + 0.5f);
                    Check.That(
                        "Carried sphere stays in contact with the platform",
                        lastY > mover!.PlatformY + 0.25f && lastY < mover.PlatformY + 0.85f);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private const string KinematicPlatformDocuments = @"
--- !u!54 &1311639328
Rigidbody:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 519420022}
  m_Mass: 1
  m_UseGravity: 1
  m_IsKinematic: 1
--- !u!114 &1311639329
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  m_GameObject: {fileID: 519420022}
  m_Enabled: 1
  m_EditorHideFlags: 0
  m_Script: {fileID: 11500000, guid: 22222222333344445555666677778888, type: 3}
  m_Name:
  m_EditorClassIdentifier:
";

        private static void FixedDeltaTimeFollowsProjectSettingsOnLoadScene()
        {
            string settingsScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/project_with_settings/Assets/Scenes/SettingsScene.unity");
            string plainScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Settings scene file exists", File.Exists(settingsScene));
            Check.That("Plain scene file exists", File.Exists(plainScene));

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptGlueRegistry.Clear();
                LoggingScript? settingsScript = null;
                ScriptGlueRegistry.Register(0UL, 1UL, () => settingsScript = new LoggingScript());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(settingsScene);
                    float expectedFixed = (float)0.03333333333333333;
                    Check.Equal("fixedDeltaTime reflects TimeManager after LoadScene", expectedFixed, Time.fixedDeltaTime);
                    Check.Equal("fixedUnscaledDeltaTime reflects TimeManager after LoadScene", expectedFixed, Time.fixedUnscaledDeltaTime);

                    host.Tick(0.0);
                    host.Tick(0.04);
                    Check.Equal("Tick(0.04) runs 1 FixedUpdate at 0.0333s timestep", 1, settingsScript == null ? -1 : settingsScript.FixedUpdateCount);

                    host.LoadScene(plainScene);
                    Check.Equal("fixedDeltaTime resets to 0.02 without ProjectSettings", 0.02f, Time.fixedDeltaTime);
                    Check.Equal("fixedUnscaledDeltaTime resets to 0.02 without ProjectSettings", 0.02f, Time.fixedUnscaledDeltaTime);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TimeScaleReachesTheNativeFixedStep()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Time scale scene file exists", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            try
            {
                int fullSteps = CountFixedSteps(scenePath, 1f);
                int halfSteps = CountFixedSteps(scenePath, 0.5f);
                Check.Equal("native fixed steps at timeScale one", 10, fullSteps);
                Check.Equal("native fixed steps at timeScale half", 5, halfSteps);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static int CountFixedSteps(string scenePath, float scale)
        {
            Time.timeScale = scale;
            ScriptGlueRegistry.Clear();
            LoggingScript? target = null;
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => target = new LoggingScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 11; tick++)
                {
                    host.Tick(0.02);
                }
            }

            return target == null ? 0 : target.FixedUpdateCount;
        }

        private static void ScriptedUpdateLogsStructuredEntryAndSphereCastHits()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Integration scene file exists", File.Exists(scenePath));

            LogRecorder.Clear();
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            int frameBefore = Time.frameCount;
            ScriptGlueRegistry.Clear();
            LoggingScript? target = null;
            PassiveScript? other = null;
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => target = new LoggingScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => other = new PassiveScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 11; tick++)
                {
                    host.Tick(0.02);
                }

                Check.That("Scripted Update ran during ticks", target != null && target.UpdateCount == 10);
                Check.Equal("Scripted Update observed the frame delta", 0.02f, target!.ObservedDeltaTime);
                Check.Equal("Time.frameCount advanced with the ticks", frameBefore + 11, Time.frameCount);

                IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
                Check.That("Scripted Update produced a log entry", entries.Count == 10);
                LogEntry entry = entries[entries.Count - 1];
                Check.That("Log entry keeps the severity", entry.Severity == LogSeverity.Info);
                Check.That("Log entry keeps the frame", entry.Frame >= 1);
                Check.That("Log entry keeps the text", entry.Message.Contains("scripted update", StringComparison.Ordinal));
                Check.That("Log entry keeps a non-empty stack", !string.IsNullOrEmpty(entry.CallStack));
                Check.That("Log entry keeps the scripted origin", ReferenceEquals(entry.Origin, target));
                Check.That("Log entry keeps the elapsed time", entry.Time > 0f);
                Check.Equal("Log search finds the scripted message", 10, LogRecorder.Search("SCRIPTED UPDATE").Count);

                Vector3 direction = Vector3.up;
                bool cast = Physics.SphereCast(
                    new Vector3(0f, -5f, 0f),
                    direction,
                    0.5f,
                    out RaycastHit hit,
                    100f,
                    1 << 5);
                Check.That("Native sphere cast hits the layer 5 collider", cast);
                Check.That("Native sphere cast resolves a collider", hit.collider != null);
                Check.That(
                    "Native sphere cast hit belongs to the nearer collider",
                    ReferenceEquals(hit.collider, target!.GetComponent<Collider>()));
                Check.That("Native sphere cast reports a distance", hit.distance > 0f && hit.distance <= 5f);
                Check.That("Native sphere cast reports a contact point", hit.point.magnitude > 0f);
                Check.That("Native sphere cast reports a normal", hit.normal.magnitude > 0f);
                Check.That(
                    "Native sphere cast normal faces the incoming ray",
                    Vector3.Dot(hit.normal, direction) < 0f);

                bool excluded = Physics.SphereCast(
                    new Vector3(0f, -5f, 0f),
                    direction,
                    0.5f,
                    out RaycastHit excludedHit,
                    100f,
                    ~(1 << 5));
                Check.That("Native sphere cast misses an excluded layer", !excluded);
                Check.That("Native sphere cast miss leaves the hit empty", excludedHit.collider == null);

                bool ordered = Physics.SphereCast(
                    new Vector3(-5f, 0f, 0f),
                    Vector3.right,
                    0.5f,
                    out RaycastHit orderedHit,
                    100f,
                    ~0);
                Check.That("Native sphere cast returns the nearest hit across layers", ordered);
                Check.That(
                    "Native sphere cast orders hits by distance",
                    ReferenceEquals(orderedHit.collider, target!.GetComponent<Collider>()));

                host.Shutdown();
            }

            Time.timeScale = savedScale;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
