namespace UnityEngine
{
    public class BuoyancyEffector2D : Component
    {
        private float _surfaceLevel;
        private float _density = 2f;
        private float _linearDrag = 5f;
        private float _angularDrag = 5f;
        private float _flowAngle;
        private float _flowMagnitude;
        private float _flowVariation;
        private bool _useColliderMask = true;
        private int _colliderMask = -1;

        public float surfaceLevel
        {
            get => _surfaceLevel;
            set
            {
                _surfaceLevel = value;
                SyncToNative();
            }
        }

        public float density
        {
            get => _density;
            set
            {
                _density = value;
                SyncToNative();
            }
        }

        public float linearDrag
        {
            get => _linearDrag;
            set
            {
                _linearDrag = value;
                SyncToNative();
            }
        }

        public float angularDrag
        {
            get => _angularDrag;
            set
            {
                _angularDrag = value;
                SyncToNative();
            }
        }

        public float flowAngle
        {
            get => _flowAngle;
            set
            {
                _flowAngle = value;
                SyncToNative();
            }
        }

        public float flowMagnitude
        {
            get => _flowMagnitude;
            set
            {
                _flowMagnitude = value;
                SyncToNative();
            }
        }

        public float flowVariation
        {
            get => _flowVariation;
            set
            {
                _flowVariation = value;
                SyncToNative();
            }
        }

        public bool useColliderMask
        {
            get => _useColliderMask;
            set
            {
                _useColliderMask = value;
                SyncToNative();
            }
        }

        public int colliderMask
        {
            get => _colliderMask;
            set
            {
                _colliderMask = value;
                SyncToNative();
            }
        }

        internal void SyncToNative()
        {
            GameObject? owner = OwnerOrNull;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return;
            }
            if (Physics2D.Provider is IPhysicsEffector2DProvider provider)
            {
                uint mask = _useColliderMask ? (uint)_colliderMask : uint.MaxValue;
                provider.TrySetBuoyancyEffector2D(
                    owner.NativeEntityId,
                    _surfaceLevel,
                    _density,
                    _linearDrag,
                    _angularDrag,
                    _flowAngle,
                    _flowMagnitude,
                    _flowVariation,
                    mask);
            }
        }
    }
}
