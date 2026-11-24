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
            SceneManagementPersistenceTests.Run();
            ApplicationTests.Run();
            PlayerPrefsTests.Run();
            ObjectDiscoveryTests.Run();
            GameObjectDiscoveryTests.Run();
            LoggingTests.Run();
            TestCharacterControllerDefaultsAndNullProvider();
            TestEndToEndCharacterControllerMoveAndSceneLoad();

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
            Check.Equal("Rigidbody default isKinematic", false, body.isKinematic);
            Check.Equal("Rigidbody default velocity x", 0f, body.velocity.x);
            Check.Equal("Rigidbody default velocity y", 0f, body.velocity.y);
            Check.Equal("Rigidbody default velocity z", 0f, body.velocity.z);
            body.mass = 2.5f;
            body.useGravity = false;
            body.isKinematic = true;
            Check.Equal("Rigidbody mass round trip", 2.5f, body.mass);
            Check.Equal("Rigidbody useGravity round trip", false, body.useGravity);
            Check.Equal("Rigidbody isKinematic round trip", true, body.isKinematic);
            body.isKinematic = false;
            Check.Equal("Rigidbody isKinematic reset round trip", false, body.isKinematic);

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

            CapsuleCollider capsule = go.AddComponent<CapsuleCollider>();
            Check.That("CapsuleCollider is a Collider", capsule is Collider);
            Check.That("GetComponent finds CapsuleCollider", ReferenceEquals(go.GetComponent<CapsuleCollider>(), capsule));
            Check.That("CapsuleCollider attachedRigidbody resolves owner Rigidbody", ReferenceEquals(capsule.attachedRigidbody, body));
            Check.Equal("CapsuleCollider default radius", 0.5f, capsule.radius);
            Check.Equal("CapsuleCollider default height", 2f, capsule.height);
            Check.Equal("CapsuleCollider default direction", 1, capsule.direction);
            Check.Equal("CapsuleCollider default center x", 0f, capsule.center.x);
            Check.Equal("CapsuleCollider default center y", 0f, capsule.center.y);
            Check.Equal("CapsuleCollider default center z", 0f, capsule.center.z);
            Check.Equal("CapsuleCollider default isTrigger", false, capsule.isTrigger);
            capsule.isTrigger = true;
            capsule.radius = 0.75f;
            capsule.height = 3.5f;
            capsule.direction = 2;
            capsule.center = new Vector3(0.1f, 0.2f, -0.3f);
            Check.Equal("CapsuleCollider isTrigger round trip", true, capsule.isTrigger);
            Check.Equal("CapsuleCollider radius round trip", 0.75f, capsule.radius);
            Check.Equal("CapsuleCollider height round trip", 3.5f, capsule.height);
            Check.Equal("CapsuleCollider direction round trip", 2, capsule.direction);
            Check.Equal("CapsuleCollider center round trip x", 0.1f, capsule.center.x);
            Check.Equal("CapsuleCollider center round trip y", 0.2f, capsule.center.y);
            Check.Equal("CapsuleCollider center round trip z", -0.3f, capsule.center.z);

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

        private static void TestCharacterControllerDefaultsAndNullProvider()
        {
            ICharacterControllerProvider? savedProvider = CharacterController.Provider;
            CharacterController.Provider = null;
            try
            {
                Check.Equal("CollisionFlags.None value", 0, (int)CollisionFlags.None);
                Check.Equal("CollisionFlags.Sides value", 1, (int)CollisionFlags.Sides);
                Check.Equal("CollisionFlags.Above value", 2, (int)CollisionFlags.Above);
                Check.Equal("CollisionFlags.Below value", 4, (int)CollisionFlags.Below);
                Check.Equal("CollisionFlags.CollidedSides alias", (int)CollisionFlags.Sides, (int)CollisionFlags.CollidedSides);
                Check.Equal("CollisionFlags.CollidedAbove alias", (int)CollisionFlags.Above, (int)CollisionFlags.CollidedAbove);
                Check.Equal("CollisionFlags.CollidedBelow alias", (int)CollisionFlags.Below, (int)CollisionFlags.CollidedBelow);

                GameObject go = new GameObject("CharacterDefaults");
                CharacterController cc = go.AddComponent<CharacterController>();
                Check.That("CharacterController is a Collider", cc is Collider);
                Check.That("GetComponent<CharacterController> resolves controller", ReferenceEquals(go.GetComponent<CharacterController>(), cc));
                Check.That("GetComponent<Collider> resolves CharacterController", ReferenceEquals(go.GetComponent<Collider>(), cc));
                Check.Equal("CharacterController default enabled", true, cc.enabled);
                Check.Equal("CharacterController default radius", 0.5f, cc.radius);
                Check.Equal("CharacterController default height", 2.0f, cc.height);
                Check.Equal("CharacterController default center x", 0f, cc.center.x);
                Check.Equal("CharacterController default center y", 0f, cc.center.y);
                Check.Equal("CharacterController default center z", 0f, cc.center.z);
                Check.Equal("CharacterController default slopeLimit", 45.0f, cc.slopeLimit);
                Check.Equal("CharacterController default stepOffset", 0.3f, cc.stepOffset);
                Check.Equal("CharacterController default skinWidth", 0.08f, cc.skinWidth);
                Check.Equal("CharacterController default minMoveDistance", 0.001f, cc.minMoveDistance);
                Check.Equal("CharacterController default isGrounded", false, cc.isGrounded);
                Check.That("CharacterController default collisionFlags is None", cc.collisionFlags == CollisionFlags.None);
                Check.That("CharacterController default velocity is zero", cc.velocity == Vector3.zero);

                cc.radius = 0.4f;
                cc.height = 1.8f;
                cc.center = new Vector3(0.1f, 0.2f, -0.1f);
                cc.slopeLimit = 50f;
                cc.stepOffset = 0.25f;
                cc.skinWidth = 0.05f;
                cc.minMoveDistance = 0.002f;
                Check.Equal("CharacterController radius round trip", 0.4f, cc.radius);
                Check.Equal("CharacterController height round trip", 1.8f, cc.height);
                Check.Equal("CharacterController center y round trip", 0.2f, cc.center.y);
                Check.Equal("CharacterController slopeLimit round trip", 50f, cc.slopeLimit);
                Check.Equal("CharacterController stepOffset round trip", 0.25f, cc.stepOffset);
                Check.Equal("CharacterController skinWidth round trip", 0.05f, cc.skinWidth);
                Check.Equal("CharacterController minMoveDistance round trip", 0.002f, cc.minMoveDistance);

                go.transform.position = new Vector3(1f, 2f, 3f);
                CollisionFlags nullProviderFlags = cc.Move(new Vector3(5f, -1f, 2f));
                Check.That("Move with null provider returns CollisionFlags.None", nullProviderFlags == CollisionFlags.None);
                Check.Equal("Move with null provider keeps isGrounded false", false, cc.isGrounded);
                Check.That("Move with null provider keeps collisionFlags None", cc.collisionFlags == CollisionFlags.None);
                Check.That("Move with null provider leaves transform position unchanged", go.transform.position == new Vector3(1f, 2f, 3f));

                bool simpleMoved = cc.SimpleMove(new Vector3(3f, 0f, 0f));
                Check.Equal("SimpleMove with null provider returns false", false, simpleMoved);
            }
            finally
            {
                CharacterController.Provider = savedProvider;
            }
        }

        private static void TestEndToEndCharacterControllerMoveAndSceneLoad()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_cc_e2e_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "character_e2e.unity");
            File.WriteAllText(scenePath, CharacterControllerE2eSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptRegistry.Clear();
                CharacterProbeScript? playerScript = null;
                PlatformVelocityScript? platformScript = null;
                ScriptRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => playerScript = new CharacterProbeScript());
                ScriptRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => platformScript = new PlatformVelocityScript());

                using (BridgeHost host = new BridgeHost())
                {
                    Check.That("BridgeHost wires CharacterController.Provider", ReferenceEquals(CharacterController.Provider, host));
                    host.LoadScene(scenePath);
                    host.Tick(0.0);

                    Check.That("Player script created on scene load", playerScript != null);
                    Check.That("Platform script created on scene load", platformScript != null);

                    CharacterController? cc = playerScript!.GetComponent<CharacterController>();
                    Check.That("CharacterController attached from YAML scene load", cc != null);
                    Check.Equal("Scene-loaded CharacterController radius", 0.4f, cc!.radius);
                    Check.Equal("Scene-loaded CharacterController height", 1.8f, cc.height);
                    Check.Equal("Scene-loaded CharacterController center y", 0.1f, cc.center.y);
                    Check.Equal("Scene-loaded CharacterController slopeLimit", 45f, cc.slopeLimit);
                    Check.Equal("Scene-loaded CharacterController stepOffset", 0.3f, cc.stepOffset);
                    Check.Equal("Scene-loaded CharacterController skinWidth", 0.05f, cc.skinWidth);
                    Check.Equal("Scene-loaded CharacterController minMoveDistance", 0.002f, cc.minMoveDistance);
                    Check.Equal("Scene-loaded CharacterController enabled", true, cc.enabled);

                    cc.radius = 0.5f;
                    cc.height = 2.0f;
                    cc.center = Vector3.zero;
                    cc.slopeLimit = 45f;
                    cc.stepOffset = 0.3f;
                    cc.skinWidth = 0.08f;
                    cc.minMoveDistance = 0.001f;

                    playerScript.transform.position = new Vector3(0f, 1.2f, 0f);
                    CollisionFlags landFlags = cc.Move(new Vector3(0f, -0.5f, 0f));
                    Check.That("Landing on ground reports CollisionFlags.Below", (landFlags & CollisionFlags.Below) != 0);
                    Check.Equal("Landing on ground sets isGrounded true", true, cc.isGrounded);
                    Check.That("Landing on ground sets collisionFlags property", cc.collisionFlags == landFlags);
                    Check.That("Landing on ground stops above floor", Math.Abs(playerScript.transform.position.y - 1.08f) < 0.05f);
                    Check.That("Move updates velocity", cc.velocity.y < -1f);

                    Vector3 groundedStart = playerScript.transform.position;
                    CollisionFlags firstMulti = cc.Move(new Vector3(0.4f, -0.05f, 0f));
                    CollisionFlags secondMulti = cc.Move(new Vector3(0.6f, -0.05f, 0.25f));
                    Check.That("First move in same frame stays grounded", (firstMulti & CollisionFlags.Below) != 0 && cc.isGrounded);
                    Check.That("Second move in same frame composes position x", Math.Abs(playerScript.transform.position.x - (groundedStart.x + 1.0f)) < 0.02f);
                    Check.That("Second move in same frame composes position z", Math.Abs(playerScript.transform.position.z - 0.25f) < 0.02f);
                    Check.That("Second move in same frame stays grounded", (secondMulti & CollisionFlags.Below) != 0 && cc.isGrounded);

                    playerScript.transform.position = new Vector3(-8.5f, 1.08f, 0f);
                    CollisionFlags wallFlags = cc.Move(new Vector3(-2.0f, -0.05f, 1.25f));
                    Check.That("Hitting wall reports CollisionFlags.Sides", (wallFlags & CollisionFlags.Sides) != 0);
                    Check.That("Hitting wall while grounded also reports Below", (wallFlags & CollisionFlags.Below) != 0);
                    Check.That("Wall blocks x motion past boundary", playerScript.transform.position.x >= -9.08f - 0.02f);
                    Check.That("Wall slide preserves z motion", Math.Abs(playerScript.transform.position.z - 1.25f) < 0.02f);

                    playerScript.transform.position = new Vector3(0f, 1.2f, -5f);
                    CollisionFlags ceilFlags = cc.Move(new Vector3(0f, 1.0f, 0f));
                    Check.That("Hitting ceiling reports CollisionFlags.Above", (ceilFlags & CollisionFlags.Above) != 0);
                    Check.Equal("Hitting ceiling leaves isGrounded false", false, cc.isGrounded);
                    Check.That("Ceiling clamps upward y position", playerScript.transform.position.y <= 1.42f + 0.02f);

                    playerScript.transform.position = new Vector3(0f, 1.08f, 0f);
                    for (int step = 0; step < 10; step++)
                    {
                        cc.Move(new Vector3(0.25f, -0.1f, 0f));
                    }
                    Check.That("Character climbs step within stepOffset (x > 1.5)", playerScript.transform.position.x > 1.5f);
                    Check.That("Character climbs step within stepOffset (y >= 1.2)", playerScript.transform.position.y >= 1.2f);
                    Check.Equal("Character remains grounded on climbed step", true, cc.isGrounded);

                    playerScript.transform.position = new Vector3(0.4f, 1.08f, 4f);
                    CollisionFlags tallStepFlags = cc.Move(new Vector3(1.0f, -0.1f, 0f));
                    Check.That("Tall step (> stepOffset) reports CollisionFlags.Sides", (tallStepFlags & CollisionFlags.Sides) != 0);
                    Check.That("Tall step blocks forward x motion", playerScript.transform.position.x <= 0.58f + 0.02f);
                    Check.That("Tall step does not raise character y", playerScript.transform.position.y < 1.2f);

                    playerScript.transform.position = new Vector3(0f, 1.08f, 8f);
                    for (int step = 0; step < 15; step++)
                    {
                        cc.Move(new Vector3(0.25f, -0.1f, 0f));
                    }
                    Check.That("Character climbs walkable slope (x > 2.0)", playerScript.transform.position.x > 2.0f);
                    Check.That("Character climbs walkable slope (y > 1.3)", playerScript.transform.position.y > 1.3f);
                    Check.Equal("Character stays grounded on walkable slope", true, cc.isGrounded);

                    cc.stepOffset = 0.2f;
                    playerScript.transform.position = new Vector3(0f, 1.08f, 12f);
                    for (int step = 0; step < 15; step++)
                    {
                        cc.Move(new Vector3(0.25f, -0.1f, 0f));
                    }
                    Check.That("Excessive slope (> slopeLimit) blocks climb (y < 1.35)", playerScript.transform.position.y < 1.35f);
                    Check.That("Excessive slope reports CollisionFlags.Sides", (cc.collisionFlags & CollisionFlags.Sides) != 0);
                    cc.stepOffset = 0.3f;

                    playerScript.transform.position = new Vector3(0.3f, 1.08f, -8f);
                    CollisionFlags standLowFlags = cc.Move(new Vector3(1.5f, -0.05f, 0f));
                    Check.That("Standing character blocked by low obstacle reports Sides", (standLowFlags & CollisionFlags.Sides) != 0);
                    Check.That("Standing character stopped before low obstacle", playerScript.transform.position.x <= 0.58f + 0.02f);

                    playerScript.transform.position = new Vector3(0.3f, 1.08f, -8f);
                    cc.height = 1.0f;
                    cc.center = new Vector3(0f, -0.5f, 0f);
                    CollisionFlags crouchFlags = cc.Move(new Vector3(1.5f, -0.05f, 0f));
                    Check.That("Crouched character passes under low obstacle without Sides", (crouchFlags & CollisionFlags.Sides) == 0);
                    Check.That("Crouched character reaches x = 1.8 under low obstacle", Math.Abs(playerScript.transform.position.x - 1.8f) < 0.02f);
                    cc.height = 2.0f;
                    cc.center = Vector3.zero;

                    playerScript.transform.position = new Vector3(1.2f, 1.0f, -12f);
                    CollisionFlags depenFlags = cc.Move(Vector3.zero);
                    Check.That("Overlapping start depenetrates character outward (x >= 1.5)", playerScript.transform.position.x >= 1.5f - 0.01f);
                    Check.That("Overlapping start depenetration sets CollisionFlags.Sides", (depenFlags & CollisionFlags.Sides) != 0);

                    Rigidbody? platformBody = platformScript!.GetComponent<Rigidbody>();
                    Check.That("Moving platform has Rigidbody", platformBody != null);
                    Physics.Provider!.TrySetBodyVelocity(platformScript.gameObject.NativeEntityId, new Vector3(4f, 0f, -2f));
                    playerScript.transform.position = new Vector3(0f, 1.2f, 16f);
                    CollisionFlags platLand = cc.Move(new Vector3(0f, -0.3f, 0f));
                    Check.That("Landing on moving platform sets Below", (platLand & CollisionFlags.Below) != 0);
                    Check.Equal("Landing on moving platform sets isGrounded", true, cc.isGrounded);
                    Vector3 onPlatformPos = playerScript.transform.position;
                    CollisionFlags platCarry = cc.Move(new Vector3(0f, -0.05f, 0f));
                    Check.That("Moving platform carry keeps character grounded", (platCarry & CollisionFlags.Below) != 0 && cc.isGrounded);
                    Check.That("Moving platform carries character along +x", Math.Abs(playerScript.transform.position.x - (onPlatformPos.x + 0.08f)) < 0.01f);
                    Check.That("Moving platform carries character along -z", Math.Abs(playerScript.transform.position.z - (onPlatformPos.z - 0.04f)) < 0.01f);

                    playerScript.transform.position = new Vector3(0f, 6.0f, 16f);
                    Check.Equal("Direct transform.position teleport resets isGrounded to false", false, cc.isGrounded);
                    Check.That("Direct transform.position teleport resets collisionFlags to None", cc.collisionFlags == CollisionFlags.None);
                    CollisionFlags postTeleportFlags = cc.Move(new Vector3(0.5f, 0f, 0f));
                    Check.That("Move in air after teleport does not inherit platform velocity", Math.Abs(playerScript.transform.position.x - 0.5f) < 0.01f);
                    Check.That("Move in air after teleport keeps z unchanged", Math.Abs(playerScript.transform.position.z - 16f) < 0.01f);
                    Check.Equal("Move in air after teleport stays ungrounded", false, cc.isGrounded);
                    Check.That("Move in air after teleport has CollisionFlags.None", postTeleportFlags == CollisionFlags.None);

                    Vector3 beforeDisablePos = playerScript.transform.position;
                    cc.enabled = false;
                    CollisionFlags disabledFlags = cc.Move(new Vector3(3f, -3f, 1f));
                    Check.That("Move when enabled = false returns CollisionFlags.None", disabledFlags == CollisionFlags.None);
                    Check.Equal("Move when enabled = false sets isGrounded false", false, cc.isGrounded);
                    Check.That("Move when enabled = false keeps position unchanged", playerScript.transform.position == beforeDisablePos);
                    cc.enabled = true;
                }
                Check.That("Disposing BridgeHost clears CharacterController.Provider", CharacterController.Provider == null);
            }
            finally
            {
                Time.timeScale = savedScale;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private sealed class CharacterProbeScript : MonoBehaviour
        {
        }

        private sealed class PlatformVelocityScript : MonoBehaviour
        {
        }

        private const string CharacterControllerE2eSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Floor
  m_Layer: 0
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: -0.5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &102
BoxCollider:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Size: {x: 20, y: 1, z: 28}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &200
GameObject:
  m_Name: BackWall
  m_Layer: 0
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -10, y: 2, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &202
BoxCollider:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Size: {x: 1, y: 4, z: 10}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &300
GameObject:
  m_Name: Ceiling
  m_Layer: 0
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 3, z: -5}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &302
BoxCollider:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 0
  m_Size: {x: 6, y: 1, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &400
GameObject:
  m_Name: SmallStep
  m_Layer: 0
--- !u!4 &401
Transform:
  m_GameObject: {fileID: 400}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 2, y: 0.1, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &402
BoxCollider:
  m_GameObject: {fileID: 400}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 0.2, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &500
GameObject:
  m_Name: TallStep
  m_Layer: 0
--- !u!4 &501
Transform:
  m_GameObject: {fileID: 500}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 2, y: 0.25, z: 4}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &502
BoxCollider:
  m_GameObject: {fileID: 500}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 0.5, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &600
GameObject:
  m_Name: WalkableSlope
  m_Layer: 0
--- !u!4 &601
Transform:
  m_GameObject: {fileID: 600}
  m_LocalRotation: {x: 0, y: 0, z: 0.2164396, w: 0.976296}
  m_LocalPosition: {x: 3, y: 0.5, z: 8}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &602
BoxCollider:
  m_GameObject: {fileID: 600}
  m_IsTrigger: 0
  m_Size: {x: 6, y: 1, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &700
GameObject:
  m_Name: SteepSlope
  m_Layer: 0
--- !u!4 &701
Transform:
  m_GameObject: {fileID: 700}
  m_LocalRotation: {x: 0, y: 0, z: 0.5, w: 0.8660254}
  m_LocalPosition: {x: 2.5, y: 1.5, z: 12}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &702
BoxCollider:
  m_GameObject: {fileID: 700}
  m_IsTrigger: 0
  m_Size: {x: 6, y: 1, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &800
GameObject:
  m_Name: LowBeam
  m_Layer: 0
--- !u!4 &801
Transform:
  m_GameObject: {fileID: 800}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 2, y: 1.9, z: -8}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &802
BoxCollider:
  m_GameObject: {fileID: 800}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 1, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &900
GameObject:
  m_Name: OverlapBlock
  m_Layer: 0
--- !u!4 &901
Transform:
  m_GameObject: {fileID: 900}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1, z: -12}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &902
BoxCollider:
  m_GameObject: {fileID: 900}
  m_IsTrigger: 0
  m_Size: {x: 2, y: 2, z: 2}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!1 &1000
GameObject:
  m_Name: MovingPlatform
  m_Layer: 0
--- !u!4 &1001
Transform:
  m_GameObject: {fileID: 1000}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: -0.5, z: 16}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!54 &1002
Rigidbody:
  m_GameObject: {fileID: 1000}
  m_Mass: 1
  m_UseGravity: 0
  m_IsKinematic: 1
--- !u!65 &1003
BoxCollider:
  m_GameObject: {fileID: 1000}
  m_IsTrigger: 0
  m_Size: {x: 10, y: 1, z: 6}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!114 &1004
MonoBehaviour:
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 22222222333344445555666677778888, type: 3}
--- !u!1 &1100
GameObject:
  m_Name: Player
  m_Layer: 2
--- !u!4 &1101
Transform:
  m_GameObject: {fileID: 1100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1.5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!65 &1102
BoxCollider:
  m_GameObject: {fileID: 1100}
  m_IsTrigger: 0
  m_Size: {x: 1, y: 2, z: 1}
  m_Center: {x: 0, y: 0, z: 0}
--- !u!143 &1103
CharacterController:
  m_GameObject: {fileID: 1100}
  m_Enabled: 1
  m_Radius: 0.4
  m_Height: 1.8
  m_Center: {x: 0, y: 0.1, z: 0}
  m_SlopeLimit: 45
  m_StepOffset: 0.3
  m_SkinWidth: 0.05
  m_MinMoveDistance: 0.002
--- !u!114 &1104
MonoBehaviour:
  m_GameObject: {fileID: 1100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
";

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
