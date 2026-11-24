using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
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
        }

        private static void TestTriggerInteractionForwardsIncludeTriggers()
        {
            RecordingPhysicsProvider provider = new RecordingPhysicsProvider();
            IPhysicsProvider? previous = Physics.Provider;
            Physics.Provider = provider;
            try
            {
                Vector3 origin = new Vector3(0f, 0f, 0f);
                Vector3 direction = new Vector3(1f, 0f, 0f);

                Physics.Raycast(origin, direction, out RaycastHit hit, 10f, -1);
                Check.That("Raycast without a trigger interaction excludes triggers", !provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("Raycast with Collide includes triggers", provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Ignore);
                Check.That("Raycast with Ignore excludes triggers", !provider.IncludeTriggers);

                Physics.Raycast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.UseGlobal);
                Check.That("Raycast with UseGlobal excludes triggers", !provider.IncludeTriggers);

                Physics.Linecast(origin, direction, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("Linecast with Collide includes triggers", provider.IncludeTriggers);

                Physics.SphereCast(origin, direction, 0.5f, out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("SphereCast with Collide includes triggers", provider.IncludeTriggers);

                Physics.BoxCast(origin, direction, new Vector3(0.5f, 0.5f, 0.5f), out hit, 10f, -1, QueryTriggerInteraction.Collide);
                Check.That("BoxCast with Collide includes triggers", provider.IncludeTriggers);

                Physics.OverlapSphere(origin, 1f, -1, QueryTriggerInteraction.Collide);
                Check.That("OverlapSphere with Collide includes triggers", provider.IncludeTriggers);
            }
            finally
            {
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

                bool linecast = Physics.Linecast(origin, direction, out RaycastHit linecastHit, 8.5f, 5);
                Check.That("Linecast reaches the provider", linecast);
                Check.Equal("Linecast calls the provider once", 1, provider.LinecastCalls);
                Check.That("Linecast forwards the origin", provider.Origin == origin);
                Check.That("Linecast forwards the direction", provider.Direction == direction);
                Check.Equal("Linecast forwards the max distance", 8.5f, provider.MaxDistance);
                Check.Equal("Linecast forwards the layer mask", 5, provider.LayerMask);
                Check.Equal("Linecast returns the provider hit distance", 4.5f, linecastHit.distance);
                Check.Equal("Linecast returns the provider hit normal z", 0f, linecastHit.normal.z);

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

                Check.That("Raycast without a provider misses", !Physics.Raycast(origin, direction, out RaycastHit raycastHit, 10f, 1));
                Check.That("Raycast without a provider leaves the collider empty", raycastHit.collider == null);
                Check.Equal("Raycast without a provider leaves the distance empty", 0f, raycastHit.distance);
                Check.That("Raycast without a provider leaves the normal empty", raycastHit.normal == Vector3.zero);

                Check.That("Linecast without a provider misses", !Physics.Linecast(origin, direction, out RaycastHit linecastHit, 10f, 1));
                Check.That("Linecast without a provider leaves the collider empty", linecastHit.collider == null);
                Check.Equal("Linecast without a provider leaves the distance empty", 0f, linecastHit.distance);

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

        private sealed class RecordingPhysicsProvider : IPhysicsProvider
        {
            public int RaycastCalls { get; private set; }

            public int LinecastCalls { get; private set; }

            public int SphereCastCalls { get; private set; }

            public int BoxCastCalls { get; private set; }

            public Vector3 Origin { get; private set; }

            public Vector3 Direction { get; private set; }

            public float Radius { get; private set; }

            public Vector3 HalfExtents { get; private set; }

            public float MaxDistance { get; private set; }

            public int LayerMask { get; private set; }

            public bool IncludeTriggers { get; private set; }

            public bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit)
            {
                RaycastCalls++;
                Record(origin, direction, Vector3.zero, maxDistance, layerMask, includeTriggers);
                hit = FixedHit();
                return true;
            }

            public bool Linecast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
            {
                LinecastCalls++;
                Record(origin, direction, Vector3.zero, maxDistance, layerMask, includeTriggers);
                hit = FixedHit();
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
