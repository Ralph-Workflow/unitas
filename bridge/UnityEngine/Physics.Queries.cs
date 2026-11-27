using System;

namespace UnityEngine
{
    public static partial class Physics
    {
        public static RaycastHit[] RaycastAll(Vector3 origin, Vector3 direction, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<RaycastHit>();
            }
            return provider.RaycastAll(origin, direction, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static RaycastHit[] RaycastAll(Ray ray, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return RaycastAll(ray.origin, ray.direction, maxDistance, layerMask, queryTriggerInteraction);
        }

        public static int RaycastNonAlloc(Vector3 origin, Vector3 direction, RaycastHit[] results, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return 0;
            }
            return provider.RaycastNonAlloc(results, origin, direction, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static int RaycastNonAlloc(Ray ray, RaycastHit[] results, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return RaycastNonAlloc(ray.origin, ray.direction, results, maxDistance, layerMask, queryTriggerInteraction);
        }

        public static RaycastHit[] SphereCastAll(Vector3 origin, float radius, Vector3 direction, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<RaycastHit>();
            }
            return provider.SphereCastAll(origin, direction, radius, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static RaycastHit[] SphereCastAll(Ray ray, float radius, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return SphereCastAll(ray.origin, radius, ray.direction, maxDistance, layerMask, queryTriggerInteraction);
        }

        public static int SphereCastNonAlloc(Vector3 origin, float radius, Vector3 direction, RaycastHit[] results, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return 0;
            }
            return provider.SphereCastNonAlloc(results, origin, direction, radius, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static int SphereCastNonAlloc(Ray ray, float radius, RaycastHit[] results, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return SphereCastNonAlloc(ray.origin, radius, ray.direction, results, maxDistance, layerMask, queryTriggerInteraction);
        }

        public static bool CapsuleCast(Vector3 point1, Vector3 point2, float radius, Vector3 direction, out RaycastHit hit, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.CapsuleCast(point1, point2, radius, direction, out hit, maxDistance, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static bool CapsuleCast(Vector3 point1, Vector3 point2, float radius, Vector3 direction, float maxDistance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            return CapsuleCast(point1, point2, radius, direction, out _, maxDistance, layerMask, queryTriggerInteraction);
        }

        public static Collider[] OverlapBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider>();
            }
            return provider.OverlapBox(center, halfExtents, orientation, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static Collider[] OverlapCapsule(Vector3 point0, Vector3 point1, float radius, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider>();
            }
            return provider.OverlapCapsule(point0, point1, radius, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static bool CheckBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.CheckBox(center, halfExtents, orientation, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static bool CheckSphere(Vector3 position, float radius, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.CheckSphere(position, radius, layerMask, IncludesTriggers(queryTriggerInteraction));
        }

        public static bool CheckCapsule(Vector3 point1, Vector3 point2, float radius, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.CheckCapsule(point1, point2, radius, layerMask, IncludesTriggers(queryTriggerInteraction));
        }
    }
}
