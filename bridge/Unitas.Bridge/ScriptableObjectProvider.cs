using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge
{
    /// <summary>
    /// Creates and destroys the ScriptableObject instances the native runtime
    /// world holds, marshalling the instance name through the runtime's own
    /// scriptable-object name buffer.
    /// </summary>
    public sealed class ScriptableObjectProvider : IScriptableObjectProvider
    {
        private readonly ulong _runtimeHandle;

        public ScriptableObjectProvider(ulong runtimeHandle)
        {
            _runtimeHandle = runtimeHandle;
        }

        public ulong CreateInstance(string name)
        {
            ArgumentNullException.ThrowIfNull(name);

            if (_runtimeHandle == 0UL)
            {
                return 0UL;
            }

            byte[] utf8 = Encoding.UTF8.GetBytes(name);
            IntPtr buffer = RuntimeInterop.unitas_runtime_scriptable_object_name_buffer(
                _runtimeHandle,
                (ulong)utf8.Length);
            if (buffer == IntPtr.Zero)
            {
                return 0UL;
            }

            Marshal.Copy(utf8, 0, buffer, utf8.Length);
            return RuntimeInterop.unitas_runtime_scriptable_object_create(
                _runtimeHandle,
                (ulong)buffer.ToInt64(),
                (ulong)utf8.Length);
        }

        public void DestroyInstance(ulong instance)
        {
            if (_runtimeHandle == 0UL || instance == 0UL)
            {
                return;
            }

            RuntimeInterop.unitas_runtime_scriptable_object_destroy(_runtimeHandle, instance);
        }

        public bool IsAlive(ulong instance)
        {
            if (_runtimeHandle == 0UL || instance == 0UL)
            {
                return false;
            }

            return RuntimeInterop.unitas_runtime_scriptable_object_is_alive(_runtimeHandle, instance) == 1;
        }
    }
}
