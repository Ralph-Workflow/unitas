using System;

namespace UnityEngine.InputSystem
{
    public sealed class Gamepad : InputDevice
    {
        public const uint ButtonSouthId = 0;
        public const uint ButtonEastId = 1;
        public const uint ButtonWestId = 2;
        public const uint ButtonNorthId = 3;
        public const uint ButtonLeftShoulderId = 4;
        public const uint ButtonRightShoulderId = 5;
        public const uint ButtonSelectId = 6;
        public const uint ButtonStartId = 7;
        public const uint ButtonLeftStickId = 8;
        public const uint ButtonRightStickId = 9;
        public const uint ButtonLeftTriggerId = 10;
        public const uint ButtonRightTriggerId = 11;
        public const uint ButtonDpadUpId = 12;
        public const uint ButtonDpadDownId = 13;
        public const uint ButtonDpadLeftId = 14;
        public const uint ButtonDpadRightId = 15;

        public const uint AxisLeftXId = 0;
        public const uint AxisLeftYId = 1;
        public const uint AxisRightXId = 2;
        public const uint AxisRightYId = 3;
        public const uint AxisLeftTriggerId = 4;
        public const uint AxisRightTriggerId = 5;

        private static Gamepad _defaultInstance = new Gamepad();
        private static Gamepad? _customCurrent;

        public Gamepad()
            : base("Gamepad", "/Gamepad")
        {
            buttonSouth = RegisterGamepadButton("buttonSouth", ButtonSouthId, "aButton", "crossButton");
            buttonEast = RegisterGamepadButton("buttonEast", ButtonEastId, "bButton", "circleButton");
            buttonWest = RegisterGamepadButton("buttonWest", ButtonWestId, "xButton", "squareButton");
            buttonNorth = RegisterGamepadButton("buttonNorth", ButtonNorthId, "yButton", "triangleButton");

            leftShoulder = RegisterGamepadButton("leftShoulder", ButtonLeftShoulderId);
            rightShoulder = RegisterGamepadButton("rightShoulder", ButtonRightShoulderId);
            selectButton = RegisterGamepadButton("selectButton", ButtonSelectId, "select");
            startButton = RegisterGamepadButton("startButton", ButtonStartId, "start");
            leftStickButton = RegisterGamepadButton("leftStickButton", ButtonLeftStickId, "leftStickPress");
            rightStickButton = RegisterGamepadButton("rightStickButton", ButtonRightStickId, "rightStickPress");

            leftTrigger = RegisterTrigger("leftTrigger", ButtonLeftTriggerId, AxisLeftTriggerId);
            rightTrigger = RegisterTrigger("rightTrigger", ButtonRightTriggerId, AxisRightTriggerId);

            ButtonControl dpadUp = CreateDpadButton("up", "/Gamepad/dpad/up", ButtonDpadUpId);
            ButtonControl dpadDown = CreateDpadButton("down", "/Gamepad/dpad/down", ButtonDpadDownId);
            ButtonControl dpadLeft = CreateDpadButton("left", "/Gamepad/dpad/left", ButtonDpadLeftId);
            ButtonControl dpadRight = CreateDpadButton("right", "/Gamepad/dpad/right", ButtonDpadRightId);
            dpad = RegisterControl(new DpadControl("dpad", "/Gamepad/dpad", this, dpadUp, dpadDown, dpadLeft, dpadRight));

            leftStick = RegisterControl(
                new StickControl(
                    "leftStick",
                    "/Gamepad/leftStick",
                    this,
                    () => new Vector2(ReadAxis(AxisLeftXId), ReadAxis(AxisLeftYId))));
            rightStick = RegisterControl(
                new StickControl(
                    "rightStick",
                    "/Gamepad/rightStick",
                    this,
                    () => new Vector2(ReadAxis(AxisRightXId), ReadAxis(AxisRightYId))));
        }

        public static Gamepad? current
        {
            get
            {
                if (_customCurrent != null)
                {
                    return _customCurrent;
                }
                IInputProvider? provider = Input.Provider;
                if (provider != null && provider.IsGamepadConnected())
                {
                    return _defaultInstance;
                }
                return null;
            }
            internal set => _customCurrent = value;
        }

        internal static Gamepad DefaultInstance => _defaultInstance;

        public ButtonControl buttonSouth { get; }
        public ButtonControl buttonEast { get; }
        public ButtonControl buttonWest { get; }
        public ButtonControl buttonNorth { get; }

        public ButtonControl aButton => buttonSouth;
        public ButtonControl bButton => buttonEast;
        public ButtonControl xButton => buttonWest;
        public ButtonControl yButton => buttonNorth;

        public ButtonControl leftShoulder { get; }
        public ButtonControl rightShoulder { get; }
        public ButtonControl selectButton { get; }
        public ButtonControl startButton { get; }
        public ButtonControl leftStickButton { get; }
        public ButtonControl rightStickButton { get; }

        public ButtonControl leftTrigger { get; }
        public ButtonControl rightTrigger { get; }

        public DpadControl dpad { get; }
        public StickControl leftStick { get; }
        public StickControl rightStick { get; }

        internal static void ResetDefault()
        {
            _customCurrent = null;
            _defaultInstance = new Gamepad();
        }

        private ButtonControl RegisterGamepadButton(string controlName, uint buttonId, params string[] aliases)
        {
            return RegisterControl(
                new ButtonControl(
                    controlName,
                    "/Gamepad/" + controlName,
                    this,
                    () => ReadButton(buttonId),
                    () => ReadButtonDown(buttonId),
                    () => ReadButtonUp(buttonId)),
                aliases);
        }

        private ButtonControl RegisterTrigger(string controlName, uint buttonId, uint axisId)
        {
            return RegisterControl(
                new ButtonControl(
                    controlName,
                    "/Gamepad/" + controlName,
                    this,
                    () => ReadButton(buttonId) || ReadTriggerValue(buttonId, axisId) >= 0.5f,
                    () => ReadButtonDown(buttonId),
                    () => ReadButtonUp(buttonId),
                    () => ReadTriggerValue(buttonId, axisId)));
        }

        private ButtonControl CreateDpadButton(string controlName, string controlPath, uint buttonId)
        {
            return new ButtonControl(
                controlName,
                controlPath,
                this,
                () => ReadButton(buttonId),
                () => ReadButtonDown(buttonId),
                () => ReadButtonUp(buttonId));
        }

        private static bool ReadButton(uint buttonId)
        {
            IInputProvider? provider = Input.Provider;
            return provider != null && provider.GetGamepadButton(buttonId);
        }

        private static bool ReadButtonDown(uint buttonId)
        {
            IInputProvider? provider = Input.Provider;
            return provider != null && provider.GetGamepadButtonDown(buttonId);
        }

        private static bool ReadButtonUp(uint buttonId)
        {
            IInputProvider? provider = Input.Provider;
            return provider != null && provider.GetGamepadButtonUp(buttonId);
        }

        private static float ReadAxis(uint axisId)
        {
            IInputProvider? provider = Input.Provider;
            return provider != null ? provider.GetGamepadAxis(axisId) : 0f;
        }

        private static float ReadTriggerValue(uint buttonId, uint axisId)
        {
            float axis = MathF.Max(0f, ReadAxis(axisId));
            if (axis > 0f)
            {
                return axis;
            }
            return ReadButton(buttonId) ? 1f : 0f;
        }
    }
}
