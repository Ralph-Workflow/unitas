using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class CollisionCounterpartTests
    {
        public static void Run()
        {
            TestCollisionCounterpartResolvesScriptlessGameObject();
            TestTriggerCounterpartResolvesScriptlessCollider();
        }

        private static void TestCollisionCounterpartResolvesScriptlessGameObject()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");

            Check.That("Collision counterpart scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            CollisionRecorderScript recordedScript = new CollisionRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            Check.That(
                "Scriptless collision counterpart delivers a gameObject to the script",
                !recordedScript.OtherGameObjectAlwaysNull);
            Check.Equal(
                "Scriptless collision counterpart gameObject is the scene crate",
                "Crate",
                recordedScript.ObservedOtherNames);
        }

        private static void TestTriggerCounterpartResolvesScriptlessCollider()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_trigger.unity");

            Check.That("Trigger counterpart scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            TriggerRecorderScript recordedScript = new TriggerRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            Check.That(
                "Scriptless trigger counterpart delivers a collider to the script",
                !recordedScript.OtherColliderAlwaysNull);
            Check.Equal(
                "Scriptless trigger counterpart collider belongs to the scene crate",
                "Crate",
                recordedScript.ObservedOtherNames);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
