using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Unitas.Scripting;

namespace Unitas.Bridge.Tests
{
    public static class IdeGenerationTests
    {
        public static void Run()
        {
            TestGenerateIdeProjectFilesAndParityWithCompilation();
            TestDeterministicRegenerationAndTargetChanges();
            TestStory3ExternalSdkBuildReportsExactSameCompileDiagnostic();
            TestCliGenerateIdeCommand();
            CodeLoopTests.RunAll();
        }

        private static void TestGenerateIdeProjectFilesAndParityWithCompilation()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "ide_parity");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };
                options.PackageVersions["com.mygame.analytics"] = "1.4.0";

                ScriptCompilationResult compileResult = ScriptCompilation.CompileProject(tempProject, options);
                Check.Equal("IDE parity: ScriptCompilation succeeds", true, compileResult.Succeeded);

                IdeGenerationResult ideResult = IdeProjectGenerator.Generate(tempProject, options);
                Check.Equal("IDE parity: IdeProjectGenerator succeeds", true, ideResult.Succeeded);
                Check.That("IDE parity: solution file exists at root", File.Exists(ideResult.SolutionPath));
                Check.Equal(
                    "IDE parity: solution sits at project root",
                    Path.GetFullPath(tempProject),
                    Path.GetFullPath(Path.GetDirectoryName(ideResult.SolutionPath) ?? string.Empty));

                string[] expectedActiveAssemblies =
                {
                    "Assembly-CSharp-firstpass",
                    "Assembly-CSharp",
                    "Assembly-CSharp-Editor",
                    "MyGame.Core",
                    "MyGame.Gameplay",
                    "MyGame.EditorTools"
                };

                Check.Equal("IDE parity: active project count is 6", expectedActiveAssemblies.Length, ideResult.Projects.Count);
                Check.That("IDE parity: excluded MyGame.FeatureGate has no .csproj", ideResult.FindProject("MyGame.FeatureGate") == null);
                Check.Equal(
                    "IDE parity: excluded MyGame.FeatureGate.csproj not written on disk",
                    false,
                    File.Exists(Path.Combine(tempProject, "MyGame.FeatureGate.csproj")));

                string slnContent = File.ReadAllText(ideResult.SolutionPath);

                for (int i = 0; i < expectedActiveAssemblies.Length; i++)
                {
                    string asmName = expectedActiveAssemblies[i];
                    IdeAssemblyProjectDescriptor? ideProj = ideResult.FindProject(asmName);
                    CompiledAssemblyRecord? compiledAsm = compileResult.FindAssembly(asmName);

                    Check.That("IDE parity: descriptor exists for " + asmName, ideProj != null);
                    Check.That("IDE parity: compiled record exists for " + asmName, compiledAsm != null);

                    if (ideProj == null || compiledAsm == null)
                    {
                        continue;
                    }

                    string expectedCsprojPath = Path.Combine(tempProject, asmName + ".csproj");
                    Check.Equal("IDE parity: csproj path at root for " + asmName, expectedCsprojPath, ideProj.CsprojPath);
                    Check.Equal("IDE parity: csproj file exists for " + asmName, true, File.Exists(ideProj.CsprojPath));
                    Check.That("IDE parity: sln lists " + asmName, slnContent.Contains("\"" + asmName + ".csproj\"", StringComparison.Ordinal));

                    Check.Equal("IDE parity: define count matches for " + asmName, compiledAsm.Defines.Count, ideProj.Defines.Count);
                    for (int d = 0; d < compiledAsm.Defines.Count; d++)
                    {
                        Check.Equal(
                            "IDE parity: define[" + d.ToString(CultureInfo.InvariantCulture) + "] matches for " + asmName,
                            compiledAsm.Defines[d],
                            ideProj.Defines[d]);
                    }

                    Check.Equal("IDE parity: source count matches for " + asmName, compiledAsm.SourceFiles.Count, ideProj.SourceFiles.Count);
                    for (int s = 0; s < compiledAsm.SourceFiles.Count; s++)
                    {
                        string expectedRel = ProjectMetadataIndex.GetRelativePathNormalized(tempProject, compiledAsm.SourceFiles[s]);
                        Check.Equal(
                            "IDE parity: source[" + s.ToString(CultureInfo.InvariantCulture) + "] matches for " + asmName,
                            expectedRel,
                            ideProj.SourceFiles[s]);
                    }

                    List<string> expectedDeps = new List<string>(compiledAsm.AssemblyReferences);
                    expectedDeps.Sort(StringComparer.Ordinal);
                    Check.Equal("IDE parity: project reference count matches for " + asmName, expectedDeps.Count, ideProj.ProjectReferences.Count);
                    for (int r = 0; r < expectedDeps.Count; r++)
                    {
                        Check.Equal(
                            "IDE parity: projectRef[" + r.ToString(CultureInfo.InvariantCulture) + "] matches for " + asmName,
                            expectedDeps[r],
                            ideProj.ProjectReferences[r]);
                    }

                    string csprojXml = File.ReadAllText(ideProj.CsprojPath);
                    Check.That(
                        "IDE parity: csproj disables default compile items for " + asmName,
                        csprojXml.Contains("<EnableDefaultCompileItems>false</EnableDefaultCompileItems>", StringComparison.Ordinal));
                    Check.That(
                        "IDE parity: csproj disables assembly info generation for " + asmName,
                        csprojXml.Contains("<GenerateAssemblyInfo>false</GenerateAssemblyInfo>", StringComparison.Ordinal));
                    Check.That(
                        "IDE parity: csproj references UnityEngine for " + asmName,
                        csprojXml.Contains("<Reference Include=\"UnityEngine\">", StringComparison.Ordinal));
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestDeterministicRegenerationAndTargetChanges()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "ide_determinism");
            try
            {
                ScriptCompilationOptions editorOptions = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                IdeGenerationResult firstRun = IdeProjectGenerator.Generate(tempProject, editorOptions);
                Check.Equal("IDE determinism: first generation succeeds", true, firstRun.Succeeded);
                string firstSln = File.ReadAllText(firstRun.SolutionPath);
                string firstCoreCsproj = File.ReadAllText(Path.Combine(tempProject, "MyGame.Core.csproj"));

                IdeGenerationResult secondRun = IdeProjectGenerator.Generate(tempProject, editorOptions);
                Check.Equal("IDE determinism: second generation succeeds", true, secondRun.Succeeded);
                string secondSln = File.ReadAllText(secondRun.SolutionPath);
                string secondCoreCsproj = File.ReadAllText(Path.Combine(tempProject, "MyGame.Core.csproj"));

                Check.Equal("IDE determinism: solution output is byte-identical across runs", firstSln, secondSln);
                Check.Equal("IDE determinism: csproj output is byte-identical across runs", firstCoreCsproj, secondCoreCsproj);

                ScriptCompilationOptions playerOptions = new ScriptCompilationOptions
                {
                    IsEditor = false,
                    IsDevelopmentBuild = true,
                    TargetPlatform = ScriptTargetPlatform.DedicatedServer,
                    TargetArchitecture = "x86_64"
                };
                playerOptions.AdditionalDefines.Add("ENABLE_EXPERIMENTAL_FEATURE");

                IdeGenerationResult playerRun = IdeProjectGenerator.Generate(tempProject, playerOptions);
                Check.Equal("IDE determinism: target switch generation succeeds", true, playerRun.Succeeded);
                Check.That("IDE determinism: player target excludes MyGame.EditorTools", playerRun.FindProject("MyGame.EditorTools") == null);
                Check.Equal(
                    "IDE determinism: player target removes stale MyGame.EditorTools.csproj",
                    false,
                    File.Exists(Path.Combine(tempProject, "MyGame.EditorTools.csproj")));
                Check.That("IDE determinism: player target excludes Assembly-CSharp-Editor", playerRun.FindProject("Assembly-CSharp-Editor") == null);
                Check.Equal(
                    "IDE determinism: player target removes stale Assembly-CSharp-Editor.csproj",
                    false,
                    File.Exists(Path.Combine(tempProject, "Assembly-CSharp-Editor.csproj")));
                Check.That("IDE determinism: define ENABLE_EXPERIMENTAL_FEATURE includes MyGame.FeatureGate", playerRun.FindProject("MyGame.FeatureGate") != null);
                Check.Equal(
                    "IDE determinism: define ENABLE_EXPERIMENTAL_FEATURE writes MyGame.FeatureGate.csproj",
                    true,
                    File.Exists(Path.Combine(tempProject, "MyGame.FeatureGate.csproj")));

                IdeAssemblyProjectDescriptor? serverCore = playerRun.FindProject("MyGame.Core");
                Check.That("IDE determinism: server player MyGame.Core includes UNITY_SERVER", serverCore != null && ContainsString(serverCore.Defines, "UNITY_SERVER"));
                Check.That("IDE determinism: server player MyGame.Core omits UNITY_EDITOR", serverCore != null && !ContainsString(serverCore.Defines, "UNITY_EDITOR"));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory3ExternalSdkBuildReportsExactSameCompileDiagnostic()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "story3_ide_build");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                IdeGenerationResult cleanGen = IdeProjectGenerator.Generate(tempProject, options);
                Check.Equal("Story 3: clean project IDE generation succeeds", true, cleanGen.Succeeded);

                IdeAssemblyProjectDescriptor? cleanGameplayProj = cleanGen.FindProject("MyGame.Gameplay");
                Check.That("Story 3: clean MyGame.Gameplay.csproj descriptor exists", cleanGameplayProj != null);
                IReadOnlyList<ScriptDiagnostic> cleanDiagnostics = SymbolQuery.EvaluateCsprojDiagnostics(cleanGameplayProj!.CsprojPath);
                Check.Equal("Story 3: generated csproj evaluation succeeds for valid project", 0, CountErrors(cleanDiagnostics));

                string brokenFile = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                string brokenContent = string.Join(
                    "\n",
                    new[]
                    {
                        "using MyGame.Core;",
                        "using UnityEngine;",
                        "",
                        "namespace MyGame.Gameplay",
                        "{",
                        "    public class GameplayController : MonoBehaviour",
                        "    {",
                        "        public int TriggerDeliberateCompileError()",
                        "        {",
                        "            int brokenValue = \"cannot_convert_string_to_int\";",
                        "            return brokenValue;",
                        "        }",
                        "    }",
                        "}",
                        ""
                    });
                File.WriteAllText(brokenFile, brokenContent);

                DiagnoseReport unitasDiagnose = ScriptCompilation.DiagnoseProject(tempProject, options);
                Check.Equal("Story 3: unitas compile fails on deliberate error", false, unitasDiagnose.Succeeded);

                ScriptDiagnostic? unitasError = FindFirstErrorDiagnostic(unitasDiagnose.CompilationResult.Diagnostics);
                Check.That("Story 3: unitas reports an error diagnostic", unitasError != null);

                IdeGenerationResult brokenGen = IdeProjectGenerator.Generate(tempProject, options);
                Check.Equal("Story 3: IDE generation succeeds on project with script syntax/type error", true, brokenGen.Succeeded);

                IdeAssemblyProjectDescriptor? gameplayProj = brokenGen.FindProject("MyGame.Gameplay");
                Check.That("Story 3: MyGame.Gameplay.csproj descriptor exists", gameplayProj != null);

                IReadOnlyList<ScriptDiagnostic> externalDiagnostics = SymbolQuery.EvaluateCsprojDiagnostics(gameplayProj!.CsprojPath);

                ScriptDiagnostic? externalError = FindFirstErrorDiagnostic(externalDiagnostics);
                Check.That("Story 3: external IDE csproj evaluation reports a parsed error diagnostic", externalError != null);

                if (unitasError != null && externalError != null)
                {
                    Check.Equal("Story 3: external IDE build reports exact same diagnostic ID (CS0029)", unitasError.Id, externalError.Id);
                    Check.Equal(
                        "Story 3: external IDE build reports exact same source file",
                        NormalizeRealPath(unitasError.File),
                        NormalizeRealPath(externalError.File));
                    Check.Equal("Story 3: external IDE build reports exact same line number", unitasError.Line, externalError.Line);
                    Check.Equal("Story 3: external IDE build reports exact same column number", unitasError.Column, externalError.Column);
                }
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestCliGenerateIdeCommand()
        {
            string fixtureRoot = ResolveFixtureProject();
            string tempProject = CloneProjectToTemp(fixtureRoot, "cli_generate_ide");
            try
            {
                int exitCode = Unitas.Scripting.Program.Main(new[]
                {
                    "generate-ide",
                    tempProject,
                    "--editor",
                    "--target",
                    "StandaloneOSX",
                    "--arch",
                    "x86_64"
                });

                Check.Equal("CLI generate-ide exits 0", 0, exitCode);
                Check.Equal(
                    "CLI generate-ide creates MyGame.Core.csproj at root",
                    true,
                    File.Exists(Path.Combine(tempProject, "MyGame.Core.csproj")));
                Check.Equal(
                    "CLI generate-ide creates Assembly-CSharp.csproj at root",
                    true,
                    File.Exists(Path.Combine(tempProject, "Assembly-CSharp.csproj")));

                string projectName = Path.GetFileName(tempProject);
                Check.Equal(
                    "CLI generate-ide creates <ProjectName>.sln at root",
                    true,
                    File.Exists(Path.Combine(tempProject, projectName + ".sln")));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static ScriptDiagnostic? FindFirstErrorDiagnostic(IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (diagnostics[i].Severity == ScriptDiagnosticSeverity.Error)
                {
                    return diagnostics[i];
                }
            }

            return null;
        }

        private static int CountErrors(IReadOnlyList<ScriptDiagnostic> diagnostics)
        {
            int count = 0;
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (diagnostics[i].Severity == ScriptDiagnosticSeverity.Error)
                {
                    count++;
                }
            }

            return count;
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

        private static string NormalizeRealPath(string path)
        {
            string full = Path.GetFullPath(path).Replace('\\', '/');
            if (full.StartsWith("/private/var/", StringComparison.Ordinal))
            {
                return full.Substring("/private".Length);
            }

            if (full.StartsWith("/private/tmp/", StringComparison.Ordinal))
            {
                return full.Substring("/private".Length);
            }

            return full;
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

        private static string CloneProjectToTemp(string sourceDir, string label)
        {
            string baseTemp = Path.GetTempPath();
            if (OperatingSystem.IsMacOS())
            {
                if (baseTemp.StartsWith("/var/", StringComparison.Ordinal) && Directory.Exists("/private" + baseTemp))
                {
                    baseTemp = "/private" + baseTemp;
                }
                else if (baseTemp.StartsWith("/tmp/", StringComparison.Ordinal) && Directory.Exists("/private" + baseTemp))
                {
                    baseTemp = "/private" + baseTemp;
                }
            }
            string tempDir = Path.Combine(
                baseTemp,
                "unitas_ide_test_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));
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
