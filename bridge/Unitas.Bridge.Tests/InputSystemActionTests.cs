using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.CompilerServices;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Unitas.Bridge.Tests
{
    public static class InputSystemActionTests
    {
        private static bool s_ran;

        public static void Run()
        {
            if (s_ran)
            {
                return;
            }
            s_ran = true;
            TestButtonActionPhaseSequenceAndTransitions();
            TestInitialStateCheckOnEnableAndMostActuatedControlRule();
            TestCompositeBindings1DAxisAnd2DVector();
            TestActionMapAndAssetLifecycleAndLookup();
            TestPlayerInputSendMessagesAndMapSwitching();
        }

        private static void TestButtonActionPhaseSequenceAndTransitions()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction jump = new InputAction("Jump", InputActionType.Button, "<Keyboard>/space");
                List<string> events = new List<string>();
                jump.started += ctx => events.Add("started:" + ctx.phase);
                jump.performed += ctx => events.Add("performed:" + ctx.phase);
                jump.canceled += ctx => events.Add("canceled:" + ctx.phase);

                Check.That("Jump starts Disabled before Enable", jump.phase == InputActionPhase.Disabled);
                Check.Equal("Jump enabled is false before Enable", false, jump.enabled);

                jump.Enable();
                Check.That("Jump moves to Waiting after Enable", jump.phase == InputActionPhase.Waiting);
                Check.Equal("Jump enabled is true after Enable", true, jump.enabled);
                Check.Equal("No events fired on idle Enable", 0, events.Count);

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                Check.Equal("Press frame event order", "started:Started,performed:Performed", string.Join(",", events));
                Check.That("Press frame phase is Performed", jump.phase == InputActionPhase.Performed);
                Check.Equal("Press frame IsPressed", true, jump.IsPressed());
                Check.Equal("Press frame isPressed property", true, jump.isPressed);
                Check.Equal("Press frame WasPressedThisFrame", true, jump.WasPressedThisFrame());
                Check.Equal("Press frame wasPressedThisFrame property", true, jump.wasPressedThisFrame);
                Check.Equal("Press frame WasPerformedThisFrame", true, jump.WasPerformedThisFrame());
                Check.Equal("Press frame triggered", true, jump.triggered);
                Check.Equal("Press frame WasReleasedThisFrame", false, jump.WasReleasedThisFrame());
                Check.Equal("Press frame ReadValue", 1f, jump.ReadValue<float>());
                Check.That("Press frame activeControl is spaceKey", ReferenceEquals(jump.activeControl, Keyboard.current!.spaceKey));

                events.Clear();
                host.Tick(0.02);

                Check.Equal("Hold frame fires no events", 0, events.Count);
                Check.That("Hold frame phase remains Performed", jump.phase == InputActionPhase.Performed);
                Check.Equal("Hold frame IsPressed", true, jump.IsPressed());
                Check.Equal("Hold frame WasPressedThisFrame false", false, jump.WasPressedThisFrame());
                Check.Equal("Hold frame WasPerformedThisFrame false", false, jump.WasPerformedThisFrame());
                Check.Equal("Hold frame WasReleasedThisFrame false", false, jump.WasReleasedThisFrame());

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);

                Check.Equal("Release frame fires canceled event", "canceled:Canceled", string.Join(",", events));
                Check.Equal("Release frame IsPressed false", false, jump.IsPressed());
                Check.Equal("Release frame WasPressedThisFrame false", false, jump.WasPressedThisFrame());
                Check.Equal("Release frame WasReleasedThisFrame true", true, jump.WasReleasedThisFrame());
                Check.Equal("Release frame wasReleasedThisFrame property true", true, jump.wasReleasedThisFrame);
                Check.Equal("Release frame ReadValue 0", 0f, jump.ReadValue<float>());

                events.Clear();
                host.Tick(0.02);

                Check.Equal("Idle frame after release WasReleasedThisFrame false", false, jump.WasReleasedThisFrame());
                Check.That("Idle frame after release phase is Waiting", jump.phase == InputActionPhase.Waiting);

                jump.Disable();
                Check.That("Disable moves action to Disabled", jump.phase == InputActionPhase.Disabled);
            }
        }

        private static void TestInitialStateCheckOnEnableAndMostActuatedControlRule()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                host.InjectKey(KeyCode.Space, true);
                host.Tick(0.02);

                InputAction preHeldAction = new InputAction("PreHeld", InputActionType.Button, "<Keyboard>/space");
                List<InputActionPhase> initialPhases = new List<InputActionPhase>();
                preHeldAction.started += ctx => initialPhases.Add(ctx.phase);
                preHeldAction.performed += ctx => initialPhases.Add(ctx.phase);

                preHeldAction.Enable();
                Check.Equal("Initial-state check fires Started and Performed on Enable", 2, initialPhases.Count);
                Check.That("Initial-state check first phase is Started", initialPhases.Count >= 1 && initialPhases[0] == InputActionPhase.Started);
                Check.That("Initial-state check second phase is Performed", initialPhases.Count >= 2 && initialPhases[1] == InputActionPhase.Performed);
                Check.That("Initial-state check leaves action in Performed", preHeldAction.phase == InputActionPhase.Performed);
                Check.Equal("Initial-state check IsPressed is true", true, preHeldAction.IsPressed());
                Check.Equal("Initial-state check ReadValue is 1", 1f, preHeldAction.ReadValue<float>());

                host.InjectKey(KeyCode.Space, false);
                host.Tick(0.02);
                preHeldAction.Disable();

                host.InjectGamepadConnected(true);
                InputAction throttle = new InputAction("Throttle", InputActionType.Value);
                throttle.AddBinding("<Gamepad>/leftStick/x");
                throttle.AddBinding("<Keyboard>/d");
                throttle.Enable();

                host.InjectGamepadAxis(Gamepad.AxisLeftXId, 0.4);
                host.Tick(0.02);
                Check.Equal("Gamepad-only actuation reads 0.4", 0.4f, throttle.ReadValue<float>());
                Check.That("Active control is gamepad leftStick.x", ReferenceEquals(throttle.activeControl, Gamepad.current!.leftStick.x));

                host.InjectKey(KeyCode.D, true);
                host.Tick(0.02);
                Check.Equal("Keyboard dKey (1.0) overrides gamepad (0.4)", 1f, throttle.ReadValue<float>());
                Check.That("Active control switches to Keyboard dKey", ReferenceEquals(throttle.activeControl, Keyboard.current!.dKey));

                host.InjectKey(KeyCode.D, false);
                host.InjectGamepadAxis(Gamepad.AxisLeftXId, -0.75);
                host.Tick(0.02);
                Check.Equal("Releasing key falls back to gamepad -0.75", -0.75f, throttle.ReadValue<float>());
                Check.That("Active control returns to gamepad leftStick.x", ReferenceEquals(throttle.activeControl, Gamepad.current.leftStick.x));

                host.InjectGamepadAxis(Gamepad.AxisLeftXId, 0.0);
                host.Tick(0.02);
                Check.Equal("Zero actuation returns ReadValue to 0", 0f, throttle.ReadValue<float>());
                Check.That("Zero actuation returns phase to Waiting", throttle.phase == InputActionPhase.Waiting);
                throttle.Disable();
            }
        }

        private static void TestCompositeBindings1DAxisAnd2DVector()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputAction strafe = new InputAction("Strafe", InputActionType.Value);
                strafe.AddCompositeBinding("composite:1DAxis")
                    .With("Negative", "<Keyboard>/a")
                    .With("Positive", "<Keyboard>/d");
                Check.Equal("Composite binding header isComposite is true", true, strafe.bindings[0].isComposite);
                Check.Equal("Composite binding header strips composite: prefix", "1DAxis", strafe.bindings[0].path);
                Check.Equal("Composite binding part isPartOfComposite is true", true, strafe.bindings[1].isPartOfComposite);
                strafe.Enable();

                host.InjectKey(KeyCode.A, true);
                host.Tick(0.02);
                Check.Equal("1DAxis negative key reads -1", -1f, strafe.ReadValue<float>());

                host.InjectKey(KeyCode.A, false);
                host.InjectKey(KeyCode.D, true);
                host.Tick(0.02);
                Check.Equal("1DAxis positive key reads +1", 1f, strafe.ReadValue<float>());

                host.InjectKey(KeyCode.D, false);
                host.Tick(0.02);
                Check.Equal("1DAxis released reads 0", 0f, strafe.ReadValue<float>());
                strafe.Disable();

                host.InjectGamepadConnected(true);
                InputAction move = new InputAction("Move", InputActionType.Value);
                move.AddCompositeBinding("2DVector")
                    .With("Up", "<Keyboard>/w")
                    .With("Down", "<Keyboard>/s")
                    .With("Left", "<Keyboard>/a")
                    .With("Right", "<Keyboard>/d");
                move.AddBinding("<Gamepad>/leftStick");
                List<InputActionPhase> movePhases = new List<InputActionPhase>();
                move.started += ctx => movePhases.Add(ctx.phase);
                move.performed += ctx => movePhases.Add(ctx.phase);
                move.canceled += ctx => movePhases.Add(ctx.phase);
                move.Enable();

                host.InjectKey(KeyCode.W, true);
                host.Tick(0.02);
                Vector2 upVec = move.ReadValue<Vector2>();
                Check.Equal("2DVector W x", 0f, upVec.x);
                Check.Equal("2DVector W y", 1f, upVec.y);
                Check.That("2DVector W phase is Performed", move.phase == InputActionPhase.Performed);

                host.InjectKey(KeyCode.D, true);
                host.Tick(0.02);
                Vector2 diagVec = move.ReadValue<Vector2>();
                Check.Equal("2DVector W+D normalized x", 0.70710677f, diagVec.x);
                Check.Equal("2DVector W+D normalized y", 0.70710677f, diagVec.y);

                host.InjectKey(KeyCode.W, false);
                host.InjectKey(KeyCode.D, false);
                host.InjectGamepadAxis(Gamepad.AxisLeftXId, -0.5);
                host.InjectGamepadAxis(Gamepad.AxisLeftYId, 0.25);
                host.Tick(0.02);
                Vector2 stickVec = move.ReadValue<Vector2>();
                Check.Equal("2DVector falls back to leftStick x", -0.5f, stickVec.x);
                Check.Equal("2DVector falls back to leftStick y", 0.25f, stickVec.y);

                movePhases.Clear();
                host.InjectGamepadAxis(Gamepad.AxisLeftXId, 0.0);
                host.InjectGamepadAxis(Gamepad.AxisLeftYId, 0.0);
                host.Tick(0.02);
                Vector2 zeroVec = move.ReadValue<Vector2>();
                Check.Equal("2DVector released x", 0f, zeroVec.x);
                Check.Equal("2DVector released y", 0f, zeroVec.y);
                Check.That("2DVector release fires Canceled", movePhases.Contains(InputActionPhase.Canceled));
                Check.That("2DVector release returns to Waiting", move.phase == InputActionPhase.Waiting);
                move.Disable();
            }
        }

        private static void TestActionMapAndAssetLifecycleAndLookup()
        {
            InputSystem.Reset();

            InputActionAsset asset = ScriptableObject.CreateInstance<InputActionAsset>();
            InputActionMap gameplay = asset.AddActionMap("Gameplay");
            InputAction fire = gameplay.AddAction("Fire", InputActionType.Button, "<Mouse>/leftButton");
            InputAction look = gameplay.AddAction("Look", InputActionType.Value, "<Mouse>/delta");

            InputActionMap ui = new InputActionMap("UI");
            InputAction submit = ui.AddAction("Submit", InputActionType.Button, "<Keyboard>/enter");
            asset.AddActionMap(ui);

            Check.Equal("Asset has 2 maps", 2, asset.actionMaps.Count);
            Check.That("FindActionMap resolves Gameplay", ReferenceEquals(asset.FindActionMap("Gameplay"), gameplay));
            Check.That("FindAction resolves Gameplay/Fire", ReferenceEquals(asset.FindAction("Gameplay/Fire"), fire));
            Check.That("FindAction resolves unqualified Submit", ReferenceEquals(asset.FindAction("Submit"), submit));
            Check.That("Map indexer resolves Look", ReferenceEquals(gameplay["Look"], look));

            asset.Enable();
            Check.Equal("Asset enabled is true", true, asset.enabled);
            Check.Equal("Gameplay map enabled is true", true, gameplay.enabled);
            Check.Equal("UI map enabled is true", true, ui.enabled);
            Check.Equal("Fire action enabled is true", true, fire.enabled);

            gameplay.Disable();
            Check.Equal("Gameplay map disabled", false, gameplay.enabled);
            Check.Equal("UI map remains enabled", true, ui.enabled);

            asset.Disable();
            Check.Equal("Asset Disable disables remaining maps", false, ui.enabled);
            ScriptableObject.Destroy(asset);
        }

        private static void TestPlayerInputSendMessagesAndMapSwitching()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            InputSystem.Reset();

            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(0.02);

                InputActionAsset asset = ScriptableObject.CreateInstance<InputActionAsset>();
                InputActionMap gameplay = asset.AddActionMap("Gameplay");
                gameplay.AddAction("Jump", InputActionType.Button, "<Keyboard>/space");
                InputAction move = gameplay.AddAction("Move", InputActionType.Value);
                move.AddCompositeBinding("2DVector")
                    .With("Up", "<Keyboard>/w")
                    .With("Down", "<Keyboard>/s")
                    .With("Left", "<Keyboard>/a")
                    .With("Right", "<Keyboard>/d");

                InputActionMap uiMap = asset.AddActionMap("UI");
                uiMap.AddAction("Submit", InputActionType.Button, "<Keyboard>/enter");

                GameObject playerObj = new GameObject("Player");
                PlayerInputReceiver receiver = playerObj.AddComponent<PlayerInputReceiver>();
                PlayerInput playerInput = playerObj.AddComponent<PlayerInput>();
                playerInput.actions = asset;
                playerInput.defaultActionMap = "Gameplay";
                playerInput.notificationBehavior = PlayerNotifications.SendMessages;
                playerInput.ActivateInput();

                Check.That("PlayerInput currentActionMap is Gameplay", ReferenceEquals(playerInput.currentActionMap, gameplay));
                Check.Equal("Gameplay map is enabled on ActivateInput", true, gameplay.enabled);
                Check.Equal("UI map starts disabled", false, uiMap.enabled);

                host.InjectKey(KeyCode.Space, true);
                host.InjectKey(KeyCode.D, true);
                host.Tick(0.02);

                Check.Equal("PlayerInput SendMessages delivered OnJump", 1, receiver.JumpCount);
                Check.Equal("PlayerInput SendMessages delivered OnMove x", 1f, receiver.LastMove.x);
                Check.Equal("PlayerInput SendMessages delivered OnMove y", 0f, receiver.LastMove.y);

                host.InjectKey(KeyCode.Space, false);
                host.InjectKey(KeyCode.D, false);
                host.Tick(0.02);

                Check.Equal("PlayerInput SendMessages delivered OnMove reset to 0 on release", 0f, receiver.LastMove.x);

                playerInput.SwitchCurrentActionMap("UI");
                Check.Equal("SwitchCurrentActionMap disables Gameplay", false, gameplay.enabled);
                Check.Equal("SwitchCurrentActionMap enables UI", true, uiMap.enabled);
                Check.That("currentActionMap is UI", ReferenceEquals(playerInput.currentActionMap, uiMap));

                host.InjectKey(KeyCode.Space, true);
                host.InjectKey(KeyCode.Return, true);
                host.Tick(0.02);

                Check.Equal("Disabled Gameplay map does not trigger OnJump", 1, receiver.JumpCount);
                Check.Equal("Enabled UI map triggers OnSubmit", 1, receiver.SubmitCount);

                UnityEngine.Object.Destroy(playerObj);
                ScriptableObject.Destroy(asset);
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }

        private sealed class PlayerInputReceiver : MonoBehaviour, IUnitasMessageHandler
        {
            public int JumpCount { get; private set; }
            public int SubmitCount { get; private set; }
            public Vector2 LastMove { get; private set; }

            public bool Dispatch(string message)
            {
                if (string.Equals(message, "OnJump", StringComparison.Ordinal))
                {
                    JumpCount++;
                    return true;
                }
                if (string.Equals(message, "OnSubmit", StringComparison.Ordinal))
                {
                    SubmitCount++;
                    return true;
                }
                return false;
            }

            public bool Dispatch<T>(string message, T argument)
            {
                if (string.Equals(message, "OnMove", StringComparison.Ordinal) && argument is InputValue val)
                {
                    LastMove = val.Get<Vector2>();
                    return true;
                }
                if (string.Equals(message, "OnJump", StringComparison.Ordinal) && argument is InputValue jumpVal)
                {
                    if (jumpVal.isPressed)
                    {
                        JumpCount++;
                    }
                    return true;
                }
                return Dispatch(message);
            }
        }
    }

    internal static class InputSystemActionTestEntry
    {
        [ModuleInitializer]
        internal static void RunOnTargetShard()
        {
            string[] args = Environment.GetCommandLineArgs();
            if (IsPerfMode(args) || !IsTargetShard(args))
            {
                return;
            }
            InputSystemActionTests.Run();
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
                return parts.Length == 2 && string.Equals(parts[0], "3", StringComparison.Ordinal);
            }
            return true;
        }
    }
}
