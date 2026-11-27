using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using Unity.Profiling;
using UnityEngine;
using UnityEngine.LowLevel;
using UnityEngine.Profiling;

namespace Unitas.Bridge.Tests
{
    public sealed class ProfilerProbeScript : MonoBehaviour
    {
        private static readonly Unity.Profiling.ProfilerMarker FixedInnerMarker =
            new Unity.Profiling.ProfilerMarker("Script.FixedInner");

        private static readonly UnityEngine.Profiling.ProfilerMarker UpdateInnerMarker =
            new UnityEngine.Profiling.ProfilerMarker("Script.UpdateInner");

        public override void FixedUpdate()
        {
            Profiler.BeginSample("Script.FixedOuter");
            using (FixedInnerMarker.Auto())
            {
            }

            Profiler.EndSample();
        }

        public override void Update()
        {
            Profiler.BeginSample("Script.UpdateOuter", this);
            using (UpdateInnerMarker.Auto())
            {
            }

            Profiler.EndSample();
        }
    }

    public static class ProfilerTests
    {
        private const string SceneFixture = "crates/unitas-runtime/tests/fixtures/scene_with_script.unity";

        private const string InitializationPhase = "UnityEngine.PlayerLoop.Initialization";
        private const string EarlyUpdatePhase = "UnityEngine.PlayerLoop.EarlyUpdate";
        private const string FixedUpdatePhase = "UnityEngine.PlayerLoop.FixedUpdate";
        private const string PreUpdatePhase = "UnityEngine.PlayerLoop.PreUpdate";
        private const string UpdatePhase = "UnityEngine.PlayerLoop.Update";
        private const string PreLateUpdatePhase = "UnityEngine.PlayerLoop.PreLateUpdate";
        private const string PostLateUpdatePhase = "UnityEngine.PlayerLoop.PostLateUpdate";

        private const string ScriptRunBehaviourUpdate = "UnityEngine.PlayerLoop.Update.ScriptRunBehaviourUpdate";
        private const string CustomUpdateType = "MyGame.PlayerLoop.ProfiledCustomUpdate";

        public static void Run()
        {
            RuntimeInterop.InstallProfilerHost();
            TestTickRecordsPlayerLoopPhasesInOrder();
            TestCustomPlayerLoopSystemSamplesNestUnderPhase();
            TestMonoBehaviourSamplesNestInCapture();
        }

        private static void TestTickRecordsPlayerLoopPhasesInOrder()
        {
            using (BridgeHost host = new BridgeHost())
            {
                host.Tick(0.02);

                Profiler.StartCapture();
                Check.Equal("Profiler.enabled is true during capture", true, Profiler.enabled);
                host.Tick(0.02);
                Profiler.StopCapture();
                Check.Equal("Profiler.enabled is false after capture stops", false, Profiler.enabled);

                ProfilerCapture capture = Profiler.GetCapture();
                Check.Equal("Capture root is named Capture", "Capture", capture.Root.Name);
                Check.That("Capture records at least one Frame node", capture.Frames.Count >= 1);

                if (capture.Frames.Count >= 1)
                {
                    ProfilerMarkerNode frame = capture.Frames[0];
                    Check.Equal("Frame node is at depth 1", 1, frame.Depth);
                    Check.Equal("Frame node is named Frame", "Frame", frame.Name);

                    List<string> phaseNames = new List<string>();
                    for (int index = 0; index < frame.Children.Count; index++)
                    {
                        ProfilerMarkerNode phase = frame.Children[index];
                        phaseNames.Add(phase.Name);
                    }

                    Check.Equal(
                        "Frame records the seven Unity player loop phases in order",
                        string.Join(
                            ",",
                            InitializationPhase,
                            EarlyUpdatePhase,
                            FixedUpdatePhase,
                            PreUpdatePhase,
                            UpdatePhase,
                            PreLateUpdatePhase,
                            PostLateUpdatePhase),
                        string.Join(",", phaseNames));
                }

                host.Shutdown();
            }
        }

        private static void TestCustomPlayerLoopSystemSamplesNestUnderPhase()
        {
            using (BridgeHost host = new BridgeHost())
            {
                Unity.Profiling.ProfilerMarker innerMarker = new Unity.Profiling.ProfilerMarker("Custom.Inner");
                PlayerLoopSystem root = UnityEngine.LowLevel.PlayerLoop.GetCurrentPlayerLoop();
                PlayerLoopSystem updatePhase = FindPhase(root, UpdatePhase);
                PlayerLoopSystem custom = new PlayerLoopSystem
                {
                    type = CustomUpdateType,
                    subSystemList = Array.Empty<PlayerLoopSystem>(),
                    updateDelegate = () =>
                    {
                        Profiler.BeginSample("Custom.Outer");
                        using (innerMarker.Auto())
                        {
                        }

                        Profiler.EndSample();
                    },
                };
                updatePhase = InsertChild(updatePhase, IndexOfChild(updatePhase, ScriptRunBehaviourUpdate) + 1, custom);
                root = ReplaceChild(root, UpdatePhase, updatePhase);
                UnityEngine.LowLevel.PlayerLoop.SetPlayerLoop(root);

                host.Tick(0.02);
                Profiler.StartCapture();
                host.Tick(0.02);
                Profiler.StopCapture();

                ProfilerCapture capture = Profiler.GetCapture();
                ProfilerMarkerNode? updateNode = capture.Root.FindDescendant(UpdatePhase);
                Check.That("Update phase is present in the capture tree", updateNode != null);

                ProfilerMarkerNode? outerNode = updateNode?.FindChild("Custom.Outer");
                Check.That("Custom.Outer sample nests directly under Update phase", outerNode != null);
                if (outerNode != null)
                {
                    Check.Equal("Custom.Outer sample is at depth 3", 3, outerNode.Depth);
                    ProfilerMarkerNode? innerNode = outerNode.FindChild("Custom.Inner");
                    Check.That("Custom.Inner marker nests directly under Custom.Outer", innerNode != null);
                    if (innerNode != null)
                    {
                        Check.Equal("Custom.Inner marker is at depth 4", 4, innerNode.Depth);
                        Check.That(
                            "Outer sample duration is at least Inner marker duration",
                            outerNode.DurationNanoseconds >= innerNode.DurationNanoseconds);
                    }
                }

                host.Shutdown();
            }
        }

        private static void TestMonoBehaviourSamplesNestInCapture()
        {
            string scenePath = ResolveFixture(SceneFixture);
            Check.That("The profiler scene fixture exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0UL, 1UL, () => new ProfilerProbeScript());

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);

                    Profiler.StartCapture();
                    host.Tick(0.02);
                    Profiler.StopCapture();

                    ProfilerCapture capture = Profiler.GetCapture();
                    ProfilerMarkerNode? fixedOuter = capture.Root.FindDescendant("Script.FixedOuter");
                    ProfilerMarkerNode? updateOuter = capture.Root.FindDescendant("Script.UpdateOuter");
                    Check.That("Script.FixedOuter sample is recorded during tick", fixedOuter != null);
                    Check.That("Script.UpdateOuter sample is recorded during tick", updateOuter != null);

                    if (fixedOuter != null)
                    {
                        ProfilerMarkerNode? fixedInner = fixedOuter.FindChild("Script.FixedInner");
                        Check.That("Script.FixedInner nests under Script.FixedOuter", fixedInner != null);
                    }

                    if (updateOuter != null)
                    {
                        ProfilerMarkerNode? updateInner = updateOuter.FindChild("Script.UpdateInner");
                        Check.That("Script.UpdateInner nests under Script.UpdateOuter", updateInner != null);
                    }

                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }
        }

        private static PlayerLoopSystem FindPhase(PlayerLoopSystem root, string phaseType)
        {
            foreach (PlayerLoopSystem child in root.subSystemList ?? Array.Empty<PlayerLoopSystem>())
            {
                if (child.type == phaseType)
                {
                    return child;
                }
            }

            return default;
        }

        private static PlayerLoopSystem ReplaceChild(PlayerLoopSystem parent, string childType, PlayerLoopSystem replacement)
        {
            List<PlayerLoopSystem> children = new List<PlayerLoopSystem>(parent.subSystemList ?? Array.Empty<PlayerLoopSystem>());
            for (int index = 0; index < children.Count; index++)
            {
                if (children[index].type == childType)
                {
                    children[index] = replacement;
                    break;
                }
            }

            parent.subSystemList = children.ToArray();
            return parent;
        }

        private static PlayerLoopSystem InsertChild(PlayerLoopSystem parent, int index, PlayerLoopSystem child)
        {
            List<PlayerLoopSystem> children = new List<PlayerLoopSystem>(parent.subSystemList ?? Array.Empty<PlayerLoopSystem>());
            children.Insert(index < 0 ? 0 : index, child);
            parent.subSystemList = children.ToArray();
            return parent;
        }

        private static int IndexOfChild(PlayerLoopSystem parent, string childType)
        {
            PlayerLoopSystem[] children = parent.subSystemList ?? Array.Empty<PlayerLoopSystem>();
            for (int index = 0; index < children.Length; index++)
            {
                if (children[index].type == childType)
                {
                    return index;
                }
            }

            return -1;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
