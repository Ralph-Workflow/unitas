using System;

namespace UnityEngine
{
    public interface IPhysicsProvider
    {
        bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit);

        bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask, QueryTriggerInteraction queryTriggerInteraction);

        bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers);

        bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers);

        Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers);

        bool TryGetBodyPosition(ulong entity, out Vector3 position);

        bool TryGetBodyVelocity(ulong entity, out Vector3 velocity);

        bool TrySetBodyPosition(ulong entity, Vector3 position);

        bool TrySetBodyVelocity(ulong entity, Vector3 velocity);

        bool TrySetBodyRotation(ulong entity, Quaternion rotation)
        {
            return false;
        }

        bool TryAddForce(ulong entity, Vector3 force, ForceMode mode);

        RaycastHit[] RaycastAll(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers)
        {
            return Array.Empty<RaycastHit>();
        }

        int RaycastNonAlloc(RaycastHit[] results, Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers)
        {
            return CopyHits(RaycastAll(origin, direction, maxDistance, layerMask, includeTriggers), results);
        }

        RaycastHit[] SphereCastAll(Vector3 origin, Vector3 direction, float radius, float maxDistance, int layerMask, bool includeTriggers)
        {
            return Array.Empty<RaycastHit>();
        }

        int SphereCastNonAlloc(RaycastHit[] results, Vector3 origin, Vector3 direction, float radius, float maxDistance, int layerMask, bool includeTriggers)
        {
            return CopyHits(SphereCastAll(origin, direction, radius, maxDistance, layerMask, includeTriggers), results);
        }

        bool CapsuleCast(Vector3 point1, Vector3 point2, float radius, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            return false;
        }

        Collider[] OverlapBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
        {
            return Array.Empty<Collider>();
        }

        Collider[] OverlapCapsule(Vector3 point0, Vector3 point1, float radius, int layerMask, bool includeTriggers)
        {
            return Array.Empty<Collider>();
        }

        bool CheckBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
        {
            return false;
        }

        bool CheckSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
        {
            return false;
        }

        bool CheckCapsule(Vector3 point1, Vector3 point2, float radius, int layerMask, bool includeTriggers)
        {
            return false;
        }

        bool TrySetBodyCollisionDetectionMode(ulong entity, CollisionDetectionMode mode)
        {
            return false;
        }

        bool TryGetCollisionSummary(ulong a, ulong b, out CollisionSummary summary)
        {
            summary = default;
            return false;
        }

        bool TryGetCollisionContact(ulong a, ulong b, uint index, out ContactPoint contact)
        {
            contact = default;
            return false;
        }

        bool TryMoveBodyPosition(ulong entity, Vector3 position)
        {
            return false;
        }

        private static int CopyHits(RaycastHit[] hits, RaycastHit[] results)
        {
            if (results == null || results.Length == 0)
            {
                return 0;
            }

            int count = Math.Min(hits.Length, results.Length);
            Array.Copy(hits, results, count);
            return count;
        }
    }

    public static partial class Physics
    {
        public const int DefaultRaycastLayers = -1;

        public static IPhysicsProvider? Provider { get; set; }

        public static bool queriesHitTriggers { get; set; } = true;

        public static bool Raycast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Raycast(origin, direction, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction), out hit);
        }

        public static bool Linecast(Vector3 start, Vector3 end, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return Linecast(start, end, out _, layerMask, queryTriggerInteraction);
        }

        public static bool Linecast(Vector3 start, Vector3 end, out RaycastHit hitInfo, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hitInfo = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Linecast(start, end, out hitInfo, layerMask, queryTriggerInteraction);
        }

        public static bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.SphereCast(origin, direction, radius, out hit, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.BoxCast(origin, direction, halfExtents, out hit, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider>();
            }
            return provider.OverlapSphere(position, radius, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        internal static bool IncludesTriggers(QueryTriggerInteraction queryTriggerInteraction)
        {
            return queryTriggerInteraction.IncludesTriggers();
        }
    }
}
