using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using Unitas.Bridge;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost.Tests
{
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int PlayerInitDelegate(ulong runtimeHandle, IntPtr bundleRoot, IntPtr persistRoot, IntPtr startupScene);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int PlayerTickDelegate(double deltaSeconds);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int PlayerShutdownDelegate();

    public static class Program
    {
        public static int Main()
        {
            PlayerHostBootTests.Run();
            PlayerHostScenePathTests.Run();
            PlayerHostArtifactTests.Run();
            return Check.FailureCount > 0 ? 1 : 0;
        }
    }

    /// <summary>
    /// Loads the built host folder the way hostfxr does: the host assembly and
    /// every managed dependency come from that one folder, nothing else.
    /// </summary>
    public sealed class HostFolderLoadContext : AssemblyLoadContext
    {
        private readonly string _folder;

        public HostFolderLoadContext(string folder)
            : base("UnitasPlayerHostArtifact", isCollectible: false)
        {
            _folder = folder;
            Resolving += (context, assemblyName) =>
            {
                string? simpleName = assemblyName.Name;
                if (simpleName == null)
                {
                    return null;
                }

                string candidate = Path.Combine(_folder, simpleName + ".dll");
                return File.Exists(candidate) ? context.LoadFromAssemblyPath(candidate) : null;
            };
        }
    }

    public static class PlayerHostArtifactTests
    {
        private static readonly string[] RequiredArtifacts =
        {
            "Unitas.PlayerHost.dll",
            "Unitas.PlayerHost.deps.json",
            "Unitas.PlayerHost.runtimeconfig.json",
            "Unitas.Bridge.dll",
            "Unitas.Scripting.dll",
            "UnityEngine.dll",
        };

        public static void Run()
        {
            string artifactFolder = Path.Combine(
                PlayerHostBootTests.FindRepoRoot(),
                "bridge",
                "Unitas.PlayerHost",
                "bin",
                FindConfiguration(),
                "net8.0");
            foreach (string artifact in RequiredArtifacts)
            {
                Check.That(
                    "host folder carries " + artifact,
                    File.Exists(Path.Combine(artifactFolder, artifact)));
            }

            string assemblyPath = Path.Combine(artifactFolder, "Unitas.PlayerHost.dll");
            if (!File.Exists(assemblyPath))
            {
                return;
            }

            var context = new HostFolderLoadContext(artifactFolder);
            var assembly = context.LoadFromAssemblyPath(assemblyPath);
            Type entry = assembly.GetType("Unitas.PlayerHost.PlayerHostEntry", throwOnError: true)
                ?? throw new InvalidOperationException("PlayerHostEntry type not found");

            PlayerInitDelegate init = GetEntry<PlayerInitDelegate>(entry, "UnitasPlayerInit");
            PlayerTickDelegate tick = GetEntry<PlayerTickDelegate>(entry, "UnitasPlayerTick");
            PlayerShutdownDelegate shutdown = GetEntry<PlayerShutdownDelegate>(entry, "UnitasPlayerShutdown");

            MethodInfo tickMethod = entry.GetMethod("UnitasPlayerTick")
                ?? throw new InvalidOperationException("Entry point not found: UnitasPlayerTick");
            Check.Equal(
                "the tick entry carries only the frame delta",
                1,
                tickMethod.GetParameters().Length);

            int invalidInit = init(0UL, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);

            Check.Equal("isolated host rejects an invalid runtime handle", 1, invalidInit);
            Check.Equal("isolated host reports NotInitialized before boot", 7, tick(0.02));
            Check.Equal("isolated host reports NotInitialized before boot on shutdown", 7, shutdown());
        }

        private static T GetEntry<T>(Type entryType, string name)
            where T : Delegate
        {
            var method = entryType.GetMethod(name)
                ?? throw new InvalidOperationException("Entry point not found: " + name);
            return Marshal.GetDelegateForFunctionPointer<T>(method.MethodHandle.GetFunctionPointer());
        }

        private static string FindConfiguration()
        {
            DirectoryInfo? current = new DirectoryInfo(AppContext.BaseDirectory);
            string configuration = "Release";
            while (current != null && !string.Equals(current.Name, "bin", StringComparison.Ordinal))
            {
                configuration = current.Name;
                current = current.Parent;
            }

            return configuration;
        }
    }

    public static class PlayerHostBootTests
    {
        private const double DeltaSeconds = 0.02;

        public static void Run()
        {
            string repoRoot = FindRepoRoot();
            string sampleRoot = Path.Combine(repoRoot, "samples", "roll-a-ball");
            string tempRoot = Path.Combine(
                Path.GetTempPath(),
                "unitas-playerhost-" + Guid.NewGuid().ToString("N"));
            string bundleRoot = Path.Combine(tempRoot, "roll-a-ball");
            string persistRoot = Path.Combine(tempRoot, "persist");
            string managedDir = Path.Combine(bundleRoot, "Managed");
            Directory.CreateDirectory(persistRoot);
            Directory.CreateDirectory(managedDir);
            try
            {
                StageBundle(sampleRoot, bundleRoot);
                CompileGameAssembly(sampleRoot, managedDir);
                ulong runtimeHandle = RuntimeInterop.unitas_runtime_new();
                Check.That("native runtime handle created", runtimeHandle != 0UL);

                IntPtr bundlePtr = Marshal.StringToCoTaskMemUTF8(bundleRoot);
                IntPtr persistPtr = Marshal.StringToCoTaskMemUTF8(persistRoot);
                string scenePath = Path.Combine(bundleRoot, "Assets", "Scenes", "RollABall.unity");
                IntPtr scenePtr = Marshal.StringToCoTaskMemUTF8(scenePath);
                int initResult;
                try
                {
                    initResult = PlayerHostEntry.Init(runtimeHandle, bundlePtr, persistPtr, scenePtr);
                }
                finally
                {
                    Marshal.FreeCoTaskMem(bundlePtr);
                    Marshal.FreeCoTaskMem(persistPtr);
                    Marshal.FreeCoTaskMem(scenePtr);
                }

                Check.Equal("UnitasPlayerInit returns success", 0, initResult);
                if (initResult != 0)
                {
                    return;
                }

                for (int frame = 0; frame < 2; frame++)
                {
                    Check.Equal("settle tick " + frame + " returns success", 0, PlayerHostEntry.Tick(DeltaSeconds));
                }

                BridgeHost? host = PlayerHostCore.Host;
                Check.That("running host is exposed for input injection", host != null);
                GameObject? ball = GameObject.Find("PlayerBall");
                Check.That("scene PlayerBall is live", ball != null);
                if (ball == null || host == null)
                {
                    return;
                }

                float startZ = ball.transform.position.z;
                int pickupsBefore = CountPickups();
                host.InjectKey(KeyCode.W, true);
                for (int frame = 0; frame < 20; frame++)
                {
                    Check.Equal("drive tick " + frame + " returns success", 0, PlayerHostEntry.Tick(DeltaSeconds));
                }

                host.InjectKey(KeyCode.W, false);
                for (int frame = 0; frame < 5; frame++)
                {
                    PlayerHostEntry.Tick(DeltaSeconds);
                }

                float endZ = ball.transform.position.z;
                Check.That("marble moved north under injected input", endZ > startZ + 0.5f);
                Check.That("a pickup was collected after driving over it", CountPickups() < pickupsBefore);

                Check.Equal("UnitasPlayerShutdown returns success", 0, PlayerHostEntry.Shutdown());
                Check.Equal("tick after shutdown reports NotInitialized", 7, PlayerHostEntry.Tick(DeltaSeconds));
                Check.Equal("double shutdown reports NotInitialized", 7, PlayerHostEntry.Shutdown());
            }
            finally
            {
                TryDelete(tempRoot);
            }
        }

        private static int CountPickups()
        {
            int count = 0;
            for (int i = 1; i <= 12; i++)
            {
                if (GameObject.Find("Pickup_" + i) != null)
                {
                    count++;
                }
            }

            return count;
        }

        public static void CompileGameAssembly(string sampleRoot, string managedDir)
        {
            var options = new ScriptCompilationOptions
            {
                OutputDirectory = managedDir,
                IsEditor = false,
                EmitGeneratedGlue = true,
                AutoLoadGeneratedGlue = false,
            };
            ScriptCompilationResult result = ScriptCompilation.CompileProject(sampleRoot, options);
            Check.That("game script assembly compiled", result.Succeeded);
            Check.That("generated glue assembly staged in Managed", result.GeneratedGlueAssemblyPath != null);
        }

        public static void StageBundle(string sampleRoot, string bundleRoot)
        {
            foreach (string directory in new[] { "Assets", "ProjectSettings", "Packages" })
            {
                CopyDirectory(Path.Combine(sampleRoot, directory), Path.Combine(bundleRoot, directory));
            }
        }

        private static void CopyDirectory(string source, string destination)
        {
            Directory.CreateDirectory(destination);
            foreach (string file in Directory.GetFiles(source))
            {
                File.Copy(file, Path.Combine(destination, Path.GetFileName(file)));
            }

            foreach (string child in Directory.GetDirectories(source))
            {
                CopyDirectory(child, Path.Combine(destination, Path.GetFileName(child)));
            }
        }

        public static string FindRepoRoot()
        {
            string? current = AppContext.BaseDirectory;
            while (!string.IsNullOrEmpty(current))
            {
                if (File.Exists(Path.Combine(current, "Makefile"))
                    && Directory.Exists(Path.Combine(current, "samples", "roll-a-ball")))
                {
                    return current;
                }

                current = Directory.GetParent(current)?.FullName;
            }

            throw new InvalidOperationException("Could not locate repository root from " + AppContext.BaseDirectory);
        }

        public static void TryDelete(string path)
        {
            try
            {
                if (Directory.Exists(path))
                {
                    Directory.Delete(path, recursive: true);
                }
            }
            catch (Exception)
            {
            }
        }
    }
}
