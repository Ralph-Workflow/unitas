using System;

namespace UnityEngine
{
    public interface IPhysicsProvider
    {
        bool Raycast(Vector3 origin, Vector3 direction, float maxDistance, int layerMask, out RaycastHit hit);

        Collider[] OverlapSphere(Vector3 position, float radius, int layerMask);
    }

    public static class Physics
    {
        public static IPhysicsProvider? Provider { get; set; }

        public static bool Raycast(Vector3 origin, Vector3 direction, out RaycastHit hit, float maxDistance, int layerMask)
        {
            hit = default;
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Raycast(origin, direction, maxDistance, layerMask, out hit);
        }

        public static Collider[] OverlapSphere(Vector3 position, float radius, int layerMask)
        {
            IPhysicsProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider>();
            }
            return provider.OverlapSphere(position, radius, layerMask);
        }
    }
}
