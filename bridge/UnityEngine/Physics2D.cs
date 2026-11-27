using System;

namespace UnityEngine
{
    public interface IPhysics2DProvider
    {
        bool Raycast2D(Vector2 origin, Vector2 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit2D hit);

        Collider2D[] OverlapPoint2D(Vector2 position, int layerMask);

        Collider2D[] OverlapCircle2D(Vector2 position, float radius, int layerMask);

        Collider2D[] OverlapBox2D(Vector2 position, Vector2 size, float angle, int layerMask);

        bool TryGetBodyVelocity2D(ulong entity, out Vector2 velocity)
        {
            velocity = default;
            return false;
        }

        bool TrySetBodyVelocity2D(ulong entity, Vector2 velocity)
        {
            return false;
        }

        bool TryMoveBodyPosition2D(ulong entity, Vector2 position)
        {
            return false;
        }

        bool TrySetBodySimulated2D(ulong entity, bool simulated)
        {
            return false;
        }

        bool TrySetBodyType2D(ulong entity, RigidbodyType2D bodyType)
        {
            return false;
        }

        bool TrySetBodyInterpolation2D(ulong entity, RigidbodyInterpolation2D interpolation)
        {
            return false;
        }

        bool TrySetBodyCollisionDetectionMode2D(ulong entity, CollisionDetectionMode2D mode)
        {
            return false;
        }

        bool TryGetCollisionSummary2D(ulong a, ulong b, out CollisionSummary2D summary)
        {
            summary = default;
            return false;
        }

        bool TryGetCollisionContact2D(ulong a, ulong b, uint index, out ContactPoint2D contact)
        {
            contact = default;
            return false;
        }
    }

    public static class Physics2D
    {
        public const int DefaultRaycastLayers = -1;

        public static IPhysics2DProvider? Provider { get; set; }

        public static bool Raycast(Vector2 origin, Vector2 direction, out RaycastHit2D hit, float maxDistance, int layerMask = DefaultRaycastLayers)
        {
            hit = default;
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Raycast2D(origin, direction, maxDistance, layerMask, true, out hit);
        }

        public static Collider2D[] OverlapPoint(Vector2 position, int layerMask = DefaultRaycastLayers)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider2D>();
            }
            return provider.OverlapPoint2D(position, layerMask);
        }

        public static Collider2D[] OverlapCircle(Vector2 position, float radius, int layerMask = DefaultRaycastLayers)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider2D>();
            }
            return provider.OverlapCircle2D(position, radius, layerMask);
        }

        public static Collider2D[] OverlapBox(Vector2 position, Vector2 size, float angle, int layerMask = DefaultRaycastLayers)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<Collider2D>();
            }
            return provider.OverlapBox2D(position, size, angle, layerMask);
        }
    }
}
