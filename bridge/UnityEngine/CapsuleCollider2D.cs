namespace UnityEngine
{
    public class CapsuleCollider2D : Collider2D
    {
        private Vector2 _size = new Vector2(0.5f, 1f);
        private int _direction = 1;

        public Vector2 size
        {
            get => _size;
            set
            {
                _size = value;
                ApplyNativeShape();
            }
        }

        public int direction
        {
            get => _direction;
            set
            {
                _direction = value;
                ApplyNativeShape();
            }
        }

        protected override void ApplyNativeShape()
        {
            GameObject? owner = gameObject;
            IPhysics2DProvider? provider = Physics2D.Provider;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || provider == null)
            {
                return;
            }
            float radius = _direction != 0 ? _size.x * 0.5f : _size.y * 0.5f;
            float height = _direction != 0 ? _size.y : _size.x;
            provider.TryCreateCapsuleCollider2D(owner.NativeEntityId, radius, height, _direction, offset, isTrigger);
        }
    }
}
