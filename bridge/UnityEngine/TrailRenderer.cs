namespace UnityEngine
{
    public sealed class TrailRenderer : Behaviour
    {
        private const float DefaultTime = 5f;
        private const float DefaultStartWidth = 1f;
        private const float DefaultEndWidth = 0f;
        private const float DefaultMinVertexDistance = 0f;

        private bool _emitting = true;
        private float _time = DefaultTime;
        private float _startWidth = DefaultStartWidth;
        private float _endWidth = DefaultEndWidth;
        private float _minVertexDistance = DefaultMinVertexDistance;
        private Color _startColor = Color.white;
        private Color _endColor = new Color(1f, 1f, 1f, 0f);

        public bool emitting
        {
            get => _emitting;
            set => _emitting = value;
        }

        public float time
        {
            get => _time;
            set => _time = value;
        }

        public float startWidth
        {
            get => _startWidth;
            set => _startWidth = value;
        }

        public float endWidth
        {
            get => _endWidth;
            set => _endWidth = value;
        }

        public Color startColor
        {
            get => _startColor;
            set => _startColor = value;
        }

        public Color endColor
        {
            get => _endColor;
            set => _endColor = value;
        }

        public float minVertexDistance
        {
            get => _minVertexDistance;
            set => _minVertexDistance = value;
        }
    }
}
