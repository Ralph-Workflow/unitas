using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost.Tests
{
    [Serializable]
    public sealed class PlayerControllerStateProbe
    {
        public float moveSpeed;
        public float boostMultiplier;
    }

    public static class PlayerHostHotReloadTests
    {
        private const double DeltaSeconds = 0.02;
        private static bool s_probeRegistered;

        public static void Run(string sampleRoot)
        {
            EnsureProbeSerializerRegistered();
            RunPlaySessionHotReloadScenarios(sampleRoot);
            RunScriptingReloadCliVerb();
        }

        private static void EnsureProbeSerializerRegistered()
        {
            if (s_probeRegistered)
            {
                return;
            }

            s_probeRegistered = true;
            JsonUtility.RegisterType<PlayerControllerStateProbe>(
                () => new PlayerControllerStateProbe(),
                (val, w) =>
                {
                    w.WriteFloat("moveSpeed", val.moveSpeed);
                    w.WriteFloat("boostMultiplier", val.boostMultiplier);
                },
                (ref PlayerControllerStateProbe target, JsonParseNode n) =>
                {
                    target.moveSpeed = n.ReadFloat("moveSpeed", target.moveSpeed);
                    target.boostMultiplier = n.ReadFloat("boostMultiplier", target.boostMultiplier);
                });
        }

        private static void RunPlaySessionHotReloadScenarios(string sampleRoot)
        {
            string tempRoot = Path.Combine(
                Path.GetTempPath(),
                "unitas-playerhost-hotreload-" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            string bundleRoot = Path.Combine(tempRoot, "roll-a-ball");
            string managedDir = Path.Combine(bundleRoot, "Managed");
            string persistRoot = Path.Combine(tempRoot, "persist");
            Directory.CreateDirectory(managedDir);
            Directory.CreateDirectory(persistRoot);
            PlayerHostCore.ScriptChangesWhilePlaying = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
            try
            {
                PlayerHostBootTests.StageBundle(sampleRoot, bundleRoot);
                PlayerHostBootTests.CompileGameAssembly(bundleRoot, managedDir);

                int initResult = BootSession(bundleRoot, persistRoot);
                Check.Equal("hot reload session boots", 0, initResult);
                if (initResult != 0)
                {
                    return;
                }

                Check.Equal("hot reload initial tick 0", 0, PlayerHostEntry.Tick(DeltaSeconds));
                Check.Equal("hot reload initial tick 1", 0, PlayerHostEntry.Tick(DeltaSeconds));

                MonoBehaviour? beforeScript = FindPlayerControllerInstance();
                Check.That("PlayerController instance exists before hot reload", !ReferenceEquals(beforeScript, null));
                if (!ReferenceEquals(beforeScript, null))
                {
                    SetPlayerMoveSpeed(beforeScript, 19.25f);
                    Check.That(
                        "serialized moveSpeed mutated to 19.25 before reload",
                        Math.Abs(ReadPlayerMoveSpeed(beforeScript) - 19.25f) < 0.01f);
                }

                string playerControllerPath = Path.Combine(bundleRoot, "Assets", "Scripts", "PlayerController.cs");
                string originalSource = File.ReadAllText(playerControllerPath);
                string v2Source = InjectHotReloadMarkerLog(originalSource, "HotReloadV2Active");
                LogRecorder.Clear();
                File.WriteAllText(playerControllerPath, v2Source);

                Check.Equal("hot reload tick after script edit", 0, PlayerHostEntry.Tick(DeltaSeconds));
                Check.Equal("hot reload second tick after script edit", 0, PlayerHostEntry.Tick(DeltaSeconds));

                MonoBehaviour? afterV2Script = FindPlayerControllerInstance();
                Check.That("PlayerController instance exists after hot reload", !ReferenceEquals(afterV2Script, null));
                Check.That(
                    "PlayerController instance was replaced on hot reload",
                    !ReferenceEquals(beforeScript, null) && !ReferenceEquals(afterV2Script, null) && !ReferenceEquals(beforeScript, afterV2Script));
                if (!ReferenceEquals(afterV2Script, null))
                {
                    float restoredSpeed = ReadPlayerMoveSpeed(afterV2Script);
                    Check.That(
                        "serialized moveSpeed preserved across hot reload",
                        Math.Abs(restoredSpeed - 19.25f) < 0.01f);
                }

                Check.That(
                    "new script code behavior active after hot reload",
                    LogRecorder.Search("HotReloadV2Active").Count > 0);

                LogRecorder.Clear();
                File.WriteAllText(
                    playerControllerPath,
                    v2Source.Replace("HotReloadV2Active", "BrokenEdit", StringComparison.Ordinal)
                    + "\npublic class BrokenSyntax { void Bad() { int x = missing_ident_xyz; } }\n");

                Check.Equal("tick survives broken script edit during play", 0, PlayerHostEntry.Tick(DeltaSeconds));

                MonoBehaviour? afterBrokenScript = FindPlayerControllerInstance();
                Check.That(
                    "last-valid script instance stays active when compile fails",
                    !ReferenceEquals(afterV2Script, null) && ReferenceEquals(afterV2Script, afterBrokenScript));
                Check.That(
                    "compile error diagnostic emitted to error log",
                    LogRecorder.OfSeverity(LogSeverity.Error).Count > 0
                    && LogRecorder.Search("PlayerController.cs").Count > 0);
                Check.That(
                    "last-valid code continues executing on tick after failed compile",
                    LogRecorder.Search("HotReloadV2Active").Count > 0);

                PlayerHostCore.ScriptChangesWhilePlaying = ScriptChangesWhilePlayingPreference.RecompileAfterFinishedPlaying;
                string v3Source = InjectHotReloadMarkerLog(originalSource, "DeferredReloadV3Marker");
                LogRecorder.Clear();
                File.WriteAllText(playerControllerPath, v3Source);

                Check.Equal("tick with deferred reload preference", 0, PlayerHostEntry.Tick(DeltaSeconds));
                MonoBehaviour? midPlayScript = FindPlayerControllerInstance();
                Check.That(
                    "deferred preference does not clobber live instance mid-play",
                    !ReferenceEquals(afterV2Script, null) && ReferenceEquals(afterV2Script, midPlayScript));
                Check.Equal(
                    "deferred preference does not run new script code mid-play",
                    0,
                    LogRecorder.Search("DeferredReloadV3Marker").Count);

                PlayerHostCore.ScriptChangesWhilePlaying = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
                Check.Equal("tick after enabling RecompileAndContinuePlaying", 0, PlayerHostEntry.Tick(DeltaSeconds));
                Check.That(
                    "pending script change reloads once preference allows it",
                    LogRecorder.Search("DeferredReloadV3Marker").Count > 0);
                MonoBehaviour? afterV3Script = FindPlayerControllerInstance();
                if (!ReferenceEquals(afterV3Script, null))
                {
                    Check.That(
                        "serialized moveSpeed still preserved after deferred reload applies",
                        Math.Abs(ReadPlayerMoveSpeed(afterV3Script) - 19.25f) < 0.01f);
                }

                Check.Equal("hot reload session shutdown", 0, PlayerHostEntry.Shutdown());
            }
            finally
            {
                PlayerHostCore.ScriptChangesWhilePlaying = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
                PlayerHostBootTests.TryDelete(tempRoot);
            }
        }

        private static void RunScriptingReloadCliVerb()
        {
            string tempProject = Path.Combine(
                Path.GetTempPath(),
                "unitas-scripting-cli-reload-" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            string scriptsDir = Path.Combine(tempProject, "Assets", "Scripts");
            Directory.CreateDirectory(scriptsDir);
            try
            {
                string scriptPath = Path.Combine(scriptsDir, "CliReloadProbe.cs");
                string metaPath = scriptPath + ".meta";
                File.WriteAllText(
                    metaPath,
                    "fileFormatVersion: 2\nguid: 8b000000000000000000000000000001\nMonoImporter:\n  serializedVersion: 2\n");
                File.WriteAllText(
                    scriptPath,
                    "using UnityEngine;\npublic class CliReloadProbe : MonoBehaviour\n{\n    public int Value = 7;\n}\n");

                int okExit = Unitas.Scripting.Program.Main(new[] { "reload", tempProject });
                Check.Equal("Unitas.Scripting reload returns 0 on valid project", 0, okExit);

                File.WriteAllText(
                    scriptPath,
                    "using UnityEngine;\npublic class CliReloadProbe : MonoBehaviour\n{\n    public int Value = broken_symbol;\n}\n");
                int failExit = Unitas.Scripting.Program.Main(new[] { "reload", tempProject });
                Check.Equal("Unitas.Scripting reload returns 1 on broken project", 1, failExit);
            }
            finally
            {
                PlayerHostBootTests.TryDelete(tempProject);
            }
        }

        private static int BootSession(string bundleRoot, string persistRoot)
        {
            ulong runtimeHandle = RuntimeInterop.unitas_runtime_new();
            Check.That("native runtime handle created", runtimeHandle != 0UL);
            string scenePath = Path.Combine(bundleRoot, "Assets", "Scenes", "RollABall.unity");
            IntPtr bundlePtr = Marshal.StringToCoTaskMemUTF8(bundleRoot);
            IntPtr persistPtr = Marshal.StringToCoTaskMemUTF8(persistRoot);
            IntPtr scenePtr = Marshal.StringToCoTaskMemUTF8(scenePath);
            try
            {
                return PlayerHostEntry.Init(runtimeHandle, bundlePtr, persistPtr, scenePtr);
            }
            finally
            {
                Marshal.FreeCoTaskMem(bundlePtr);
                Marshal.FreeCoTaskMem(persistPtr);
                Marshal.FreeCoTaskMem(scenePtr);
            }
        }

        private static MonoBehaviour? FindPlayerControllerInstance()
        {
            GameObject? ball = GameObject.Find("PlayerBall");
            if (ball == null)
            {
                return null;
            }

            MonoBehaviour[] behaviours = ball.GetComponents<MonoBehaviour>();
            for (int i = 0; i < behaviours.Length; i++)
            {
                MonoBehaviour candidate = behaviours[i];
                Type? attachedType = ScriptGlueRegistry.GetAttachedRuntimeType(candidate);
                if (attachedType != null && string.Equals(attachedType.Name, "PlayerController", StringComparison.Ordinal))
                {
                    return candidate;
                }
            }

            return behaviours.Length > 0 ? behaviours[0] : null;
        }

        private static float ReadPlayerMoveSpeed(MonoBehaviour instance)
        {
            if (ScriptGlueRegistry.TryCaptureState(instance, out string? json) && !string.IsNullOrEmpty(json))
            {
                PlayerControllerStateProbe? probe = JsonUtility.FromJson<PlayerControllerStateProbe>(json);
                if (probe != null)
                {
                    return probe.moveSpeed;
                }
            }

            return -1f;
        }

        private static void SetPlayerMoveSpeed(MonoBehaviour instance, float moveSpeed)
        {
            string json = "{\"moveSpeed\":" + moveSpeed.ToString("R", CultureInfo.InvariantCulture) + ",\"boostMultiplier\":1.65}";
            ScriptGlueRegistry.TryRestoreState(instance, json);
        }

        private static string InjectHotReloadMarkerLog(string source, string marker)
        {
            const string anchor = "if (Time.timeScale <= 0f)";
            int idx = source.IndexOf(anchor, StringComparison.Ordinal);
            if (idx < 0)
            {
                throw new InvalidOperationException("Could not find Update anchor in PlayerController.cs");
            }

            return source.Substring(0, idx)
                + "Debug.Log(\"" + marker + " speed=\" + moveSpeed.ToString(System.Globalization.CultureInfo.InvariantCulture));\n            "
                + source.Substring(idx);
        }
    }
}
