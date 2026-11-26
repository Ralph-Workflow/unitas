using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using Unitas.Bridge;
using UnityEngine;

namespace Unitas.PlayerHost.Tests
{
    /// <summary>
    /// The player host loads the startup scene the native side names, so the
    /// managed side never rescans the bundle for a scene file.
    /// </summary>
    public static class PlayerHostScenePathTests
    {
        public static void Run()
        {
            string repoRoot = PlayerHostBootTests.FindRepoRoot();
            string sampleRoot = Path.Combine(repoRoot, "samples", "roll-a-ball");
            string tempRoot = Path.Combine(
                Path.GetTempPath(),
                "unitas-playerhost-scene-" + Guid.NewGuid().ToString("N"));
            string bundleRoot = Path.Combine(tempRoot, "roll-a-ball");
            string persistRoot = Path.Combine(tempRoot, "persist");
            string managedDir = Path.Combine(bundleRoot, "Managed");
            Directory.CreateDirectory(persistRoot);
            Directory.CreateDirectory(managedDir);
            try
            {
                PlayerHostBootTests.StageBundle(sampleRoot, bundleRoot);
                PlayerHostBootTests.CompileGameAssembly(sampleRoot, managedDir);

                ulong runtimeHandle = RuntimeInterop.unitas_runtime_new();
                Check.That("native runtime handle created", runtimeHandle != 0UL);

                string missingScene = Path.Combine(bundleRoot, "Assets", "Scenes", "Absent.unity");
                IntPtr bundlePtr = Marshal.StringToCoTaskMemUTF8(bundleRoot);
                IntPtr persistPtr = Marshal.StringToCoTaskMemUTF8(persistRoot);
                IntPtr missingPtr = Marshal.StringToCoTaskMemUTF8(missingScene);
                try
                {
                    int missingResult = PlayerHostEntry.Init(runtimeHandle, bundlePtr, persistPtr, missingPtr);
                    Check.Equal("a startup scene that does not exist is refused", 2, missingResult);
                    Check.That("the refused boot left no host behind", PlayerHostCore.Host == null);

                    int noSceneResult = PlayerHostEntry.Init(runtimeHandle, bundlePtr, persistPtr, IntPtr.Zero);
                    Check.Equal("a missing startup scene path is refused", 1, noSceneResult);
                    Check.That("the refused boot left no host behind", PlayerHostCore.Host == null);
                }
                finally
                {
                    Marshal.FreeCoTaskMem(bundlePtr);
                    Marshal.FreeCoTaskMem(persistPtr);
                    Marshal.FreeCoTaskMem(missingPtr);
                }
            }
            finally
            {
                PlayerHostBootTests.TryDelete(tempRoot);
            }
        }
    }
}
