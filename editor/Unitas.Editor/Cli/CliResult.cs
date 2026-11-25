using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class CliExitCode
    {
        public const int Success = 0;
        public const int InvalidInvocation = 2;
        public const int ProjectOpenFailed = 3;
        public const int UntrustedProject = 4;
        public const int CompilationFailed = 5;
        public const int TestOrScenarioFailed = 6;
        public const int BuildFailed = 7;
        public const int MissingToolchainOrEntitlement = 8;
        public const int UnsupportedOperation = 9;
        public const int CancelledOrTimedOut = 10;

        public static string NameOf(int exitCode)
        {
            switch (exitCode)
            {
                case Success: return "Success";
                case InvalidInvocation: return "InvalidInvocation";
                case ProjectOpenFailed: return "ProjectOpenFailed";
                case UntrustedProject: return "UntrustedProject";
                case CompilationFailed: return "CompilationFailed";
                case TestOrScenarioFailed: return "TestOrScenarioFailed";
                case BuildFailed: return "BuildFailed";
                case MissingToolchainOrEntitlement: return "MissingToolchainOrEntitlement";
                case UnsupportedOperation: return "UnsupportedOperation";
                case CancelledOrTimedOut: return "CancelledOrTimedOut";
                default: return "UnknownExitCode";
            }
        }
    }

    public sealed record CliDiagnostic(
        string Code,
        string Severity,
        string Message,
        string? Location,
        string? Argument)
    {
        public JsonObject ToValue()
        {
            JsonObject value = new JsonObject
            {
                ["code"] = Code,
                ["severity"] = Severity,
                ["message"] = Message,
                ["location"] = Location,
            };
            if (Argument != null)
            {
                value["argument"] = Argument;
            }

            return value;
        }
    }

    public sealed record CliOperationRecord(
        string Operation,
        string Outcome,
        string CompatibilityState,
        IReadOnlyList<CliDiagnostic> Diagnostics,
        IReadOnlyList<string> Artifacts,
        string? LogLocation,
        JsonNode? Result)
    {
        public JsonObject ToValue()
        {
            JsonArray diagnosticsArray = new JsonArray();
            foreach (CliDiagnostic diagnostic in Diagnostics)
            {
                diagnosticsArray.Add(diagnostic.ToValue());
            }

            JsonArray artifactsArray = new JsonArray();
            foreach (string artifact in Artifacts)
            {
                artifactsArray.Add(artifact);
            }

            return new JsonObject
            {
                ["operation"] = Operation,
                ["outcome"] = Outcome,
                ["compatibilityState"] = CompatibilityState,
                ["diagnostics"] = diagnosticsArray,
                ["artifacts"] = artifactsArray,
                ["logLocation"] = LogLocation,
                ["result"] = Result?.DeepClone(),
            };
        }
    }

    public sealed class CliResult
    {
        public CliResult(
            bool ok,
            int exitCode,
            string exitStatusName,
            bool batchMode,
            bool noGraphics,
            bool quit,
            string? projectPath,
            string? logFile,
            IReadOnlyList<string> unrecognizedArguments,
            IReadOnlyList<CliOperationRecord> operations,
            IReadOnlyList<string> logEntries)
        {
            ArgumentNullException.ThrowIfNull(exitStatusName);
            ArgumentNullException.ThrowIfNull(unrecognizedArguments);
            ArgumentNullException.ThrowIfNull(operations);
            ArgumentNullException.ThrowIfNull(logEntries);
            Ok = ok;
            ExitCode = exitCode;
            ExitStatusName = exitStatusName;
            BatchMode = batchMode;
            NoGraphics = noGraphics;
            Quit = quit;
            ProjectPath = projectPath;
            LogFile = logFile;
            UnrecognizedArguments = unrecognizedArguments;
            Operations = operations;
            LogEntries = logEntries;
        }

        public bool Ok { get; }

        public int ExitCode { get; }

        public string ExitStatusName { get; }

        public bool BatchMode { get; }

        public bool NoGraphics { get; }

        public bool Quit { get; }

        public string? ProjectPath { get; }

        public string? LogFile { get; }

        public IReadOnlyList<string> UnrecognizedArguments { get; }

        public IReadOnlyList<CliOperationRecord> Operations { get; }

        public IReadOnlyList<string> LogEntries { get; }

        public JsonObject ToValue()
        {
            JsonArray unrecognizedArray = new JsonArray();
            foreach (string argument in UnrecognizedArguments)
            {
                unrecognizedArray.Add(argument);
            }

            JsonArray operationsArray = new JsonArray();
            foreach (CliOperationRecord operation in Operations)
            {
                operationsArray.Add(operation.ToValue());
            }

            return new JsonObject
            {
                ["ok"] = Ok,
                ["exitCode"] = ExitCode,
                ["exitStatusName"] = ExitStatusName,
                ["batchMode"] = BatchMode,
                ["noGraphics"] = NoGraphics,
                ["quit"] = Quit,
                ["projectPath"] = ProjectPath,
                ["logFile"] = LogFile,
                ["unrecognizedArguments"] = unrecognizedArray,
                ["operations"] = operationsArray,
            };
        }

        public string ToJsonString()
        {
            return ToValue().ToJsonString();
        }
    }
}
