namespace UnityEngine
{
    public interface IPhysicsEffector2DProvider
    {
        bool TrySetAreaEffector2D(
            ulong entity,
            float forceAngle,
            float forceMagnitude,
            float forceVariation,
            float drag,
            float angularDrag,
            bool useGlobalAngle,
            uint colliderMask);

        bool TrySetPointEffector2D(
            ulong entity,
            float forceMagnitude,
            float forceVariation,
            float distanceScale,
            float drag,
            float angularDrag,
            uint colliderMask);

        bool TrySetBuoyancyEffector2D(
            ulong entity,
            float surfaceLevel,
            float density,
            float linearDrag,
            float angularDrag,
            float flowAngle,
            float flowMagnitude,
            float flowVariation,
            uint colliderMask);
    }

    public class AreaEffector2D : Component
    {
        private float _forceAngle;
        private float _forceMagnitude = 10f;
        private float _forceVariation;
        private float _drag;
        private float _angularDrag;
        private bool _useGlobalAngle;
        private bool _useColliderMask = true;
        private int _colliderMask = -1;

        public float forceAngle
        {
            get => _forceAngle;
            set
            {
                _forceAngle = value;
                SyncToNative();
            }
        }

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

        public bool useGlobalAngle
        {
            get => _useGlobalAngle;
            set
            {
                _useGlobalAngle = value;
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
                provider.TrySetAreaEffector2D(
                    owner.NativeEntityId,
                    _forceAngle,
                    _forceMagnitude,
                    _forceVariation,
                    _drag,
                    _angularDrag,
                    _useGlobalAngle,
                    mask);
            }
        }
    }
}
