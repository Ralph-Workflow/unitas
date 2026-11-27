namespace UnityEngine
{
    public class Rigidbody2D : Component
    {
        private float _mass = 1f;
        private float _gravityScale = 1f;
        private RigidbodyType2D _bodyType = RigidbodyType2D.Dynamic;
        private bool _simulated = true;
        private RigidbodyInterpolation2D _interpolation = RigidbodyInterpolation2D.None;
        private CollisionDetectionMode2D _collisionDetectionMode = CollisionDetectionMode2D.Discrete;
        private Vector2 _velocity;

        public float mass
        {
            get => _mass;
            set => _mass = value;
        }

        public float gravityScale
        {
            get => _gravityScale;
            set => _gravityScale = value;
        }

        public RigidbodyType2D bodyType
        {
            get => _bodyType;
            set
            {
                _bodyType = value;
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyType2D(owner.NativeEntityId, value);
                }
            }
        }

        public bool simulated
        {
            get => _simulated;
            set
            {
                _simulated = value;
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodySimulated2D(owner.NativeEntityId, value);
                }
            }
        }

        public RigidbodyInterpolation2D interpolation
        {
            get => _interpolation;
            set
            {
                _interpolation = value;
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyInterpolation2D(owner.NativeEntityId, value);
                }
            }
        }

        public CollisionDetectionMode2D collisionDetectionMode
        {
            get => _collisionDetectionMode;
            set
            {
                _collisionDetectionMode = value;
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyCollisionDetectionMode2D(owner.NativeEntityId, value);
                }
            }
        }

        public Vector2 velocity
        {
            get
            {
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null && provider.TryGetBodyVelocity2D(owner.NativeEntityId, out Vector2 nativeVelocity))
                {
                    return nativeVelocity;
                }
                return _velocity;
            }
            set
            {
                _velocity = value;
                GameObject? owner = gameObject;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
                {
                    provider.TrySetBodyVelocity2D(owner.NativeEntityId, value);
                }
            }
        }

        public void MovePosition(Vector2 position)
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner != null && !owner.IsDestroyed && owner.NativeEntityId != 0 && provider != null)
            {
                provider.TryMoveBodyPosition2D(owner.NativeEntityId, position);
            }
        }
    }
}
