using System;

namespace UnityEngine
{
    public interface IInputProvider
    {
        bool GetKey(KeyCode key);

        bool GetKeyDown(KeyCode key);

        bool GetKeyUp(KeyCode key);

        bool GetMouseButton(int button);

        bool GetMouseButtonDown(int button);

        bool GetMouseButtonUp(int button);

        Vector3 GetMousePosition();

        Vector3 GetMousePositionDelta();

        Vector3 GetMouseScrollDelta();

        bool TryGetAxis(string axisName, out float value);

        bool TryGetAxisRaw(string axisName, out float value);

        bool TryGetInjectedAxis(string axisName, out float value);

        bool TryGetButton(string buttonName, out bool value);

        bool TryGetButtonDown(string buttonName, out bool value);

        bool TryGetButtonUp(string buttonName, out bool value);
    }

    public static class Input
    {
        public static IInputProvider? Provider { get; set; }

        public static Vector3 mousePosition
        {
            get
            {
                IInputProvider? provider = Provider;
                if (provider == null)
                {
                    return Vector3.zero;
                }
                Vector3 pos = provider.GetMousePosition();
                return new Vector3(pos.x, pos.y, 0f);
            }
        }

        public static Vector3 mousePositionDelta
        {
            get
            {
                IInputProvider? provider = Provider;
                if (provider == null)
                {
                    return Vector3.zero;
                }
                Vector3 delta = provider.GetMousePositionDelta();
                return new Vector3(delta.x, delta.y, 0f);
            }
        }

        public static Vector3 mouseScrollDelta
        {
            get
            {
                IInputProvider? provider = Provider;
                if (provider == null)
                {
                    return Vector3.zero;
                }
                Vector3 delta = provider.GetMouseScrollDelta();
                return new Vector3(delta.x, delta.y, 0f);
            }
        }

        public static bool GetKey(KeyCode key)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetKey(key);
        }

        public static bool GetKey(string name)
        {
            if (!TryParseKeyName(name, out KeyCode key))
            {
                Debug.LogError("Input Key named: " + name + " is unknown");
                return false;
            }
            return GetKey(key);
        }

        public static bool GetKeyDown(KeyCode key)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetKeyDown(key);
        }

        public static bool GetKeyDown(string name)
        {
            if (!TryParseKeyName(name, out KeyCode key))
            {
                Debug.LogError("Input Key named: " + name + " is unknown");
                return false;
            }
            return GetKeyDown(key);
        }

        public static bool GetKeyUp(KeyCode key)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetKeyUp(key);
        }

        public static bool GetKeyUp(string name)
        {
            if (!TryParseKeyName(name, out KeyCode key))
            {
                Debug.LogError("Input Key named: " + name + " is unknown");
                return false;
            }
            return GetKeyUp(key);
        }

        public static bool GetMouseButton(int button)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetMouseButton(button);
        }

        public static bool GetMouseButtonDown(int button)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetMouseButtonDown(button);
        }

        public static bool GetMouseButtonUp(int button)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            return provider.GetMouseButtonUp(button);
        }

        public static float GetAxis(string axisName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return 0f;
            }
            if (string.IsNullOrEmpty(axisName))
            {
                Debug.LogError("Input Axis '' is not setup.");
                return 0f;
            }
            if (!provider.TryGetAxis(axisName, out float value))
            {
                Debug.LogError("Input Axis " + axisName + " is not setup.");
                return 0f;
            }
            return value;
        }

        public static float GetAxisRaw(string axisName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return 0f;
            }
            if (string.IsNullOrEmpty(axisName))
            {
                Debug.LogError("Input Axis '' is not setup.");
                return 0f;
            }
            if (!provider.TryGetAxisRaw(axisName, out float value))
            {
                Debug.LogError("Input Axis " + axisName + " is not setup.");
                return 0f;
            }
            return value;
        }

        // Returns the latest injected device value (gamepad stick) for an axis,
        // or 0f when the axis is unknown or has no injection. Unlike GetAxis,
        // button-derived values are never reported.
        public static float GetInjectedAxis(string axisName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return 0f;
            }
            if (string.IsNullOrEmpty(axisName))
            {
                Debug.LogError("Input Axis '' is not setup.");
                return 0f;
            }
            if (!provider.TryGetInjectedAxis(axisName, out float value))
            {
                return 0f;
            }
            return value;
        }

        public static bool GetButton(string buttonName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            if (string.IsNullOrEmpty(buttonName))
            {
                Debug.LogError("Input Button '' is not setup.");
                return false;
            }
            if (!provider.TryGetButton(buttonName, out bool value))
            {
                Debug.LogError("Input Button " + buttonName + " is not setup.");
                return false;
            }
            return value;
        }

        public static bool GetButtonDown(string buttonName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            if (string.IsNullOrEmpty(buttonName))
            {
                Debug.LogError("Input Button '' is not setup.");
                return false;
            }
            if (!provider.TryGetButtonDown(buttonName, out bool value))
            {
                Debug.LogError("Input Button " + buttonName + " is not setup.");
                return false;
            }
            return value;
        }

        public static bool GetButtonUp(string buttonName)
        {
            IInputProvider? provider = Provider;
            if (provider == null)
            {
                return false;
            }
            if (string.IsNullOrEmpty(buttonName))
            {
                Debug.LogError("Input Button '' is not setup.");
                return false;
            }
            if (!provider.TryGetButtonUp(buttonName, out bool value))
            {
                Debug.LogError("Input Button " + buttonName + " is not setup.");
                return false;
            }
            return value;
        }

        public static bool TryParseKeyName(string name, out KeyCode key)
        {
            key = KeyCode.None;
            if (string.IsNullOrEmpty(name))
            {
                return false;
            }

            if (name.Length == 1)
            {
                char ch = name[0];
                if (ch >= 'a' && ch <= 'z')
                {
                    key = (KeyCode)((int)KeyCode.A + (ch - 'a'));
                    return true;
                }
                if (ch >= 'A' && ch <= 'Z')
                {
                    key = (KeyCode)((int)KeyCode.A + (ch - 'A'));
                    return true;
                }
                if (ch >= '0' && ch <= '9')
                {
                    key = (KeyCode)((int)KeyCode.Alpha0 + (ch - '0'));
                    return true;
                }
                return false;
            }

            if (string.Equals(name, "space", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Space;
                return true;
            }
            if (string.Equals(name, "return", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Return;
                return true;
            }
            if (string.Equals(name, "escape", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Escape;
                return true;
            }
            if (string.Equals(name, "tab", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Tab;
                return true;
            }
            if (string.Equals(name, "backspace", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Backspace;
                return true;
            }
            if (string.Equals(name, "delete", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Delete;
                return true;
            }
            if (string.Equals(name, "up", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.UpArrow;
                return true;
            }
            if (string.Equals(name, "down", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.DownArrow;
                return true;
            }
            if (string.Equals(name, "left", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.LeftArrow;
                return true;
            }
            if (string.Equals(name, "right", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.RightArrow;
                return true;
            }
            if (string.Equals(name, "left shift", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.LeftShift;
                return true;
            }
            if (string.Equals(name, "right shift", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.RightShift;
                return true;
            }
            if (string.Equals(name, "left ctrl", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.LeftControl;
                return true;
            }
            if (string.Equals(name, "right ctrl", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.RightControl;
                return true;
            }
            if (string.Equals(name, "left alt", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.LeftAlt;
                return true;
            }
            if (string.Equals(name, "right alt", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.RightAlt;
                return true;
            }
            if (string.Equals(name, "mouse 0", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Mouse0;
                return true;
            }
            if (string.Equals(name, "mouse 1", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Mouse1;
                return true;
            }
            if (string.Equals(name, "mouse 2", StringComparison.OrdinalIgnoreCase))
            {
                key = KeyCode.Mouse2;
                return true;
            }

            return false;
        }
    }
}
