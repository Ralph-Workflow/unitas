using System;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class Program
    {
        public static int Main()
        {
            TestVector3();
            TestGameObjectAndTransform();
            TestMonoBehaviourLifecycleOrder();
            TestEndToEndRustLifecycleOrder();

            if (Check.FailureCount > 0)
            {
                Console.WriteLine("Total failures: " + Check.FailureCount);
                return 1;
            }

            Console.WriteLine("All checks passed.");
            return 0;
        }

        private static void TestVector3()
        {
            Vector3 a = new Vector3(1f, 2f, 3f);
            Vector3 b = new Vector3(4f, 5f, 6f);

            Vector3 sum = a + b;
            Check.Equal("Vector3 add x", 5f, sum.x);
            Check.Equal("Vector3 add y", 7f, sum.y);
            Check.Equal("Vector3 add z", 9f, sum.z);

            Vector3 diff = b - a;
            Check.Equal("Vector3 sub x", 3f, diff.x);
            Check.Equal("Vector3 sub y", 3f, diff.y);
            Check.Equal("Vector3 sub z", 3f, diff.z);

            Vector3 scaled = a * 2f;
            Check.Equal("Vector3 mul x", 2f, scaled.x);
            Check.Equal("Vector3 mul y", 4f, scaled.y);
            Check.Equal("Vector3 mul z", 6f, scaled.z);

            Vector3 cross = Vector3.Cross(Vector3.right, Vector3.up);
            Check.Equal("Vector3 cross x", Vector3.forward.x, cross.x);
            Check.Equal("Vector3 cross y", Vector3.forward.y, cross.y);
            Check.Equal("Vector3 cross z", Vector3.forward.z, cross.z);

            float dist = Vector3.Distance(new Vector3(1f, 2f, 3f), new Vector3(4f, 6f, 3f));
            Check.Equal("Vector3 distance", 5f, dist);

            float dot = Vector3.Dot(a, b);
            Check.Equal("Vector3 dot", 32f, dot);

            Check.Equal("Vector3 zero magnitude", 0f, Vector3.zero.magnitude);
            Check.Equal("Vector3 up normalized y", 1f, Vector3.up.normalized.y);
        }

        private static void TestGameObjectAndTransform()
        {
            GameObject parent = new GameObject("Parent");
            GameObject child = new GameObject("Child");

            Check.Equal("Parent name", "Parent", parent.name);
            Check.Equal("Child name", "Child", child.name);

            child.transform.parent = parent.transform;
            Check.That("Child parent set", child.transform.parent == parent.transform);
            Check.Equal("Parent childCount", 1, parent.transform.childCount);
            Check.That("Parent GetChild(0)", parent.transform.GetChild(0) == child.transform);

            child.transform.localPosition = new Vector3(1f, 2f, 3f);
            Check.Equal("Child localPosition x", 1f, child.transform.localPosition.x);
            Check.Equal("Child position x", 1f, child.transform.position.x);

            child.transform.Translate(new Vector3(1f, 1f, 1f));
            Check.Equal("Child translated position x", 2f, child.transform.position.x);
            Check.Equal("Child translated position y", 3f, child.transform.position.y);
            Check.Equal("Child translated position z", 4f, child.transform.position.z);

            TestComponent comp = child.AddComponent<TestComponent>();
            Check.That("Component added", comp != null);
            if (comp != null)
            {
                Check.That("GetComponent finds component", child.GetComponent<TestComponent>() == comp);
                Check.That("Component gameObject matches", comp.gameObject == child);
                Check.That("Component transform matches", comp.transform == child.transform);
                Check.Equal("Component name matches gameObject", "Child", comp.name);
            }
        }

        private static void TestMonoBehaviourLifecycleOrder()
        {
            LifecycleScript script = new LifecycleScript();
            script.Awake();
            script.OnEnable();
            script.Start();
            script.FixedUpdate();
            script.Update();
            script.LateUpdate();
            script.OnDisable();
            script.OnDestroy();

            string expected = "Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate, OnDisable, OnDestroy";
            Check.Equal("MonoBehaviour lifecycle order", expected, script.ObservedOrder);
        }

        private static void TestEndToEndRustLifecycleOrder()
        {
            string? envScene = Environment.GetEnvironmentVariable("UNITAS_TEST_SCENE");
            string scenePath = !string.IsNullOrEmpty(envScene) && File.Exists(envScene)
                ? Path.GetFullPath(envScene)
                : (File.Exists("crates/unitas-runtime/tests/fixtures/scene_with_script.unity")
                    ? Path.GetFullPath("crates/unitas-runtime/tests/fixtures/scene_with_script.unity")
                    : Path.GetFullPath("../../../../../crates/unitas-runtime/tests/fixtures/scene_with_script.unity"));

            Check.That("Scene file exists", File.Exists(scenePath));

            ScriptRegistry.Clear();
            LifecycleScript recordedScript = new LifecycleScript();
            ScriptRegistry.Register(0UL, 1UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);
                host.Tick(0.02);
                host.Shutdown();
            }

            string expected = "Awake, OnEnable, Start, FixedUpdate, Update, LateUpdate, OnDisable, OnDestroy";
            Check.Equal("End-to-end lifecycle order", expected, recordedScript.ObservedOrder);
            Console.WriteLine("Observed lifecycle order: " + recordedScript.ObservedOrder);
        }
    }
}
