using System;

namespace UnityEngine
{
    public static partial class Physics2D
    {
        public static RaycastHit2D CircleCast(Vector2 origin, float radius, Vector2 direction, float distance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            RaycastHit2D[] hits = CircleCast(origin, radius, direction, distance, layerMask, ContactFilter2D.noFilter);
            return hits.Length > 0 ? hits[0] : default;
        }

        public static RaycastHit2D[] CircleCast(Vector2 origin, float radius, Vector2 direction, float distance, ContactFilter2D contactFilter)
        {
            return CircleCast(origin, radius, direction, distance, DefaultRaycastLayers, contactFilter);
        }

        public static RaycastHit2D[] CircleCast(Vector2 origin, float radius, Vector2 direction, float distance, int layerMask, ContactFilter2D contactFilter)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<RaycastHit2D>();
            }
            return provider.CircleCast2D(origin, radius, direction, distance, layerMask, contactFilter);
        }

        public static RaycastHit2D CapsuleCast(Vector2 origin, Vector2 size, CapsuleDirection2D capsuleDirection, float angle, Vector2 direction, float distance = Mathf.Infinity, int layerMask = DefaultRaycastLayers, QueryTriggerInteraction queryTriggerInteraction = QueryTriggerInteraction.UseGlobal)
        {
            RaycastHit2D[] hits = CapsuleCast(origin, size, capsuleDirection, angle, direction, distance, layerMask, ContactFilter2D.noFilter);
            return hits.Length > 0 ? hits[0] : default;
        }

        public static RaycastHit2D[] CapsuleCast(Vector2 origin, Vector2 size, CapsuleDirection2D capsuleDirection, float angle, Vector2 direction, float distance, ContactFilter2D contactFilter)
        {
            return CapsuleCast(origin, size, capsuleDirection, angle, direction, distance, DefaultRaycastLayers, contactFilter);
        }

        public static RaycastHit2D[] CapsuleCast(Vector2 origin, Vector2 size, CapsuleDirection2D capsuleDirection, float angle, Vector2 direction, float distance, int layerMask, ContactFilter2D contactFilter)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<RaycastHit2D>();
            }
            return provider.CapsuleCast2D(origin, size, capsuleDirection, angle, direction, distance, layerMask, contactFilter);
        }
    }
}
