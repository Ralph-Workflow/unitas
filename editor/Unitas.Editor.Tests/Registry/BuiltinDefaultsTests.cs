using System.Collections.Generic;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class BuiltinDefaultsTests
    {
        public static void Run()
        {
            RegistersProjectStatusToolAndKnowledgeScopesResource();
            BuiltinDefaultsAreReadProjectScopedAndUntrusted();
            RegisteringBuiltinDefaultsTwiceFailsWithDuplicateTool();
        }

        private static void RegistersProjectStatusToolAndKnowledgeScopesResource()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);

            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            Check.Equal("builtin: one default tool registered", 1, tools.Count);
            Check.Equal("builtin: default tool name is project.status", "project.status", tools[0].Name);
            Check.Equal(
                "builtin: default tool description",
                "Read current unitas project status summary",
                tools[0].Description);
            Check.That(
                "builtin: default tool requires ReadProject",
                tools[0].RequiredScope == Scope.ReadProject);

            IReadOnlyList<ResourceDefinition> resources = host.ListResources();
            Check.Equal("builtin: one default resource registered", 1, resources.Count);
            Check.Equal(
                "builtin: default resource uri is unitas://knowledge/scopes",
                "unitas://knowledge/scopes",
                resources[0].Uri);
            Check.Equal(
                "builtin: default resource name",
                "AUT-007 Permission Scopes",
                resources[0].Name);
            Check.Equal(
                "builtin: default resource mimeType",
                "application/json",
                resources[0].MimeType);
            Check.That(
                "builtin: default resource requires ReadProject",
                resources[0].RequiredScope == Scope.ReadProject);
        }

        private static void BuiltinDefaultsAreReadProjectScopedAndUntrusted()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));

            JsonNode statusEnvelope = host.CallTool(session, "project.status", new JsonObject());
            Check.That(
                "builtin: project.status returns untrusted envelope",
                UntrustedContent.IsEnvelope(statusEnvelope));
            Check.Equal(
                "builtin: project.status origin",
                "unitas://project/status",
                statusEnvelope["origin"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "builtin: project.status state is ready",
                "ready",
                statusEnvelope["content"]?["state"]?.GetValue<string>() ?? string.Empty);

            JsonNode scopesEnvelope = host.ReadResource(session, "unitas://knowledge/scopes");
            Check.That(
                "builtin: knowledge/scopes returns untrusted envelope",
                UntrustedContent.IsEnvelope(scopesEnvelope));
            Check.Equal(
                "builtin: knowledge/scopes origin",
                "unitas://knowledge/scopes",
                scopesEnvelope["origin"]?.GetValue<string>() ?? string.Empty);
            JsonArray? scopeArray = scopesEnvelope["content"]?["scopes"] as JsonArray;
            Check.Equal(
                "builtin: knowledge/scopes lists all 11 fixed scopes",
                Scopes.All.Length,
                scopeArray?.Count ?? 0);
            Check.Equal(
                "builtin: knowledge/scopes first scope is read-project",
                "read-project",
                scopeArray?[0]?.GetValue<string>() ?? string.Empty);
        }

        private static void RegisteringBuiltinDefaultsTwiceFailsWithDuplicateTool()
        {
            McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            bool threwDuplicate = false;
            try
            {
                BuiltinDefaults.Register(host);
            }
            catch (McpError.DuplicateTool error)
            {
                threwDuplicate = error.Name == "project.status";
            }

            Check.That("builtin: second registration throws DuplicateTool", threwDuplicate);
        }
    }
}
