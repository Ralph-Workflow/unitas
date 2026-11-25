using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.Loader;
using Unitas.Bridge.Messaging;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class CompilationTests
    {
        public static string? LastGeneratedGlueAssemblyPath { get; private set; }

        public static void EnsureCompiledGlueLoaded()
        {
            if (string.IsNullOrEmpty(LastGeneratedGlueAssemblyPath) || !File.Exists(LastGeneratedGlueAssemblyPath))
            {
                TestGeneratedGlueAndZeroReflectionMessageDispatch();
            }

            if (!string.IsNullOrEmpty(LastGeneratedGlueAssemblyPath) && File.Exists(LastGeneratedGlueAssemblyPath))
            {
                ScriptGlueRegistry.LoadGlueAssembly(LastGeneratedGlueAssemblyPath);
            }
        }

        public static void Run()
        {
            TestStory6AssembliesAsmrefDefinesAndVersionDefines();
            TestStory6ReferenceCycleDiagnostic();
            TestStory7TargetDefinesEditorAndPlayers();
            TestStory1And4FailedEditPreservesLastValidAndIncrementalRecompile();
            TestStory10IncompatibleNativePluginDllImportDiagnostic();
            TestGeneratedGlueAndZeroReflectionMessageDispatch();
            TestCliCompileEntryPoint();
        }

        private static void TestStory6AssembliesAsmrefDefinesAndVersionDefines()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "story6");
            try
            {
                ScriptCompilationOptions editorOptions = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };
                editorOptions.PackageVersions["com.mygame.analytics"] = "1.4.0";

                ScriptCompilationResult result = ScriptCompilation.CompileProject(tempProject, editorOptions);
                Check.Equal("Story 6 editor compile succeeds", true, result.Succeeded);
                Check.That("Story 6 active state is Current", result.ActiveState == ActiveAssemblyState.Current);

                CompiledAssemblyRecord? firstPass = result.FindAssembly("Assembly-CSharp-firstpass");
                CompiledAssemblyRecord? mainAsm = result.FindAssembly("Assembly-CSharp");
                CompiledAssemblyRecord? editorAsm = result.FindAssembly("Assembly-CSharp-Editor");
                CompiledAssemblyRecord? coreAsm = result.FindAssembly("MyGame.Core");
                CompiledAssemblyRecord? gameplayAsm = result.FindAssembly("MyGame.Gameplay");
                CompiledAssemblyRecord? editorToolsAsm = result.FindAssembly("MyGame.EditorTools");
                CompiledAssemblyRecord? featureGateAsm = result.FindAssembly("MyGame.FeatureGate");

                Check.That("Story 6 produces Assembly-CSharp-firstpass", firstPass != null && !firstPass.IsExcluded && File.Exists(firstPass.OutputPath));
                Check.That("Story 6 produces Assembly-CSharp", mainAsm != null && !mainAsm.IsExcluded && File.Exists(mainAsm.OutputPath));
                Check.That("Story 6 produces Assembly-CSharp-Editor", editorAsm != null && !editorAsm.IsExcluded && File.Exists(editorAsm.OutputPath));
                Check.That("Story 6 produces MyGame.Core", coreAsm != null && !coreAsm.IsExcluded && File.Exists(coreAsm.OutputPath));
                Check.That("Story 6 produces MyGame.Gameplay", gameplayAsm != null && !gameplayAsm.IsExcluded && File.Exists(gameplayAsm.OutputPath));
                Check.That("Story 6 includes MyGame.EditorTools in editor", editorToolsAsm != null && !editorToolsAsm.IsExcluded && File.Exists(editorToolsAsm.OutputPath));
                Check.That("Story 6 excludes MyGame.FeatureGate when defineConstraint is false", featureGateAsm != null && featureGateAsm.IsExcluded);

                Check.Equal("Story 6 MyGame.Core includes .asmref folder script", 2, coreAsm != null ? coreAsm.SourceFiles.Count : 0);
                Check.That("Story 6 MyGame.Core versionDefines adds HAS_ANALYTICS_1_2", coreAsm != null && ContainsString(coreAsm.Defines, "HAS_ANALYTICS_1_2"));

                ScriptCompilationOptions withFeatureGate = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };
                withFeatureGate.AdditionalDefines.Add("ENABLE_EXPERIMENTAL_FEATURE");
                ScriptCompilationResult gatedResult = ScriptCompilation.CompileProject(tempProject, withFeatureGate);
                Check.Equal("Story 6 compile with defineConstraint enabled succeeds", true, gatedResult.Succeeded);
                CompiledAssemblyRecord? enabledGateAsm = gatedResult.FindAssembly("MyGame.FeatureGate");
                Check.That("Story 6 MyGame.FeatureGate compiles when defineConstraint is satisfied", enabledGateAsm != null && !enabledGateAsm.IsExcluded && File.Exists(enabledGateAsm.OutputPath));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory6ReferenceCycleDiagnostic()
        {
            string tempProject = CreateEmptyTempProject("story6_cycle");
            try
            {
                string alphaDir = Path.Combine(tempProject, "Assets", "Alpha");
                string betaDir = Path.Combine(tempProject, "Assets", "Beta");
                Directory.CreateDirectory(alphaDir);
                Directory.CreateDirectory(betaDir);

                File.WriteAllText(
                    Path.Combine(alphaDir, "Cycle.Alpha.asmdef"),
                    "{\n  \"name\": \"Cycle.Alpha\",\n  \"references\": [ \"Cycle.Beta\" ]\n}\n");
                File.WriteAllText(Path.Combine(alphaDir, "AlphaScript.cs"), "public class AlphaScript {}\n");

                File.WriteAllText(
                    Path.Combine(betaDir, "Cycle.Beta.asmdef"),
                    "{\n  \"name\": \"Cycle.Beta\",\n  \"references\": [ \"Cycle.Alpha\" ]\n}\n");
                File.WriteAllText(Path.Combine(betaDir, "BetaScript.cs"), "public class BetaScript {}\n");

                ScriptCompilationResult result = ScriptCompilation.CompileProject(tempProject);
                Check.Equal("Story 6 cycle compilation fails", false, result.Succeeded);

                bool foundCycleDiag = false;
                for (int i = 0; i < result.Diagnostics.Count; i++)
                {
                    ScriptDiagnostic d = result.Diagnostics[i];
                    if (d.Severity == ScriptDiagnosticSeverity.Error
                        && d.Message.Contains("Cycle.Alpha", StringComparison.Ordinal)
                        && d.Message.Contains("Cycle.Beta", StringComparison.Ordinal))
                    {
                        foundCycleDiag = true;
                        break;
                    }
                }

                Check.Equal("Story 6 cycle diagnostic names both Cycle.Alpha and Cycle.Beta", true, foundCycleDiag);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory7TargetDefinesEditorAndPlayers()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "story7");
            try
            {
                ScriptCompilationOptions editorOpts = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    IsDevelopmentBuild = false,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64",
                    OutputDirectory = Path.Combine(tempProject, "Library", "OutEditor")
                };
                ScriptCompilationResult editorRes = ScriptCompilation.CompileProject(tempProject, editorOpts);
                Check.Equal("Story 7 editor target compiles", true, editorRes.Succeeded);
                CompiledAssemblyRecord? editorCore = editorRes.FindAssembly("MyGame.Core");
                Check.That("Story 7 editor defines UNITY_EDITOR", editorCore != null && ContainsString(editorCore.Defines, "UNITY_EDITOR"));
                Check.That("Story 7 editor defines UNITY_2022_3_OR_NEWER", editorCore != null && ContainsString(editorCore.Defines, "UNITY_2022_3_OR_NEWER"));
                Check.That("Story 7 editor defines CSC_CUSTOM_SYMBOL from csc.rsp", editorCore != null && ContainsString(editorCore.Defines, "CSC_CUSTOM_SYMBOL"));
                Check.That("Story 7 editor omits DEVELOPMENT_BUILD when not configured", editorCore != null && !ContainsString(editorCore.Defines, "DEVELOPMENT_BUILD"));

                ScriptCompilationOptions releaseStandaloneOpts = new ScriptCompilationOptions
                {
                    IsEditor = false,
                    IsDevelopmentBuild = false,
                    TargetPlatform = ScriptTargetPlatform.StandaloneWindows64,
                    TargetArchitecture = "x86_64",
                    OutputDirectory = Path.Combine(tempProject, "Library", "OutPlayerWinRelease")
                };
                ScriptCompilationResult releaseWinRes = ScriptCompilation.CompileProject(tempProject, releaseStandaloneOpts);
                Check.Equal("Story 7 StandaloneWindows64 release player compiles", true, releaseWinRes.Succeeded);
                CompiledAssemblyRecord? winCore = releaseWinRes.FindAssembly("MyGame.Core");
                CompiledAssemblyRecord? winEditorTools = releaseWinRes.FindAssembly("MyGame.EditorTools");
                CompiledAssemblyRecord? winPredefEditor = releaseWinRes.FindAssembly("Assembly-CSharp-Editor");
                Check.That("Story 7 player omits UNITY_EDITOR", winCore != null && !ContainsString(winCore.Defines, "UNITY_EDITOR"));
                Check.That("Story 7 Windows player defines UNITY_STANDALONE_WIN", winCore != null && ContainsString(winCore.Defines, "UNITY_STANDALONE_WIN"));
                Check.That("Story 7 player excludes MyGame.EditorTools", winEditorTools != null && winEditorTools.IsExcluded);
                Check.That("Story 7 player excludes Assembly-CSharp-Editor", winPredefEditor != null && winPredefEditor.IsExcluded);

                ScriptCompilationOptions devServerOpts = new ScriptCompilationOptions
                {
                    IsEditor = false,
                    IsDevelopmentBuild = true,
                    TargetPlatform = ScriptTargetPlatform.DedicatedServer,
                    TargetArchitecture = "x86_64",
                    OutputDirectory = Path.Combine(tempProject, "Library", "OutPlayerServerDev")
                };
                ScriptCompilationResult devServerRes = ScriptCompilation.CompileProject(tempProject, devServerOpts);
                Check.Equal("Story 7 DedicatedServer development player compiles", true, devServerRes.Succeeded);
                CompiledAssemblyRecord? serverCore = devServerRes.FindAssembly("MyGame.Core");
                Check.That("Story 7 server dev player defines UNITY_SERVER", serverCore != null && ContainsString(serverCore.Defines, "UNITY_SERVER"));
                Check.That("Story 7 server dev player defines DEVELOPMENT_BUILD", serverCore != null && ContainsString(serverCore.Defines, "DEVELOPMENT_BUILD"));
                Check.That("Story 7 server dev player omits UNITY_EDITOR", serverCore != null && !ContainsString(serverCore.Defines, "UNITY_EDITOR"));
                Check.That("Story 7 server dev player defines CSC_CUSTOM_SYMBOL", serverCore != null && ContainsString(serverCore.Defines, "CSC_CUSTOM_SYMBOL"));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory1And4FailedEditPreservesLastValidAndIncrementalRecompile()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "story4");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                ScriptCompilationResult initial = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("Story 4 initial compile succeeds", true, initial.Succeeded);
                Check.That("Story 4 initial state is Current", initial.ActiveState == ActiveAssemblyState.Current);

                CompiledAssemblyRecord? initialCore = initial.FindAssembly("MyGame.Core");
                CompiledAssemblyRecord? initialGameplay = initial.FindAssembly("MyGame.Gameplay");
                CompiledAssemblyRecord? initialFirstPass = initial.FindAssembly("Assembly-CSharp-firstpass");
                Check.That("Story 4 initial MyGame.Core DLL exists", initialCore != null && File.Exists(initialCore.OutputPath));
                byte[] validGameplayBytes = File.ReadAllBytes(initialGameplay!.OutputPath);

                ScriptCompilationResult noOp = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("Story 1 no-op recompile succeeds", true, noOp.Succeeded);
                Check.Equal("Story 1 no-op recompile touches 0 assemblies", 0, noOp.RecompiledAssemblies.Count);

                string gameplayScriptPath = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                string validGameplaySource = File.ReadAllText(gameplayScriptPath);
                File.WriteAllText(
                    gameplayScriptPath,
                    "using UnityEngine;\nnamespace MyGame.Gameplay\n{\n    public class GameplayController : MonoBehaviour\n    {\n        public void BrokenMethod() { int x = \"not_an_int\"; }\n    }\n}\n");

                ScriptCompilationResult failedEdit = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("Story 4 broken edit fails compilation", false, failedEdit.Succeeded);
                Check.That("Story 4 failed edit keeps LastValidFallback active", failedEdit.ActiveState == ActiveAssemblyState.LastValidFallback);
                Check.That("Story 4 failed edit diagnostic reports CS0029", HasDiagnosticWithIdAndLine(failedEdit.Diagnostics, "CS0029", 6, "MyGame.Gameplay"));

                byte[] afterFailGameplayBytes = File.ReadAllBytes(initialGameplay.OutputPath);
                Check.Equal("Story 4 failed edit preserves last valid DLL bytes", validGameplayBytes.Length, afterFailGameplayBytes.Length);
                Check.Equal("Story 4 failed edit preserves exact DLL content", true, ByteArraysEqual(validGameplayBytes, afterFailGameplayBytes));

                string fixedGameplaySource = validGameplaySource.Replace("DescribeCount(42)", "DescribeCount(99)", StringComparison.Ordinal);
                File.WriteAllText(gameplayScriptPath, fixedGameplaySource);

                ScriptCompilationResult fixedEdit = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("Story 4 fixed edit compiles", true, fixedEdit.Succeeded);
                Check.That("Story 4 fixed edit returns to Current state", fixedEdit.ActiveState == ActiveAssemblyState.Current);

                Check.That("Story 4 fixing Gameplay recompiles MyGame.Gameplay", ContainsString(fixedEdit.RecompiledAssemblies, "MyGame.Gameplay"));
                Check.That("Story 4 fixing Gameplay recompiles dependent Assembly-CSharp", ContainsString(fixedEdit.RecompiledAssemblies, "Assembly-CSharp"));
                Check.That("Story 4 fixing Gameplay leaves unaffected MyGame.Core untouched", !ContainsString(fixedEdit.RecompiledAssemblies, "MyGame.Core"));
                Check.That("Story 4 fixing Gameplay leaves unaffected Assembly-CSharp-firstpass untouched", !ContainsString(fixedEdit.RecompiledAssemblies, initialFirstPass!.Name));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory10IncompatibleNativePluginDllImportDiagnostic()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "story10");
            try
            {
                string callerPath = Path.Combine(tempProject, "Assets", "Gameplay", "NativeMathCaller.cs");
                File.WriteAllText(
                    callerPath,
                    "using System.Runtime.InteropServices;\nusing UnityEngine;\nnamespace MyGame.Gameplay\n{\n    public class NativeMathCaller : MonoBehaviour\n    {\n        [DllImport(\"libfastmath\")]\n        private static extern int fast_add(int a, int b);\n        public int CallNative() => fast_add(1, 2);\n    }\n}\n");
                File.WriteAllText(
                    callerPath + ".meta",
                    "fileFormatVersion: 2\nguid: 30000000000000000000000000000099\nMonoImporter:\n  serializedVersion: 2\n");

                ScriptCompilationOptions matchingArchOptions = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };
                ScriptCompilationResult matchingResult = ScriptCompilation.CompileProject(tempProject, matchingArchOptions);
                Check.Equal("Story 10 matching x86_64 architecture compiles cleanly", true, matchingResult.Succeeded);

                ScriptCompilationOptions mismatchedArchOptions = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "ARM64"
                };
                ScriptCompilationResult mismatchResult = ScriptCompilation.CompileProject(tempProject, mismatchedArchOptions);
                Check.Equal("Story 10 incompatible ARM64 architecture fails compilation", false, mismatchResult.Succeeded);

                bool foundPluginDiag = false;
                for (int i = 0; i < mismatchResult.Diagnostics.Count; i++)
                {
                    ScriptDiagnostic d = mismatchResult.Diagnostics[i];
                    if (d.Severity == ScriptDiagnosticSeverity.Error
                        && d.Message.Contains("libfastmath", StringComparison.Ordinal)
                        && d.Message.Contains("fast_add", StringComparison.Ordinal)
                        && d.Message.Contains("x86_64", StringComparison.Ordinal)
                        && d.Message.Contains("ARM64", StringComparison.Ordinal))
                    {
                        foundPluginDiag = true;
                        break;
                    }
                }

                Check.Equal("Story 10 diagnostic names library, target, architecture, and calling member", true, foundPluginDiag);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestGeneratedGlueAndZeroReflectionMessageDispatch()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "glue_dispatch");
            try
            {
                ScriptGlueRegistry.Clear();
                MessageDispatcher.Clear();

                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64",
                    EmitGeneratedGlue = true,
                    AutoLoadGeneratedGlue = false
                };

                ScriptCompilationResult result = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("Glue compilation succeeds", true, result.Succeeded);
                Check.That("Generated glue DLL path exists", !string.IsNullOrEmpty(result.GeneratedGlueAssemblyPath) && File.Exists(result.GeneratedGlueAssemblyPath));
                Check.That("Generated glue source exists in obj dir", !string.IsNullOrEmpty(result.GeneratedGlueSourcePath) && File.Exists(result.GeneratedGlueSourcePath));

                string permanentGluePath = Path.Combine(
                    Path.GetTempPath(),
                    "unitas_shared_glue_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture) + ".dll");
                File.Copy(result.GeneratedGlueAssemblyPath!, permanentGluePath, true);
                string? glueOutDir = Path.GetDirectoryName(result.GeneratedGlueAssemblyPath!);
                if (!string.IsNullOrEmpty(glueOutDir))
                {
                    string[] siblingDlls = Directory.GetFiles(glueOutDir, "*.dll");
                    string permanentDir = Path.GetDirectoryName(permanentGluePath) ?? Path.GetTempPath();
                    for (int i = 0; i < siblingDlls.Length; i++)
                    {
                        string fileName = Path.GetFileName(siblingDlls[i]);
                        if (!string.Equals(fileName, Path.GetFileName(result.GeneratedGlueAssemblyPath!), StringComparison.OrdinalIgnoreCase))
                        {
                            File.Copy(siblingDlls[i], Path.Combine(permanentDir, fileName), true);
                        }
                    }
                }

                LastGeneratedGlueAssemblyPath = permanentGluePath;
                ScriptGlueRegistry.LoadGlueAssembly(result.GeneratedGlueAssemblyPath!);

                bool created = ScriptGlueRegistry.TryCreate(
                    "20000000000000000000000000000002",
                    out MonoBehaviour? counterInstance,
                    out IUnitasMessageHandler? handler);

                Check.Equal("ScriptGlueRegistry creates CoreCounter by GUID", true, created);
                Check.That("CoreCounter instance is non-null", counterInstance != null);
                Check.That("CoreCounter generated message handler is non-null", handler != null);

                if (counterInstance != null && handler != null)
                {
                    Check.Equal("Generated handler dispatches public 0-arg Increment", true, handler.Dispatch("Increment"));
                    Check.Equal("Generated handler dispatches public 1-arg AddAmount", true, handler.Dispatch("AddAmount", 7));
                    Check.Equal("Generated handler dispatches public 1-arg SetLabel", true, handler.Dispatch("SetLabel", "Ready"));
                    Check.Equal("Generated handler dispatches private 0-arg OnSecretPing", true, handler.Dispatch("OnSecretPing"));
                    Check.Equal("Generated handler dispatches private 1-arg OnSecretBonus", true, handler.Dispatch("OnSecretBonus", 12));
                    Check.Equal("Generated handler returns false for unknown method", false, handler.Dispatch("NonExistentMethod"));

                    bool resolvedFromRegistry = ScriptGlueRegistry.TryGetHandler(counterInstance, out IUnitasMessageHandler? lookedUpHandler);
                    Check.Equal("ScriptGlueRegistry.TryGetHandler resolves generated handler", true, resolvedFromRegistry && lookedUpHandler != null);

                    IUnitasMessageHandler? resolvedViaDispatcher = MessageDispatcher.ResolveHandler(counterInstance);
                    Check.That("MessageDispatcher.ResolveHandler finds registered handler", resolvedViaDispatcher != null);

                    MonoBehaviour? viaGlueCreate = ScriptGlueRegistry.Create(0x2000000000000000UL, 0x0000000000000002UL);
                    Check.That("ScriptGlueRegistry.Create also resolves compiled script GUID", viaGlueCreate != null);
                }

                using (BridgeHost host = new BridgeHost())
                {
                    string scenePath = Path.Combine(tempProject, "Assets", "CounterScene.unity");
                    File.WriteAllText(
                        scenePath,
                        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_ObjectHideFlags: 0\n  serializedVersion: 6\n  m_Component:\n  - component: {fileID: 101}\n  - component: {fileID: 102}\n  m_Layer: 0\n  m_Name: CounterObject\n  m_TagString: Untagged\n  m_IsActive: 1\n--- !u!4 &101\nTransform:\n  m_GameObject: {fileID: 100}\n  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n  m_LocalPosition: {x: 0, y: 0, z: 0}\n  m_LocalScale: {x: 1, y: 1, z: 1}\n  m_Children: []\n  m_Father: {fileID: 0}\n--- !u!114 &102\nMonoBehaviour:\n  m_GameObject: {fileID: 100}\n  m_Enabled: 1\n  m_Script: {fileID: 11500000, guid: 20000000000000000000000000000002, type: 3}\n");
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    MonoBehaviour? attachedScript = host.GetScript(1UL);
                    Check.That("BridgeHost attaches compiled script by GUID on scene load", attachedScript != null);
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestCliCompileEntryPoint()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "cli_test");
            try
            {
                int exitOk = Unitas.Scripting.Program.Main(new[]
                {
                    "compile",
                    tempProject,
                    "--editor",
                    "--target",
                    "StandaloneOSX",
                    "--arch",
                    "x86_64"
                });
                Check.Equal("CLI compile valid project exits 0", 0, exitOk);
                string expectedMainDll = Path.Combine(tempProject, "Library", "UnitasScriptAssemblies", "Assembly-CSharp.dll");
                Check.Equal("CLI writes Assembly-CSharp.dll to Library/UnitasScriptAssemblies", true, File.Exists(expectedMainDll));

                string brokenFile = Path.Combine(tempProject, "Assets", "Scripts", "MainGameBootstrap.cs");
                File.WriteAllText(brokenFile, "public class MainGameBootstrap { public void Bad() { unknown_symbol(); } }\n");

                int exitFail = Unitas.Scripting.Program.Main(new[]
                {
                    "compile",
                    tempProject,
                    "--editor",
                    "--target",
                    "StandaloneOSX",
                    "--arch",
                    "x86_64"
                });
                Check.Equal("CLI compile broken project exits 1", 1, exitFail);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static bool HasDiagnosticWithIdAndLine(
            IReadOnlyList<ScriptDiagnostic> diagnostics,
            string expectedId,
            int expectedLine,
            string expectedAssembly)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                ScriptDiagnostic d = diagnostics[i];
                if (string.Equals(d.Id, expectedId, StringComparison.OrdinalIgnoreCase)
                    && d.Line == expectedLine
                    && string.Equals(d.Assembly, expectedAssembly, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool ContainsString(IReadOnlyList<string> list, string target)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (string.Equals(list[i], target, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool ByteArraysEqual(byte[] a, byte[] b)
        {
            if (a.Length != b.Length)
            {
                return false;
            }

            for (int i = 0; i < a.Length; i++)
            {
                if (a[i] != b[i])
                {
                    return false;
                }
            }

            return true;
        }

        private static string ResolveFixtureProject()
        {
            string baseDir = AppContext.BaseDirectory;
            string? current = baseDir;
            while (!string.IsNullOrEmpty(current))
            {
                string candidate = Path.Combine(current, "bridge", "Unitas.Bridge.Tests", "Fixtures", "ScriptingProject");
                if (Directory.Exists(candidate))
                {
                    return candidate;
                }

                string localCandidate = Path.Combine(current, "Fixtures", "ScriptingProject");
                if (Directory.Exists(localCandidate))
                {
                    return localCandidate;
                }

                current = Path.GetDirectoryName(current);
            }

            return Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), "bridge", "Unitas.Bridge.Tests", "Fixtures", "ScriptingProject"));
        }

        private static string CreateEmptyTempProject(string label)
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_compile_test_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            Directory.CreateDirectory(Path.Combine(tempDir, "Assets"));
            return tempDir;
        }

        private static string CloneProjectToTemp(string sourceDir, string label)
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_compile_test_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
            CopyDirectoryRecursive(sourceDir, tempDir);
            return tempDir;
        }

        private static void CopyDirectoryRecursive(string sourceDir, string targetDir)
        {
            Directory.CreateDirectory(targetDir);
            string[] files = Directory.GetFiles(sourceDir);
            for (int i = 0; i < files.Length; i++)
            {
                string fileName = Path.GetFileName(files[i]);
                File.Copy(files[i], Path.Combine(targetDir, fileName), true);
            }

            string[] dirs = Directory.GetDirectories(sourceDir);
            for (int i = 0; i < dirs.Length; i++)
            {
                string dirName = Path.GetFileName(dirs[i]);
                if (string.Equals(dirName, "Library", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(dirName, "obj", StringComparison.OrdinalIgnoreCase)
                    || string.Equals(dirName, "bin", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                CopyDirectoryRecursive(dirs[i], Path.Combine(targetDir, dirName));
            }
        }

        private static void DeleteDirectoryIfExists(string dir)
        {
            if (Directory.Exists(dir))
            {
                try
                {
                    Directory.Delete(dir, true);
                }
                catch (IOException)
                {
                }
            }
        }
    }
}
