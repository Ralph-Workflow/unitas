namespace UnityEngine
{
    public class Rigidbody : Component
    {
        private float _mass = 1f;
        private bool _useGravity = true;
        private bool _isKinematic;
        private Vector3 _velocity;

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
            set => _velocity = value;
        }
    }
}
