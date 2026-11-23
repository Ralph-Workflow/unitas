namespace UnityEngine
{
    public class SphereCollider : Collider
    {
        private float _radius = 0.5f;
        private Vector3 _center;

        public float radius
        {
            get => _radius;
            set => _radius = value;
        }

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }
    }
}
