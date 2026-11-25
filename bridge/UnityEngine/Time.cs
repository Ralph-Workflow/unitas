namespace UnityEngine
{
    public interface ITimeScaleHost
    {
        void ApplyTimeScale(float timeScale);

        bool TryGetTimeScale(out float timeScale);
    }

    public static class Time
    {
        private const float DefaultFixedStep = 0.02f;
        private const float SmoothAlpha = 0.1f;

        private static float _fixedDeltaTime = DefaultFixedStep;
        private static float _deltaTime;
        private static float _unscaledDeltaTime;
        private static float _timeScale = 1f;
        private static float _time;
        private static float _unscaledTime;
        private static int _frameCount;
        private static float _realtimeSinceStartup;
        private static float _smoothDeltaTime;
        private static bool _hasSmoothSample;
        private static float _captureDeltaTime;

        public static float deltaTime => _deltaTime;

        public static float unscaledDeltaTime => _unscaledDeltaTime;

        public static float captureDeltaTime
        {
            get => _captureDeltaTime;
            set => _captureDeltaTime = value > 0f && float.IsFinite(value) ? value : 0f;
        }

        public static int captureFramerate
        {
            get => _captureDeltaTime > 0f ? (int)System.Math.Round(1.0 / _captureDeltaTime) : 0;
            set => _captureDeltaTime = value > 0 ? 1f / value : 0f;
        }

        public static ITimeScaleHost? TimeScaleHost { get; set; }

        public static float timeScale
        {
            get
            {
                if (TimeScaleHost != null && TimeScaleHost.TryGetTimeScale(out float scale))
                {
                    return scale;
                }
                return _timeScale;
            }
            set
            {
                _timeScale = value;
                TimeScaleHost?.ApplyTimeScale(value);
            }
        }

        public static void PushTimeScaleToHost()
        {
            TimeScaleHost?.ApplyTimeScale(_timeScale);
        }

        public static float time => _time;

        public static float unscaledTime => _unscaledTime;

        public static float fixedDeltaTime => _fixedDeltaTime;

        public static float fixedUnscaledDeltaTime => _fixedDeltaTime;

        public static int frameCount => _frameCount;

        public static float realtimeSinceStartup => _realtimeSinceStartup;

        public static float smoothDeltaTime => _smoothDeltaTime;

        public static void SetFixedDeltaTime(float value)
        {
            if (value > 0f && float.IsFinite(value))
            {
                _fixedDeltaTime = value;
            }
            else
            {
                _fixedDeltaTime = DefaultFixedStep;
            }
        }

        public static void Advance(double deltaSeconds)
        {
            float wallDelta = (float)deltaSeconds;
            float unscaled = _captureDeltaTime > 0f ? _captureDeltaTime : wallDelta;
            float scaled = unscaled * _timeScale;
            _unscaledDeltaTime = unscaled;
            _deltaTime = scaled;
            _unscaledTime += unscaled;
            _time += scaled;
            _frameCount++;
            _realtimeSinceStartup += wallDelta;
            if (_hasSmoothSample)
            {
                _smoothDeltaTime += (scaled - _smoothDeltaTime) * SmoothAlpha;
            }
            else
            {
                _smoothDeltaTime = scaled;
                _hasSmoothSample = true;
            }
        }

        public static void Reset()
        {
            _fixedDeltaTime = DefaultFixedStep;
            _deltaTime = 0f;
            _unscaledDeltaTime = 0f;
            _timeScale = 1f;
            TimeScaleHost?.ApplyTimeScale(1f);
            _time = 0f;
            _unscaledTime = 0f;
            _frameCount = 0;
            _realtimeSinceStartup = 0f;
            _smoothDeltaTime = 0f;
            _hasSmoothSample = false;
            _captureDeltaTime = 0f;
        }
    }
}
