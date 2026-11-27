using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.LowLevel;

namespace Unitas.Bridge.Tests
{
    /// <summary>Records the script phases the player loop dispatches.</summary>
    public class PlayerLoopProbeScript : MonoBehaviour
    {
        private readonly List<string> _trace;

        public PlayerLoopProbeScript(List<string> trace)
        {
            _trace = trace;
        }

        public override void FixedUpdate()
        {
            _trace.Add("F:script");
        }

        public override void Update()
        {
            _trace.Add("U:script");
        }

        public override void LateUpdate()
        {
            _trace.Add("L:script");
        }
    }

    public static class PlayerLoopTests
    {
        private const string SceneFixture = "crates/unitas-runtime/tests/fixtures/scene_with_script.unity";

        private const string RootType = "UnityEngine.LowLevel.PlayerLoop";
        private const string InitializationPhase = "UnityEngine.PlayerLoop.Initialization";
        private const string EarlyUpdatePhase = "UnityEngine.PlayerLoop.EarlyUpdate";
        private const string FixedUpdatePhase = "UnityEngine.PlayerLoop.FixedUpdate";
        private const string PreUpdatePhase = "UnityEngine.PlayerLoop.PreUpdate";
        private const string UpdatePhase = "UnityEngine.PlayerLoop.Update";
        private const string PreLateUpdatePhase = "UnityEngine.PlayerLoop.PreLateUpdate";
        private const string PostLateUpdatePhase = "UnityEngine.PlayerLoop.PostLateUpdate";

        private const string ScriptRunBehaviourFixedUpdate = "UnityEngine.PlayerLoop.FixedUpdate.ScriptRunBehaviourFixedUpdate";
        private const string ScriptRunBehaviourUpdate = "UnityEngine.PlayerLoop.Update.ScriptRunBehaviourUpdate";

        private const string CustomFixedType = "MyGame.PlayerLoop.BeforeFixed";
        private const string CustomUpdateType = "MyGame.PlayerLoop.AfterUpdate";

        public static void Run()
        {
            TestDefaultPlayerLoopMatchesTheUnityPhaseTree();
            TestSetPlayerLoopRoundTripsTheDefaultTree();
            TestCustomSystemRunsAtItsPositionBetweenBuiltIns();
            TestRemovingABuiltInSystemDisablesItsWork();
        }

        private static void TestDefaultPlayerLoopMatchesTheUnityPhaseTree()
        {
            using (BridgeHost host = new BridgeHost())
            {
                PlayerLoopSystem root = UnityEngine.LowLevel.PlayerLoop.GetDefaultPlayerLoop();
                Check.Equal("The default player loop root is the Unity player loop type", RootType, root.type ?? string.Empty);
                Check.Equal(
                    "The default player loop holds the seven Unity phases in order",
                    string.Join(",", InitializationPhase, EarlyUpdatePhase, FixedUpdatePhase, PreUpdatePhase, UpdatePhase, PreLateUpdatePhase, PostLateUpdatePhase),
                    string.Join(",", ChildTypes(root)));
                Check.Equal(
                    "The Update phase holds the Unity Update subsystems in order",
                    string.Join(",", ScriptRunBehaviourUpdate, "UnityEngine.PlayerLoop.Update.ScriptRunDelayedTasks", "UnityEngine.PlayerLoop.Update.DirectorUpdate", "UnityEngine.PlayerLoop.Update.AudioUpdate"),
                    string.Join(",", ChildTypes(FindPhase(root, UpdatePhase))));
                Check.Equal(
                    "The FixedUpdate phase holds the Unity fixed subsystems in order",
                    string.Join(
                        ",",
                        "UnityEngine.PlayerLoop.FixedUpdate.PhysicsFixedUpdate",
                        "UnityEngine.PlayerLoop.FixedUpdate.Physics2DFixedUpdate",
                        ScriptRunBehaviourFixedUpdate),
                    string.Join(",", ChildTypes(FindPhase(root, FixedUpdatePhase)).Take(3)));
                host.Shutdown();
            }
        }

        private static void TestSetPlayerLoopRoundTripsTheDefaultTree()
        {
            using (BridgeHost host = new BridgeHost())
            {
                PlayerLoopSystem root = UnityEngine.LowLevel.PlayerLoop.GetDefaultPlayerLoop();
                UnityEngine.LowLevel.PlayerLoop.SetPlayerLoop(root);

                PlayerLoopSystem current = UnityEngine.LowLevel.PlayerLoop.GetCurrentPlayerLoop();
                Check.Equal("The round-tripped player loop keeps the root type", RootType, current.type ?? string.Empty);
                Check.Equal(
                    "The round-tripped player loop keeps the Unity phases",
                    string.Join(",", ChildTypes(root)),
                    string.Join(",", ChildTypes(current)));
                Check.Equal(
                    "The round-tripped player loop keeps the Update subsystems",
                    string.Join(",", ChildTypes(FindPhase(root, UpdatePhase))),
                    string.Join(",", ChildTypes(FindPhase(current, UpdatePhase))));
                host.Shutdown();
            }
        }

        private static void TestCustomSystemRunsAtItsPositionBetweenBuiltIns()
        {
            string scenePath = ResolveFixture(SceneFixture);
            Check.That("The player loop scene fixture exists", File.Exists(scenePath));

            List<string> trace = new List<string>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0UL, 1UL, () => new PlayerLoopProbeScript(trace));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    PlayerLoopSystem root = UnityEngine.LowLevel.PlayerLoop.GetCurrentPlayerLoop();

                    PlayerLoopSystem fixedPhase = FindPhase(root, FixedUpdatePhase);
                    fixedPhase = InsertChild(fixedPhase, IndexOfChild(fixedPhase, ScriptRunBehaviourFixedUpdate), CustomSystem(CustomFixedType, trace, "C:beforeFixed"));

                    PlayerLoopSystem updatePhase = FindPhase(root, UpdatePhase);
                    updatePhase = InsertChild(updatePhase, IndexOfChild(updatePhase, ScriptRunBehaviourUpdate) + 1, CustomSystem(CustomUpdateType, trace, "C:afterUpdate"));

                    root = ReplaceChild(root, FixedUpdatePhase, fixedPhase);
                    root = ReplaceChild(root, UpdatePhase, updatePhase);
                    UnityEngine.LowLevel.PlayerLoop.SetPlayerLoop(root);

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
                "A custom system runs at the position the managed tree gives it",
                string.Join(",", "C:beforeFixed", "F:script", "U:script", "C:afterUpdate", "L:script"),
                string.Join(",", trace));
        }

        private static void TestRemovingABuiltInSystemDisablesItsWork()
        {
            string scenePath = ResolveFixture(SceneFixture);
            Check.That("The player loop scene fixture exists", File.Exists(scenePath));

            List<string> trace = new List<string>();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0UL, 1UL, () => new PlayerLoopProbeScript(trace));

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    PlayerLoopSystem root = UnityEngine.LowLevel.PlayerLoop.GetCurrentPlayerLoop();
                    PlayerLoopSystem updatePhase = FindPhase(root, UpdatePhase);
                    updatePhase = RemoveChild(updatePhase, ScriptRunBehaviourUpdate);
                    root = ReplaceChild(root, UpdatePhase, updatePhase);
                    UnityEngine.LowLevel.PlayerLoop.SetPlayerLoop(root);

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
                "Removing the script update system leaves the other script phases running",
                string.Join(",", "F:script", "L:script"),
                string.Join(",", trace));
        }

        private static PlayerLoopSystem CustomSystem(string type, List<string> trace, string entry)
        {
            return new PlayerLoopSystem
            {
                type = type,
                subSystemList = Array.Empty<PlayerLoopSystem>(),
                updateDelegate = () => trace.Add(entry),
            };
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

        private static PlayerLoopSystem RemoveChild(PlayerLoopSystem parent, string childType)
        {
            List<PlayerLoopSystem> children = new List<PlayerLoopSystem>(parent.subSystemList ?? Array.Empty<PlayerLoopSystem>());
            children.RemoveAll(child => child.type == childType);
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

        private static string[] ChildTypes(PlayerLoopSystem node)
        {
            PlayerLoopSystem[] children = node.subSystemList ?? Array.Empty<PlayerLoopSystem>();
            string[] types = new string[children.Length];
            for (int index = 0; index < children.Length; index++)
            {
                types[index] = children[index].type ?? string.Empty;
            }

            return types;
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
