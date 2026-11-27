namespace UnityEngine.InputSystem
{
    public sealed class Mouse : InputDevice
    {
        private static Mouse _defaultInstance = new Mouse();
        private static Mouse? _customCurrent;

        public Mouse()
            : base("Mouse", "/Mouse")
        {
            leftButton = RegisterControl(
                new ButtonControl(
                    "leftButton",
                    "/Mouse/leftButton",
                    this,
                    () => Input.GetMouseButton(0),
                    () => Input.GetMouseButtonDown(0),
                    () => Input.GetMouseButtonUp(0)),
                "press");
            rightButton = RegisterControl(
                new ButtonControl(
                    "rightButton",
                    "/Mouse/rightButton",
                    this,
                    () => Input.GetMouseButton(1),
                    () => Input.GetMouseButtonDown(1),
                    () => Input.GetMouseButtonUp(1)));
            middleButton = RegisterControl(
                new ButtonControl(
                    "middleButton",
                    "/Mouse/middleButton",
                    this,
                    () => Input.GetMouseButton(2),
                    () => Input.GetMouseButtonDown(2),
                    () => Input.GetMouseButtonUp(2)));
            position = RegisterControl(
                new Vector2Control(
                    "position",
                    "/Mouse/position",
                    this,
                    () =>
                    {
                        Vector3 pos = Input.mousePosition;
                        return new Vector2(pos.x, pos.y);
                    }));
            delta = RegisterControl(
                new Vector2Control(
                    "delta",
                    "/Mouse/delta",
                    this,
                    () =>
                    {
                        Vector3 d = Input.mousePositionDelta;
                        return new Vector2(d.x, d.y);
                    }));
            scroll = RegisterControl(
                new Vector2Control(
                    "scroll",
                    "/Mouse/scroll",
                    this,
                    () =>
                    {
                        Vector3 s = Input.mouseScrollDelta;
                        return new Vector2(s.x, s.y);
                    }));
        }

        public static Mouse? current
        {
            get => _customCurrent ?? _defaultInstance;
            internal set => _customCurrent = value;
        }

        internal static Mouse DefaultInstance => _defaultInstance;

        public ButtonControl leftButton { get; }

        public ButtonControl rightButton { get; }

        public ButtonControl middleButton { get; }

        public Vector2Control position { get; }

        public Vector2Control delta { get; }

        public Vector2Control scroll { get; }

        internal static void ResetDefault()
        {
            _customCurrent = null;
            _defaultInstance = new Mouse();
        }
    }
}
