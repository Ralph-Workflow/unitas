using System;
using System.Runtime.InteropServices;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge
{
    public sealed partial class BridgeHost : IDisposable, IPhysicsProvider, IPhysics2DProvider, ICharacterControllerProvider, IInputProvider, ISceneHost, IApplicationHost, IAudioProvider, IRuntimeObjectProvider
    {
        public void InjectKey(KeyCode key, bool pressed)
        {
            InjectKey((uint)key, pressed);
        }

        public void InjectKey(uint keyCode, bool pressed)
        {
            int result = RuntimeInterop.unitas_runtime_input_key_event(_runtimeHandle, keyCode, pressed ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject key event", result);
            }
        }

        public void InjectMouseButton(int button, bool pressed)
        {
            ArgumentOutOfRangeException.ThrowIfNegative(button);
            int result = RuntimeInterop.unitas_runtime_input_mouse_button_event(_runtimeHandle, (uint)button, pressed ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject mouse button event", result);
            }
        }

        public void InjectPointerPosition(double x, double y)
        {
            int result = RuntimeInterop.unitas_runtime_input_pointer_move(_runtimeHandle, x, y);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject pointer position", result);
            }
        }

        public void InjectScroll(double x, double y)
        {
            int result = RuntimeInterop.unitas_runtime_input_scroll(_runtimeHandle, x, y);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject scroll delta", result);
            }
        }

        public void InjectAxis(string axisName, double value)
        {
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                throw new ArgumentException("axis name must be a non-empty string", nameof(axisName));
            }

            int result = RuntimeInterop.unitas_runtime_input_axis_event(_runtimeHandle, buffer, length, value);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject axis value", result);
            }
        }

        public bool GetKey(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStateDown) != 0;
        }

        public bool GetKeyDown(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStatePressed) != 0;
        }

        public bool GetKeyUp(KeyCode key)
        {
            return (RuntimeInterop.unitas_runtime_input_key_state(_runtimeHandle, (uint)key) & InputStateReleased) != 0;
        }

        public bool GetMouseButton(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStateDown) != 0;
        }

        public bool GetMouseButtonDown(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStatePressed) != 0;
        }

        public bool GetMouseButtonUp(int button)
        {
            return button >= 0 && (RuntimeInterop.unitas_runtime_input_mouse_state(_runtimeHandle, (uint)button) & InputStateReleased) != 0;
        }

        public Vector3 GetMousePosition()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_pointer(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[32];
            Marshal.Copy(buffer, bytes, 0, 32);
            float x = (float)BitConverter.ToDouble(bytes, 0);
            float y = (float)BitConverter.ToDouble(bytes, 8);
            return new Vector3(x, y, 0f);
        }

        public Vector3 GetMousePositionDelta()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 32);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_pointer(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[32];
            Marshal.Copy(buffer, bytes, 0, 32);
            float dx = (float)BitConverter.ToDouble(bytes, 16);
            float dy = (float)BitConverter.ToDouble(bytes, 24);
            return new Vector3(dx, dy, 0f);
        }

        public Vector3 GetMouseScrollDelta()
        {
            IntPtr buffer = RuntimeInterop.unitas_runtime_path_buffer(_runtimeHandle, 16);
            if (buffer == IntPtr.Zero || RuntimeInterop.unitas_runtime_input_scroll_state(_runtimeHandle, buffer) != 0)
            {
                return Vector3.zero;
            }
            byte[] bytes = new byte[16];
            Marshal.Copy(buffer, bytes, 0, 16);
            float sx = (float)BitConverter.ToDouble(bytes, 0);
            float sy = (float)BitConverter.ToDouble(bytes, 8);
            return new Vector3(sx, sy, 0f);
        }

        public bool TryGetAxis(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_axis(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetAxisRaw(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_axis_raw(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetInjectedAxis(string axisName, out float value)
        {
            value = 0f;
            if (!TryWriteStringToPathBuffer(axisName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            double result = RuntimeInterop.unitas_runtime_input_injected_axis(_runtimeHandle, buffer, length);
            if (RuntimeInterop.unitas_runtime_error_message(_runtimeHandle) != IntPtr.Zero)
            {
                return false;
            }
            value = (float)result;
            return true;
        }

        public bool TryGetButton(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStateDown, out value);
        }

        public bool TryGetButtonDown(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStatePressed, out value);
        }

        public bool TryGetButtonUp(string buttonName, out bool value)
        {
            return TryQueryButtonState(buttonName, InputStateReleased, out value);
        }

        public void InjectGamepadConnected(bool connected)
        {
            int result = RuntimeInterop.unitas_runtime_input_gamepad_set_connected(_runtimeHandle, connected ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject gamepad connection state", result);
            }
        }

        public void InjectGamepadButton(uint button, bool pressed)
        {
            int result = RuntimeInterop.unitas_runtime_input_gamepad_button_event(_runtimeHandle, button, pressed ? 1u : 0u);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject gamepad button event", result);
            }
        }

        public void InjectGamepadAxis(uint axis, double value)
        {
            int result = RuntimeInterop.unitas_runtime_input_gamepad_axis_event(_runtimeHandle, axis, value);
            if (result != 0)
            {
                ThrowRuntimeError("Failed to inject gamepad axis event", result);
            }
        }

        public bool IsGamepadConnected()
        {
            return RuntimeInterop.unitas_runtime_input_gamepad_connected(_runtimeHandle) != 0;
        }

        public bool GetGamepadButton(uint button)
        {
            return (RuntimeInterop.unitas_runtime_input_gamepad_button_state(_runtimeHandle, button) & InputStateDown) != 0;
        }

        public bool GetGamepadButtonDown(uint button)
        {
            return (RuntimeInterop.unitas_runtime_input_gamepad_button_state(_runtimeHandle, button) & InputStatePressed) != 0;
        }

        public bool GetGamepadButtonUp(uint button)
        {
            return (RuntimeInterop.unitas_runtime_input_gamepad_button_state(_runtimeHandle, button) & InputStateReleased) != 0;
        }

        public float GetGamepadAxis(uint axis)
        {
            return (float)RuntimeInterop.unitas_runtime_input_gamepad_axis(_runtimeHandle, axis);
        }

        private bool TryQueryButtonState(string buttonName, uint stateMask, out bool value)
        {
            value = false;
            if (!TryWriteStringToPathBuffer(buttonName, out IntPtr buffer, out ulong length))
            {
                return false;
            }
            int mask = RuntimeInterop.unitas_runtime_input_button_state(_runtimeHandle, buffer, length);
            if (mask < 0)
            {
                return false;
            }
            value = ((uint)mask & stateMask) != 0;
            return true;
        }
    }
}
