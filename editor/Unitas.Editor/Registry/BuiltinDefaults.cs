using System;
using System.IO;
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

        public static void RegisterAll(McpHost host, PlaySessionManager playSessions)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(playSessions);
            Register(host);
            AuditTools.Register(host);
            PlayTools.Register(host, playSessions);
            CaptureTool.Register(host, playSessions);
            ScenarioTools.Register(host);
            ChangeReviewTools.Register(host, new ChangeSetStore(host));
            BuildTools.Register(host);
            OperationTools.Register(host);
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

        public static EditorRuntime CreateStartupRuntime(string[]? args = null)
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(args),
                CreateStartupAuditLog(args));
            PlaySessionManager playSessions = new PlaySessionManager();
            RegisterAll(host, playSessions);
            return new EditorRuntime(host, playSessions);
        }

        public static McpHost CreateStartupHost(string[]? args = null)
        {
            McpHost host = new McpHost(
                new SystemClock(),
                PersonCredential.FromStartup(args),
                CreateStartupAuditLog(args));
            PlaySessionManager playSessions = new PlaySessionManager();
            host.OwnResource(playSessions);
            RegisterAll(host, playSessions);
            return host;
        }

        private static AuditLog CreateStartupAuditLog(string[]? args)
        {
            string projectPath = ResolveStartupProjectPath(args);
            try
            {
                return new AuditLog(AuditStore.ForProject(projectPath));
            }
            catch (Exception error) when (error is McpError
                || error is IOException
                || error is UnauthorizedAccessException
                || error is ArgumentException)
            {
                string reason = "audit store unavailable for project " + projectPath + ": " + error.Message;
                Console.Error.WriteLine("[unitas] " + reason);
                return new AuditLog(null, reason);
            }
        }

        private static string ResolveStartupProjectPath(string[]? args)
        {
            string? requested = null;
            if (args != null)
            {
                for (int index = 0; index < args.Length - 1; index++)
                {
                    if (string.Equals(args[index], CliArgs.ProjectPathOption, StringComparison.Ordinal))
                    {
                        requested = args[index + 1];
                        break;
                    }
                }
            }

            if (string.IsNullOrWhiteSpace(requested))
            {
                return Directory.GetCurrentDirectory();
            }

            return Path.GetFullPath(requested);
        }
    }
}
