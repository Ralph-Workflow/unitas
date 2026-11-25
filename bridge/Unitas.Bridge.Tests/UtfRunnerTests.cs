using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Threading;
using System.Xml;
using Unitas.Testing;

namespace Unitas.Bridge.Tests
{
    public static class UtfRunnerTests
    {
        public static void Run()
        {
            TestStory15EditModeAndPlayModeTestsWithCategoryFilterAndNUnit3Xml();
            TestStory16UnexpectedErrorLogFailsAndLogAssertExpectPasses();
            TestStory17CancelledTestRunReportsOnlyCompletedTests();
            TestUnityPlatformRetryPrebuildSetupPostBuildCleanupAndNamePatterns();
            TestPlayModeFreshSceneIsolationAndEditStateRestoration();
            TestBrokenTestAssemblyReportsCompileFailure();
            TestCliRunCommandExitCodes();
        }

        private static void TestStory15EditModeAndPlayModeTestsWithCategoryFilterAndNUnit3Xml()
        {
            string tempProject = CreateTempTestProject("shared_a");
            try
            {
                string allXmlPath = Path.Combine(tempProject, "TestResults", "all.xml");
                TestRunResult allResult = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.All,
                        ResultsXmlPath = allXmlPath
                    });

                Check.Equal("Story 15 all-mode test compilation succeeds", true, allResult.CompileSucceeded);
                Check.Equal("Story 15 discovers 2 test assemblies (EditMode and PlayMode)", 2, allResult.Assemblies.Count);

                bool sawEditModeAsm = false;
                bool sawPlayModeAsm = false;
                for (int i = 0; i < allResult.Assemblies.Count; i++)
                {
                    DiscoveredTestAssembly asm = allResult.Assemblies[i];
                    if (string.Equals(asm.Name, "MyGame.Tests.EditMode", StringComparison.Ordinal)
                        && asm.Mode == TestExecutionMode.EditMode)
                    {
                        sawEditModeAsm = true;
                    }
                    else if (string.Equals(asm.Name, "MyGame.Tests.PlayMode", StringComparison.Ordinal)
                        && asm.Mode == TestExecutionMode.PlayMode)
                    {
                        sawPlayModeAsm = true;
                    }
                }

                Check.Equal("Story 15 recognizes EditMode assembly", true, sawEditModeAsm);
                Check.Equal("Story 15 recognizes PlayMode assembly", true, sawPlayModeAsm);

                string smokeXmlPath = Path.Combine(tempProject, "TestResults", "smoke.xml");
                TestRunResult smokeHeadless = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.All,
                        CategoryFilter = "Smoke",
                        ResultsXmlPath = smokeXmlPath,
                        SkipCompilation = true
                    });

                Check.Equal("Story 15 Smoke category run compiles cleanly", true, smokeHeadless.CompileSucceeded);
                Check.Equal("Story 15 Smoke category run succeeds with 0 failures", true, smokeHeadless.Succeeded);
                Check.Equal("Story 15 Smoke category selects 5 test cases across EditMode and PlayMode", 5, smokeHeadless.SelectedTestCaseCount);
                Check.Equal("Story 15 Smoke category passes all 5 test cases", 5, smokeHeadless.PassedCount);
                Check.Equal("Story 15 Smoke category has 0 failed test cases", 0, smokeHeadless.FailedCount);

                TestCaseResult? editSimple = smokeHeadless.FindTest("MyGame.EditTests.MathEditTests.SimpleAddition");
                TestCaseResult? editCase1 = smokeHeadless.FindTest("MyGame.EditTests.MathEditTests.MultiplyCases(2,3,6)");
                TestCaseResult? editCase2 = smokeHeadless.FindTest("MyGame.EditTests.MathEditTests.MultiplyCases(4,5,20)");
                TestCaseResult? playTest = smokeHeadless.FindTest("MyGame.PlayTests.GameplayPlayTests.GameObjectSpawnInPlayMode");
                TestCaseResult? playUnityTest = smokeHeadless.FindTest("MyGame.PlayTests.GameplayPlayTests.MultiFrameCoroutineSteps");
                TestCaseResult? excludedSlow = smokeHeadless.FindTest("MyGame.EditTests.MathEditTests.SlowNonSmokeCalculation");

                Check.That("Story 15 ran EditMode [Test] SimpleAddition", editSimple != null && editSimple.Outcome == TestOutcome.Passed);
                Check.That("Story 15 ran EditMode [TestCase(2,3,6)]", editCase1 != null && editCase1.Outcome == TestOutcome.Passed);
                Check.That("Story 15 ran EditMode [TestCase(4,5,20)]", editCase2 != null && editCase2.Outcome == TestOutcome.Passed);
                Check.That("Story 15 ran PlayMode [Test] GameObjectSpawnInPlayMode", playTest != null && playTest.Outcome == TestOutcome.Passed);
                Check.That("Story 15 ran PlayMode [UnityTest] MultiFrameCoroutineSteps", playUnityTest != null && playUnityTest.Outcome == TestOutcome.Passed);
                Check.That("Story 15 excluded non-matching category test SlowNonSmokeCalculation", excludedSlow == null);

                Check.Equal("Story 15 writes NUnit 3 XML file to disk", true, File.Exists(smokeXmlPath));
                ValidateNUnit3Xml(
                    File.ReadAllText(smokeXmlPath),
                    expectedSelected: 5,
                    expectedTotal: 5,
                    expectedPassed: 5,
                    expectedFailed: 0,
                    expectedOverallResult: "Passed");

                TestRunResult editOnly = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.EditMode,
                        CategoryFilter = "Smoke",
                        SkipCompilation = true
                    });
                TestRunResult playOnly = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.PlayMode,
                        CategoryFilter = "Smoke",
                        SkipCompilation = true
                    });

                Check.Equal("Story 15 EditMode-only Smoke count matches EditMode subset (3)", 3, editOnly.PassedCount);
                Check.Equal("Story 15 PlayMode-only Smoke count matches PlayMode subset (2)", 2, playOnly.PassedCount);
                Check.Equal(
                    "Story 15 combined outcomes match individual mode runs",
                    smokeHeadless.PassedCount,
                    editOnly.PassedCount + playOnly.PassedCount);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory16UnexpectedErrorLogFailsAndLogAssertExpectPasses()
        {
            string tempProject = CreateTempTestProject("shared_a");
            try
            {
                string logXmlPath = Path.Combine(tempProject, "TestResults", "logging.xml");
                TestRunResult logRun = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.EditMode,
                        CategoryFilter = "Logging",
                        ResultsXmlPath = logXmlPath,
                        SkipCompilation = true
                    });

                Check.Equal("Story 16 logging test run compiles", true, logRun.CompileSucceeded);
                Check.Equal("Story 16 logging category selects 3 tests", 3, logRun.SelectedTestCaseCount);
                Check.Equal("Story 16 logging run has 1 passing and 2 failing tests", 1, logRun.PassedCount);
                Check.Equal("Story 16 logging run records 2 failures", 2, logRun.FailedCount);

                TestCaseResult? unexpectedErr = logRun.FindTest("MyGame.EditTests.LoggingTests.LogsUnexpectedErrorFails");
                TestCaseResult? expectedErr = logRun.FindTest("MyGame.EditTests.LoggingTests.LogsExpectedErrorWithLogAssertPasses");
                TestCaseResult? missingExpectedErr = logRun.FindTest("MyGame.EditTests.LoggingTests.MissingExpectedErrorFails");

                Check.That(
                    "Story 16 test logging unexpected Debug.LogError fails even when Assert passes",
                    unexpectedErr != null
                    && unexpectedErr.Outcome == TestOutcome.Failed
                    && unexpectedErr.Message != null
                    && unexpectedErr.Message.Contains("Unexpected log message received", StringComparison.Ordinal)
                    && unexpectedErr.Message.Contains("Boom error", StringComparison.Ordinal));

                Check.That(
                    "Story 16 same error log declared with LogAssert.Expect passes",
                    expectedErr != null && expectedErr.Outcome == TestOutcome.Passed);

                Check.That(
                    "Story 16 test expecting an error log that never arrives fails",
                    missingExpectedErr != null
                    && missingExpectedErr.Outcome == TestOutcome.Failed
                    && missingExpectedErr.Message != null
                    && missingExpectedErr.Message.Contains("Expected log not received", StringComparison.Ordinal));

                ValidateNUnit3Xml(
                    File.ReadAllText(logXmlPath),
                    expectedSelected: 3,
                    expectedTotal: 3,
                    expectedPassed: 1,
                    expectedFailed: 2,
                    expectedOverallResult: "Failed");
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestStory17CancelledTestRunReportsOnlyCompletedTests()
        {
            string tempProject = CreateTempTestProject("shared_a");
            try
            {
                using CancellationTokenSource cts = new CancellationTokenSource();
                List<string> completedNames = new List<string>();
                string cancelXmlPath = Path.Combine(tempProject, "TestResults", "cancelled.xml");

                TestRunOptions options = new TestRunOptions
                {
                    Mode = TestModeFilter.All,
                    CategoryFilter = "Smoke",
                    ResultsXmlPath = cancelXmlPath,
                    CancellationToken = cts.Token,
                    OnTestCompleted = tc =>
                    {
                        completedNames.Add(tc.Descriptor.FullName);
                        if (completedNames.Count >= 2)
                        {
                            cts.Cancel();
                        }
                    }
                };

                TestRunResult cancelledRun = TestRunner.RunProject(tempProject, options);

                Check.Equal("Story 17 cancelled run marks WasCancelled = true", true, cancelledRun.WasCancelled);
                Check.Equal("Story 17 cancelled run Succeeded is false", false, cancelledRun.Succeeded);
                Check.Equal("Story 17 selected 5 total tests", 5, cancelledRun.SelectedTestCaseCount);
                Check.Equal("Story 17 completed exactly 2 tests before cancellation", 2, cancelledRun.CompletedCount);
                Check.Equal("Story 17 passed count is 2 (never reports remaining 3 as passed)", 2, cancelledRun.PassedCount);
                Check.Equal("Story 17 not-run count is 3", 3, cancelledRun.NotRunCount);
                Check.Equal("Story 17 Results list contains only the 2 completed tests", 2, cancelledRun.Results.Count);

                Check.Equal("Story 17 writes cancelled NUnit 3 XML file", true, File.Exists(cancelXmlPath));
                string xmlText = File.ReadAllText(cancelXmlPath);
                ValidateNUnit3Xml(
                    xmlText,
                    expectedSelected: 5,
                    expectedTotal: 2,
                    expectedPassed: 2,
                    expectedFailed: 0,
                    expectedOverallResult: "Failed");

                XmlDocument doc = new XmlDocument();
                doc.LoadXml(xmlText);
                XmlNodeList? caseNodes = doc.GetElementsByTagName("test-case");
                Check.Equal("Story 17 XML contains only 2 <test-case> elements for completed tests", 2, caseNodes != null ? caseNodes.Count : -1);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestUnityPlatformRetryPrebuildSetupPostBuildCleanupAndNamePatterns()
        {
            string tempProject = CreateTempTestProject("shared_b");
            try
            {
                string gapsFile = Path.Combine(tempProject, "Assets", "Tests", "EditMode", "UtfGapTests.cs");
                string runningPlatformName = UnityEngine.Application.platform.ToString();
                string otherPlatformName = string.Equals(runningPlatformName, "WindowsPlayer", StringComparison.Ordinal)
                    ? "Android"
                    : "WindowsPlayer";

                File.WriteAllText(
                    gapsFile,
                    "using System.Collections.Generic;\n"
                    + "using System.IO;\n"
                    + "using NUnit.Framework;\n"
                    + "using UnityEngine;\n"
                    + "using UnityEngine.TestTools;\n"
                    + "namespace MyGame.EditTests\n"
                    + "{\n"
                    + "    public class LifecycleRecorder : IPrebuildSetup, IPostBuildCleanup\n"
                    + "    {\n"
                    + "        public static readonly List<string> Events = new List<string>();\n"
                    + "        public static string LogFilePath => Path.Combine(Application.dataPath, \"lifecycle_events.txt\");\n"
                    + "        public void Setup()\n"
                    + "        {\n"
                    + "            Events.Add(\"PrebuildSetup\");\n"
                    + "            File.AppendAllText(LogFilePath, \"PrebuildSetup;\\n\");\n"
                    + "        }\n"
                    + "        public void Cleanup()\n"
                    + "        {\n"
                    + "            Events.Add(\"PostBuildCleanup\");\n"
                    + "            File.AppendAllText(LogFilePath, \"PostBuildCleanup;\\n\");\n"
                    + "        }\n"
                    + "    }\n"
                    + "    [Category(\"Gaps\")]\n"
                    + "    [PrebuildSetup(typeof(LifecycleRecorder))]\n"
                    + "    [PostBuildCleanup(typeof(LifecycleRecorder))]\n"
                    + "    public class UtfGapTests\n"
                    + "    {\n"
                    + "        private static int _flakyAttempts;\n"
                    + "        private static int _alwaysFailAttempts;\n"
                    + "        [Test]\n"
                    + "        [UnityPlatform(RuntimePlatform." + runningPlatformName + ")]\n"
                    + "        public void IncludedCurrentPlatformRunsAndVerifiesPrebuildSetup()\n"
                    + "        {\n"
                    + "            Assert.IsTrue(LifecycleRecorder.Events.Contains(\"PrebuildSetup\"));\n"
                    + "        }\n"
                    + "        [Test]\n"
                    + "        [UnityPlatform(RuntimePlatform." + otherPlatformName + ")]\n"
                    + "        public void IncludedNonMatchingPlatformIsSkipped()\n"
                    + "        {\n"
                    + "            Assert.Fail(\"Should have been skipped on this platform\");\n"
                    + "        }\n"
                    + "        [Test]\n"
                    + "        [UnityPlatform(exclude = new[] { RuntimePlatform." + runningPlatformName + " })]\n"
                    + "        public void ExcludedCurrentPlatformIsSkipped()\n"
                    + "        {\n"
                    + "            Assert.Fail(\"Should have been skipped because current platform is excluded\");\n"
                    + "        }\n"
                    + "        [Test]\n"
                    + "        [Retry(3)]\n"
                    + "        public void FlakyTestPassesOnRetry()\n"
                    + "        {\n"
                    + "            _flakyAttempts++;\n"
                    + "            if (_flakyAttempts < 3)\n"
                    + "            {\n"
                    + "                Assert.Fail(\"Transient failure on attempt \" + _flakyAttempts);\n"
                    + "            }\n"
                    + "            Assert.AreEqual(3, _flakyAttempts);\n"
                    + "        }\n"
                    + "        [Test]\n"
                    + "        [Retry(2)]\n"
                    + "        public void AlwaysFailingTestExhaustsRetries()\n"
                    + "        {\n"
                    + "            _alwaysFailAttempts++;\n"
                    + "            Assert.Fail(\"Persistent failure after \" + _alwaysFailAttempts + \" attempts\");\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n");

                TestRunResult gapsRun = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.EditMode,
                        CategoryFilter = "Gaps",
                        SkipCompilation = true
                    });

                Check.Equal("UTF gaps test run compiles cleanly", true, gapsRun.CompileSucceeded);
                Check.Equal("UTF gaps selects 5 tests", 5, gapsRun.SelectedTestCaseCount);
                Check.Equal("UTF gaps passes 2 tests (included platform + retry success)", 2, gapsRun.PassedCount);
                Check.Equal("UTF gaps skips 2 non-matching [UnityPlatform] tests", 2, gapsRun.SkippedCount);
                Check.Equal("UTF gaps fails 1 test that exhausts [Retry(2)]", 1, gapsRun.FailedCount);

                TestCaseResult? includedPlat = gapsRun.FindTest("IncludedCurrentPlatformRunsAndVerifiesPrebuildSetup");
                TestCaseResult? nonMatchingInclude = gapsRun.FindTest("IncludedNonMatchingPlatformIsSkipped");
                TestCaseResult? excludedPlat = gapsRun.FindTest("ExcludedCurrentPlatformIsSkipped");
                TestCaseResult? flakyPass = gapsRun.FindTest("FlakyTestPassesOnRetry");
                TestCaseResult? retryExhausted = gapsRun.FindTest("AlwaysFailingTestExhaustsRetries");

                Check.That(
                    "[UnityPlatform] matching current platform runs and sees PrebuildSetup state",
                    includedPlat != null && includedPlat.Outcome == TestOutcome.Passed);
                Check.That(
                    "[UnityPlatform] non-matching include skips with platform reason",
                    nonMatchingInclude != null
                    && nonMatchingInclude.Outcome == TestOutcome.Skipped
                    && !string.IsNullOrEmpty(nonMatchingInclude.Message)
                    && nonMatchingInclude.Message.Contains(otherPlatformName, StringComparison.Ordinal));
                Check.That(
                    "[UnityPlatform] matching exclude skips with platform reason",
                    excludedPlat != null
                    && excludedPlat.Outcome == TestOutcome.Skipped
                    && !string.IsNullOrEmpty(excludedPlat.Message)
                    && excludedPlat.Message.Contains(runningPlatformName, StringComparison.Ordinal));
                Check.That(
                    "[Retry(3)] passes when retry attempt succeeds",
                    flakyPass != null && flakyPass.Outcome == TestOutcome.Passed);
                Check.That(
                    "[Retry(2)] fails after exhausting 2 attempts",
                    retryExhausted != null
                    && retryExhausted.Outcome == TestOutcome.Failed
                    && retryExhausted.Message != null
                    && retryExhausted.Message.Contains("2 attempts", StringComparison.Ordinal));

                string lifecycleLogPath = Path.Combine(tempProject, "Assets", "lifecycle_events.txt");
                string lifecycleLog = File.Exists(lifecycleLogPath) ? File.ReadAllText(lifecycleLogPath) : string.Empty;
                Check.That(
                    "[PrebuildSetup] and [PostBuildCleanup] both execute around test run",
                    lifecycleLog.Contains("PrebuildSetup;", StringComparison.Ordinal)
                    && lifecycleLog.Contains("PostBuildCleanup;", StringComparison.Ordinal)
                    && lifecycleLog.IndexOf("PrebuildSetup;", StringComparison.Ordinal) < lifecycleLog.IndexOf("PostBuildCleanup;", StringComparison.Ordinal));

                string exactXmlPath = Path.Combine(tempProject, "TestResults", "filter_exact.xml");
                TestRunResult exactFilter = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.All,
                        TestNameFilter = "MyGame.EditTests.MathEditTests.SimpleAddition",
                        ResultsXmlPath = exactXmlPath,
                        SkipCompilation = true
                    });
                Check.Equal("Exact --test filter selects 1 test", 1, exactFilter.SelectedTestCaseCount);
                Check.Equal("Exact --test filter passes", 1, exactFilter.PassedCount);

                string wildcardXmlPath = Path.Combine(tempProject, "TestResults", "filter_wildcard.xml");
                TestRunResult wildcardFilter = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.All,
                        TestNameFilter = "MyGame.EditTests.*.MultiplyCases*",
                        ResultsXmlPath = wildcardXmlPath,
                        SkipCompilation = true
                    });
                Check.Equal("Wildcard --test filter selects 2 parameterized cases", 2, wildcardFilter.SelectedTestCaseCount);
                Check.Equal("Wildcard --test filter passes all 2 cases", 2, wildcardFilter.PassedCount);

                int cliFilterExit = Unitas.Testing.Program.Main(new[]
                {
                    "run",
                    tempProject,
                    "--mode",
                    "All",
                    "--test",
                    "MyGame.EditTests.*.MultiplyCases*",
                    "--results",
                    Path.Combine(tempProject, "TestResults", "cli_wildcard.xml")
                });
                Check.Equal("CLI --test wildcard filter exits 0", 0, cliFilterExit);
                ValidateNUnit3Xml(
                    File.ReadAllText(Path.Combine(tempProject, "TestResults", "cli_wildcard.xml")),
                    expectedSelected: 2,
                    expectedTotal: 2,
                    expectedPassed: 2,
                    expectedFailed: 0,
                    expectedOverallResult: "Passed");
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestPlayModeFreshSceneIsolationAndEditStateRestoration()
        {
            string tempProject = CreateTempTestProject("shared_b");
            UnityEngine.GameObject editTimeObject = new UnityEngine.GameObject("EditTimeSurvivorObject");
            editTimeObject.transform.position = new UnityEngine.Vector3(7f, 8f, 9f);
            try
            {
                string playIsoFile = Path.Combine(tempProject, "Assets", "Tests", "PlayMode", "PlayModeIsolationTests.cs");
                File.WriteAllText(
                    playIsoFile,
                    "using NUnit.Framework;\n"
                    + "using UnityEngine;\n"
                    + "using UnityEngine.SceneManagement;\n"
                    + "namespace MyGame.PlayTests\n"
                    + "{\n"
                    + "    [Category(\"Isolation\")]\n"
                    + "    public class PlayModeIsolationTests\n"
                    + "    {\n"
                    + "        private static string _firstSceneName = string.Empty;\n"
                    + "        [Test]\n"
                    + "        public void PlayTest1StartsInFreshTestSceneAndSpawnsLeakedObject()\n"
                    + "        {\n"
                    + "            Scene active = SceneManager.GetActiveScene();\n"
                    + "            Assert.IsTrue(active.IsValid());\n"
                    + "            Assert.IsTrue(active.name.StartsWith(\"UnitasPlayModeTestScene_\"));\n"
                    + "            _firstSceneName = active.name;\n"
                    + "            new GameObject(\"LeakedFromPlayTest1\");\n"
                    + "        }\n"
                    + "        [Test]\n"
                    + "        public void PlayTest2GetsDistinctFreshSceneWithoutLeakedObject()\n"
                    + "        {\n"
                    + "            Scene active = SceneManager.GetActiveScene();\n"
                    + "            Assert.IsTrue(active.IsValid());\n"
                    + "            Assert.IsTrue(active.name.StartsWith(\"UnitasPlayModeTestScene_\"));\n"
                    + "            Assert.AreNotEqual(_firstSceneName, active.name);\n"
                    + "            Assert.IsNull(GameObject.Find(\"LeakedFromPlayTest1\"));\n"
                    + "        }\n"
                    + "    }\n"
                    + "}\n");

                TestRunResult isoRun = TestRunner.RunProject(
                    tempProject,
                    new TestRunOptions
                    {
                        Mode = TestModeFilter.PlayMode,
                        CategoryFilter = "Isolation"
                    });

                Check.Equal("PlayMode isolation run compiles", true, isoRun.CompileSucceeded);
                Check.Equal("PlayMode isolation run selects 2 tests", 2, isoRun.SelectedTestCaseCount);
                Check.Equal("PlayMode isolation run passes both tests", 2, isoRun.PassedCount);

                Check.That(
                    "Edit-time state object survives PlayMode test run unchanged",
                    string.Equals(editTimeObject.name, "EditTimeSurvivorObject", StringComparison.Ordinal)
                    && editTimeObject.transform.position == new UnityEngine.Vector3(7f, 8f, 9f));
                Check.That(
                    "PlayMode test objects are cleaned up after play session ends",
                    UnityEngine.GameObject.Find("LeakedFromPlayTest1") == null);
            }
            finally
            {
                UnityEngine.Object.Destroy(editTimeObject);
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestBrokenTestAssemblyReportsCompileFailure()
        {
            string tempProject = CreateTempTestProject("broken_compile");
            try
            {
                string brokenTestFile = Path.Combine(tempProject, "Assets", "Tests", "EditMode", "BrokenSyntaxTests.cs");
                File.WriteAllText(
                    brokenTestFile,
                    "using NUnit.Framework;\nnamespace MyGame.EditTests\n{\n    public class BrokenSyntaxTests\n    {\n        [Test]\n        public void Bad() { int x = \"broken\"; }\n    }\n}\n");

                TestRunResult result = TestRunner.RunProject(tempProject);
                Check.Equal("Broken test assembly compileSucceeded is false", false, result.CompileSucceeded);
                Check.Equal("Broken test assembly Succeeded is false", false, result.Succeeded);
                Check.That("Broken test assembly reports compiler diagnostics", result.Diagnostics.Count > 0);
                Check.That(
                    "Broken test assembly marks discovered test assemblies as unrunnable with reason",
                    result.Assemblies.Count > 0 && !result.Assemblies[0].CanRun && !string.IsNullOrEmpty(result.Assemblies[0].UnrunnableReason));
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void TestCliRunCommandExitCodes()
        {
            string tempProject = CreateTempTestProject("shared_a");
            try
            {
                string xmlPath = Path.Combine(tempProject, "TestResults", "cli_smoke.xml");
                int passExit = Unitas.Testing.Program.Main(new[]
                {
                    "run",
                    tempProject,
                    "--mode",
                    "All",
                    "--category",
                    "Smoke",
                    "--results",
                    xmlPath
                });

                Check.Equal("CLI run passing category exits 0", 0, passExit);
                Check.Equal("CLI run writes results XML", true, File.Exists(xmlPath));

                int failExit = Unitas.Testing.Program.Main(new[]
                {
                    "run",
                    tempProject,
                    "--mode",
                    "EditMode",
                    "--category",
                    "Logging"
                });

                Check.Equal("CLI run failing category exits 1", 1, failExit);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void ValidateNUnit3Xml(
            string xmlText,
            int expectedSelected,
            int expectedTotal,
            int expectedPassed,
            int expectedFailed,
            string expectedOverallResult)
        {
            XmlDocument doc = new XmlDocument();
            doc.LoadXml(xmlText);

            XmlElement? root = doc.DocumentElement;
            Check.That("NUnit 3 XML root is <test-run>", root != null && string.Equals(root.Name, "test-run", StringComparison.Ordinal));
            if (root == null)
            {
                return;
            }

            Check.Equal(
                "NUnit 3 XML testcasecount",
                expectedSelected.ToString(CultureInfo.InvariantCulture),
                root.GetAttribute("testcasecount"));
            Check.Equal(
                "NUnit 3 XML total",
                expectedTotal.ToString(CultureInfo.InvariantCulture),
                root.GetAttribute("total"));
            Check.Equal(
                "NUnit 3 XML passed",
                expectedPassed.ToString(CultureInfo.InvariantCulture),
                root.GetAttribute("passed"));
            Check.Equal(
                "NUnit 3 XML failed",
                expectedFailed.ToString(CultureInfo.InvariantCulture),
                root.GetAttribute("failed"));
            Check.Equal(
                "NUnit 3 XML result",
                expectedOverallResult,
                root.GetAttribute("result"));
        }

        private static string CreateTempTestProject(string label)
        {
            // One directory per group lets the repeated runs reuse the compiled
            // test assemblies instead of recompiling the same sources every run.
            string root = Path.Combine(Path.GetTempPath(), "unitas_utf_test_" + label);
            if (Directory.Exists(root))
            {
                Directory.Delete(root, true);
            }

            string runtimeDir = Path.Combine(root, "Assets", "Scripts");
            string editTestsDir = Path.Combine(root, "Assets", "Tests", "EditMode");
            string playTestsDir = Path.Combine(root, "Assets", "Tests", "PlayMode");

            Directory.CreateDirectory(runtimeDir);
            Directory.CreateDirectory(editTestsDir);
            Directory.CreateDirectory(playTestsDir);

            File.WriteAllText(
                Path.Combine(runtimeDir, "MyGame.Runtime.asmdef"),
                "{\n  \"name\": \"MyGame.Runtime\"\n}\n");

            File.WriteAllText(
                Path.Combine(runtimeDir, "ScoreCalculator.cs"),
                "namespace MyGame\n{\n    public static class ScoreCalculator\n    {\n        public static int Add(int a, int b) => a + b;\n        public static int Multiply(int a, int b) => a * b;\n    }\n}\n");

            File.WriteAllText(
                Path.Combine(editTestsDir, "MyGame.Tests.EditMode.asmdef"),
                "{\n  \"name\": \"MyGame.Tests.EditMode\",\n  \"references\": [ \"MyGame.Runtime\" ],\n  \"includePlatforms\": [ \"Editor\" ],\n  \"defineConstraints\": [ \"UNITY_INCLUDE_TESTS\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(editTestsDir, "MathEditTests.cs"),
                "using NUnit.Framework;\n"
                + "using MyGame;\n"
                + "namespace MyGame.EditTests\n"
                + "{\n"
                + "    public class MathEditTests\n"
                + "    {\n"
                + "        private int _bias;\n"
                + "        [SetUp]\n"
                + "        public void SetUp() { _bias = 10; }\n"
                + "        [TearDown]\n"
                + "        public void TearDown() { _bias = 0; }\n"
                + "        [Test]\n"
                + "        [Category(\"Smoke\")]\n"
                + "        public void SimpleAddition()\n"
                + "        {\n"
                + "            Assert.AreEqual(15, ScoreCalculator.Add(5, _bias));\n"
                + "        }\n"
                + "        [TestCase(2, 3, 6)]\n"
                + "        [TestCase(4, 5, 20)]\n"
                + "        [Category(\"Smoke\")]\n"
                + "        public void MultiplyCases(int a, int b, int expected)\n"
                + "        {\n"
                + "            Assert.AreEqual(expected, ScoreCalculator.Multiply(a, b));\n"
                + "        }\n"
                + "        [Test]\n"
                + "        [Category(\"Slow\")]\n"
                + "        public void SlowNonSmokeCalculation()\n"
                + "        {\n"
                + "            Assert.AreEqual(100, ScoreCalculator.Multiply(10, 10));\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            File.WriteAllText(
                Path.Combine(editTestsDir, "LoggingTests.cs"),
                "using NUnit.Framework;\n"
                + "using UnityEngine;\n"
                + "using UnityEngine.TestTools;\n"
                + "namespace MyGame.EditTests\n"
                + "{\n"
                + "    [Category(\"Logging\")]\n"
                + "    public class LoggingTests\n"
                + "    {\n"
                + "        [Test]\n"
                + "        public void LogsUnexpectedErrorFails()\n"
                + "        {\n"
                + "            Debug.LogError(\"Boom error: unexpected failure\");\n"
                + "            Assert.AreEqual(1, 1);\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void LogsExpectedErrorWithLogAssertPasses()\n"
                + "        {\n"
                + "            LogAssert.Expect(LogType.Error, \"Boom error: expected\");\n"
                + "            Debug.LogError(\"Boom error: expected\");\n"
                + "            LogAssert.NoUnexpectedReceived();\n"
                + "            Assert.AreEqual(1, 1);\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void MissingExpectedErrorFails()\n"
                + "        {\n"
                + "            LogAssert.Expect(LogType.Error, \"Never logged error\");\n"
                + "            Assert.IsTrue(true);\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            File.WriteAllText(
                Path.Combine(playTestsDir, "MyGame.Tests.PlayMode.asmdef"),
                "{\n  \"name\": \"MyGame.Tests.PlayMode\",\n  \"references\": [ \"MyGame.Runtime\" ],\n  \"optionalUnityReferences\": [ \"TestAssemblies\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(playTestsDir, "GameplayPlayTests.cs"),
                "using System.Collections;\n"
                + "using NUnit.Framework;\n"
                + "using UnityEngine;\n"
                + "using UnityEngine.TestTools;\n"
                + "namespace MyGame.PlayTests\n"
                + "{\n"
                + "    [Category(\"Smoke\")]\n"
                + "    public class GameplayPlayTests\n"
                + "    {\n"
                + "        private GameObject _actor;\n"
                + "        [SetUp]\n"
                + "        public void SetUp()\n"
                + "        {\n"
                + "            _actor = new GameObject(\"PlayActor\");\n"
                + "            _actor.transform.position = new Vector3(1f, 0f, 0f);\n"
                + "        }\n"
                + "        [TearDown]\n"
                + "        public void TearDown()\n"
                + "        {\n"
                + "            if (_actor != null)\n"
                + "            {\n"
                + "                Object.Destroy(_actor);\n"
                + "            }\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void GameObjectSpawnInPlayMode()\n"
                + "        {\n"
                + "            Assert.IsNotNull(_actor);\n"
                + "            Assert.AreEqual(\"PlayActor\", _actor.name);\n"
                + "            Assert.AreEqual(1f, _actor.transform.position.x, 0.001f);\n"
                + "        }\n"
                + "        [UnityTest]\n"
                + "        public IEnumerator MultiFrameCoroutineSteps()\n"
                + "        {\n"
                + "            int startFrame = Time.frameCount;\n"
                + "            _actor.transform.position = new Vector3(2f, 0f, 0f);\n"
                + "            yield return null;\n"
                + "            Assert.Greater(Time.frameCount, startFrame);\n"
                + "            _actor.transform.position = new Vector3(5f, 0f, 0f);\n"
                + "            yield return new WaitForSeconds(0.03f);\n"
                + "            Assert.AreEqual(5f, _actor.transform.position.x, 0.001f);\n"
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
