using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Threading;
using System.Xml;
using Unitas.Testing;

namespace Unitas.Bridge.Tests
{
    public static class UtfHardeningTests
    {
        public static void RunAll()
        {
            string tempProject = CreateHardeningTestProject();
            try
            {
                VerifyDeepCoroutineYieldsAndLogAssertAndXmlParity(tempProject);
                VerifyAssemblyCategoryAndNamePatternFilters(tempProject);
                VerifyMidRunCancellationReporting(tempProject);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        public static void DeepCoroutineYieldsAndLogAssertAndXmlParity()
        {
            string tempProject = CreateHardeningTestProject();
            try
            {
                VerifyDeepCoroutineYieldsAndLogAssertAndXmlParity(tempProject);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        public static void AssemblyCategoryAndNamePatternFilters()
        {
            string tempProject = CreateHardeningTestProject();
            try
            {
                VerifyAssemblyCategoryAndNamePatternFilters(tempProject);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        public static void MidRunCancellationReporting()
        {
            string tempProject = CreateHardeningTestProject();
            try
            {
                VerifyMidRunCancellationReporting(tempProject);
            }
            finally
            {
                DeleteDirectoryIfExists(tempProject);
            }
        }

        private static void VerifyDeepCoroutineYieldsAndLogAssertAndXmlParity(string tempProject)
        {
            string xmlPath = Path.Combine(tempProject, "TestResults", "hardening_full.xml");
            TestRunResult fullRun = TestRunner.RunProject(
                tempProject,
                new TestRunOptions
                {
                    Mode = TestModeFilter.All,
                    ResultsXmlPath = xmlPath,
                    FrameDeltaSeconds = 0.02
                });

            Check.Equal("UTF Hardening full run compiles cleanly", true, fullRun.CompileSucceeded);
            Check.Equal("UTF Hardening discovers 3 test assemblies", 3, fullRun.Assemblies.Count);
            Check.Equal("UTF Hardening selects 8 total test cases", 8, fullRun.SelectedTestCaseCount);
            Check.Equal("UTF Hardening completes 8 test cases", 8, fullRun.CompletedCount);
            Check.Equal("UTF Hardening passes 5 test cases", 5, fullRun.PassedCount);
            Check.Equal("UTF Hardening fails 2 test cases", 2, fullRun.FailedCount);
            Check.Equal("UTF Hardening skips 1 test case", 1, fullRun.SkippedCount);
            Check.Equal("UTF Hardening not-run count is 0", 0, fullRun.NotRunCount);

            TestCaseResult? deepYield = fullRun.FindTest("Hardening.PlayTests.CoroutineDepthTests.DeepNestedCoroutineYields");
            Check.That(
                "[UnityTest] deep coroutine yields (null, WaitForSeconds under scaled time, WaitForFixedUpdate, WaitForEndOfFrame, nested IEnumerator, WaitUntil, WaitWhile) pass",
                deepYield != null && deepYield.Outcome == TestOutcome.Passed);
            Check.That(
                "[UnityTest] deep coroutine test captures Debug.Log output",
                deepYield != null && deepYield.CapturedLogs.Count > 0);

            TestCaseResult? logExpectPass = fullRun.FindTest("Hardening.EditTests.LogHardeningTests.ExpectAndNoUnexpectedReceivedPasses");
            Check.That(
                "LogAssert.Expect + LogAssert.NoUnexpectedReceived passes when expected error matches",
                logExpectPass != null && logExpectPass.Outcome == TestOutcome.Passed);

            TestCaseResult? noUnexpectedFail = fullRun.FindTest("Hardening.EditTests.LogHardeningTests.NoUnexpectedReceivedThrowsImmediatelyOnPriorUnexpectedError");
            Check.That(
                "LogAssert.NoUnexpectedReceived immediately fails test when unexpected error was logged earlier",
                noUnexpectedFail != null
                && noUnexpectedFail.Outcome == TestOutcome.Failed
                && noUnexpectedFail.Message != null
                && noUnexpectedFail.Message.Contains("Unexpected log message received", StringComparison.Ordinal)
                && noUnexpectedFail.Message.Contains("Prior unexpected error", StringComparison.Ordinal)
                && !noUnexpectedFail.Message.Contains("ShouldNotReachAfterNoUnexpectedReceived", StringComparison.Ordinal));

            TestCaseResult? ignoreFailingPass = fullRun.FindTest("Hardening.EditTests.LogHardeningTests.IgnoreFailingMessagesSuppressesUnexpectedErrors");
            Check.That(
                "LogAssert.ignoreFailingMessages = true suppresses unexpected error failure during test",
                ignoreFailingPass != null && ignoreFailingPass.Outcome == TestOutcome.Passed);

            TestCaseResult? resetAfterIgnoreFail = fullRun.FindTest("Hardening.EditTests.LogHardeningTests.SubsequentTestFailsBecauseIgnoreFailingMessagesResetsBetweenTests");
            Check.That(
                "LogAssert.ignoreFailingMessages resets to false between tests so subsequent unexpected error fails",
                resetAfterIgnoreFail != null
                && resetAfterIgnoreFail.Outcome == TestOutcome.Failed
                && resetAfterIgnoreFail.Message != null
                && resetAfterIgnoreFail.Message.Contains("Unexpected error after reset", StringComparison.Ordinal));

            TestCaseResult? skippedCase = fullRun.FindTest("Hardening.EditTests.LogHardeningTests.IgnoredTestRecordsSkipReason");
            Check.That(
                "[Ignore] test case is marked Skipped with reason",
                skippedCase != null
                && skippedCase.Outcome == TestOutcome.Skipped
                && string.Equals(skippedCase.Message, "Hardening skip reason", StringComparison.Ordinal));

            Check.Equal("UTF Hardening writes NUnit 3 XML to disk", true, File.Exists(xmlPath));
            VerifyNUnit3XmlParity(File.ReadAllText(xmlPath));
        }

        private static void VerifyNUnit3XmlParity(string xmlText)
        {
            XmlDocument doc = new XmlDocument();
            doc.LoadXml(xmlText);

            XmlElement? root = doc.DocumentElement;
            Check.That("NUnit 3 XML has <test-run> root", root != null && string.Equals(root.Name, "test-run", StringComparison.Ordinal));
            if (root == null)
            {
                return;
            }

            Check.Equal("NUnit 3 XML <test-run> testcasecount is 8", "8", root.GetAttribute("testcasecount"));
            Check.Equal("NUnit 3 XML <test-run> total is 8", "8", root.GetAttribute("total"));
            Check.Equal("NUnit 3 XML <test-run> passed is 5", "5", root.GetAttribute("passed"));
            Check.Equal("NUnit 3 XML <test-run> failed is 2", "2", root.GetAttribute("failed"));
            Check.Equal("NUnit 3 XML <test-run> skipped is 1", "1", root.GetAttribute("skipped"));
            Check.Equal("NUnit 3 XML <test-run> result is Failed", "Failed", root.GetAttribute("result"));

            XmlNodeList? caseNodes = doc.GetElementsByTagName("test-case");
            Check.Equal("NUnit 3 XML has 8 <test-case> elements", 8, caseNodes != null ? caseNodes.Count : 0);
            if (caseNodes == null)
            {
                return;
            }

            bool verifiedDurationOnAll = true;
            bool sawFailureMessageAndStack = false;
            bool sawSkipReasonMessage = false;
            bool sawCapturedOutput = false;

            for (int i = 0; i < caseNodes.Count; i++)
            {
                if (!(caseNodes[i] is XmlElement tc))
                {
                    continue;
                }

                string durationAttr = tc.GetAttribute("duration");
                if (string.IsNullOrEmpty(durationAttr)
                    || !double.TryParse(durationAttr, NumberStyles.Float, CultureInfo.InvariantCulture, out double dur)
                    || dur < 0.0)
                {
                    verifiedDurationOnAll = false;
                }

                string fullName = tc.GetAttribute("fullname");
                if (string.Equals(fullName, "Hardening.EditTests.LogHardeningTests.NoUnexpectedReceivedThrowsImmediatelyOnPriorUnexpectedError", StringComparison.Ordinal))
                {
                    XmlElement? failure = tc["failure"];
                    XmlElement? msg = failure?["message"];
                    XmlElement? stack = failure?["stack-trace"];
                    if (failure != null
                        && msg != null
                        && msg.InnerText.Contains("Prior unexpected error", StringComparison.Ordinal)
                        && stack != null
                        && !string.IsNullOrEmpty(stack.InnerText))
                    {
                        sawFailureMessageAndStack = true;
                    }
                }
                else if (string.Equals(fullName, "Hardening.EditTests.LogHardeningTests.IgnoredTestRecordsSkipReason", StringComparison.Ordinal))
                {
                    XmlElement? reason = tc["reason"];
                    XmlElement? msg = reason?["message"];
                    if (reason != null
                        && msg != null
                        && string.Equals(msg.InnerText, "Hardening skip reason", StringComparison.Ordinal))
                    {
                        sawSkipReasonMessage = true;
                    }
                }
                else if (string.Equals(fullName, "Hardening.PlayTests.CoroutineDepthTests.DeepNestedCoroutineYields", StringComparison.Ordinal))
                {
                    XmlElement? output = tc["output"];
                    if (output != null && output.InnerText.Contains("DeepCoroutineStep:Completed", StringComparison.Ordinal))
                    {
                        sawCapturedOutput = true;
                    }
                }
            }

            Check.Equal("NUnit 3 XML every <test-case> carries a non-negative duration attribute", true, verifiedDurationOnAll);
            Check.Equal("NUnit 3 XML failed <test-case> carries <failure><message> and <stack-trace>", true, sawFailureMessageAndStack);
            Check.Equal("NUnit 3 XML skipped <test-case> carries <reason><message>", true, sawSkipReasonMessage);
            Check.Equal("NUnit 3 XML <test-case> with logs carries <output>", true, sawCapturedOutput);
        }

        private static void VerifyAssemblyCategoryAndNamePatternFilters(string tempProject)
        {
            string sharedOut = Path.Combine(tempProject, "Library", "UnitasScriptAssemblies");

            TestRunResult extraAsmOnly = TestRunner.RunProject(
                tempProject,
                new TestRunOptions
                {
                    Mode = TestModeFilter.All,
                    AssemblyFilter = "Hardening.Tests.ExtraEdit",
                    OutputDirectory = sharedOut,
                    SkipCompilation = true
                });

            Check.Equal("AssemblyFilter selects only tests in Hardening.Tests.ExtraEdit (2)", 2, extraAsmOnly.SelectedTestCaseCount);
            Check.Equal("AssemblyFilter passes both tests in Hardening.Tests.ExtraEdit", 2, extraAsmOnly.PassedCount);
            Check.That(
                "AssemblyFilter includes ExtraAssemblyAlphaTest",
                extraAsmOnly.FindTest("Hardening.ExtraTests.ExtraSuite.ExtraAssemblyAlphaTest") != null);
            Check.That(
                "AssemblyFilter excludes EditMode assembly tests",
                extraAsmOnly.FindTest("Hardening.EditTests.LogHardeningTests.ExpectAndNoUnexpectedReceivedPasses") == null);

            TestRunResult multiAsmAndCategory = TestRunner.RunProject(
                tempProject,
                new TestRunOptions
                {
                    Mode = TestModeFilter.All,
                    AssemblyFilters = new[] { "Hardening.Tests.ExtraEdit", "Hardening.Tests.PlayMode" },
                    CategoryFilter = "CoroutineDepth,ExtraCatA",
                    OutputDirectory = sharedOut,
                    SkipCompilation = true
                });

            Check.Equal("AssemblyFilters + CategoryFilter selects 2 matching tests across 2 assemblies", 2, multiAsmAndCategory.SelectedTestCaseCount);
            Check.Equal("AssemblyFilters + CategoryFilter passes both selected tests", 2, multiAsmAndCategory.PassedCount);

            TestRunResult namePatternFilter = TestRunner.RunProject(
                tempProject,
                new TestRunOptions
                {
                    Mode = TestModeFilter.All,
                    AssemblyFilter = "Hardening.Tests.ExtraEdit",
                    TestNameFilter = "*Beta*",
                    OutputDirectory = sharedOut,
                    SkipCompilation = true
                });

            Check.Equal("AssemblyFilter + wildcard TestNameFilter selects 1 test", 1, namePatternFilter.SelectedTestCaseCount);
            Check.Equal("AssemblyFilter + wildcard TestNameFilter passes ExtraAssemblyBetaTest", 1, namePatternFilter.PassedCount);
            Check.That(
                "Wildcard TestNameFilter matched ExtraAssemblyBetaTest",
                namePatternFilter.FindTest("Hardening.ExtraTests.ExtraSuite.ExtraAssemblyBetaTest") != null);
        }

        private static void VerifyMidRunCancellationReporting(string tempProject)
        {
            using CancellationTokenSource cts = new CancellationTokenSource();
            int completedCallbackCount = 0;
            string cancelXmlPath = Path.Combine(tempProject, "TestResults", "hardening_cancelled.xml");

            TestRunResult cancelledRun = TestRunner.RunProject(
                tempProject,
                new TestRunOptions
                {
                    Mode = TestModeFilter.EditMode,
                    ResultsXmlPath = cancelXmlPath,
                    CancellationToken = cts.Token,
                    SkipCompilation = true,
                    OnTestCompleted = _ =>
                    {
                        completedCallbackCount++;
                        if (completedCallbackCount >= 2)
                        {
                            cts.Cancel();
                        }
                    }
                });

            Check.Equal("Cancellation mid-run sets WasCancelled = true", true, cancelledRun.WasCancelled);
            Check.Equal("Cancellation mid-run sets Succeeded = false", false, cancelledRun.Succeeded);
            Check.Equal("Cancellation mid-run selected 7 EditMode tests", 7, cancelledRun.SelectedTestCaseCount);
            Check.Equal("Cancellation mid-run completed 2 tests before stopping", 2, cancelledRun.CompletedCount);
            Check.Equal("Cancellation mid-run reports NotRunCount = 5 for remaining unrun tests", 5, cancelledRun.NotRunCount);
            Check.That("Cancellation mid-run never marks unrun tests as passed", cancelledRun.PassedCount <= 2);

            XmlDocument doc = new XmlDocument();
            doc.LoadXml(File.ReadAllText(cancelXmlPath));
            XmlElement? root = doc.DocumentElement;
            Check.Equal("Cancelled NUnit 3 XML result is Failed", "Failed", root != null ? root.GetAttribute("result") : string.Empty);
            Check.Equal("Cancelled NUnit 3 XML label is Cancelled", "Cancelled", root != null ? root.GetAttribute("label") : string.Empty);
        }

        private static string CreateHardeningTestProject()
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas_utf_hardening_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture));

            string editDir = Path.Combine(root, "Assets", "Tests", "EditMode");
            string extraEditDir = Path.Combine(root, "Assets", "Tests", "ExtraEdit");
            string playDir = Path.Combine(root, "Assets", "Tests", "PlayMode");

            Directory.CreateDirectory(editDir);
            Directory.CreateDirectory(extraEditDir);
            Directory.CreateDirectory(playDir);

            File.WriteAllText(
                Path.Combine(editDir, "Hardening.Tests.EditMode.asmdef"),
                "{\n  \"name\": \"Hardening.Tests.EditMode\",\n  \"includePlatforms\": [ \"Editor\" ],\n  \"defineConstraints\": [ \"UNITY_INCLUDE_TESTS\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(editDir, "LogHardeningTests.cs"),
                "using System.Text.RegularExpressions;\n"
                + "using NUnit.Framework;\n"
                + "using UnityEngine;\n"
                + "using UnityEngine.TestTools;\n"
                + "namespace Hardening.EditTests\n"
                + "{\n"
                + "    [Category(\"LogHardening\")]\n"
                + "    public class LogHardeningTests\n"
                + "    {\n"
                + "        [Test]\n"
                + "        public void ExpectAndNoUnexpectedReceivedPasses()\n"
                + "        {\n"
                + "            LogAssert.Expect(LogType.Error, new Regex(\"Expected regex error [0-9]+\"));\n"
                + "            Debug.LogError(\"Expected regex error 42\");\n"
                + "            LogAssert.NoUnexpectedReceived();\n"
                + "            Assert.AreEqual(42, 42);\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void NoUnexpectedReceivedThrowsImmediatelyOnPriorUnexpectedError()\n"
                + "        {\n"
                + "            Debug.LogError(\"Prior unexpected error before check\");\n"
                + "            LogAssert.NoUnexpectedReceived();\n"
                + "            Assert.Fail(\"ShouldNotReachAfterNoUnexpectedReceived\");\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void IgnoreFailingMessagesSuppressesUnexpectedErrors()\n"
                + "        {\n"
                + "            LogAssert.ignoreFailingMessages = true;\n"
                + "            Debug.LogError(\"Suppressed error 1\");\n"
                + "            LogAssert.NoUnexpectedReceived();\n"
                + "            Assert.IsTrue(LogAssert.ignoreFailingMessages);\n"
                + "        }\n"
                + "        [Test]\n"
                + "        public void SubsequentTestFailsBecauseIgnoreFailingMessagesResetsBetweenTests()\n"
                + "        {\n"
                + "            Assert.IsFalse(LogAssert.ignoreFailingMessages);\n"
                + "            Debug.LogError(\"Unexpected error after reset\");\n"
                + "        }\n"
                + "        [Test]\n"
                + "        [Ignore(\"Hardening skip reason\")]\n"
                + "        public void IgnoredTestRecordsSkipReason()\n"
                + "        {\n"
                + "            Assert.Fail(\"Should not execute ignored test\");\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            File.WriteAllText(
                Path.Combine(extraEditDir, "Hardening.Tests.ExtraEdit.asmdef"),
                "{\n  \"name\": \"Hardening.Tests.ExtraEdit\",\n  \"includePlatforms\": [ \"Editor\" ],\n  \"defineConstraints\": [ \"UNITY_INCLUDE_TESTS\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(extraEditDir, "ExtraSuite.cs"),
                "using NUnit.Framework;\n"
                + "namespace Hardening.ExtraTests\n"
                + "{\n"
                + "    public class ExtraSuite\n"
                + "    {\n"
                + "        [Test]\n"
                + "        [Category(\"ExtraCatA\")]\n"
                + "        public void ExtraAssemblyAlphaTest()\n"
                + "        {\n"
                + "            Assert.AreEqual(10, 5 + 5);\n"
                + "        }\n"
                + "        [Test]\n"
                + "        [Category(\"ExtraCatB\")]\n"
                + "        public void ExtraAssemblyBetaTest()\n"
                + "        {\n"
                + "            Assert.AreEqual(20, 4 * 5);\n"
                + "        }\n"
                + "    }\n"
                + "}\n");

            File.WriteAllText(
                Path.Combine(playDir, "Hardening.Tests.PlayMode.asmdef"),
                "{\n  \"name\": \"Hardening.Tests.PlayMode\",\n  \"optionalUnityReferences\": [ \"TestAssemblies\" ]\n}\n");

            File.WriteAllText(
                Path.Combine(playDir, "CoroutineDepthTests.cs"),
                "using System.Collections;\n"
                + "using NUnit.Framework;\n"
                + "using UnityEngine;\n"
                + "using UnityEngine.TestTools;\n"
                + "namespace Hardening.PlayTests\n"
                + "{\n"
                + "    [Category(\"CoroutineDepth\")]\n"
                + "    public class CoroutineDepthTests\n"
                + "    {\n"
                + "        private int _steps;\n"
                + "        [UnityTest]\n"
                + "        public IEnumerator DeepNestedCoroutineYields()\n"
                + "        {\n"
                + "            int startFrame = Time.frameCount;\n"
                + "            yield return null;\n"
                + "            Assert.Greater(Time.frameCount, startFrame);\n"
                + "            _steps++;\n"
                + "            Time.timeScale = 2.0f;\n"
                + "            int beforeWaitSecondsFrame = Time.frameCount;\n"
                + "            yield return new WaitForSeconds(0.08f);\n"
                + "            int elapsedScaledFrames = Time.frameCount - beforeWaitSecondsFrame;\n"
                + "            Assert.Greater(elapsedScaledFrames, 0);\n"
                + "            Assert.Less(elapsedScaledFrames, 4);\n"
                + "            Time.timeScale = 1.0f;\n"
                + "            yield return new WaitForFixedUpdate();\n"
                + "            _steps++;\n"
                + "            yield return new WaitForEndOfFrame();\n"
                + "            _steps++;\n"
                + "            yield return LevelOneNestedRoutine();\n"
                + "            Assert.AreEqual(6, _steps);\n"
                + "            int waitCounter = 0;\n"
                + "            yield return new WaitUntil(() => { waitCounter++; return waitCounter >= 3; });\n"
                + "            Assert.AreEqual(3, waitCounter);\n"
                + "            int whileCounter = 0;\n"
                + "            yield return new WaitWhile(() => { whileCounter++; return whileCounter < 3; });\n"
                + "            Assert.AreEqual(3, whileCounter);\n"
                + "            Debug.Log(\"DeepCoroutineStep:Completed\");\n"
                + "        }\n"
                + "        private IEnumerator LevelOneNestedRoutine()\n"
                + "        {\n"
                + "            _steps++;\n"
                + "            yield return null;\n"
                + "            yield return LevelTwoNestedRoutine();\n"
                + "        }\n"
                + "        private IEnumerator LevelTwoNestedRoutine()\n"
                + "        {\n"
                + "            _steps++;\n"
                + "            yield return new WaitForEndOfFrame();\n"
                + "            _steps++;\n"
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
