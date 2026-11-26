using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class HeadlessCli
    {
        public const int ExitSuccess = CliExitCode.Success;
        public const int ExitInvalidInvocation = CliExitCode.InvalidInvocation;
        public const int ExitProjectOpenFailed = CliExitCode.ProjectOpenFailed;
        public const int ExitUntrustedProject = CliExitCode.UntrustedProject;
        public const int ExitCompilationFailed = CliExitCode.CompilationFailed;
        public const int ExitTestOrScenarioFailed = CliExitCode.TestOrScenarioFailed;
        public const int ExitBuildFailed = CliExitCode.BuildFailed;
        public const int ExitMissingToolchainOrEntitlement = CliExitCode.MissingToolchainOrEntitlement;
        public const int ExitUnsupportedOperation = CliExitCode.UnsupportedOperation;
        public const int ExitCancelledOrTimedOut = CliExitCode.CancelledOrTimedOut;

        public const string CliAgentName = "headless-cli";
        public const string CliClientName = "cli";
        public const string CliAuthorizedBy = "cli-local";

        public static bool IsCliInvocation(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                return false;
            }

            string first = args[0] ?? string.Empty;
            if (string.Equals(first, "--stdio", StringComparison.Ordinal)
                || string.Equals(first, "--http", StringComparison.Ordinal))
            {
                return false;
            }

            foreach (string raw in args)
            {
                string argument = raw ?? string.Empty;
                if (CliArgs.IsKnownFlagOrOption(argument))
                {
                    return true;
                }

                if (argument.StartsWith('-')
                    && !string.Equals(argument, "--stdio", StringComparison.Ordinal)
                    && !string.Equals(argument, "--http", StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        public static int Run(McpHost host, string[] args, TextWriter? output = null)
        {
            CliResult result = Execute(host, args);
            TextWriter writer = output ?? Console.Out;
            writer.WriteLine(result.ToJsonString());
            return result.ExitCode;
        }

        public static CliResult Execute(McpHost host, string[] args)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(args);

            CliArgs parsed = CliArgs.Parse(args);
            string? resolvedLogLocation = ResolveLogLocation(parsed.LogFile);
            List<CliOperationRecord> operations = new List<CliOperationRecord>();
            List<string> logLines = new List<string>();

            if (parsed.HasValidationErrors)
            {
                List<CliDiagnostic> diagnostics = new List<CliDiagnostic>();
                HashSet<string> missingSet = new HashSet<string>(parsed.MissingValueArguments, StringComparer.Ordinal);
                foreach (string argument in parsed.UnrecognizedArguments)
                {
                    bool isMissingValue = missingSet.Contains(argument);
                    string code = isMissingValue
                        ? "cli.missing-argument-value"
                        : "cli.unrecognized-argument";
                    string message = isMissingValue
                        ? ("Option " + argument + " requires a value.")
                        : ("Unrecognized command-line argument: " + argument);
                    diagnostics.Add(new CliDiagnostic(
                        code,
                        "error",
                        message,
                        "argv",
                        argument));
                    AppendAttributedLog(logLines, "cli.parse", "invalid-invocation", message);
                }

                operations.Add(new CliOperationRecord(
                    "cli.parse",
                    "invalid-invocation",
                    "Unsupported",
                    diagnostics,
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));

                FlushLogFile(resolvedLogLocation, logLines);
                return BuildResult(
                    CliExitCode.InvalidInvocation,
                    parsed,
                    parsed.ProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
            }

            string resolvedProjectPath = ResolveProjectPath(parsed.ProjectPath);
            if (!Directory.Exists(resolvedProjectPath))
            {
                string message = "Project directory could not be opened because it does not exist: " + resolvedProjectPath;
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.project-not-found",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.ProjectPathOption);
                AppendAttributedLog(logLines, "project.open", "project-open-failed", message);
                operations.Add(new CliOperationRecord(
                    "project.open",
                    "project-open-failed",
                    "Supported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));

                FlushLogFile(resolvedLogLocation, logLines);
                return BuildResult(
                    CliExitCode.ProjectOpenFailed,
                    parsed,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
            }

            string sessionId = host.CreateSession(
                new SessionIdentity(CliAgentName, CliClientName, CliAuthorizedBy));

            int exitCode = CliExitCode.Success;
            if (!TryRunProjectOpen(
                host,
                sessionId,
                resolvedProjectPath,
                resolvedLogLocation,
                operations,
                logLines,
                out int projectOpenExitCode))
            {
                FlushLogFile(resolvedLogLocation, logLines);
                return BuildResult(
                    projectOpenExitCode,
                    parsed,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
            }

            if (parsed.RequestsTestExecution)
            {
                int testExit = RunTestsOperation(
                    host,
                    sessionId,
                    parsed,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
                exitCode = CombineExitCode(exitCode, testExit);
            }

            if (!string.IsNullOrEmpty(parsed.ExecuteMethod))
            {
                int methodExit = RunExecuteMethodOperation(
                    host,
                    sessionId,
                    parsed.ExecuteMethod!,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
                exitCode = CombineExitCode(exitCode, methodExit);
            }

            if (!string.IsNullOrEmpty(parsed.BuildTarget))
            {
                int buildExit = RunBuildTargetOperation(
                    host,
                    sessionId,
                    parsed.BuildTarget!,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
                exitCode = CombineExitCode(exitCode, buildExit);
            }

            if (!string.IsNullOrEmpty(parsed.RunScenario))
            {
                int scenarioExit = RunScenarioOperation(
                    host,
                    sessionId,
                    parsed.RunScenario!,
                    resolvedProjectPath,
                    resolvedLogLocation,
                    operations,
                    logLines);
                exitCode = CombineExitCode(exitCode, scenarioExit);
            }

            FlushLogFile(resolvedLogLocation, logLines);
            return BuildResult(
                exitCode,
                parsed,
                resolvedProjectPath,
                resolvedLogLocation,
                operations,
                logLines);
        }

        private static bool TryRunProjectOpen(
            McpHost host,
            string sessionId,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            List<CliOperationRecord> operations,
            List<string> logLines,
            out int exitCode)
        {
            if (!HasRegisteredTool(host, "project.status"))
            {
                string message = "Required editor command project.status is not registered on the headless host.";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.project-status-unavailable",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.ProjectPathOption);
                AppendAttributedLog(logLines, "project.open", "project-open-failed", message);
                operations.Add(new CliOperationRecord(
                    "project.open",
                    "project-open-failed",
                    "Unsupported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                exitCode = CliExitCode.ProjectOpenFailed;
                return false;
            }

            try
            {
                JsonNode envelope = host.CallTool(
                    sessionId,
                    "project.status",
                    new JsonObject
                    {
                        ["projectPath"] = resolvedProjectPath,
                    });
                AppendAttributedLog(
                    logLines,
                    "project.open",
                    "succeeded",
                    "Opened project at " + resolvedProjectPath);
                operations.Add(new CliOperationRecord(
                    "project.open",
                    "succeeded",
                    "Supported",
                    Array.Empty<CliDiagnostic>(),
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    envelope));
                exitCode = CliExitCode.Success;
                return true;
            }
            catch (McpError error)
            {
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.project-open-error",
                    "error",
                    error.Message,
                    resolvedProjectPath,
                    CliArgs.ProjectPathOption);
                AppendAttributedLog(logLines, "project.open", "project-open-failed", error.Message);
                operations.Add(new CliOperationRecord(
                    "project.open",
                    "project-open-failed",
                    "Supported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                exitCode = CliExitCode.ProjectOpenFailed;
                return false;
            }
        }

        private static int RunTestsOperation(
            McpHost host,
            string sessionId,
            CliArgs parsed,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            List<CliOperationRecord> operations,
            List<string> logLines)
        {
            const string operationName = "tests.run";
            string platform = string.IsNullOrEmpty(parsed.TestPlatform) ? "EditMode" : parsed.TestPlatform!;
            if (!HasRegisteredTool(host, operationName))
            {
                string message =
                    "Command-line operation -runTests (platform="
                    + platform
                    + ") is not supported because editor command "
                    + operationName
                    + " is not registered.";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.unsupported.runTests",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.RunTestsFlag);
                AppendAttributedLog(logLines, operationName, "unsupported-operation", message);
                operations.Add(new CliOperationRecord(
                    operationName,
                    "unsupported-operation",
                    "Unsupported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                return CliExitCode.UnsupportedOperation;
            }

            JsonObject arguments = new JsonObject
            {
                ["projectPath"] = resolvedProjectPath,
                ["testPlatform"] = platform,
            };
            if (!string.IsNullOrEmpty(parsed.TestResults))
            {
                arguments["testResults"] = parsed.TestResults;
            }

            List<string> artifacts = new List<string>();
            if (!string.IsNullOrEmpty(parsed.TestResults))
            {
                artifacts.Add(parsed.TestResults!);
            }

            return InvokeRegisteredToolOperation(
                host,
                sessionId,
                operationName,
                CliArgs.RunTestsFlag,
                arguments,
                resolvedProjectPath,
                resolvedLogLocation,
                artifacts,
                CliExitCode.TestOrScenarioFailed,
                operations,
                logLines);
        }

        private static int RunExecuteMethodOperation(
            McpHost host,
            string sessionId,
            string methodName,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            List<CliOperationRecord> operations,
            List<string> logLines)
        {
            const string operationName = "editor.execute_method";
            if (!HasRegisteredTool(host, operationName))
            {
                string message =
                    "Command-line operation -executeMethod "
                    + methodName
                    + " is not supported because editor command "
                    + operationName
                    + " is not registered.";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.unsupported.executeMethod",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.ExecuteMethodOption);
                AppendAttributedLog(logLines, operationName, "unsupported-operation", message);
                operations.Add(new CliOperationRecord(
                    operationName,
                    "unsupported-operation",
                    "Unsupported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                return CliExitCode.UnsupportedOperation;
            }

            JsonObject arguments = new JsonObject
            {
                ["projectPath"] = resolvedProjectPath,
                ["method"] = methodName,
            };

            return InvokeRegisteredToolOperation(
                host,
                sessionId,
                operationName,
                CliArgs.ExecuteMethodOption,
                arguments,
                resolvedProjectPath,
                resolvedLogLocation,
                Array.Empty<string>(),
                CliExitCode.CompilationFailed,
                operations,
                logLines);
        }

        private static int RunBuildTargetOperation(
            McpHost host,
            string sessionId,
            string buildTarget,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            List<CliOperationRecord> operations,
            List<string> logLines)
        {
            const string operationName = "build.player";
            if (!HasRegisteredTool(host, operationName))
            {
                string message =
                    "Command-line operation -buildTarget "
                    + buildTarget
                    + " is not supported because editor command "
                    + operationName
                    + " is not registered.";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.unsupported.buildTarget",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.BuildTargetOption);
                AppendAttributedLog(logLines, operationName, "unsupported-operation", message);
                operations.Add(new CliOperationRecord(
                    operationName,
                    "unsupported-operation",
                    "Unsupported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                return CliExitCode.UnsupportedOperation;
            }

            JsonObject arguments = new JsonObject
            {
                ["projectPath"] = resolvedProjectPath,
                ["buildTarget"] = buildTarget,
            };

            return InvokeRegisteredToolOperation(
                host,
                sessionId,
                operationName,
                CliArgs.BuildTargetOption,
                arguments,
                resolvedProjectPath,
                resolvedLogLocation,
                Array.Empty<string>(),
                CliExitCode.BuildFailed,
                operations,
                logLines);
        }

        private static int RunScenarioOperation(
            McpHost host,
            string sessionId,
            string scenarioPath,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            List<CliOperationRecord> operations,
            List<string> logLines)
        {
            const string operationName = "scenario.run";
            if (!HasRegisteredTool(host, operationName))
            {
                string message =
                    "Command-line operation -runScenario is not supported because editor command "
                    + operationName
                    + " is not registered.";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.unsupported.runScenario",
                    "error",
                    message,
                    resolvedProjectPath,
                    CliArgs.RunScenarioOption);
                AppendAttributedLog(logLines, operationName, "unsupported-operation", message);
                operations.Add(new CliOperationRecord(
                    operationName,
                    "unsupported-operation",
                    "Unsupported",
                    new[] { diagnostic },
                    Array.Empty<string>(),
                    resolvedLogLocation,
                    null));
                return CliExitCode.UnsupportedOperation;
            }

            host.GrantStanding(CliAgentName, Scope.ExecuteProjectCode, GrantAuthority.Person);

            // A headless scenario is a person-driven local invocation, so the
            // session it runs in may author scene content through the editor's
            // own scene commands; the grant is session scoped and never stands.
            host.GrantScope(sessionId, Scope.EditContent, GrantAuthority.Person);
            JsonObject arguments = new JsonObject
            {
                ["scenarioPath"] = scenarioPath,
            };

            return InvokeRegisteredToolOperation(
                host,
                sessionId,
                operationName,
                CliArgs.RunScenarioOption,
                arguments,
                resolvedProjectPath,
                resolvedLogLocation,
                Array.Empty<string>(),
                CliExitCode.TestOrScenarioFailed,
                operations,
                logLines);
        }

        private static int InvokeRegisteredToolOperation(
            McpHost host,
            string sessionId,
            string toolName,
            string argumentFlag,
            JsonObject arguments,
            string resolvedProjectPath,
            string? resolvedLogLocation,
            IReadOnlyList<string> defaultArtifacts,
            int defaultFailureExitCode,
            List<CliOperationRecord> operations,
            List<string> logLines)
        {
            try
            {
                JsonNode envelope = host.CallTool(sessionId, toolName, arguments);
                JsonObject? content = (envelope as JsonObject)?["content"] as JsonObject;
                string? reportedCompatibility = content?["compatibilityState"]?.GetValue<string>();
                if (string.Equals(reportedCompatibility, "Unsupported", StringComparison.Ordinal))
                {
                    string reason = content?["message"]?.GetValue<string>()
                        ?? ("Operation " + toolName + " reported Unsupported.");
                    CliDiagnostic diagnostic = new CliDiagnostic(
                        "cli.unsupported." + toolName,
                        "error",
                        reason,
                        resolvedProjectPath,
                        argumentFlag);
                    AppendAttributedLog(logLines, toolName, "unsupported-operation", reason);
                    operations.Add(new CliOperationRecord(
                        toolName,
                        "unsupported-operation",
                        "Unsupported",
                        new[] { diagnostic },
                        defaultArtifacts,
                        resolvedLogLocation,
                        envelope));
                    return CliExitCode.UnsupportedOperation;
                }

                bool toolReportedFailure = content?["ok"] is JsonValue okVal
                    && okVal.TryGetValue(out bool toolOk)
                    && !toolOk;
                if (toolReportedFailure)
                {
                    string errorDetail = content?["error"]?.GetValue<string>()
                        ?? ("Operation " + toolName + " failed.");
                    CliDiagnostic diagnostic = new CliDiagnostic(
                        "cli.operation-failed." + toolName,
                        "error",
                        errorDetail,
                        resolvedProjectPath,
                        argumentFlag);
                    AppendAttributedLog(logLines, toolName, "failed", errorDetail);
                    operations.Add(new CliOperationRecord(
                        toolName,
                        "failed",
                        reportedCompatibility ?? "Supported",
                        new[] { diagnostic },
                        defaultArtifacts,
                        resolvedLogLocation,
                        envelope));
                    return defaultFailureExitCode;
                }

                AppendAttributedLog(logLines, toolName, "succeeded", "Completed " + toolName);
                operations.Add(new CliOperationRecord(
                    toolName,
                    "succeeded",
                    reportedCompatibility ?? "Supported",
                    Array.Empty<CliDiagnostic>(),
                    defaultArtifacts,
                    resolvedLogLocation,
                    envelope));
                return CliExitCode.Success;
            }
            catch (McpError.MissingScope missingScope)
            {
                int mappedExitCode = missingScope.Scope == Scope.ExecuteProjectCode
                    ? CliExitCode.UntrustedProject
                    : defaultFailureExitCode;
                string outcome = missingScope.Scope == Scope.ExecuteProjectCode
                    ? "refused-untrusted-project"
                    : "refused-missing-scope";
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.missing-scope",
                    "error",
                    missingScope.Message,
                    resolvedProjectPath,
                    argumentFlag);
                AppendAttributedLog(logLines, toolName, outcome, missingScope.Message);
                operations.Add(new CliOperationRecord(
                    toolName,
                    outcome,
                    "Supported",
                    new[] { diagnostic },
                    defaultArtifacts,
                    resolvedLogLocation,
                    null));
                return mappedExitCode;
            }
            catch (McpError error)
            {
                CliDiagnostic diagnostic = new CliDiagnostic(
                    "cli.tool-error",
                    "error",
                    error.Message,
                    resolvedProjectPath,
                    argumentFlag);
                AppendAttributedLog(logLines, toolName, "failed", error.Message);
                operations.Add(new CliOperationRecord(
                    toolName,
                    "failed",
                    "Supported",
                    new[] { diagnostic },
                    defaultArtifacts,
                    resolvedLogLocation,
                    null));
                return defaultFailureExitCode;
            }
        }

        private static bool HasRegisteredTool(McpHost host, string toolName)
        {
            foreach (ToolDefinition definition in host.ListTools())
            {
                if (string.Equals(definition.Name, toolName, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static int CombineExitCode(int currentExitCode, int operationExitCode)
        {
            if (currentExitCode != CliExitCode.Success)
            {
                return currentExitCode;
            }

            return operationExitCode;
        }

        private static string ResolveProjectPath(string? requestedProjectPath)
        {
            if (string.IsNullOrWhiteSpace(requestedProjectPath))
            {
                return Directory.GetCurrentDirectory();
            }

            return Path.GetFullPath(requestedProjectPath);
        }

        private static string? ResolveLogLocation(string? requestedLogFile)
        {
            if (string.IsNullOrWhiteSpace(requestedLogFile))
            {
                return null;
            }

            if (string.Equals(requestedLogFile, "-", StringComparison.Ordinal))
            {
                return "-";
            }

            return Path.GetFullPath(requestedLogFile);
        }

        private static void AppendAttributedLog(
            List<string> logLines,
            string operation,
            string outcome,
            string detail)
        {
            logLines.Add("[" + operation + "] " + outcome + ": " + detail);
        }

        private static void FlushLogFile(string? resolvedLogLocation, List<string> logLines)
        {
            if (string.IsNullOrEmpty(resolvedLogLocation)
                || string.Equals(resolvedLogLocation, "-", StringComparison.Ordinal))
            {
                return;
            }

            string? directory = Path.GetDirectoryName(resolvedLogLocation);
            if (!string.IsNullOrEmpty(directory) && !Directory.Exists(directory))
            {
                Directory.CreateDirectory(directory);
            }

            File.WriteAllLines(resolvedLogLocation, logLines);
        }

        private static CliResult BuildResult(
            int exitCode,
            CliArgs parsed,
            string? resolvedProjectPath,
            string? resolvedLogLocation,
            IReadOnlyList<CliOperationRecord> operations,
            IReadOnlyList<string> logLines)
        {
            return new CliResult(
                exitCode == CliExitCode.Success,
                exitCode,
                CliExitCode.NameOf(exitCode),
                parsed.BatchMode,
                parsed.NoGraphics,
                parsed.Quit,
                resolvedProjectPath,
                resolvedLogLocation,
                parsed.UnrecognizedArguments,
                operations,
                logLines);
        }
    }
}
