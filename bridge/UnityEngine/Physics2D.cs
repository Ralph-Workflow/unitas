using System;

namespace UnityEngine
{
    public interface IPhysics2DProvider
    {
        bool Raycast2D(Vector2 origin, Vector2 direction, float maxDistance, int layerMask, bool includeTriggers, out RaycastHit2D hit);

        Collider2D[] OverlapPoint2D(Vector2 position, int layerMask);

        Collider2D[] OverlapCircle2D(Vector2 position, float radius, int layerMask);

        Collider2D[] OverlapBox2D(Vector2 position, Vector2 size, float angle, int layerMask);

        RaycastHit2D[] BoxCast2D(Vector2 origin, Vector2 size, float angle, Vector2 direction, float distance, int layerMask, ContactFilter2D filter)
        {
            return Array.Empty<RaycastHit2D>();
        }

        bool TrySetBodyConstraints2D(ulong entity, RigidbodyConstraints2D constraints)
        {
            return false;
        }

        bool TryCreateCapsuleCollider2D(ulong entity, float radius, float height, int direction, Vector2 offset, bool isTrigger)
        {
            return false;
        }

        bool TryCreatePolygonCollider2D(ulong entity, ulong attachedBody, Vector2[] points, Vector2 offset, bool isTrigger, uint layer)
        {
            return false;
        }

        bool TryCreateEdgeCollider2D(ulong entity, ulong attachedBody, Vector2[] points, Vector2 offset, bool isTrigger, uint layer)
        {
            return false;
        }

        bool TrySetCompositeCollider2D(ulong entity, ulong compositeEntity, uint geometryType)
        {
            return false;
        }

        bool TryBuildTilemapCollider2D(ulong entity, ulong compositeEntity, string tilemapPath)
        {
            return false;
        }

        bool TrySetColliderMaterial2D(ulong entity, float friction, float bounciness)
        {
            return false;
        }

        bool TryApplyPhysics2DSettings(Vector2 gravity, int velocityIterations, int positionIterations, bool queriesHitTriggers)
        {
            return false;
        }

        bool TrySetLayerCollision2D(int layerA, int layerB, bool enabled)
        {
            return false;
        }

        bool TryGetLayerCollision2D(int layerA, int layerB, out bool enabled)
        {
            enabled = true;
            return false;
        }

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
        private static readonly int[] s_fallbackLayerMasks = CreateDefaultLayerMasks();
        private static Vector2 s_gravity = new Vector2(0f, -9.81f);
        private static int s_velocityIterations = 8;
        private static int s_positionIterations = 3;
        private static bool s_queriesHitTriggers = true;

        public static IPhysics2DProvider? Provider { get; set; }

        public static Vector2 gravity
        {
            get => s_gravity;
            set
            {
                s_gravity = value;
                PushSettingsToProvider();
            }
        }

        public static int velocityIterations
        {
            get => s_velocityIterations;
            set
            {
                s_velocityIterations = Math.Max(1, value);
                PushSettingsToProvider();
            }
        }

        public static int positionIterations
        {
            get => s_positionIterations;
            set
            {
                s_positionIterations = Math.Max(1, value);
                PushSettingsToProvider();
            }
        }

        public static bool queriesHitTriggers
        {
            get => s_queriesHitTriggers;
            set
            {
                s_queriesHitTriggers = value;
                PushSettingsToProvider();
            }
        }

        public static void IgnoreLayerCollision(int layer1, int layer2, bool ignore = true)
        {
            if (layer1 < 0 || layer1 >= 32 || layer2 < 0 || layer2 >= 32)
            {
                return;
            }
            if (ignore)
            {
                s_fallbackLayerMasks[layer1] &= ~(1 << layer2);
                s_fallbackLayerMasks[layer2] &= ~(1 << layer1);
            }
            else
            {
                s_fallbackLayerMasks[layer1] |= 1 << layer2;
                s_fallbackLayerMasks[layer2] |= 1 << layer1;
            }
            IPhysics2DProvider? provider = Provider;
            if (provider != null)
            {
                provider.TrySetLayerCollision2D(layer1, layer2, !ignore);
            }
        }

        public static bool GetIgnoreLayerCollision(int layer1, int layer2)
        {
            if (layer1 < 0 || layer1 >= 32 || layer2 < 0 || layer2 >= 32)
            {
                return false;
            }
            IPhysics2DProvider? provider = Provider;
            if (provider != null && provider.TryGetLayerCollision2D(layer1, layer2, out bool enabled))
            {
                return !enabled;
            }
            return (s_fallbackLayerMasks[layer1] & (1 << layer2)) == 0;
        }

        public static int GetLayerCollisionMask(int layer)
        {
            if (layer < 0 || layer >= 32)
            {
                return 0;
            }
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return s_fallbackLayerMasks[layer];
            }
            int mask = 0;
            bool anyNative = false;
            for (int other = 0; other < 32; other++)
            {
                if (provider.TryGetLayerCollision2D(layer, other, out bool enabled))
                {
                    anyNative = true;
                    if (enabled)
                    {
                        mask |= 1 << other;
                    }
                }
            }
            return anyNative ? mask : s_fallbackLayerMasks[layer];
        }

        public static void SetLayerCollisionMask(int layer, int layerMask)
        {
            if (layer < 0 || layer >= 32)
            {
                return;
            }
            s_fallbackLayerMasks[layer] = layerMask;
            for (int other = 0; other < 32; other++)
            {
                bool enabled = (layerMask & (1 << other)) != 0;
                if (enabled)
                {
                    s_fallbackLayerMasks[other] |= 1 << layer;
                }
                else
                {
                    s_fallbackLayerMasks[other] &= ~(1 << layer);
                }
            }
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return;
            }
            for (int other = 0; other < 32; other++)
            {
                bool enabled = (layerMask & (1 << other)) != 0;
                provider.TrySetLayerCollision2D(layer, other, enabled);
            }
        }

        public static bool Raycast(Vector2 origin, Vector2 direction, out RaycastHit2D hit, float maxDistance, int layerMask = DefaultRaycastLayers)
        {
            hit = default;
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.Raycast2D(origin, direction, maxDistance, layerMask, s_queriesHitTriggers, out hit);
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

        public static RaycastHit2D[] BoxCast(Vector2 origin, Vector2 size, float angle, Vector2 direction, float distance, ContactFilter2D contactFilter, int layerMask = DefaultRaycastLayers)
        {
            IPhysics2DProvider? provider = Provider;
            if (provider == null)
            {
                return Array.Empty<RaycastHit2D>();
            }
            return provider.BoxCast2D(origin, size, angle, direction, distance, layerMask, contactFilter);
        }

        private static void PushSettingsToProvider()
        {
            IPhysics2DProvider? provider = Provider;
            if (provider != null)
            {
                provider.TryApplyPhysics2DSettings(s_gravity, s_velocityIterations, s_positionIterations, s_queriesHitTriggers);
            }
        }

        private static int[] CreateDefaultLayerMasks()
        {
            int[] masks = new int[32];
            for (int i = 0; i < 32; i++)
            {
                masks[i] = -1;
            }
            return masks;
        }
    }
}
