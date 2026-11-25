using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class OperationToolsTests
    {
        public static void Run()
        {
            RegistersOperationToolsWithTheReadProjectScope();
            ListReturnsTheHostOperationsInStartOrder();
            GetReturnsOneOperationAndRefusesAnUnknownId();
            LiveHostRegistersTheChangeReviewBuildAndOperationTools();
        }

        private static void RegistersOperationToolsWithTheReadProjectScope()
        {
            using McpHost host = NewHost();
            OperationTools.Register(host);

            ToolDefinition? list = FindTool(host, "operations.list");
            ToolDefinition? get = FindTool(host, "operations.get");
            Check.That("operations: operations.list is registered", list != null);
            Check.That("operations: operations.get is registered", get != null);
            Check.That(
                "operations: operations.list requires ReadProject",
                list != null && list.RequiredScope == Scope.ReadProject);
            Check.That(
                "operations: operations.get requires ReadProject",
                get != null && get.RequiredScope == Scope.ReadProject);
            Check.That(
                "operations: operations.get schema declares id",
                get != null && get.InputSchema["properties"]?["id"] != null);
        }

        private static void ListReturnsTheHostOperationsInStartOrder()
        {
            using McpHost host = NewHost();
            OperationTools.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            string first = host.StartOperation(session, "build-player", Scope.ReadProject);
            string second = host.StartOperation(session, "import-assets", Scope.ReadProject);

            JsonNode envelope = host.CallTool(session, "operations.list", new JsonObject());
            Check.That(
                "operations: list returns an untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            Check.Equal(
                "operations: list origin",
                "unitas://operations/list",
                envelope["origin"]?.GetValue<string>() ?? string.Empty);
            JsonArray? operations = envelope["content"]?["operations"] as JsonArray;
            Check.Equal("operations: list returns both operations", 2, operations?.Count ?? 0);
            if (operations == null || operations.Count != 2)
            {
                return;
            }

            JsonObject firstOperation = (JsonObject)operations[0]!;
            Check.Equal("operations: list keeps the start order", first, firstOperation["id"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "operations: list names the operation",
                "build-player",
                firstOperation["operation"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "operations: list reports the pending status",
                "pending",
                firstOperation["status"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "operations: list keeps the second operation",
                second,
                ((JsonObject)operations[1]!)["id"]?.GetValue<string>() ?? string.Empty);
            Check.That(
                "operations: list is audited as an allowed tool call",
                HasAllowedToolRecord(host, "operations.list"));
        }

        private static void GetReturnsOneOperationAndRefusesAnUnknownId()
        {
            using McpHost host = NewHost();
            OperationTools.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            string id = host.StartOperation(session, "build-player", Scope.ReadProject);
            host.CompleteOperation(id, "unitas://build/player", new JsonObject { ["artifact"] = "App.app" });

            JsonNode envelope = host.CallTool(session, "operations.get", new JsonObject { ["id"] = id });
            Check.That(
                "operations: get returns an untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            Check.Equal(
                "operations: get origin",
                "unitas://operations/get",
                envelope["origin"]?.GetValue<string>() ?? string.Empty);
            JsonObject? content = envelope["content"] as JsonObject;
            Check.Equal(
                "operations: get returns the requested operation",
                id,
                content?["id"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "operations: get reports the completed status",
                "completed",
                content?["status"]?.GetValue<string>() ?? string.Empty);
            Check.That(
                "operations: get carries the untrusted result envelope",
                UntrustedContent.IsEnvelope(content?["result"]));

            bool missingIdRefused = false;
            try
            {
                host.CallTool(session, "operations.get", new JsonObject());
            }
            catch (McpError.InvalidParams)
            {
                missingIdRefused = true;
            }

            Check.That("operations: get requires an id", missingIdRefused);

            bool unknownRefused = false;
            try
            {
                host.CallTool(session, "operations.get", new JsonObject { ["id"] = "op-999" });
            }
            catch (McpError.UnknownOperation)
            {
                unknownRefused = true;
            }

            Check.That("operations: get refuses an unknown operation id", unknownRefused);
            Check.That(
                "operations: get is audited as an allowed tool call",
                HasAllowedToolRecord(host, "operations.get"));
        }

        private static void LiveHostRegistersTheChangeReviewBuildAndOperationTools()
        {
            using McpHost host = NewHost();
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(host, playSessions);

            foreach (string name in new[]
            {
                "changes.list",
                "changes.show",
                "changes.approve",
                "changes.reject",
                "build.licensing",
                "operations.list",
                "operations.get",
            })
            {
                Check.That("operations: the live host registers " + name, FindTool(host, name) != null);
            }

            bool pendingResourceRegistered = false;
            foreach (ResourceDefinition resource in host.ListResources())
            {
                if (resource.Uri == ChangeReviewTools.PendingOrigin)
                {
                    pendingResourceRegistered = true;
                }
            }

            Check.That(
                "operations: the live host registers the pending change-sets resource",
                pendingResourceRegistered);
        }

        private static ToolDefinition? FindTool(McpHost host, string name)
        {
            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            foreach (ToolDefinition candidate in tools)
            {
                if (candidate.Name == name)
                {
                    return candidate;
                }
            }

            return null;
        }

        private static bool HasAllowedToolRecord(McpHost host, string target)
        {
            foreach (AuditRecord record in host.Audit.Records)
            {
                if (record.Target == target
                    && record.TargetKind == AuditTargetKind.Tool
                    && record.Outcome.StatusString == "allowed")
                {
                    return true;
                }
            }

            return false;
        }

        private static McpHost NewHost()
        {
            return new McpHost(new FixedClock(1_700_000_000_000L));
        }
    }
}
