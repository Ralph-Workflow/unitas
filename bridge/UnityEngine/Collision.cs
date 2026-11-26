namespace UnityEngine
{
    public struct ContactPoint
    {
        public Vector3 point;
        public Vector3 normal;
        public float impulse;
    }

    public struct CollisionSummary
    {
        public float impulse;
        public Vector3 relativeVelocity;
        public int contactCount;
        public Vector3 point;
        public Vector3 normal;
    }

    public class Collision
    {
        private readonly ulong _entityA;
        private readonly ulong _entityB;
        private CollisionSummary? _summary;

        public Collision(GameObject? gameObject, ulong entityA = 0, ulong entityB = 0)
        {
            this.gameObject = gameObject;
            _entityA = entityA;
            _entityB = entityB;
        }

        public GameObject? gameObject { get; }

        public int contactCount => Summary.contactCount;

        public float impulse => Summary.impulse;

        public Vector3 relativeVelocity => Summary.relativeVelocity;

        public Vector3 point => Summary.point;

        public Vector3 normal => Summary.normal;

        public ContactPoint GetContact(int index)
        {
            IPhysicsProvider? provider = Physics.Provider;
            if (provider == null || index < 0 || index >= contactCount)
            {
                return default;
            }
            if (provider.TryGetCollisionContact(_entityA, _entityB, (uint)index, out ContactPoint contact))
            {
                return contact;
            }
            return default;
        }

        private CollisionSummary Summary
        {
            get
            {
                if (_summary.HasValue)
                {
                    return _summary.Value;
                }
                CollisionSummary summary = default;
                IPhysicsProvider? provider = Physics.Provider;
                if (provider != null)
                {
                    provider.TryGetCollisionSummary(_entityA, _entityB, out summary);
                }
                _summary = summary;
                return summary;
            }
        }
    }
}
