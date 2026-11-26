namespace UnityEngine
{
    public class Rigidbody : Component
    {
        private float _mass = 1f;
        private bool _useGravity = true;
        private bool _isKinematic;
        private Vector3 _velocity;
        private CollisionDetectionMode _collisionDetectionMode;

        public float mass
        {
            get => _mass;
            set => _mass = value;
        }

        public bool useGravity
        {
            get => _useGravity;
            set => _useGravity = value;
        }

        public bool isKinematic
        {
            get => _isKinematic;
            set => _isKinematic = value;
        }

        public Vector3 velocity
        {
            get
            {
                GameObject? owner = gameObject;
                IPhysicsProvider? provider = Physics.Provider;
                if (!_isKinematic && owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null && provider.TryGetBodyVelocity(owner.NativeEntityId, out Vector3 nativeVelocity))
                {
                    return nativeVelocity;
                }
                return _velocity;
            }
            set
            {
                _velocity = value;
                GameObject? owner = gameObject;
                IPhysicsProvider? provider = Physics.Provider;
                if (!_isKinematic && owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyVelocity(owner.NativeEntityId, value);
                }
            }
        }
        public CollisionDetectionMode collisionDetectionMode
        {
            get => _collisionDetectionMode;
            set
            {
                _collisionDetectionMode = value;
                GameObject? owner = gameObject;
                IPhysicsProvider? provider = Physics.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyCollisionDetectionMode(owner.NativeEntityId, value);
                }
            }
        }
        public void MovePosition(Vector3 position)
        {
            GameObject? owner = gameObject;
            IPhysicsProvider? provider = Physics.Provider;
            if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
            {
                provider.TryMoveBodyPosition(owner.NativeEntityId, position);
            }
        }
        public void AddForce(Vector3 force)
        {
            AddForce(force, ForceMode.Force);
        }
        public void AddForce(Vector3 force, ForceMode mode)
        {
            GameObject? owner = gameObject;
            IPhysicsProvider? provider = Physics.Provider;
            if (!_isKinematic && owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
            {
                provider.TryAddForce(owner.NativeEntityId, force, mode);
            }
        }
    }
}
