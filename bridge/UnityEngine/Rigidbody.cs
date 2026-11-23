namespace UnityEngine
{
    public class Rigidbody : Component
    {
        private float _mass = 1f;
        private bool _useGravity = true;
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

        public Vector3 velocity
        {
            get => _velocity;
            set => _velocity = value;
        }
    }
}
