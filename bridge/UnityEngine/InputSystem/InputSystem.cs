using System;
using System.Collections.Generic;

namespace UnityEngine.InputSystem
{
    public static class InputSystem
    {
        private static readonly List<InputDevice> AddedDevices = new List<InputDevice>();

        internal static event Action? OnBeforeUpdate;

        internal static event Action? OnUpdate;

        public static IReadOnlyList<InputDevice> devices
        {
            get
            {
                List<InputDevice> list = new List<InputDevice>(3 + AddedDevices.Count);
                Keyboard? kb = Keyboard.current;
                if (kb != null)
                {
                    list.Add(kb);
                }
                Mouse? mouse = Mouse.current;
                if (mouse != null)
                {
                    list.Add(mouse);
                }
                Gamepad? gamepad = Gamepad.current;
                if (gamepad != null && !list.Contains(gamepad))
                {
                    list.Add(gamepad);
                }
                for (int i = 0; i < AddedDevices.Count; i++)
                {
                    InputDevice added = AddedDevices[i];
                    if (!list.Contains(added))
                    {
                        list.Add(added);
                    }
                }
                return list;
            }
        }

        public static void Update()
        {
            OnBeforeUpdate?.Invoke();
            InputAction.UpdateEnabledActions();
            OnUpdate?.Invoke();
        }

        public static InputControl? FindControl(string path)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                return null;
            }

            string trimmed = path.Trim();
            if (!TrySplitDeviceAndControlPath(trimmed, out string deviceToken, out string controlPath))
            {
                return null;
            }

            InputDevice? targetDevice = ResolveDevice(deviceToken);
            if (targetDevice == null)
            {
                return null;
            }

            if (string.IsNullOrEmpty(controlPath))
            {
                return targetDevice;
            }

            return targetDevice.TryGetControl(controlPath);
        }

        internal static void AddDevice(InputDevice device)
        {
            if (device == null)
            {
                throw new ArgumentNullException(nameof(device));
            }
            if (!AddedDevices.Contains(device))
            {
                AddedDevices.Add(device);
            }
            if (device is Keyboard kb)
            {
                Keyboard.current = kb;
            }
            else if (device is Mouse mouse)
            {
                Mouse.current = mouse;
            }
            else if (device is Gamepad gamepad)
            {
                Gamepad.current = gamepad;
            }
        }

        internal static void RemoveDevice(InputDevice device)
        {
            if (device == null)
            {
                return;
            }
            AddedDevices.Remove(device);
            if (ReferenceEquals(Keyboard.current, device))
            {
                Keyboard.current = null;
            }
            else if (ReferenceEquals(Mouse.current, device))
            {
                Mouse.current = null;
            }
            else if (ReferenceEquals(Gamepad.current, device))
            {
                Gamepad.current = null;
            }
        }

        internal static void Reset()
        {
            InputAction.ResetRegistry();
            AddedDevices.Clear();
            Keyboard.ResetDefault();
            Mouse.ResetDefault();
            Gamepad.ResetDefault();
        }

        private static bool TrySplitDeviceAndControlPath(
            string path,
            out string deviceToken,
            out string controlPath)
        {
            deviceToken = string.Empty;
            controlPath = string.Empty;

            if (path.StartsWith('<'))
            {
                int closeBracket = path.IndexOf('>', StringComparison.Ordinal);
                if (closeBracket <= 1)
                {
                    return false;
                }
                deviceToken = path.Substring(1, closeBracket - 1).Trim();
                controlPath = path.Substring(closeBracket + 1).TrimStart('/');
                return deviceToken.Length > 0;
            }

            string stripped = path.TrimStart('/');
            if (stripped.Length == 0)
            {
                return false;
            }

            int slash = stripped.IndexOf('/', StringComparison.Ordinal);
            if (slash < 0)
            {
                deviceToken = stripped;
                return true;
            }

            deviceToken = stripped.Substring(0, slash).Trim();
            controlPath = stripped.Substring(slash + 1).TrimStart('/');
            return deviceToken.Length > 0;
        }

        private static InputDevice? ResolveDevice(string deviceToken)
        {
            for (int i = AddedDevices.Count - 1; i >= 0; i--)
            {
                if (string.Equals(AddedDevices[i].name, deviceToken, StringComparison.OrdinalIgnoreCase))
                {
                    return AddedDevices[i];
                }
            }

            if (string.Equals(deviceToken, "Keyboard", StringComparison.OrdinalIgnoreCase))
            {
                return Keyboard.current ?? Keyboard.DefaultInstance;
            }

            if (string.Equals(deviceToken, "Mouse", StringComparison.OrdinalIgnoreCase)
                || string.Equals(deviceToken, "Pointer", StringComparison.OrdinalIgnoreCase))
            {
                return Mouse.current ?? Mouse.DefaultInstance;
            }

            if (string.Equals(deviceToken, "Gamepad", StringComparison.OrdinalIgnoreCase))
            {
                return Gamepad.current ?? Gamepad.DefaultInstance;
            }

            return null;
        }
    }
}
