using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class QuerySurfaceTests
    {
        private const float SphereCastRadius = 0.25f;

        public static void Run()
        {
            TestQueryProviderDelegation();
            TestMissingQueryProviderReturnsFalse();
            TestQueryRecordDecode();
            TestNativeSphereCast();
            TestTriggerInteractionForwardsIncludeTriggers();
            TestQueriesHitTriggersGlobalToggle();
            TestBridgeHostLinecastSegment();
            TestBridgeHostTriggerColliderQueryFiltering();
            TestBridgeHostCapsuleColliderSceneRaycast();
        }

        private static void TestTriggerInteractionForwardsIncludeTriggers()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            bool previousQueriesHitTriggers = Physics.queriesHitTriggers;
            Physics.Provider = provider;
            Physics.queriesHitTriggers = true;
            try
            {
                Vector3 origin = new Vector3(0f, 0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);
                Vector3 end = new Vector3(10f, 0f, 0f);

                Physics.Raycast(origin, direction, out RaycastHit hit, 10f, -1);
                Check.That("Raycast without a trigger interaction uses global default to include triggers", provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("Raycast with Collide includes triggers", provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Ignore);
                Check.That("Raycast with Ignore excludes triggers", !provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Raycast with UseGlobal includes triggers when queriesHitTriggers is true", provider.IncludeTriggers);

                Physics.Linecast(origin, end, out hit, -1, QueryTriggerInteraction.Collide);
                Check.That("Linecast with Collide includes triggers", provider.IncludeTriggers);
                Check.That("Linecast with Collide forwards queryTriggerInteraction", provider.LastTriggerInteraction == QueryTriggerInteraction.Collide);

                Physics.Linecast(origin, end, out hit, -1, QueryTriggerInteraction.Ignore);
                Check.That("Linecast with Ignore excludes triggers", !provider.IncludeTriggers);
                Check.That("Linecast with Ignore forwards queryTriggerInteraction", provider.LastTriggerInteraction == QueryTriggerInteraction.Ignore);

                Physics.SphereCast(origin, direction, 0.5f, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("SphereCast with Collide includes triggers", provider.IncludeTriggers);

                Physics.BoxCast(origin, direction, new Vector3(0.5f, 0.5f, 0.5f), out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("BoxCast with Collide includes triggers", provider.IncludeTriggers);

                Physics.OverlapSphere(origin, 1f, -1, QueryTriggerInteraction.Collide);
                Check.That("OverlapSphere with Collide includes triggers", provider.IncludeTriggers);
            }
            finally
            {
                Physics.queriesHitTriggers = previousQueriesHitTriggers;
                Physics.Provider = previous;
            }
        }

        private static void TestQueriesHitTriggersGlobalToggle()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            bool previousQueriesHitTriggers = Physics.queriesHitTriggers;
            Physics.Provider = provider;
            try
            {
                Check.That("Physics.queriesHitTriggers defaults to true", Physics.queriesHitTriggers);
                Check.That("QueryTriggerInteraction.UseGlobal IncludesTriggers is true by default", QueryTriggerInteraction.UseGlobal.IncludesTriggers());
                Check.That("QueryTriggerInteraction.Collide IncludesTriggers is true", QueryTriggerInteraction.Collide.IncludesTriggers());
                Check.That("QueryTriggerInteraction.Ignore IncludesTriggers is false", !QueryTriggerInteraction.Ignore.IncludesTriggers());

                Vector3 origin = new Vector3(0f, 0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);
                Vector3 end = new Vector3(10f, 0f, 0f);

                Physics.queriesHitTriggers = false;
                Check.That("QueryTriggerInteraction.UseGlobal IncludesTriggers is false when queriesHitTriggers is false", !QueryTriggerInteraction.UseGlobal.IncludesTriggers());

                Physics.Raycast(origin, direction, out RaycastHit hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Raycast with UseGlobal excludes triggers when queriesHitTriggers is false", !provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("Raycast with Collide still includes triggers when queriesHitTriggers is false", provider.IncludeTriggers);

                Physics.Linecast(origin, end, out hit, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Linecast with UseGlobal excludes triggers when queriesHitTriggers is false", !provider.IncludeTriggers);

                Physics.SphereCast(origin, direction, 0.5f, out hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("SphereCast with UseGlobal excludes triggers when queriesHitTriggers is false", !provider.IncludeTriggers);

                Physics.BoxCast(origin, direction, Vector3.one, out hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("BoxCast with UseGlobal excludes triggers when queriesHitTriggers is false", !provider.IncludeTriggers);

                Physics.OverlapSphere(origin, 1f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("OverlapSphere with UseGlobal excludes triggers when queriesHitTriggers is false", !provider.IncludeTriggers);

                Physics.queriesHitTriggers = true;
                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Raycast with UseGlobal includes triggers after restoring queriesHitTriggers to true", provider.IncludeTriggers);

                Physics.Linecast(origin, end, out hit, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Linecast with UseGlobal includes triggers after restoring queriesHitTriggers to true", provider.IncludeTriggers);
            }
            finally
            {
                Physics.queriesHitTriggers = previousQueriesHitTriggers;
                Physics.Provider = previous;
            }
        }

        private static void TestQueryProviderDelegation()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = provider;
            try
            {
                Vector3 origin = new Vector3(1f, 2f, 3f);
                Vector3 direction = new Vector3(0f, 1f, 0f);
                Vector3 end = new Vector3(1f, 10.5f, 3f);
                Vector3 halfExtents = new Vector3(0.5f, 0.25f, 0.75f);

                bool raycast = Physics.Raycast(origin, direction, out RaycastHit raycastHit, 12.5f, 3);
                Check.That("Raycast reaches the provider", raycast);
                Check.Equal("Raycast calls the provider once", 1, provider.RaycastCalls);
                Check.That("Raycast forwards the origin", provider.Origin == origin);
                Check.That("Raycast forwards the direction", provider.Direction == direction);
                Check.Equal("Raycast forwards the max distance", 12.5f, provider.MaxDistance);
                Check.Equal("Raycast forwards the layer mask", 3, provider.LayerMask);
                Check.Equal("Raycast returns the provider hit distance", 4.5f, raycastHit.distance);
                Check.Equal("Raycast returns the provider hit point x", 1f, raycastHit.point.x);
                Check.Equal("Raycast returns the provider hit normal y", 1f, raycastHit.normal.y);
                Check.That("Raycast returns the provider collider", raycastHit.collider != null);

                bool linecast = Physics.Linecast(origin, end, out RaycastHit linecastHit, 5);
                Check.That("Linecast reaches the provider", linecast);
                Check.Equal("Linecast calls the provider once", 1, provider.LinecastCalls);
                Check.That("Linecast forwards the start", provider.Origin == origin);
                Check.That("Linecast forwards the end", provider.End == end);
                Check.That("Linecast derives the direction", provider.Direction == direction);
                Check.Equal("Linecast derives the max distance", 8.5f, provider.MaxDistance);
                Check.Equal("Linecast forwards the layer mask", 5, provider.LayerMask);
                Check.Equal("Linecast returns the provider hit distance", 4.5f, linecastHit.distance);
                Check.Equal("Linecast returns the provider hit normal z", 0f, linecastHit.normal.z);

                bool linecastNoHitOut = Physics.Linecast(origin, end, 5, QueryTriggerInteraction.Ignore);
                Check.That("Linecast overload without out RaycastHit reaches the provider", linecastNoHitOut);
                Check.Equal("Linecast overload increments call count", 2, provider.LinecastCalls);
                Check.That("Linecast overload forwards start", provider.Origin == origin);
                Check.That("Linecast overload forwards end", provider.End == end);
                Check.Equal("Linecast overload forwards layer mask", 5, provider.LayerMask);
                Check.That("Linecast overload forwards trigger interaction", !provider.IncludeTriggers);

                bool sphereCast = Physics.SphereCast(origin, direction, 2.5f, out RaycastHit sphereCastHit, 6.5f, 7);
                Check.That("SphereCast reaches the provider", sphereCast);
                Check.Equal("SphereCast calls the provider once", 1, provider.SphereCastCalls);
                Check.That("SphereCast forwards the origin", provider.Origin == origin);
                Check.That("SphereCast forwards the direction", provider.Direction == direction);
                Check.Equal("SphereCast forwards the radius", 2.5f, provider.Radius);
                Check.Equal("SphereCast forwards the max distance", 6.5f, provider.MaxDistance);
                Check.Equal("SphereCast forwards the layer mask", 7, provider.LayerMask);
                Check.Equal("SphereCast returns the provider hit distance", 4.5f, sphereCastHit.distance);

                bool boxCast = Physics.BoxCast(origin, direction, halfExtents, out RaycastHit boxCastHit, 4.5f, 9);
                Check.That("BoxCast reaches the provider", boxCast);
                Check.Equal("BoxCast calls the provider once", 1, provider.BoxCastCalls);
                Check.That("BoxCast forwards the origin", provider.Origin == origin);
                Check.That("BoxCast forwards the direction", provider.Direction == direction);
                Check.That("BoxCast forwards the half extents", provider.HalfExtents == halfExtents);
                Check.Equal("BoxCast forwards the max distance", 4.5f, provider.MaxDistance);
                Check.Equal("BoxCast forwards the layer mask", 9, provider.LayerMask);
                Check.Equal("BoxCast returns the provider hit distance", 4.5f, boxCastHit.distance);
                Check.Equal("BoxCast returns the provider hit point z", 3f, boxCastHit.point.z);
            }
            finally
            {
                Physics.Provider = previous;
            }
        }

        private static void TestMissingQueryProviderReturnsFalse()
        {
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = null;
            try
            {
                Vector3 origin = new Vector3(1f, 2f, 3f);
                Vector3 direction = new Vector3(0f, 1f, 0f);
                Vector3 end = new Vector3(1f, 12f, 3f);

                Check.That("Raycast without a provider misses", !Physics.Raycast(origin, direction, out RaycastHit raycastHit, 10f, 1));
                Check.That("Raycast without a provider leaves the collider empty", raycastHit.collider == null);
                Check.Equal("Raycast without a provider leaves the distance empty", 0f, raycastHit.distance);
                Check.That("Raycast without a provider leaves the normal empty", raycastHit.normal == Vector3.zero);

                Check.That("Linecast without a provider misses", !Physics.Linecast(origin, end, out RaycastHit linecastHit, 1));
                Check.That("Linecast without a provider leaves the collider empty", linecastHit.collider == null);
                Check.Equal("Linecast without a provider leaves the distance empty", 0f, linecastHit.distance);
                Check.That("Linecast overload without out RaycastHit misses when provider is missing", !Physics.Linecast(origin, end, 1));

                Check.That("SphereCast without a provider misses", !Physics.SphereCast(origin, direction, 1f, out RaycastHit sphereCastHit, 10f, 1));
                Check.That("SphereCast without a provider leaves the collider empty", sphereCastHit.collider == null);
                Check.Equal("SphereCast without a provider leaves the distance empty", 0f, sphereCastHit.distance);

                Check.That("BoxCast without a provider misses", !Physics.BoxCast(origin, direction, Vector3.one, out RaycastHit boxCastHit, 10f, 1));
                Check.That("BoxCast without a provider leaves the collider empty", boxCastHit.collider == null);
                Check.Equal("BoxCast without a provider leaves the distance empty", 0f, boxCastHit.distance);
            }
            finally
            {
                Physics.Provider = previous;
            }
        }

        private static void TestBridgeHostLinecastSegment()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Linecast scene file exists", File.Exists(scenePath));

            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => new LinecastTargetScript());
            ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new LinecastTargetScript());

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                bool hitSegment = Physics.Linecast(
                    new Vector3(0f, -5f, 0f),
                    new Vector3(0f, 5f, 0f),
                    out RaycastHit hit,
                    1 << 5);
                Check.That("BridgeHost Linecast hits collider along segment", hitSegment);
                Check.That("BridgeHost Linecast resolves hit collider", hit.collider != null);
                Check.Equal("BridgeHost Linecast reports hit distance", 4.5f, hit.distance);
                Check.Equal("BridgeHost Linecast reports hit point y", -0.5f, hit.point.y);

                bool shortSegment = Physics.Linecast(
                    new Vector3(0f, -5f, 0f),
                    new Vector3(0f, -1f, 0f),
                    out RaycastHit shortHit,
                    1 << 5);
                Check.That("BridgeHost Linecast misses collider beyond segment end", !shortSegment);
                Check.That("BridgeHost Linecast short miss leaves collider null", shortHit.collider == null);

                bool degenerateSegment = Physics.Linecast(
                    new Vector3(0f, -5f, 0f),
                    new Vector3(0f, -5f, 0f),
                    out RaycastHit degenerateHit,
                    1 << 5);
                Check.That("BridgeHost Linecast misses when start equals end", !degenerateSegment);
                Check.That("BridgeHost Linecast degenerate miss leaves collider null", degenerateHit.collider == null);

                host.Shutdown();
            }
        }

        private static void TestBridgeHostTriggerColliderQueryFiltering()
        {
            string baseScenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query.unity");
            Check.That("Trigger query base scene file exists", File.Exists(baseScenePath));

            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_trigger_query_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string tempScenePath = Path.Combine(tempDir, "trigger_query.unity");
            string yaml = File.ReadAllText(baseScenePath)
                .Replace(
                    "  m_Radius: 0.5\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!114 &1311639333",
                    "  m_IsTrigger: 1\n  m_Radius: 0.5\n  m_Center: {x: 0, y: 0, z: 0}\n--- !u!114 &1311639333",
                    StringComparison.Ordinal);
            File.WriteAllText(tempScenePath, yaml);

            bool savedQueriesHitTriggers = Physics.queriesHitTriggers;
            try
            {
                ScriptGlueRegistry.Clear();
                TriggerTargetScript? triggerScript = null;
                ScriptGlueRegistry.Register(0x1111222233334444UL, 0x5555666677778888UL, () => triggerScript = new TriggerTargetScript());
                ScriptGlueRegistry.Register(0x2222222233334444UL, 0x5555666677778888UL, () => new LinecastTargetScript());

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScenePath);
                    host.Tick(0.02);

                    Check.That("Trigger target script initialized", triggerScript != null);
                    Collider? expectedCollider = triggerScript?.GetComponent<Collider>();
                    Check.That("Trigger target has attached Collider", expectedCollider != null && expectedCollider.isTrigger);

                    Vector3 origin = new Vector3(0f, -5f, 0f);
                    Vector3 direction = new Vector3(0f, 1f, 0f);
                    int layerMask = 1 << 5;

                    Physics.queriesHitTriggers = true;
                    bool hitDefaultGlobal = Physics.Raycast(origin, direction, out RaycastHit globalHit, 100f, layerMask);
                    Check.That("BridgeHost Raycast with default UseGlobal hits trigger when queriesHitTriggers is true", hitDefaultGlobal);
                    Check.That("BridgeHost Raycast with UseGlobal resolves trigger collider", ReferenceEquals(globalHit.collider, expectedCollider));
                    Check.Equal("BridgeHost Raycast trigger hit distance", 4.5f, globalHit.distance);

                    bool hitCollide = Physics.Raycast(origin, direction, out RaycastHit collideHit, 100f, layerMask, QueryTriggerInteraction.Collide);
                    Check.That("BridgeHost Raycast with Collide hits trigger collider", hitCollide);
                    Check.That("BridgeHost Raycast with Collide resolves trigger collider", ReferenceEquals(collideHit.collider, expectedCollider));

                    bool hitIgnore = Physics.Raycast(origin, direction, out RaycastHit ignoreHit, 100f, layerMask, QueryTriggerInteraction.Ignore);
                    Check.That("BridgeHost Raycast with Ignore excludes trigger collider", !hitIgnore);
                    Check.That("BridgeHost Raycast with Ignore leaves collider null", ignoreHit.collider == null);

                    Physics.queriesHitTriggers = false;
                    bool hitGlobalDisabled = Physics.Raycast(origin, direction, out RaycastHit globalDisabledHit, 100f, layerMask, QueryTriggerInteraction.UseGlobal);
                    Check.That("BridgeHost Raycast with UseGlobal excludes trigger when queriesHitTriggers is false", !hitGlobalDisabled);
                    Check.That("BridgeHost Raycast with UseGlobal and queriesHitTriggers false leaves collider null", globalDisabledHit.collider == null);

                    bool hitCollideWhenGlobalDisabled = Physics.Raycast(origin, direction, out RaycastHit collideOverrideHit, 100f, layerMask, QueryTriggerInteraction.Collide);
                    Check.That("BridgeHost Raycast with Collide hits trigger even when queriesHitTriggers is false", hitCollideWhenGlobalDisabled);
                    Check.That("BridgeHost Raycast with Collide override resolves trigger collider", ReferenceEquals(collideOverrideHit.collider, expectedCollider));

                    Collider[] overlapCollide = Physics.OverlapSphere(Vector3.zero, 1f, layerMask, QueryTriggerInteraction.Collide);
                    Check.Equal("BridgeHost OverlapSphere with Collide includes trigger collider", 1, overlapCollide.Length);
                    Check.That("BridgeHost OverlapSphere with Collide resolves trigger collider", overlapCollide.Length == 1 && ReferenceEquals(overlapCollide[0], expectedCollider));

                    Collider[] overlapIgnore = Physics.OverlapSphere(Vector3.zero, 1f, layerMask, QueryTriggerInteraction.Ignore);
                    Check.Equal("BridgeHost OverlapSphere with Ignore excludes trigger collider", 0, overlapIgnore.Length);

                    host.Shutdown();
                }
            }
            finally
            {
                Physics.queriesHitTriggers = savedQueriesHitTriggers;
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static void TestBridgeHostCapsuleColliderSceneRaycast()
        {
            string capsuleFixturePath = ResolveFixture("crates/unitas-asset/tests/fixtures/physics_capsule.unity");
            Check.That("Capsule physics fixture exists", File.Exists(capsuleFixturePath));

            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_capsule_query_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string tempScenePath = Path.Combine(tempDir, "capsule_query.unity");
            string yaml = File.ReadAllText(capsuleFixturePath) + CapsuleScriptDocuments;
            File.WriteAllText(tempScenePath, yaml);

            try
            {
                ScriptGlueRegistry.Clear();
                CapsuleTargetScript? pillarScript = null;
                CapsuleTargetScript? defaultCapsuleScript = null;
                ScriptGlueRegistry.Register(
                    0x1111222233334444UL,
                    0x5555666677778888UL,
                    () => pillarScript = new CapsuleTargetScript(0.25f, 2f, 2, new Vector3(0.1f, 0.2f, -0.3f), true));
                ScriptGlueRegistry.Register(
                    0x2222222233334444UL,
                    0x5555666677778888UL,
                    () => defaultCapsuleScript = new CapsuleTargetScript(0.5f, 2f, 1, Vector3.zero, false));

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScenePath);
                    host.Tick(0.02);

                    Check.That("DefaultCapsule script initialized", defaultCapsuleScript != null);
                    Check.That("Pillar script initialized", pillarScript != null);
                    CapsuleCollider? defaultCapsule = defaultCapsuleScript?.Capsule;
                    CapsuleCollider? pillarCapsule = pillarScript?.Capsule;
                    Check.That("DefaultCapsule has CapsuleCollider component", defaultCapsule != null);
                    Check.That("Pillar has CapsuleCollider component", pillarCapsule != null);

                    bool hitSolidCapsule = Physics.Raycast(
                        new Vector3(-5f, 0f, 0f),
                        new Vector3(1f, 0f, 0f),
                        out RaycastHit solidHit,
                        100f,
                        ~0,
                        QueryTriggerInteraction.Ignore);
                    Check.That("BridgeHost Raycast hits scene CapsuleCollider", hitSolidCapsule);
                    Check.That("BridgeHost Raycast resolves CapsuleCollider instance", ReferenceEquals(solidHit.collider, defaultCapsule));
                    Check.Equal("BridgeHost Raycast CapsuleCollider distance", 4.5f, solidHit.distance);
                    Check.Equal("BridgeHost Raycast CapsuleCollider hit point x", -0.5f, solidHit.point.x);
                    Check.Equal("BridgeHost Raycast CapsuleCollider hit point y", 0f, solidHit.point.y);
                    Check.Equal("BridgeHost Raycast CapsuleCollider hit point z", 0f, solidHit.point.z);

                    bool hitCapEnd = Physics.Raycast(
                        new Vector3(0f, 5f, 0f),
                        new Vector3(0f, -1f, 0f),
                        out RaycastHit capHit,
                        100f,
                        ~0,
                        QueryTriggerInteraction.Ignore);
                    Check.That("BridgeHost Raycast hits scene CapsuleCollider top hemisphere", hitCapEnd);
                    Check.That("BridgeHost Raycast top hemisphere resolves CapsuleCollider", ReferenceEquals(capHit.collider, defaultCapsule));
                    Check.Equal("BridgeHost Raycast CapsuleCollider top cap distance", 4f, capHit.distance);
                    Check.Equal("BridgeHost Raycast CapsuleCollider top cap point y", 1f, capHit.point.y);

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

        private const string CapsuleScriptDocuments = @"
--- !u!114 &1004
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 1000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
--- !u!114 &2003
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 2000}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 22222222333344445555666677778888, type: 3}
";

        private static void TestQueryRecordDecode()
        {
            Check.Equal("Query record size is 64 bytes", 64, RuntimeInterop.QueryRecordSize);

            byte[] bytes = new byte[RuntimeInterop.QueryRecordSize];
            WriteUInt64(bytes, 0, 0x1122334455667788UL);
            WriteDouble(bytes, 8, 12.5);
            WriteDouble(bytes, 16, 1.5);
            WriteDouble(bytes, 24, -2.5);
            WriteDouble(bytes, 32, 3.5);
            WriteDouble(bytes, 40, 0.25);
            WriteDouble(bytes, 48, -0.5);
            WriteDouble(bytes, 56, 0.75);

            IntPtr buffer = Marshal.AllocHGlobal(bytes.Length);
            try
            {
                Marshal.Copy(bytes, 0, buffer, bytes.Length);
                QueryHit[] hits = RuntimeInterop.ReadQueryHits(buffer, 1);
                Check.Equal("Query hit count", 1, hits.Length);
                Check.That("Query hit entity", hits[0].Entity == 0x1122334455667788UL);
                Check.Equal("Query hit distance", 12.5, hits[0].Distance);
                Check.Equal("Query hit point x", 1.5, hits[0].X);
                Check.Equal("Query hit point y", -2.5, hits[0].Y);
                Check.Equal("Query hit point z", 3.5, hits[0].Z);
                Check.Equal("Query hit normal x", 0.25, hits[0].NX);
                Check.Equal("Query hit normal y", -0.5, hits[0].NY);
                Check.Equal("Query hit normal z", 0.75, hits[0].NZ);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        private static void TestNativeSphereCast()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_physics_query_abi.unity");
            Check.That("Sphere cast scene file exists", File.Exists(scenePath));

            ulong runtime = RuntimeInterop.unitas_runtime_new();
            Check.That("Native runtime handle is created", runtime != 0);
            if (runtime == 0)
            {
                return;
            }

            try
            {
                byte[] pathBytes = Encoding.UTF8.GetBytes(scenePath);
                IntPtr pathBuffer = RuntimeInterop.unitas_runtime_path_buffer(runtime, (ulong)pathBytes.Length);
                Check.That("Native path buffer is allocated", pathBuffer != IntPtr.Zero);
                Marshal.Copy(pathBytes, 0, pathBuffer, pathBytes.Length);
                int loadResult = RuntimeInterop.unitas_runtime_load_scene(runtime, pathBuffer, (ulong)pathBytes.Length);
                Check.Equal("Native scene load succeeds", 0, loadResult);

                int count = RuntimeInterop.unitas_runtime_sphere_cast(
                    runtime,
                    -5.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    0.0,
                    SphereCastRadius,
                    100.0,
                    uint.MaxValue,
                    0);
                Check.That("Native sphere cast reports a hit", count >= 1);
                if (count < 1)
                {
                    return;
                }

                IntPtr buffer = RuntimeInterop.unitas_runtime_query_buffer(
                    runtime,
                    (uint)(count * RuntimeInterop.QueryRecordSize));
                Check.That("Native query buffer is allocated", buffer != IntPtr.Zero);
                QueryHit[] hits = RuntimeInterop.ReadQueryHits(buffer, count);
                Check.Equal("Native sphere cast hit count", count, hits.Length);

                for (int index = 1; index < hits.Length; index++)
                {
                    Check.That(
                        "Native sphere cast hits are ordered by distance",
                        hits[index - 1].Distance <= hits[index].Distance);
                }

                QueryHit nearest = hits[0];
                Check.That("Native sphere cast stops at the first collider", nearest.Distance > 4.0 && nearest.Distance < 4.6);
                Check.That("Native sphere cast hit point is on the collider", nearest.X < -0.2 && nearest.X > -0.6);
                Check.Equal("Native sphere cast hit point y", 0.0, nearest.Y);
                Check.Equal("Native sphere cast hit point z", 0.0, nearest.Z);

                double normalLength = Math.Sqrt((nearest.NX * nearest.NX) + (nearest.NY * nearest.NY) + (nearest.NZ * nearest.NZ));
                Check.That("Native sphere cast hit normal is a unit vector", Math.Abs(normalLength - 1.0) < 0.01);
                Check.That("Native sphere cast hit normal faces the cast axis", Math.Abs(nearest.NX) > 0.9);
                Check.Equal("Native sphere cast hit normal y", 0.0, nearest.NY);
                Check.Equal("Native sphere cast hit normal z", 0.0, nearest.NZ);
            }
            catch (EntryPointNotFoundException exception)
            {
                Console.WriteLine("Native sphere cast is unavailable: " + exception.Message);
                Check.That("Native sphere cast returns an ordered hit with a normal", false);
            }
            finally
            {
                int destroyResult = RuntimeInterop.unitas_runtime_destroy(runtime);
                Check.Equal("Native runtime destroy succeeds", 0, destroyResult);
            }
        }

        private static void WriteUInt64(byte[] bytes, int offset, ulong value)
        {
            byte[] encoded = BitConverter.GetBytes(value);
            Array.Copy(encoded, 0, bytes, offset, encoded.Length);
        }

        private static void WriteDouble(byte[] bytes, int offset, double value)
        {
            byte[] encoded = BitConverter.GetBytes(value);
            Array.Copy(encoded, 0, bytes, offset, encoded.Length);
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private sealed class LinecastTargetScript : MonoBehaviour
        {
            public override void Awake()
            {
                SphereCollider collider = gameObject.AddComponent<SphereCollider>();
                collider.radius = 0.5f;
            }
        }

        private sealed class TriggerTargetScript : MonoBehaviour
        {
            public override void Awake()
            {
                Collider? existing = gameObject.GetComponent<Collider>();
                if (existing != null)
                {
                    existing.isTrigger = true;
                }
            }
        }

        private sealed class CapsuleTargetScript : MonoBehaviour
        {
            private readonly float _radius;
            private readonly float _height;
            private readonly int _direction;
            private readonly Vector3 _center;
            private readonly bool _isTrigger;

            public CapsuleTargetScript(float radius, float height, int direction, Vector3 center, bool isTrigger)
            {
                _radius = radius;
                _height = height;
                _direction = direction;
                _center = center;
                _isTrigger = isTrigger;
            }

            public CapsuleCollider? Capsule { get; private set; }

            public override void Awake()
            {
                SphereCollider? placeholder = gameObject.GetComponent<SphereCollider>();
                if (placeholder != null)
                {
                    UnityEngine.Object.Destroy(placeholder);
                }

                CapsuleCollider capsule = gameObject.GetComponent<CapsuleCollider>() ?? gameObject.AddComponent<CapsuleCollider>();
                capsule.radius = _radius;
                capsule.height = _height;
                capsule.direction = _direction;
                capsule.center = _center;
                capsule.isTrigger = _isTrigger;
                Capsule = capsule;
            }
        }

        private sealed class RecordingPhysicsProvider : IPhysicsProvider
        {
            public int RaycastCalls { get; private set; }

            public int LinecastCalls { get; private set; }

            public int SphereCastCalls { get; private set; }

            public int BoxCastCalls { get; private set; }

            public Vector3 Origin { get; private set; }

            public Vector3 End { get; private set; }

            public Vector3 Direction { get; private set; }

            public float Radius { get; private set; }

            public Vector3 HalfExtents { get; private set; }

            public float MaxDistance { get; private set; }

            public int LayerMask { get; private set; }

            public bool IncludeTriggers { get; private set; }

            public QueryTriggerInteraction LastTriggerInteraction { get; private set; }

            public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
            {
                RaycastCalls++;
                Record(origin, direction, Vector3.zero, maxDistance, layerMask, includeTriggers);
                hit = FixedHit();
                return true;
            }

            public bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction)
            {
                LinecastCalls++;
                End = end;
                LastTriggerInteraction = queryTriggerInteraction;
                Vector3 delta = end - start;
                float maxDistance = delta.magnitude;
                Vector3 direction = maxDistance > 0f ? delta / maxDistance : Vector3.zero;
                Record(start, direction, Vector3.zero, maxDistance, layerMask, queryTriggerInteraction.IncludesTriggers());
                hitInfo = FixedHit();
                return true;
            }

            public bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                SphereCastCalls++;
                Record(origin, direction, Vector3.zero, maxDistance, layerMask, includeTriggers);
                Radius = radius;
                hit = FixedHit();
                return true;
            }

            public bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                BoxCastCalls++;
                Record(origin, direction, halfExtents, maxDistance, layerMask, includeTriggers);
                hit = FixedHit();
                return true;
            }

            public Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
            {
                IncludeTriggers = includeTriggers;
                return Array.Empty<Collider>();
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

            public bool TrySetBodyPosition(ulong entity, Vector3 position)
            {
                return false;
            }

            public bool TrySetBodyVelocity(ulong entity, Vector3 velocity)
            {
                return false;
            }

            private void Record(Vector3 origin, Vector3 direction, Vector3 halfExtents, float maxDistance, int layerMask, bool includeTriggers)
            {
                Origin = origin;
                Direction = direction;
                HalfExtents = halfExtents;
                MaxDistance = maxDistance;
                LayerMask = layerMask;
                IncludeTriggers = includeTriggers;
            }

            private static RaycastHit FixedHit()
            {
                return new RaycastHit
                {
                    collider = new SphereCollider(),
                    point = new Vector3(1f, 2f, 3f),
                    normal = new Vector3(0f, 1f, 0f),
                    distance = 4.5f,
                };
            }
        }
    }
}
