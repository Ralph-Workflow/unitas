using System;
using UnityEngine;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost
    {
        public RaycastHit[] RaycastAll(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_raycast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return ReadRaycastHitArray(count);
        }

        public int RaycastNonAlloc(RaycastHit[] results, Vector3 origin, Vector3 direction, float maxDistance, int layerMask, bool includeTriggers)
        {
            if (results == null || results.Length == 0)
            {
                return 0;
            }
            RaycastHit[] hits = RaycastAll(origin, direction, maxDistance, layerMask, includeTriggers);
            int count = Math.Min(hits.Length, results.Length);
            Array.Copy(hits, results, count);
            return count;
        }

        public RaycastHit[] SphereCastAll(Vector3 origin, Vector3 direction, float radius, float maxDistance, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_sphere_cast(
                _runtimeHandle,
                origin.x,
                origin.y,
                origin.z,
                direction.x,
                direction.y,
                direction.z,
                radius,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return ReadRaycastHitArray(count);
        }

        public int SphereCastNonAlloc(RaycastHit[] results, Vector3 origin, Vector3 direction, float radius, float maxDistance, int layerMask, bool includeTriggers)
        {
            if (results == null || results.Length == 0)
            {
                return 0;
            }
            RaycastHit[] hits = SphereCastAll(origin, direction, radius, maxDistance, layerMask, includeTriggers);
            int count = Math.Min(hits.Length, results.Length);
            Array.Copy(hits, results, count);
            return count;
        }

        public bool CapsuleCast(Vector3 point1, Vector3 point2, float radius, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask, bool includeTriggers)
        {
            hit = default;
            int count = RuntimeInterop.unitas_runtime_capsule_cast(
                _runtimeHandle,
                point1.x,
                point1.y,
                point1.z,
                point2.x,
                point2.y,
                point2.z,
                radius,
                direction.x,
                direction.y,
                direction.z,
                maxDistance,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return TakeNearestHit(count, out hit);
        }

        public Collider[] OverlapBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_box(
                _runtimeHandle,
                center.x,
                center.y,
                center.z,
                halfExtents.x,
                halfExtents.y,
                halfExtents.z,
                orientation.x,
                orientation.y,
                orientation.z,
                orientation.w,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return ReadColliderArray(count);
        }

        public Collider[] OverlapCapsule(Vector3 point0, Vector3 point1, float radius, int layerMask, bool includeTriggers)
        {
            int count = RuntimeInterop.unitas_runtime_overlap_capsule(
                _runtimeHandle,
                point0.x,
                point0.y,
                point0.z,
                point1.x,
                point1.y,
                point1.z,
                radius,
                (uint)layerMask,
                includeTriggers ? 1 : 0);
            return ReadColliderArray(count);
        }

        public bool CheckBox(Vector3 center, Vector3 halfExtents, Quaternion orientation, int layerMask, bool includeTriggers)
        {
            return RuntimeInterop.unitas_runtime_check_box(
                _runtimeHandle,
                center.x,
                center.y,
                center.z,
                halfExtents.x,
                halfExtents.y,
                halfExtents.z,
                orientation.x,
                orientation.y,
                orientation.z,
                orientation.w,
                (uint)layerMask,
                includeTriggers ? 1 : 0) > 0;
        }

        public bool CheckSphere(Vector3 position, float radius, int layerMask, bool includeTriggers)
        {
            return RuntimeInterop.unitas_runtime_check_sphere(
                _runtimeHandle,
                position.x,
                position.y,
                position.z,
                radius,
                (uint)layerMask,
                includeTriggers ? 1 : 0) > 0;
        }

        public bool CheckCapsule(Vector3 point1, Vector3 point2, float radius, int layerMask, bool includeTriggers)
        {
            return RuntimeInterop.unitas_runtime_check_capsule(
                _runtimeHandle,
                point1.x,
                point1.y,
                point1.z,
                point2.x,
                point2.y,
                point2.z,
                radius,
                (uint)layerMask,
                includeTriggers ? 1 : 0) > 0;
        }

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
            return ReadRaycastHit2DArray(count, distance);
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
            return ReadRaycastHit2DArray(count, distance);
        }

        private RaycastHit[] ReadRaycastHitArray(int count)
        {
            if (count <= 0)
            {
                return Array.Empty<RaycastHit>();
            }
            QueryHit[] hits = ReadQueryHits(count);
            RaycastHit[] results = new RaycastHit[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                QueryHit current = hits[index];
                results[index] = new RaycastHit
                {
                    collider = GetCollider(current.Entity)!,
                    point = new Vector3((float)current.X, (float)current.Y, (float)current.Z),
                    normal = new Vector3((float)current.NX, (float)current.NY, (float)current.NZ),
                    distance = (float)current.Distance,
                };
            }
            return results;
        }

        private Collider[] ReadColliderArray(int count)
        {
            if (count <= 0)
            {
                return Array.Empty<Collider>();
            }
            QueryHit[] hits = ReadQueryHits(count);
            Collider[] colliders = new Collider[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                colliders[index] = GetCollider(hits[index].Entity)!;
            }
            return colliders;
        }

        private RaycastHit2D[] ReadRaycastHit2DArray(int count, float distance)
        {
            if (count <= 0)
            {
                return Array.Empty<RaycastHit2D>();
            }
            QueryHit2D[] hits = ReadQueryHits2D(count);
            RaycastHit2D[] results = new RaycastHit2D[hits.Length];
            for (int index = 0; index < hits.Length; index++)
            {
                QueryHit2D hit = hits[index];
                results[index] = new RaycastHit2D
                {
                    collider = ResolveCollider2DByEntity(hit.Entity),
                    point = new Vector2(hit.X, hit.Y),
                    normal = new Vector2(hit.NX, hit.NY),
                    distance = hit.Distance,
                    fraction = distance > 0f ? hit.Distance / distance : 0f,
                };
            }
            return results;
        }
    }
}
