using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    [Serializable]
    public sealed class ReloadProbeStateSnapshot
    {
        public int Counter;
        public string Build = string.Empty;
    }

    public static class ScriptDomainReloadTests
    {
        private const string ProbeGuid = "7a000000000000000000000000000001";
        private static bool s_snapshotRegistered;

        public static void Run()
        {
            EnsureSnapshotSerializerRegistered();
            ExecuteReloadScenario();
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static void ExecuteReloadScenario()
        {
            string projectRoot = CreateTempProject();
            try
            {
                WriteProbeSource(projectRoot, "v1");

                WeakReference? replacedContext = RunFirstAndSecondReload(projectRoot, out ulong guidHi, out ulong guidLo, out WeakReference? secondContextRef);

                CollectGarbage();

                Check.Equal(
                    "The replaced load context is collectible",
                    false,
                    replacedContext != null && replacedContext.IsAlive);

                WriteBrokenProbeSource(projectRoot);

                ScriptDomainReload.Result failed = ScriptDomainReload.Reload(NewRequest(projectRoot, Array.Empty<MonoBehaviour>()));
                Check.Equal("A failed compile reports failure", false, failed.Succeeded);
                Check.Equal("A failed compile reports that the compile itself failed", false, failed.CompileSucceeded);
                Check.That("A failed compile keeps the last valid assembly state", failed.ActiveState == ActiveAssemblyState.LastValidFallback);
                Check.That("A failed compile reports diagnostics", failed.Diagnostics.Count > 0);
                Check.That("A failed compile names the broken source file", ContainsSourceFile(failed.Diagnostics, "ReloadProbe.cs"));
                Check.Equal(
                    "A failed compile keeps the current load context",
                    true,
                    failed.Context != null && secondContextRef != null && ReferenceEquals(failed.Context, secondContextRef.Target));
                Check.Equal("A failed compile creates no replacement instances", 0, failed.RecreatedInstances.Count);

                bool survivorCreated = ScriptGlueRegistry.TryCreate(guidHi, guidLo, out MonoBehaviour? survivor, out _);
                Check.Equal("The last valid script stays creatable after a failed compile", true, survivorCreated);
                if (survivor != null)
                {
                    Check.Equal("The last valid script keeps running the last valid code", "v2", ReadBuildVersion(survivor));
                }
            }
            finally
            {
                DeleteDirectory(projectRoot);
            }
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        private static WeakReference? RunFirstAndSecondReload(
            string projectRoot,
            out ulong guidHi,
            out ulong guidLo,
            out WeakReference? secondContextRef)
        {
            ScriptDomainReload.Result first = ScriptDomainReload.Reload(NewRequest(projectRoot, Array.Empty<MonoBehaviour>()));
            Check.Equal("Reload compiles the project and loads the game assembly", true, first.Succeeded);
            Check.That("Reload reports the current assembly state", first.ActiveState == ActiveAssemblyState.Current);
            Check.That("Reload created a collectible load context", first.Context != null);
            Check.That("The first reload has no replaced load context", first.ReplacedContext == null);

            (guidHi, guidLo) = ScriptGlueRegistry.ParseGuid(ProbeGuid);
            secondContextRef = null;
            bool created = ScriptGlueRegistry.TryCreate(guidHi, guidLo, out MonoBehaviour? probe, out IUnitasMessageHandler? handler);
            Check.Equal("The compiled script is creatable through the glue registry", true, created);
            Check.That("The compiled script instance exists", probe != null);
            Check.That("The compiled script has a generated message handler", handler != null);

            if (probe == null || handler == null)
            {
                return null;
            }

            Check.Equal("The script starts on the first build", "v1", ReadBuildVersion(probe));
            Check.Equal("The generated handler dispatches the script method", true, handler.Dispatch("SetCounter", 42));
            Check.Equal("The script field changed through the handler", 42, ReadIntField(probe, "Counter"));

            WriteProbeSource(projectRoot, "v2");

            List<MonoBehaviour> live = new List<MonoBehaviour> { probe };
            ScriptDomainReload.Result second = ScriptDomainReload.Reload(NewRequest(projectRoot, live));
            Check.Equal("The reload of a recompiled project succeeds", true, second.Succeeded);
            Check.Equal("The reload recreates the live script instance", 1, second.RecreatedInstances.Count);

            MonoBehaviour? replacement = second.RecreatedInstances.Count > 0 ? second.RecreatedInstances[0] : null;
            Check.That("The reload produced a replacement script instance", replacement != null);

            if (replacement != null)
            {
                Check.Equal("The reloaded script runs the recompiled code", "v2", ReadBuildVersion(replacement));
                Check.Equal("The reloaded script keeps its serialized field state", 42, ReadIntField(replacement, "Counter"));
                Check.Equal(
                    "The reloaded script type comes from the new load context",
                    true,
                    ScriptGlueRegistry.GetInstanceLoadContext(replacement) == second.Context);
            }

            secondContextRef = second.Context != null ? new WeakReference(second.Context) : null;
            WeakReference? replacedContext = second.ReplacedContext;
            Check.That("The reload reports the replaced load context", replacedContext != null);
            live.Clear();
            return replacedContext;
        }

        private static void EnsureSnapshotSerializerRegistered()
        {
            if (s_snapshotRegistered)
            {
                return;
            }

            s_snapshotRegistered = true;
            JsonUtility.RegisterType<ReloadProbeStateSnapshot>(
                () => new ReloadProbeStateSnapshot(),
                (val, w) =>
                {
                    w.WriteInt("Counter", val.Counter);
                    w.WriteString("Build", val.Build);
                },
                (ref ReloadProbeStateSnapshot target, JsonParseNode n) =>
                {
                    target.Counter = n.ReadInt("Counter", target.Counter);
                    target.Build = n.ReadString("Build", target.Build);
                });
        }

        private static ScriptDomainReload.Request NewRequest(string projectRoot, IReadOnlyList<MonoBehaviour> liveInstances)
        {
            return new ScriptDomainReload.Request
            {
                ProjectRoot = projectRoot,
                Options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64",
                    EmitGeneratedGlue = true,
                    AutoLoadGeneratedGlue = false,
                },
                LiveInstances = liveInstances,
            };
        }

        private static string CreateTempProject()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_domain_reload_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(Path.Combine(tempDir, "Assets", "Scripts"));
            return tempDir;
        }

        private static void DeleteDirectory(string directory)
        {
            try
            {
                if (Directory.Exists(directory))
                {
                    Directory.Delete(directory, true);
                }
            }
            catch (IOException)
            {
            }
        }

        private static void WriteProbeSource(string projectRoot, string build)
        {
            string scriptPath = Path.Combine(projectRoot, "Assets", "Scripts", "ReloadProbe.cs");
            File.WriteAllText(
                scriptPath,
                "using UnityEngine;\n"
                + "\n"
                + "public class ReloadProbe : MonoBehaviour\n"
                + "{\n"
                + "    public int Counter = 1;\n"
                + "\n"
                + "    [EditorCallableMethod]\n"
                + "    public static string GetBuild() => \"" + build + "\";\n"
                + "\n"
                + "    public void SetCounter(int value)\n"
                + "    {\n"
                + "        Counter = value;\n"
                + "    }\n"
                + "}\n");

            WriteMeta(projectRoot);
        }

        private static void WriteBrokenProbeSource(string projectRoot)
        {
            string scriptPath = Path.Combine(projectRoot, "Assets", "Scripts", "ReloadProbe.cs");
            File.WriteAllText(
                scriptPath,
                "using UnityEngine;\n"
                + "\n"
                + "public class ReloadProbe : MonoBehaviour\n"
                + "{\n"
                + "    public int Counter = 1;\n"
                + "\n"
                + "    [EditorCallableMethod]\n"
                + "    public static string GetBuild() => \"broken\";\n"
                + "\n"
                + "    public void SetCounter(int value)\n"
                + "    {\n"
                + "        Counter = missing_symbol;\n"
                + "    }\n"
                + "}\n");

            WriteMeta(projectRoot);
        }

        private static void WriteMeta(string projectRoot)
        {
            string metaPath = Path.Combine(projectRoot, "Assets", "Scripts", "ReloadProbe.cs.meta");
            File.WriteAllText(
                metaPath,
                "fileFormatVersion: 2\n"
                + "guid: " + ProbeGuid + "\n"
                + "MonoImporter:\n"
                + "  serializedVersion: 2\n");
        }

        private static string ReadBuildVersion(MonoBehaviour instance)
        {
            _ = instance;
            if (EditorMethodRegistry.TryInvoke("ReloadProbe.GetBuild", out object? result)
                && result is string buildText)
            {
                return buildText;
            }

            return string.Empty;
        }

        private static int ReadIntField(MonoBehaviour instance, string fieldName)
        {
            if (string.Equals(fieldName, "Counter", StringComparison.Ordinal)
                && ScriptGlueRegistry.TryCaptureState(instance, out string? json)
                && !string.IsNullOrEmpty(json))
            {
                ReloadProbeStateSnapshot? snapshot = JsonUtility.FromJson<ReloadProbeStateSnapshot>(json);
                if (snapshot != null)
                {
                    return snapshot.Counter;
                }
            }

            return -1;
        }

        private static bool ContainsSourceFile(IReadOnlyList<ScriptDiagnostic> diagnostics, string fileName)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (diagnostics[i].File.Contains(fileName, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static void CollectGarbage()
        {
            for (int i = 0; i < 10; i++)
            {
                GC.Collect(GC.MaxGeneration, GCCollectionMode.Forced, blocking: true, compacting: true);
                GC.WaitForPendingFinalizers();
            }

            GC.Collect(GC.MaxGeneration, GCCollectionMode.Forced, blocking: true, compacting: true);
        }
    }
}
