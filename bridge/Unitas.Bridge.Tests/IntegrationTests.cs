using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
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

    public static class IntegrationTests
    {
        public static void Run()
        {
            ScriptedUpdateLogsStructuredEntryAndSphereCastHits();
            TimeScaleReachesTheNativeFixedStep();
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
            ScriptRegistry.Clear();
            LoggingScript? target = null;
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => target = new LoggingScript());

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
            ScriptRegistry.Clear();
            LoggingScript? target = null;
            PassiveScript? other = null;
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => target = new LoggingScript());
            ScriptRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => other = new PassiveScript());

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
