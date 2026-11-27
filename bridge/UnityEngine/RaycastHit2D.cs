namespace UnityEngine
{
    public struct RaycastHit2D
    {
        public Collider2D? collider;
        public Vector2 point;
        public Vector2 normal;
        public float distance;
        public float fraction;

        public Rigidbody2D? rigidbody
        {
            get
            {
                Collider2D? hitCollider = collider;
                return hitCollider == null ? null : hitCollider.attachedRigidbody;
            }
        }
    }
}
