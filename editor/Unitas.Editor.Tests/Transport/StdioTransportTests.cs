using System;
using System.Collections.Generic;
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
            StdioInitializeToolsAndResourcesRoundTrip().GetAwaiter().GetResult();
            StdioErrorMappingPreservesJsonRpcCodes().GetAwaiter().GetResult();
        }

        private static async Task StdioInitializeToolsAndResourcesRoundTrip()
        {
            using McpHost host = BuildTestHost();
            StdioTransportHost transportHost = new StdioTransportHost(host);

            Pipe clientToServer = new Pipe();
            Pipe serverToClient = new Pipe();
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(10));

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
                    cts.Token).ConfigureAwait(false);
                Check.That(
                    "stdio: session is created and active",
                    !string.IsNullOrEmpty(transportHost.ActiveSessionId));
                Check.Equal("stdio: tools/list includes builtin and registered tools", 3, tools.Count);
                Check.Equal("stdio: tools/list tool 0 is project.status", "project.status", tools[0].Name);
                Check.Equal("stdio: tools/list tool 1 is scene.edit", "scene.edit", tools[1].Name);
                Check.Equal("stdio: tools/list tool 2 is scene.inspect", "scene.inspect", tools[2].Name);

                CallToolResult statusResult = await client.CallToolAsync(
                    new CallToolRequestParams { Name = "project.status" },
                    cts.Token).ConfigureAwait(false);
                JsonNode? statusStructured = ParseStructuredContent(statusResult);
                Check.That(
                    "stdio: project.status call returns untrusted envelope",
                    UntrustedContent.IsEnvelope(statusStructured));

                CallToolResult inspectResult = await client.CallToolAsync(
                    new CallToolRequestParams { Name = "scene.inspect" },
                    cts.Token).ConfigureAwait(false);
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
                        cts.Token).AsTask()).ConfigureAwait(false);
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
                    cts.Token).ConfigureAwait(false);
                Check.Equal("stdio: resources/list includes builtin and registered resources", 2, resources.Count);
                Check.Equal(
                    "stdio: resources/list resource 0 is unitas://knowledge/scopes",
                    "unitas://knowledge/scopes",
                    resources[0].Uri);
                Check.Equal(
                    "stdio: resources/list resource 1 is unitas://project/manifest",
                    "unitas://project/manifest",
                    resources[1].Uri);

                ReadResourceResult scopesRead = await client.ReadResourceAsync(
                    new Uri("unitas://knowledge/scopes"),
                    (RequestOptions?)null,
                    cts.Token).ConfigureAwait(false);
                JsonNode? scopesEnvelope = ParseResourceEnvelope(scopesRead);
                Check.That(
                    "stdio: reading unitas://knowledge/scopes returns untrusted envelope",
                    UntrustedContent.IsEnvelope(scopesEnvelope));

                ReadResourceResult manifestRead = await client.ReadResourceAsync(
                    new Uri("unitas://project/manifest"),
                    (RequestOptions?)null,
                    cts.Token).ConfigureAwait(false);
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
                        cts.Token).AsTask()).ConfigureAwait(false);
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
                    cts.Token).ConfigureAwait(false);
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
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(10));

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
                        cts.Token).AsTask()).ConfigureAwait(false);
                Check.That("stdio: unknown tool maps to McpProtocolException", unknownTool != null);
                Check.Equal(
                    "stdio: unknown tool maps to -32000",
                    StdioTransportHost.ServerErrorCode,
                    unknownTool != null ? (int)unknownTool.ErrorCode : 0);

                McpProtocolException? unknownResource = await CatchProtocolAsync(() =>
                    client.ReadResourceAsync(
                        new Uri("unitas://unknown/resource"),
                        (RequestOptions?)null,
                        cts.Token).AsTask()).ConfigureAwait(false);
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
