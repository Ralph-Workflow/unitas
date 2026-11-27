namespace UnityEngine
{
    public class PlatformEffector2D : Component
    {
        private bool _useSideFriction;
        private bool _useSideBounce;
        private float _sideArc;
        private bool _useOneWay = true;
        private float _surfaceArc = 180f;
        private float _rotationalOffset;

        public bool useSideFriction
        {
            get => _useSideFriction;
            set => _useSideFriction = value;
        }

        public bool useSideBounce
        {
            get => _useSideBounce;
            set => _useSideBounce = value;
        }

        public float sideArc
        {
            get => _sideArc;
            set => _sideArc = value;
        }

        public bool useOneWay
        {
            get => _useOneWay;
            set => _useOneWay = value;
        }

        public float surfaceArc
        {
            get => _surfaceArc;
            set => _surfaceArc = value;
        }

        public float rotationalOffset
        {
            get => _rotationalOffset;
            set => _rotationalOffset = value;
        }
    }
}
