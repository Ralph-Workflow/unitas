using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using Unitas.Bridge;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost.Tests
{
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public unsafe delegate int PlayerInitDelegate(ulong runtimeHandle, byte* bundleRoot, byte* persistRoot);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int PlayerTickDelegate(double deltaSeconds);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int PlayerShutdownDelegate();

    public static class Program
    {
        public static int Main()
        {
            PlayerHostBootTests.Run();
            return Check.FailureCount > 0 ? 1 : 0;
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

                PlayerInitDelegate init = GetEntry<PlayerInitDelegate>("UnitasPlayerInit");
                PlayerTickDelegate tick = GetEntry<PlayerTickDelegate>("UnitasPlayerTick");
                PlayerShutdownDelegate shutdown = GetEntry<PlayerShutdownDelegate>("UnitasPlayerShutdown");

                ulong runtimeHandle = RuntimeInterop.unitas_runtime_new();
                Check.That("native runtime handle created", runtimeHandle != 0UL);

                byte[] bundleUtf8 = Encoding.UTF8.GetBytes(bundleRoot + "\0");
                byte[] persistUtf8 = Encoding.UTF8.GetBytes(persistRoot + "\0");
                int initResult;
                unsafe
                {
                    fixed (byte* bundlePtr = bundleUtf8)
                    {
                        fixed (byte* persistPtr = persistUtf8)
                        {
                            initResult = init(runtimeHandle, bundlePtr, persistPtr);
                        }
                    }
                }

                Check.Equal("UnitasPlayerInit returns success", 0, initResult);
                if (initResult != 0)
                {
                    return;
                }

                for (int frame = 0; frame < 2; frame++)
                {
                    Check.Equal("settle tick " + frame + " returns success", 0, tick(DeltaSeconds));
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
                host.InjectKey(KeyCode.W, true);
                for (int frame = 0; frame < 20; frame++)
                {
                    Check.Equal("drive tick " + frame + " returns success", 0, tick(DeltaSeconds));
                }

                host.InjectKey(KeyCode.W, false);
                for (int frame = 0; frame < 5; frame++)
                {
                    tick(DeltaSeconds);
                }

                float endZ = ball.transform.position.z;
                Check.That("marble moved north under injected input", endZ > startZ + 0.5f);
                int score = ReadScore();
                Check.Equal("score increased after driving over a pickup", 1, score > 0 ? 1 : 0);

                Check.Equal("UnitasPlayerShutdown returns success", 0, shutdown());
                Check.Equal("tick after shutdown reports NotInitialized", 7, tick(DeltaSeconds));
                Check.Equal("double shutdown reports NotInitialized", 7, shutdown());
            }
            finally
            {
                TryDelete(tempRoot);
            }
        }

        private static T GetEntry<T>(string name)
            where T : Delegate
        {
            MethodInfo method = typeof(PlayerHostEntry).GetMethod(
                name,
                BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic)
                ?? throw new InvalidOperationException("Entry point not found: " + name);
            return Marshal.GetDelegateForFunctionPointer<T>(method.MethodHandle.GetFunctionPointer());
        }

        private static int ReadScore()
        {
            GameObject? sensor = GameObject.Find("PickupSensor");
            if (sensor == null)
            {
                return -1;
            }

            foreach (Component component in sensor.GetComponents<Component>())
            {
                Type type = component.GetType();
                if (!string.Equals(type.Name, "ScoreManager", StringComparison.Ordinal))
                {
                    continue;
                }

                PropertyInfo? score = type.GetProperty("Score", BindingFlags.Public | BindingFlags.Instance);
                if (score?.GetValue(component) is int value)
                {
                    return value;
                }
            }

            return -1;
        }

        private static void CompileGameAssembly(string sampleRoot, string managedDir)
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

        private static void StageBundle(string sampleRoot, string bundleRoot)
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

        private static string FindRepoRoot()
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

        private static void TryDelete(string path)
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
