using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Testing
{
    public static class Program
    {
        public static int Main(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                Console.Error.WriteLine("Usage: Unitas.Testing run <projectRoot> [--mode EditMode|PlayMode|All] [--category <cat>] [--test <filter>] [--assembly <asm>] [--results <xmlPath>]");
                return 1;
            }

            string command = args[0];
            if (!string.Equals(command, "run", StringComparison.OrdinalIgnoreCase))
            {
                Console.Error.WriteLine("Unknown command '" + command + "'. Expected 'run'.");
                return 1;
            }

            if (args.Length < 2 || string.IsNullOrWhiteSpace(args[1]))
            {
                Console.Error.WriteLine("Missing <projectRoot> argument for 'run'.");
                return 1;
            }

            string projectRoot = args[1];
            TestRunOptions options = new TestRunOptions();

            for (int i = 2; i < args.Length; i++)
            {
                string arg = args[i];
                if (string.Equals(arg, "--mode", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.Mode = ParseMode(args[i]);
                }
                else if (string.Equals(arg, "--category", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.CategoryFilter = args[i];
                }
                else if (string.Equals(arg, "--test", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.TestNameFilter = args[i];
                }
                else if (string.Equals(arg, "--assembly", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.AssemblyFilter = args[i];
                }
                else if (string.Equals(arg, "--results", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.ResultsXmlPath = args[i];
                }
                else if (string.Equals(arg, "--out", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    options.OutputDirectory = args[i];
                }
                else if (string.Equals(arg, "--max-tests", StringComparison.OrdinalIgnoreCase) && i + 1 < args.Length)
                {
                    i++;
                    if (int.TryParse(args[i], NumberStyles.Integer, CultureInfo.InvariantCulture, out int maxTests))
                    {
                        options.MaxTestsToRun = maxTests;
                    }
                }
            }

            TestRunResult result = TestRunner.RunProject(projectRoot, options);
            if (!result.CompileSucceeded)
            {
                for (int i = 0; i < result.Diagnostics.Count; i++)
                {
                    Console.Error.WriteLine(result.Diagnostics[i].FormatCli());
                }

                return 1;
            }

            for (int i = 0; i < result.Results.Count; i++)
            {
                TestCaseResult tc = result.Results[i];
                Console.WriteLine(tc.Outcome + ": " + tc.Descriptor.FullName);
                if (tc.Outcome == TestOutcome.Failed && !string.IsNullOrEmpty(tc.Message))
                {
                    Console.WriteLine("  " + tc.Message);
                }
            }

            Console.WriteLine(
                "Total: " + result.CompletedCount.ToString(CultureInfo.InvariantCulture)
                + ", Passed: " + result.PassedCount.ToString(CultureInfo.InvariantCulture)
                + ", Failed: " + result.FailedCount.ToString(CultureInfo.InvariantCulture)
                + ", Skipped: " + result.SkippedCount.ToString(CultureInfo.InvariantCulture)
                + ", NotRun: " + result.NotRunCount.ToString(CultureInfo.InvariantCulture));

            return result.Succeeded ? 0 : 1;
        }

        private static TestModeFilter ParseMode(string value)
        {
            if (string.Equals(value, "EditMode", StringComparison.OrdinalIgnoreCase))
            {
                return TestModeFilter.EditMode;
            }

            if (string.Equals(value, "PlayMode", StringComparison.OrdinalIgnoreCase))
            {
                return TestModeFilter.PlayMode;
            }

            return TestModeFilter.All;
        }
    }
}
