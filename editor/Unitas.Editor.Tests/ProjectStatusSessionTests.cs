using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Pipelines;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using ModelContextProtocol;
using ModelContextProtocol.Client;
using ModelContextProtocol.Protocol;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    // U4: real project.status state plus the scripted stdio MCP session proof
    // against samples/roll-a-ball (initialize -> project.status -> scenario.run).
    public static class ProjectStatusSessionTests
    {
        private const string RollABallRoot = "samples/roll-a-ball";
        private const string CollectScenario = "samples/roll-a-ball/Scenarios/collect_all_pickups.scenario.json";

        public static void Run()
        {
            StatusToolReportsRealScenesObjectsAndTrust();
            ScriptedStdioSessionRunsStatusAndScenario().GetAwaiter().GetResult();
        }

        private static void StatusToolReportsRealScenesObjectsAndTrust()
        {
            string projectRoot = ResolveRepoPath(RollABallRoot);
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            Check.Equal("status: one default tool", 1, tools.Count);
            Check.That(
                "status: project.status keeps the ReadProject scope gate",
                tools[0].RequiredScope == Scope.ReadProject);

            string session = host.CreateSession(new SessionIdentity("agent-status", "stdio", "person-1"));
            JsonNode envelope = host.CallTool(
                session,
                "project.status",
                new JsonObject { ["projectPath"] = projectRoot });

            Check.That("status: envelope is untrusted", UntrustedContent.IsEnvelope(envelope));
            Check.Equal(
                "status: origin",
                "unitas://project/status",
                envelope["origin"]?.GetValue<string>() ?? string.Empty);
            JsonObject content = (JsonObject)envelope["content"]!;
            Check.Equal(
                "status: state is ready",
                "ready",
                content["state"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "status: project root is the opened project",
                Path.GetFullPath(projectRoot),
                content["projectPath"]?.GetValue<string>() ?? string.Empty);

            JsonArray scenes = (JsonArray)content["scenes"]!;
            Check.Equal("status: roll-a-ball has two scenes", 2, scenes.Count);
            JsonObject rollABall = FindScene(scenes, "RollABall.unity");
            JsonObject physicsLab = FindScene(scenes, "PhysicsLab.unity");
            Check.Equal(
                "status: RollABall scene path",
                "Assets/Scenes/RollABall.unity",
                rollABall["path"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "status: RollABall declares 19 GameObjects",
                19,
                rollABall["objects"]?.GetValue<int>() ?? 0);
            Check.Equal(
                "status: PhysicsLab declares 13 GameObjects",
                13,
                physicsLab["objects"]?.GetValue<int>() ?? 0);
            Check.Equal(
                "status: total declared objects",
                32,
                content["objectsTotal"]?.GetValue<int>() ?? 0);

            Check.That(
                "status: trust state present",
                content["trust"] is JsonObject);
            Check.Equal(
                "status: session without the execute grant is untrusted for project code",
                false,
                content["trust"]?["mayRunProjectCode"]?.GetValue<bool>() ?? true);

            Check.Equal("status: one audit record", 1, host.Audit.Count);
            Check.Equal(
                "status: audit records project.status",
                "project.status",
                host.Audit.Records[0].Target);
        }

        private static async Task ScriptedStdioSessionRunsStatusAndScenario()
        {
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            string stateDir = Path.Combine(
                Path.GetTempPath(),
                "unitas-status-session-" + Guid.NewGuid().ToString("N"));
            Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateDir);
            try
            {
                await ScriptedStdioSessionRunsStatusAndScenarioCore();
            }
            finally
            {
                Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, previousStateDir);
                try
                {
                    if (Directory.Exists(stateDir))
                    {
                        Directory.Delete(stateDir, true);
                    }
                }
                catch (IOException)
                {
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }

        private static async Task ScriptedStdioSessionRunsStatusAndScenarioCore()
        {
            using McpHost host = BuiltinDefaults.CreateStartupHost();
            StdioTransportHost transportHost = new StdioTransportHost(host);
            Pipe clientToServer = new Pipe();
            Pipe serverToClient = new Pipe();
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(60));

            Task serverTask = transportHost.RunAsync(
                clientToServer.Reader.AsStream(),
                serverToClient.Writer.AsStream(),
                cts.Token);
            StreamClientTransport clientTransport = new StreamClientTransport(
                clientToServer.Writer.AsStream(),
                serverToClient.Reader.AsStream(),
                null);
            McpClient client = await McpClient.CreateAsync(
                clientTransport,
                new McpClientOptions
                {
                    ClientInfo = new Implementation
                    {
                        Name = "status-session-client",
                        Version = "1.0.0",
                    },
                },
                null,
                cts.Token).ConfigureAwait(false);

            try
            {
                string projectRoot = ResolveRepoPath(RollABallRoot);
                string scenarioPath = ResolveRepoPath(CollectScenario);

                CallToolResult statusResult = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = "project.status",
                        Arguments = new Dictionary<string, JsonElement>
                        {
                            ["projectPath"] = JsonArg(JsonSerializer.Serialize(projectRoot)),
                        },
                    },
                    cts.Token).ConfigureAwait(false);
                JsonNode? statusEnvelope = ParseStructuredContent(statusResult);
                Check.That(
                    "session: project.status returns an untrusted envelope",
                    UntrustedContent.IsEnvelope(statusEnvelope));
                JsonArray? scenes = statusEnvelope?["content"]?["scenes"] as JsonArray;
                Check.That(
                    "session: project.status reports real scene data",
                    scenes != null && scenes.Count == 2);
                bool sawObjects = false;
                if (scenes != null)
                {
                    foreach (JsonNode? node in scenes)
                    {
                        if ((node?["objects"]?.GetValue<int>() ?? 0) > 0)
                        {
                            sawObjects = true;
                        }
                    }
                }

                Check.That("session: scenes carry positive object counts", sawObjects);

                string sessionId = transportHost.ActiveSessionId ?? string.Empty;
                Check.That("session: stdio session is active", !string.IsNullOrEmpty(sessionId));
                host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

                CallToolResult scenarioResult = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = "scenario.run",
                        Arguments = new Dictionary<string, JsonElement>
                        {
                            ["scenarioPath"] = JsonArg(JsonSerializer.Serialize(scenarioPath)),
                        },
                    },
                    cts.Token).ConfigureAwait(false);
                JsonNode? scenarioEnvelope = ParseStructuredContent(scenarioResult);
                Check.That(
                    "session: scenario.run returns an untrusted envelope",
                    UntrustedContent.IsEnvelope(scenarioEnvelope));
                Check.Equal(
                    "session: scenario.run origin",
                    "unitas://scenario/run",
                    scenarioEnvelope?["origin"]?.GetValue<string>() ?? string.Empty);
                JsonObject scenarioContent = (JsonObject)scenarioEnvelope!["content"]!;
                Check.Equal(
                    "session: collect_all_pickups passes",
                    true,
                    scenarioContent["ok"]?.GetValue<bool>() ?? false);
                Check.Equal(
                    "session: collect_all_pickups status is passed",
                    "passed",
                    scenarioContent["status"]?.GetValue<string>() ?? string.Empty);
                string? operationId = scenarioEnvelope["operationId"]?.GetValue<string>();
                Check.That("session: scenario.run carries an operation handle", operationId != null);
                if (operationId != null)
                {
                    Check.Equal(
                        "session: scenario operation completed",
                        "completed",
                        host.GetOperation(operationId).State.StatusString);
                }

                List<string> targets = new List<string>();
                foreach (AuditRecord record in host.Audit.Records)
                {
                    targets.Add(record.Target);
                }

                Check.Equal(
                    "session: audit recorded grant, status, scenario operation and scenario call",
                    4,
                    host.Audit.Count);
                Check.That(
                    "session: audit records project.status",
                    targets.Contains("project.status"));
                Check.That(
                    "session: audit records the scenario.run tool call",
                    targets.Contains("scenario.run"));
            }
            finally
            {
                await client.DisposeAsync().ConfigureAwait(false);
                await cts.CancelAsync().ConfigureAwait(false);
                await clientToServer.Writer.CompleteAsync().ConfigureAwait(false);
                await serverToClient.Writer.CompleteAsync().ConfigureAwait(false);
                try
                {
                    await serverTask.ConfigureAwait(false);
                }
                catch (OperationCanceledException)
                {
                }
            }
        }

        private static JsonObject FindScene(JsonArray scenes, string fileName)
        {
            foreach (JsonNode? node in scenes)
            {
                JsonObject scene = (JsonObject)node!;
                if (string.Equals(
                        scene["path"]?.GetValue<string>(),
                        "Assets/Scenes/" + fileName,
                        StringComparison.Ordinal))
                {
                    return scene;
                }
            }

            throw new InvalidOperationException("scene not found: " + fileName);
        }

        private static string ResolveRepoPath(string relativePath)
        {
            DirectoryInfo? directory = new DirectoryInfo(Directory.GetCurrentDirectory());
            for (int depth = 0; depth < 12 && directory != null; depth++)
            {
                string candidate = Path.Combine(directory.FullName, relativePath);
                if (File.Exists(candidate) || Directory.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            throw new FileNotFoundException("path not found from current directory: " + relativePath);
        }

        private static JsonElement JsonArg(string rawJson)
        {
            using JsonDocument document = JsonDocument.Parse(rawJson);
            return document.RootElement.Clone();
        }

        private static JsonNode? ParseStructuredContent(CallToolResult result)
        {
            if (result.StructuredContent.HasValue)
            {
                return JsonNode.Parse(result.StructuredContent.Value.GetRawText());
            }

            if (result.Content.Count > 0 && result.Content[0] is TextContentBlock textBlock)
            {
                return JsonNode.Parse(textBlock.Text);
            }

            return null;
        }
    }
}
