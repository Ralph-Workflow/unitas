namespace UnityEngine
{
    public class SurfaceEffector2D : Component
    {
        private float _speed;
        private float _forceScale = 1f;
        private bool _useFriction = true;
        private bool _useBounce;

        public float speed
        {
            get => _speed;
            set => _speed = value;
        }

        public float forceScale
        {
            get => _forceScale;
            set => _forceScale = value;
        }

        public bool useFriction
        {
            get => _useFriction;
            set => _useFriction = value;
        }

        public bool useBounce
        {
            get => _useBounce;
            set => _useBounce = value;
        }
    }
}
