using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.CompilerServices;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Unitas.Bridge.Tests
{
    public static class InputSystemRebindTests
    {
        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestRebindCompletesOnInjectedControlActivity();
            TestRebindCancelLeavesBindingsUnchanged();
            TestRebindTimesOut();
            TestExcludedControlIsRejected();
            TestBindingOverridePersistsOnAction();
        }

        private static void TestRebindCompletesOnInjectedControlActivity()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                RebindOperation operation = RebindOperation.Start(jump, 0);
                RebindStatus completedStatus = RebindStatus.Waiting;
                operation.completed += op => { completedStatus = op.status; };

                Check.That("Rebind starts in Waiting", operation.status == RebindStatus.Waiting);
                Check.That("Rebind is not done while waiting", !operation.isDone);
                Check.That("Rebind has no bound control while waiting", operation.boundControl == null);

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                Check.That("Rebind completes on injected key activity", operation.status == RebindStatus.Completed);
                Check.That("Rebind completion event fired", completedStatus == RebindStatus.Completed);
                Check.That("Rebind is done after completing", operation.isDone);
                Check.That("Rebind captured the space key", ReferenceEquals(operation.boundControl, Keyboard.current!.spaceKey));
                Check.Equal("Rebind reports the bound path", "/Keyboard/space", operation.boundPath);

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);
            }
        }

        private static void TestRebindCancelLeavesBindingsUnchanged()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                string before = SnapshotBindings(jump);
                RebindOperation operation = RebindOperation.Start(jump, 0);
                RebindStatus canceledStatus = RebindStatus.Waiting;
                operation.canceled += op => { canceledStatus = op.status; };

                operation.Cancel();

                Check.That("Cancelled rebind reports Canceled", operation.status == RebindStatus.Canceled);
                Check.That("Cancelled rebind is done", operation.isDone);
                Check.That("Cancelled rebind has no bound control", operation.boundControl == null);
                Check.That("Cancelled rebind fired the canceled event", canceledStatus == RebindStatus.Canceled);
                Check.Equal("Cancelled rebind left the bindings unchanged", before, SnapshotBindings(jump));

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                Check.That("Cancelled rebind ignores later activity", operation.status == RebindStatus.Canceled);
                Check.Equal("Cancelled rebind left the bindings unchanged after activity", before, SnapshotBindings(jump));

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);
            }
        }

        private static void TestRebindTimesOut()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                string before = SnapshotBindings(jump);
                RebindOperation operation = RebindOperation.Start(jump, 0, 0.1f);
                RebindStatus timedOutStatus = RebindStatus.Waiting;
                operation.timedOut += op => { timedOutStatus = op.status; };

                for (int i = 0; i < 10 && !operation.isDone; i++)
                {
                    host.Tick(0.02);
                }

                Check.That("Rebind times out", operation.status == RebindStatus.TimedOut);
                Check.That("Rebind timeout event fired", timedOutStatus == RebindStatus.TimedOut);
                Check.That("Timed out rebind has no bound control", operation.boundControl == null);
                Check.Equal("Timed out rebind left the bindings unchanged", before, SnapshotBindings(jump));

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                Check.That("Timed out rebind ignores later activity", operation.status == RebindStatus.TimedOut);
                Check.Equal("Timed out rebind left the bindings unchanged after activity", before, SnapshotBindings(jump));

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);
            }
        }

        private static void TestExcludedControlIsRejected()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                string before = SnapshotBindings(jump);
                RebindOperation operation = RebindOperation.Start(jump, 0, 0f, "<Keyboard>/space");
                Check.That("Rebind with an exclusion starts in Waiting", operation.status == RebindStatus.Waiting);

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                Check.That("Excluded control does not complete the rebind", operation.status == RebindStatus.Waiting);
                Check.That("Excluded control is not bound", operation.boundControl == null);
                Check.Equal("Excluded control left the bindings unchanged", before, SnapshotBindings(jump));

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);

                host.InjectKey(KeyCode.W, true);
                host.Tick(0.02);

                Check.That("Rebind completes with a control that is not excluded", operation.status == RebindStatus.Completed);
                Check.That("Rebind captured the w key", ReferenceEquals(operation.boundControl, Keyboard.current!.wKey));
                Check.Equal("Rebind applied the binding override", "/Keyboard/w", jump.bindings[0].overridePath);

                host.InjectKey(KeyCode.W, false);
                host.Tick(0.02);
            }
        }

        private static void TestBindingOverridePersistsOnAction()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                RebindOperation operation = RebindOperation.Start(jump, 0);

                host.InjectKey(KeyCode.W, true);
                host.Tick(0.02);

                Check.That("Rebind completed for the override test", operation.status == RebindStatus.Completed);

                InputBinding binding = jump.bindings[0];
                Check.Equal("Override persists on the binding", "/Keyboard/w", binding.overridePath);
                Check.Equal("Effective path follows the override", "/Keyboard/w", binding.effectivePath);
                Check.Equal("Original binding path is untouched", "<Keyboard>/space", binding.path);
                Check.Equal("Action resolves one control", 1, jump.controls.Count);
                Check.That("Action resolves the overridden control", ReferenceEquals(jump.controls[0], Keyboard.current!.wKey));

                jump.Enable();
                host.Tick(0.02);

                Check.That("Enabled action reads the overridden control", ReferenceEquals(jump.activeControl, Keyboard.current!.wKey));
                Check.Equal("Override persists across frames", "/Keyboard/w", jump.bindings[0].overridePath);

                jump.Disable();
                host.InjectKey(KeyCode.W, false);
                host.Tick(0.02);
            }
        }

        private static string SnapshotBindings(InputAction action)
        {
            List<string> entries = new List<string>();
            for (int i = 0; i < action.bindings.Count; i++)
            {
                InputBinding binding = action.bindings[i];
                entries.Add(binding.path + "|" + binding.overridePath);
            }
            return string.Join(";", entries);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
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
                return parts.Length == 2 && string.Equals(parts[0], "6", StringComparison.Ordinal);
            }
            return true;
        }
    }

    internal static class InputSystemRebindTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsTargetShard(args))
            {
                return;
            }
            InputSystemRebindTests.Run();
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
                return parts.Length == 2 && string.Equals(parts[0], "6", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
