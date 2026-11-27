using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost
    {
        public RaycastHit2D[] CircleCast2D(Vector2 origin, float radius, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
        {
            int count = RuntimeInterop.unitas_runtime_circle_cast2d(
                _runtimeHandle,
                origin.x,
                origin.y,
                radius,
                direction.x,
                direction.y,
                distance,
                filter.useTriggers ? 1 : 0,
                (uint)layerMask,
                filter.useDepth ? 1 : 0,
                filter.minDepth,
                filter.maxDepth,
                filter.useOutsideNormalAngle ? 1 : 0,
                filter.minNormalAngle,
                filter.maxNormalAngle);
            return BuildRaycastHits2D(count, distance);
        }

        public RaycastHit2D[] CapsuleCast2D(Vector2 origin, Vector2 size, CapsuleDirection2D capsuleDirection, float angle, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
        {
            int count = RuntimeInterop.unitas_runtime_capsule_cast2d(
                _runtimeHandle,
                origin.x,
                origin.y,
                size.x,
                size.y,
                (uint)capsuleDirection,
                angle,
                direction.x,
                direction.y,
                distance,
                filter.useTriggers ? 1 : 0,
                (uint)layerMask,
                filter.useDepth ? 1 : 0,
                filter.minDepth,
                filter.maxDepth,
                filter.useOutsideNormalAngle ? 1 : 0,
                filter.minNormalAngle,
                filter.maxNormalAngle);
            return BuildRaycastHits2D(count, distance);
        }

        private RaycastHit2D[] BuildRaycastHits2D(int count, float distance)
        {
            if (count <= 0)
            {
                return Array.Empty<RaycastHit2D>();
            }
            QueryHit2D[] hits = ReadQueryHits2D(count);
            List<RaycastHit2D> results = new List<RaycastHit2D>(hits.Length);
            for (int index = 0; index < hits.Length; index++)
            {
                QueryHit2D hit = hits[index];
                results.Add(new RaycastHit2D
                {
                    collider = ResolveCollider2DByEntity(hit.Entity),
                    point = new Vector2(hit.X, hit.Y),
                    normal = new Vector2(hit.NX, hit.NY),
                    distance = hit.Distance,
                    fraction = distance > 0f ? hit.Distance / distance : 0f,
                });
            }
            return results.ToArray();
        }
    }
}
