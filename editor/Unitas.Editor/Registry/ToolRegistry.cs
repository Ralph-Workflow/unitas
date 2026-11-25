using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public delegate ToolOutput ToolHandler(JsonNode arguments);

    public sealed record ToolDefinition(string Name, string Description, JsonNode InputSchema, Scope RequiredScope)
    {
        public JsonNode ToValue()
        {
            return new JsonObject
            {
                ["name"] = Name,
                ["description"] = Description,
                ["inputSchema"] = InputSchema.DeepClone(),
                ["requiredScope"] = Scopes.AsString(RequiredScope),
            };
        }
    }

    public sealed record ToolOutput(string Origin, JsonNode Payload, IReadOnlyList<string> AffectedIdentities, string? OperationId)
    {
        public static ToolOutput Untrusted(string origin, JsonNode payload)
        {
            return new ToolOutput(origin, payload, Array.Empty<string>(), null);
        }

        public ToolOutput WithAffectedIdentities(params string[] affected)
        {
            return this with { AffectedIdentities = affected };
        }

        public ToolOutput WithOperation(string operationId)
        {
            return this with { OperationId = operationId };
        }

        public JsonNode ToEnvelopedValue()
        {
            JsonNode envelope = UntrustedContent.Ensure(Origin, Payload.DeepClone());
            if (OperationId != null && envelope is JsonObject map)
            {
                map["operationId"] = OperationId;
            }

            return envelope;
        }
    }

    internal sealed class RegisteredTool
    {
        public RegisteredTool(ToolDefinition definition, ToolHandler handler)
        {
            Definition = definition;
            Handler = handler;
        }

        public ToolDefinition Definition { get; }

        public ToolHandler Handler { get; }
    }

    public sealed class ToolRegistry
    {
        // Sorted so listing order matches the BTreeMap ordering of the Rust registry.
        private readonly SortedDictionary<string, RegisteredTool> tools =
            new SortedDictionary<string, RegisteredTool>(StringComparer.Ordinal);

        public void Register(ToolDefinition definition, ToolHandler handler)
        {
            ArgumentNullException.ThrowIfNull(definition);
            ArgumentNullException.ThrowIfNull(handler);
            if (tools.ContainsKey(definition.Name))
            {
                throw new McpError.DuplicateTool(definition.Name);
            }

            tools.Add(definition.Name, new RegisteredTool(definition, handler));
        }

        public IReadOnlyList<ToolDefinition> List()
        {
            List<ToolDefinition> definitions = new List<ToolDefinition>();
            foreach (RegisteredTool tool in tools.Values)
            {
                definitions.Add(tool.Definition);
            }

            return definitions;
        }

        public ToolDefinition Get(string name)
        {
            return GetRegistered(name).Definition;
        }

        internal RegisteredTool GetRegistered(string name)
        {
            if (!tools.TryGetValue(name, out RegisteredTool? tool))
            {
                throw new McpError.UnknownTool(name);
            }

            return tool;
        }
    }
}
