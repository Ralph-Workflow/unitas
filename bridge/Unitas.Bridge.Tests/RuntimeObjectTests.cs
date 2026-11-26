using System;
using System.Collections.Generic;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class RuntimeObjectProbeScript : MonoBehaviour
    {
        private readonly List<string> _events = new List<string>();
        private readonly List<Collider?> _triggerColliders = new List<Collider?>();

        public IReadOnlyList<string> Events => _events;

        public string ObservedOrder => string.Join(", ", _events);

        public IReadOnlyList<Collider?> TriggerColliders => _triggerColliders;

        public override void Awake()
        {
            _events.Add("Awake");
        }

        public override void OnEnable()
        {
            _events.Add("OnEnable");
        }

        public override void Start()
        {
            _events.Add("Start");
        }

        public override void Update()
        {
            if (!_events.Contains("Update"))
            {
                _events.Add("Update");
            }
        }

        public override void OnTriggerEnter(Collider? other)
        {
            _events.Add("OnTriggerEnter");
            _triggerColliders.Add(other);
        }
    }

    public static class RuntimeObjectTests
    {
        public static void Run()
        {
            TestRuntimeGameObjectAndScriptLifecycle();
            TestRuntimeTriggerColliderReceivesOnTriggerEnter();
            TestRuntimeObjectDestroyFakeNull();
        }

        private static void TestRuntimeGameObjectAndScriptLifecycle()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("RuntimeObjectTests scene_with_script exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            LifecycleScript sceneScript = new LifecycleScript();
            ScriptGlueRegistry.Register(0UL, 1UL, () => sceneScript);

            using (BridgeHost host = new BridgeHost())
            {
                Check.That("BridgeHost installs RuntimeObjectProvider.Current", ReferenceEquals(RuntimeObjectProvider.Current, host));
                host.LoadScene(scenePath);
                host.Tick(0.02);

                GameObject runtimeObject = new GameObject("RuntimeProbe");
                Check.That("Runtime GameObject receives non-zero native entity id", runtimeObject.NativeEntityId != 0UL);

                RuntimeObjectProbeScript probe = runtimeObject.AddComponent<RuntimeObjectProbeScript>();
                Check.That("AddComponent returns non-null probe script", probe != null);
                Check.That("Probe script is attached to the runtime GameObject", ReferenceEquals(probe!.gameObject, runtimeObject));
                Check.Equal("Runtime GameObject holds one probe script component", 1, runtimeObject.GetComponents<RuntimeObjectProbeScript>().Length);

                host.Tick(0.02);
                host.Tick(0.02);

                Check.Equal("Runtime probe receives Awake, OnEnable, Start, Update in order", "Awake, OnEnable, Start, Update", probe.ObservedOrder);
                host.Shutdown();
            }

            Check.That("BridgeHost shutdown clears RuntimeObjectProvider.Current", RuntimeObjectProvider.Current == null);
        }

        private static void TestRuntimeTriggerColliderReceivesOnTriggerEnter()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_trigger.unity");
            Check.That("RuntimeObjectTests scene_physics_trigger exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            TriggerRecorderScript sceneScript = new TriggerRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => sceneScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                GameObject runtimeTrigger = new GameObject("RuntimeTrigger");
                Check.That("Runtime trigger object has non-zero native entity id", runtimeTrigger.NativeEntityId != 0UL);
                runtimeTrigger.transform.position = new Vector3(0f, 0f, 0f);

                SphereCollider collider = runtimeTrigger.AddComponent<SphereCollider>();
                collider.isTrigger = true;
                collider.radius = 0.5f;

                RuntimeObjectProbeScript probe = runtimeTrigger.AddComponent<RuntimeObjectProbeScript>();
                Check.That("Runtime trigger probe attached", probe != null);

                for (int tick = 0; tick < 20; tick++)
                {
                    host.Tick(0.02);
                }

                Check.That("Runtime trigger probe received OnTriggerEnter", probe!.TriggerColliders.Count > 0);
                Collider? other = probe.TriggerColliders.Count > 0 ? probe.TriggerColliders[0] : null;
                Check.That("Runtime trigger OnTriggerEnter resolved scene collider", other != null);
                if (other != null)
                {
                    Check.That(
                        "Runtime trigger OnTriggerEnter resolved a scene object collider",
                        other.gameObject.name == "Ball" || other.gameObject.name == "Crate");
                }

                host.Shutdown();
            }
        }

        private static void TestRuntimeObjectDestroyFakeNull()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("RuntimeObjectTests destroy scene exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            LifecycleScript sceneScript = new LifecycleScript();
            ScriptGlueRegistry.Register(0UL, 1UL, () => sceneScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                GameObject runtimeObject = new GameObject("DoomedRuntimeObject");
                ulong entity = runtimeObject.NativeEntityId;
                Check.That("Doomed runtime object has non-zero native entity id", entity != 0UL);

                RuntimeObjectProbeScript probe = runtimeObject.AddComponent<RuntimeObjectProbeScript>();
                host.Tick(0.02);

                UnityEngine.Object.Destroy(runtimeObject);
                host.Tick(0.02);

                Check.That("Destroyed runtime GameObject compares equal to null", runtimeObject == null);
                Check.That("Destroyed runtime MonoBehaviour compares equal to null", probe == null);
                host.Shutdown();
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
