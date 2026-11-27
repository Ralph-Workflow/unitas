using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public delegate ToolOutput ToolHandler(JsonNode arguments);

    public sealed record ToolDefinition(
        string Name,
        string Description,
        JsonNode InputSchema,
        Scope RequiredScope,
        string? Reversibility = null)
    {
        public string EffectiveReversibility => Reversibility ?? ClassifyReversibility(Name, RequiredScope);

        public bool IsReversible => !string.Equals(EffectiveReversibility, "irreversible", StringComparison.Ordinal);

        public JsonNode ToValue()
        {
            return new JsonObject
            {
                ["name"] = Name,
                ["description"] = Description,
                ["inputSchema"] = InputSchema.DeepClone(),
                ["requiredScope"] = Scopes.AsString(RequiredScope),
                ["reversibility"] = EffectiveReversibility,
                ["reversible"] = IsReversible,
            };
        }

        private static string ClassifyReversibility(string name, Scope requiredScope)
        {
            if (IsIrreversibleTool(name))
            {
                return "irreversible";
            }

            if (IsReversibleDebugTool(name))
            {
                return "reversible";
            }

            if (requiredScope == Scope.ReadProject || IsReadOnlyDebugTool(name))
            {
                return "read-only";
            }

            return "reversible";
        }

        private static bool IsIrreversibleTool(string name)
        {
            switch (name)
            {
                case "scene.save":
                case "build.player":
                case "scripts.compile":
                case "scripts.diagnose":
                case "tests.run":
                case "scenario.run":
                case "editor.execute_method":
                case "editor.invoke_command":
                case "debug.evaluate":
                case "permissions.request_grant":
                case "permissions.person_grant":
                case "audit.delete":
                case "audit.clear":
                case "audit.alter":
                    return true;
                default:
                    return false;
            }
        }

        private static bool IsReversibleDebugTool(string name)
        {
            switch (name)
            {
                case "debug.set_breakpoints":
                case "debug.continue":
                case "debug.step":
                case "changes.approve":
                case "changes.reject":
                    return true;
                default:
                    return false;
            }
        }

        private static bool IsReadOnlyDebugTool(string name)
        {
            switch (name)
            {
                case "debug.stack":
                case "debug.locals":
                case "debug.wait_stopped":
                    return true;
                default:
                    return false;
            }
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

        public IReadOnlyList<ToolDefinition> Definitions()
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
