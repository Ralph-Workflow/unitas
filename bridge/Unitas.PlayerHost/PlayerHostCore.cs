using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
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
    }

    public static class PlayerHostCore
    {
        private static readonly object Gate = new object();
        private static readonly StreamWriter ManagedLogWriter =
            new StreamWriter(Console.OpenStandardError()) { AutoFlush = true };
        private static BridgeHost? _host;
        private static AssemblyLoadContext? _loadContext;

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

                    string[] assemblies = Directory.GetFiles(managedDir, "*.dll");
                    Array.Sort(assemblies, StringComparer.Ordinal);
                    loadContext = ScriptGlueRegistry.CreateCollectibleContext("UnitasPlayerHost", managedDir);
                    foreach (string path in assemblies)
                    {
                        try
                        {
                            ScriptGlueRegistry.LoadGlueAssembly(path, loadContext);
                        }
                        catch (Exception)
                        {
                            ScriptGlueRegistry.ClearSession();
                            loadContext.Unload();
                            loadContext = null;
                            return (int)PlayerHostStage.AssemblyLoadFailed;
                        }
                    }

                    Application.PersistentDataPathOverride = Path.GetFullPath(persistRoot);
                    host = new BridgeHost(runtimeHandle);
                    host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                    host.LoadScene(fullScenePath);
                    _host = host;
                    _loadContext = loadContext;
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
