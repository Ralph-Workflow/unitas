namespace UnityEngine
{
    public abstract class Collider2D : Component
    {
        private bool _enabled = true;
        private bool _isTrigger;
        private Vector2 _offset;

        public bool enabled
        {
            get => _enabled;
            set => _enabled = value;
        }

        public bool isTrigger
        {
            get => _isTrigger;
            set => _isTrigger = value;
        }

        public Vector2 offset
        {
            get => _offset;
            set => _offset = value;
        }

        public Rigidbody2D? attachedRigidbody
        {
            get
            {
                GameObject? owner = gameObject;
                return owner == null ? null : owner.GetComponent<Rigidbody2D>();
            }
        }
    }
}
