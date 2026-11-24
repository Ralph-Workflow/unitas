namespace UnityEngine
{
    public struct RaycastHit
    {
        public Collider collider;
        public Vector3 point;
        public Vector3 normal;
        public float distance;

        public Rigidbody? rigidbody
        {
            get
            {
                Collider? hitCollider = collider;
                return hitCollider == null ? null : hitCollider.attachedRigidbody;
            }
        }
    }
}
