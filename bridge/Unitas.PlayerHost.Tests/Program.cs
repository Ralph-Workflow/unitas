using System;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using Unitas.Bridge;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.PlayerHost.Tests
{
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
                LogRecorder.Clear();

                ulong runtimeHandle = RuntimeInterop.unitas_runtime_new();
                Check.That("native runtime handle created", runtimeHandle != 0UL);

                IntPtr bundlePtr = Marshal.StringToCoTaskMemUTF8(bundleRoot);
                IntPtr persistPtr = Marshal.StringToCoTaskMemUTF8(persistRoot);
                int initResult;
                try
                {
                    initResult = PlayerHostEntry.Init(runtimeHandle, bundlePtr, persistPtr);
                }
                finally
                {
                    Marshal.FreeCoTaskMem(bundlePtr);
                    Marshal.FreeCoTaskMem(persistPtr);
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
                int score = ReadScore();
                Check.Equal("score increased after driving over a pickup", 1, score > 0 ? 1 : 0);

                Check.Equal("UnitasPlayerShutdown returns success", 0, PlayerHostEntry.Shutdown());
                Check.Equal("tick after shutdown reports NotInitialized", 7, PlayerHostEntry.Tick(DeltaSeconds));
                Check.Equal("double shutdown reports NotInitialized", 7, PlayerHostEntry.Shutdown());
            }
            finally
            {
                TryDelete(tempRoot);
            }
        }

        private static int ReadScore()
        {
            const string Prefix = "Pickup collected score ";
            foreach (LogEntry entry in LogRecorder.Entries)
            {
                if (!entry.Message.StartsWith(Prefix, StringComparison.Ordinal))
                {
                    continue;
                }

                string reported = entry.Message.Substring(Prefix.Length);
                if (int.TryParse(reported, NumberStyles.Integer, CultureInfo.InvariantCulture, out int collected))
                {
                    return collected;
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
