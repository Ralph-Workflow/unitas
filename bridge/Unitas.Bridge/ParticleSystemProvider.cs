using UnityEngine;

namespace Unitas.Bridge
{
    public sealed class RuntimeParticleSystemProvider : IParticleSystemProvider
    {
        private readonly ulong _runtimeHandle;

        public RuntimeParticleSystemProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public bool EnsureSystem(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_ensure(_runtimeHandle, entity) == 0;
        }

        public bool HasSystem(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_has(_runtimeHandle, entity) > 0;
        }

        public bool Play(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_play(_runtimeHandle, entity) == 0;
        }

        public bool Pause(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_pause(_runtimeHandle, entity) == 0;
        }

        public bool Stop(ulong entity, ParticleSystemStopBehavior stopBehavior)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_stop(_runtimeHandle, entity, (uint)stopBehavior) == 0;
        }

        public bool Clear(ulong entity)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_clear(_runtimeHandle, entity) == 0;
        }

        public bool Simulate(ulong entity, float deltaTime, bool restart)
        {
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_simulate(_runtimeHandle, entity, deltaTime, restart ? (byte)1 : (byte)0) == 0;
        }

        public bool Emit(ulong entity, int count)
        {
            if (_runtimeHandle == 0 || entity == 0 || count < 0)
            {
                return false;
            }
            return RuntimeInterop.unitas_runtime_particle_system_emit(_runtimeHandle, entity, (uint)count) == 0;
        }

        public bool TryGetIsPlaying(ulong entity, out bool isPlaying)
        {
            isPlaying = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_particle_system_is_playing(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            isPlaying = code != 0;
            return true;
        }

        public bool TryGetIsPaused(ulong entity, out bool isPaused)
        {
            isPaused = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_particle_system_is_paused(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            isPaused = code != 0;
            return true;
        }

        public bool TryGetIsStopped(ulong entity, out bool isStopped)
        {
            isStopped = false;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_particle_system_is_stopped(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            isStopped = code != 0;
            return true;
        }

        public bool TryGetParticleCount(ulong entity, out int count)
        {
            count = 0;
            if (_runtimeHandle == 0 || entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_particle_system_particle_count(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            count = code;
            return true;
        }
    }
}
