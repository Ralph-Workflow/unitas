using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed record NativeToolDescriptor(string Name, string Description, JsonNode InputSchema, Scope RequiredScope, string Origin);
}
