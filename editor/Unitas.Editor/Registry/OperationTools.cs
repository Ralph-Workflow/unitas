using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    // Read-level views over the operation handles the host already maintains, so
    // a client can follow a long-running tool without a second channel.
    public static class OperationTools
    {
        public const string ListOrigin = "unitas://operations/list";
        public const string GetOrigin = "unitas://operations/get";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "operations.list",
                    "Enumerate the operation handles the host has started",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                _ => ToolOutput.Untrusted(ListOrigin, ListPayload(host)));

            host.RegisterTool(
                new ToolDefinition(
                    "operations.get",
                    "Show one operation handle by id",
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
                arguments => HandleGet(host, arguments));
        }

        private static ToolOutput HandleGet(McpHost host, JsonNode arguments)
        {
            string id = RequireId(arguments);
            return ToolOutput.Untrusted(GetOrigin, host.GetOperation(id).ToValue());
        }

        private static JsonObject ListPayload(McpHost host)
        {
            JsonArray operations = new JsonArray();
            foreach (OperationHandle handle in host.Operations.All)
            {
                operations.Add(handle.ToValue());
            }

            return new JsonObject
            {
                ["operations"] = operations,
            };
        }

        private static string RequireId(JsonNode arguments)
        {
            if (arguments is JsonObject argumentObject
                && argumentObject["id"] is JsonValue value
                && value.TryGetValue(out string? id)
                && !string.IsNullOrWhiteSpace(id))
            {
                return id;
            }

            throw new McpError.InvalidParams("operations.get requires a non-empty 'id' argument");
        }
    }
}
