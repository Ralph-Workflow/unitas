using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using ModelContextProtocol;
using ModelContextProtocol.Protocol;
using ModelContextProtocol.Server;

namespace Unitas.Editor.Mcp
{
    public sealed class StdioTransportHost
    {
        public const string ProtocolVersion = "2024-11-05";
        public const string ServerName = "unitas-mcp";
        public const string ServerVersion = "0.1.0";
        public const int MissingScopeErrorCode = -32001;
        public const int SelfWideningErrorCode = -32002;
        public const int ServerErrorCode = -32000;

        private readonly McpHost host;
        private readonly object transportLock = new object();
        private string? activeSessionId;

        public StdioTransportHost(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            this.host = host;
        }

        public McpHost Host => host;

        public string? ActiveSessionId
        {
            get
            {
                lock (transportLock)
                {
                    return activeSessionId;
                }
            }
        }

        public static StdioTransportHost WithDefaults()
        {
            return new StdioTransportHost(BuiltinDefaults.CreateHostWithDefaults());
        }

        public McpServerOptions BuildServerOptions()
        {
            return new McpServerOptions
            {
                ServerInfo = new Implementation
                {
                    Name = ServerName,
                    Version = ServerVersion,
                },
                ProtocolVersion = ProtocolVersion,
                Capabilities = new ServerCapabilities
                {
                    Tools = new ToolsCapability(),
                    Resources = new ResourcesCapability(),
                },
                Handlers = new McpServerHandlers
                {
                    ListToolsHandler = HandleListToolsAsync,
                    CallToolHandler = HandleCallToolAsync,
                    ListResourcesHandler = HandleListResourcesAsync,
                    ReadResourceHandler = HandleReadResourceAsync,
                },
            };
        }

        public Task RunConsoleAsync(CancellationToken cancellationToken = default)
        {
            return RunAsync(
                Console.OpenStandardInput(),
                Console.OpenStandardOutput(),
                cancellationToken);
        }

        public async Task RunAsync(
            Stream inputStream,
            Stream outputStream,
            CancellationToken cancellationToken = default)
        {
            ArgumentNullException.ThrowIfNull(inputStream);
            ArgumentNullException.ThrowIfNull(outputStream);

            McpServerOptions options = BuildServerOptions();
            StreamServerTransport transport = new StreamServerTransport(
                inputStream,
                outputStream,
                "stdio",
                null);
            try
            {
                McpServer server = McpServer.Create(transport, options, null, null);
                try
                {
                    await server.RunAsync(cancellationToken).ConfigureAwait(false);
                }
                finally
                {
                    await server.DisposeAsync().ConfigureAwait(false);
                }
            }
            finally
            {
                await transport.DisposeAsync().ConfigureAwait(false);
            }
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

        private string EnsureSession(McpServer? server)
        {
            lock (transportLock)
            {
                if (!string.IsNullOrEmpty(activeSessionId))
                {
                    return activeSessionId!;
                }

                string clientName = server?.ClientInfo?.Name ?? "mcp-stdio-client";
                activeSessionId = host.CreateSession(
                    new SessionIdentity(clientName, "stdio", "stdio-local"));
                return activeSessionId;
            }
        }

        private ValueTask<ListToolsResult> HandleListToolsAsync(
            RequestContext<ListToolsRequestParams> context,
            CancellationToken cancellationToken)
        {
            EnsureSession(context.Server);
            IReadOnlyList<ToolDefinition> definitions = host.ListTools();
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
                envelope = host.CallTool(sessionId, requestParams.Name, arguments);
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
            IReadOnlyList<ResourceDefinition> definitions = host.ListResources();
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
            string mimeType = ResolveResourceMimeType(uri);
            JsonNode envelope;
            try
            {
                envelope = host.ReadResource(sessionId, uri);
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
    }
}
