using System;

namespace UnityEngine.InputSystem
{
    public abstract class InputControl
    {
        protected InputControl(string name, string path, InputDevice? device)
        {
            this.name = name ?? string.Empty;
            this.path = path ?? string.Empty;
            this.device = device;
        }

        public string name { get; }

        public string path { get; }

        public InputDevice? device { get; internal set; }

        public abstract bool isPressed { get; }

        public abstract bool wasPressedThisFrame { get; }

        public abstract bool wasReleasedThisFrame { get; }

        public virtual float EvaluateMagnitude()
        {
            return isPressed ? 1f : 0f;
        }

        public virtual InputControl? TryGetChildControl(string childName)
        {
            return null;
        }
    }

    public abstract class InputControl<TValue> : InputControl where TValue : struct
    {
        protected InputControl(string name, string path, InputDevice? device)
            : base(name, path, device)
        {
        }

        public abstract TValue ReadValue();
    }

    public class ButtonControl : InputControl<float>
    {
        private readonly Func<bool> _isPressed;
        private readonly Func<bool> _wasPressed;
        private readonly Func<bool> _wasReleased;
        private readonly Func<float>? _readValue;

        public ButtonControl(
            string name,
            string path,
            InputDevice? device,
            Func<bool> isPressed,
            Func<bool> wasPressed,
            Func<bool> wasReleased,
            Func<float>? readValue = null)
            : base(name, path, device)
        {
            _isPressed = isPressed;
            _wasPressed = wasPressed;
            _wasReleased = wasReleased;
            _readValue = readValue;
        }

        public float pressPoint { get; set; } = 0.5f;

        public override bool isPressed => _isPressed();

        public override bool wasPressedThisFrame => _wasPressed();

        public override bool wasReleasedThisFrame => _wasReleased();

        public override float ReadValue()
        {
            if (_readValue != null)
            {
                return _readValue();
            }
            return isPressed ? 1f : 0f;
        }

        public override float EvaluateMagnitude()
        {
            return MathF.Abs(ReadValue());
        }
    }

    public sealed class KeyControl : ButtonControl
    {
        public KeyControl(string name, string path, InputDevice? device, Key key, KeyCode keyCode)
            : base(
                name,
                path,
                device,
                () => Input.GetKey(keyCode),
                () => Input.GetKeyDown(keyCode),
                () => Input.GetKeyUp(keyCode))
        {
            this.keyCode = key;
            unityKeyCode = keyCode;
        }

        public Key keyCode { get; }

        internal KeyCode unityKeyCode { get; }
    }

    public class AxisControl : InputControl<float>
    {
        private readonly Func<float> _readValue;
        private readonly Func<bool>? _wasPressed;
        private readonly Func<bool>? _wasReleased;

        public AxisControl(
            string name,
            string path,
            InputDevice? device,
            Func<float> readValue,
            Func<bool>? wasPressed = null,
            Func<bool>? wasReleased = null)
            : base(name, path, device)
        {
            _readValue = readValue;
            _wasPressed = wasPressed;
            _wasReleased = wasReleased;
        }

        public float pressPoint { get; set; } = 0.5f;

        public override bool isPressed => MathF.Abs(ReadValue()) >= pressPoint;

        public override bool wasPressedThisFrame => _wasPressed != null && _wasPressed();

        public override bool wasReleasedThisFrame => _wasReleased != null && _wasReleased();

        public override float ReadValue()
        {
            return _readValue();
        }

        public override float EvaluateMagnitude()
        {
            return MathF.Abs(ReadValue());
        }
    }

    public class Vector2Control : InputControl<Vector2>
    {
        private readonly Func<Vector2> _readValue;

        public Vector2Control(string name, string path, InputDevice? device, Func<Vector2> readValue)
            : base(name, path, device)
        {
            _readValue = readValue;
            x = new AxisControl("x", path + "/x", device, () => _readValue().x);
            y = new AxisControl("y", path + "/y", device, () => _readValue().y);
        }

        public AxisControl x { get; }

        public AxisControl y { get; }

        public override bool isPressed => EvaluateMagnitude() >= 0.5f;

        public override bool wasPressedThisFrame => x.wasPressedThisFrame || y.wasPressedThisFrame;

        public override bool wasReleasedThisFrame => x.wasReleasedThisFrame || y.wasReleasedThisFrame;

        public override Vector2 ReadValue()
        {
            return _readValue();
        }

        public override float EvaluateMagnitude()
        {
            return ReadValue().magnitude;
        }

        public override InputControl? TryGetChildControl(string childName)
        {
            if (string.Equals(childName, "x", StringComparison.OrdinalIgnoreCase))
            {
                return x;
            }
            if (string.Equals(childName, "y", StringComparison.OrdinalIgnoreCase))
            {
                return y;
            }
            return null;
        }
    }

    public class StickControl : Vector2Control
    {
        public StickControl(string name, string path, InputDevice? device, Func<Vector2> readValue)
            : base(name, path, device, readValue)
        {
            up = CreateDirectionButton("up", path + "/up", device, () => MathF.Max(0f, ReadValue().y));
            down = CreateDirectionButton("down", path + "/down", device, () => MathF.Max(0f, -ReadValue().y));
            left = CreateDirectionButton("left", path + "/left", device, () => MathF.Max(0f, -ReadValue().x));
            right = CreateDirectionButton("right", path + "/right", device, () => MathF.Max(0f, ReadValue().x));
        }

        public ButtonControl up { get; }

        public ButtonControl down { get; }

        public ButtonControl left { get; }

        public ButtonControl right { get; }

        public override InputControl? TryGetChildControl(string childName)
        {
            InputControl? axisChild = base.TryGetChildControl(childName);
            if (axisChild != null)
            {
                return axisChild;
            }
            if (string.Equals(childName, "up", StringComparison.OrdinalIgnoreCase))
            {
                return up;
            }
            if (string.Equals(childName, "down", StringComparison.OrdinalIgnoreCase))
            {
                return down;
            }
            if (string.Equals(childName, "left", StringComparison.OrdinalIgnoreCase))
            {
                return left;
            }
            if (string.Equals(childName, "right", StringComparison.OrdinalIgnoreCase))
            {
                return right;
            }
            return null;
        }

        private static ButtonControl CreateDirectionButton(
            string dirName,
            string dirPath,
            InputDevice? device,
            Func<float> readComponent)
        {
            return new ButtonControl(
                dirName,
                dirPath,
                device,
                () => readComponent() >= 0.5f,
                () => false,
                () => false,
                readComponent);
        }
    }

    public sealed class DpadControl : Vector2Control
    {
        public DpadControl(
            string name,
            string path,
            InputDevice? device,
            ButtonControl up,
            ButtonControl down,
            ButtonControl left,
            ButtonControl right)
            : base(name, path, device, () => CombineDpad(up, down, left, right))
        {
            this.up = up;
            this.down = down;
            this.left = left;
            this.right = right;
        }

        public ButtonControl up { get; }

        public ButtonControl down { get; }

        public ButtonControl left { get; }

        public ButtonControl right { get; }

        public override bool isPressed => up.isPressed || down.isPressed || left.isPressed || right.isPressed;

        public override bool wasPressedThisFrame =>
            up.wasPressedThisFrame || down.wasPressedThisFrame || left.wasPressedThisFrame || right.wasPressedThisFrame;

        public override bool wasReleasedThisFrame =>
            up.wasReleasedThisFrame || down.wasReleasedThisFrame || left.wasReleasedThisFrame || right.wasReleasedThisFrame;

        public override InputControl? TryGetChildControl(string childName)
        {
            InputControl? axisChild = base.TryGetChildControl(childName);
            if (axisChild != null)
            {
                return axisChild;
            }
            if (string.Equals(childName, "up", StringComparison.OrdinalIgnoreCase))
            {
                return up;
            }
            if (string.Equals(childName, "down", StringComparison.OrdinalIgnoreCase))
            {
                return down;
            }
            if (string.Equals(childName, "left", StringComparison.OrdinalIgnoreCase))
            {
                return left;
            }
            if (string.Equals(childName, "right", StringComparison.OrdinalIgnoreCase))
            {
                return right;
            }
            return null;
        }

        private static Vector2 CombineDpad(
            ButtonControl up,
            ButtonControl down,
            ButtonControl left,
            ButtonControl right)
        {
            float horizontal = (right.isPressed ? 1f : 0f) - (left.isPressed ? 1f : 0f);
            float vertical = (up.isPressed ? 1f : 0f) - (down.isPressed ? 1f : 0f);
            return new Vector2(horizontal, vertical);
        }
    }
}
