using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Unitas.Scripting;
using Unitas.Testing;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class AgentCodeLoopTests
    {
        public static void Run()
        {
            RunAll();
        }

        public static void RunAll()
        {
            TestAut005EndToEndAgentCodeWorkflow();
        }

        private static void TestAut005EndToEndAgentCodeWorkflow()
        {
            Func<string, CodeLoopTestSummary?>? previousRunner = CodeLoop.DefaultTestRunner;
            CodeLoop.DefaultTestRunner = ExecuteProjectTests;
            string tempProject = CreateTempAut005Project("aut005_e2e");
            try
            {
                ScriptCompilationOptions options = new ScriptCompilationOptions
                {
                    IsEditor = true,
                    TargetPlatform = ScriptTargetPlatform.StandaloneOSX,
                    TargetArchitecture = "x86_64",
                    AutoLoadGeneratedGlue = true
                };

                CodeLoopReport initialReport = CodeLoop.Run(tempProject, options);
                Check.Equal("AUT-005 initial project compiles cleanly", true, initialReport.CompileSucceeded);
                Check.Equal("AUT-005 initial runningCompilation is 'new'", "new", initialReport.RunningCompilation);

                string brokenShieldSource =
                    "using UnityEngine;\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public class ShieldBooster : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int ShieldPoints = 75;\n"
                    + "        public string ActivationTag = \"ShieldReady\";\n"
                    + "\n"
                    + "        public override void Awake()\n"
                    + "        {\n"
                    + "            int bad = undeclared_aut005_shield_symbol + ShieldPoints;\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";

                const string shieldGuid = "30000000000000000000000000000077";
                ScriptAuthoringResult addedBroken = CodeLoop.AddOrUpdateScript(
                    tempProject,
                    "Assets/Gameplay/ShieldBooster.cs",
                    brokenShieldSource,
                    shieldGuid);

                Check.Equal("AUT-005 AddOrUpdateScript writes ShieldBooster.cs", true, File.Exists(addedBroken.ScriptPath));
                Check.Equal("AUT-005 AddOrUpdateScript writes ShieldBooster.cs.meta", true, File.Exists(addedBroken.MetaPath));
                Check.Equal("AUT-005 AddOrUpdateScript preserves deterministic script GUID", shieldGuid, addedBroken.Guid);

                CodeLoopReport brokenReport = CodeLoop.Run(tempProject, options);
                Check.Equal("AUT-005 broken MonoBehaviour compileSucceeded is false", false, brokenReport.CompileSucceeded);
                Check.Equal("AUT-005 broken MonoBehaviour overall succeeded is false", false, brokenReport.Succeeded);
                Check.Equal("AUT-005 broken MonoBehaviour preserves runningCompilation 'lastValid'", "lastValid", brokenReport.RunningCompilation);
                Check.That("AUT-005 broken MonoBehaviour reports errorCount >= 1", brokenReport.ErrorCount >= 1);

                DiagnoseEntry? cs0103 = FindDiagnosticById(brokenReport.Diagnostics, "CS0103");
                Check.That("AUT-005 broken MonoBehaviour returns structured CS0103 diagnostic", cs0103 != null);
                if (cs0103 != null)
                {
                    Check.Equal("AUT-005 CS0103 identifier is CS0103", "CS0103", cs0103.Identifier);
                    Check.Equal("AUT-005 CS0103 severity is error", "error", cs0103.Severity);
                    Check.Equal("AUT-005 CS0103 assembly is MyGame.Gameplay", "MyGame.Gameplay", cs0103.Assembly);
                    Check.Equal("AUT-005 CS0103 file matches ShieldBooster.cs", NormalizeRealPath(addedBroken.ScriptPath), NormalizeRealPath(cs0103.File));
                    Check.Equal("AUT-005 CS0103 line is 12", 12, cs0103.Line);
                    Check.Equal("AUT-005 CS0103 column is 23", 23, cs0103.Column);
                }

                string fixedShieldSource =
                    "using UnityEngine;\n"
                    + "\n"
                    + "namespace MyGame.Gameplay\n"
                    + "{\n"
                    + "    public class ShieldBooster : MonoBehaviour\n"
                    + "    {\n"
                    + "        public int ShieldPoints = 75;\n"
                    + "        public string ActivationTag = \"ShieldReady\";\n"
                    + "        public int TicksExecuted;\n"
                    + "\n"
                    + "        public override void Awake()\n"
                    + "        {\n"
                    + "            gameObject.name = \"HeroPrefab_\" + ActivationTag + \"_\" + ShieldPoints;\n"
                    + "            transform.position = new Vector3(ShieldPoints, 1f, 0f);\n"
                    + "        }\n"
                    + "\n"
                    + "        public override void Start()\n"
                    + "        {\n"
                    + "            transform.position = new Vector3(ShieldPoints, 2f, 0f);\n"
                    + "        }\n"
                    + "\n"
                    + "        public override void Update()\n"
                    + "        {\n"
                    + "            TicksExecuted++;\n"
                    + "            transform.position = new Vector3(ShieldPoints + TicksExecuted, 2f, 0f);\n"
                    + "        }\n"
                    + "\n"
                    + "        public int ComputeEffectiveShield(int baseArmor)\n"
                    + "        {\n"
                    + "            return baseArmor + ShieldPoints;\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n";

                ScriptAuthoringResult fixedScript = CodeLoop.AddOrUpdateScript(
                    tempProject,
                    "Assets/Gameplay/ShieldBooster.cs",
                    fixedShieldSource);
                Check.Equal("AUT-005 fixing script preserves existing .meta GUID", shieldGuid, fixedScript.Guid);

                string testFilePath = Path.Combine(tempProject, "Assets", "Tests", "EditMode", "GameplayEditTests.cs");
                File.WriteAllText(
                    testFilePath,
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
                    + "        public void ShieldBoosterComputesEffectiveShield()\n"
                    + "        {\n"
                    + "            GameObject go = new GameObject(\"ShieldActor\");\n"
                    + "            ShieldBooster booster = go.AddComponent<ShieldBooster>();\n"
                    + "            Assert.AreEqual(100, booster.ComputeEffectiveShield(25));\n"
                    + "            Object.Destroy(go);\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n");

                Dictionary<string, string> serializedFields = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["ShieldPoints"] = "120",
                    ["ActivationTag"] = "Overcharged"
                };

                PrefabComponentAttachment attachment = CodeLoop.AttachComponentToPrefab(
                    tempProject,
                    "Assets/Prefabs/Hero.prefab",
                    "Assets/Gameplay/ShieldBooster.cs",
                    targetObjectName: "HeroRoot",
                    serializedFields: serializedFields);

                Check.Equal("AUT-005 AttachComponentToPrefab resolves script GUID", shieldGuid, attachment.ScriptGuid);
                Check.Equal("AUT-005 AttachComponentToPrefab targets HeroRoot", "HeroRoot", attachment.ObjectPath);
                Check.That("AUT-005 AttachComponentToPrefab assigns valid component fileID", attachment.ComponentFileId > 0);

                ContentUsageSearchResult usagesByName = CodeLoop.FindScriptUsages(tempProject, "ShieldBooster", options);
                Check.Equal("AUT-005 FindScriptUsages by class name resolves script GUID", shieldGuid, usagesByName.ScriptGuid);
                Check.Equal("AUT-005 FindScriptUsages by class name resolves full type name", "MyGame.Gameplay.ShieldBooster", usagesByName.ScriptTypeName);
                Check.That("AUT-005 FindScriptUsages finds usage in prefab and scene", usagesByName.Usages.Count >= 1);

                ContentScriptUsageRecord? prefabUsage = null;
                for (int i = 0; i < usagesByName.Usages.Count; i++)
                {
                    if (string.Equals(usagesByName.Usages[i].AssetKind, "prefab", StringComparison.OrdinalIgnoreCase))
                    {
                        prefabUsage = usagesByName.Usages[i];
                        break;
                    }
                }

                Check.That("AUT-005 FindScriptUsages locates Hero.prefab usage", prefabUsage != null);
                if (prefabUsage != null)
                {
                    Check.Equal("AUT-005 prefab usage relative path is Assets/Prefabs/Hero.prefab", "Assets/Prefabs/Hero.prefab", prefabUsage.RelativeAssetPath);
                    Check.Equal("AUT-005 prefab usage objectPath is HeroRoot", "HeroRoot", prefabUsage.ObjectPath);
                    Check.Equal("AUT-005 prefab usage scriptGuid matches", shieldGuid, prefabUsage.ScriptGuid);
                    Check.Equal(
                        "AUT-005 prefab usage serialized field ShieldPoints is 120",
                        true,
                        prefabUsage.SerializedFields.TryGetValue("ShieldPoints", out string? shieldVal) && shieldVal == "120");
                    Check.Equal(
                        "AUT-005 prefab usage serialized field ActivationTag is Overcharged",
                        true,
                        prefabUsage.SerializedFields.TryGetValue("ActivationTag", out string? tagVal) && tagVal == "Overcharged");
                }

                ContentUsageSearchResult usagesByGuid = CodeLoop.FindScriptUsages(tempProject, shieldGuid, options);
                Check.Equal("AUT-005 FindScriptUsages by GUID returns matching usage count", usagesByName.Usages.Count, usagesByGuid.Usages.Count);

                string scenePath = Path.Combine(tempProject, "Assets", "Scenes", "Main.unity");
                CodeLoopReport fullLoopReport = CodeLoop.Run(
                    tempProject,
                    options,
                    typeFilter: "ShieldBooster",
                    testRunner: ExecuteProjectTests,
                    usageQuery: "ShieldBooster",
                    enterPlayMode: true,
                    playSceneOrPrefabPath: scenePath,
                    playTickCount: 3,
                    playFrameDeltaSeconds: 0.02);

                Check.Equal("AUT-005 full loop after fix compileSucceeded is true", true, fullLoopReport.CompileSucceeded);
                Check.Equal("AUT-005 full loop after fix overall succeeded is true", true, fullLoopReport.Succeeded);
                Check.Equal("AUT-005 full loop runningCompilation is 'new'", "new", fullLoopReport.RunningCompilation);
                Check.Equal("AUT-005 full loop errorCount is 0", 0, fullLoopReport.ErrorCount);
                Check.That("AUT-005 full loop symbol query finds ShieldBooster", fullLoopReport.Symbols != null && fullLoopReport.Symbols.FindType("MyGame.Gameplay.ShieldBooster") != null);

                Check.That("AUT-005 full loop executes and passes project tests", fullLoopReport.Tests != null && fullLoopReport.Tests.Succeeded);
                if (fullLoopReport.Tests != null)
                {
                    Check.Equal("AUT-005 full loop runs 2 project tests", 2, fullLoopReport.Tests.PassedCount);
                    Check.Equal("AUT-005 full loop has 0 test failures", 0, fullLoopReport.Tests.FailedCount);
                }

                Check.That("AUT-005 full loop includes usage search results", fullLoopReport.Usages != null && fullLoopReport.Usages.Usages.Count >= 1);
                Check.That("AUT-005 full loop enters play mode without manual refresh", fullLoopReport.PlayMode != null && fullLoopReport.PlayMode.Succeeded);

                if (fullLoopReport.PlayMode != null)
                {
                    Check.Equal("AUT-005 play mode runs on 'new' compilation", "new", fullLoopReport.PlayMode.RunningCompilation);
                    Check.Equal("AUT-005 play mode ticks 3 frames", 3, fullLoopReport.PlayMode.TicksExecuted);
                    Check.That("AUT-005 play mode instantiates and runs newly added MonoBehaviour", fullLoopReport.PlayMode.ActiveScriptCount >= 1);

                    bool sawShieldBoosterInPlay = false;
                    for (int i = 0; i < fullLoopReport.PlayMode.ExecutedBehaviours.Count; i++)
                    {
                        PlayModeBehaviourObservation obs = fullLoopReport.PlayMode.ExecutedBehaviours[i];
                        if (string.Equals(obs.FullTypeName, "MyGame.Gameplay.ShieldBooster", StringComparison.Ordinal))
                        {
                            sawShieldBoosterInPlay = true;
                            Check.Equal("AUT-005 play mode ShieldBooster is enabled", true, obs.Enabled);
                            Check.Equal("AUT-005 play mode ShieldBooster Awake mutated GameObject name", "HeroPrefab_ShieldReady_75", obs.GameObjectName);
                            Check.Equal("AUT-005 play mode ShieldBooster Update advanced position x across 3 ticks", 78f, obs.PositionX);
                            Check.Equal("AUT-005 play mode ShieldBooster Start set position y to 2", 2f, obs.PositionY);
                        }
                    }

                    Check.Equal("AUT-005 play mode executed ShieldBooster on prefab instance in scene", true, sawShieldBoosterInPlay);
                }

                TextWriter savedOut = Console.Out;
                try
                {
                    using (StringWriter usagesOut = new StringWriter())
                    {
                        Console.SetOut(usagesOut);
                        int usagesExit = Unitas.Scripting.Program.Main(new[]
                        {
                            "usages",
                            tempProject,
                            "--script",
                            "ShieldBooster"
                        });
                        Console.SetOut(savedOut);

                        Check.Equal("AUT-005 CLI usages exits 0", 0, usagesExit);
                        using (JsonDocument usagesDoc = JsonDocument.Parse(usagesOut.ToString()))
                        {
                            JsonElement root = usagesDoc.RootElement;
                            Check.Equal("AUT-005 CLI usages JSON scriptGuid matches", shieldGuid, root.GetProperty("scriptGuid").GetString() ?? string.Empty);
                            JsonElement usagesArr = root.GetProperty("usages");
                            Check.That("AUT-005 CLI usages JSON lists Hero.prefab", usagesArr.GetArrayLength() >= 1);
                        }
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

        private static DiagnoseEntry? FindDiagnosticById(IReadOnlyList<DiagnoseEntry> diagnostics, string id)
        {
            for (int i = 0; i < diagnostics.Count; i++)
            {
                if (string.Equals(diagnostics[i].Identifier, id, StringComparison.OrdinalIgnoreCase))
                {
                    return diagnostics[i];
                }
            }

            return null;
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

        private static string CreateTempAut005Project(string label)
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
                "unitas_aut005_" + label + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));

            string projectSettingsDir = Path.Combine(root, "ProjectSettings");
            string gameplayDir = Path.Combine(root, "Assets", "Gameplay");
            string prefabsDir = Path.Combine(root, "Assets", "Prefabs");
            string scenesDir = Path.Combine(root, "Assets", "Scenes");
            string editTestsDir = Path.Combine(root, "Assets", "Tests", "EditMode");

            Directory.CreateDirectory(projectSettingsDir);
            Directory.CreateDirectory(gameplayDir);
            Directory.CreateDirectory(prefabsDir);
            Directory.CreateDirectory(scenesDir);
            Directory.CreateDirectory(editTestsDir);

            File.WriteAllText(
                Path.Combine(projectSettingsDir, "ProjectVersion.txt"),
                "m_EditorVersion: 2022.3.20f1\n");

            File.WriteAllText(
                Path.Combine(gameplayDir, "MyGame.Gameplay.asmdef"),
                "{\n  \"name\": \"MyGame.Gameplay\"\n}\n");
            File.WriteAllText(
                Path.Combine(gameplayDir, "MyGame.Gameplay.asmdef.meta"),
                "fileFormatVersion: 2\nguid: 30000000000000000000000000000001\n");

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
                Path.Combine(gameplayDir, "GameplayController.cs.meta"),
                "fileFormatVersion: 2\nguid: 30000000000000000000000000000002\nMonoImporter:\n  serializedVersion: 2\n");

            const string heroPrefabGuid = "40000000000000000000000000000001";
            File.WriteAllText(
                Path.Combine(prefabsDir, "Hero.prefab"),
                "%YAML 1.1\n"
                + "%TAG !u! tag:unity3d.com,2011:\n"
                + "--- !u!1 &2400000\n"
                + "GameObject:\n"
                + "  m_ObjectHideFlags: 0\n"
                + "  m_Component:\n"
                + "  - component: {fileID: 400000}\n"
                + "  m_Layer: 0\n"
                + "  m_Name: HeroRoot\n"
                + "  m_IsActive: 1\n"
                + "--- !u!4 &400000\n"
                + "Transform:\n"
                + "  m_ObjectHideFlags: 0\n"
                + "  m_GameObject: {fileID: 2400000}\n"
                + "  serializedVersion: 2\n"
                + "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n"
                + "  m_LocalPosition: {x: 0, y: 0, z: 0}\n"
                + "  m_LocalScale: {x: 1, y: 1, z: 1}\n"
                + "  m_Children: []\n"
                + "  m_Father: {fileID: 0}\n");
            File.WriteAllText(
                Path.Combine(prefabsDir, "Hero.prefab.meta"),
                "fileFormatVersion: 2\nguid: " + heroPrefabGuid + "\nNativeFormatImporter:\n  mainObjectFileID: 100100000\n");

            File.WriteAllText(
                Path.Combine(scenesDir, "Main.unity"),
                "%YAML 1.1\n"
                + "%TAG !u! tag:unity3d.com,2011:\n"
                + "--- !u!1 &1000\n"
                + "GameObject:\n"
                + "  m_ObjectHideFlags: 0\n"
                + "  m_Component:\n"
                + "  - component: {fileID: 1001}\n"
                + "  m_Layer: 0\n"
                + "  m_Name: ArenaRoot\n"
                + "  m_IsActive: 1\n"
                + "--- !u!4 &1001\n"
                + "Transform:\n"
                + "  m_ObjectHideFlags: 0\n"
                + "  m_GameObject: {fileID: 1000}\n"
                + "  serializedVersion: 2\n"
                + "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n"
                + "  m_LocalPosition: {x: 0, y: 0, z: 0}\n"
                + "  m_LocalScale: {x: 1, y: 1, z: 1}\n"
                + "  m_Children: []\n"
                + "  m_Father: {fileID: 0}\n"
                + "--- !u!1001 &2000\n"
                + "PrefabInstance:\n"
                + "  m_ObjectHideFlags: 0\n"
                + "  serializedVersion: 2\n"
                + "  m_Modification:\n"
                + "    serializedVersion: 3\n"
                + "    m_TransformParent: {fileID: 1001}\n"
                + "    m_Modifications: []\n"
                + "    m_RemovedComponents: []\n"
                + "    m_RemovedGameObjects: []\n"
                + "    m_AddedGameObjects: []\n"
                + "    m_AddedComponents: []\n"
                + "  m_SourcePrefab: {fileID: 100100000, guid: " + heroPrefabGuid + ", type: 3}\n");

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
