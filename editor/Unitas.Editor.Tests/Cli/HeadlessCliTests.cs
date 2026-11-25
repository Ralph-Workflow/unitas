using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class HeadlessCliTests
    {
        private static readonly string[] StdioArg = { "--stdio" };
        private static readonly string[] HttpZeroArgs = { "--http", "0" };
        private static readonly string[] BatchQuitArgs = { "-batchmode", "-quit" };
        private static readonly string[] ProjectPathDotArgs = { "-projectPath", "." };
        private static readonly string[] CustomUnknownFlagArg = { "-customUnknownFlag" };
        private static readonly string[] UnrecognizedTestArgs = { "-batchmode", "-unknownFlagAlpha", "--mysteryOption", "-quit" };
        private static readonly string[] MissingProjectPathValArgs = { "-batchmode", "-projectPath" };

        public static void Run()
        {
            ParsesAllTenUnityCommandLineArguments();
            IsCliInvocationDistinguishesCliFromStdioAndHttp();
            UnrecognizedArgumentsAreReportedByNameAndReturnInvalidInvocation();
            MissingOptionValueReportsOptionByNameAndReturnsInvalidInvocation();
            NonExistentProjectPathReturnsProjectOpenFailed();
            UnsupportedRunTestsExecuteMethodAndBuildTargetReturnUnsupportedOperation();
            RegisteredToolsExecuteThroughMcpHostAndHonorUntrustedProjectScope();
            LiveSmokeOpensSampleProjectHeadlessWritesAttributedLogAndExitsZero();
        }

        private static void ParsesAllTenUnityCommandLineArguments()
        {
            string[] args =
            {
                "-batchmode",
                "-nographics",
                "-quit",
                "-projectPath",
                "/tmp/sample-project",
                "-buildTarget",
                "StandaloneOSX",
                "-executeMethod",
                "BuildScript.PerformBuild",
                "-logFile",
                "/tmp/unitas-build.log",
                "-runTests",
                "-testPlatform",
                "EditMode",
                "-testResults",
                "/tmp/test-results.xml",
            };

            CliArgs parsed = CliArgs.Parse(args);
            Check.Equal("cli-args: batchMode parsed", true, parsed.BatchMode);
            Check.Equal("cli-args: noGraphics parsed", true, parsed.NoGraphics);
            Check.Equal("cli-args: quit parsed", true, parsed.Quit);
            Check.Equal("cli-args: projectPath parsed", "/tmp/sample-project", parsed.ProjectPath ?? string.Empty);
            Check.Equal("cli-args: buildTarget parsed", "StandaloneOSX", parsed.BuildTarget ?? string.Empty);
            Check.Equal("cli-args: executeMethod parsed", "BuildScript.PerformBuild", parsed.ExecuteMethod ?? string.Empty);
            Check.Equal("cli-args: logFile parsed", "/tmp/unitas-build.log", parsed.LogFile ?? string.Empty);
            Check.Equal("cli-args: runTests parsed", true, parsed.RunTests);
            Check.Equal("cli-args: testPlatform parsed", "EditMode", parsed.TestPlatform ?? string.Empty);
            Check.Equal("cli-args: testResults parsed", "/tmp/test-results.xml", parsed.TestResults ?? string.Empty);
            Check.Equal("cli-args: zero unrecognized arguments", 0, parsed.UnrecognizedArguments.Count);
            Check.Equal("cli-args: zero missing values", 0, parsed.MissingValueArguments.Count);
            Check.Equal("cli-args: HasValidationErrors is false", false, parsed.HasValidationErrors);
        }

        private static void IsCliInvocationDistinguishesCliFromStdioAndHttp()
        {
            Check.Equal("cli-detect: empty args is false", false, HeadlessCli.IsCliInvocation(Array.Empty<string>()));
            Check.Equal("cli-detect: --stdio is false", false, HeadlessCli.IsCliInvocation(StdioArg));
            Check.Equal("cli-detect: --http is false", false, HeadlessCli.IsCliInvocation(HttpZeroArgs));
            Check.Equal("cli-detect: -batchmode is true", true, HeadlessCli.IsCliInvocation(BatchQuitArgs));
            Check.Equal("cli-detect: -projectPath is true", true, HeadlessCli.IsCliInvocation(ProjectPathDotArgs));
            Check.Equal("cli-detect: unknown flag is true so CLI reports it", true, HeadlessCli.IsCliInvocation(CustomUnknownFlagArg));
        }

        private static void UnrecognizedArgumentsAreReportedByNameAndReturnInvalidInvocation()
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            using StringWriter output = new StringWriter();
            int exitCode = HeadlessCli.Run(
                host,
                UnrecognizedTestArgs,
                output);

            Check.Equal("cli-unrecognized: exit code is InvalidInvocation (2)", CliExitCode.InvalidInvocation, exitCode);
            JsonObject? root = JsonNode.Parse(output.ToString()) as JsonObject;
            Check.That("cli-unrecognized: output is valid JSON object", root != null);
            Check.Equal("cli-unrecognized: ok is false", false, root?["ok"]?.GetValue<bool>() ?? true);
            Check.Equal("cli-unrecognized: exitCode field is 2", CliExitCode.InvalidInvocation, root?["exitCode"]?.GetValue<int>() ?? -1);
            Check.Equal("cli-unrecognized: exitStatusName is InvalidInvocation", "InvalidInvocation", root?["exitStatusName"]?.GetValue<string>() ?? string.Empty);

            JsonArray? unrecognized = root?["unrecognizedArguments"] as JsonArray;
            Check.Equal("cli-unrecognized: two unrecognized arguments listed", 2, unrecognized?.Count ?? 0);
            Check.Equal("cli-unrecognized: first unrecognized argument named", "-unknownFlagAlpha", unrecognized?[0]?.GetValue<string>() ?? string.Empty);
            Check.Equal("cli-unrecognized: second unrecognized argument named", "--mysteryOption", unrecognized?[1]?.GetValue<string>() ?? string.Empty);

            JsonArray? operations = root?["operations"] as JsonArray;
            Check.Equal("cli-unrecognized: one cli.parse operation recorded", 1, operations?.Count ?? 0);
            JsonObject? parseOp = operations?[0] as JsonObject;
            Check.Equal("cli-unrecognized: operation outcome is invalid-invocation", "invalid-invocation", parseOp?["outcome"]?.GetValue<string>() ?? string.Empty);
        }

        private static void MissingOptionValueReportsOptionByNameAndReturnsInvalidInvocation()
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            CliResult result = HeadlessCli.Execute(
                host,
                MissingProjectPathValArgs);

            Check.Equal("cli-missing-val: exit code is InvalidInvocation (2)", CliExitCode.InvalidInvocation, result.ExitCode);
            Check.Equal("cli-missing-val: ok is false", false, result.Ok);
            Check.Equal("cli-missing-val: -projectPath listed in UnrecognizedArguments", 1, result.UnrecognizedArguments.Count);
            Check.Equal("cli-missing-val: missing option name is -projectPath", "-projectPath", result.UnrecognizedArguments[0]);
            Check.Equal("cli-missing-val: diagnostic code is cli.missing-argument-value", "cli.missing-argument-value", result.Operations[0].Diagnostics[0].Code);
        }

        private static void NonExistentProjectPathReturnsProjectOpenFailed()
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            string missingPath = Path.Combine(Path.GetTempPath(), "unitas-cli-missing-" + Guid.NewGuid().ToString("N"));
            using StringWriter output = new StringWriter();
            int exitCode = HeadlessCli.Run(
                host,
                new[] { "-batchmode", "-nographics", "-quit", "-projectPath", missingPath },
                output);

            Check.Equal("cli-missing-project: exit code is ProjectOpenFailed (3)", CliExitCode.ProjectOpenFailed, exitCode);
            JsonObject? root = JsonNode.Parse(output.ToString()) as JsonObject;
            Check.Equal("cli-missing-project: ok is false", false, root?["ok"]?.GetValue<bool>() ?? true);
            Check.Equal("cli-missing-project: exitStatusName is ProjectOpenFailed", "ProjectOpenFailed", root?["exitStatusName"]?.GetValue<string>() ?? string.Empty);

            JsonObject? openOp = (root?["operations"] as JsonArray)?[0] as JsonObject;
            Check.Equal("cli-missing-project: operation is project.open", "project.open", openOp?["operation"]?.GetValue<string>() ?? string.Empty);
            Check.Equal("cli-missing-project: outcome is project-open-failed", "project-open-failed", openOp?["outcome"]?.GetValue<string>() ?? string.Empty);
        }

        private static void UnsupportedRunTestsExecuteMethodAndBuildTargetReturnUnsupportedOperation()
        {
            string tempProject = CreateTempProjectDirectory();
            try
            {
                McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));

                CliResult runTestsResult = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-nographics",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-runTests",
                        "-testPlatform",
                        "EditMode",
                        "-testResults",
                        Path.Combine(tempProject, "results.xml"),
                    });
                Check.Equal("cli-unsupported-tests: exit code is UnsupportedOperation (9)", CliExitCode.UnsupportedOperation, runTestsResult.ExitCode);
                Check.Equal("cli-unsupported-tests: ok is false", false, runTestsResult.Ok);
                Check.Equal("cli-unsupported-tests: two operations (project.open + tests.run)", 2, runTestsResult.Operations.Count);
                Check.Equal("cli-unsupported-tests: tests.run outcome is unsupported-operation", "unsupported-operation", runTestsResult.Operations[1].Outcome);
                Check.Equal("cli-unsupported-tests: tests.run compatibilityState is Unsupported", "Unsupported", runTestsResult.Operations[1].CompatibilityState);
                Check.Equal("cli-unsupported-tests: diagnostic code is cli.unsupported.runTests", "cli.unsupported.runTests", runTestsResult.Operations[1].Diagnostics[0].Code);

                CliResult executeMethodResult = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-executeMethod",
                        "MyEditorScript.BuildAll",
                    });
                Check.Equal("cli-unsupported-method: exit code is UnsupportedOperation (9)", CliExitCode.UnsupportedOperation, executeMethodResult.ExitCode);
                Check.Equal("cli-unsupported-method: ok is false", false, executeMethodResult.Ok);
                Check.Equal("cli-unsupported-method: outcome is unsupported-operation", "unsupported-operation", executeMethodResult.Operations[1].Outcome);
                Check.Equal("cli-unsupported-method: compatibilityState is Unsupported", "Unsupported", executeMethodResult.Operations[1].CompatibilityState);
                Check.Equal("cli-unsupported-method: diagnostic code is cli.unsupported.executeMethod", "cli.unsupported.executeMethod", executeMethodResult.Operations[1].Diagnostics[0].Code);
                Check.That(
                    "cli-unsupported-method: diagnostic message names method",
                    executeMethodResult.Operations[1].Diagnostics[0].Message.Contains("MyEditorScript.BuildAll", StringComparison.Ordinal));

                CliResult buildTargetResult = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-buildTarget",
                        "StandaloneOSX",
                    });
                Check.Equal("cli-unsupported-build: exit code is UnsupportedOperation (9)", CliExitCode.UnsupportedOperation, buildTargetResult.ExitCode);
                Check.Equal("cli-unsupported-build: ok is false", false, buildTargetResult.Ok);
                Check.Equal("cli-unsupported-build: outcome is unsupported-operation", "unsupported-operation", buildTargetResult.Operations[1].Outcome);
                Check.Equal("cli-unsupported-build: compatibilityState is Unsupported", "Unsupported", buildTargetResult.Operations[1].CompatibilityState);
                Check.Equal("cli-unsupported-build: diagnostic code is cli.unsupported.buildTarget", "cli.unsupported.buildTarget", buildTargetResult.Operations[1].Diagnostics[0].Code);
                Check.That(
                    "cli-unsupported-build: diagnostic message names target",
                    buildTargetResult.Operations[1].Diagnostics[0].Message.Contains("StandaloneOSX", StringComparison.Ordinal));
            }
            finally
            {
                if (Directory.Exists(tempProject))
                {
                    Directory.Delete(tempProject, true);
                }
            }
        }

        private static void RegisteredToolsExecuteThroughMcpHostAndHonorUntrustedProjectScope()
        {
            string tempProject = CreateTempProjectDirectory();
            try
            {
                McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
                host.RegisterTool(
                    new ToolDefinition(
                        "editor.execute_method",
                        "Execute static editor method",
                        new JsonObject { ["type"] = "object" },
                        Scope.ExecuteProjectCode),
                    arguments => ToolOutput.Untrusted(
                        "unitas://editor/execute_method",
                        new JsonObject
                        {
                            ["ok"] = true,
                            ["method"] = arguments["method"]?.GetValue<string>(),
                        }));

                CliResult untrustedResult = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-executeMethod",
                        "BuildTools.Run",
                    });
                Check.Equal(
                    "cli-untrusted: missing ExecuteProjectCode scope returns UntrustedProject (4)",
                    CliExitCode.UntrustedProject,
                    untrustedResult.ExitCode);
                Check.Equal(
                    "cli-untrusted: operation outcome is refused-untrusted-project",
                    "refused-untrusted-project",
                    untrustedResult.Operations[1].Outcome);

                host.GrantStanding(HeadlessCli.CliAgentName, Scope.ExecuteProjectCode, GrantAuthority.Person);
                CliResult trustedResult = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-executeMethod",
                        "BuildTools.Run",
                    });
                Check.Equal("cli-trusted: granted ExecuteProjectCode scope exits 0", CliExitCode.Success, trustedResult.ExitCode);
                Check.Equal("cli-trusted: operation outcome is succeeded", "succeeded", trustedResult.Operations[1].Outcome);
                Check.That(
                    "cli-trusted: result carries McpHost untrusted envelope",
                    UntrustedContent.IsEnvelope(trustedResult.Operations[1].Result));
            }
            finally
            {
                if (Directory.Exists(tempProject))
                {
                    Directory.Delete(tempProject, true);
                }
            }
        }

        private static void LiveSmokeOpensSampleProjectHeadlessWritesAttributedLogAndExitsZero()
        {
            string tempDir = CreateTempProjectDirectory();
            try
            {
                string sampleProjectDir = ResolveSampleProjectDirectory(tempDir);
                string logFilePath = Path.Combine(tempDir, "headless-smoke.log");
                McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));

                using StringWriter output = new StringWriter();
                int exitCode = HeadlessCli.Run(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-nographics",
                        "-quit",
                        "-projectPath",
                        sampleProjectDir,
                        "-logFile",
                        logFilePath,
                    },
                    output);

                Check.Equal("cli-smoke: exit code is Success (0)", CliExitCode.Success, exitCode);
                JsonObject? root = JsonNode.Parse(output.ToString()) as JsonObject;
                Check.That("cli-smoke: stdout is machine-readable JSON object", root != null);
                Check.Equal("cli-smoke: ok is true", true, root?["ok"]?.GetValue<bool>() ?? false);
                Check.Equal("cli-smoke: exitCode is 0", 0, root?["exitCode"]?.GetValue<int>() ?? -1);
                Check.Equal("cli-smoke: exitStatusName is Success", "Success", root?["exitStatusName"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("cli-smoke: batchMode is true", true, root?["batchMode"]?.GetValue<bool>() ?? false);
                Check.Equal("cli-smoke: noGraphics is true", true, root?["noGraphics"]?.GetValue<bool>() ?? false);
                Check.Equal("cli-smoke: quit is true", true, root?["quit"]?.GetValue<bool>() ?? false);

                JsonArray? operations = root?["operations"] as JsonArray;
                Check.Equal("cli-smoke: one operation recorded", 1, operations?.Count ?? 0);
                JsonObject? openOp = operations?[0] as JsonObject;
                Check.Equal("cli-smoke: operation is project.open", "project.open", openOp?["operation"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("cli-smoke: outcome is succeeded", "succeeded", openOp?["outcome"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("cli-smoke: compatibilityState is Supported", "Supported", openOp?["compatibilityState"]?.GetValue<string>() ?? string.Empty);
                Check.That(
                    "cli-smoke: operation result is McpHost untrusted envelope",
                    UntrustedContent.IsEnvelope(openOp?["result"]));
                Check.Equal(
                    "cli-smoke: operation result state from project.status is ready",
                    "ready",
                    openOp?["result"]?["content"]?["state"]?.GetValue<string>() ?? string.Empty);

                Check.Equal("cli-smoke: McpHost recorded audit entry for project.status", 1, host.Audit.Count);
                Check.Equal("cli-smoke: audit target is project.status", "project.status", host.Audit.Records[0].Target);

                Check.That("cli-smoke: log file was written", File.Exists(logFilePath));
                string logContent = File.ReadAllText(logFilePath);
                Check.That(
                    "cli-smoke: log entry is attributed to [project.open]",
                    logContent.Contains("[project.open] succeeded:", StringComparison.Ordinal));
            }
            finally
            {
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static string CreateTempProjectDirectory()
        {
            string path = Path.Combine(Path.GetTempPath(), "unitas-cli-test-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(path);
            return path;
        }

        private static string ResolveSampleProjectDirectory(string fallbackDir)
        {
            string candidate = Path.GetFullPath(
                Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "crates", "unitas-build", "tests", "fixtures"));
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            return fallbackDir;
        }
    }
}
