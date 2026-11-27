using System;
using System.Collections.Generic;

namespace Unitas.Editor.Mcp
{
    public sealed class CliArgs
    {
        public const string BatchModeFlag = "-batchmode";
        public const string NoGraphicsFlag = "-nographics";
        public const string QuitFlag = "-quit";
        public const string ProjectPathOption = "-projectPath";
        public const string BuildTargetOption = "-buildTarget";
        public const string ExecuteMethodOption = "-executeMethod";
        public const string LogFileOption = "-logFile";
        public const string RunTestsFlag = "-runTests";
        public const string TestPlatformOption = "-testPlatform";
        public const string TestResultsOption = "-testResults";
        public const string RunScenarioOption = "-runScenario";
        public const string ShellFlag = "--shell";
        public const string ProjectOption = "--project";
        public const string ShellProjectOption = "--shell-project";
        public const string FramesOption = "--frames";
        public const string CaptureOutOption = "--capture-out";
        public const string HeadlessFlag = "--headless";

        public static readonly IReadOnlyList<string> KnownArguments = new[]
        {
            BatchModeFlag,
            NoGraphicsFlag,
            QuitFlag,
            ProjectPathOption,
            BuildTargetOption,
            ExecuteMethodOption,
            LogFileOption,
            RunTestsFlag,
            TestPlatformOption,
            TestResultsOption,
            RunScenarioOption,
        };

        public static bool IsShellInvocation(string[]? args)
        {
            if (args == null || args.Length == 0)
            {
                return false;
            }

            foreach (string arg in args)
            {
                if (string.Equals(arg, ShellFlag, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        public CliArgs(
            bool batchMode,
            bool noGraphics,
            bool quit,
            string? projectPath,
            string? buildTarget,
            string? executeMethod,
            string? logFile,
            bool runTests,
            string? testPlatform,
            string? testResults,
            string? runScenario,
            IReadOnlyList<string> unrecognizedArguments,
            IReadOnlyList<string> missingValueArguments)
        {
            ArgumentNullException.ThrowIfNull(unrecognizedArguments);
            ArgumentNullException.ThrowIfNull(missingValueArguments);
            BatchMode = batchMode;
            NoGraphics = noGraphics;
            Quit = quit;
            ProjectPath = projectPath;
            BuildTarget = buildTarget;
            ExecuteMethod = executeMethod;
            LogFile = logFile;
            RunTests = runTests;
            TestPlatform = testPlatform;
            TestResults = testResults;
            RunScenario = runScenario;
            UnrecognizedArguments = unrecognizedArguments;
            MissingValueArguments = missingValueArguments;
        }

        public bool BatchMode { get; }

        public bool NoGraphics { get; }

        public bool Quit { get; }

        public string? ProjectPath { get; }

        public string? BuildTarget { get; }

        public string? ExecuteMethod { get; }

        public string? LogFile { get; }

        public bool RunTests { get; }

        public string? TestPlatform { get; }

        public string? TestResults { get; }

        public string? RunScenario { get; }

        public IReadOnlyList<string> UnrecognizedArguments { get; }

        public IReadOnlyList<string> MissingValueArguments { get; }

        public bool HasValidationErrors =>
            UnrecognizedArguments.Count > 0 || MissingValueArguments.Count > 0;

        public bool RequestsTestExecution =>
            RunTests
            || !string.IsNullOrEmpty(TestPlatform)
            || !string.IsNullOrEmpty(TestResults);

        public static CliArgs Parse(string[] args)
        {
            ArgumentNullException.ThrowIfNull(args);

            bool batchMode = false;
            bool noGraphics = false;
            bool quit = false;
            string? projectPath = null;
            string? buildTarget = null;
            string? executeMethod = null;
            string? logFile = null;
            bool runTests = false;
            string? testPlatform = null;
            string? testResults = null;
            string? runScenario = null;
            List<string> unrecognized = new List<string>();
            List<string> missingValues = new List<string>();

            int index = 0;
            while (index < args.Length)
            {
                string current = args[index] ?? string.Empty;
                if (string.Equals(current, BatchModeFlag, StringComparison.Ordinal))
                {
                    batchMode = true;
                    index++;
                }
                else if (string.Equals(current, NoGraphicsFlag, StringComparison.Ordinal))
                {
                    noGraphics = true;
                    index++;
                }
                else if (string.Equals(current, QuitFlag, StringComparison.Ordinal))
                {
                    quit = true;
                    index++;
                }
                else if (string.Equals(current, RunTestsFlag, StringComparison.Ordinal))
                {
                    runTests = true;
                    index++;
                }
                else if (string.Equals(current, ProjectPathOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        projectPath = value;
                    }
                }
                else if (string.Equals(current, BuildTargetOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        buildTarget = value;
                    }
                }
                else if (string.Equals(current, ExecuteMethodOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        executeMethod = value;
                    }
                }
                else if (string.Equals(current, LogFileOption, StringComparison.Ordinal))
                {
                    if (TryConsumeOtherOrLogValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        logFile = value;
                    }
                }
                else if (string.Equals(current, TestPlatformOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        testPlatform = value;
                    }
                }
                else if (string.Equals(current, TestResultsOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        testResults = value;
                    }
                }
                else if (string.Equals(current, RunScenarioOption, StringComparison.Ordinal))
                {
                    if (TryConsumeValue(args, ref index, current, unrecognized, missingValues, out string? value))
                    {
                        runScenario = value;
                    }
                }
                else
                {
                    unrecognized.Add(current);
                    index++;
                }
            }

            return new CliArgs(
                batchMode,
                noGraphics,
                quit,
                projectPath,
                buildTarget,
                executeMethod,
                logFile,
                runTests,
                testPlatform,
                testResults,
                runScenario,
                unrecognized,
                missingValues);
        }

        private static bool TryConsumeValue(
            string[] args,
            ref int index,
            string optionName,
            List<string> unrecognized,
            List<string> missingValues,
            out string? value)
        {
            if (index + 1 >= args.Length)
            {
                unrecognized.Add(optionName);
                missingValues.Add(optionName);
                index++;
                value = null;
                return false;
            }

            string candidate = args[index + 1] ?? string.Empty;
            if (string.IsNullOrEmpty(candidate) || IsKnownFlagOrOption(candidate))
            {
                unrecognized.Add(optionName);
                missingValues.Add(optionName);
                index++;
                value = null;
                return false;
            }

            value = candidate;
            index += 2;
            return true;
        }

        private static bool TryConsumeOtherOrLogValue(
            string[] args,
            ref int index,
            string optionName,
            List<string> unrecognized,
            List<string> missingValues,
            out string? value)
        {
            if (index + 1 >= args.Length)
            {
                unrecognized.Add(optionName);
                missingValues.Add(optionName);
                index++;
                value = null;
                return false;
            }

            string candidate = args[index + 1] ?? string.Empty;
            if (string.IsNullOrEmpty(candidate)
                || (!string.Equals(candidate, "-", StringComparison.Ordinal) && IsKnownFlagOrOption(candidate)))
            {
                unrecognized.Add(optionName);
                missingValues.Add(optionName);
                index++;
                value = null;
                return false;
            }

            value = candidate;
            index += 2;
            return true;
        }

        public static bool IsKnownFlagOrOption(string argument)
        {
            foreach (string known in KnownArguments)
            {
                if (string.Equals(known, argument, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }
    }
}
