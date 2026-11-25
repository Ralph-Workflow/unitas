using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class ChangeReviewTools
    {
        public const string PendingOrigin = "unitas://changes/pending";

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

            host.RegisterTool(
                new ToolDefinition(
                    "changes.list",
                    "Enumerate change sets held under the project review policy (EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                _ => ToolOutput.Untrusted(PendingOrigin, ListPayload(store)));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.show",
                    "Show one change set by id",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["id"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("id"),
                    },
                    Scope.ReadProject),
                arguments => HandleShow(store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.approve",
                    "Approve a pending change set and apply it (person-only, EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["id"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("id"),
                    },
                    Scope.ReadProject),
                arguments => HandleApprove(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.reject",
                    "Reject a pending change set with a reason (person-only, EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["id"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["reason"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("id", "reason"),
                    },
                    Scope.ReadProject),
                arguments => HandleReject(host, store, arguments));

            host.RegisterResource(
                new ResourceDefinition(
                    PendingOrigin,
                    "Pending Change Sets",
                    "Change sets currently held for person review under the project review policy",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(PendingOrigin, PendingPayload(store)));
        }

        // A mutating tool calls this instead of executing: the mutation is held
        // as a pending change set and stays unapplied until a person approves it.
        public static ToolOutput HoldMutation(
            McpHost host,
            ChangeSetStore store,
            string toolName,
            JsonNode arguments)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);
            ArgumentNullException.ThrowIfNull(toolName);
            string sessionId = RequireCallerSession(host);
            SessionIdentity identity = host.GetSessionIdentity(sessionId);
            JsonObject operation = new JsonObject
            {
                ["tool"] = toolName,
                ["arguments"] = arguments?.DeepClone(),
            };
            ChangeSet set = store.Hold(identity.Agent, operation);
            JsonObject payload = new JsonObject
            {
                ["status"] = "held-for-review",
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(PendingOrigin, payload)
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleShow(ChangeSetStore store, JsonNode arguments)
        {
            ChangeSet set = store.Get(RequireId(arguments, "changes.show"));
            return ToolOutput.Untrusted(PendingOrigin, set.ToValue());
        }

        private static ToolOutput HandleApprove(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.approve");
            ChangeSet set = store.Approve(RequireId(arguments, "changes.approve"));
            return ToolOutput.Untrusted(PendingOrigin, set.ToValue())
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleReject(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.reject");
            string id = RequireId(arguments, "changes.reject");
            string? reason = ExtractOptionalString(arguments, "reason");
            if (string.IsNullOrWhiteSpace(reason))
            {
                throw new McpError.InvalidParams("changes.reject requires a 'reason'");
            }

            ChangeSet set = store.Reject(id, reason!);
            return ToolOutput.Untrusted(PendingOrigin, set.ToValue())
                .WithAffectedIdentities(set.Id);
        }

        private static void RequirePerson(McpHost host, JsonNode arguments, string actionName)
        {
            string sessionId = RequireCallerSession(host);
            SessionIdentity identity = host.GetSessionIdentity(sessionId);
            string? token = ExtractOptionalString(arguments, "personCredential");
            host.PersonCredential.RequirePerson(token, identity.Client, actionName);
        }

        private static JsonObject PendingPayload(ChangeSetStore store)
        {
            JsonArray changeSets = new JsonArray();
            foreach (ChangeSet set in store.Pending)
            {
                changeSets.Add(set.ToValue());
            }

            return new JsonObject
            {
                ["changeSets"] = changeSets,
            };
        }

        private static JsonObject ListPayload(ChangeSetStore store)
        {
            JsonArray changeSets = new JsonArray();
            foreach (ChangeSet set in store.All)
            {
                changeSets.Add(set.ToValue());
            }

            return new JsonObject
            {
                ["changeSets"] = changeSets,
            };
        }

        private static string RequireCallerSession(McpHost host)
        {
            string? sessionId = host.CurrentSessionId;
            if (string.IsNullOrEmpty(sessionId))
            {
                throw new McpError.InvalidRequest("tool invocation requires an active MCP session");
            }

            return sessionId!;
        }

        private static string RequireId(JsonNode arguments, string actionName)
        {
            string? id = ExtractOptionalString(arguments, "id");
            if (string.IsNullOrWhiteSpace(id))
            {
                throw new McpError.InvalidParams(actionName + " requires an 'id'");
            }

            return id!;
        }

        private static string? ExtractOptionalString(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            return null;
        }
    }
}
