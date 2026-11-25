using System;
using System.Collections.Generic;
using System.Net;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Hosting.Server;
using Microsoft.AspNetCore.Hosting.Server.Features;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using ModelContextProtocol;
using ModelContextProtocol.AspNetCore;
using ModelContextProtocol.Protocol;
using ModelContextProtocol.Server;

namespace Unitas.Editor.Mcp
{
    public sealed class HttpTransportHost : IAsyncDisposable
    {
        public const string ProtocolVersion = "2024-11-05";
        public const string ServerName = "unitas-mcp";
        public const string ServerVersion = "0.1.0";
        public const string ClientTransportName = "http";
        public const string DefaultAuthorizedBy = "http-local";
        public const string AgentHeaderName = "X-Unitas-Agent";
        public const string AuthorizedByHeaderName = "X-Unitas-Authorized-By";
        public const int MissingScopeErrorCode = -32001;
        public const int SelfWideningErrorCode = -32002;
        public const int ServerErrorCode = -32000;

        private readonly McpHost host;
        private readonly int requestedPort;
        private readonly Dictionary<string, string> mcpToHostSession =
            new Dictionary<string, string>(StringComparer.Ordinal);
        private readonly Dictionary<string, string> agentToHostSession =
            new Dictionary<string, string>(StringComparer.Ordinal);
        private readonly Dictionary<string, PendingIdentity> pendingHeaders =
            new Dictionary<string, PendingIdentity>(StringComparer.Ordinal);
        private readonly List<string> createdSessionIds = new List<string>();
        private WebApplication? app;
        private Uri? endpointUri;
        private int boundPort;

        public HttpTransportHost(McpHost host, int port = 0)
        {
            ArgumentNullException.ThrowIfNull(host);
            if (port < 0 || port > 65535)
            {
                throw new ArgumentOutOfRangeException(nameof(port), "Port must be between 0 and 65535.");
            }

            this.host = host;
            requestedPort = port;
        }

        public McpHost Host => host;

        public int Port => boundPort;

        public Uri EndpointUri =>
            endpointUri ?? throw new InvalidOperationException("HttpTransportHost has not been started.");

        public IReadOnlyList<string> CreatedSessionIds
        {
            get
            {
                lock (host)
                {
                    return new List<string>(createdSessionIds);
                }
            }
        }

        public string? TryGetHostSessionId(string mcpOrAgentIdentifier)
        {
            ArgumentNullException.ThrowIfNull(mcpOrAgentIdentifier);
            lock (host)
            {
                if (mcpToHostSession.TryGetValue(mcpOrAgentIdentifier, out string? byMcp))
                {
                    return byMcp;
                }

                if (agentToHostSession.TryGetValue(mcpOrAgentIdentifier, out string? byAgent))
                {
                    return byAgent;
                }

                return null;
            }
        }

        public async Task StartAsync(CancellationToken cancellationToken = default)
        {
            if (app != null)
            {
                throw new InvalidOperationException("HttpTransportHost is already running.");
            }

            WebApplicationBuilder builder = WebApplication.CreateSlimBuilder();
            builder.Logging.ClearProviders();
            builder.WebHost.ConfigureKestrel(kestrel =>
            {
                kestrel.Listen(IPAddress.Loopback, requestedPort);
            });

            builder.Services
                .AddMcpServer(ConfigureMcpServerOptions)
                .WithHttpTransport(transport =>
                {
                    transport.SessionMode = HttpServerSessionMode.Stateful;
                    transport.ConfigureSessionOptions = ConfigurePerSessionOptionsAsync;
                });

            WebApplication builtApp = builder.Build();
            builtApp.MapMcp();

            await builtApp.StartAsync(cancellationToken).ConfigureAwait(false);
            app = builtApp;
            ResolveBoundAddress(builtApp);
        }

        public async Task RunAsync(CancellationToken cancellationToken = default)
        {
            await StartAsync(cancellationToken).ConfigureAwait(false);
            try
            {
                TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>(
                    TaskCreationOptions.RunContinuationsAsynchronously);
                using CancellationTokenRegistration registration = cancellationToken.Register(
                    () => tcs.TrySetResult(true));
                await tcs.Task.ConfigureAwait(false);
            }
            finally
            {
                await StopAsync(CancellationToken.None).ConfigureAwait(false);
            }
        }

        public async Task StopAsync(CancellationToken cancellationToken = default)
        {
            WebApplication? running = app;
            app = null;
            if (running != null)
            {
                try
                {
                    await running.StopAsync(cancellationToken).ConfigureAwait(false);
                }
                finally
                {
                    await running.DisposeAsync().ConfigureAwait(false);
                }
            }
        }

        public async ValueTask DisposeAsync()
        {
            await StopAsync(CancellationToken.None).ConfigureAwait(false);
        }

        public static McpProtocolException MapError(McpError error)
        {
            ArgumentNullException.ThrowIfNull(error);
            switch (error)
            {
                case McpError.MissingScope missingScope:
                    return new McpProtocolException(
                        missingScope.Message,
                        (McpErrorCode)MissingScopeErrorCode);
                case McpError.SelfWidening selfWidening:
                    return new McpProtocolException(
                        selfWidening.Message,
                        (McpErrorCode)SelfWideningErrorCode);
                case McpError.UnknownMethod unknownMethod:
                    return new McpProtocolException(
                        unknownMethod.Message,
                        McpErrorCode.MethodNotFound);
                case McpError.InvalidParams invalidParams:
                    return new McpProtocolException(
                        invalidParams.Message,
                        McpErrorCode.InvalidParams);
                case McpError.InvalidRequest invalidRequest:
                    return new McpProtocolException(
                        invalidRequest.Message,
                        McpErrorCode.InvalidRequest);
                case McpError.ParseError parseError:
                    return new McpProtocolException(
                        parseError.Message,
                        McpErrorCode.ParseError);
                default:
                    return new McpProtocolException(
                        error.Message,
                        (McpErrorCode)ServerErrorCode);
            }
        }

        private void ConfigureMcpServerOptions(McpServerOptions options)
        {
            options.ServerInfo = new Implementation
            {
                Name = ServerName,
                Version = ServerVersion,
            };
            options.ProtocolVersion = ProtocolVersion;
            options.Capabilities = new ServerCapabilities
            {
                Tools = new ToolsCapability(),
                Resources = new ResourcesCapability(),
            };
            options.Handlers = new McpServerHandlers
            {
                ListToolsHandler = HandleListToolsAsync,
                CallToolHandler = HandleCallToolAsync,
                ListResourcesHandler = HandleListResourcesAsync,
                ReadResourceHandler = HandleReadResourceAsync,
            };
        }

        private Task ConfigurePerSessionOptionsAsync(
            HttpContext httpContext,
            McpServerOptions options,
            CancellationToken cancellationToken)
        {
            string? headerAgent = ExtractHeader(httpContext, AgentHeaderName);
            string? headerAuthorizedBy = ExtractHeader(httpContext, AuthorizedByHeaderName);
            string connectionKey = BuildConnectionKey(httpContext);
            if (!string.IsNullOrEmpty(connectionKey)
                && (!string.IsNullOrEmpty(headerAgent) || !string.IsNullOrEmpty(headerAuthorizedBy)))
            {
                lock (host)
                {
                    pendingHeaders[connectionKey] = new PendingIdentity(headerAgent, headerAuthorizedBy);
                }
            }

            ConfigureMcpServerOptions(options);
            return Task.CompletedTask;
        }

        private void ResolveBoundAddress(WebApplication startedApp)
        {
            IServer server = startedApp.Services.GetRequiredService<IServer>();
            IServerAddressesFeature? addressesFeature = server.Features.Get<IServerAddressesFeature>();
            if (addressesFeature != null)
            {
                foreach (string address in addressesFeature.Addresses)
                {
                    if (Uri.TryCreate(address, UriKind.Absolute, out Uri? parsed))
                    {
                        boundPort = parsed.Port;
                        endpointUri = new Uri("http://127.0.0.1:" + boundPort + "/");
                        return;
                    }
                }
            }

            boundPort = requestedPort;
            endpointUri = new Uri("http://127.0.0.1:" + boundPort + "/");
        }

        private string EnsureSession(McpServer? server)
        {
            string mcpSessionKey = server?.SessionId ?? string.Empty;
            string clientInfoName = server?.ClientInfo?.Name ?? string.Empty;

            lock (host)
            {
                if (!string.IsNullOrEmpty(mcpSessionKey)
                    && mcpToHostSession.TryGetValue(mcpSessionKey, out string? existingSessionId))
                {
                    return existingSessionId;
                }

                string agentName = !string.IsNullOrEmpty(clientInfoName)
                    ? clientInfoName
                    : ("mcp-http-client-" + (createdSessionIds.Count + 1));
                string authorizedBy = DefaultAuthorizedBy;

                if (pendingHeaders.Count > 0)
                {
                    string? matchedKey = null;
                    PendingIdentity? matchedPending = null;
                    foreach (KeyValuePair<string, PendingIdentity> entry in pendingHeaders)
                    {
                        matchedKey = entry.Key;
                        matchedPending = entry.Value;
                        break;
                    }

                    if (matchedKey != null && matchedPending != null)
                    {
                        pendingHeaders.Remove(matchedKey);
                        if (!string.IsNullOrEmpty(matchedPending.Agent))
                        {
                            agentName = matchedPending.Agent!;
                        }

                        if (!string.IsNullOrEmpty(matchedPending.AuthorizedBy))
                        {
                            authorizedBy = matchedPending.AuthorizedBy!;
                        }
                    }
                }

                string hostSessionId = host.CreateSession(
                    new SessionIdentity(agentName, ClientTransportName, authorizedBy));
                createdSessionIds.Add(hostSessionId);
                if (!string.IsNullOrEmpty(mcpSessionKey))
                {
                    mcpToHostSession[mcpSessionKey] = hostSessionId;
                }

                if (!string.IsNullOrEmpty(agentName))
                {
                    agentToHostSession[agentName] = hostSessionId;
                }

                return hostSessionId;
            }
        }

        private ValueTask<ListToolsResult> HandleListToolsAsync(
            RequestContext<ListToolsRequestParams> context,
            CancellationToken cancellationToken)
        {
            EnsureSession(context.Server);
            IReadOnlyList<ToolDefinition> definitions;
            lock (host)
            {
                definitions = host.ListTools();
            }

            List<Tool> tools = new List<Tool>(definitions.Count);
            foreach (ToolDefinition definition in definitions)
            {
                tools.Add(new Tool
                {
                    Name = definition.Name,
                    Description = definition.Description,
                    InputSchema = ToJsonElement(definition.InputSchema),
                    Meta = new JsonObject
                    {
                        ["requiredScope"] = Scopes.AsString(definition.RequiredScope),
                    },
                });
            }

            return new ValueTask<ListToolsResult>(new ListToolsResult
            {
                Tools = tools,
            });
        }

        private ValueTask<CallToolResult> HandleCallToolAsync(
            RequestContext<CallToolRequestParams> context,
            CancellationToken cancellationToken)
        {
            string sessionId = EnsureSession(context.Server);
            CallToolRequestParams? requestParams = context.Params;
            if (requestParams == null || string.IsNullOrEmpty(requestParams.Name))
            {
                throw MapError(new McpError.InvalidParams("missing tool name"));
            }

            JsonObject arguments = ToArgumentsJsonObject(requestParams.Arguments);
            JsonNode envelope;
            try
            {
                lock (host)
                {
                    envelope = host.CallTool(sessionId, requestParams.Name, arguments);
                }
            }
            catch (McpError error)
            {
                throw MapError(error);
            }

            string text = envelope.ToJsonString();
            JsonElement structured = ToJsonElement(envelope);
            return new ValueTask<CallToolResult>(new CallToolResult
            {
                Content = new List<ContentBlock>
                {
                    new TextContentBlock
                    {
                        Text = text,
                        Meta = new JsonObject
                        {
                            ["untrusted"] = true,
                        },
                    },
                },
                StructuredContent = structured,
                IsError = false,
            });
        }

        private ValueTask<ListResourcesResult> HandleListResourcesAsync(
            RequestContext<ListResourcesRequestParams> context,
            CancellationToken cancellationToken)
        {
            EnsureSession(context.Server);
            IReadOnlyList<ResourceDefinition> definitions;
            lock (host)
            {
                definitions = host.ListResources();
            }

            List<Resource> resources = new List<Resource>(definitions.Count);
            foreach (ResourceDefinition definition in definitions)
            {
                resources.Add(new Resource
                {
                    Uri = definition.Uri,
                    Name = definition.Name,
                    Description = definition.Description,
                    MimeType = definition.MimeType,
                    Meta = new JsonObject
                    {
                        ["requiredScope"] = Scopes.AsString(definition.RequiredScope),
                    },
                });
            }

            return new ValueTask<ListResourcesResult>(new ListResourcesResult
            {
                Resources = resources,
            });
        }

        private ValueTask<ReadResourceResult> HandleReadResourceAsync(
            RequestContext<ReadResourceRequestParams> context,
            CancellationToken cancellationToken)
        {
            string sessionId = EnsureSession(context.Server);
            ReadResourceRequestParams? requestParams = context.Params;
            if (requestParams == null || string.IsNullOrEmpty(requestParams.Uri))
            {
                throw MapError(new McpError.InvalidParams("missing resource uri"));
            }

            string uri = requestParams.Uri;
            string mimeType;
            JsonNode envelope;
            try
            {
                lock (host)
                {
                    mimeType = ResolveResourceMimeType(uri);
                    envelope = host.ReadResource(sessionId, uri);
                }
            }
            catch (McpError error)
            {
                throw MapError(error);
            }

            string text = envelope.ToJsonString();
            return new ValueTask<ReadResourceResult>(new ReadResourceResult
            {
                Contents = new List<ResourceContents>
                {
                    new TextResourceContents
                    {
                        Uri = uri,
                        MimeType = mimeType,
                        Text = text,
                        Meta = new JsonObject
                        {
                            ["untrusted"] = true,
                            ["structuredContent"] = envelope.DeepClone(),
                        },
                    },
                },
            });
        }

        private string ResolveResourceMimeType(string uri)
        {
            foreach (ResourceDefinition definition in host.ListResources())
            {
                if (string.Equals(definition.Uri, uri, StringComparison.Ordinal))
                {
                    return definition.MimeType;
                }
            }

            return "application/json";
        }

        private static string? ExtractHeader(HttpContext httpContext, string headerName)
        {
            if (httpContext.Request.Headers.TryGetValue(headerName, out Microsoft.Extensions.Primitives.StringValues values)
                && values.Count > 0)
            {
                string? raw = values[0];
                if (!string.IsNullOrWhiteSpace(raw))
                {
                    return raw.Trim();
                }
            }

            return null;
        }

        private static string BuildConnectionKey(HttpContext httpContext)
        {
            return httpContext.TraceIdentifier ?? Guid.NewGuid().ToString("N");
        }

        private static JsonObject ToArgumentsJsonObject(
            IDictionary<string, JsonElement>? arguments)
        {
            JsonObject result = new JsonObject();
            if (arguments == null)
            {
                return result;
            }

            foreach (KeyValuePair<string, JsonElement> pair in arguments)
            {
                result[pair.Key] = JsonNode.Parse(pair.Value.GetRawText());
            }

            return result;
        }

        private static JsonElement ToJsonElement(JsonNode node)
        {
            using JsonDocument document = JsonDocument.Parse(node.ToJsonString());
            return document.RootElement.Clone();
        }

        private sealed record PendingIdentity(string? Agent, string? AuthorizedBy);
    }
}
