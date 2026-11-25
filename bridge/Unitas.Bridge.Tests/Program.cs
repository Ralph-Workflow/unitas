using System;
using System.Globalization;
using System.IO;
using System.Text;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class Program
    {
        private static readonly byte[] ExpectedAnimatorControllerBytes =
        {
            85, 65, 78, 67, 1, 0, 0, 0, 1, 0, 0, 0,
            67, 1, 0, 0, 0, 5, 0, 0, 0, 83, 112, 101,
            101, 100, 0, 0, 0, 192, 63, 2, 0, 0, 0, 4,
            0, 0, 0, 73, 100, 108, 101, 0, 0, 128, 63, 0,
            1, 4, 0, 0, 0, 67, 108, 105, 112, 0, 0, 128,
            63, 0, 0, 112, 66, 1, 1, 0, 0, 0, 0, 0,
            0, 0, 9, 0, 0, 0, 84, 114, 97, 110, 115, 102,
            111, 114, 109, 15, 0, 0, 0, 108, 111, 99, 97, 108,
            80, 111, 115, 105, 116, 105, 111, 110, 46, 120, 0, 2,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 128, 63,
            0, 0, 32, 65, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 1, 0, 0, 0, 0, 0, 0, 63, 6, 0, 0,
            0, 79, 110, 83, 116, 101, 112, 0, 0, 192, 63, 2,
            0, 0, 0, 3, 0, 0, 0, 116, 97, 103, 3, 0,
            0, 0, 82, 117, 110, 0, 0, 128, 63, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128,
            62, 1, 0, 0, 0, 5, 0, 0, 0, 83, 112, 101,
            101, 100, 2, 0, 0, 0, 63,
        };

        public static int Main(string[] args)
        {
            if (args.Length == 1 && args[0] == "--perf")
            {
                ScriptingPerfTests.Run();
                return Check.FailureCount > 0 ? 1 : 0;
            }

            TestVector3();
            TestGameObjectAndTransform();
            TestMonoBehaviourLifecycleOrder();
            TestPhysicsComponentDefaults();
            TestEndToEndRustLifecycleOrder();
            TestEndToEndRustCollisionOrder();
            TestEndToEndRustTriggerOrder();
            CollisionCounterpartTests.Run();
            SceneTransformSeedTests.Run();
            TestSceneReloadDropsPreviousSceneScripts();
            TestLayerMaskConversions();
            TestEndToEndRustPhysicsQuery();
            TimeDiagnosticsTests.Run();
            QuerySurfaceTests.Run();
            RandomTests.Run();
            IntegrationTests.Run();
            CoroutineTests.Run();
            TimeSemanticsTests.Run();
            LifetimeTests.Run();
            CloneTests.Run();
            ActivationTests.Run();
            ExceptionContainmentTests.Run();
            InputTests.Run();
            SceneManagementTests.Run();
            SceneManagementIntegrationTests.Run();
            SceneManagementPersistenceTests.Run();
            SceneMergeTests.Run();
            ApplicationTests.Run();
            RuntimeInitializeTests.Run();
            PlayerPrefsTests.Run();
            ObjectDiscoveryTests.Run();
            GameObjectDiscoveryTests.Run();
            LoggingTests.Run();
            RenderTests.Run();
            PlaytestTests.Run();
            RollABallScenarioTests.Run();
            TestCharacterControllerDefaultsAndNullProvider();
            TestEndToEndCharacterControllerMoveAndSceneLoad();
            TestAudioDefaultsAndNullProvider();
            TestEndToEndAudioPlaybackAndSceneLoad();
            TestAnimatorDefaultsAndNullProvider();
            TestAnimatorControllerBinaryEncoding();
            TestEndToEndAnimatorControllerAndTransitions();
            CoreApiTests.Run();
            ValueTypeCompletionTests.Run();
            CompilationTests.Run();
            EventsTests.Run();
            UnityEventArityTests.Run();
            UnityWebRequestMethodTests.Run();
            ResourcesSceneTests.Run();
            SceneAcceptanceTests.Run();
            LifecycleAcceptanceTests.Run();
            CoroutineAcceptanceTests.Run();
            PlayerPrefsPersistenceTests.Run();
            JsonUtilityTests.Run();
            JsonUtilityRoundTripTests.Run();
            SerializerMatrixTests.Run();
            IdeGenerationTests.Run();
            AgentWorkflowTests.Run();
            AgentCodeLoopTests.Run();
            UtfRunnerTests.Run();
            UtfHardeningTests.Run();
            DebuggingTests.Run();
            DapSteppingTests.Run();
            EditorMenuTests.Run();
            LifecycleMessageTests.Run();
            ExecutionOrderDispatchTests.Run();
            FixedStepDeterminismTests.Run();
            SaveMigrationTests.Run();

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

            ScriptGlueRegistry.Clear();
            LifecycleScript recordedScript = new LifecycleScript();
            ScriptGlueRegistry.Register(0UL, 1UL, () => recordedScript);

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

            string observed = recordedScript.ObservedOrder;
            Console.WriteLine("Observed collision order: " + observed);

            string[] events = observed.Split(", ");
            bool exactlyOneEnterThenStaysThenExit = events.Length >= 3;
            for (int index = 1; index < events.Length - 1; index++)
            {
                if (events[index] != "Stay")
                {
                    exactlyOneEnterThenStaysThenExit = false;
                }
            }

            Check.That(
                "End-to-end collision order is exactly Enter, Stays, Exit",
                exactlyOneEnterThenStaysThenExit && events[0] == "Enter" && events[events.Length - 1] == "Exit");
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
            Check.That("Collision gameObject is resolved when the other party carries no script", !recordedScript.OtherGameObjectAlwaysNull);
        }

        private static void TestEndToEndRustTriggerOrder()
        {
            string? envScene = Environment.GetEnvironmentVariable("UNITAS_TEST_SCENE");
            string scenePath = !string.IsNullOrEmpty(envScene) && File.Exists(envScene)
                ? Path.GetFullPath(envScene)
                : ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_trigger.unity");

            Check.That("Trigger scene file exists", File.Exists(scenePath));

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
            Check.That("Trigger other collider is resolved when the other party carries no script", !recordedScript.OtherColliderAlwaysNull);
        }

        private static void TestSceneReloadDropsPreviousSceneScripts()
        {
            string scriptedScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision.unity");
            string plainScene = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_collision_no_script.unity");

            Check.That("Scripted collision scene file exists", File.Exists(scriptedScene));
            Check.That("Plain collision scene file exists", File.Exists(plainScene));

            ScriptGlueRegistry.Clear();
            CollisionRecorderScript recordedScript = new CollisionRecorderScript();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => recordedScript);

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

            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new PhysicsQueryScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new PhysicsQueryScript());

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
                ScriptGlueRegistry.Clear();
                CharacterProbeScript? playerScript = null;
                PlatformVelocityScript? platformScript = null;
                ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => playerScript = new CharacterProbeScript());
                ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => platformScript = new PlatformVelocityScript());

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
                    CollisionFlags firstMulti = cc.Move(new Vector3(0.2f, -0.05f, 0f));
                    CollisionFlags secondMulti = cc.Move(new Vector3(0.2f, -0.05f, 0.25f));
                    Check.That("First move in same frame stays grounded", (firstMulti & CollisionFlags.Below) != 0 && cc.isGrounded);
                    Check.That("Second move in same frame composes position x", Math.Abs(playerScript.transform.position.x - (groundedStart.x + 0.4f)) < 0.02f);
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
                    Check.That("Kinematic rotation write reaches the physics world", Physics.Provider!.TrySetBodyRotation(platformScript.gameObject.NativeEntityId, Quaternion.AngleAxis(90f, Vector3.up)));
                    Check.That("Kinematic rotation write restores the original orientation", Physics.Provider!.TrySetBodyRotation(platformScript.gameObject.NativeEntityId, Quaternion.identity));
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

        private static void TestAudioDefaultsAndNullProvider()
        {
            IAudioProvider? savedProvider = AudioSource.Provider;
            AudioSource.Provider = null;
            float savedListenerVolume = AudioListener.volume;
            bool savedListenerPause = AudioListener.pause;
            try
            {
                Check.Equal("AudioSourceState.Scheduled value", 0, (int)AudioSourceState.Scheduled);
                Check.Equal("AudioSourceState.Loading value", 1, (int)AudioSourceState.Loading);
                Check.Equal("AudioSourceState.Playing value", 2, (int)AudioSourceState.Playing);
                Check.Equal("AudioSourceState.Paused value", 3, (int)AudioSourceState.Paused);
                Check.Equal("AudioSourceState.Virtualized value", 4, (int)AudioSourceState.Virtualized);
                Check.Equal("AudioSourceState.Stopped value", 5, (int)AudioSourceState.Stopped);
                Check.Equal("AudioSourceState.Completed value", 6, (int)AudioSourceState.Completed);
                Check.Equal("AudioSourceState.Failed value", 7, (int)AudioSourceState.Failed);

                Check.Equal("AudioSilenceReason.None value", -1, (int)AudioSilenceReason.None);
                Check.Equal("AudioSilenceReason.ZeroVolume value", 0, (int)AudioSilenceReason.ZeroVolume);
                Check.Equal("AudioSilenceReason.Muted value", 1, (int)AudioSilenceReason.Muted);
                Check.Equal("AudioSilenceReason.Paused value", 2, (int)AudioSilenceReason.Paused);
                Check.Equal("AudioSilenceReason.Virtualized value", 3, (int)AudioSilenceReason.Virtualized);
                Check.Equal("AudioSilenceReason.OutOfRange value", 4, (int)AudioSilenceReason.OutOfRange);
                Check.Equal("AudioSilenceReason.NoListener value", 5, (int)AudioSilenceReason.NoListener);
                Check.Equal("AudioSilenceReason.UnsupportedClip value", 6, (int)AudioSilenceReason.UnsupportedClip);
                Check.Equal("AudioSilenceReason.Stopped value", 7, (int)AudioSilenceReason.Stopped);
                Check.Equal("AudioSilenceReason.Completed value", 8, (int)AudioSilenceReason.Completed);

                AudioClip emptyClip = new AudioClip();
                Check.Equal("AudioClip default name", string.Empty, emptyClip.name);
                Check.Equal("AudioClip default length", 0f, emptyClip.length);
                Check.Equal("AudioClip default channels", 1, emptyClip.channels);
                Check.Equal("AudioClip default frequency", 44100, emptyClip.frequency);
                Check.Equal("AudioClip default sampleRate", 44100, emptyClip.sampleRate);
                Check.Equal("AudioClip default samples", 0, emptyClip.samples);
                Check.That("AudioClip default NativeClipId is 0", emptyClip.NativeClipId == 0UL);

                AudioClip createdClip = AudioClip.Create("Tone", 500, 2, 1000);
                Check.Equal("AudioClip.Create name", "Tone", createdClip.name);
                Check.Equal("AudioClip.Create length", 0.5f, createdClip.length);
                Check.Equal("AudioClip.Create channels", 2, createdClip.channels);
                Check.Equal("AudioClip.Create frequency", 1000, createdClip.frequency);
                Check.Equal("AudioClip.Create samples", 500, createdClip.samples);
                Check.That("AudioClip.Create with null provider leaves NativeClipId 0", createdClip.NativeClipId == 0UL);
                Check.That("AudioClip.LoadFromFile with null provider returns null", AudioClip.LoadFromFile("nonexistent.wav") == null);

                GameObject listenerGo = new GameObject("ListenerDefaults");
                AudioListener listener = listenerGo.AddComponent<AudioListener>();
                Check.Equal("AudioListener default static volume", 1.0f, AudioListener.volume);
                Check.Equal("AudioListener default static pause", false, AudioListener.pause);
                Check.That("AudioListener default forward is (0, 0, 1)", listener.forward == new Vector3(0f, 0f, 1f));
                Check.That("AudioListener default Velocity is zero", listener.Velocity == Vector3.zero);
                UnityEngine.Object.Destroy(listenerGo);

                GameObject go = new GameObject("AudioDefaults");
                AudioSource source = go.AddComponent<AudioSource>();
                Check.That("GetComponent<AudioSource> resolves source", ReferenceEquals(go.GetComponent<AudioSource>(), source));
                Check.Equal("AudioSource default enabled", true, source.enabled);
                Check.Equal("AudioSource default volume", 1.0f, source.volume);
                Check.Equal("AudioSource default pitch", 1.0f, source.pitch);
                Check.Equal("AudioSource default priority", 128, source.priority);
                Check.Equal("AudioSource default minDistance", 1.0f, source.minDistance);
                Check.Equal("AudioSource default maxDistance", 500.0f, source.maxDistance);
                Check.Equal("AudioSource default playOnAwake", true, source.playOnAwake);
                Check.Equal("AudioSource default loop", false, source.loop);
                Check.Equal("AudioSource default mute", false, source.mute);
                Check.Equal("AudioSource default spatialBlend", 0.0f, source.spatialBlend);
                Check.Equal("AudioSource default time", 0.0f, source.time);
                Check.Equal("AudioSource default isPlaying", false, source.isPlaying);
                Check.That("AudioSource default state is Stopped", source.state == AudioSourceState.Stopped);
                Check.That("AudioSource default silenceReason is Stopped", source.silenceReason == AudioSilenceReason.Stopped);
                Check.That("AudioSource default clip is null", source.clip == null);

                source.clip = createdClip;
                source.volume = 0.75f;
                source.pitch = 1.25f;
                source.priority = 64;
                source.minDistance = 2.0f;
                source.maxDistance = 50.0f;
                source.playOnAwake = false;
                source.loop = true;
                source.spatialBlend = 0.5f;
                source.time = 0.2f;
                Check.That("AudioSource clip round trip", ReferenceEquals(source.clip, createdClip));
                Check.Equal("AudioSource volume round trip", 0.75f, source.volume);
                Check.Equal("AudioSource pitch round trip", 1.25f, source.pitch);
                Check.Equal("AudioSource priority round trip", 64, source.priority);
                Check.Equal("AudioSource minDistance round trip", 2.0f, source.minDistance);
                Check.Equal("AudioSource maxDistance round trip", 50.0f, source.maxDistance);
                Check.Equal("AudioSource playOnAwake round trip", false, source.playOnAwake);
                Check.Equal("AudioSource loop round trip", true, source.loop);
                Check.Equal("AudioSource spatialBlend round trip", 0.5f, source.spatialBlend);
                Check.Equal("AudioSource time round trip", 0.2f, source.time);

                source.mute = true;
                Check.Equal("AudioSource mute round trip", true, source.mute);
                Check.That("AudioSource silenceReason when muted (null provider)", source.silenceReason == AudioSilenceReason.Muted);
                source.mute = false;

                source.volume = 0f;
                Check.That("AudioSource silenceReason when zero volume (null provider)", source.silenceReason == AudioSilenceReason.ZeroVolume);
                source.volume = 1f;
                Check.That("AudioSource silenceReason when unmuted and stopped (null provider)", source.silenceReason == AudioSilenceReason.Stopped);

                source.Play();
                Check.Equal("AudioSource Play with null provider sets isPlaying true", true, source.isPlaying);
                Check.That("AudioSource Play with null provider sets state Playing", source.state == AudioSourceState.Playing);

                source.Pause();
                Check.Equal("AudioSource Pause with null provider sets isPlaying false", false, source.isPlaying);
                Check.That("AudioSource Pause with null provider sets state Paused", source.state == AudioSourceState.Paused);

                source.UnPause();
                Check.Equal("AudioSource UnPause with null provider sets isPlaying true", true, source.isPlaying);
                Check.That("AudioSource UnPause with null provider sets state Playing", source.state == AudioSourceState.Playing);

                source.Stop();
                Check.Equal("AudioSource Stop with null provider sets isPlaying false", false, source.isPlaying);
                Check.That("AudioSource Stop with null provider sets state Stopped", source.state == AudioSourceState.Stopped);
                Check.Equal("AudioSource Stop with null provider resets time to 0", 0f, source.time);

                source.PlayOneShot(createdClip, 0.5f);
                Check.Equal("AudioSource PlayOneShot with null provider sets isPlaying true", true, source.isPlaying);
                source.Stop();
                UnityEngine.Object.Destroy(go);
                UnityEngine.Object.Destroy(emptyClip);
                UnityEngine.Object.Destroy(createdClip);
            }
            finally
            {
                AudioListener.volume = savedListenerVolume;
                AudioListener.pause = savedListenerPause;
                AudioSource.Provider = savedProvider;
            }
        }

        private static void TestEndToEndAudioPlaybackAndSceneLoad()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_audio_e2e_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string wavPath = Path.Combine(tempDir, "beep.wav");
            string wavMetaPath = Path.Combine(tempDir, "beep.wav.meta");
            string scenePath = Path.Combine(tempDir, "audio_e2e.unity");

            File.WriteAllBytes(wavPath, BuildTestWav16BitPcmMono(1000, 500));
            File.WriteAllText(wavMetaPath, "fileFormatVersion: 2\nguid: 00000000000000000000000000000082\n");
            File.WriteAllText(scenePath, AudioE2eSceneYaml);

            float savedScale = Time.timeScale;
            bool savedListenerPause = AudioListener.pause;
            Time.timeScale = 1f;
            AudioListener.pause = false;
            try
            {
                ScriptGlueRegistry.Clear();
                AudioStarterScript? starterScript = null;
                ScriptGlueRegistry.Register(0x1111111111111111UL, 0x2222222222222222UL, () => starterScript = new AudioStarterScript());

                using (BridgeHost host = new BridgeHost())
                {
                    Check.That("BridgeHost wires AudioSource.Provider", ReferenceEquals(AudioSource.Provider, host));
                    host.LoadScene(scenePath);
                    host.Tick(0.016);

                    Check.That("AudioStarterScript created and Start ran on first tick", starterScript != null && starterScript.StartCalled);
                    Check.Equal("Start observed scene-loaded volume 0.8", 0.8f, starterScript!.ObservedVolume);
                    Check.Equal("Start observed scene-loaded pitch 1.0", 1.0f, starterScript.ObservedPitch);
                    Check.Equal("Start observed scene-loaded priority 64", 64, starterScript.ObservedPriority);
                    Check.Equal("Start observed scene-loaded playOnAwake false", false, starterScript.ObservedPlayOnAwake);
                    Check.That("Start observed non-null scene-loaded AudioClip", starterScript.ObservedClipPresent);
                    Check.Equal("Start observed scene-loaded clip length 0.5s", 0.5f, starterScript.ObservedClipLength);
                    Check.Equal("Start observed scene-loaded clip channels 1", 1, starterScript.ObservedClipChannels);
                    Check.Equal("Start observed scene-loaded clip frequency 1000", 1000, starterScript.ObservedClipFrequency);

                    AudioSource? audioSource = starterScript.GetComponent<AudioSource>();
                    Check.That("AudioSource component remains attached", audioSource != null);
                    ulong entityId = starterScript.gameObject.NativeEntityId;
                    Check.That("AudioSource GameObject has non-zero NativeEntityId", entityId != 0UL);

                    Check.Equal("AudioSource.isPlaying is true after Start calls Play()", true, audioSource!.isPlaying);
                    Check.That("AudioSource.state is Playing after Start", audioSource.state == AudioSourceState.Playing);
                    Check.That("Native GetAudioState is Playing after Start", host.GetAudioState(entityId) == AudioSourceState.Playing);
                    Check.That("AudioSource.silenceReason is None while audible", audioSource.silenceReason == AudioSilenceReason.None);
                    Check.That("Native GetAudioSilenceReason is None (-1) while audible", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);

                    float[] mixedSamples = host.MixAudio(64);
                    Check.Equal("MixAudio(64) returns 128 interleaved stereo samples", 128, mixedSamples.Length);
                    bool hasNonZeroSample = false;
                    for (int i = 0; i < mixedSamples.Length; i++)
                    {
                        if (Math.Abs(mixedSamples[i]) > 1e-4f)
                        {
                            hasNonZeroSample = true;
                            break;
                        }
                    }
                    Check.That("MixAudio(64) produces non-zero stereo samples", hasNonZeroSample);

                    audioSource.mute = true;
                    Check.That("Native GetAudioSilenceReason is Muted when mute = true", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.Muted);
                    Check.That("AudioSource.silenceReason is Muted when mute = true", audioSource.silenceReason == AudioSilenceReason.Muted);
                    audioSource.mute = false;
                    Check.That("Native GetAudioSilenceReason returns to None when unmuted", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);

                    audioSource.volume = 0f;
                    Check.That("Native GetAudioSilenceReason is ZeroVolume when volume = 0", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.ZeroVolume);
                    audioSource.volume = 0.8f;
                    Check.That("Native GetAudioSilenceReason returns to None when volume restored", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);

                    audioSource.Pause();
                    Check.Equal("AudioSource.isPlaying is false when paused", false, audioSource.isPlaying);
                    Check.That("AudioSource.state is Paused after Pause()", audioSource.state == AudioSourceState.Paused);
                    Check.That("Native GetAudioState is Paused after Pause()", host.GetAudioState(entityId) == AudioSourceState.Paused);
                    Check.That("Native GetAudioSilenceReason is Paused after Pause()", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.Paused);

                    audioSource.UnPause();
                    Check.Equal("AudioSource.isPlaying is true after UnPause()", true, audioSource.isPlaying);
                    Check.That("AudioSource.state is Playing after UnPause()", audioSource.state == AudioSourceState.Playing);
                    Check.That("Native GetAudioState is Playing after UnPause()", host.GetAudioState(entityId) == AudioSourceState.Playing);
                    Check.That("Native GetAudioSilenceReason is None after UnPause()", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);

                    GameObject listenerObj = new GameObject("MainAudioListener");
                    listenerObj.transform.position = new Vector3(0f, 0f, 0f);
                    AudioListener listener = listenerObj.AddComponent<AudioListener>();
                    listener.forward = new Vector3(0f, 0f, 1f);

                    audioSource.spatialBlend = 1.0f;
                    audioSource.minDistance = 1.0f;
                    audioSource.maxDistance = 10.0f;
                    starterScript.transform.position = new Vector3(0f, 0f, 20f);
                    host.Tick(0.01);
                    Check.That("Native GetAudioSilenceReason is OutOfRange at distance 20 (maxDistance 10)", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.OutOfRange);
                    Check.That("AudioSource.silenceReason is OutOfRange at distance 20", audioSource.silenceReason == AudioSilenceReason.OutOfRange);

                    double timeAtOutOfRange = host.GetAudioTime(entityId);
                    starterScript.transform.position = new Vector3(0f, 0f, 2f);
                    host.Tick(0.1);
                    double timeAfterInRangeTick = host.GetAudioTime(entityId);
                    Check.That("Native GetAudioSilenceReason is None after moving back inside range (z = 2)", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);
                    Check.That("GetAudioTime advanced after 0.1s tick in range", timeAfterInRangeTick > timeAtOutOfRange + 0.08);

                    UnityEngine.Object.Destroy(listenerObj);
                    host.Tick(0.0);
                    Check.That("Native GetAudioSilenceReason is NoListener when active AudioListener is removed", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.NoListener);

                    listenerObj = new GameObject("RestoredAudioListener");
                    listenerObj.transform.position = new Vector3(0f, 0f, 0f);
                    listener = listenerObj.AddComponent<AudioListener>();
                    listener.forward = new Vector3(0f, 0f, 1f);

                    Time.timeScale = 0f;
                    host.Tick(0.1);
                    double timeAfterZeroScaleTick = host.GetAudioTime(entityId);
                    Check.Equal("GetAudioTime is unchanged when Time.timeScale = 0", timeAfterInRangeTick, timeAfterZeroScaleTick);
                    Check.That("SyncAudioBeforeTick restores active AudioListener pose", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.None);
                    Time.timeScale = 1f;

                    host.SetPaused(true);
                    host.Tick(0.2);
                    double timeWhileAppPaused = host.GetAudioTime(entityId);
                    Check.Equal("GetAudioTime is unchanged while application is paused", timeAfterZeroScaleTick, timeWhileAppPaused);
                    host.SetPaused(false);

                    AudioClip createdNativeClip = AudioClip.Create("GeneratedTone", 200, 1, 1000);
                    Check.That("AudioClip.Create with active provider registers non-zero NativeClipId", createdNativeClip.NativeClipId != 0UL);
                    AudioClip? loadedFileClip = AudioClip.LoadFromFile(wavPath);
                    Check.That("AudioClip.LoadFromFile loads WAV clip with non-zero NativeClipId", loadedFileClip != null && loadedFileClip.NativeClipId != 0UL);
                    Check.Equal("AudioClip.LoadFromFile duration is 0.5s", 0.5f, loadedFileClip!.length);
                    UnityEngine.Object.Destroy(createdNativeClip);
                    UnityEngine.Object.Destroy(loadedFileClip);

                    host.Tick(0.5);
                    Check.Equal("AudioSource.isPlaying is false after clip duration completes", false, audioSource.isPlaying);
                    Check.That("AudioSource.state is Completed after advancing past duration", audioSource.state == AudioSourceState.Completed);
                    Check.That("Native GetAudioState is Completed after advancing past duration", host.GetAudioState(entityId) == AudioSourceState.Completed);
                    Check.That("Native GetAudioSilenceReason is Completed after advancing past duration", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.Completed);

                    audioSource.Stop();
                    Check.That("Native GetAudioState is Stopped after Stop()", host.GetAudioState(entityId) == AudioSourceState.Stopped);
                    Check.That("Native GetAudioSilenceReason is Stopped after Stop()", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.Stopped);

                    audioSource.Play();
                    Check.That("Native GetAudioState is Playing before enabled = false", host.GetAudioState(entityId) == AudioSourceState.Playing);
                    audioSource.enabled = false;
                    Check.That("Disabling AudioSource stops playback natively", host.GetAudioState(entityId) == AudioSourceState.Stopped);
                    audioSource.enabled = true;

                    audioSource.Play();
                    Check.That("Native GetAudioState is Playing before SetActive(false)", host.GetAudioState(entityId) == AudioSourceState.Playing);
                    starterScript.gameObject.SetActive(false);
                    Check.That("Deactivating GameObject stops playback natively", host.GetAudioState(entityId) == AudioSourceState.Stopped);
                    starterScript.gameObject.SetActive(true);

                    audioSource.Play();
                    Check.That("Native GetAudioState is Playing before scene teardown", host.GetAudioState(entityId) == AudioSourceState.Playing);
                    string emptyScenePath = Path.Combine(tempDir, "empty_teardown.unity");
                    File.WriteAllText(emptyScenePath, EmptyTeardownSceneYaml);
                    host.LoadScene(emptyScenePath);
                    Check.That("Scene reload tears down previous scene native AudioSource", host.GetAudioState(entityId) == AudioSourceState.Stopped);
                    Check.That("Scene reload native SilenceReason is Stopped for torn-down entity", host.GetAudioSilenceReason(entityId) == AudioSilenceReason.Stopped);

                    UnityEngine.Object.Destroy(listenerObj);
                    host.Shutdown();
                }
                Check.That("Disposing BridgeHost clears AudioSource.Provider", AudioSource.Provider == null);
            }
            finally
            {
                AudioListener.pause = savedListenerPause;
                Time.timeScale = savedScale;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestAnimatorDefaultsAndNullProvider()
        {
            IAnimationProvider? savedProvider = Animator.Provider;
            Animator.Provider = null;
            try
            {
                GameObject go = new GameObject("AnimatorDefaults");
                Animator animator = go.AddComponent<Animator>();

                Check.Equal("Animator default speed", 1f, animator.speed);
                Check.That("Animator has no controller by default", animator.runtimeAnimatorController == null);
                Check.That("Animator state info is empty without a provider", string.IsNullOrEmpty(animator.GetCurrentAnimatorStateInfo().name));

                animator.SetFloat("Speed", 1f);
                animator.SetInteger("Count", 2);
                animator.SetBool("Enabled", true);
                animator.SetTrigger("Go");
                animator.ResetTrigger("Go");
                animator.speed = 2f;

                Check.Equal("Animator GetFloat returns 0 without a provider", 0f, animator.GetFloat("Speed"));
                Check.Equal("Animator GetInteger returns 0 without a provider", 0, animator.GetInteger("Count"));
                Check.Equal("Animator GetBool returns false without a provider", false, animator.GetBool("Enabled"));
                Check.Equal("Animator speed is stored without a provider", 2f, animator.speed);
            }
            finally
            {
                Animator.Provider = savedProvider;
            }
        }

        private static void TestAnimatorControllerBinaryEncoding()
        {
            AnimationClip clip = new AnimationClip("Clip");
            clip.duration = 1f;
            clip.sampleRate = 60f;
            clip.wrapMode = AnimationWrapMode.Loop;
            clip.SetCurve(
                string.Empty,
                AnimationPropertyType.Float,
                "localPosition.x",
                new AnimationCurve(
                    new Keyframe(0f, 0f, 0f, 0f, KeyframeInterpolationMode.Linear),
                    new Keyframe(1f, 10f, 0f, 0f, KeyframeInterpolationMode.Linear)));
            clip.AddEvent(0.5f, "OnStep", 1.5f, 2, "tag");

            AnimatorController controller = new AnimatorController("C");
            controller.AddParameter("Speed", 1.5f);
            controller.AddState("Idle", clip);
            controller.AddState("Run", null);
            controller.SetEntryState("Idle");
            controller.AddTransition(
                "Idle",
                "Run",
                0.25f,
                new AnimatorCondition("Speed", AnimatorConditionMode.Greater, 0.5f));

            byte[] encoded = AnimatorControllerEncoder.Encode(controller);
            byte[] expected = ExpectedAnimatorControllerBytes;

            Check.Equal("Animator controller encodes to the expected byte length", expected.Length, encoded.Length);
            Check.That("Animator controller encodes to the expected binary layout", BytesEqual(expected, encoded));
        }

        private static void TestEndToEndAnimatorControllerAndTransitions()
        {
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                string firstRun = RunAnimatorScenario();
                string secondRun = RunAnimatorScenario();
                Check.Equal("Animator scenario is repeatable", firstRun, secondRun);
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static string RunAnimatorScenario()
        {
            StringBuilder log = new StringBuilder();
            using (BridgeHost host = new BridgeHost())
            {
                GameObject target = new GameObject("AnimatedTarget");
                Animator animator = target.AddComponent<Animator>();
                animator.runtimeAnimatorController = BuildAnimatorController();

                host.Tick(0.0);

                AnimatorStateInfo info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "initial", info);
                Check.Equal("Animator starts in the entry state", "Idle", info.name);
                Check.Equal("Animator starts with zero normalized time", 0f, info.normalizedTime);

                animator.SetFloat("Speed", 1f);
                Check.Equal("Animator float parameter round-trips", 1f, animator.GetFloat("Speed"));

                host.Tick(0.1);
                info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "idleTick", info);
                Check.Equal("Animator stays in Idle before the trigger fires", "Idle", info.name);
                Check.Equal("Animator normalized time advances", 0.1f, info.normalizedTime);
                Check.Equal("Animator applies the sampled clip position", 1f, info.position.x);

                animator.SetTrigger("Go");
                host.Tick(0.1);
                info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "transition", info);
                Check.Equal("Animator transitions to Run once the trigger fires", "Run", info.name);
                Check.Equal("Animator resets the state time on transition", 0f, info.normalizedTime);
                Check.Equal("Animator samples the Run clip after the transition", 100f, info.position.x);

                host.Tick(0.1);
                info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "consumedTrigger", info);
                Check.Equal("Animator keeps the Run state while the trigger is consumed", "Run", info.name);
                Check.Equal("Animator advances the Run state", 0.1f, info.normalizedTime);
                Check.Equal("Animator keeps sampling the Run clip", 101f, info.position.x);

                animator.SetTrigger("Go");
                host.Tick(0.1);
                info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "secondTrigger", info);
                Check.Equal("Animator returns to Idle on the second trigger", "Idle", info.name);
                Check.Equal("Animator samples the Idle clip after the second transition", 0f, info.position.x);

                Time.timeScale = 0.5f;
                host.Tick(0.2);
                info = animator.GetCurrentAnimatorStateInfo();
                AppendAnimatorState(log, "halfSpeed", info);
                Check.Equal("Animator advances at half rate when the time scale is 0.5", 0.1f, info.normalizedTime);
                Check.Equal("Animator samples at half rate when the time scale is 0.5", 1f, info.position.x);
                Time.timeScale = 1f;

                host.Shutdown();
            }

            return log.ToString();
        }

        private static AnimatorController BuildAnimatorController()
        {
            AnimationClip idleClip = new AnimationClip("IdleClip");
            idleClip.duration = 1f;
            idleClip.sampleRate = 60f;
            idleClip.wrapMode = AnimationWrapMode.Loop;
            idleClip.SetCurve(
                string.Empty,
                AnimationPropertyType.Float,
                "localPosition.x",
                new AnimationCurve(
                    new Keyframe(0f, 0f, 0f, 0f, KeyframeInterpolationMode.Linear),
                    new Keyframe(1f, 10f, 0f, 0f, KeyframeInterpolationMode.Linear)));

            AnimationClip runClip = new AnimationClip("RunClip");
            runClip.duration = 1f;
            runClip.sampleRate = 60f;
            runClip.wrapMode = AnimationWrapMode.Loop;
            runClip.SetCurve(
                string.Empty,
                AnimationPropertyType.Float,
                "localPosition.x",
                new AnimationCurve(
                    new Keyframe(0f, 100f, 0f, 0f, KeyframeInterpolationMode.Linear),
                    new Keyframe(1f, 110f, 0f, 0f, KeyframeInterpolationMode.Linear)));

            AnimatorController controller = new AnimatorController("Locomotion");
            controller.AddParameter("Speed", 0f);
            controller.AddTriggerParameter("Go");
            controller.AddState("Idle", idleClip);
            controller.AddState("Run", runClip);
            controller.SetEntryState("Idle");
            controller.AddTransition(
                "Idle",
                "Run",
                new AnimatorCondition("Speed", AnimatorConditionMode.Greater, 0.5f),
                new AnimatorCondition("Go", AnimatorConditionMode.If));
            controller.AddTransition(
                "Run",
                "Idle",
                new AnimatorCondition("Go", AnimatorConditionMode.If));
            return controller;
        }

        private static void AppendAnimatorState(StringBuilder log, string label, AnimatorStateInfo info)
        {
            log.Append(label)
                .Append(':')
                .Append(info.name)
                .Append(',')
                .Append(info.normalizedTime.ToString("F4", CultureInfo.InvariantCulture))
                .Append(',')
                .Append(info.position.x.ToString("F4", CultureInfo.InvariantCulture))
                .Append(';');
        }

        private static bool BytesEqual(byte[] expected, byte[] actual)
        {
            if (expected.Length != actual.Length)
            {
                return false;
            }

            for (int index = 0; index < expected.Length; index++)
            {
                if (expected[index] != actual[index])
                {
                    return false;
                }
            }

            return true;
        }

        private static byte[] BuildTestWav16BitPcmMono(int sampleRate, int frameCount)
        {
            int dataBytes = frameCount * 2;
            int riffSize = 36 + dataBytes;
            byte[] buffer = new byte[44 + dataBytes];

            buffer[0] = (byte)'R';
            buffer[1] = (byte)'I';
            buffer[2] = (byte)'F';
            buffer[3] = (byte)'F';
            BitConverter.GetBytes(riffSize).CopyTo(buffer, 4);
            buffer[8] = (byte)'W';
            buffer[9] = (byte)'A';
            buffer[10] = (byte)'V';
            buffer[11] = (byte)'E';

            buffer[12] = (byte)'f';
            buffer[13] = (byte)'m';
            buffer[14] = (byte)'t';
            buffer[15] = (byte)' ';
            BitConverter.GetBytes(16).CopyTo(buffer, 16);
            BitConverter.GetBytes((short)1).CopyTo(buffer, 20);
            BitConverter.GetBytes((short)1).CopyTo(buffer, 22);
            BitConverter.GetBytes(sampleRate).CopyTo(buffer, 24);
            BitConverter.GetBytes(sampleRate * 2).CopyTo(buffer, 28);
            BitConverter.GetBytes((short)2).CopyTo(buffer, 32);
            BitConverter.GetBytes((short)16).CopyTo(buffer, 34);

            buffer[36] = (byte)'d';
            buffer[37] = (byte)'a';
            buffer[38] = (byte)'t';
            buffer[39] = (byte)'a';
            BitConverter.GetBytes(dataBytes).CopyTo(buffer, 40);

            for (int i = 0; i < frameCount; i++)
            {
                short sample = (i % 2 == 0) ? (short)12000 : (short)-12000;
                BitConverter.GetBytes(sample).CopyTo(buffer, 44 + (i * 2));
            }

            return buffer;
        }

        private sealed class AudioStarterScript : MonoBehaviour
        {
            public bool StartCalled { get; private set; }

            public float ObservedVolume { get; private set; }

            public float ObservedPitch { get; private set; }

            public int ObservedPriority { get; private set; }

            public bool ObservedPlayOnAwake { get; private set; }

            public bool ObservedClipPresent { get; private set; }

            public float ObservedClipLength { get; private set; }

            public int ObservedClipChannels { get; private set; }

            public int ObservedClipFrequency { get; private set; }

            public override void Start()
            {
                StartCalled = true;
                AudioSource? source = GetComponent<AudioSource>();
                if (source != null)
                {
                    ObservedVolume = source.volume;
                    ObservedPitch = source.pitch;
                    ObservedPriority = source.priority;
                    ObservedPlayOnAwake = source.playOnAwake;
                    AudioClip? clip = source.clip;
                    if (clip != null)
                    {
                        ObservedClipPresent = true;
                        ObservedClipLength = clip.length;
                        ObservedClipChannels = clip.channels;
                        ObservedClipFrequency = clip.frequency;
                    }
                    source.Play();
                }
            }
        }

        private const string AudioE2eSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: Emitter
  m_Layer: 0
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!82 &102
AudioSource:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_audioClip: {fileID: 8300000, guid: 00000000000000000000000000000082, type: 3}
  m_PlayOnAwake: 0
  m_Volume: 0.8
  m_Pitch: 1
  Loop: 0
  Mute: 0
  Priority: 64
  m_SpatialBlend: 0
  MinDistance: 1
  MaxDistance: 500
--- !u!114 &103
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11111111111111112222222222222222, type: 3}
";

        private const string EmptyTeardownSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: EmptyRoot
  m_Layer: 0
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
";

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
