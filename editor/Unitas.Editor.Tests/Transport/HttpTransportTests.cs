using System;
using System.Collections.Generic;
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
    public static class HttpTransportTests
    {
        public static void Run()
        {
            ConcurrentHttpClientsHaveIsolatedSessionsScopesAndAuditRecordsWhilePersonSessionRemainsUnaffected()
                .GetAwaiter()
                .GetResult();
            ConcurrentClientsKeepTheirOwnAuditIdentity()
                .GetAwaiter()
                .GetResult();
            HttpSelfWideningAndUnknownPrimitiveErrorsAreMappedAndAudited()
                .GetAwaiter()
                .GetResult();
        }

        private static async Task ConcurrentHttpClientsHaveIsolatedSessionsScopesAndAuditRecordsWhilePersonSessionRemainsUnaffected()
        {
            McpHost host = BuildTestHost();
            string personSessionId = host.CreateSession(
                new SessionIdentity("editor-person", "local-ui", "person-local"));
            host.GrantScope(personSessionId, Scope.Deploy, GrantAuthority.Person);

            HttpTransportHost transportHost = new HttpTransportHost(host, 0);
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await transportHost.StartAsync(cts.Token).ConfigureAwait(false);
            try
            {
                Check.That("http: server binds a positive loopback port", transportHost.Port > 0);
                Check.Equal("http: server binds to 127.0.0.1", "127.0.0.1", transportHost.EndpointUri.Host);

                HttpClientTransport transportA = CreateClientTransport(
                    transportHost.EndpointUri,
                    "agent-alpha",
                    "person-alpha");
                try
                {
                    McpClient clientA = await ConnectClientAsync(
                        transportA,
                        "agent-alpha",
                        cts.Token).ConfigureAwait(false);
                    try
                    {
                        HttpClientTransport transportB = CreateClientTransport(
                            transportHost.EndpointUri,
                            "agent-beta",
                            "person-beta");
                        try
                        {
                            McpClient clientB = await ConnectClientAsync(
                                transportB,
                                "agent-beta",
                                cts.Token).ConfigureAwait(false);
                            try
                            {
                                IList<McpClientTool> toolsA = await clientA.ListToolsAsync(
                                    (RequestOptions?)null,
                                    cts.Token).ConfigureAwait(false);
                                IList<McpClientTool> toolsB = await clientB.ListToolsAsync(
                                    (RequestOptions?)null,
                                    cts.Token).ConfigureAwait(false);
                                Check.Equal("http: client A lists 3 tools", 3, toolsA.Count);
                                Check.Equal("http: client B lists 3 tools", 3, toolsB.Count);

                                string? sessionA = transportHost.TryGetHostSessionId("agent-alpha");
                                string? sessionB = transportHost.TryGetHostSessionId("agent-beta");
                                Check.That("http: client A has a resolved host session", !string.IsNullOrEmpty(sessionA));
                                Check.That("http: client B has a resolved host session", !string.IsNullOrEmpty(sessionB));
                                Check.That(
                                    "http: client A and client B have distinct host sessions",
                                    !string.Equals(sessionA, sessionB, StringComparison.Ordinal));
                                Check.That(
                                    "http: client sessions are distinct from person session",
                                    !string.Equals(sessionA, personSessionId, StringComparison.Ordinal)
                                        && !string.Equals(sessionB, personSessionId, StringComparison.Ordinal));

                                CallToolResult inspectA = await clientA.CallToolAsync(
                                    new CallToolRequestParams { Name = "scene.inspect" },
                                    cts.Token).ConfigureAwait(false);
                                CallToolResult inspectB = await clientB.CallToolAsync(
                                    new CallToolRequestParams { Name = "scene.inspect" },
                                    cts.Token).ConfigureAwait(false);
                                Check.That(
                                    "http: client A read-project tool call returns untrusted envelope",
                                    UntrustedContent.IsEnvelope(ParseStructuredContent(inspectA)));
                                Check.That(
                                    "http: client B read-project tool call returns untrusted envelope",
                                    UntrustedContent.IsEnvelope(ParseStructuredContent(inspectB)));

                                host.GrantScope(sessionA!, Scope.EditContent, GrantAuthority.Person);

                                CallToolResult editA = await clientA.CallToolAsync(
                                    new CallToolRequestParams
                                    {
                                        Name = "scene.edit",
                                        Arguments = new Dictionary<string, JsonElement>
                                        {
                                            ["target"] = JsonDocument.Parse("\"obj:PlayerA\"").RootElement.Clone(),
                                        },
                                    },
                                    cts.Token).ConfigureAwait(false);
                                JsonNode? editAStructured = ParseStructuredContent(editA);
                                Check.That(
                                    "http: client A after grant calls scene.edit with untrusted envelope",
                                    UntrustedContent.IsEnvelope(editAStructured));
                                Check.Equal(
                                    "http: client A scene.edit updates obj:PlayerA",
                                    "obj:PlayerA",
                                    editAStructured?["content"]?["updated"]?.GetValue<string>() ?? string.Empty);

                                McpProtocolException? refusedEditB = await CatchProtocolAsync(() =>
                                    clientB.CallToolAsync(
                                        new CallToolRequestParams
                                        {
                                            Name = "scene.edit",
                                            Arguments = new Dictionary<string, JsonElement>
                                            {
                                                ["target"] = JsonDocument.Parse("\"obj:PlayerB\"").RootElement.Clone(),
                                            },
                                        },
                                        cts.Token).AsTask()).ConfigureAwait(false);
                                Check.That(
                                    "http: client B is refused scene.edit while only client A holds EditContent",
                                    refusedEditB != null);
                                Check.Equal(
                                    "http: client B refusal error code is -32001",
                                    HttpTransportHost.MissingScopeErrorCode,
                                    refusedEditB != null ? (int)refusedEditB.ErrorCode : 0);
                                Check.That(
                                    "http: client B refusal message names edit-content",
                                    refusedEditB != null
                                        && refusedEditB.Message.Contains("edit-content", StringComparison.Ordinal));

                                host.GrantScope(sessionB!, Scope.Build, GrantAuthority.Person);

                                CallToolResult buildB = await clientB.CallToolAsync(
                                    new CallToolRequestParams
                                    {
                                        Name = "project.build",
                                        Arguments = new Dictionary<string, JsonElement>
                                        {
                                            ["target"] = JsonDocument.Parse("\"macOS\"").RootElement.Clone(),
                                        },
                                    },
                                    cts.Token).ConfigureAwait(false);
                                JsonNode? buildBStructured = ParseStructuredContent(buildB);
                                Check.That(
                                    "http: client B after Build grant calls project.build with untrusted envelope",
                                    UntrustedContent.IsEnvelope(buildBStructured));
                                Check.Equal(
                                    "http: client B project.build target is macOS",
                                    "macOS",
                                    buildBStructured?["content"]?["built"]?.GetValue<string>() ?? string.Empty);

                                McpProtocolException? refusedBuildA = await CatchProtocolAsync(() =>
                                    clientA.CallToolAsync(
                                        new CallToolRequestParams
                                        {
                                            Name = "project.build",
                                            Arguments = new Dictionary<string, JsonElement>
                                            {
                                                ["target"] = JsonDocument.Parse("\"macOS\"").RootElement.Clone(),
                                            },
                                        },
                                        cts.Token).AsTask()).ConfigureAwait(false);
                                Check.That(
                                    "http: client A is refused project.build because Build was granted only to client B",
                                    refusedBuildA != null);
                                Check.Equal(
                                    "http: client A build refusal error code is -32001",
                                    HttpTransportHost.MissingScopeErrorCode,
                                    refusedBuildA != null ? (int)refusedBuildA.ErrorCode : 0);

                                Uri manifestUri = new Uri("unitas://project/manifest");
                                ReadResourceResult resourceA = await clientA.ReadResourceAsync(
                                    manifestUri,
                                    (RequestOptions?)null,
                                    cts.Token).ConfigureAwait(false);
                                ReadResourceResult resourceB = await clientB.ReadResourceAsync(
                                    manifestUri,
                                    (RequestOptions?)null,
                                    cts.Token).ConfigureAwait(false);
                                Check.That(
                                    "http: client A resource read returns untrusted envelope",
                                    UntrustedContent.IsEnvelope(ParseResourceEnvelope(resourceA)));
                                Check.That(
                                    "http: client B resource read returns untrusted envelope",
                                    UntrustedContent.IsEnvelope(ParseResourceEnvelope(resourceB)));

                                McpError? personEditRefused = CatchHostError(() =>
                                    host.CallTool(
                                        personSessionId,
                                        "scene.edit",
                                        new JsonObject { ["target"] = "obj:Person" }));
                                Check.That(
                                    "http: person session scopes remain unaffected by client A and B grants",
                                    personEditRefused is McpError.MissingScope);
                                string personDeployOp = host.StartOperation(personSessionId, "deploy.macos", Scope.Deploy);
                                Check.That(
                                    "http: person session retains its own Deploy scope",
                                    !string.IsNullOrEmpty(personDeployOp));

                                VerifyAuditRecords(host.Audit.Records, sessionA!, sessionB!, personSessionId);
                            }
                            finally
                            {
                                await clientB.DisposeAsync().ConfigureAwait(false);
                            }
                        }
                        finally
                        {
                            await transportB.DisposeAsync().ConfigureAwait(false);
                        }
                    }
                    finally
                    {
                        await clientA.DisposeAsync().ConfigureAwait(false);
                    }
                }
                finally
                {
                    await transportA.DisposeAsync().ConfigureAwait(false);
                }
            }
            finally
            {
                await transportHost.DisposeAsync().ConfigureAwait(false);
            }
        }

        private static async Task ConcurrentClientsKeepTheirOwnAuditIdentity()
        {
            McpHost host = BuildTestHost();
            HttpTransportHost transportHost = new HttpTransportHost(host, 0);
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await transportHost.StartAsync(cts.Token).ConfigureAwait(false);
            try
            {
                HttpClientTransport transportA = CreateClientTransport(
                    transportHost.EndpointUri,
                    "agent-alpha",
                    "person-alpha");
                HttpClientTransport transportB = CreateClientTransport(
                    transportHost.EndpointUri,
                    "agent-beta",
                    "person-beta");
                try
                {
                    McpClient clientA = await ConnectClientAsync(
                        transportA,
                        "agent-alpha",
                        cts.Token).ConfigureAwait(false);
                    try
                    {
                        McpClient clientB = await ConnectClientAsync(
                            transportB,
                            "agent-beta",
                            cts.Token).ConfigureAwait(false);
                        try
                        {
                            await clientB.CallToolAsync(
                                new CallToolRequestParams { Name = "scene.inspect" },
                                cts.Token).ConfigureAwait(false);
                            await clientA.ReadResourceAsync(
                                new Uri("unitas://project/manifest"),
                                (RequestOptions?)null,
                                cts.Token).ConfigureAwait(false);

                            string? sessionA = transportHost.TryGetHostSessionId("agent-alpha");
                            string? sessionB = transportHost.TryGetHostSessionId("agent-beta");
                            Check.That(
                                "http: concurrent client A has a host session",
                                !string.IsNullOrEmpty(sessionA));
                            Check.That(
                                "http: concurrent client B has a host session",
                                !string.IsNullOrEmpty(sessionB));
                            Check.That(
                                "http: concurrent clients keep distinct sessions",
                                !string.Equals(sessionA, sessionB, StringComparison.Ordinal));

                            IReadOnlyList<AuditRecord> records = host.Audit.Records;
                            Check.That(
                                "http: concurrent clients produce audit records",
                                records.Count >= 2);
                            bool sawClientBCall = false;
                            bool sawClientACall = false;
                            foreach (AuditRecord record in records)
                            {
                                if (record.TargetKind == AuditTargetKind.Tool
                                    && record.Target == "scene.inspect")
                                {
                                    sawClientBCall = true;
                                    Check.Equal(
                                        "http: client B audit record carries agent-beta",
                                        "agent-beta",
                                        record.Identity.Agent);
                                    Check.Equal(
                                        "http: client B audit record carries person-beta",
                                        "person-beta",
                                        record.Identity.AuthorizedBy);
                                }
                                else if (record.TargetKind == AuditTargetKind.Resource
                                    && record.Target == "unitas://project/manifest")
                                {
                                    sawClientACall = true;
                                    Check.Equal(
                                        "http: client A audit record carries agent-alpha",
                                        "agent-alpha",
                                        record.Identity.Agent);
                                    Check.Equal(
                                        "http: client A audit record carries person-alpha",
                                        "person-alpha",
                                        record.Identity.AuthorizedBy);
                                }
                            }

                            Check.That("http: client B call is audited", sawClientBCall);
                            Check.That("http: client A call is audited", sawClientACall);
                        }
                        finally
                        {
                            await clientB.DisposeAsync().ConfigureAwait(false);
                        }
                    }
                    finally
                    {
                        await clientA.DisposeAsync().ConfigureAwait(false);
                    }
                }
                finally
                {
                    await transportA.DisposeAsync().ConfigureAwait(false);
                    await transportB.DisposeAsync().ConfigureAwait(false);
                }
            }
            finally
            {
                await transportHost.DisposeAsync().ConfigureAwait(false);
            }
        }

        private static async Task HttpSelfWideningAndUnknownPrimitiveErrorsAreMappedAndAudited()
        {
            McpHost host = BuildTestHost();
            HttpTransportHost transportHost = new HttpTransportHost(host, 0);
            using CancellationTokenSource cts = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await transportHost.StartAsync(cts.Token).ConfigureAwait(false);
            try
            {
                HttpClientTransport clientTransport = CreateClientTransport(
                    transportHost.EndpointUri,
                    "agent-gamma",
                    "person-gamma");
                try
                {
                    McpClient client = await ConnectClientAsync(
                        clientTransport,
                        "agent-gamma",
                        cts.Token).ConfigureAwait(false);
                    try
                    {
                        McpProtocolException? selfWidening = await CatchProtocolAsync(() =>
                            client.CallToolAsync(
                                new CallToolRequestParams
                                {
                                    Name = "permissions/grant",
                                    Arguments = new Dictionary<string, JsonElement>
                                    {
                                        ["scope"] = JsonDocument.Parse("\"edit-scripts\"").RootElement.Clone(),
                                    },
                                },
                                cts.Token).AsTask()).ConfigureAwait(false);
                        Check.That("http: self-widening tool call throws McpProtocolException", selfWidening != null);
                        Check.Equal(
                            "http: self-widening error code is -32002",
                            HttpTransportHost.SelfWideningErrorCode,
                            selfWidening != null ? (int)selfWidening.ErrorCode : 0);

                        McpProtocolException? unknownTool = await CatchProtocolAsync(() =>
                            client.CallToolAsync(
                                new CallToolRequestParams { Name = "no.such.tool" },
                                cts.Token).AsTask()).ConfigureAwait(false);
                        Check.That("http: unknown tool throws McpProtocolException", unknownTool != null);
                        Check.Equal(
                            "http: unknown tool error code is -32000",
                            HttpTransportHost.ServerErrorCode,
                            unknownTool != null ? (int)unknownTool.ErrorCode : 0);

                        IReadOnlyList<AuditRecord> records = host.Audit.Records;
                        Check.Equal("http: self-widening is recorded in audit log", 1, records.Count);
                        Check.That(
                            "http: self-widening audit record outcome is RefusedSelfWidening",
                            records[0].Outcome is AuditOutcome.RefusedSelfWidening);
                        Check.Equal(
                            "http: self-widening audit record identity agent is agent-gamma",
                            "agent-gamma",
                            records[0].Identity.Agent);
                    }
                    finally
                    {
                        await client.DisposeAsync().ConfigureAwait(false);
                    }
                }
                finally
                {
                    await clientTransport.DisposeAsync().ConfigureAwait(false);
                }
            }
            finally
            {
                await transportHost.DisposeAsync().ConfigureAwait(false);
            }
        }

        private static void VerifyAuditRecords(
            IReadOnlyList<AuditRecord> records,
            string sessionA,
            string sessionB,
            string personSessionId)
        {
            int countA = 0;
            int countB = 0;
            int countPerson = 0;
            bool sawEditAllowedForA = false;
            bool sawEditRefusedForB = false;
            bool sawBuildAllowedForB = false;
            bool sawBuildRefusedForA = false;
            bool sawResourceReadForA = false;
            bool sawResourceReadForB = false;

            foreach (AuditRecord record in records)
            {
                if (string.Equals(record.SessionId, sessionA, StringComparison.Ordinal))
                {
                    countA++;
                    Check.Equal("http: audit record for A carries agent-alpha", "agent-alpha", record.Identity.Agent);
                    Check.Equal("http: audit record for A carries http client", "http", record.Identity.Client);
                    Check.Equal(
                        "http: audit record for A carries person-alpha authorized_by",
                        "person-alpha",
                        record.Identity.AuthorizedBy);
                    if (record.TargetKind == AuditTargetKind.Tool
                        && record.Target == "scene.edit"
                        && record.Outcome is AuditOutcome.Allowed)
                    {
                        sawEditAllowedForA = true;
                        Check.Equal(
                            "http: allowed scene.edit record carries agent-alpha",
                            "agent-alpha",
                            record.Identity.Agent);
                    }

                    if (record.TargetKind == AuditTargetKind.Tool
                        && record.Target == "project.build"
                        && record.Outcome is AuditOutcome.RefusedMissingScope)
                    {
                        sawBuildRefusedForA = true;
                    }

                    if (record.TargetKind == AuditTargetKind.Resource
                        && record.Target == "unitas://project/manifest"
                        && record.Outcome is AuditOutcome.Allowed)
                    {
                        sawResourceReadForA = true;
                    }
                }
                else if (string.Equals(record.SessionId, sessionB, StringComparison.Ordinal))
                {
                    countB++;
                    Check.Equal("http: audit record for B carries agent-beta", "agent-beta", record.Identity.Agent);
                    Check.Equal("http: audit record for B carries http client", "http", record.Identity.Client);
                    Check.Equal(
                        "http: audit record for B carries person-beta authorized_by",
                        "person-beta",
                        record.Identity.AuthorizedBy);
                    if (record.TargetKind == AuditTargetKind.Tool
                        && record.Target == "scene.edit"
                        && record.Outcome is AuditOutcome.RefusedMissingScope)
                    {
                        sawEditRefusedForB = true;
                    }

                    if (record.TargetKind == AuditTargetKind.Tool
                        && record.Target == "project.build"
                        && record.Outcome is AuditOutcome.Allowed)
                    {
                        sawBuildAllowedForB = true;
                        Check.Equal(
                            "http: allowed project.build record carries agent-beta",
                            "agent-beta",
                            record.Identity.Agent);
                    }

                    if (record.TargetKind == AuditTargetKind.Resource
                        && record.Target == "unitas://project/manifest"
                        && record.Outcome is AuditOutcome.Allowed)
                    {
                        sawResourceReadForB = true;
                    }
                }
                else if (string.Equals(record.SessionId, personSessionId, StringComparison.Ordinal))
                {
                    countPerson++;
                    Check.Equal(
                        "http: audit record for person carries editor-person",
                        "editor-person",
                        record.Identity.Agent);
                }
            }

            Check.That("http: client A has audit records", countA >= 5);
            Check.That("http: client B has audit records", countB >= 5);
            Check.That("http: person session has audit records", countPerson >= 3);
            Check.That("http: audit log records allowed scene.edit for client A", sawEditAllowedForA);
            Check.That("http: audit log records refused scene.edit for client B", sawEditRefusedForB);
            Check.That("http: audit log records allowed project.build for client B", sawBuildAllowedForB);
            Check.That("http: audit log records refused project.build for client A", sawBuildRefusedForA);
            Check.That("http: audit log records resource read for client A", sawResourceReadForA);
            Check.That("http: audit log records resource read for client B", sawResourceReadForB);
        }

        private static HttpClientTransport CreateClientTransport(
            Uri endpointUri,
            string agentName,
            string authorizedBy)
        {
            HttpClientTransportOptions transportOptions = new HttpClientTransportOptions
            {
                Endpoint = endpointUri,
                Name = agentName,
                AdditionalHeaders = new Dictionary<string, string>
                {
                    [HttpTransportHost.AgentHeaderName] = agentName,
                    [HttpTransportHost.AuthorizedByHeaderName] = authorizedBy,
                },
            };
            return new HttpClientTransport(transportOptions, null);
        }

        private static async Task<McpClient> ConnectClientAsync(
            HttpClientTransport clientTransport,
            string agentName,
            CancellationToken cancellationToken)
        {
            return await McpClient.CreateAsync(
                clientTransport,
                new McpClientOptions
                {
                    ClientInfo = new Implementation
                    {
                        Name = agentName,
                        Version = "1.0.0",
                    },
                },
                null,
                cancellationToken).ConfigureAwait(false);
        }

        private static McpHost BuildTestHost()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            host.RegisterTool(
                new ToolDefinition(
                    "scene.inspect",
                    "Inspect active scene hierarchy",
                    new JsonObject { ["type"] = "object" },
                    Scope.ReadProject),
                _ => ToolOutput
                    .Untrusted("scene:Main", new JsonObject { ["objects"] = new JsonArray("Camera", "Player") })
                    .WithAffectedIdentities("scene:Main"));
            host.RegisterTool(
                new ToolDefinition(
                    "scene.edit",
                    "Modify a scene property",
                    new JsonObject { ["type"] = "object" },
                    Scope.EditContent),
                arguments =>
                {
                    string target = arguments["target"]?.GetValue<string>() ?? "unknown";
                    return ToolOutput
                        .Untrusted("scene:Main", new JsonObject { ["updated"] = target })
                        .WithAffectedIdentities(target);
                });
            host.RegisterTool(
                new ToolDefinition(
                    "project.build",
                    "Build project player target",
                    new JsonObject { ["type"] = "object" },
                    Scope.Build),
                arguments =>
                {
                    string target = arguments["target"]?.GetValue<string>() ?? "default";
                    return ToolOutput
                        .Untrusted("build:Pipeline", new JsonObject { ["built"] = target })
                        .WithAffectedIdentities("build:" + target);
                });
            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://project/manifest",
                    "Project Manifest",
                    "Manifest metadata",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(
                    "project:manifest",
                    new JsonObject { ["name"] = "RollABall" }));
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

        private static McpError? CatchHostError(Func<JsonNode> action)
        {
            try
            {
                action();
                return null;
            }
            catch (McpError error)
            {
                return error;
            }
        }
    }
}
