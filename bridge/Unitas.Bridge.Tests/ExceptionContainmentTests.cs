using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;

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
        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public sealed class ThrowingAwakeScript : MonoBehaviour
    {
        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public override void Awake()
        {
            throw new InvalidOperationException("awake failure");
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public sealed class InactiveSelfScript : MonoBehaviour
    {
        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public override void Awake()
        {
            gameObject.SetActive(false);
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public sealed class SelfDestroyingScript : MonoBehaviour
    {
        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public int DestroyCount;

        public override void Awake()
        {
            UnityEngine.Object.Destroy(this);
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }

        public override void OnDestroy()
        {
            DestroyCount++;
        }
    }

    public sealed class DisabledSelfScript : MonoBehaviour
    {
        public int AwakeCount;

        public int EnableCount;

        public int StartCount;

        public int UpdateCount;

        public override void Awake()
        {
            AwakeCount++;
            enabled = false;
        }

        public override void OnEnable()
        {
            EnableCount++;
        }

        public override void Start()
        {
            StartCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public static class ExceptionContainmentTests
    {
        private const int TickCount = 3;
        private const int UpdateTickCount = TickCount - 1;

        public static void Run()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_lifecycle_scripts.unity");
            Check.That("Exception containment scene file exists", File.Exists(scenePath));

            LogRecorder.Clear();
            ScriptGlueRegistry.Clear();

            ThrowingUpdateScript throwing = new ThrowingUpdateScript();
            SiblingCountingScript sibling = new SiblingCountingScript();
            ThrowingAwakeScript awakeThrowing = new ThrowingAwakeScript();
            InactiveSelfScript inactive = new InactiveSelfScript();
            SelfDestroyingScript destroyed = new SelfDestroyingScript();
            DisabledSelfScript disabled = new DisabledSelfScript();

            ScriptGlueRegistry.Register("33333333333333333333333333333333", () => throwing);
            ScriptGlueRegistry.Register("44444444444444444444444444444444", () => sibling);
            ScriptGlueRegistry.Register("55555555555555555555555555555555", () => awakeThrowing);
            ScriptGlueRegistry.Register("66666666666666666666666666666666", () => inactive);
            ScriptGlueRegistry.Register("77777777777777777777777777777777", () => destroyed);
            ScriptGlueRegistry.Register("88888888888888888888888888888888", () => disabled);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < TickCount; tick++)
                {
                    host.Tick(0.02);
                }

                host.Shutdown();
            }

            IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
            List<LogEntry> throwingErrors = new List<LogEntry>();
            List<LogEntry> awakeErrors = new List<LogEntry>();
            int throwingScriptOriginErrors = 0;
            for (int i = 0; i < errors.Count; i++)
            {
                LogEntry entry = errors[i];
                Check.That("error entry carries a stack trace", !string.IsNullOrEmpty(entry.CallStack));
                if (ReferenceEquals(entry.Origin, throwing))
                {
                    throwingErrors.Add(entry);
                    throwingScriptOriginErrors++;
                }
                else if (ReferenceEquals(entry.Origin, awakeThrowing))
                {
                    awakeErrors.Add(entry);
                    throwingScriptOriginErrors++;
                }
            }

            for (int i = 0; i < throwingErrors.Count; i++)
            {
                Check.That("error entry carries the throwing script as origin", ReferenceEquals(throwingErrors[i].Origin, throwing));
            }

            Check.That("the throwing script ran its update", throwing.UpdateCount > 0);
            Check.Equal("one error entry per thrown update", throwing.UpdateCount, throwingErrors.Count);
            Check.Equal("every error entry carries a throwing script as origin", errors.Count, throwingScriptOriginErrors);
            Check.Equal("one error entry for the awake throw", 1, awakeErrors.Count);
            Check.That("the awake error entry carries a stack trace", awakeErrors.Count == 1 && !string.IsNullOrEmpty(awakeErrors[0].CallStack));
            Check.That("the awake error entry carries the awake-throwing script as origin", awakeErrors.Count == 1 && ReferenceEquals(awakeErrors[0].Origin, awakeThrowing));

            Check.Equal("the sibling updates once per update tick", UpdateTickCount, sibling.UpdateCount);
            Check.Equal("the sibling update count matches the throwing script", throwing.UpdateCount, sibling.UpdateCount);
            Check.Equal("the eligible sibling receives OnEnable", 1, sibling.EnableCount);
            Check.Equal("the eligible sibling receives Start", 1, sibling.StartCount);
            Check.Equal("the awake-throwing script keeps updating after the awake throw", UpdateTickCount, awakeThrowing.UpdateCount);
            Check.Equal("the awake-throwing script receives OnEnable", 1, awakeThrowing.EnableCount);
            Check.Equal("the awake-throwing script receives Start", 1, awakeThrowing.StartCount);
            Check.Equal("the inactive script receives no OnEnable", 0, inactive.EnableCount);
            Check.Equal("the inactive script receives no Start", 0, inactive.StartCount);
            Check.Equal("the inactive script receives no Update", 0, inactive.UpdateCount);
            Check.Equal("the destroyed script receives no OnEnable", 0, destroyed.EnableCount);
            Check.Equal("the destroyed script receives no Start", 0, destroyed.StartCount);
            Check.Equal("the destroyed script receives no Update", 0, destroyed.UpdateCount);
            Check.Equal("the destroyed script receives OnDestroy exactly once", 1, destroyed.DestroyCount);
            Check.Equal("the disabled script receives Awake", 1, disabled.AwakeCount);
            Check.Equal("the disabled script receives no OnEnable", 0, disabled.EnableCount);
            Check.Equal("the disabled script receives no Start", 0, disabled.StartCount);
            Check.Equal("the disabled script receives no Update", 0, disabled.UpdateCount);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
