using System;

namespace UnityEngine.AI
{
    public enum NavMeshObstacleShape
    {
        Capsule = 0,
        Box = 1,
    }

    public sealed class NavMeshObstacle : Behaviour
    {
        private bool _carving;
        private bool _carveOnlyStationary = true;
        private NavMeshObstacleShape _shape = NavMeshObstacleShape.Box;
        private Vector3 _center = Vector3.zero;
        private Vector3 _size = Vector3.one;
        private float _radius = 0.5f;
        private float _height = 1f;
        private Vector3 _velocity = Vector3.zero;

        public bool carving
        {
            get
            {
                if (TryNativeEntity(out ulong entity)
                    && NavMesh.Provider!.TryGetObstacleCarving(entity, out bool nativeCarving))
                {
                    return nativeCarving;
                }
                return _carving;
            }
            set
            {
                _carving = value;
                PushToNative();
            }
        }

        public bool carveOnlyStationary
        {
            get => _carveOnlyStationary;
            set
            {
                _carveOnlyStationary = value;
                PushToNative();
            }
        }

        public NavMeshObstacleShape shape
        {
            get => _shape;
            set
            {
                _shape = value;
                PushToNative();
            }
        }

        public Vector3 center
        {
            get => _center;
            set
            {
                _center = value;
                PushToNative();
            }
        }

        public Vector3 size
        {
            get => _size;
            set
            {
                _size = value;
                _radius = Math.Max(0.01f, value.x * 0.5f);
                _height = Math.Max(0.01f, value.y);
                PushToNative();
            }
        }

        public float radius
        {
            get => _radius;
            set
            {
                _radius = Math.Max(0.01f, value);
                _size = new Vector3(_radius * 2f, _height, _radius * 2f);
                PushToNative();
            }
        }

        public float height
        {
            get => _height;
            set
            {
                _height = Math.Max(0.01f, value);
                _size = new Vector3(_size.x, _height, _size.z);
                PushToNative();
            }
        }

        public Vector3 velocity
        {
            get => _velocity;
            set => _velocity = value;
        }

        private void PushToNative()
        {
            if (!TryNativeEntity(out ulong entity))
            {
                return;
            }
            NavMesh.Provider!.ConfigureObstacle(
                entity,
                enabled,
                _carving,
                _carveOnlyStationary,
                _shape,
                _center,
                _size,
                _radius,
                _height);
        }

        private bool TryNativeEntity(out ulong entity)
        {
            entity = 0UL;
            if (NavMesh.Provider == null)
            {
                return false;
            }
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }
            entity = owner.NativeEntityId;
            return entity != 0UL;
        }
    }
}
