using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using Unitas.Bridge;

namespace Unitas.Bridge.Tests
{
    public sealed class ThrowingUpdateScript : MonoBehaviour
    {
        public int UpdateCount;

        public override void Update()
        {
            UpdateCount++;
            throw new InvalidOperationException("script failure");
        }
    }

    public sealed class SiblingCountingScript : MonoBehaviour
    {
        public int UpdateCount;

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public static class ExceptionContainmentTests
    {
        public static void Run()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Exception containment scene file exists", File.Exists(scenePath));

            LogRecorder.Clear();
            ScriptRegistry.Clear();
            ThrowingUpdateScript throwing = new ThrowingUpdateScript();
            ScriptRegistry.Register(0UL, 1UL, () => throwing);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);
                host.Tick(0.02);
                host.Tick(0.02);
                host.Shutdown();
            }

            IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
            Check.That("the throwing script ran its update", throwing.UpdateCount > 0);
            Check.Equal("one error entry per thrown update", throwing.UpdateCount, errors.Count);
            for (int i = 0; i < errors.Count; i++)
            {
                Check.That("error entry carries a stack trace", !string.IsNullOrEmpty(errors[i].CallStack));
                Check.That("error entry carries the throwing script as origin", ReferenceEquals(errors[i].Origin, throwing));
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
