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
    public static class StdioTransportTests
    {
        public static void Run()
        {
            RequestWindowIsNotTheSession();
            StdioInitializeToolsAndResourcesRoundTrip().GetAwaiter().GetResult();
            StdioErrorMappingPreservesJsonRpcCodes().GetAwaiter().GetResult();
            StdioProductionToolSetAndCaptureEndToEnd().GetAwaiter().GetResult();
        }

        // A request window bounds one request, never the session the later
        // requests travel over. Starting the live play worker loads the whole
        // sample bundle, so the request window has to cover a worker start on a
        // machine that also runs the other verification gates.
        private const int RequestWindowSeconds = 60;

        private static CancellationToken RequestWindow(CancellationTokenSource session)
        {
            // The window bounds a hung request, not a busy machine: the shared runner
            // saturates the host, so a local MCP round trip can outlast a short window.
            // Every request carries its own window, so a request that outruns the
            // window cannot cancel the requests that follow it.
            CancellationTokenSource window =
                CancellationTokenSource.CreateLinkedTokenSource(session.Token);
            window.CancelAfter(TimeSpan.FromSeconds(RequestWindowSeconds));
            return window.Token;
        }

        // A request window is its own source: when it expires it must not end the
        // session that the requests after it travel over, and the session must
        // still end the windows of the requests that are in flight.
        private static void RequestWindowIsNotTheSession()
        {
            using CancellationTokenSource session = new CancellationTokenSource();
            CancellationToken window = RequestWindow(session);
            Check.That(
                "stdio: a request window does not share the session token",
                window != session.Token);
            session.Cancel();
            Check.That(
                "stdio: a request window follows the session token",
                window.IsCancellationRequested);
        }

        private static async Task StdioInitializeToolsAndResourcesRoundTrip()
        {
            using McpHost host = BuildTestHost();
            StdioTransportHost transportHost = new StdioTransportHost(host);

            Pipe clientToServer = new Pipe();
            Pipe serverToClient = new Pipe();
            // The transport lives until the test tears it down, so one slow request
            // cannot end the session the requests after it travel over.
            using CancellationTokenSource cts = new CancellationTokenSource();

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
                        Name = "test-client",
                        Version = "1.0.0",
                    },
                },
                null,
                cts.Token).ConfigureAwait(false);
            try
            {
                Check.Equal(
                    "stdio: initialize returns server name unitas-mcp",
                    "unitas-mcp",
                    client.ServerInfo?.Name ?? string.Empty);

                IList<McpClientTool> tools = await client.ListToolsAsync(
                    (RequestOptions?)null,
                    RequestWindow(cts)).ConfigureAwait(false);
                Check.That(
                    "stdio: session is created and active",
                    !string.IsNullOrEmpty(transportHost.ActiveSessionId));
                VerifyStdioRegisteredToolsList(tools);

                CallToolResult statusResult = await client.CallToolAsync(
                    new CallToolRequestParams { Name = "project.status" },
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? statusStructured = ParseStructuredContent(statusResult);
                Check.That(
                    "stdio: project.status call returns untrusted envelope",
                    UntrustedContent.IsEnvelope(statusStructured));

                CallToolResult inspectResult = await client.CallToolAsync(
                    new CallToolRequestParams { Name = "scene.inspect" },
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? inspectStructured = ParseStructuredContent(inspectResult);
                Check.That(
                    "stdio: scene.inspect call returns untrusted envelope",
                    UntrustedContent.IsEnvelope(inspectStructured));
                Check.Equal(
                    "stdio: scene.inspect origin",
                    "scene:Sample",
                    inspectStructured?["origin"]?.GetValue<string>() ?? string.Empty);

                McpProtocolException? missingScopeError = await CatchProtocolAsync(() =>
                    client.CallToolAsync(
                        new CallToolRequestParams
                        {
                            Name = "scene.edit",
                            Arguments = new Dictionary<string, JsonElement>
                            {
                                ["name"] = JsonDocument.Parse("\"Ball\"").RootElement.Clone(),
                            },
                        },
                        RequestWindow(cts)).AsTask()).ConfigureAwait(false);
                Check.That("stdio: scene.edit before grant throws McpProtocolException", missingScopeError != null);
                Check.Equal(
                    "stdio: missing scope error code is -32001",
                    StdioTransportHost.MissingScopeErrorCode,
                    missingScopeError != null ? (int)missingScopeError.ErrorCode : 0);
                Check.That(
                    "stdio: missing scope error names edit-content",
                    missingScopeError != null && missingScopeError.Message.Contains("edit-content", StringComparison.Ordinal));

                IList<McpClientResource> resources = await client.ListResourcesAsync(
                    (RequestOptions?)null,
                    RequestWindow(cts)).ConfigureAwait(false);
                Check.Equal("stdio: resources/list includes builtin and registered resources", 4, resources.Count);
                Check.Equal(
                    "stdio: resources/list resource 0 is unitas://knowledge/docs",
                    "unitas://knowledge/docs",
                    resources[0].Uri);
                Check.Equal(
                    "stdio: resources/list resource 1 is unitas://knowledge/grants",
                    "unitas://knowledge/grants",
                    resources[1].Uri);
                Check.Equal(
                    "stdio: resources/list resource 2 is unitas://knowledge/scopes",
                    "unitas://knowledge/scopes",
                    resources[2].Uri);
                Check.Equal(
                    "stdio: resources/list resource 3 is unitas://project/manifest",
                    "unitas://project/manifest",
                    resources[3].Uri);

                ReadResourceResult scopesRead = await client.ReadResourceAsync(
                    new Uri("unitas://knowledge/scopes"),
                    (RequestOptions?)null,
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? scopesEnvelope = ParseResourceEnvelope(scopesRead);
                Check.That(
                    "stdio: reading unitas://knowledge/scopes returns untrusted envelope",
                    UntrustedContent.IsEnvelope(scopesEnvelope));

                ReadResourceResult manifestRead = await client.ReadResourceAsync(
                    new Uri("unitas://project/manifest"),
                    (RequestOptions?)null,
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? manifestEnvelope = ParseResourceEnvelope(manifestRead);
                Check.That(
                    "stdio: reading unitas://project/manifest returns untrusted envelope",
                    UntrustedContent.IsEnvelope(manifestEnvelope));

                McpProtocolException? selfWideningError = await CatchProtocolAsync(() =>
                    client.CallToolAsync(
                        new CallToolRequestParams
                        {
                            Name = "permissions/grant",
                            Arguments = new Dictionary<string, JsonElement>
                            {
                                ["scope"] = JsonDocument.Parse("\"edit-content\"").RootElement.Clone(),
                            },
                        },
                        RequestWindow(cts)).AsTask()).ConfigureAwait(false);
                Check.That("stdio: self-widening tool call throws McpProtocolException", selfWideningError != null);
                Check.Equal(
                    "stdio: self-widening error code is -32002",
                    StdioTransportHost.SelfWideningErrorCode,
                    selfWideningError != null ? (int)selfWideningError.ErrorCode : 0);

                string sessionId = transportHost.ActiveSessionId ?? string.Empty;
                host.GrantScope(sessionId, Scope.EditContent, GrantAuthority.Person);

                CallToolResult editResult = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = "scene.edit",
                        Arguments = new Dictionary<string, JsonElement>
                        {
                            ["name"] = JsonDocument.Parse("\"Ball\"").RootElement.Clone(),
                        },
                    },
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? editStructured = ParseStructuredContent(editResult);
                Check.That(
                    "stdio: scene.edit after person grant returns untrusted envelope",
                    UntrustedContent.IsEnvelope(editStructured));
                Check.Equal(
                    "stdio: scene.edit echoes applied argument",
                    "Ball",
                    editStructured?["content"]?["applied"]?["name"]?.GetValue<string>() ?? string.Empty);

                string opId = host.StartOperation(sessionId, "scene.bake", Scope.EditContent);
                host.CompleteOperation(opId, "baker", new JsonObject { ["status"] = "baked" });
                OperationHandle opHandle = host.GetOperation(opId);
                Check.Equal("stdio: operation tracks completion", "completed", opHandle.State.StatusString);

                Check.Equal("stdio: audit log records all operations", 9, host.Audit.Count);
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

        private static async Task StdioErrorMappingPreservesJsonRpcCodes()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            StdioTransportHost transportHost = new StdioTransportHost(host);

            Pipe clientToServer = new Pipe();
            Pipe serverToClient = new Pipe();
            // The transport lives until the test tears it down, so one slow request
            // cannot end the session the requests after it travel over.
            using CancellationTokenSource cts = new CancellationTokenSource();

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
                        Name = "error-test-client",
                        Version = "1.0.0",
                    },
                },
                null,
                cts.Token).ConfigureAwait(false);
            try
            {
                McpProtocolException? unknownTool = await CatchProtocolAsync(() =>
                    client.CallToolAsync(
                        new CallToolRequestParams { Name = "no.such.tool" },
                        RequestWindow(cts)).AsTask()).ConfigureAwait(false);
                Check.That("stdio: unknown tool maps to McpProtocolException", unknownTool != null);
                Check.Equal(
                    "stdio: unknown tool maps to -32000",
                    StdioTransportHost.ServerErrorCode,
                    unknownTool != null ? (int)unknownTool.ErrorCode : 0);

                McpProtocolException? unknownResource = await CatchProtocolAsync(() =>
                    client.ReadResourceAsync(
                        new Uri("unitas://unknown/resource"),
                        (RequestOptions?)null,
                        RequestWindow(cts)).AsTask()).ConfigureAwait(false);
                Check.That("stdio: unknown resource maps to McpProtocolException", unknownResource != null);
                Check.Equal(
                    "stdio: unknown resource maps to -32000",
                    StdioTransportHost.ServerErrorCode,
                    unknownResource != null ? (int)unknownResource.ErrorCode : 0);
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

        private static async Task StdioProductionToolSetAndCaptureEndToEnd()
        {
            string? previousStateDir = Environment.GetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable);
            string stateDir = Path.Combine(
                Path.GetTempPath(),
                "unitas-editor-stdio-" + Guid.NewGuid().ToString("N"));
            Environment.SetEnvironmentVariable(AuditStore.StateDirectoryEnvironmentVariable, stateDir);
            try
            {
                await StdioProductionToolSetAndCaptureEndToEndCore();
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

        private static async Task StdioProductionToolSetAndCaptureEndToEndCore()
        {
            using McpHost host = BuiltinDefaults.CreateStartupHost();
            StdioTransportHost transportHost = new StdioTransportHost(host);
            Pipe clientToServer = new Pipe();
            Pipe serverToClient = new Pipe();
            // The transport lives until the test tears it down, so one slow request
            // cannot end the session the requests after it travel over.
            using CancellationTokenSource cts = new CancellationTokenSource();
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
                        Name = "production-stdio-client",
                        Version = "1.0.0",
                    },
                },
                null,
                cts.Token).ConfigureAwait(false);
            string playSessionId = string.Empty;
            try
            {
                IList<McpClientTool> tools = await client.ListToolsAsync(
                    (RequestOptions?)null,
                    RequestWindow(cts)).ConfigureAwait(false);
                Check.That(
                    "stdio_production: tools/list exposes play.enter",
                    HasTool(tools, "play.enter"));
                Check.That(
                    "stdio_production: tools/list exposes play.step",
                    HasTool(tools, "play.step"));
                Check.That(
                    "stdio_production: tools/list exposes capture",
                    HasTool(tools, "capture"));
                Check.That(
                    "stdio_production: tools/list exposes audit.export",
                    HasTool(tools, "audit.export"));
                Check.That(
                    "stdio_production: tools/list exposes engine.abi_status",
                    HasTool(tools, "engine.abi_status"));
                VerifyProductionToolsCarryReversibilityMeta(tools);

                string sessionId = transportHost.ActiveSessionId ?? string.Empty;
                Check.That(
                    "stdio_production: stdio session is created and active",
                    !string.IsNullOrEmpty(sessionId));
                host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

                CallToolResult enterResult = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = "play.enter",
                        Arguments = new Dictionary<string, JsonElement>(),
                    },
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? enterStructured = ParseStructuredContent(enterResult);
                Check.That(
                    "stdio_production: play.enter returns an untrusted envelope",
                    UntrustedContent.IsEnvelope(enterStructured));
                playSessionId = enterStructured?["content"]?["playSessionId"]?.GetValue<string>() ?? string.Empty;
                Check.That(
                    "stdio_production: play.enter starts the live worker and returns a play session id",
                    !string.IsNullOrEmpty(playSessionId));

                CallToolResult stepResult = await client.CallToolAsync(
                    new CallToolRequestParams
                    {
                        Name = "play.step",
                        Arguments = new Dictionary<string, JsonElement>
                        {
                            ["playSessionId"] = JsonArg(JsonSerializer.Serialize(playSessionId)),
                            ["frames"] = JsonArg("4"),
                        },
                    },
                    RequestWindow(cts)).ConfigureAwait(false);
                JsonNode? stepStructured = ParseStructuredContent(stepResult);
                Check.That(
                    "stdio_production: play.step returns an untrusted envelope",
                    UntrustedContent.IsEnvelope(stepStructured));
                Check.Equal(
                    "stdio_production: play.step advanced the live worker by 4 frames",
                    4,
                    ReadInt(stepStructured?["content"]?["frames"], -1));

                CallToolResult? captureResult = null;
                try
                {
                    captureResult = await client.CallToolAsync(
                        new CallToolRequestParams
                        {
                            Name = "capture",
                            Arguments = new Dictionary<string, JsonElement>
                            {
                                ["playSessionId"] = JsonArg(JsonSerializer.Serialize(playSessionId)),
                                ["width"] = JsonArg("64"),
                                ["height"] = JsonArg("36"),
                                ["id_map"] = JsonArg("true"),
                            },
                        },
                        RequestWindow(cts)).ConfigureAwait(false);
                }
                catch (McpProtocolException refusal)
                {
                    Check.That(
                        "stdio_production: a player without the device feature refuses the capture and names the requirement",
                        refusal.Message.Contains("capture requires the device feature", StringComparison.Ordinal));
                }

                if (captureResult is not null)
                {
                    JsonNode? captureStructured = ParseStructuredContent(captureResult);
                    Check.That(
                        "stdio_production: capture returns an untrusted envelope",
                        UntrustedContent.IsEnvelope(captureStructured));
                    JsonNode? captureContent = captureStructured?["content"];
                    string base64Image = captureContent?["image"]?.GetValue<string>() ?? string.Empty;
                    int imageBytes = 0;
                    if (!string.IsNullOrEmpty(base64Image))
                    {
                        imageBytes = Convert.FromBase64String(base64Image).Length;
                    }

                    Check.Equal(
                        "stdio_production: capture base64 RGBA8 image decodes to width * height * 4 bytes",
                        64 * 36 * 4,
                        imageBytes);
                    JsonNode? idMapNode = captureContent?["id_map"];
                    JsonArray? idMap = idMapNode as JsonArray;
                    Check.Equal(
                        "stdio_production: capture id_map has width * height entries",
                        64 * 36,
                        idMap?.Count ?? 0);
                    JsonNode? objectsNode = captureContent?["objects"];
                    JsonArray? objects = objectsNode as JsonArray;
                    Check.That(
                        "stdio_production: capture objects list is non-empty",
                        objects != null && objects.Count > 0);
                }
            }
            finally
            {
                if (!string.IsNullOrEmpty(playSessionId))
                {
                    try
                    {
                        await client.CallToolAsync(
                            new CallToolRequestParams
                            {
                                Name = "play.exit",
                                Arguments = new Dictionary<string, JsonElement>
                                {
                                    ["playSessionId"] = JsonArg(JsonSerializer.Serialize(playSessionId)),
                                },
                            },
                            RequestWindow(cts)).ConfigureAwait(false);
                    }
                    catch (Exception)
                    {
                    }
                }

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

        private static void VerifyStdioRegisteredToolsList(IList<McpClientTool> tools)
        {
            Check.Equal("stdio: tools/list includes builtin and registered tools", 3, tools.Count);
            Check.Equal("stdio: tools/list tool 0 is project.status", "project.status", tools[0].Name);
            Check.Equal("stdio: tools/list tool 1 is scene.edit", "scene.edit", tools[1].Name);
            Check.Equal("stdio: tools/list tool 2 is scene.inspect", "scene.inspect", tools[2].Name);
            Check.Equal(
                "stdio: project.status meta reversibility is read-only",
                "read-only",
                tools[0].ProtocolTool.Meta?["reversibility"]?.GetValue<string>() ?? string.Empty);
            Check.That(
                "stdio: project.status meta reversible is true",
                tools[0].ProtocolTool.Meta?["reversible"]?.GetValue<bool>() == true);
            Check.Equal(
                "stdio: scene.edit meta reversibility is reversible",
                "reversible",
                tools[1].ProtocolTool.Meta?["reversibility"]?.GetValue<string>() ?? string.Empty);
            Check.That(
                "stdio: scene.edit meta reversible is true",
                tools[1].ProtocolTool.Meta?["reversible"]?.GetValue<bool>() == true);
            Check.Equal(
                "stdio: scene.inspect meta reversibility is read-only",
                "read-only",
                tools[2].ProtocolTool.Meta?["reversibility"]?.GetValue<string>() ?? string.Empty);
        }

        private static bool HasTool(IList<McpClientTool> tools, string name)
        {
            foreach (McpClientTool tool in tools)
            {
                if (string.Equals(tool.Name, name, StringComparison.Ordinal))
                {
                    return true;
                }
            }

            return false;
        }

        private static void VerifyProductionToolsCarryReversibilityMeta(IList<McpClientTool> tools)
        {
            foreach (McpClientTool tool in tools)
            {
                string rev = tool.ProtocolTool.Meta?["reversibility"]?.GetValue<string>() ?? string.Empty;
                bool valid = string.Equals(rev, "read-only", StringComparison.Ordinal)
                    || string.Equals(rev, "reversible", StringComparison.Ordinal)
                    || string.Equals(rev, "irreversible", StringComparison.Ordinal);
                Check.That(
                    "stdio_production: tool " + tool.Name + " meta carries valid reversibility",
                    valid);
                bool expectedReversible = !string.Equals(rev, "irreversible", StringComparison.Ordinal);
                Check.Equal(
                    "stdio_production: tool " + tool.Name + " meta reversible matches reversibility",
                    expectedReversible,
                    tool.ProtocolTool.Meta?["reversible"]?.GetValue<bool>() ?? !expectedReversible);
            }
        }

        private static JsonElement JsonArg(string rawJson)
        {
            using JsonDocument document = JsonDocument.Parse(rawJson);
            return document.RootElement.Clone();
        }

        private static int ReadInt(JsonNode? node, int fallback)
        {
            if (node is JsonValue value && value.TryGetValue(out long parsed))
            {
                return (int)parsed;
            }

            return fallback;
        }

        private static McpHost BuildTestHost()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults(new FixedClock(1_700_000_000_000L));
            host.RegisterTool(
                new ToolDefinition(
                    "scene.inspect",
                    "Inspect active scene",
                    new JsonObject { ["type"] = "object" },
                    Scope.ReadProject),
                _ => ToolOutput.Untrusted(
                    "scene:Sample",
                    new JsonObject
                    {
                        ["rootObjects"] = new JsonArray("MainCamera", "Ball"),
                    }));
            host.RegisterTool(
                new ToolDefinition(
                    "scene.edit",
                    "Edit scene object",
                    new JsonObject { ["type"] = "object" },
                    Scope.EditContent),
                args => ToolOutput.Untrusted(
                    "scene:Sample",
                    new JsonObject
                    {
                        ["applied"] = args.DeepClone(),
                    }));
            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://project/manifest",
                    "Project Manifest",
                    "Manifest metadata",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(
                    "project:manifest",
                    new JsonObject
                    {
                        ["name"] = "RollABall",
                    }));
            return host;
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

        private static JsonNode? ParseResourceEnvelope(ReadResourceResult result)
        {
            if (result.Contents.Count > 0 && result.Contents[0] is TextResourceContents textResource)
            {
                if (!string.IsNullOrEmpty(textResource.Text))
                {
                    return JsonNode.Parse(textResource.Text);
                }
            }

            return null;
        }

        private static async Task<McpProtocolException?> CatchProtocolAsync(Func<Task> action)
        {
            try
            {
                await action().ConfigureAwait(false);
                return null;
            }
            catch (McpProtocolException error)
            {
                return error;
            }
        }
    }
}
