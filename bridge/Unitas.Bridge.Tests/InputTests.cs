using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class InputTests
    {
        public static void Run()
        {
            TestInertDefaultsWithoutProvider();
            TestFrameExactKeyTransitionsAndStringOverloads();
            TestMouseButtonsPointerAndScrollDelta();
            TestAxisRampSnapGravityInversionAndProjectSettingsDiscovery();
            TestSceneReloadResetsInputState();
            TestUnknownAxisAndButtonLogErrors();
        }

        private static void TestInertDefaultsWithoutProvider()
        {
            IInputProvider? savedProvider = Input.Provider;
            Input.Provider = null;
            LogRecorder.Clear();
            try
            {
                Check.Equal("Inert GetKey without provider", false, Input.GetKey(KeyCode.Space));
                Check.Equal("Inert GetKeyDown without provider", false, Input.GetKeyDown(KeyCode.Space));
                Check.Equal("Inert GetKeyUp without provider", false, Input.GetKeyUp(KeyCode.Space));
                Check.Equal("Inert GetKey string without provider", false, Input.GetKey("space"));
                Check.Equal("Inert GetKeyDown string without provider", false, Input.GetKeyDown("space"));
                Check.Equal("Inert GetKeyUp string without provider", false, Input.GetKeyUp("space"));
                Check.Equal("Inert GetMouseButton without provider", false, Input.GetMouseButton(0));
                Check.Equal("Inert GetMouseButtonDown without provider", false, Input.GetMouseButtonDown(0));
                Check.Equal("Inert GetMouseButtonUp without provider", false, Input.GetMouseButtonUp(0));
                Check.Equal("Inert GetButton without provider", false, Input.GetButton("Fire1"));
                Check.Equal("Inert GetButtonDown without provider", false, Input.GetButtonDown("Fire1"));
                Check.Equal("Inert GetButtonUp without provider", false, Input.GetButtonUp("Fire1"));
                Check.Equal("Inert GetAxis without provider", 0f, Input.GetAxis("Horizontal"));
                Check.Equal("Inert GetAxisRaw without provider", 0f, Input.GetAxisRaw("Horizontal"));
                Check.Equal("Inert mousePosition x without provider", 0f, Input.mousePosition.x);
                Check.Equal("Inert mousePosition y without provider", 0f, Input.mousePosition.y);
                Check.Equal("Inert mousePosition z without provider", 0f, Input.mousePosition.z);
                Check.Equal("Inert mousePositionDelta x without provider", 0f, Input.mousePositionDelta.x);
                Check.Equal("Inert mousePositionDelta y without provider", 0f, Input.mousePositionDelta.y);
                Check.Equal("Inert mousePositionDelta z without provider", 0f, Input.mousePositionDelta.z);
                Check.Equal("Inert mouseScrollDelta x without provider", 0f, Input.mouseScrollDelta.x);
                Check.Equal("Inert mouseScrollDelta y without provider", 0f, Input.mouseScrollDelta.y);
                Check.Equal("Inert mouseScrollDelta z without provider", 0f, Input.mouseScrollDelta.z);
                Check.Equal("Inert queries without provider log zero errors", 0, LogRecorder.OfSeverity(LogSeverity.Error).Count);
            }
            finally
            {
                Input.Provider = savedProvider;
            }
        }

        private static void TestFrameExactKeyTransitionsAndStringOverloads()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("Input key test scene exists", File.Exists(scenePath));

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptRegistry.Clear();
                InputRecorderScript recorder = new InputRecorderScript { WatchedKey = KeyCode.Space };
                ScriptRegistry.Register(0UL, 1UL, () => recorder);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);

                    host.InjectKey(KeyCode.Space, true);
                    host.InjectKey(KeyCode.RightArrow, true);
                    host.InjectKey(KeyCode.A, true);
                    host.Tick(0.02);

                    Check.Equal("Frame 1 FixedUpdate count", 1, recorder.FixedSnapshots.Count);
                    Check.Equal("Frame 1 Update count", 1, recorder.UpdateSnapshots.Count);
                    InputFrameSnapshot fixed1 = recorder.LastFixedSnapshot;
                    InputFrameSnapshot update1 = recorder.LastUpdateSnapshot;
                    Check.Equal("Frame 1 FixedUpdate GetKeyDown is true", true, fixed1.KeyDown);
                    Check.Equal("Frame 1 FixedUpdate GetKey is true", true, fixed1.KeyHeld);
                    Check.Equal("Frame 1 FixedUpdate GetKeyUp is false", false, fixed1.KeyUp);
                    Check.Equal("Frame 1 Update GetKeyDown is true", true, update1.KeyDown);
                    Check.Equal("Frame 1 Update GetKey is true", true, update1.KeyHeld);
                    Check.Equal("Frame 1 Update GetKeyUp is false", false, update1.KeyUp);
                    Check.Equal("Frame 1 Input.GetKey(\"space\") is true", true, Input.GetKey("space"));
                    Check.Equal("Frame 1 Input.GetKeyDown(\"right\") is true", true, Input.GetKeyDown("right"));

                    host.Tick(0.02);
                    InputFrameSnapshot fixed2 = recorder.LastFixedSnapshot;
                    InputFrameSnapshot update2 = recorder.LastUpdateSnapshot;
                    Check.Equal("Frame 2 FixedUpdate GetKeyDown is false", false, fixed2.KeyDown);
                    Check.Equal("Frame 2 FixedUpdate GetKey remains true", true, fixed2.KeyHeld);
                    Check.Equal("Frame 2 FixedUpdate GetKeyUp is false", false, fixed2.KeyUp);
                    Check.Equal("Frame 2 Update GetKeyDown is false", false, update2.KeyDown);
                    Check.Equal("Frame 2 Update GetKey remains true", true, update2.KeyHeld);
                    Check.Equal("Frame 2 Update GetKeyUp is false", false, update2.KeyUp);
                    Check.Equal("Frame 2 Input.GetKey(\"space\") remains true", true, Input.GetKey("space"));
                    Check.Equal("Frame 2 Input.GetKeyDown(\"right\") is false", false, Input.GetKeyDown("right"));

                    host.InjectKey(KeyCode.Space, false);
                    host.InjectKey(KeyCode.RightArrow, false);
                    host.InjectKey(KeyCode.A, false);
                    host.Tick(0.02);
                    InputFrameSnapshot fixed3 = recorder.LastFixedSnapshot;
                    InputFrameSnapshot update3 = recorder.LastUpdateSnapshot;
                    Check.Equal("Frame 3 FixedUpdate GetKeyDown is false", false, fixed3.KeyDown);
                    Check.Equal("Frame 3 FixedUpdate GetKey is false", false, fixed3.KeyHeld);
                    Check.Equal("Frame 3 FixedUpdate GetKeyUp is true", true, fixed3.KeyUp);
                    Check.Equal("Frame 3 Update GetKeyDown is false", false, update3.KeyDown);
                    Check.Equal("Frame 3 Update GetKey is false", false, update3.KeyHeld);
                    Check.Equal("Frame 3 Update GetKeyUp is true", true, update3.KeyUp);
                    Check.Equal("Frame 3 Input.GetKeyUp(\"a\") is true", true, Input.GetKeyUp("a"));

                    host.Tick(0.02);
                    InputFrameSnapshot fixed4 = recorder.LastFixedSnapshot;
                    InputFrameSnapshot update4 = recorder.LastUpdateSnapshot;
                    Check.Equal("Frame 4 FixedUpdate GetKeyUp is false", false, fixed4.KeyUp);
                    Check.Equal("Frame 4 Update GetKeyUp is false", false, update4.KeyUp);
                    Check.Equal("Frame 4 Input.GetKeyUp(\"a\") is false", false, Input.GetKeyUp("a"));

                    LogRecorder.Clear();
                    Check.Equal("Unknown key string GetKey returns false", false, Input.GetKey("not_a_key"));
                    Check.Equal("Unknown key string GetKeyDown returns false", false, Input.GetKeyDown("not_a_key"));
                    Check.Equal("Unknown key string GetKeyUp returns false", false, Input.GetKeyUp("not_a_key"));
                    IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                    Check.Equal("Unknown key string logs error entries", 3, errors.Count);
                    Check.That(
                        "Unknown key error message mentions key name",
                        errors.Count > 0 && errors[0].Message.Contains("not_a_key", StringComparison.Ordinal));
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TestMouseButtonsPointerAndScrollDelta()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptRegistry.Clear();
                InputRecorderScript recorder = new InputRecorderScript { WatchedMouseButton = 0 };
                ScriptRegistry.Register(0UL, 1UL, () => recorder);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);

                    host.InjectMouseButton(0, true);
                    host.InjectPointerPosition(120.0, 45.0);
                    host.InjectScroll(0.0, 2.5);
                    host.Tick(0.02);

                    InputFrameSnapshot snap1 = recorder.LastUpdateSnapshot;
                    Check.Equal("Mouse button 0 down on press frame", true, snap1.MouseButtonDown);
                    Check.Equal("Mouse button 0 held on press frame", true, snap1.MouseButtonHeld);
                    Check.Equal("Mouse button 0 up false on press frame", false, snap1.MouseButtonUp);
                    Check.Equal("mousePosition x is 120", 120f, snap1.MousePosition.x);
                    Check.Equal("mousePosition y is 45", 45f, snap1.MousePosition.y);
                    Check.Equal("mousePosition z is 0", 0f, snap1.MousePosition.z);
                    Check.Equal("mousePositionDelta x is 120", 120f, snap1.MousePositionDelta.x);
                    Check.Equal("mousePositionDelta y is 45", 45f, snap1.MousePositionDelta.y);
                    Check.Equal("mousePositionDelta z is 0", 0f, snap1.MousePositionDelta.z);
                    Check.Equal("mouseScrollDelta x is 0", 0f, snap1.MouseScrollDelta.x);
                    Check.Equal("mouseScrollDelta y is 2.5", 2.5f, snap1.MouseScrollDelta.y);
                    Check.Equal("mouseScrollDelta z is 0", 0f, snap1.MouseScrollDelta.z);
                    Check.Equal("Mouse X axis is 12.0", 12.0f, snap1.MouseX);
                    Check.Equal("Mouse Y axis is 4.5", 4.5f, snap1.MouseY);
                    Check.Equal("Mouse ScrollWheel axis is 0.25", 0.25f, snap1.MouseScrollWheel);
                    Check.Equal("Fire1 button down is true", true, snap1.Fire1Down);
                    Check.Equal("Fire1 button held is true", true, snap1.Fire1Held);
                    Check.Equal("Fire1 button up is false", false, snap1.Fire1Up);

                    host.Tick(0.02);
                    InputFrameSnapshot snap2 = recorder.LastUpdateSnapshot;
                    Check.Equal("Mouse button 0 down resets next frame", false, snap2.MouseButtonDown);
                    Check.Equal("Mouse button 0 stays held next frame", true, snap2.MouseButtonHeld);
                    Check.Equal("mousePosition x stays 120", 120f, snap2.MousePosition.x);
                    Check.Equal("mousePosition y stays 45", 45f, snap2.MousePosition.y);
                    Check.Equal("mousePositionDelta x resets to 0", 0f, snap2.MousePositionDelta.x);
                    Check.Equal("mousePositionDelta y resets to 0", 0f, snap2.MousePositionDelta.y);
                    Check.Equal("mouseScrollDelta x resets to 0", 0f, snap2.MouseScrollDelta.x);
                    Check.Equal("mouseScrollDelta y resets to 0", 0f, snap2.MouseScrollDelta.y);
                    Check.Equal("Mouse X axis resets to 0", 0f, snap2.MouseX);
                    Check.Equal("Mouse Y axis resets to 0", 0f, snap2.MouseY);
                    Check.Equal("Mouse ScrollWheel axis resets to 0", 0f, snap2.MouseScrollWheel);
                    Check.Equal("Fire1 button down resets next frame", false, snap2.Fire1Down);
                    Check.Equal("Fire1 button held remains true", true, snap2.Fire1Held);

                    host.InjectMouseButton(0, false);
                    host.Tick(0.02);
                    InputFrameSnapshot snap3 = recorder.LastUpdateSnapshot;
                    Check.Equal("Mouse button 0 up is true on release frame", true, snap3.MouseButtonUp);
                    Check.Equal("Mouse button 0 held is false on release frame", false, snap3.MouseButtonHeld);
                    Check.Equal("Fire1 button up is true on release frame", true, snap3.Fire1Up);
                    Check.Equal("Fire1 button held is false on release frame", false, snap3.Fire1Held);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TestAxisRampSnapGravityInversionAndProjectSettingsDiscovery()
        {
            string inputManagerFixture = ResolveFixture("crates/unitas-asset/tests/fixtures/InputManager.asset");
            string sceneFixture = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            Check.That("InputManager fixture exists", File.Exists(inputManagerFixture));
            Check.That("Scene fixture exists", File.Exists(sceneFixture));

            string tempRoot = Path.Combine(Path.GetTempPath(), "unitas_input_test_" + Guid.NewGuid().ToString("N"));
            string settingsDir = Path.Combine(tempRoot, "ProjectSettings");
            string assetsDir = Path.Combine(tempRoot, "Assets");
            Directory.CreateDirectory(settingsDir);
            Directory.CreateDirectory(assetsDir);

            string tempInputManager = Path.Combine(settingsDir, "InputManager.asset");
            string tempScene = Path.Combine(assetsDir, "scene.unity");
            File.Copy(inputManagerFixture, tempInputManager, true);
            File.Copy(sceneFixture, tempScene, true);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptRegistry.Clear();
                InputRecorderScript recorder = new InputRecorderScript();
                ScriptRegistry.Register(0UL, 1UL, () => recorder);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(tempScene);
                    host.Tick(0.02);

                    host.InjectKey(KeyCode.RightArrow, true);
                    host.Tick(0.1);
                    Check.Equal("Horizontal ramp step 1 (0.3)", 0.3f, recorder.LastUpdateSnapshot.Horizontal);
                    Check.Equal("HorizontalRaw is 1.0 on step 1", 1.0f, recorder.LastUpdateSnapshot.HorizontalRaw);

                    host.Tick(0.1);
                    Check.Equal("Horizontal ramp step 2 (0.6)", 0.6f, recorder.LastUpdateSnapshot.Horizontal);

                    host.Tick(0.1);
                    Check.Equal("Horizontal ramp step 3 (0.9)", 0.9f, recorder.LastUpdateSnapshot.Horizontal);

                    host.Tick(0.1);
                    Check.Equal("Horizontal ramp step 4 clamps to 1.0", 1.0f, recorder.LastUpdateSnapshot.Horizontal);

                    host.InjectKey(KeyCode.RightArrow, false);
                    host.InjectKey(KeyCode.LeftArrow, true);
                    host.Tick(0.1);
                    Check.Equal("Horizontal snaps through 0 to -0.3 on opposite press", -0.3f, recorder.LastUpdateSnapshot.Horizontal);
                    Check.Equal("HorizontalRaw is -1.0 on opposite press", -1.0f, recorder.LastUpdateSnapshot.HorizontalRaw);

                    host.InjectKey(KeyCode.LeftArrow, false);
                    host.Tick(0.05);
                    Check.Equal("Horizontal decays toward 0 by gravity * dt (-0.15)", -0.15f, recorder.LastUpdateSnapshot.Horizontal);
                    Check.Equal("HorizontalRaw is 0 when released", 0f, recorder.LastUpdateSnapshot.HorizontalRaw);

                    host.Tick(0.05);
                    Check.Equal("Horizontal decays to 0", 0f, recorder.LastUpdateSnapshot.Horizontal);

                    host.InjectKey(KeyCode.UpArrow, true);
                    host.Tick(0.1);
                    Check.Equal("Inverted Vertical produces -0.3 on UpArrow", -0.3f, recorder.LastUpdateSnapshot.Vertical);
                    Check.Equal("Inverted VerticalRaw produces -1.0 on UpArrow", -1.0f, recorder.LastUpdateSnapshot.VerticalRaw);
                }
            }
            finally
            {
                Time.timeScale = savedScale;
                if (Directory.Exists(tempRoot))
                {
                    Directory.Delete(tempRoot, true);
                }
            }
        }

        private static void TestSceneReloadResetsInputState()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            try
            {
                ScriptRegistry.Clear();
                InputRecorderScript recorder = new InputRecorderScript { WatchedKey = KeyCode.RightArrow };
                ScriptRegistry.Register(0UL, 1UL, () => recorder);

                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);

                    host.InjectKey(KeyCode.RightArrow, true);
                    host.InjectMouseButton(0, true);
                    host.InjectPointerPosition(80.0, 60.0);
                    host.InjectScroll(1.0, 3.0);
                    host.Tick(0.1);

                    Check.Equal("Pre-reload key is held", true, Input.GetKey(KeyCode.RightArrow));
                    Check.Equal("Pre-reload Horizontal is non-zero", 0.3f, Input.GetAxis("Horizontal"));
                    Check.Equal("Pre-reload mousePosition x is 80", 80f, Input.mousePosition.x);

                    recorder.ClearSnapshots();
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);

                    Check.Equal("Post-reload key held is reset to false", false, Input.GetKey(KeyCode.RightArrow));
                    Check.Equal("Post-reload key down is false", false, Input.GetKeyDown(KeyCode.RightArrow));
                    Check.Equal("Post-reload mouse button held is reset to false", false, Input.GetMouseButton(0));
                    Check.Equal("Post-reload mousePosition x is reset to 0", 0f, Input.mousePosition.x);
                    Check.Equal("Post-reload mousePosition y is reset to 0", 0f, Input.mousePosition.y);
                    Check.Equal("Post-reload mousePositionDelta x is 0", 0f, Input.mousePositionDelta.x);
                    Check.Equal("Post-reload mouseScrollDelta y is 0", 0f, Input.mouseScrollDelta.y);
                    Check.Equal("Post-reload Horizontal axis is reset to 0", 0f, Input.GetAxis("Horizontal"));
                    Check.Equal("Post-reload HorizontalRaw axis is reset to 0", 0f, Input.GetAxisRaw("Horizontal"));
                }
            }
            finally
            {
                Time.timeScale = savedScale;
            }
        }

        private static void TestUnknownAxisAndButtonLogErrors()
        {
            string scenePath = ResolveFixture("crates/unitas-runtime/tests/fixtures/scene_with_script.unity");
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                LogRecorder.Clear();

                float axisVal = Input.GetAxis("NonExistentAxis");
                float axisRawVal = Input.GetAxisRaw("NonExistentAxis");
                bool btnVal = Input.GetButton("NonExistentButton");
                bool btnDownVal = Input.GetButtonDown("NonExistentButton");
                bool btnUpVal = Input.GetButtonUp("NonExistentButton");

                Check.Equal("Unknown GetAxis returns 0", 0f, axisVal);
                Check.Equal("Unknown GetAxisRaw returns 0", 0f, axisRawVal);
                Check.Equal("Unknown GetButton returns false", false, btnVal);
                Check.Equal("Unknown GetButtonDown returns false", false, btnDownVal);
                Check.Equal("Unknown GetButtonUp returns false", false, btnUpVal);

                IReadOnlyList<LogEntry> errors = LogRecorder.OfSeverity(LogSeverity.Error);
                Check.Equal("Unknown axis and button calls record 5 error entries", 5, errors.Count);
                Check.That(
                    "First error mentions NonExistentAxis",
                    errors.Count >= 1 && errors[0].Message.Contains("NonExistentAxis", StringComparison.Ordinal));
                Check.That(
                    "Third error mentions NonExistentButton",
                    errors.Count >= 3 && errors[2].Message.Contains("NonExistentButton", StringComparison.Ordinal));
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
