namespace UnityEngine
{
    public class Collision2D
    {
        private readonly ulong _entityA;
        private readonly ulong _entityB;
        private CollisionSummary2D? _summary;

        public Collision2D(GameObject? gameObject, ulong entityA = 0, ulong entityB = 0)
        {
            this.gameObject = gameObject;
            _entityA = entityA;
            _entityB = entityB;
        }

        public GameObject? gameObject { get; }

        public int contactCount => Summary.contactCount;

        public float impulse => Summary.impulse;

        public Vector2 relativeVelocity => Summary.relativeVelocity;

        public Vector2 point => Summary.point;

        public Vector2 normal => Summary.normal;

        public ContactPoint2D GetContact(int index)
        {
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (provider == null || index < 0 || index >= contactCount)
            {
                return default;
            }
            if (provider.TryGetCollisionContact2D(_entityA, _entityB, (uint)index, out ContactPoint2D contact))
            {
                return contact;
            }
            return default;
        }

        private CollisionSummary2D Summary
        {
            get
            {
                if (_summary.HasValue)
                {
                    return _summary.Value;
                }
                CollisionSummary2D summary = default;
                IPhysics2DProvider? provider = Physics2D.Provider;
                if (provider != null)
                {
                    provider.TryGetCollisionSummary2D(_entityA, _entityB, out summary);
                }
                _summary = summary;
                return summary;
            }
        }
    }
}
