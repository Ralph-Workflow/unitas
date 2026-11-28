using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Security.Cryptography;
using System.Text;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost
{
    internal static class PlayerHostHotReloadWatcher
    {
        private const string PreferenceEnvVar = "UNITAS_SCRIPT_CHANGES_WHILE_PLAYING";

        [MethodImpl(MethodImplOptions.NoInlining)]
        public static ScriptDomainReload.Result ReloadLiveSession(
            BridgeHost host,
            string bundleRoot,
            string managedDir,
            AssemblyLoadContext? loadContext)
        {
            Dictionary<ulong, MonoBehaviour> scriptsMap = FieldAccessor.ScriptsRef(host);
            List<LiveSlotSnapshot> slots = CaptureLiveSlots(scriptsMap);
            List<MonoBehaviour> liveInstances = new List<MonoBehaviour>(slots.Count);
            for (int i = 0; i < slots.Count; i++)
            {
                liveInstances.Add(slots[i].Instance);
            }

            ScriptDomainReload.Result reloadResult = ScriptDomainReload.Reload(new ScriptDomainReload.Request
            {
                ProjectRoot = bundleRoot,
                Options = BuildReloadCompilationOptions(managedDir),
                LiveInstances = liveInstances,
                ReplacedContext = loadContext,
            });

            EmitDiagnostics(reloadResult.Diagnostics);

            if (!reloadResult.Succeeded)
            {
                if (!string.IsNullOrEmpty(reloadResult.Error))
                {
                    Debug.LogError(reloadResult.Error);
                }

                return reloadResult;
            }

            RebindRecreatedInstances(scriptsMap, slots, reloadResult.RecreatedInstances);
            return reloadResult;
        }

        public static ScriptCompilationOptions BuildReloadCompilationOptions(string managedDir)
        {
            return new ScriptCompilationOptions
            {
                OutputDirectory = managedDir,
                IsEditor = false,
                EmitGeneratedGlue = true,
                AutoLoadGeneratedGlue = false,
            };
        }

        public static void EmitDiagnostics(IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                ScriptDiagnostic diagnostic = diagnostics[i];
                string formatted = diagnostic.FormatCli();
                if (diagnostic.Severity == ScriptDiagnosticSeverity.Error)
                {
                    Debug.LogError(formatted);
                }
                else if (diagnostic.Severity == ScriptDiagnosticSeverity.Warning)
                {
                    Debug.LogWarning(formatted);
                }
                else
                {
                    Debug.Log(formatted);
                }
            }
        }

        public static ScriptChangesWhilePlayingPreference ReadConfiguredPreference(string? bundleRoot)
        {
            string? envValue = Environment.GetEnvironmentVariable(PreferenceEnvVar);
            if (TryParsePreferenceValue(envValue, out ScriptChangesWhilePlayingPreference parsedEnv))
            {
                return parsedEnv;
            }

            if (!string.IsNullOrEmpty(bundleRoot))
            {
                string settingsDir = Path.Combine(bundleRoot, "ProjectSettings");
                if (Directory.Exists(settingsDir))
                {
                    string[] files = Directory.GetFiles(settingsDir, "*.asset");
                    Array.Sort(files, StringComparer.Ordinal);
                    for (int i = 0; i < files.Length; i++)
                    {
                        if (TryReadPreferenceFromYamlFile(files[i], out ScriptChangesWhilePlayingPreference parsedYaml))
                        {
                            return parsedYaml;
                        }
                    }
                }
            }

            return ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
        }

        public static string ComputeQuickFileStamp(string bundleRoot)
        {
            string assetsDir = Path.Combine(bundleRoot, "Assets");
            if (!Directory.Exists(assetsDir))
            {
                return string.Empty;
            }

            List<string> trackedFiles = new List<string>();
            CollectWatchedScriptFiles(assetsDir, trackedFiles);
            trackedFiles.Sort(StringComparer.Ordinal);

            StringBuilder builder = new StringBuilder();
            for (int i = 0; i < trackedFiles.Count; i++)
            {
                string filePath = trackedFiles[i];
                try
                {
                    FileInfo info = new FileInfo(filePath);
                    builder.Append(filePath)
                        .Append('|')
                        .Append(info.Length.ToString(CultureInfo.InvariantCulture))
                        .Append('|')
                        .Append(info.LastWriteTimeUtc.Ticks.ToString(CultureInfo.InvariantCulture))
                        .Append('\n');
                }
                catch (IOException)
                {
                }
            }

            return builder.ToString();
        }

        public static string ComputeProjectScriptSignature(string bundleRoot)
        {
            string assetsDir = Path.Combine(bundleRoot, "Assets");
            if (!Directory.Exists(assetsDir))
            {
                return string.Empty;
            }

            List<string> trackedFiles = new List<string>();
            CollectWatchedScriptFiles(assetsDir, trackedFiles);
            trackedFiles.Sort(StringComparer.Ordinal);

            StringBuilder builder = new StringBuilder();
            for (int i = 0; i < trackedFiles.Count; i++)
            {
                string filePath = trackedFiles[i];
                builder.Append(filePath).Append('\n');
                try
                {
                    byte[] contentBytes = File.ReadAllBytes(filePath);
                    byte[] contentHash = SHA256.HashData(contentBytes);
                    for (int b = 0; b < contentHash.Length; b++)
                    {
                        builder.Append(contentHash[b].ToString("x2", CultureInfo.InvariantCulture));
                    }
                }
                catch (IOException)
                {
                }

                builder.Append('\n');
            }

            byte[] finalHash = SHA256.HashData(Encoding.UTF8.GetBytes(builder.ToString()));
            StringBuilder hex = new StringBuilder(finalHash.Length * 2);
            for (int i = 0; i < finalHash.Length; i++)
            {
                hex.Append(finalHash[i].ToString("x2", CultureInfo.InvariantCulture));
            }

            return hex.ToString();
        }

        private static List<LiveSlotSnapshot> CaptureLiveSlots(Dictionary<ulong, MonoBehaviour> scriptsMap)
        {
            List<ulong> handles = new List<ulong>(scriptsMap.Keys);
            handles.Sort();
            List<LiveSlotSnapshot> slots = new List<LiveSlotSnapshot>(handles.Count);
            for (int i = 0; i < handles.Count; i++)
            {
                ulong handle = handles[i];
                if (!scriptsMap.TryGetValue(handle, out MonoBehaviour? instance) || ReferenceEquals(instance, null))
                {
                    continue;
                }

                if (!ScriptGlueRegistry.TryGetRegisteredGuid(instance, out ulong guidHi, out ulong guidLo))
                {
                    continue;
                }

                if (ScriptGlueRegistry.GetBuiltInUiComponent(guidHi, guidLo) != ScriptGlueRegistry.BuiltInUiComponent.None)
                {
                    continue;
                }

                GameObject? owner = FieldAccessor.ComponentGameObjectRef(instance);
                int componentIndex = -1;
                if (!ReferenceEquals(owner, null))
                {
                    List<Component> comps = FieldAccessor.GameObjectComponentsRef(owner);
                    for (int c = 0; c < comps.Count; c++)
                    {
                        if (ReferenceEquals(comps[c], instance))
                        {
                            componentIndex = c;
                            break;
                        }
                    }
                }

                slots.Add(new LiveSlotSnapshot(handle, instance, owner, componentIndex, instance.enabled));
            }

            return slots;
        }

        private static void RebindRecreatedInstances(
            Dictionary<ulong, MonoBehaviour> scriptsMap,
            List<LiveSlotSnapshot> slots,
            IReadOnlyList<MonoBehaviour> recreatedInstances)
        {
            int count = Math.Min(slots.Count, recreatedInstances.Count);
            for (int i = 0; i < count; i++)
            {
                LiveSlotSnapshot slot = slots[i];
                MonoBehaviour replacement = recreatedInstances[i];
                if (ReferenceEquals(replacement, null))
                {
                    continue;
                }

                if (!slot.Enabled)
                {
                    replacement.enabled = false;
                }

                GameObject? owner = slot.Owner;
                if (!ReferenceEquals(owner, null))
                {
                    FieldAccessor.ComponentGameObjectRef(replacement) = owner;
                    FieldAccessor.ObjectSceneRef(replacement) = owner.scene;
                    List<Component> comps = FieldAccessor.GameObjectComponentsRef(owner);
                    for (int c = comps.Count - 1; c >= 0; c--)
                    {
                        if (ReferenceEquals(comps[c], slot.Instance))
                        {
                            comps.RemoveAt(c);
                        }
                    }

                    if (slot.ComponentIndex >= 0 && slot.ComponentIndex <= comps.Count)
                    {
                        comps.Insert(slot.ComponentIndex, replacement);
                    }
                    else
                    {
                        comps.Add(replacement);
                    }
                }

                scriptsMap[slot.Handle] = replacement;
            }

            for (int i = 0; i < count; i++)
            {
                MonoBehaviour? replacement = recreatedInstances[i];
                GameObject? owner = slots[i].Owner;
                if (!ReferenceEquals(replacement, null) && !ReferenceEquals(owner, null) && owner && owner.activeInHierarchy)
                {
                    InvokeReloadCallbackSafely(replacement, replacement.Awake);
                    if (replacement.isActiveAndEnabled)
                    {
                        InvokeReloadCallbackSafely(replacement, replacement.OnEnable);
                    }
                }
            }
        }

        private static void InvokeReloadCallbackSafely(MonoBehaviour script, Action callback)
        {
            UnityEngine.Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = script;
            try
            {
                callback();
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                Debug.LogException(ex, script);
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        private static bool TryReadPreferenceFromYamlFile(
            string filePath,
            out ScriptChangesWhilePlayingPreference preference)
        {
            preference = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
            string[] lines;
            try
            {
                lines = File.ReadAllLines(filePath);
            }
            catch (IOException)
            {
                return false;
            }

            for (int i = 0; i < lines.Length; i++)
            {
                string trimmed = lines[i].Trim();
                if (!trimmed.StartsWith("scriptChangesWhilePlaying:", StringComparison.OrdinalIgnoreCase)
                    && !trimmed.StartsWith("m_ScriptChangesWhilePlaying:", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                int colon = trimmed.IndexOf(':', StringComparison.Ordinal);
                if (colon > 0)
                {
                    string rawVal = trimmed.Substring(colon + 1).Trim();
                    if (TryParsePreferenceValue(rawVal, out preference))
                    {
                        return true;
                    }
                }
            }

            return false;
        }

        private static bool TryParsePreferenceValue(
            string? value,
            out ScriptChangesWhilePlayingPreference preference)
        {
            preference = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
            if (string.IsNullOrWhiteSpace(value))
            {
                return false;
            }

            string trimmed = value.Trim();
            if (string.Equals(trimmed, "0", StringComparison.Ordinal)
                || string.Equals(trimmed, "true", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "reload", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "RecompileAndContinuePlaying", StringComparison.OrdinalIgnoreCase))
            {
                preference = ScriptChangesWhilePlayingPreference.RecompileAndContinuePlaying;
                return true;
            }

            if (string.Equals(trimmed, "1", StringComparison.Ordinal)
                || string.Equals(trimmed, "false", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "defer", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "disabled", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "RecompileAfterFinishedPlaying", StringComparison.OrdinalIgnoreCase))
            {
                preference = ScriptChangesWhilePlayingPreference.RecompileAfterFinishedPlaying;
                return true;
            }

            if (string.Equals(trimmed, "2", StringComparison.Ordinal)
                || string.Equals(trimmed, "stop", StringComparison.OrdinalIgnoreCase)
                || string.Equals(trimmed, "StopPlayingAndRecompile", StringComparison.OrdinalIgnoreCase))
            {
                preference = ScriptChangesWhilePlayingPreference.StopPlayingAndRecompile;
                return true;
            }

            return false;
        }

        private static void CollectWatchedScriptFiles(string directory, List<string> results)
        {
            string[] files;
            try
            {
                files = Directory.GetFiles(directory);
            }
            catch (IOException)
            {
                return;
            }

            for (int i = 0; i < files.Length; i++)
            {
                string file = files[i];
                if (file.EndsWith(".cs", StringComparison.OrdinalIgnoreCase)
                    || file.EndsWith(".asmdef", StringComparison.OrdinalIgnoreCase)
                    || file.EndsWith(".asmref", StringComparison.OrdinalIgnoreCase)
                    || file.EndsWith(".rsp", StringComparison.OrdinalIgnoreCase))
                {
                    results.Add(Path.GetFullPath(file));
                }
            }

            string[] subdirs;
            try
            {
                subdirs = Directory.GetDirectories(directory);
            }
            catch (IOException)
            {
                return;
            }

            Array.Sort(subdirs, StringComparer.Ordinal);
            for (int i = 0; i < subdirs.Length; i++)
            {
                CollectWatchedScriptFiles(subdirs[i], results);
            }
        }

        private readonly struct LiveSlotSnapshot
        {
            public LiveSlotSnapshot(
                ulong handle,
                MonoBehaviour instance,
                GameObject? owner,
                int componentIndex,
                bool enabled)
            {
                Handle = handle;
                Instance = instance;
                Owner = owner;
                ComponentIndex = componentIndex;
                Enabled = enabled;
            }

            public ulong Handle { get; }

            public MonoBehaviour Instance { get; }

            public GameObject? Owner { get; }

            public int ComponentIndex { get; }

            public bool Enabled { get; }
        }

        private static class FieldAccessor
        {
            [UnsafeAccessor(UnsafeAccessorKind.Field, Name = "_scripts")]
            public static extern ref Dictionary<ulong, MonoBehaviour> ScriptsRef(BridgeHost host);

            [UnsafeAccessor(UnsafeAccessorKind.Field, Name = "_components")]
            public static extern ref List<Component> GameObjectComponentsRef(GameObject gameObject);

            [UnsafeAccessor(UnsafeAccessorKind.Field, Name = "_gameObject")]
            public static extern ref GameObject? ComponentGameObjectRef(Component component);

            [UnsafeAccessor(UnsafeAccessorKind.Field, Name = "_scene")]
            public static extern ref UnityEngine.SceneManagement.Scene ObjectSceneRef(UnityEngine.Object obj);
        }
    }
}
