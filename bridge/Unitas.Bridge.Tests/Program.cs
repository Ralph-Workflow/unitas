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
            TestPhysicsComponentDefaults();
            TestEndToEndRustLifecycleOrder();
            TestEndToEndRustCollisionOrder();
            TestEndToEndRustTriggerOrder();
            TestSceneReloadDropsPreviousSceneScripts();
            TestLayerMaskConversions();
            TestEndToEndRustPhysicsQuery();
            TimeDiagnosticsTests.Run();
            QuerySurfaceTests.Run();
            IntegrationTests.Run();
            CoroutineTests.Run();
            LifetimeTests.Run();
            CloneTests.Run();
            ActivationTests.Run();
            ExceptionContainmentTests.Run();
            InputTests.Run();
            SceneManagementTests.Run();
            SceneManagementIntegrationTests.Run();

            if (Check.FailureCount > 0)
            {
                Console.WriteLine("Total failures: " + Check.FailureCount);
                return 1;
            }

            Console.WriteLine((object)"All checks passed.");
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

        private static void TestPhysicsComponentDefaults()
        {
            GameObject go = new GameObject("PhysicsDefaults");

            Rigidbody body = go.AddComponent<Rigidbody>();
            Check.Equal("Rigidbody default mass", 1f, body.mass);
            Check.Equal("Rigidbody default useGravity", true, body.useGravity);
            Check.Equal("Rigidbody default velocity x", 0f, body.velocity.x);
            Check.Equal("Rigidbody default velocity y", 0f, body.velocity.y);
            Check.Equal("Rigidbody default velocity z", 0f, body.velocity.z);
            body.mass = 2.5f;
            body.useGravity = false;
            Check.Equal("Rigidbody mass round trip", 2.5f, body.mass);
            Check.Equal("Rigidbody useGravity round trip", false, body.useGravity);

            BoxCollider box = go.AddComponent<BoxCollider>();
            Check.That("BoxCollider is a Collider", box is Collider);
            Check.Equal("BoxCollider default size x", 1f, box.size.x);
            Check.Equal("BoxCollider default size y", 1f, box.size.y);
            Check.Equal("BoxCollider default size z", 1f, box.size.z);
            Check.Equal("BoxCollider default center x", 0f, box.center.x);
            Check.Equal("BoxCollider default center y", 0f, box.center.y);
            Check.Equal("BoxCollider default center z", 0f, box.center.z);
            Check.Equal("BoxCollider default isTrigger", false, box.isTrigger);
            box.isTrigger = true;
            Check.Equal("BoxCollider isTrigger round trip", true, box.isTrigger);
            box.size = new Vector3(2f, 3f, 4f);
            Check.Equal("BoxCollider size round trip x", 2f, box.size.x);
            Check.Equal("BoxCollider size round trip z", 4f, box.size.z);

            SphereCollider sphere = go.AddComponent<SphereCollider>();
            Check.That("SphereCollider is a Collider", sphere is Collider);
            Check.Equal("SphereCollider default radius", 0.5f, sphere.radius);
            Check.Equal("SphereCollider default center x", 0f, sphere.center.x);
            Check.Equal("SphereCollider default center y", 0f, sphere.center.y);
            Check.Equal("SphereCollider default center z", 0f, sphere.center.z);
            Check.Equal("SphereCollider default isTrigger", false, sphere.isTrigger);
            sphere.isTrigger = true;
            Check.Equal("SphereCollider isTrigger round trip", true, sphere.isTrigger);
            sphere.radius = 1.25f;
            Check.Equal("SphereCollider radius round trip", 1.25f, sphere.radius);

            Collision empty = new Collision(null);
            Check.That("Collision without other party has null gameObject", empty.gameObject == null);
            Collision filled = new Collision(go);
            Check.That("Collision exposes the other party gameObject", ReferenceEquals(filled.gameObject, go));
        }

        private static void TestEndToEndRustLifecycleOrder()
        {
            string? envScene = Environment.GetEnvironmentVariable("UNITAS_TEST_SCENE");
            string scenePath = !string.IsNullOrEmpty(envScene) && File.Exists(envScene)
                ? Path.GetFullPath(envScene)
                : ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");

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

        private static void TestEndToEndRustCollisionOrder()
        {
            string? envScene = Environment.GetEnvironmentVariable("UNITAS_TEST_SCENE");
            string scenePath = !string.IsNullOrEmpty(envScene) && File.Exists(envScene)
                ? Path.GetFullPath(envScene)
                : ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");

            Check.That("Collision scene file exists", File.Exists(scenePath));

            ScriptRegistry.Clear();
            CollisionRecorderScript recordedScript = new CollisionRecorderScript();
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            string observed = recordedScript.ObservedOrder;
            Console.WriteLine("Observed collision order: " + observed);

            string expected = "Enter";
            for (int stay = 0; stay < 23; stay++)
            {
                expected += ", Stay";
            }

            expected += ", Exit";
            Check.Equal("End-to-end collision order", expected, observed);

            string[] events = observed.Split(", ");
            Check.That("Collision order starts with Enter", events[0] == "Enter");
            Check.That("Collision order ends with Exit", events[events.Length - 1] == "Exit");

            bool sawStay = false;
            bool onlyKnownEvents = true;
            foreach (string name in events)
            {
                if (name == "Stay")
                {
                    sawStay = true;
                }
                else if (name != "Enter" && name != "Exit")
                {
                    onlyKnownEvents = false;
                }
            }

            Check.That("Collision order contains a Stay", sawStay);
            Check.That("Collision order holds only Enter, Stay and Exit", onlyKnownEvents);
            Check.That("Collision gameObject is null when the other party is unregistered", recordedScript.OtherGameObjectAlwaysNull);
        }

        private static void TestEndToEndRustTriggerOrder()
        {
            string? envScene = Environment.GetEnvironmentVariable("UNITAS_TEST_SCENE");
            string scenePath = !string.IsNullOrEmpty(envScene) && File.Exists(envScene)
                ? Path.GetFullPath(envScene)
                : ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_trigger.unity");

            Check.That("Trigger scene file exists", File.Exists(scenePath));

            ScriptRegistry.Clear();
            TriggerRecorderScript recordedScript = new TriggerRecorderScript();
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
            }

            string observed = recordedScript.ObservedOrder;
            Console.WriteLine("Observed trigger order: " + observed);

            string expected = "Enter";
            for (int stay = 0; stay < 23; stay++)
            {
                expected += ", Stay";
            }

            expected += ", Exit";
            Check.Equal("End-to-end trigger order", expected, observed);

            string[] events = observed.Split(", ");
            Check.That("Trigger order starts with Enter", events[0] == "Enter");
            Check.That("Trigger order ends with Exit", events[events.Length - 1] == "Exit");

            bool sawStay = false;
            bool onlyKnownEvents = true;
            foreach (string name in events)
            {
                if (name == "Stay")
                {
                    sawStay = true;
                }
                else if (name != "Enter" && name != "Exit")
                {
                    onlyKnownEvents = false;
                }
            }

            Check.That("Trigger order contains a Stay", sawStay);
            Check.That("Trigger order holds only Enter, Stay and Exit", onlyKnownEvents);
            Check.That("Trigger other collider is null when the other party is unregistered", recordedScript.OtherColliderAlwaysNull);
        }

        private static void TestSceneReloadDropsPreviousSceneScripts()
        {
            string scriptedScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            string plainScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision_no_script.unity");

            Check.That("Scripted collision scene file exists", File.Exists(scriptedScene));
            Check.That("Plain collision scene file exists", File.Exists(plainScene));

            ScriptRegistry.Clear();
            CollisionRecorderScript recordedScript = new CollisionRecorderScript();
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scriptedScene);
                host.Tick(0.02);
                string afterFirstScene = recordedScript.ObservedOrder;
                host.LoadScene(plainScene);
                for (int tick = 0; tick < 80; tick++)
                {
                    host.Tick(0.02);
                }
                host.Shutdown();
                Check.Equal(
                    "Reloaded scene does not dispatch to the previous scene script",
                    afterFirstScene,
                    recordedScript.ObservedOrder);
            }
        }

        private static void TestLayerMaskConversions()
        {
            LayerMask mask = 32;
            int raw = mask;
            Check.Equal("LayerMask converts to int", 32, raw);

            LayerMask fromInt = 5;
            Check.Equal("LayerMask converts from int", 5, fromInt.value);

            LayerMask mutable = 0;
            mutable.value = 32;
            Check.Equal("LayerMask value round trip", 32, mutable.value);
            Check.Equal("LayerMask implicit conversion to int", 32, (int)mutable);
        }

        private static void TestEndToEndRustPhysicsQuery()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");

            Check.That("Physics query scene file exists", File.Exists(scenePath));

            ScriptRegistry.Clear();
            ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new PhysicsQueryScript());
            ScriptRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new PhysicsQueryScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                for (int tick = 0; tick < 10; tick++)
                {
                    host.Tick(0.02);
                }

                bool found = Physics.Raycast(new Vector3(0f, -5f, 0f), new Vector3(0f, 1f, 0f), out RaycastHit hit, 100f, 1 << 5);
                Check.That("Raycast hits the layer 5 collider", found);
                Check.That("Raycast hit collider is not null", hit.collider != null);
                Check.Equal("Raycast hit distance", 4.5f, hit.distance);
                Check.Equal("Raycast hit point x", 0f, hit.point.x);
                Check.Equal("Raycast hit point y", -0.5f, hit.point.y);
                Check.Equal("Raycast hit point z", 0f, hit.point.z);

                bool excluded = Physics.Raycast(new Vector3(0f, -5f, 0f), new Vector3(0f, 1f, 0f), out RaycastHit excludedHit, 100f, ~(1 << 5));
                Check.That("Raycast misses a collider on an excluded layer", !excluded);
                Check.That("Raycast miss leaves the hit empty", excludedHit.collider == null);

                Collider[] all = Physics.OverlapSphere(new Vector3(0f, 0f, 0f), 2.5f, ~0);
                Check.Equal("OverlapSphere returns every overlapping collider", 2, all.Length);
                Check.That(
                    "OverlapSphere colliders are not null",
                    all.Length == 2 && all[0] != null && all[1] != null);

                Collider[] layered = Physics.OverlapSphere(new Vector3(0f, 0f, 0f), 2.5f, 1 << 5);
                Check.Equal("OverlapSphere returns only the layer 5 collider", 1, layered.Length);
                Check.That("OverlapSphere layered collider is not null", layered.Length == 1 && layered[0] != null);

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
