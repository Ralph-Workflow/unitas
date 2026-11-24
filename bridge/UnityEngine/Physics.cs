using System;

namespace UnityEngine
{
    public interface IPhysicsProvider
    {
        bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit hit);

        bool Linecast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers);

        bool SphereCast(Vector3 origin, Vector3 direction, float radius, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers);

        bool BoxCast(Vector3 origin, Vector3 direction, Vector3 halfExtents, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers);

        Collider[] OverlapSphere(Vector3 position, float radius, int layerMask, bool includeTriggers);

        bool TryGetBodyPosition(ulong entity, out Vector3 position);

        bool TryGetBodyVelocity(ulong entity, out Vector3 velocity);

        bool TrySetBodyPosition(ulong entity, Vector3 position);

        bool TrySetBodyVelocity(ulong entity, Vector3 velocity);
    }

    public static class Physics
    {
        public static IPhysicsProvider? Provider { get; set; }

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

        public static bool Linecast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Linecast(origin, direction, out hit, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
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

        private static bool IncludesTriggers(QueryTriggerInteraction queryTriggerInteraction)
        {
            return queryTriggerInteraction == QueryTriggerInteraction.Collide;
        }
    }
}
