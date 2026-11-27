namespace UnityEngine
{
    public class PointEffector2D : Component
    {
        private float _forceMagnitude = -10f;
        private float _forceVariation;
        private float _distanceScale = 1f;
        private float _drag;
        private float _angularDrag;
        private bool _useColliderMask = true;
        private int _colliderMask = -1;

        public float forceMagnitude
        {
            get => _forceMagnitude;
            set
            {
                _forceMagnitude = value;
                SyncToNative();
            }
        }

        public float forceVariation
        {
            get => _forceVariation;
            set
            {
                _forceVariation = value;
                SyncToNative();
            }
        }

        public float distanceScale
        {
            get => _distanceScale;
            set
            {
                _distanceScale = value;
                SyncToNative();
            }
        }

        public float drag
        {
            get => _drag;
            set
            {
                _drag = value;
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
                provider.TrySetPointEffector2D(
                    owner.NativeEntityId,
                    _forceMagnitude,
                    _forceVariation,
                    _distanceScale,
                    _drag,
                    _angularDrag,
                    mask);
            }
        }
    }
}
