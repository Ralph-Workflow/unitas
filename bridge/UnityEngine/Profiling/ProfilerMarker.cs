using System;

namespace Unity.Profiling
{
    public readonly struct ProfilerMarker
    {
        private readonly string _name;

        public ProfilerMarker(string name)
        {
            _name = name ?? string.Empty;
        }

        public string Name => _name;

        public void Begin()
        {
            UnityEngine.Profiling.Profiler.BeginSample(_name);
        }

        public void Begin(UnityEngine.Object? contextUnityObject)
        {
            UnityEngine.Profiling.Profiler.BeginSample(_name, contextUnityObject);
        }

        public void End()
        {
            UnityEngine.Profiling.Profiler.EndSample();
        }

        public AutoScope Auto()
        {
            Begin();
            return new AutoScope(true);
        }

        public readonly struct AutoScope : IDisposable
        {
            private readonly bool _active;

            internal AutoScope(bool active)
            {
                _active = active;
            }

            public void Dispose()
            {
                if (_active)
                {
                    UnityEngine.Profiling.Profiler.EndSample();
                }
            }
        }
    }
}

namespace UnityEngine.Profiling
{
    public readonly struct ProfilerMarker
    {
        private readonly string _name;

        public ProfilerMarker(string name)
        {
            _name = name ?? string.Empty;
        }

        public string Name => _name;

        public void Begin()
        {
            Profiler.BeginSample(_name);
        }

        public void Begin(Object? contextUnityObject)
        {
            Profiler.BeginSample(_name, contextUnityObject);
        }

        public void End()
        {
            Profiler.EndSample();
        }

        public AutoScope Auto()
        {
            Begin();
            return new AutoScope(true);
        }

        public readonly struct AutoScope : IDisposable
        {
            private readonly bool _active;

            internal AutoScope(bool active)
            {
                _active = active;
            }

            public void Dispose()
            {
                if (_active)
                {
                    Profiler.EndSample();
                }
            }
        }
    }
}
