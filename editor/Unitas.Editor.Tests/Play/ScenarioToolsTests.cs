using System;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;
using Unitas.Scripting;

namespace Unitas.Editor.Tests.Play
{
    public static class ScenarioToolsTests
    {
        // The sample's acceptance scenarios for the current twelve-pickup
        // arena; the older collect_all_pickups/restart_recovery documents
        // still describe the pre-marble-rush four-pickup route.
        private const string CollectScenario = "samples/marble-rush/Scenarios/collect_all.json";
        private const string RestartScenario = "samples/marble-rush/Scenarios/retry_clean.json";
        private const string ScenePath = "samples/marble-rush/Assets/Scenes/MarbleRush.unity";

        public static void Run()
        {
            CliArgsParsesRunScenarioOption();
            EnsureSampleProjectScriptsCompiled();
            RegisteredToolRunsCollectAllPickupsWithAllCheckpointsPassed();
            RegisteredToolRunsRestartRecoveryWithExpectedObservations();
            RegisteredToolReportsMissingScenarioFileAsMcpError();
            HeadlessCliMapsScenarioOutcomeToExitCodes();
        }

        private static void CliArgsParsesRunScenarioOption()
        {
            CliArgs parsed = CliArgs.Parse(
                new[] { "-batchmode", "-runScenario", "samples/marble-rush/Scenarios/collect_all.json" });
            Check.Equal(
                "cli-args: runScenario parsed",
                "samples/marble-rush/Scenarios/collect_all.json",
                parsed.RunScenario ?? string.Empty);
            Check.Equal("cli-args: zero unrecognized arguments with -runScenario", 0, parsed.UnrecognizedArguments.Count);
            Check.Equal("cli-args: zero missing values with -runScenario", 0, parsed.MissingValueArguments.Count);

            CliArgs missing = CliArgs.Parse(new[] { "-runScenario" });
            Check.Equal("cli-args: -runScenario without value reports a missing value", 1, missing.MissingValueArguments.Count);
            Check.Equal("cli-args: missing option name is -runScenario", "-runScenario", missing.MissingValueArguments[0]);
        }

        // The scenarios drive the real marble-rush project, so the editor can
        // only load its scripts when the project is compiled. The compiled
        // output is rebuilt from the current sources because a compile state
        // left behind by another lane can name an assembly as current while
        // the file on disk still comes from older sources.
        private static void EnsureSampleProjectScriptsCompiled()
        {
            string projectRoot = ResolveRepoDirectory("samples/marble-rush");
            ScriptCompilationResult result = ScriptCompilation.CompileProject(
                projectRoot,
                new ScriptCompilationOptions());
            Check.That("scenario-run: the sample project scripts compile", result.Succeeded);
        }

        private static void RegisteredToolRunsCollectAllPickupsWithAllCheckpointsPassed()
        {
            using McpHost host = CreateScenarioHost(out string sessionId);
            JsonObject envelope = RunScenario(host, sessionId, CollectScenario);

            Check.That("scenario-run: collect envelope is untrusted", UntrustedContent.IsEnvelope(envelope));
            JsonObject content = (JsonObject)envelope["content"]!;
            Check.Equal("scenario-run: collect ok is true", true, content["ok"]?.GetValue<bool>() ?? false);
            Check.Equal("scenario-run: collect status is passed", "passed", content["status"]?.GetValue<string>() ?? string.Empty);
            Check.Equal("scenario-run: collect setup ok", true, content["setup"]?["ok"]?.GetValue<bool>() ?? false);

            JsonArray checkpoints = (JsonArray)content["checkpoints"]!;
            Check.Equal("scenario-run: collect reports 5 checkpoints", 5, checkpoints.Count);
            int passed = 0;
            foreach (JsonNode? node in checkpoints)
            {
                JsonObject checkpoint = (JsonObject)node!;
                if (string.Equals(
                        checkpoint["status"]?.GetValue<string>(),
                        "passed",
                        StringComparison.Ordinal))
                {
                    passed++;
                }
            }

            Check.Equal("scenario-run: collect all 5 checkpoints passed", 5, passed);
            JsonObject totalFrames = FindCheckpoint(content, "frame_count_sanity");
            Check.Equal(
                "scenario-run: collect total frames observed",
                "443",
                totalFrames["observed"]?.GetValue<string>() ?? string.Empty);
            Check.That("scenario-run: collect result carries an operation handle", envelope["operationId"] != null);
            string operationId = envelope["operationId"]!.GetValue<string>();
            Check.Equal(
                "scenario-run: collect operation is completed",
                "completed",
                host.GetOperation(operationId).State.StatusString);
        }

        private static void RegisteredToolRunsRestartRecoveryWithExpectedObservations()
        {
            using McpHost host = CreateScenarioHost(out string sessionId);
            JsonObject envelope = RunScenario(host, sessionId, RestartScenario);

            JsonObject content = (JsonObject)envelope["content"]!;
            Check.Equal("scenario-run: restart ok is true", true, content["ok"]?.GetValue<bool>() ?? false);
            Check.Equal("scenario-run: restart status is passed", "passed", content["status"]?.GetValue<string>() ?? string.Empty);

            JsonObject restarted = FindCheckpoint(content, "player_back_at_start");
            double[] observed = ParseVector(restarted["observed"]?.GetValue<string>() ?? string.Empty);
            Check.That("scenario-run: restart lands on start x", Math.Abs(observed[0]) < 0.001);
            Check.That("scenario-run: restart lands on start y", Math.Abs(observed[1] - 0.5) < 0.001);
            Check.That("scenario-run: restart lands on start z", Math.Abs(observed[2]) < 0.001);

            Check.Equal("scenario-run: restart total frames observed", 170, TotalFrames(content));

            JsonObject retryLog = FindCheckpoint(content, "retry_contract_log");
            Check.Equal(
                "scenario-run: restart retry log checkpoint passed",
                "passed",
                retryLog["status"]?.GetValue<string>() ?? string.Empty);
        }

        private static void RegisteredToolReportsMissingScenarioFileAsMcpError()
        {
            using McpHost host = CreateScenarioHost(out string sessionId);
            bool reported = false;
            try
            {
                host.CallTool(
                    sessionId,
                    "scenario.run",
                    new JsonObject { ["scenarioPath"] = "samples/marble-rush/Scenarios/no_such.scenario.json" });
            }
            catch (McpError.InvalidRequest)
            {
                reported = true;
            }

            Check.That("scenario-run: missing scenario file is a reported tool error", reported);
        }

        private static void HeadlessCliMapsScenarioOutcomeToExitCodes()
        {
            string tempProject = Path.Combine(
                Path.GetTempPath(),
                "unitas-scenario-cli-test-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempProject);
            try
            {
                using McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
                ScenarioTools.Register(host);

                string passingScenario = WritePassingScenario(tempProject);
                CliResult success = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-nographics",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-runScenario",
                        passingScenario,
                    });
                Check.Equal("scenario-cli: passing scenario exits 0", CliExitCode.Success, success.ExitCode);
                Check.Equal("scenario-cli: exit status name is Success", "Success", success.ExitStatusName);
                Check.Equal("scenario-cli: project.open and scenario.run operations recorded", 2, success.Operations.Count);
                Check.Equal(
                    "scenario-cli: scenario.run operation succeeded",
                    "succeeded",
                    success.Operations[1].Outcome);

                string failingScenario = WriteFailingScenario(tempProject);
                CliResult failure = HeadlessCli.Execute(
                    host,
                    new[]
                    {
                        "-batchmode",
                        "-quit",
                        "-projectPath",
                        tempProject,
                        "-runScenario",
                        failingScenario,
                    });
                Check.Equal(
                    "scenario-cli: failing scenario exits TestOrScenarioFailed (6)",
                    CliExitCode.TestOrScenarioFailed,
                    failure.ExitCode);
                Check.Equal(
                    "scenario-cli: failing exit status name is TestOrScenarioFailed",
                    "TestOrScenarioFailed",
                    failure.ExitStatusName);
            }
            finally
            {
                Directory.Delete(tempProject, recursive: true);
            }
        }

        private static McpHost CreateScenarioHost(out string sessionId)
        {
            McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            ScenarioTools.Register(host);
            string created = host.CreateSession(new SessionIdentity("agent-a", "cli", "person-1"));
            host.GrantScope(created, Scope.ExecuteProjectCode, GrantAuthority.Person);
            sessionId = created;
            return host;
        }

        private static JsonObject RunScenario(McpHost host, string sessionId, string scenarioPath)
        {
            return (JsonObject)host.CallTool(
                sessionId,
                "scenario.run",
                new JsonObject { ["scenarioPath"] = scenarioPath });
        }

        private static double[] ParseVector(string formatted)
        {
            string trimmed = formatted.Trim('(', ')');
            string[] parts = trimmed.Split(',');
            double[] values = new double[parts.Length];
            for (int index = 0; index < parts.Length; index++)
            {
                values[index] = double.Parse(parts[index].Trim(), CultureInfo.InvariantCulture);
            }

            return values;
        }

        private static JsonObject FindCheckpoint(JsonObject content, string id)
        {
            JsonArray checkpoints = (JsonArray)content["checkpoints"]!;
            foreach (JsonNode? node in checkpoints)
            {
                JsonObject checkpoint = (JsonObject)node!;
                if (string.Equals(checkpoint["id"]?.GetValue<string>(), id, StringComparison.Ordinal))
                {
                    return checkpoint;
                }
            }

            throw new InvalidOperationException("checkpoint not found: " + id);
        }

        private static int TotalFrames(JsonObject content)
        {
            JsonArray steps = (JsonArray)content["steps"]!;
            JsonObject last = (JsonObject)steps[steps.Count - 1]!;
            return last["frame_end"]?.GetValue<int>() ?? 0;
        }

        private static string WritePassingScenario(string directory)
        {
            JsonObject scenario = new JsonObject
            {
                ["name"] = "passing-checkpoint",
                ["scene"] = ResolveRepoPath(ScenePath),
                ["steps"] = new JsonArray(
                    new JsonObject
                    {
                        ["tick"] = new JsonObject
                        {
                            ["frames"] = 1,
                            ["delta_seconds"] = 0.02,
                        },
                    }),
                ["checkpoints"] = new JsonArray(
                    new JsonObject
                    {
                        ["id"] = "single-frame-count",
                        ["after_step"] = 1,
                        ["kind"] = "frame_count",
                        ["expected"] = 1,
                    }),
            };
            string path = Path.Combine(directory, "passing.scenario.json");
            File.WriteAllText(path, scenario.ToJsonString());
            return path;
        }

        private static string WriteFailingScenario(string directory)
        {
            JsonObject scenario = new JsonObject
            {
                ["name"] = "failing-checkpoint",
                ["scene"] = ResolveRepoPath(ScenePath),
                ["steps"] = new JsonArray(
                    new JsonObject
                    {
                        ["tick"] = new JsonObject
                        {
                            ["frames"] = 1,
                            ["delta_seconds"] = 0.02,
                        },
                    }),
                ["checkpoints"] = new JsonArray(
                    new JsonObject
                    {
                        ["id"] = "impossible-frame-count",
                        ["after_step"] = 1,
                        ["kind"] = "frame_count",
                        ["expected"] = 9999,
                    }),
            };
            string path = Path.Combine(directory, "failing.scenario.json");
            File.WriteAllText(path, scenario.ToJsonString());
            return path;
        }

        private static string ResolveRepoPath(string relativePath)
        {
            return ResolveFromWorkingDirectory(relativePath, File.Exists, "path");
        }

        private static string ResolveRepoDirectory(string relativePath)
        {
            return ResolveFromWorkingDirectory(relativePath, Directory.Exists, "directory");
        }

        private static string ResolveFromWorkingDirectory(
            string relativePath,
            Func<string, bool> exists,
            string kind)
        {
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                string candidate = Path.Combine(directory.FullName, relativePath);
                if (exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            throw new FileNotFoundException(kind + " not found from current directory: " + relativePath);
        }
    }
}
