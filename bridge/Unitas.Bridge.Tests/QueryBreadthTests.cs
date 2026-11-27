using System;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class QueryBreadthTests
    {
        public static void Run()
        {
            TestMissingProviderFallbacks();
            TestProviderDelegation3D();
            TestProviderDelegation2D();
            TestNativeRaycastAllAndNonAlloc();
            TestNativeSphereCastAllAndNonAlloc();
            TestNativeCapsuleCast3D();
            TestNativeOverlapsAndChecks3D();
            TestNativeCircleCastAndCapsuleCast2D();
        }

        private static void TestMissingProviderFallbacks()
        {
            IPhysicsProvider? previous3D = Physics.Provider;
            IPhysics2DProvider? previous2D = Physics2D.Provider;
            Physics.Provider = null;
            Physics2D.Provider = null;
            try
            {
                Vector3 origin = new Vector3(0f, 0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);
                Ray ray = new Ray(origin, direction);
                RaycastHit[] buffer = new RaycastHit[4];

                Check.Equal("RaycastAll without provider returns empty array", 0, Physics.RaycastAll(origin, direction, 10f).Length);
                Check.Equal("RaycastAll Ray overload without provider returns empty array", 0, Physics.RaycastAll(ray, 10f).Length);
                Check.Equal("RaycastNonAlloc without provider returns 0", 0, Physics.RaycastNonAlloc(origin, direction, buffer, 10f));
                Check.Equal("RaycastNonAlloc Ray overload without provider returns 0", 0, Physics.RaycastNonAlloc(ray, buffer, 10f));
                Check.Equal("SphereCastAll without provider returns empty array", 0, Physics.SphereCastAll(origin, 0.5f, direction, 10f).Length);
                Check.Equal("SphereCastAll Ray overload without provider returns empty array", 0, Physics.SphereCastAll(ray, 0.5f, 10f).Length);
                Check.Equal("SphereCastNonAlloc without provider returns 0", 0, Physics.SphereCastNonAlloc(origin, 0.5f, direction, buffer, 10f));
                Check.Equal("SphereCastNonAlloc Ray overload without provider returns 0", 0, Physics.SphereCastNonAlloc(ray, 0.5f, buffer, 10f));
                Check.That("CapsuleCast without provider returns false", !Physics.CapsuleCast(origin, Vector3.up, 0.5f, direction, out RaycastHit capsuleHit, 10f));
                Check.That("CapsuleCast without provider leaves collider null", capsuleHit.collider == null);
                Check.That("CapsuleCast bool-only overload without provider returns false", !Physics.CapsuleCast(origin, Vector3.up, 0.5f, direction, 10f));
                Check.Equal("OverlapBox without provider returns empty array", 0, Physics.OverlapBox(origin, Vector3.one, Quaternion.identity).Length);
                Check.Equal("OverlapCapsule without provider returns empty array", 0, Physics.OverlapCapsule(origin, Vector3.up, 0.5f).Length);
                Check.That("CheckBox without provider returns false", !Physics.CheckBox(origin, Vector3.one, Quaternion.identity));
                Check.That("CheckSphere without provider returns false", !Physics.CheckSphere(origin, 1f));
                Check.That("CheckCapsule without provider returns false", !Physics.CheckCapsule(origin, Vector3.up, 0.5f));

                RaycastHit2D circleHit = Physics2D.CircleCast(Vector2.zero, 0.5f, Vector2.right, 10f);
                Check.That("Physics2D.CircleCast without provider returns empty hit", !circleHit);
                RaycastHit2D[] circleHits = Physics2D.CircleCast(Vector2.zero, 0.5f, Vector2.right, 10f, ContactFilter2D.noFilter);
                Check.Equal("Physics2D.CircleCast array overload without provider returns empty array", 0, circleHits.Length);
                RaycastHit2D capsuleHit2D = Physics2D.CapsuleCast(Vector2.zero, new Vector2(1f, 2f), CapsuleDirection2D.Vertical, 0f, Vector2.right, 10f);
                Check.That("Physics2D.CapsuleCast without provider returns empty hit", !capsuleHit2D);
                RaycastHit2D[] capsuleHits2D = Physics2D.CapsuleCast(Vector2.zero, new Vector2(1f, 2f), CapsuleDirection2D.Vertical, 0f, Vector2.right, 10f, ContactFilter2D.noFilter);
                Check.Equal("Physics2D.CapsuleCast array overload without provider returns empty array", 0, capsuleHits2D.Length);
            }
            finally
            {
                Physics.Provider = previous3D;
                Physics2D.Provider = previous2D;
            }
        }

        private static void TestProviderDelegation3D()
        {
            BreadthRecordingPhysicsProvider provider = new BreadthRecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            bool previousQueriesHitTriggers = Physics.queriesHitTriggers;
            Physics.Provider = provider;
            Physics.queriesHitTriggers = true;
            try
            {
                Vector3 origin = new Vector3(1f, 2f, 3f);
                Vector3 direction = new Vector3(0f, 1f, 0f);
                RaycastHit[] buffer = new RaycastHit[2];

                RaycastHit[] rayHits = Physics.RaycastAll(origin, direction, 15f, 5, QueryTriggerInteraction.Ignore);
                Check.Equal("RaycastAll returns all provider hits", 3, rayHits.Length);
                Check.Equal("RaycastAll forwards layerMask", 5, provider.LastLayerMask);
                Check.That("RaycastAll forwards Ignore trigger interaction", !provider.LastIncludeTriggers);

                int rayNonAllocCount = Physics.RaycastNonAlloc(new Ray(origin, direction), buffer, 15f, 7, QueryTriggerInteraction.Collide);
                Check.Equal("RaycastNonAlloc caps result count to buffer length", 2, rayNonAllocCount);
                Check.Equal("RaycastNonAlloc populates first hit distance", 1f, buffer[0].distance);
                Check.Equal("RaycastNonAlloc populates second hit distance", 2f, buffer[1].distance);
                Check.That("RaycastNonAlloc forwards Collide trigger interaction", provider.LastIncludeTriggers);

                RaycastHit[] sphereHits = Physics.SphereCastAll(origin, 0.75f, direction, 20f, 9, QueryTriggerInteraction.Ignore);
                Check.Equal("SphereCastAll returns all provider hits", 3, sphereHits.Length);
                Check.Equal("SphereCastAll forwards radius", 0.75f, provider.LastRadius);

                Array.Clear(buffer, 0, buffer.Length);
                int sphereNonAllocCount = Physics.SphereCastNonAlloc(origin, 0.75f, direction, buffer, 20f, 9, QueryTriggerInteraction.Collide);
                Check.Equal("SphereCastNonAlloc caps result count to buffer length", 2, sphereNonAllocCount);
                Check.Equal("SphereCastNonAlloc populates second hit distance", 2f, buffer[1].distance);

                Vector3 p1 = new Vector3(0f, -1f, 0f);
                Vector3 p2 = new Vector3(0f, 1f, 0f);
                bool capsuleCast = Physics.CapsuleCast(p1, p2, 0.5f, direction, out RaycastHit capsuleHit, 12f, 11, QueryTriggerInteraction.Ignore);
                Check.That("CapsuleCast reaches provider", capsuleCast);
                Check.That("CapsuleCast forwards point1", provider.LastPoint1 == p1);
                Check.That("CapsuleCast forwards point2", provider.LastPoint2 == p2);
                Check.Equal("CapsuleCast forwards radius", 0.5f, provider.LastRadius);
                Check.Equal("CapsuleCast returns hit distance", 1f, capsuleHit.distance);

                Quaternion rot = Quaternion.Euler(0f, 45f, 0f);
                Collider[] boxOverlaps = Physics.OverlapBox(origin, Vector3.one, rot, 13, QueryTriggerInteraction.Ignore);
                Check.Equal("OverlapBox returns provider colliders", 2, boxOverlaps.Length);
                Check.That("OverlapBox forwards orientation", provider.LastOrientation == rot);

                Collider[] capsuleOverlaps = Physics.OverlapCapsule(p1, p2, 0.4f, 15, QueryTriggerInteraction.Collide);
                Check.Equal("OverlapCapsule returns provider colliders", 2, capsuleOverlaps.Length);
                Check.Equal("OverlapCapsule forwards radius", 0.4f, provider.LastRadius);

                Check.That("CheckBox delegates to provider", Physics.CheckBox(origin, Vector3.one, rot, 3, QueryTriggerInteraction.Ignore));
                Check.That("CheckSphere delegates to provider", Physics.CheckSphere(origin, 1.25f, 3, QueryTriggerInteraction.Collide));
                Check.That("CheckCapsule delegates to provider", Physics.CheckCapsule(p1, p2, 0.6f, 3, QueryTriggerInteraction.Ignore));
            }
            finally
            {
                Physics.queriesHitTriggers = previousQueriesHitTriggers;
                Physics.Provider = previous;
            }
        }

        private static void TestProviderDelegation2D()
        {
            BreadthRecordingPhysics2DProvider provider = new BreadthRecordingPhysics2DProvider();
            IPhysics2DProvider? previous = Physics2D.Provider;
            Physics2D.Provider = provider;
            try
            {
                RaycastHit2D circleHit = Physics2D.CircleCast(new Vector2(1f, 2f), 0.5f, Vector2.right, 8f, 3);
                Check.That("Physics2D.CircleCast single-hit overload returns hit", (bool)circleHit);
                Check.Equal("Physics2D.CircleCast forwards radius", 0.5f, provider.LastRadius);
                Check.Equal("Physics2D.CircleCast returns nearest hit distance", 2.5f, circleHit.distance);

                RaycastHit2D capsuleHit = Physics2D.CapsuleCast(
                    new Vector2(-2f, 0f),
                    new Vector2(0.8f, 2.0f),
                    CapsuleDirection2D.Horizontal,
                    30f,
                    Vector2.up,
                    12f,
                    7);
                Check.That("Physics2D.CapsuleCast single-hit overload returns hit", (bool)capsuleHit);
                Check.That("Physics2D.CapsuleCast forwards capsule size", provider.LastSize == new Vector2(0.8f, 2.0f));
                Check.That("Physics2D.CapsuleCast forwards capsule direction", provider.LastCapsuleDirection == CapsuleDirection2D.Horizontal);
                Check.Equal("Physics2D.CapsuleCast forwards angle", 30f, provider.LastAngle);
                Check.Equal("Physics2D.CapsuleCast returns nearest hit distance", 2.5f, capsuleHit.distance);
            }
            finally
            {
                Physics2D.Provider = previous;
            }
        }

        private static void TestNativeRaycastAllAndNonAlloc()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("RaycastAll scene fixture exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new NoOpScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new NoOpScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Vector3 origin = new Vector3(-5f, 0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);

                RaycastHit[] allHits = Physics.RaycastAll(origin, direction, 20f, ~0);
                Check.Equal("BridgeHost RaycastAll hits both collinear spheres", 2, allHits.Length);
                if (allHits.Length == 2)
                {
                    Check.That("BridgeHost RaycastAll orders nearer hit first", allHits[0].distance < allHits[1].distance);
                    Check.Equal("BridgeHost RaycastAll first hit distance", 4.5f, allHits[0].distance);
                    Check.Equal("BridgeHost RaycastAll second hit distance", 6.5f, allHits[1].distance);
                    Check.That("BridgeHost RaycastAll resolves both colliders", allHits[0].collider != null && allHits[1].collider != null);
                    Check.That("BridgeHost RaycastAll resolves distinct colliders", !ReferenceEquals(allHits[0].collider, allHits[1].collider));
                }

                RaycastHit[] cappedBuffer = new RaycastHit[1];
                int cappedCount = Physics.RaycastNonAlloc(origin, direction, cappedBuffer, 20f, ~0);
                Check.Equal("BridgeHost RaycastNonAlloc respects 1-element buffer", 1, cappedCount);
                Check.Equal("BridgeHost RaycastNonAlloc writes nearest hit into buffer", 4.5f, cappedBuffer[0].distance);

                RaycastHit[] fullBuffer = new RaycastHit[4];
                int fullCount = Physics.RaycastNonAlloc(new Ray(origin, direction), fullBuffer, 20f, ~0);
                Check.Equal("BridgeHost RaycastNonAlloc returns total hits when buffer is large enough", 2, fullCount);
                Check.Equal("BridgeHost RaycastNonAlloc second hit distance", 6.5f, fullBuffer[1].distance);

                host.Shutdown();
            }
        }

        private static void TestNativeSphereCastAllAndNonAlloc()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new NoOpScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new NoOpScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Vector3 origin = new Vector3(-5f, 0.6f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);

                RaycastHit[] rayMiss = Physics.RaycastAll(origin, direction, 20f, ~0);
                Check.Equal("Offset ray misses spheres of radius 0.5 at y=0", 0, rayMiss.Length);

                RaycastHit[] sphereHits = Physics.SphereCastAll(origin, 0.25f, direction, 20f, ~0);
                Check.Equal("BridgeHost SphereCastAll hits both spheres from y=0.6 with radius 0.25", 2, sphereHits.Length);
                if (sphereHits.Length == 2)
                {
                    Check.That("BridgeHost SphereCastAll orders hits by distance", sphereHits[0].distance < sphereHits[1].distance);
                    Check.That("BridgeHost SphereCastAll resolves both colliders", sphereHits[0].collider != null && sphereHits[1].collider != null);
                }

                RaycastHit[] buffer = new RaycastHit[1];
                int count = Physics.SphereCastNonAlloc(origin, 0.25f, direction, buffer, 20f, ~0);
                Check.Equal("BridgeHost SphereCastNonAlloc caps to buffer capacity", 1, count);
                Check.That("BridgeHost SphereCastNonAlloc populates collider", buffer[0].collider != null);

                host.Shutdown();
            }
        }

        private static void TestNativeCapsuleCast3D()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new NoOpScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new NoOpScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Vector3 p1 = new Vector3(-5f, 0.75f, 0f);
                Vector3 p2 = new Vector3(-5f, 2.0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);

                bool hit = Physics.CapsuleCast(p1, p2, 0.4f, direction, out RaycastHit hitInfo, 20f, 1 << 5);
                Check.That("BridgeHost CapsuleCast hits sphere when lower hemisphere sweeps through y=0.35..0.5", hit);
                Check.That("BridgeHost CapsuleCast resolves hit collider", hitInfo.collider != null);
                Check.That("BridgeHost CapsuleCast distance is positive and finite", hitInfo.distance > 4.0f && hitInfo.distance < 5.0f);

                bool miss = Physics.CapsuleCast(
                    new Vector3(-5f, 2.0f, 0f),
                    new Vector3(-5f, 3.0f, 0f),
                    0.25f,
                    direction,
                    out RaycastHit missInfo,
                    20f,
                    ~0);
                Check.That("BridgeHost CapsuleCast misses when swept above spheres", !miss);
                Check.That("BridgeHost CapsuleCast miss leaves collider null", missInfo.collider == null);

                host.Shutdown();
            }
        }

        private static void TestNativeOverlapsAndChecks3D()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new NoOpScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new NoOpScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Collider[] boxBoth = Physics.OverlapBox(new Vector3(1f, 0f, 0f), new Vector3(1.2f, 0.25f, 0.25f), Quaternion.identity, ~0);
                Check.Equal("BridgeHost OverlapBox spanning x=[-0.2, 2.2] overlaps both spheres", 2, boxBoth.Length);

                Collider[] boxLayer5 = Physics.OverlapBox(new Vector3(1f, 0f, 0f), new Vector3(1.2f, 0.25f, 0.25f), Quaternion.identity, 1 << 5);
                Check.Equal("BridgeHost OverlapBox with layer 5 mask overlaps only QueryTarget", 1, boxLayer5.Length);

                Collider[] capsuleBoth = Physics.OverlapCapsule(new Vector3(0f, 0f, 0f), new Vector3(2f, 0f, 0f), 0.2f, ~0);
                Check.Equal("BridgeHost OverlapCapsule from x=0 to x=2 overlaps both spheres", 2, capsuleBoth.Length);

                Collider[] capsuleMiss = Physics.OverlapCapsule(new Vector3(0f, 3f, 0f), new Vector3(2f, 3f, 0f), 0.2f, ~0);
                Check.Equal("BridgeHost OverlapCapsule at y=3 overlaps nothing", 0, capsuleMiss.Length);

                Check.That("BridgeHost CheckBox returns true when overlapping sphere at origin", Physics.CheckBox(Vector3.zero, new Vector3(0.2f, 0.2f, 0.2f), Quaternion.identity, ~0));
                Check.That("BridgeHost CheckBox returns false when far from colliders", !Physics.CheckBox(new Vector3(0f, 5f, 0f), new Vector3(0.2f, 0.2f, 0.2f), Quaternion.identity, ~0));

                Check.That("BridgeHost CheckSphere returns true when overlapping sphere at (2,0,0)", Physics.CheckSphere(new Vector3(2.6f, 0f, 0f), 0.2f, ~0));
                Check.That("BridgeHost CheckSphere returns false when separated", !Physics.CheckSphere(new Vector3(2.8f, 0f, 0f), 0.2f, ~0));
                Check.That("BridgeHost CheckSphere respects layer mask", !Physics.CheckSphere(new Vector3(2.6f, 0f, 0f), 0.2f, 1 << 5));

                Check.That("BridgeHost CheckCapsule returns true when segment touches sphere", Physics.CheckCapsule(new Vector3(0f, 0.8f, 0f), new Vector3(0f, 2.0f, 0f), 0.35f, ~0));
                Check.That("BridgeHost CheckCapsule returns false when segment is clear", !Physics.CheckCapsule(new Vector3(0f, 1.0f, 0f), new Vector3(0f, 2.0f, 0f), 0.35f, ~0));

                host.Shutdown();
            }
        }

        private static void TestNativeCircleCastAndCapsuleCast2D()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_query_breadth_2d_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "Query2D.unity");
            string sceneYaml = string.Join(
                "\n",
                new string[]
                {
                    "%YAML 1.1",
                    "%TAG !u! tag:unity3d.com,2011:",
                    "--- !u!1 &100",
                    "GameObject:",
                    "  m_Name: TargetBox2D",
                    "  m_Layer: 0",
                    "  m_IsActive: 1",
                    "--- !u!4 &101",
                    "Transform:",
                    "  m_GameObject: {fileID: 100}",
                    "  m_LocalPosition: {x: 5, y: 0, z: 0}",
                    "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                    "  m_LocalScale: {x: 1, y: 1, z: 1}",
                    "--- !u!61 &102",
                    "BoxCollider2D:",
                    "  m_GameObject: {fileID: 100}",
                    "  m_IsTrigger: 0",
                    "  m_Offset: {x: 0, y: 0}",
                    "  m_Size: {x: 2, y: 2}",
                    "--- !u!1 &200",
                    "GameObject:",
                    "  m_Name: FarCircle2D",
                    "  m_Layer: 0",
                    "  m_IsActive: 1",
                    "--- !u!4 &201",
                    "Transform:",
                    "  m_GameObject: {fileID: 200}",
                    "  m_LocalPosition: {x: 10, y: 0, z: 0}",
                    "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}",
                    "  m_LocalScale: {x: 1, y: 1, z: 1}",
                    "--- !u!58 &202",
                    "CircleCollider2D:",
                    "  m_GameObject: {fileID: 200}",
                    "  m_IsTrigger: 0",
                    "  m_Offset: {x: 0, y: 0}",
                    "  m_Radius: 1",
                });
            File.WriteAllText(scenePath, sceneYaml);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);

                    RaycastHit2D circleHit = Physics2D.CircleCast(new Vector2(0f, 0f), 0.5f, Vector2.right, 20f);
                    Check.That("BridgeHost Physics2D.CircleCast hits TargetBox2D", (bool)circleHit);
                    Check.That("BridgeHost Physics2D.CircleCast resolves TargetBox2D collider", circleHit.collider != null && circleHit.collider.gameObject.name == "TargetBox2D");
                    Check.That("BridgeHost Physics2D.CircleCast stops at distance ~3.5", Math.Abs(circleHit.distance - 3.5f) < 0.1f);
                    Check.That("BridgeHost Physics2D.CircleCast normal points left", circleHit.normal.x < -0.9f);

                    RaycastHit2D[] allCircleHits = Physics2D.CircleCast(new Vector2(0f, 0f), 0.5f, Vector2.right, 20f, ContactFilter2D.noFilter);
                    Check.Equal("BridgeHost Physics2D.CircleCast array overload hits both 2D colliders", 2, allCircleHits.Length);
                    if (allCircleHits.Length == 2)
                    {
                        Check.That("BridgeHost Physics2D.CircleCast orders hits by distance", allCircleHits[0].distance < allCircleHits[1].distance);
                    }

                    RaycastHit2D verticalCapsuleHit = Physics2D.CapsuleCast(
                        new Vector2(0f, 1.75f),
                        new Vector2(1.0f, 2.0f),
                        CapsuleDirection2D.Vertical,
                        0f,
                        Vector2.right,
                        20f);
                    Check.That("BridgeHost Physics2D.CapsuleCast vertical capsule hits TargetBox2D from y=1.75", (bool)verticalCapsuleHit);
                    Check.That("BridgeHost Physics2D.CapsuleCast resolves TargetBox2D collider", verticalCapsuleHit.collider != null && verticalCapsuleHit.collider.gameObject.name == "TargetBox2D");

                    RaycastHit2D horizontalCapsuleMiss = Physics2D.CapsuleCast(
                        new Vector2(0f, 1.75f),
                        new Vector2(2.0f, 1.0f),
                        CapsuleDirection2D.Horizontal,
                        0f,
                        Vector2.right,
                        20f);
                    Check.That("BridgeHost Physics2D.CapsuleCast horizontal capsule (half-height 0.5) at y=1.75 passes above TargetBox2D (top y=1.0)", !horizontalCapsuleMiss);

                    host.Shutdown();
                }
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private sealed class NoOpScript : MonoBehaviour
        {
        }

        private sealed class BreadthRecordingPhysicsProvider : IPhysicsProvider
        {
            private readonly Collider _firstCollider;
            private readonly Collider _secondCollider;
            private readonly Collider _thirdCollider;

            public BreadthRecordingPhysicsProvider()
            {
                GameObject go1 = new GameObject("Breadth1");
                _firstCollider = go1.AddComponent<SphereCollider>();
                GameObject go2 = new GameObject("Breadth2");
                _secondCollider = go2.AddComponent<SphereCollider>();
                GameObject go3 = new GameObject("Breadth3");
                _thirdCollider = go3.AddComponent<SphereCollider>();
            }

            public Vector3 LastPoint1 { get; private set; }

            public Vector3 LastPoint2 { get; private set; }

            public Quaternion LastOrientation { get; private set; }

            public float LastRadius { get; private set; }

            public int LastLayerMask { get; private set; }

            public bool LastIncludeTriggers { get; private set; }

            public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
            {
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                hit = MakeHit(_firstCollider, 1f);
                return true;
            }

            public RaycastHit[] RaycastAll(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers)
            {
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return new RaycastHit[]
                {
                    MakeHit(_firstCollider, 1f),
                    MakeHit(_secondCollider, 2f),
                    MakeHit(_thirdCollider, 3f),
                };
            }

            public bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction)
            {
                hitInfo = MakeHit(_firstCollider, 1f);
                return true;
            }

            public bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                hit = MakeHit(_firstCollider, 1f);
                return true;
            }

            public RaycastHit[] SphereCastAll(Vector3 origin, Vector3 direction, float radius, float maxDistance, int layerMask, bool includeTriggers)
            {
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return new RaycastHit[]
                {
                    MakeHit(_firstCollider, 1f),
                    MakeHit(_secondCollider, 2f),
                    MakeHit(_thirdCollider, 3f),
                };
            }

            public bool CapsuleCast(Vector3 point1, Vector3 point2, float radius, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                LastPoint1 = point1;
                LastPoint2 = point2;
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                hit = MakeHit(_firstCollider, 1f);
                return true;
            }

            public bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                hit = MakeHit(_firstCollider, 1f);
                return true;
            }

            public Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
            {
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return new Collider[] { _firstCollider, _secondCollider };
            }

            public Collider[] OverlapBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
            {
                LastOrientation = orientation;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return new Collider[] { _firstCollider, _secondCollider };
            }

            public Collider[] OverlapCapsule(Vector3 point0, Vector3 point1, float radius, int layerMask, bool includeTriggers)
            {
                LastPoint1 = point0;
                LastPoint2 = point1;
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return new Collider[] { _firstCollider, _secondCollider };
            }

            public bool CheckBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
            {
                LastOrientation = orientation;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return true;
            }

            public bool CheckSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
            {
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return true;
            }

            public bool CheckCapsule(Vector3 start, Vector3 end, float radius, int layerMask, bool includeTriggers)
            {
                LastPoint1 = start;
                LastPoint2 = end;
                LastRadius = radius;
                LastLayerMask = layerMask;
                LastIncludeTriggers = includeTriggers;
                return true;
            }

            public bool TryGetBodyPosition(ulong entity, out Vector3 position)
            {
                position = Vector3.zero;
                return false;
            }

            public bool TryGetBodyVelocity(ulong entity, out Vector3 velocity)
            {
                velocity = Vector3.zero;
                return false;
            }

            public bool TrySetBodyPosition(ulong entity, Vector3 position) => false;

            public bool TrySetBodyVelocity(ulong entity, Vector3 velocity) => false;

            public bool TryAddForce(ulong entity, Vector3 force, ForceMode mode) => false;

            private static RaycastHit MakeHit(Collider collider, float distance)
            {
                return new RaycastHit
                {
                    collider = collider,
                    distance = distance,
                    point = new Vector3(distance, 0f, 0f),
                    normal = new Vector3(-1f, 0f, 0f),
                };
            }
        }

        private sealed class BreadthRecordingPhysics2DProvider : IPhysics2DProvider
        {
            private readonly Collider2D _collider;

            public BreadthRecordingPhysics2DProvider()
            {
                GameObject go = new GameObject("Breadth2D");
                _collider = go.AddComponent<BoxCollider2D>();
            }

            public float LastRadius { get; private set; }

            public Vector2 LastSize { get; private set; }

            public CapsuleDirection2D LastCapsuleDirection { get; private set; }

            public float LastAngle { get; private set; }

            public bool Raycast2D(Vector2 origin, Vector2 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit2D hit)
            {
                hit = default;
                return false;
            }

            public Collider2D[] OverlapPoint2D(Vector2 position, int layerMask) => Array.Empty<Collider2D>();

            public Collider2D[] OverlapCircle2D(Vector2 position, float radius, int layerMask) => Array.Empty<Collider2D>();

            public Collider2D[] OverlapBox2D(Vector2 position, Vector2 size, float angle, int layerMask) => Array.Empty<Collider2D>();

            public RaycastHit2D[] CircleCast2D(Vector2 origin, float radius, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
            {
                LastRadius = radius;
                return new RaycastHit2D[]
                {
                    new RaycastHit2D
                    {
                        collider = _collider,
                        distance = 2.5f,
                        point = new Vector2(2.5f, 0f),
                        normal = new Vector2(-1f, 0f),
                        fraction = 0.25f,
                    },
                };
            }

            public RaycastHit2D[] CapsuleCast2D(Vector2 origin, Vector2 size, CapsuleDirection2D capsuleDirection, float angle, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
            {
                LastSize = size;
                LastCapsuleDirection = capsuleDirection;
                LastAngle = angle;
                return new RaycastHit2D[]
                {
                    new RaycastHit2D
                    {
                        collider = _collider,
                        distance = 2.5f,
                        point = new Vector2(0f, 2.5f),
                        normal = new Vector2(0f, -1f),
                        fraction = 0.25f,
                    },
                };
            }
        }
    }
}
