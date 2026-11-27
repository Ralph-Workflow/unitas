using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.CompilerServices;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Unitas.Bridge.Tests
{
    public static class InputSystemDeviceTests
    {
        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestInertDefaultsWithoutProvider();
            TestKeyboardControlReadsTransitionsAndPathResolution();
            TestMouseControlReadsButtonsPositionDeltaAndScroll();
            TestGamepadConnectionButtonsTriggersSticksAndDpad();
            TestNativeGamepadInteropThroughBridgeHost();
            TestInputSystemFindControlAndUpdateHookInBridgeHostTick();
        }

        private static void TestInertDefaultsWithoutProvider()
        {
            IInputProvider? savedProvider = Input.Provider;
            Input.Provider = null;
            InputSystem.Reset();
            try
            {
                Keyboard? keyboard = Keyboard.current;
                Mouse? mouse = Mouse.current;
                Check.That("Keyboard.current is available without provider", keyboard != null);
                Check.That("Mouse.current is available without provider", mouse != null);
                Check.That("Gamepad.current is null without provider", Gamepad.current == null);

                Check.Equal("Inert spaceKey isPressed", false, keyboard!.spaceKey.isPressed);
                Check.Equal("Inert spaceKey wasPressedThisFrame", false, keyboard.spaceKey.wasPressedThisFrame);
                Check.Equal("Inert spaceKey wasReleasedThisFrame", false, keyboard.spaceKey.wasReleasedThisFrame);
                Check.Equal("Inert spaceKey ReadValue", 0f, keyboard.spaceKey.ReadValue());
                Check.Equal("Inert anyKey isPressed", false, keyboard.anyKey.isPressed);

                Check.Equal("Inert leftButton isPressed", false, mouse!.leftButton.isPressed);
                Check.Equal("Inert position x", 0f, mouse.position.ReadValue().x);
                Check.Equal("Inert delta y", 0f, mouse.delta.ReadValue().y);
                Check.Equal("Inert scroll y", 0f, mouse.scroll.ReadValue().y);
                Check.Equal("InputSystem.devices has Keyboard and Mouse when disconnected", 2, InputSystem.devices.Count);
            }
            finally
            {
                Input.Provider = savedProvider;
            }
        }

        private static void TestKeyboardControlReadsTransitionsAndPathResolution()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Keyboard keyboard = Keyboard.current!;
                Check.That("Keyboard device reference on spaceKey matches keyboard", ReferenceEquals(keyboard.spaceKey.device, keyboard));
                Check.That("Keyboard indexer Key.Space returns spaceKey", ReferenceEquals(keyboard[Key.Space], keyboard.spaceKey));
                Check.That("Keyboard TryGetControl space matches spaceKey", ReferenceEquals(keyboard.TryGetControl("space"), keyboard.spaceKey));
                Check.That("Keyboard TryGetControl spaceKey matches spaceKey", ReferenceEquals(keyboard.TryGetControl("spaceKey"), keyboard.spaceKey));
                Check.That("Keyboard TryGetControl w matches wKey", ReferenceEquals(keyboard.TryGetControl("w"), keyboard.wKey));
                Check.That("Keyboard TryGetControl upArrow matches upArrowKey", ReferenceEquals(keyboard.TryGetControl("upArrow"), keyboard.upArrowKey));
                Check.That("Keyboard TryGetControl leftCtrl matches leftCtrlKey", ReferenceEquals(keyboard.TryGetControl("leftCtrl"), keyboard.leftCtrlKey));

                host.InjectKey(KeyCode.Space, true);
                host.InjectKey(KeyCode.W, true);
                host.Tick(0.02);

                Check.Equal("Frame 1 spaceKey isPressed", true, keyboard.spaceKey.isPressed);
                Check.Equal("Frame 1 spaceKey wasPressedThisFrame", true, keyboard.spaceKey.wasPressedThisFrame);
                Check.Equal("Frame 1 spaceKey wasReleasedThisFrame", false, keyboard.spaceKey.wasReleasedThisFrame);
                Check.Equal("Frame 1 spaceKey ReadValue", 1f, keyboard.spaceKey.ReadValue());
                Check.Equal("Frame 1 spaceKey EvaluateMagnitude", 1f, keyboard.spaceKey.EvaluateMagnitude());
                Check.Equal("Frame 1 wKey isPressed", true, keyboard.wKey.isPressed);
                Check.Equal("Frame 1 anyKey isPressed", true, keyboard.anyKey.isPressed);
                Check.Equal("Frame 1 anyKey wasPressedThisFrame", true, keyboard.anyKey.wasPressedThisFrame);

                host.Tick(0.02);

                Check.Equal("Frame 2 spaceKey stays isPressed", true, keyboard.spaceKey.isPressed);
                Check.Equal("Frame 2 spaceKey wasPressedThisFrame clears", false, keyboard.spaceKey.wasPressedThisFrame);
                Check.Equal("Frame 2 spaceKey wasReleasedThisFrame false", false, keyboard.spaceKey.wasReleasedThisFrame);

                host.InjectKey(KeyCode.Space, false);
                host.InjectKey(KeyCode.W, false);
                host.Tick(0.02);

                Check.Equal("Frame 3 spaceKey isPressed false", false, keyboard.spaceKey.isPressed);
                Check.Equal("Frame 3 spaceKey wasPressedThisFrame false", false, keyboard.spaceKey.wasPressedThisFrame);
                Check.Equal("Frame 3 spaceKey wasReleasedThisFrame true", true, keyboard.spaceKey.wasReleasedThisFrame);
                Check.Equal("Frame 3 spaceKey ReadValue 0", 0f, keyboard.spaceKey.ReadValue());
                Check.Equal("Frame 3 anyKey wasReleasedThisFrame true", true, keyboard.anyKey.wasReleasedThisFrame);

                host.Tick(0.02);
                Check.Equal("Frame 4 spaceKey wasReleasedThisFrame clears", false, keyboard.spaceKey.wasReleasedThisFrame);
            }
        }

        private static void TestMouseControlReadsButtonsPositionDeltaAndScroll()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Mouse mouse = Mouse.current!;
                host.InjectMouseButton(0, true);
                host.InjectMouseButton(1, true);
                host.InjectPointerPosition(64.0, 128.0);
                host.InjectScroll(1.25, -2.5);
                host.Tick(0.02);

                Check.Equal("Mouse leftButton isPressed", true, mouse.leftButton.isPressed);
                Check.Equal("Mouse leftButton wasPressedThisFrame", true, mouse.leftButton.wasPressedThisFrame);
                Check.Equal("Mouse leftButton ReadValue", 1f, mouse.leftButton.ReadValue());
                Check.Equal("Mouse rightButton isPressed", true, mouse.rightButton.isPressed);
                Check.Equal("Mouse middleButton isPressed", false, mouse.middleButton.isPressed);

                Vector2 pos = mouse.position.ReadValue();
                Vector2 delta = mouse.delta.ReadValue();
                Vector2 scroll = mouse.scroll.ReadValue();
                Check.Equal("Mouse position x", 64f, pos.x);
                Check.Equal("Mouse position y", 128f, pos.y);
                Check.Equal("Mouse position.x axis ReadValue", 64f, mouse.position.x.ReadValue());
                Check.Equal("Mouse delta x", 64f, delta.x);
                Check.Equal("Mouse delta y", 128f, delta.y);
                Check.Equal("Mouse scroll x", 1.25f, scroll.x);
                Check.Equal("Mouse scroll y", -2.5f, scroll.y);

                host.Tick(0.02);
                Check.Equal("Mouse leftButton wasPressedThisFrame clears", false, mouse.leftButton.wasPressedThisFrame);
                Check.Equal("Mouse delta x clears next frame", 0f, mouse.delta.ReadValue().x);
                Check.Equal("Mouse scroll y clears next frame", 0f, mouse.scroll.ReadValue().y);

                host.InjectMouseButton(0, false);
                host.Tick(0.02);
                Check.Equal("Mouse leftButton wasReleasedThisFrame", true, mouse.leftButton.wasReleasedThisFrame);
                Check.Equal("Mouse leftButton isPressed false on release", false, mouse.leftButton.isPressed);
            }
        }

        private static void TestGamepadConnectionButtonsTriggersSticksAndDpad()
        {
            FakeGamepadInputProvider fake = new FakeGamepadInputProvider();
            IInputProvider? savedProvider = Input.Provider;
            Input.Provider = fake;
            InputSystem.Reset();
            try
            {
                Check.That("Gamepad.current is null when disconnected", Gamepad.current == null);
                Check.Equal("InputSystem.devices count when gamepad disconnected", 2, InputSystem.devices.Count);

                fake.Connected = true;
                Gamepad? gamepad = Gamepad.current;
                Check.That("Gamepad.current is non-null when connected", gamepad != null);
                Check.Equal("InputSystem.devices count when gamepad connected", 3, InputSystem.devices.Count);
                Check.That("Gamepad aButton aliases buttonSouth", ReferenceEquals(gamepad!.aButton, gamepad.buttonSouth));
                Check.That("Gamepad bButton aliases buttonEast", ReferenceEquals(gamepad.bButton, gamepad.buttonEast));
                Check.That("Gamepad xButton aliases buttonWest", ReferenceEquals(gamepad.xButton, gamepad.buttonWest));
                Check.That("Gamepad yButton aliases buttonNorth", ReferenceEquals(gamepad.yButton, gamepad.buttonNorth));

                fake.SetButtonFrame(0, true, true, false);
                fake.SetButtonFrame(12, true, true, false);
                fake.SetButtonFrame(15, true, true, false);
                fake.SetAxis(0, 0.75f);
                fake.SetAxis(1, -0.5f);
                fake.SetAxis(4, 0.8f);

                Check.Equal("Gamepad buttonSouth isPressed", true, gamepad.buttonSouth.isPressed);
                Check.Equal("Gamepad buttonSouth wasPressedThisFrame", true, gamepad.buttonSouth.wasPressedThisFrame);
                Check.Equal("Gamepad buttonSouth ReadValue", 1f, gamepad.buttonSouth.ReadValue());
                Check.Equal("Gamepad leftStick x", 0.75f, gamepad.leftStick.ReadValue().x);
                Check.Equal("Gamepad leftStick y", -0.5f, gamepad.leftStick.ReadValue().y);
                Check.Equal("Gamepad leftStick right button isPressed", true, gamepad.leftStick.right.isPressed);
                Check.Equal("Gamepad leftStick down button isPressed", true, gamepad.leftStick.down.isPressed);
                Check.Equal("Gamepad leftStick up button isPressed", false, gamepad.leftStick.up.isPressed);
                Check.Equal("Gamepad leftTrigger ReadValue", 0.8f, gamepad.leftTrigger.ReadValue());
                Check.Equal("Gamepad leftTrigger isPressed", true, gamepad.leftTrigger.isPressed);
                Check.Equal("Gamepad dpad up isPressed", true, gamepad.dpad.up.isPressed);
                Check.Equal("Gamepad dpad right isPressed", true, gamepad.dpad.right.isPressed);
                Check.Equal("Gamepad dpad ReadValue x", 1f, gamepad.dpad.ReadValue().x);
                Check.Equal("Gamepad dpad ReadValue y", 1f, gamepad.dpad.ReadValue().y);
            }
            finally
            {
                Input.Provider = savedProvider;
                InputSystem.Reset();
            }
        }

        private static void TestNativeGamepadInteropThroughBridgeHost()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                Check.That("Native BridgeHost Gamepad.current starts null", Gamepad.current == null);

                host.InjectGamepadConnected(true);
                Gamepad? gamepad = Gamepad.current;
                Check.That("Native BridgeHost Gamepad.current non-null after InjectGamepadConnected(true)", gamepad != null);

                host.InjectGamepadButton(Gamepad.ButtonSouthId, true);
                host.InjectGamepadButton(Gamepad.ButtonDpadUpId, true);
                host.InjectGamepadButton(Gamepad.ButtonDpadRightId, true);
                host.InjectGamepadAxis(Gamepad.AxisLeftXId, 0.75);
                host.InjectGamepadAxis(Gamepad.AxisLeftYId, -0.5);
                host.InjectGamepadAxis(Gamepad.AxisLeftTriggerId, 0.9);
                host.Tick(0.02);

                Check.Equal("Native Gamepad buttonSouth isPressed frame 1", true, gamepad!.buttonSouth.isPressed);
                Check.Equal("Native Gamepad buttonSouth wasPressedThisFrame frame 1", true, gamepad.buttonSouth.wasPressedThisFrame);
                Check.Equal("Native Gamepad buttonSouth wasReleasedThisFrame frame 1", false, gamepad.buttonSouth.wasReleasedThisFrame);
                Check.Equal("Native Gamepad leftStick x", 0.75f, gamepad.leftStick.ReadValue().x);
                Check.Equal("Native Gamepad leftStick y", -0.5f, gamepad.leftStick.ReadValue().y);
                Check.Equal("Native Gamepad leftTrigger ReadValue", 0.9f, gamepad.leftTrigger.ReadValue());
                Check.Equal("Native Gamepad leftTrigger isPressed", true, gamepad.leftTrigger.isPressed);
                Check.Equal("Native Gamepad dpad ReadValue x", 1f, gamepad.dpad.ReadValue().x);
                Check.Equal("Native Gamepad dpad ReadValue y", 1f, gamepad.dpad.ReadValue().y);

                host.Tick(0.02);
                Check.Equal("Native Gamepad buttonSouth stays held frame 2", true, gamepad.buttonSouth.isPressed);
                Check.Equal("Native Gamepad buttonSouth wasPressedThisFrame clears frame 2", false, gamepad.buttonSouth.wasPressedThisFrame);

                host.InjectGamepadButton(Gamepad.ButtonSouthId, false);
                host.Tick(0.02);
                Check.Equal("Native Gamepad buttonSouth isPressed false frame 3", false, gamepad.buttonSouth.isPressed);
                Check.Equal("Native Gamepad buttonSouth wasReleasedThisFrame true frame 3", true, gamepad.buttonSouth.wasReleasedThisFrame);

                host.Tick(0.02);
                Check.Equal("Native Gamepad buttonSouth wasReleasedThisFrame clears frame 4", false, gamepad.buttonSouth.wasReleasedThisFrame);

                host.InjectGamepadConnected(false);
                Check.That("Native Gamepad.current returns null after disconnect", Gamepad.current == null);
            }
        }

        private static void TestInputSystemFindControlAndUpdateHookInBridgeHostTick()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            InputControl? kbSpace = InputSystem.FindControl("<Keyboard>/space");
            InputControl? kbSpaceKey = InputSystem.FindControl("<Keyboard>/spaceKey");
            InputControl? kbW = InputSystem.FindControl("<Keyboard>/w");
            InputControl? mousePos = InputSystem.FindControl("<Mouse>/position");
            InputControl? mouseLeft = InputSystem.FindControl("<Mouse>/leftButton");
            InputControl? gpSouth = InputSystem.FindControl("<Gamepad>/buttonSouth");
            InputControl? gpStickX = InputSystem.FindControl("<Gamepad>/leftStick/x");
            InputControl? gpDpadUp = InputSystem.FindControl("<Gamepad>/dpad/up");

            Check.That("FindControl <Keyboard>/space resolves spaceKey", ReferenceEquals(kbSpace, Keyboard.current!.spaceKey));
            Check.That("FindControl <Keyboard>/spaceKey resolves spaceKey", ReferenceEquals(kbSpaceKey, Keyboard.current.spaceKey));
            Check.That("FindControl <Keyboard>/w resolves wKey", ReferenceEquals(kbW, Keyboard.current.wKey));
            Check.That("FindControl <Mouse>/position resolves mouse.position", ReferenceEquals(mousePos, Mouse.current!.position));
            Check.That("FindControl <Mouse>/leftButton resolves mouse.leftButton", ReferenceEquals(mouseLeft, Mouse.current.leftButton));
            Check.That("FindControl <Gamepad>/buttonSouth resolves even when disconnected", gpSouth != null);
            Check.That("FindControl <Gamepad>/leftStick/x resolves child axis", gpStickX is AxisControl);
            Check.That("FindControl <Gamepad>/dpad/up resolves child button", gpDpadUp is ButtonControl);

            int updateCallCount = 0;
            bool spaceWasPressedDuringInputUpdate = false;
            bool scriptSawInputUpdateAlreadyRan = false;
            Action onUpdate = () =>
            {
                updateCallCount++;
                spaceWasPressedDuringInputUpdate = Keyboard.current!.spaceKey.wasPressedThisFrame;
            };

            InputSystem.OnUpdate += onUpdate;
            try
            {
                ScriptGlueRegistry.Clear();
                UpdateOrderProbeScript probe = new UpdateOrderProbeScript(() =>
                {
                    scriptSawInputUpdateAlreadyRan = updateCallCount == 2 && Keyboard.current!.spaceKey.wasPressedThisFrame;
                });
                ScriptGlueRegistry.Register(0UL, 1UL, () => probe);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    Check.Equal("InputSystem.OnUpdate ran on first tick", 1, updateCallCount);

                    host.InjectKey(KeyCode.Space, true);
                    host.Tick(0.02);
                    Check.Equal("InputSystem.OnUpdate ran on second tick", 2, updateCallCount);
                    Check.Equal("InputSystem.OnUpdate observed current frame spaceKey press", true, spaceWasPressedDuringInputUpdate);
                    Check.Equal("MonoBehaviour.Update ran after InputSystem.Update", true, scriptSawInputUpdateAlreadyRan);
                }
            }
            finally
            {
                InputSystem.OnUpdate -= onUpdate;
                InputSystem.Reset();
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private sealed class UpdateOrderProbeScript : MonoBehaviour
        {
            private readonly Action _onUpdate;

            public UpdateOrderProbeScript(Action onUpdate)
            {
                _onUpdate = onUpdate;
            }

            public override void Update()
            {
                _onUpdate();
            }
        }

        private sealed class FakeGamepadInputProvider : IInputProvider
        {
            private readonly Dictionary<uint, (bool down, bool pressed, bool released)> _buttons =
                new Dictionary<uint, (bool down, bool pressed, bool released)>();
            private readonly Dictionary<uint, float> _axes = new Dictionary<uint, float>();

            public bool Connected { get; set; }

            public void SetButtonFrame(uint button, bool down, bool pressed, bool released)
            {
                _buttons[button] = (down, pressed, released);
            }

            public void SetAxis(uint axis, float value)
            {
                _axes[axis] = value;
            }

            public bool GetKey(KeyCode key) => false;
            public bool GetKeyDown(KeyCode key) => false;
            public bool GetKeyUp(KeyCode key) => false;
            public bool GetMouseButton(int button) => false;
            public bool GetMouseButtonDown(int button) => false;
            public bool GetMouseButtonUp(int button) => false;
            public Vector3 GetMousePosition() => Vector3.zero;
            public Vector3 GetMousePositionDelta() => Vector3.zero;
            public Vector3 GetMouseScrollDelta() => Vector3.zero;
            public bool TryGetAxis(string axisName, out float value)
            {
                value = 0f;
                return false;
            }

            public bool TryGetAxisRaw(string axisName, out float value)
            {
                value = 0f;
                return false;
            }

            public bool TryGetInjectedAxis(string axisName, out float value)
            {
                value = 0f;
                return false;
            }

            public bool TryGetButton(string buttonName, out bool value)
            {
                value = false;
                return false;
            }

            public bool TryGetButtonDown(string buttonName, out bool value)
            {
                value = false;
                return false;
            }

            public bool TryGetButtonUp(string buttonName, out bool value)
            {
                value = false;
                return false;
            }

            public bool IsGamepadConnected() => Connected;

            public bool GetGamepadButton(uint button) =>
                _buttons.TryGetValue(button, out (bool down, bool pressed, bool released) state) && state.down;

            public bool GetGamepadButtonDown(uint button) =>
                _buttons.TryGetValue(button, out (bool down, bool pressed, bool released) state) && state.pressed;

            public bool GetGamepadButtonUp(uint button) =>
                _buttons.TryGetValue(button, out (bool down, bool pressed, bool released) state) && state.released;

            public float GetGamepadAxis(uint axis) =>
                _axes.TryGetValue(axis, out float value) ? value : 0f;
        }
    }

    internal static class InputSystemDeviceTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsTargetShard(args))
            {
                return;
            }
            InputSystemDeviceTests.Run();
        }

        private static bool IsPerfMode(string[] args)
        {
            for (int i = 0; i < args.Length; i++)
            {
                if (string.Equals(args[i], "--perf", StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool IsTargetShard(string[] args)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (!string.Equals(args[i], "--shard", StringComparison.Ordinal))
                {
                    continue;
                }
                string[] parts = args[i + 1].Split('/');
                return parts.Length == 2 && string.Equals(parts[0], "2", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
