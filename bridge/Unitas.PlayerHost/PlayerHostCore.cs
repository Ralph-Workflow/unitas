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
                try
                {
                    // The control protocol owns stdout; managed logging must not
                    // corrupt the JSON the host parses. Only the player process
                    // asks for that: the editor hosts the same managed code and
                    // speaks its own JSON protocol on stdout.
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
                    foreach (string path in assemblies)
                    {
                        string simpleName = Path.GetFileNameWithoutExtension(path);
                        if (IsAlreadyLoaded(simpleName))
                        {
                            continue;
                        }

                        AssemblyLoadContext.Default.LoadFromAssemblyPath(path);
                    }

                    foreach (string path in assemblies)
                    {
                        try
                        {
                            ScriptGlueRegistry.LoadGlueAssembly(path);
                        }
                        catch (Exception)
                        {
                            return (int)PlayerHostStage.AssemblyLoadFailed;
                        }
                    }

                    Application.PersistentDataPathOverride = Path.GetFullPath(persistRoot);
                    host = new BridgeHost(runtimeHandle);
                    host.BuildKind = ApplicationBuildKind.ReleasePlayer;
                    host.LoadScene(fullScenePath);
                    _host = host;
                    host = null;
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception u0debug)
                {
                    Console.Error.WriteLine($"U0DEBUG Init failed: {u0debug}");
                    return (int)PlayerHostStage.BootFailed;
                }
                finally
                {
                    host?.Dispose();
                }
            }
        }

        public static int Tick(
            double deltaSeconds,
            IntPtr hudBuffer,
            int hudCapacity,
            IntPtr scoreOut,
            IntPtr totalOut,
            IntPtr wonOut)
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
                    PublishHud(hudBuffer, hudCapacity, scoreOut, totalOut, wonOut);
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
                try
                {
                    host.Shutdown();
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception)
                {
                    return (int)PlayerHostStage.ShutdownFailed;
                }
            }
        }

        private static void PublishHud(
            IntPtr hudBuffer,
            int hudCapacity,
            IntPtr scoreOut,
            IntPtr totalOut,
            IntPtr wonOut)
        {
            PlayerHostHud.Read(out int score, out int total, out bool won, out string text);
            if (scoreOut != IntPtr.Zero)
            {
                Marshal.WriteInt32(scoreOut, score);
            }

            if (totalOut != IntPtr.Zero)
            {
                Marshal.WriteInt32(totalOut, total);
            }

            if (wonOut != IntPtr.Zero)
            {
                Marshal.WriteInt32(wonOut, won ? 1 : 0);
            }

            if (hudBuffer == IntPtr.Zero || hudCapacity <= 1)
            {
                return;
            }

            byte[] bytes = System.Text.Encoding.UTF8.GetBytes(text);
            int length = Math.Min(bytes.Length, hudCapacity - 1);
            if (length > 0)
            {
                Marshal.Copy(bytes, 0, hudBuffer, length);
            }

            Marshal.WriteByte(hudBuffer, (byte)length);
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

        private static bool IsAlreadyLoaded(string simpleName)
        {
            foreach (var assembly in AssemblyLoadContext.Default.Assemblies)
            {
                if (string.Equals(assembly.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            return false;
        }
    }
}
