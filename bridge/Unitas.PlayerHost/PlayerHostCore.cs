using System;
using System.Collections.Generic;
using System.IO;
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

        public static int Init(ulong runtimeHandle, string? bundleRoot, string? persistRoot)
        {
            lock (Gate)
            {
                if (_host != null)
                {
                    return (int)PlayerHostStage.AlreadyInitialized;
                }

                if (runtimeHandle == 0UL
                    || string.IsNullOrEmpty(bundleRoot)
                    || string.IsNullOrEmpty(persistRoot))
                {
                    return (int)PlayerHostStage.InvalidArgument;
                }

                BridgeHost? host = null;
                try
                {
                    string fullBundleRoot = Path.GetFullPath(bundleRoot);
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
                    string? scenePath = FindScene(fullBundleRoot);
                    if (scenePath == null)
                    {
                        return (int)PlayerHostStage.BundleInvalid;
                    }

                    host.LoadScene(scenePath);
                    _host = host;
                    host = null;
                    return (int)PlayerHostStage.Ok;
                }
                catch (Exception)
                {
                    return (int)PlayerHostStage.BootFailed;
                }
                finally
                {
                    host?.Dispose();
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

        private static string? FindScene(string bundleRoot)
        {
            var scenes = new List<string>();
            var skippedDirectories = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "Library",
                "obj",
                "bin",
            };
            CollectScenes(bundleRoot, skippedDirectories, scenes);
            scenes.Sort(StringComparer.Ordinal);
            string bundleName = NormalizeName(Path.GetFileName(bundleRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)));
            foreach (string scene in scenes)
            {
                if (string.Equals(NormalizeName(Path.GetFileNameWithoutExtension(scene)), bundleName, StringComparison.OrdinalIgnoreCase))
                {
                    return scene;
                }
            }

            return scenes.Count > 0 ? scenes[0] : null;
        }

        private static string NormalizeName(string name)
        {
            return name.Replace("-", string.Empty, StringComparison.Ordinal).Replace("_", string.Empty, StringComparison.Ordinal);
        }

        private static void CollectScenes(string directory, HashSet<string> skippedDirectories, List<string> scenes)
        {
            foreach (string scene in Directory.GetFiles(directory, "*.unity"))
            {
                scenes.Add(Path.GetFullPath(scene));
            }

            foreach (string child in Directory.GetDirectories(directory))
            {
                if (!skippedDirectories.Contains(Path.GetFileName(child)))
                {
                    CollectScenes(child, skippedDirectories, scenes);
                }
            }
        }
    }
}
