using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;

namespace Unitas.Benchmark
{
    public sealed class UsageException : Exception
    {
        public UsageException(string message)
            : base(message)
        {
        }
    }

    public sealed class BenchmarkOptions
    {
        public const string EditorOption = "--editor";
        public const string ProjectOption = "--project";
        public const string PlanOption = "--plan";
        public const string ReportOption = "--report";

        private BenchmarkOptions(
            string editorPath,
            string projectDirectory,
            string planPath,
            string? reportPath)
        {
            EditorPath = editorPath;
            ProjectDirectory = projectDirectory;
            PlanPath = planPath;
            ReportPath = reportPath;
        }

        public string EditorPath { get; }

        public string ProjectDirectory { get; }

        public string PlanPath { get; }

        public string? ReportPath { get; }

        public static BenchmarkOptions Parse(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                throw new UsageException("no arguments were given");
            }

            Dictionary<string, string> values = new Dictionary<string, string>(StringComparer.Ordinal);
            int index = 0;
            while (index < args.Length)
            {
                string option = args[index];
                if (option != EditorOption
                    && option != ProjectOption
                    && option != PlanOption
                    && option != ReportOption)
                {
                    throw new UsageException("unknown option: " + option);
                }

                if (index + 1 >= args.Length)
                {
                    throw new UsageException(option + " needs a value");
                }

                if (values.ContainsKey(option))
                {
                    throw new UsageException(option + " was given twice");
                }

                values[option] = args[index + 1];
                index += 2;
            }

            string editor = Require(values, EditorOption);
            string project = Require(values, ProjectOption);
            string plan = Require(values, PlanOption);
            string? report = values.TryGetValue(ReportOption, out string? reportPath)
                ? reportPath
                : null;

            if (!File.Exists(editor))
            {
                throw new UsageException("the editor assembly does not exist: " + editor);
            }

            if (!Directory.Exists(project))
            {
                throw new UsageException("the project directory does not exist: " + project);
            }

            if (!File.Exists(plan))
            {
                throw new UsageException("the step plan does not exist: " + plan);
            }

            return new BenchmarkOptions(
                FullPath(editor, EditorOption),
                FullPath(project, ProjectOption),
                FullPath(plan, PlanOption),
                report == null ? null : FullPath(report, ReportOption));
        }

        private static string Require(Dictionary<string, string> values, string option)
        {
            if (!values.TryGetValue(option, out string? value) || value.Length == 0)
            {
                throw new UsageException(option + " is required");
            }

            return value;
        }

        private static string FullPath(string path, string option)
        {
            try
            {
                return Path.GetFullPath(path);
            }
            catch (Exception error) when (error is ArgumentException
                || error is NotSupportedException
                || error is PathTooLongException)
            {
                throw new UsageException(option + " received an invalid path: " + error.Message);
            }
        }
    }

    public static class Program
    {
        public const int SuccessExitCode = 0;
        public const int FailedStepExitCode = 1;
        public const int UsageExitCode = 2;

        public const string CredentialVariable = "UNITAS_BENCHMARK_PERSON_CREDENTIAL";

        public const string CredentialSourceVariable = "UNITAS_BENCHMARK_CREDENTIAL_SOURCE";

        private const string UsageText =
            "usage: Unitas.Benchmark --editor <dll> --project <dir> --plan <agent-steps.json> --report <report.json>";

        public static async Task<int> Main(string[] args)
        {
            BenchmarkOptions options;
            try
            {
                options = BenchmarkOptions.Parse(args);
            }
            catch (UsageException error)
            {
                await Console.Error.WriteLineAsync("Unitas.Benchmark: " + error.Message).ConfigureAwait(false);
                await Console.Error.WriteLineAsync(UsageText).ConfigureAwait(false);
                return UsageExitCode;
            }

            StepPlan plan;
            try
            {
                plan = StepPlan.Load(options.PlanPath);
            }
            catch (UsageException error)
            {
                await Console.Error.WriteLineAsync("Unitas.Benchmark: " + error.Message).ConfigureAwait(false);
                return UsageExitCode;
            }

            string? credential = ReadPersonCredential();
            BenchmarkReport report;
            try
            {
                await using McpSession session = await McpSession.StartAsync(
                    options.EditorPath,
                    options.ProjectDirectory,
                    credential,
                    CancellationToken.None).ConfigureAwait(false);
                IReadOnlyList<string> tools = await session
                    .ListToolNamesAsync(CancellationToken.None)
                    .ConfigureAwait(false);
                report = await RunStepsAsync(session, plan, tools, credential).ConfigureAwait(false);
            }
            catch (Exception error)
            {
                await Console.Error.WriteLineAsync(
                    "Unitas.Benchmark: " + error.Message).ConfigureAwait(false);
                return FailedStepExitCode;
            }

            string json = report.ToJson();
            if (options.ReportPath != null)
            {
                try
                {
                    string? reportDirectory = Path.GetDirectoryName(options.ReportPath);
                    if (!string.IsNullOrEmpty(reportDirectory))
                    {
                        Directory.CreateDirectory(reportDirectory);
                    }

                    await File.WriteAllTextAsync(options.ReportPath, json).ConfigureAwait(false);
                }
                catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
                {
                    await Console.Error.WriteLineAsync(
                        "Unitas.Benchmark: the report could not be written: " + error.Message).ConfigureAwait(false);
                    return FailedStepExitCode;
                }
            }

            await Console.Out.WriteLineAsync(json).ConfigureAwait(false);
            return report.AllStepsSucceeded ? SuccessExitCode : FailedStepExitCode;
        }

        private static async Task<BenchmarkReport> RunStepsAsync(
            McpSession session,
            StepPlan plan,
            IReadOnlyList<string> tools,
            string? credential)
        {
            List<StepReport> reports = new List<StepReport>();
            foreach (Step step in plan.Steps)
            {
                ToolCallOutcome outcome = await session
                    .CallAsync(step.Tool, step.Arguments, CancellationToken.None)
                    .ConfigureAwait(false);
                reports.Add(new StepReport(
                    step.Role,
                    step.Tool,
                    (JsonObject?)BenchmarkReport.Redact(step.Arguments, credential) ?? new JsonObject(),
                    outcome.Ok,
                    outcome.Error,
                    BenchmarkReport.Redact(outcome.Result, credential)));
            }

            return new BenchmarkReport(reports, tools);
        }

        private static string? ReadPersonCredential()
        {
            string? direct = Environment.GetEnvironmentVariable(
                McpSession.PersonCredentialEnvironmentVariable);
            if (!string.IsNullOrWhiteSpace(direct))
            {
                return direct;
            }

            string source = CredentialVariable;
            string? named = Environment.GetEnvironmentVariable(CredentialSourceVariable);
            if (!string.IsNullOrWhiteSpace(named))
            {
                source = named;
            }

            string? credential = Environment.GetEnvironmentVariable(source);
            return string.IsNullOrWhiteSpace(credential) ? null : credential;
        }
    }
}
