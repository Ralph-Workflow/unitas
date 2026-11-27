using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class KnowledgeResourceTests
    {
        public static void Run()
        {
            EveryProductionToolHasValidReversibilityAndConsistentBoolean();
            RepresentativeToolsClassifyToExpectedReversibility();
            AllThreeKnowledgeResourcesAreListedAndReadableWithVersionAndDisclosures();
        }

        private static void EveryProductionToolHasValidReversibilityAndConsistentBoolean()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(host, playSessions);

            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            Check.That("knowledge_rev: production host registers tools", tools.Count > 20);
            foreach (ToolDefinition tool in tools)
            {
                string rev = tool.EffectiveReversibility;
                bool validCategory = string.Equals(rev, "read-only", StringComparison.Ordinal)
                    || string.Equals(rev, "reversible", StringComparison.Ordinal)
                    || string.Equals(rev, "irreversible", StringComparison.Ordinal);
                Check.That(
                    "knowledge_rev: " + tool.Name + " has valid EffectiveReversibility (" + rev + ")",
                    validCategory);

                bool expectedBool = !string.Equals(rev, "irreversible", StringComparison.Ordinal);
                Check.Equal(
                    "knowledge_rev: " + tool.Name + " IsReversible matches EffectiveReversibility",
                    expectedBool,
                    tool.IsReversible);

                JsonNode serialized = tool.ToValue();
                Check.Equal(
                    "knowledge_rev: " + tool.Name + " ToValue() exposes reversibility",
                    rev,
                    serialized["reversibility"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "knowledge_rev: " + tool.Name + " ToValue() exposes reversible",
                    expectedBool,
                    serialized["reversible"]?.GetValue<bool>() ?? !expectedBool);
            }
        }

        private static void RepresentativeToolsClassifyToExpectedReversibility()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(host, playSessions);

            AssertToolReversibility(host, "scene.hierarchy", "read-only", true);
            AssertToolReversibility(host, "project.status", "read-only", true);
            AssertToolReversibility(host, "debug.stack", "read-only", true);
            AssertToolReversibility(host, "scene.set_property", "reversible", true);
            AssertToolReversibility(host, "editor.undo", "reversible", true);
            AssertToolReversibility(host, "play.enter", "reversible", true);
            AssertToolReversibility(host, "debug.step", "reversible", true);
            AssertToolReversibility(host, "scene.save", "irreversible", false);
            AssertToolReversibility(host, "scripts.compile", "irreversible", false);
            AssertToolReversibility(host, "editor.execute_method", "irreversible", false);
            AssertToolReversibility(host, "permissions.person_grant", "irreversible", false);
            AssertToolReversibility(host, "audit.delete", "irreversible", false);

            ToolDefinition buildPlayer = new ToolDefinition(
                "build.player",
                "Package a project directory into a player bundle",
                new JsonObject { ["type"] = "object" },
                Scope.Build);
            Check.Equal(
                "knowledge_rev: build.player classifies as irreversible",
                "irreversible",
                buildPlayer.EffectiveReversibility);
            Check.That(
                "knowledge_rev: build.player IsReversible is false",
                !buildPlayer.IsReversible);
        }

        private static void AssertToolReversibility(
            McpHost host,
            string toolName,
            string expectedReversibility,
            bool expectedIsReversible)
        {
            ToolDefinition? found = null;
            foreach (ToolDefinition candidate in host.ListTools())
            {
                if (string.Equals(candidate.Name, toolName, StringComparison.Ordinal))
                {
                    found = candidate;
                    break;
                }
            }

            Check.That("knowledge_rev: tool " + toolName + " is registered", found != null);
            if (found == null)
            {
                return;
            }

            Check.Equal(
                "knowledge_rev: " + toolName + " EffectiveReversibility",
                expectedReversibility,
                found.EffectiveReversibility);
            Check.Equal(
                "knowledge_rev: " + toolName + " IsReversible",
                expectedIsReversible,
                found.IsReversible);
        }

        private static void AllThreeKnowledgeResourcesAreListedAndReadableWithVersionAndDisclosures()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            BuiltinDefaults.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-k", "stdio", "stdio-local"));

            VerifyScopesKnowledgeResource(host, session);
            VerifyGrantsKnowledgeResource(host, session);
            VerifyDocsKnowledgeResource(host, session);
        }

        private static void VerifyScopesKnowledgeResource(McpHost host, string session)
        {
            JsonNode envelope = host.ReadResource(session, "unitas://knowledge/scopes");
            Check.That(
                "knowledge_scopes: untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            JsonNode? content = envelope["content"];
            Check.Equal(
                "knowledge_scopes: version is 0.1.0",
                "0.1.0",
                content?["version"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "knowledge_scopes: defaultScope is read-project",
                "read-project",
                content?["defaultScope"]?.GetValue<string>() ?? string.Empty);
            JsonObject? descriptions = content?["descriptions"] as JsonObject;
            Check.That(
                "knowledge_scopes: descriptions include execute-project-code",
                descriptions != null && descriptions["execute-project-code"] != null);
        }

        private static void VerifyGrantsKnowledgeResource(McpHost host, string session)
        {
            JsonNode envelope = host.ReadResource(session, "unitas://knowledge/grants");
            Check.That(
                "knowledge_grants: untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            JsonNode? content = envelope["content"];
            Check.Equal(
                "knowledge_grants: version is 0.1.0",
                "0.1.0",
                content?["version"]?.GetValue<string>() ?? string.Empty);
            JsonArray? disclosures = content?["grantDisclosures"] as JsonArray;
            Check.Equal(
                "knowledge_grants: three mandatory grant disclosures present",
                3,
                disclosures?.Count ?? 0);
            string joined = disclosures?.ToJsonString() ?? string.Empty;
            Check.That(
                "knowledge_grants: discloses edit-scripts + execute-project-code OS account privilege",
                joined.Contains("edit-scripts", StringComparison.Ordinal)
                    && joined.Contains("execute-project-code", StringComparison.Ordinal));
            Check.That(
                "knowledge_grants: discloses secret file readability by project code",
                joined.Contains("secret", StringComparison.Ordinal));
            Check.That(
                "knowledge_grants: discloses shell or file access boundary outside MCP",
                joined.Contains("shell or file access", StringComparison.Ordinal));
            JsonArray? personOnly = content?["personOnlyActions"] as JsonArray;
            Check.That(
                "knowledge_grants: lists personOnlyActions",
                (personOnly?.Count ?? 0) >= 6);
            Check.Equal(
                "knowledge_grants: default reviewPolicy is require-approval",
                "require-approval",
                content?["defaults"]?["reviewPolicy"]?.GetValue<string>() ?? string.Empty);
        }

        private static void VerifyDocsKnowledgeResource(McpHost host, string session)
        {
            JsonNode envelope = host.ReadResource(session, "unitas://knowledge/docs");
            Check.That(
                "knowledge_docs: untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            JsonNode? content = envelope["content"];
            Check.Equal(
                "knowledge_docs: version is 0.1.0",
                "0.1.0",
                content?["version"]?.GetValue<string>() ?? string.Empty);
            Check.Equal(
                "knowledge_docs: unityBaseline is 2022.3.62f1",
                "2022.3.62f1",
                content?["unityBaseline"]?.GetValue<string>() ?? string.Empty);
            JsonArray? documents = content?["documents"] as JsonArray;
            Check.Equal(
                "knowledge_docs: six product spec areas documented",
                6,
                documents?.Count ?? 0);
            JsonArray? tiers = content?["apiReference"]?["tiers"] as JsonArray;
            Check.Equal(
                "knowledge_docs: four compatibility tiers listed",
                4,
                tiers?.Count ?? 0);
            JsonObject? diagnostics = content?["diagnostics"] as JsonObject;
            Check.That(
                "knowledge_docs: diagnostics include CS0103 and permission-denied",
                diagnostics != null
                    && diagnostics["CS0103"] != null
                    && diagnostics["permission-denied"] != null);
            JsonObject? guides = content?["guides"] as JsonObject;
            Check.That(
                "knowledge_docs: workflow guides include scene-authoring-and-review",
                guides != null && guides["scene-authoring-and-review"] != null);
        }
    }
}
