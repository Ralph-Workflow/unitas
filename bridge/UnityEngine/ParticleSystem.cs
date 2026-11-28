namespace UnityEngine
{
    public enum ParticleSystemStopBehavior
    {
        StopEmittingAndClear = 0,
        StopEmitting = 1,
    }

    public interface IParticleSystemProvider
    {
        bool EnsureSystem(ulong entity);

        bool HasSystem(ulong entity);

        bool Play(ulong entity);

        bool Pause(ulong entity);

        bool Stop(ulong entity, ParticleSystemStopBehavior stopBehavior);

        bool Clear(ulong entity);

        bool Simulate(ulong entity, float deltaTime, bool restart);

        bool Emit(ulong entity, int count);

        bool TryGetIsPlaying(ulong entity, out bool isPlaying);

        bool TryGetIsPaused(ulong entity, out bool isPaused);

        bool TryGetIsStopped(ulong entity, out bool isStopped);

        bool TryGetParticleCount(ulong entity, out int count);
    }

    public sealed class ParticleSystem : Component
    {
        public static IParticleSystemProvider? Provider { get; set; }

        private bool _isPlaying;
        private bool _isPaused;
        private int _particleCount;

        public bool isPlaying
        {
            get
            {
                if (TryNativeEntity(out ulong entity) && Provider!.TryGetIsPlaying(entity, out bool native))
                {
                    return native;
                }
                return _isPlaying;
            }
        }

        public bool isPaused
        {
            get
            {
                if (TryNativeEntity(out ulong entity) && Provider!.TryGetIsPaused(entity, out bool native))
                {
                    return native;
                }
                return _isPaused;
            }
        }

        public bool isStopped
        {
            get
            {
                if (TryNativeEntity(out ulong entity) && Provider!.TryGetIsStopped(entity, out bool native))
                {
                    return native;
                }
                return !_isPlaying && !_isPaused;
            }
        }

        public int particleCount
        {
            get
            {
                if (TryNativeEntity(out ulong entity) && Provider!.TryGetParticleCount(entity, out int native))
                {
                    return native;
                }
                return _particleCount;
            }
        }

        public void Play()
        {
            Play(true);
        }

        public void Play(bool withChildren)
        {
            _isPlaying = true;
            _isPaused = false;
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Play(entity);
            }
            if (withChildren)
            {
                ApplyToChildren(child => child.Play(false));
            }
        }

        public void Pause()
        {
            Pause(true);
        }

        public void Pause(bool withChildren)
        {
            if (_isPlaying)
            {
                _isPlaying = false;
                _isPaused = true;
            }
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Pause(entity);
            }
            if (withChildren)
            {
                ApplyToChildren(child => child.Pause(false));
            }
        }

        public void Stop()
        {
            Stop(true, ParticleSystemStopBehavior.StopEmitting);
        }

        public void Stop(bool withChildren)
        {
            Stop(withChildren, ParticleSystemStopBehavior.StopEmitting);
        }

        public void Stop(ParticleSystemStopBehavior stopBehavior)
        {
            Stop(true, stopBehavior);
        }

        public void Stop(bool withChildren, ParticleSystemStopBehavior stopBehavior)
        {
            _isPlaying = false;
            _isPaused = false;
            if (stopBehavior == ParticleSystemStopBehavior.StopEmittingAndClear)
            {
                _particleCount = 0;
            }
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Stop(entity, stopBehavior);
            }
            if (withChildren)
            {
                ApplyToChildren(child => child.Stop(false, stopBehavior));
            }
        }

        public void Clear()
        {
            Clear(true);
        }

        public void Clear(bool withChildren)
        {
            _particleCount = 0;
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Clear(entity);
            }
            if (withChildren)
            {
                ApplyToChildren(child => child.Clear(false));
            }
        }

        public void Simulate(float t)
        {
            Simulate(t, true, true, true);
        }

        public void Simulate(float t, bool withChildren)
        {
            Simulate(t, withChildren, true, true);
        }

        public void Simulate(float t, bool withChildren, bool restart)
        {
            Simulate(t, withChildren, restart, true);
        }

        public void Simulate(float t, bool withChildren, bool restart, bool fixedTimeStep)
        {
            _ = fixedTimeStep;
            if (restart)
            {
                _particleCount = 0;
            }
            _isPlaying = false;
            _isPaused = true;
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Simulate(entity, t, restart);
            }
            if (withChildren)
            {
                ApplyToChildren(child => child.Simulate(t, false, restart, fixedTimeStep));
            }
        }

        public void Emit(int count)
        {
            if (count <= 0)
            {
                return;
            }
            _particleCount += count;
            if (TryEnsureNativeEntity(out ulong entity))
            {
                Provider!.Emit(entity, count);
            }
        }

        private void ApplyToChildren(System.Action<ParticleSystem> action)
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }
            ParticleSystem[] children = owner.GetComponentsInChildren<ParticleSystem>(true);
            for (int i = 0; i < children.Length; i++)
            {
                ParticleSystem child = children[i];
                if (!ReferenceEquals(child, this))
                {
                    action(child);
                }
            }
        }

        private bool TryNativeEntity(out ulong entity)
        {
            entity = 0;
            if (Provider == null)
            {
                return false;
            }
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }
            entity = owner.NativeEntityId;
            return entity != 0;
        }

        private bool TryEnsureNativeEntity(out ulong entity)
        {
            if (!TryNativeEntity(out entity))
            {
                return false;
            }
            Provider!.EnsureSystem(entity);
            return true;
        }
    }
}
