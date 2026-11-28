using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using System.Text;
using System.Text.Json.Nodes;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost
{
    public enum PlayerHostStage
    {
        Ok = 0,
        InvalidArgument = 1,
        BundleInvalid = 2,
        AssemblyLoadFailed = 3,
        BootFailed = 4,
        TickFailed = 5,
        ShutdownFailed = 6,
        NotInitialized = 7,
        AlreadyInitialized = 8,
        BufferTooSmall = 9,
    }

    public enum ScriptChangesWhilePlayingPreference
    {
        RecompileAndContinuePlaying = 0,
        RecompileAfterFinishedPlaying = 1,
        StopPlayingAndRecompile = 2,
    }

    public static class PlayerHostCore
    {
        private static readonly object Gate = new object();
        private static readonly StreamWriter ManagedLogWriter =
            new StreamWriter(Console.OpenStandardError()) { AutoFlush = true };
        private static BridgeHost? _host;
        private static AssemblyLoadContext? _loadContext;
        private static string? _bundleRoot;
        private static string? _managedDir;
        private static string _lastWatchedStamp = string.Empty;
        private static string _lastAttemptedSignature = string.Empty;
        private static ScriptChangesWhilePlayingPreference? _preferenceOverride;

        public static ScriptChangesWhilePlayingPreference ScriptChangesWhilePlaying
        {
            get
            {
                lock (Gate)
                {
                    if (_preferenceOverride.HasValue)
                    {
                        return _preferenceOverride.Value;
                    }

                    return PlayerHostHotReloadWatcher.ReadConfiguredPreference(_bundleRoot);
                }
            }
            set
            {
                lock (Gate)
                {
                    _preferenceOverride = value;
                }
            }
        }

        public static BridgeHost? Host
        {
            get
            {
                lock (Gate)
                {
                    return _host;
                }
            }
        }

        public static WeakReference? CaptureLoadContextWeakReference()
        {
            lock (Gate)
            {
                return _loadContext != null ? new WeakReference(_loadContext) : null;
            }
        }

        public static int Init(
            ulong runtimeHandle,
            string? bundleRoot,
            string? persistRoot,
            string? startupScenePath)
        {
            lock (Gate)
            {
                if (_host != null)
                {
                    return (int)PlayerHostStage.AlreadyInitialized;
                }

                if (runtimeHandle == 0UL
                    || string.IsNullOrEmpty(bundleRoot)
                    || string.IsNullOrEmpty(persistRoot)
                    || string.IsNullOrEmpty(startupScenePath))
                {
                    return (int)PlayerHostStage.InvalidArgument;
                }

                BridgeHost? host = null;
                AssemblyLoadContext? loadContext = null;
                try
                {
                    if (Environment.GetEnvironmentVariable("UNITAS_MANAGED_LOG_TO_STDERR") == "1")
                    {
                        Console.SetOut(ManagedLogWriter);
                    }

                    string fullBundleRoot = Path.GetFullPath(bundleRoot);
                    string fullScenePath = Path.GetFullPath(startupScenePath);
                    if (!File.Exists(fullScenePath))
                    {
                        return (int)PlayerHostStage.BundleInvalid;
                    }

                    BindRuntimeLibrary(fullBundleRoot);
                    string managedDir = Path.Combine(fullBundleRoot, "Managed");
                    if (!Directory.Exists(managedDir))
                    {
                        return (int)PlayerHostStage.BundleInvalid;
                    }

                    loadContext = LoadManagedAssemblies(managedDir);
                    if (loadContext == null)
                    {
                        return (int)PlayerHostStage.AssemblyLoadFailed;
                    }

                    Application.PersistentDataPathOverride = Path.GetFullPath(persistRoot);
                    host = new BridgeHost(runtimeHandle);
                    host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                    host.LoadScene(fullScenePath);
                    _host = host;
                    _loadContext = loadContext;
                    _bundleRoot = fullBundleRoot;
                    _managedDir = managedDir;
                    _lastWatchedStamp = PlayerHostHotReloadWatcher.ComputeQuickFileStamp(fullBundleRoot);
                    _lastAttemptedSignature = PlayerHostHotReloadWatcher.ComputeProjectScriptSignature(fullBundleRoot);
                    host = null;
                    loadContext = null;
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception)
                {
                    return (int)PlayerHostStage.BootFailed;
                }
                finally
                {
                    host?.Dispose();
                    if (loadContext != null)
                    {
                        ScriptGlueRegistry.ClearSession();
                        loadContext.Unload();
                    }
                }
            }
        }

        public static int Tick(double deltaSeconds)
        {
            lock (Gate)
            {
                BridgeHost? host = _host;
                if (host == null)
                {
                    return (int)PlayerHostStage.NotInitialized;
                }

                try
                {
                    PollAndReloadScriptChanges(host);
                    if (_host == null)
                    {
                        return (int)PlayerHostStage.Ok;
                    }

                    host.Tick(deltaSeconds);
                    UnityEngine.UI.Ui.PumpClicks();
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception)
                {
                    return (int)PlayerHostStage.TickFailed;
                }
            }
        }

        public static int Shutdown()
        {
            lock (Gate)
            {
                BridgeHost? host = _host;
                if (host == null)
                {
                    return (int)PlayerHostStage.NotInitialized;
                }

                _host = null;
                AssemblyLoadContext? loadContext = _loadContext;
                _loadContext = null;
                _bundleRoot = null;
                _managedDir = null;
                _lastWatchedStamp = string.Empty;
                _lastAttemptedSignature = string.Empty;
                try
                {
                    host.Shutdown();
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception)
                {
                    return (int)PlayerHostStage.ShutdownFailed;
                }
                finally
                {
                    ScriptGlueRegistry.ClearSession();
                    loadContext?.Unload();
                }
            }
        }

        /// Serialize the recorded runtime log lines the filters select as a JSON
        /// array. The worker control loop calls this between ticks, so the
        /// recorder is not mutated while it is read.
        public static byte[] CollectLogsJson(int severity, long afterFrame, int limit)
        {
            JsonArray entries = new JsonArray();
            int added = 0;
            foreach (LogEntry entry in LogRecorder.Entries)
            {
                if (added >= limit)
                {
                    break;
                }

                if (severity >= 0 && (int)entry.Severity != severity)
                {
                    continue;
                }

                if (afterFrame >= 0 && entry.Frame <= afterFrame)
                {
                    continue;
                }

                entries.Add(new JsonObject
                {
                    ["severity"] = entry.Severity.ToString().ToLowerInvariant(),
                    ["frame"] = entry.Frame,
                    ["time"] = entry.Time,
                    ["message"] = entry.Message,
                    ["callStack"] = entry.CallStack,
                    ["origin"] = entry.Origin?.name,
                });
                added++;
            }

            return Encoding.UTF8.GetBytes(entries.ToJsonString());
        }

        private static AssemblyLoadContext? LoadManagedAssemblies(string managedDir)
        {
            string[] assemblies = Directory.GetFiles(managedDir, "*.dll");
            Array.Sort(assemblies, StringComparer.Ordinal);
            AssemblyLoadContext loadContext = ScriptGlueRegistry.CreateCollectibleContext("UnitasPlayerHost", managedDir);
            string glueFileName = ScriptCompilation.GeneratedGlueAssemblyName + ".dll";
            string? gluePath = null;
            foreach (string path in assemblies)
            {
                if (string.Equals(Path.GetFileName(path), glueFileName, StringComparison.OrdinalIgnoreCase))
                {
                    gluePath = path;
                    continue;
                }

                string simpleName = Path.GetFileNameWithoutExtension(path);
                bool alreadyInDefault = false;
                foreach (var existing in AssemblyLoadContext.Default.Assemblies)
                {
                    if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                    {
                        alreadyInDefault = true;
                        break;
                    }
                }

                if (alreadyInDefault)
                {
                    continue;
                }

                try
                {
                    ScriptGlueRegistry.LoadAssemblyIntoContext(path, loadContext);
                }
                catch (Exception)
                {
                    ScriptGlueRegistry.ClearSession();
                    loadContext.Unload();
                    return null;
                }
            }

            if (gluePath != null)
            {
                try
                {
                    ScriptGlueRegistry.LoadGlueAssembly(gluePath, loadContext);
                }
                catch (Exception)
                {
                    ScriptGlueRegistry.ClearSession();
                    loadContext.Unload();
                    return null;
                }
            }

            return loadContext;
        }

        private static void PollAndReloadScriptChanges(BridgeHost host)
        {
            string? bundleRoot = _bundleRoot;
            string? managedDir = _managedDir;
            if (string.IsNullOrEmpty(bundleRoot) || string.IsNullOrEmpty(managedDir))
            {
                return;
            }

            string currentStamp = PlayerHostHotReloadWatcher.ComputeQuickFileStamp(bundleRoot);
            if (string.IsNullOrEmpty(currentStamp)
                || string.Equals(currentStamp, _lastWatchedStamp, StringComparison.Ordinal))
            {
                return;
            }

            ScriptChangesWhilePlayingPreference preference =
                _preferenceOverride ?? PlayerHostHotReloadWatcher.ReadConfiguredPreference(bundleRoot);
            if (preference == ScriptChangesWhilePlayingPreference.RecompileAfterFinishedPlaying)
            {
                return;
            }

            string currentSignature = PlayerHostHotReloadWatcher.ComputeProjectScriptSignature(bundleRoot);
            if (string.IsNullOrEmpty(currentSignature)
                || string.Equals(currentSignature, _lastAttemptedSignature, StringComparison.Ordinal))
            {
                _lastWatchedStamp = currentStamp;
                return;
            }

            _lastAttemptedSignature = currentSignature;

            if (preference == ScriptChangesWhilePlayingPreference.StopPlayingAndRecompile)
            {
                ExecuteReloadWithoutLiveSession(bundleRoot, managedDir, currentStamp);
                Shutdown();
                return;
            }

            ExecuteHotReloadOnLiveSession(host, bundleRoot, managedDir, currentStamp);
        }

        private static void ExecuteReloadWithoutLiveSession(string bundleRoot, string managedDir, string stamp)
        {
            ScriptDomainReload.Result result = ScriptDomainReload.Reload(new ScriptDomainReload.Request
            {
                ProjectRoot = bundleRoot,
                Options = PlayerHostHotReloadWatcher.BuildReloadCompilationOptions(managedDir),
                LiveInstances = Array.Empty<MonoBehaviour>(),
                ReplacedContext = _loadContext,
            });
            PlayerHostHotReloadWatcher.EmitDiagnostics(result.Diagnostics);
            if (result.Succeeded)
            {
                _loadContext = result.Context;
                _lastWatchedStamp = stamp;
            }
            else if (!string.IsNullOrEmpty(result.Error))
            {
                Debug.LogError(result.Error);
            }
        }

        [MethodImpl(MethodImplOptions.NoInlining)]
        private static void ExecuteHotReloadOnLiveSession(
            BridgeHost host,
            string bundleRoot,
            string managedDir,
            string stamp)
        {
            ScriptDomainReload.Result reloadResult = PlayerHostHotReloadWatcher.ReloadLiveSession(
                host,
                bundleRoot,
                managedDir,
                _loadContext);
            if (!reloadResult.Succeeded)
            {
                return;
            }

            _loadContext = reloadResult.Context;
            _lastWatchedStamp = stamp;
        }

        private static void BindRuntimeLibrary(string bundleRoot)
        {
            string? configured = Environment.GetEnvironmentVariable("UNITAS_RUNTIME_LIBRARY");
            if (!string.IsNullOrEmpty(configured))
            {
                return;
            }

            string candidate = Path.Combine(bundleRoot, RuntimeLibraryFileName());
            if (File.Exists(candidate))
            {
                Environment.SetEnvironmentVariable("UNITAS_RUNTIME_LIBRARY", candidate);
            }
        }

        private static string RuntimeLibraryFileName()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return "unitas_runtime.dll";
            }

            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                return "libunitas_runtime.dylib";
            }

            return "libunitas_runtime.so";
        }
    }
}
