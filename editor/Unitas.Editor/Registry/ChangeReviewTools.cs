using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class ChangeReviewTools
    {
        public const string PendingOrigin = "unitas://changes/pending";

        public const string PolicyOrigin = "unitas://changes/policy";

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

            host.RegisterTool(
                new ToolDefinition(
                    "changes.list",
                    "Enumerate change sets held under the project review policy, itemized and ordered risk-first (EDT-008)",
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
                    "Show one change set by id with its items ordered risk-first",
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

            host.RegisterTool(
                new ToolDefinition(
                    "changes.accept_item",
                    "Accept one item of a pending change set (person-only, EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["id"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["itemId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("id", "itemId"),
                    },
                    Scope.ReadProject),
                arguments => HandleAcceptItem(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.reject_item",
                    "Reject one item of a pending change set (person-only, EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["id"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["itemId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("id", "itemId"),
                    },
                    Scope.ReadProject),
                arguments => HandleRejectItem(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.revert",
                    "Revert an applied change set and restore the files it changed (person-only, EDT-008)",
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
                arguments => HandleRevert(host, store, arguments));

            RegisterPolicyTools(host, store);

            host.RegisterResource(
                new ResourceDefinition(
                    PendingOrigin,
                    "Pending Change Sets",
                    "Change sets currently held for person review under the project review policy",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(PendingOrigin, PendingPayload(store)));
        }

        private static void RegisterPolicyTools(McpHost host, ChangeSetStore store)
        {
            host.RegisterTool(
                new ToolDefinition(
                    "changes.policy.get",
                    "Report the project review policy (EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                _ => ToolOutput.Untrusted(PolicyOrigin, store.Policy.ToValue()));

            host.RegisterTool(
                new ToolDefinition(
                    "changes.policy.set",
                    "Update the project review policy; absent keys keep their current values (person-only, EDT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["requireApproval"] = new JsonObject
                            {
                                ["type"] = "boolean",
                            },
                            ["holdAgentWrites"] = new JsonObject
                            {
                                ["type"] = "boolean",
                            },
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandlePolicySet(host, store, arguments));
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
            store.EnsureAgentWritesAllowed(toolName, arguments);
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
            return ToolOutput.Untrusted(PendingOrigin, RiskOrderedValue(set));
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
            JsonObject payload = set.ToValue();
            payload["restore"] = store.RestoreReport(set.Id);
            return ToolOutput.Untrusted(PendingOrigin, payload)
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleAcceptItem(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.accept_item");
            string id = RequireId(arguments, "changes.accept_item");
            string itemId = RequireItemId(arguments, "changes.accept_item");
            ChangeSet set = store.AcceptItem(id, itemId);
            return ToolOutput.Untrusted(PendingOrigin, RiskOrderedValue(set))
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleRejectItem(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.reject_item");
            string id = RequireId(arguments, "changes.reject_item");
            string itemId = RequireItemId(arguments, "changes.reject_item");
            ChangeSet set = store.RejectItem(id, itemId);
            return ToolOutput.Untrusted(PendingOrigin, RiskOrderedValue(set))
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleRevert(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.revert");
            string id = RequireId(arguments, "changes.revert");
            JsonObject report = store.Revert(id);
            ChangeSet set = store.Get(id);
            JsonObject payload = RiskOrderedValue(set);
            payload["restore"] = report;
            return ToolOutput.Untrusted(PendingOrigin, payload)
                .WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandlePolicySet(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            RequirePerson(host, arguments, "changes.policy.set");
            ReviewPolicy current = store.Policy;
            ReviewPolicy updated = new ReviewPolicy(
                OptionalBool(arguments, "requireApproval") ?? current.RequireApproval,
                OptionalBool(arguments, "holdAgentWrites") ?? current.HoldAgentWrites);
            store.SetPolicy(updated);
            return ToolOutput.Untrusted(PolicyOrigin, updated.ToValue());
        }

        private static JsonObject RiskOrderedValue(ChangeSet set)
        {
            JsonObject value = set.ToValue();
            if (value["items"] is not JsonArray items)
            {
                return value;
            }

            List<(int Severity, int Index, JsonNode Item)> entries = new List<(int, int, JsonNode)>();
            for (int index = 0; index < items.Count; index++)
            {
                if (items[index] is JsonNode item)
                {
                    entries.Add((SeverityOf(item), index, item.DeepClone()));
                }
            }

            entries.Sort((left, right) => left.Severity != right.Severity
                ? left.Severity.CompareTo(right.Severity)
                : left.Index.CompareTo(right.Index));
            JsonArray ordered = new JsonArray();
            foreach ((int _, int _, JsonNode item) in entries)
            {
                ordered.Add(item);
            }

            value["items"] = ordered;
            return value;
        }

        private static int SeverityOf(JsonNode item)
        {
            string risk = item["risk"]?.GetValue<string>() ?? string.Empty;
            for (int index = 0; index < ChangeRisks.SeverityOrder.Count; index++)
            {
                if (string.Equals(ChangeRisks.AsString(ChangeRisks.SeverityOrder[index]), risk, StringComparison.Ordinal))
                {
                    return index;
                }
            }

            return ChangeRisks.SeverityOrder.Count;
        }

        private static bool? OptionalBool(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out bool parsed))
            {
                return parsed;
            }

            return null;
        }

        private static string RequireItemId(JsonNode arguments, string actionName)
        {
            string? itemId = ExtractOptionalString(arguments, "itemId");
            if (string.IsNullOrWhiteSpace(itemId))
            {
                throw new McpError.InvalidParams(actionName + " requires an 'itemId'");
            }

            return itemId!;
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
                changeSets.Add(RiskOrderedValue(set));
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
                changeSets.Add(RiskOrderedValue(set));
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
