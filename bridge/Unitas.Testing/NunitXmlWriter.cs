using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Xml;

namespace Unitas.Testing
{
    public static class NunitXmlWriter
    {
        public static string GenerateXml(
            string projectRoot,
            int selectedTestCaseCount,
            bool wasCancelled,
            bool compileSucceeded,
            double totalDurationSeconds,
            IReadOnlyList<TestCaseResult> results,
            IReadOnlyList<string> failureMessages)
        {
            int passed = 0;
            int failed = 0;
            int skipped = 0;
            int inconclusive = 0;
            int completedTotal = 0;

            for (int i = 0; i < results.Count; i++)
            {
                switch (results[i].Outcome)
                {
                    case TestOutcome.Passed:
                        passed++;
                        completedTotal++;
                        break;
                    case TestOutcome.Failed:
                        failed++;
                        completedTotal++;
                        break;
                    case TestOutcome.Skipped:
                        skipped++;
                        completedTotal++;
                        break;
                    case TestOutcome.Inconclusive:
                        inconclusive++;
                        completedTotal++;
                        break;
                    case TestOutcome.NotRun:
                        break;
                }
            }

            string overallResult;
            string? runLabel = null;
            if (!compileSucceeded)
            {
                overallResult = "Failed";
                runLabel = "Invalid";
            }
            else if (wasCancelled)
            {
                overallResult = "Failed";
                runLabel = "Cancelled";
            }
            else if (failed > 0)
            {
                overallResult = "Failed";
            }
            else
            {
                overallResult = "Passed";
            }

            StringBuilder sb = new StringBuilder();
            XmlWriterSettings settings = new XmlWriterSettings
            {
                Indent = true,
                Encoding = Encoding.UTF8,
                OmitXmlDeclaration = false
            };

            using (StringWriter sw = new StringWriter(sb, CultureInfo.InvariantCulture))
            using (XmlWriter writer = XmlWriter.Create(sw, settings))
            {
                writer.WriteStartDocument();
                writer.WriteStartElement("test-run");
                writer.WriteAttributeString("id", "2");
                writer.WriteAttributeString("testcasecount", selectedTestCaseCount.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("result", overallResult);
                if (!string.IsNullOrEmpty(runLabel))
                {
                    writer.WriteAttributeString("label", runLabel);
                }

                writer.WriteAttributeString("total", completedTotal.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("passed", passed.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("failed", failed.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("inconclusive", inconclusive.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("skipped", skipped.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("asserts", completedTotal.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("engine-version", "3.5.0.0");
                writer.WriteAttributeString("clr-version", Environment.Version.ToString());
                writer.WriteAttributeString("duration", totalDurationSeconds.ToString("F6", CultureInfo.InvariantCulture));

                string projectName = string.IsNullOrEmpty(projectRoot)
                    ? "UnitasProject"
                    : Path.GetFileName(projectRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));

                writer.WriteStartElement("test-suite");
                writer.WriteAttributeString("type", "Project");
                writer.WriteAttributeString("id", "1000");
                writer.WriteAttributeString("name", projectName);
                writer.WriteAttributeString("fullname", projectName);
                writer.WriteAttributeString("testcasecount", selectedTestCaseCount.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("result", overallResult);
                if (!string.IsNullOrEmpty(runLabel))
                {
                    writer.WriteAttributeString("label", runLabel);
                }

                writer.WriteAttributeString("total", completedTotal.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("passed", passed.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("failed", failed.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("inconclusive", inconclusive.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("skipped", skipped.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("asserts", completedTotal.ToString(CultureInfo.InvariantCulture));
                writer.WriteAttributeString("duration", totalDurationSeconds.ToString("F6", CultureInfo.InvariantCulture));

                if (failureMessages.Count > 0)
                {
                    writer.WriteStartElement("failure");
                    writer.WriteStartElement("message");
                    writer.WriteCData(string.Join("\n", failureMessages));
                    writer.WriteEndElement();
                    writer.WriteEndElement();
                }

                Dictionary<string, List<TestCaseResult>> byAssembly =
                    new Dictionary<string, List<TestCaseResult>>(StringComparer.Ordinal);
                List<string> assemblyOrder = new List<string>();

                for (int i = 0; i < results.Count; i++)
                {
                    TestCaseResult tr = results[i];
                    if (tr.Outcome == TestOutcome.NotRun)
                    {
                        continue;
                    }

                    string asmName = tr.Descriptor.AssemblyName;
                    if (!byAssembly.TryGetValue(asmName, out List<TestCaseResult>? list))
                    {
                        list = new List<TestCaseResult>();
                        byAssembly[asmName] = list;
                        assemblyOrder.Add(asmName);
                    }

                    list.Add(tr);
                }

                int nextId = 1001;
                for (int a = 0; a < assemblyOrder.Count; a++)
                {
                    string asmName = assemblyOrder[a];
                    List<TestCaseResult> asmCases = byAssembly[asmName];
                    WriteAssemblySuite(writer, asmName, asmCases, ref nextId);
                }

                writer.WriteEndElement();
                writer.WriteEndElement();
                writer.WriteEndDocument();
            }

            return sb.ToString();
        }

        private static void WriteAssemblySuite(
            XmlWriter writer,
            string assemblyName,
            List<TestCaseResult> cases,
            ref int nextId)
        {
            int passed = 0;
            int failed = 0;
            int skipped = 0;
            int inconclusive = 0;
            double duration = 0.0;

            for (int i = 0; i < cases.Count; i++)
            {
                duration += cases[i].DurationSeconds;
                switch (cases[i].Outcome)
                {
                    case TestOutcome.Passed:
                        passed++;
                        break;
                    case TestOutcome.Failed:
                        failed++;
                        break;
                    case TestOutcome.Skipped:
                        skipped++;
                        break;
                    case TestOutcome.Inconclusive:
                        inconclusive++;
                        break;
                }
            }

            string suiteResult = failed > 0 ? "Failed" : "Passed";
            int suiteId = nextId++;

            writer.WriteStartElement("test-suite");
            writer.WriteAttributeString("type", "Assembly");
            writer.WriteAttributeString("id", suiteId.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("name", assemblyName + ".dll");
            writer.WriteAttributeString("fullname", assemblyName);
            writer.WriteAttributeString("testcasecount", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("result", suiteResult);
            writer.WriteAttributeString("total", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("passed", passed.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("failed", failed.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("inconclusive", inconclusive.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("skipped", skipped.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("asserts", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("duration", duration.ToString("F6", CultureInfo.InvariantCulture));

            Dictionary<string, List<TestCaseResult>> byFixture =
                new Dictionary<string, List<TestCaseResult>>(StringComparer.Ordinal);
            List<string> fixtureOrder = new List<string>();

            for (int i = 0; i < cases.Count; i++)
            {
                string fixtureName = cases[i].Descriptor.FullClassName;
                if (!byFixture.TryGetValue(fixtureName, out List<TestCaseResult>? list))
                {
                    list = new List<TestCaseResult>();
                    byFixture[fixtureName] = list;
                    fixtureOrder.Add(fixtureName);
                }

                list.Add(cases[i]);
            }

            for (int f = 0; f < fixtureOrder.Count; f++)
            {
                string fixtureFullName = fixtureOrder[f];
                List<TestCaseResult> fixtureCases = byFixture[fixtureFullName];
                WriteFixtureSuite(writer, fixtureFullName, fixtureCases, ref nextId);
            }

            writer.WriteEndElement();
        }

        private static void WriteFixtureSuite(
            XmlWriter writer,
            string fixtureFullName,
            List<TestCaseResult> cases,
            ref int nextId)
        {
            int passed = 0;
            int failed = 0;
            int skipped = 0;
            int inconclusive = 0;
            double duration = 0.0;

            for (int i = 0; i < cases.Count; i++)
            {
                duration += cases[i].DurationSeconds;
                switch (cases[i].Outcome)
                {
                    case TestOutcome.Passed:
                        passed++;
                        break;
                    case TestOutcome.Failed:
                        failed++;
                        break;
                    case TestOutcome.Skipped:
                        skipped++;
                        break;
                    case TestOutcome.Inconclusive:
                        inconclusive++;
                        break;
                }
            }

            string suiteResult = failed > 0 ? "Failed" : "Passed";
            string shortName = cases[0].Descriptor.ClassName;
            int fixtureId = nextId++;

            writer.WriteStartElement("test-suite");
            writer.WriteAttributeString("type", "TestFixture");
            writer.WriteAttributeString("id", fixtureId.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("name", shortName);
            writer.WriteAttributeString("fullname", fixtureFullName);
            writer.WriteAttributeString("classname", fixtureFullName);
            writer.WriteAttributeString("testcasecount", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("result", suiteResult);
            writer.WriteAttributeString("total", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("passed", passed.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("failed", failed.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("inconclusive", inconclusive.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("skipped", skipped.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("asserts", cases.Count.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("duration", duration.ToString("F6", CultureInfo.InvariantCulture));

            for (int i = 0; i < cases.Count; i++)
            {
                WriteTestCase(writer, cases[i], ref nextId);
            }

            writer.WriteEndElement();
        }

        private static void WriteTestCase(XmlWriter writer, TestCaseResult testCase, ref int nextId)
        {
            int caseId = nextId++;
            string resultText = testCase.Outcome switch
            {
                TestOutcome.Passed => "Passed",
                TestOutcome.Failed => "Failed",
                TestOutcome.Skipped => "Skipped",
                TestOutcome.Inconclusive => "Inconclusive",
                _ => "Skipped"
            };

            writer.WriteStartElement("test-case");
            writer.WriteAttributeString("id", caseId.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("name", testCase.Descriptor.DisplayName);
            writer.WriteAttributeString("fullname", testCase.Descriptor.FullName);
            writer.WriteAttributeString("methodname", testCase.Descriptor.MethodName);
            writer.WriteAttributeString("classname", testCase.Descriptor.FullClassName);
            writer.WriteAttributeString("runstate", testCase.Descriptor.IsIgnored ? "Ignored" : "Runnable");
            writer.WriteAttributeString("result", resultText);
            writer.WriteAttributeString("duration", testCase.DurationSeconds.ToString("F6", CultureInfo.InvariantCulture));
            writer.WriteAttributeString("asserts", "1");

            if (testCase.Descriptor.Categories.Count > 0)
            {
                writer.WriteStartElement("properties");
                for (int c = 0; c < testCase.Descriptor.Categories.Count; c++)
                {
                    writer.WriteStartElement("property");
                    writer.WriteAttributeString("name", "Category");
                    writer.WriteAttributeString("value", testCase.Descriptor.Categories[c]);
                    writer.WriteEndElement();
                }

                writer.WriteEndElement();
            }

            if (testCase.Outcome == TestOutcome.Failed)
            {
                writer.WriteStartElement("failure");
                if (!string.IsNullOrEmpty(testCase.Message))
                {
                    writer.WriteStartElement("message");
                    writer.WriteCData(testCase.Message);
                    writer.WriteEndElement();
                }

                if (!string.IsNullOrEmpty(testCase.StackTrace))
                {
                    writer.WriteStartElement("stack-trace");
                    writer.WriteCData(testCase.StackTrace);
                    writer.WriteEndElement();
                }

                writer.WriteEndElement();
            }
            else if (testCase.Outcome == TestOutcome.Skipped && !string.IsNullOrEmpty(testCase.Message))
            {
                writer.WriteStartElement("reason");
                writer.WriteStartElement("message");
                writer.WriteCData(testCase.Message);
                writer.WriteEndElement();
                writer.WriteEndElement();
            }

            if (testCase.CapturedLogs.Count > 0)
            {
                writer.WriteStartElement("output");
                writer.WriteCData(string.Join("\n", testCase.CapturedLogs));
                writer.WriteEndElement();
            }

            writer.WriteEndElement();
        }
    }
}
