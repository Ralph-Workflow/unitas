namespace UnityEngine
{
    public sealed class CapsuleCollider : Collider
    {
        private Vector3 _center;
        private float _radius = 0.5f;
        private float _height = 2f;
        private int _direction = 1;

        public Vector3 center
        {
            get => _center;
            set => _center = value;
        }

        public float radius
        {
            get => _radius;
            set => _radius = value;
        }

        public float height
        {
            get => _height;
            set => _height = value;
        }

        public int direction
        {
            get => _direction;
            set => _direction = value;
        }
    }
}
