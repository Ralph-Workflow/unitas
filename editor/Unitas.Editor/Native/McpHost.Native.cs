using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed partial class McpHost
    {
        // Every native tool is registered through the same pipeline as a built-in:
        // scope-gated by the authorizer, enveloped by ToolOutput, and audited by CallTool.
        public void RegisterNativeTools(IEnumerable<NativeToolDescriptor> descriptors, Func<string, JsonNode, JsonNode> dispatch)
        {
            ArgumentNullException.ThrowIfNull(descriptors);
            ArgumentNullException.ThrowIfNull(dispatch);
            foreach (NativeToolDescriptor descriptor in descriptors)
            {
                NativeToolDescriptor tool = descriptor;
                RegisterTool(
                    new ToolDefinition(tool.Name, tool.Description, tool.InputSchema, tool.RequiredScope),
                    arguments => ToolOutput.Untrusted(tool.Origin, dispatch(tool.Name, arguments)));
            }
        }
    }
}
