using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class BuiltinDefaults
    {
        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            host.RegisterTool(
                new ToolDefinition(
                    "project.status",
                    "Read current unitas project status summary",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                _ => ToolOutput.Untrusted(
                    "unitas://project/status",
                    new JsonObject
                    {
                        ["state"] = "ready",
                    }));

            host.RegisterResource(
                new ResourceDefinition(
                    "unitas://knowledge/scopes",
                    "AUT-007 Permission Scopes",
                    "Fixed permission scope catalog for unitas MCP sessions",
                    "application/json",
                    Scope.ReadProject),
                () =>
                {
                    JsonArray scopes = new JsonArray();
                    foreach (Scope scope in Scopes.All)
                    {
                        scopes.Add(Scopes.AsString(scope));
                    }

                    return ResourceContent.Untrusted(
                        "unitas://knowledge/scopes",
                        new JsonObject
                        {
                            ["scopes"] = scopes,
                        });
                });
        }

        public static void RegisterAll(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            Register(host);
            AuditTools.Register(host);
            PlaySessionManager playSessionManager = PlayTools.Register(host);
            host.OwnResource(playSessionManager);
            CaptureTool.Register(host, playSessionManager);
            host.TryRegisterNativeLibrary();
        }

        public static McpHost CreateHostWithDefaults()
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(null),
                new AuditLog());
            Register(host);
            return host;
        }

        public static McpHost CreateHostWithDefaults(IClock clock)
        {
            McpHost host = new McpHost(clock);
            Register(host);
            return host;
        }

        public static McpHost CreateStartupHost(string[]? args = null)
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(args),
                new AuditLog());
            RegisterAll(host);
            return host;
        }
    }
}
