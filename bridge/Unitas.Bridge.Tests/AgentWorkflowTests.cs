using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Unitas.Scripting;
using Unitas.Testing;

namespace Unitas.Bridge.Tests
{
    public static class AgentWorkflowTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            TestAgentCodeLoopEndToEndAndCliLoopCommand();
        }

        private static void TestAgentCodeLoopEndToEndAndCliLoopCommand()
        {
            Func<string, CodeLoopTestSummary?>? previousRunner = CodeLoop.DefaultTestRunner;
            CodeLoop.DefaultTestRunner = ExecuteProjectTests;
            string tempProject = CreateTempAgentWorkflowProject("aut005_loop");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64"
                };

                string monoBehaviourPath = Path.Combine(tempProject, "Assets", "Gameplay", "GameplayController.cs");
                string originalMonoBehaviourSource = File.ReadAllText(monoBehaviourPath);

                string brokenSource =
                    "using UnityEngine;\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public class GameplayController : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int Health = 100;\n"
                    + "\n"
                    + "        public int ComputeNextScore()\n"
                    + "        {\n"
                    + "            return undeclared_agent_loop_identifier + Health;\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";

                TextWriter savedOut = Console.Out;
                try
                {
                    CodeLoopReport cleanReport;
                    using (StringWriter cleanCliOut = new StringWriter())
                    {
                        Console.SetOut(cleanCliOut);
                        int cleanCliExit = Unitas.Scripting.Program.Main(new[]
                        {
                            "loop",
                            tempProject,
                            "--editor",
                            "--target",
                            "StandaloneOSX",
                            "--arch",
                            "x86_64"
                        });
                        Console.SetOut(savedOut);

                        Check.Equal("AgentWorkflow CLI loop: initial clean state exits 0", 0, cleanCliExit);
                        using (JsonDocument cleanDoc = JsonDocument.Parse(cleanCliOut.ToString()))
                        {
                            JsonElement root = cleanDoc.RootElement;
                            Check.Equal("AgentWorkflow loop: clean compile and test run succeeds", true, root.GetProperty("succeeded").GetBoolean());
                            Check.Equal("AgentWorkflow loop: clean compileSucceeded is true", true, root.GetProperty("compileSucceeded").GetBoolean());
                            Check.Equal("AgentWorkflow loop: clean runningCompilation is 'new'", "new", root.GetProperty("runningCompilation").GetString() ?? string.Empty);
                            Check.Equal("AgentWorkflow loop: clean errorCount is 0", 0, root.GetProperty("errorCount").GetInt32());
                            Check.That("AgentWorkflow loop: clean report discovers project symbols", root.GetProperty("symbolCount").GetInt32() >= 1);
                            JsonElement testsElem = root.GetProperty("tests");
                            Check.Equal("AgentWorkflow loop: clean report executes project tests", true, testsElem.GetProperty("succeeded").GetBoolean());
                            Check.Equal("AgentWorkflow loop: clean test run passes 2 tests", 2, testsElem.GetProperty("passedCount").GetInt32());
                            Check.Equal("AgentWorkflow loop: clean test run has 0 failures", 0, testsElem.GetProperty("failedCount").GetInt32());
                            Check.Equal("AgentWorkflow loop JSON: clean tests.results count 2", 2, testsElem.GetProperty("results").GetArrayLength());
                        }
                    }

                    File.WriteAllText(monoBehaviourPath, brokenSource);

                    using (StringWriter brokenCliOut = new StringWriter())
                    {
                        Console.SetOut(brokenCliOut);
                        int brokenCliExit = Unitas.Scripting.Program.Main(new[]
                        {
                            "loop",
                            tempProject,
                            "--editor",
                            "--target",
                            "StandaloneOSX",
                            "--arch",
                            "x86_64"
                        });
                        Console.SetOut(savedOut);

                        Check.Equal("AgentWorkflow CLI loop: broken state exits 1", 1, brokenCliExit);
                        using (JsonDocument brokenDoc = JsonDocument.Parse(brokenCliOut.ToString()))
                        {
                            JsonElement root = brokenDoc.RootElement;
                            Check.Equal("AgentWorkflow loop: broken MonoBehaviour edit reports succeeded false", false, root.GetProperty("succeeded").GetBoolean());
                            Check.Equal("AgentWorkflow loop: broken MonoBehaviour edit reports compileSucceeded false", false, root.GetProperty("compileSucceeded").GetBoolean());
                            Check.Equal("AgentWorkflow loop: broken edit preserves runningCompilation 'lastValid'", "lastValid", root.GetProperty("runningCompilation").GetString() ?? string.Empty);
                            Check.That("AgentWorkflow loop: broken edit reports errorCount >= 1", root.GetProperty("errorCount").GetInt32() >= 1);
                            Check.Equal("AgentWorkflow loop: broken edit skips test execution", JsonValueKind.Null.ToString(), root.GetProperty("tests").ValueKind.ToString());
                            JsonElement diags = root.GetProperty("diagnostics");
                            Check.That("AgentWorkflow loop JSON: broken diagnostics non-empty", diags.GetArrayLength() >= 1);
                            JsonElement firstDiag = diags[0];
                            Check.Equal("AgentWorkflow loop: broken edit diagnostic includes CS0103", "CS0103", firstDiag.GetProperty("identifier").GetString() ?? string.Empty);
                            Check.Equal("AgentWorkflow loop: CS0103 severity is error", "error", firstDiag.GetProperty("severity").GetString() ?? string.Empty);
                            Check.Equal("AgentWorkflow loop: CS0103 assembly is MyGame.Gameplay", "MyGame.Gameplay", firstDiag.GetProperty("assembly").GetString() ?? string.Empty);
                            Check.Equal("AgentWorkflow loop: CS0103 file matches GameplayController.cs", NormalizeRealPath(monoBehaviourPath), NormalizeRealPath(firstDiag.GetProperty("file").GetString() ?? string.Empty));
                            Check.Equal("AgentWorkflow loop: CS0103 line is 11", 11, firstDiag.GetProperty("line").GetInt32());
                            Check.Equal("AgentWorkflow loop: CS0103 column is 20", 20, firstDiag.GetProperty("column").GetInt32());
                        }
                    }

                    File.WriteAllText(monoBehaviourPath, originalMonoBehaviourSource);

                    cleanReport = CodeLoop.Run(tempProject, options, "GameplayController");
                    Check.Equal("AgentWorkflow loop: fixed MonoBehaviour succeeds", true, cleanReport.Succeeded);
                    Check.Equal("AgentWorkflow loop: fixed MonoBehaviour runningCompilation is 'new'", "new", cleanReport.RunningCompilation);
                    Check.Equal("AgentWorkflow loop: fixed MonoBehaviour errorCount is 0", 0, cleanReport.ErrorCount);
                    Check.That("AgentWorkflow loop: fixed MonoBehaviour symbol query finds GameplayController", cleanReport.Symbols != null && cleanReport.Symbols.FindType("MyGame.Gameplay.GameplayController") != null);
                    Check.That("AgentWorkflow loop: fixed MonoBehaviour runs and passes tests", cleanReport.Tests != null && cleanReport.Tests.Succeeded && cleanReport.Tests.PassedCount == 2);

                    using (JsonDocument fixedDoc = JsonDocument.Parse(cleanReport.ToJson()))
                    {
                        JsonElement root = fixedDoc.RootElement;
                        Check.Equal("AgentWorkflow CLI loop: clean JSON succeeded is true", true, root.GetProperty("succeeded").GetBoolean());
                        Check.Equal("AgentWorkflow CLI loop: clean JSON runningCompilation is 'new'", "new", root.GetProperty("runningCompilation").GetString() ?? string.Empty);
                        Check.Equal("AgentWorkflow CLI loop: clean JSON errorCount is 0", 0, root.GetProperty("errorCount").GetInt32());
                        Check.Equal("AgentWorkflow CLI loop: clean JSON symbolCount is 1 for --type GameplayController", 1, root.GetProperty("symbolCount").GetInt32());
                        Check.Equal("AgentWorkflow CLI loop: clean JSON tests.passedCount is 2", 2, root.GetProperty("tests").GetProperty("passedCount").GetInt32());
                    }
                }
                finally
                {
                    Console.SetOut(savedOut);
                }
            }
            finally
            {
                CodeLoop.DefaultTestRunner = previousRunner;
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static CodeLoopTestSummary ExecuteProjectTests(string projectRoot)
        {
            TestRunResult testResult = TestRunner.RunProject(
                projectRoot,
                new TestRunOptions
                {
                    Mode = TestModeFilter.All
                });

            List<CodeLoopTestCaseRecord> records = new List<CodeLoopTestCaseRecord>(testResult.Results.Count);
            for (int i = 0; i < testResult.Results.Count; i++)
            {
                TestCaseResult r = testResult.Results[i];
                records.Add(new CodeLoopTestCaseRecord(
                    r.Descriptor.FullName,
                    r.Outcome.ToString(),
                    r.DurationSeconds,
                    r.Message));
            }

            return new CodeLoopTestSummary(
                testResult.Succeeded,
                testResult.SelectedTestCaseCount,
                testResult.CompletedCount,
                testResult.PassedCount,
                testResult.FailedCount,
                testResult.SkippedCount,
                records);
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

        private static string CreateTempAgentWorkflowProject(string label)
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

            string root = Path.Combine(
                baseTemp,
                "unitas_agent_workflow_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));

            string gameplayDir = Path.Combine(root, "Assets", "Gameplay");
            string editTestsDir = Path.Combine(root, "Assets", "Tests", "EditMode");
            Directory.CreateDirectory(gameplayDir);
            Directory.CreateDirectory(editTestsDir);

            File.WriteAllText(
                Path.Combine(gameplayDir, "MyGame.Gameplay.asmdef"),
                "{\n  \"name\": \"MyGame.Gameplay\"\n}\n");

            File.WriteAllText(
                Path.Combine(gameplayDir, "GameplayController.cs"),
                "using UnityEngine;\n"
                + "\n"
                + "namespace MyGame.Gameplay\n"
                + "{\n"
                + "    public class GameplayController : MonoBehaviour\n"
                + "    {\n"
                + "        public int Health = 100;\n"
                + "\n"
                + "        public int AddScore(int baseScore, int bonus)\n"
                + "        {\n"
                + "            return baseScore + bonus;\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            File.WriteAllText(
                Path.Combine(editTestsDir, "MyGame.Tests.EditMode.asmdef"),
                "{\n  \"name\": \"MyGame.Tests.EditMode\",\n  \"references\": [ \"MyGame.Gameplay\" ],\n  \"includePlatforms\": [ \"Editor\" ],\n  \"defineConstraints\": [ \"UNITY_INCLUDE_TESTS\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(editTestsDir, "GameplayEditTests.cs"),
                "using NUnit.Framework;\n"
                + "using MyGame.Gameplay;\n"
                + "using UnityEngine;\n"
                + "\n"
                + "namespace MyGame.EditTests\n"
                + "{\n"
                + "    public class GameplayEditTests\n"
                + "    {\n"
                + "        [Test]\n"
                + "        public void ControllerDefaultHealthIs100()\n"
                + "        {\n"
                + "            GameObject go = new GameObject(\"AgentLoopActor\");\n"
                + "            GameplayController controller = go.AddComponent<GameplayController>();\n"
                + "            Assert.AreEqual(100, controller.Health);\n"
                + "            Object.Destroy(go);\n"
                + "        }\n"
                + "\n"
                + "        [Test]\n"
                + "        public void ControllerAddScoreComputesSum()\n"
                + "        {\n"
                + "            GameObject go = new GameObject(\"AgentLoopScoreActor\");\n"
                + "            GameplayController controller = go.AddComponent<GameplayController>();\n"
                + "            Assert.AreEqual(42, controller.AddScore(30, 12));\n"
                + "            Object.Destroy(go);\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            return root;
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
