using System;
using System.Runtime.InteropServices;
using UnityEngine;

namespace Unitas.Bridge
{
    /// Reads and writes the native trail behind a managed TrailRenderer.
    public sealed class RuntimeTrailProvider : ITrailProvider
    {
        private const int SettingsFloats = 12;
        private const int SettingsBytes = SettingsFloats * sizeof(float);

        private readonly ulong _runtimeHandle;
        private readonly float[] _settings = new float[SettingsFloats];

        public RuntimeTrailProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public bool TrySetTrailEmitting(ulong entity, bool emitting)
        {
            if (entity == 0)
            {
                return false;
            }
            bool before = TryGetTrailEmitting(entity, out bool previous) && previous;
            if (RuntimeInterop.unitas_runtime_trail_set_emitting(_runtimeHandle, entity, emitting ? (byte)1 : (byte)0) != 0)
            {
                return false;
            }
            if (TryGetTrailEmitting(entity, out bool after) && after != before)
            {
                Debug.Log($"Trail emitting {after}");
            }
            return true;
        }

        public bool TryGetTrailEmitting(ulong entity, out bool emitting)
        {
            emitting = false;
            if (entity == 0)
            {
                return false;
            }
            int code = RuntimeInterop.unitas_runtime_trail_get_emitting(_runtimeHandle, entity);
            if (code < 0)
            {
                return false;
            }
            emitting = code != 0;
            return true;
        }

        public bool TrySetTrailSettings(ulong entity, TrailSettings settings)
        {
            if (entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)SettingsBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            WriteSettings(settings);
            Marshal.Copy(_settings, 0, buffer, SettingsFloats);
            return RuntimeInterop.unitas_runtime_trail_configure(_runtimeHandle, entity, buffer) == 0;
        }

        public bool TryGetTrailSettings(ulong entity, out TrailSettings settings)
        {
            settings = default;
            if (entity == 0)
            {
                return false;
            }
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, (ulong)SettingsBytes);
            if (buffer == IntPtr.Zero)
            {
                return false;
            }
            if (RuntimeInterop.unitas_runtime_trail_get_settings(_runtimeHandle, entity, buffer) != 0)
            {
                return false;
            }
            Marshal.Copy(buffer, _settings, 0, SettingsFloats);
            settings = new TrailSettings(
                _settings[0],
                _settings[1],
                _settings[2],
                _settings[3],
                new Color(_settings[4], _settings[5], _settings[6], _settings[7]),
                new Color(_settings[8], _settings[9], _settings[10], _settings[11]));
            return true;
        }

        private void WriteSettings(TrailSettings settings)
        {
            _settings[0] = settings.Time;
            _settings[1] = settings.StartWidth;
            _settings[2] = settings.EndWidth;
            _settings[3] = settings.MinVertexDistance;
            _settings[4] = settings.StartColor.r;
            _settings[5] = settings.StartColor.g;
            _settings[6] = settings.StartColor.b;
            _settings[7] = settings.StartColor.a;
            _settings[8] = settings.EndColor.r;
            _settings[9] = settings.EndColor.g;
            _settings[10] = settings.EndColor.b;
            _settings[11] = settings.EndColor.a;
        }
    }
}
