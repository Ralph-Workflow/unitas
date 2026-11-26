using System;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Play
{
    public static class ScenarioToolsTests
    {
        private const string CollectScenario = "samples/roll-a-ball/Scenarios/collect_all_pickups.scenario.json";
        private const string RestartScenario = "samples/roll-a-ball/Scenarios/restart_recovery.scenario.json";
        private const string ScenePath = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";

        public static void Run()
        {
            CliArgsParsesRunScenarioOption();
            RegisteredToolRunsCollectAllPickupsWithAllCheckpointsPassed();
            RegisteredToolRunsRestartRecoveryWithExpectedObservations();
            RegisteredToolReportsMissingScenarioFileAsMcpError();
            HeadlessCliMapsScenarioOutcomeToExitCodes();
        }

        private static void CliArgsParsesRunScenarioOption()
        {
            CliArgs parsed = CliArgs.Parse(
                new[] { "-batchmode", "-runScenario", "samples/roll-a-ball/Scenarios/collect_all_pickups.scenario.json" });
            Check.Equal(
                "cli-args: runScenario parsed",
                "samples/roll-a-ball/Scenarios/collect_all_pickups.scenario.json",
                parsed.RunScenario ?? string.Empty);
            Check.Equal("cli-args: zero unrecognized arguments with -runScenario", 0, parsed.UnrecognizedArguments.Count);
            Check.Equal("cli-args: zero missing values with -runScenario", 0, parsed.MissingValueArguments.Count);

            CliArgs missing = CliArgs.Parse(new[] { "-runScenario" });
            Check.Equal("cli-args: -runScenario without value reports a missing value", 1, missing.MissingValueArguments.Count);
            Check.Equal("cli-args: missing option name is -runScenario", "-runScenario", missing.MissingValueArguments[0]);
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
            Check.Equal("scenario-run: collect reports 9 checkpoints", 9, checkpoints.Count);
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

            Check.Equal("scenario-run: collect all 9 checkpoints passed", 9, passed);
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

            JsonObject restarted = FindCheckpoint(content, "restarted-position");
            double[] observed = ParseVector(restarted["observed"]?.GetValue<string>() ?? string.Empty);
            Check.That("scenario-run: restart lands on start x", Math.Abs(observed[0]) < 0.001);
            Check.That("scenario-run: restart lands on start y", Math.Abs(observed[1] - 0.5) < 0.001);
            Check.That("scenario-run: restart lands on start z", Math.Abs(observed[2]) < 0.001);

            JsonObject totalFrames = FindCheckpoint(content, "total-frames");
            Check.Equal("scenario-run: restart total frames observed", "590", totalFrames["observed"]?.GetValue<string>() ?? string.Empty);

            JsonObject winLog = FindCheckpoint(content, "win-log");
            Check.Equal(
                "scenario-run: restart win log checkpoint passed",
                "passed",
                winLog["status"]?.GetValue<string>() ?? string.Empty);
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
                    new JsonObject { ["scenarioPath"] = "samples/roll-a-ball/Scenarios/no_such.scenario.json" });
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
                        ResolveRepoPath(CollectScenario),
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
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                string candidate = Path.Combine(directory.FullName, relativePath);
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            throw new FileNotFoundException("path not found from current directory: " + relativePath);
        }
    }
}
