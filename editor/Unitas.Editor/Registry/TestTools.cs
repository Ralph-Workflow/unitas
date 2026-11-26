using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Scripting;
using Unitas.Testing;

namespace Unitas.Editor.Mcp
{
    public static class TestTools
    {
        public const string RunOrigin = "unitas://tests/run";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "tests.run",
                    "Run the project's tests and return the pass and fail summary",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["testPlatform"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["testResults"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleRun(host, arguments));
        }

        private static ToolOutput HandleRun(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "tests.run");
            TestRunOptions options = new TestRunOptions
            {
                Mode = ParseMode(ProjectAccess.OptionalString(arguments, "testPlatform")),
            };
            string? resultsPath = ProjectAccess.OptionalString(arguments, "testResults");
            if (resultsPath != null)
            {
                options.ResultsXmlPath = Path.GetFullPath(resultsPath);
            }

            TestRunResult result;
            try
            {
                result = TestRunner.RunProject(root, options);
            }
            catch (Exception error) when (error is IOException
                || error is InvalidOperationException
                || error is ArgumentException)
            {
                throw new McpError.InvalidRequest("tests.run failed: " + error.Message);
            }

            JsonArray failures = new JsonArray();
            foreach (TestCaseResult testCase in result.Results)
            {
                if (testCase.Outcome != TestOutcome.Failed)
                {
                    continue;
                }

                failures.Add(new JsonObject
                {
                    ["name"] = testCase.Descriptor.FullName,
                    ["outcome"] = testCase.Outcome.ToString(),
                    ["message"] = testCase.Message ?? string.Empty,
                });
            }

            JsonArray diagnostics = new JsonArray();
            foreach (ScriptDiagnostic diagnostic in result.Diagnostics)
            {
                diagnostics.Add(new JsonObject
                {
                    ["identifier"] = diagnostic.Id,
                    ["message"] = diagnostic.Message,
                    ["file"] = diagnostic.File,
                    ["line"] = diagnostic.Line,
                    ["column"] = diagnostic.Column,
                });
            }

            JsonObject payload = new JsonObject
            {
                ["ok"] = result.Succeeded,
                ["compileSucceeded"] = result.CompileSucceeded,
                ["selected"] = result.SelectedTestCaseCount,
                ["completed"] = result.CompletedCount,
                ["passed"] = result.PassedCount,
                ["failed"] = result.FailedCount,
                ["skipped"] = result.SkippedCount,
                ["notRun"] = result.NotRunCount,
                ["durationSeconds"] = result.DurationSeconds,
                ["resultsXmlPath"] = result.ResultsXmlPath,
                ["failures"] = failures,
                ["diagnostics"] = diagnostics,
            };
            return ToolOutput.Untrusted(RunOrigin, payload);
        }

        private static TestModeFilter ParseMode(string? platform)
        {
            if (string.Equals(platform, "EditMode", StringComparison.OrdinalIgnoreCase))
            {
                return TestModeFilter.EditMode;
            }

            if (string.Equals(platform, "PlayMode", StringComparison.OrdinalIgnoreCase))
            {
                return TestModeFilter.PlayMode;
            }

            return TestModeFilter.All;
        }
    }
}
